//! A rocket's normal force and centre of pressure: every component's terms, built once from a
//! [`Layout`] and summed at each flow condition.
//!
//! The coefficient at an angle of attack `α` is `C_N = C_Nα(α) α`, with the slope defined as
//! `C_N/α` (Niskanen 2009 eq. 3.8) and the centre of pressure as the moment sum
//! `X = Σ C_Nα,i X_i / Σ C_Nα,i` (Barrowman 1966 p. 38; Niskanen eq. 3.29). The terms:
//!
//! - bodies of revolution, `(2/A_ref)ΔA · sin α/α` at `X_B`, plus body lift
//!   `K (A_plan/A_ref) sin² α / α` at the planform centroid ([`crate::body`]);
//! - a step in radius where one body component meets the next, `(2/A_ref)ΔA · sin α/α` at the
//!   joint, reported with the aft component. This extrapolates Barrowman 1966 eq. 10 over the whole
//!   body to a transition of zero length; Barrowman 1967 p. 18 assumes no discontinuities;
//! - fin sets, `(C_Nα)₁ Σ sin² Λ_k · f_N · K_T(B)` at the fin's centre of pressure, both at the
//!   flow's Mach number ([`crate::fins::FinAero`]), and for one or two fins the side force
//!   `(C_Nα)₁ Σ sin Λ cos Λ · K_T(B)` across the flow's plane ([`crate::fins::side_sum`]).
//!
//! The bodies' terms don't change with Mach: slender-body theory's slope and centre of pressure
//! hold at any Mach number (Barrowman 1967 p. 18), and body lift is Galejs's cross-flow term.
//!
//! Launch lugs and rail buttons add drag only, and internal parts sit inside the body. Tube fins
//! have no cited normal-force method yet and are refused, as is any part kind this model doesn't
//! know. Stations are metres aft of the nose tip.

use std::f64::consts::PI;

use hpr_design::{Layout, Part, PlacedComponent};
use serde::{Deserialize, Serialize};

use crate::body::{BODY_LIFT_K, BodyGeometry, sinc};
use crate::drag::{
    BUILDUP_MACH_LIMIT, ComponentDrag, ComponentDragTerms, Drag, DragConditions,
    SUBSONIC_MACH_LIMIT, axial_drag_alpha_factor, body_friction_form_factor,
};
use crate::error::{AeroError, check_dimension, check_mach};
use crate::fins::{FinAero, FinLoading, fin_count_factor, interference_factor, roll_sum, side_sum};

/// The top of the normal force's range: Mach 5, where the hypersonic region begins (Niskanen 2009
/// Table 3.1, p. 19). [`AeroModel::normal_force`] refuses it and anything faster.
pub const NORMAL_FORCE_MACH_LIMIT: f64 = 5.0;
use crate::table::DragTable;

/// The air-relative flow at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Flow {
    /// Mach number: in `[0, 5)` for the normal force, and in `[0, 1)` for the drag buildup until
    /// its transonic terms arrive.
    pub mach: f64,
    /// Total angle of attack between the body axis `+z_B` and the air-relative velocity, rad, in
    /// `[0, π]`. The models are small-angle models (see `docs/physics/aero.md`).
    pub alpha_rad: f64,
    /// Roll angle of the lateral airflow, rad from `x_B` toward `y_B`: the direction in which the
    /// air crosses the body. Only fin sets of one or two fins depend on it.
    pub roll_rad: f64,
}

impl Flow {
    /// A flow at `mach`, angle of attack `alpha_rad` and lateral-flow roll `roll_rad`. Checked
    /// when used ([`Flow::validate`]).
    pub fn new(mach: f64, alpha_rad: f64, roll_rad: f64) -> Self {
        Self {
            mach,
            alpha_rad,
            roll_rad,
        }
    }

    /// Straight into the wind at `mach`.
    pub fn axial(mach: f64) -> Self {
        Self::new(mach, 0.0, 0.0)
    }

