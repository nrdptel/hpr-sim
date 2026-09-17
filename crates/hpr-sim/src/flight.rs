//! A flight from ignition to the ground: the pad, the rail and free flight, their events, and how
//! the flight ends.
//!
//! [`Simulation::run`] integrates the rigid-body equations (`crate::dynamics`) phase by phase:
//!
//! - **Pad:** the state holds still until the force along the rail exceeds friction (liftoff).
//! - **Rail:** one degree of freedom along the rail until the last guide leaves its top (rail
//!   exit, `crate::rail`). If the rocket stops on the rail it falls back to the pad phase.
//! - **Free:** six degrees of freedom until the ground.
//! - **Descent:** once a recovery device deploys, a point mass under the open devices' drag area
//!   (`crate::recovery`), with the attitude frozen where it deployed.
//!
//! Every thrust-curve knot and every burnout is a stop time, so no step straddles a change in the
//! thrust's slope or the end of a burn, and each interval knows which motors burn. Liftoff, rail
//! exit, apogee (the centre of mass's height rate crossing zero), ground contact (its ellipsoidal
//! height reaching the site's) and user events are located by the integrator's event finder.
//!
//! A flight ends on the ground ([`Termination::GroundHit`]), with no liftoff by the last burnout
//! ([`Termination::NoLiftoff`]), stalled back onto the pad ([`Termination::StalledOnRail`]), at
//! the time cap ([`Termination::TimeCap`]) or at the step limit ([`Termination::StepLimit`]).
//! Anything else that stops it is an error. With no recovery device the flight is ballistic to
//! the ground.
//!
//! Method: `docs/physics/flight.md` and `docs/physics/recovery.md`.

use std::fmt;
use std::ops::ControlFlow;

use hpr_aero::{AeroModel, DragTable};
use hpr_core::DVec3;
use hpr_design::Rocket;
use hpr_design::checks::{check, has_errors};
use serde::{Deserialize, Serialize};

use crate::dynamics::{Conditions, Evaluation, Phase, Vehicle};
use crate::environment::Environment;
use crate::error::SimError;
use crate::events::Direction;
use crate::integrator::{
    Adaptive, Advance, IntegrationError, Integrator, Method, OdeSystem, Stats, Step,
};
use crate::rail::{Guides, Rail};
use crate::recorder::{FlightStep, Observer, Sample};
use crate::recovery::{self, Device, Run, Trigger};
use crate::state::{STATE_LEN, State};

/// How a flight is integrated and when it gives up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FlightSettings {
    /// The integration method and tolerances.
    pub method: Method,
    /// The flight stops with [`Termination::TimeCap`] at this time after ignition, s.
    pub max_time_s: f64,
    /// The flight stops with [`Termination::StepLimit`] after this many attempted steps.
    pub step_limit: u64,
    /// Fly a design whose checks report errors.
    pub accept_design_errors: bool,
}

impl Default for FlightSettings {
    /// Dormand–Prince 5(4) with `rtol = atol = 1e-8` and unit weights (1.1 ms and an apogee
    /// converged to 3e-5 m for a Level 2 flight, `docs/physics/flight.md`), a one-hour time cap, a
    /// limit of 10⁶ steps, and design errors refused.
    fn default() -> Self {
        Self {
            method: Method::DormandPrince54(Adaptive::default()),
            max_time_s: 3600.0,
            step_limit: crate::integrator::DEFAULT_STEP_LIMIT,
            accept_design_errors: false,
        }
    }
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EventKind {
    /// The force along the rail first exceeded friction.
    Liftoff,
    /// The last rail guide left the top of the rail.
    RailExit,
    /// The last motor's thrust curve ended.
    Burnout,
    /// The centre of mass's height rate crossed zero from above.
    Apogee,
    /// The centre of mass reached the launch site's ellipsoidal height, descending.
    GroundHit,
    /// A recovery device's charge fired, by its index in the flight's devices.
    Trigger(usize),
    /// A recovery device deployed (line stretch), its lag after the trigger, by its index. The
    /// first deployment starts the descent phase. A device that was released before its charge
    /// fired never deploys.
    Deployment(usize),
    /// A recovery device was released, by its index: the device that releases it is fully open
    /// from this instant, so the drag area never dips between them.
    Release(usize),
    /// A user event, by its index in the order added.
    User(usize),
}

/// An event and the flight's sample at it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlightEvent {
    /// What happened.
    pub kind: EventKind,
    /// The flight at that instant.
    pub sample: Sample,
}

