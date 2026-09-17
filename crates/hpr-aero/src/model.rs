//! A rocket's normal force and centre of pressure: every component's terms, built once from a
//! [`Layout`] and summed at each flow condition.
//!
//! The coefficient at an angle of attack `α` is `C_N = C_Nα(α) α`, with the slope defined as
//! `C_N/α` (Niskanen 2009 eq. 3.8) and the centre of pressure as the moment sum
//! `X = Σ C_Nα,i X_i / Σ C_Nα,i` (Barrowman 1966 p. 38; Niskanen eq. 3.29). The terms:
//!
//! - bodies of revolution, `(2/A_ref)ΔA · sin α/α` at `X_B`, plus body lift
//!   `K (A_plan/A_ref) sin² α / α` at the planform centroid ([`crate::body`]);
//! - fin sets, `(C_Nα)₁ Σ sin² Λ_k · f_N · K_T(B)` at the quarter mean aerodynamic chord
//!   ([`crate::fins`]).
//!
//! Launch lugs and rail buttons add drag only, and internal parts sit inside the body. Tube fins
//! have no cited normal-force method yet and are refused. Stations are metres aft of the nose tip.

use std::f64::consts::PI;

use hpr_design::{Layout, Part, PlacedComponent, Profile, Wall, revolve};
use serde::{Deserialize, Serialize};

use crate::body::{BODY_LIFT_K, BodyGeometry, sinc};
use crate::error::{AeroError, check_dimension, check_mach};
use crate::fins::{FinGeometry, fin_count_factor, interference_factor, roll_sum};

/// The air-relative flow at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    /// Mach number, in `[0, 1)` for the subsonic models.
    pub mach: f64,
    /// Total angle of attack between the body axis `+z_B` and the air-relative velocity, rad, in
    /// `[0, π]`. The models are small-angle models (see `docs/physics/aero.md`).
    pub alpha_rad: f64,
    /// Roll angle of the lateral airflow, rad from `x_B` toward `y_B`: the direction in which the
    /// air crosses the body. Only fin sets of one or two fins depend on it.
    pub roll_rad: f64,
}

impl Flow {
    /// Straight into the wind at `mach`.
    pub fn axial(mach: f64) -> Self {
        Self {
            mach,
            alpha_rad: 0.0,
            roll_rad: 0.0,
        }
    }

    /// Checks the Mach number and the angles.
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] outside `[0, 1)`, and [`AeroError::Domain`] for an angle of attack
    /// outside `[0, π]` or a non-finite roll.
    pub fn validate(&self) -> Result<(), AeroError> {
        check_mach(self.mach)?;
        if !(0.0..=PI).contains(&self.alpha_rad) {
            return Err(AeroError::Domain {
                what: "angle of attack",
                value: self.alpha_rad,
            });
        }
        if !self.roll_rad.is_finite() {
            return Err(AeroError::Domain {
                what: "flow roll angle",
                value: self.roll_rad,
            });
        }
        Ok(())
    }
}

/// The normal force of a whole rocket, or of one component, at a flow condition.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormalForce {
    /// Normal-force coefficient `C_N` on the reference area, in the plane of the flow.
    pub coefficient: f64,
    /// `C_N/α` per radian; at `α = 0`, the slope `∂C_N/∂α`.
    pub slope_per_rad: f64,
    /// Centre of pressure, m aft of the nose tip; `None` when the slope is zero.
    pub cp_station_m: Option<f64>,
}

impl NormalForce {
    fn new(slope: f64, moment: f64, alpha_rad: f64) -> Self {
        Self {
            coefficient: slope * alpha_rad,
            slope_per_rad: slope,
            cp_station_m: (slope != 0.0).then(|| moment / slope),
        }
    }
}

/// One component's share of the normal force.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentNormalForce {
    /// The component's id.
    pub id: String,
    /// Its normal force.
    pub normal_force: NormalForce,
}

