//! What a flight comes to: its peaks, its apogee, its stability margins, the ejection delay that
//! would fire at apogee, and where each body landed ([the flight-metrics milestone][m1-10a] and
//! [its decision record][adr-077]; the guide's [Flight metrics][page] page).
//!
//! - [`FlightMetrics`] is an [`Observer`]: fly a [`Simulation`] with it, then ask it for a
//!   [`FlightSummary`]. It finds each peak on the integrator's dense output, not in a recorded
//!   table, so a peak between two rows is not missed ([Loft lesson L34][l34]).
//! - [`stability`] gives the static margin and the margin at the flight's Mach number at one
//!   instant, and [`margin`] the margin of any flow. A margin that would be a ratio of nearly cancelling
//!   slopes is `None`, with the pitch-moment slope beside it ([L33][l33]).
//! - [`optimum_delays`] flies the rocket with its charges held and gives, for each motor, the delay
//!   from its burnout to that apogee, so it doesn't depend on the delay flown ([L94][l94]).
//! - Anything that did not happen is `None`, never a zero, and every height names its datum
//!   ([L35][l35]).
//!
//! **Heights.** A height here is the centre of mass's ellipsoidal height above the launch site's,
//! [`Sample::height_above_ground_m`]. The centre of mass starts above the site (the rocket stands
//! on the rail), at [`FlightSummary::launch_height_m`]; [`Apogee::gain_m`] counts from there, as
//! OpenRocket's altitude does.
//!
//! [l33]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l33
//! [l34]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l34
//! [l35]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l35
//! [l94]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l94
//! [m1-10a]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-10a
//! [adr-077]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-077-flight-metrics-peaks-on-the-dense-output-margins-only-where-they-mean-something-and-none-for-what-didnt-happen-2026-09-26
//! [page]: https://nrdptel.github.io/hpr-sim/physics/metrics.html

use hpr_aero::{AeroModel, Flow};
use serde::{Deserialize, Serialize};

use crate::dynamics::Phase;
use crate::environment::Environment;
use crate::error::SimError;
use crate::flight::{EventKind, FlightEvent, FlightResult, Simulation, Termination};
use crate::recorder::{FlightStep, Observer, Sample};
use crate::recovery::BodySample;

/// The largest ratio `κ = Σ |C_Nα,i| / C_Nα` at which a margin is given: `√10`.
///
/// The centre of pressure is `x_cp = Σ C_Nα,i x_i / C_Nα`, over the parts the model adds up, with
/// each part's station `x_i` on the rocket, in `[0, L]`. Since `x_cp − x_j = Σ C_Nα,i (x_i − x_j) / C_Nα`, it lies within `κ L` of every
/// station. An error `ε C_Nα,j` in one component's slope moves it by
/// `ε C_Nα,j (x_j − x_cp) / C_Nα`, so by at most `ε κ² L`. A rocket whose slopes all push the same
/// way has `κ = 1`, and a 1% error in one slope moves its centre of pressure by at most 1% of its
/// length. As the net slope goes to zero, `κ` runs away: the loads become a pure couple with no
/// line of action, and the quotient is noise (Loft published ±12 to 15 calibres so,
/// [lesson L33][l33]). At `κ = √10` a 1% error in one slope can move the centre of pressure by a
/// tenth of the rocket's length; past it hpr gives no margin, only the pitch-moment slope, which
/// stays finite. The limit is a chosen bound on that sensitivity, not a measurement. The bound
/// holds for parts that each carry a force at a station: a part that is a pure couple (a step and
/// a flare of equal slopes) has none, and `κ` doesn't count it.
///
/// [l33]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l33
pub const MARGIN_CONDITION_LIMIT: f64 = 3.162_277_660_168_379_5;

/// The Mach number of the static margin: the air at rest, as RocketPy's `static_margin` takes it.
pub const STATIC_MARGIN_MACH: f64 = 0.0;

/// A peak over the flight, with where and when it came.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Peak {
    /// The peak value, in the quantity's own unit.
    pub value: f64,
    /// When it came, s after launch.
    pub time_s: f64,
    /// The centre of mass's height above the launch site then, m.
    pub height_above_ground_m: f64,
}

/// The stability margin of one flow.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Margin {
    /// The Mach number.
    pub mach: f64,
    /// The total angle of attack, rad.
    pub angle_of_attack_rad: f64,
    /// The rocket's normal-force slope `C_Nα` on the reference area, per radian (at an angle of
    /// attack, the secant `C_N/α`, [`hpr_aero::NormalForce::slope_per_rad`]).
    pub normal_force_slope_per_rad: f64,
    /// `Σ |C_Nα,i|` over the components, per radian: the scale against which the net slope is
    /// judged ([`MARGIN_CONDITION_LIMIT`]). With a normal-force table it is the table's own slope's
    /// magnitude.
    pub slope_magnitude_sum_per_rad: f64,
    /// The pitch-moment slope about the centre of mass on the reference area and diameter, per
    /// radian: `C_mα = −(Σ C_Nα,i x_i − C_Nα x_cg) / d`, stations `x` aft of the nose tip, from
    /// [`hpr_aero::NormalForce::moment_slope_m`]. Negative restores. It is finite when the margin
    /// is not, and with a margin it is `−C_Nα · margin`.
    pub pitch_moment_slope_per_rad: f64,
    /// The centre of pressure, m aft of the nose tip; `None` when the margin is.
    pub cp_station_m: Option<f64>,
    /// The margin `(x_cp − x_cg) / d`, calibres: positive with the centre of pressure aft of the
    /// centre of mass. `None` when the net slope is not positive or is below
    /// `Σ |C_Nα,i| / MARGIN_CONDITION_LIMIT`, where the quotient would be noise.
    pub margin_cal: Option<f64>,
}

/// The rocket's stability at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Stability {
    /// When, s after launch.
    pub time_s: f64,
    /// The centre of mass's height above the launch site then, m.
    pub height_above_ground_m: f64,
    /// The dynamic pressure then, Pa: how hard the air presses on the margin's moment.
    pub dynamic_pressure_pa: f64,
    /// The centre of mass, m aft of the nose tip.
    pub cg_station_m: f64,
    /// The reference diameter `d` the margins are counted in, m (the sustainer's after a powered
    /// separation).
    pub reference_diameter_m: f64,
    /// The static margin: the air along the axis at [`STATIC_MARGIN_MACH`], with the centre of
    /// mass of this instant (RocketPy's `static_margin`).
    pub static_margin: Margin,
    /// The flight margin: the air along the axis at the flight's Mach number, with the centre of
    /// mass of this instant (RocketPy's `stability_margin`). The angle of attack is left out: near
    /// apogee it swings toward 90° as the rocket slows and tips over, and a least margin that
    /// followed it would land wherever large angles stopped being counted.
    pub flight_margin: Margin,
}

/// The margin of `aero` in `flow` with the centre of mass at `cg_station_m` (m aft of the nose
/// tip). See [`Margin`] and [`MARGIN_CONDITION_LIMIT`] for when it is `None`.
///
/// # Errors
///
/// The aerodynamic model's, for a flow outside its range.
pub fn margin(aero: &AeroModel, flow: &Flow, cg_station_m: f64) -> Result<Margin, SimError> {
    let normal = aero.normal_force(flow)?;
    let d = aero.reference_diameter_m();
    let slope = normal.slope_per_rad;
    let scale = if aero.normal_force_table().is_some() {
        // The table gives one force at one station.
        slope.abs()
    } else {
        aero.components(flow)?
            .iter()
            .map(|c| c.normal_force.slope_per_rad.abs())
            .sum()
    };
    let conditioned = slope > 0.0 && slope * MARGIN_CONDITION_LIMIT >= scale;
    let cp_station_m = normal.cp_station_m.filter(|_| conditioned);
    Ok(Margin {
        mach: flow.mach,
        angle_of_attack_rad: flow.alpha_rad,
        normal_force_slope_per_rad: slope,
        slope_magnitude_sum_per_rad: scale,
        pitch_moment_slope_per_rad: -(normal.moment_slope_m - slope * cg_station_m) / d,
        cp_station_m,
        margin_cal: cp_station_m.map(|cp| (cp - cg_station_m) / d),
    })
}