/// Why a flight ended (Loft lesson L25: every way of stopping is named).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Termination {
    /// The centre of mass reached the ground.
    GroundHit,
    /// Every motor burned out before the rocket lifted off.
    NoLiftoff,
    /// The rocket lifted off but stopped on the rail and every motor has burned out.
    StalledOnRail,
    /// The time cap was reached.
    TimeCap,
    /// The integrator's step limit was reached.
    StepLimit,
}

/// A finished flight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightResult {
    /// Why it ended.
    pub termination: Termination,
    /// Its events, in order.
    pub events: Vec<FlightEvent>,
    /// The flight where it ended.
    pub final_sample: Sample,
    /// The integrator's work.
    pub stats: Stats,
}

impl FlightResult {
    /// The first event of `kind`.
    #[must_use]
    pub fn event(&self, kind: EventKind) -> Option<&FlightEvent> {
        self.events.iter().find(|event| event.kind == kind)
    }
}

/// A user event: `function(sample)` crossing zero in `direction` during free flight. It is
/// recorded and the flight continues.
pub struct UserEvent {
    /// A name for reports.
    pub name: String,
    /// The crossing direction.
    pub direction: Direction,
    /// The event function.
    pub function: Box<dyn Fn(&Sample) -> f64 + Send + Sync>,
}

impl fmt::Debug for UserEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UserEvent")
            .field("name", &self.name)
            .field("direction", &self.direction)
            .finish_non_exhaustive()
    }
}

/// A rocket, its surroundings, its rail and its settings, ready to fly.
#[derive(Debug)]
pub struct Simulation {
    vehicle: Vehicle,
    environment: Environment,
    rail: Rail,
    guides: Guides,
    settings: FlightSettings,
    user_events: Vec<UserEvent>,
    devices: Vec<Device>,
    /// The trigger times known before the flight, one per device: a time after ignition or a
    /// motor's delay after its burnout, and `None` for the triggers the flight watches for.
    trigger_times_s: Vec<Option<f64>>,
}

impl Simulation {
    /// Assembles `rocket` in configuration `configuration_id`, runs its checks and builds its
    /// aerodynamic model.
    ///
    /// # Errors
    ///
    /// [`SimError::DesignChecks`] if the checks report errors and the settings don't accept them;
    /// errors assembling the design or building its aerodynamics; a bad rail, rail geometry or
    /// time cap.
    pub fn new(
        rocket: &Rocket,
        configuration_id: &str,
        environment: Environment,
        rail: Rail,
        settings: FlightSettings,
    ) -> Result<Self, SimError> {
        let findings = check(rocket)?;
        if has_errors(&findings) && !settings.accept_design_errors {
            return Err(SimError::DesignChecks(findings));
        }
        settings.method.validate()?;
        if !(settings.max_time_s.is_finite() && settings.max_time_s > 0.0) {
            return Err(SimError::Domain {
                what: "time cap",
                value: settings.max_time_s,
            });
        }
        rail.validate()?;
        let assembly = rocket.assemble(configuration_id)?;
        let aero = AeroModel::new(&assembly.layout)?;
        let guides = Guides::of(&assembly);
        let travel = guides.exit_travel_m(rail.length_m);
        if travel <= 0.0 {
            return Err(SimError::Domain {
                what: "rail travel to exit (the rail is shorter than the aft end's lead on the last guide)",
                value: travel,
            });
        }
        Ok(Self {
            vehicle: Vehicle::new(assembly, aero)?,
            environment,
            rail,
            guides,
            settings,
            user_events: Vec::new(),
            devices: Vec::new(),
            trigger_times_s: Vec::new(),
        })
    }

