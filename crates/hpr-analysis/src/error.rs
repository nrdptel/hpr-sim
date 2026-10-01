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
    /// Too few of something for what was asked.
    #[error("{what}: {count} given, at least {minimum} needed")]
    TooFew {
        /// What is counted.
        what: &'static str,
        /// The count.
        count: usize,
        /// The fewest that will do.
        minimum: usize,
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
    /// A model's output that isn't a finite number, at the given point of a sensitivity design.
    #[error("the output at point {index} is not finite: {value}")]
    Output {
        /// The point's index in the design's order.
        index: usize,
        /// The output.
        value: f64,
    },
    /// Two sensitivity factors with one name, which would make their results ambiguous.
    #[error("two factors named {0:?}")]
    DuplicateFactor(String),
    /// Two optimization variables with one name, which would make the result ambiguous.
    #[error("two variables named {0:?}")]
    DuplicateVariable(String),
    /// An optimizer's first candidates, of which not one fell inside the variables' bounds in
    /// the draws allowed: the steps are too large for the bounds.
    #[error("no candidate inside the variables' bounds in {draws} draws")]
    OutOfBounds {
        /// How many draws were made.
        draws: usize,
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
