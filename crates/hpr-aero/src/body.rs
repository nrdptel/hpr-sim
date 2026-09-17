//! Bodies of revolution (nose cones, body tubes, transitions): Barrowman's normal-force slope and
//! centre of pressure, and Galejs's body lift.
//!
//! - **Potential flow** (slender-body theory). A body whose cross-section area runs from `A(0)` at
//!   its fore end to `A(l)` at its aft end has
//!   `(C_Nα)_B = (2/A_ref)[A(l) − A(0)]` (Barrowman 1966 eq. 10, 1967 eq. 3-65; Niskanen 2009
//!   eq. 3.19) and its centre of pressure `X_B = [l A(l) − V] / [A(l) − A(0)]` aft of its fore end,
//!   with `V` its volume (Barrowman 1966 eq. 28, 1967 eq. 3-89; Niskanen eq. 3.28). The moment
//!   slope `(2/A_ref)[l A(l) − V]` (Niskanen eq. 3.25 times `d`) stays well conditioned when
//!   `A(l) ≈ A(0)`. At an angle of attack `α`, Niskanen keeps the `sin α / α` factor of the
//!   crossflow `v₀ sin α` (eq. 3.19). Slender-body theory has no Mach term: the body's slope is
//!   the same at every subsonic Mach number (Barrowman 1967 p. 3; Niskanen p. 22).
//! - **Body lift** (Galejs, after Hoerner p. 3-11; Niskanen eq. 3.26–3.27):
//!   `C_N = K (A_plan/A_ref) sin² α` with `K = 1.1`, acting at the centroid of the side-view
//!   (planform) area. It is zero at `α = 0`, so it doesn't change `C_Nα` there.
//!
//! See `docs/physics/aero.md`.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use crate::error::{AeroError, check_dimension};

/// Galejs's body-lift constant `K` (Niskanen 2009 eq. 3.26: "K ≈ 1.1"; Galejs quotes Hoerner's
/// 1.1 to 1.5 and fitted 1.0 to his own data).
pub const BODY_LIFT_K: f64 = 1.1;

/// The aerodynamic geometry of one body component, in its own frame (fore end at 0, stations
/// positive aft).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BodyGeometry {
    /// Length, m.
    pub length_m: f64,
    /// Cross-section area at the fore end, m².
    pub fore_area_m2: f64,
    /// Cross-section area at the aft end, m².
    pub aft_area_m2: f64,
    /// Volume enclosed by the outer surface, m³.
    pub volume_m3: f64,
    /// Side-view (planform) area, m².
    pub planform_area_m2: f64,
    /// Centroid of the planform area, m aft of the fore end.
    pub planform_centroid_m: f64,
}

impl BodyGeometry {
    /// A cylinder of `radius_m` and `length_m`.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a non-positive or non-finite dimension.
    pub fn cylinder(length_m: f64, radius_m: f64) -> Result<Self, AeroError> {
        check_dimension("body length", length_m, false)?;
        check_dimension("body radius", radius_m, false)?;
        let area = PI * radius_m * radius_m;
        Ok(Self {
            length_m,
            fore_area_m2: area,
            aft_area_m2: area,
            volume_m3: area * length_m,
            planform_area_m2: 2.0 * radius_m * length_m,
            planform_centroid_m: 0.5 * length_m,
        })
    }

    /// Checks that every field is finite and in range.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a non-finite or negative value, or a length or volume that isn't
    /// positive.
    pub fn validate(&self) -> Result<(), AeroError> {
        check_dimension("body length", self.length_m, false)?;
        check_dimension("body fore area", self.fore_area_m2, true)?;
        check_dimension("body aft area", self.aft_area_m2, true)?;
        check_dimension("body volume", self.volume_m3, false)?;
        check_dimension("body planform area", self.planform_area_m2, false)?;
        if !self.planform_centroid_m.is_finite() {
            return Err(AeroError::Domain {
                what: "body planform centroid",
                value: self.planform_centroid_m,
            });
        }
        Ok(())
    }

    /// Normal-force slope at `α → 0`, per radian: `(C_Nα)_B = (2/A_ref)[A(l) − A(0)]`
    /// (Barrowman 1967 eq. 3-65; Niskanen 2009 eq. 3.19). Negative for a boattail.
    pub fn normal_force_slope(&self, reference_area_m2: f64) -> f64 {
        2.0 * (self.aft_area_m2 - self.fore_area_m2) / reference_area_m2
    }

    /// Moment slope about the fore end at `α → 0`, m per radian:
    /// `(C_Nα)_B X_B = (2/A_ref)[l A(l) − V]` (Niskanen 2009 eq. 3.25 times the reference length).
    pub fn moment_slope_m(&self, reference_area_m2: f64) -> f64 {
        2.0 * (self.length_m * self.aft_area_m2 - self.volume_m3) / reference_area_m2
    }

