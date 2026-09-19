//! The U.S. Standard Atmosphere, 1976, from −5 km to 86 km geometric altitude, with an optional
//! temperature offset and sea-level pressure.
//!
//! Source: *U.S. Standard Atmosphere, 1976*, NOAA-S/T 76-1562 (NOAA, NASA and USAF, Washington,
//! 1976), part 1, pinned as `us-std-atmosphere-1976`. Equation, table and page numbers below are
//! that document's. `docs/physics/atmosphere.md` has the details and the tests that pin them.
//!
//! **Model.** Below 86 km the atmosphere is a sequence of layers in **geopotential** altitude `H`,
//! each with a constant gradient `L_M,b` of the molecular-scale temperature `T_M` (Table 4):
//!
//! ```text
//! H   = r₀ Z / (r₀ + Z)                                            (18)
//! T_M = T_M,b + L_M,b (H − H_b)                                    (23)
//! P   = P_b [T_M,b / T_M]^(g₀′ M₀ / (R* L_M,b))        L_M,b ≠ 0     (33a)
//! P   = P_b exp[−g₀′ M₀ (H − H_b) / (R* T_M,b)]        L_M,b = 0     (33b)
//! ρ   = P M₀ / (R* T_M)                                            (42)
//! T   = T_M M / M₀                                                 (22)
//! a   = (γ R* T_M / M₀)^½                                          (50)
//! μ   = β T^(3/2) / (T + S)                                        (51)
//! ```
//!
//! `Z` is geometric altitude, `T` the kinetic temperature, and `M/M₀` the molecular-weight ratio,
//! 1 below 80 km and tabulated from 80 to 86 km (Table 8). The printed tables leave `M/M₀` out
//! below 86 km and print `T = T_M` there (p. 9); this module follows the equations, so from 80 to
//! 85.5 km its kinetic temperature is up to 0.036% and its viscosity up to 0.031% below the printed
//! values.
//!
//! **Offsets.** [`Ussa76::with_offset`] adds a constant `ΔT` to `T_M` at every geopotential height
//! and integrates the hydrostatic equation from a chosen sea-level pressure, so pressure and
//! density stay consistent with the warmer or colder temperature. With `ΔT = 0` and
//! `P₀ = 101325 Pa` it is the standard itself. [`Ussa76::anchored`] picks `ΔT` and `P₀` to pass
//! through a measured temperature and pressure at one height, such as the launch site.
//!
//! **Outside −5 km to 86 km** the model extends the lowest layer downward and continues
//! isothermally above 86 km, and flags every such sample as extrapolated. The real standard is
//! also isothermal (186.87 K) from 86 to 91 km and warms above that, and its composition changes
//! above 86 km, so pressure and density there are rough.

use hpr_core::gravity::STANDARD_GRAVITY_MPS2;
use hpr_core::interp::Side;
use serde::{Deserialize, Serialize};

use crate::air::{AirSample, AirState, Atmosphere};
use crate::error::{AtmosError, finite, positive};

/// Universal gas constant `R*` as the 1976 standard adopts it, J/(kmol·K) (p. 3; Table 2 on p. 2
/// misprints the exponent's sign). It is not the current CODATA value (8.314462…e3); the
/// standard's tables are computed with this one.
pub const GAS_CONSTANT_J_PER_KMOL_K: f64 = 8.314_32e3;

/// Sea-level mean molecular weight of air `M₀`, kg/kmol (p. 9, eq. 21).
pub const SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL: f64 = 28.964_4;

/// Effective Earth radius `r₀` for geopotential altitude, m (pp. 4 and 8).
pub const EARTH_RADIUS_M: f64 = 6_356_766.0;

/// Sea-level pressure `P₀`, Pa (Table 2).
pub const SEA_LEVEL_PRESSURE_PA: f64 = 101_325.0;

/// Sea-level temperature `T₀`, K (Table 2).
pub const SEA_LEVEL_TEMPERATURE_K: f64 = 288.15;

/// Ratio of specific heats of air `γ` (Table 2).
pub const RATIO_OF_SPECIFIC_HEATS: f64 = 1.40;

/// Sutherland's constant `β` for the viscosity of air, kg/(s·m·K^½) (Table 2 and p. 19).
pub const SUTHERLAND_BETA: f64 = 1.458e-6;

/// Sutherland's constant `S` for the viscosity of air, K (p. 19). Table 2 and p. 4 print 110 K,
/// but the tables are computed with 110.4 K: sea-level viscosity is 1.7894e-5 Pa·s with it and
/// 1.7912e-5 with 110.
pub const SUTHERLAND_S_K: f64 = 110.4;

/// Specific gas constant of dry air, `R* / M₀`, J/(kg·K).
pub const DRY_AIR_GAS_CONSTANT_J_PER_KG_K: f64 =
    GAS_CONSTANT_J_PER_KMOL_K / SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL;

/// Lowest geometric altitude the standard defines, m.
pub const MIN_HEIGHT_M: f64 = -5_000.0;

/// Highest geometric altitude of the model, m. Above this the standard uses a different
/// formulation, which this crate does not implement.
pub const MAX_HEIGHT_M: f64 = 86_000.0;