    /// Checks the Mach number against the normal force's range, and the angles.
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] outside `[0, 5)` ([`NORMAL_FORCE_MACH_LIMIT`]), and
    /// [`AeroError::Domain`] for an angle of attack outside `[0, π]` or a non-finite roll.
    pub fn validate(&self) -> Result<(), AeroError> {
        check_mach(self.mach, NORMAL_FORCE_MACH_LIMIT)?;
        self.validate_angles()
    }

    /// As [`Flow::validate`], for the drag buildup's range `[0, 1)`.
    fn validate_for_buildup(&self) -> Result<(), AeroError> {
        check_mach(self.mach, BUILDUP_MACH_LIMIT)?;
        self.validate_angles()
    }

    /// Checks the angles only.
    fn validate_angles(&self) -> Result<(), AeroError> {
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
#[non_exhaustive]
pub struct NormalForce {
    /// Normal-force coefficient `C_N` on the reference area, in the plane of the flow.
    pub coefficient: f64,
    /// `C_N/α` per radian; at `α = 0`, the slope `∂C_N/∂α`.
    pub slope_per_rad: f64,
    /// `Σ C_N,i X_i`, m: the normal force's moment about the nose tip per unit dynamic pressure and
    /// reference area, defined even when the net force is zero.
    pub moment_m: f64,
    /// Centre of pressure, m aft of the nose tip; `None` when the slope is zero, or so small
    /// against its terms (below 1e-12 of `Σ |C_Nα,i|`) that the ratio would be noise.
    pub cp_station_m: Option<f64>,
    /// Side-force coefficient across the plane of the flow, along `z_B` × the lateral-flow
    /// direction. Only fin sets of one or two fins produce it ([`crate::fins::side_sum`]).
    pub side_coefficient: f64,
    /// `Σ C_Y,i X_i`, m: the side force's moment about the nose tip per unit dynamic pressure and
    /// reference area.
    pub side_moment_m: f64,
}

/// One component's contributions per radian: slope, moment slope about the nose tip, side slope
/// and side moment slope, and `Σ |terms|` of the slope to judge cancellation.
#[derive(Clone, Copy, Default)]
struct Term {
    slope: f64,
    moment: f64,
    side: f64,
    side_moment: f64,
    scale: f64,
}

impl Term {
    fn add(self, other: Term) -> Term {
        Term {
            slope: self.slope + other.slope,
            moment: self.moment + other.moment,
            side: self.side + other.side,
            side_moment: self.side_moment + other.side_moment,
            scale: self.scale + other.scale,
        }
    }
}

impl NormalForce {
    fn new(term: Term, alpha_rad: f64) -> Self {
        let cancelled = term.slope.abs() <= 1e-12 * term.scale;
        Self {
            coefficient: term.slope * alpha_rad,
            slope_per_rad: term.slope,
            moment_m: term.moment * alpha_rad,
            cp_station_m: (term.slope != 0.0 && !cancelled).then(|| term.moment / term.slope),
            side_coefficient: term.side * alpha_rad,
            side_moment_m: term.side_moment * alpha_rad,
        }
    }
}

/// One component's share of the normal force.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ComponentNormalForce {
    /// The component's id.
    pub id: String,
    /// Its normal force.
    pub normal_force: NormalForce,
}

/// A body component's precomputed terms.
///
/// Serialize-only, like [`AeroModel`]: the terms are computed by [`AeroModel::new`], not read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct BodyAero {
    /// The component's id.
    pub id: String,
    /// Station of its fore end, m.
    pub fore_station_m: f64,
    /// Its geometry.
    pub geometry: BodyGeometry,
    /// The step in cross-section area from the previous body component's aft end to this one's
    /// fore end, m² (zero for the first body component).
    pub step_area_m2: f64,
    /// Potential-flow slope at `α → 0`, per radian, with the step.
    pub slope_per_rad: f64,
    /// Potential-flow moment slope about the nose tip, m per radian, with the step.
    pub moment_slope_m: f64,
    /// Body lift `K A_plan / A_ref`: `C_N = lift_factor · sin² α`.
    pub lift_factor: f64,
    /// Station of the body lift, m.
    pub lift_station_m: f64,
}

/// A fin set's precomputed terms.
///
/// Serialize-only, like [`AeroModel`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct FinSetAero {
    /// The component's id.
    pub id: String,
    /// Number of fins.
    pub count: u32,
    /// Roll angle of the first fin, rad.
    pub base_angle_rad: f64,
    /// One fin's normal force through the speed regimes, and its geometry.
    pub fin: FinAero,
    /// Fin–fin factor `f_N`.
    pub count_factor: f64,
    /// Fin–body interference `K_T(B)`.
    pub interference: f64,
    /// Station of the fins' root leading edge, m aft of the nose tip.
    pub fore_station_m: f64,
}

impl FinSetAero {
    /// The set's centre of pressure at `mach`, m aft of the nose tip.
    ///
    /// # Errors
    ///
    /// As [`FinAero::loading`].
    pub fn cp_station_m(&self, mach: f64) -> Result<f64, AeroError> {
        Ok(self.fore_station_m + self.fin.loading(mach)?.cp_m)
    }
}

