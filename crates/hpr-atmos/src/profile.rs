//! Custom atmospheres from soundings or forecasts: temperature, pressure, humidity and wind at
//! levels, interpolated in height, with the offset standard atmosphere beyond them.
//!
//! **Between two levels** `i` and `i + 1`, interpolation runs in geopotential height `H` (the
//! 1976 standard's, [`geopotential_from_geometric_m`]), at fraction
//! `t = (H − H_i)/(H_{i+1} − H_i)`:
//!
//! ```text
//! T = T_i + t (T_{i+1} − T_i)
//! U = U_i + t (U_{i+1} − U_i)                                      relative humidity
//! ln P = ln P_i + ln(P_{i+1}/P_i) · ln(T/T_i) / ln(T_{i+1}/T_i)
//! ```
//!
//! The pressure form is exact for a dry hydrostatic layer whose temperature is linear in
//! geopotential height, as every layer of the standard atmosphere is, and it passes through both
//! levels' pressures whatever the data. When the temperatures are equal it is log-linear.
//!
//! **Missing pressures** above the lowest level are filled in hydrostatically from the level
//! below, with virtual temperature linear in geopotential height (the hypsometric equation, as in
//! WMO-No. 8 (2023), Vol. I, eqs. 12.17 and 12.18):
//!
//! ```text
//! ln(P_{i+1}/P_i) = −(g₀/R_d) (H_{i+1} − H_i) · ln(T_v,i+1/T_v,i) / (T_v,i+1 − T_v,i)
//! ```
//!
//! **Beyond the levels** the profile continues as the 1976 standard atmosphere, offset to pass
//! through the end level's temperature and pressure ([`Ussa76::anchored`]), and flags the sample.
//! Below the lowest level the lowest level's relative humidity is held. Above the highest, its
//! vapour mole fraction is held, capped at saturation, so a humid top does not put water into the
//! cold stratosphere.
//!
//! Heights are geometric above mean sea level. Soundings and forecasts usually report WMO
//! geopotential height; convert it with [`geometric_from_wmo_geopotential_m`].

use hpr_core::interp::Side;
use serde::{Deserialize, Serialize};

use crate::air::{AirSample, Atmosphere};
use crate::error::{AtmosError, finite, positive};
use crate::moist::{check_relative_humidity, moist_air_unchecked, saturation_vapour_pressure_pa};
use crate::ussa76::{DRY_AIR_GAS_CONSTANT_J_PER_KG_K, Ussa76, geopotential_from_geometric_m};
use crate::wind::{LayeredWind, WindInterpolation, WindLevel};
use hpr_core::gravity::STANDARD_GRAVITY_MPS2;

/// Iterations of the hydrostatic fill with humidity: the virtual temperature depends on the
/// pressure being solved for only through `e/p`, a few percent at most, so each iteration gains
/// about two digits.
const FILL_ITERATIONS: usize = 8;

/// One level of a sounding or forecast profile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundingLevel {
    /// Geometric height above mean sea level, m.
    pub height_msl_m: f64,
    /// Temperature, K.
    pub temperature_k: f64,
    /// Pressure, Pa. Required on the lowest level; filled in hydrostatically where omitted above.
    #[serde(default)]
    pub pressure_pa: Option<f64>,
    /// Relative humidity with respect to liquid water, as a fraction in `[0, 1]`. Give it on every
    /// level or on none; without it the air is dry.
    #[serde(default)]
    pub relative_humidity: Option<f64>,
    /// Wind speed, m/s. Give it with a direction, on every level or on none.
    #[serde(default)]
    pub wind_speed_m_s: Option<f64>,
    /// Direction the wind blows from, clockwise from true north, rad.
    #[serde(default)]
    pub wind_direction_from_rad: Option<f64>,
}