/// A body component's precomputed terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyAero {
    /// The component's id.
    pub id: String,
    /// Station of its fore end, m.
    pub fore_station_m: f64,
    /// Its geometry.
    pub geometry: BodyGeometry,
    /// Potential-flow slope at `α → 0`, per radian.
    pub slope_per_rad: f64,
    /// Potential-flow moment slope about the nose tip, m per radian.
    pub moment_slope_m: f64,
    /// Body lift `K A_plan / A_ref`: `C_N = lift_factor · sin² α`.
    pub lift_factor: f64,
    /// Station of the body lift, m.
    pub lift_station_m: f64,
}

/// A fin set's precomputed terms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinSetAero {
    /// The component's id.
    pub id: String,
    /// Number of fins.
    pub count: u32,
    /// Roll angle of the first fin, rad.
    pub base_angle_rad: f64,
    /// One fin's geometry.
    pub geometry: FinGeometry,
    /// Fin–fin factor `f_N`.
    pub count_factor: f64,
    /// Fin–body interference `K_T(B)`.
    pub interference: f64,
    /// Centre of pressure, m aft of the nose tip.
    pub cp_station_m: f64,
}

impl FinSetAero {
    /// The set's slope per radian at `mach` and `roll_rad`.
    fn slope(&self, reference_area_m2: f64, mach: f64, roll_rad: f64) -> Result<f64, AeroError> {
        Ok(self.geometry.single_fin_slope(reference_area_m2, mach)?
            * roll_sum(self.count, self.base_angle_rad, roll_rad)
            * self.count_factor
            * self.interference)
    }
}

/// A rocket's normal-force model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AeroModel {
    reference_area_m2: f64,
    bodies: Vec<BodyAero>,
    fin_sets: Vec<FinSetAero>,
}

impl AeroModel {
    /// Builds the terms of every component of `layout`.
    ///
    /// # Errors
    ///
    /// [`AeroError::InComponent`] naming the component, around:
    /// - [`AeroError::Unsupported`] for tube fins;
    /// - [`AeroError::Domain`] for a fin set of more than eight fins or without a body radius;
    /// - design errors from a profile, a planform or a volume integral.
    ///
    /// [`AeroError::Domain`] for a non-positive reference diameter.
    pub fn new(layout: &Layout) -> Result<Self, AeroError> {
        check_dimension("reference diameter", layout.reference_diameter_m, false)?;
        let reference_area_m2 = layout.reference_area_m2();
        let mut bodies = Vec::new();
        let mut fin_sets = Vec::new();
        for component in &layout.components {
            let in_component = |e: AeroError| AeroError::InComponent {
                id: component.id.clone(),
                source: Box::new(e),
            };
            match &component.part {
                Part::NoseCone(nose) => {
                    let geometry = nose
                        .profile()
                        .map_err(AeroError::from)
                        .and_then(|p| revolved(&p))
                        .map_err(in_component)?;
                    bodies.push(body_terms(component, geometry, reference_area_m2));
                }
                Part::Transition(transition) => {
                    let geometry = transition
                        .profile()
                        .map_err(AeroError::from)
                        .and_then(|p| revolved(&p))
                        .map_err(in_component)?;
                    bodies.push(body_terms(component, geometry, reference_area_m2));
                }
                Part::BodyTube(tube) => {
                    let geometry = BodyGeometry::cylinder(tube.length_m, tube.outer_radius_m)
                        .map_err(in_component)?;
                    bodies.push(body_terms(component, geometry, reference_area_m2));
                }
                Part::FinSet(set) => {
                    let terms = (|| {
                        let geometry = FinGeometry::from_planform(&set.planform)?;
                        let body_radius = component.body_radius_m.ok_or(AeroError::Domain {
                            what: "body radius at the fins",
                            value: f64::NAN,
                        })?;
                        Ok(FinSetAero {
                            id: component.id.clone(),
                            count: set.count,
                            base_angle_rad: set.base_angle_rad,
                            geometry,
                            count_factor: fin_count_factor(set.count)?,
                            interference: interference_factor(geometry.span_m, body_radius)?,
                            cp_station_m: component.fore_station_m
                                + geometry.centre_of_pressure_m(),
                        })
                    })()
                    .map_err(in_component)?;
                    fin_sets.push(terms);
                }
                Part::TubeFinSet(_) => {
                    return Err(in_component(AeroError::Unsupported(
                        "tube fins (no cited normal-force method yet)".to_owned(),
                    )));
                }
                // Lugs and rail buttons: drag only. Everything else is inside the body.
                _ => {}
            }
        }
        Ok(Self {
            reference_area_m2,
            bodies,
            fin_sets,
        })
    }