/// A rocket's aerodynamic model: normal force, centre of pressure and drag.
///
/// Serialize-only, for inspection: a model is built from a [`Layout`] by [`AeroModel::new`], which
/// checks what it builds.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AeroModel {
    reference_area_m2: f64,
    length_m: f64,
    bodies: Vec<BodyAero>,
    fin_sets: Vec<FinSetAero>,
    drag_terms: Vec<ComponentDragTerms>,
    drag_table: Option<DragTable>,
}

impl AeroModel {
    /// Builds the terms of every component of `layout`.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Domain`] for a non-positive reference diameter, rocket length or body radius.
    /// - [`AeroError::InComponent`] naming the component, around:
    ///   - [`AeroError::Unsupported`] for tube fins, or a part kind or fin cross-section this model
    ///     doesn't know;
    ///   - [`AeroError::Domain`] for a fin set of more than eight fins, a non-finite station, or a
    ///     drag input out of range (a negative fin thickness, a launch lug's wall thicker than its
    ///     radius, a rail button's base and flange taller than the button, a negative roughness);
    ///   - [`AeroError::Layout`] for a fin set without the radius of its body tube;
    ///   - design errors from a profile, a planform or a volume integral.
    pub fn new(layout: &Layout) -> Result<Self, AeroError> {
        check_dimension("reference diameter", layout.reference_diameter_m, false)?;
        let reference_area_m2 = layout.reference_area_m2();
        let length_m = layout.length_m;
        check_dimension("rocket length", length_m, false)?;
        let mut max_radius: f64 = 0.0;
        for component in layout.body() {
            if let Some(radius) = component.part.max_radius_m()? {
                max_radius = max_radius.max(radius);
            }
        }
        check_dimension("maximum body radius", max_radius, false)?;
        let form_factor = body_friction_form_factor(length_m / (2.0 * max_radius))?;
        let mut bodies = Vec::new();
        let mut fin_sets = Vec::new();
        let mut drag_terms = Vec::new();
        let mut previous_aft_area: Option<f64> = None;
        let mut last_body_terms: Option<usize> = None;
        for component in &layout.components {
            let in_component = |e: AeroError| AeroError::InComponent {
                id: component.id.clone(),
                source: Box::new(e),
            };
            if !component.fore_station_m.is_finite() {
                return Err(in_component(AeroError::Domain {
                    what: "component station",
                    value: component.fore_station_m,
                }));
            }
            let body = match &component.part {
                Part::NoseCone(nose) => Some(
                    nose.profile()
                        .map_err(AeroError::from)
                        .and_then(|p| BodyGeometry::from_profile(&p)),
                ),
                Part::Transition(transition) => Some(
                    transition
                        .profile()
                        .map_err(AeroError::from)
                        .and_then(|p| BodyGeometry::from_profile(&p)),
                ),
                Part::BodyTube(tube) => {
                    Some(BodyGeometry::cylinder(tube.length_m, tube.outer_radius_m))
                }
                Part::FinSet(set) => {
                    let terms = (|| {
                        let fin = FinAero::new(&set.planform, reference_area_m2)?;
                        let body_radius = component.body_radius_m.ok_or_else(|| {
                            AeroError::Layout(
                                "a fin set needs the radius of the body tube it is on".to_owned(),
                            )
                        })?;
                        Ok(FinSetAero {
                            id: component.id.clone(),
                            count: set.count,
                            base_angle_rad: set.base_angle_rad,
                            count_factor: fin_count_factor(set.count)?,
                            interference: interference_factor(fin.geometry.span_m, body_radius)?,
                            fore_station_m: component.fore_station_m,
                            fin,
                        })
                    })()
                    .map_err(in_component)?;
                    drag_terms.push(
                        ComponentDragTerms::fins(
                            component,
                            set,
                            &terms.fin.geometry,
                            length_m,
                            reference_area_m2,
                        )
                        .map_err(in_component)?,
                    );
                    fin_sets.push(terms);
                    None
                }
                Part::TubeFinSet(_) => {
                    return Err(in_component(AeroError::Unsupported(
                        "tube fins (no cited normal-force method yet)".to_owned(),
                    )));
                }
                // Drag only.
                Part::LaunchLug(lug) => {
                    drag_terms.push(
                        ComponentDragTerms::launch_lugs(
                            component,
                            lug,
                            length_m,
                            reference_area_m2,
                        )
                        .map_err(in_component)?,
                    );
                    None
                }
                Part::RailButton(button) => {
                    drag_terms.push(
                        ComponentDragTerms::rail_buttons(
                            component,
                            button,
                            length_m,
                            reference_area_m2,
                        )
                        .map_err(in_component)?,
                    );
                    None
                }
                // Inside the body.
                Part::InnerTube(_)
                | Part::CenteringRing(_)
                | Part::MassComponent(_)
                | Part::Parachute(_)
                | Part::Streamer(_)
                | Part::ShockCord(_) => None,
                other => {
                    return Err(in_component(AeroError::Unsupported(format!(
                        "a {} part",
                        other.kind_name()
                    ))));
                }
            };
            if let Some(geometry) = body {
                let geometry = geometry.map_err(in_component)?;
                let step = previous_aft_area.map_or(0.0, |aft| geometry.fore_area_m2 - aft);
                last_body_terms = Some(drag_terms.len());
                drag_terms.push(
                    ComponentDragTerms::body(
                        component,
                        &geometry,
                        previous_aft_area,
                        form_factor,
                        length_m,
                        reference_area_m2,
                    )
                    .map_err(in_component)?,
                );
                previous_aft_area = Some(geometry.aft_area_m2);
                bodies.push(body_terms(component, geometry, step, reference_area_m2));
            }
        }
        // The aft base belongs to the last body component.
        if let (Some(index), Some(last)) = (last_body_terms, bodies.last()) {
            drag_terms[index].base_area_m2 = last.geometry.aft_area_m2;
        }
        Ok(Self {
            reference_area_m2,
            length_m,
            bodies,
            fin_sets,
            drag_terms,
            drag_table: None,
        })
    }