/// The static margin and the flight margin of `aero` at `time_s`, with the centre of mass
/// `height_above_ground_m` above the launch site and at `cg_station_m` on the airframe, and the
/// flight's air at Mach `mach` pressing with `dynamic_pressure_pa`.
///
/// # Errors
///
/// As [`margin`].
pub fn stability(
    aero: &AeroModel,
    time_s: f64,
    height_above_ground_m: f64,
    dynamic_pressure_pa: f64,
    cg_station_m: f64,
    mach: f64,
) -> Result<Stability, SimError> {
    Ok(Stability {
        time_s,
        height_above_ground_m,
        dynamic_pressure_pa,
        cg_station_m,
        reference_diameter_m: aero.reference_diameter_m(),
        static_margin: margin(aero, &Flow::axial(STATIC_MARGIN_MACH), cg_station_m)?,
        flight_margin: margin(aero, &Flow::axial(mach), cg_station_m)?,
    })
}

/// The apogee of the flight's centre of mass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Apogee {
    /// When, s after launch.
    pub time_s: f64,
    /// The centre of mass's ellipsoidal height above the launch site, m.
    pub height_above_ground_m: f64,
    /// The rise of the centre of mass from where it stood at launch, m: the height above the site
    /// less [`FlightSummary::launch_height_m`], as OpenRocket's altitude counts. `None` for a
    /// flight started in the air ([`Simulation::run_free`]).
    pub gain_m: Option<f64>,
}

/// Where something landed: the centre of mass as it reached the launch site's ellipsoidal
/// height.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Landing {
    /// Which separated body ([`crate::BodyFlight::body`]), or `None` for the flight itself: the
    /// stack, or after a powered separation the sustainer.
    pub body: Option<usize>,
    /// When, s after launch.
    pub time_s: f64,
    /// Geodetic latitude, degrees north (WGS 84).
    pub latitude_deg: f64,
    /// Longitude, degrees east (WGS 84).
    pub longitude_deg: f64,
    /// East of the launch site in its local frame, m.
    pub east_m: f64,
    /// North of the launch site, m.
    pub north_m: f64,
    /// The horizontal distance from the launch site, m.
    pub distance_m: f64,
    /// The speed of the centre of mass at the ground hit, relative to the ground, m/s.
    pub ground_hit_speed_m_s: f64,
    /// Its downward speed then, m/s.
    pub descent_rate_m_s: f64,
}

impl Landing {
    /// The landing at `sample`, for `body`, located on `environment`'s ellipsoid.
    ///
    /// # Errors
    ///
    /// [`SimError::Core`] if the position has no geodetic coordinates.
    pub fn at(
        body: Option<usize>,
        sample: &BodySample,
        environment: &Environment,
    ) -> Result<Self, SimError> {
        let place = environment
            .earth
            .frame()
            .geodetic_from_enu(sample.cg_enu_m)?;
        let (east_m, north_m) = (sample.cg_enu_m.x, sample.cg_enu_m.y);
        Ok(Self {
            body,
            time_s: sample.time_s,
            latitude_deg: place.latitude_rad.to_degrees(),
            longitude_deg: place.longitude_rad.to_degrees(),
            east_m,
            north_m,
            distance_m: east_m.hypot(north_m),
            ground_hit_speed_m_s: sample.cg_velocity_enu_m_s.length(),
            descent_rate_m_s: -sample.vertical_speed_m_s,
        })
    }
}

/// The metrics of one flight. Every field that may not have happened is an `Option`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightSummary {
    /// Why the flight ended.
    pub termination: Termination,
    /// The centre of mass's height above the launch site as the rocket stood at launch, m. `None`
    /// for a flight started in the air ([`Simulation::run_free`]).
    pub launch_height_m: Option<f64>,
    /// The speed as the last rail guide left the rail, m/s, with its time and height.
    pub rail_exit_speed_m_s: Option<Peak>,
    /// The apogee of the centre of mass.
    pub apogee: Option<Apogee>,
    /// The largest speed of the centre of mass relative to the ground after liftoff, m/s.
    pub max_speed_m_s: Option<Peak>,
    /// The largest Mach number after liftoff.
    pub max_mach: Option<Peak>,
    /// The largest dynamic pressure after liftoff ("max q"), Pa.
    pub max_dynamic_pressure_pa: Option<Peak>,
    /// The largest acceleration of the nose tip (the body origin) relative to the launch frame
    /// from liftoff until a recovery device opens, m/s²: the boost and the coast, from the
    /// equations of motion.
    pub max_acceleration_m_s2: Option<Peak>,
    /// The largest acceleration under the recovery devices, m/s²: the opening shock, which
    /// follows the inflation model, kept apart from the boost's.
    pub max_descent_acceleration_m_s2: Option<Peak>,
    /// The smallest static margin from the rail exit to apogee or the first deployment,
    /// calibres, where it is defined, found inside steps as a peak is. After a powered separation
    /// the sustainer's margins count its own diameter.
    pub min_static_margin_cal: Option<Peak>,
    /// The smallest flight margin over the same span, calibres, found the same way. RocketPy's
    /// `min_stability_margin` takes its least over the whole flight, so it can differ.
    pub min_flight_margin_cal: Option<Peak>,
    /// The stability as the last rail guide left the rail.
    pub rail_exit_stability: Option<Stability>,
    /// Where the flight itself landed: the stack, or after a powered separation the sustainer.
    pub landing: Option<Landing>,
    /// Where each separated body landed, in body order; a body that didn't land is left out.
    pub body_landings: Vec<Landing>,
}

impl FlightSummary {
    /// The flight's own ground-hit speed, m/s, if it landed.
    #[must_use]
    pub fn ground_hit_speed_m_s(&self) -> Option<f64> {
        self.landing.map(|landing| landing.ground_hit_speed_m_s)
    }
}

/// Watches a flight and keeps its peaks, and its stability from the rail exit to apogee or the
/// first deployment.
///
/// Each step is sampled at its start, middle and end. When the parabola through the three has its
/// top inside the step, a golden-section search on the dense output finds the peak there; the
/// largest of what it finds and the three samples is kept. The least margins are found the same
/// way, with the parabola turned over. Thrust-curve knots and events end steps, so a thrust
/// spike's peak is a step's end. Nothing is kept on the pad.
///
/// A watcher keeps one flight: [`FlightMetrics::clear`] it before watching another, and
/// [`FlightMetrics::summary`] refuses a flight whose steps it didn't all see, once each. Like
/// [`crate::Recorder`], it serializes what it holds for inspection and is built with
/// [`FlightMetrics::new`], not deserialized.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FlightMetrics {
    launch_height_m: Option<f64>,
    max_speed: Option<Peak>,
    max_mach: Option<Peak>,
    max_q: Option<Peak>,
    max_acceleration: Option<Peak>,
    max_descent_acceleration: Option<Peak>,
    /// Whether any step was seen, and the last one's end.
    last_end_s: Option<f64>,
    /// How many steps it saw: a flight's accepted steps, one each.
    steps: u64,
    on_rail: bool,
    past_apogee: bool,
    /// Whether a separation came since the last step: the next starts on another rocket.
    separated: bool,
    rail_exit: Option<Stability>,
    stability: Vec<Stability>,
    min_static_margin: Option<Peak>,
    min_flight_margin: Option<Peak>,
}

/// What a peak is of.
#[derive(Debug, Clone, Copy)]
enum Quantity {
    Speed,
    Mach,
    DynamicPressure,
    Acceleration,
}

impl Quantity {
    const ALL: [Quantity; 4] = [
        Quantity::Speed,
        Quantity::Mach,
        Quantity::DynamicPressure,
        Quantity::Acceleration,
    ];

    fn of(self, sample: &Sample) -> f64 {
        match self {
            Quantity::Speed => sample.cg_velocity_enu_m_s.length(),
            Quantity::Mach => sample.mach,
            Quantity::DynamicPressure => sample.dynamic_pressure_pa,
            Quantity::Acceleration => sample.acceleration_enu_m_s2.length(),
        }
    }
}

impl FlightMetrics {
    /// A watcher with nothing seen.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets what it saw, ready for another flight.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// The stability from the rail exit (or the start of a flight begun in the air) to apogee or
    /// the first deployment, whichever comes first: at the rail exit and at every step's end, in
    /// time order. A powered separation adds the sustainer's own entry at the split, after the
    /// stack's and at the same time, so the watcher needs the flight's events as well as its
    /// steps.
    #[must_use]
    pub fn stability(&self) -> &[Stability] {
        &self.stability
    }