/// `g₀′ M₀ / R*`, K per geopotential metre: the hydrostatic constant of eqs. 33a and 33b.
const HYDROSTATIC_K_PER_M: f64 =
    STANDARD_GRAVITY_MPS2 * SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL / GAS_CONSTANT_J_PER_KMOL_K;

/// Layer bases: geopotential altitude `H_b` (m′) and molecular-scale temperature gradient
/// `L_M,b` (K/m′), Table 4. The seventh base, `H_7 = 84 852 m′`, is the top of the model.
const LAYERS: [(f64, f64); 7] = [
    (0.0, -0.0065),
    (11_000.0, 0.0),
    (20_000.0, 0.001),
    (32_000.0, 0.0028),
    (47_000.0, 0.0),
    (51_000.0, -0.0028),
    (71_000.0, -0.002),
];

/// Molecular-weight ratio `M/M₀` at geometric altitudes 80.0, 80.5, …, 86.0 km (Table 8).
const MOLECULAR_WEIGHT_RATIO: [f64; 13] = [
    1.000_000, 0.999_996, 0.999_989, 0.999_971, 0.999_941, 0.999_909, 0.999_870, 0.999_829,
    0.999_786, 0.999_741, 0.999_694, 0.999_641, 0.999_579,
];

/// Geometric altitude where Table 8 starts, m.
const MOLECULAR_WEIGHT_TABLE_START_M: f64 = 80_000.0;

/// Spacing of Table 8, m.
const MOLECULAR_WEIGHT_TABLE_STEP_M: f64 = 500.0;

/// Geopotential altitude `H` (m′) from geometric altitude `Z` (m), eq. 18, with the standard's
/// constant `g₀` and effective radius `r₀`. Heights at or below `−r₀` have no geopotential.
///
/// # Errors
///
/// [`AtmosError::Domain`] if `geometric_m` is not finite or not above `−r₀`.
pub fn geopotential_from_geometric_m(geometric_m: f64) -> Result<f64, AtmosError> {
    let z = finite("geometric altitude (m)", geometric_m)?;
    if z <= -EARTH_RADIUS_M {
        return Err(AtmosError::Domain {
            what: "geometric altitude (m)",
            value: z,
        });
    }
    Ok(EARTH_RADIUS_M * z / (EARTH_RADIUS_M + z))
}

/// Geometric altitude `Z` (m) from geopotential altitude `H` (m′), the inverse of eq. 18:
/// `Z = r₀ H / (r₀ − H)`. `H` must be below `r₀`, the geopotential altitude of infinity.
///
/// # Errors
///
/// [`AtmosError::Domain`] if `geopotential_m` is not finite or not below `r₀`.
pub fn geometric_from_geopotential_m(geopotential_m: f64) -> Result<f64, AtmosError> {
    let h = finite("geopotential altitude (m')", geopotential_m)?;
    if h >= EARTH_RADIUS_M {
        return Err(AtmosError::Domain {
            what: "geopotential altitude (m')",
            value: h,
        });
    }
    Ok(EARTH_RADIUS_M * h / (EARTH_RADIUS_M - h))
}

/// Molecular-weight ratio `M/M₀` at geometric altitude `z` (m): 1 below 80 km, Table 8 linearly
/// interpolated from 80 to 86 km, and held at its 86 km value above.
fn molecular_weight_ratio(z: f64) -> f64 {
    if z <= MOLECULAR_WEIGHT_TABLE_START_M {
        return 1.0;
    }
    let position = (z - MOLECULAR_WEIGHT_TABLE_START_M) / MOLECULAR_WEIGHT_TABLE_STEP_M;
    let last = MOLECULAR_WEIGHT_RATIO.len() - 1;
    if position >= 12.0 {
        return MOLECULAR_WEIGHT_RATIO[last];
    }
    // 0 < position < 12, so the floor is an index from 0 to 11.
    // Cast: position is in (0, 12), so its floor fits a usize.
    let i = position.floor() as usize;
    let t = position - position.floor();
    MOLECULAR_WEIGHT_RATIO[i] + t * (MOLECULAR_WEIGHT_RATIO[i + 1] - MOLECULAR_WEIGHT_RATIO[i])
}

/// The index of the layer containing geopotential altitude `h`. Below the first base it is the
/// first layer; above the 86 km top the caller holds the top values instead.
fn standard_layer(h: f64) -> usize {
    LAYERS.iter().rposition(|&(base, _)| h >= base).unwrap_or(0)
}

/// The U.S. Standard Atmosphere, 1976, optionally offset in temperature and sea-level pressure.
///
/// It serializes as its two parameters, `temperature_offset_k` and `sea_level_pressure_pa`, and
/// re-checks them when deserialized.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Ussa76Data", into = "Ussa76Data")]
pub struct Ussa76 {
    temperature_offset_k: f64,
    sea_level_pressure_pa: f64,
    /// Molecular-scale temperature at each layer base, offset included, K.
    base_temperatures_k: [f64; 7],
    /// Pressure at each layer base, Pa.
    base_pressures_pa: [f64; 7],
    /// Geopotential altitude of `Z = 86 km`, m′.
    top_geopotential_m: f64,
    /// Molecular-scale temperature at the 86 km top, K.
    top_temperature_k: f64,
    /// Pressure at the 86 km top, Pa.
    top_pressure_pa: f64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ussa76Data {
    temperature_offset_k: f64,
    sea_level_pressure_pa: f64,
}

impl TryFrom<Ussa76Data> for Ussa76 {
    type Error = AtmosError;

