//! Monte Carlo dispersion: one rocket flown many times, each time with its uncertain inputs drawn
//! afresh, to see how far its apogee and landing spread.
//!
//! **Guide:** [Monte Carlo dispersion][guide] explains what each dispersion does, with a worked
//! run, and how far to trust the spread it gives.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/monte-carlo.html
//!
//! A [`MonteCarlo`] holds the nominal flight ([`FlightInputs`]) and a [`Dispersion`]: a standard
//! deviation for each uncertain input. Each sample draws every dispersed input from a normal
//! distribution about its nominal value, as RocketPy's stochastic classes do by default
//! (RocketPy 1.13.0's `rocketpy/stochastic/stochastic_model.py:190-199`, a `(nominal, standard deviation)` pair is a normal
//! distribution), flies the flight, and keeps what it drew and what the flight came to
//! ([`Sample`]). A [`Run`] is the samples in order; [`Run::apogee`] is the spread of their
//! apogees, with the samples that failed counted, not dropped.
//!
//! # Reproducibility
//!
//! Every number a sample draws comes from its own stream, keyed by the run's seed, the sample's
//! index, the input and, for an input with several copies, which copy
//! ([`SeededRng::for_stream`]). So sample `k` is the same flight whatever the number of samples,
//! however they are spread over threads (`MonteCarlo::run_parallel`, with the `parallel`
//! feature), and whichever other inputs are dispersed: turning on a drag dispersion doesn't
//! change the wind a sample flies. On one platform a run is bit for bit the same every time.
//!
//! # What each dispersion does
//!
//! Each is a standard deviation, zero (the default) for an input left at its nominal value. A
//! dispersion of zero draws nothing and changes nothing, so a run with no dispersion flies the
//! nominal flight in every sample, bit for bit.
//!
//! | Field | Each sample flies |
//! |---|---|
//! | `dry_mass_sd_fraction` | each stage's mass without motors times `1 + σ z`, its inertia scaled with it ([`hpr_design::Overrides`]) |
//! | `cg_sd_m` | each stage's centre of mass moved `σ z` aft (forward when negative), its inertia about the centre kept |
//! | `drag_sd_fraction` | the rocket's zero-lift drag coefficient times `1 + σ z` ([`Simulation::with_drag_scale`]) |
//! | `impulse_sd_fraction` | each motor's thrust and propellant mass both times `1 + σ z`, so its specific impulse is kept ([`dispersed_motor`]) |
//! | `burn_time_sd_fraction` | each motor's thrust curve stretched in time by `1 + σ z`, its thrust divided by the same, so its impulse is kept |
//! | `ejection_delay_sd_s` | each motor's ejection delay plus `σ z` seconds, not below zero |
//! | `wind_speed_sd_fraction` | the wind at every height times `1 + σ z`, calm below zero ([`DispersedWind`]) |
//! | `wind_heading_sd_rad` | the wind at every height turned `σ z` clockwise, about the nominal (forecast) direction |
//! | `rail_elevation_sd_rad` | the rail's angle above the horizon plus `σ z`; past vertical it leans the other way |
//! | `rail_azimuth_sd_rad` | the rail's heading plus `σ z`, about the nominal heading |
//! | `deployment_lag_sd_s` | each recovery device's lag after its trigger plus `σ z` seconds, not below zero |
//!
//! `z` is a standard normal deviate drawn for that sample, input and copy: a stage, a motor in the
//! flown configuration (a cluster's motors share one), or a recovery device. A draw that leaves
//! an input impossible (a negative mass, a rail below the horizon) fails that sample, which is
//! counted in the run ([`Outcome::Failed`]). The two delays and the wind's speed are cut at zero
//! instead, since a charge can't fire before its event and a wind can't blow at less than calm: a
//! normal tail past zero becomes zero.
//!
//! # Left out
//!
//! Dispersions are independent normals: no correlations between inputs, no other
//! distributions. The rail's elevation is dispersed in the plane of its heading, so a vertical
//! rail with only its elevation dispersed leans along one line, as RocketPy's does (an inclination
//! and a heading, `rocketpy/stochastic/stochastic_flight.py:21-24`); a draw past vertical leans
//! it the other way, so its [`Draw`] entry is not then the elevation flown. A cluster's motors are dispersed as one. Moving a stage's centre of mass keeps
//! its inertia about the centre. The drag scale multiplies the zero-lift drag only, not the
//! normal force or the moments. A thrust curve stretched in time keeps its shape. Nothing is
//! dispersed in the atmosphere's temperature or pressure, a motor's ignition time, a recovery
//! device's drag, or anything of a flight's events, separations and staging that [`FlightInputs`]
//! doesn't hold.

use std::f64::consts::{FRAC_PI_2, PI};
use std::sync::Arc;

use hpr_aero::DragModel;
use hpr_aero::table::DragTable;
use hpr_atmos::{AtmosError, Wind, WindSample};
use hpr_core::DVec3;
use hpr_core::random::SeededRng;
use hpr_design::Rocket;
use hpr_motor::motor::{Propellant, PropellantColumn};
use hpr_motor::{BatesGrains, Delay, SolidMotor, ThrustCurve};
use hpr_sim::{
    Device, Environment, FlightMetrics, FlightSettings, FlightSummary, Rail, SimError, Simulation,
};
use serde::{Deserialize, Serialize};

use crate::error::AnalysisError;
use crate::statistics::Distribution;

/// A drag override for the whole flight, in place of hpr's drag buildup: another tool's table or
/// a model of your own ([`Simulation::with_drag_table`], [`Simulation::with_shared_drag_model`]).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum DragOverride {
    /// A `C_D0(M)` table.
    Table(DragTable),
    /// A drag model.
    Model(Arc<dyn DragModel>),
}

/// Everything one flight is flown from: the inputs of [`Simulation::new`] and the options a
/// Monte Carlo run disperses. `hpr::FlightBuilder::inputs` makes one from the facade's builder.
#[derive(Debug, Clone)]
pub struct FlightInputs {
    /// The design.
    pub rocket: Rocket,
    /// The id of the configuration flown: which motors are in it.
    pub configuration_id: String,
    /// The atmosphere and the wind.
    pub environment: Environment,
    /// The launch rail.
    pub rail: Rail,
    /// The integrator's settings.
    pub settings: FlightSettings,
    /// The recovery devices, none for a ballistic flight ([`Simulation::with_recovery`]).
    pub recovery: Vec<Device>,
    /// A drag table or model in place of hpr's drag buildup, if any.
    pub drag: Option<DragOverride>,
    /// The factor on the rocket's zero-lift drag ([`Simulation::with_drag_scale`]); 1 leaves it.
    pub drag_scale: f64,
}