    /// Flies another tool's `C_D0(M)` table instead of the drag buildup.
    #[must_use]
    pub fn with_drag_table(mut self, table: DragTable) -> Self {
        self.vehicle.aero = self.vehicle.aero.clone().with_drag_table(table);
        self
    }

    /// Adds a user event, checked during free flight and the descent.
    #[must_use]
    pub fn with_event(mut self, event: UserEvent) -> Self {
        self.user_events.push(event);
        self
    }

    /// Flies with these recovery devices, in the order given: a device's index in this list names
    /// it in [`EventKind`] and in [`crate::recovery::Device::released_by`].
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] for a device whose drag area, lag, inflation, trigger or release index
    /// is outside its domain, or whose trigger names a motor that isn't there or has no ejection
    /// delay in seconds.
    pub fn with_recovery(mut self, devices: Vec<Device>) -> Result<Self, SimError> {
        self.trigger_times_s = recovery::plan(&devices, &self.vehicle.assembly.motors)?;
        self.devices = devices;
        Ok(self)
    }

    /// The recovery devices.
    #[must_use]
    pub fn recovery(&self) -> &[Device] {
        &self.devices
    }

    /// The assembled design.
    #[must_use]
    pub fn assembly(&self) -> &hpr_design::Assembly {
        &self.vehicle.assembly
    }

    /// The aerodynamic model.
    #[must_use]
    pub fn aero(&self) -> &AeroModel {
        &self.vehicle.aero
    }

    /// The rail guides.
    #[must_use]
    pub fn guides(&self) -> Guides {
        self.guides
    }

    /// The rail.
    #[must_use]
    pub fn rail(&self) -> Rail {
        self.rail
    }

    /// The environment.
    #[must_use]
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// The settings.
    #[must_use]
    pub fn settings(&self) -> FlightSettings {
        self.settings
    }

    /// The state at ignition: on the rail, aft end at its foot, at rest.
    #[must_use]
    pub fn initial_state(&self) -> State {
        let attitude = self.rail.attitude();
        State {
            position_enu_m: self.rail.direction_enu() * self.guides.aft_station_m,
            velocity_enu_m_s: DVec3::ZERO,
            attitude,
            body_rate_rad_s: DVec3::ZERO,
        }
    }

    /// Flies from ignition on the pad until the flight ends.
    ///
    /// # Errors
    ///
    /// [`SimError`] from the models or the integrator (other than the step limit, which is a
    /// [`Termination`]), or from the observer.
    pub fn run(&self, observer: &mut dyn Observer) -> Result<FlightResult, SimError> {
        self.fly(0.0, self.initial_state(), Phase::Pad, observer)
    }

    /// Flies freely from `state` at `t0_s` seconds after ignition, as after a rail exit or from a
    /// restart: the motors burn as their curves say at that time.
    ///
    /// # Errors
    ///
    /// As [`Self::run`].
    pub fn run_free(
        &self,
        t0_s: f64,
        state: State,
        observer: &mut dyn Observer,
    ) -> Result<FlightResult, SimError> {
        self.fly(t0_s, state, Phase::Free, observer)
    }

    /// The sample at `(t, y)` in `phase` during the interval `window`, with `drag_area_m2` of
    /// recovery devices open.
    fn sample(
        &self,
        phase: Phase,
        window: (f64, f64),
        t: f64,
        y: &[f64; STATE_LEN],
        drag_area_m2: f64,
    ) -> Result<Sample, SimError> {
        let evaluation = self.evaluate(phase, window, t, y, drag_area_m2)?;
        Ok(sample_of(phase, t, y, &evaluation))
    }

    fn evaluate(
        &self,
        phase: Phase,
        window: (f64, f64),
        t: f64,
        y: &[f64; STATE_LEN],
        drag_area_m2: f64,
    ) -> Result<Evaluation, SimError> {
        self.vehicle.evaluate(
            &self.environment,
            self.rail.friction_coefficient,
            Conditions {
                phase,
                window,
                drag_area_m2,
            },
            t,
            y,
        )
    }

    fn fly(
        &self,
        t0: f64,
        state: State,
        start_phase: Phase,
        observer: &mut dyn Observer,
    ) -> Result<FlightResult, SimError> {
        if !t0.is_finite() || t0 < 0.0 || t0 >= self.settings.max_time_s {
            return Err(SimError::Domain {
                what: "start time (must be from ignition and before the time cap)",
                value: t0,
            });
        }
        if start_phase == Phase::Free {
            let height = self
                .evaluate(Phase::Free, (t0, t0), t0, &state.to_array(), 0.0)?
                .height_above_ground_m;
            if height <= 0.0 {
                return Err(SimError::Domain {
                    what: "starting height of the centre of mass above the ground",
                    value: height,
                });
            }
        }
        let mut integrator = Integrator::new(self.settings.method, t0, state.to_array())?
            .with_step_limit(self.settings.step_limit);
        let cap = self.settings.max_time_s;
        let mut stops = self.vehicle.thrust_knots_s();
        stops.push(cap);
        stops.extend(self.trigger_times_s.iter().flatten().copied());
        stops.retain(|t| *t <= cap);
        stops.sort_by(f64::total_cmp);
        stops.dedup();
        let burnout_s = self.vehicle.burnout_s();
        let rail_origin = state.position_enu_m;
        let exit_travel_m = self.guides.exit_travel_m(self.rail.length_m);

        let mut phase = start_phase;
        let mut lifted = start_phase != Phase::Pad;
        let mut burnout_recorded = t0 >= burnout_s;
        let mut events: Vec<FlightEvent> = Vec::new();
        let mut run = Run::new(self.devices.len());
        let record = |events: &mut Vec<FlightEvent>, observer: &mut dyn Observer, kind, sample| {
            let event = FlightEvent { kind, sample };
            observer.event(&event);
            events.push(event);
        };

        let termination = loop {
            let t = integrator.time_s();
            let mut y = *integrator.state();
            if t >= cap {
                break Termination::TimeCap;
            }

            // Recovery: fire the charges whose time or height has come, then deploy the devices
            // whose lag has run out. A lag of zero deploys in the same pass, and a deployment can
            // release another device, so this repeats until nothing more happens.
            while !self.devices.is_empty() && matches!(phase, Phase::Free | Phase::Descent) {
                let mut again = false;
                let window = (t, next_stop(&stops, t, cap));
                let area = run.drag_area_m2(&self.devices, t);
                // One evaluation serves every device in the pass: they all ask about the same
                // `(t, y)`. It is only made when a pending device needs the flight's state, and
                // it is not one of the integrator's, so `Stats` doesn't count it.
                let mut here: Option<Evaluation> = None;
                for index in 0..self.devices.len() {
                    if !run.pending(index) {
                        continue;
                    }
                    // `plan` gives `trigger_times_s` one entry per device, in order.
                    let fires = match self.devices[index].trigger {
                        Trigger::Time { .. } | Trigger::MotorDelay { .. } => {
                            self.trigger_times_s[index].is_some_and(|time| t >= time)
                        }
                        // Descending: RocketPy's own apogee trigger (`y[5] < 0`), so a flight that
                        // starts past its apogee still deploys. The apogee event fires this too,
                        // and whichever comes first wins.
                        Trigger::Apogee => {
                            let e = self.evaluation(&mut here, phase, window, t, &y, area)?;
                            e.vertical_speed_m_s < 0.0
                        }
                        Trigger::Altitude {
                            height_above_ground_m,
                        } => {
                            // An altimeter's main setting: descending, at or below the height.
                            // A rocket already below it at apogee fires there, as the event on
                            // the height never crosses it (RocketPy's numeric trigger).
                            let e = self.evaluation(&mut here, phase, window, t, &y, area)?;
                            e.vertical_speed_m_s < 0.0
                                && e.height_above_ground_m <= height_above_ground_m
                        }
                    };
                    if fires {
                        let deploy_s = run.trigger(&self.devices, index, t);
                        insert_stop(&mut stops, deploy_s, cap);
                        let sample = self.sample(phase, window, t, &y, area)?;
                        record(&mut events, observer, EventKind::Trigger(index), sample);
                        again = true;
                    }
                }
                for index in 0..self.devices.len() {
                    if !run.waiting(index) || run.deploy_s(index) > t {
                        continue;
                    }
                    if run.released_s(index).is_some_and(|released| released <= t) {
                        // Cut away before its own charge fired: the canopy never flies.
                        run.abandon(index);
                        again = true;
                        continue;
                    }
                    let evaluation = self.evaluation(&mut here, phase, window, t, &y, area)?;
                    let full_s = run.deploy(&self.devices, index, t, evaluation.airspeed_m_s);
                    insert_stop(&mut stops, full_s, cap);
                    if phase != Phase::Descent {
                        // The descent is a point mass: the attitude freezes where it deployed.
                        phase = Phase::Descent;
                        let mut frozen = State::from_array(&y);
                        frozen.body_rate_rad_s = DVec3::ZERO;
                        y = frozen.to_array();
                        integrator.reset(t, y)?;
                        here = None;
                    }
                    let area = run.drag_area_m2(&self.devices, t);
                    let sample = self.sample(phase, window, t, &y, area)?;
                    record(&mut events, observer, EventKind::Deployment(index), sample);
                    again = true;
                }
                // A release happens when the device that releases it is fully open, which is a
                // stop time, so the drag area never dips between a drogue and a filling main.
                for index in 0..self.devices.len() {
                    if run.release_due(index, t) {
                        run.release(index);
                        let area = run.drag_area_m2(&self.devices, t);
                        let sample = self.sample(phase, window, t, &y, area)?;
                        record(&mut events, observer, EventKind::Release(index), sample);
                        again = true;
                    }
                }
                if !again {
                    break;
                }
            }

            let next = next_stop(&stops, t, cap);
            let window = (t, next);
            if phase == Phase::Pad {
                if self.evaluate(Phase::Pad, window, t, &y, 0.0)?.rail_force_n > 0.0 {
                    phase = Phase::Rail;
                    lifted = true;
                    let sample = self.sample(phase, window, t, &y, 0.0)?;
                    record(&mut events, observer, EventKind::Liftoff, sample);
                } else if t >= burnout_s {
                    break if lifted {
                        Termination::StalledOnRail
                    } else {
                        Termination::NoLiftoff
                    };
                }
            }
            let watches = self.watches(phase, &run);
            let mut system = PhaseSystem {
                simulation: self,
                phase,
                window,
                rail_origin,
                exit_travel_m,
                canopies: Canopies {
                    devices: &self.devices,
                    run: &run,
                },
                watches: &watches,
                observer: &mut *observer,
                failure: None,
                cache: None,
            };
            let outcome = integrator.advance(&mut system, next);
            if let Some(error) = system.failure.take() {
                return Err(error);
            }
            let outcome = match outcome {
                Ok(outcome) => outcome,
                Err(IntegrationError::StepLimit { .. }) => break Termination::StepLimit,
                Err(IntegrationError::Derivative { source, .. }) => return Err(source),
                Err(error) => return Err(SimError::Integration(Box::new(error))),
            };
            let t = integrator.time_s();
            let y = *integrator.state();
            let area = run.drag_area_m2(&self.devices, t);
            // Burnout is a stop time, but an event can end the step on it first.
            if !burnout_recorded && t >= burnout_s {
                burnout_recorded = true;
                let sample = self.sample(phase, window, t, &y, area)?;
                record(&mut events, observer, EventKind::Burnout, sample);
            }
            match outcome {
                Advance::Reached => {}
                Advance::Events => {
                    let fired = integrator.fired_events().to_vec();
                    let mut ground = false;
                    let mut next_phase = phase;
                    for index in fired {
                        // `event_count` is `watches.len()`, and the integrator numbers its events
                        // by it, so a fired index is always in range.
                        match watches[index] {
                            Watch::RailExit => {
                                next_phase = Phase::Free;
                                let sample = self.sample(Phase::Free, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::RailExit, sample);
                            }
                            Watch::RailStall => next_phase = Phase::Pad,
                            Watch::RailForce => {
                                next_phase = Phase::Rail;
                                lifted = true;
                                let sample = self.sample(Phase::Rail, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::Liftoff, sample);
                            }
                            Watch::Apogee => {
                                let sample = self.sample(phase, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::Apogee, sample);
                                for device in 0..self.devices.len() {
                                    if self.devices[device].trigger == Trigger::Apogee
                                        && run.pending(device)
                                    {
                                        let deploy_s = run.trigger(&self.devices, device, t);
                                        insert_stop(&mut stops, deploy_s, cap);
                                        record(
                                            &mut events,
                                            observer,
                                            EventKind::Trigger(device),
                                            sample,
                                        );
                                    }
                                }
                            }
                            Watch::Ground => {
                                let sample = self.sample(phase, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::GroundHit, sample);
                                ground = true;
                            }
                            Watch::Altitude(device) => {
                                if run.pending(device) {
                                    let deploy_s = run.trigger(&self.devices, device, t);
                                    insert_stop(&mut stops, deploy_s, cap);
                                    let sample = self.sample(phase, window, t, &y, area)?;
                                    record(
                                        &mut events,
                                        observer,
                                        EventKind::Trigger(device),
                                        sample,
                                    );
                                }
                            }
                            Watch::User(user) => {
                                let sample = self.sample(phase, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::User(user), sample);
                            }
                        }
                    }
                    if ground {
                        break Termination::GroundHit;
                    }
                    if next_phase == Phase::Pad && phase == Phase::Rail {
                        // Stopped on the rail: at rest where it stopped.
                        let mut at_rest = State::from_array(&y);
                        at_rest.velocity_enu_m_s = DVec3::ZERO;
                        integrator.reset(t, at_rest.to_array())?;
                    }
                    phase = next_phase;
                }
                _ => {}
            }
        };

        let t = integrator.time_s();
        let y = *integrator.state();
        let next = next_stop(&stops, t, f64::INFINITY);
        let area = run.drag_area_m2(&self.devices, t);
        let final_sample = self.sample(phase, (t, next.max(t)), t, &y, area)?;
        Ok(FlightResult {
            termination,
            events,
            final_sample,
            stats: integrator.stats(),
        })
    }

    /// The evaluation at `(t, y)` for the recovery scan, made once per pass and reused.
    fn evaluation(
        &self,
        cache: &mut Option<Evaluation>,
        phase: Phase,
        window: (f64, f64),
        t: f64,
        y: &[f64; STATE_LEN],
        drag_area_m2: f64,
    ) -> Result<Evaluation, SimError> {
        if let Some(evaluation) = cache {
            return Ok(*evaluation);
        }
        let evaluation = self.evaluate(phase, window, t, y, drag_area_m2)?;
        *cache = Some(evaluation);
        Ok(evaluation)
    }

    /// What the integrator watches for in `phase`, in event order.
    fn watches(&self, phase: Phase, run: &Run) -> Vec<Watch> {
        match phase {
            Phase::Pad => vec![Watch::RailForce],
            Phase::Rail => vec![Watch::RailExit, Watch::RailStall],
            Phase::Free | Phase::Descent => {
                let mut watches = vec![Watch::Apogee, Watch::Ground];
                for (index, device) in self.devices.iter().enumerate() {
                    if matches!(device.trigger, Trigger::Altitude { .. }) && run.pending(index) {
                        watches.push(Watch::Altitude(index));
                    }
                }
                watches.extend((0..self.user_events.len()).map(Watch::User));
                watches
            }
        }
    }
}