    /// Reference area, m².
    pub fn reference_area_m2(&self) -> f64 {
        self.reference_area_m2
    }

    /// The body components' terms.
    pub fn bodies(&self) -> &[BodyAero] {
        &self.bodies
    }

    /// The fin sets' terms.
    pub fn fin_sets(&self) -> &[FinSetAero] {
        &self.fin_sets
    }

    /// The whole rocket's normal force at `flow`.
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`].
    pub fn normal_force(&self, flow: &Flow) -> Result<NormalForce, AeroError> {
        flow.validate()?;
        let (potential, lift) = alpha_factors(flow.alpha_rad);
        let (mut slope, mut moment) = (0.0, 0.0);
        for body in &self.bodies {
            slope += body.slope_per_rad * potential + body.lift_factor * lift;
            moment +=
                body.moment_slope_m * potential + body.lift_factor * lift * body.lift_station_m;
        }
        for set in &self.fin_sets {
            let s = set.slope(self.reference_area_m2, flow.mach, flow.roll_rad)?;
            slope += s;
            moment += s * set.cp_station_m;
        }
        Ok(NormalForce::new(slope, moment, flow.alpha_rad))
    }

    /// Each component's normal force at `flow`, bodies first, then fin sets, in layout order.
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`].
    pub fn components(&self, flow: &Flow) -> Result<Vec<ComponentNormalForce>, AeroError> {
        flow.validate()?;
        let (potential, lift) = alpha_factors(flow.alpha_rad);
        let mut out = Vec::with_capacity(self.bodies.len() + self.fin_sets.len());
        for body in &self.bodies {
            let slope = body.slope_per_rad * potential + body.lift_factor * lift;
            let moment =
                body.moment_slope_m * potential + body.lift_factor * lift * body.lift_station_m;
            out.push(ComponentNormalForce {
                id: body.id.clone(),
                normal_force: NormalForce::new(slope, moment, flow.alpha_rad),
            });
        }
        for set in &self.fin_sets {
            let slope = set.slope(self.reference_area_m2, flow.mach, flow.roll_rad)?;
            out.push(ComponentNormalForce {
                id: set.id.clone(),
                normal_force: NormalForce::new(slope, slope * set.cp_station_m, flow.alpha_rad),
            });
        }
        Ok(out)
    }
}

/// The per-radian factors of the potential-flow term (`sin α/α`) and of body lift
/// (`sin² α/α = sin α · sin α/α`).
fn alpha_factors(alpha_rad: f64) -> (f64, f64) {
    let s = sinc(alpha_rad);
    (s, alpha_rad.sin() * s)
}

/// A nose cone's or transition's geometry from its outer profile.
fn revolved(profile: &Profile) -> Result<BodyGeometry, AeroError> {
    let g = revolve(profile, Wall::Filled {})?;
    let area = |r: f64| PI * r * r;
    let geometry = BodyGeometry {
        length_m: profile.length_m(),
        fore_area_m2: area(profile.fore_radius_m()),
        aft_area_m2: area(profile.aft_radius_m()),
        volume_m3: g.volume_m3,
        planform_area_m2: g.planform_area_m2,
        planform_centroid_m: g.planform_centroid_m,
    };
    geometry.validate()?;
    Ok(geometry)
}

