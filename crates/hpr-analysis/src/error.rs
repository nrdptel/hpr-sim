//! Errors of the analysis tools.

use hpr_design::DesignError;
use hpr_motor::MotorError;
use hpr_sim::SimError;

/// What an analysis refuses or can't do.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AnalysisError {
    /// A number outside the range it is defined on.
    #[error("{what} out of range: {value}")]
    Domain {
        /// What the number is.
        what: &'static str,
        /// The number.
        value: f64,
    },
    /// A count larger than it can be.
    #[error("{what}: {count} is more than {limit}")]
    Count {
        /// What is counted.
        what: &'static str,
        /// The count.
        count: usize,
        /// Its limit.
        limit: usize,
    },
    /// A list of the wrong length.
    #[error("{what}: {length} given, {expected} expected")]
    Length {
        /// What the list is, and what it is counted against.
        what: &'static str,
        /// Its length.
        length: usize,
        /// The length it should have.
        expected: usize,
    },
    /// A rocket configuration the design doesn't have.
    #[error("no configuration {0:?} in the design")]
    NoConfiguration(String),
    /// Something the analysis has no model for.
    #[error("unsupported: {0}")]
    Unsupported(String),
    /// From the design.
    #[error(transparent)]
    Design(#[from] DesignError),
    /// From a motor.
    #[error(transparent)]
    Motor(#[from] MotorError),
    /// From a flight.
    #[error(transparent)]
    Sim(#[from] SimError),
}
