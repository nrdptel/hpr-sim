//! Mass that moves along the airframe in flight: a ballast weight or a payload slid fore or aft
//! on a trigger, with the rocket's mass properties and its equations of motion following it.
//!
//! A [`MassShift`] moves one part carried inside the airframe, with everything inside it, by a
//! set distance along the axis over a set time, starting on a trigger as a recovery device does.
//! It follows a cycloid (the cam designer's "cycloidal motion"): its speed and acceleration are
//! zero at both ends, so the centre of mass moves smoothly. The rocket's mass is unchanged; its
//! centre of mass and inertia follow the part ([`hpr_design::MassProperties::with_part_moved`]).
//! The equations of motion take the moving centre of mass's terms as they take a burning motor's,
//! and add the part's angular momentum relative to the airframe, which is not zero when it moves
//! off the axis (the decision record on moving mass, [ADR-087][adr-087]).
//!
//! Method: `docs/physics/moving-mass.md`.
//!
//! [adr-087]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-087-mass-that-moves-along-the-airframe-2026-09-26

use std::f64::consts::TAU;

use hpr_core::{DMat3, DVec3};
use hpr_design::{Assembly, MassProperties, Rocket};
use serde::{Deserialize, Serialize};

use crate::dynamics::MassState;
use crate::error::SimError;
use crate::pieces::node;
use crate::recovery::Trigger;

/// The shortest time a [`MassShift`] may take, s.
///
/// A shorter move is nearer an impact than a motion: the cycloid's peak acceleration is
/// `2π travel/T²`, 63 km/s² for each metre of travel at this bound, and the part's stop at the end
/// would be a shock the rigid airframe here doesn't have. The equations take the shift's rates in
/// closed form, so they hold at any duration; the integrator follows the move in its own steps.
pub const MIN_SHIFT_DURATION_S: f64 = 0.01;

/// A part carried inside the airframe moving along its axis: `travel_m` aft (forward when
/// negative) over `duration_s`, starting on `trigger`.
///
/// The part is an internal component named by its id, and it moves with everything inside it. It
/// can't be a body component or an external one, one copy of a cluster's, or hold a motor (the
/// motor would stay where it is). It can't sit in a stage, or inside a component, whose
/// overridden mass covers it, since the override doesn't say how much of that mass is the part's.
/// Shifts of one part add; a part can't move inside another part that moves. Every shift of a
/// part, forward ones together and aft ones together, must keep it inside the component that
/// holds it.
///
/// The position along the travel is the cycloid `s(τ) = τ − sin(2πτ)/2π` of the fraction of the
/// time gone, `τ = (t − t₀)/T`: at rest at both ends, fastest at the middle at `2 travel/T`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct MassShift {
    /// When the part starts to move: the same triggers as a recovery device's.
    pub trigger: Trigger,
    /// The id of the internal component that moves.
    pub component: String,
    /// How far it moves along the axis, m: positive aft, toward the tail.
    pub travel_m: f64,
    /// How long it takes, s: at least [`MIN_SHIFT_DURATION_S`].
    pub duration_s: f64,
}

impl MassShift {
    /// Internal component `component` moving `travel_m` aft (forward when negative) over
    /// `duration_s`, starting on `trigger`.
    #[must_use]
    pub fn new(
        trigger: Trigger,
        component: impl Into<String>,
        travel_m: f64,
        duration_s: f64,
    ) -> Self {
        Self {
            trigger,
            component: component.into(),
            travel_m,
            duration_s,
        }
    }
}

/// The cycloid `s(τ) = τ − sin(2πτ)/2π` and its first two derivatives in `τ`, held at its ends
/// outside `[0, 1]`.
fn cycloid(tau: f64) -> (f64, f64, f64) {
    if tau <= 0.0 {
        (0.0, 0.0, 0.0)
    } else if tau >= 1.0 {
        (1.0, 0.0, 0.0)
    } else {
        let (sin, cos) = (TAU * tau).sin_cos();
        (tau - sin / TAU, 1.0 - cos, TAU * sin)
    }
}

/// One shift as the equations fly it.
#[derive(Debug, Clone)]
struct ShiftTerms {
    /// Which of [`Shifts::parts`] moves.
    part: usize,
    travel_m: f64,
    duration_s: f64,
    /// When it starts on the flight's clock, s: infinite while not known.
    start_s: f64,
}

impl ShiftTerms {
    /// The part's offset along body `z` from this shift at `t`, and its first two time
    /// derivatives, m, m/s, m/s². Aft is `−z`.
    fn offset(&self, t: f64) -> (f64, f64, f64) {
        if !self.start_s.is_finite() {
            return (0.0, 0.0, 0.0);
        }
        let (s, ds, dds) = cycloid((t - self.start_s) / self.duration_s);
        let along = -self.travel_m;
        (
            along * s,
            along * ds / self.duration_s,
            along * dds / (self.duration_s * self.duration_s),
        )
    }
}