impl FlightInputs {
    /// The inputs of a flight of `rocket`'s configuration `configuration_id`, with the default
    /// integrator, no recovery, hpr's own drag and no drag scale.
    pub fn new(
        rocket: Rocket,
        configuration_id: impl Into<String>,
        environment: Environment,
        rail: Rail,
    ) -> Self {
        Self {
            rocket,
            configuration_id: configuration_id.into(),
            environment,
            rail,
            settings: FlightSettings::default(),
            recovery: Vec::new(),
            drag: None,
            drag_scale: 1.0,
        }
    }

    /// The simulation these inputs fly.
    ///
    /// # Errors
    ///
    /// As [`Simulation::new`], [`Simulation::with_drag_scale`] (a scale that is negative or not
    /// finite) and [`Simulation::with_recovery`].
    pub fn simulation(&self) -> Result<Simulation, SimError> {
        let mut simulation = Simulation::new(
            &self.rocket,
            &self.configuration_id,
            self.environment.clone(),
            self.rail,
            self.settings,
        )?;
        match &self.drag {
            Some(DragOverride::Table(table)) => {
                simulation = simulation.with_drag_table(table.clone());
            }
            Some(DragOverride::Model(model)) => {
                simulation = simulation.with_shared_drag_model(Arc::clone(model));
            }
            None => {}
        }
        if self.drag_scale != 1.0 {
            simulation = simulation.with_drag_scale(self.drag_scale)?;
        }
        if self.recovery.is_empty() {
            Ok(simulation)
        } else {
            simulation.with_recovery(self.recovery.clone())
        }
    }

    /// Flies the flight to the ground and gives its metrics.
    ///
    /// # Errors
    ///
    /// As [`FlightInputs::simulation`], [`Simulation::run`] and
    /// [`FlightMetrics::summary`].
    pub fn fly(&self) -> Result<FlightSummary, SimError> {
        let simulation = self.simulation()?;
        let mut metrics = FlightMetrics::new();
        let result = simulation.run(&mut metrics)?;
        metrics.summary(&result, &self.environment)
    }
}

/// The standard deviation of each dispersed input; zero, the default, leaves an input at its
/// nominal value. The module's docs say what each does to a flight.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Dispersion {
    /// Each stage's mass without motors, as a fraction of it (0.02 is 2%).
    pub dry_mass_sd_fraction: f64,
    /// Each stage's centre of mass along the axis, m.
    pub cg_sd_m: f64,
    /// The rocket's zero-lift drag coefficient, as a fraction of it.
    pub drag_sd_fraction: f64,
    /// Each motor's total impulse, as a fraction of it, its propellant mass with it.
    pub impulse_sd_fraction: f64,
    /// Each motor's burn time, as a fraction of it, at the same impulse.
    pub burn_time_sd_fraction: f64,
    /// Each motor's ejection delay, s.
    pub ejection_delay_sd_s: f64,
    /// The wind's speed at every height, as a fraction of it.
    pub wind_speed_sd_fraction: f64,
    /// The wind's direction, rad, about the nominal one.
    pub wind_heading_sd_rad: f64,
    /// The rail's angle above the horizon, rad.
    pub rail_elevation_sd_rad: f64,
    /// The rail's heading, rad.
    pub rail_azimuth_sd_rad: f64,
    /// Each recovery device's lag after its trigger, s.
    pub deployment_lag_sd_s: f64,
}

impl Dispersion {
    /// The fields with their names, for checks.
    fn named(&self) -> [(&'static str, f64); 11] {
        [
            ("dry-mass standard deviation", self.dry_mass_sd_fraction),
            ("centre-of-mass standard deviation (m)", self.cg_sd_m),
            ("drag standard deviation", self.drag_sd_fraction),
            ("impulse standard deviation", self.impulse_sd_fraction),
            ("burn-time standard deviation", self.burn_time_sd_fraction),
            (
                "ejection-delay standard deviation (s)",
                self.ejection_delay_sd_s,
            ),
            ("wind-speed standard deviation", self.wind_speed_sd_fraction),
            (
                "wind-heading standard deviation (rad)",
                self.wind_heading_sd_rad,
            ),
            (
                "rail-elevation standard deviation (rad)",
                self.rail_elevation_sd_rad,
            ),
            (
                "rail-azimuth standard deviation (rad)",
                self.rail_azimuth_sd_rad,
            ),
            (
                "deployment-lag standard deviation (s)",
                self.deployment_lag_sd_s,
            ),
        ]
    }

    /// Checks that every standard deviation is finite and not negative.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] naming the first that isn't.
    pub fn validate(&self) -> Result<(), AnalysisError> {
        for (what, value) in self.named() {
            if !(value.is_finite() && value >= 0.0) {
                return Err(AnalysisError::Domain { what, value });
            }
        }
        Ok(())
    }
}

/// The dispersed inputs, each with its own stream ([`SeededRng::for_stream`]). The numbers are
/// part of every seeded result: changing one changes the samples drawn.
#[derive(Debug, Clone, Copy)]
enum Input {
    DryMass = 1,
    CentreOfMass = 2,
    Drag = 3,
    Impulse = 4,
    BurnTime = 5,
    EjectionDelay = 6,
    WindSpeed = 7,
    WindHeading = 8,
    RailElevation = 9,
    RailAzimuth = 10,
    DeploymentLag = 11,
}

/// What one sample drew: the factor or offset for each dispersed input, and its nominal value
/// (1 or 0) for one left alone. The lists run over the design's stages, the flown
/// configuration's motors and the recovery devices, in order. [`MonteCarlo::inputs`] turns a
/// draw into the flight it flies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Draw {
    /// Each stage's mass factor.
    pub dry_mass_scale: Vec<f64>,
    /// Each stage's centre-of-mass shift, m aft.
    pub cg_shift_m: Vec<f64>,
    /// The drag factor.
    pub drag_scale: f64,
    /// Each motor's impulse factor.
    pub impulse_scale: Vec<f64>,
    /// Each motor's burn-time factor.
    pub burn_time_scale: Vec<f64>,
    /// Each motor's ejection-delay offset, s, before the cut at zero.
    pub ejection_delay_offset_s: Vec<f64>,
    /// The wind-speed factor, before the cut at zero.
    pub wind_speed_scale: f64,
    /// The wind's turn, rad clockwise.
    pub wind_turn_rad: f64,
    /// The rail's elevation offset, rad.
    pub rail_elevation_offset_rad: f64,
    /// The rail's heading offset, rad clockwise.
    pub rail_azimuth_offset_rad: f64,
    /// Each recovery device's lag offset, s, before the cut at zero.
    pub deployment_lag_offset_s: Vec<f64>,
}

/// What a sample's flight came to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// It flew to the ground.
    Flown {
        /// Its metrics.
        summary: Box<FlightSummary>,
    },
    /// Its inputs were refused or the flight stopped with an error.
    Failed {
        /// Where: making its inputs from the draw, or flying them.
        at: FailedAt,
        /// The error.
        reason: String,
    },
}