fn body_terms(component: &PlacedComponent, geometry: BodyGeometry, a_ref: f64) -> BodyAero {
    let slope = geometry.normal_force_slope(a_ref);
    BodyAero {
        id: component.id.clone(),
        fore_station_m: component.fore_station_m,
        geometry,
        slope_per_rad: slope,
        moment_slope_m: slope * component.fore_station_m + geometry.moment_slope_m(a_ref),
        lift_factor: BODY_LIFT_K * geometry.planform_area_m2 / a_ref,
        lift_station_m: component.fore_station_m + geometry.planform_centroid_m,
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_PI_2;

    use hpr_design::{
        FinPlanform, LaunchLug, NoseShape, Part, Position, ReferenceDiameter, TubeFinSet,
    };
    use proptest::prelude::*;

    use super::*;
    use crate::testing::{body_part, component, fin_set, finned_rocket, material, nose, one_stage};

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = if want == 0.0 {
            got.abs()
        } else {
            ((got - want) / want).abs()
        };
        assert!(
            err <= rel,
            "{what}: got {got}, want {want}, rel err {err:e}"
        );
    }

    fn model(rocket: &hpr_design::Rocket) -> AeroModel {
        AeroModel::new(&rocket.layout().unwrap()).unwrap()
    }

    fn flow(mach: f64, alpha_rad: f64, roll_rad: f64) -> Flow {
        Flow {
            mach,
            alpha_rad,
            roll_rad,
        }
    }

    /// A cone on a cylinder, broadside and at small angles: the potential term scales with
    /// `sin α`, body lift with `sin² α` at the planform centroids (a cone's `½ L D` at `2L/3`, a
    /// cylinder's `L D` at its middle; Galejs Table 1), and the slope at `α → 0` is the sum of the
    /// Barrowman slopes.
    #[test]
    fn angle_of_attack_terms() {
        let (l_n, l_t, r) = (0.2, 0.8, 0.03);
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, l_n, r), None),
                component("tube", body_part(l_t, r, r), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let a_ref = PI * r * r;
        let lift_nose = BODY_LIFT_K * r * l_n / a_ref;
        let lift_tube = BODY_LIFT_K * 2.0 * r * l_t / a_ref;

        let broadside = m.normal_force(&flow(0.3, FRAC_PI_2, 0.0)).unwrap();
        let want = 2.0 + lift_nose + lift_tube;
        close(broadside.coefficient, want, 1e-10, "C_N at 90°");
        let moment =
            2.0 * (2.0 * l_n / 3.0) + lift_nose * (2.0 * l_n / 3.0) + lift_tube * (l_n + 0.5 * l_t);
        close(
            broadside.cp_station_m.unwrap(),
            moment / want,
            1e-10,
            "CP at 90°",
        );

        let zero = m.normal_force(&Flow::axial(0.3)).unwrap();
        assert_eq!(zero.coefficient, 0.0);
        close(zero.slope_per_rad, 2.0, 1e-15, "slope at 0");
        // C_N(α)/α tends to the slope, with body lift adding (K A_plan/A_ref) α.
        for alpha in [1e-6, 1e-3, 0.05] {
            let f = m.normal_force(&flow(0.3, alpha, 0.0)).unwrap();
            let want = 2.0 * alpha.sin() + (lift_nose + lift_tube) * alpha.sin().powi(2);
            close(f.coefficient, want, 1e-12, "C_N");
            close(f.slope_per_rad, want / alpha, 1e-12, "C_N/α");
        }
        // Body lift pulls the CP aft as α grows.
        let cp = |alpha| {
            m.normal_force(&flow(0.3, alpha, 0.0))
                .unwrap()
                .cp_station_m
                .unwrap()
        };
        assert!(cp(0.02) > cp(0.0) && cp(0.2) > cp(0.02));
        // The components add up.
        let parts = m.components(&flow(0.3, 0.2, 0.0)).unwrap();
        let total = m.normal_force(&flow(0.3, 0.2, 0.0)).unwrap();
        let sum: f64 = parts.iter().map(|c| c.normal_force.coefficient).sum();
        close(sum, total.coefficient, 1e-14, "component sum");
    }

    /// Mach changes the fins' slope by Prandtl–Glauert and nothing else; the bodies and every CP
    /// stay put.
    #[test]
    fn mach_changes_only_the_fins() {
        let m = model(&finned_rocket(4));
        let at = |mach| m.components(&Flow::axial(mach)).unwrap();
        let (slow, fast) = (at(0.0), at(0.9));
        for (a, b) in slow.iter().zip(&fast) {
            assert_eq!(
                a.normal_force.cp_station_m, b.normal_force.cp_station_m,
                "{}",
                a.id
            );
            if a.id == "fins" {
                let set = &m.fin_sets()[0];
                let ratio = set
                    .geometry
                    .single_fin_slope(m.reference_area_m2(), 0.9)
                    .unwrap()
                    / set
                        .geometry
                        .single_fin_slope(m.reference_area_m2(), 0.0)
                        .unwrap();
                assert!(ratio > 1.1, "{ratio}");
                close(
                    b.normal_force.slope_per_rad / a.normal_force.slope_per_rad,
                    ratio,
                    1e-14,
                    "fin ratio",
                );
            } else {
                assert_eq!(
                    a.normal_force.slope_per_rad, b.normal_force.slope_per_rad,
                    "{}",
                    a.id
                );
            }
        }
    }

    /// Four fins don't care about roll; two fins lift only when the flow crosses them.
    #[test]
    fn two_fin_sets_depend_on_roll() {
        let four = model(&finned_rocket(4));
        let slope =
            |m: &AeroModel, roll| m.normal_force(&flow(0.2, 0.0, roll)).unwrap().slope_per_rad;
        close(slope(&four, 0.0), slope(&four, 0.4), 1e-15, "four fins");

        let two = model(&finned_rocket(2));
        let bodies: f64 = two.bodies().iter().map(|b| b.slope_per_rad).sum();
        let set = &two.fin_sets()[0];
        let one_fin = set
            .geometry
            .single_fin_slope(two.reference_area_m2(), 0.2)
            .unwrap()
            * set.interference;
        // Flow along the fins' plane: no fin force. Across it: both fins at sin² = 1.
        close(slope(&two, 0.0), bodies, 1e-13, "along the fins");
        close(
            slope(&two, FRAC_PI_2),
            bodies + 2.0 * one_fin,
            1e-13,
            "across the fins",
        );
        // A two-fin set across the flow matches four fins' N/2 = 2.
        close(
            slope(&two, FRAC_PI_2),
            slope(&four, 0.0),
            1e-13,
            "two across = four",
        );
    }

    /// Refusals: tube fins, nine fins, Mach 1, angles outside `[0, π]`. Lugs add no normal force.
    #[test]
    fn unsupported_inputs_are_refused() {
        let mut rocket = finned_rocket(4);
        rocket.stages[0].components[3].children.push(component(
            "tube-fins",
            Part::TubeFinSet(TubeFinSet {
                count: 6,
                length_m: 0.1,
                outer_radius_m: 0.01,
                thickness_m: 0.001,
                base_angle_rad: 0.0,
                material: material(),
            }),
            Some(Position::Bottom { aft_offset_m: 0.0 }),
        ));
        let err = AeroModel::new(&rocket.layout().unwrap()).unwrap_err();
        assert!(
            matches!(&err, AeroError::InComponent { id, source } if id == "tube-fins"
                && matches!(**source, AeroError::Unsupported(_))),
            "{err}"
        );

        let err = AeroModel::new(&finned_rocket(9).layout().unwrap()).unwrap_err();
        assert!(
            matches!(&err, AeroError::InComponent { id, .. } if id == "fins"),
            "{err}"
        );

        let m = model(&finned_rocket(4));
        for bad in [
            flow(1.0, 0.0, 0.0),
            flow(-0.01, 0.0, 0.0),
            flow(f64::NAN, 0.0, 0.0),
        ] {
            assert!(matches!(m.normal_force(&bad), Err(AeroError::Mach { .. })));
        }
        for bad in [
            flow(0.3, -1e-9, 0.0),
            flow(0.3, PI + 1e-9, 0.0),
            flow(0.3, f64::NAN, 0.0),
            flow(0.3, 0.1, f64::INFINITY),
        ] {
            assert!(matches!(
                m.normal_force(&bad),
                Err(AeroError::Domain { .. })
            ));
            assert!(m.components(&bad).is_err());
        }

        let mut lugged = finned_rocket(4);
        lugged.stages[0].components[1].children.push(component(
            "lug",
            Part::LaunchLug(LaunchLug {
                length_m: 0.05,
                outer_radius_m: 0.004,
                thickness_m: 0.0005,
                angle_rad: 0.0,
                count: 1,
                spacing_m: 0.0,
                material: material(),
            }),
            Some(Position::Middle { aft_offset_m: 0.0 }),
        ));
        let with = model(&lugged).normal_force(&flow(0.3, 0.1, 0.0)).unwrap();
        assert_eq!(with, m.normal_force(&flow(0.3, 0.1, 0.0)).unwrap());
    }

    /// A bare tube has no normal force, and so no CP, at `α = 0`; body lift gives it one at its
    /// middle at any other angle.
    #[test]
    fn a_bare_tube_has_no_cp_at_zero_incidence() {
        let tube = one_stage(
            vec![component("tube", body_part(1.0, 0.05, 0.05), None)],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&tube);
        assert_eq!(
            m.normal_force(&Flow::axial(0.5)).unwrap().cp_station_m,
            None
        );
        let f = m.normal_force(&flow(0.5, 0.1, 0.0)).unwrap();
        close(f.cp_station_m.unwrap(), 0.5, 1e-15, "tube lift CP");
    }

    proptest! {
        /// Barrowman's coefficients are dimensionless: scaling every length by `k` leaves the slope
        /// alone and scales the CP by `k`. A custom reference diameter scales the slope by
        /// `(d/d′)²` and leaves the CP alone.
        #[test]
        fn scaling_leaves_slopes_and_scales_the_cp(
            k in 0.1f64..10.0,
            nose_fineness in 1.5f64..8.0,
            radius in 0.01f64..0.1,
            root in 0.02f64..0.3,
            tip_ratio in 0.0f64..1.0,
            span in 0.01f64..0.3,
            sweep in -0.1f64..0.3,
            count in 1u32..=8,
            alpha in 0.0f64..0.5,
            reference in 0.5f64..2.0,
        ) {
            let build = |s: f64, custom: Option<f64>| {
                let (r, l) = (s * radius, s * (root + 0.5));
                let mut tube = component("tube", body_part(l, r, r), None);
                let planform = FinPlanform::Trapezoidal {
                    root_chord_m: s * root,
                    tip_chord_m: s * root * tip_ratio,
                    span_m: s * span,
                    sweep_m: s * sweep,
                };
                tube.children = vec![component(
                    "fins",
                    fin_set(count, planform),
                    Some(Position::Bottom { aft_offset_m: 0.0 }),
                )];
                let ogive = NoseShape::Ogive { radius_ratio: 1.0 };
                let nose = component("nose", nose(ogive, 2.0 * nose_fineness * r, r), None);
                let mut rocket = one_stage(vec![nose, tube], ReferenceDiameter::Maximum {});
                if let Some(d) = custom {
                    rocket.reference_diameter = ReferenceDiameter::Custom { diameter_m: d };
                }
                model(&rocket).normal_force(&flow(0.4, alpha, 0.3)).unwrap()
            };
            let rel = |a: f64, b: f64| (a / b - 1.0).abs();
            let base = build(1.0, None);
            let base_cp = base.cp_station_m.unwrap();
            let scaled = build(k, None);
            prop_assert!(rel(scaled.slope_per_rad, base.slope_per_rad) < 1e-9);
            prop_assert!(rel(scaled.cp_station_m.unwrap(), k * base_cp) < 1e-9);
            let d = 2.0 * radius * reference;
            let custom = build(1.0, Some(d));
            let factor = (2.0 * radius / d).powi(2);
            prop_assert!(rel(custom.slope_per_rad, factor * base.slope_per_rad) < 1e-12);
            prop_assert!(rel(custom.cp_station_m.unwrap(), base_cp) < 1e-12);
        }
    }
}