/// A flight's mass shifts: the parts that move, as the design places them, and each shift.
#[derive(Debug, Clone, Default)]
pub(crate) struct Shifts {
    /// Each moving part with everything inside it, where the design puts it.
    parts: Vec<MassProperties>,
    terms: Vec<ShiftTerms>,
}

impl Shifts {
    /// The shifts of `assembly` (of `rocket`), each starting at `start_s` (one per shift, `None`
    /// while not known).
    ///
    /// # Errors
    ///
    /// [`SimError::Shift`] for a part that can't move ([`MassShift`] says which), and
    /// [`SimError::Domain`] for a travel that is not finite or a duration shorter than
    /// [`MIN_SHIFT_DURATION_S`] or not finite.
    pub(crate) fn new(
        rocket: &Rocket,
        assembly: &Assembly,
        shifts: &[MassShift],
        start_s: &[Option<f64>],
    ) -> Result<Self, SimError> {
        let components = &assembly.layout.components;
        let refuse = |what: &'static str, component: &str| SimError::Shift {
            what,
            component: component.to_owned(),
        };
        let mut moved: Vec<usize> = Vec::new();
        let mut terms = Vec::with_capacity(shifts.len());
        for (shift, start_s) in shifts.iter().zip(start_s) {
            let id = shift.component.as_str();
            if !shift.travel_m.is_finite() {
                return Err(SimError::Domain {
                    what: "travel of a mass shift, m",
                    value: shift.travel_m,
                });
            }
            if !(shift.duration_s.is_finite() && shift.duration_s >= MIN_SHIFT_DURATION_S) {
                return Err(SimError::Domain {
                    what: "duration of a mass shift, s (0.01 s or more)",
                    value: shift.duration_s,
                });
            }
            let index = components
                .iter()
                .position(|component| component.id == id)
                .ok_or_else(|| {
                    refuse("a mass shift names a component the design doesn't have", id)
                })?;
            let placed = &components[index];
            if placed.parent.is_none() {
                return Err(refuse(
                    "a mass shift moves a part carried inside the airframe, and this is a body \
                     component",
                    id,
                ));
            }
            if placed.body_radius_m.is_some() {
                return Err(refuse(
                    "a mass shift moves a part carried inside the airframe, and this part is \
                     outside it",
                    id,
                ));
            }
            if placed.copies_m.len() != 1 {
                return Err(refuse(
                    "a mass shift of a part that isn't exactly one part (one of several copies in \
                     a cluster of tubes, or none)",
                    id,
                ));
            }
            if !moved.contains(&index) {
                moved.push(index);
            }
            terms.push(ShiftTerms {
                part: moved.iter().position(|&part| part == index).unwrap_or(0),
                travel_m: shift.travel_m,
                duration_s: shift.duration_s,
                start_s: start_s.unwrap_or(f64::INFINITY),
            });
        }

        // What each moving part holds, and what holds it.
        let inside = |index: usize, ancestor: usize| {
            let mut at = components[index].parent;
            while let Some(parent) = at {
                if parent == ancestor {
                    return true;
                }
                at = components[parent].parent;
            }
            false
        };
        for &index in &moved {
            let id = components[index].id.as_str();
            if moved
                .iter()
                .any(|&other| other != index && inside(index, other))
            {
                return Err(refuse(
                    "a mass shift of a part inside another that moves",
                    id,
                ));
            }
            let holds_motor = assembly.motors.iter().any(|motor| {
                components
                    .iter()
                    .position(|component| component.id == motor.mount)
                    .is_some_and(|mount| mount == index || inside(mount, index))
            });
            if holds_motor {
                return Err(refuse(
                    "a mass shift of a part that holds a motor (the motor would stay where it is)",
                    id,
                ));
            }
            let stage = &assembly.layout.stages[components[index].stage];
            if rocket
                .stages
                .iter()
                .find(|written| written.id == stage.id)
                .is_some_and(|written| !written.overrides.is_empty())
            {
                return Err(refuse(
                    "a mass shift in a stage whose mass is overridden (the override doesn't say \
                     how much of it is the part's)",
                    id,
                ));
            }
            let mut at = components[index].parent;
            while let Some(parent) = at {
                if node(rocket, &components[parent].id).is_some_and(|holder| {
                    holder.overrides_include_children && !holder.overrides.is_empty()
                }) {
                    return Err(refuse(
                        "a mass shift inside a component whose overridden mass covers what it \
                         holds (the override doesn't say how much of it is the part's)",
                        id,
                    ));
                }
                at = components[parent].parent;
            }
            // Every shift forward together, and every one aft, keep it inside its holder.
            let (mut forward_m, mut aft_m) = (0.0, 0.0);
            for (shift, term) in shifts.iter().zip(&terms) {
                if moved[term.part] == index {
                    if shift.travel_m < 0.0 {
                        forward_m -= shift.travel_m;
                    } else {
                        aft_m += shift.travel_m;
                    }
                }
            }
            let part = &components[index];
            if let Some(holder) = part.parent.map(|parent| &components[parent])
                && (part.fore_station_m - forward_m < holder.fore_station_m
                    || part.fore_station_m + part.length_m + aft_m
                        > holder.fore_station_m + holder.length_m)
            {
                return Err(refuse(
                    "a mass shift that can take the part out of the component that holds it",
                    id,
                ));
            }
        }

