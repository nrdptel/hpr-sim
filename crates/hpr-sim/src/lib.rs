//! The 6-DOF flight engine: state, launch rail phase, integrators, events, recovery, separation and
//! the recorder.
//!
//! **Guide:** [How a flight is simulated][guide-flight], [Rigid-body flight][guide-rigid-body],
//! [Time integration][guide-integration] and [Recovery][guide-recovery]: the models, their sources,
//! how well they are validated and what they leave out.
//!
//! [guide-flight]: https://nrdptel.github.io/hpr-sim/how-a-flight-is-simulated.html
//! [guide-rigid-body]: https://nrdptel.github.io/hpr-sim/physics/flight.html
//! [guide-integration]: https://nrdptel.github.io/hpr-sim/physics/integration.html
//! [guide-recovery]: https://nrdptel.github.io/hpr-sim/physics/recovery.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
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
//! Status: the flight from the pad to the ground under parachutes, streamers or tumbling, a
//! separation whose bodies each land, and staging: each motor lights at its own time, and a
//! separation with the nose's body still to burn lets that body fly on as a sustainer while the
//! booster descends ([`flight`]). A cluster flies one motor in each of its tubes, and a tube can
//! be set never to light. A `.ork` file's staging settings and clusters are not flown yet
//! (milestone [M1.9][roadmap] of the roadmap).

pub mod dynamics;
pub mod environment;
pub mod error;
pub mod events;
pub mod flight;
pub mod integrator;
pub mod rail;
pub mod recorder;
pub mod recovery;
mod staging;
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
