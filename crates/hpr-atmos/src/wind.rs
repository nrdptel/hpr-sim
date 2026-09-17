//! Mean wind models: constant, power law, logarithmic law and tabulated layers.
//!
//! **Conventions** (`docs/physics/wind.md`):
//!
//! - A wind model returns the velocity of the air in the launch frame's East-North-Up axes, m/s.
//!   Mean wind is horizontal, so the up component is zero.
//! - Directions are **meteorological**: the direction the wind blows *from*, clockwise from true
//!   north, in radians. A 10 m/s wind from the west (`3π/2`) has velocity `(+10, 0, 0)`:
//!
//! ```text
//! v_E = −V sin θ_from,    v_N = −V cos θ_from
//! ```
//!
//! - Models are queried with the geometric height above mean sea level, like the atmosphere
//!   ([`crate::Atmosphere`]). Laws written in height above ground carry the ground's height.
//!
//! Turbulence is separate: see [`crate::dryden`].

use std::f64::consts::{PI, TAU};
use std::fmt;

use hpr_core::DVec3;
use hpr_core::interp::Side;
use serde::{Deserialize, Serialize};

use crate::error::{AtmosError, finite, positive};

/// A wind velocity and whether the model extrapolated to produce it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindSample {
    /// Velocity of the air in East-North-Up axes, m/s.
    pub velocity_enu_m_s: DVec3,
    /// `Some` when the height was outside the model's data or valid range; `None` inside it.
    pub extrapolated: Option<Side>,
}

/// A mean wind model: the wind velocity as a function of height.
pub trait Wind: fmt::Debug + Send + Sync {
    /// The wind at geometric height `height_msl_m` above mean sea level.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the height is not finite.
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError>;
}

/// East-North-Up velocity of a horizontal wind of `speed_m_s` blowing from `direction_from_rad`.
pub fn velocity_from_speed_direction(speed_m_s: f64, direction_from_rad: f64) -> DVec3 {
    DVec3::new(
        -speed_m_s * direction_from_rad.sin(),
        -speed_m_s * direction_from_rad.cos(),
        0.0,
    )
}

/// Wraps an angle into `[0, 2π)`.
fn wrap_direction(angle_rad: f64) -> f64 {
    let wrapped = angle_rad.rem_euclid(TAU);
    // rem_euclid can round up to exactly 2π for tiny negative inputs.
    if wrapped >= TAU { 0.0 } else { wrapped }
}

fn check_direction(direction_from_rad: f64) -> Result<f64, AtmosError> {
    Ok(wrap_direction(finite(
        "wind direction (rad)",
        direction_from_rad,
    )?))
}

fn check_speed(speed_m_s: f64) -> Result<f64, AtmosError> {
    let speed = finite("wind speed (m/s)", speed_m_s)?;
    if speed < 0.0 {
        return Err(AtmosError::Domain {
            what: "wind speed (m/s)",
            value: speed,
        });
    }
    Ok(speed)
}

/// The same wind at every height.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ConstantWindData", into = "ConstantWindData")]
pub struct ConstantWind {
    speed_m_s: f64,
    direction_from_rad: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConstantWindData {
    speed_m_s: f64,
    direction_from_rad: f64,
}

impl TryFrom<ConstantWindData> for ConstantWind {
    type Error = AtmosError;

    fn try_from(data: ConstantWindData) -> Result<Self, AtmosError> {
        ConstantWind::new(data.speed_m_s, data.direction_from_rad)
    }
}

impl From<ConstantWind> for ConstantWindData {
    fn from(wind: ConstantWind) -> Self {
        ConstantWindData {
            speed_m_s: wind.speed_m_s,
            direction_from_rad: wind.direction_from_rad,
        }
    }
}

impl ConstantWind {
    /// A constant wind of `speed_m_s` from `direction_from_rad` (wrapped into `[0, 2π)`).
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the speed is negative or either value is not finite.
    pub fn new(speed_m_s: f64, direction_from_rad: f64) -> Result<Self, AtmosError> {
        Ok(ConstantWind {
            speed_m_s: check_speed(speed_m_s)?,
            direction_from_rad: check_direction(direction_from_rad)?,
        })
    }

