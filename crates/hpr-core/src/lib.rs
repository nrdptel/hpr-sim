//! Core math, units, frames, Earth and gravity models, interpolation tables and shared error types.
//!
//! Vectors, quaternions and matrices are glam's `f64` types, re-exported here so every crate uses
//! the same ones. Quantities are SI; frames and sign conventions follow `docs/physics/frames.md`.

pub mod attitude;
pub mod earth;
pub mod error;
pub mod frames;
pub mod geodesy;
pub mod gravity;
pub mod interp;

pub use error::CoreError;
pub use glam::{DMat3, DQuat, DVec3};
