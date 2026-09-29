//! The facade's error type.

use hpr_aero::AeroError;
use hpr_atmos::AtmosError;
use hpr_core::CoreError;
use hpr_design::DesignError;
use hpr_motor::MotorError;
use hpr_sim::SimError;
use thiserror::Error;

/// Anything the builder API can refuse: its own checks, and every error of the crates it calls.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// A number outside its domain, such as a negative length or a NaN, found as it was given.
    #[error("{what} is {value}, outside its domain")]
    Domain {
        /// What the number is.
        what: &'static str,
        /// The number.
        value: f64,
    },
    /// Parts added in an order the rocket can't be built in, such as fins before any body tube.
    #[error("{0}")]
    Order(&'static str),
    /// A material id that isn't built in; [`hpr_design::materials`] lists them.
    #[error("no built-in material `{0}` (`hpr_design::materials` lists them)")]
    UnknownMaterial(String),
    /// No motor with a thrust curve in the bundled catalog matches the name.
    #[error("no motor with a bundled thrust curve matches `{0}`")]
    NoSuchMotor(String),
    /// Several different motors match the name; the designations are listed.
    #[error("`{name}` matches several motors: {}", candidates.join(", "))]
    AmbiguousMotor {
        /// The name asked for.
        name: String,
        /// The designations it matches.
        candidates: Vec<String>,
    },
    /// A motor file holds no motor, or more than one where one was expected.
    #[error("the motor file holds {0} motors, not one")]
    MotorCount(usize),
    /// A flight of a rocket with no motor: give it one with [`crate::Rocket::set_motor`].
    #[error("the rocket has no motor; give it one with `Rocket::set_motor`")]
    NoMotor,
    /// A configuration id the design doesn't hold.
    #[error("the design has no configuration `{0}`")]
    NoSuchConfiguration(String),
    /// From the design: its tree, parts and mass properties.
    #[error(transparent)]
    Design(#[from] DesignError),
    /// From the motor models and motor files.
    #[error(transparent)]
    Motor(#[from] MotorError),
    /// From the flight simulation.
    #[error(transparent)]
    Sim(#[from] SimError),
    /// From the aerodynamic model.
    #[error(transparent)]
    Aero(#[from] AeroError),
    /// From the atmosphere and wind models.
    #[error(transparent)]
    Atmos(#[from] AtmosError),
    /// From the geodesy: a launch site that isn't on the Earth.
    #[error(transparent)]
    Core(#[from] CoreError),
}

/// `value` if it is finite and positive, [`Error::Domain`] otherwise.
pub(crate) fn positive(what: &'static str, value: f64) -> Result<f64, Error> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(Error::Domain { what, value })
    }
}

/// `value` if it is finite and not negative, [`Error::Domain`] otherwise.
pub(crate) fn non_negative(what: &'static str, value: f64) -> Result<f64, Error> {
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(Error::Domain { what, value })
    }
}

/// `value` if it is finite, [`Error::Domain`] otherwise.
pub(crate) fn finite(what: &'static str, value: f64) -> Result<f64, Error> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(Error::Domain { what, value })
    }
}