    /// The summary of `result`, the flight this watched, flown in `environment`.
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] if it didn't see each of `result`'s accepted steps once, or its last
    /// step ended after `result` did (it watched another flight, or wasn't cleared between two);
    /// [`SimError::Core`] if a landing has no geodetic coordinates.
    pub fn summary(
        &self,
        result: &FlightResult,
        environment: &Environment,
    ) -> Result<FlightSummary, SimError> {
        // A flight can end without a step (before its first, or on a stop a few ulps ahead that
        // the clock just moves to), so the steps are counted rather than its end matched.
        if self.steps != result.stats.accepted_steps
            || self
                .last_end_s
                .is_some_and(|end| end > result.final_sample.time_s)
        {
            return Err(SimError::Domain {
                what: "steps this watcher saw (it must watch each step of the flight it sums up, \
                       and be cleared before another)",
                value: self.steps as f64,
            });
        }
        let apogee = result.event(EventKind::Apogee).map(|event| {
            let height = event.sample.height_above_ground_m;
            Apogee {
                time_s: event.sample.time_s,
                height_above_ground_m: height,
                gain_m: self.launch_height_m.map(|start| height - start),
            }
        });
        let rail_exit = result.event(EventKind::RailExit).map(|event| event.sample);
        let landing = if result.termination == Termination::GroundHit {
            let s = &result.final_sample;
            Some(Landing::at(
                None,
                &BodySample {
                    time_s: s.time_s,
                    cg_enu_m: s.cg_enu_m,
                    cg_velocity_enu_m_s: s.cg_velocity_enu_m_s,
                    height_above_ground_m: s.height_above_ground_m,
                    vertical_speed_m_s: s.vertical_speed_m_s,
                    airspeed_m_s: s.airspeed_m_s,
                    recovery_drag_area_m2: s.recovery_drag_area_m2,
                    mass_kg: s.mass_kg,
                },
                environment,
            )?)
        } else {
            None
        };
        let body_landings = result
            .bodies
            .iter()
            .filter(|body| body.termination == Termination::GroundHit)
            .map(|body| Landing::at(Some(body.body), &body.final_sample, environment))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FlightSummary {
            termination: result.termination,
            launch_height_m: self.launch_height_m,
            rail_exit_speed_m_s: rail_exit.map(|s| Peak {
                value: s.cg_velocity_enu_m_s.length(),
                time_s: s.time_s,
                height_above_ground_m: s.height_above_ground_m,
            }),
            apogee,
            max_speed_m_s: self.max_speed,
            max_mach: self.max_mach,
            max_dynamic_pressure_pa: self.max_q,
            max_acceleration_m_s2: self.max_acceleration,
            max_descent_acceleration_m_s2: self.max_descent_acceleration,
            min_static_margin_cal: self.min_static_margin,
            min_flight_margin_cal: self.min_flight_margin,
            rail_exit_stability: self.rail_exit,
            landing,
            body_landings,
        })
    }

    fn slot(&mut self, quantity: Quantity, phase: Phase) -> &mut Option<Peak> {
        match quantity {
            Quantity::Speed => &mut self.max_speed,
            Quantity::Mach => &mut self.max_mach,
            Quantity::DynamicPressure => &mut self.max_q,
            Quantity::Acceleration if phase == Phase::Descent => &mut self.max_descent_acceleration,
            Quantity::Acceleration => &mut self.max_acceleration,
        }
    }
}

impl Observer for FlightMetrics {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let phase = step.phase();
        let (a, b) = (step.start_s(), step.end_s());
        let start = step.sample(a)?;
        if self.last_end_s.is_none() {
            match phase {
                Phase::Pad | Phase::Rail => {
                    self.launch_height_m = Some(start.height_above_ground_m)
                }
                // A flight begun in the air on its way down has no apogee to come.
                _ if start.vertical_speed_m_s <= 0.0 => self.past_apogee = true,
                _ => {}
            }
        }
        self.steps += 1;
        self.last_end_s = Some(b);
        if phase == Phase::Pad {
            return Ok(());
        }
        let samples = [start, step.sample(0.5 * (a + b))?, step.sample(b)?];
        for quantity in Quantity::ALL {
            let best = step_peak(step, quantity, &samples)?;
            let slot = self.slot(quantity, phase);
            if slot.is_none_or(|peak| best.value > peak.value) {
                *slot = Some(best);
            }
        }
        if phase == Phase::Rail {
            self.on_rail = true;
        }
        // On the rail the rail holds the rocket, so stability starts at the rail exit.
        if !self.past_apogee && phase == Phase::Free {
            // A step starts where the last ended, on the same rocket, unless a separation came
            // between: then the sustainer's own margins at the split start the step.
            let first = match self.stability.last() {
                Some(last) if !self.separated => *last,
                _ => {
                    let first = step.stability(a)?;
                    if self.stability.is_empty() && self.on_rail {
                        self.rail_exit = Some(first);
                    }
                    self.stability.push(first);
                    first
                }
            };
            let end = step.stability(b)?;
            let entries = [first, step.stability(0.5 * (a + b))?, end];
            for (pick, slot) in [
                (
                    static_of as fn(&Stability) -> &Margin,
                    &mut self.min_static_margin,
                ),
                (flight_of, &mut self.min_flight_margin),
            ] {
                if let Some(least) = least_margin(step, a, b, &entries, pick)?
                    && slot.is_none_or(|peak| clearly_below(least.value, peak.value))
                {
                    *slot = Some(least);
                }
            }
            self.stability.push(end);
        }
        self.separated = false;
        Ok(())
    }

    fn event(&mut self, event: &FlightEvent) {
        match event.kind {
            EventKind::Apogee => self.past_apogee = true,
            EventKind::Separation => self.separated = true,
            _ => {}
        }
    }
}

/// The golden ratio's inverse, `(√5 − 1)/2`.
const INVERSE_PHI: f64 = 0.618_033_988_749_894_9;

/// How finely a peak's time is found, relative to the flight's clock (at least 1 s): 1e-9. The
/// value's error goes as the square of the time's, so it is far below the integration's.
const PEAK_TIME_RESOLUTION: f64 = 1e-9;

/// The largest of `value` on `[a, b]`, where it has one top: a golden-section search (Kiefer
/// 1953) on a step's dense output, to [`PEAK_TIME_RESOLUTION`].
fn golden_max(
    mut a: f64,
    mut b: f64,
    mut value: impl FnMut(f64) -> Result<Peak, SimError>,
) -> Result<Peak, SimError> {
    let mut c = b - INVERSE_PHI * (b - a);
    let mut d = a + INVERSE_PHI * (b - a);
    let mut fc = value(c)?;
    let mut fd = value(d)?;
    while b - a > PEAK_TIME_RESOLUTION * b.abs().max(1.0) {
        if fc.value >= fd.value {
            b = d;
            (d, fd) = (c, fc);
            c = b - INVERSE_PHI * (b - a);
            fc = value(c)?;
        } else {
            a = c;
            (c, fc) = (d, fd);
            d = a + INVERSE_PHI * (b - a);
            fd = value(d)?;
        }
    }
    Ok(if fc.value >= fd.value { fc } else { fd })
}

/// Where the parabola through `(−1, va)`, `(0, vm)` and `(1, vb)` has its top inside: it bends
/// down, and its top `x = (va − vb) / (2 (va − 2 vm + vb))` has `|x| < 1`.
fn top_inside(va: f64, vm: f64, vb: f64) -> bool {
    let bend = va - 2.0 * vm + vb;
    bend < 0.0 && (va - vb).abs() < -2.0 * bend
}

/// The peak of `quantity` on the step, from its start, middle and end samples and, when the
/// parabola through them has its top inside, a search between.
fn step_peak(
    step: &dyn FlightStep,
    quantity: Quantity,
    samples: &[Sample; 3],
) -> Result<Peak, SimError> {
    let peak_of = |sample: &Sample| Peak {
        value: quantity.of(sample),
        time_s: sample.time_s,
        height_above_ground_m: sample.height_above_ground_m,
    };
    let [start, middle, end] = samples.each_ref().map(peak_of);
    let mut best = [middle, end]
        .into_iter()
        .fold(start, |x, y| if y.value > x.value { y } else { x });
    if top_inside(start.value, middle.value, end.value) {
        let found = golden_max(step.start_s(), step.end_s(), |t| {
            step.sample(t).map(|sample| peak_of(&sample))
        })?;
        if found.value > best.value {
            best = found;
        }
    }
    Ok(best)
}

fn static_of(s: &Stability) -> &Margin {
    &s.static_margin
}

fn flight_of(s: &Stability) -> &Margin {
    &s.flight_margin
}

/// How far below a least margin another must be to replace it: this fraction of the least, or of
/// 1 calibre below 1 calibre. A flat margin, which rounding can nudge by an ulp, keeps its first
/// time.
const MARGIN_TIE: f64 = 1e-12;

