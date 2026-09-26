//! Staging: what a sustainer flies on after a powered separation.
//!
//! A [`Separation`] cuts the stack after a stage boundary. When the nose's body (body 0) still
//! has a motor to burn at that moment, it is a sustainer and flies on in six degrees of freedom on
//! the models built here: the design cut after the boundary, with the nose body's motors, and its
//! own aerodynamics ([`hpr_aero::AeroModel`] on the cut layout, whose aft end is the sustainer's).
//! The flight's state is the nose tip's, which the sustainer keeps, so it carries across the split
//! unchanged; the mass steps down by the booster's, which descends as a point mass as every body
//! did before (the decision record on separation, [ADR-014][adr-014]). Each motor burns on its own
//! clock from its [`hpr_design::Ignition`], and a motor lit by the separation counts from it (the
//! decision record on staging, [ADR-074][adr-074]).
//!
//! Method: `docs/physics/staging.md`.
//!
//! [adr-014]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-014-separation-bodies-their-masses-and-their-descents-2026-09-17
//! [adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25

use hpr_aero::AeroModel;
use hpr_design::{Assembly, Configuration, Ignition, Rocket};

use crate::error::SimError;
use crate::recovery::Separation;

/// What a sustainer flies on after a powered separation.
#[derive(Debug, Clone)]
pub(crate) struct Sustainer {
    /// The design cut after the separation's boundary, with the nose body's motors.
    pub(crate) assembly: Assembly,
    /// Its aerodynamics.
    pub(crate) aero: AeroModel,
    /// Each of its motors' index among the whole stack's.
    pub(crate) motors: Vec<usize>,
}

impl Sustainer {
    /// The models of `rocket`'s stages `0..=separation.after_stage` with the motors `stack` puts
    /// in them.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if those stages carry no motor (a separation of them isn't powered),
    /// and errors assembling the cut design or building its aerodynamics.
    pub(crate) fn of(
        rocket: &Rocket,
        configuration_id: &str,
        stack: &Assembly,
        separation: Separation,
    ) -> Result<Self, SimError> {
        let motors: Vec<usize> = stack
            .motors
            .iter()
            .enumerate()
            .filter(|(_, motor)| motor.stage <= separation.after_stage)
            .map(|(index, _)| index)
            .collect();
        if motors.is_empty() {
            return Err(SimError::Domain {
                what: "count of motors in a sustainer (a powered separation's nose body has none)",
                value: 0.0,
            });
        }
        let mut cut = rocket.clone();
        cut.stages.truncate(separation.after_stage + 1);
        // One configuration with the nose body's motors, in the stack's order. Their ignition
        // times come from the stack's (a burnout can name a booster's mount, which the cut has
        // not), so each is written as lit at launch and given its time when the sustainer flies.
        cut.configurations = vec![Configuration {
            id: configuration_id.to_owned(),
            name: String::new(),
            motors: motors
                .iter()
                .map(|&index| {
                    let mut mounted = stack.motors[index].mounted.clone();
                    mounted.ignition = Ignition::Launch;
                    mounted
                })
                .collect(),
        }];
        let assembly = cut.assemble(configuration_id)?;
        let aero = AeroModel::new(&assembly.layout)?;
        Ok(Self {
            assembly,
            aero,
            motors,
        })
    }
}

#[cfg(test)]
mod tests {
    use hpr_design::{Ignition, Rocket};

    use crate::environment::Environment;
    use crate::error::SimError;
    use crate::flight::{EventKind, FlightResult, FlightSettings, Simulation, Termination};
    use crate::rail::Rail;
    use crate::recorder::{FlightStep, Observer, Sample};
    use crate::recovery::{CanopyType, Device, DeviceDrag, Separation, Trigger};
    use crate::testing::{constant_drag, design, site};

    /// The synthetic two-stage design's configuration: a J760 in the booster, an I175 in the
    /// sustainer.
    const CONFIGURATION: &str = "j760-i175";
    const BOOSTER_MOUNT: &str = "booster-motor-mount";
    const SUSTAINER_MOUNT: &str = "sustainer-motor-mount";