    /// This model with `table` replacing the drag buildup's zero-lift drag
    /// ([`crate::table`]).
    #[must_use]
    pub fn with_drag_table(mut self, table: DragTable) -> Self {
        self.drag_table = Some(table);
        self
    }

    /// The drag override table, if any.
    pub fn drag_table(&self) -> Option<&DragTable> {
        self.drag_table.as_ref()
    }

    /// The components' precomputed drag terms, in layout order.
    pub fn drag_terms(&self) -> &[ComponentDragTerms] {
        &self.drag_terms
    }

    /// Rocket length for the Reynolds number: nose tip to the aft end of the last body component,
    /// m.
    pub fn length_m(&self) -> f64 {
        self.length_m
    }

    /// The whole rocket's drag at `flow` and `conditions`: the zero-lift drag of the buildup, or of
    /// the override table when there is one, and the axial coefficient at the flow's angle of
    /// attack.
    ///
    /// # Errors
    ///
    /// - [`AeroError::Mach`] outside `[0, 1)` for the buildup, whose transonic terms arrive in
    ///   [M1.8][m1-8]; with an override table any finite Mach number from 0 is accepted
    ///   ([`AeroError::Domain`] otherwise).
    /// - [`AeroError::Domain`] for an angle of attack outside `[0, π]` or a non-finite roll.
    /// - As [`DragConditions::validate`].
    /// - [`AeroError::Table`] from the table lookup, and [`AeroError::Domain`] if the drag isn't
    ///   finite.
    ///
    /// [m1-8]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8
    pub fn drag(&self, flow: &Flow, conditions: &DragConditions) -> Result<Drag, AeroError> {
        conditions.validate()?;
        let factor = axial_drag_alpha_factor(flow.alpha_rad)?;
        let mut drag = if let Some(table) = &self.drag_table {
            flow.validate_angles()?;
            let lookup = table.lookup(flow.mach, conditions.thrusting)?;
            let scale = match table.reference_diameter_m {
                Some(d) => {
                    check_dimension("drag table reference diameter", d, false)?;
                    0.25 * PI * d * d / self.reference_area_m2
                }
                None => 1.0,
            };
            Drag {
                zero_lift_coefficient: lookup.value * scale,
                table: Some(lookup),
                ..Drag::default()
            }
        } else {
            flow.validate_for_buildup()?;
            let reynolds = conditions.reynolds_per_m * self.length_m;
            let mut sum = Drag::default();
            for terms in &self.drag_terms {
                let d = terms.evaluate(
                    reynolds,
                    flow.mach,
                    conditions.thrusting_motor_area_m2,
                    self.reference_area_m2,
                )?;
                sum.friction += d.friction;
                sum.pressure += d.pressure;
                sum.base += d.base;
                sum.parasitic += d.parasitic;
            }
            sum.zero_lift_coefficient = sum.friction + sum.pressure + sum.base + sum.parasitic;
            sum.beyond_subsonic_methods = flow.mach > SUBSONIC_MACH_LIMIT;
            sum
        };
        drag.axial_coefficient = drag.zero_lift_coefficient * factor;
        if !(drag.zero_lift_coefficient.is_finite() && drag.axial_coefficient.is_finite()) {
            return Err(AeroError::Domain {
                what: "drag coefficient",
                value: drag.zero_lift_coefficient,
            });
        }
        Ok(drag)
    }

