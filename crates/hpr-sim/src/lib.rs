//! The 6-DOF flight engine: state, launch rail phase, integrators, events, recovery, staging and
//! the recorder.
//!
//! - [`flight`]: a [`Simulation`] of a rocket, its [`Environment`] and its [`Rail`], flown from
//!   ignition through the pad, rail and free-flight phases to the ground.
//! - [`dynamics`]: the rigid-body equations of motion with varying mass and jet damping.
//! - [`rail`]: the rail's geometry and friction, and where a design's guides leave it.
//! - [`recovery`]: recovery devices, their triggers and inflation, the descent under them, and
//!   separation into bodies that each fly to their own landing.
//! - [`recorder`]: the [`Observer`] trait, flight [`Sample`]s and the channel [`Recorder`].
//! - [`integrator`]: adaptive Dormand–Prince 5(4) with dense output, and fixed-step RK4, advancing
//!   to stop times and events.
//! - [`events`]: event directions and Brent's root finder.
//!
//! Status: M1.7 covers the flight from the pad to the ground under parachutes, streamers or
//! tumbling, and a separation whose bodies each land. Staging (M1.9) is not here yet: every motor
//! ignites at `t = 0`, and a separation must follow the last burnout.
//!
//! Method: `docs/physics/flight.md`, `docs/physics/recovery.md` and
//! `docs/physics/integration.md`.

pub mod dynamics;
pub mod environment;
pub mod error;
pub mod events;
pub mod flight;
pub mod integrator;
pub mod rail;
pub mod recorder;
pub mod recovery;
pub mod state;

pub use dynamics::Phase;
pub use environment::Environment;
pub use error::SimError;
pub use events::{Direction, EVENT_TIME_RESOLUTION_S, RootError, find_root};
pub use flight::{
    EventKind, FlightEvent, FlightResult, FlightSettings, Simulation, Termination, UserEvent,
};
pub use integrator::{
    Adaptive, Advance, DEFAULT_STEP_LIMIT, IntegrationError, Integrator, Method, OdeSystem,
    SettingsError, Stats, Step,
};
pub use rail::{Guides, Rail};
pub use recorder::{Channel, FlightStep, Observer, Recorder, Sample};
pub use recovery::{
    BodyEvent, BodyFlight, BodySample, CanopyType, Device, DeviceDrag, Inflation, Separation,
    StreamerModel, Trigger, terminal_speed_m_s,
};
pub use state::{STATE_LEN, State};

#[cfg(test)]
mod testing;

#[cfg(test)]
mod tests;
