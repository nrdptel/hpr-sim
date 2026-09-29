//! A guide to the `hpr` crate, in the API reference: what to read first, and how the pieces fit.
//!
//! This module has no code, only chapters. Each is short, and the code in it is compiled and run
//! by CI, so it stays true. The documentation site says the same things at more length, with the
//! numbers the example programs print; each of the first four chapters links its page.
//!
//! 1. [`building`]: a rocket, part by part from the nose back.
//! 2. [`flying`]: where it flies, from what rail, and what a flight tells you.
//! 3. [`custom_models`]: a drag model, a wind or an atmosphere of your own in hpr's place.
//! 4. [`beneath`]: the crates under the builder, for what it doesn't offer.
//! 5. [`examples`]: the example programs, and what each shows.
//!
//! **How far to trust it.** The builder adds no physics; its numbers are those of the models
//! beneath it. The site's [Accuracy][accuracy] page says how well each has been checked, with
//! every comparison made. Read it before trusting a number.
//!
//! [accuracy]: https://nrdptel.github.io/hpr-sim/accuracy.html

pub mod building {
    //! A rocket, part by part from the nose back.
    //!
    //! [`Rocket::new`](crate::Rocket::new) takes a name and the airframe's outside diameter. The
    //! `add_` methods then stack its parts in the order they sit, from the nose tip aft:
    //!
    //! - [`add_nose`](crate::Rocket::add_nose): the nose cone, first or not at all.
    //! - [`add_tube`](crate::Rocket::add_tube) and
    //!   [`add_transition`](crate::Rocket::add_transition): body tubes, and a change in diameter
    //!   between them.
    //! - [`add_fins`](crate::Rocket::add_fins), [`add_motor_tube`](crate::Rocket::add_motor_tube)
    //!   and [`add_mass`](crate::Rocket::add_mass): parts on or in the last tube added.
    //! - [`set_motor`](crate::Rocket::set_motor): a [`Motor`](crate::Motor) in the motor tube,
    //!   from the built-in catalog or a RASP `.eng` file, the thrust-curve format ThrustCurve.org
    //!   serves.
    //! - [`add_parachute`](crate::Rocket::add_parachute): a recovery device and what opens it.
    //!
    //! Every part names its material ([`material`](crate::rocket::material) finds a built-in one
    //! by id), and a hollow part its wall. There are no defaults for either, since each would be
    //! a guess at the rocket's mass. Each number is checked as the part is added, and a part out
    //! of order is refused with [`Error::Order`](crate::Error::Order).
    //!
    //! Once built, a rocket weighs itself ([`mass_properties`](crate::Rocket::mass_properties))
    //! and finds its stability margin ([`margin`](crate::Rocket::margin)):
    //!
    //! ```
    //! use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
    //! use hpr::{FinPlanform, Motor, NoseShape, Position, Rocket};
    //!
    //! let mut rocket = Rocket::new("Small", 0.0563)?;
    //! rocket
    //!     .add_nose(Nose::hollow(
    //!         NoseShape::Ogive { radius_ratio: 1.0 },
    //!         0.22,
    //!         0.0015,
    //!         material("abs")?,
    //!     ))?
    //!     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
    //!     .add_fins(Fins::new(
    //!         3,
    //!         FinPlanform::Trapezoidal {
    //!             root_chord_m: 0.1,
    //!             tip_chord_m: 0.04,
    //!             span_m: 0.045,
    //!             sweep_m: 0.05,
    //!         },
    //!         0.003175,
    //!         material("birch_plywood")?,
    //!     ))?
    //!     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
    //!     // The parachute and its bay as one 200 g mass, 7 cm below the top of the tube.
    //!     .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?
    //!     .set_motor(Motor::from_catalog("H54")?)?;
    //!
    //! // At liftoff, 0 s after ignition, with the air along the axis at Mach 0.3.
    //! let liftoff = rocket.mass_properties(0.0)?;
    //! let margin = rocket.margin(0.0, 0.3)?;
    //! assert!(liftoff.mass_kg > 0.4);
    //! assert!(margin.margin_cal.is_some_and(|calibres| calibres > 1.0));
    //! # Ok::<(), hpr::Error>(())
    //! ```
    //!
    //! The site's page [The builder][builder] walks through a whole rocket, with a recovery bay
    //! and a parachute.
    //!
    //! [builder]: https://nrdptel.github.io/hpr-sim/the-builder.html
}

