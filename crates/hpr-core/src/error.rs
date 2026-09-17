//! Error types shared by the core crate.

use thiserror::Error;

/// An error from a core model: a table that can't be built, a lookup it refuses, or an input
/// outside a model's domain.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum CoreError {
    /// A table needs at least `min` knots.
    #[error("a table needs at least {min} knots, got {got}")]
    TableTooShort {
        /// The minimum number of knots.
        min: usize,
        /// The number supplied.
        got: usize,
    },
    /// The abscissa and ordinate columns have different lengths.
    #[error("table columns differ in length: {xs} abscissae, {ys} ordinates")]
    TableLengthMismatch {
        /// Number of abscissae.
        xs: usize,
        /// Number of ordinates.
        ys: usize,
    },
    /// A table value is NaN or infinite.
    #[error("table value at index {index} is not finite")]
    TableNotFinite {
        /// Index of the offending knot.
        index: usize,
    },
    /// Abscissae must strictly increase.
    #[error("table abscissae must strictly increase; index {index} does not")]
    TableNotIncreasing {
        /// Index of the first abscissa that is not greater than its predecessor.
        index: usize,
    },
    /// A lookup outside the table, on a table whose policy is [`crate::interp::Extrapolation::Error`].
    #[error("lookup at {x} is outside the table range [{min}, {max}]")]
    OutOfRange {
        /// The requested abscissa.
        x: f64,
        /// The first abscissa.
        min: f64,
        /// The last abscissa.
        max: f64,
    },
    /// A lookup at NaN.
    #[error("lookup at NaN")]
    NanLookup,
    /// An input outside a model's domain, such as a latitude beyond ±90°.
    #[error("{what} is outside its domain: {value}")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The offending value.
        value: f64,
    },
}
