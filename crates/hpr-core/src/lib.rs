//! Core math, units, frames, Earth and gravity models, interpolation tables and shared error types.
//!
//! **Guide:** [Frames and sign conventions][guide-frames], [Geodesy][guide-geodesy],
//! [Gravity][guide-gravity], [Interpolation tables][guide-interpolation] and [Adaptive
//! quadrature][guide-quadrature]: the models, their sources, how well they are validated and what
//! they leave out.
//!
//! [guide-frames]: https://nrdptel.github.io/hpr-sim/physics/frames.html
//! [guide-geodesy]: https://nrdptel.github.io/hpr-sim/physics/geodesy.html
//! [guide-gravity]: https://nrdptel.github.io/hpr-sim/physics/gravity.html
//! [guide-interpolation]: https://nrdptel.github.io/hpr-sim/physics/interpolation.html
//! [guide-quadrature]: https://nrdptel.github.io/hpr-sim/physics/quadrature.html
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
pub mod quadrature;
pub mod random;

pub use error::CoreError;
pub use glam::{DMat3, DQuat, DVec3};