/// The first stop after `t`, or `cap`.
fn next_stop(stops: &[f64], t: f64, cap: f64) -> f64 {
    stops.iter().copied().find(|stop| *stop > t).unwrap_or(cap)
}

/// Adds `t` to the sorted stop times, unless it is past `cap` or already there.
fn insert_stop(stops: &mut Vec<f64>, t: f64, cap: f64) {
    if !t.is_finite() || t > cap {
        return;
    }
    match stops.binary_search_by(|stop| stop.total_cmp(&t)) {
        Ok(_) => {}
        Err(index) => stops.insert(index, t),
    }
}

/// What the integrator watches for, in the order the events are numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Watch {
    /// The pad's force margin rising through zero: liftoff.
    RailForce,
    /// The travel along the rail reaching the exit travel.
    RailExit,
    /// The speed along the rail falling through zero: a stall.
    RailStall,
    /// The centre of mass's height rate falling through zero: apogee.
    Apogee,
    /// The centre of mass reaching the ground, descending.
    Ground,
    /// A device's deployment height, descending.
    Altitude(usize),
    /// A user event.
    User(usize),
}

/// The recovery devices of a flight and their progress through it.
#[derive(Debug, Clone, Copy)]
struct Canopies<'a> {
    devices: &'a [Device],
    run: &'a Run,
}

