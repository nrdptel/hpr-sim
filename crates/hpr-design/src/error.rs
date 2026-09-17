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
    /// A design tree that doesn't hold together, such as fins attached to an inner tube or an
    /// automatic radius with nothing to take it from.
    #[error("{id}: {message}")]
    Tree {
        /// The id of the stage, component or configuration at fault.
        id: String,
        /// What is wrong.
        message: String,
    },
    /// Two stages, components or configurations share an id, or an id is empty.
    #[error("ids must be unique and non-empty, but `{0}` is not")]
    DuplicateId(String),
    /// A reference to an id that doesn't exist.
    #[error("no {what} has the id `{id}`")]
    UnknownId {
        /// What was looked for.
        what: &'static str,
        /// The id.
        id: String,
    },
    /// A numerical method failed, such as an integral that didn't converge.
    #[error(transparent)]
    Numerics(#[from] CoreError),
}