    /// Calm air.
    pub fn calm() -> Self {
        ConstantWind {
            speed_m_s: 0.0,
            direction_from_rad: 0.0,
        }
    }
}

impl Wind for ConstantWind {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        finite("height (m)", height_msl_m)?;
        Ok(WindSample {
            velocity_enu_m_s: velocity_from_speed_direction(
                self.speed_m_s,
                self.direction_from_rad,
            ),
            extrapolated: None,
        })
    }
}

/// A wind whose speed grows with height above ground as a power law, from one direction:
///
/// ```text
/// V(z) = V_ref (z / z_ref)^α,   z = H − H_ground > 0;   V = 0 for z ≤ 0
/// ```
///
/// Source: NASA/TM-2008-215633, *Terrestrial Environment (Climatic) Criteria Guidelines for Use
/// in Aerospace Vehicle Development* (2008), §2.2.5.2, eq. 2.1, pinned as `nasa-tm-2008-215633`.
/// There it describes peak winds below 150 m, with `z_ref = 18.3 m` and exponents from 0.14 to
/// about 0.2 (Table 2-1); `docs/physics/wind.md` says how to choose one. Heights below ground are
/// flagged.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PowerLawWindData", into = "PowerLawWindData")]
pub struct PowerLawWind {
    reference_speed_m_s: f64,
    reference_height_agl_m: f64,
    exponent: f64,
    direction_from_rad: f64,
    ground_msl_m: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PowerLawWindData {
    reference_speed_m_s: f64,
    reference_height_agl_m: f64,
    exponent: f64,
    direction_from_rad: f64,
    ground_msl_m: f64,
}

impl TryFrom<PowerLawWindData> for PowerLawWind {
    type Error = AtmosError;

    fn try_from(d: PowerLawWindData) -> Result<Self, AtmosError> {
        PowerLawWind::new(
            d.reference_speed_m_s,
            d.reference_height_agl_m,
            d.exponent,
            d.direction_from_rad,
            d.ground_msl_m,
        )
    }
}

impl From<PowerLawWind> for PowerLawWindData {
    fn from(w: PowerLawWind) -> Self {
        PowerLawWindData {
            reference_speed_m_s: w.reference_speed_m_s,
            reference_height_agl_m: w.reference_height_agl_m,
            exponent: w.exponent,
            direction_from_rad: w.direction_from_rad,
            ground_msl_m: w.ground_msl_m,
        }
    }
}

impl PowerLawWind {
    /// A power-law profile through `reference_speed_m_s` at `reference_height_agl_m` above a
    /// ground at `ground_msl_m`, with exponent `exponent`, blowing from `direction_from_rad`.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the speed is negative, the reference height is not positive,
    /// the exponent is negative, or any value is not finite.
    pub fn new(
        reference_speed_m_s: f64,
        reference_height_agl_m: f64,
        exponent: f64,
        direction_from_rad: f64,
        ground_msl_m: f64,
    ) -> Result<Self, AtmosError> {
        let exponent = finite("power-law exponent", exponent)?;
        if exponent < 0.0 {
            return Err(AtmosError::Domain {
                what: "power-law exponent",
                value: exponent,
            });
        }
        Ok(PowerLawWind {
            reference_speed_m_s: check_speed(reference_speed_m_s)?,
            reference_height_agl_m: positive("reference height (m)", reference_height_agl_m)?,
            exponent,
            direction_from_rad: check_direction(direction_from_rad)?,
            ground_msl_m: finite("ground height (m)", ground_msl_m)?,
        })
    }
}

impl Wind for PowerLawWind {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        let z = finite("height (m)", height_msl_m)? - self.ground_msl_m;
        let (speed, extrapolated) = if z > 0.0 {
            let ratio = z / self.reference_height_agl_m;
            (self.reference_speed_m_s * ratio.powf(self.exponent), None)
        } else {
            (0.0, (z < 0.0).then_some(Side::Below))
        };
        Ok(WindSample {
            velocity_enu_m_s: velocity_from_speed_direction(speed, self.direction_from_rad),
            extrapolated,
        })
    }
}

