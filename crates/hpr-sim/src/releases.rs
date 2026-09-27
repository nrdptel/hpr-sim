//! Mass released in flight: ballast dropped or a payload let go on a trigger, with the rest of the
//! rocket flying on in six degrees of freedom and the part falling to the ground on its own.
//!
//! A [`MassRelease`] lets one part carried inside the airframe, with everything inside it, leave
//! at an instant. At that instant the rocket's mass properties step to the rest's
//! ([`hpr_design::MassProperties::without_part`]), and its state carries straight across: the
//! state is the nose tip's, which the rest keeps, as a sustainer keeps it at a powered separation.
//! The part leaves at the velocity its own centre of mass had as part of the airframe,
//! `v_O + ω × c`, so the mass and the momentum of the two together are those of the rocket just
//! before. The part then falls as a point mass under its own drag area to the ground (the decision
//! record on released mass, [ADR-088][adr-088]).
//!
//! Method: the documentation site's [Released mass][page] page.
//!
//! [page]: https://github.com/nrdptel/hpr-sim/blob/main/docs/physics/released-mass.md
//! [adr-088]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-088-mass-released-in-flight-2026-09-26

use hpr_core::DVec3;
use hpr_design::{Assembly, MassProperties, Rocket};
use serde::{Deserialize, Serialize};

use crate::error::SimError;
use crate::events::Direction;
use crate::flight::{EventKind, Simulation, Termination};
use crate::integrator::{Advance, IntegrationError, Integrator, OdeSystem, Stats};
use crate::recovery::{BodyEvent, BodySample, Trigger};
use crate::shifts::{NotAPart, NotCarried, check_carried, inside, locate_part};

/// A part carried inside the airframe that leaves it on `trigger`, and then falls to the ground
/// on its own under `drag_area_m2`.
///
/// The part is an internal component named by its id, and it leaves with everything inside it.
/// It can't be a body component or an external one, one copy of a cluster's, or hold a motor.
/// Its mass can't be under an override: not its stage's, and not one on a component around it
/// that covers what that component holds, since the override doesn't say how much of the mass is
/// the part's. A part can be released once, and not from inside another part that is released.
///
/// The drag area `C_D S` is the part's own once it is out, m²: a tumbling weight's, or its
/// parachute's taken as open at once. It must be positive: a part with none would fall as if in a
/// vacuum, which is a wrong number rather than a model.
///
/// A release can't come before the rocket leaves the rail: the part has nowhere to go on the pad.
///
/// ```
/// use hpr_sim::{MassRelease, Trigger};
///
/// // The part with id "payload" leaves at apogee and falls under 0.3 m² of parachute.
/// let release = MassRelease::new(Trigger::Apogee, "payload", 0.3);
/// assert_eq!(release.drag_area_m2, 0.3);
/// ```
///
/// Give it to a flight with [`crate::Simulation::with_releases`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct MassRelease {
    /// When the part leaves: the same triggers as a recovery device's.
    pub trigger: Trigger,
    /// The id of the internal component that leaves.
    pub component: String,
    /// The part's own drag area `C_D S` once it is out, m²: positive.
    pub drag_area_m2: f64,
}

impl MassRelease {
    /// Internal component `component` leaving on `trigger`, then falling under `drag_area_m2`.
    #[must_use]
    pub fn new(trigger: Trigger, component: impl Into<String>, drag_area_m2: f64) -> Self {
        Self {
            trigger,
            component: component.into(),
            drag_area_m2,
        }
    }
}

/// A released part's own flight, from the instant it left the rocket to its landing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleasedFlight {
    /// Which release, by its index in the flight's releases ([`crate::Simulation::with_releases`]).
    pub release: usize,
    /// The part as it left: its centre of mass where it was in the airframe, with the velocity
    /// that point had, `v_O + ω × c`.
    pub start_sample: BodySample,
    /// Why its flight ended.
    pub termination: Termination,
    /// Its events, in order: its own [`EventKind::Apogee`] if it left climbing, and its
    /// [`EventKind::GroundHit`].
    pub events: Vec<BodyEvent>,
    /// Where it ended.
    pub final_sample: BodySample,
    /// The integrator's work on it.
    pub stats: Stats,
}