    /// Centre of pressure of the potential-flow term, m aft of the fore end:
    /// `X_B = [l A(l) − V] / [A(l) − A(0)]` (Barrowman 1966 eq. 28; Niskanen 2009 eq. 3.28).
    /// `None` when the two end areas are equal, where the body has no potential-flow normal force.
    pub fn centre_of_pressure_m(&self) -> Option<f64> {
        let delta = self.aft_area_m2 - self.fore_area_m2;
        (delta != 0.0).then(|| (self.length_m * self.aft_area_m2 - self.volume_m3) / delta)
    }

    /// Galejs's body-lift normal-force coefficient at `alpha_rad`:
    /// `C_N = K (A_plan/A_ref) sin² α` (Niskanen 2009 eq. 3.26), acting at
    /// [`BodyGeometry::planform_centroid_m`] (eq. 3.27).
    pub fn lift_coefficient(&self, reference_area_m2: f64, alpha_rad: f64) -> f64 {
        let s = alpha_rad.sin();
        BODY_LIFT_K * self.planform_area_m2 / reference_area_m2 * s * s
    }
}

/// `sin x / x`, with its series near zero.
pub(crate) fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-4 {
        1.0 - x * x / 6.0
    } else {
        x.sin() / x
    }
}

#[cfg(test)]
mod tests {
    use hpr_design::{NoseShape, Profile, Wall, revolve};

