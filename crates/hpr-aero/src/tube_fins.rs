//! Tube fins: a ring of short open tubes around the body, each flown as an annular wing (a ring
//! airfoil).
//!
//! **Normal force.** One tube of mean diameter `d` and length `L`, with `λ = L/d`, takes
//! Weissinger's approximation for a thin ring wing (Weissinger 1955, as quoted by Wagner 2021
//! eq. 15, for `λ < 5`), on the area `d L`:
//!
//! `C_Lα = π² / (1 + πλ/2 + λ arctan(1.2 λ))` per radian.
//!
//! Short rings tend to Ribner's lifting-line result `π²` (Wagner eq. 13), and long ones to
//! slender-body theory's `π/λ`, which is Hoerner's `L = q d² π α` for a ring of small aspect ratio
//! (Hoerner 1965 p. 7-13): the ring deflects the air inside it as well as the air around it, so it
//! lifts twice as much as a solid body of its diameter. Fletcher's measured slopes on five rings
//! (NACA TN 4117, 1957, Fig. 11, at Mach 0.13) lie within 3% of it.
//!
//! Compressibility follows Göthert's rule, as Barrowman's fin slope does: the slope at Mach `M` is
//! the incompressible slope of the ring stretched to `λ/β`, over `β = √(1 − M²)`. It leaves the
//! slender limit unchanged and turns the lifting-line one into Prandtl–Glauert's. Nothing measures
//! tube fins near the speed of sound, where the flow through a tube may choke, so the model
//! refuses Mach [`TUBE_FIN_MACH_LIMIT`] and above.
//!
//! **Centre of pressure.** Against the ring's aspect ratio `A = d/L`, at the stretched ring's
//! `β A` faster than Mach 0 ([`ring_centre_fraction`]): Fletcher's measured aerodynamic centre
//! from `A = 2/3` to 3 ([`FLETCHER_AERODYNAMIC_CENTRE`], his Fig. 8), where his rings act as
//! wings, held at 3 beyond; below `A = 2/3`, a straight line to the leading edge at `A = 0`, where
//! slender-body theory puts a thin ring's lift (it all comes where the ring's apparent mass
//! appears, at its leading edge). Fletcher's fifth ring, at `A = 1/3`, is left out: its centre
//! sits ahead of its leading edge, which he puts down to that thick ring (a Clark Y section, 11.7%
//! of a chord three diameters long) acting like a body of revolution, and a paper tube is not
//! thick. No thin tube's centre is measured.
//!
//! **The set.** `N` tubes add `N` times one tube's slope, with no interference from the body or
//! between the tubes: none is measured or cited. The body's own crossflow disturbance at the tubes
//! goes as `R²/s² e^{−2iφ}` around it, which sums to zero over three or more tubes evenly spaced,
//! so the model refuses fewer than three. This is a derivation, not a measurement
//! (`docs/physics/aero.md`, *Tube fins*).

use std::f64::consts::PI;

use hpr_design::{PlacedComponent, TubeFinSet};
use serde::Serialize;

use crate::error::{AeroError, check_dimension, check_mach};

/// The top of the tube-fin model's range, Mach 0.8, where hpr's fin model leaves its subsonic
/// method ([`crate::fins::TRANSONIC_START_MACH`]). No source covers tube fins faster; a judgement.
pub const TUBE_FIN_MACH_LIMIT: f64 = 0.8;

/// Fletcher's measured aerodynamic centre of five annular airfoils, `(A, x_ac/c)`: the aspect
/// ratio `A = d/c` (diameter over chord) and the aerodynamic centre's distance aft of the leading
/// edge as a fraction of the chord, from α = 0° to 10° at Mach 0.13 (NACA TN 4117, 1957, Fig. 8,
/// p. 16). Read from the chart to its finest grid line, 0.02 of the chord, and checked against
/// the text: the centre moves aft as `A` rises, and sits ahead of the leading edge at `A = 1/3`
/// (p. 4–5).
pub const FLETCHER_AERODYNAMIC_CENTRE: [(f64, f64); 5] = [
    (1.0 / 3.0, -0.11),
    (2.0 / 3.0, 0.143),
    (1.0, 0.203),
    (1.5, 0.253),
    (3.0, 0.355),
];

/// The points [`ring_centre_fraction`] joins, `(A, x_ac/c)`: slender-body theory's leading edge
/// at `A = 0`, then Fletcher's four rings that act as wings ([`FLETCHER_AERODYNAMIC_CENTRE`] from
/// `A = 2/3`).
pub const THIN_RING_CENTRE: [(f64, f64); 5] = [
    (0.0, 0.0),
    FLETCHER_AERODYNAMIC_CENTRE[1],
    FLETCHER_AERODYNAMIC_CENTRE[2],
    FLETCHER_AERODYNAMIC_CENTRE[3],
    FLETCHER_AERODYNAMIC_CENTRE[4],
];