/// A flight's releases: each part that leaves, as the design places it.
#[derive(Debug, Clone, Default)]
pub(crate) struct Releases {
    /// Each release's part with everything inside it, where the design puts it.
    parts: Vec<MassProperties>,
}

impl Releases {
    /// The releases of `assembly` (of `rocket`).
    ///
    /// # Errors
    ///
    /// [`SimError::Release`] for a part that can't be released ([`MassRelease`] says which), and
    /// [`SimError::Domain`] for a drag area that is not positive and finite.
    pub(crate) fn new(
        rocket: &Rocket,
        assembly: &Assembly,
        releases: &[MassRelease],
    ) -> Result<Self, SimError> {
        let components = &assembly.layout.components;
        let refuse = |what: &'static str, component: &str| SimError::Release {
            what,
            component: component.to_owned(),
        };
        let mut indices: Vec<usize> = Vec::with_capacity(releases.len());
        for release in releases {
            let id = release.component.as_str();
            if !(release.drag_area_m2.is_finite() && release.drag_area_m2 > 0.0) {
                return Err(SimError::Domain {
                    what: "drag area of a released part, m² (positive)",
                    value: release.drag_area_m2,
                });
            }
            let index = locate_part(assembly, id).map_err(|refusal| {
                refuse(
                    match refusal {
                        NotAPart::Missing => {
                            "a mass release names a component the design doesn't have"
                        }
                        NotAPart::BodyComponent => {
                            "a mass release lets go of a part carried inside the airframe, and \
                             this is a body component"
                        }
                        NotAPart::Outside => {
                            "a mass release lets go of a part carried inside the airframe, and \
                             this part is outside it"
                        }
                        NotAPart::NotOnePart => {
                            "a mass release of a part that isn't exactly one part (one of several \
                             copies in a cluster of tubes, or none)"
                        }
                    },
                    id,
                )
            })?;
            if indices.contains(&index) {
                return Err(refuse("a mass release of a part released already", id));
            }
            indices.push(index);
        }
        for &index in &indices {
            let id = components[index].id.as_str();
            if indices
                .iter()
                .any(|&other| other != index && inside(components, index, other))
            {
                return Err(refuse(
                    "a mass release of a part inside another that is released",
                    id,
                ));
            }
            check_carried(rocket, assembly, index).map_err(|refusal| {
                refuse(
                    match refusal {
                        NotCarried::HoldsMotor => {
                            "a mass release of a part that holds a motor (the rocket's motors \
                             stay with it)"
                        }
                        NotCarried::StageOverride => {
                            "a mass release in a stage whose mass is overridden (the override \
                             doesn't say how much of it is the part's)"
                        }
                        NotCarried::CoveredOverride => {
                            "a mass release inside a component whose overridden mass covers what \
                             it holds (the override doesn't say how much of it is the part's)"
                        }
                    },
                    id,
                )
            })?;
        }
        Ok(Self {
            parts: indices
                .iter()
                .map(|&index| components[index].with_children)
                .collect(),
        })
    }

    /// Release `index`'s part with everything inside it, where the design puts it, in body axes.
    pub(crate) fn part(&self, index: usize) -> &MassProperties {
        &self.parts[index]
    }
}

/// A released part as the integrator sees it: a point mass under its own drag area, with its
/// centre of mass and that point's velocity as the state.
///
/// The equations are a separated body's (`docs/physics/recovery.md`) with a fixed drag area:
/// `m a = −½ ρ (C_D S) |v − w| (v − w) + m (g + a_Coriolis)`.
struct PartSystem<'a> {
    simulation: &'a Simulation,
    mass_kg: f64,
    drag_area_m2: f64,
    /// Whether it is watching for its own apogee, which is event 1 when it is.
    apogee: bool,
    failure: Option<SimError>,
}

impl OdeSystem<6> for PartSystem<'_> {
    type Error = SimError;

    fn derivative(&mut self, _t_s: f64, y: &[f64; 6]) -> Result<[f64; 6], SimError> {
        self.simulation
            .point_mass_derivative(y, self.mass_kg, self.drag_area_m2)
    }

    fn event_count(&self) -> usize {
        1 + usize::from(self.apogee)
    }

    fn event_direction(&self, _index: usize) -> Direction {
        Direction::Falling
    }

    fn event_value(&mut self, index: usize, t_s: f64, y: &[f64; 6]) -> f64 {
        match self
            .simulation
            .point_sample(t_s, y, self.mass_kg, self.drag_area_m2)
        {
            // The ground, then its apogee if it is looking for one.
            Ok(sample) if index == 0 => sample.height_above_ground_m,
            Ok(sample) => sample.vertical_speed_m_s,
            Err(error) => {
                self.failure.get_or_insert(error);
                f64::NAN
            }
        }
    }
}