    /// Each component's share of the drag buildup at `flow` and `conditions`, in layout order: its
    /// zero-lift coefficient and parts, and its axial coefficient at the flow's angle of attack.
    ///
    /// These are always the buildup's terms. With an override table, [`AeroModel::drag`] returns
    /// the table's value instead of their sum, so they don't add up to it.
    ///
    /// # Errors
    ///
    /// As [`AeroModel::drag`] without a table, and [`DragConditions::validate`].
    pub fn buildup_components(
        &self,
        flow: &Flow,
        conditions: &DragConditions,
    ) -> Result<Vec<ComponentDrag>, AeroError> {
        flow.validate_for_buildup()?;
        conditions.validate()?;
        let factor = axial_drag_alpha_factor(flow.alpha_rad)?;
        let reynolds = conditions.reynolds_per_m * self.length_m;
        self.drag_terms
            .iter()
            .map(|terms| {
                let mut drag = terms.evaluate(
                    reynolds,
                    flow.mach,
                    conditions.thrusting_motor_area_m2,
                    self.reference_area_m2,
                )?;
                drag.axial_coefficient *= factor;
                Ok(ComponentDrag {
                    id: terms.id.clone(),
                    drag,
                })
            })
            .collect()
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

    /// Each component's id and contributions at a validated `flow`: bodies first, then fin sets,
    /// in layout order.
    fn terms<'a>(&'a self, flow: &Flow) -> impl Iterator<Item = (&'a str, Term)> + 'a {
        let (potential, lift) = alpha_factors(flow.alpha_rad);
        let (mach, roll) = (flow.mach, flow.roll_rad);
        let bodies = self
            .bodies
            .iter()
            .map(move |body| (body.id.as_str(), body_term(body, potential, lift)));
        let fins = self
            .fin_sets
            .iter()
            .map(move |set| (set.id.as_str(), fin_term(set, mach, roll)));
        bodies.chain(fins)
    }

    /// The number of components with a normal-force term: the bodies, then the fin sets, in the
    /// order of [`Self::components`].
    pub fn component_count(&self) -> usize {
        self.bodies.len() + self.fin_sets.len()
    }

    /// Component `index`'s normal force at `flow`, in the order of [`Self::components`], without
    /// allocating. A flight engine evaluates each component at its own local flow, which includes
    /// the airspeed the body's rotation adds at the component.
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`], and [`AeroError::Domain`] for an index past
    /// [`Self::component_count`].
    pub fn component_normal_force(
        &self,
        index: usize,
        flow: &Flow,
    ) -> Result<NormalForce, AeroError> {
        flow.validate()?;
        let term = if let Some(body) = self.bodies.get(index) {
            let (potential, lift) = alpha_factors(flow.alpha_rad);
            body_term(body, potential, lift)
        } else if let Some(set) = self.fin_sets.get(index - self.bodies.len()) {
            fin_term(set, flow.mach, flow.roll_rad)
        } else {
            return Err(AeroError::Domain {
                what: "component index",
                value: index as f64,
            });
        };
        Ok(NormalForce::new(term, flow.alpha_rad))
    }

    /// The station, m aft of the nose tip, of component `index`'s small-angle centre of
    /// pressure at `mach`: where a flight engine takes the component's local airspeed. A body with
    /// no potential-flow slope (a cylinder) uses its body-lift station; a fin set's moves with
    /// Mach. `None` for an index past [`Self::component_count`], or a Mach number outside
    /// `[0, 5)` for a fin set.
    pub fn component_station_m(&self, index: usize, mach: f64) -> Option<f64> {
        if let Some(body) = self.bodies.get(index) {
            // As `NormalForce`'s CP: a slope that cancels to rounding (a step in radius offsetting
            // a taper) has no potential-flow station.
            let step_slope = 2.0 * body.step_area_m2 / self.reference_area_m2;
            let scale = (body.slope_per_rad - step_slope).abs() + step_slope.abs();
            Some(if body.slope_per_rad.abs() <= 1e-12 * scale {
                body.lift_station_m
            } else {
                body.moment_slope_m / body.slope_per_rad
            })
        } else {
            self.fin_sets
                .get(index - self.bodies.len())
                .and_then(|set| set.cp_station_m(mach).ok())
        }
    }

    /// The whole rocket's normal force at `flow`.
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`].
    pub fn normal_force(&self, flow: &Flow) -> Result<NormalForce, AeroError> {
        flow.validate()?;
        let total = self
            .terms(flow)
            .fold(Term::default(), |sum, (_, term)| sum.add(term));
        Ok(NormalForce::new(total, flow.alpha_rad))
    }

