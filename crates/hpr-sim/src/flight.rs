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
//! Every ignition, thrust-curve knot and burnout is a stop time, so no step straddles a change in
//! the thrust's slope or the start or end of a burn, and each interval knows which motors burn.
//! Each motor burns on its own clock from its ignition ([`hpr_design::Ignition`]).
//!
//! **Staging.** A [`Separation`] whose nose body still has a motor to burn is powered: that body,
//! the sustainer, flies on in six degrees of freedom on its own stages' aerodynamics and mass, and
//! the aft body, the booster, descends to its landing as a point mass ([`FlightResult::bodies`]).
//! The state is the nose tip's, which the sustainer keeps, so it carries straight across the split
//! (the decision record on staging, [ADR-074][adr-074]). Liftoff, rail
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
//!
//! [adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25

use std::fmt;
use std::ops::ControlFlow;

use hpr_aero::{AeroModel, DragTable, NormalForceTable};
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
use crate::metrics::Stability;
use crate::pieces::{Ejection, Pieces};
use crate::rail::{Guides, Rail};
use crate::recorder::{FlightStep, Observer, Sample};
use crate::recovery::{self, BodyEvent, BodyFlight, BodySample, Device, Run, Separation, Trigger};
use crate::shifts::{MassShift, Shifts};
use crate::staging::Sustainer;
use crate::state::{STATE_LEN, State};

/// The airspeed below which a body with no attitude of its own is taken to be still in the air, m/s:
/// the way an ejection's push points is then up rather than along its drift (ADR-086).
const STILL_AIR_M_S: f64 = 1e-3;

/// How a flight is integrated and when it gives up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FlightSettings {
    /// The integration method and tolerances.
    pub method: Method,
    /// The flight stops with [`Termination::TimeCap`] at this time after launch, s.
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
    /// Every motor lit, or due to light at a known time, has burned out: the last motor's thrust
    /// curve ended. It is recorded again if a motor lights afterwards, as a sustainer lit by its
    /// separation does.
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
    /// The stack came apart at its separation's stage boundary; the descents of the bodies that
    /// don't fly on are in [`FlightResult::bodies`].
    Separation,
    /// A piece left the airframe at an ejection, by its index in the flight's ejections
    /// ([`crate::Ejection`]); the bodies it leaves are in [`FlightResult::bodies`].
    Ejection(usize),
    /// A part started to move along the airframe, by its index in the flight's mass shifts
    /// ([`crate::MassShift`]).
    Shift(usize),
    /// A user event, by its index in the order added.
    User(usize),
    /// A motor lit after launch, by its index in [`hpr_design::Assembly::motors`].
    Ignition(usize),
}

/// An event and the flight's sample at it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FlightEvent {
    /// What happened.
    pub kind: EventKind,
    /// The flight at that instant.
    pub sample: Sample,
}

/// Why a flight ended ([Loft lesson L25][l25]: every way of stopping is named).
///
/// [l25]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l25
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
    /// The stack separated, and each body flew on as its own descent
    /// ([`FlightResult::bodies`]).
    Separated,
}

/// A finished flight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightResult {
    /// Why it ended.
    pub termination: Termination,
    /// Its events, in order.
    pub events: Vec<FlightEvent>,
    /// The flight where it ended. After a [`Termination::Separated`] this is the **stack** at the
    /// separation, not a landing: the landings are in `bodies` (see [`Self::landings`]). After a
    /// powered separation it is the sustainer's.
    pub final_sample: Sample,
    /// The integrator's work.
    pub stats: Stats,
    /// The descents of the separated bodies, in body order: every body that flew on its own when
    /// the flight ended with [`Termination::Separated`] (pieces whose parting never fired land as
    /// one body), the booster alone after a powered separation (the sustainer's
    /// flight is the rest of this result), and none otherwise.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bodies: Vec<BodyFlight>,
}

impl FlightResult {
    /// Whether every separated body in [`Self::bodies`] landed. A flight that ends with
    /// [`Termination::Separated`] says only that the stack came apart: each body's own
    /// [`BodyFlight::termination`] says whether it reached the ground, and a body can run out of
    /// time or steps on its own. After a powered separation the sustainer's own landing is
    /// [`Self::termination`].
    #[must_use]
    pub fn bodies_landed(&self) -> bool {
        !self.bodies.is_empty()
            && self
                .bodies
                .iter()
                .all(|body| body.termination == Termination::GroundHit)
    }