impl Simulation {
    /// Flies release `release`'s part, of `mass_kg`, from `t0` with its centre of mass at
    /// `cg_enu_m` moving at `velocity_enu_m_s`, to the ground or the time cap.
    pub(crate) fn fly_released(
        &self,
        release: usize,
        t0: f64,
        (cg_enu_m, velocity_enu_m_s): (DVec3, DVec3),
        mass_kg: f64,
    ) -> Result<ReleasedFlight, SimError> {
        let settings = self.settings();
        let drag_area_m2 = self.releases()[release].drag_area_m2;
        let start = [
            cg_enu_m.x,
            cg_enu_m.y,
            cg_enu_m.z,
            velocity_enu_m_s.x,
            velocity_enu_m_s.y,
            velocity_enu_m_s.z,
        ];
        let start_sample = self.point_sample(t0, &start, mass_kg, drag_area_m2)?;
        if start_sample.height_above_ground_m <= 0.0 {
            // The ground event is a falling crossing, so a part that starts below the site would
            // integrate underground to the time cap.
            return Err(SimError::Domain {
                what: "starting height of a released part's centre of mass above the ground",
                value: start_sample.height_above_ground_m,
            });
        }
        let mut integrator =
            Integrator::new(settings.method, t0, start)?.with_step_limit(settings.step_limit);
        let cap = settings.max_time_s;
        let mut events: Vec<BodyEvent> = Vec::new();
        let termination = loop {
            if integrator.time_s() >= cap {
                break Termination::TimeCap;
            }
            let apogee = events.is_empty() && start_sample.vertical_speed_m_s > 0.0;
            let mut system = PartSystem {
                simulation: self,
                mass_kg,
                drag_area_m2,
                apogee,
                failure: None,
            };
            let outcome = integrator.advance(&mut system, cap);
            if let Some(error) = system.failure.take() {
                return Err(error);
            }
            let outcome = match outcome {
                Ok(outcome) => outcome,
                Err(IntegrationError::StepLimit { .. }) => break Termination::StepLimit,
                Err(IntegrationError::Derivative { source, .. }) => return Err(source),
                Err(error) => return Err(SimError::Integration(Box::new(error))),
            };
            if let Advance::Events = outcome {
                let fired = integrator.fired_events().to_vec();
                let (t, y) = (integrator.time_s(), *integrator.state());
                let sample = self.point_sample(t, &y, mass_kg, drag_area_m2)?;
                if apogee && fired.contains(&1) {
                    events.push(BodyEvent {
                        kind: EventKind::Apogee,
                        sample,
                        after: None,
                    });
                }
                if fired.contains(&0) {
                    events.push(BodyEvent {
                        kind: EventKind::GroundHit,
                        sample,
                        after: None,
                    });
                    break Termination::GroundHit;
                }
            }
        };
        let final_sample = self.point_sample(
            integrator.time_s(),
            integrator.state(),
            mass_kg,
            drag_area_m2,
        )?;
        Ok(ReleasedFlight {
            release,
            start_sample,
            termination,
            events,
            final_sample,
            stats: integrator.stats(),
        })
    }
}

#[cfg(test)]
mod tests {
    use hpr_core::DMat3;
    use hpr_design::Part;

    use super::*;
    use crate::MassShift;
    use crate::flight::{FlightSettings, Simulation};
    use crate::integrator::{Adaptive, Method};
    use crate::pieces::Ejection;
    use crate::rail::Rail;
    use crate::recorder::Sample;
    use crate::recovery::{Device, DeviceDrag, Separation, terminal_speed_m_s};
    use crate::state::State;
    use crate::testing::{
        BALLAST_KG, Ends, UniformAir, analytic_environment, design, with_ballast, with_sleeve,
    };

    const G: f64 = 9.806_65;
    /// The release: 5 s after launch, well after the I175's burnout at 2.5 s.
    const RELEASE_S: f64 = 5.0;
    /// The ballast's own drag area once it is out, m².
    const PART_DRAG_AREA_M2: f64 = 0.01;

