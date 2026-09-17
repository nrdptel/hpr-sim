//! Error types for the atmosphere and wind models.

use hpr_core::CoreError;
use thiserror::Error;

/// An error from an atmosphere or wind model: an input outside a model's domain, or a profile
/// that can't be built.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum AtmosError {
    /// An error from a core table or model.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// An input outside a model's domain, such as a negative pressure or a non-finite height.
    #[error("{what} is outside its domain: {value}")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The offending value.
        value: f64,
    },
    /// A profile needs at least `min` levels.
    #[error("a profile needs at least {min} levels, got {got}")]
    TooFewLevels {
        /// The minimum number of levels.
        min: usize,
        /// The number supplied.
        got: usize,
    },
    /// Profile heights must strictly increase.
    #[error("profile heights must strictly increase; level {index} does not")]
    HeightsNotIncreasing {
        /// Index of the first level whose height is not above its predecessor's.
        index: usize,
    },
    /// An optional column (humidity or wind) is given on some levels but not on this one.
    #[error("level {index} has no {column}, but other levels do; give it on every level or none")]
    IncompleteColumn {
        /// Index of the level without the value.
        index: usize,
        /// The column.
        column: &'static str,
    },
    /// The lowest level of a sounding must give its pressure; the levels above may be filled in
    /// hydrostatically.
    #[error("the lowest sounding level must give its pressure")]
    MissingBasePressure,
}

/// Returns `value` if it is finite, or [`AtmosError::Domain`] naming `what`.
pub(crate) fn finite(what: &'static str, value: f64) -> Result<f64, AtmosError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(AtmosError::Domain { what, value })
    }
}

/// Returns `value` if it is finite and strictly positive, or [`AtmosError::Domain`].
pub(crate) fn positive(what: &'static str, value: f64) -> Result<f64, AtmosError> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(AtmosError::Domain { what, value })
    }
}