    /// Where the flight put things on the ground: the final sample's position when it (or after a
    /// powered separation, the sustainer) landed, then each separated body's landing.
    #[must_use]
    pub fn landings(&self) -> Vec<BodySample> {
        let bodies = self
            .bodies
            .iter()
            .filter(|body| body.termination == Termination::GroundHit)
            .map(|body| body.final_sample);
        if self.termination == Termination::GroundHit {
            std::iter::once(BodySample {
                time_s: self.final_sample.time_s,
                cg_enu_m: self.final_sample.cg_enu_m,
                cg_velocity_enu_m_s: self.final_sample.cg_velocity_enu_m_s,
                height_above_ground_m: self.final_sample.height_above_ground_m,
                vertical_speed_m_s: self.final_sample.vertical_speed_m_s,
                airspeed_m_s: self.final_sample.airspeed_m_s,
                recovery_drag_area_m2: self.final_sample.recovery_drag_area_m2,
                mass_kg: self.final_sample.mass_kg,
            })
            .chain(bodies)
            .collect()
        } else {
            bodies.collect()
        }
    }

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
    /// The trigger times known before the flight, one per device: a time after launch or a
    /// motor's delay after its burnout, and `None` for the triggers the flight watches for.
    trigger_times_s: Vec<Option<f64>>,
    separation: Option<Separation>,
    /// The separation's trigger time, when it is one that is known before the flight.
    separation_time_s: Option<f64>,
    /// The pieces that leave the airframe, and when.
    ejections: Vec<Ejection>,
    /// Each ejection's trigger time, when it is one that is known before the flight.
    ejection_times_s: Vec<Option<f64>>,
    /// The parts that move along the airframe, in the order given.
    shifts: Vec<MassShift>,
    /// The design, kept to build a sustainer's models from at a powered separation.
    rocket: Rocket,
    /// The configuration flown.
    configuration_id: String,
    /// Whether the aerodynamics were replaced by a table, which is the whole stack's only.
    aero_overridden: bool,
    /// Whether the recovery charges are held: no device on the stack fires, whatever its trigger
    /// ([`crate::metrics::optimum_delays`] flies the ascent so).
    recovery_held: bool,
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
        // No separation yet, so a motor lit by one has no time.
        let ignition_s = assembly.ignition_times_s(|_| None);
        let guides = Guides::of(&assembly);
        let travel = guides.exit_travel_m(rail.length_m);
        if travel <= 0.0 {
            return Err(SimError::Domain {
                what: "rail travel to exit (the rail is shorter than the aft end's lead on the last guide)",
                value: travel,
            });
        }
        Ok(Self {
            vehicle: Vehicle::lit(assembly, aero, ignition_s)?,
            environment,
            rail,
            guides,
            settings,
            user_events: Vec::new(),
            devices: Vec::new(),
            trigger_times_s: Vec::new(),
            separation: None,
            separation_time_s: None,
            ejections: Vec::new(),
            ejection_times_s: Vec::new(),
            shifts: Vec::new(),
            rocket: rocket.clone(),
            configuration_id: configuration_id.to_owned(),
            aero_overridden: false,
            recovery_held: false,
        })
    }

    /// Flies another tool's `C_D0(M)` table instead of the drag buildup.
    #[must_use]
    pub fn with_drag_table(mut self, table: DragTable) -> Self {
        self.vehicle.aero = self.vehicle.aero.clone().with_drag_table(table);
        self.aero_overridden = true;
        self
    }

    /// Flies another tool's normal force and centre of pressure, against Mach number and angle of
    /// attack, instead of hpr's own ([`hpr_aero::NormalForceTable`], read from a RASAero II
    /// export). The table sets the static normal force at the centre of mass's airflow; the pitch
    /// and yaw damping stay hpr's, from the airspeed the rotation adds at each component, since a
    /// table has none (the decision record on normal-force overrides, [ADR-032][adr-032]). The
    /// flight still refuses Mach 5 and faster, where hpr's components, which give that damping,
    /// end.
    ///
    /// [adr-032]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-032-normal-force-overrides-from-rasaero-ii-the-static-force-replaced-hprs-damping-kept-2026-09-19
    ///
    /// # Errors
    ///
    /// [`SimError::Aero`] around [`hpr_aero::AeroError::Domain`] for a centre of pressure in the
    /// table outside the rocket ([`hpr_aero::AeroModel::with_normal_force_table`]).
    ///
    /// # Examples
    ///
    /// Valetudo from a 3 m rail on a small export with invented numbers: 9 per radian, and the
    /// centre of pressure 55 inches from the nose tip.
    ///
    /// ```
    /// use hpr_aero::NormalForceTable;
    /// use hpr_core::geodesy::Geodetic;
    /// use hpr_design::Rocket;
    /// use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};
    ///
    /// let rocket: Rocket = serde_json::from_str(include_str!(
    ///     "../../../validation/designs/rocketpy-valetudo.json"
    /// ))?;
    /// // The text of a RASAero II export; a program would read it from the file.
    /// let export = "Mach,Alpha,CN,CN Potential,CP\n\
    ///               0,0,0,0,55\n\
    ///               1,0,0,0,55\n\
    ///               0,2,0.314159,0.314159,55\n\
    ///               1,2,0.314159,0.314159,55\n";
    /// // On RASAero II's reference, the body's largest section, which hpr rescales to the
    /// // rocket's reference area.
    /// let table = NormalForceTable::from_rasaero_csv(export)?;
    /// let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    /// let simulation = Simulation::new(
    ///     &rocket,
    ///     "example",
    ///     Environment::standard(site)?,
    ///     Rail::vertical(3.0),
    ///     FlightSettings::default(),
    /// )?
    /// .with_normal_force_table(table)?;
    /// let flight = simulation.run(&mut ())?;
    /// assert!(flight.event(EventKind::Apogee).is_some());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_normal_force_table(mut self, table: NormalForceTable) -> Result<Self, SimError> {
        self.vehicle.aero = self.vehicle.aero.clone().with_normal_force_table(table)?;
        self.aero_overridden = true;
        Ok(self)
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
        self.trigger_times_s = recovery::plan(
            &devices,
            &self.vehicle.assembly.motors,
            self.vehicle.ignition_s(),
        )?;
        if self.parts() {
            // Already given a separation or ejections, so the bodies are known; otherwise the
            // check waits for them, and for the flight, so that the builders work in any order.
            check_bodies(&devices, self.body_count(), false)?;
        }
        self.devices = devices;
        Ok(self)
    }

    /// The recovery devices.
    #[must_use]
    pub fn recovery(&self) -> &[Device] {
        &self.devices
    }

    /// Flies with a separation: at its trigger the stack comes apart at the stage boundary, and
    /// each body descends under its own devices ([`crate::recovery::Separation`]); or, when the
    /// nose's body still has a motor to burn, it flies on as a sustainer and the aft body
    /// descends.
    ///
    /// Call this after [`Self::with_recovery`]: it checks the devices against the bodies.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if the design has no stage aft of the split, if a body carries no
    /// device (the descent has no airframe drag, so it would fall as if in a vacuum), if the
    /// trigger is out of its domain, or if its time is known and an aft body's motor burns past
    /// it, or if it is timed from a motor with no ignition known before the flight, so that it
    /// could never fire; [`SimError::Parting`] if an ejection already given parts at its stage
    /// boundary. The same checks run again if [`Self::with_recovery`] is called afterwards, so the
    /// builders can be given in any order. A device on a body that nothing makes is refused when
    /// the flight starts, since an ejection given later can make it.
    pub fn with_separation(mut self, separation: Separation) -> Result<Self, SimError> {
        self.check_no_shifts()?;
        let stages = self.vehicle.assembly.layout.stages.len();
        if separation.stages_of(1, stages).is_none() {
            return Err(SimError::Domain {
                what: "stage boundary of a separation (there is no stage aft of it)",
                value: separation.after_stage as f64,
            });
        }
        if !self.ejections.is_empty() {
            // Its boundary may be a joint an ejection parts at too.
            Pieces::new(
                &self.rocket,
                &self.vehicle.assembly,
                Some(separation),
                &self.ejections,
            )?;
        }
        if !self.devices.is_empty() {
            // With the devices already given they are checked now; given afterwards they are
            // checked then, and either way again when the flight starts.
            check_bodies(&self.devices, 2 + self.ejections.len(), false)?;
        }
        if let Trigger::Altitude {
            height_above_ground_m,
        } = separation.trigger
            && !(height_above_ground_m.is_finite() && height_above_ground_m > 0.0)
        {
            return Err(SimError::Domain {
                what: "height above the launch site at which the stack separates, m",
                value: height_above_ground_m,
            });
        }
        let time_s = recovery::trigger_time_s(
            separation.trigger,
            &self.vehicle.assembly.motors,
            self.vehicle.ignition_s(),
        )?;
        if let Some(time_s) = time_s {
            // A body's mass is held constant through its descent. For a trigger whose time is
            // known now, say so now rather than in the middle of a flight.
            self.check_aft_body_spent(separation, self.vehicle.ignition_s(), time_s)?;
        }
        if let (None, Trigger::MotorDelay { motor } | Trigger::Burnout { motor, .. }) =
            (time_s, separation.trigger)
        {
            // Timed from a motor with no ignition known before the flight: one lit by this very
            // separation, or one that never lights. It would never fire, and the stack would
            // land whole. (A `Time` always has its time.)
            return Err(SimError::Domain {
                what: "index of the motor a separation is timed from (it has no ignition time \
                       before the separation, so the separation could never fire)",
                value: motor as f64,
            });
        }
        self.separation_time_s = time_s;
        self.separation = Some(separation);
        Ok(self)
    }

    /// Flies with ejections: at each one's trigger a piece leaves the airframe, at the joint aft of
    /// a body component or as a payload from inside it, and each body flies on to its own landing
    /// under its own devices ([`crate::Ejection`]). With a separation as well, the separation's aft
    /// body is body 1 and ejection `k` makes body `k + 2`; without one, ejection `k` makes body
    /// `k + 1`.
    ///
    /// Call this after [`Self::with_recovery`]: it checks the devices against the bodies. The
    /// builders can be given in any order, and the checks run again when the flight starts.
    ///
    /// # Errors
    ///
    /// [`SimError::Parting`] for a parting the design can't make; [`SimError::Domain`] if a body
    /// carries no device, if a trigger or impulse is out of its domain, if its time is known and a
    /// motor burns past it, or if it is timed from a motor with no ignition known before the
    /// flight. A device on a body that nothing makes, and a pushed payload in the nose's piece, are
    /// refused when the flight starts. In flight, an ejection that fires while a motor burns, or
    /// ahead of a separation that would light one, is an error, and so are a powered separation in
    /// a flight with ejections and a pushed payload whose section's forward joint hasn't parted.
    pub fn with_ejections(mut self, ejections: Vec<Ejection>) -> Result<Self, SimError> {
        if !ejections.is_empty() {
            self.check_no_shifts()?;
        }
        Pieces::new(
            &self.rocket,
            &self.vehicle.assembly,
            self.separation,
            &ejections,
        )?;
        let bodies = 1 + usize::from(self.separation.is_some()) + ejections.len();
        if !self.devices.is_empty() && !ejections.is_empty() {
            check_bodies(&self.devices, bodies, false)?;
        }
        let mut times_s = Vec::with_capacity(ejections.len());
        for ejection in &ejections {
            if let Trigger::Altitude {
                height_above_ground_m,
            } = ejection.trigger
                && !(height_above_ground_m.is_finite() && height_above_ground_m > 0.0)
            {
                return Err(SimError::Domain {
                    what: "height above the launch site at which a piece is ejected, m",
                    value: height_above_ground_m,
                });
            }
            if !(ejection.impulse_n_s.is_finite() && ejection.impulse_n_s >= 0.0) {
                return Err(SimError::Domain {
                    what: "impulse of an ejection, N·s (zero or more: it pushes the two sides \
                           apart)",
                    value: ejection.impulse_n_s,
                });
            }
            let time_s = recovery::trigger_time_s(
                ejection.trigger,
                &self.vehicle.assembly.motors,
                self.vehicle.ignition_s(),
            )?;
            if let Some(time_s) = time_s {
                // Every body is a point mass of constant mass once the airframe parts.
                self.check_spent(self.vehicle.ignition_s(), time_s)?;
            }
            if let (None, Trigger::MotorDelay { motor } | Trigger::Burnout { motor, .. }) =
                (time_s, ejection.trigger)
            {
                return Err(SimError::Domain {
                    what: "index of the motor an ejection is timed from (it has no ignition time \
                           before the flight, so the ejection could never fire)",
                    value: motor as f64,
                });
            }
            times_s.push(time_s);
        }
        self.ejections = ejections;
        self.ejection_times_s = times_s;
        Ok(self)
    }

    /// The ejections, in the order given.
    #[must_use]
    pub fn ejections(&self) -> &[Ejection] {
        &self.ejections
    }

    /// Flies with parts that move along the airframe ([`MassShift`]), in the order given: a
    /// shift's index in this list names it in [`EventKind::Shift`]. A shift with a trigger known
    /// before the flight (a time, or a motor's burnout or delay) starts then; the flight watches
    /// for the apogee and for a height, descending, as it does for a recovery device's.
    ///
    /// # Errors
    ///
    /// [`SimError::Shift`] for a part that can't move ([`MassShift`] says which);
    /// [`SimError::Domain`] for a travel, duration or trigger outside its domain, or a trigger on
    /// a motor with no ignition known before the flight, which could never fire (and, in flight,
    /// for a shift that starts before the rocket leaves the rail);
    /// [`SimError::Unsupported`] with a separation or ejections, whose pieces are fixed before the
    /// flight with every part where the design puts it.
    pub fn with_shifts(mut self, shifts: Vec<MassShift>) -> Result<Self, SimError> {
        if !shifts.is_empty() && self.parts() {
            return Err(Self::shifts_and_partings());
        }
        let mut starts_s = Vec::with_capacity(shifts.len());
        for shift in &shifts {
            if let Trigger::Altitude {
                height_above_ground_m,
            } = shift.trigger
                && !(height_above_ground_m.is_finite() && height_above_ground_m > 0.0)
            {
                return Err(SimError::Domain {
                    what: "height above the launch site at which a part starts to move, m",
                    value: height_above_ground_m,
                });
            }
            // The check is a device's; its errors are put in a shift's words.
            let start_s = recovery::trigger_time_s(
                shift.trigger,
                &self.vehicle.assembly.motors,
                self.vehicle.ignition_s(),
            )
            .map_err(|error| match (error, shift.trigger) {
                (SimError::Domain { value, .. }, Trigger::Time { .. }) => SimError::Domain {
                    what: "start time of a mass shift after launch, s",
                    value,
                },
                (SimError::Domain { value, .. }, Trigger::MotorDelay { .. }) => SimError::Domain {
                    what: "the motor whose ejection delay starts a mass shift (it isn't there, \
                           or has no delay in seconds that is zero or more)",
                    value,
                },
                (SimError::Domain { value, .. }, Trigger::Burnout { .. }) => SimError::Domain {
                    what: "the motor, or the delay after its burnout, that starts a mass shift \
                           (the motor isn't there, or the delay isn't zero or more, s)",
                    value,
                },
                (error, _) => error,
            })?;
            if let (None, Trigger::MotorDelay { motor } | Trigger::Burnout { motor, .. }) =
                (start_s, shift.trigger)
            {
                return Err(SimError::Domain {
                    what: "index of the motor a mass shift is timed from (it has no ignition \
                           time before the flight, so the shift could never start)",
                    value: motor as f64,
                });
            }
            starts_s.push(start_s);
        }
        self.vehicle.shifts =
            Shifts::new(&self.rocket, &self.vehicle.assembly, &shifts, &starts_s)?;
        self.shifts = shifts;
        Ok(self)
    }

    /// The mass shifts, in the order given.
    #[must_use]
    pub fn shifts(&self) -> &[MassShift] {
        &self.shifts
    }

    /// The stack's mass properties at `t_s` as `flight` flew it: the design's, its motors burned
    /// to `t_s`, with each part that moves where it was then. `flight` must be a flight of this
    /// simulation; nothing checks that it is. A shift whose trigger is known before the flight (a
    /// time, or a motor's burnout or delay) starts then, whether or not `flight` got that far; one
    /// the flight watched for starts where `flight` records it ([`EventKind::Shift`]), and hasn't
    /// started if it doesn't. They are the whole stack's, in body axes about its centre of mass,
    /// before any separation (and a flight with a separation has no shifts).
    #[must_use]
    pub fn mass_properties(&self, flight: &FlightResult, t_s: f64) -> hpr_design::MassProperties {
        let mut shifts = self.vehicle.shifts.clone();
        for index in 0..self.shifts.len() {
            if let Some(event) = flight.event(EventKind::Shift(index)) {
                shifts.start(index, event.sample.time_s);
            }
        }
        let whole = self
            .vehicle
            .assembly
            .mass_properties_lit(t_s, self.vehicle.ignition_s());
        shifts.apply(whole, t_s)
    }

    /// Refuses a separation or ejections in a flight with mass shifts.
    fn check_no_shifts(&self) -> Result<(), SimError> {
        if self.shifts.is_empty() {
            Ok(())
        } else {
            Err(Self::shifts_and_partings())
        }
    }

    /// The refusal of mass shifts with a separation or ejections.
    fn shifts_and_partings() -> SimError {
        SimError::Unsupported {
            what: "a mass shift in a flight with a separation or ejections (the pieces are fixed \
                   before the flight, with every part where the design puts it)",
        }
    }

    /// The drag area of piece `piece` tumbling on its own, for a device on the body it leads:
    /// [`crate::recovery::DeviceDrag::tumbling`]'s model over the piece's own body components and
    /// fin sets. Piece 0 is the nose's, the separation makes piece 1, and each ejection the next
    /// ([`crate::Ejection`]), so piece `k` leads body `k`. Call it after [`Self::with_ejections`]
    /// and [`Self::with_separation`], which fix the pieces.
    ///
    /// It is the piece's own area only: a body that still carries another section, until that
    /// section's own parting, tumbles with its lead piece's area. The model was fitted to whole
    /// model rockets tumbling, so a lone nose cone is outside its fit: see the recovery page's
    /// [tumble section](https://nrdptel.github.io/hpr-sim/physics/recovery.html#tumble).
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] for a piece the airframe doesn't part into, a payload (it has no
    /// body tube or fin of its own), and as [`crate::recovery::DeviceDrag::tumbling`].
    pub fn tumbling_piece(&self, piece: usize) -> Result<recovery::DeviceDrag, SimError> {
        Pieces::new(
            &self.rocket,
            &self.vehicle.assembly,
            self.separation,
            &self.ejections,
        )?
        .tumbling(piece, &self.vehicle.assembly)
    }

    /// Refuses an ejection at `t_s` while any motor lit at `ignition_s` burns: once the airframe
    /// parts, every body is a point mass of constant mass.
    fn check_spent(&self, ignition_s: &[Option<f64>], t_s: f64) -> Result<(), SimError> {
        for (placed, ignition) in self.vehicle.assembly.motors.iter().zip(ignition_s) {
            if let Some(ignition_s) = ignition {
                let burnout_s = ignition_s + placed.mounted.motor.burnout_time_s();
                if burnout_s > t_s {
                    return Err(SimError::Domain {
                        what: "time of an ejection (every motor must have burned out by then; \
                               this is when one of them does)",
                        value: burnout_s,
                    });
                }
            }
        }
        Ok(())
    }

    /// Refuses a separation at `t_s` while a motor of its aft body, lit at `ignition_s`, burns or
    /// is still to light: that body descends as a point mass of constant mass.
    fn check_aft_body_spent(
        &self,
        separation: Separation,
        ignition_s: &[Option<f64>],
        t_s: f64,
    ) -> Result<(), SimError> {
        for (placed, ignition) in self.vehicle.assembly.motors.iter().zip(ignition_s) {
            if placed.stage <= separation.after_stage {
                continue;
            }
            if let Some(ignition_s) = ignition {
                let burnout_s = ignition_s + placed.mounted.motor.burnout_time_s();
                if burnout_s > t_s {
                    return Err(SimError::Domain {
                        what: "time of a separation (the aft body's motors must have burned out \
                               by then; this is when one of them does)",
                        value: burnout_s,
                    });
                }
            }
        }
        Ok(())
    }

    /// The separation, if the flight has one.
    #[must_use]
    pub fn separation(&self) -> Option<Separation> {
        self.separation
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

    /// This simulation with its recovery charges held: the stack's devices never fire, so it
    /// coasts through its apogee as if every delay were long. A separation that lights a motor
    /// ahead of it still happens, and a separated body's devices act as they would; one that
    /// doesn't is held with the charges. User events, which can't be copied, are left
    /// out.
    pub(crate) fn with_recovery_held(&self) -> Self {
        Self {
            vehicle: self.vehicle.clone(),
            environment: self.environment.clone(),
            rail: self.rail,
            guides: self.guides,
            settings: self.settings,
            user_events: Vec::new(),
            devices: self.devices.clone(),
            // No charge fires, so none of their times is a stop.
            trigger_times_s: vec![None; self.devices.len()],
            separation: self.separation,
            separation_time_s: self.separation_time_s,
            ejections: self.ejections.clone(),
            ejection_times_s: self.ejection_times_s.clone(),
            shifts: self.shifts.clone(),
            rocket: self.rocket.clone(),
            configuration_id: self.configuration_id.clone(),
            aero_overridden: self.aero_overridden,
            recovery_held: true,
        }
    }

    /// Flies from ignition on the pad until the flight ends.
    ///
    /// # Errors
    ///
    /// [`SimError`] from the models or the integrator (other than the step limit, which is a
    /// [`Termination`]), or from the observer. The checks that wait for every builder run here
    /// too: a device on a body nothing makes, and a pushed payload in the nose's piece
    /// ([`Self::with_ejections`]).
    pub fn run(&self, observer: &mut dyn Observer) -> Result<FlightResult, SimError> {
        self.fly(0.0, self.initial_state(), Phase::Pad, observer)
    }

    /// Flies freely from `state` at `t0_s` seconds after launch, as after a rail exit or from a
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
        vehicle: &Vehicle,
        phase: Phase,
        window: (f64, f64),
        t: f64,
        y: &[f64; STATE_LEN],
        drag_area_m2: f64,
    ) -> Result<Sample, SimError> {
        let evaluation = self.evaluate(vehicle, phase, window, t, y, drag_area_m2)?;
        Ok(sample_of(phase, t, y, &evaluation))
    }

    fn evaluate(
        &self,
        vehicle: &Vehicle,
        phase: Phase,
        window: (f64, f64),
        t: f64,
        y: &[f64; STATE_LEN],
        drag_area_m2: f64,
    ) -> Result<Evaluation, SimError> {
        vehicle.evaluate(
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
        // The builders can be given in either order, and the last one wins, so the devices and
        // the bodies are checked against each other here as well.
        check_bodies(&self.devices, self.body_count(), true)?;
        if self
            .ejections
            .iter()
            .any(|ejection| ejection.impulse_n_s > 0.0)
        {
            Pieces::new(
                &self.rocket,
                &self.vehicle.assembly,
                self.separation,
                &self.ejections,
            )?
            .check_pushed_payloads(&self.ejections)?;
        }
        if start_phase == Phase::Free {
            let height = self
                .evaluate(
                    &self.vehicle,
                    Phase::Free,
                    (t0, t0),
                    t0,
                    &state.to_array(),
                    0.0,
                )?
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
        // What a powered separation changes: the trigger and ignition times that count from it,
        // the vehicle flying, and the booster's descent.
        let mut trigger_times_s = self.trigger_times_s.clone();
        let mut ignition_s = self.vehicle.ignition_s().to_vec();
        let mut ignited: Vec<bool> = ignition_s
            .iter()
            .map(|ignition| ignition.is_some_and(|time| time <= t0))
            .collect();
        let mut sustainer: Option<Vehicle> = None;
        // The stack once a shift the flight watched for has started, with its start set.
        let mut shifted: Option<Vehicle> = None;
        // Which shifts have started; one that started before the flight's start isn't recorded.
        let mut shift_started: Vec<bool> = (0..self.shifts.len())
            .map(|index| {
                self.vehicle
                    .shifts
                    .start_s(index)
                    .is_some_and(|start_s| start_s < t0)
            })
            .collect();
        let mut staged = false;
        let mut booster: Vec<BodyFlight> = Vec::new();
        let mut stops = self.vehicle.thrust_knots_s();
        stops.push(cap);
        stops.extend(trigger_times_s.iter().flatten().copied());
        stops.extend(self.separation_time_s);
        stops.extend(self.ejection_times_s.iter().flatten().copied());
        stops.extend(self.vehicle.shifts.knots_s());
        stops.retain(|t| *t <= cap);
        stops.sort_by(f64::total_cmp);
        stops.dedup();
        let mut burnout_s = self.vehicle.burnout_s();
        let rail_origin = state.position_enu_m;
        let exit_travel_m = self.guides.exit_travel_m(self.rail.length_m);

        let mut phase = start_phase;
        // When a device froze the stack's attitude, which says nothing of its axis after that.
        let mut frozen_at_s = None;
        let mut lifted = start_phase != Phase::Pad;
        let mut burnout_recorded = t0 >= burnout_s;
        let mut separated = false;
        // The splits that have happened, in piece order: the separation, then the ejections.
        let mut opened = vec![false; self.body_count() - 1];
        // An unpowered separation or an ejection that a held flight skips.
        let mut separation_held = false;
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

            // Mass shifts: one whose start is known begins at that stop time; the flight watches
            // for the apogee and the heights of the rest, as it does for a device's. A shift
            // makes no step in the state, so the integrator carries on.
            for (index, started) in shift_started.iter_mut().enumerate() {
                if *started {
                    continue;
                }
                let stack = shifted.as_ref().unwrap_or(&self.vehicle);
                let window = (t, next_stop(&stops, t, cap));
                let area = self.ascent_drag_area_m2(&run, t);
                let starts = match stack.shifts.start_s(index) {
                    Some(start_s) => t >= start_s,
                    None if matches!(phase, Phase::Free | Phase::Descent) => {
                        match self.shifts[index].trigger {
                            // At or past the apogee, as a device's apogee trigger is.
                            Trigger::Apogee => {
                                self.evaluate(stack, phase, window, t, &y, area)?
                                    .vertical_speed_m_s
                                    <= 0.0
                            }
                            Trigger::Altitude {
                                height_above_ground_m,
                            } => {
                                let e = self.evaluate(stack, phase, window, t, &y, area)?;
                                e.vertical_speed_m_s < 0.0
                                    && e.height_above_ground_m <= height_above_ground_m
                            }
                            Trigger::Time { .. }
                            | Trigger::MotorDelay { .. }
                            | Trigger::Burnout { .. } => false,
                        }
                    }
                    None => false,
                };
                if !starts {
                    continue;
                }
                if matches!(phase, Phase::Pad | Phase::Rail) {
                    // The rail has no stop at its foot: a part thrown aft on the pad could push
                    // the rocket up the rail and leave it there.
                    return Err(SimError::Domain {
                        what: "start time of a mass shift, s (it must start once the rocket has \
                               left the rail)",
                        value: t,
                    });
                }
                if stack.shifts.start_s(index).is_none() {
                    shifted
                        .get_or_insert_with(|| self.vehicle.clone())
                        .shifts
                        .start(index, t);
                }
                let stack = shifted.as_ref().unwrap_or(&self.vehicle);
                for stop_s in stack.shifts.stops_s(index) {
                    if stop_s > t {
                        insert_stop(&mut stops, stop_s, cap);
                    }
                }
                *started = true;
                let window = (t, next_stop(&stops, t, cap));
                let sample = self.sample(stack, phase, window, t, &y, area)?;
                record(&mut events, observer, EventKind::Shift(index), sample);
            }
            let vehicle = sustainer
                .as_ref()
                .or(shifted.as_ref())
                .unwrap_or(&self.vehicle);

            // Motors lit after launch: their ignitions are stop times, so each is found here at
            // its own time.
            for index in 0..ignition_s.len() {
                if !ignited[index] && ignition_s[index].is_some_and(|time| t >= time) {
                    ignited[index] = true;
                    let window = (t, next_stop(&stops, t, cap));
                    let area = self.ascent_drag_area_m2(&run, t);
                    let sample = self.sample(vehicle, phase, window, t, &y, area)?;
                    record(&mut events, observer, EventKind::Ignition(index), sample);
                }
            }

            // Recovery: fire the charges whose time or height has come, then deploy the devices
            // whose lag has run out. A lag of zero deploys in the same pass, and a deployment can
            // release another device, so this repeats until nothing more happens. Only the
            // devices of body 0 act before a separation: a device meant for another body has a
            // drag area computed for that body (a booster's tumbling area, say), which is not a
            // model of the whole stack.
            while !self.recovery_held
                && !self.devices.is_empty()
                && matches!(phase, Phase::Free | Phase::Descent)
            {
                let mut again = false;
                let window = (t, next_stop(&stops, t, cap));
                let area = self.ascent_drag_area_m2(&run, t);
                // One evaluation serves every device in the pass: they all ask about the same
                // `(t, y)`. It is only made when a pending device needs the flight's state, and
                // it is not one of the integrator's, so `Stats` doesn't count it.
                let mut here: Option<Evaluation> = None;
                for index in 0..self.devices.len() {
                    if !run.pending(index) || !self.acts_before_separation(index) {
                        continue;
                    }
                    // `plan` gives `trigger_times_s` one entry per device, in order.
                    let fires = match self.devices[index].trigger {
                        Trigger::Time { .. }
                        | Trigger::MotorDelay { .. }
                        | Trigger::Burnout { .. } => trigger_times_s
                            .get(index)
                            .copied()
                            .flatten()
                            .is_some_and(|time| t >= time),
                        // At or past the apogee, which is RocketPy's own trigger (`y[5] < 0`)
                        // widened to include a flight that starts exactly at its apogee: with
                        // `< 0` such a flight has no crossing for the apogee event to find
                        // either, and would wait for the next interval. The apogee event fires
                        // this too, and whichever comes first wins.
                        Trigger::Apogee => {
                            let e = cached(&mut here, || {
                                self.evaluate(vehicle, phase, window, t, &y, area)
                            })?;
                            e.vertical_speed_m_s <= 0.0
                        }
                        Trigger::Altitude {
                            height_above_ground_m,
                        } => {
                            // An altimeter's main setting: descending, at or below the height.
                            // A rocket already below it at apogee fires there, as the event on
                            // the height never crosses it (RocketPy's numeric trigger).
                            let e = cached(&mut here, || {
                                self.evaluate(vehicle, phase, window, t, &y, area)
                            })?;
                            e.vertical_speed_m_s < 0.0
                                && e.height_above_ground_m <= height_above_ground_m
                        }
                    };
                    if fires {
                        let deploy_s = run.trigger(&self.devices, index, t);
                        insert_stop(&mut stops, deploy_s, cap);
                        let sample = self.sample(vehicle, phase, window, t, &y, area)?;
                        record(&mut events, observer, EventKind::Trigger(index), sample);
                        again = true;
                    }
                }
                for index in 0..self.devices.len() {
                    if !run.waiting(index)
                        || run.deploy_s(index) > t
                        || !self.acts_before_separation(index)
                    {
                        continue;
                    }
                    if run.released_s(index).is_some_and(|released| released <= t) {
                        // Cut away before its own charge fired: the canopy never flies.
                        run.abandon(index);
                        again = true;
                        continue;
                    }
                    let evaluation = cached(&mut here, || {
                        self.evaluate(vehicle, phase, window, t, &y, area)
                    })?;
                    let full_s = run.deploy(&self.devices, index, t, evaluation.airspeed_m_s);
                    insert_stop(&mut stops, full_s, cap);
                    if phase != Phase::Descent {
                        // The descent is a point mass: the attitude freezes where it deployed and
                        // the body rates go, snubbed by the lines and the canopy. The state's
                        // velocity is the nose tip's, so it is shifted to keep the centre of mass
                        // moving as it was: dropping `ω` while holding `v_O` would change the
                        // centre of mass's momentum with nothing to do it (found in review).
                        // Written this way it reads as what it is; the `ṙ_cg` terms cancel, so
                        // the net shift is `q(ω × r_cg)`.
                        phase = Phase::Descent;
                        frozen_at_s = Some(t);
                        let mut frozen = State::from_array(&y);
                        frozen.velocity_enu_m_s = evaluation.cg_velocity_enu_m_s
                            - frozen.unit_attitude().mul_vec3(evaluation.mass.cg_rate_m_s);
                        frozen.body_rate_rad_s = DVec3::ZERO;
                        y = frozen.to_array();
                        integrator.reset(t, y)?;
                        here = None;
                    }
                    let area = self.ascent_drag_area_m2(&run, t);
                    let sample = self.sample(vehicle, phase, window, t, &y, area)?;
                    record(&mut events, observer, EventKind::Deployment(index), sample);
                    again = true;
                }
                // A release happens when the device that releases it is fully open, which is a
                // stop time, so the drag area never dips between a drogue and a filling main.
                for index in 0..self.devices.len() {
                    if run.release_due(index, t) && self.acts_before_separation(index) {
                        run.release(index);
                        let area = self.ascent_drag_area_m2(&run, t);
                        let sample = self.sample(vehicle, phase, window, t, &y, area)?;
                        record(&mut events, observer, EventKind::Release(index), sample);
                        again = true;
                    }
                }
                if !again {
                    break;
                }
            }

            // The separation and the ejections: the same triggers as a device's. When the
            // separation fires with the nose's body still to burn, that body flies on as the
            // sustainer and the booster descends; otherwise the ascent ends and every body flies
            // on as a point mass.
            if self.parts()
                && !staged
                && !separation_held
                && matches!(phase, Phase::Free | Phase::Descent)
            {
                let window = (t, next_stop(&stops, t, cap));
                let area = self.ascent_drag_area_m2(&run, t);
                let mut here: Option<Evaluation> = None;
                let mut fires = |trigger: Trigger, time_s: Option<f64>| -> Result<bool, SimError> {
                    Ok(match trigger {
                        Trigger::Time { .. }
                        | Trigger::MotorDelay { .. }
                        | Trigger::Burnout { .. } => time_s.is_some_and(|time| t >= time),
                        // At or past the apogee, as a device's apogee trigger is.
                        Trigger::Apogee => {
                            cached(&mut here, || {
                                self.evaluate(vehicle, phase, window, t, &y, area)
                            })?
                            .vertical_speed_m_s
                                <= 0.0
                        }
                        Trigger::Altitude {
                            height_above_ground_m,
                        } => {
                            let e = cached(&mut here, || {
                                self.evaluate(vehicle, phase, window, t, &y, area)
                            })?;
                            e.vertical_speed_m_s < 0.0
                                && e.height_above_ground_m <= height_above_ground_m
                        }
                    })
                };
                let separates = match self.separation {
                    Some(separation) => fires(separation.trigger, self.separation_time_s)?,
                    None => false,
                };
                let mut ejected = Vec::new();
                for (index, ejection) in self.ejections.iter().enumerate() {
                    if fires(ejection.trigger, self.ejection_times_s[index])? {
                        ejected.push(index);
                    }
                }
                if separates || !ejected.is_empty() {
                    // The stack's motors as the separation lights them: one lit by it counts from
                    // now. A body's mass is held constant through its descent, so the booster's
                    // motors must be spent.
                    let mut lit = ignition_s.clone();
                    // The separation, when it fires with a motor ahead of it still to burn.
                    let mut powered: Option<Separation> = None;
                    if let Some(separation) = self.separation.filter(|_| separates) {
                        lit = self.vehicle.assembly.ignition_times_s(|stage| {
                            (stage == separation.after_stage).then_some(t)
                        });
                        let burning = self.vehicle.assembly.motors.iter().zip(&lit).any(
                            |(placed, ignition)| {
                                placed.stage <= separation.after_stage
                                    && ignition.is_some_and(|ignition| {
                                        ignition + placed.mounted.motor.burnout_time_s() > t
                                    })
                            },
                        );
                        powered = burning.then_some(separation);
                        // Checked before a held flight holds it, so that the delay's flight
                        // refuses what the flown one would.
                        self.check_aft_body_spent(separation, &lit, t)?;
                    }
                    if powered.is_some() && !self.ejections.is_empty() {
                        // The sustainer flies on a cut design whose components aren't the
                        // pieces the ejections were given for.
                        return Err(SimError::Domain {
                            what: "time of a powered separation in a flight with ejections (the \
                                   pieces of a sustainer aren't tracked)",
                            value: t,
                        });
                    }
                    if !ejected.is_empty() {
                        // Every body is a point mass of constant mass once the airframe parts.
                        self.check_spent(&lit, t)?;
                        if let Some(separation) = self.separation.filter(|_| !separates)
                            && self
                                .vehicle
                                .assembly
                                .ignition_times_s(|stage| {
                                    (stage == separation.after_stage).then_some(t)
                                })
                                .iter()
                                .zip(&lit)
                                .any(|(would, is)| would.is_some() && is.is_none())
                        {
                            return Err(SimError::Domain {
                                what: "time of an ejection before the separation that lights a \
                                       motor (the pieces would never light it)",
                                value: t,
                            });
                        }
                    }
                    if self.recovery_held && powered.is_none() {
                        // With nothing ahead of it left to burn it is part of the recovery, so it
                        // is held with the charges.
                        separation_held = true;
                        continue;
                    }
                    let sample = self.sample(vehicle, phase, window, t, &y, area)?;
                    if separates {
                        record(&mut events, observer, EventKind::Separation, sample);
                    }
                    for &index in &ejected {
                        record(&mut events, observer, EventKind::Ejection(index), sample);
                    }
                    let Some(separation) = powered else {
                        if separates {
                            opened[0] = true;
                        }
                        let first = usize::from(self.separation.is_some());
                        for &index in &ejected {
                            opened[first + index] = true;
                        }
                        ignition_s = lit;
                        separated = true;
                        break Termination::Separated;
                    };
                    if phase == Phase::Descent {
                        // The descent is a point mass under canopies; a sustainer under thrust
                        // is not something it can fly.
                        return Err(SimError::Domain {
                            what: "time of a separation whose nose body still has a motor to \
                                   burn (it comes after a recovery device opened on that body)",
                            value: t,
                        });
                    }
                    if self.aero_overridden {
                        return Err(SimError::Domain {
                            what: "time of a powered separation (a drag or normal-force table \
                                   is the whole stack's, and the sustainer has none)",
                            value: t,
                        });
                    }
                    // Built only now, so an unpowered separation never needs the cut design.
                    let model = Sustainer::of(
                        &self.rocket,
                        &self.configuration_id,
                        &self.vehicle.assembly,
                        separation,
                    )?;
                    let lit_here = model.motors.iter().map(|&index| lit[index]).collect();
                    let flown = Vehicle::lit(model.assembly, model.aero, lit_here)?;
                    booster = self.fly_bodies(
                        t,
                        &State::from_array(&y),
                        &mut run,
                        &lit,
                        (
                            &[true],
                            true,
                            frozen_at_s.is_some_and(|frozen_s| frozen_s < t),
                        ),
                    )?;
                    // The booster's descent has no airframe drag, and a powered separation comes
                    // near the top speed, so a coast to its device would climb as if in a vacuum
                    // (twice the sustainer's apogee in review). Its device must open at once.
                    let open_at_split = self.devices.iter().enumerate().any(|(index, device)| {
                        device.body == 1
                            && run.devices[index]
                                .deployed_s
                                .is_some_and(|deployed_s| deployed_s <= t)
                    });
                    if !open_at_split {
                        return Err(SimError::Domain {
                            what: "time of a powered separation (the booster falls with no \
                                   airframe drag, so a device on it must open at the separation: \
                                   one triggered at `Time { time_s: 0.0 }` with no lag does, as \
                                   a booster's devices act only once it flies)",
                            value: t,
                        });
                    }
                    trigger_times_s =
                        recovery::plan(&self.devices, &self.vehicle.assembly.motors, &lit)?;
                    for time in trigger_times_s
                        .iter()
                        .flatten()
                        .copied()
                        .chain(flown.thrust_knots_s())
                    {
                        if time > t {
                            insert_stop(&mut stops, time, cap);
                        }
                    }
                    if flown.burnout_s() > t {
                        burnout_recorded = false;
                    }
                    burnout_s = flown.burnout_s();
                    ignition_s = lit;
                    staged = true;
                    sustainer = Some(flown);
                    // The mass steps here, so the integrator starts afresh from the same state:
                    // the nose tip's, which the sustainer keeps.
                    integrator.reset(t, y)?;
                    continue;
                }
            }

            let next = next_stop(&stops, t, cap);
            let window = (t, next);
            if phase == Phase::Pad {
                if self
                    .evaluate(vehicle, Phase::Pad, window, t, &y, 0.0)?
                    .rail_force_n
                    > 0.0
                {
                    phase = Phase::Rail;
                    lifted = true;
                    let sample = self.sample(vehicle, phase, window, t, &y, 0.0)?;
                    record(&mut events, observer, EventKind::Liftoff, sample);
                } else if t >= burnout_s {
                    break if lifted {
                        Termination::StalledOnRail
                    } else {
                        Termination::NoLiftoff
                    };
                }
            }
            let watches = self.watches(phase, &run, !staged && !separation_held, &shift_started);
            let mut system = PhaseSystem {
                simulation: self,
                vehicle,
                phase,
                window,
                rail_origin,
                exit_travel_m,
                canopies: Canopies {
                    devices: &self.devices,
                    run: &run,
                    body: self.parts().then_some(0),
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
            let area = self.ascent_drag_area_m2(&run, t);
            // Burnout is a stop time, but an event can end the step on it first.
            if !burnout_recorded && t >= burnout_s {
                burnout_recorded = true;
                let sample = self.sample(vehicle, phase, window, t, &y, area)?;
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
                                let sample =
                                    self.sample(vehicle, Phase::Free, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::RailExit, sample);
                            }
                            Watch::RailStall => next_phase = Phase::Pad,
                            Watch::RailForce => {
                                next_phase = Phase::Rail;
                                lifted = true;
                                let sample =
                                    self.sample(vehicle, Phase::Rail, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::Liftoff, sample);
                            }
                            Watch::Apogee => {
                                let sample = self.sample(vehicle, phase, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::Apogee, sample);
                                for device in 0..self.devices.len() {
                                    // Only the stack's own: a body's device waits for its body,
                                    // which finds its own apogee.
                                    if !self.recovery_held
                                        && self.devices[device].trigger == Trigger::Apogee
                                        && run.pending(device)
                                        && self.acts_before_separation(device)
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
                                let sample = self.sample(vehicle, phase, window, t, &y, area)?;
                                record(&mut events, observer, EventKind::GroundHit, sample);
                                ground = true;
                            }
                            Watch::Altitude(device) => {
                                if !self.recovery_held && run.pending(device) {
                                    let deploy_s = run.trigger(&self.devices, device, t);
                                    insert_stop(&mut stops, deploy_s, cap);
                                    let sample =
                                        self.sample(vehicle, phase, window, t, &y, area)?;
                                    record(
                                        &mut events,
                                        observer,
                                        EventKind::Trigger(device),
                                        sample,
                                    );
                                }
                            }
                            Watch::SeparationHeight
                            | Watch::EjectionHeight(_)
                            | Watch::ShiftHeight(_) => {
                                // The separation, ejection or shift itself fires at the top of
                                // the next pass, which is where its burnout check and its bodies
                                // live, and where a shift's start is set.
                            }
                            Watch::User(user) => {
                                let sample = self.sample(vehicle, phase, window, t, &y, area)?;
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
        let vehicle = sustainer
            .as_ref()
            .or(shifted.as_ref())
            .unwrap_or(&self.vehicle);
        let next = next_stop(&stops, t, f64::INFINITY);
        let area = self.ascent_drag_area_m2(&run, t);
        let final_sample = self.sample(vehicle, phase, (t, next.max(t)), t, &y, area)?;
        let bodies = if separated {
            self.fly_bodies(
                t,
                &State::from_array(&y),
                &mut run,
                &ignition_s,
                (
                    &opened,
                    false,
                    frozen_at_s.is_some_and(|frozen_s| frozen_s < t),
                ),
            )?
        } else {
            booster
        };
        Ok(FlightResult {
            termination,
            events,
            final_sample,
            stats: integrator.stats(),
            bodies,
        })
    }

    /// Flies the bodies of a stack that came apart at `t` to their landings, with the stack's
    /// motors lit at `ignition_s`: `open[split]` says which splits happened (the separation, then
    /// the ejections), `skip_nose` leaves out body 0 when it flies on as a sustainer, and `frozen`
    /// says a device froze the stack's attitude before `t` (one that opens as the stack parts
    /// leaves the axis it had then).
    ///
    /// Each body is a point mass with its own pieces' and motors' mass, starting where its own
    /// centre of mass was with the velocity that point already had, plus the push of each
    /// ejection's impulse on its side. The bodies share the flight's devices and their progress: a
    /// device that had already opened stays open on whichever body carries it. A body that parts
    /// again on the way down hands the piece that leaves to a body of its own, flown after it.
    fn fly_bodies(
        &self,
        t: f64,
        state: &State,
        run: &mut Run,
        ignition_s: &[Option<f64>],
        (open, skip_nose, frozen): (&[bool], bool, bool),
    ) -> Result<Vec<BodyFlight>, SimError> {
        if !self.parts() {
            return Ok(Vec::new());
        }
        let pieces = Pieces::new(
            &self.rocket,
            &self.vehicle.assembly,
            self.separation,
            &self.ejections,
        )?;
        let mut open = open.to_vec();
        open.resize(pieces.count() - 1, false);
        let attitude = state.unit_attitude();
        // A trigger on a motor the separation lit has its time only now.
        let trigger_times_s =
            recovery::plan(&self.devices, &self.vehicle.assembly.motors, ignition_s)?;
        let leaders = pieces.leaders(&open);
        let mut starts = Vec::new();
        for body in 0..pieces.count() {
            if leaders[body] != body || (skip_nose && body == 0) {
                continue;
            }
            let mass = pieces.mass_properties(
                |piece| leaders[piece] == body,
                &self.vehicle.assembly,
                t,
                ignition_s,
            );
            // Its own centre of mass, and the velocity that point had: `v_O + ω × r` in `L`.
            let cg_enu_m = state.point_enu_m(mass.cg_m);
            let velocity_enu_m_s =
                state.velocity_enu_m_s + attitude.mul_vec3(state.body_rate_rad_s.cross(mass.cg_m));
            starts.push(Start {
                body,
                t_s: t,
                cg_enu_m,
                velocity_enu_m_s,
                mass_kg: checked_body_mass(mass.mass_kg)?,
            });
        }
        // Each split that parts the stack here pushes its two sides apart: along the airframe's
        // axis while it flies with nothing open, and by its velocity through the air once a
        // device has frozen its attitude at some earlier time (ADR-086).
        let firing: Vec<usize> = (0..open.len()).filter(|&index| open[index]).collect();
        if firing
            .iter()
            .any(|&index| self.split_impulse_n_s(index) > 0.0)
        {
            let nose_ward = if frozen {
                let (mut kg, mut momentum, mut moment) = (0.0, DVec3::ZERO, DVec3::ZERO);
                for start in &starts {
                    kg += start.mass_kg;
                    momentum += start.velocity_enu_m_s * start.mass_kg;
                    moment += start.cg_enu_m * start.mass_kg;
                }
                let hanging =
                    run.hung_before(&self.devices, |index| self.acts_before_separation(index), t);
                self.nose_ward_enu(moment / kg, momentum / kg, hanging)?
            } else {
                attitude.mul_vec3(DVec3::Z)
            };
            self.push_apart(
                (&firing, &pieces, &open, &leaders),
                nose_ward,
                &mut starts,
                t,
            )?;
        }
        let mut queue: std::collections::VecDeque<Start> = starts.into();
        let mut bodies = Vec::new();
        while let Some(start) = queue.pop_front() {
            let mut split = Split {
                pieces: &pieces,
                open: &mut open,
                ignition_s,
                queue: &mut queue,
            };
            bodies.push(self.fly_body(start, &mut split, run, &trigger_times_s)?);
        }
        bodies.sort_by_key(|body| body.body);
        Ok(bodies)
    }

    /// Flies one body as a point mass from its `start` to its landing, parting it on the way
    /// down at each of its splits that fires.
    fn fly_body(
        &self,
        start: Start,
        split: &mut Split<'_>,
        run: &mut Run,
        trigger_times_s: &[Option<f64>],
    ) -> Result<BodyFlight, SimError> {
        let Start {
            body,
            t_s: t0,
            cg_enu_m,
            velocity_enu_m_s,
            mut mass_kg,
        } = start;
        let cap = self.settings.max_time_s;
        let start = [
            cg_enu_m.x,
            cg_enu_m.y,
            cg_enu_m.z,
            velocity_enu_m_s.x,
            velocity_enu_m_s.y,
            velocity_enu_m_s.z,
        ];
        let mut integrator = Integrator::new(self.settings.method, t0, start)?
            .with_step_limit(self.settings.step_limit);
        // Every time this body's devices already have: their trigger times, and — for a device
        // that was triggered, deployed or released before the separation — its deployment, the
        // end of its filling and its release. Without these the descent would step straight past
        // them (found in review).
        let mut stops: Vec<f64> = Vec::new();
        for index in 0..self.devices.len() {
            if self.devices[index].body != body {
                continue;
            }
            stops.extend(trigger_times_s.get(index).copied().flatten());
            stops.extend(run.times_of(index));
        }
        // And its splits' known times.
        let mut leaders = split.pieces.leaders(split.open);
        for index in split.pending(&leaders, body) {
            stops.extend(self.split(index).1);
        }
        stops.retain(|time| *time > t0 && *time <= cap);
        stops.push(cap);
        stops.sort_by(f64::total_cmp);
        stops.dedup();
        let mut events: Vec<BodyEvent> = Vec::new();
        let start_sample = self.body_sample(body, mass_kg, t0, &start, run)?;
        if start_sample.height_above_ground_m <= 0.0 {
            // The ground event is a falling crossing, so a body that starts below the site would
            // integrate underground to the time cap.
            return Err(SimError::Domain {
                what: "starting height of a separated body's centre of mass above the ground",
                value: start_sample.height_above_ground_m,
            });
        }
        let mine: Vec<usize> = (0..self.devices.len())
            .filter(|index| self.devices[*index].body == body)
            .collect();

        let termination = loop {
            let t = integrator.time_s();
            let mut y = *integrator.state();
            if t >= cap {
                break Termination::TimeCap;
            }
            // Charges, deployments and releases, as in the main loop.
            loop {
                let mut again = false;
                let sample = self.body_sample(body, mass_kg, t, &y, run)?;
                for &index in &mine {
                    if !run.pending(index) {
                        continue;
                    }
                    let fires = match self.devices[index].trigger {
                        Trigger::Time { .. }
                        | Trigger::MotorDelay { .. }
                        | Trigger::Burnout { .. } => trigger_times_s
                            .get(index)
                            .copied()
                            .flatten()
                            .is_some_and(|time| t >= time),
                        Trigger::Apogee => sample.vertical_speed_m_s <= 0.0,
                        Trigger::Altitude {
                            height_above_ground_m,
                        } => {
                            sample.vertical_speed_m_s < 0.0
                                && sample.height_above_ground_m <= height_above_ground_m
                        }
                    };
                    if fires {
                        let deploy_s = run.trigger(&self.devices, index, t);
                        insert_stop(&mut stops, deploy_s, cap);
                        events.push(BodyEvent {
                            kind: EventKind::Trigger(index),
                            sample,
                            after: None,
                        });
                        again = true;
                    }
                }
                for &index in &mine {
                    if !run.waiting(index) || run.deploy_s(index) > t {
                        continue;
                    }
                    if run.released_s(index).is_some_and(|released| released <= t) {
                        run.abandon(index);
                        again = true;
                        continue;
                    }
                    let full_s = run.deploy(&self.devices, index, t, sample.airspeed_m_s);
                    insert_stop(&mut stops, full_s, cap);
                    events.push(BodyEvent {
                        kind: EventKind::Deployment(index),
                        sample: self.body_sample(body, mass_kg, t, &y, run)?,
                        after: None,
                    });
                    again = true;
                }
                for &index in &mine {
                    if run.release_due(index, t) {
                        run.release(index);
                        events.push(BodyEvent {
                            kind: EventKind::Release(index),
                            sample: self.body_sample(body, mass_kg, t, &y, run)?,
                            after: None,
                        });
                        again = true;
                    }
                }
                if !again {
                    break;
                }
            }

            // Its splits whose trigger has come part it at once, as the stack parts at the first
            // parting: each piece they make starts here, at this body's point and velocity plus
            // the pushes on its sides, and is flown after it. Which fire, and the way the pushes
            // point, are decided on the body as the pass starts, so neither depends on the order
            // the splits were given in (found in review: taking them one at a time let a push
            // delay a split, or share one charge's push with pieces another then parted). A split
            // that hasn't fired is asked again at the same instant, on the pushed body.
            let pending = split.pending(&leaders, body);
            let fires = |index: usize, sample: &BodySample| {
                let (trigger, time_s, _) = self.split(index);
                match trigger {
                    Trigger::Time { .. } | Trigger::MotorDelay { .. } | Trigger::Burnout { .. } => {
                        time_s.is_some_and(|time| t >= time)
                    }
                    Trigger::Apogee => sample.vertical_speed_m_s <= 0.0,
                    Trigger::Altitude {
                        height_above_ground_m,
                    } => {
                        sample.vertical_speed_m_s < 0.0
                            && sample.height_above_ground_m <= height_above_ground_m
                    }
                }
            };
            let before = if pending.is_empty() {
                None
            } else {
                Some(self.body_sample(body, mass_kg, t, &y, run)?)
            };
            let firing: Vec<usize> = before.map_or_else(Vec::new, |before| {
                pending
                    .iter()
                    .copied()
                    .filter(|&index| fires(index, &before))
                    .collect()
            });
            if let (Some(before), false) = (before, firing.is_empty()) {
                // A separation still to come here lights no motor: an ejection ahead of one that
                // would is refused in the ascent.
                for &index in &firing {
                    split.open[index] = true;
                }
                leaders = split.pieces.leaders(split.open);
                let mass_of = |lead: usize| {
                    checked_body_mass(
                        split
                            .pieces
                            .mass_properties(
                                |piece| leaders[piece] == lead,
                                &self.vehicle.assembly,
                                t,
                                split.ignition_s,
                            )
                            .mass_kg,
                    )
                };
                // This body, then the body each split makes, led by the split's piece.
                let mut parts = vec![Start {
                    body,
                    t_s: t,
                    cg_enu_m: before.cg_enu_m,
                    velocity_enu_m_s: before.cg_velocity_enu_m_s,
                    mass_kg: mass_of(body)?,
                }];
                for &index in &firing {
                    parts.push(Start {
                        body: index + 1,
                        mass_kg: mass_of(index + 1)?,
                        ..parts[0]
                    });
                }
                if firing
                    .iter()
                    .any(|&index| self.split_impulse_n_s(index) > 0.0)
                {
                    let hanging =
                        run.hung_before(&self.devices, |index| self.devices[index].body == body, t);
                    let nose_ward =
                        self.nose_ward_enu(before.cg_enu_m, before.cg_velocity_enu_m_s, hanging)?;
                    let open = &*split.open;
                    self.push_apart(
                        (&firing, split.pieces, open, &leaders),
                        nose_ward,
                        &mut parts,
                        t,
                    )?;
                }
                mass_kg = parts[0].mass_kg;
                y[3] = parts[0].velocity_enu_m_s.x;
                y[4] = parts[0].velocity_enu_m_s.y;
                y[5] = parts[0].velocity_enu_m_s.z;
                let after = self.body_sample(body, mass_kg, t, &y, run)?;
                for &index in &firing {
                    events.push(BodyEvent {
                        kind: self.split(index).2,
                        sample: before,
                        after: Some(after),
                    });
                }
                split.queue.extend(parts.into_iter().skip(1));
                // The mass steps here, and a push steps the velocity, so the integrator starts
                // afresh from the new state.
                integrator.reset(t, y)?;
                continue;
            }

            let next = next_stop(&stops, t, cap);
            // The heights this body watches for: its devices' and its splits'.
            let mut watches: Vec<f64> = mine
                .iter()
                .filter(|index| run.pending(**index))
                .filter_map(|index| match self.devices[*index].trigger {
                    Trigger::Altitude {
                        height_above_ground_m,
                    } => Some(height_above_ground_m),
                    _ => None,
                })
                .collect();
            for &index in &pending {
                if let Trigger::Altitude {
                    height_above_ground_m,
                } = self.split(index).0
                {
                    watches.push(height_above_ground_m);
                }
            }
            // Every body watches for its own apogee: an apogee charge on a body separated while
            // climbing would never fire without it (found in review), and since the ascent ends
            // at the separation this is the only place a staged flight can record a peak.
            let apogee = events.iter().all(|event| event.kind != EventKind::Apogee);
            let mut system = BodySystem {
                simulation: self,
                body,
                mass_kg,
                run,
                apogee,
                watches: &watches,
                failure: None,
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
            if let Advance::Events = outcome {
                let fired = integrator.fired_events().to_vec();
                let t = integrator.time_s();
                let y = *integrator.state();
                if apogee && fired.contains(&1) {
                    // The body's own apogee: the ascent's ended at the separation, so this is the
                    // only place a staged flight can record one.
                    events.push(BodyEvent {
                        kind: EventKind::Apogee,
                        sample: self.body_sample(body, mass_kg, t, &y, run)?,
                        after: None,
                    });
                }
                if fired.contains(&0) {
                    events.push(BodyEvent {
                        kind: EventKind::GroundHit,
                        sample: self.body_sample(body, mass_kg, t, &y, run)?,
                        after: None,
                    });
                    break Termination::GroundHit;
                }
            }
        };

        let t = integrator.time_s();
        let y = *integrator.state();
        let final_sample = self.body_sample(body, mass_kg, t, &y, run)?;
        if termination == Termination::GroundHit
            && !mine
                .iter()
                .any(|&index| run.devices[index].deployed_s.is_some())
        {
            // Every body must carry a device, and its device must actually open: a trigger that
            // never becomes true (a timer set after the body lands, say) would
            // otherwise drop the body with no drag at all, which is a wrong number rather than a
            // missing feature (found in review). A device that opened on the stack before the
            // separation counts: its deployment is in the flight's events, not the body's.
            return Err(SimError::Domain {
                what: "a separated body reached the ground with no device open (its triggers \
                       never fired); the body",
                value: body as f64,
            });
        }
        let pieces: Vec<usize> = (0..leaders.len())
            .filter(|piece| leaders[*piece] == body)
            .collect();
        Ok(BodyFlight {
            body,
            stages: split.pieces.stages(|piece| leaders[piece] == body),
            pieces,
            mass_kg,
            start_sample,
            termination,
            events,
            final_sample,
            stats: integrator.stats(),
        })
    }

    /// One separated body at `(t, y)`, where `y` is its centre of mass and that point's velocity.
    fn body_sample(
        &self,
        body: usize,
        mass_kg: f64,
        t: f64,
        y: &[f64; 6],
        run: &Run,
    ) -> Result<BodySample, SimError> {
        let cg_enu_m = DVec3::new(y[0], y[1], y[2]);
        let velocity_enu_m_s = DVec3::new(y[3], y[4], y[5]);
        let frame = self.environment.earth.frame();
        let height_above_ground_m =
            frame.geodetic_from_enu(cg_enu_m)?.height_m - frame.origin().height_m;
        let (up_enu, wind_enu) = self.up_and_wind_enu(cg_enu_m)?;
        Ok(BodySample {
            time_s: t,
            cg_enu_m,
            cg_velocity_enu_m_s: velocity_enu_m_s,
            height_above_ground_m,
            vertical_speed_m_s: up_enu.dot(velocity_enu_m_s),
            airspeed_m_s: (velocity_enu_m_s - wind_enu).length(),
            recovery_drag_area_m2: run.body_drag_area_m2(&self.devices, body, t),
            mass_kg,
        })
    }

    /// The way the nose of a body with no attitude of its own is taken to point, as a unit vector
    /// in the launch frame, for the push of an ejection (ADR-086): a body `hanging` from a device
    /// points its forward end against its velocity through the air, toward the device, assumed to
    /// have left through that end; any other points its nose along that velocity, as a stable
    /// airframe does. Below [`STILL_AIR_M_S`] that velocity is drift or round-off rather
    /// than a flight path, and the nose is taken to point up.
    fn nose_ward_enu(
        &self,
        cg_enu_m: DVec3,
        velocity_enu_m_s: DVec3,
        hanging: bool,
    ) -> Result<DVec3, SimError> {
        let (up_enu, wind_enu) = self.up_and_wind_enu(cg_enu_m)?;
        let through_air = velocity_enu_m_s - wind_enu;
        let speed_m_s = through_air.length();
        Ok(if speed_m_s < STILL_AIR_M_S {
            up_enu
        } else if hanging {
            -through_air / speed_m_s
        } else {
            through_air / speed_m_s
        })
    }

    /// The local vertical and the wind at the point `cg_enu_m` of the launch frame.
    fn up_and_wind_enu(&self, cg_enu_m: DVec3) -> Result<(DVec3, DVec3), SimError> {
        let frame = self.environment.earth.frame();
        let geodetic = frame.geodetic_from_enu(cg_enu_m)?;
        let up_ecef = DVec3::new(
            geodetic.latitude_rad.cos() * geodetic.longitude_rad.cos(),
            geodetic.latitude_rad.cos() * geodetic.longitude_rad.sin(),
            geodetic.latitude_rad.sin(),
        );
        let up_enu = frame.ecef_from_enu_rotation().transpose() * up_ecef;
        let height_msl_m = geodetic.height_m - self.environment.geoid_undulation_m;
        let wind_enu = self.environment.wind.wind(height_msl_m)?.velocity_enu_m_s;
        Ok((up_enu, wind_enu))
    }

    /// Whether the airframe can come apart: it has a separation or an ejection.
    fn parts(&self) -> bool {
        self.separation.is_some() || !self.ejections.is_empty()
    }

    /// How many bodies the airframe can come apart into: one per piece.
    fn body_count(&self) -> usize {
        1 + usize::from(self.separation.is_some()) + self.ejections.len()
    }

    /// Pushes apart the bodies in `parts` (each a body's start, by its lead piece) at the splits
    /// `firing`, all parting at `t`: each split's own piece's body takes `±J` along `nose_ward`,
    /// `+J` if the piece is forward of the split, and the body on its other side the opposite,
    /// each over its own mass, so the momentum is unchanged (ADR-086). `open` says which splits
    /// have happened, these among them, and `leaders` gives each piece's body after them.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] for a pushed payload whose section's forward joint hasn't parted, so
    /// that it has no way out forward.
    fn push_apart(
        &self,
        (firing, pieces, open, leaders): (&[usize], &Pieces, &[bool], &[usize]),
        nose_ward: DVec3,
        parts: &mut [Start],
        t: f64,
    ) -> Result<(), SimError> {
        for &index in firing {
            let impulse_n_s = self.split_impulse_n_s(index);
            if impulse_n_s == 0.0 {
                continue;
            }
            let piece = index + 1;
            if let Some(host) = pieces.host(piece)
                && !(host > 0 && open[host - 1])
            {
                return Err(SimError::Domain {
                    what: "time of a pushed payload's ejection (it leaves forward, and the joint \
                           forward of the section that carries it has not parted by then)",
                    value: t,
                });
            }
            let (other, forward) = pieces.across(piece);
            let push = nose_ward * if forward { impulse_n_s } else { -impulse_n_s };
            for (lead, push) in [(leaders[piece], push), (leaders[other], -push)] {
                let part = parts.iter_mut().find(|part| part.body == lead);
                // Both sides of a split that fires here fly: only a sustainer's body is left out,
                // and a powered separation is refused with ejections, whose splits alone push.
                debug_assert!(part.is_some(), "a pushed body {lead} doesn't fly");
                if let Some(part) = part {
                    part.velocity_enu_m_s += push / part.mass_kg;
                }
            }
        }
        Ok(())
    }

    /// The impulse of split `split`, in piece order, N·s: an ejection's own, and none for the
    /// separation.
    fn split_impulse_n_s(&self, split: usize) -> f64 {
        match split.checked_sub(usize::from(self.separation.is_some())) {
            Some(ejection) => self.ejections[ejection].impulse_n_s,
            None => 0.0,
        }
    }

    /// Split `split` of the flight, in piece order (the separation first, then the ejections):
    /// its trigger, its time when that is known before the flight, and its event.
    fn split(&self, split: usize) -> (Trigger, Option<f64>, EventKind) {
        match (self.separation, split) {
            (Some(separation), 0) => (
                separation.trigger,
                self.separation_time_s,
                EventKind::Separation,
            ),
            _ => {
                let ejection = split - usize::from(self.separation.is_some());
                (
                    self.ejections[ejection].trigger,
                    self.ejection_times_s[ejection],
                    EventKind::Ejection(ejection),
                )
            }
        }
    }

    /// The drag area acting on the stack before it comes apart, m²: body 0's devices, which with
    /// no separation or ejection is all of them.
    fn ascent_drag_area_m2(&self, run: &Run, t: f64) -> f64 {
        if self.parts() {
            run.body_drag_area_m2(&self.devices, 0, t)
        } else {
            run.drag_area_m2(&self.devices, t)
        }
    }

    /// Whether device `index` acts on the stack before it comes apart: only body 0's do.
    fn acts_before_separation(&self, index: usize) -> bool {
        !self.parts() || self.devices[index].body == 0
    }

    /// What the integrator watches for in `phase`, in event order.
    fn watches(
        &self,
        phase: Phase,
        run: &Run,
        separation_pending: bool,
        shift_started: &[bool],
    ) -> Vec<Watch> {
        match phase {
            Phase::Pad => vec![Watch::RailForce],
            Phase::Rail => vec![Watch::RailExit, Watch::RailStall],
            Phase::Free | Phase::Descent => {
                let mut watches = vec![Watch::Apogee, Watch::Ground];
                for (index, device) in self.devices.iter().enumerate() {
                    if matches!(device.trigger, Trigger::Altitude { .. })
                        && run.pending(index)
                        && self.acts_before_separation(index)
                    {
                        watches.push(Watch::Altitude(index));
                    }
                }
                // The separation's own height, so it is located rather than polled at the next
                // boundary that happens to exist.
                if let Some(Trigger::Altitude { .. }) = self
                    .separation
                    .filter(|_| separation_pending)
                    .map(|separation| separation.trigger)
                {
                    watches.push(Watch::SeparationHeight);
                }
                if separation_pending {
                    for (index, ejection) in self.ejections.iter().enumerate() {
                        if matches!(ejection.trigger, Trigger::Altitude { .. }) {
                            watches.push(Watch::EjectionHeight(index));
                        }
                    }
                }
                for (index, shift) in self.shifts.iter().enumerate() {
                    if matches!(shift.trigger, Trigger::Altitude { .. })
                        && !shift_started.get(index).copied().unwrap_or(true)
                    {
                        watches.push(Watch::ShiftHeight(index));
                    }
                }
                watches.extend((0..self.user_events.len()).map(Watch::User));
                watches
            }
        }
    }
}

/// Checks the devices against the `bodies` a separation and ejections make, whichever builder ran
/// last: one when there is neither.
///
/// When the airframe can part, every body needs at least one device, and no device may name a
/// body that doesn't exist. Without a parting there is only body 0, so a device that names another
/// would have its drag area counted on the whole rocket and never be flown on a body of its own.
/// A builder checks only the first (`complete` false): a separation or ejection given after it
/// can still make the body a device names, and an ejection's number counts the separation.
fn check_bodies(devices: &[Device], bodies: usize, complete: bool) -> Result<(), SimError> {
    if let Some(device) = devices
        .iter()
        .find(|device| complete && device.body >= bodies)
    {
        return Err(SimError::Domain {
            what: if bodies > 1 {
                "body a device is attached to (the separation and ejections don't make it)"
            } else {
                "body a device is attached to (there is no separation or ejection, so there is \
                 only body 0)"
            },
            value: device.body as f64,
        });
    }
    if bodies > 1 {
        for body in 0..bodies {
            if !devices.iter().any(|device| device.body == body) {
                return Err(SimError::Domain {
                    what: "recovery devices on a separated body (every body needs one: a descent \
                           has no airframe drag)",
                    value: body as f64,
                });
            }
        }
    }
    Ok(())
}

/// The first stop after `t`, or `cap`.
fn next_stop(stops: &[f64], t: f64, cap: f64) -> f64 {
    stops.iter().copied().find(|stop| *stop > t).unwrap_or(cap)
}

/// The evaluation for the recovery scan, made once per pass and reused.
fn cached(
    cache: &mut Option<Evaluation>,
    evaluate: impl FnOnce() -> Result<Evaluation, SimError>,
) -> Result<Evaluation, SimError> {
    if let Some(evaluation) = cache {
        return Ok(*evaluation);
    }
    let evaluation = evaluate()?;
    *cache = Some(evaluation);
    Ok(evaluation)
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
    /// The separation's height, descending.
    SeparationHeight,
    /// An ejection's height, descending, by its index.
    EjectionHeight(usize),
    /// A mass shift's height, descending, by its index.
    ShiftHeight(usize),
    /// A user event.
    User(usize),
}

/// Where a body's own flight starts: its number, the time, its centre of mass and that point's
/// velocity, and its mass.
#[derive(Debug, Clone, Copy)]
struct Start {
    body: usize,
    t_s: f64,
    cg_enu_m: DVec3,
    velocity_enu_m_s: DVec3,
    mass_kg: f64,
}

/// What the bodies' flights share about the airframe's parting: its pieces, which splits have
/// happened, the motors' ignitions, and the bodies still to fly.
struct Split<'a> {
    pieces: &'a Pieces,
    open: &'a mut Vec<bool>,
    ignition_s: &'a [Option<f64>],
    queue: &'a mut std::collections::VecDeque<Start>,
}

impl Split<'_> {
    /// The splits still to happen inside `body`, given each piece's body in `leaders`: those whose
    /// piece is still joined to it.
    fn pending(&self, leaders: &[usize], body: usize) -> Vec<usize> {
        (0..self.open.len())
            .filter(|&index| !self.open[index] && leaders[index + 1] == body)
            .collect()
    }
}

/// A body's mass, refused unless it is finite and positive.
fn checked_body_mass(mass_kg: f64) -> Result<f64, SimError> {
    if mass_kg.is_finite() && mass_kg > 0.0 {
        Ok(mass_kg)
    } else {
        Err(SimError::Domain {
            what: "mass of a separated body, kg",
            value: mass_kg,
        })
    }
}

/// One separated body as the integrator sees it: a point mass under its open devices' drag area,
/// with its centre of mass and that point's velocity as the state.
///
/// The equations are the descent phase's (`docs/physics/recovery.md`) with no thrust and no
/// airframe: `m a = −½ ρ (C_D S)(t) |v − w| (v − w) + m (g + a_Coriolis)`.
struct BodySystem<'a> {
    simulation: &'a Simulation,
    body: usize,
    mass_kg: f64,
    run: &'a Run,
    /// Whether the body is watching for its own apogee, which is event 1 when it is.
    apogee: bool,
    /// The heights this body is watching for, after the apogee: its devices' deployment heights
    /// and its splits'.
    watches: &'a [f64],
    failure: Option<SimError>,
}

impl BodySystem<'_> {
    /// The height above the site and the drag acceleration at `(t, y)`.
    fn sample(&self, t_s: f64, y: &[f64; 6]) -> Result<BodySample, SimError> {
        self.simulation
            .body_sample(self.body, self.mass_kg, t_s, y, self.run)
    }
}

impl OdeSystem<6> for BodySystem<'_> {
    type Error = SimError;

    fn derivative(&mut self, t_s: f64, y: &[f64; 6]) -> Result<[f64; 6], SimError> {
        let cg_enu_m = DVec3::new(y[0], y[1], y[2]);
        let velocity_enu_m_s = DVec3::new(y[3], y[4], y[5]);
        let environment = &self.simulation.environment;
        let frame = environment.earth.frame();
        let geodetic = frame.geodetic_from_enu(cg_enu_m)?;
        let height_msl_m = geodetic.height_m - environment.geoid_undulation_m;
        let air = environment.atmosphere.air(height_msl_m)?.air;
        let wind_enu = environment.wind.wind(height_msl_m)?.velocity_enu_m_s;
        let gravity_enu = environment.earth.gravity_enu_mps2(cg_enu_m)?;
        let coriolis_enu = environment
            .earth
            .rotation_acceleration_enu_mps2(velocity_enu_m_s);
        let drag_area_m2 = self
            .run
            .body_drag_area_m2(&self.simulation.devices, self.body, t_s);
        let air_velocity = velocity_enu_m_s - wind_enu;
        let speed = air_velocity.length();
        let drag_enu = if air.density_kg_m3 > 0.0 && speed > 0.0 && drag_area_m2 > 0.0 {
            air_velocity * (-0.5 * air.density_kg_m3 * drag_area_m2 * speed / self.mass_kg)
        } else {
            DVec3::ZERO
        };
        let acceleration = drag_enu + gravity_enu + coriolis_enu;
        Ok([
            velocity_enu_m_s.x,
            velocity_enu_m_s.y,
            velocity_enu_m_s.z,
            acceleration.x,
            acceleration.y,
            acceleration.z,
        ])
    }

    fn event_count(&self) -> usize {
        1 + usize::from(self.apogee) + self.watches.len()
    }

    fn event_direction(&self, _index: usize) -> Direction {
        Direction::Falling
    }

    fn event_value(&mut self, index: usize, t_s: f64, y: &[f64; 6]) -> f64 {
        let sample = match self.sample(t_s, y) {
            Ok(sample) => sample,
            Err(error) => {
                self.failure.get_or_insert(error);
                return f64::NAN;
            }
        };
        // The ground, then this body's apogee if it is looking for one, then each watched
        // device's deployment height. Every one is a falling crossing, so a device fires on the
        // way down only.
        if index == 0 {
            return sample.height_above_ground_m;
        }
        if self.apogee && index == 1 {
            return sample.vertical_speed_m_s;
        }
        let watch = index - 1 - usize::from(self.apogee);
        self.watches
            .get(watch)
            .map_or(f64::NAN, |height_m| sample.height_above_ground_m - height_m)
    }
}

/// The recovery devices of a flight and their progress through it.
#[derive(Debug, Clone, Copy)]
struct Canopies<'a> {
    devices: &'a [Device],
    run: &'a Run,
    /// The body whose devices act, when a separation means only some of them do. `None` is all
    /// of them, which is what a flight without a separation has.
    body: Option<usize>,
}