    fn simulation(
        rocket: &Rocket,
        air: UniformAir,
        g: f64,
        settings: FlightSettings,
    ) -> Simulation {
        Simulation::new(
            rocket,
            "i175",
            analytic_environment(air, g),
            Rail::vertical(3.0),
            settings,
        )
        .unwrap()
    }

    fn release(trigger: Trigger) -> MassRelease {
        MassRelease::new(trigger, "ballast", PART_DRAG_AREA_M2)
    }

    fn tight() -> Method {
        Method::DormandPrince54(Adaptive {
            relative_tolerance: 1e-12,
            absolute_tolerance: 1e-12,
            ..Adaptive::default()
        })
    }

    /// The ballast, with what it holds, as the design places it.
    fn ballast(sim: &Simulation) -> MassProperties {
        sim.assembly()
            .layout
            .find("ballast")
            .unwrap()
            .1
            .with_children
    }

    fn close(a: f64, b: f64, tolerance: f64, what: &str) {
        assert!(
            (a - b).abs() <= tolerance,
            "{what}: {a} vs {b} ({:e})",
            a - b
        );
    }

    fn close_vec(a: DVec3, b: DVec3, tolerance: f64, what: &str) {
        assert!(
            (a - b).length() <= tolerance,
            "{what}: {a:?} vs {b:?} ({:e})",
            (a - b).length()
        );
    }

    #[test]
    fn mass_properties_after_a_release_match_the_hand_calculation() {
        let sim = simulation(
            &with_ballast(0.01),
            UniformAir::sea_level(),
            G,
            FlightSettings::default(),
        )
        .with_releases(vec![release(Trigger::Time { time_s: RELEASE_S })])
        .unwrap();
        let mut ends = Ends::default();
        let result = sim.run(&mut ends).unwrap();
        let event = result.event(EventKind::MassRelease(0)).unwrap().sample;
        assert_eq!(event.time_s, RELEASE_S);

        // By hand, as two bodies: the part (m, centre c, own inertia I_p) and the rest (M − m).
        // The rest's centre is (M cg − m c)/(M − m). About the whole's centre the inertia is the
        // two bodies' own plus μ(|L|² E − L Lᵀ), with μ = m(M − m)/M the reduced mass and L the
        // vector from the rest's centre to the part's; so the rest's own is what is left.
        let part = ballast(&sim);
        let whole = sim.assembly().mass_properties(RELEASE_S);
        let (m, big_m) = (part.mass_kg, whole.mass_kg);
        assert_eq!(m, BALLAST_KG);
        let rest_kg = big_m - m;
        let rest_cg = (whole.cg_m * big_m - part.cg_m * m) / rest_kg;
        let l = part.cg_m - rest_cg;
        let mu = m * rest_kg / big_m;
        let pair = DMat3::from_diagonal(DVec3::splat(l.length_squared()))
            - DMat3::from_cols(l * l.x, l * l.y, l * l.z);
        let rest_inertia = whole.inertia_kg_m2 - part.inertia_kg_m2 - pair * mu;

        // Before, the design's; at the release and after, the rest's.
        assert_eq!(
            sim.mass_properties(&result, RELEASE_S - 1e-3),
            sim.assembly().mass_properties(RELEASE_S - 1e-3)
        );
        for t_s in [RELEASE_S, RELEASE_S + 1.0] {
            let got = sim.mass_properties(&result, t_s);
            let what = format!("at {t_s} s");
            close(got.mass_kg, rest_kg, 1e-15, &what);
            close_vec(got.cg_m, rest_cg, 1e-15, &what);
            let diff = (got.inertia_kg_m2 - rest_inertia)
                .to_cols_array()
                .iter()
                .fold(0.0_f64, |most, v| most.max(v.abs()));
            assert!(diff <= 1e-15, "{what}: inertia off by {diff:e}");
        }

        // The flight flew them: each step's mass, and its centre where the rest's is.
        for sample in &ends.0 {
            let expected = if sample.time_s <= RELEASE_S {
                sim.assembly().mass_properties(sample.time_s).mass_kg
            } else {
                rest_kg
            };
            close(sample.mass_kg, expected, 1e-15, "a step's mass");
            if sample.time_s > RELEASE_S {
                close_vec(
                    sample.cg_enu_m,
                    sample.state.point_enu_m(rest_cg),
                    1e-12,
                    "a step's centre",
                );
            }
        }

        // The part leaves from where it was, with the velocity that point had, and lands.
        assert_eq!(result.released.len(), 1);
        let flown = &result.released[0];
        assert_eq!(flown.release, 0);
        let start = flown.start_sample;
        assert_eq!(start.time_s, RELEASE_S);
        assert_eq!(start.mass_kg, m);
        assert_eq!(start.recovery_drag_area_m2, PART_DRAG_AREA_M2);
        let state = event.state;
        close_vec(
            start.cg_enu_m,
            state.point_enu_m(part.cg_m),
            1e-12,
            "the part's start",
        );
        close_vec(
            start.cg_velocity_enu_m_s,
            state.velocity_enu_m_s
                + state
                    .unit_attitude()
                    .mul_vec3(state.body_rate_rad_s.cross(part.cg_m)),
            1e-12,
            "the part's velocity",
        );
        assert_eq!(flown.termination, Termination::GroundHit);
        assert!(flown.final_sample.height_above_ground_m.abs() < 1e-6);
        assert_eq!(result.termination, Termination::GroundHit);
    }