    /// Each component's normal force at `flow`, bodies first, then fin sets, in layout order. A
    /// step in radius is part of the component aft of it.
    ///
    /// # Errors
    ///
    /// As [`Flow::validate`].
    pub fn components(&self, flow: &Flow) -> Result<Vec<ComponentNormalForce>, AeroError> {
        flow.validate()?;
        Ok(self
            .terms(flow)
            .map(|(id, term)| ComponentNormalForce {
                id: id.to_owned(),
                normal_force: NormalForce::new(term, flow.alpha_rad),
            })
            .collect())
    }
}

/// A fin set's contribution at a checked `mach` and flow roll `roll`, per radian of `α`.
fn fin_term(set: &FinSetAero, mach: f64, roll: f64) -> Term {
    let FinLoading {
        slope_per_rad,
        cp_m,
    } = set.fin.loading_at(mach);
    let per_set = slope_per_rad * set.count_factor * set.interference;
    let station = set.fore_station_m + cp_m;
    let slope = per_set * roll_sum(set.count, set.base_angle_rad, roll);
    let side = per_set * side_sum(set.count, set.base_angle_rad, roll);
    Term {
        slope,
        moment: slope * station,
        side,
        side_moment: side * station,
        scale: slope.abs(),
    }
}

/// A body's contribution at the potential-flow and body-lift factors of [`alpha_factors`].
fn body_term(body: &BodyAero, potential: f64, lift: f64) -> Term {
    let (attached, lift) = (body.slope_per_rad * potential, body.lift_factor * lift);
    Term {
        slope: attached + lift,
        moment: body.moment_slope_m * potential + lift * body.lift_station_m,
        scale: attached.abs() + lift.abs(),
        ..Term::default()
    }
}

/// The per-radian factors of the potential-flow term (`sin α/α`) and of body lift
/// (`sin² α/α = sin α · sin α/α`).
fn alpha_factors(alpha_rad: f64) -> (f64, f64) {
    let s = sinc(alpha_rad);
    (s, alpha_rad.sin() * s)
}

