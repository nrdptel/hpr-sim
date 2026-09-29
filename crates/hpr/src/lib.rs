//! The hpr-sim library facade: a builder API for environments, motors, rockets and flights, and
//! the workspace's crates re-exported.
//!
//! **Guide:** [Start here][guide-start] says what works and how far to trust it, [Getting
//! started][guide-first] flies a first rocket, and [The builder][guide-builder] walks through
//! this crate's API.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [guide-first]: https://nrdptel.github.io/hpr-sim/getting-started.html
//! [guide-builder]: https://nrdptel.github.io/hpr-sim/the-builder.html
//!
//! Four types make a flight, in the order a program needs them:
//!
//! - [`Environment`]: the launch site, the atmosphere and the wind.
//! - [`Motor`]: a motor from the built-in catalog or a RASP `.eng` file.
//! - [`Rocket`]: parts added from the nose back, the motor, and the recovery devices.
//! - [`Flight`]: the rocket flown from a rail, with its apogee, speeds and landing.
//!
//! ```
//! use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
//! use hpr::{
//!     CanopyType, Device, DeviceDrag, Environment, Flight, Motor, NoseShape, Position, Rocket,
//!     Trigger,
//! };
//!
//! let environment = Environment::new(32.99, -106.97, 1400.0)?;
//! let mut rocket = Rocket::new("Small", 0.0563)?;
//! rocket
//!     .add_nose(Nose::hollow(
//!         NoseShape::Ogive { radius_ratio: 1.0 },
//!         0.22,
//!         0.0015,
//!         material("abs")?,
//!     ))?
//!     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
//!     .add_fins(Fins::trapezoidal(
//!         3,
//!         [0.1, 0.04, 0.045, 0.05],
//!         0.003175,
//!         material("birch_plywood")?,
//!     ))?
//!     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
//!     .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?
//!     .set_motor(Motor::from_catalog("H54")?.with_delay_s(10.0)?)?
//!     .add_parachute(Device::new(
//!         "parachute",
//!         DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
//!         Trigger::MotorDelay { motor: 0 },
//!     ));
//! let flight = Flight::builder(&rocket, &environment, 1.8).fly()?;
//! assert!(flight.apogee_m().is_some_and(|apogee_m| apogee_m > 1000.0));
//! # Ok::<(), hpr::Error>(())
//! ```
//!
//! The builder makes the same things the crates below it use, and hands them over: a rocket's
//! [`Rocket::design`] is an [`hpr_design::Rocket`], a flight's [`FlightBuilder::simulation`] an
//! [`hpr_sim::Simulation`]. For what the builder doesn't offer, such as staging, clusters, pods
//! or mass shifts, build those with the crates, which this one re-exports by name.
//!
//! The `net` feature adds the online data sources from `hpr-net`, and the `parquet` feature
//! turns on `hpr-sim`'s Parquet export (`hpr_sim::export::parquet`). [`ork`] flies the stage
//! separation an OpenRocket `.ork` file describes.

pub mod environment;
pub mod error;
pub mod flight;
pub mod motor;
pub mod ork;
pub mod rocket;

pub use environment::Environment;
pub use error::Error;
pub use flight::{Flight, FlightBuilder};
pub use motor::Motor;
pub use rocket::Rocket;

pub use hpr_aero;
pub use hpr_analysis;
pub use hpr_atmos;
pub use hpr_core;
pub use hpr_design;
pub use hpr_flightdata;
pub use hpr_forensics;
pub use hpr_format;
pub use hpr_io;
pub use hpr_motor;
#[cfg(feature = "net")]
pub use hpr_net;
pub use hpr_sim;

pub use hpr_design::{FinCrossSection, NoseShape, Position};
pub use hpr_sim::{CanopyType, Device, DeviceDrag, Trigger};

#[cfg(test)]
mod tests;