/// A wind whose speed follows the neutral logarithmic law above ground, from one direction:
///
/// ```text
/// V(z) = V_ref ln(z / z₀) / ln(z_ref / z₀),   z = H − H_ground > z₀;   V = 0 for 0 ≤ z ≤ z₀
/// ```
///
/// with roughness length `z₀`. Sources: MIL-F-8785C §3.7.3.2 (with `z_ref = 20 ft`), and the
/// neutral surface-layer law `V = (u*/κ) ln(z/z₀)` of WMO-No. 8 (2023), Vol. I, chapter 5 annex,
/// whose Davenport–Wieringa classes give `z₀ = 0.03 m` for open grassland. Heights below ground
/// are flagged; see `docs/physics/wind.md`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LogLawWindData", into = "LogLawWindData")]
pub struct LogLawWind {
    reference_speed_m_s: f64,
    reference_height_agl_m: f64,
    roughness_length_m: f64,
    direction_from_rad: f64,
    ground_msl_m: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LogLawWindData {
    reference_speed_m_s: f64,
    reference_height_agl_m: f64,
    roughness_length_m: f64,
    direction_from_rad: f64,
    ground_msl_m: f64,
}

impl TryFrom<LogLawWindData> for LogLawWind {
    type Error = AtmosError;

    fn try_from(d: LogLawWindData) -> Result<Self, AtmosError> {
        LogLawWind::new(
            d.reference_speed_m_s,
            d.reference_height_agl_m,
            d.roughness_length_m,
            d.direction_from_rad,
            d.ground_msl_m,
        )
    }
}

impl From<LogLawWind> for LogLawWindData {
    fn from(w: LogLawWind) -> Self {
        LogLawWindData {
            reference_speed_m_s: w.reference_speed_m_s,
            reference_height_agl_m: w.reference_height_agl_m,
            roughness_length_m: w.roughness_length_m,
            direction_from_rad: w.direction_from_rad,
            ground_msl_m: w.ground_msl_m,
        }
    }
}

impl LogLawWind {
    /// A logarithmic profile through `reference_speed_m_s` at `reference_height_agl_m` above a
    /// ground at `ground_msl_m` with roughness length `roughness_length_m`, blowing from
    /// `direction_from_rad`.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the speed is negative, the roughness length is not positive, the
    /// reference height is not above the roughness length, or any value is not finite.
    pub fn new(
        reference_speed_m_s: f64,
        reference_height_agl_m: f64,
        roughness_length_m: f64,
        direction_from_rad: f64,
        ground_msl_m: f64,
    ) -> Result<Self, AtmosError> {
        let z0 = positive("roughness length (m)", roughness_length_m)?;
        let z_ref = positive("reference height (m)", reference_height_agl_m)?;
        if z_ref <= z0 {
            return Err(AtmosError::Domain {
                what: "reference height above the roughness length (m)",
                value: z_ref,
            });
        }
        Ok(LogLawWind {
            reference_speed_m_s: check_speed(reference_speed_m_s)?,
            reference_height_agl_m: z_ref,
            roughness_length_m: z0,
            direction_from_rad: check_direction(direction_from_rad)?,
            ground_msl_m: finite("ground height (m)", ground_msl_m)?,
        })
    }
}

impl Wind for LogLawWind {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        let z = finite("height (m)", height_msl_m)? - self.ground_msl_m;
        let z0 = self.roughness_length_m;
        let speed = if z > z0 {
            self.reference_speed_m_s * (z / z0).ln() / (self.reference_height_agl_m / z0).ln()
        } else {
            0.0
        };
        Ok(WindSample {
            velocity_enu_m_s: velocity_from_speed_direction(speed, self.direction_from_rad),
            extrapolated: (z < 0.0).then_some(Side::Below),
        })
    }
}

/// How a [`LayeredWind`] fills in between its levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WindInterpolation {
    /// Speed linearly in height, and direction linearly along the shorter arc between the two
    /// levels (clockwise when they are exactly opposite). A wind that veers keeps its speed.
    #[default]
    SpeedDirection,
    /// The East and North components linearly in height, as RocketPy does. Speed dips between
    /// levels whose directions differ.
    Components,
}

/// One level of a [`LayeredWind`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindLevel {
    /// Geometric height above mean sea level, m.
    pub height_msl_m: f64,
    /// Wind speed, m/s.
    pub speed_m_s: f64,
    /// Direction the wind blows from, clockwise from true north, rad.
    pub direction_from_rad: f64,
}