fn body_terms(
    component: &PlacedComponent,
    geometry: BodyGeometry,
    step_area_m2: f64,
    a_ref: f64,
) -> BodyAero {
    let station = component.fore_station_m;
    let step_slope = 2.0 * step_area_m2 / a_ref;
    let slope = geometry.normal_force_slope(a_ref);
    BodyAero {
        id: component.id.clone(),
        fore_station_m: station,
        geometry,
        step_area_m2,
        slope_per_rad: slope + step_slope,
        moment_slope_m: (slope + step_slope) * station + geometry.moment_slope_m(a_ref),
        lift_factor: BODY_LIFT_K * geometry.planform_area_m2 / a_ref,
        lift_station_m: station + geometry.planform_centroid_m,
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

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
        Flow::new(mach, alpha_rad, roll_rad)
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

    /// Through subsonic flow, Mach changes the fins' slope by Prandtl–Glauert and nothing else;
    /// the bodies and every CP stay put.
    #[test]
    fn mach_changes_only_the_fins() {
        let m = model(&finned_rocket(4));
        let at = |mach| m.components(&Flow::axial(mach)).unwrap();
        let (slow, fast) = (at(0.0), at(0.8));
        for (a, b) in slow.iter().zip(&fast) {
            assert_eq!(
                a.normal_force.cp_station_m, b.normal_force.cp_station_m,
                "{}",
                a.id
            );
            if a.id == "fins" {
                let set = &m.fin_sets()[0];
                let ratio = set
                    .fin
                    .geometry
                    .single_fin_slope(m.reference_area_m2(), 0.8)
                    .unwrap()
                    / set
                        .fin
                        .geometry
                        .single_fin_slope(m.reference_area_m2(), 0.0)
                        .unwrap();
                assert!(ratio > 1.05, "{ratio}");
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
            .fin
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
            flow(NORMAL_FORCE_MACH_LIMIT, 0.0, 0.0),
            flow(-0.01, 0.0, 0.0),
            flow(f64::NAN, 0.0, 0.0),
        ] {
            assert!(matches!(
                m.normal_force(&bad),
                Err(AeroError::Mach { limit, .. }) if limit == NORMAL_FORCE_MACH_LIMIT
            ));
        }
        // The normal force flies on past Mach 1; the drag buildup doesn't, until M1.8b.
        assert!(m.normal_force(&flow(1.0, 0.1, 0.0)).is_ok());
        let coasting = DragConditions::coasting(1e7);
        assert!(matches!(
            m.drag(&flow(1.0, 0.0, 0.0), &coasting),
            Err(AeroError::Mach { limit, .. }) if limit == 1.0
        ));
        assert!(
            m.buildup_components(&flow(1.0, 0.0, 0.0), &coasting)
                .is_err()
        );
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
        fn components_sum_to_the_total(
            count in 1u32..=8,
            mach in 0.0f64..0.99,
            alpha in 0.0f64..PI,
            roll in -4.0f64..4.0,
        ) {
            let m = model(&finned_rocket(count));
            let f = flow(mach, alpha, roll);
            let total = m.normal_force(&f).unwrap();
            let parts = m.components(&f).unwrap();
            let (c, moment) = parts.iter().fold((0.0, 0.0), |(c, x), p| {
                (c + p.normal_force.coefficient, x + p.normal_force.moment_m)
            });
            let tol = 1e-12 * (1.0 + total.coefficient.abs());
            prop_assert!((c - total.coefficient).abs() <= tol);
            prop_assert!((moment - total.moment_m).abs() <= 1e-12 * (1.0 + total.moment_m.abs()));
            let side: f64 = parts.iter().map(|p| p.normal_force.side_coefficient).sum();
            prop_assert!((side - total.side_coefficient).abs() <= 1e-12 * (1.0 + total.side_coefficient.abs()));
            let slope: f64 = parts.iter().map(|p| p.normal_force.slope_per_rad).sum();
            prop_assert!((slope - total.slope_per_rad).abs() <= 1e-12 * total.slope_per_rad.abs());
            // The allocation-free per-component path gives the same terms, and each component's
            // small-angle station is its centre of pressure as α → 0.
            prop_assert_eq!(m.component_count(), parts.len());
            let small = flow(mach, 1e-6, roll);
            for (index, part) in parts.iter().enumerate() {
                prop_assert_eq!(&m.component_normal_force(index, &f).unwrap(), &part.normal_force);
                let station = m.component_station_m(index, mach).unwrap();
                if let Some(cp) = m.component_normal_force(index, &small).unwrap().cp_station_m {
                    prop_assert!((station - cp).abs() <= 1e-6 * (1.0 + cp.abs()));
                }
            }
            prop_assert!(m.component_normal_force(parts.len(), &f).is_err());
            prop_assert!(m.component_station_m(parts.len(), mach).is_none());
        }

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

    /// A step in radius where two body components meet counts as a zero-length transition at the
    /// joint, so the body's total slope is Barrowman 1966 eq. 10 over the whole body: `2` for any
    /// pointed body however its radii step.
    #[test]
    fn radius_steps_count_at_the_joint() {
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.027), None),
                component("tube", body_part(0.5, 0.029, 0.029), None),
                component("tail", body_part(0.3, 0.025, 0.025), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let a_ref = PI * 0.029 * 0.029;
        let total = m.normal_force(&Flow::axial(0.3)).unwrap();
        close(
            total.slope_per_rad,
            2.0 * PI * 0.025 * 0.025 / a_ref,
            1e-14,
            "eq. 10",
        );
        let parts = m.components(&Flow::axial(0.3)).unwrap();
        let tube = parts[1].normal_force;
        close(
            tube.slope_per_rad,
            2.0 * PI * (0.029f64.powi(2) - 0.027f64.powi(2)) / a_ref,
            1e-14,
            "step up",
        );
        close(
            tube.cp_station_m.unwrap(),
            0.2,
            1e-14,
            "step up at the joint",
        );
        let tail = parts[2].normal_force;
        assert!(tail.slope_per_rad < 0.0);
        close(
            tail.cp_station_m.unwrap(),
            0.7,
            1e-14,
            "step down at the joint",
        );
        assert_eq!(m.bodies()[0].step_area_m2, 0.0);
    }

    /// A freeform fin set through the model equals the same trapezoid given as a trapezoid.
    #[test]
    fn freeform_fins_through_the_model() {
        let trapezoid = finned_rocket(3);
        let mut freeform = trapezoid.clone();
        if let Part::FinSet(set) = &mut freeform.stages[0].components[3].children[0].part {
            set.planform = FinPlanform::Freeform {
                points_m: vec![[0.0, 0.0], [0.07, 0.06], [0.12, 0.06], [0.12, 0.0]],
            };
        }
        let (a, b) = (model(&trapezoid), model(&freeform));
        let f = flow(0.7, 0.1, 0.0);
        let (fa, fb) = (a.normal_force(&f).unwrap(), b.normal_force(&f).unwrap());
        close(fb.coefficient, fa.coefficient, 1e-13, "C_N");
        close(
            fb.cp_station_m.unwrap(),
            fa.cp_station_m.unwrap(),
            1e-13,
            "CP",
        );
    }

    /// `Flow` and `NormalForce` round-trip through JSON, and a misspelt flow field is refused.
    #[test]
    fn flow_and_results_round_trip() {
        let f = flow(0.3, 0.1, -0.2);
        let back: Flow = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(back, f);
        assert!(
            serde_json::from_str::<Flow>(
                r#"{"mach":0.3,"alpha_rad":0.1,"roll_rad":0,"aoa_deg":5}"#
            )
            .is_err()
        );
        let n = model(&finned_rocket(4)).normal_force(&f).unwrap();
        let back: NormalForce = serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();
        assert_eq!(back, n);
    }

    /// Layouts that don't hold together: fins without a body radius, a non-finite station.
    #[test]
    fn inconsistent_layouts_are_refused() {
        let layout = finned_rocket(4).layout().unwrap();
        let (fins, _) = layout.find("fins").unwrap();
        let mut no_radius = layout.clone();
        no_radius.components[fins].body_radius_m = None;
        let err = AeroModel::new(&no_radius).unwrap_err();
        assert!(
            matches!(&err, AeroError::InComponent { id, source } if id == "fins"
                && matches!(**source, AeroError::Layout(_))),
            "{err}"
        );
        assert_eq!(err, err.clone());
        let mut nan = layout;
        nan.components[0].fore_station_m = f64::NAN;
        assert!(matches!(
            AeroModel::new(&nan),
            Err(AeroError::InComponent { .. })
        ));
    }

    /// A two-fin set pushes along its fins' common normal: at 45° to the flow its side share
    /// equals its in-plane share, with the side moment at the fins' CP. Four fins have none.
    #[test]
    fn two_fin_sets_push_across_the_flow() {
        let two = model(&finned_rocket(2));
        let set = &two.fin_sets()[0];
        let alpha = 0.05;
        let f = two.normal_force(&flow(0.4, alpha, FRAC_PI_4)).unwrap();
        let one_fin = set
            .fin
            .geometry
            .single_fin_slope(two.reference_area_m2(), 0.4)
            .unwrap()
            * set.interference;
        close(f.side_coefficient, one_fin * alpha, 1e-13, "side");
        close(
            f.side_moment_m,
            one_fin * alpha * set.cp_station_m(0.4).unwrap(),
            1e-13,
            "side moment",
        );
        let fins = &two.components(&flow(0.4, alpha, FRAC_PI_4)).unwrap()[4];
        close(
            fins.normal_force.coefficient,
            one_fin * alpha,
            1e-13,
            "in plane",
        );
        let four = model(&finned_rocket(4))
            .normal_force(&flow(0.4, alpha, 0.3))
            .unwrap();
        assert_eq!((four.side_coefficient, four.side_moment_m), (0.0, 0.0));
    }

    /// A body whose areas cancel to round-off has no CP rather than a CP at 1e14 m; its moment is
    /// still reported.
    #[test]
    fn a_cancelled_slope_has_no_cp() {
        let rocket = one_stage(
            vec![
                component("nose", nose(NoseShape::Conical {}, 0.2, 0.0254), None),
                component("tube", body_part(0.5, 0.0254, 0.0254), None),
                component("tail", body_part(0.3, 0.0254, 1e-9), None),
            ],
            ReferenceDiameter::Maximum {},
        );
        let m = model(&rocket);
        let f = m.normal_force(&Flow::axial(0.3)).unwrap();
        assert!(f.slope_per_rad.abs() < 1e-12, "{}", f.slope_per_rad);
        assert_eq!(f.cp_station_m, None);
        let moving = m.normal_force(&flow(0.3, 0.01, 0.0)).unwrap();
        assert!(moving.moment_m.is_finite() && moving.cp_station_m.is_some());
    }
}