        Ok(Self {
            parts: moved
                .iter()
                .map(|&index| components[index].with_children)
                .collect(),
            terms,
        })
    }

    /// Whether there are none.
    pub(crate) fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Starts shift `index` at `t_s`.
    pub(crate) fn start(&mut self, index: usize, t_s: f64) {
        if let Some(term) = self.terms.get_mut(index) {
            term.start_s = t_s;
        }
    }

    /// When shift `index` starts, s: `None` while not known.
    pub(crate) fn start_s(&self, index: usize) -> Option<f64> {
        self.terms
            .get(index)
            .map(|term| term.start_s)
            .filter(|t| t.is_finite())
    }

    /// The known starts and ends, s.
    pub(crate) fn knots_s(&self) -> Vec<f64> {
        self.terms
            .iter()
            .filter(|term| term.start_s.is_finite())
            .flat_map(|term| [term.start_s, term.start_s + term.duration_s])
            .collect()
    }

    /// When shift `index` ends, once its start is known, s.
    pub(crate) fn end_s(&self, index: usize) -> Option<f64> {
        self.start_s(index)
            .map(|start| start + self.terms[index].duration_s)
    }

    /// Each moving part's offset along body `z` at `t`, and its first two time derivatives.
    fn offsets(&self, t: f64) -> Vec<(f64, f64, f64)> {
        let mut offsets = vec![(0.0, 0.0, 0.0); self.parts.len()];
        for term in &self.terms {
            let (s, v, a) = term.offset(t);
            let sum = &mut offsets[term.part];
            *sum = (sum.0 + s, sum.1 + v, sum.2 + a);
        }
        offsets
    }

    /// `whole` (the design's mass properties at `t`, every part where the design puts it) with
    /// each moving part where it is at `t`.
    pub(crate) fn apply(&self, whole: MassProperties, t: f64) -> MassProperties {
        self.parts
            .iter()
            .zip(self.offsets(t))
            .filter(|(_, (offset, _, _))| *offset != 0.0)
            .fold(whole, |whole, (part, (offset, _, _))| {
                whole.with_part_moved(part, DVec3::new(0.0, 0.0, offset))
            })
    }

    /// Moves `state`, the design's mass state at `t` with every part where the design puts it
    /// (and `mass_second_kg_s2` its mass's second derivative, `M″`), to the moving parts' places,
    /// adding their rates in closed form. With `M` the whole's mass and, for each part, `m` its
    /// mass, `δ` its offset and `c = ρ₀ + δ` its centre,
    ///
    /// ```text
    /// r   = r_a + Σ m δ/M
    /// r′  = r_a′ + Σ m (δ′/M − δ M′/M²)
    /// r″  = r_a″ + Σ m (δ″/M − 2 δ′ M′/M² + δ (2M′²/M³ − M″/M²))
    /// I_O = I_Oa + Σ m (J(c) − J(ρ₀)),   J(v) = |v|² E − v vᵀ
    /// I_O′ = I_Oa′ + Σ m (2 (c·δ′) E − δ′ cᵀ − c δ′ᵀ)
    /// h   = Σ m c × δ′,   h′ = Σ m c × δ″   (δ′ × δ′ = 0)
    /// ```
    ///
    /// where `r_a` and `I_Oa` are the design's, and `h` is the parts' angular momentum about the
    /// nose tip relative to the airframe: a part only translates, so every point of it moves at
    /// `δ′`.
    pub(crate) fn shift_state(&self, state: &mut MassState, mass_second_kg_s2: f64, t: f64) {
        let offsets = self.offsets(t);
        let whole = MassProperties {
            mass_kg: state.mass_kg,
            cg_m: state.cg_m,
            inertia_kg_m2: state.inertia_cg,
        };
        let moved = self.apply(whole, t);
        state.cg_m = moved.cg_m;
        state.inertia_cg = moved.inertia_kg_m2;
        state.inertia_o = moved.inertia_about(DVec3::ZERO);
        let big_m = state.mass_kg;
        let (dm, ddm) = (state.mass_rate_kg_s, mass_second_kg_s2);
        for (part, (offset, speed, acceleration)) in self.parts.iter().zip(offsets) {
            if speed == 0.0 && acceleration == 0.0 && (offset == 0.0 || dm == 0.0) {
                continue;
            }
            let m = part.mass_kg;
            let (d, v, a) = (DVec3::Z * offset, DVec3::Z * speed, DVec3::Z * acceleration);
            let m2 = big_m * big_m;
            state.cg_rate_m_s += v * (m / big_m) - d * (m * dm / m2);
            state.cg_accel_m_s2 += a * (m / big_m) - v * (2.0 * m * dm / m2)
                + d * (m * (2.0 * dm * dm / (m2 * big_m) - ddm / m2));
            let c = part.cg_m + d;
            state.inertia_o_rate +=
                (DMat3::from_diagonal(DVec3::splat(2.0 * c.dot(v))) - outer(v, c) - outer(c, v))
                    * m;
            state.relative_momentum += c.cross(v) * m;
            state.relative_momentum_rate += c.cross(a) * m;
        }
    }
}

