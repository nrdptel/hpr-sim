//! Error types for the motor model, its file formats and the catalog.

use thiserror::Error;

/// An error from the motor crate: a value outside a model's domain, a thrust curve or motor that
/// can't be built, or a motor file that can't be read.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum MotorError {
    /// An input outside a model's domain, such as a negative mass or a non-finite time.
    #[error("{what} is outside its domain: {value}")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The offending value.
        value: f64,
    },
    /// Text that doesn't parse as the value it should hold.
    #[error("can't read {what} from {text:?}")]
    Parse {
        /// What the text should hold.
        what: &'static str,
        /// The text.
        text: String,
    },
    /// A thrust curve needs positive total impulse and a positive NFPA 1125 burn time.
    #[error("a thrust curve needs positive total impulse and burn time")]
    NoThrust,
    /// Thrust-curve times must not decrease.
    #[error("thrust-curve times must not decrease; sample {index} at {time_s} s is earlier")]
    TimesDecreasing {
        /// Index of the first sample whose time is before its predecessor's.
        index: usize,
        /// Its time, s.
        time_s: f64,
    },
    /// A motor file is malformed.
    #[error("{format} line {line}: {message}")]
    Syntax {
        /// The file format (`.eng` or `.rse`).
        format: &'static str,
        /// The 1-based line number.
        line: usize,
        /// What is wrong.
        message: String,
    },
    /// A motor catalog index that doesn't deserialize.
    #[error("invalid motor catalog index: {0}")]
    Catalog(String),
    /// Masses or geometry that contradict each other, such as a propellant mass above the total
    /// mass, or grains that don't fit.
    #[error("inconsistent motor: {0}")]
    Inconsistent(String),
}
