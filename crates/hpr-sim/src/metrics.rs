//! What a flight comes to: its peaks, its apogee, its stability margins, the ejection delay that
//! would fire at apogee, and where each body landed ([the flight-metrics milestone][m1-10a] and
//! [its decision record][adr-077]; the guide's [Flight metrics][page] page).
//!
//! - [`FlightMetrics`] is an [`Observer`]: fly a [`Simulation`] with it, then ask it for a
//!   [`FlightSummary`]. It finds each peak on the integrator's dense output, not in a recorded
//!   table, so a peak between two rows is not missed ([Loft lesson L34][l34]).
//! - [`stability`] gives the static margin and the margin in the flight's own air at one instant,
//!   and [`margin`] the margin of one flow. A margin that would be a ratio of nearly cancelling
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

/// The largest ratio `Σ |C_Nα,i| / C_Nα` at which a margin is given.
///
/// The centre of pressure is `x_cp = Σ C_Nα,i x_i / Σ C_Nα,i`. An error `ε C_Nα,j` in one
/// component's slope moves it by `ε C_Nα,j (x_j − x_cp) / C_Nα`, which is at most
/// `ε κ L` for a rocket of length `L`, where `κ = Σ |C_Nα,i| / C_Nα`. A rocket whose slopes all
/// push the same way has `κ = 1`; a boattail's negative slope raises it a little. As the net slope
/// goes to zero, `κ` runs away: the loads become a pure couple with no line of action, and the
/// quotient is noise (Loft published ±12 to 15 calibres so, [lesson L33][l33]). At `κ = 10` a 1%
/// error in one component's slope can move the centre of pressure by a tenth of the rocket's
/// length; past it hpr gives no margin, only the pitch-moment slope, which stays finite.
///
/// [l33]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l33
pub const MARGIN_CONDITION_LIMIT: f64 = 10.0;

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
    /// radian: `C_mα = −Σ C_Nα,i (x_i − x_cg) / d`, stations `x` aft of the nose tip. Negative
    /// restores. It is finite when the margin is not.
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
    /// The centre of mass, m aft of the nose tip.
    pub cg_station_m: f64,
    /// The reference diameter `d` the margins are counted in, m.
    pub reference_diameter_m: f64,
    /// The static margin: the air along the axis at [`STATIC_MARGIN_MACH`], with the centre of
    /// mass of this instant (RocketPy's `static_margin`).
    pub static_margin: Margin,
    /// The margin in the flight's own air: its Mach number, total angle of attack and the roll
    /// of the crossing air, as the equations of motion see them at the centre of mass. RocketPy's
    /// `stability_margin` is this at zero angle of attack.
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
    let (scale, moment_slope) = if aero.normal_force_table().is_some() {
        // The table gives one force at one station.
        let arm = normal.cp_station_m.map_or(0.0, |cp| cp - cg_station_m);
        (slope.abs(), -slope * arm / d)
    } else {
        let components = aero.components(flow)?;
        let scale = components
            .iter()
            .map(|c| c.normal_force.slope_per_rad.abs())
            .sum();
        // A component whose own slope is zero has no station and adds no moment.
        let moment = components
            .iter()
            .map(|c| {
                let force = c.normal_force;
                force
                    .cp_station_m
                    .map_or(0.0, |cp| force.slope_per_rad * (cp - cg_station_m))
            })
            .sum::<f64>();
        (scale, -moment / d)
    };
    let conditioned = slope > 0.0 && slope * MARGIN_CONDITION_LIMIT >= scale;
    let cp_station_m = normal.cp_station_m.filter(|_| conditioned);
    Ok(Margin {
        mach: flow.mach,
        angle_of_attack_rad: flow.alpha_rad,
        normal_force_slope_per_rad: slope,
        slope_magnitude_sum_per_rad: scale,
        pitch_moment_slope_per_rad: moment_slope,
        cp_station_m,
        margin_cal: cp_station_m.map(|cp| (cp - cg_station_m) / d),
    })
}

/// The static margin and the flight margin of `aero` at `time_s`, with the centre of mass
/// `height_above_ground_m` above the launch site and at `cg_station_m` on the airframe, and the
/// flight's air at `flow`.
///
/// # Errors
///
/// As [`margin`].
pub fn stability(
    aero: &AeroModel,
    time_s: f64,
    height_above_ground_m: f64,
    cg_station_m: f64,
    flow: &Flow,
) -> Result<Stability, SimError> {
    Ok(Stability {
        time_s,
        height_above_ground_m,
        cg_station_m,
        reference_diameter_m: aero.reference_diameter_m(),
        static_margin: margin(aero, &Flow::axial(STATIC_MARGIN_MACH), cg_station_m)?,
        flight_margin: margin(aero, flow, cg_station_m)?,
    })
}