/// Whether margin `a` is below `b` by more than [`MARGIN_TIE`].
fn clearly_below(a: f64, b: f64) -> bool {
    b - a > MARGIN_TIE * b.abs().max(1.0)
}

/// The least of one margin on the step `[a, b]`, from its `entries` at the start, middle and end
/// and, when all three are defined and the parabola through them has its bottom inside, a search
/// between; `None` if it is nowhere defined among them.
fn least_margin(
    step: &dyn FlightStep,
    a: f64,
    b: f64,
    entries: &[Stability; 3],
    pick: fn(&Stability) -> &Margin,
) -> Result<Option<Peak>, SimError> {
    let at = |s: &Stability| {
        pick(s).margin_cal.map(|value| Peak {
            value,
            time_s: s.time_s,
            height_above_ground_m: s.height_above_ground_m,
        })
    };
    let mut best: Option<Peak> = None;
    let mut keep = |candidate: Peak| {
        if best.is_none_or(|peak| clearly_below(candidate.value, peak.value)) {
            best = Some(candidate);
        }
    };
    let points = entries.each_ref().map(at);
    points.into_iter().flatten().for_each(&mut keep);
    if let [Some(start), Some(middle), Some(end)] = points
        && top_inside(-start.value, -middle.value, -end.value)
    {
        // Searched as the largest of its negative; an undefined margin is never the least.
        let found = golden_max(a, b, |t| {
            step.stability(t).map(|s| {
                let peak = at(&s);
                Peak {
                    value: peak.map_or(f64::NEG_INFINITY, |p| -p.value),
                    time_s: s.time_s,
                    height_above_ground_m: s.height_above_ground_m,
                }
            })
        })?;
        if found.value.is_finite() {
            keep(Peak {
                value: -found.value,
                ..found
            });
        }
    }
    Ok(best)
}

/// The ejection delay that would fire a motor's charge at apogee.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OptimumDelay {
    /// The motor, by its index in [`hpr_design::Assembly::motors`].
    pub motor: usize,
    /// When it burned out, s after launch.
    pub burnout_s: f64,
    /// When the rocket reached apogee with its charges held, s after launch.
    pub apogee_s: f64,
    /// How high its centre of mass was then, m above the launch site.
    pub apogee_height_above_ground_m: f64,
    /// The delay from its burnout to that apogee, s.
    pub delay_s: f64,
}