pub mod flying {
    //! Where a rocket flies, from what rail, and what a flight tells you.
    //!
    //! An [`Environment`](crate::Environment) is the launch site (latitude, longitude,
    //! elevation), the atmosphere and the wind: the 1976 US Standard Atmosphere and calm air until
    //! you change them.
    //! [`Flight::builder`](crate::Flight::builder) takes the rocket, the environment and the
    //! rail's length; its methods lean the rail
    //! ([`inclination_deg`](crate::FlightBuilder::inclination_deg),
    //! [`heading_deg`](crate::FlightBuilder::heading_deg)) and set the integrator
    //! ([`settings`](crate::FlightBuilder::settings)). [`fly`](crate::FlightBuilder::fly) flies it
    //! to the ground.
    //!
    //! A [`Flight`](crate::Flight) has the numbers most asked for (apogee, top speed, rail exit
    //! speed, landing) as methods, and every metric in its
    //! [`summary`](crate::Flight::summary). Heights are the centre of gravity's, above the
    //! launch site; the rocket starts on the rail, so the first height isn't zero.
    //!
    //! To keep the path, pass an observer to [`fly_with`](crate::FlightBuilder::fly_with): a
    //! [`Recorder`](crate::hpr_sim::Recorder) keeps a row of the channels you ask for at a steady
    //! interval and at every event.
    //!
    //! ```
    //! # use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
    //! # use hpr::{FinPlanform, Motor, NoseShape, Position, Rocket};
    //! # let mut rocket = Rocket::new("Small", 0.0563)?;
    //! # rocket
    //! #     .add_nose(Nose::hollow(NoseShape::Ogive { radius_ratio: 1.0 }, 0.22, 0.0015, material("abs")?))?
    //! #     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
    //! #     .add_fins(Fins::new(
    //! #         3,
    //! #         FinPlanform::Trapezoidal { root_chord_m: 0.1, tip_chord_m: 0.04, span_m: 0.045, sweep_m: 0.05 },
    //! #         0.003175,
    //! #         material("birch_plywood")?,
    //! #     ))?
    //! #     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
    //! #     .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?
    //! #     .set_motor(Motor::from_catalog("H54")?)?;
    //! use hpr::hpr_sim::{Channel, Recorder};
    //! use hpr::{Environment, Flight};
    //!
    //! // Spaceport America, 1,400 m up, with 5 m/s of wind from the west.
    //! let environment = Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(5.0, 270.0)?;
    //! // Height every half second, and at each event.
    //! let mut recorder = Recorder::new(vec![Channel::Time, Channel::HeightAboveGround], Some(0.5))
    //!     .map_err(hpr::Error::Sim)?;
    //! let flight = Flight::builder(&rocket, &environment, 1.8)
    //!     .inclination_deg(85.0)
    //!     .heading_deg(270.0)
    //!     .fly_with(&mut recorder)?;
    //! let highest = recorder.rows().iter().map(|row| row[1]).fold(0.0, f64::max);
    //! assert_eq!(Some(highest), flight.apogee_m());
    //! # Ok::<(), hpr::Error>(())
    //! ```
    //!
    //! The site's [Flight metrics][metrics] page defines each metric, and
    //! [Recording a trajectory][trajectory] each channel.
    //!
    //! [metrics]: https://nrdptel.github.io/hpr-sim/physics/metrics.html
    //! [trajectory]: https://nrdptel.github.io/hpr-sim/recording-a-trajectory.html
}

pub mod custom_models {
    //! A drag model, a wind or an atmosphere of your own, flown in hpr's place.
    //!
    //! Three of hpr's models are traits a program can implement:
    //!
    //! | Trait | What it gives | Where it goes |
    //! | --- | --- | --- |
    //! | [`DragModel`](crate::hpr_aero::DragModel) | the rocket's zero-lift drag coefficient at a flow | [`FlightBuilder::drag_model`](crate::FlightBuilder::drag_model) |
    //! | [`Wind`](crate::hpr_atmos::Wind) | the wind's velocity at a height | [`Environment::with_wind`](crate::Environment::with_wind) |
    //! | [`Atmosphere`](crate::hpr_atmos::Atmosphere) | the air's pressure, temperature and density at a height | [`Environment::with_atmosphere`](crate::Environment::with_atmosphere) |
    //!
    //! A drag model replaces the zero-lift drag only, as a drag table from another tool does. The
    //! flight still scales it for the angle of attack, and the normal force, centre of pressure,
    //! roll and damping stay hpr's, so the margin a rocket reports doesn't change. A model is
    //! asked a [`DragQuery`](crate::hpr_aero::DragQuery): the Mach number and angles, the
    //! Reynolds number, whether a motor burns, and hpr's own drag at that flow
    //! ([`DragQuery::buildup`](crate::hpr_aero::DragQuery::buildup)), so a model can adjust hpr's
    //! number instead of replacing it. The coefficient is on the rocket's reference area, by
    //! default a circle of its largest body diameter, and unlike a drag table's it isn't
    //! rescaled: a curve measured on another area is converted before it is returned
    //! ([`DragQuery::reference_area_m2`](crate::hpr_aero::DragQuery::reference_area_m2)).
    //!
    //! ```
    //! # use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
    //! # use hpr::{FinPlanform, Motor, NoseShape, Position, Rocket};
    //! # let mut rocket = Rocket::new("Small", 0.0563)?;
    //! # rocket
    //! #     .add_nose(Nose::hollow(NoseShape::Ogive { radius_ratio: 1.0 }, 0.22, 0.0015, material("abs")?))?
    //! #     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
    //! #     .add_fins(Fins::new(
    //! #         3,
    //! #         FinPlanform::Trapezoidal { root_chord_m: 0.1, tip_chord_m: 0.04, span_m: 0.045, sweep_m: 0.05 },
    //! #         0.003175,
    //! #         material("birch_plywood")?,
    //! #     ))?
    //! #     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
    //! #     .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?
    //! #     .set_motor(Motor::from_catalog("H54")?)?;
    //! use hpr::hpr_aero::{AeroError, DragModel, DragQuery};
    //! use hpr::{Environment, Flight};
    //!
    //! /// hpr's own drag below Mach 0.5, and 0.6 from there: a made-up rule, to show the idea. It
    //! /// jumps at Mach 0.5, which a real model would smooth.
    //! #[derive(Debug)]
    //! struct Mine;
    //!
    //! impl DragModel for Mine {
    //!     fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
    //!         if query.mach() < 0.5 {
    //!             Ok(query.buildup()?.zero_lift_coefficient)
    //!         } else {
    //!             Ok(0.6)
    //!         }
    //!     }
    //! }
    //!
    //! let environment = Environment::new(32.99, -106.97, 1400.0)?;
    //! let flight = Flight::builder(&rocket, &environment, 1.8)
    //!     .drag_model(Mine)
    //!     .fly()?;
    //! assert!(flight.max_mach().is_some_and(|mach| mach > 0.5));
    //! # Ok::<(), hpr::Error>(())
    //! ```
    //!
    //! **How far to trust it:** as far as the model, and no further than hpr's other models,
    //! which still fly the rest of the rocket and are not yet validated against real flights.
    //! hpr refuses a drag coefficient that is negative or not finite; it can't know whether a
    //! model is right. It doesn't check a wind's value where it reads it: a wind that isn't
    //! finite is caught while the rocket climbs when the drag's Reynolds number comes out
    //! non-finite, so the error names that, not the wind. A model is asked many times a step, so
    //! keep it quick, and give the same answer to the same question: a flight is only as
    //! repeatable as its models.
    //!
    //! The site's page [Models of your own][custom] runs the `custom_drag` and `custom_wind`
    //! examples.
    //!
    //! [custom]: https://nrdptel.github.io/hpr-sim/custom-models.html
}