/// The outer product `u vᵀ`.
fn outer(u: DVec3, v: DVec3) -> DMat3 {
    DMat3::from_cols(u * v.x, u * v.y, u * v.z)
}

#[cfg(test)]
mod tests {
    use hpr_design::{Component, Part, Position};

    use super::*;
    use crate::flight::{EventKind, FlightResult, FlightSettings, Simulation, Termination};
    use crate::integrator::{Adaptive, Method};
    use crate::metrics::FlightMetrics;
    use crate::pieces::Ejection;
    use crate::rail::Rail;
    use crate::recorder::{FlightStep, Observer, Sample};
    use crate::state::State;
    use crate::testing::{UniformAir, analytic_environment, design};

    const G: f64 = 9.806_65;
    const BALLAST_KG: f64 = 0.2;
    /// The shift: 0.3 m aft over 1 s from 5 s, well after the I175's burnout at 2.5 s.
    const TRAVEL_M: f64 = 0.3;
    const START_S: f64 = 5.0;
    const DURATION_S: f64 = 1.0;

    /// The 54 mm single-stage test design with 0.2 kg of ballast, a cylinder 50 mm long and 15 mm
    /// in radius, carried in its airframe 0.1 m aft of the airframe's forward end, `offset_m` off
    /// the axis.
    fn with_ballast(offset_m: f64) -> Rocket {
        let mut rocket = design("synthetic-54mm-three-fin");
        let airframe = &mut rocket.stages[0].components[1];
        assert_eq!(airframe.id, "sustainer-airframe");
        let mut ballast: Component = airframe
            .children
            .iter()
            .find(|child| child.id == "altimeter")
            .cloned()
            .unwrap();
        ballast.id = "ballast".to_owned();
        let Part::MassComponent(mass) = &mut ballast.part else {
            panic!("the altimeter is a mass component");
        };
        mass.mass_kg = BALLAST_KG;
        mass.packing.length_m = 0.05;
        mass.packing.radius_m = 0.015;
        mass.packing.radial_offset_m = offset_m;
        ballast.position = Some(Position::Top { aft_offset_m: 0.1 });
        airframe.children.push(ballast);
        rocket
    }

    fn simulation(rocket: &Rocket, settings: FlightSettings) -> Simulation {
        Simulation::new(
            rocket,
            "i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            settings,
        )
        .unwrap()
    }

    fn shift(trigger: Trigger) -> MassShift {
        MassShift::new(trigger, "ballast", TRAVEL_M, DURATION_S)
    }

    /// The ballast's centre, and it with what it holds, as the design places it.
    fn ballast(sim: &Simulation) -> MassProperties {
        sim.assembly()
            .layout
            .components
            .iter()
            .find(|component| component.id == "ballast")
            .unwrap()
            .with_children
    }

    fn close(a: f64, b: f64, tolerance: f64, what: &str) {
        assert!(
            (a - b).abs() <= tolerance,
            "{what}: {a} vs {b} ({:e})",
            a - b
        );
    }