/// The optimum ejection delay of each motor that burns out before apogee: the time from its
/// burnout to the apogee of the same flight with every recovery charge held, so that the answer
/// is a property of the rocket, its motors and its air, and not of the delay flown
/// ([Loft lesson L94][l94]).
///
/// - The held flight holds the stack's devices and any separation with nothing ahead of it left
///   to burn, which is part of the recovery. A powered separation still happens.
/// - A motor that burns out after that apogee, or never lights, has none. Nor does a motor in a
///   body a powered separation drops: its charge fires in that body, which never reaches the
///   stack's apogee.
///
/// It is `None` when the held flight has no apogee (it never lifted off, or hit its time cap).
///
/// # Errors
///
/// The flight's.
///
/// [l94]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l94
pub fn optimum_delays(simulation: &Simulation) -> Result<Option<Vec<OptimumDelay>>, SimError> {
    let held = simulation.with_recovery_held();
    let result = held.run(&mut ())?;
    let Some((apogee_s, apogee_height_above_ground_m)) = result
        .event(EventKind::Apogee)
        .map(|event| (event.sample.time_s, event.sample.height_above_ground_m))
    else {
        return Ok(None);
    };
    let assembly = simulation.assembly();
    // A motor lit by a separation has its time only from the flight's ignition event.
    let known = assembly.ignition_times_s(|_| None);
    let dropped = |stage: usize| {
        result
            .bodies
            .iter()
            .any(|body| (body.stages.0..=body.stages.1).contains(&stage))
    };
    Ok(Some(
        assembly
            .motors
            .iter()
            .enumerate()
            .filter(|(_, placed)| !dropped(placed.stage))
            .filter_map(|(motor, placed)| {
                let ignition_s = result
                    .event(EventKind::Ignition(motor))
                    .map(|event| event.sample.time_s)
                    .or(known.get(motor).copied().flatten())?;
                let burnout_s = ignition_s + placed.mounted.motor.burnout_time_s();
                (burnout_s <= apogee_s).then_some(OptimumDelay {
                    motor,
                    burnout_s,
                    apogee_s,
                    apogee_height_above_ground_m,
                    delay_s: apogee_s - burnout_s,
                })
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use hpr_aero::{AeroModel, Flow};
    use hpr_atmos::ConstantWind;
    use hpr_design::Rocket;
    use serde_json::json;

    use super::*;
    use crate::flight::FlightSettings;
    use crate::integrator::{Adaptive, Method};
    use crate::rail::Rail;
    use crate::recorder::{Channel, Recorder};
    use crate::recovery::{Device, DeviceDrag, Trigger};
    use crate::testing::{UniformAir, analytic_environment, design, site, windy_environment};

    const G: f64 = 9.806_65;

    fn valetudo(environment: Environment, settings: FlightSettings) -> Simulation {
        Simulation::new(
            &design("rocketpy-valetudo"),
            "example",
            environment,
            Rail::vertical(3.0),
            settings,
        )
        .unwrap()
    }

    fn capped(max_time_s: f64) -> FlightSettings {
        FlightSettings {
            max_time_s,
            ..FlightSettings::default()
        }
    }

    fn fly(simulation: &Simulation) -> (FlightResult, FlightMetrics) {
        let mut metrics = FlightMetrics::new();
        let result = simulation.run(&mut metrics).unwrap();
        (result, metrics)
    }

    /// A conical nose `0.3` m long of radius `R = 0.05` m, a `0.5` m tube, and a conical boattail
    /// `0.4` m long down to `r_m`, with no fins: Barrowman gives the nose a slope of 2 at
    /// `2/3` of its length and the boattail `2((r/R)² − 1)` at `L/3 · (1 + 1/(1 + R/r))` aft of
    /// its fore end (Barrowman's eq. 44, `hpr_aero`'s hand values, L89).
    fn nose_and_boattail(r_m: f64) -> AeroModel {
        let material = json!({"name": "test", "density": {"kind": "bulk", "kg_m3": 1000.0}});
        let rocket: Rocket = serde_json::from_value(json!({
            "name": "",
            "stages": [{
                "id": "stage",
                "name": "",
                "components": [
                    {"id": "nose", "name": "", "part": {"nose_cone": {
                        "shape": {"kind": "conical"}, "length_m": 0.3, "base_radius_m": 0.05,
                        "wall": {"kind": "filled"}, "shoulder": null, "material": material}}},
                    {"id": "tube", "name": "", "part": {"body_tube": {
                        "length_m": 0.5, "outer_radius_m": 0.05, "thickness_m": 0.005,
                        "material": material}}},
                    {"id": "boattail", "name": "", "part": {"transition": {
                        "shape": {"kind": "conical"}, "length_m": 0.4, "fore_radius_m": 0.05,
                        "aft_radius_m": r_m, "wall": {"kind": "filled"}, "material": material}}}
                ]
            }],
            "reference_diameter": {"kind": "maximum"},
            "configurations": []
        }))
        .unwrap();
        AeroModel::new(&rocket.layout().unwrap()).unwrap()
    }

    /// The hand values of [`nose_and_boattail`]: `(slope, Σ |slopes|, C_mα about x_cg, x_cp)`.
    fn nose_and_boattail_by_hand(r_m: f64, cg_m: f64) -> (f64, f64, f64, f64) {
        let (big_r, d) = (0.05, 0.1);
        let (nose_slope, nose_x) = (2.0, 0.3 * 2.0 / 3.0);
        let tail_slope = 2.0 * ((r_m / big_r).powi(2) - 1.0);
        let tail_x = 0.8 + 0.4 / 3.0 * (1.0 + 1.0 / (1.0 + big_r / r_m));
        let slope = nose_slope + tail_slope;
        let moment = -(nose_slope * (nose_x - cg_m) + tail_slope * (tail_x - cg_m)) / d;
        let cp = (nose_slope * nose_x + tail_slope * tail_x) / slope;
        (slope, nose_slope + tail_slope.abs(), moment, cp)
    }

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = ((got - want) / want).abs();
        assert!(
            err <= rel,
            "{what}: got {got}, want {want}, rel err {err:e}"
        );
    }

    #[test]
    fn static_margin_undefined_when_cn_alpha_near_zero() {
        // L33: a boattail down to a tenth of the radius all but cancels the nose's slope
        // (2 − 1.98 = 0.02 against a sum of 3.98). The quotient still exists, and it is the kind
        // of number Loft published: hundreds of calibres.
        let cg_m = 0.5;
        let aero = nose_and_boattail(0.005);
        let (slope, sum, moment, cp) = nose_and_boattail_by_hand(0.005, cg_m);
        let raw = aero.normal_force(&Flow::axial(0.0)).unwrap();
        close(raw.slope_per_rad, slope, 1e-9, "net slope");
        close(raw.cp_station_m.unwrap(), cp, 1e-9, "the quotient");
        assert!(
            ((cp - cg_m) / 0.1).abs() > 100.0,
            "a margin of {} cal",
            (cp - cg_m) / 0.1
        );
        let static_margin = stability(&aero, 0.0, 0.0, 0.0, cg_m, 0.3)
            .unwrap()
            .static_margin;
        assert_eq!(static_margin.margin_cal, None);
        assert_eq!(static_margin.cp_station_m, None);
        close(
            static_margin.slope_magnitude_sum_per_rad,
            sum,
            1e-12,
            "Σ |slopes|",
        );
        // The couple stays: finite, and destabilising (positive).
        close(
            static_margin.pitch_moment_slope_per_rad,
            moment,
            1e-9,
            "C_mα",
        );
        assert!(moment > 0.0);

        // Both sides of the limit: κ = Σ|C_Nα,i| / C_Nα = 2/ρ² − 1 with ρ = r/R, so the margin
        // is given from κ = √10, ρ = √(2/(1 + √10)) = 0.6932, up.
        for (r_m, defined) in [
            (0.035, true),
            (0.0345, false),
            (0.04, true),
            (0.0215, false),
        ] {
            let (slope, sum, moment, cp) = nose_and_boattail_by_hand(r_m, cg_m);
            assert_eq!(sum / slope <= MARGIN_CONDITION_LIMIT, defined, "r = {r_m}");
            let m = margin(&nose_and_boattail(r_m), &Flow::axial(0.0), cg_m).unwrap();
            close(m.pitch_moment_slope_per_rad, moment, 1e-9, "C_mα");
            if defined {
                close(m.margin_cal.unwrap(), (cp - cg_m) / 0.1, 1e-9, "margin");
                // C_mα = −C_Nα · margin.
                close(
                    m.pitch_moment_slope_per_rad,
                    -slope * m.margin_cal.unwrap(),
                    1e-9,
                    "C_mα",
                );
            } else {
                assert_eq!(m.margin_cal, None, "r = {r_m}");
            }
        }
    }

    #[test]
    fn ordinary_rockets_keep_their_margin() {
        // The other side of the limit: every design in `validation/designs/`, at Mach 0 to 2 and
        // angles of attack of 0° to 20° (the points below), is far from it. Its slopes nearly all push one way.
        let mut worst: f64 = 0.0;
        for name in [
            "synthetic-54mm-three-fin",
            "rocketpy-valetudo",
            "rocketpy-calisto-tests-motor-at-minus-1.373",
            "rocketpy-ndrt-2020-nose-to-tail",
            "rocketpy-prometheus-2022-generic-motor",
            "rocketpy-juno-iii",
            "synthetic-two-stage-75mm-54mm",
            "mil-hdbk-762-sample-rocket",
            "rocketpy-bella-lui",
            "rocketpy-calisto-getting-started-motor-at-minus-1.255",
            "rocketpy-cavour",
            "wind-tunnel-arcas-robin-long",
            "wind-tunnel-arcas-robin-short",
        ] {
            let aero = AeroModel::new(&design(name).layout().unwrap()).unwrap();
            for mach in [0.0, 0.3, 0.8, 1.2, 2.0] {
                for alpha_deg in [0.0, 5.0, 10.0, 20.0] {
                    let flow = Flow::new(mach, f64::to_radians(alpha_deg), 0.0);
                    let m = margin(&aero, &flow, 0.0).unwrap();
                    assert!(m.margin_cal.is_some(), "{name} at {mach}, {alpha_deg}°");
                    worst = worst.max(m.slope_magnitude_sum_per_rad / m.normal_force_slope_per_rad);
                }
            }
        }
        // 1.35 at worst.
        assert!(worst < 1.5, "{worst}");
    }

    #[test]
    fn a_pure_couple_keeps_its_moment() {
        // A nose (slope 2 at 0.2 m), then a step down from R = 0.05 m to 0.03 m at 0.8 m and a
        // conical flare back to 0.05 m over 0.2 m: the step's −2(1 − 0.36) = −1.28 at 0.8 m and
        // the flare's +1.28 at 0.8 + (0.2/3)(1 + 1/1.6) m cancel, leaving that component a pure
        // couple. It still turns the rocket.
        let material = json!({"name": "test", "density": {"kind": "bulk", "kg_m3": 1000.0}});
        let tube = |length_m: f64| {
            json!({"body_tube": {"length_m": length_m, "outer_radius_m": 0.05,
                "thickness_m": 0.005, "material": material}})
        };
        let rocket: Rocket = serde_json::from_value(json!({
            "name": "",
            "stages": [{"id": "stage", "name": "", "components": [
                {"id": "nose", "name": "", "part": {"nose_cone": {
                    "shape": {"kind": "conical"}, "length_m": 0.3, "base_radius_m": 0.05,
                    "wall": {"kind": "filled"}, "shoulder": null, "material": material}}},
                {"id": "tube", "name": "", "part": tube(0.5)},
                {"id": "flare", "name": "", "part": {"transition": {
                    "shape": {"kind": "conical"}, "length_m": 0.2, "fore_radius_m": 0.03,
                    "aft_radius_m": 0.05, "wall": {"kind": "filled"}, "material": material}}},
                {"id": "tail", "name": "", "part": tube(0.3)}
            ]}],
            "reference_diameter": {"kind": "maximum"},
            "configurations": []
        }))
        .unwrap();
        let aero = AeroModel::new(&rocket.layout().unwrap()).unwrap();
        let cg_m = 0.6;
        let couple = -1.28 * 0.8 + 1.28 * (0.8 + 0.2 / 3.0 * (1.0 + 1.0 / 1.6));
        let m = margin(&aero, &Flow::axial(0.0), cg_m).unwrap();
        close(m.normal_force_slope_per_rad, 2.0, 1e-12, "net slope");
        let cp = (2.0 * 0.2 + couple) / 2.0;
        close(m.margin_cal.unwrap(), (cp - cg_m) / 0.1, 1e-9, "margin");
        let moment = -(2.0 * 0.2 + couple - 2.0 * cg_m) / 0.1;
        close(m.pitch_moment_slope_per_rad, moment, 1e-9, "C_mα");
        close(
            m.pitch_moment_slope_per_rad,
            -2.0 * m.margin_cal.unwrap(),
            1e-12,
            "−C_Nα · margin",
        );
    }

    #[test]
    fn a_normal_force_table_gives_its_own_margin() {
        // A table of one column: C_Nα = 10 on the rocket's reference area, at 1.2 m at every Mach
        // number. The margin is the table's, and κ is 1.
        use hpr_aero::{NormalForceColumn, NormalForceTable};
        use hpr_core::interp::{Extrapolation, Interpolation, Table1D};
        let constant = |y: f64| {
            Table1D::new(
                vec![0.0, 2.0],
                vec![y, y],
                Interpolation::Linear,
                Extrapolation::Clamp,
            )
            .unwrap()
        };
        let table = NormalForceTable::new(vec![NormalForceColumn::new(
            0.0,
            constant(10.0),
            constant(1.2),
        )])
        .unwrap();
        let sim = valetudo(Environment::standard(site()).unwrap(), capped(60.0))
            .with_normal_force_table(table)
            .unwrap();
        let d = sim.aero().reference_diameter_m();
        let m = margin(sim.aero(), &Flow::axial(0.3), 0.8).unwrap();
        close(m.normal_force_slope_per_rad, 10.0, 1e-12, "slope");
        assert_eq!(m.slope_magnitude_sum_per_rad, m.normal_force_slope_per_rad);
        close(m.margin_cal.unwrap(), (1.2 - 0.8) / d, 1e-12, "margin");
        close(
            m.pitch_moment_slope_per_rad,
            -10.0 * (1.2 - 0.8) / d,
            1e-12,
            "C_mα",
        );
    }

    #[test]
    fn peak_acceleration_is_analytic_and_excludes_opening_shock() {
        // L34, first half: a vertical boost in a vacuum under uniform gravity, with no rotation.
        // The nose tip's acceleration is (T − m r″ − 2ṁ r′ + m̈(n − r))/m − g along the axis, from
        // the motor and the assembly directly (as the powered-climb test integrates it). Its
        // largest value on a fine scan, and at every thrust-curve knot from both sides, is the
        // peak.
        let sim = valetudo(analytic_environment(UniformAir::vacuum(), G), capped(4.0));
        let (_, metrics) = fly(&sim);
        let assembly = sim.assembly();
        let placed = &assembly.motors[0];
        let motor = &placed.mounted.motor;
        let burnout_s = motor.burnout_time_s();
        let h = 1e-5;
        let mut knots: Vec<f64> = motor
            .curve()
            .times_s()
            .iter()
            .copied()
            .filter(|t| *t > 0.0 && *t < burnout_s)
            .collect();
        knots.insert(0, 0.0);
        knots.push(burnout_s);
        let props = |t: f64| assembly.mass_properties(t);
        // On the knot interval [a, b], derivatives taken inside it and extended to t.
        let by_hand = |t: f64, a: f64, b: f64| {
            let c = t.clamp(a + h, b - h);
            let (m, r) = (props(t).mass_kg, props(t).cg_m.z);
            let r_mid = props(c).cg_m.z;
            let r2 = (props(c + h).cg_m.z - 2.0 * r_mid + props(c - h).cg_m.z) / (h * h);
            let r1 = (props(c + h).cg_m.z - props(c - h).cg_m.z) / (2.0 * h) + (t - c) * r2;
            let mdot = -motor.state(t).mass_flow_kg_s;
            let mddot = -(motor.state(c + h).mass_flow_kg_s - motor.state(c - h).mass_flow_kg_s)
                / (2.0 * h);
            let thrust = motor.thrust_at_pressure_n(t, 0.0);
            (thrust - m * r2 - 2.0 * mdot * r1 + mddot * (placed.nozzle_m.z - r)) / m - G
        };
        let mut expected: f64 = 0.0;
        for pair in knots.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let n = 2000;
            for i in 0..=n {
                let t = a + (b - a) * f64::from(i) / f64::from(n);
                expected = expected.max(by_hand(t, a, b));
            }
        }
        let peak = metrics.max_acceleration.unwrap();
        close(peak.value, expected, 1e-6, "peak acceleration");
        assert!(peak.time_s < burnout_s);
        // What Loft did: a finite difference of the speed recorded at 100 Hz, which averages the
        // acceleration over each interval and reads the thrust spike low.
        let mut recorder =
            Recorder::new(vec![Channel::Time, Channel::Velocity], Some(0.01)).unwrap();
        sim.run(&mut recorder).unwrap();
        let rows = recorder.rows();
        let differenced = rows
            .windows(2)
            .map(|w| (w[1][3] - w[0][3]) / (w[1][0] - w[0][0]))
            .fold(0.0, f64::max);
        // 1.3% low on this motor's first ramp.
        assert!(
            differenced < 0.99 * peak.value,
            "{differenced} vs {}",
            peak.value
        );

        // Second half: the same boost in air, with a large canopy opening at once while the rocket
        // still climbs fast. The boost's peak is the flight's without the canopy, and the opening
        // shock, several times larger, is kept apart.
        let air = || analytic_environment(UniformAir::sea_level(), G);
        let (_, plain) = fly(&valetudo(air(), capped(60.0)));
        let canopy = Device::new(
            "main",
            DeviceDrag::DragArea { cd_s_m2: 4.0 },
            Trigger::Time { time_s: 4.0 },
        );
        let early = valetudo(air(), capped(60.0))
            .with_recovery(vec![canopy])
            .unwrap();
        let (result, shocked) = fly(&early);
        let deployed = result.event(EventKind::Deployment(0)).unwrap().sample;
        assert!(deployed.airspeed_m_s > 100.0, "{}", deployed.airspeed_m_s);
        let boost = shocked.max_acceleration.unwrap();
        close(
            boost.value,
            plain.max_acceleration.unwrap().value,
            1e-12,
            "boost peak",
        );
        let shock = shocked.max_descent_acceleration.unwrap();
        assert!(
            shock.value > 3.0 * boost.value,
            "{} vs {}",
            shock.value,
            boost.value
        );
        assert_eq!(shock.time_s, deployed.time_s);
        assert_eq!(plain.max_descent_acceleration, None);
    }

    #[test]
    fn unlanded_flight_has_no_ground_hit_speed_and_outputs_name_datum() {
        // L35: a flight stopped by its time cap at 5 s has no landing, no ground-hit speed and no
        // apogee, and says so with `None`, not a zero.
        let environment = || Environment::standard(site()).unwrap();
        let sim = valetudo(environment(), capped(5.0));
        let (result, metrics) = fly(&sim);
        assert_eq!(result.termination, Termination::TimeCap);
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        assert_eq!(summary.landing, None);
        assert_eq!(summary.ground_hit_speed_m_s(), None);
        assert_eq!(summary.apogee, None);
        assert_eq!(summary.max_descent_acceleration_m_s2, None);
        let json = serde_json::to_value(&summary).unwrap();
        assert_eq!(json["landing"], serde_json::Value::Null);
        assert_eq!(json["apogee"], serde_json::Value::Null);

        // The whole flight: the heights name their datum. The centre of mass starts above the
        // site, the apogee's gain counts from there, and the landing is on the site's ellipsoidal
        // height, where the flight stops.
        let sim = valetudo(environment(), capped(600.0));
        let (result, metrics) = fly(&sim);
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        // On a vertical rail the nose tip starts at the aft guide's station above the site, and
        // the centre of mass that far less its own station.
        let start = summary.launch_height_m.unwrap();
        let cg_station_m = -sim.assembly().mass_properties(0.0).cg_m.z;
        close(
            start,
            sim.guides().aft_station_m - cg_station_m,
            1e-9,
            "start",
        );
        let apogee = summary.apogee.unwrap();
        assert_eq!(apogee.gain_m, Some(apogee.height_above_ground_m - start));
        let landing = summary.landing.unwrap();
        assert_eq!(
            summary.ground_hit_speed_m_s(),
            Some(landing.ground_hit_speed_m_s)
        );
        assert!(result.final_sample.height_above_ground_m.abs() < 1e-6);
        let json = serde_json::to_value(&summary).unwrap();
        for field in ["launch_height_m", "apogee"] {
            assert!(
                json[field].is_number() || json[field].is_object(),
                "{field}"
            );
        }
        assert!(json["apogee"]["height_above_ground_m"].is_number());
        assert!(json["apogee"]["gain_m"].is_number());
    }

    #[test]
    fn optimum_delay_independent_of_flown_delay() {
        // L94: a delay of 1 s opens the canopy while the rocket climbs; one of 20 s opens it long
        // after apogee. The optimum is the same for both, and is the time from burnout to the
        // apogee of a flight with no recovery at all.
        let environment = || Environment::standard(site()).unwrap();
        let flown = |delay_s: f64| {
            valetudo(environment(), capped(600.0))
                .with_recovery(vec![Device::new(
                    "main",
                    DeviceDrag::DragArea { cd_s_m2: 1.0 },
                    Trigger::Burnout { motor: 0, delay_s },
                )])
                .unwrap()
        };
        let (early, late) = (flown(1.0), flown(20.0));
        let bare = valetudo(environment(), capped(600.0));
        let bare_apogee_s = bare
            .run(&mut ())
            .unwrap()
            .event(EventKind::Apogee)
            .unwrap()
            .sample
            .time_s;
        let early_result = early.run(&mut ()).unwrap();
        let opened_s = early_result
            .event(EventKind::Deployment(0))
            .unwrap()
            .sample
            .time_s;
        assert!(
            opened_s < bare_apogee_s - 5.0,
            "{opened_s} vs {bare_apogee_s}"
        );

        let a = optimum_delays(&early).unwrap().unwrap();
        let b = optimum_delays(&late).unwrap().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 1);
        let burnout_s = bare.assembly().motors[0].mounted.motor.burnout_time_s();
        assert_eq!(a[0].burnout_s, burnout_s);
        close(a[0].apogee_s, bare_apogee_s, 1e-9, "apogee");
        close(a[0].delay_s, bare_apogee_s - burnout_s, 1e-9, "delay");
        // It doesn't touch the simulation it was given.
        assert_eq!(early.run(&mut ()).unwrap(), early_result);
    }

    #[test]
    fn peaks_are_refined_inside_steps() {
        // Max q and max Mach come between the integrator's steps, some in a step's outer quarter,
        // where its middle sample is below an end. On each rocket every peak is at least every
        // sample of a millisecond record, and within the record's spacing of the best.
        for (name, configuration) in [
            ("rocketpy-valetudo", "example"),
            ("rocketpy-juno-iii", "example"),
            ("rocketpy-calisto-tests-motor-at-minus-1.373", "example"),
        ] {
            let sim = Simulation::new(
                &design(name),
                configuration,
                Environment::standard(site()).unwrap(),
                Rail::vertical(5.0),
                capped(20.0),
            )
            .unwrap();
            let mut recorder = Recorder::new(
                vec![Channel::Time, Channel::DynamicPressure, Channel::Mach],
                Some(1e-3),
            )
            .unwrap();
            sim.run(&mut recorder).unwrap();
            let (result, metrics) = fly(&sim);
            let summary = metrics.summary(&result, sim.environment()).unwrap();
            for (column, peak) in [
                (1, summary.max_dynamic_pressure_pa.unwrap()),
                (2, summary.max_mach.unwrap()),
            ] {
                let recorded = recorder
                    .rows()
                    .iter()
                    .map(|row| row[column])
                    .fold(0.0, f64::max);
                assert!(
                    peak.value >= recorded,
                    "{name}: {} < {recorded}",
                    peak.value
                );
                assert!(
                    peak.value - recorded < 1e-4 * peak.value,
                    "{name}: {}",
                    peak.value
                );
            }
            if name == "rocketpy-valetudo" {
                // At the speed's peak the acceleration is zero, so q = ½ρv² falls there with the
                // density (dq/dt = ½v² dρ/dt < 0) and peaked earlier; the Mach number still rises
                // with the falling speed of sound, and peaks later.
                let (q, mach) = (
                    summary.max_dynamic_pressure_pa.unwrap(),
                    summary.max_mach.unwrap(),
                );
                let speed = summary.max_speed_m_s.unwrap();
                assert!(q.time_s < speed.time_s, "{} vs {}", q.time_s, speed.time_s);
                assert!(
                    speed.time_s < mach.time_s,
                    "{} vs {}",
                    speed.time_s,
                    mach.time_s
                );
                assert!(q.height_above_ground_m > 0.0);
            }
        }
    }

    #[test]
    fn landings_are_placed_on_the_ellipsoid() {
        // A 6 m/s wind from the west carries the rocket east. Over a few hundred metres the
        // landing's latitude and longitude are the site's plus the local displacement over the
        // radii of curvature M and N at the site's height, to second order in distance/radius.
        let wind = ConstantWind::new(6.0, 1.5 * std::f64::consts::PI).unwrap();
        let canopy = Device::new(
            "main",
            DeviceDrag::DragArea { cd_s_m2: 0.5 },
            Trigger::Apogee,
        );
        let sim = Simulation::new(
            &design("rocketpy-valetudo"),
            "example",
            windy_environment(wind),
            Rail::vertical(3.0),
            capped(600.0),
        )
        .unwrap()
        .with_recovery(vec![canopy])
        .unwrap();
        let (result, metrics) = fly(&sim);
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        let landing = summary.landing.unwrap();
        assert!(landing.east_m > 100.0, "{}", landing.east_m);
        let place = site();
        let (a, f) = (6_378_137.0, 1.0 / 298.257_223_563);
        let e2 = f * (2.0 - f);
        let s = place.latitude_rad.sin();
        let w = (1.0 - e2 * s * s).sqrt();
        let n = a / w + place.height_m;
        let m = a * (1.0 - e2) / (w * w * w) + place.height_m;
        let lat = place.latitude_rad + landing.north_m / m;
        let lon = place.longitude_rad + landing.east_m / (n * place.latitude_rad.cos());
        let second_order = (landing.distance_m / a).powi(2);
        assert!((landing.latitude_deg.to_radians() - lat).abs() < 10.0 * second_order);
        assert!((landing.longitude_deg.to_radians() - lon).abs() < 10.0 * second_order);
        close(
            landing.distance_m,
            landing.east_m.hypot(landing.north_m),
            1e-15,
            "distance",
        );
        assert_eq!(landing.body, None);
        assert!(landing.descent_rate_m_s > 0.0);
    }

    #[test]
    fn stability_is_kept_from_rail_exit_to_apogee() {
        let sim = valetudo(Environment::standard(site()).unwrap(), capped(600.0));
        let (result, metrics) = fly(&sim);
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        let series = metrics.stability();
        let exit_s = result.event(EventKind::RailExit).unwrap().sample.time_s;
        let apogee_s = summary.apogee.unwrap().time_s;
        assert_eq!(series.first().unwrap().time_s, exit_s);
        assert_eq!(series.last().unwrap().time_s, apogee_s);
        assert!(series.windows(2).all(|w| w[0].time_s < w[1].time_s));
        // The static margin at the rail exit: the centre of pressure at Mach 0 against the centre
        // of mass then, from the assembly and the aerodynamic model directly.
        let first = series[0];
        let lit = sim.assembly().ignition_times_s(|_| None);
        let cg_m = -sim.assembly().mass_properties_lit(exit_s, &lit).cg_m.z;
        close(first.cg_station_m, cg_m, 1e-12, "centre of mass");
        let cp_m = sim
            .aero()
            .normal_force(&Flow::axial(0.0))
            .unwrap()
            .cp_station_m
            .unwrap();
        let d = sim.aero().reference_diameter_m();
        close(
            first.static_margin.margin_cal.unwrap(),
            (cp_m - cg_m) / d,
            1e-12,
            "static margin",
        );
        assert_eq!(summary.rail_exit_stability, Some(first));
        let min = summary.min_static_margin_cal.unwrap();
        assert!(
            series
                .iter()
                .all(|s| s.static_margin.margin_cal.unwrap() >= min.value)
        );
        // The least margins are found inside steps, so they are at or below every entry.
        let least = summary.min_flight_margin_cal.unwrap();
        assert!(
            series
                .iter()
                .all(|s| s.flight_margin.margin_cal.unwrap() >= least.value)
        );
        assert!(least.value > 3.0, "{least:?}");

        // The flight margin is the model's at the flight's own Mach number with the air along the
        // axis, whatever the angle of attack: at the rail exit in a crosswind the rocket meets the
        // air more than 0.2 rad off its axis, and the margin leaves that out.
        let windy = valetudo(
            windy_environment(ConstantWind::new(5.0, 1.5 * std::f64::consts::PI).unwrap()),
            capped(600.0),
        );
        let (result, metrics) = fly(&windy);
        let summary = metrics.summary(&result, windy.environment()).unwrap();
        let exit = result.event(EventKind::RailExit).unwrap().sample;
        assert!(
            exit.angle_of_attack_rad > 0.2,
            "{}",
            exit.angle_of_attack_rad
        );
        let cg_m = -windy
            .assembly()
            .mass_properties_lit(exit.time_s, &lit)
            .cg_m
            .z;
        let expected = margin(windy.aero(), &Flow::axial(exit.mach), cg_m).unwrap();
        let got = summary.rail_exit_stability.unwrap().flight_margin;
        assert_eq!(got.angle_of_attack_rad, 0.0);
        close(got.mach, exit.mach, 1e-12, "Mach number");
        close(
            got.margin_cal.unwrap(),
            expected.margin_cal.unwrap(),
            1e-12,
            "flight margin",
        );
    }

    #[test]
    fn least_margins_do_not_depend_on_where_steps_end() {
        // The same flight flown in steps of at most 1 ms gives the same least margins, to a
        // millionth of a calibre: Valetudo in calm air off a vertical and an 84° rail and in a
        // crosswind, and Prometheus. On all four both leasts are at the rail exit, where the rocket
        // is heaviest and its centre of mass furthest aft: the search inside steps is checked on a
        // two-stage flight (`staging::tests::metrics_follow_a_powered_separation`) instead.
        let fine = |settings: FlightSettings| FlightSettings {
            method: Method::DormandPrince54(Adaptive {
                max_step_s: Some(1e-3),
                ..Adaptive::default()
            }),
            ..settings
        };
        let tilted = Rail {
            elevation_rad: 84.0_f64.to_radians(),
            ..Rail::vertical(2.0)
        };
        let calm = || Environment::standard(site()).unwrap();
        let wind =
            || windy_environment(ConstantWind::new(5.0, 1.5 * std::f64::consts::PI).unwrap());
        let cases = [
            ("rocketpy-valetudo", calm(), Rail::vertical(3.0), 15.0),
            ("rocketpy-valetudo", calm(), tilted, 15.0),
            ("rocketpy-valetudo", wind(), Rail::vertical(3.0), 15.0),
            (
                "rocketpy-prometheus-2022-generic-motor",
                calm(),
                Rail::vertical(5.0),
                60.0,
            ),
        ];
        for (name, environment, rail, max_time_s) in cases {
            let least = |settings: FlightSettings| {
                let sim = Simulation::new(
                    &design(name),
                    "example",
                    environment.clone(),
                    rail,
                    settings,
                )
                .unwrap();
                let (result, metrics) = fly(&sim);
                let summary = metrics.summary(&result, sim.environment()).unwrap();
                let series = metrics.stability().to_vec();
                (
                    summary.min_static_margin_cal.unwrap(),
                    summary.min_flight_margin_cal.unwrap(),
                    series,
                )
            };
            let (coarse_static, coarse_flight, series) = least(capped(max_time_s));
            assert_eq!(coarse_flight.time_s, series[0].time_s, "{name}");
            assert_eq!(coarse_static.time_s, series[0].time_s, "{name}");
            let (fine_static, fine_flight, _) = least(fine(capped(max_time_s)));
            for (coarse, fine, what) in [
                (coarse_static, fine_static, "static"),
                (coarse_flight, fine_flight, "flight"),
            ] {
                assert!(
                    (coarse.value - fine.value).abs() < 1e-6,
                    "{name} {what}: {coarse:?} against {fine:?}"
                );
            }
        }
    }

    /// A step whose only use is its margins: both are `margin_cal(t)`.
    struct MarginStep {
        margin_cal: fn(f64) -> Option<f64>,
    }

    impl FlightStep for MarginStep {
        fn phase(&self) -> Phase {
            Phase::Free
        }
        fn start_s(&self) -> f64 {
            0.0
        }
        fn end_s(&self) -> f64 {
            1.0
        }
        fn state_at(&self, _t_s: f64) -> crate::state::State {
            unreachable!("the margin search reads only the stability")
        }
        fn sample(&self, _t_s: f64) -> Result<Sample, SimError> {
            unreachable!("the margin search reads only the stability")
        }
        fn stability(&self, t_s: f64) -> Result<Stability, SimError> {
            let margin_cal = (self.margin_cal)(t_s);
            let margin = Margin {
                mach: 0.0,
                angle_of_attack_rad: 0.0,
                normal_force_slope_per_rad: 1.0,
                slope_magnitude_sum_per_rad: 1.0,
                pitch_moment_slope_per_rad: -margin_cal.unwrap_or(0.0),
                cp_station_m: margin_cal,
                margin_cal,
            };
            Ok(Stability {
                time_s: t_s,
                height_above_ground_m: 100.0 * t_s,
                dynamic_pressure_pa: 0.0,
                cg_station_m: 0.0,
                reference_diameter_m: 1.0,
                static_margin: margin,
                flight_margin: margin,
            })
        }
    }

    fn least_of(margin_cal: fn(f64) -> Option<f64>) -> Option<Peak> {
        let step = MarginStep { margin_cal };
        let entries = [0.0, 0.5, 1.0].map(|t| step.stability(t).unwrap());
        least_margin(&step, 0.0, 1.0, &entries, flight_of).unwrap()
    }

    #[test]
    fn a_least_margin_between_step_ends_is_found() {
        // 2 + (t − 0.37)² reads 2.137, 2.017 and 2.397 at the step's start, middle and end; the
        // parabola through them has its bottom inside, and the search finds 2 at 0.37 s.
        let least = least_of(|t| Some(2.0 + (t - 0.37).powi(2))).unwrap();
        close(least.value, 2.0, 1e-15, "least");
        assert!((least.time_s - 0.37).abs() < 1e-7, "{least:?}");
        close(least.height_above_ground_m, 37.0, 1e-5, "height");
        // Falling across the step, it is least at the end, with no search.
        let least = least_of(|t| Some(3.0 - t)).unwrap();
        assert_eq!((least.value, least.time_s), (2.0, 1.0));
        // Undefined at the middle: the least of the defined ends, and no search through the gap.
        let least = least_of(|t| (t != 0.5).then_some(2.0 + (t - 0.37).powi(2))).unwrap();
        assert_eq!(least.time_s, 0.0);
        assert_eq!(least_of(|_| None), None);
    }

    #[test]
    fn a_summary_needs_the_flight_it_watched() {
        // A watcher that saw nothing, or saw another flight too, refuses to sum one up; cleared, it
        // sums up the next. A flight started in the air has no launch height, and no climb.
        // The refusal names the steps the watcher saw.
        let refused = |summary: Result<FlightSummary, SimError>, steps: u64| match summary {
            Err(SimError::Domain { what, value }) => {
                assert!(what.starts_with("steps this watcher saw"), "{what}");
                assert_eq!(value, steps as f64);
            }
            other => panic!("{other:?}"),
        };
        let sim = valetudo(Environment::standard(site()).unwrap(), capped(20.0));
        let (result, mut metrics) = fly(&sim);
        let steps = result.stats.accepted_steps;
        refused(FlightMetrics::new().summary(&result, sim.environment()), 0);
        // The same steps, but a flight that ended before the watcher's last step did.
        let mut early = result.clone();
        early.final_sample.time_s -= 1.0;
        refused(metrics.summary(&early, sim.environment()), steps);
        let other = valetudo(Environment::standard(site()).unwrap(), capped(15.0));
        let other_result = other.run(&mut metrics).unwrap();
        refused(
            metrics.summary(&other_result, other.environment()),
            steps + other_result.stats.accepted_steps,
        );
        metrics.clear();
        let other_result = other.run(&mut metrics).unwrap();
        let summary = metrics.summary(&other_result, other.environment()).unwrap();
        assert!(summary.rail_exit_stability.is_some());

        let burnout = result.event(EventKind::Burnout).unwrap().sample;
        let mut aloft = FlightMetrics::new();
        let free = sim
            .run_free(burnout.time_s, burnout.state, &mut aloft)
            .unwrap();
        let summary = aloft.summary(&free, sim.environment()).unwrap();
        assert_eq!(summary.launch_height_m, None);
        assert_eq!(summary.apogee.unwrap().gain_m, None);
        assert_eq!(summary.rail_exit_stability, None);

        // Reused forward in time, not cleared: the first flight's steps are still counted.
        let short = valetudo(Environment::standard(site()).unwrap(), capped(5.0));
        let (short_result, mut reused) = fly(&short);
        let apogee = result.event(EventKind::Apogee).unwrap().sample;
        let free = sim
            .run_free(apogee.time_s, apogee.state, &mut reused)
            .unwrap();
        refused(
            reused.summary(&free, sim.environment()),
            short_result.stats.accepted_steps + free.stats.accepted_steps,
        );

        // Begun in the air on its way down, it keeps no stability: its apogee is behind it.
        let mut falling = FlightMetrics::new();
        let mut state = apogee.state;
        state.velocity_enu_m_s.z = -1.0;
        let free = sim.run_free(apogee.time_s, state, &mut falling).unwrap();
        falling.summary(&free, sim.environment()).unwrap();
        assert!(falling.stability().is_empty());

        // A flight can end without a step, or on a stop so close ahead that the clock just moves
        // there; it is still summed up.
        let stepless = valetudo(
            Environment::standard(site()).unwrap(),
            FlightSettings {
                step_limit: 0,
                ..capped(20.0)
            },
        );
        let (result, metrics) = fly(&stepless);
        assert_eq!(result.stats.accepted_steps, 0);
        let summary = metrics.summary(&result, stepless.environment()).unwrap();
        assert_eq!(summary.termination, Termination::StepLimit);
        assert_eq!(summary.max_speed_m_s, None);
        let cap_s = f64::from_bits(5.0_f64.to_bits() + 3);
        let at_cap = valetudo(Environment::standard(site()).unwrap(), capped(cap_s))
            .with_recovery(vec![Device::new(
                "main",
                DeviceDrag::canopy(crate::recovery::CanopyType::FlatCircular, 1.0),
                Trigger::Time { time_s: 5.0 },
            )])
            .unwrap();
        let (result, metrics) = fly(&at_cap);
        assert_eq!(result.termination, Termination::TimeCap);
        assert_eq!(result.final_sample.time_s, cap_s);
        metrics.summary(&result, at_cap.environment()).unwrap();
    }
}