/// A thin ring wing's normal-force slope per radian on the area `d L`, at a length-to-diameter
/// ratio `λ = L/d`: Weissinger's `π² / (1 + πλ/2 + λ arctan(1.2 λ))` (Wagner 2021 eq. 15).
///
/// # Errors
///
/// [`AeroError::Domain`] for a `λ` that is not finite and positive.
pub fn ring_lift_slope(length_over_diameter: f64) -> Result<f64, AeroError> {
    check_dimension("tube length over diameter", length_over_diameter, false)?;
    Ok(weissinger(length_over_diameter))
}

/// [`ring_lift_slope`] at a checked `λ`.
fn weissinger(l: f64) -> f64 {
    PI * PI / (1.0 + 0.5 * PI * l + l * (1.2 * l).atan())
}

/// A thin ring's aerodynamic centre aft of its leading edge, as a fraction of its length, at an
/// aspect ratio `A = d/L`: [`THIN_RING_CENTRE`] interpolated linearly in `A`, and held at `A = 3`
/// beyond it.
///
/// # Errors
///
/// [`AeroError::Domain`] for an `A` that is not finite and positive.
pub fn ring_centre_fraction(aspect_ratio: f64) -> Result<f64, AeroError> {
    check_dimension("tube diameter over length", aspect_ratio, false)?;
    Ok(fletcher_centre(aspect_ratio))
}

/// [`ring_centre_fraction`] at a checked `A`.
fn fletcher_centre(aspect_ratio: f64) -> f64 {
    let table = &THIN_RING_CENTRE;
    let last = table[table.len() - 1];
    for pair in table.windows(2) {
        let [(a0, x0), (a1, x1)] = [pair[0], pair[1]];
        if aspect_ratio <= a1 {
            return x0 + (x1 - x0) * (aspect_ratio - a0) / (a1 - a0);
        }
    }
    last.1
}

/// A tube fin set's precomputed terms.
///
/// Serialize-only, like [`crate::AeroModel`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct TubeFinSetAero {
    /// The component's id.
    pub id: String,
    /// Number of tubes, at least 3.
    pub count: u32,
    /// Station of the tubes' leading edges, m aft of the nose tip.
    pub fore_station_m: f64,
    /// A tube's length `L`, m.
    pub length_m: f64,
    /// A tube's mean diameter `d`, m: its outer and inner radii added.
    pub mean_diameter_m: f64,
    /// Distance of each tube's axis from the rocket's axis, m: the body's radius plus the tube's
    /// outer radius.
    pub axis_radius_m: f64,
    /// One tube's area `d L` over the reference area.
    pub area_ratio: f64,
}

impl TubeFinSetAero {
    /// The terms of `set` on `component`, on a rocket of reference area `reference_area_m2`.
    ///
    /// # Errors
    ///
    /// [`AeroError::Unsupported`] for fewer than three tubes, [`AeroError::Layout`] for a set
    /// without the radius of its body tube, and [`AeroError::Domain`] for a bad dimension.
    pub fn new(
        component: &PlacedComponent,
        set: &TubeFinSet,
        reference_area_m2: f64,
    ) -> Result<Self, AeroError> {
        if set.count < 3 {
            return Err(AeroError::Unsupported(format!(
                "{} tube fins (the model needs three or more, evenly spaced, for the body's flow \
                 to cancel around them)",
                set.count
            )));
        }
        let body_radius = component.body_radius_m.ok_or_else(|| {
            AeroError::Layout("a tube fin set needs the radius of the body tube it is on".into())
        })?;
        check_dimension("body radius", body_radius, false)?;
        check_dimension("tube fin length", set.length_m, false)?;
        check_dimension("tube fin outer radius", set.outer_radius_m, false)?;
        check_dimension("tube fin thickness", set.thickness_m, true)?;
        check_dimension("reference area", reference_area_m2, false)?;
        // A wall thicker than the radius is a solid rod, which is not a ring wing.
        let inner = set.outer_radius_m - set.thickness_m;
        if inner <= 0.0 {
            return Err(AeroError::Unsupported(
                "solid tube fins (a wall as thick as the tube's radius)".to_owned(),
            ));
        }
        let mean_diameter = set.outer_radius_m + inner;
        Ok(Self {
            id: component.id.clone(),
            count: set.count,
            fore_station_m: component.fore_station_m,
            length_m: set.length_m,
            mean_diameter_m: mean_diameter,
            axis_radius_m: body_radius + set.outer_radius_m,
            area_ratio: mean_diameter * set.length_m / reference_area_m2,
        })
    }