    #[test]
    fn a_release_conserves_mass_and_momentum_in_free_space() {
        // No air and no gravity, the motor spent, the rocket turning about all three axes: nothing
        // acts on the rocket or on the part once it is out, so the two together keep the momentum
        // and the angular momentum the rocket had. The ballast is 1 cm off the axis, so it leaves
        // with a velocity across the rocket's and carries angular momentum away.
        let t0 = 10.0;
        let release_s = t0 + 0.5;
        let sim = simulation(
            &with_ballast(0.01),
            UniformAir::vacuum(),
            0.0,
            FlightSettings {
                method: tight(),
                max_time_s: t0 + 2.0,
                ..FlightSettings::default()
            },
        )
        .with_releases(vec![release(Trigger::Time { time_s: release_s })])
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
        let result = sim.run_free(t0, state, &mut ends).unwrap();
        assert_eq!(result.termination, Termination::TimeCap);
        let before = result.event(EventKind::MassRelease(0)).unwrap().sample;
        assert_eq!(before.time_s, release_s);
        let flown = &result.released[0];
        assert_eq!(flown.termination, Termination::TimeCap);
        let part = ballast(&sim);
        let whole = sim.assembly().mass_properties(t0);
        let rest = sim.mass_properties(&result, release_s);

        // Mass: the rest and the part make the rocket.
        close(
            rest.mass_kg + flown.start_sample.mass_kg,
            whole.mass_kg,
            1e-15,
            "mass",
        );
        // About the launch frame's origin: `Σ m x × v` plus each body's spin `R I ω`. The part
        // flies as a point mass, so its own spin, `R I_p ω` at the release, is carried as it was.
        let spin = |inertia: DMat3, sample: &Sample| {
            sample
                .state
                .unit_attitude()
                .mul_vec3(inertia * sample.state.body_rate_rad_s)
        };
        let momentum0 = before.cg_velocity_enu_m_s * whole.mass_kg;
        let angular0 = before.cg_enu_m.cross(momentum0) + spin(whole.inertia_kg_m2, &before);
        let part_spin = spin(part.inertia_kg_m2, &before);
        let v_part = flown.start_sample.cg_velocity_enu_m_s;
        let (mut linear_error, mut angular_error, mut after) = (0.0_f64, 0.0_f64, 0);
        for sample in ends.0.iter().filter(|sample| sample.time_s > release_s) {
            after += 1;
            let x_part = flown.start_sample.cg_enu_m + v_part * (sample.time_s - release_s);
            let momentum = sample.cg_velocity_enu_m_s * rest.mass_kg + v_part * part.mass_kg;
            let angular = sample
                .cg_enu_m
                .cross(sample.cg_velocity_enu_m_s * rest.mass_kg)
                + spin(rest.inertia_kg_m2, sample)
                + x_part.cross(v_part * part.mass_kg)
                + part_spin;
            linear_error = linear_error.max((momentum - momentum0).length() / momentum0.length());
            angular_error = angular_error.max((angular - angular0).length() / angular0.length());
        }
        assert!(after > 10, "{after} steps after the release");
        // Measured: see the ADR.
        assert!(linear_error < 1e-12, "momentum: {linear_error:e}");
        assert!(angular_error < 1e-10, "angular momentum: {angular_error:e}");
        // The part keeps its velocity with nothing acting on it.
        close_vec(
            flown.final_sample.cg_velocity_enu_m_s,
            v_part,
            1e-12,
            "the part's final velocity",
        );
        // It leaves across the rocket's velocity: ω × c is not small here.
        assert!((v_part - before.cg_velocity_enu_m_s).length() > 0.1);
        // The part's own spin is a small share of the whole's angular momentum.
        assert!(part_spin.length() / angular0.length() < 1e-3);

        // A flight that starts after the release starts without the part, and records nothing.
        let later = sim.run_free(t0 + 1.0, state, &mut ()).unwrap();
        assert!(later.event(EventKind::MassRelease(0)).is_none());
        assert!(later.released.is_empty());
        close(later.final_sample.mass_kg, rest.mass_kg, 1e-15, "later");
    }