/// The apogee of the flight's centre of mass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Apogee {
    /// When, s after launch.
    pub time_s: f64,
    /// The centre of mass's ellipsoidal height above the launch site, m.
    pub height_above_ground_m: f64,
    /// The rise of the centre of mass from where it started, m: the height above the site less
    /// [`FlightSummary::launch_height_m`]. OpenRocket's altitude counts so.
    pub gain_m: f64,
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
    /// The centre of mass's height above the launch site where the flight started, m.
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
    /// The smallest static margin from the rail exit to apogee, calibres, where it is defined.
    pub min_static_margin_cal: Option<Peak>,
    /// The smallest flight margin from the rail exit to apogee, calibres, where it is defined.
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

/// Watches a flight and keeps its peaks, and its stability from the rail exit to apogee.
///
/// Each step is sampled at its start, middle and end; when the middle is above both ends a
/// golden-section search on the dense output finds the peak inside. Thrust-curve knots and events
/// end steps, so a thrust spike's peak is a step's end. Nothing is kept on the pad.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct FlightMetrics {
    launch_height_m: Option<f64>,
    max_speed: Option<Peak>,
    max_mach: Option<Peak>,
    max_q: Option<Peak>,
    max_acceleration: Option<Peak>,
    max_descent_acceleration: Option<Peak>,
    past_apogee: bool,
    stability: Vec<Stability>,
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

    /// The stability from the rail exit to apogee, at every step's end and at the rail exit, in
    /// time order.
    #[must_use]
    pub fn stability(&self) -> &[Stability] {
        &self.stability
    }

    /// The summary of `result`, the flight this watched, flown in `environment`.
    ///
    /// # Errors
    ///
    /// [`SimError::Core`] if a landing has no geodetic coordinates.
    pub fn summary(
        &self,
        result: &FlightResult,
        environment: &Environment,
    ) -> Result<FlightSummary, SimError> {
        let apogee = result.event(EventKind::Apogee).map(|event| {
            let height = event.sample.height_above_ground_m;
            Apogee {
                time_s: event.sample.time_s,
                height_above_ground_m: height,
                gain_m: height - self.launch_height_m.unwrap_or(height),
            }
        });
        let rail_exit = result.event(EventKind::RailExit).map(|event| event.sample);
        let minimum = |pick: fn(&Stability) -> &Margin| {
            self.stability
                .iter()
                .filter_map(|s| {
                    pick(s).margin_cal.map(|value| Peak {
                        value,
                        time_s: s.time_s,
                        height_above_ground_m: s.height_above_ground_m,
                    })
                })
                .min_by(|a, b| a.value.total_cmp(&b.value))
        };
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
            min_static_margin_cal: minimum(|s| &s.static_margin),
            min_flight_margin_cal: minimum(|s| &s.flight_margin),
            rail_exit_stability: rail_exit.and_then(|exit| {
                self.stability
                    .iter()
                    .find(|s| s.time_s == exit.time_s)
                    .copied()
            }),
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
        if self.launch_height_m.is_none() {
            self.launch_height_m = Some(start.height_above_ground_m);
        }
        if phase == Phase::Pad {
            return Ok(());
        }
        let middle = step.sample(0.5 * (a + b))?;
        let end = step.sample(b)?;
        for quantity in Quantity::ALL {
            let (va, vm, vb) = (quantity.of(&start), quantity.of(&middle), quantity.of(&end));
            let best = if vm > va && vm > vb {
                golden_peak(step, quantity, a, b)?
            } else if va >= vb {
                Peak {
                    value: va,
                    time_s: a,
                    height_above_ground_m: start.height_above_ground_m,
                }
            } else {
                Peak {
                    value: vb,
                    time_s: b,
                    height_above_ground_m: end.height_above_ground_m,
                }
            };
            let slot = self.slot(quantity, phase);
            if slot.is_none_or(|peak| best.value > peak.value) {
                *slot = Some(best);
            }
        }
        // On the rail the rail holds the rocket, and its slow climb through the wind makes angles
        // of attack near 90°, so stability starts at the rail exit.
        if !self.past_apogee && phase == Phase::Free {
            if self.stability.is_empty() {
                self.stability.push(step.stability(a)?);
            }
            self.stability.push(step.stability(b)?);
        }
        Ok(())
    }

    fn event(&mut self, event: &FlightEvent) {
        if event.kind == EventKind::Apogee {
            self.past_apogee = true;
        }
    }
}

/// The golden ratio's inverse, `(√5 − 1)/2`.
const INVERSE_PHI: f64 = 0.618_033_988_749_894_9;

/// How finely a peak's time is found, relative to the flight's clock (at least 1 s): 1e-9. The
/// value's error goes as the square of the time's, so it is far below the integration's.
const PEAK_TIME_RESOLUTION: f64 = 1e-9;

