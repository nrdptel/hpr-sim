//! Error types for the atmosphere and wind models.

use thiserror::Error;

/// An error from an atmosphere or wind model: an input outside a model's domain, or a profile
/// that can't be built.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum AtmosError {
    /// An input outside a model's domain, such as a negative pressure or a non-finite height.
    #[error("{what} is outside its domain: {value}")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The offending value.
        value: f64,
    },
    /// A sounding profile or a layered wind was given no levels.
    #[error("a profile or wind table needs at least one level")]
    NoLevels,
    /// A gust field has no samples.
    #[error("a gust field needs at least one sample")]
    EmptyGustField,
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
    /// A sounding level's pressure is not below the pressure of the level beneath it. Pressure
    /// must fall with height; a pressure in hPa given as Pa is the usual cause.
    #[error(
        "sounding level {index} has pressure {pressure_pa} Pa, not below {below_pa} Pa beneath it"
    )]
    PressureNotDecreasing {
        /// Index of the level.
        index: usize,
        /// Its pressure, Pa.
        pressure_pa: f64,
        /// The pressure of the level below, given or filled in, Pa.
        below_pa: f64,
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