    /// The two-stage design with the sustainer lit by `ignition`.
    fn two_stage(ignition: Ignition) -> Rocket {
        let mut rocket = design("synthetic-two-stage-75mm-54mm");
        let configuration = rocket
            .configurations
            .iter_mut()
            .find(|c| c.id == CONFIGURATION)
            .unwrap();
        let sustainer = configuration
            .motors
            .iter_mut()
            .find(|m| m.mount == SUSTAINER_MOUNT)
            .unwrap();
        sustainer.ignition = ignition;
        rocket
    }

    /// The index of the motor in `mount` among the assembly's motors.
    fn motor_index(sim: &Simulation, mount: &str) -> usize {
        sim.assembly()
            .motors
            .iter()
            .position(|m| m.mount == mount)
            .unwrap()
    }

    /// The design flown from a 6 m vertical rail in the standard atmosphere: a canopy on the
    /// sustainer at apogee, and the booster tumbling from the separation.
    fn staged(rocket: &Rocket, separation: impl Fn(&Simulation) -> Separation) -> Simulation {
        staged_with(
            rocket,
            Rail::vertical(6.0),
            BOOSTER_AT_SEPARATION,
            separation,
        )
        .unwrap()
    }

    /// A booster's devices act only once it flies, so a time of zero opens it at the separation.
    const BOOSTER_AT_SEPARATION: Trigger = Trigger::Time { time_s: 0.0 };

    /// As [`staged`], from `rail`, with the booster's tumble fired by `booster`.
    fn staged_with(
        rocket: &Rocket,
        rail: Rail,
        booster: Trigger,
        separation: impl Fn(&Simulation) -> Separation,
    ) -> Result<Simulation, SimError> {
        let sim = Simulation::new(
            rocket,
            CONFIGURATION,
            Environment::standard(site()).unwrap(),
            rail,
            FlightSettings::default(),
        )
        .unwrap();
        let tumble = DeviceDrag::tumbling_stages(sim.assembly(), (1, 1)).unwrap();
        let separation = separation(&sim);
        sim.with_recovery(vec![
            Device::new(
                "sustainer main",
                DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
                Trigger::Apogee,
            ),
            Device::new("booster tumble", tumble, booster).on_body(1),
        ])
        .unwrap()
        .with_separation(separation)
    }

    /// The booster's burnout in the flight's time: it lights at launch.
    fn booster_burnout_s(sim: &Simulation) -> f64 {
        sim.assembly().motors[motor_index(sim, BOOSTER_MOUNT)]
            .mounted
            .motor
            .burnout_time_s()
    }

    /// The serial plan: the separation 0.5 s after the booster's burnout, the sustainer lit 1 s
    /// after it, so it coasts half a second on its own before it lights.
    fn serial_plan() -> Simulation {
        let rocket = two_stage(Ignition::Burnout {
            mount: BOOSTER_MOUNT.to_owned(),
            delay_s: 1.0,
        });
        staged(&rocket, |sim| {
            Separation::new(
                Trigger::Burnout {
                    motor: motor_index(sim, BOOSTER_MOUNT),
                    delay_s: 0.5,
                },
                0,
            )
        })
    }

    /// The flight's sample at the start of every accepted step.
    #[derive(Default)]
    struct StepStarts(Vec<Sample>);

