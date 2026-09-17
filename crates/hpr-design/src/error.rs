//! Error types for the design model.

use hpr_core::CoreError;
use thiserror::Error;

/// An error from the design model: a dimension outside its domain, geometry that contradicts
/// itself, or mass properties no real body could have.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum DesignError {
    /// An input outside its domain, such as a negative length.
    #[error("{what} is outside its domain: {value}")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The offending value.
        value: f64,
    },
    /// Dimensions that contradict each other, such as a wall thicker than the radius.
    #[error("inconsistent geometry: {0}")]
    Geometry(String),
    /// An inertia tensor that no real body has.
    #[error("unphysical inertia: {0}")]
    UnphysicalInertia(String),
    /// A material of the wrong kind for the part, such as a surface density for a solid.
    #[error("{part} needs a {expected} material, but {material} is a {actual} material")]
    MaterialKind {
        /// The part.
        part: &'static str,
        /// The material's name.
        material: String,
        /// The kind the part needs.
        expected: &'static str,
        /// The material's kind.
        actual: &'static str,
    },
    /// A numerical method failed, such as an integral that didn't converge.
    #[error(transparent)]
    Numerics(#[from] CoreError),
}
