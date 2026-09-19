//! Errors from setting up or running a flight.

use hpr_aero::AeroError;
use hpr_atmos::AtmosError;
use hpr_core::CoreError;
use hpr_design::{DesignError, Finding};
use thiserror::Error;

use crate::integrator::{IntegrationError, SettingsError};

/// An error setting up or running a flight.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SimError {
    /// An input outside its domain, such as a negative rail length.
    #[error("{what} is outside its domain: {value}")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The offending value.
        value: f64,
    },
    /// The design's checks found errors, and the settings don't accept them.
    #[error("the design has {} error finding(s); the first is {:?}", .0.len(), .0.first())]
    DesignChecks(Vec<Finding>),
    /// The design can't be assembled.
    #[error(transparent)]
    Design(#[from] DesignError),
    /// The aerodynamic models refused the design or a flow condition (for example Mach 5 or
    /// faster, past the normal force's and the drag buildup's range).
    #[error(transparent)]
    Aero(#[from] AeroError),
    /// The atmosphere or wind model refused a height.
    #[error(transparent)]
    Atmosphere(#[from] AtmosError),
    /// A geodesy, gravity or numerics error.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// Bad integrator settings or initial state.
    #[error(transparent)]
    Settings(#[from] SettingsError),
    /// The integration failed for a reason other than the step limit.
    #[error("the integration failed")]
    Integration(#[source] Box<IntegrationError<SimError>>),
}