    impl Observer for StepStarts {
        fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
            self.0.push(step.sample(step.start_s())?);
            Ok(())
        }
    }

    fn time_of(result: &FlightResult, kind: EventKind) -> f64 {
        result
            .event(kind)
            .unwrap_or_else(|| panic!("no {kind:?} in {:?}", kinds(result)))
            .sample
            .time_s
    }

    fn kinds(result: &FlightResult) -> Vec<EventKind> {
        result.events.iter().map(|event| event.kind).collect()
    }

    #[test]
    fn serial_plan_timing_and_mass_step() {
        // Loft lesson L93: the sustainer lights at the booster's burnout plus its delay, the mass
        // steps down by the booster's at the separation, and a trigger the flight never reaches
        // never lights anything.
        let sim = serial_plan();
        let sustainer = motor_index(&sim, SUSTAINER_MOUNT);
        let mut starts = StepStarts::default();
        let result = sim.run(&mut starts).unwrap();
        let burnout_s = booster_burnout_s(&sim);
        let separation_s = time_of(&result, EventKind::Separation);
        let ignition_s = time_of(&result, EventKind::Ignition(sustainer));
        assert!(
            (separation_s - (burnout_s + 0.5)).abs() < 1e-12,
            "{separation_s}"
        );
        assert!(
            (ignition_s - (burnout_s + 1.0)).abs() < 1e-12,
            "{ignition_s}"
        );

        // The mass steps down by exactly the booster's, and holds until the sustainer lights.
        let stack_kg = result.event(EventKind::Separation).unwrap().sample.mass_kg;
        assert_eq!(result.bodies.len(), 1, "the booster alone descends");
        let booster = &result.bodies[0];
        assert_eq!((booster.body, booster.stages), (1, (1, 1)));
        let sustainer_kg = stack_kg - booster.mass_kg;
        let coasting: Vec<&Sample> = starts
            .0
            .iter()
            .filter(|s| s.time_s >= separation_s && s.time_s < ignition_s)
            .collect();
        assert!(coasting.len() >= 2, "{}", coasting.len());
        for sample in coasting {
            assert!(
                (sample.mass_kg - sustainer_kg).abs() < 1e-12 * stack_kg,
                "{} kg at {} s against {sustainer_kg}",
                sample.mass_kg,
                sample.time_s
            );
        }
        // The booster is a real share: its structure and a spent J760.
        assert!(booster.mass_kg > 0.2 * stack_kg && booster.mass_kg < 0.8 * stack_kg);
        // The sustainer lands with only its own structure and its spent motor.
        let spent = super::super::recovery::body_mass_properties(
            sim.assembly(),
            (0, 0),
            f64::MAX,
            &[Some(0.0), Some(0.0)],
        );
        assert_eq!(result.termination, Termination::GroundHit);
        assert!(
            (result.final_sample.mass_kg - spent.mass_kg).abs() < 1e-12 * spent.mass_kg,
            "{} against {}",
            result.final_sample.mass_kg,
            spent.mass_kg
        );

        // Unreachable: a sustainer lit by a separation that the flight never reaches (timed after
        // it lands) never lights, and is carried to the ground loaded.
        let rocket = two_stage(Ignition::Separation { delay_s: 0.0 });
        let sim = staged(&rocket, |_| {
            Separation::new(Trigger::Time { time_s: 3_000.0 }, 0)
        });
        let result = sim.run(&mut ()).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        assert!(result.event(EventKind::Separation).is_none());
        assert!(result.event(EventKind::Ignition(sustainer)).is_none());
        assert!(result.bodies.is_empty());
        let loaded = sim
            .assembly()
            .mass_properties_lit(f64::MAX, &[Some(0.0), None]);
        assert!(
            (result.final_sample.mass_kg - loaded.mass_kg).abs() < 1e-12 * loaded.mass_kg,
            "{} against {}",
            result.final_sample.mass_kg,
            loaded.mass_kg
        );
    }

    #[test]
    fn apogee_separation_fires_in_flight_and_booster_flies_to_landing() {
        // Loft lesson L30: Loft fixed the separation before the flight, so an apogee or height
        // trigger fell back to the burnout, and it never flew the booster. Here an apogee
        // separation fires at the flight's own apogee, a height one where the flight passes the
        // height, and every booster flies to the ground.
        let rocket = two_stage(Ignition::Launch);
        let sim = staged(&rocket, |_| Separation::new(Trigger::Apogee, 0));
        let result = sim.run(&mut ()).unwrap();
        let apogee_s = time_of(&result, EventKind::Apogee);
        let separation_s = time_of(&result, EventKind::Separation);
        assert_eq!(separation_s, apogee_s);
        assert!(separation_s > time_of(&result, EventKind::Burnout) + 5.0);
        assert_eq!(result.termination, Termination::Separated);
        assert!(result.bodies_landed(), "{:?}", result.bodies);
        assert_eq!(result.landings().len(), 2);

        // A height, on the way down: 300 m, located where the flight crosses it.
        let sim = staged(&rocket, |_| {
            Separation::new(
                Trigger::Altitude {
                    height_above_ground_m: 300.0,
                },
                0,
            )
        });
        let result = sim.run(&mut ()).unwrap();
        let at = result.event(EventKind::Separation).unwrap().sample;
        assert!(
            (at.height_above_ground_m - 300.0).abs() < 1e-6,
            "{}",
            at.height_above_ground_m
        );
        assert!(at.vertical_speed_m_s < 0.0);
        assert!(result.bodies_landed(), "{:?}", result.bodies);

        // Powered: the booster flies to the ground while the sustainer flies on to its own.
        let result = serial_plan().run(&mut ()).unwrap();
        assert_eq!(result.termination, Termination::GroundHit);
        assert!(result.bodies_landed(), "{:?}", result.bodies);
        let landings = result.landings();
        assert_eq!(landings.len(), 2);
        assert!(
            landings
                .iter()
                .all(|l| l.height_above_ground_m.abs() < 1e-6)
        );
        // Two separate landings, not one body counted twice.
        assert!(landings[0].time_s != landings[1].time_s);
    }

    #[test]
    fn staged_events_come_in_order() {
        let sim = serial_plan();
        let sustainer = motor_index(&sim, SUSTAINER_MOUNT);
        let result = sim.run(&mut ()).unwrap();
        let order = [
            EventKind::Liftoff,
            EventKind::RailExit,
            EventKind::Separation,
            EventKind::Ignition(sustainer),
            EventKind::Burnout,
            EventKind::Apogee,
            EventKind::GroundHit,
        ];
        let times: Vec<f64> = order.iter().map(|&kind| time_of(&result, kind)).collect();
        for pair in times.windows(2) {
            assert!(pair[0] < pair[1], "{order:?}: {times:?}");
        }
        // The canopy's charge fires at the apogee and, with no lag, it deploys there too.
        for kind in [EventKind::Trigger(0), EventKind::Deployment(0)] {
            assert_eq!(time_of(&result, kind), time_of(&result, EventKind::Apogee));
        }
        // Every event is in time order, and the burnout is recorded once, at the sustainer's.
        for pair in result.events.windows(2) {
            assert!(pair[0].sample.time_s <= pair[1].sample.time_s);
        }
        let burnouts = result
            .events
            .iter()
            .filter(|e| e.kind == EventKind::Burnout)
            .count();
        assert_eq!(burnouts, 1);
        let i175 = &sim.assembly().motors[sustainer].mounted.motor;
        let expected = time_of(&result, EventKind::Ignition(sustainer)) + i175.burnout_time_s();
        assert!((time_of(&result, EventKind::Burnout) - expected).abs() < 1e-12);
        // No motor lit at launch has an ignition event.
        let booster = motor_index(&sim, BOOSTER_MOUNT);
        assert!(result.event(EventKind::Ignition(booster)).is_none());
        // The booster's own flight starts at the separation and runs in order to the ground.
        let body = &result.bodies[0];
        assert_eq!(
            body.start_sample.time_s,
            time_of(&result, EventKind::Separation)
        );
        for pair in body.events.windows(2) {
            assert!(pair[0].sample.time_s <= pair[1].sample.time_s);
        }
        assert_eq!(body.termination, Termination::GroundHit);

        // A sustainer lit by its separation: the burnout is recorded at the booster's, when
        // nothing else is due, and again at the sustainer's.
        let rocket = two_stage(Ignition::Separation { delay_s: 0.25 });
        let sim = staged(&rocket, |sim| {
            Separation::new(
                Trigger::Burnout {
                    motor: motor_index(sim, BOOSTER_MOUNT),
                    delay_s: 0.5,
                },
                0,
            )
        });
        let result = sim.run(&mut ()).unwrap();
        let burnouts: Vec<f64> = result
            .events
            .iter()
            .filter(|e| e.kind == EventKind::Burnout)
            .map(|e| e.sample.time_s)
            .collect();
        let separation_s = time_of(&result, EventKind::Separation);
        let ignition_s = time_of(&result, EventKind::Ignition(sustainer));
        assert!((ignition_s - (separation_s + 0.25)).abs() < 1e-12);
        assert_eq!(burnouts.len(), 2, "{burnouts:?}");
        assert!((burnouts[0] - booster_burnout_s(&sim)).abs() < 1e-12);
        assert!((burnouts[1] - (ignition_s + i175.burnout_time_s())).abs() < 1e-12);
    }

    #[test]
    fn a_powered_separation_conserves_linear_momentum() {
        // The separation adds no impulse (the decision record on separation, ADR-014): the
        // sustainer keeps the nose tip's motion and the booster leaves with its own centre of
        // mass's velocity, so their momenta add to the stack's. The sustainer is unlit and the
        // booster spent, so no centre of mass moves inside its body. A rail 10° off vertical
        // makes the rocket turn, so the rotation's share, `ω × r`, is not zero.
        let rocket = two_stage(Ignition::Burnout {
            mount: BOOSTER_MOUNT.to_owned(),
            delay_s: 1.0,
        });
        let rail = Rail {
            elevation_rad: 80.0_f64.to_radians(),
            ..Rail::vertical(6.0)
        };
        let sim = staged_with(&rocket, rail, BOOSTER_AT_SEPARATION, |sim| {
            Separation::new(
                Trigger::Burnout {
                    motor: motor_index(sim, BOOSTER_MOUNT),
                    delay_s: 0.5,
                },
                0,
            )
        })
        .unwrap();
        let mut starts = StepStarts::default();
        let result = sim.run(&mut starts).unwrap();
        let stack = result.event(EventKind::Separation).unwrap().sample;
        let booster = &result.bodies[0];
        let sustainer = starts
            .0
            .iter()
            .find(|s| s.time_s == stack.time_s && s.mass_kg < stack.mass_kg)
            .expect("the sustainer's first step");
        let rate = sim
            .run(&mut ())
            .unwrap()
            .event(EventKind::Separation)
            .unwrap()
            .sample
            .state
            .body_rate_rad_s
            .length();
        assert!(rate > 1e-3, "{rate}");
        let before = stack.cg_velocity_enu_m_s * stack.mass_kg;
        let after = sustainer.cg_velocity_enu_m_s * sustainer.mass_kg
            + booster.start_sample.cg_velocity_enu_m_s * booster.mass_kg;
        assert!(
            (after - before).length() < 1e-9 * before.length(),
            "{after:?} against {before:?}"
        );
        // And the masses add up.
        assert!(
            (sustainer.mass_kg + booster.mass_kg - stack.mass_kg).abs() < 1e-12 * stack.mass_kg
        );
    }

    #[test]
    fn staging_refuses_what_it_cannot_fly() {
        // A drag table is the whole stack's; the sustainer would fly it on the wrong shape.
        let rocket = two_stage(Ignition::Burnout {
            mount: BOOSTER_MOUNT.to_owned(),
            delay_s: 1.0,
        });
        let sim = Simulation::new(
            &rocket,
            CONFIGURATION,
            Environment::standard(site()).unwrap(),
            Rail::vertical(6.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_drag_table(constant_drag(0.5));
        let booster = motor_index(&sim, BOOSTER_MOUNT);
        let tumble = DeviceDrag::tumbling_stages(sim.assembly(), (1, 1)).unwrap();
        let sim = sim
            .with_recovery(vec![
                Device::new(
                    "sustainer main",
                    DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
                    Trigger::Apogee,
                ),
                Device::new("booster tumble", tumble, Trigger::Apogee).on_body(1),
            ])
            .unwrap();
        let error = sim
            .with_separation(Separation::new(
                Trigger::Burnout {
                    motor: booster,
                    delay_s: 0.5,
                },
                0,
            ))
            .unwrap()
            .run(&mut ())
            .expect_err("a table flown past a powered separation");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");

        // A separation timed before the booster burns out, and a delay out of its domain.
        for (motor_delay_s, what) in [(-0.5, "negative delay"), (f64::NAN, "delay NaN")] {
            let error = Simulation::new(
                &rocket,
                CONFIGURATION,
                Environment::standard(site()).unwrap(),
                Rail::vertical(6.0),
                FlightSettings::default(),
            )
            .unwrap()
            .with_separation(Separation::new(
                Trigger::Burnout {
                    motor: booster,
                    delay_s: motor_delay_s,
                },
                0,
            ))
            .expect_err(what);
            assert!(
                matches!(error, SimError::Domain { .. }),
                "{what}: {error:?}"
            );
        }
        let error = Simulation::new(
            &rocket,
            CONFIGURATION,
            Environment::standard(site()).unwrap(),
            Rail::vertical(6.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_separation(Separation::new(Trigger::Time { time_s: 0.5 }, 0))
        .expect_err("the booster still burning");
        let burnout_s = rocket.assemble(CONFIGURATION).unwrap().motors[booster]
            .mounted
            .motor
            .burnout_time_s();
        assert!(
            matches!(error, SimError::Domain { value, .. } if value == burnout_s),
            "{error:?}"
        );

        // A booster whose device waits for its own apogee would coast there with no drag.
        let sim = staged_with(&rocket, Rail::vertical(6.0), Trigger::Apogee, |_| {
            Separation::new(
                Trigger::Burnout {
                    motor: booster,
                    delay_s: 0.5,
                },
                0,
            )
        })
        .unwrap();
        let error = sim.run(&mut ()).expect_err("a booster with nothing open");
        assert!(
            matches!(error, SimError::Domain { value, .. } if (value - (burnout_s + 0.5)).abs() < 1e-12),
            "{error:?}"
        );

        // A separation timed from the sustainer it lights could never fire.
        let lit_by_it = two_stage(Ignition::Separation { delay_s: 0.0 });
        let error = staged_with(
            &lit_by_it,
            Rail::vertical(6.0),
            BOOSTER_AT_SEPARATION,
            |sim| {
                Separation::new(
                    Trigger::Burnout {
                        motor: motor_index(sim, SUSTAINER_MOUNT),
                        delay_s: 0.5,
                    },
                    0,
                )
            },
        )
        .expect_err("a separation that could never fire");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");

        // A sustainer with a canopy already open is in the point-mass descent, which can't fly
        // it under thrust.
        let sim = Simulation::new(
            &rocket,
            CONFIGURATION,
            Environment::standard(site()).unwrap(),
            Rail::vertical(6.0),
            FlightSettings::default(),
        )
        .unwrap();
        let tumble = DeviceDrag::tumbling_stages(sim.assembly(), (1, 1)).unwrap();
        let error = sim
            .with_recovery(vec![
                Device::new(
                    "sustainer main",
                    DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
                    Trigger::Time { time_s: 1.0 },
                ),
                Device::new("booster tumble", tumble, BOOSTER_AT_SEPARATION).on_body(1),
            ])
            .unwrap()
            .with_separation(Separation::new(
                Trigger::Burnout {
                    motor: booster,
                    delay_s: 0.5,
                },
                0,
            ))
            .unwrap()
            .run(&mut ())
            .expect_err("a powered separation in the descent");
        assert!(matches!(error, SimError::Domain { .. }), "{error:?}");
    }

    #[test]
    fn an_air_start_lights_at_its_time_and_burns_on_its_own_clock() {
        // No separation: the sustainer lights 3 s after launch with the spent booster still on.
        let rocket = two_stage(Ignition::Time { time_s: 3.0 });
        let sim = Simulation::new(
            &rocket,
            CONFIGURATION,
            Environment::standard(site()).unwrap(),
            Rail::vertical(6.0),
            FlightSettings::default(),
        )
        .unwrap()
        .with_recovery(vec![Device::new(
            "main",
            DeviceDrag::canopy(CanopyType::FlatCircular, 1.5),
            Trigger::Apogee,
        )])
        .unwrap();
        let sustainer = motor_index(&sim, SUSTAINER_MOUNT);
        let result = sim.run(&mut ()).unwrap();
        let lit = result.event(EventKind::Ignition(sustainer)).unwrap().sample;
        assert_eq!(lit.time_s, 3.0);
        // Lit, it is still loaded: the booster spent, the sustainer full.
        let expected = sim
            .assembly()
            .mass_properties_lit(3.0, &[Some(0.0), Some(3.0)]);
        assert!((lit.mass_kg - expected.mass_kg).abs() < 1e-12 * expected.mass_kg);
        let full = sim.assembly().motors[sustainer]
            .mass_properties(0.0)
            .mass_kg;
        let dry = sim.assembly().motors[sustainer]
            .dry_mass_properties()
            .mass_kg;
        let booster_dry = sim.assembly().dry_mass_properties().mass_kg - dry;
        assert!(
            (lit.mass_kg - (booster_dry + full)).abs() < 1e-9,
            "{}",
            lit.mass_kg
        );
        // It burns out on its own clock, and the rocket lands with both motors spent.
        let i175 = &sim.assembly().motors[sustainer].mounted.motor;
        assert!(
            (time_of(&result, EventKind::Burnout) - (3.0 + i175.burnout_time_s())).abs() < 1e-12
        );
        let spent = sim.assembly().dry_mass_properties().mass_kg;
        assert!((result.final_sample.mass_kg - spent).abs() < 1e-12 * spent);
        assert_eq!(result.termination, Termination::GroundHit);
    }
}