    fn try_from(data: Ussa76Data) -> Result<Self, AtmosError> {
        Ussa76::with_offset(data.temperature_offset_k, data.sea_level_pressure_pa)
    }
}

impl From<Ussa76> for Ussa76Data {
    fn from(model: Ussa76) -> Self {
        Ussa76Data {
            temperature_offset_k: model.temperature_offset_k,
            sea_level_pressure_pa: model.sea_level_pressure_pa,
        }
    }
}

impl Default for Ussa76 {
    fn default() -> Self {
        Ussa76::standard()
    }
}

/// The lowest standard molecular-scale temperature anywhere in the model: the 86 km top,
/// `T_M,6 + L_M,6 (H(86 km) − H_6)`, a little above Table 4's 186.946 K at `H_7 = 84.852 km′`.
fn coldest_standard_temperature_k() -> f64 {
    let top = EARTH_RADIUS_M * MAX_HEIGHT_M / (EARTH_RADIUS_M + MAX_HEIGHT_M);
    let (base, lapse) = LAYERS[6];
    standard_base_temperatures_k()[6] + lapse * (top - base)
}

/// `T_M,b` of Table 4, from `T₀` and the gradients.
fn standard_base_temperatures_k() -> [f64; 7] {
    let mut temperatures = [SEA_LEVEL_TEMPERATURE_K; 7];
    for b in 1..7 {
        let (base, lapse) = LAYERS[b - 1];
        temperatures[b] = temperatures[b - 1] + lapse * (LAYERS[b].0 - base);
    }
    temperatures
}

/// Pressure at geopotential `h` within the layer that starts at `(base_h, base_t, base_p)` with
/// gradient `lapse`, eqs. 33a and 33b.
fn layer_pressure(h: f64, base_h: f64, lapse: f64, base_t: f64, base_p: f64) -> f64 {
    if lapse == 0.0 {
        base_p * (-HYDROSTATIC_K_PER_M * (h - base_h) / base_t).exp()
    } else {
        let t = base_t + lapse * (h - base_h);
        base_p * (base_t / t).powf(HYDROSTATIC_K_PER_M / lapse)
    }
}

impl Ussa76 {
    /// The standard itself: no offset and `P₀ = 101325 Pa`.
    pub fn standard() -> Self {
        Ussa76::build(0.0, SEA_LEVEL_PRESSURE_PA)
    }

    /// The standard with its molecular-scale temperature offset by `temperature_offset_k` at every
    /// geopotential height, and hydrostatic pressure from `sea_level_pressure_pa` at `H = 0`.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the offset is not finite or would make the temperature anywhere
    /// in the model zero or negative (it must exceed −186.9 K), or if the sea-level pressure is
    /// not finite and positive.
    pub fn with_offset(
        temperature_offset_k: f64,
        sea_level_pressure_pa: f64,
    ) -> Result<Self, AtmosError> {
        let offset = finite("temperature offset (K)", temperature_offset_k)?;
        if coldest_standard_temperature_k() + offset <= 0.0 {
            return Err(AtmosError::Domain {
                what: "temperature offset (K)",
                value: offset,
            });
        }
        let p0 = positive("sea-level pressure (Pa)", sea_level_pressure_pa)?;
        Ok(Ussa76::build(offset, p0))
    }

    /// The offset standard that passes through a measured kinetic `temperature_k` and
    /// `pressure_pa` at geometric `height_msl_m`: `ΔT` makes the temperature match there, and `P₀`
    /// scales the pressure profile to match. Launch-site conditions are the usual use.
    ///
    /// The offset holds all the way up, which a real hot or cold day does not: anchoring +20 K at
    /// a 1400 m field (at the standard's field pressure) makes the air 5.7% thinner at 3 km but
    /// 14% denser at 20 km and 30% denser at 30 km than the standard
    /// (`validation/oracles/atmosphere/conventions.py`). For flights far above the field, prefer
    /// a sounding.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the height is not finite or not above `−r₀`, the temperature or
    /// pressure is not finite and positive, or the implied offset or sea-level pressure is out of
    /// range (see [`Ussa76::with_offset`]).
    pub fn anchored(
        height_msl_m: f64,
        temperature_k: f64,
        pressure_pa: f64,
    ) -> Result<Self, AtmosError> {
        let h = geopotential_from_geometric_m(height_msl_m)?;
        let temperature = positive("anchor temperature (K)", temperature_k)?;
        let pressure = positive("anchor pressure (Pa)", pressure_pa)?;
        let standard = Ussa76::standard();
        let offset = temperature / molecular_weight_ratio(height_msl_m)
            - standard.molecular_scale_temperature_k(h);
        // Pressure is proportional to P₀ at a fixed offset, so scale a unit profile.
        let offset_unit = Ussa76::with_offset(offset, 1.0)?;
        let unit_pressure = offset_unit.pressure_pa(h);
        Ussa76::with_offset(offset, pressure / unit_pressure)
    }

