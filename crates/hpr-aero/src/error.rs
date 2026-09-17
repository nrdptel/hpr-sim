//! Error types for the aerodynamic models.

use hpr_core::CoreError;
use hpr_design::DesignError;
use thiserror::Error;

/// An error from the aerodynamic models: an input outside the models' domain, a design they can't
/// describe, or an error from the design itself.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum AeroError {
    /// An input outside its domain, such as a negative span or a non-finite angle.
    #[error("{what} is outside its domain: {value}")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The offending value.
        value: f64,
    },
    /// A Mach number the models don't cover yet. The subsonic models need `0 ≤ M < 1`; transonic
    /// and supersonic flow arrive in M1.8.
    #[error("Mach {mach} is outside the subsonic models' range [0, 1)")]
    Mach {
        /// The Mach number.
        mach: f64,
    },
    /// A part the models have no cited method for, such as tube fins.
    #[error("no aerodynamic model: {0}")]
    Unsupported(String),
    /// A layout that doesn't hold together for the aerodynamic models, such as a fin set without
    /// the radius of its body tube.
    #[error("inconsistent layout: {0}")]
    Layout(String),
    /// An error in one component, with its id.
    #[error("{id}: {source}")]
    InComponent {
        /// The component's id.
        id: String,
        /// The error.
        source: Box<AeroError>,
    },
    /// An error from the design model, such as a profile whose volume integral fails.
    #[error(transparent)]
    Design(#[from] DesignError),
    /// CSV text that doesn't read as a table, with its 1-based line (0 when the text has no
    /// rows).
    #[error("CSV line {line}: {message}")]
    Csv {
        /// The line, counting from 1.
        line: usize,
        /// What is wrong.
        message: String,
    },
    /// A table that doesn't hold together, such as Mach numbers that don't increase.
    #[error(transparent)]
    Table(#[from] CoreError),
}

/// Checks that `value` is finite and positive (or non-negative when `allow_zero`).
pub(crate) fn check_dimension(
    what: &'static str,
    value: f64,
    allow_zero: bool,
) -> Result<(), AeroError> {
    let ok = value.is_finite() && (value > 0.0 || (allow_zero && value == 0.0));
    if ok {
        Ok(())
    } else {
        Err(AeroError::Domain { what, value })
    }
}

/// Checks a subsonic Mach number.
pub(crate) fn check_mach(mach: f64) -> Result<(), AeroError> {
    if mach.is_finite() && (0.0..1.0).contains(&mach) {
        Ok(())
    } else {
        Err(AeroError::Mach { mach })
    }
}