    #[test]
    fn a_payload_let_go_under_the_drogue_lands_slower_and_falls_at_its_own_speed() {
        // In uniform sea-level air, a 0.3 m² drogue opens at apogee and the ballast leaves at
        // 150 m on the way down, under 0.05 m² of its own. Each settles at its own terminal speed
        // √(2 m g / (ρ C_D S)): the rest under the drogue, lighter than the rocket was, and the
        // ballast on its own. A speed nears its terminal one at the rate 2g/v_t, so the 20 s of
        // the fall bring the 8 m/s ballast to it to far below the tolerance.
        let drogue = Device::new(
            "drogue",
            DeviceDrag::DragArea { cd_s_m2: 0.3 },
            Trigger::Apogee,
        );
        let sim = simulation(
            &with_ballast(0.0),
            UniformAir::sea_level(),
            G,
            FlightSettings {
                method: tight(),
                ..FlightSettings::default()
            },
        )
        .with_recovery(vec![drogue])
        .unwrap()
        .with_releases(vec![MassRelease::new(
            Trigger::Altitude {
                height_above_ground_m: 150.0,
            },
            "ballast",
            0.05,
        )])
        .unwrap();
        let result = sim.run(&mut ()).unwrap();
        let rho = UniformAir::sea_level().0.density_kg_m3;
        let event = result.event(EventKind::MassRelease(0)).unwrap().sample;
        close(event.height_above_ground_m, 150.0, 1e-6, "release height");
        let whole = sim.assembly().mass_properties(event.time_s);
        let rest = sim.mass_properties(&result, event.time_s);
        // Before: the whole rocket's terminal speed under the drogue.
        close(
            event.cg_velocity_enu_m_s.length(),
            terminal_speed_m_s(whole.mass_kg, 0.3, rho, G),
            1e-6,
            "before",
        );
        assert_eq!(result.termination, Termination::GroundHit);
        close(
            result.final_sample.cg_velocity_enu_m_s.length(),
            terminal_speed_m_s(rest.mass_kg, 0.3, rho, G),
            1e-6,
            "the rest",
        );
        let flown = &result.released[0];
        assert_eq!(flown.termination, Termination::GroundHit);
        // It left falling, so it has no apogee of its own.
        assert_eq!(flown.events.len(), 1);
        assert_eq!(flown.events[0].kind, EventKind::GroundHit);
        close(
            flown.final_sample.cg_velocity_enu_m_s.length(),
            terminal_speed_m_s(BALLAST_KG, 0.05, rho, G),
            1e-6,
            "the part",
        );
    }

