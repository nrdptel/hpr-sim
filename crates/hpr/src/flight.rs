//! A flight: a rocket launched from a rail in an environment, flown to the ground.

use hpr_sim::metrics::{FlightMetrics, FlightSummary, Landing};
use hpr_sim::{FlightResult, FlightSettings, Observer, Rail, Simulation};

use crate::environment::Environment;
use crate::error::{Error, finite};
use crate::rocket::Rocket;

/// How a flight is launched: which rocket, where, and from what rail. Made by
/// [`Flight::builder`]; [`FlightBuilder::fly`] flies it.
#[derive(Debug, Clone)]
pub struct FlightBuilder<'a> {
    rocket: &'a Rocket,
    environment: &'a Environment,
    rail_length_m: f64,
    inclination_deg: f64,
    heading_deg: f64,
    settings: FlightSettings,
}

impl FlightBuilder<'_> {
    /// The rail's angle above the horizon, degrees: 90 is vertical, the default.
    #[must_use]
    pub fn inclination_deg(mut self, inclination_deg: f64) -> Self {
        self.inclination_deg = inclination_deg;
        self
    }

    /// The direction the rail leans toward, clockwise from true north, degrees: 0, the
    /// default, is north and 90 is east. It matters only off the vertical.
    #[must_use]
    pub fn heading_deg(mut self, heading_deg: f64) -> Self {
        self.heading_deg = heading_deg;
        self
    }

    /// The integrator and its limits, in place of the defaults ([`FlightSettings`]).
    #[must_use]
    pub fn settings(mut self, settings: FlightSettings) -> Self {
        self.settings = settings;
        self
    }

    /// The [`Simulation`] [`FlightBuilder::fly`] runs, for what the facade doesn't offer: user
    /// events, staging, mass shifts and the rest of [`hpr_sim`].
    ///
    /// # Errors
    ///
    /// - [`Error::NoMotor`] for a rocket with no motor.
    /// - [`Error::Domain`] for an inclination or heading that isn't finite.
    /// - [`Error::Sim`] for what [`Simulation::new`] and [`Simulation::with_recovery`] refuse: a
    ///   rail that isn't positive in length, an inclination outside `(0°, 90°]`, a design with
    ///   errors in it ([`hpr_design::checks`]), a recovery device the flight can't fly.
    pub fn simulation(&self) -> Result<Simulation, Error> {
        let configuration_id = self.rocket.configuration_id().ok_or(Error::NoMotor)?;
        let rail = Rail {
            length_m: self.rail_length_m,
            azimuth_rad: finite("rail heading, degrees", self.heading_deg)?.to_radians(),
            elevation_rad: finite("rail inclination, degrees", self.inclination_deg)?.to_radians(),
            ..Rail::vertical(self.rail_length_m)
        };
        rail.validate()?;
        let simulation = Simulation::new(
            self.rocket.design(),
            configuration_id,
            self.environment.sim().clone(),
            rail,
            self.settings,
        )?;
        if self.rocket.recovery().is_empty() {
            Ok(simulation)
        } else {
            Ok(simulation.with_recovery(self.rocket.recovery().to_vec())?)
        }
    }

    /// Flies the rocket from the rail to the ground.
    ///
    /// # Errors
    ///
    /// As [`FlightBuilder::simulation`], and [`Error::Sim`] for a flight that fails on the way,
    /// such as a model asked about a flow it doesn't cover.
    pub fn fly(&self) -> Result<Flight, Error> {
        self.fly_with(&mut ())
    }

    /// Flies the rocket, showing `observer` every step of the flight as it goes: a
    /// [`hpr_sim::Recorder`] keeps a trajectory, and an [`Observer`] of your own can watch for
    /// anything.
    ///
    /// # Errors
    ///
    /// As [`FlightBuilder::fly`], and whatever `observer` returns.
    pub fn fly_with(&self, observer: &mut dyn Observer) -> Result<Flight, Error> {
        let simulation = self.simulation()?;
        let mut metrics = FlightMetrics::new();
        let result = simulation.run(&mut (&mut metrics, observer))?;
        let summary = metrics.summary(&result, self.environment.sim())?;
        Ok(Flight { result, summary })
    }
}

/// A flown flight: what happened, and its metrics.
///
/// Heights are the rocket's centre of gravity's, above the launch site: the rocket stands on
/// the rail at the start, so the first height is not zero. Speeds are relative to the ground.
/// [`Flight::summary`] has every metric; the methods below are the ones most asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Flight {
    result: FlightResult,
    summary: FlightSummary,
}

impl Flight {
    /// A flight of `rocket` in `environment` from a vertical rail `rail_length_m` long, measured
    /// from the rocket's aft end at the start to the rail's top. Set the rest on the builder;
    /// [`FlightBuilder::fly`] flies it.
    #[must_use]
    pub fn builder<'a>(
        rocket: &'a Rocket,
        environment: &'a Environment,
        rail_length_m: f64,
    ) -> FlightBuilder<'a> {
        FlightBuilder {
            rocket,
            environment,
            rail_length_m,
            inclination_deg: 90.0,
            heading_deg: 0.0,
            settings: FlightSettings::default(),
        }
    }

    /// Every metric of the flight: apogee, peaks, stability margins, landings.
    #[must_use]
    pub fn summary(&self) -> &FlightSummary {
        &self.summary
    }

    /// The flight itself: its events, its end, and every body's.
    #[must_use]
    pub fn result(&self) -> &FlightResult {
        &self.result
    }

    /// The apogee's height above the launch site, m; `None` if the flight ended before it.
    #[must_use]
    pub fn apogee_m(&self) -> Option<f64> {
        self.summary
            .apogee
            .map(|apogee| apogee.height_above_ground_m)
    }

    /// When the apogee came, s after launch.
    #[must_use]
    pub fn apogee_time_s(&self) -> Option<f64> {
        self.summary.apogee.map(|apogee| apogee.time_s)
    }

    /// The top speed, m/s.
    #[must_use]
    pub fn max_speed_m_s(&self) -> Option<f64> {
        self.summary.max_speed_m_s.map(|peak| peak.value)
    }

    /// The top Mach number.
    #[must_use]
    pub fn max_mach(&self) -> Option<f64> {
        self.summary.max_mach.map(|peak| peak.value)
    }

    /// The speed as the rocket left the rail, m/s.
    #[must_use]
    pub fn rail_exit_speed_m_s(&self) -> Option<f64> {
        self.summary.rail_exit_speed_m_s.map(|peak| peak.value)
    }

    /// Where and how fast the rocket landed; `None` if it never reached the ground.
    #[must_use]
    pub fn landing(&self) -> Option<&Landing> {
        self.summary.landing.as_ref()
    }
}