/// The largest value of `quantity` on the step `[a, b]`, whose middle is above both ends: a
/// golden-section search (Kiefer 1953) on the step's dense output, each point one evaluation of
/// the equations of motion.
fn golden_peak(
    step: &dyn FlightStep,
    quantity: Quantity,
    mut a: f64,
    mut b: f64,
) -> Result<Peak, SimError> {
    let value = |t: f64| step.sample(t).map(|sample| (quantity.of(&sample), sample));
    let mut c = b - INVERSE_PHI * (b - a);
    let mut d = a + INVERSE_PHI * (b - a);
    let (mut fc, mut sc) = value(c)?;
    let (mut fd, mut sd) = value(d)?;
    while b - a > PEAK_TIME_RESOLUTION * b.abs().max(1.0) {
        if fc >= fd {
            b = d;
            (d, fd, sd) = (c, fc, sc);
            c = b - INVERSE_PHI * (b - a);
            (fc, sc) = value(c)?;
        } else {
            a = c;
            (c, fc, sc) = (d, fd, sd);
            d = a + INVERSE_PHI * (b - a);
            (fd, sd) = value(d)?;
        }
    }
    let (value, at) = if fc >= fd { (fc, sc) } else { (fd, sd) };
    Ok(Peak {
        value,
        time_s: at.time_s,
        height_above_ground_m: at.height_above_ground_m,
    })
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
    /// The delay from its burnout to that apogee, s.
    pub delay_s: f64,
}

/// The optimum ejection delay of each motor that burns out before apogee: the time from its
/// burnout to the apogee of the same flight with every recovery charge held, so that the answer
/// is a property of the rocket, its motors and its air, and not of the delay flown
/// ([Loft lesson L94][l94]). A motor that burns out after that apogee, or never lights, has none.
///
/// It is `None` when the held flight has no apogee: it ended first (a separation after burnout
/// ends the stack's flight), or never lifted off.
///
/// # Errors
///
/// The flight's.
///
/// [l94]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l94
pub fn optimum_delays(simulation: &Simulation) -> Result<Option<Vec<OptimumDelay>>, SimError> {
    let held = simulation.with_recovery_held();
    let result = held.run(&mut ())?;
    let Some(apogee_s) = result
        .event(EventKind::Apogee)
        .map(|event| event.sample.time_s)
    else {
        return Ok(None);
    };
    let assembly = simulation.assembly();
    // A motor lit by a separation has its time only from the flight's ignition event.
    let known = assembly.ignition_times_s(|_| None);
    Ok(Some(
        assembly
            .motors
            .iter()
            .enumerate()
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
        let static_margin = stability(&aero, 0.0, 0.0, cg_m, &Flow::axial(0.3))
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
        // is given from ρ = √(2/11) = 0.4264 up.
        for (r_m, defined) in [(0.0215, true), (0.0210, false), (0.04, true)] {
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
        assert_eq!(apogee.gain_m, apogee.height_above_ground_m - start);
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
        // Max q, max speed and max Mach come between the integrator's steps; each peak is above
        // every sample of a millisecond record and is a local maximum on the dense output.
        let sim = valetudo(Environment::standard(site()).unwrap(), capped(60.0));
        let mut recorder = Recorder::new(
            vec![Channel::Time, Channel::DynamicPressure, Channel::Mach],
            Some(1e-3),
        )
        .unwrap();
        let result = sim.run(&mut recorder).unwrap();
        let (_, metrics) = fly(&sim);
        let summary = metrics.summary(&result, sim.environment()).unwrap();
        let q = summary.max_dynamic_pressure_pa.unwrap();
        let recorded_q = recorder.rows().iter().map(|r| r[1]).fold(0.0, f64::max);
        let recorded_mach = recorder.rows().iter().map(|r| r[2]).fold(0.0, f64::max);
        assert!(q.value >= recorded_q, "{} < {recorded_q}", q.value);
        // The record's spacing bounds how far below the peak its best sample can be.
        assert!(
            q.value - recorded_q < 1e-4 * q.value,
            "{} vs {recorded_q}",
            q.value
        );
        let mach = summary.max_mach.unwrap();
        assert!(mach.value >= recorded_mach);
        assert!(mach.value - recorded_mach < 1e-4 * mach.value);
        // Max q comes after burnout here, as the rocket coasts faster than the air thins.
        // At the speed's peak the acceleration is zero, so q = ½ρv² falls there with the density
        // (dq/dt = ½v² dρ/dt < 0) and peaked earlier; the Mach number still rises with the
        // falling speed of sound, and peaks later.
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
        let exit = summary.rail_exit_stability.unwrap();
        assert_eq!(exit, first);
        assert!(exit.flight_margin.mach > 0.0);
        let min = summary.min_static_margin_cal.unwrap();
        assert!(
            series
                .iter()
                .all(|s| s.static_margin.margin_cal.unwrap() >= min.value)
        );
    }
}
