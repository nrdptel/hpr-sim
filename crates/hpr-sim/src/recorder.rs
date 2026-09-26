//! Watching a flight: the [`Observer`] trait, the [`Sample`] of derived quantities at an instant,
//! and the [`Recorder`], which keeps a chosen set of [`Channel`]s at a fixed interval.

use hpr_core::DVec3;
use serde::{Deserialize, Serialize};

use crate::dynamics::Phase;
use crate::error::SimError;
use crate::flight::FlightEvent;
use crate::state::State;

/// The flight's state and the quantities derived from it at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// Time since launch, s: the flight's clock, from which each motor's ignition is counted.
    pub time_s: f64,
    /// The phase the flight was in.
    pub phase: Phase,
    /// The integrated state.
    pub state: State,
    /// The centre of mass in the launch frame, m.
    pub cg_enu_m: DVec3,
    /// The centre of mass's velocity relative to the launch frame, m/s.
    pub cg_velocity_enu_m_s: DVec3,
    /// The centre of mass's ellipsoidal height above the launch site, m.
    pub height_above_ground_m: f64,
    /// The rate of that height, m/s.
    pub vertical_speed_m_s: f64,
    /// The nose tip's acceleration relative to the launch frame, m/s².
    pub acceleration_enu_m_s2: DVec3,
    /// The airspeed of the centre of mass, m/s.
    pub airspeed_m_s: f64,
    /// Its Mach number.
    pub mach: f64,
    /// The total angle of attack at the centre of mass, rad.
    pub angle_of_attack_rad: f64,
    /// The dynamic pressure, Pa.
    pub dynamic_pressure_pa: f64,
    /// The axial force coefficient `C_A` on the reference area.
    pub axial_coefficient: f64,
    /// The thrust, N.
    pub thrust_n: f64,
    /// The mass, kg.
    pub mass_kg: f64,
    /// The drag area `C_D S` of the open recovery devices, m² (zero before any deploys).
    pub recovery_drag_area_m2: f64,
}

/// What a recorder can keep. Vector channels take three columns (east, north, up or x, y, z).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Channel {
    /// Time since launch, s.
    Time,
    /// The nose tip's position in the launch frame, m.
    Position,
    /// The nose tip's velocity, m/s.
    Velocity,
    /// The attitude quaternion `(w, x, y, z)`, body to launch frame.
    Attitude,
    /// The body rates, rad/s.
    BodyRates,
    /// The centre of mass's position, m.
    CgPosition,
    /// The centre of mass's height above the site, m.
    HeightAboveGround,
    /// The vertical speed, m/s.
    VerticalSpeed,
    /// The nose tip's acceleration, m/s².
    Acceleration,
    /// The airspeed, m/s.
    Airspeed,
    /// The Mach number.
    Mach,
    /// The total angle of attack, rad.
    AngleOfAttack,
    /// The dynamic pressure, Pa.
    DynamicPressure,
    /// The axial force coefficient.
    AxialCoefficient,
    /// The thrust, N.
    Thrust,
    /// The mass, kg.
    Mass,
    /// The open recovery devices' drag area `C_D S`, m².
    RecoveryDragArea,
}

impl Channel {
    /// Every channel, in column order.
    pub const ALL: &'static [Channel] = &[
        Channel::Time,
        Channel::Position,
        Channel::Velocity,
        Channel::Attitude,
        Channel::BodyRates,
        Channel::CgPosition,
        Channel::HeightAboveGround,
        Channel::VerticalSpeed,
        Channel::Acceleration,
        Channel::Airspeed,
        Channel::Mach,
        Channel::AngleOfAttack,
        Channel::DynamicPressure,
        Channel::AxialCoefficient,
        Channel::Thrust,
        Channel::Mass,
        Channel::RecoveryDragArea,
    ];

    /// The column names, with units.
    #[must_use]
    pub fn columns(self) -> Vec<String> {
        let three = |base: &str, unit: &str, axes: [&str; 3]| {
            axes.iter()
                .map(|axis| format!("{base}_{axis}_{unit}"))
                .collect()
        };
        let one = |name: &str| vec![name.to_owned()];
        let enu = ["east", "north", "up"];
        let xyz = ["x", "y", "z"];
        match self {
            Self::Time => one("time_s"),
            Self::Position => three("position", "m", enu),
            Self::Velocity => three("velocity", "m_s", enu),
            Self::Attitude => ["attitude_w", "attitude_x", "attitude_y", "attitude_z"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            Self::BodyRates => three("body_rate", "rad_s", xyz),
            Self::CgPosition => three("cg", "m", enu),
            Self::HeightAboveGround => one("height_above_ground_m"),
            Self::VerticalSpeed => one("vertical_speed_m_s"),
            Self::Acceleration => three("acceleration", "m_s2", enu),
            Self::Airspeed => one("airspeed_m_s"),
            Self::Mach => one("mach"),
            Self::AngleOfAttack => one("angle_of_attack_rad"),
            Self::DynamicPressure => one("dynamic_pressure_pa"),
            Self::AxialCoefficient => one("axial_coefficient"),
            Self::Thrust => one("thrust_n"),
            Self::Mass => one("mass_kg"),
            Self::RecoveryDragArea => one("recovery_drag_area_m2"),
        }
    }

    /// Appends this channel's values from `sample` to `row`.
    pub fn push(self, sample: &Sample, row: &mut Vec<f64>) {
        let s = &sample.state;
        match self {
            Self::Time => row.push(sample.time_s),
            Self::Position => row.extend(s.position_enu_m.to_array()),
            Self::Velocity => row.extend(s.velocity_enu_m_s.to_array()),
            Self::Attitude => {
                let q = s.unit_attitude();
                row.extend([q.w, q.x, q.y, q.z]);
            }
            Self::BodyRates => row.extend(s.body_rate_rad_s.to_array()),
            Self::CgPosition => row.extend(sample.cg_enu_m.to_array()),
            Self::HeightAboveGround => row.push(sample.height_above_ground_m),
            Self::VerticalSpeed => row.push(sample.vertical_speed_m_s),
            Self::Acceleration => row.extend(sample.acceleration_enu_m_s2.to_array()),
            Self::Airspeed => row.push(sample.airspeed_m_s),
            Self::Mach => row.push(sample.mach),
            Self::AngleOfAttack => row.push(sample.angle_of_attack_rad),
            Self::DynamicPressure => row.push(sample.dynamic_pressure_pa),
            Self::AxialCoefficient => row.push(sample.axial_coefficient),
            Self::Thrust => row.push(sample.thrust_n),
            Self::Mass => row.push(sample.mass_kg),
            Self::RecoveryDragArea => row.push(sample.recovery_drag_area_m2),
        }
    }
}