    #[test]
    fn mass_properties_before_during_and_after_a_shift_match_the_hand_calculation() {
        let sim = simulation(&with_ballast(0.0), FlightSettings::default())
            .with_shifts(vec![shift(Trigger::Time { time_s: START_S })])
            .unwrap();
        let result = sim.run(&mut ()).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        let started = result.event(EventKind::Shift(0)).unwrap().sample;
        assert_eq!(started.time_s, START_S);

        // By hand, for a part of mass m moved aft by Δ in a rocket of mass M, as two bodies: the
        // part and the rest. The centre moves aft by mΔ/M. About it the inertia is the two
        // bodies' own plus μ(|L|² E − L Lᵀ), with μ = m(M − m)/M the reduced mass and L the
        // vector from the rest's centre to the part's, `(ρ − cg) M/(M − m)`, whose `z` falls by
        // Δ. The rail buttons put the rocket's centre 30 µm off the axis, so `L` has an `x`.
        let part = ballast(&sim);
        let still = sim.assembly().mass_properties(START_S);
        let (m, big_m) = (part.mass_kg, still.mass_kg);
        let mu = m * (big_m - m) / big_m;
        let l = (part.cg_m - still.cg_m) * (big_m / (big_m - m));
        // Before; halfway, where the cycloid is at half the travel exactly; and after.
        for (t_s, fraction) in [(4.9, 0.0), (5.5, 0.5), (6.5, 1.0)] {
            let delta = TRAVEL_M * fraction;
            let got = sim.mass_properties(&result, t_s);
            let what = format!("at {t_s} s");
            assert_eq!(got.mass_kg, big_m, "{what}");
            close(got.cg_m.x, still.cg_m.x, 1e-18, &what);
            close(got.cg_m.y, still.cg_m.y, 1e-18, &what);
            close(got.cg_m.z, still.cg_m.z - m * delta / big_m, 1e-15, &what);
            let (i, i0) = (got.inertia_kg_m2, still.inertia_kg_m2);
            let (x, y, z0, z1) = (l.x, l.y, l.z, l.z - delta);
            // I_xx = μ(y² + z²), I_yy = μ(x² + z²), I_zz = μ(x² + y²), I_xz = −μ x z, I_yz = −μ y z.
            let change = |i: f64, i0: f64, before: f64, after: f64, which: &str| {
                close(
                    i - i0,
                    mu * (after - before),
                    1e-15,
                    &format!("{what}, {which}"),
                );
            };
            change(
                i.x_axis.x,
                i0.x_axis.x,
                y * y + z0 * z0,
                y * y + z1 * z1,
                "I_xx",
            );
            change(
                i.y_axis.y,
                i0.y_axis.y,
                x * x + z0 * z0,
                x * x + z1 * z1,
                "I_yy",
            );
            change(i.z_axis.z, i0.z_axis.z, 0.0, 0.0, "I_zz");
            change(i.x_axis.y, i0.x_axis.y, 0.0, 0.0, "I_xy");
            change(i.x_axis.z, i0.x_axis.z, -x * z0, -x * z1, "I_xz");
            change(i.y_axis.z, i0.y_axis.z, -y * z0, -y * z1, "I_yz");
        }
        // Measured: M = 0.8188 kg after the burn, so the centre moves 7.328 cm aft.
        close(big_m, 0.818_80, 1e-5, "mass");
        close(m * TRAVEL_M / big_m, 0.073_28, 1e-5, "centre's travel");
    }

    #[test]
    fn a_moving_mass_shifts_the_static_margin_by_the_hand_calculation() {
        // The static margin is the centre of pressure at Mach 0 against the centre of mass of the
        // instant, so on the coast, with the mass fixed, the hand calculation says it moves by
        // −m Δ s(τ)/(M d) as the ballast travels Δ s(τ) aft.
        let sim = simulation(&with_ballast(0.0), FlightSettings::default())
            .with_shifts(vec![shift(Trigger::Time { time_s: START_S })])
            .unwrap();
        let mut metrics = FlightMetrics::new();
        let result = sim.run(&mut metrics).unwrap();
        let apogee_s = result.event(EventKind::Apogee).unwrap().sample.time_s;
        assert!(apogee_s > START_S + DURATION_S + 1.0, "{apogee_s}");
        let part = ballast(&sim);
        let still = sim.assembly().mass_properties(START_S);
        let coast: Vec<_> = metrics
            .stability()
            .iter()
            .filter(|sample| sample.time_s > 3.0)
            .collect();
        let before = coast.first().unwrap();
        let d = before.reference_diameter_m;
        let margin_before = before.static_margin.margin_cal.unwrap();
        let (mut during, mut after) = (0, 0);
        for sample in &coast {
            let (s, _, _) = cycloid((sample.time_s - START_S) / DURATION_S);
            let expected = margin_before - part.mass_kg * TRAVEL_M * s / (still.mass_kg * d);
            let got = sample.static_margin.margin_cal.unwrap();
            close(
                got,
                expected,
                1e-12,
                &format!("margin at {} s", sample.time_s),
            );
            close(
                sample.cg_station_m,
                -still.cg_m.z + part.mass_kg * TRAVEL_M * s / still.mass_kg,
                1e-15,
                "centre of mass",
            );
            if s > 0.0 && s < 1.0 {
                during += 1;
            } else if s == 1.0 {
                after += 1;
            }
        }
        assert!(during >= 3 && after >= 3, "{during} during, {after} after");
        // Measured: 4.30 calibres before, 3.00 after the ballast's 0.3 m (1.30 calibres less).
        let margin_after = coast.last().unwrap().static_margin.margin_cal.unwrap();
        close(margin_before, 4.2973, 1e-4, "before");
        close(margin_before - margin_after, 1.3016, 1e-4, "change");
    }