impl Canopies<'_> {
    /// The open devices' drag area at `t`, m².
    fn drag_area_m2(&self, t: f64) -> f64 {
        self.run.drag_area_m2(self.devices, t)
    }
}

fn sample_of(phase: Phase, t: f64, y: &[f64; STATE_LEN], e: &Evaluation) -> Sample {
    Sample {
        time_s: t,
        phase,
        state: State::from_array(y),
        cg_enu_m: e.cg_enu_m,
        cg_velocity_enu_m_s: e.cg_velocity_enu_m_s,
        height_above_ground_m: e.height_above_ground_m,
        vertical_speed_m_s: e.vertical_speed_m_s,
        acceleration_enu_m_s2: e.acceleration_enu_m_s2,
        airspeed_m_s: e.airspeed_m_s,
        mach: e.mach,
        angle_of_attack_rad: e.angle_of_attack_rad,
        dynamic_pressure_pa: e.dynamic_pressure_pa,
        axial_coefficient: e.axial_coefficient,
        thrust_n: e.thrust_n,
        mass_kg: e.mass.mass_kg,
        recovery_drag_area_m2: e.recovery_drag_area_m2,
    }
}

/// One phase over one interval, as the integrator sees it.
struct PhaseSystem<'a> {
    simulation: &'a Simulation,
    phase: Phase,
    window: (f64, f64),
    rail_origin: DVec3,
    exit_travel_m: f64,
    canopies: Canopies<'a>,
    watches: &'a [Watch],
    observer: &'a mut dyn Observer,
    /// An error from an event function or the observer, returned after the integrator stops.
    failure: Option<SimError>,
    /// The last evaluation: the integrator evaluates the derivative at a step's end before the
    /// events there.
    cache: Option<(f64, [f64; STATE_LEN], Evaluation)>,
}