    #[test]
    fn a_release_comes_at_apogee_and_a_part_let_go_climbing_has_its_own() {
        let sim = |trigger: Trigger| {
            simulation(
                &with_ballast(0.0),
                UniformAir::sea_level(),
                G,
                FlightSettings::default(),
            )
            .with_releases(vec![release(trigger)])
            .unwrap()
        };
        let result = sim(Trigger::Apogee).run(&mut ()).unwrap();
        let apogee = result.event(EventKind::Apogee).unwrap().sample.time_s;
        let released = result.event(EventKind::MassRelease(0)).unwrap().sample;
        assert_eq!(released.time_s, apogee);
        // Let go climbing, 1 s after burnout, the part rises to an apogee of its own.
        let result = sim(Trigger::Burnout {
            motor: 0,
            delay_s: 1.0,
        })
        .run(&mut ())
        .unwrap();
        let flown = &result.released[0];
        assert!(flown.start_sample.vertical_speed_m_s > 0.0);
        let kinds: Vec<EventKind> = flown.events.iter().map(|event| event.kind).collect();
        assert_eq!(kinds, [EventKind::Apogee, EventKind::GroundHit]);
        assert!(flown.events[0].sample.vertical_speed_m_s.abs() < 1e-6);
    }

    /// The `what` and the component of a refused release, or the error when it is another kind.
    fn release_refusal(sim: Simulation, releases: Vec<MassRelease>) -> (&'static str, String) {
        match sim.with_releases(releases) {
            Err(SimError::Release { what, component }) => (what, component),
            other => panic!("{other:?}"),
        }
    }

    /// Flies `rocket` with its design checks' errors accepted: these tests are of the releases.
    fn lenient(rocket: &Rocket) -> Simulation {
        simulation(
            rocket,
            UniformAir::sea_level(),
            G,
            FlightSettings {
                accept_design_errors: true,
                ..FlightSettings::default()
            },
        )
    }

    #[test]
    fn releases_that_cannot_be_made_are_refused() {
        let time = Trigger::Time { time_s: RELEASE_S };
        let of = |id: &str| MassRelease::new(time, id, PART_DRAG_AREA_M2);
        let refused = |releases: Vec<MassRelease>, rocket: &Rocket, starts: &str, id: &str| {
            let (what, component) = release_refusal(lenient(rocket), releases);
            assert!(what.starts_with(starts), "{id}: {what}");
            assert_eq!(component, id);
        };
        let ballasted = with_ballast(0.0);
        refused(
            vec![of("no-such-part")],
            &ballasted,
            "a mass release names a component",
            "no-such-part",
        );
        refused(
            vec![of("sustainer-airframe")],
            &ballasted,
            "a mass release lets go of a part carried inside the airframe, and this is a body",
            "sustainer-airframe",
        );
        refused(
            vec![of("sustainer-rail-buttons")],
            &ballasted,
            "a mass release lets go of a part carried inside the airframe, and this part is \
             outside",
            "sustainer-rail-buttons",
        );
        refused(
            vec![of("sustainer-motor-mount")],
            &ballasted,
            "a mass release of a part that holds a motor",
            "sustainer-motor-mount",
        );
        refused(
            vec![of("ballast"), of("ballast")],
            &ballasted,
            "a mass release of a part released already",
            "ballast",
        );
        // A part inside another that is released; the sleeve could leave on its own.
        lenient(&with_sleeve())
            .with_releases(vec![of("sleeve")])
            .unwrap();
        refused(
            vec![of("sleeve"), of("held")],
            &with_sleeve(),
            "a mass release of a part inside another that is released",
            "held",
        );
        // One of a cluster's copies.
        let mut clustered = with_sleeve();
        let sleeve = clustered.stages[0].components[1]
            .children
            .iter_mut()
            .find(|child| child.id == "sleeve")
            .unwrap();
        let Part::InnerTube(tube) = &mut sleeve.part else {
            panic!("the motor mount is an inner tube");
        };
        tube.cluster_m = vec![[0.004, 0.0], [-0.004, 0.0]];
        refused(
            vec![of("held")],
            &clustered,
            "a mass release of a part that isn't exactly one",
            "held",
        );
        // A stage whose mass is overridden.
        let mut overridden = with_ballast(0.0);
        overridden.stages[0].overrides.mass_kg = Some(1.0);
        refused(
            vec![of("ballast")],
            &overridden,
            "a mass release in a stage whose mass is overridden",
            "ballast",
        );
        // A holder whose overridden mass covers what it holds; one covering itself alone is fine.
        for covers_children in [false, true] {
            let mut overridden = with_ballast(0.0);
            let airframe = &mut overridden.stages[0].components[1];
            airframe.overrides.mass_kg = Some(0.5);
            airframe.overrides_include_children = covers_children;
            let result = lenient(&overridden).with_releases(vec![of("ballast")]);
            if covers_children {
                assert!(
                    matches!(&result, Err(SimError::Release { what, component })
                        if what.starts_with("a mass release inside a component whose overridden")
                            && component == "ballast"),
                    "{result:?}"
                );
            } else {
                result.unwrap();
            }
        }
        // A drag area that is not positive and finite.
        for area in [0.0, -0.1, f64::NAN, f64::INFINITY] {
            let result =
                lenient(&ballasted).with_releases(vec![MassRelease::new(time, "ballast", area)]);
            assert!(
                matches!(&result, Err(SimError::Domain { what, .. })
                    if what.starts_with("drag area of a released part")),
                "{area}: {result:?}"
            );
        }
    }