    #[test]
    fn a_shift_s_rates_are_the_derivatives_of_its_mass_properties() {
        // Through the burn, where the mass falls, and on the coast: the closed forms against
        // central differences of the mass properties the shift gives, 1 µs apart for the rates
        // and 0.1 ms for the acceleration (at 1 µs its rounding is 2e-4 m/s²).
        for (start_s, at_s) in [(0.5, 1.2), (4.0, 4.3)] {
            let sim = simulation(&with_ballast(0.01), FlightSettings::default());
            let mut vehicle = crate::dynamics::Vehicle::lit(
                sim.assembly().clone(),
                sim.aero().clone(),
                sim.assembly().ignition_times_s(|_| None),
            )
            .unwrap();
            vehicle.shifts = Shifts::new(
                &with_ballast(0.01),
                sim.assembly(),
                &[shift(Trigger::Time { time_s: start_s })],
                &[Some(start_s)],
            )
            .unwrap();
            let window = (start_s, start_s + DURATION_S);
            let state = vehicle.mass_state(at_s, window);
            let h = 1e-6;
            let at = |t: f64| {
                let whole = vehicle
                    .assembly
                    .mass_properties_lit(t, vehicle.ignition_s());
                vehicle.shifts.apply(whole, t)
            };
            let (minus, mid, plus) = (at(at_s - h), at(at_s), at(at_s + h));
            let what = format!("at {at_s} s");
            assert_eq!(state.cg_m, mid.cg_m, "{what}");
            let rate = (plus.cg_m - minus.cg_m) / (2.0 * h);
            let wide = 1e-4;
            let accel =
                (at(at_s + wide).cg_m - 2.0 * mid.cg_m + at(at_s - wide).cg_m) / (wide * wide);
            let inertia_rate =
                (plus.inertia_about(DVec3::ZERO) - minus.inertia_about(DVec3::ZERO)) * (0.5 / h);
            assert!((state.cg_rate_m_s - rate).length() < 1e-9, "{what}: r′");
            // Measured: 6e-12 and 2e-11 m/s for r′; 2.3e-8 and 2.9e-8 of r″, the 0.1 ms
            // difference's own truncation, `(2π h/T)²/12 = 3.3e-8`.
            assert!(
                (state.cg_accel_m_s2 - accel).length() < 1e-7 * accel.length(),
                "{what}: r″ {:?} vs {accel:?}",
                state.cg_accel_m_s2
            );
            let error = (state.inertia_o_rate - inertia_rate)
                .to_cols_array()
                .iter()
                .fold(0.0_f64, |m, v| m.max(v.abs()));
            assert!(error < 1e-9, "{what}: I′ {error:e}");
            // The part's own relative angular momentum, off the axis by 1 cm in x: `m c × δ′`.
            let (_, ds, _) = cycloid((at_s - start_s) / DURATION_S);
            let speed = -TRAVEL_M * ds / DURATION_S;
            let part = ballast(&sim);
            let expected =
                DVec3::new(part.cg_m.y * speed, -part.cg_m.x * speed, 0.0) * part.mass_kg;
            assert!(
                (state.relative_momentum - expected).length() < 1e-18,
                "{what}: h"
            );
            assert!(expected.length() > 1e-4, "{what}: h {expected:?}");
        }
    }

    /// The samples at the end of every step.
    #[derive(Default)]
    struct Ends(Vec<Sample>);