/// A wind tabulated at levels, such as a sounding or a forecast, interpolated between them.
///
/// Below the lowest level and above the highest, it holds the end level's wind and flags the
/// sample. Put the surface observation (for example the 10 m wind) in the table as its lowest
/// level, so the profile blends from the surface up instead of stepping at the first level
/// aloft (Loft lesson L6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "LayeredWindData", into = "LayeredWindData")]
pub struct LayeredWind {
    levels: Vec<WindLevel>,
    interpolation: WindInterpolation,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayeredWindData {
    levels: Vec<WindLevel>,
    #[serde(default)]
    interpolation: WindInterpolation,
}

impl TryFrom<LayeredWindData> for LayeredWind {
    type Error = AtmosError;

    fn try_from(data: LayeredWindData) -> Result<Self, AtmosError> {
        LayeredWind::new(data.levels, data.interpolation)
    }
}

impl From<LayeredWind> for LayeredWindData {
    fn from(wind: LayeredWind) -> Self {
        LayeredWindData {
            levels: wind.levels,
            interpolation: wind.interpolation,
        }
    }
}

impl LayeredWind {
    /// A tabulated wind from `levels`, with directions wrapped into `[0, 2π)`.
    ///
    /// # Errors
    ///
    /// - [`AtmosError::TooFewLevels`] with no levels.
    /// - [`AtmosError::HeightsNotIncreasing`] unless heights strictly increase.
    /// - [`AtmosError::Domain`] for a negative speed or any value that is not finite.
    pub fn new(
        levels: Vec<WindLevel>,
        interpolation: WindInterpolation,
    ) -> Result<Self, AtmosError> {
        if levels.is_empty() {
            return Err(AtmosError::TooFewLevels { min: 1, got: 0 });
        }
        let mut checked = Vec::with_capacity(levels.len());
        for (index, level) in levels.into_iter().enumerate() {
            let height = finite("wind level height (m)", level.height_msl_m)?;
            if let Some(previous) = checked.last().map(|l: &WindLevel| l.height_msl_m)
                && height <= previous
            {
                return Err(AtmosError::HeightsNotIncreasing { index });
            }
            checked.push(WindLevel {
                height_msl_m: height,
                speed_m_s: check_speed(level.speed_m_s)?,
                direction_from_rad: check_direction(level.direction_from_rad)?,
            });
        }
        Ok(LayeredWind {
            levels: checked,
            interpolation,
        })
    }

    /// The levels, lowest first.
    pub fn levels(&self) -> &[WindLevel] {
        &self.levels
    }

    /// How the table interpolates.
    pub fn interpolation(&self) -> WindInterpolation {
        self.interpolation
    }
}

impl Wind for LayeredWind {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        let z = finite("height (m)", height_msl_m)?;
        let levels = &self.levels;
        // `new` guarantees at least one level.
        let (first, last) = match (levels.first(), levels.last()) {
            (Some(first), Some(last)) => (first, last),
            _ => return Err(AtmosError::TooFewLevels { min: 1, got: 0 }),
        };
        let hold = |level: &WindLevel, side| WindSample {
            velocity_enu_m_s: velocity_from_speed_direction(
                level.speed_m_s,
                level.direction_from_rad,
            ),
            extrapolated: side,
        };
        if z < first.height_msl_m {
            return Ok(hold(first, Some(Side::Below)));
        }
        if z > last.height_msl_m {
            return Ok(hold(last, Some(Side::Above)));
        }
        // First level strictly above z; z ≥ first height, so upper ≥ 1.
        let upper = levels.partition_point(|l| l.height_msl_m <= z);
        if upper >= levels.len() {
            return Ok(hold(last, None));
        }
        let (a, b) = (&levels[upper - 1], &levels[upper]);
        let t = (z - a.height_msl_m) / (b.height_msl_m - a.height_msl_m);
        let velocity = match self.interpolation {
            WindInterpolation::SpeedDirection => {
                let speed = a.speed_m_s + t * (b.speed_m_s - a.speed_m_s);
                let mut turn = b.direction_from_rad - a.direction_from_rad;
                // Shorter arc, in (−π, π]: exactly opposite winds turn clockwise.
                if turn > PI {
                    turn -= TAU;
                } else if turn <= -PI {
                    turn += TAU;
                }
                velocity_from_speed_direction(speed, a.direction_from_rad + t * turn)
            }
            WindInterpolation::Components => {
                let va = velocity_from_speed_direction(a.speed_m_s, a.direction_from_rad);
                let vb = velocity_from_speed_direction(b.speed_m_s, b.direction_from_rad);
                va + t * (vb - va)
            }
        };
        Ok(WindSample {
            velocity_enu_m_s: velocity,
            extrapolated: None,
        })
    }
}