    #[test]
    fn triggers_a_release_cannot_have_are_refused_in_its_own_words() {
        let domain = |result: Result<Simulation, SimError>| match result {
            Err(SimError::Domain { what, .. }) => what,
            other => panic!("{other:?}"),
        };
        let sim = || lenient(&with_ballast(0.0));
        for (trigger, starts) in [
            (
                Trigger::Altitude {
                    height_above_ground_m: -1.0,
                },
                "height above the launch site at which a part is released",
            ),
            (
                Trigger::Time { time_s: -1.0 },
                "time of a mass release after launch",
            ),
            (
                Trigger::MotorDelay { motor: 3 },
                "index of the motor whose delay releases a part",
            ),
        ] {
            let what = domain(sim().with_releases(vec![release(trigger)]));
            assert!(what.starts_with(starts), "{what}");
        }
        let mut plugged = with_ballast(0.0);
        plugged.configurations[0].motors[0].delay = None;
        let what = domain(
            lenient(&plugged).with_releases(vec![release(Trigger::MotorDelay { motor: 0 })]),
        );
        assert!(
            what.starts_with("the motor whose delay releases a part has no ejection"),
            "{what}"
        );
        let mut failed = with_ballast(0.0);
        failed.configurations[0].motors[0].failed_tubes = vec![0];
        let what = domain(
            lenient(&failed).with_releases(vec![release(Trigger::Burnout {
                motor: 0,
                delay_s: 1.0,
            })]),
        );
        assert!(
            what.starts_with("index of the motor a mass release is timed from"),
            "{what}"
        );
        // A release that would come on the pad or the rail is refused when it comes.
        for time_s in [0.0, 0.1] {
            let error = sim()
                .with_releases(vec![release(Trigger::Time { time_s })])
                .unwrap()
                .run(&mut ())
                .unwrap_err();
            assert!(
                matches!(&error, SimError::Domain { what, value }
                    if what.starts_with("time of a mass release, s (it must come once")
                        && *value == time_s),
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_release_is_refused_with_partings_or_shifts_in_either_order() {
        let time = Trigger::Time { time_s: RELEASE_S };
        let unsupported = |result: Result<Simulation, SimError>| {
            assert!(
                matches!(&result, Err(SimError::Unsupported { what })
                    if what.starts_with("a mass release in a flight with")),
                "{result:?}"
            );
        };
        let sim = || lenient(&with_ballast(0.0));
        let shift = MassShift::new(time, "ballast", 0.1, 1.0);
        let ejection = Ejection::aft_of(Trigger::Apogee, "nose");
        unsupported(
            sim()
                .with_releases(vec![release(time)])
                .unwrap()
                .with_shifts(vec![shift.clone()]),
        );
        unsupported(
            sim()
                .with_shifts(vec![shift])
                .unwrap()
                .with_releases(vec![release(time)]),
        );
        unsupported(
            sim()
                .with_releases(vec![release(time)])
                .unwrap()
                .with_ejections(vec![ejection.clone()]),
        );
        unsupported(
            sim()
                .with_ejections(vec![ejection])
                .unwrap()
                .with_releases(vec![release(time)]),
        );
        unsupported(
            sim()
                .with_releases(vec![release(time)])
                .unwrap()
                .with_separation(Separation::new(Trigger::Apogee, 0)),
        );
        let two_stage = Simulation::new(
            &design("synthetic-two-stage-75mm-54mm"),
            "j760-i175",
            analytic_environment(UniformAir::sea_level(), G),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_separation(Separation::new(Trigger::Apogee, 0))
        .unwrap();
        unsupported(two_stage.with_releases(vec![release(time)]));
    }
}