    impl Observer for Ends {
        fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
            self.0.push(step.sample(step.end_s())?);
            Ok(())
        }
    }

    #[test]
    fn a_part_moving_off_the_axis_keeps_both_momenta_in_free_space() {
        // No air and no gravity, the motor spent, the rocket turning about all three axes: nothing
        // acts on it, so its centre of mass keeps its velocity and its angular momentum about that
        // centre is constant in the launch frame. The ballast, 1 cm off the axis, slides 0.3 m
        // aft over 1 s; its angular momentum relative to the airframe is what `ω × h + h′` carries.
        let t0 = 10.0;
        let settings = FlightSettings {
            method: Method::DormandPrince54(Adaptive {
                relative_tolerance: 1e-12,
                absolute_tolerance: 1e-12,
                ..Adaptive::default()
            }),
            max_time_s: t0 + 2.0,
            ..FlightSettings::default()
        };
        let sim = Simulation::new(
            &with_ballast(0.01),
            "i175",
            analytic_environment(UniformAir::vacuum(), 0.0),
            Rail::vertical(3.0),
            settings,
        )
        .unwrap()
        .with_shifts(vec![MassShift::new(
            Trigger::Time { time_s: t0 + 0.5 },
            "ballast",
            TRAVEL_M,
            DURATION_S,
        )])
        .unwrap();
        let attitude = Rail::vertical(3.0).attitude();
        let cg_m = sim.assembly().mass_properties(t0).cg_m;
        let state = State {
            position_enu_m: DVec3::new(0.0, 0.0, 1000.0) - attitude.mul_vec3(cg_m),
            velocity_enu_m_s: DVec3::new(3.0, -2.0, 10.0),
            attitude,
            body_rate_rad_s: DVec3::new(0.5, 0.2, 3.0),
        };
        let mut ends = Ends::default();
        let result: FlightResult = sim.run_free(t0, state, &mut ends).unwrap();
        assert_eq!(result.termination, Termination::TimeCap);
        assert!(result.event(EventKind::Shift(0)).is_some());

        let part = ballast(&sim);
        let momentum = |sample: &Sample| {
            let t = sample.time_s;
            let (s, ds, _) = cycloid((t - t0 - 0.5) / DURATION_S);
            let rho = part.cg_m - DVec3::Z * (TRAVEL_M * s);
            let rho_rate = -DVec3::Z * (TRAVEL_M * ds / DURATION_S);
            let whole = sim.mass_properties(&result, t);
            let relative = (rho - whole.cg_m).cross(rho_rate) * part.mass_kg;
            let body = whole.inertia_kg_m2 * sample.state.body_rate_rad_s + relative;
            (sample.state.unit_attitude().mul_vec3(body), relative)
        };
        let (h0, _) = momentum(&ends.0[0]);
        let v0 = ends.0[0].cg_velocity_enu_m_s;
        let (mut angular_error, mut linear_error, mut relative_peak) = (0.0_f64, 0.0_f64, 0.0_f64);
        for sample in &ends.0 {
            let (h, relative) = momentum(sample);
            angular_error = angular_error.max((h - h0).length() / h0.length());
            linear_error = linear_error.max((sample.cg_velocity_enu_m_s - v0).length());
            relative_peak = relative_peak.max(relative.length() / h0.length());
        }
        assert!(ends.0.len() > 20, "{} steps", ends.0.len());
        // Measured: 6.9e-12 of the angular momentum and 2.6e-12 m/s, against a relative angular
        // momentum that peaks at 1.8% of the whole: without its terms the error would be of that
        // order.
        assert!(angular_error < 1e-10, "angular momentum: {angular_error:e}");
        assert!(
            linear_error < 1e-10,
            "centre's velocity: {linear_error:e} m/s"
        );
        assert!(
            relative_peak > 1e-2,
            "relative angular momentum: {relative_peak:e}"
        );
    }

    #[test]
    fn a_shift_starts_at_apogee_or_at_its_height_on_the_way_down() {
        for (trigger, what) in [
            (Trigger::Apogee, "apogee"),
            (
                Trigger::Altitude {
                    height_above_ground_m: 200.0,
                },
                "height",
            ),
        ] {
            let sim = simulation(&with_ballast(0.0), FlightSettings::default())
                .with_shifts(vec![shift(trigger)])
                .unwrap();
            let result = sim.run(&mut ()).unwrap();
            let started = result.event(EventKind::Shift(0)).unwrap().sample;
            let apogee = result.event(EventKind::Apogee).unwrap().sample;
            match trigger {
                Trigger::Apogee => close(started.time_s, apogee.time_s, 0.0, what),
                _ => {
                    assert!(started.vertical_speed_m_s < 0.0, "{started:?}");
                    close(started.height_above_ground_m, 200.0, 1e-6, what);
                }
            }
            // It is where the design put it until then, and the travel on from its end.
            let part = ballast(&sim);
            let still = sim.assembly().mass_properties(started.time_s);
            let before = sim.mass_properties(&result, started.time_s - 1e-3);
            let after = sim.mass_properties(&result, started.time_s + DURATION_S);
            assert_eq!(
                before,
                sim.assembly().mass_properties(started.time_s - 1e-3)
            );
            close(
                after.cg_m.z,
                still.cg_m.z - part.mass_kg * TRAVEL_M / still.mass_kg,
                1e-15,
                what,
            );
        }
    }

    /// The `what` of a refused shift.
    fn refusal(shifts: Vec<MassShift>) -> SimError {
        simulation(&with_ballast(0.0), FlightSettings::default())
            .with_shifts(shifts)
            .unwrap_err()
    }

    #[test]
    fn shifts_that_cannot_be_made_are_refused() {
        let time = Trigger::Time { time_s: START_S };
        let refused = |component: &str, travel_m: f64, starts: &str| {
            let error = refusal(vec![MassShift::new(time, component, travel_m, 1.0)]);
            let SimError::Shift {
                what,
                component: id,
            } = &error
            else {
                panic!("{error:?}");
            };
            assert!(what.starts_with(starts), "{component}: {what}");
            assert_eq!(id, component);
        };
        refused("no-such-part", 0.1, "a mass shift names a component");
        refused(
            "sustainer-airframe",
            0.1,
            "a mass shift moves a part carried inside the airframe, and this is a body",
        );
        refused(
            "sustainer-rail-buttons",
            0.1,
            "a mass shift moves a part carried inside the airframe, and this part is outside",
        );
        refused(
            "sustainer-motor-mount",
            0.1,
            "a mass shift of a part that holds a motor",
        );
        // 0.1 m from the airframe's forward end: 0.15 m forward leaves it.
        refused("ballast", -0.15, "a mass shift that can take the part out");
        refused("ballast", 0.8, "a mass shift that can take the part out");
        // Forward and aft each add up: two of 0.04 m forward stay inside, three don't.
        let forward = MassShift::new(time, "ballast", -0.04, 1.0);
        simulation(&with_ballast(0.0), FlightSettings::default())
            .with_shifts(vec![forward.clone(), forward.clone()])
            .unwrap();
        assert!(matches!(
            refusal(vec![forward.clone(), forward.clone(), forward]),
            SimError::Shift { what, .. } if what.starts_with("a mass shift that can take")
        ));

        for (travel_m, duration_s, starts) in [
            (f64::NAN, 1.0, "travel of a mass shift"),
            (0.1, 0.009, "duration of a mass shift"),
            (0.1, f64::INFINITY, "duration of a mass shift"),
        ] {
            let error = refusal(vec![MassShift::new(time, "ballast", travel_m, duration_s)]);
            assert!(
                matches!(&error, SimError::Domain { what, .. } if what.starts_with(starts)),
                "{error:?}"
            );
        }
        let error = refusal(vec![shift(Trigger::Altitude {
            height_above_ground_m: -1.0,
        })]);
        assert!(
            matches!(&error, SimError::Domain { what, .. } if what.starts_with("height above the launch site at which a part")),
            "{error:?}"
        );

        // With ejections, in either order.
        let ejection = Ejection::aft_of(Trigger::Apogee, "nose");
        let error = simulation(&with_ballast(0.0), FlightSettings::default())
            .with_shifts(vec![shift(time)])
            .unwrap()
            .with_ejections(vec![ejection.clone()])
            .unwrap_err();
        assert!(
            matches!(error, SimError::Unsupported { what } if what.starts_with("a mass shift in a flight with"))
        );
        let error = simulation(&with_ballast(0.0), FlightSettings::default())
            .with_ejections(vec![ejection])
            .unwrap()
            .with_shifts(vec![shift(time)])
            .unwrap_err();
        assert!(
            matches!(error, SimError::Unsupported { what } if what.starts_with("a mass shift in a flight with"))
        );
    }

    #[test]
    fn a_part_inside_another_that_moves_is_refused() {
        let mut rocket = with_ballast(0.0);
        let airframe = &mut rocket.stages[0].components[1];
        let mount = airframe
            .children
            .iter_mut()
            .find(|child| child.id == "sustainer-motor-mount")
            .unwrap();
        let mut held = mount.clone();
        held.children.clear();
        let mut weight = rocket.stages[0].components[1]
            .children
            .iter()
            .find(|child| child.id == "ballast")
            .cloned()
            .unwrap();
        weight.id = "held".to_owned();
        weight.position = Some(Position::Top { aft_offset_m: 0.0 });
        let _ = held;
        rocket.stages[0].components[1]
            .children
            .iter_mut()
            .find(|child| child.id == "sustainer-motor-mount")
            .unwrap()
            .children
            .push(weight);
        let time = Trigger::Time { time_s: START_S };
        let error = simulation(&rocket, FlightSettings::default())
            .with_shifts(vec![
                MassShift::new(time, "held", 0.01, 1.0),
                MassShift::new(time, "sustainer-motor-mount", 0.01, 1.0),
            ])
            .unwrap_err();
        assert!(
            matches!(&error, SimError::Shift { what, component } if what.starts_with("a mass shift of a part inside another") && component == "held"),
            "{error:?}"
        );
    }

    #[test]
    fn the_cycloid_rests_at_both_ends_and_is_fastest_halfway() {
        assert_eq!(cycloid(-0.5), (0.0, 0.0, 0.0));
        assert_eq!(cycloid(1.5), (1.0, 0.0, 0.0));
        let (s, ds, dds) = cycloid(0.5);
        assert!((s - 0.5).abs() < 1e-16);
        assert!((ds - 2.0).abs() < 1e-15);
        assert!(dds.abs() < 1e-14);
        // A quarter of the way: s = ¼ − 1/2π, s′ = 1, s″ = 2π.
        let (s, ds, dds) = cycloid(0.25);
        assert!((s - (0.25 - 1.0 / TAU)).abs() < 1e-16);
        assert!((ds - 1.0).abs() < 1e-15);
        assert!((dds - TAU).abs() < 1e-14);
        // Its derivatives are the derivatives: central differences agree.
        let h = 1e-6;
        for tau in [0.1, 0.3, 0.7, 0.9] {
            let (s_minus, ds_minus, _) = cycloid(tau - h);
            let (s_plus, ds_plus, _) = cycloid(tau + h);
            let (_, ds, dds) = cycloid(tau);
            assert!(((s_plus - s_minus) / (2.0 * h) - ds).abs() < 1e-8, "{tau}");
            assert!(
                ((ds_plus - ds_minus) / (2.0 * h) - dds).abs() < 1e-7,
                "{tau}"
            );
        }
    }
}
