//! A flight: a rocket launched from a rail in an environment, flown to the ground.

use std::sync::Arc;

use hpr_aero::{DragModel, DragTable};
use hpr_sim::metrics::{FlightMetrics, FlightSummary, Landing};
use hpr_sim::{FlightResult, FlightSettings, Observer, Rail, Simulation};
use serde::{Deserialize, Serialize};

use crate::environment::Environment;
use crate::error::{Error, finite};
use crate::rocket::Rocket;

/// How a flight is launched: which rocket, where, and from what rail. Made by
/// [`Flight::builder`]; [`FlightBuilder::fly`] flies it.
#[derive(Debug, Clone)]
pub struct FlightBuilder<'a> {
    rocket: &'a Rocket,
    environment: &'a Environment,
    rail: Rail,
    /// The rail's angles, degrees, where set: they take the place of the rail's own.
    inclination_deg: Option<f64>,
    heading_deg: Option<f64>,
    settings: FlightSettings,
    /// A drag model flown in place of hpr's drag buildup, where set.
    drag_model: Option<Arc<dyn DragModel>>,
    /// A drag table flown in place of hpr's drag buildup, where set; never with a model.
    drag_table: Option<DragTable>,
}

impl FlightBuilder<'_> {
    /// The rail's angle above the horizon, degrees: 90, the default, is vertical, and 85 leans
    /// 5° off it. (OpenRocket's launch rod angle is measured from the vertical instead: its 5° is
    /// 85 here.)
    #[must_use]
    pub fn inclination_deg(mut self, inclination_deg: f64) -> Self {
        self.inclination_deg = Some(inclination_deg);
        self
    }

    /// The direction the rail leans toward, clockwise from true north, degrees: 0, the
    /// default, is north and 90 is east. A wind's direction is where it blows from, so a rail
    /// leaning into a west wind has both at 270. On a vertical rail the heading still turns the
    /// rocket about its axis, which way its fins face. That matters to a set of one or two fins,
    /// whose lift depends on which way the air meets them; three or more equal fins lift nearly
    /// the same whichever way they face.
    #[must_use]
    pub fn heading_deg(mut self, heading_deg: f64) -> Self {
        self.heading_deg = Some(heading_deg);
        self
    }

    /// The whole rail, in place of the one [`Flight::builder`] made: its length, its angles in
    /// radians, the rocket's roll on it and its friction ([`Rail`]).
    /// [`FlightBuilder::inclination_deg`] and [`FlightBuilder::heading_deg`], called before or
    /// after, still set its angles.
    #[must_use]
    pub fn rail(mut self, rail: Rail) -> Self {
        self.rail = rail;
        self
    }

    /// The integrator and its limits, in place of the defaults ([`FlightSettings`]).
    #[must_use]
    pub fn settings(mut self, settings: FlightSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Flies `model`'s drag in place of hpr's drag buildup: a model of your own, from a wind
    /// tunnel, another tool or your flights, or hpr's own adjusted
    /// ([`hpr_aero::custom`], [`Simulation::with_drag_model`]). The model gives the zero-lift
    /// drag coefficient on the rocket's reference area, by default a circle of its largest body
    /// diameter; unlike a drag table's, a model's number isn't rescaled, so a curve measured on
    /// another area is converted before it is returned. The normal force, centre of pressure,
    /// roll and damping stay hpr's, so the stability margin a [`Rocket`] reports doesn't change.
    /// The last model or table set is the one flown ([`FlightBuilder::drag_table`]), and every
    /// flight of this builder shares it.
    ///
    /// ```
    /// # use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
    /// # use hpr::{Environment, FinPlanform, Flight, Motor, NoseShape, Position, Rocket};
    /// use hpr::hpr_aero::{AeroError, DragModel, DragQuery};
    ///
    /// /// hpr's own drag, 20% higher.
    /// #[derive(Debug)]
    /// struct Rougher;
    ///
    /// impl DragModel for Rougher {
    ///     fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
    ///         Ok(1.2 * query.buildup()?.zero_lift_coefficient)
    ///     }
    /// }
    ///
    /// # let mut rocket = Rocket::new("Small", 0.0563)?;
    /// # rocket
    /// #     .add_nose(Nose::hollow(NoseShape::Ogive { radius_ratio: 1.0 }, 0.22, 0.0015, material("abs")?))?
    /// #     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
    /// #     .add_fins(Fins::new(
    /// #         3,
    /// #         FinPlanform::Trapezoidal { root_chord_m: 0.1, tip_chord_m: 0.04, span_m: 0.045, sweep_m: 0.05 },
    /// #         0.003175,
    /// #         material("birch_plywood")?,
    /// #     ))?
    /// #     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
    /// #     .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?
    /// #     .set_motor(Motor::from_catalog("H54")?)?;
    /// let environment = Environment::new(32.99, -106.97, 1400.0)?;
    /// let launch = Flight::builder(&rocket, &environment, 1.8);
    /// let own = launch.fly()?.apogee_m().ok_or("no apogee")?;
    /// let rougher = launch.drag_model(Rougher).fly()?.apogee_m().ok_or("no apogee")?;
    /// assert!(rougher < own);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn drag_model(self, model: impl DragModel + 'static) -> Self {
        self.shared_drag_model(Arc::new(model))
    }

    /// As [`FlightBuilder::drag_model`], with a model already shared: one model, a large table
    /// say, flown by many builders without a copy for each.
    #[must_use]
    pub fn shared_drag_model(mut self, model: Arc<dyn DragModel>) -> Self {
        self.drag_model = Some(model);
        self.drag_table = None;
        self
    }

    /// Flies `table`'s drag in place of hpr's drag buildup: another tool's zero-lift drag
    /// coefficient `C_D0` against Mach number, as RocketPy's `power_off_drag` and
    /// `power_on_drag` curves are ([`DragTable`], [`Simulation::with_drag_table`]). Its power-on
    /// curve, where it has one, is flown while a motor thrusts, and its power-off curve at every
    /// other time. Between its Mach numbers the table interpolates linearly; past its ends it
    /// holds the end values. A table on a reference diameter of its own
    /// ([`DragTable::with_reference_diameter_m`]) is rescaled to the rocket's reference area by
    /// the ratio of the two areas, `C_D0 · (d_table / d_rocket)²`. The normal force, centre of
    /// pressure, roll and damping stay hpr's. The last model or table set is the one flown
    /// ([`FlightBuilder::drag_model`]).
    ///
    /// ```
    /// # use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
    /// # use hpr::{Environment, FinPlanform, Flight, Motor, NoseShape, Position, Rocket};
    /// use hpr::hpr_aero::DragTable;
    ///
    /// # let mut rocket = Rocket::new("Small", 0.0563)?;
    /// # rocket
    /// #     .add_nose(Nose::hollow(NoseShape::Ogive { radius_ratio: 1.0 }, 0.22, 0.0015, material("abs")?))?
    /// #     .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
    /// #     .add_fins(Fins::new(
    /// #         3,
    /// #         FinPlanform::Trapezoidal { root_chord_m: 0.1, tip_chord_m: 0.04, span_m: 0.045, sweep_m: 0.05 },
    /// #         0.003175,
    /// #         material("birch_plywood")?,
    /// #     ))?
    /// #     .add_motor_tube(MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?))?
    /// #     .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }))?
    /// #     .set_motor(Motor::from_catalog("H54")?)?;
    /// let environment = Environment::new(32.99, -106.97, 1400.0)?;
    /// let launch = Flight::builder(&rocket, &environment, 1.8);
    /// // C_D0 of 0.45 up to Mach 0.8, 0.6 by Mach 1.2; 0.05 less while the motor burns.
    /// let table = DragTable::from_csv(
    ///     "mach,cd\n0,0.45\n0.8,0.45\n1.2,0.6\n",
    ///     Some("mach,cd\n0,0.40\n0.8,0.40\n1.2,0.55\n"),
    /// )?;
    /// let tabled = launch.clone().drag_table(table).fly()?.apogee_m().ok_or("no apogee")?;
    /// let own = launch.fly()?.apogee_m().ok_or("no apogee")?;
    /// assert_ne!(tabled, own);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn drag_table(mut self, table: DragTable) -> Self {
        self.drag_table = Some(table);
        self.drag_model = None;
        self
    }

    /// The [`Simulation`] [`FlightBuilder::fly`] runs, for what the facade doesn't offer: user
    /// events, staging, mass shifts and the rest of [`hpr_sim`].
    ///
    /// # Errors
    ///
    /// - [`Error::NoMotor`] for a rocket with no motor.
    /// - [`Error::Domain`] for an inclination outside `(0°, 90°]` or a heading that isn't
    ///   finite.
    /// - [`Error::Sim`] for what [`Simulation::new`] and [`Simulation::with_recovery`] refuse: a
    ///   rail that isn't positive in length, a design with errors in it
    ///   ([`hpr_design::checks`]), a recovery device the flight can't fly.
    pub fn simulation(&self) -> Result<Simulation, Error> {
        let configuration_id = self.rocket.configuration_id().ok_or(Error::NoMotor)?;
        let mut rail = self.rail;
        if let Some(inclination_deg) = self.inclination_deg {
            if !(inclination_deg > 0.0 && inclination_deg <= 90.0) {
                return Err(Error::Domain {
                    what: "rail inclination, degrees above the horizon",
                    value: inclination_deg,
                });
            }
            rail.elevation_rad = inclination_deg.to_radians();
        }
        if let Some(heading_deg) = self.heading_deg {
            rail.azimuth_rad = finite("rail heading, degrees", heading_deg)?.to_radians();
        }
        rail.validate()?;
        let mut simulation = Simulation::new(
            self.rocket.design(),
            configuration_id,
            self.environment.sim().clone(),
            rail,
            self.settings,
        )?;
        if let Some(model) = &self.drag_model {
            simulation = simulation.with_shared_drag_model(Arc::clone(model));
        }
        if let Some(table) = &self.drag_table {
            simulation = simulation.with_drag_table(table.clone());
        }
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
/// [`Flight::summary`] has every metric; the methods below are the ones most asked for. A flight
/// serializes and reads back as a record; one read back isn't flown again or checked.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Flight {
    result: FlightResult,
    summary: FlightSummary,
}

impl Flight {
    /// A flight of `rocket` in `environment` from a vertical, frictionless rail `rail_length_m`
    /// long, measured from the rocket's aft end at the start to the rail's top. Set the rest on
    /// the builder; [`FlightBuilder::fly`] flies it.
    #[must_use]
    pub fn builder<'a>(
        rocket: &'a Rocket,
        environment: &'a Environment,
        rail_length_m: f64,
    ) -> FlightBuilder<'a> {
        FlightBuilder {
            rocket,
            environment,
            rail: Rail::vertical(rail_length_m),
            inclination_deg: None,
            heading_deg: None,
            settings: FlightSettings::default(),
            drag_model: None,
            drag_table: None,
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