/// Where a sample failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailedAt {
    /// Making its inputs: the draw left one impossible, such as a motor with no impulse
    /// ([`MonteCarlo::inputs`]).
    Inputs,
    /// Flying them: the flight refused them, such as a negative mass or a rail below the
    /// horizon, or stopped with an error ([`FlightInputs::fly`]).
    Flight,
}

/// One sample of a run: its index, what it drew, and its outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// Its index in the run, from 0.
    pub index: u64,
    /// What it drew.
    pub draw: Draw,
    /// What its flight came to.
    pub outcome: Outcome,
}

impl Sample {
    /// The flight's metrics, `None` for a sample that failed.
    pub fn summary(&self) -> Option<&FlightSummary> {
        match &self.outcome {
            Outcome::Flown { summary } => Some(summary),
            Outcome::Failed { .. } => None,
        }
    }
}

/// The samples of a run, in order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    /// The seed.
    pub seed: u64,
    /// The samples, by index.
    pub samples: Vec<Sample>,
}

impl Run {
    /// The samples that failed.
    pub fn failed(&self) -> impl Iterator<Item = &Sample> {
        self.samples
            .iter()
            .filter(|sample| matches!(sample.outcome, Outcome::Failed { .. }))
    }

    /// The distribution of `value` over every sample: a failed sample, or one for which `value`
    /// gives `None`, counts as tried with no value.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a value that isn't finite.
    pub fn distribution(
        &self,
        value: impl Fn(&FlightSummary) -> Option<f64>,
    ) -> Result<Distribution, AnalysisError> {
        let values = self
            .samples
            .iter()
            .filter_map(|sample| sample.summary().and_then(&value))
            .collect();
        Distribution::new(values, self.samples.len())
    }

    /// The distribution of the apogee's height above the ground, m.
    ///
    /// # Errors
    ///
    /// As [`Run::distribution`].
    pub fn apogee(&self) -> Result<Distribution, AnalysisError> {
        self.distribution(|summary| {
            summary
                .apogee
                .as_ref()
                .map(|apogee| apogee.height_above_ground_m)
        })
    }
}

/// A stage's nominal mass without motors and its centre, as the layout places them.
#[derive(Debug, Clone, Copy)]
struct StageMass {
    mass_kg: f64,
    /// Aft of the stage's forward end, m: what [`hpr_design::Overrides::cg_aft_m`] sets.
    cg_aft_m: f64,
}

/// A rocket's nominal flight and the dispersion of its inputs, ready to fly samples.
#[derive(Debug, Clone)]
pub struct MonteCarlo {
    nominal: FlightInputs,
    dispersion: Dispersion,
    stages: Vec<StageMass>,
    configuration: usize,
}

impl MonteCarlo {
    /// A run of `nominal` dispersed by `dispersion`.
    ///
    /// # Errors
    ///
    /// - As [`Dispersion::validate`].
    /// - [`AnalysisError::NoConfiguration`] if the design has no configuration of that id.
    /// - [`AnalysisError::Design`] if the design can't be laid out.
    /// - [`AnalysisError::Unsupported`] for a dispersed impulse or burn time on a motor whose
    ///   propellant model [`dispersed_motor`] doesn't know.
    pub fn new(nominal: FlightInputs, dispersion: Dispersion) -> Result<Self, AnalysisError> {
        dispersion.validate()?;
        let configuration = nominal
            .rocket
            .configurations
            .iter()
            .position(|c| c.id == nominal.configuration_id)
            .ok_or_else(|| AnalysisError::NoConfiguration(nominal.configuration_id.clone()))?;
        let stages = nominal
            .rocket
            .layout()?
            .stages
            .iter()
            .map(|stage| StageMass {
                mass_kg: stage.mass.mass_kg,
                // Stations run aft from the nose; body z runs forward ([`hpr_design::Overrides`]).
                cg_aft_m: -stage.mass.cg_m.z - stage.fore_station_m,
            })
            .collect();
        if dispersion.impulse_sd_fraction > 0.0 || dispersion.burn_time_sd_fraction > 0.0 {
            for mounted in &nominal.rocket.configurations[configuration].motors {
                dispersed_motor(&mounted.motor, 1.0, 1.0)?;
            }
        }
        Ok(Self {
            nominal,
            dispersion,
            stages,
            configuration,
        })
    }

    /// The nominal flight.
    pub fn nominal(&self) -> &FlightInputs {
        &self.nominal
    }

    /// The dispersion.
    pub fn dispersion(&self) -> &Dispersion {
        &self.dispersion
    }

    /// What sample `index` of a run seeded with `seed` draws.
    pub fn draw(&self, seed: u64, index: u64) -> Draw {
        let d = &self.dispersion;
        let normal = |input: Input, copy: usize, sd: f64| {
            if sd > 0.0 {
                // Cast: a copy's index is far below 2⁶⁴.
                sd * SeededRng::for_stream(seed, &[index, input as u64, copy as u64])
                    .standard_normal()
            } else {
                0.0
            }
        };
        let per = |count: usize, input: Input, sd: f64, base: f64| -> Vec<f64> {
            (0..count)
                .map(|copy| base + normal(input, copy, sd))
                .collect()
        };
        let stages = self.nominal.rocket.stages.len();
        let motors = self.nominal.rocket.configurations[self.configuration]
            .motors
            .len();
        let devices = self.nominal.recovery.len();
        Draw {
            dry_mass_scale: per(stages, Input::DryMass, d.dry_mass_sd_fraction, 1.0),
            cg_shift_m: per(stages, Input::CentreOfMass, d.cg_sd_m, 0.0),
            drag_scale: 1.0 + normal(Input::Drag, 0, d.drag_sd_fraction),
            impulse_scale: per(motors, Input::Impulse, d.impulse_sd_fraction, 1.0),
            burn_time_scale: per(motors, Input::BurnTime, d.burn_time_sd_fraction, 1.0),
            ejection_delay_offset_s: per(motors, Input::EjectionDelay, d.ejection_delay_sd_s, 0.0),
            wind_speed_scale: 1.0 + normal(Input::WindSpeed, 0, d.wind_speed_sd_fraction),
            wind_turn_rad: normal(Input::WindHeading, 0, d.wind_heading_sd_rad),
            rail_elevation_offset_rad: normal(Input::RailElevation, 0, d.rail_elevation_sd_rad),
            rail_azimuth_offset_rad: normal(Input::RailAzimuth, 0, d.rail_azimuth_sd_rad),
            deployment_lag_offset_s: per(devices, Input::DeploymentLag, d.deployment_lag_sd_s, 0.0),
        }
    }