    use super::*;

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = ((got - want) / want).abs();
        assert!(
            err <= rel,
            "{what}: got {got}, want {want}, rel err {err:e}"
        );
    }

    fn from_profile(profile: &Profile) -> BodyGeometry {
        let g = revolve(profile, Wall::Filled {}).unwrap();
        let area = |r: f64| PI * r * r;
        BodyGeometry {
            length_m: profile.length_m(),
            fore_area_m2: area(profile.fore_radius_m()),
            aft_area_m2: area(profile.aft_radius_m()),
            volume_m3: g.volume_m3,
            planform_area_m2: g.planform_area_m2,
            planform_centroid_m: g.planform_centroid_m,
        }
    }

    /// Conical frustum CP from Barrowman 1966 eq. 44 (with the p. 20 correction), `d₁` fore:
    /// `X = (L/3)[1 + 1/(1 + d₁/d₂)]`. It holds for boattails too (p. 21).
    fn frustum_cp(length: f64, fore_radius: f64, aft_radius: f64) -> f64 {
        length / 3.0 * (1.0 + 1.0 / (1.0 + fore_radius / aft_radius))
    }

    /// Loft lesson L9: the CP of a non-conical transition comes from its own volume, not from the
    /// conical-frustum formula.
    #[test]
    fn ogive_transition_cp_uses_volume_form() {
        let (length, fore, aft) = (0.12, 0.02, 0.04);
        let ogive = from_profile(
            &Profile::transition(
                NoseShape::Ogive { radius_ratio: 1.0 },
                length,
                fore,
                aft,
                false,
            )
            .unwrap(),
        );
        let cone = from_profile(
            &Profile::transition(NoseShape::Conical {}, length, fore, aft, false).unwrap(),
        );
        // The conical transition reproduces eq. 44 exactly.
        close(
            cone.centre_of_pressure_m().unwrap(),
            frustum_cp(length, fore, aft),
            1e-11,
            "conical transition CP",
        );
        // The ogive's volume form, written out independently: X = [l A(l) − V] / ΔA with V from
        // Simpson's rule on the profile's radius.
        let profile = Profile::transition(
            NoseShape::Ogive { radius_ratio: 1.0 },
            length,
            fore,
            aft,
            false,
        )
        .unwrap();
        let n = 20_000;
        let h = length / f64::from(n);
        let volume: f64 = (0..=n)
            .map(|i| {
                let r = profile.radius_m(f64::from(i) * h);
                let w = if i == 0 || i == n {
                    1.0
                } else if i % 2 == 1 {
                    4.0
                } else {
                    2.0
                };
                w * PI * r * r
            })
            .sum::<f64>()
            * h
            / 3.0;
        let want = (length * PI * aft * aft - volume) / (PI * (aft * aft - fore * fore));
        close(
            ogive.centre_of_pressure_m().unwrap(),
            want,
            1e-9,
            "ogive transition CP",
        );
        // The two differ by far more than round-off: the conical formula is wrong for an ogive.
        let gap = ogive.centre_of_pressure_m().unwrap() - frustum_cp(length, fore, aft);
        assert!(gap.abs() > 1e-3 * length, "gap {gap}");
        // Same slope: it depends on the end areas only.
        assert_eq!(ogive.normal_force_slope(1.0), cone.normal_force_slope(1.0));
    }

    /// The potential-flow terms at their limits: a cone, a cylinder (no force, no CP), a boattail
    /// (negative slope, eq. 44 still holds), a thin transition (the moment stays finite), and the
    /// cone's moment slope `(2/A_ref)(l A − V) = (4/3) l` for `A_ref = A`.
    #[test]
    fn potential_flow_terms_at_their_limits() {
        let cone = from_profile(&Profile::nose(NoseShape::Conical {}, 0.3, 0.05).unwrap());
        let a_ref = PI * 0.05 * 0.05;
        close(cone.normal_force_slope(a_ref), 2.0, 1e-15, "cone slope");
        close(cone.centre_of_pressure_m().unwrap(), 0.2, 1e-11, "cone CP");
        close(cone.moment_slope_m(a_ref), 0.4, 1e-11, "cone moment");

        let tube = BodyGeometry::cylinder(0.5, 0.05).unwrap();
        assert_eq!(tube.normal_force_slope(a_ref), 0.0);
        assert_eq!(tube.centre_of_pressure_m(), None);
        assert!(tube.moment_slope_m(a_ref).abs() < 1e-15);

        let boattail = from_profile(
            &Profile::transition(NoseShape::Conical {}, 0.1, 0.05, 0.03, false).unwrap(),
        );
        close(
            boattail.normal_force_slope(a_ref),
            2.0 * (0.36 - 1.0),
            1e-14,
            "boattail slope",
        );
        close(
            boattail.centre_of_pressure_m().unwrap(),
            frustum_cp(0.1, 0.05, 0.03),
            1e-11,
            "boattail CP",
        );

        // A transition 1 µm wide has a vanishing slope and a CP near its middle; its moment
        // slope stays tiny and finite.
        let thin = from_profile(
            &Profile::transition(NoseShape::Conical {}, 0.1, 0.05, 0.050_001, false).unwrap(),
        );
        assert!(thin.normal_force_slope(a_ref) > 0.0);
        close(thin.centre_of_pressure_m().unwrap(), 0.05, 1e-4, "thin CP");
        assert!(thin.moment_slope_m(a_ref).abs() < 1e-5);
    }

    /// Body lift: zero at `α = 0`, `K A_plan/A_ref` broadside, symmetric in the sign of `α`, and
    /// a cylinder's planform `2 r l` acting at its middle (Galejs Table 1).
    #[test]
    fn body_lift_limits() {
        let tube = BodyGeometry::cylinder(0.8, 0.04).unwrap();
        let a_ref = PI * 0.04 * 0.04;
        assert_eq!(tube.lift_coefficient(a_ref, 0.0), 0.0);
        let broadside = BODY_LIFT_K * 2.0 * 0.04 * 0.8 / a_ref;
        close(
            tube.lift_coefficient(a_ref, std::f64::consts::FRAC_PI_2),
            broadside,
            1e-15,
            "broadside",
        );
        close(
            tube.lift_coefficient(a_ref, 0.1),
            tube.lift_coefficient(a_ref, -0.1),
            1e-15,
            "symmetric",
        );
        close(
            tube.lift_coefficient(a_ref, 0.01),
            broadside * 0.01f64.sin().powi(2),
            1e-15,
            "small angle",
        );
        assert_eq!(tube.planform_centroid_m, 0.4);
        // A cone's planform is ½ L D at 2L/3 (Galejs Table 1).
        let cone = from_profile(&Profile::nose(NoseShape::Conical {}, 0.3, 0.05).unwrap());
        close(
            cone.planform_area_m2,
            0.5 * 0.3 * 0.1,
            1e-12,
            "cone planform",
        );
        close(
            cone.planform_centroid_m,
            0.2,
            1e-12,
            "cone planform centroid",
        );
    }

    #[test]
    fn sinc_is_continuous() {
        assert_eq!(sinc(0.0), 1.0);
        for x in [1e-5, 9.9e-5, 1e-4, 1.01e-4] {
            close(sinc(x), x.sin() / x, 1e-15, "sinc");
        }
    }

    #[test]
    fn bad_geometry_is_refused() {
        assert!(BodyGeometry::cylinder(0.0, 0.1).is_err());
        assert!(BodyGeometry::cylinder(1.0, f64::NAN).is_err());
        let mut g = BodyGeometry::cylinder(1.0, 0.1).unwrap();
        g.volume_m3 = -1.0;
        assert!(g.validate().is_err());
    }
}