    fn build(offset: f64, p0: f64) -> Self {
        let mut base_temperatures_k = standard_base_temperatures_k();
        for t in &mut base_temperatures_k {
            *t += offset;
        }
        let mut base_pressures_pa = [p0; 7];
        for b in 1..7 {
            let (base, lapse) = LAYERS[b - 1];
            base_pressures_pa[b] = layer_pressure(
                LAYERS[b].0,
                base,
                lapse,
                base_temperatures_k[b - 1],
                base_pressures_pa[b - 1],
            );
        }
        let top_geopotential_m = EARTH_RADIUS_M * MAX_HEIGHT_M / (EARTH_RADIUS_M + MAX_HEIGHT_M);
        let (base, lapse) = LAYERS[6];
        let top_temperature_k = base_temperatures_k[6] + lapse * (top_geopotential_m - base);
        let top_pressure_pa = layer_pressure(
            top_geopotential_m,
            base,
            lapse,
            base_temperatures_k[6],
            base_pressures_pa[6],
        );
        Ussa76 {
            temperature_offset_k: offset,
            sea_level_pressure_pa: p0,
            base_temperatures_k,
            base_pressures_pa,
            top_geopotential_m,
            top_temperature_k,
            top_pressure_pa,
        }
    }

    /// The temperature offset `ΔT`, K.
    pub fn temperature_offset_k(&self) -> f64 {
        self.temperature_offset_k
    }

    /// The pressure at `H = 0`, Pa.
    pub fn sea_level_pressure_pa(&self) -> f64 {
        self.sea_level_pressure_pa
    }

    /// Molecular-scale temperature `T_M` at geopotential altitude `h` (m′), offset included.
    fn molecular_scale_temperature_k(&self, h: f64) -> f64 {
        if h > self.top_geopotential_m {
            return self.top_temperature_k;
        }
        let b = standard_layer(h);
        let (base, lapse) = LAYERS[b];
        self.base_temperatures_k[b] + lapse * (h - base)
    }

    /// Pressure at geopotential altitude `h` (m′).
    fn pressure_pa(&self, h: f64) -> f64 {
        if h > self.top_geopotential_m {
            return layer_pressure(
                h,
                self.top_geopotential_m,
                0.0,
                self.top_temperature_k,
                self.top_pressure_pa,
            );
        }
        let b = standard_layer(h);
        let (base, lapse) = LAYERS[b];
        layer_pressure(
            h,
            base,
            lapse,
            self.base_temperatures_k[b],
            self.base_pressures_pa[b],
        )
    }

