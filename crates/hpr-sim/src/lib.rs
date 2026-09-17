//! The 6-DOF flight engine: state, launch rail phase, integrators, events, recovery, staging and
//! the recorder.
//!
//! - [`integrator`]: adaptive Dormand–Prince 5(4) with dense output, and fixed-step RK4, advancing
//!   to stop times and events.
//! - [`events`]: event functions, their directions and Brent's root finder.
//!
//! Status: M1.6a covers the integrators and events. The rigid-body flight arrives in M1.6b,
//! recovery in M1.7 and staging in M1.9.
//!
//! Method: `docs/physics/integration.md`.

pub mod events;
pub mod integrator;

pub use events::{Direction, EVENT_TIME_RESOLUTION_S, RootError, find_root};
pub use integrator::{
    Adaptive, Advance, DEFAULT_STEP_LIMIT, IntegrationError, Integrator, Method, OdeSystem,
    SettingsError, Stats, Step,
};

#[cfg(test)]
mod testing;