/// An atmosphere built from a sounding or a forecast profile.
///
/// It serializes as its levels as given and its wind interpolation, and rebuilds (and
/// re-checks) everything else when deserialized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SoundingProfileData", into = "SoundingProfileData")]
pub struct SoundingProfile {
    levels: Vec<SoundingLevel>,
    wind_interpolation: WindInterpolation,
    heights_m: Vec<f64>,
    /// Geopotential height of each level, m′.
    geopotentials_m: Vec<f64>,
    temperatures_k: Vec<f64>,
    pressures_pa: Vec<f64>,
    /// Relative humidity per level; empty for dry air.
    humidities: Vec<f64>,
    below: Ussa76,
    above: Ussa76,
    /// Vapour mole fraction at the highest level.
    top_vapour_fraction: f64,
    wind: Option<LayeredWind>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SoundingProfileData {
    levels: Vec<SoundingLevel>,
    #[serde(default)]
    wind_interpolation: WindInterpolation,
}

impl TryFrom<SoundingProfileData> for SoundingProfile {
    type Error = AtmosError;

    fn try_from(data: SoundingProfileData) -> Result<Self, AtmosError> {
        SoundingProfile::new(data.levels, data.wind_interpolation)
    }
}

impl From<SoundingProfile> for SoundingProfileData {
    fn from(profile: SoundingProfile) -> Self {
        SoundingProfileData {
            levels: profile.levels,
            wind_interpolation: profile.wind_interpolation,
        }
    }
}

/// Vapour pressure (Pa) for relative humidity `u` (none for dry air) at temperature `t`.
fn vapour(humidity: Option<f64>, t: f64) -> f64 {
    humidity.map_or(0.0, |u| u * saturation_vapour_pressure_pa(t))
}

/// Virtual temperature `T / (1 − x_v (1 − M_v/M₀))` from the moist-air state's density.
fn virtual_temperature(t: f64, p: f64, e: f64) -> f64 {
    let air = moist_air_unchecked(t, p, e);
    p / (DRY_AIR_GAS_CONSTANT_J_PER_KG_K * air.density_kg_m3)
}

/// `ln(b/a)/(b − a)`, the reciprocal log-mean of two positive numbers, without cancellation when
/// they are close.
fn inverse_log_mean(a: f64, b: f64) -> f64 {
    let r = (b - a) / a;
    if r == 0.0 {
        1.0 / a
    } else {
        r.ln_1p() / (r * a)
    }
}

impl SoundingProfile {
    /// A profile from `levels`, lowest first, whose wind (if the levels give one) interpolates as
    /// `wind_interpolation`.
    ///
    /// # Errors
    ///
    /// - [`AtmosError::TooFewLevels`] with no levels.
    /// - [`AtmosError::HeightsNotIncreasing`] unless heights strictly increase.
    /// - [`AtmosError::MissingBasePressure`] if the lowest level has no pressure.
    /// - [`AtmosError::IncompleteColumn`] if humidity or wind is given on some levels but not all,
    ///   or a wind speed comes without its direction.
    /// - [`AtmosError::Domain`] for a non-positive temperature or pressure, a relative humidity
    ///   outside `[0, 1]`, a negative wind speed, a non-finite value, or end levels whose offset
    ///   standard atmosphere is out of range.
    pub fn new(
        levels: Vec<SoundingLevel>,
        wind_interpolation: WindInterpolation,
    ) -> Result<Self, AtmosError> {
        let Some(first) = levels.first() else {
            return Err(AtmosError::TooFewLevels { min: 1, got: 0 });
        };
        let has_humidity = first.relative_humidity.is_some();
        let has_wind = first.wind_speed_m_s.is_some() || first.wind_direction_from_rad.is_some();
        if first.pressure_pa.is_none() {
            return Err(AtmosError::MissingBasePressure);
        }

        let n = levels.len();
        let mut heights_m = Vec::with_capacity(n);
        let mut geopotentials_m = Vec::with_capacity(n);
        let mut temperatures_k = Vec::with_capacity(n);
        let mut humidities = Vec::with_capacity(if has_humidity { n } else { 0 });
        let mut wind_levels = Vec::with_capacity(if has_wind { n } else { 0 });
        for (index, level) in levels.iter().enumerate() {
            let z = finite("sounding level height (m)", level.height_msl_m)?;
            if heights_m.last().is_some_and(|&previous| z <= previous) {
                return Err(AtmosError::HeightsNotIncreasing { index });
            }
            heights_m.push(z);
            geopotentials_m.push(geopotential_from_geometric_m(z)?);
            temperatures_k.push(positive("sounding temperature (K)", level.temperature_k)?);
            if let Some(p) = level.pressure_pa {
                positive("sounding pressure (Pa)", p)?;
            }
            match (has_humidity, level.relative_humidity) {
                (true, Some(u)) => humidities.push(check_relative_humidity(u)?),
                (false, None) => {}
                _ => {
                    return Err(AtmosError::IncompleteColumn {
                        index,
                        column: "relative humidity",
                    });
                }
            }
            match (
                has_wind,
                level.wind_speed_m_s,
                level.wind_direction_from_rad,
            ) {
                (true, Some(speed), Some(direction)) => wind_levels.push(WindLevel {
                    height_msl_m: z,
                    speed_m_s: speed,
                    direction_from_rad: direction,
                }),
                (false, None, None) => {}
                _ => {
                    return Err(AtmosError::IncompleteColumn {
                        index,
                        column: "wind speed and direction",
                    });
                }
            }
        }

        let humidity_at = |i: usize| humidities.get(i).copied();
        let mut pressures_pa: Vec<f64> = Vec::with_capacity(n);
        for (i, level) in levels.iter().enumerate() {
            let pressure = match (level.pressure_pa, pressures_pa.last()) {
                (Some(p), _) => p,
                (None, Some(&below)) => {
                    let (t0, t1) = (temperatures_k[i - 1], temperatures_k[i]);
                    let dh = geopotentials_m[i] - geopotentials_m[i - 1];
                    let tv0 = virtual_temperature(t0, below, vapour(humidity_at(i - 1), t0));
                    let k = STANDARD_GRAVITY_MPS2 / DRY_AIR_GAS_CONSTANT_J_PER_KG_K * dh;
                    let mut p = below * (-k * inverse_log_mean(t0, t1)).exp();
                    if has_humidity {
                        for _ in 0..FILL_ITERATIONS {
                            let tv1 = virtual_temperature(t1, p, vapour(humidity_at(i), t1));
                            p = below * (-k * inverse_log_mean(tv0, tv1)).exp();
                        }
                    }
                    positive("filled sounding pressure (Pa)", p)?
                }
                // The first level's pressure was checked above.
                (None, None) => return Err(AtmosError::MissingBasePressure),
            };
            pressures_pa.push(pressure);
        }

        let last = n - 1;
        let below = Ussa76::anchored(heights_m[0], temperatures_k[0], pressures_pa[0])?;
        let above = Ussa76::anchored(heights_m[last], temperatures_k[last], pressures_pa[last])?;
        let top_vapour_fraction =
            (vapour(humidity_at(last), temperatures_k[last]) / pressures_pa[last]).min(1.0);
        let wind = if has_wind {
            Some(LayeredWind::new(wind_levels, wind_interpolation)?)
        } else {
            None
        };
        Ok(SoundingProfile {
            levels,
            wind_interpolation,
            heights_m,
            geopotentials_m,
            temperatures_k,
            pressures_pa,
            humidities,
            below,
            above,
            top_vapour_fraction,
            wind,
        })
    }