impl Canopies<'_> {
    /// The drag area at `t` of the devices that act on what is being flown, m². It has to match
    /// the loop's `ascent_drag_area_m2`, because this is what the equations and the recorded rows
    /// see.
    fn drag_area_m2(&self, t: f64) -> f64 {
        match self.body {
            Some(body) => self.run.body_drag_area_m2(self.devices, body, t),
            None => self.run.drag_area_m2(self.devices, t),
        }
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
    vehicle: &'a Vehicle,
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
            self.vehicle,
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
            Some(
                Watch::RailStall
                | Watch::Apogee
                | Watch::Ground
                | Watch::Altitude(_)
                | Watch::SeparationHeight
                | Watch::EjectionHeight(_)
                | Watch::ShiftHeight(_),
            ) => Direction::Falling,
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
            Watch::SeparationHeight | Watch::EjectionHeight(_) | Watch::ShiftHeight(_) => {
                let trigger = match watch {
                    Watch::EjectionHeight(index) => {
                        self.simulation.ejections.get(index).map(|e| e.trigger)
                    }
                    Watch::ShiftHeight(index) => {
                        self.simulation.shifts.get(index).map(|s| s.trigger)
                    }
                    _ => self.simulation.separation.map(|s| s.trigger),
                };
                let height_m = match trigger {
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
            vehicle: self.vehicle,
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
    vehicle: &'a Vehicle,
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
            self.vehicle,
            self.phase,
            self.window,
            t_s,
            &self.step.state_at(t_s),
            self.canopies.drag_area_m2(t_s),
        )
    }

    fn stability(&self, t_s: f64) -> Result<Stability, SimError> {
        let e = self.simulation.evaluate(
            self.vehicle,
            self.phase,
            self.window,
            t_s,
            &self.step.state_at(t_s),
            self.canopies.drag_area_m2(t_s),
        )?;
        crate::metrics::stability(
            &self.vehicle.aero,
            t_s,
            e.height_above_ground_m,
            e.dynamic_pressure_pa,
            -e.mass.cg_m.z,
            e.mach,
        )
    }
}