pub mod beneath {
    //! The crates under the builder, for what it doesn't offer.
    //!
    //! The builder makes the same things the crates below it use, and hands them over:
    //!
    //! - [`Rocket::design`](crate::Rocket::design) is the design tree, an [`hpr_design::Rocket`],
    //!   as a design file holds it. Add what the builder can't (clusters, pods, launch lugs, rail
    //!   buttons, stages) with [`hpr_design`], and make a rocket of it again with
    //!   [`Rocket::from_design`](crate::Rocket::from_design).
    //! - [`FlightBuilder::simulation`](crate::FlightBuilder::simulation) is the
    //!   [`hpr_sim::Simulation`] a flight runs. Its methods add a stage separation, events of your
    //!   own, moving or released masses, or another tool's drag table;
    //!   [`run`](crate::hpr_sim::Simulation::run) flies it.
    //! - [`Environment::sim`](crate::Environment::sim) is the [`hpr_sim::Environment`], and
    //!   [`Environment::from_sim`](crate::Environment::from_sim) wraps one built directly, with a
    //!   geoid undulation, say.
    //!
    //! Every crate of the workspace is re-exported by name: `hpr::hpr_aero` is the aerodynamics,
    //! `hpr::hpr_motor` the motors, `hpr::hpr_io` the file formats, and so on. The site's
    //! [API reference][api] page lists them all.
    //!
    //! [api]: https://nrdptel.github.io/hpr-sim/api.html
}

pub mod examples {
    //! The example programs, and what each shows.
    //!
    //! Each runs with `cargo run --example <name> -p hpr` from anywhere in the repository. What
    //! each prints is committed beside it as `<name>.output.txt`, and CI checks, on macOS,
    //! Windows and Linux, that it still prints exactly that.
    //!
    //! | Example | What it shows |
    //! | --- | --- |
    //! | [`build_and_fly`][build_and_fly] | a rocket built part by part, weighed, and flown in a wind under a parachute |
    //! | [`motor_choice`][motor_choice] | one rocket on each 29 mm motor in the catalog, with the best ejection delay |
    //! | [`fin_sizing`][fin_sizing] | fins of five spans: the margin, the apogee and the drift of each |
    //! | [`custom_drag`][custom_drag] | drag models of your own in place of hpr's |
    //! | [`custom_wind`][custom_wind] | a wind model of your own, which turns with height |
    //! | [`ork_two_stage`][ork_two_stage] | a two-stage OpenRocket file flown through the crates beneath |
    //! | [`fin_flutter`][fin_flutter] | the fin flutter speed along a flight |
    //! | [`era5_weather`][era5_weather] | a flight in a day's weather from an ERA5 file |
    //!
    //! [build_and_fly]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/build_and_fly.rs
    //! [motor_choice]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/motor_choice.rs
    //! [fin_sizing]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/fin_sizing.rs
    //! [custom_drag]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/custom_drag.rs
    //! [custom_wind]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/custom_wind.rs
    //! [ork_two_stage]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/ork_two_stage.rs
    //! [fin_flutter]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/fin_flutter.rs
    //! [era5_weather]: https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/era5_weather.rs
}