/// Any of the mean wind models, tagged by `model` when serialized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "snake_case")]
#[non_exhaustive]
pub enum WindModel {
    /// [`ConstantWind`].
    Constant(ConstantWind),
    /// [`PowerLawWind`].
    PowerLaw(PowerLawWind),
    /// [`LogLawWind`].
    LogLaw(LogLawWind),
    /// [`LayeredWind`].
    Layered(LayeredWind),
}

impl Wind for WindModel {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        match self {
            WindModel::Constant(w) => w.wind(height_msl_m),
            WindModel::PowerLaw(w) => w.wind(height_msl_m),
            WindModel::LogLaw(w) => w.wind(height_msl_m),
            WindModel::Layered(w) => w.wind(height_msl_m),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(degrees: f64) -> f64 {
        degrees.to_radians()
    }

    fn assert_close(actual: DVec3, expected: DVec3, tolerance: f64) {
        assert!(
            (actual - expected).length() <= tolerance,
            "{actual:?} vs {expected:?}"
        );
    }

    fn speed_and_direction_from(velocity: DVec3) -> (f64, f64) {
        let speed = velocity.x.hypot(velocity.y);
        (speed, wrap_direction((-velocity.x).atan2(-velocity.y)))
    }

    /// Loft lesson L6: a forecast profile blends from the surface wind up to the first level aloft
    /// instead of stepping there, and it interpolates speed and heading, not just one vector.
    #[test]
    fn layered_wind_interpolates_speed_and_heading() {
        // A 1000 m site: the 10 m wind is 4 m/s from 350°, and 12 m/s from 30° at 2010 m.
        let wind = LayeredWind::new(
            vec![
                WindLevel {
                    height_msl_m: 1010.0,
                    speed_m_s: 4.0,
                    direction_from_rad: deg(350.0),
                },
                WindLevel {
                    height_msl_m: 2010.0,
                    speed_m_s: 12.0,
                    direction_from_rad: deg(30.0),
                },
            ],
            WindInterpolation::SpeedDirection,
        )
        .unwrap();

        // Halfway: 8 m/s from 10°, turning through north along the shorter arc.
        let mid = wind.wind(1510.0).unwrap();
        assert_eq!(mid.extrapolated, None);
        let (speed, direction) = speed_and_direction_from(mid.velocity_enu_m_s);
        assert!((speed - 8.0).abs() < 1e-12);
        assert!((direction - deg(10.0)).abs() < 1e-12);

        // A quarter of the way: 6 m/s from 0°.
        let (speed, direction) =
            speed_and_direction_from(wind.wind(1260.0).unwrap().velocity_enu_m_s);
        assert!((speed - 6.0).abs() < 1e-12);
        assert!(direction.min(TAU - direction) < 1e-12);

        // No step: just above the surface level the wind is still the surface wind.
        let near_surface = wind.wind(1010.001).unwrap().velocity_enu_m_s;
        let surface = velocity_from_speed_direction(4.0, deg(350.0));
        assert_close(near_surface, surface, 1e-4);
        assert_close(wind.wind(1010.0).unwrap().velocity_enu_m_s, surface, 1e-15);
        assert_close(
            wind.wind(2010.0).unwrap().velocity_enu_m_s,
            velocity_from_speed_direction(12.0, deg(30.0)),
            1e-14,
        );
    }

