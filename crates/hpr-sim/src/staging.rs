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
use hpr_design::{Assembly, Configuration, Ignition, MountedMotor, Rocket};

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
        // One configuration with the nose body's motors, in the stack's order: each mount once,
        // since a cluster's mount places its motor in every tube again, in the same order. Their
        // ignition times come from the stack's (a burnout can name a booster's mount, which the
        // cut has not), so each is written as lit at launch and given its time when the sustainer
        // flies.
        let mut mounts: Vec<MountedMotor> = Vec::new();
        for &index in &motors {
            let mounted = &stack.motors[index].mounted;
            if mounts.iter().all(|m| m.mount != mounted.mount) {
                let mut mounted = mounted.clone();
                mounted.ignition = Ignition::Launch;
                mounts.push(mounted);
            }
        }
        cut.configurations = vec![Configuration {
            id: configuration_id.to_owned(),
            name: String::new(),
            motors: mounts,
        }];
        let assembly = cut.assemble(configuration_id)?;
        // The cut places the same motors in the same order; a failure is kept with its tube.
        debug_assert!(
            assembly.motors.len() == motors.len()
                && assembly.motors.iter().zip(&motors).all(|(cut, &index)| {
                    let stacked = &stack.motors[index];
                    cut.nozzle_m == stacked.nozzle_m
                        && cut.tube == stacked.tube
                        && cut.fails == stacked.fails
                })
        );
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
    use hpr_core::{DQuat, DVec3};

    use crate::testing::{UniformAir, analytic_environment, constant_drag, design, site};

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

    /// A sustainer whose mount is a cluster of two flies on after a powered separation with both
    /// its motors: each lights at the booster's burnout plus its delay and burns out on its own
    /// clock, and the sustainer lands with both spent. (Two I175s aft leave this sustainer
    /// unstable: it is 0.165 rad off the flow when they light and tumbles, so its apogee is no
    /// test of the thrust; `a_cluster_sums_its_motors_thrust_and_mass` is.)
    #[test]
    fn a_clustered_sustainer_flies_on_with_every_motor() {
        clustered_sustainer(&[]);
    }

    /// A sustainer tube that fails to light stays out through the separation: its motor never
    /// lights and lands loaded, while the other tube's flies as before.
    #[test]
    fn a_failed_sustainer_tube_stays_out_after_the_separation() {
        clustered_sustainer(&[1]);
    }

    /// The two-stage rocket with its sustainer mount a pair of tubes (8 mm apart, so they cross:
    /// a timing test, not a buildable rocket), each tube in `failed` never lighting.
    fn clustered_sustainer(failed: &[usize]) {
        let mut rocket = two_stage(Ignition::Burnout {
            mount: BOOSTER_MOUNT.to_owned(),
            delay_s: 1.0,
        });
        rocket.configurations[0]
            .motors
            .iter_mut()
            .find(|m| m.mount == SUSTAINER_MOUNT)
            .unwrap()
            .failed_tubes = failed.to_vec();
        let mount = rocket.stages[0].components[1]
            .children
            .iter_mut()
            .find(|c| c.id == SUSTAINER_MOUNT)
            .unwrap();
        let hpr_design::Part::InnerTube(tube) = &mut mount.part else {
            panic!("an inner tube");
        };
        tube.cluster_m = vec![[-0.004, 0.0], [0.004, 0.0]];
        let separation = |sim: &Simulation| {
            Separation::new(
                Trigger::Burnout {
                    motor: motor_index(sim, BOOSTER_MOUNT),
                    delay_s: 0.5,
                },
                0,
            )
        };
        let sim = staged(&rocket, separation);
        assert_eq!(sim.assembly().motors.len(), 3);
        let result = sim.run(&mut ()).unwrap();
        let burnout_s = booster_burnout_s(&sim);
        let mut ignitions = [Some(0.0); 3];
        for tube in 0..2 {
            let motor = 1 + tube;
            assert_eq!(sim.assembly().motors[motor].tube, tube);
            if failed.contains(&tube) {
                assert!(result.event(EventKind::Ignition(motor)).is_none());
                ignitions[motor] = None;
                continue;
            }
            let lit = time_of(&result, EventKind::Ignition(motor));
            assert!((lit - (burnout_s + 1.0)).abs() < 1e-12, "{motor}: {lit}");
        }
        let i175 = &sim.assembly().motors[1].mounted.motor;
        assert!(
            (time_of(&result, EventKind::Burnout) - (burnout_s + 1.0 + i175.burnout_time_s()))
                .abs()
                < 1e-12
        );
        let spent = super::super::recovery::body_mass_properties(
            sim.assembly(),
            (0, 0),
            f64::MAX,
            &ignitions,
        );
        assert_eq!(result.termination, Termination::GroundHit);
        assert!(
            (result.final_sample.mass_kg - spent.mass_kg).abs() < 1e-12 * spent.mass_kg,
            "{} against {}",
            result.final_sample.mass_kg,
            spent.mass_kg
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
        let rate = stack.state.body_rate_rad_s.length();
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
                Device::new("booster tumble", tumble, BOOSTER_AT_SEPARATION).on_body(1),
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
        assert!(
            matches!(error, SimError::Domain { what, .. } if what.contains("table")),
            "{error:?}"
        );

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
                matches!(error, SimError::Domain { what, .. } if what.contains("after a motor's burnout")),
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
        let nothing_open = |error: &SimError| {
            matches!(error, SimError::Domain { what, value }
                if what.contains("booster falls") && (value - (burnout_s + 0.5)).abs() < 1e-12)
        };
        assert!(nothing_open(&error), "{error:?}");
        // Nor one that opens after the split: any lag leaves a coast with nothing open.
        let rocket_sim = Simulation::new(
            &rocket,
            CONFIGURATION,
            Environment::standard(site()).unwrap(),
            Rail::vertical(6.0),
            FlightSettings::default(),
        )
        .unwrap();
        let tumble = DeviceDrag::tumbling_stages(rocket_sim.assembly(), (1, 1)).unwrap();
        let error = rocket_sim
            .with_recovery(vec![
                Device::new(
                    "sustainer main",
                    DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
                    Trigger::Apogee,
                ),
                Device::new("booster tumble", tumble, BOOSTER_AT_SEPARATION)
                    .with_lag_s(0.1)
                    .on_body(1),
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
            .expect_err("a booster whose device opens after the split");
        assert!(nothing_open(&error), "{error:?}");

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
        let sustainer = rocket
            .assemble(CONFIGURATION)
            .unwrap()
            .motors
            .iter()
            .position(|m| m.mount == SUSTAINER_MOUNT)
            .unwrap();
        assert!(
            matches!(error, SimError::Domain { what, value }
                if what.contains("could never fire") && value == sustainer as f64),
            "{error:?}"
        );

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
        assert!(
            matches!(error, SimError::Domain { what, .. } if what.contains("still has a motor to burn")),
            "{error:?}"
        );
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
        let mut ignition_s = vec![Some(0.0); sim.assembly().motors.len()];
        ignition_s[sustainer] = Some(3.0);
        let expected = sim.assembly().mass_properties_lit(3.0, &ignition_s);
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

    /// The single-stage design with its motor mount a cluster of three tubes on a ring of radius
    /// `A` (at 0°, 120° and 240°) holding its I175 in each, and `failed` the tubes whose motor
    /// fails. The tubes would not fit its airframe; the equations don't ask.
    fn cluster_of_three(failed: Vec<usize>) -> hpr_design::Assembly {
        let mut rocket = design("synthetic-54mm-three-fin");
        let mount = rocket.stages[0].components[1]
            .children
            .iter_mut()
            .find(|c| c.id == SUSTAINER_MOUNT)
            .unwrap();
        let hpr_design::Part::InnerTube(tube) = &mut mount.part else {
            panic!("an inner tube");
        };
        tube.cluster_m = [0.0_f64, 120.0, 240.0]
            .iter()
            .map(|a| [A * a.to_radians().cos(), A * a.to_radians().sin()])
            .collect();
        rocket.configurations[0].motors[0].failed_tubes = failed;
        let id = rocket.configurations[0].id.clone();
        rocket.assemble(&id).unwrap()
    }

    /// The cluster's ring radius, m.
    const A: f64 = 0.02;

    /// The angular acceleration of `assembly` at rest in a vacuum, `t` seconds into the burn, its
    /// motors lit as it says, and the evaluation.
    fn at_rest(assembly: &hpr_design::Assembly, t: f64) -> (DVec3, crate::dynamics::Evaluation) {
        use crate::dynamics::{Conditions, Phase, Vehicle};
        let ignition_s = assembly.ignition_times_s(|_| None);
        let aero = hpr_aero::AeroModel::new(&assembly.layout).unwrap();
        let vehicle = Vehicle::lit(assembly.clone(), aero, ignition_s).unwrap();
        let environment = analytic_environment(UniformAir::vacuum(), 9.806_65);
        let state = crate::state::State {
            position_enu_m: DVec3::new(0.0, 0.0, 1000.0),
            velocity_enu_m_s: DVec3::ZERO,
            attitude: DQuat::IDENTITY,
            body_rate_rad_s: DVec3::ZERO,
        };
        let evaluation = vehicle
            .evaluate(
                &environment,
                0.0,
                Conditions {
                    phase: Phase::Free,
                    window: (t - 0.1, t + 0.1),
                    drag_area_m2: 0.0,
                },
                t,
                &state.to_array(),
            )
            .unwrap();
        (
            DVec3::from_slice(&evaluation.derivative[10..13]),
            evaluation,
        )
    }

    /// The angular acceleration the lit motors' thrust gives `assembly` at rest, by hand: each
    /// lit motor pushes `T ẑ` at its nozzle `pᵢ`, so about the centre of mass `c` the moment is
    /// `M = Σ (pᵢ − c) × T ẑ = T (Σ (yᵢ − c_y), −Σ (xᵢ − c_x), 0)`, and `ω̇ = I_c⁻¹ M`.
    fn by_hand(assembly: &hpr_design::Assembly, t: f64) -> DVec3 {
        let lit = assembly.ignition_times_s(|_| None);
        let mass = assembly.mass_properties_lit(t, &lit);
        let c = mass.cg_m;
        let mut moment = DVec3::ZERO;
        for (placed, ignition) in assembly.motors.iter().zip(&lit) {
            if ignition.is_some() {
                let thrust = placed.mounted.motor.thrust_at_pressure_n(t, 0.0);
                let (x, y) = (placed.nozzle_m.x - c.x, placed.nozzle_m.y - c.y);
                moment += DVec3::new(thrust * y, -thrust * x, 0.0);
            }
        }
        mass.inertia_kg_m2.inverse() * moment
    }

    /// Three motors in one mount sum their thrust and their mass: at rest, 1 s into the burn, the
    /// cluster pushes three times the one motor's thrust and weighs the structure and three
    /// motors. On the ring the thrusts balance, so it turns only as far as its centre of mass sits
    /// off the axis (its rail buttons put it 0.033 mm aside at 1 s): as the hand calculation
    /// says, to the mass-flow terms (below), 3.2e-6 of it here; the check allows 1e-5.
    #[test]
    fn a_cluster_sums_its_motors_thrust_and_mass() {
        let assembly = cluster_of_three(Vec::new());
        assert_eq!(assembly.motors.len(), 3);
        let motor = &assembly.motors[0].mounted.motor;
        let t = 1.0;
        let (omega_dot, evaluation) = at_rest(&assembly, t);
        // In a vacuum the thrust is the curve's plus the exit's full pressure term.
        let one = motor.thrust_at_pressure_n(t, 0.0);
        assert!((evaluation.thrust_n - 3.0 * one).abs() <= 1e-12 * one);
        let want = assembly.layout.structure.mass_kg + 3.0 * motor.state(t).total.mass_kg;
        assert!((evaluation.mass.mass_kg - want).abs() <= 1e-12 * want);
        let want = by_hand(&assembly, t);
        let error = (omega_dot - want).length() / want.length();
        assert!(error <= 1e-5, "{omega_dot:?} vs {want:?}: {error:e}");
    }

    /// A motor out gives the pitch moment the hand calculation predicts (Loft lesson L31, whose
    /// clusters were on the axis only). With the motor at 0° out, the two lit at 120° and 240°
    /// push on the side away from it: `Σ xᵢ = −A`, `Σ yᵢ = 0`, so about the centre of mass `c`
    /// (pulled toward the loaded motor) the moment is `T (−2 c_y, A + 2 c_x, 0)`, a pitch about
    /// `+y` that leans the nose toward the motor out, 225 times the full cluster's; the rocket's
    /// products of inertia (its fins and rail buttons) turn a little of it into roll and yaw
    /// through `I_c⁻¹`. The rest of the equations add the mass-flow terms (the centre's own motion
    /// as two motors burn and one doesn't, and the jets'), which here are 3.7e-7 of it; the check
    /// allows 1e-6.
    #[test]
    fn cluster_motor_out_produces_pitch_moment() {
        let assembly = cluster_of_three(vec![0]);
        let fails: Vec<bool> = assembly.motors.iter().map(|m| m.fails).collect();
        assert_eq!(fails, [true, false, false]);
        let t = 1.0;
        let (omega_dot, evaluation) = at_rest(&assembly, t);
        let motor = &assembly.motors[1].mounted.motor;
        let thrust = motor.thrust_at_pressure_n(t, 0.0);
        assert!((evaluation.thrust_n - 2.0 * thrust).abs() <= 1e-12 * thrust);
        let lit = assembly.ignition_times_s(|_| None);
        let mass = assembly.mass_properties_lit(t, &lit);
        let c = mass.cg_m;
        assert!(c.x > 1e-3, "{c:?}");
        let moment = DVec3::new(-2.0 * thrust * c.y, thrust * (A + 2.0 * c.x), 0.0);
        let want = mass.inertia_kg_m2.inverse() * moment;
        // The numbers the design page works through: 193.98 N, 1.33 mm, 4.396 N m, 41.80 rad/s².
        assert!((thrust - 193.98).abs() < 5e-3 && (c.x - 1.33e-3).abs() < 5e-6);
        assert!((moment.y - 4.396).abs() < 5e-4 && (want.y - 41.80).abs() < 5e-3);
        assert!(((by_hand(&assembly, t) - want).length()) <= 1e-12 * want.length());
        assert!(want.y > 0.0);
        let error = (omega_dot - want).length() / want.length();
        assert!(error <= 1e-6, "{omega_dot:?} vs {want:?}: {error:e}");
        // Mostly a pitch; the full cluster's turn is under a fortieth of it (1/225 measured).
        assert!(omega_dot.y > 50.0 * omega_dot.x.abs().max(omega_dot.z.abs()));
        let (full, _) = at_rest(&cluster_of_three(Vec::new()), t);
        assert!(full.length() < omega_dot.length() / 40.0, "{full:?}");
    }

    #[test]
    fn metrics_follow_a_powered_separation() {
        // The serial plan: the booster drops under power and lands on its own. The summary gives
        // the sustainer's landing and the booster's, and the margins switch to the sustainer's
        // diameter at the split. Only the sustainer's motor has an optimum delay: the booster's
        // charge would fire in the booster, which never reaches the sustainer's apogee.
        let sim = serial_plan();
        let mut metrics = crate::metrics::FlightMetrics::new();
        let result = sim.run(&mut metrics).unwrap();
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        assert!(summary.landing.is_some());
        assert_eq!(summary.body_landings.len(), 1);
        assert_eq!(summary.body_landings[0].body, Some(1));
        let split_s = time_of(&result, EventKind::Separation);
        let series = metrics.stability();
        let before = series.iter().find(|s| s.time_s < split_s).unwrap();
        let after = series.iter().rev().find(|s| s.time_s > split_s).unwrap();
        assert!(after.reference_diameter_m < before.reference_diameter_m);

        let best = crate::metrics::optimum_delays(&sim).unwrap().unwrap();
        let sustainer = motor_index(&sim, SUSTAINER_MOUNT);
        assert_eq!(best.len(), 1);
        assert_eq!(best[0].motor, sustainer);
        let lit_s = booster_burnout_s(&sim) + 1.0;
        let burnout_s = lit_s
            + sim.assembly().motors[sustainer]
                .mounted
                .motor
                .burnout_time_s();
        assert!((best[0].burnout_s - burnout_s).abs() < 1e-9);
        assert!((best[0].delay_s - (best[0].apogee_s - burnout_s)).abs() < 1e-9);
    }

    #[test]
    fn a_held_flight_holds_a_separation_after_the_last_burnout() {
        // The sustainer lights 1 s after the booster burns out, still attached, and the stack
        // separates a delay after the sustainer burns out: part of the recovery. However long
        // the delay, the held flight coasts through to the same apogee.
        let rocket = two_stage(Ignition::Burnout {
            mount: BOOSTER_MOUNT.to_owned(),
            delay_s: 1.0,
        });
        let with_delay = |delay_s: f64| {
            staged(&rocket, |sim| {
                Separation::new(
                    Trigger::Burnout {
                        motor: motor_index(sim, SUSTAINER_MOUNT),
                        delay_s,
                    },
                    0,
                )
            })
        };
        let (short, long) = (with_delay(3.0), with_delay(30.0));
        let flown = short.run(&mut ()).unwrap();
        assert_eq!(flown.termination, Termination::Separated);
        let a = crate::metrics::optimum_delays(&short).unwrap().unwrap();
        let b = crate::metrics::optimum_delays(&long).unwrap().unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 2);
        // Each flight still stops its steps at its own separation time, so the two agree to the
        // integration's tolerance rather than bit for bit: the apogee is where the vertical speed
        // crosses zero at g, so 3e-5 m/s of it is 3e-6 s.
        for (a, b) in a.iter().zip(&b) {
            assert_eq!((a.motor, a.burnout_s), (b.motor, b.burnout_s));
            assert!((a.delay_s - b.delay_s).abs() < 1e-5, "{a:?} vs {b:?}");
        }
        // The short delay's separation came before that apogee.
        assert!(time_of(&flown, EventKind::Separation) < a[0].apogee_s);
    }
}