/// One accepted integration step of a flight, with its dense output.
pub trait FlightStep {
    /// The phase.
    fn phase(&self) -> Phase;
    /// When the step starts, s.
    fn start_s(&self) -> f64;
    /// When it ends, s.
    fn end_s(&self) -> f64;
    /// The state at `t` in `[start_s, end_s]`, from the dense output.
    fn state_at(&self, t_s: f64) -> State;
    /// The sample at `t` in `[start_s, end_s]`: the dense output's state and one evaluation of
    /// the equations of motion there.
    ///
    /// # Errors
    ///
    /// Whatever the evaluation returns (it succeeded at the step's own stages).
    fn sample(&self, t_s: f64) -> Result<Sample, SimError>;
}

/// Watches a flight as it runs. Both methods do nothing by default; `()` watches nothing.
pub trait Observer {
    /// Sees every accepted integration step, in order.
    ///
    /// # Errors
    ///
    /// An error stops the flight and is returned from it.
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let _ = step;
        Ok(())
    }

    /// Sees every flight event as it is recorded.
    fn event(&mut self, event: &FlightEvent) {
        let _ = event;
    }
}

impl Observer for () {}

/// Records chosen channels, one row per sample.
///
/// - With an interval, a row at every multiple of `interval_s` after launch that falls within the
///   flight, and a row at every event (unless the last row already has that time).
/// - Without one, a row at the flight's first step's start and at every step's end; events end
///   steps, so they are among the rows.
///
/// A recorder keeps one flight: [`Recorder::clear`] it before recording another. It serializes its
/// settings and rows for inspection; it is built with [`Recorder::new`], not deserialized.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Recorder {
    channels: Vec<Channel>,
    interval_s: Option<f64>,
    /// The next interval sample is at `next_index · interval_s`.
    next_index: u64,
    rows: Vec<Vec<f64>>,
    samples: usize,
    last_time_s: Option<f64>,
}

impl Recorder {
    /// A recorder of `channels`, sampling every `interval_s` seconds (`None`: every step's end).
    ///
    /// # Errors
    ///
    /// [`SimError::Domain`] for an interval that isn't finite and positive.
    pub fn new(channels: Vec<Channel>, interval_s: Option<f64>) -> Result<Self, SimError> {
        if let Some(dt) = interval_s
            && !(dt.is_finite() && dt > 0.0)
        {
            return Err(SimError::Domain {
                what: "recorder interval",
                value: dt,
            });
        }
        Ok(Self {
            channels,
            interval_s,
            next_index: 0,
            rows: Vec::new(),
            samples: 0,
            last_time_s: None,
        })
    }

    /// Forgets the recorded rows, ready for another flight.
    pub fn clear(&mut self) {
        self.next_index = 0;
        self.rows.clear();
        self.samples = 0;
        self.last_time_s = None;
    }

    /// The column names.
    #[must_use]
    pub fn columns(&self) -> Vec<String> {
        self.channels.iter().flat_map(|c| c.columns()).collect()
    }

    /// The recorded rows.
    #[must_use]
    pub fn rows(&self) -> &[Vec<f64>] {
        &self.rows
    }

    fn record(&mut self, sample: &Sample) {
        let mut row = Vec::new();
        for channel in &self.channels {
            channel.push(sample, &mut row);
        }
        self.rows.push(row);
        self.samples += 1;
        self.last_time_s = Some(sample.time_s);
    }
}

impl Observer for Recorder {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        match self.interval_s {
            None => {
                if self.samples == 0 {
                    self.record(&step.sample(step.start_s())?);
                }
                self.record(&step.sample(step.end_s())?);
            }
            Some(dt) => {
                // A flight that starts after launch samples from its start.
                let first = (step.start_s() / dt).ceil();
                if (self.next_index as f64) < first {
                    self.next_index = first as u64;
                }
                if !(dt.is_finite() && dt > 0.0) {
                    return Err(SimError::Domain {
                        what: "recorder interval",
                        value: dt,
                    });
                }
                loop {
                    let t = self.next_index as f64 * dt;
                    if t > step.end_s() {
                        break;
                    }
                    let sample = step.sample(t)?;
                    self.record(&sample);
                    self.next_index += 1;
                }
            }
        }
        Ok(())
    }

    fn event(&mut self, event: &FlightEvent) {
        if self.interval_s.is_some() && self.last_time_s != Some(event.sample.time_s) {
            self.record(&event.sample);
        }
    }
}