    #[test]
    fn components_interpolation_averages_the_vectors() {
        let levels = vec![
            WindLevel {
                height_msl_m: 0.0,
                speed_m_s: 10.0,
                direction_from_rad: deg(270.0),
            },
            WindLevel {
                height_msl_m: 100.0,
                speed_m_s: 10.0,
                direction_from_rad: deg(0.0),
            },
        ];
        let components = LayeredWind::new(levels.clone(), WindInterpolation::Components).unwrap();
        // From the west (+E) and from the north (−N): the average is (5, −5), speed 7.07.
        assert_close(
            components.wind(50.0).unwrap().velocity_enu_m_s,
            DVec3::new(5.0, -5.0, 0.0),
            1e-14,
        );
        // The polar form keeps 10 m/s, from 315°.
        let polar = LayeredWind::new(levels, WindInterpolation::SpeedDirection).unwrap();
        let (speed, direction) =
            speed_and_direction_from(polar.wind(50.0).unwrap().velocity_enu_m_s);
        assert!((speed - 10.0).abs() < 1e-12);
        assert!((direction - deg(315.0)).abs() < 1e-12);
    }

    #[test]
    fn opposite_directions_turn_clockwise() {
        let wind = LayeredWind::new(
            vec![
                WindLevel {
                    height_msl_m: 0.0,
                    speed_m_s: 5.0,
                    direction_from_rad: deg(90.0),
                },
                WindLevel {
                    height_msl_m: 10.0,
                    speed_m_s: 5.0,
                    direction_from_rad: deg(270.0),
                },
            ],
            WindInterpolation::SpeedDirection,
        )
        .unwrap();
        let (_, direction) = speed_and_direction_from(wind.wind(5.0).unwrap().velocity_enu_m_s);
        assert!((direction - deg(180.0)).abs() < 1e-12);
    }

    #[test]
    fn layered_wind_holds_and_flags_beyond_its_levels() {
        let wind = LayeredWind::new(
            vec![
                WindLevel {
                    height_msl_m: 100.0,
                    speed_m_s: 3.0,
                    direction_from_rad: deg(180.0),
                },
                WindLevel {
                    height_msl_m: 900.0,
                    speed_m_s: 9.0,
                    direction_from_rad: deg(200.0),
                },
            ],
            WindInterpolation::default(),
        )
        .unwrap();
        let below = wind.wind(0.0).unwrap();
        assert_eq!(below.extrapolated, Some(Side::Below));
        assert_close(below.velocity_enu_m_s, DVec3::new(0.0, 3.0, 0.0), 1e-14);
        let above = wind.wind(5000.0).unwrap();
        assert_eq!(above.extrapolated, Some(Side::Above));
        assert_close(
            above.velocity_enu_m_s,
            velocity_from_speed_direction(9.0, deg(200.0)),
            1e-15,
        );
        // A single level is a constant wind with flags.
        let single = LayeredWind::new(
            vec![WindLevel {
                height_msl_m: 10.0,
                speed_m_s: 2.0,
                direction_from_rad: 0.0,
            }],
            WindInterpolation::default(),
        )
        .unwrap();
        assert_eq!(single.wind(10.0).unwrap().extrapolated, None);
        assert_eq!(single.wind(11.0).unwrap().extrapolated, Some(Side::Above));
    }

    #[test]
    fn meteorological_direction_convention() {
        // From the west blows toward the east; from the north blows toward the south.
        let west = ConstantWind::new(10.0, deg(270.0)).unwrap();
        assert_close(
            west.wind(0.0).unwrap().velocity_enu_m_s,
            DVec3::new(10.0, 0.0, 0.0),
            1e-14,
        );
        let north = ConstantWind::new(10.0, 0.0).unwrap();
        assert_close(
            north.wind(123.0).unwrap().velocity_enu_m_s,
            DVec3::new(0.0, -10.0, 0.0),
            1e-14,
        );
        assert_eq!(
            ConstantWind::calm().wind(0.0).unwrap().velocity_enu_m_s,
            DVec3::ZERO
        );
        // Directions wrap.
        let wrapped = ConstantWind::new(1.0, deg(-90.0)).unwrap();
        assert!((wrapped.direction_from_rad - deg(270.0)).abs() < 1e-12);
    }