    /// The flight inputs `draw` gives: the nominal ones, with each input whose entry isn't its
    /// nominal value (a factor of 1, an offset of 0) changed as the module's docs say. An entry at
    /// its nominal value leaves its input as it is, so a draw with no dispersion flies the
    /// nominal flight bit for bit; any other entry is applied, whatever the dispersion, so a
    /// draw read back or written by hand flies as it says.
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::Count`] for a list whose length isn't the design's number of stages,
    ///   the configuration's number of motors or the number of recovery devices.
    /// - [`AnalysisError::Domain`] for an entry that isn't finite, and from
    ///   [`dispersed_motor`] for an impulse or burn-time factor at or below zero.
    /// - [`AnalysisError::Unsupported`] and [`AnalysisError::Motor`] as [`dispersed_motor`].
    pub fn inputs(&self, draw: &Draw) -> Result<FlightInputs, AnalysisError> {
        self.check(draw)?;
        let mut inputs = self.nominal.clone();
        for ((stage, nominal), (&scale, &shift)) in inputs
            .rocket
            .stages
            .iter_mut()
            .zip(&self.stages)
            .zip(draw.dry_mass_scale.iter().zip(&draw.cg_shift_m))
        {
            if scale != 1.0 {
                stage.overrides.mass_kg = Some(nominal.mass_kg * scale);
                // An inertia the design sets replaces the scaled one, so it scales too.
                if let Some(inertia) = &mut stage.overrides.inertia {
                    for value in [
                        &mut inertia.xx_kg_m2,
                        &mut inertia.yy_kg_m2,
                        &mut inertia.zz_kg_m2,
                        &mut inertia.xy_kg_m2,
                        &mut inertia.xz_kg_m2,
                        &mut inertia.yz_kg_m2,
                    ] {
                        *value *= scale;
                    }
                }
            }
            if shift != 0.0 {
                stage.overrides.cg_aft_m = Some(nominal.cg_aft_m + shift);
            }
        }
        let motors = &mut inputs.rocket.configurations[self.configuration].motors;
        for (mounted, ((&impulse, &burn), &delay)) in motors.iter_mut().zip(
            draw.impulse_scale
                .iter()
                .zip(&draw.burn_time_scale)
                .zip(&draw.ejection_delay_offset_s),
        ) {
            if impulse != 1.0 || burn != 1.0 {
                mounted.motor = dispersed_motor(&mounted.motor, impulse, burn)?;
            }
            if delay != 0.0
                && let Some(Delay::Seconds(delay_s)) = mounted.delay
            {
                mounted.delay = Some(Delay::Seconds((delay_s + delay).max(0.0)));
            }
        }
        if draw.drag_scale != 1.0 {
            inputs.drag_scale *= draw.drag_scale;
        }
        if draw.wind_speed_scale != 1.0 || draw.wind_turn_rad != 0.0 {
            inputs.environment.wind = Arc::new(DispersedWind::new(
                Arc::clone(&inputs.environment.wind),
                draw.wind_speed_scale.max(0.0),
                draw.wind_turn_rad,
            )?);
        }
        if draw.rail_elevation_offset_rad != 0.0 || draw.rail_azimuth_offset_rad != 0.0 {
            inputs.rail = tilted(
                inputs.rail,
                draw.rail_elevation_offset_rad,
                draw.rail_azimuth_offset_rad,
            );
        }
        for (device, &offset) in inputs
            .recovery
            .iter_mut()
            .zip(&draw.deployment_lag_offset_s)
        {
            if offset != 0.0 {
                device.lag_s = (device.lag_s + offset).max(0.0);
            }
        }
        Ok(inputs)
    }

    /// Checks that `draw` has an entry for each stage, motor and recovery device, and that every
    /// entry is finite.
    fn check(&self, draw: &Draw) -> Result<(), AnalysisError> {
        let stages = self.nominal.rocket.stages.len();
        let motors = self.nominal.rocket.configurations[self.configuration]
            .motors
            .len();
        let devices = self.nominal.recovery.len();
        let lists: [(&'static str, &[f64], usize); 6] = [
            (
                "dry-mass factors, against the stages",
                &draw.dry_mass_scale,
                stages,
            ),
            (
                "centre-of-mass shifts, against the stages",
                &draw.cg_shift_m,
                stages,
            ),
            (
                "impulse factors, against the motors",
                &draw.impulse_scale,
                motors,
            ),
            (
                "burn-time factors, against the motors",
                &draw.burn_time_scale,
                motors,
            ),
            (
                "ejection-delay offsets, against the motors",
                &draw.ejection_delay_offset_s,
                motors,
            ),
            (
                "deployment-lag offsets, against the recovery devices",
                &draw.deployment_lag_offset_s,
                devices,
            ),
        ];
        for (what, list, count) in lists {
            if list.len() != count {
                return Err(AnalysisError::Length {
                    what,
                    length: list.len(),
                    expected: count,
                });
            }
            if let Some(&bad) = list.iter().find(|v| !v.is_finite()) {
                return Err(AnalysisError::Domain {
                    what: "entry of a draw",
                    value: bad,
                });
            }
        }
        for value in [
            draw.drag_scale,
            draw.wind_speed_scale,
            draw.wind_turn_rad,
            draw.rail_elevation_offset_rad,
            draw.rail_azimuth_offset_rad,
        ] {
            if !value.is_finite() {
                return Err(AnalysisError::Domain {
                    what: "entry of a draw",
                    value,
                });
            }
        }
        Ok(())
    }

    /// Draws and flies sample `index` of a run seeded with `seed`. A draw that makes an input
    /// impossible, or a flight that stops with an error, is a failed sample, not an error.
    pub fn sample(&self, seed: u64, index: u64) -> Sample {
        let draw = self.draw(seed, index);
        let outcome = match self.inputs(&draw) {
            Err(error) => Outcome::Failed {
                at: FailedAt::Inputs,
                reason: error.to_string(),
            },
            Ok(inputs) => match inputs.fly() {
                Ok(summary) => Outcome::Flown {
                    summary: Box::new(summary),
                },
                Err(error) => Outcome::Failed {
                    at: FailedAt::Flight,
                    reason: error.to_string(),
                },
            },
        };
        Sample {
            index,
            draw,
            outcome,
        }
    }

    /// Flies samples `0..count` of a run seeded with `seed`, one after another.
    pub fn run(&self, seed: u64, count: u64) -> Run {
        Run {
            seed,
            samples: (0..count).map(|index| self.sample(seed, index)).collect(),
        }
    }

    /// As [`MonteCarlo::run`], with the samples spread over the threads of the current rayon
    /// pool: the global one, or one a caller's `rayon::ThreadPool::install` sets up to choose
    /// how many. The run is the same, bit for bit, whatever the number of threads.
    #[cfg(feature = "parallel")]
    pub fn run_parallel(&self, seed: u64, count: u64) -> Run {
        use rayon::prelude::*;
        Run {
            seed,
            samples: (0..count)
                .into_par_iter()
                .map(|index| self.sample(seed, index))
                .collect(),
        }
    }
}

/// `motor` with its total impulse times `impulse_scale` and its burn time times
/// `burn_time_scale`: thrust `F′(t) = (k/s) F(t/s)` and propellant mass `k m_p`, with `k` and `s`
/// the two factors. The impulse is `k I` and the effective exhaust velocity `I/m_p` (the specific
/// impulse) is kept, as a motor of the same propellant burning more or less of it would; the dry
/// mass, the nozzle and the propellant's shape are kept. A column's mass is scaled; BATES grains'
/// density, so their geometry and regression are kept.
///
/// # Errors
///
/// - [`AnalysisError::Domain`] for a factor that isn't finite and positive.
/// - [`AnalysisError::Unsupported`] for a propellant model this doesn't know.
/// - As [`SolidMotor::new`].
pub fn dispersed_motor(
    motor: &SolidMotor,
    impulse_scale: f64,
    burn_time_scale: f64,
) -> Result<SolidMotor, AnalysisError> {
    for (what, value) in [
        ("impulse factor", impulse_scale),
        ("burn-time factor", burn_time_scale),
    ] {
        if !(value.is_finite() && value > 0.0) {
            return Err(AnalysisError::Domain { what, value });
        }
    }
    let curve = motor.curve();
    let thrust_scale = impulse_scale / burn_time_scale;
    let curve = ThrustCurve::new(
        curve
            .times_s()
            .iter()
            .map(|t| t * burn_time_scale)
            .collect(),
        curve.thrusts_n().iter().map(|f| f * thrust_scale).collect(),
    )?;
    let propellant = match motor.propellant() {
        Propellant::Column(column) => Propellant::Column(PropellantColumn {
            mass_kg: column.mass_kg * impulse_scale,
            ..*column
        }),
        Propellant::Grains(grains) => Propellant::Grains(BatesGrains {
            density_kg_m3: grains.density_kg_m3 * impulse_scale,
            ..*grains
        }),
        other => {
            return Err(AnalysisError::Unsupported(format!(
                "dispersing the impulse of a motor whose propellant is {other:?}"
            )));
        }
    };
    Ok(SolidMotor::new(
        curve,
        propellant,
        motor.dry(),
        motor.nozzle(),
    )?)
}

/// `rail` with its elevation and heading moved by the offsets. An elevation past vertical leans
/// the other way: `E′ = π − E`, the heading and roll turned half a turn, the same attitude
/// (`R_z(−A) R_x(E − π/2) R_z(φ)`, `hpr_core::frames::LaunchAngles`).
fn tilted(rail: Rail, elevation_offset_rad: f64, azimuth_offset_rad: f64) -> Rail {
    let mut tilted = rail;
    tilted.elevation_rad += elevation_offset_rad;
    tilted.azimuth_rad += azimuth_offset_rad;
    if tilted.elevation_rad > FRAC_PI_2 {
        tilted.elevation_rad = PI - tilted.elevation_rad;
        tilted.azimuth_rad += PI;
        tilted.roll_rad += PI;
    }
    tilted
}

/// A wind model's wind, scaled and turned: at every height the velocity is `speed_scale` times
/// the base model's, its horizontal part turned `turn_rad` clockwise seen from above. Turning
/// the velocity turns the direction the wind blows from by the same angle, so a dispersed heading
/// stays about the forecast's.
#[derive(Debug, Clone)]
pub struct DispersedWind {
    base: Arc<dyn Wind>,
    speed_scale: f64,
    turn_rad: f64,
}

impl DispersedWind {
    /// `base` scaled by `speed_scale` and turned `turn_rad` clockwise.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a scale that is negative or not finite, or a turn that isn't
    /// finite.
    pub fn new(
        base: Arc<dyn Wind>,
        speed_scale: f64,
        turn_rad: f64,
    ) -> Result<Self, AnalysisError> {
        if !(speed_scale.is_finite() && speed_scale >= 0.0) {
            return Err(AnalysisError::Domain {
                what: "wind-speed factor",
                value: speed_scale,
            });
        }
        if !turn_rad.is_finite() {
            return Err(AnalysisError::Domain {
                what: "wind turn (rad)",
                value: turn_rad,
            });
        }
        Ok(Self {
            base,
            speed_scale,
            turn_rad,
        })
    }
}

impl Wind for DispersedWind {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        let mut sample = self.base.wind(height_msl_m)?;
        let v = sample.velocity_enu_m_s;
        let (sin, cos) = self.turn_rad.sin_cos();
        // A bearing θ clockwise from north is (sin θ, cos θ) in east and north; turning it to
        // θ + δ gives these.
        sample.velocity_enu_m_s =
            DVec3::new(v.x * cos + v.y * sin, v.y * cos - v.x * sin, v.z) * self.speed_scale;
        Ok(sample)
    }
}

#[cfg(test)]
mod tests {
    use hpr_atmos::ConstantWind;
    use hpr_core::frames::LaunchAngles;
    use hpr_core::geodesy::Geodetic;
    use hpr_sim::{DeviceDrag, Trigger};

    use super::*;

    const SEED: u64 = 20_261_001;
    /// `SeededRng::for_stream(42, &[7]).next_u64()`.
    const GOLDEN_STREAM: u64 = 2_627_254_379_500_910_771;
    /// Sample 0's drag and impulse factors with `every_dispersion()` and `SEED`, as macOS
    /// computes them: a normal deviate takes a logarithm, which another platform's library may
    /// round differently in the last bit, so these are held to 1e-14.
    const GOLDEN_DRAG: f64 = 1.012_497_722_244_850_1;
    const GOLDEN_IMPULSE: f64 = 1.020_485_907_939_679_7;

    fn valetudo() -> Rocket {
        serde_json::from_str(include_str!(
            "../../../validation/designs/rocketpy-valetudo.json"
        ))
        .unwrap()
    }

    /// Valetudo in a 5 m/s wind from the west, off a 5.2 m rail at 85° heading north, with a
    /// drogue at apogee.
    fn nominal() -> FlightInputs {
        let site = Geodetic::from_degrees(32.99, -106.97, 1400.0).unwrap();
        let environment = Environment::standard(site)
            .unwrap()
            .with_wind(ConstantWind::new(5.0, 1.5 * PI).unwrap());
        let mut rail = Rail::vertical(5.2);
        rail.elevation_rad = 85_f64.to_radians();
        let mut inputs = FlightInputs::new(valetudo(), "example", environment, rail);
        inputs.recovery = vec![
            Device::new(
                "drogue",
                DeviceDrag::DragArea { cd_s_m2: 0.3 },
                Trigger::Apogee,
            )
            .with_lag_s(1.0),
        ];
        inputs
    }

    fn every_dispersion() -> Dispersion {
        Dispersion {
            dry_mass_sd_fraction: 0.02,
            cg_sd_m: 0.01,
            drag_sd_fraction: 0.05,
            impulse_sd_fraction: 0.03,
            burn_time_sd_fraction: 0.03,
            ejection_delay_sd_s: 0.5,
            wind_speed_sd_fraction: 0.2,
            wind_heading_sd_rad: 10_f64.to_radians(),
            rail_elevation_sd_rad: 1_f64.to_radians(),
            rail_azimuth_sd_rad: 2_f64.to_radians(),
            deployment_lag_sd_s: 0.3,
        }
    }

    /// Loft lesson L52: Loft scaled the thrust but not the propellant mass, so its dispersed
    /// motors burned with a different specific impulse. Here both scale, and the effective
    /// exhaust velocity `I/m_p` is the nominal motor's.
    #[test]
    fn impulse_dispersion_preserves_specific_impulse() {
        let nominal = nominal();
        let motor = &nominal.rocket.configurations[0].motors[0].motor.clone();
        let impulse = motor.curve().total_impulse_ns();
        let exhaust = impulse / motor.propellant_initial_mass_kg();
        let burn = motor.curve().burn_time_s();
        for (k, s) in [(1.1, 1.0), (0.9, 1.2), (1.05, 0.8)] {
            let dispersed = dispersed_motor(motor, k, s).unwrap();
            let new_impulse = dispersed.curve().total_impulse_ns();
            let new_exhaust = new_impulse / dispersed.propellant_initial_mass_kg();
            assert!((new_impulse / impulse - k).abs() < 1e-14, "{k} {s}");
            assert!((new_exhaust / exhaust - 1.0).abs() < 1e-14, "{k} {s}");
            assert!(
                (dispersed.curve().burn_time_s() / burn - s).abs() < 1e-12,
                "{k} {s}"
            );
            assert_eq!(dispersed.dry(), motor.dry());
        }
        // And in a run: every sample's motor keeps it.
        let run = MonteCarlo::new(nominal, every_dispersion()).unwrap();
        for index in 0..20 {
            let draw = run.draw(SEED, index);
            let inputs = run.inputs(&draw).unwrap();
            let flown = &inputs.rocket.configurations[0].motors[0].motor;
            let flown_exhaust =
                flown.curve().total_impulse_ns() / flown.propellant_initial_mass_kg();
            assert!((flown_exhaust / exhaust - 1.0).abs() < 1e-14);
            // Each factor where it belongs: swapped, `I/m_p` would still hold.
            let flown_impulse = flown.curve().total_impulse_ns() / impulse;
            let flown_burn = flown.curve().burn_time_s() / burn;
            assert!((flown_impulse - draw.impulse_scale[0]).abs() < 1e-14);
            assert!((flown_burn - draw.burn_time_scale[0]).abs() < 1e-12);
            assert_ne!(draw.impulse_scale[0], draw.burn_time_scale[0]);
        }
        // A motor known only by its envelope holds its propellant as a column; its mass scales.
        let column =
            SolidMotor::from_envelope(motor.curve().clone(), 0.075, 0.6, 2.0, 3.5).unwrap();
        assert!(matches!(column.propellant(), Propellant::Column(_)));
        let heavier = dispersed_motor(&column, 1.2, 0.9).unwrap();
        assert!((heavier.propellant_initial_mass_kg() / 2.0 - 1.2).abs() < 1e-15);
        assert_eq!(heavier.dry(), column.dry());
        let ratio = |m: &SolidMotor| m.curve().total_impulse_ns() / m.propellant_initial_mass_kg();
        assert!((ratio(&heavier) / ratio(&column) - 1.0).abs() < 1e-14);
        // A factor at or below zero is refused.
        for bad in [0.0, -0.1, f64::NAN] {
            assert!(matches!(
                dispersed_motor(motor, bad, 1.0),
                Err(AnalysisError::Domain {
                    what: "impulse factor",
                    ..
                })
            ));
            assert!(matches!(
                dispersed_motor(motor, 1.0, bad),
                Err(AnalysisError::Domain {
                    what: "burn-time factor",
                    ..
                })
            ));
        }
    }

    /// Loft lesson L53: Loft drew every sample from one stream, so adding a draw reshuffled every
    /// later sample and parallel runs weren't reproducible. Here sample `k` is the same whatever
    /// the run's length or thread count, and turning another dispersion on leaves its draws.
    #[test]
    fn sample_k_independent_of_n_and_thread_count() {
        let run = MonteCarlo::new(nominal(), every_dispersion()).unwrap();
        let short = run.run(SEED, 3);
        let long = run.run(SEED, 6);
        assert_eq!(short.samples[..], long.samples[..3]);
        assert_eq!(run.sample(SEED, 4), long.samples[4]);
        assert!(short.failed().next().is_none());
        // Another seed, other samples.
        assert_ne!(run.draw(SEED + 1, 0), run.draw(SEED, 0));
        // Each input has its own stream: with the drag dispersion off, the rest draw the same.
        let without_drag = MonteCarlo::new(
            nominal(),
            Dispersion {
                drag_sd_fraction: 0.0,
                ..every_dispersion()
            },
        )
        .unwrap();
        let (all, some) = (run.draw(SEED, 2), without_drag.draw(SEED, 2));
        assert_eq!(some.drag_scale, 1.0);
        assert_eq!(
            Draw {
                drag_scale: 1.0,
                ..all.clone()
            },
            some
        );
        // The run's first samples are a shorter run's, but one shared stream would give that
        // too: what a shared stream can't give is sample 4 flown on its own, above, or the same
        // draws with the drag dispersion off.
        #[cfg(feature = "parallel")]
        for threads in [1, 2, 5] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .unwrap();
            assert_eq!(
                pool.install(|| run.run_parallel(SEED, 6)),
                long,
                "{threads}"
            );
        }
    }

    /// Loft lesson L54: Loft dropped impossible samples silently and computed its probabilities
    /// over the survivors. Here a sample whose draw the flight refuses is kept as failed, with
    /// its reason, and the apogee's distribution counts it as tried with no value.
    #[test]
    fn failed_samples_are_counted_and_reported() {
        // A 60% mass spread draws a negative mass now and then.
        let run = MonteCarlo::new(
            nominal(),
            Dispersion {
                dry_mass_sd_fraction: 0.6,
                ..Dispersion::default()
            },
        )
        .unwrap()
        .run(SEED, 16);
        let failed: Vec<&Sample> = run.failed().collect();
        assert!(!failed.is_empty() && failed.len() < 16, "{}", failed.len());
        for sample in &failed {
            assert!(sample.draw.dry_mass_scale[0] < 0.0, "{sample:?}");
            let Outcome::Failed { at, reason } = &sample.outcome else {
                unreachable!("filtered on failure")
            };
            assert_eq!(*at, FailedAt::Flight);
            assert!(reason.contains("mass"), "{reason}");
        }
        let apogee = run.apogee().unwrap();
        assert_eq!(apogee.attempted(), 16);
        let failed_count = failed.len();
        assert_eq!(apogee.missing(), failed.len());
        // A share over every sample tried, bounded by the failures both ways.
        let share = apogee.share_at_least(0.0).unwrap().unwrap();
        assert_eq!(share.low, (16 - failed_count) as f64 / 16.0);
        assert_eq!(share.high, 1.0);
        // A draw that leaves a motor with no impulse fails before it flies.
        let run = MonteCarlo::new(
            nominal(),
            Dispersion {
                impulse_sd_fraction: 1.0,
                ..Dispersion::default()
            },
        )
        .unwrap()
        .run(SEED, 16);
        let failed: Vec<&Sample> = run.failed().collect();
        assert!(
            !failed.is_empty(),
            "no impulse factor at or below zero in 16"
        );
        for sample in failed {
            assert!(sample.draw.impulse_scale[0] <= 0.0, "{sample:?}");
            assert!(
                matches!(&sample.outcome, Outcome::Failed { at: FailedAt::Inputs, reason }
                    if reason.contains("impulse factor")),
                "{sample:?}"
            );
        }
    }

    /// Loft lesson L55: Loft drew wind and rail bearings uniformly at random, throwing the
    /// forecast heading away. Here the wind turns about the nominal direction, and the rail about
    /// its nominal heading, by normal deviates of the standard deviation asked for.
    #[test]
    fn wind_heading_dispersed_about_nominal() {
        let sd = 10_f64.to_radians();
        let run = MonteCarlo::new(
            nominal(),
            Dispersion {
                wind_heading_sd_rad: sd,
                rail_azimuth_sd_rad: sd,
                ..Dispersion::default()
            },
        )
        .unwrap();
        let n = 2000;
        let mut from = Vec::with_capacity(n);
        let mut headings = Vec::with_capacity(n);
        for index in 0..n as u64 {
            let inputs = run.inputs(&run.draw(SEED, index)).unwrap();
            let wind = inputs
                .environment
                .wind
                .wind(1500.0)
                .unwrap()
                .velocity_enu_m_s;
            // The direction it blows from, clockwise from north, about the nominal west (270°).
            let bearing = (-wind.x).atan2(-wind.y);
            from.push(bearing.rem_euclid(2.0 * PI) - 1.5 * PI);
            assert!((wind.truncate().length() - 5.0).abs() < 1e-12);
            headings.push(inputs.rail.azimuth_rad);
        }
        for turns in [from, headings] {
            let d = Distribution::new(turns, n).unwrap();
            let (mean, spread) = (d.mean().unwrap(), d.standard_deviation().unwrap());
            // Within four standard errors of 0 and of the deviation asked for.
            assert!(mean.abs() < 4.0 * sd / (n as f64).sqrt(), "{mean}");
            assert!(
                (spread / sd - 1.0).abs() < 4.0 / (2.0 * (n as f64 - 1.0)).sqrt(),
                "{spread}"
            );
        }
    }

    /// Loft lesson L96: the same seed gives the same samples, and no dispersion gives the
    /// nominal flight in every sample, so a spread of exactly zero.
    #[test]
    fn zero_dispersion_reproduces_nominal_flight() {
        let nominal_flight = nominal().fly().unwrap();
        let run = MonteCarlo::new(nominal(), Dispersion::default())
            .unwrap()
            .run(SEED, 3);
        for sample in &run.samples {
            assert_eq!(sample.summary(), Some(&nominal_flight));
        }
        let apogee = run.apogee().unwrap();
        let nominal_apogee = nominal_flight.apogee.unwrap().height_above_ground_m;
        assert_eq!(apogee.mean(), Some(nominal_apogee));
        assert_eq!(apogee.standard_deviation(), Some(0.0));
        // The same seed, the same run, to the bit; it serializes and reads back whole.
        let dispersed = MonteCarlo::new(nominal(), every_dispersion()).unwrap();
        let first = dispersed.run(SEED, 2);
        assert_eq!(first, dispersed.run(SEED, 2));
        let json = serde_json::to_string(&first).unwrap();
        assert_eq!(serde_json::from_str::<Run>(&json).unwrap(), first);
    }

    #[test]
    fn each_dispersion_moves_its_input() {
        let run = MonteCarlo::new(nominal(), every_dispersion()).unwrap();
        let draw = run.draw(SEED, 1);
        let inputs = run.inputs(&draw).unwrap();
        let base = run.nominal();
        // Mass and centre: the stage's override, from the layout's nominal values.
        let layout = base.rocket.layout().unwrap();
        let flown = inputs.rocket.layout().unwrap();
        let (before, after) = (&layout.stages[0], &flown.stages[0]);
        let mass = after.mass.mass_kg / before.mass.mass_kg;
        assert!((mass - draw.dry_mass_scale[0]).abs() < 1e-14, "{mass}");
        let shift = before.mass.cg_m.z - after.mass.cg_m.z;
        assert!((shift - draw.cg_shift_m[0]).abs() < 1e-14, "{shift}");
        let inertia = after.mass.inertia_kg_m2.x_axis.x / before.mass.inertia_kg_m2.x_axis.x;
        assert!(
            (inertia - draw.dry_mass_scale[0]).abs() < 1e-14,
            "{inertia}"
        );
        // Drag.
        assert_eq!(inputs.drag_scale, draw.drag_scale);
        // Delays: the motor's (Valetudo's has none) and the drogue's lag.
        assert_eq!(
            inputs.rocket.configurations[0].motors[0].delay,
            base.rocket.configurations[0].motors[0].delay
        );
        assert_eq!(
            inputs.recovery[0].lag_s,
            (1.0 + draw.deployment_lag_offset_s[0]).max(0.0)
        );
        // Wind: scaled.
        let speed = |i: &FlightInputs| {
            i.environment
                .wind
                .wind(1500.0)
                .unwrap()
                .velocity_enu_m_s
                .length()
        };
        assert!((speed(&inputs) / speed(base) - draw.wind_speed_scale).abs() < 1e-14);
        // Rail.
        assert_eq!(
            inputs.rail.elevation_rad,
            base.rail.elevation_rad + draw.rail_elevation_offset_rad
        );
        // The flight flies, and its apogee isn't the nominal one.
        let flight = run.sample(SEED, 1);
        let apogee = |s: &FlightSummary| s.apogee.as_ref().unwrap().height_above_ground_m;
        let nominal_apogee = apogee(&base.fly().unwrap());
        assert_ne!(apogee(flight.summary().unwrap()), nominal_apogee);
    }

    #[test]
    fn an_ejection_delay_moves_and_stops_at_zero() {
        let mut base = nominal();
        base.rocket.configurations[0].motors[0].delay = Some(Delay::Seconds(0.3));
        let run = MonteCarlo::new(
            base,
            Dispersion {
                ejection_delay_sd_s: 1.0,
                deployment_lag_sd_s: 1.0,
                ..Dispersion::default()
            },
        )
        .unwrap();
        let mut cut = [false; 2];
        let mut moved = [false; 2];
        for index in 0..40 {
            let draw = run.draw(SEED, index);
            let inputs = run.inputs(&draw).unwrap();
            let delay = draw.ejection_delay_offset_s[0] + 0.3;
            let Some(Delay::Seconds(flown)) = inputs.rocket.configurations[0].motors[0].delay
            else {
                unreachable!("a delay in seconds stays one")
            };
            assert_eq!(flown, delay.max(0.0));
            let lag = draw.deployment_lag_offset_s[0] + 1.0;
            assert_eq!(inputs.recovery[0].lag_s, lag.max(0.0));
            for (i, value) in [delay, lag].into_iter().enumerate() {
                cut[i] |= value < 0.0;
                moved[i] |= value > 0.0;
            }
        }
        assert_eq!((cut, moved), ([true; 2], [true; 2]));
    }

    #[test]
    fn a_rail_past_vertical_leans_the_other_way() {
        let rail = Rail {
            azimuth_rad: 0.4,
            roll_rad: 0.1,
            ..Rail::vertical(2.0)
        };
        let leaned = tilted(rail, 0.03, 0.0);
        assert!((leaned.elevation_rad - (FRAC_PI_2 - 0.03)).abs() < 1e-15);
        // The same attitude as the angles taken past vertical.
        let past = LaunchAngles {
            azimuth_rad: 0.4,
            elevation_rad: FRAC_PI_2 + 0.03,
            roll_rad: 0.1,
        };
        let flown = LaunchAngles {
            azimuth_rad: leaned.azimuth_rad,
            elevation_rad: leaned.elevation_rad,
            roll_rad: leaned.roll_rad,
        };
        let (a, b) = (past.to_quaternion(), flown.to_quaternion());
        assert!(a.dot(b).abs() > 1.0 - 1e-15, "{a:?} {b:?}");
        assert!(leaned.validate().is_ok());
        // Below vertical, nothing turns.
        let below = tilted(rail, -0.03, 0.2);
        assert_eq!(below.roll_rad, 0.1);
        assert_eq!(below.azimuth_rad, 0.4 + 0.2);
    }

    #[test]
    fn a_dispersed_wind_turns_clockwise() {
        // From the north at 4 m/s, turned a quarter turn clockwise: from the east, blowing west.
        let north = Arc::new(ConstantWind::new(4.0, 0.0).unwrap());
        let turned = DispersedWind::new(north, 1.5, FRAC_PI_2).unwrap();
        let v = turned.wind(100.0).unwrap().velocity_enu_m_s;
        assert!((v - DVec3::new(-6.0, 0.0, 0.0)).length() < 1e-14, "{v:?}");
        for (scale, turn) in [(-0.1, 0.0), (f64::NAN, 0.0), (1.0, f64::INFINITY)] {
            assert!(matches!(
                DispersedWind::new(Arc::new(ConstantWind::calm()), scale, turn),
                Err(AnalysisError::Domain { .. })
            ));
        }
    }

    /// The drag scale reaches the flight: `FlightInputs` flies it as
    /// `Simulation::with_drag_scale` does, and a sample's factor multiplies a nominal scale.
    #[test]
    fn the_drag_scale_is_flown() {
        let mut scaled = nominal();
        scaled.drag_scale = 1.1;
        let by_hand = {
            let simulation = nominal()
                .simulation()
                .unwrap()
                .with_drag_scale(1.1)
                .unwrap();
            let mut metrics = FlightMetrics::new();
            let result = simulation.run(&mut metrics).unwrap();
            metrics.summary(&result, &scaled.environment).unwrap()
        };
        assert_eq!(scaled.fly().unwrap(), by_hand);
        assert_ne!(nominal().fly().unwrap(), by_hand);
        let run = MonteCarlo::new(
            scaled,
            Dispersion {
                drag_sd_fraction: 0.05,
                ..Dispersion::default()
            },
        )
        .unwrap();
        let draw = run.draw(SEED, 0);
        assert_eq!(run.inputs(&draw).unwrap().drag_scale, 1.1 * draw.drag_scale);
    }

    /// A draw is checked against the rocket before it flies, and every entry that isn't at its
    /// nominal value is flown, whatever the dispersion: a draw written by hand flies as it says.
    #[test]
    fn a_draw_is_checked_and_flown_as_written() {
        let run = MonteCarlo::new(nominal(), Dispersion::default()).unwrap();
        let good = run.draw(SEED, 0);
        // Every list is checked against the rocket: one stage, one motor, one recovery device.
        type Field = fn(&mut Draw) -> &mut Vec<f64>;
        let fields: [Field; 6] = [
            |d| &mut d.dry_mass_scale,
            |d| &mut d.cg_shift_m,
            |d| &mut d.impulse_scale,
            |d| &mut d.burn_time_scale,
            |d| &mut d.ejection_delay_offset_s,
            |d| &mut d.deployment_lag_offset_s,
        ];
        for field in fields {
            for length in [0, 2] {
                let mut draw = good.clone();
                field(&mut draw).resize(length, 0.0);
                assert!(
                    matches!(
                        run.inputs(&draw),
                        Err(AnalysisError::Length { length: l, expected: 1, .. }) if l == length
                    ),
                    "{draw:?}"
                );
            }
        }
        // An entry that isn't a number is refused before it reaches a model that might not.
        let mut draw = good.clone();
        draw.rail_elevation_offset_rad = f64::NAN;
        assert!(matches!(
            run.inputs(&draw),
            Err(AnalysisError::Domain { what: "entry of a draw", value }) if value.is_nan()
        ));
        // No dispersion asked for, but the draw says 20% more drag and a calm wind (a factor
        // below zero is cut to calm).
        let mut draw = good;
        draw.drag_scale = 1.2;
        draw.wind_speed_scale = -0.3;
        let inputs = run.inputs(&draw).unwrap();
        assert_eq!(inputs.drag_scale, 1.2);
        let wind = inputs
            .environment
            .wind
            .wind(1500.0)
            .unwrap()
            .velocity_enu_m_s;
        assert_eq!(wind.length(), 0.0);
    }

    /// The numbers a seed gives are part of every seeded result: these pin the stream key's fold
    /// and the order of a draw's inputs, so a change to either shows here first.
    #[test]
    fn a_seed_s_draws_are_pinned() {
        assert_eq!(SeededRng::for_stream(42, &[7]).next_u64(), GOLDEN_STREAM);
        let draw = MonteCarlo::new(nominal(), every_dispersion())
            .unwrap()
            .draw(SEED, 0);
        for (drawn, golden) in [
            (draw.drag_scale, GOLDEN_DRAG),
            (draw.impulse_scale[0], GOLDEN_IMPULSE),
        ] {
            assert!((drawn - golden).abs() < 1e-14, "{drawn:?}");
        }
    }

    #[test]
    fn bad_setups_are_refused() {
        let error = MonteCarlo::new(
            nominal(),
            Dispersion {
                cg_sd_m: -0.1,
                ..Dispersion::default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            AnalysisError::Domain { what: "centre-of-mass standard deviation (m)", value }
                if value == -0.1
        ));
        let mut elsewhere = nominal();
        elsewhere.configuration_id = "missing".to_owned();
        assert!(matches!(
            MonteCarlo::new(elsewhere, Dispersion::default()),
            Err(AnalysisError::NoConfiguration(id)) if id == "missing"
        ));
    }
}