    /// The air at geometric height `height_msl_m` (see [`Atmosphere::air`]).
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the height is not finite or not above `−r₀`.
    pub fn sample(&self, height_msl_m: f64) -> Result<AirSample, AtmosError> {
        let h = geopotential_from_geometric_m(height_msl_m)?;
        let t_m = self.molecular_scale_temperature_k(h);
        let pressure = self.pressure_pa(h);
        let temperature = t_m * molecular_weight_ratio(height_msl_m);
        let air = AirState {
            temperature_k: temperature,
            pressure_pa: pressure,
            density_kg_m3: pressure / (DRY_AIR_GAS_CONSTANT_J_PER_KG_K * t_m),
            speed_of_sound_m_s: (RATIO_OF_SPECIFIC_HEATS * DRY_AIR_GAS_CONSTANT_J_PER_KG_K * t_m)
                .sqrt(),
            dynamic_viscosity_pa_s: sutherland_viscosity_pa_s(temperature),
        };
        let extrapolated = if height_msl_m < MIN_HEIGHT_M {
            Some(Side::Below)
        } else if height_msl_m > MAX_HEIGHT_M {
            Some(Side::Above)
        } else {
            None
        };
        Ok(AirSample { air, extrapolated })
    }
}

impl Atmosphere for Ussa76 {
    fn air(&self, height_msl_m: f64) -> Result<AirSample, AtmosError> {
        self.sample(height_msl_m)
    }
}

/// Dynamic viscosity of air by Sutherland's law with the 1976 standard's constants, eq. 51:
/// `μ = β T^(3/2) / (T + S)`, Pa·s, for kinetic temperature `T` in K.
pub fn sutherland_viscosity_pa_s(temperature_k: f64) -> f64 {
    SUTHERLAND_BETA * temperature_k * temperature_k.sqrt() / (temperature_k + SUTHERLAND_S_K)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct TableFixture {
        rows: Vec<Row>,
    }

    #[derive(Deserialize)]
    struct Row {
        geometric_altitude_m: f64,
        printed: Printed,
        si: Si,
    }

    #[derive(Deserialize)]
    struct Si {
        geopotential_altitude_m: f64,
        temperature_k: f64,
        molecular_scale_temperature_k: f64,
        pressure_pa: f64,
        density_kgpm3: f64,
        speed_of_sound_mps: Option<f64>,
        dynamic_viscosity_pas: Option<f64>,
        kinematic_viscosity_m2ps: Option<f64>,
    }

    #[derive(Deserialize)]
    struct Printed {
        geopotential_altitude_m: String,
        temperature_k: String,
        molecular_scale_temperature_k: String,
        pressure_mb: String,
        density_kgpm3: String,
        speed_of_sound_mps: Option<String>,
        dynamic_viscosity_pas: Option<String>,
        kinematic_viscosity_m2ps: Option<String>,
    }

    /// Table I and Table III of the 1976 standard at 32 geometric altitudes from −2 km to 86 km,
    /// transcribed from the scan and cross-checked by `validation/oracles/ussa76/tables.py`.
    fn table() -> TableFixture {
        serde_json::from_str(include_str!(
            "../../../validation/fixtures/atmosphere/ussa76-table-i.json"
        ))
        .unwrap()
    }

    /// A printed number and its last printed digit's place value, from text such as `2.2699E+2`
    /// (value 226.99, count 0.01) or `216.774` (count 0.001).
    fn parse_printed(text: &str) -> (f64, f64) {
        let value: f64 = text.parse().unwrap();
        let (mantissa, exponent) = match text.split_once('E') {
            Some((m, e)) => (m, e.parse::<i32>().unwrap()),
            None => (text, 0),
        };
        let decimals = mantissa.split_once('.').map_or(0, |(_, d)| d.len());
        let decimals = i32::try_from(decimals).unwrap();
        (value, 10f64.powi(exponent - decimals))
    }

    /// Number of last-digit counts between a computed value and the printed text.
    fn counts_off(computed: f64, printed: &str) -> f64 {
        let (value, count) = parse_printed(printed);
        (computed - value).abs() / count
    }

    /// Relative difference from the printed value; zero when both are zero (sea level's `H`).
    fn relative_error(computed: f64, printed: &str) -> f64 {
        let (value, _) = parse_printed(printed);
        if value == 0.0 {
            return computed.abs();
        }
        ((computed - value) / value).abs()
    }

    /// The M1.2 *done when*: the model matches the 1976 tables at 32 altitudes to 0.1%, for
    /// temperature, pressure, density, speed of sound, dynamic and kinematic viscosity.
    ///
    /// It holds a tighter line too: every printed value is within one count of its last digit,
    /// with two documented exceptions.
    /// - From 80 to 85.5 km the tables print `T = T_M` and compute viscosity from it (p. 9),
    ///   where this model applies `M/M₀`. There the printed temperature is checked against `T_M`,
    ///   the kinetic temperature against the print to 0.04%, and viscosity to 0.1% only.
    /// - The 84 km density prints `9.6940E-6` where the equations give `9.69387e-6`, 1.3
    ///   counts; the same row's `ρ/ρ₀` column gives `9.6939e-6`, so the print is off.
    #[test]
    fn matches_the_1976_tables_at_32_altitudes() {
        let model = Ussa76::standard();
        let rows = table().rows;
        assert_eq!(rows.len(), 32);
        let mut compared = 0;
        for row in &rows {
            let z = row.geometric_altitude_m;
            let p = &row.printed;
            let sample = model.sample(z).unwrap();
            assert_eq!(sample.extrapolated, None, "{z}");
            let air = sample.air;
            let h = geopotential_from_geometric_m(z).unwrap();
            let t_m = model.molecular_scale_temperature_k(h);
            let tables_omit_m_over_m0 = (80_000.0..86_000.0).contains(&z) && z > 80_000.0;
            let label = |what: &str| format!("{what} at {z} m");

            let mut check = |computed: f64, printed: &str, what: &str, one_count: bool| {
                assert!(relative_error(computed, printed) < 1e-3, "{}", label(what));
                if one_count {
                    let off = counts_off(computed, printed);
                    assert!(
                        off <= 1.0,
                        "{}: {computed} vs {printed}, {off} counts",
                        label(what)
                    );
                }
                compared += 1;
            };

            check(h, &p.geopotential_altitude_m, "geopotential altitude", true);
            check(t_m, &p.molecular_scale_temperature_k, "T_M", true);
            if tables_omit_m_over_m0 {
                check(t_m, &p.temperature_k, "printed T (= T_M)", true);
                assert!(relative_error(air.temperature_k, &p.temperature_k) < 4e-4);
            } else {
                check(air.temperature_k, &p.temperature_k, "T", true);
            }
            check(air.pressure_pa / 100.0, &p.pressure_mb, "P", true);
            let density_one_count = z != 84_000.0;
            check(air.density_kg_m3, &p.density_kgpm3, "ρ", density_one_count);
            if z == 84_000.0 {
                assert!(counts_off(air.density_kg_m3, &p.density_kgpm3) < 1.5);
            }
            if let Some(printed) = &p.speed_of_sound_mps {
                check(air.speed_of_sound_m_s, printed, "a", true);
            }
            if let Some(printed) = &p.dynamic_viscosity_pas {
                check(
                    air.dynamic_viscosity_pa_s,
                    printed,
                    "μ",
                    !tables_omit_m_over_m0,
                );
                if tables_omit_m_over_m0 {
                    let as_printed = sutherland_viscosity_pa_s(t_m);
                    check(as_printed, printed, "μ(T_M)", true);
                }
            }
            if let Some(printed) = &p.kinematic_viscosity_m2ps {
                let nu = air.kinematic_viscosity_m2_s();
                check(nu, printed, "ν", !tables_omit_m_over_m0);
            }
        }
        // 32 rows × 5 columns, plus speed of sound and both viscosities on the 31 rows below
        // 86 km, plus the `T_M`-based viscosity on the 3 rows at 82, 84 and 85 km.
        assert_eq!(compared, 32 * 5 + 31 * 3 + 3);
    }

    /// The fixture's SI column is the printed column converted (millibars × 100), so a test
    /// reading one reads the other. CI never runs the Python cross-check, so this runs here.
    #[test]
    fn fixture_si_values_are_the_printed_values() {
        for row in table().rows {
            let (p, si) = (&row.printed, &row.si);
            let parsed = |text: &str| parse_printed(text).0;
            let z = row.geometric_altitude_m;
            assert_eq!(
                si.geopotential_altitude_m,
                parsed(&p.geopotential_altitude_m),
                "{z}"
            );
            assert_eq!(si.temperature_k, parsed(&p.temperature_k), "{z}");
            assert_eq!(
                si.molecular_scale_temperature_k,
                parsed(&p.molecular_scale_temperature_k),
                "{z}"
            );
            let (mb, count) = parse_printed(&p.pressure_mb);
            assert!((si.pressure_pa - 100.0 * mb).abs() <= 1e-6 * count, "{z}");
            assert_eq!(si.density_kgpm3, parsed(&p.density_kgpm3), "{z}");
            for (si_value, printed) in [
                (si.speed_of_sound_mps, &p.speed_of_sound_mps),
                (si.dynamic_viscosity_pas, &p.dynamic_viscosity_pas),
                (si.kinematic_viscosity_m2ps, &p.kinematic_viscosity_m2ps),
            ] {
                assert_eq!(si_value, printed.as_deref().map(parsed), "{z}");
            }
        }
    }

    /// Loft lesson L2: 11 km *geometric* is 216.774 K and 22,699.96 Pa (Table I prints 2.2699E+2
    /// mb, so within one count), not the 216.65 K and 22,632 Pa of 11 km geopotential.
    #[test]
    fn geometric_11_km_matches_the_1976_tables() {
        let air = Ussa76::standard().sample(11_000.0).unwrap().air;
        assert!(
            (air.temperature_k - 216.774).abs() < 5e-4,
            "{}",
            air.temperature_k
        );
        assert!(
            (air.pressure_pa - 22_699.0).abs() <= 1.0,
            "{}",
            air.pressure_pa
        );
        assert!((air.pressure_pa - 22_699.96).abs() < 0.01);
        // And 11 km geopotential is the tropopause base.
        let base = Ussa76::standard()
            .sample(geometric_from_geopotential_m(11_000.0).unwrap())
            .unwrap()
            .air;
        assert!((base.temperature_k - 216.65).abs() < 1e-9);
        assert!(
            (base.pressure_pa - 22_632.06).abs() < 0.01,
            "{}",
            base.pressure_pa
        );
    }

    /// Loft lesson L3: all seven layers, so 50 km is in the isothermal stratopause at 270.65 K and
    /// 79.779 Pa, and the 32 km lapse does not run on (70 km is 219.585 K, not 335 K).
    #[test]
    fn fifty_km_is_270_65_k_and_79_779_pa() {
        let model = Ussa76::standard();
        let air = model.sample(50_000.0).unwrap().air;
        assert!((air.temperature_k - 270.65).abs() < 1e-9);
        assert!(
            (air.pressure_pa - 79.779).abs() < 5e-4,
            "{}",
            air.pressure_pa
        );
        let seventy = model.sample(70_000.0).unwrap().air;
        assert!(
            (seventy.temperature_k - 219.585).abs() < 5e-4,
            "{}",
            seventy.temperature_k
        );
    }

    /// Loft lesson L4: Sutherland's law with the standard's `β = 1.458e-6` and `S = 110.4 K` gives
    /// 1.7894e-5 Pa·s at sea level (Table III), where `S = 110 K` would give 1.7912e-5.
    #[test]
    fn sea_level_viscosity_is_1_7894e_5() {
        let mu = Ussa76::standard()
            .sample(0.0)
            .unwrap()
            .air
            .dynamic_viscosity_pa_s;
        assert!((mu - 1.7894e-5).abs() <= 0.5e-9, "{mu}");
        let with_110 = SUTHERLAND_BETA * 288.15_f64.powf(1.5) / (288.15 + 110.0);
        assert!((with_110 - 1.7912e-5).abs() < 1e-9);
    }

    #[derive(Deserialize)]
    struct ConstantsFixture {
        constants: Constants,
        layers: Layers,
        table_8: Table8,
    }

    #[derive(Deserialize)]
    struct Constants {
        gas_constant_jpkmolk: Value,
        sea_level_molecular_weight_kgpkmol: Value,
        g0_mps2: Value,
        earth_radius_m: Value,
        sea_level_pressure_pa: Value,
        sea_level_temperature_k: Value,
        sutherland_beta_kgpsmk12: Value,
        sutherland_constant_k: Value,
        ratio_of_specific_heats: Value,
        kinetic_temperature_86_km_k: Value,
    }

    #[derive(Deserialize)]
    struct Value {
        value: f64,
    }

    #[derive(Deserialize)]
    struct Layers {
        rows: Vec<LayerRow>,
    }

    #[derive(Deserialize)]
    struct LayerRow {
        base_geopotential_km: f64,
        gradient_kpkm: Option<f64>,
    }

    #[derive(Deserialize)]
    struct Table8 {
        by_geometric: Vec<Table8Row>,
    }

    #[derive(Deserialize)]
    struct Table8Row {
        geometric_altitude_m: f64,
        m_over_m0: f64,
    }

    /// The constants, Table 4 and Table 8 in the code are the transcribed ones.
    #[test]
    fn constants_match_the_transcription() {
        let fixture: ConstantsFixture = serde_json::from_str(include_str!(
            "../../../validation/fixtures/atmosphere/ussa76-constants.json"
        ))
        .unwrap();
        let c = fixture.constants;
        assert_eq!(GAS_CONSTANT_J_PER_KMOL_K, c.gas_constant_jpkmolk.value);
        assert_eq!(
            SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL,
            c.sea_level_molecular_weight_kgpkmol.value
        );
        assert_eq!(STANDARD_GRAVITY_MPS2, c.g0_mps2.value);
        assert_eq!(EARTH_RADIUS_M, c.earth_radius_m.value);
        assert_eq!(SEA_LEVEL_PRESSURE_PA, c.sea_level_pressure_pa.value);
        assert_eq!(SEA_LEVEL_TEMPERATURE_K, c.sea_level_temperature_k.value);
        assert_eq!(SUTHERLAND_BETA, c.sutherland_beta_kgpsmk12.value);
        assert_eq!(SUTHERLAND_S_K, c.sutherland_constant_k.value);
        assert_eq!(RATIO_OF_SPECIFIC_HEATS, c.ratio_of_specific_heats.value);

        let rows = fixture.layers.rows;
        assert_eq!(rows.len(), LAYERS.len() + 1);
        for (row, &(base, lapse)) in rows.iter().zip(&LAYERS) {
            assert!((row.base_geopotential_km * 1000.0 - base).abs() < 1e-9);
            assert!((row.gradient_kpkm.unwrap() / 1000.0 - lapse).abs() < 1e-15);
        }
        assert!((rows[7].base_geopotential_km - 84.852).abs() < 1e-12);

        let table_8 = fixture.table_8.by_geometric;
        assert_eq!(table_8.len(), MOLECULAR_WEIGHT_RATIO.len());
        for (i, row) in table_8.iter().enumerate() {
            let z = MOLECULAR_WEIGHT_TABLE_START_M + MOLECULAR_WEIGHT_TABLE_STEP_M * i as f64;
            assert_eq!(row.geometric_altitude_m, z);
            assert_eq!(row.m_over_m0, MOLECULAR_WEIGHT_RATIO[i]);
            assert_eq!(molecular_weight_ratio(z), row.m_over_m0);
        }

        // Eq. 25: the kinetic temperature at 86 km is 186.8673 K, to its last printed digit. (The
        // standard evaluates it at the rounded H₇ = 84.852 km′ with M₇/M₀ = 0.9995788; this model
        // uses the exact H(86 km) and Table 8's 0.999579, and lands 8e-5 K away.)
        let top = Ussa76::standard().sample(MAX_HEIGHT_M).unwrap().air;
        assert!((top.temperature_k - c.kinetic_temperature_86_km_k.value).abs() <= 1e-4);
    }

    /// Hydrostatic balance `dP/dZ = −ρ g(Z)` with `g = g₀ (r₀/(r₀ + Z))²` (eq. 17), by central
    /// differences, for the standard and offset atmospheres alike.
    fn assert_hydrostatic(model: &Ussa76, z: f64) {
        let dz = 0.5;
        let above = model.sample(z + dz).unwrap().air.pressure_pa;
        let below = model.sample(z - dz).unwrap().air.pressure_pa;
        let air = model.sample(z).unwrap().air;
        let g = STANDARD_GRAVITY_MPS2 * (EARTH_RADIUS_M / (EARTH_RADIUS_M + z)).powi(2);
        let gradient = (above - below) / (2.0 * dz);
        let expected = -air.density_kg_m3 * g;
        assert!(
            ((gradient - expected) / expected).abs() < 1e-6,
            "z = {z}: {gradient} vs {expected}"
        );
    }

    #[test]
    fn standard_is_hydrostatic_in_every_layer() {
        let model = Ussa76::standard();
        for z in [
            -4_000.0, 1_000.0, 15_000.0, 25_000.0, 40_000.0, 49_000.0, 60_000.0, 78_000.0,
        ] {
            assert_hydrostatic(&model, z);
        }
    }

    proptest! {
        #[test]
        fn offset_atmospheres_are_hydrostatic_and_offset(
            offset in -60.0..60.0_f64,
            p0 in 60_000.0..120_000.0_f64,
            z in -4_000.0..79_000.0_f64,
        ) {
            let model = Ussa76::with_offset(offset, p0).unwrap();
            // The central difference spans ±0.5 m; across a layer's base the lapse rate turns, and
            // a difference over the corner is not the derivative on either side (issue #74).
            let at_corner = LAYERS[1..].iter().any(|&(base, _)| {
                (z - geometric_from_geopotential_m(base).unwrap()).abs() < 1.0
            });
            if !at_corner {
                assert_hydrostatic(&model, z);
            }
            let standard = Ussa76::standard().sample(z).unwrap().air;
            let air = model.sample(z).unwrap().air;
            prop_assert!((air.temperature_k - standard.temperature_k - offset).abs() < 1e-9);
            prop_assert!(air.density_kg_m3 > 0.0 && air.speed_of_sound_m_s > 0.0);
        }

        #[test]
        fn anchored_atmosphere_passes_through_its_anchor(
            z in -500.0..5_000.0_f64,
            temperature in 230.0..330.0_f64,
            pressure in 50_000.0..108_000.0_f64,
        ) {
            let model = Ussa76::anchored(z, temperature, pressure).unwrap();
            let air = model.sample(z).unwrap().air;
            prop_assert!((air.temperature_k - temperature).abs() < 1e-9);
            prop_assert!(((air.pressure_pa - pressure) / pressure).abs() < 1e-12);
        }

        #[test]
        fn layers_join_continuously(offset in -60.0..60.0_f64) {
            let model = Ussa76::with_offset(offset, SEA_LEVEL_PRESSURE_PA).unwrap();
            for &(base, _) in &LAYERS[1..] {
                let z = geometric_from_geopotential_m(base).unwrap();
                let below = model.sample(z - 1e-6).unwrap().air;
                let above = model.sample(z + 1e-6).unwrap().air;
                prop_assert!((below.temperature_k - above.temperature_k).abs() < 1e-7);
                prop_assert!(((below.pressure_pa - above.pressure_pa) / above.pressure_pa).abs() < 1e-9);
            }
        }

        #[test]
        fn geopotential_round_trips(z in -100_000.0..1.0e7_f64) {
            let h = geopotential_from_geometric_m(z).unwrap();
            let back = geometric_from_geopotential_m(h).unwrap();
            prop_assert!((back - z).abs() <= 1e-9 * z.abs().max(1.0));
        }
    }

    #[test]
    fn standard_offset_is_the_standard() {
        assert_eq!(
            Ussa76::with_offset(0.0, SEA_LEVEL_PRESSURE_PA).unwrap(),
            Ussa76::standard()
        );
        assert_eq!(Ussa76::default(), Ussa76::standard());
        // A hot day: +20 K at a 1400 m field at 85 kPa.
        let hot = Ussa76::anchored(1400.0, 288.15 - 6.5 * 1.4 + 20.0, 85_000.0).unwrap();
        assert!((hot.temperature_offset_k() - 20.0).abs() < 0.01);
        assert!(hot.sea_level_pressure_pa() > 95_000.0 && hot.sea_level_pressure_pa() < 105_000.0);
    }

    #[test]
    fn extrapolation_is_flagged_and_finite() {
        let model = Ussa76::standard();
        assert_eq!(model.sample(MIN_HEIGHT_M).unwrap().extrapolated, None);
        assert_eq!(model.sample(MAX_HEIGHT_M).unwrap().extrapolated, None);
        let low = model.sample(-6_000.0).unwrap();
        assert_eq!(low.extrapolated, Some(Side::Below));
        assert!(low.air.temperature_k > 320.65 && low.air.pressure_pa > 1.9e5);
        let high = model.sample(120_000.0).unwrap();
        assert_eq!(high.extrapolated, Some(Side::Above));
        assert!(high.air.pressure_pa > 0.0 && high.air.pressure_pa < 0.02);
        assert!((high.air.temperature_k - 186.8673).abs() <= 1e-4);
        // Far away, still finite and non-negative.
        let far = model.sample(1.0e9).unwrap().air;
        assert!(far.pressure_pa >= 0.0 && far.density_kg_m3 >= 0.0);
        assert!(model.sample(f64::NAN).is_err());
        assert!(model.sample(-EARTH_RADIUS_M).is_err());
        assert!(geometric_from_geopotential_m(EARTH_RADIUS_M).is_err());
    }

    #[test]
    fn invalid_offsets_are_rejected_and_serde_round_trips() {
        assert!(Ussa76::with_offset(-190.0, SEA_LEVEL_PRESSURE_PA).is_err());
        assert!(Ussa76::with_offset(f64::NAN, SEA_LEVEL_PRESSURE_PA).is_err());
        assert!(Ussa76::with_offset(10.0, 0.0).is_err());
        assert!(Ussa76::anchored(0.0, -1.0, 100_000.0).is_err());
        assert!(Ussa76::anchored(0.0, 288.0, f64::INFINITY).is_err());
        let model = Ussa76::with_offset(12.5, 98_000.0).unwrap();
        let json = serde_json::to_string(&model).unwrap();
        assert_eq!(
            json,
            r#"{"temperature_offset_k":12.5,"sea_level_pressure_pa":98000.0}"#
        );
        assert_eq!(serde_json::from_str::<Ussa76>(&json).unwrap(), model);
        let bad = r#"{"temperature_offset_k":-500.0,"sea_level_pressure_pa":98000.0}"#;
        assert!(serde_json::from_str::<Ussa76>(bad).is_err());
    }
}