    #[test]
    fn power_law_passes_through_its_reference() {
        let wind = PowerLawWind::new(5.0, 10.0, 1.0 / 7.0, deg(270.0), 1400.0).unwrap();
        let at_reference = wind.wind(1410.0).unwrap();
        assert_close(
            at_reference.velocity_enu_m_s,
            DVec3::new(5.0, 0.0, 0.0),
            1e-14,
        );
        let at_80 = wind.wind(1480.0).unwrap().velocity_enu_m_s.x;
        assert!((at_80 - 5.0 * 8.0_f64.powf(1.0 / 7.0)).abs() < 1e-13);
        assert_eq!(wind.wind(1400.0).unwrap().velocity_enu_m_s, DVec3::ZERO);
        let below = wind.wind(1390.0).unwrap();
        assert_eq!(below.velocity_enu_m_s, DVec3::ZERO);
        assert_eq!(below.extrapolated, Some(Side::Below));
    }

    #[test]
    fn log_law_passes_through_its_reference_and_vanishes_at_the_roughness_length() {
        let wind = LogLawWind::new(6.0, 10.0, 0.03, deg(180.0), 0.0).unwrap();
        assert_close(
            wind.wind(10.0).unwrap().velocity_enu_m_s,
            DVec3::new(0.0, 6.0, 0.0),
            1e-14,
        );
        let at_100 = wind.wind(100.0).unwrap().velocity_enu_m_s.y;
        let expected = 6.0 * (100.0_f64 / 0.03).ln() / (10.0_f64 / 0.03).ln();
        assert!((at_100 - expected).abs() < 1e-13);
        assert_eq!(wind.wind(0.03).unwrap().velocity_enu_m_s, DVec3::ZERO);
        assert_eq!(wind.wind(-1.0).unwrap().extrapolated, Some(Side::Below));
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        assert!(ConstantWind::new(-1.0, 0.0).is_err());
        assert!(ConstantWind::new(1.0, f64::NAN).is_err());
        assert!(PowerLawWind::new(1.0, 0.0, 0.14, 0.0, 0.0).is_err());
        assert!(PowerLawWind::new(1.0, 10.0, -0.1, 0.0, 0.0).is_err());
        assert!(LogLawWind::new(1.0, 0.01, 0.03, 0.0, 0.0).is_err());
        assert!(LogLawWind::new(1.0, 10.0, 0.0, 0.0, 0.0).is_err());
        assert!(matches!(
            LayeredWind::new(vec![], WindInterpolation::default()),
            Err(AtmosError::TooFewLevels { .. })
        ));
        let level = |h| WindLevel {
            height_msl_m: h,
            speed_m_s: 1.0,
            direction_from_rad: 0.0,
        };
        assert!(matches!(
            LayeredWind::new(vec![level(0.0), level(0.0)], WindInterpolation::default()),
            Err(AtmosError::HeightsNotIncreasing { index: 1 })
        ));
        assert!(ConstantWind::calm().wind(f64::INFINITY).is_err());
    }

    #[test]
    fn wind_models_round_trip_through_json() {
        let models = vec![
            WindModel::Constant(ConstantWind::new(3.0, 1.0).unwrap()),
            WindModel::PowerLaw(PowerLawWind::new(5.0, 10.0, 0.14, 2.0, 100.0).unwrap()),
            WindModel::LogLaw(LogLawWind::new(5.0, 10.0, 0.05, 3.0, 100.0).unwrap()),
            WindModel::Layered(
                LayeredWind::new(
                    vec![WindLevel {
                        height_msl_m: 5.0,
                        speed_m_s: 1.0,
                        direction_from_rad: 0.5,
                    }],
                    WindInterpolation::Components,
                )
                .unwrap(),
            ),
        ];
        for model in models {
            let json = serde_json::to_string(&model).unwrap();
            let back: WindModel = serde_json::from_str(&json).unwrap();
            assert_eq!(back, model, "{json}");
            assert_eq!(back.wind(50.0).unwrap(), model.wind(50.0).unwrap());
        }
        let negative = r#"{"model":"constant","speed_m_s":-3.0,"direction_from_rad":0.0}"#;
        assert!(serde_json::from_str::<WindModel>(negative).is_err());
        let unknown = r#"{"model":"constant","speed_m_s":3.0,"direction_from_rad":0.0,"x":1}"#;
        assert!(serde_json::from_str::<WindModel>(unknown).is_err());
    }
}