    /// The levels as given.
    pub fn levels(&self) -> &[SoundingLevel] {
        &self.levels
    }

    /// The pressure at each level, Pa, with omitted ones filled in.
    pub fn pressures_pa(&self) -> &[f64] {
        &self.pressures_pa
    }

    /// The profile's wind, if its levels give one.
    pub fn wind(&self) -> Option<&LayeredWind> {
        self.wind.as_ref()
    }

    /// The air at geometric height `height_msl_m` (see [`Atmosphere::air`]).
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the height is not finite or is at or below `−r₀`.
    pub fn sample(&self, height_msl_m: f64) -> Result<AirSample, AtmosError> {
        let z = finite("height (m)", height_msl_m)?;
        let n = self.heights_m.len();
        let humidity = |i: usize| self.humidities.get(i).copied();
        if z < self.heights_m[0] {
            let air = self.below.sample(z)?.air;
            let e = vapour(humidity(0), air.temperature_k);
            return Ok(AirSample {
                air: moist_air_unchecked(air.temperature_k, air.pressure_pa, e),
                extrapolated: Some(Side::Below),
            });
        }
        if z > self.heights_m[n - 1] {
            let air = self.above.sample(z)?.air;
            let e = (self.top_vapour_fraction * air.pressure_pa)
                .min(saturation_vapour_pressure_pa(air.temperature_k));
            return Ok(AirSample {
                air: moist_air_unchecked(air.temperature_k, air.pressure_pa, e),
                extrapolated: Some(Side::Above),
            });
        }
        // First level strictly above z, in 1..=n.
        let upper = self.heights_m.partition_point(|&h| h <= z);
        if upper >= n {
            let t = self.temperatures_k[n - 1];
            let e = vapour(humidity(n - 1), t);
            return Ok(AirSample {
                air: moist_air_unchecked(t, self.pressures_pa[n - 1], e),
                extrapolated: None,
            });
        }
        let i = upper - 1;
        let h = geopotential_from_geometric_m(z)?;
        let (h0, h1) = (self.geopotentials_m[i], self.geopotentials_m[upper]);
        let fraction = ((h - h0) / (h1 - h0)).clamp(0.0, 1.0);
        let (t0, t1) = (self.temperatures_k[i], self.temperatures_k[upper]);
        let t = t0 + fraction * (t1 - t0);
        let (p0, p1) = (self.pressures_pa[i], self.pressures_pa[upper]);
        let r = (t1 - t0) / t0;
        let shape = if r == 0.0 {
            fraction
        } else {
            (fraction * r).ln_1p() / r.ln_1p()
        };
        let p = p0 * ((p1 / p0).ln() * shape).exp();
        let e = match (humidity(i), humidity(upper)) {
            (Some(u0), Some(u1)) => (u0 + fraction * (u1 - u0)) * saturation_vapour_pressure_pa(t),
            _ => 0.0,
        };
        Ok(AirSample {
            air: moist_air_unchecked(t, p, e),
            extrapolated: None,
        })
    }
}

impl Atmosphere for SoundingProfile {
    fn air(&self, height_msl_m: f64) -> Result<AirSample, AtmosError> {
        self.sample(height_msl_m)
    }
}

/// `γ₄₅ = 9.80665 m/s²`, the gravity that defines the geopotential metre.
const GAMMA_45_MPS2: f64 = STANDARD_GRAVITY_MPS2;

/// `γ_s(φ)` and `R(φ)` of WMO-No. 8 (2023), Vol. I, eq. 12.16.
fn wmo_gravity_and_radius(latitude_rad: f64) -> Result<(f64, f64), AtmosError> {
    let phi = finite("latitude (rad)", latitude_rad)?;
    if phi.abs() > std::f64::consts::FRAC_PI_2 {
        return Err(AtmosError::Domain {
            what: "latitude (rad)",
            value: phi,
        });
    }
    let s2 = phi.sin().powi(2);
    let gamma_s = 9.780_325 * (1.0 + 0.001_931_85 * s2) / (1.0 - 0.006_694_35 * s2).sqrt();
    let radius_m = 6_378_137.0 / (1.006_803 - 0.006_706 * s2);
    Ok((gamma_s, radius_m))
}

/// WMO geopotential height `Z` (gpm) of geometric height `height_msl_m` at geodetic latitude
/// `latitude_rad`, by WMO-No. 8 (2023), Vol. I, eqs. 12.15 and 12.16 (after Mahoney):
///
/// ```text
/// Z = (γ_s(φ)/γ₄₅) · R(φ) z / (R(φ) + z)
/// γ_s(φ) = 9.780325 (1 + 0.00193185 sin²φ) / (1 − 0.00669435 sin²φ)^½
/// R(φ) = 6378.137 km / (1.006803 − 0.006706 sin²φ)
/// ```
///
/// WMO puts 30 km at 29.7785 km of geopotential at the equator and 29.932 km at 80° N.
///
/// # Errors
///
/// [`AtmosError::Domain`] if a value is not finite, the latitude is beyond ±90°, or the height
/// is at or below `−R(φ)`.
pub fn wmo_geopotential_from_geometric_m(
    height_msl_m: f64,
    latitude_rad: f64,
) -> Result<f64, AtmosError> {
    let (gamma_s, radius) = wmo_gravity_and_radius(latitude_rad)?;
    let z = finite("geometric height (m)", height_msl_m)?;
    if z <= -radius {
        return Err(AtmosError::Domain {
            what: "geometric height (m)",
            value: z,
        });
    }
    Ok(gamma_s / GAMMA_45_MPS2 * radius * z / (radius + z))
}

/// Geometric height above mean sea level of WMO geopotential height `geopotential_height_m`
/// (gpm) at latitude `latitude_rad`: the inverse of [`wmo_geopotential_from_geometric_m`],
/// `z = R Z′ / (R − Z′)` with `Z′ = Z γ₄₅/γ_s(φ)`.
///
/// # Errors
///
/// [`AtmosError::Domain`] if a value is not finite, the latitude is beyond ±90°, or `Z′` is at
/// or above `R(φ)`.
pub fn geometric_from_wmo_geopotential_m(
    geopotential_height_m: f64,
    latitude_rad: f64,
) -> Result<f64, AtmosError> {
    let (gamma_s, radius) = wmo_gravity_and_radius(latitude_rad)?;
    let scaled =
        finite("geopotential height (gpm)", geopotential_height_m)? * GAMMA_45_MPS2 / gamma_s;
    if scaled >= radius {
        return Err(AtmosError::Domain {
            what: "geopotential height (gpm)",
            value: geopotential_height_m,
        });
    }
    Ok(radius * scaled / (radius - scaled))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moist::moist_air;
    use crate::ussa76::{EARTH_RADIUS_M, geometric_from_geopotential_m};
    use crate::wind::Wind;

    fn level(z: f64, t: f64, p: Option<f64>, u: Option<f64>) -> SoundingLevel {
        SoundingLevel {
            height_msl_m: z,
            temperature_k: t,
            pressure_pa: p,
            relative_humidity: u,
            wind_speed_m_s: None,
            wind_direction_from_rad: None,
        }
    }

    /// `P(z₁)` by integrating `dP/dz = −ρ g` from `(z₀, P₀)` with the profile's own interpolated
    /// temperature and humidity, by the second-order midpoint method on 20 000 steps. The density
    /// is evaluated at the running pressure, so the integral does not use the profile's
    /// interpolated pressure.
    fn integrate_pressure(profile: &SoundingProfile, z0: f64, p0: f64, z1: f64) -> f64 {
        let steps = 20_000;
        let dz = (z1 - z0) / f64::from(steps);
        let vapour_fraction = |z: f64| {
            // e/p from the profile's air: ρ R_d T / p = 1 − x_v (1 − M_v/M₀).
            let air = profile.sample(z).unwrap().air;
            let deficit = 1.0
                - air.density_kg_m3 * DRY_AIR_GAS_CONSTANT_J_PER_KG_K * air.temperature_k
                    / air.pressure_pa;
            let x = deficit
                / (1.0
                    - crate::moist::WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL
                        / crate::ussa76::SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL);
            (air.temperature_k, x * air.pressure_pa)
        };
        let weight = |z: f64, p: f64| {
            let (t, e) = vapour_fraction(z);
            let g = STANDARD_GRAVITY_MPS2 * (EARTH_RADIUS_M / (EARTH_RADIUS_M + z)).powi(2);
            moist_air_unchecked(t, p, e.max(0.0)).density_kg_m3 * g
        };
        let mut p = p0;
        for k in 0..steps {
            let z = z0 + f64::from(k) * dz;
            let half = p - 0.5 * dz * weight(z, p);
            p -= dz * weight(z + 0.5 * dz, half);
        }
        p
    }

    /// Loft lesson L5: a sounding's measured temperatures (here a hot, dry-adiabatic afternoon
    /// with an inversion aloft) replace the standard lapse from the field up, humidity lowers the
    /// density, and the omitted pressures aloft follow hydrostatically from the field pressure.
    #[test]
    fn sounding_temperature_overrides_standard_lapse() {
        let field = level(1400.0, 308.15, Some(85_500.0), Some(0.2));
        let profile = SoundingProfile::new(
            vec![
                field,
                level(2400.0, 300.15, None, Some(0.15)),
                level(3400.0, 301.15, None, Some(0.10)),
            ],
            WindInterpolation::default(),
        )
        .unwrap();
        let standard_lapse = Ussa76::anchored(1400.0, 308.15, 85_500.0).unwrap();

        // Temperature follows the sounding: 8 K/km down to 2400 m, then a 1 K inversion. 2900 m is
        // halfway in geometric height and 4e-5 past halfway in geopotential height.
        let mid = profile.sample(2900.0).unwrap();
        assert_eq!(mid.extrapolated, None);
        let h = |z| geopotential_from_geometric_m(z).unwrap();
        let fraction = (h(2900.0) - h(2400.0)) / (h(3400.0) - h(2400.0));
        assert!((fraction - 0.5 - 3.9e-5).abs() < 1e-6);
        assert!((mid.air.temperature_k - (300.15 + fraction)).abs() < 1e-12);
        let top = profile.sample(3400.0).unwrap().air;
        assert!((top.temperature_k - 301.15).abs() < 1e-12);
        let lapse_top = standard_lapse.sample(3400.0).unwrap().air;
        assert!((lapse_top.temperature_k - (308.15 - 6.5 * 2.0)).abs() < 0.05);
        assert!(top.temperature_k - lapse_top.temperature_k > 5.0);

        // The filled pressures are hydrostatic for the sounding's temperatures and humidity. The
        // fill takes virtual temperature linear in height, while the profile interpolates
        // temperature and relative humidity linearly; vapour pressure is exponential in
        // temperature, so the two differ by about 0.06 K mid-layer here, and the pressures by
        // 1.1e-5 (a tenth of a metre of height). Dry air agrees to 1e-10 (next test).
        for (z, p) in [
            (2400.0, profile.pressures_pa()[1]),
            (3400.0, profile.pressures_pa()[2]),
        ] {
            let integrated = integrate_pressure(&profile, 1400.0, 85_500.0, z);
            assert!(
                ((p - integrated) / integrated).abs() < 2e-5,
                "{z}: {p} vs {integrated}"
            );
        }

        // Humid air is lighter than dry air at the same temperature and pressure.
        let dry = moist_air(top.temperature_k, top.pressure_pa, 0.0).unwrap();
        let lighter = 1.0 - top.density_kg_m3 / dry.density_kg_m3;
        assert!((0.0015..0.0035).contains(&lighter), "{lighter}");
        // And the sounding's density at 3400 m differs from the standard-lapse guess by over 1%.
        assert!((top.density_kg_m3 / lapse_top.density_kg_m3 - 1.0).abs() > 0.01);
    }

    /// Levels taken from the standard atmosphere reproduce it between them, because both are
    /// linear in geopotential height: temperature to rounding, and pressure and density to 1e-12.
    #[test]
    fn standard_levels_reproduce_the_standard_between_them() {
        let standard = Ussa76::standard();
        let heights = [
            0.0,
            5_000.0,
            geometric_from_geopotential_m(11_000.0).unwrap(),
            15_000.0,
            geometric_from_geopotential_m(20_000.0).unwrap(),
            26_000.0,
        ];
        let levels = heights
            .iter()
            .map(|&z| {
                let air = standard.sample(z).unwrap().air;
                level(z, air.temperature_k, Some(air.pressure_pa), None)
            })
            .collect();
        let profile = SoundingProfile::new(levels, WindInterpolation::default()).unwrap();
        for z in heights {
            let ours = profile.sample(z).unwrap().air;
            let reference = standard.sample(z).unwrap().air;
            assert_eq!(ours.temperature_k, reference.temperature_k);
            assert_eq!(ours.pressure_pa, reference.pressure_pa);
            assert!((ours.density_kg_m3 / reference.density_kg_m3 - 1.0).abs() < 1e-15);
        }
        for z in [2_500.0, 8_000.0, 13_000.0, 17_500.0, 23_000.0] {
            let ours = profile.sample(z).unwrap().air;
            let reference = standard.sample(z).unwrap().air;
            assert!(
                (ours.temperature_k - reference.temperature_k).abs() < 1e-10,
                "{z}"
            );
            for (a, b) in [
                (ours.pressure_pa, reference.pressure_pa),
                (ours.density_kg_m3, reference.density_kg_m3),
            ] {
                assert!(((a - b) / b).abs() < 1e-12, "{z}: {a} vs {b}");
            }
        }
    }

    /// Dry, the fill and the interpolation make the same assumption, so they agree to the
    /// integration's own error.
    #[test]
    fn filled_pressures_are_hydrostatic_for_dry_air() {
        let profile = SoundingProfile::new(
            vec![
                level(1400.0, 308.15, Some(85_500.0), None),
                level(2400.0, 300.15, None, None),
                level(3400.0, 301.15, None, None),
            ],
            WindInterpolation::default(),
        )
        .unwrap();
        for (z, p) in [
            (2400.0, profile.pressures_pa()[1]),
            (3400.0, profile.pressures_pa()[2]),
        ] {
            let integrated = integrate_pressure(&profile, 1400.0, 85_500.0, z);
            assert!(
                ((p - integrated) / integrated).abs() < 1e-10,
                "{z}: {p} vs {integrated}"
            );
        }
    }

    /// A dry fill from sea level to the tropopause base gives the standard's base pressure.
    #[test]
    fn hydrostatic_fill_matches_the_standard() {
        let tropopause = geometric_from_geopotential_m(11_000.0).unwrap();
        let profile = SoundingProfile::new(
            vec![
                level(0.0, 288.15, Some(101_325.0), None),
                level(tropopause, 216.65, None, None),
            ],
            WindInterpolation::default(),
        )
        .unwrap();
        let p = profile.pressures_pa()[1];
        let reference = Ussa76::standard()
            .sample(tropopause)
            .unwrap()
            .air
            .pressure_pa;
        assert!(
            ((p - reference) / reference).abs() < 1e-12,
            "{p} vs {reference}"
        );
    }

    #[test]
    fn beyond_the_levels_follows_the_offset_standard() {
        let profile = SoundingProfile::new(
            vec![
                level(500.0, 295.0, Some(95_000.0), Some(0.6)),
                level(2_000.0, 288.0, Some(80_000.0), Some(1.0)),
            ],
            WindInterpolation::default(),
        )
        .unwrap();
        let top = profile.sample(2_000.0).unwrap().air;
        let just_above = profile.sample(2_000.001).unwrap();
        assert_eq!(just_above.extrapolated, Some(Side::Above));
        assert!((just_above.air.temperature_k - top.temperature_k).abs() < 1e-4);
        assert!((just_above.air.pressure_pa / top.pressure_pa - 1.0).abs() < 1e-6);
        assert!((just_above.air.density_kg_m3 / top.density_kg_m3 - 1.0).abs() < 1e-5);
        // Above, the standard lapse (6.5 K/km) continues from the top level.
        let aloft = profile.sample(6_000.0).unwrap().air;
        assert!((aloft.temperature_k - (288.0 - 6.5 * 4.0)).abs() < 0.05);
        // The saturated top's vapour is capped at saturation in the cold air aloft.
        let e_sat = saturation_vapour_pressure_pa(aloft.temperature_k);
        let capped = moist_air(aloft.temperature_k, aloft.pressure_pa, e_sat).unwrap();
        assert!((aloft.density_kg_m3 / capped.density_kg_m3 - 1.0).abs() < 1e-14);

        let below = profile.sample(0.0).unwrap();
        assert_eq!(below.extrapolated, Some(Side::Below));
        assert!(below.air.pressure_pa > 95_000.0 && below.air.temperature_k > 295.0);
        assert!(profile.sample(f64::NAN).is_err());
    }

    #[test]
    fn a_single_level_is_an_anchored_standard_atmosphere() {
        let profile = SoundingProfile::new(
            vec![level(1_000.0, 280.0, Some(90_000.0), None)],
            WindInterpolation::default(),
        )
        .unwrap();
        let anchored = Ussa76::anchored(1_000.0, 280.0, 90_000.0).unwrap();
        assert_eq!(profile.sample(1_000.0).unwrap().extrapolated, None);
        for z in [0.0, 3_000.0] {
            let ours = profile.sample(z).unwrap().air;
            let reference = anchored.sample(z).unwrap().air;
            assert!((ours.density_kg_m3 / reference.density_kg_m3 - 1.0).abs() < 1e-14);
        }
    }

    #[test]
    fn wind_columns_make_a_layered_wind() {
        let mut low = level(0.0, 290.0, Some(100_000.0), None);
        low.wind_speed_m_s = Some(3.0);
        low.wind_direction_from_rad = Some(1.0);
        let mut high = level(1_000.0, 283.0, None, None);
        high.wind_speed_m_s = Some(9.0);
        high.wind_direction_from_rad = Some(2.0);
        let profile = SoundingProfile::new(vec![low, high], WindInterpolation::Components).unwrap();
        let wind = profile.wind().unwrap();
        assert_eq!(wind.interpolation(), WindInterpolation::Components);
        assert_eq!(wind.levels().len(), 2);
        assert!(wind.wind(500.0).is_ok());
        let dry = SoundingProfile::new(
            vec![level(0.0, 290.0, Some(100_000.0), None)],
            WindInterpolation::default(),
        )
        .unwrap();
        assert!(dry.wind().is_none());
    }

    #[test]
    fn invalid_profiles_are_rejected() {
        let ok = level(0.0, 290.0, Some(100_000.0), Some(0.5));
        let build = |levels| SoundingProfile::new(levels, WindInterpolation::default());
        assert!(matches!(
            build(vec![]),
            Err(AtmosError::TooFewLevels { .. })
        ));
        assert!(matches!(
            build(vec![level(0.0, 290.0, None, None)]),
            Err(AtmosError::MissingBasePressure)
        ));
        assert!(matches!(
            build(vec![ok, level(0.0, 280.0, None, Some(0.5))]),
            Err(AtmosError::HeightsNotIncreasing { index: 1 })
        ));
        assert!(matches!(
            build(vec![ok, level(100.0, 280.0, None, None)]),
            Err(AtmosError::IncompleteColumn { index: 1, .. })
        ));
        let mut windless_direction = level(100.0, 280.0, None, Some(0.5));
        windless_direction.wind_direction_from_rad = Some(0.0);
        assert!(matches!(
            build(vec![ok, windless_direction]),
            Err(AtmosError::IncompleteColumn { index: 1, .. })
        ));
        assert!(build(vec![level(0.0, 290.0, Some(100_000.0), Some(1.5))]).is_err());
        assert!(build(vec![level(0.0, -1.0, Some(100_000.0), None)]).is_err());
        assert!(build(vec![level(0.0, 290.0, Some(0.0), None)]).is_err());
        assert!(build(vec![level(f64::NAN, 290.0, Some(1e5), None)]).is_err());
    }

    #[test]
    fn profiles_round_trip_through_json() {
        let json = r#"{"levels":[
            {"height_msl_m":1400.0,"temperature_k":300.0,"pressure_pa":86000.0,
             "relative_humidity":0.3,"wind_speed_m_s":4.0,"wind_direction_from_rad":3.0},
            {"height_msl_m":3000.0,"temperature_k":290.0,
             "relative_humidity":0.2,"wind_speed_m_s":10.0,"wind_direction_from_rad":3.5}
        ]}"#;
        let profile: SoundingProfile = serde_json::from_str(json).unwrap();
        assert_eq!(
            profile.wind_interpolation,
            WindInterpolation::SpeedDirection
        );
        let back: SoundingProfile =
            serde_json::from_str(&serde_json::to_string(&profile).unwrap()).unwrap();
        assert_eq!(back, profile);
        let unknown = r#"{"levels":[{"height_msl_m":0.0,"temperature_k":300.0,"pressure_pa":1e5,"dew_point_k":280.0}]}"#;
        assert!(serde_json::from_str::<SoundingProfile>(unknown).is_err());
    }

    /// WMO-No. 8 (2023), Vol. I, p. 416: 30 km geometric is 29.7785 km of geopotential at the
    /// equator and 29.932 km at 80° N.
    #[test]
    fn wmo_geopotential_matches_the_guide() {
        let equator = wmo_geopotential_from_geometric_m(30_000.0, 0.0).unwrap();
        assert!((equator - 29_778.5).abs() <= 0.05, "{equator}");
        let north = wmo_geopotential_from_geometric_m(30_000.0, 80_f64.to_radians()).unwrap();
        assert!((north - 29_932.0).abs() <= 0.5, "{north}");
        for latitude in [-90.0_f64, -33.0, 0.0, 32.9, 45.0, 89.0] {
            for z in [-400.0, 0.0, 1_500.0, 12_000.0, 40_000.0] {
                let gpm = wmo_geopotential_from_geometric_m(z, latitude.to_radians()).unwrap();
                let back = geometric_from_wmo_geopotential_m(gpm, latitude.to_radians()).unwrap();
                assert!((back - z).abs() < 1e-8, "{latitude} {z}");
            }
        }
        assert!(wmo_geopotential_from_geometric_m(0.0, 2.0).is_err());
        assert!(geometric_from_wmo_geopotential_m(1e8, 0.0).is_err());
    }
}