    /// The whole set's normal-force slope per radian on the reference area at `mach`, and its
    /// centre of pressure, m aft of the nose tip: `N C_Lα(λ/β)/β · d L/A_ref` at
    /// `x_ac(β A) L` behind the leading edge.
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] outside `[0, 0.8)`.
    pub fn loading(&self, mach: f64) -> Result<(f64, f64), AeroError> {
        check_tube_fin_mach(mach)?;
        Ok(self.loading_at(mach))
    }

    /// [`Self::loading`] at a Mach number already checked.
    pub(crate) fn loading_at(&self, mach: f64) -> (f64, f64) {
        let beta = (1.0 - mach * mach).sqrt();
        // Both positive: the dimensions are checked in `new` and `β > 0.6` below Mach 0.8.
        let length_over_diameter = self.length_m / self.mean_diameter_m;
        let slope = f64::from(self.count) * weissinger(length_over_diameter / beta) / beta
            * self.area_ratio;
        let centre = fletcher_centre(beta / length_over_diameter);
        (slope, self.fore_station_m + centre * self.length_m)
    }

    /// The set's roll damping `C_lp` at `mach` on a reference diameter `reference_diameter_m`:
    /// each tube crosses the air at `p ρ` under the roll rate `p`, `ρ` its axis's distance from the
    /// rocket's, so its normal force about the axis gives `−2 C_Nα ρ²/d²`, as a pod's does
    /// ([`crate::AeroModel::roll`]).
    ///
    /// # Errors
    ///
    /// As [`Self::loading`].
    pub fn roll_damping(&self, mach: f64, reference_diameter_m: f64) -> Result<f64, AeroError> {
        check_tube_fin_mach(mach)?;
        check_dimension("reference diameter", reference_diameter_m, false)?;
        let (slope, _) = self.loading_at(mach);
        let rho = self.axis_radius_m;
        Ok(-2.0 * slope * rho * rho / (reference_diameter_m * reference_diameter_m))
    }
}

/// Checks a Mach number in the tube-fin model's range, `[0, 0.8)`.
///
/// # Errors
///
/// [`AeroError::Mach`] outside it.
pub(crate) fn check_tube_fin_mach(mach: f64) -> Result<(), AeroError> {
    check_mach(mach, TUBE_FIN_MACH_LIMIT, "the tube-fin model")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fletcher's measured lift-curve slopes, per degree on the area `d c`, against `A = d/c`
    /// (NACA TN 4117, 1957, Fig. 11, p. 19), read to the chart's finest grid line, 0.002.
    const FLETCHER_SLOPE_PER_DEG: [(f64, f64); 5] = [
        (1.0 / 3.0, 0.018),
        (2.0 / 3.0, 0.034),
        (1.0, 0.049),
        (1.5, 0.069),
        (3.0, 0.102),
    ];

    #[test]
    fn weissinger_s_slope_lies_within_three_per_cent_of_fletcher_s_rings() {
        for (a, measured) in FLETCHER_SLOPE_PER_DEG {
            let ours = ring_lift_slope(1.0 / a).unwrap() * PI / 180.0;
            // The largest gap is 2.3%, at A = 2/3 and 3.
            assert!(
                (ours - measured).abs() <= 0.03 * measured,
                "A = {a}: {ours} against {measured}"
            );
        }
        // At A = 1 by hand: π²/(1 + π/2 + arctan 1.2) = 2.8633 per radian.
        let by_hand = PI * PI / (1.0 + 0.5 * PI + 1.2_f64.atan());
        assert!((ring_lift_slope(1.0).unwrap() - by_hand).abs() < 1e-15);
        assert!((by_hand - 2.8633).abs() < 1e-4);
    }

    #[test]
    fn a_ring_tends_to_lifting_line_and_slender_body_theory() {
        // Short: Ribner's lifting-line π² (Wagner 2021 eq. 13, at λ → 0).
        assert!((ring_lift_slope(1e-9).unwrap() - PI * PI).abs() < 1e-7);
        // Long: slender-body theory's L = q d² π α (Hoerner 1965 p. 7-13), `π/λ` on `d L`, which
        // the arctan approaches as `1/(1.2 λ)`.
        for l in [1e3, 1e5] {
            let ratio = ring_lift_slope(l).unwrap() / (PI / l);
            assert!((ratio - 1.0).abs() < 2.0 / l, "λ = {l}: {ratio}");
        }
    }

    #[test]
    fn fletcher_s_centre_is_interpolated_to_the_leading_edge_and_held_past_a_3() {
        for (a, x) in &FLETCHER_AERODYNAMIC_CENTRE[1..] {
            assert!((ring_centre_fraction(*a).unwrap() - x).abs() < 1e-15);
        }
        // Halfway between A = 1 and 1.5.
        assert!((ring_centre_fraction(1.25).unwrap() - 0.228).abs() < 1e-15);
        // Below A = 2/3, toward the leading edge: at Fletcher's thick A = 1/3 ring, half his
        // A = 2/3 fraction, aft of the thick ring's measured -0.11.
        let third = ring_centre_fraction(1.0 / 3.0).unwrap();
        assert!((third - 0.0715).abs() < 1e-15, "{third}");
        assert!(third > FLETCHER_AERODYNAMIC_CENTRE[0].1);
        assert!(ring_centre_fraction(1e-12).unwrap().abs() < 1e-12);
        assert_eq!(ring_centre_fraction(3.0).unwrap(), 0.355);
        assert_eq!(ring_centre_fraction(10.0).unwrap(), 0.355);
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(matches!(
                ring_centre_fraction(bad),
                Err(AeroError::Domain {
                    what: "tube diameter over length",
                    ..
                })
            ));
        }
        assert!(matches!(
            ring_lift_slope(0.0),
            Err(AeroError::Domain {
                what: "tube length over diameter",
                ..
            })
        ));
    }
}