impl PhaseSystem<'_> {
    fn evaluation(&mut self, t: f64, y: &[f64; STATE_LEN]) -> Result<Evaluation, SimError> {
        if let Some((ct, cy, evaluation)) = &self.cache
            && *ct == t
            && cy == y
        {
            return Ok(*evaluation);
        }
        let evaluation = self.simulation.evaluate(
            self.phase,
            self.window,
            t,
            y,
            self.canopies.drag_area_m2(t),
        )?;
        self.cache = Some((t, *y, evaluation));
        Ok(evaluation)
    }

    /// An event value, or NaN with the error kept for the caller.
    fn or_fail(&mut self, value: Result<f64, SimError>) -> f64 {
        match value {
            Ok(value) => value,
            Err(error) => {
                self.failure.get_or_insert(error);
                f64::NAN
            }
        }
    }
}

impl OdeSystem<STATE_LEN> for PhaseSystem<'_> {
    type Error = SimError;

    fn derivative(&mut self, t_s: f64, y: &[f64; STATE_LEN]) -> Result<[f64; STATE_LEN], SimError> {
        self.evaluation(t_s, y).map(|e| e.derivative)
    }

    fn event_count(&self) -> usize {
        self.watches.len()
    }

    fn event_direction(&self, index: usize) -> Direction {
        match self.watches.get(index) {
            Some(Watch::RailForce | Watch::RailExit) => Direction::Rising,
            Some(Watch::RailStall | Watch::Apogee | Watch::Ground | Watch::Altitude(_)) => {
                Direction::Falling
            }
            Some(Watch::User(user)) => self
                .simulation
                .user_events
                .get(*user)
                .map_or(Direction::Either, |event| event.direction),
            None => Direction::Either,
        }
    }

    fn event_value(&mut self, index: usize, t_s: f64, y: &[f64; STATE_LEN]) -> f64 {
        let state = State::from_array(y);
        let Some(watch) = self.watches.get(index).copied() else {
            return f64::NAN;
        };
        match watch {
            Watch::RailExit => {
                let along = self.simulation.rail.direction_enu();
                (state.position_enu_m - self.rail_origin).dot(along) - self.exit_travel_m
            }
            Watch::RailStall => state
                .velocity_enu_m_s
                .dot(self.simulation.rail.direction_enu()),
            Watch::Apogee => {
                let value = self.evaluation(t_s, y).map(|e| e.vertical_speed_m_s);
                self.or_fail(value)
            }
            Watch::Ground => {
                let value = self.evaluation(t_s, y).map(|e| e.height_above_ground_m);
                self.or_fail(value)
            }
            Watch::Altitude(device) => {
                let height_m = match self.simulation.devices.get(device).map(|d| d.trigger) {
                    Some(Trigger::Altitude {
                        height_above_ground_m,
                    }) => height_above_ground_m,
                    _ => return f64::NAN,
                };
                let value = self
                    .evaluation(t_s, y)
                    .map(|e| e.height_above_ground_m - height_m);
                self.or_fail(value)
            }
            Watch::User(user) => {
                let value = self
                    .evaluation(t_s, y)
                    .map(|e| sample_of(self.phase, t_s, y, &e));
                match value {
                    Ok(sample) => self
                        .simulation
                        .user_events
                        .get(user)
                        .map_or(f64::NAN, |event| (event.function)(&sample)),
                    Err(error) => self.or_fail(Err(error)),
                }
            }
            Watch::RailForce => {
                let value = self.evaluation(t_s, y).map(|e| e.rail_force_n);
                self.or_fail(value)
            }
        }
    }

    fn accept_step(&mut self, step: &Step<STATE_LEN>) -> ControlFlow<()> {
        let view = StepView {
            simulation: self.simulation,
            phase: self.phase,
            window: self.window,
            canopies: self.canopies,
            step,
        };
        match self.observer.step(&view) {
            Ok(()) => ControlFlow::Continue(()),
            Err(error) => {
                self.failure.get_or_insert(error);
                ControlFlow::Break(())
            }
        }
    }
}

/// An accepted step as the observer sees it.
struct StepView<'a> {
    simulation: &'a Simulation,
    phase: Phase,
    window: (f64, f64),
    canopies: Canopies<'a>,
    step: &'a Step<STATE_LEN>,
}

impl FlightStep for StepView<'_> {
    fn phase(&self) -> Phase {
        self.phase
    }

    fn start_s(&self) -> f64 {
        self.step.start_s()
    }

    fn end_s(&self) -> f64 {
        self.step.end_s()
    }

    fn state_at(&self, t_s: f64) -> State {
        State::from_array(&self.step.state_at(t_s))
    }

    fn sample(&self, t_s: f64) -> Result<Sample, SimError> {
        self.simulation.sample(
            self.phase,
            self.window,
            t_s,
            &self.step.state_at(t_s),
            self.canopies.drag_area_m2(t_s),
        )
    }
}
