//! Moist air: saturation vapour pressure, and the density and speed of sound of humid air.
//!
//! Air is treated as an ideal mixture of dry air (the 1976 standard's `M₀`, `γ = 1.4`) and water
//! vapour (see `docs/physics/atmosphere.md`):
//!
//! ```text
//! x_v = e / p                                         vapour mole fraction (Dalton)
//! M   = (1 − x_v) M₀ + x_v M_v                        mixture molar mass
//! ρ   = p M / (R* T)  =  p / (R_d T_v)                density
//! T_v = T / (1 − x_v (1 − M_v/M₀))                    virtual temperature (WMO-No. 8 eq. 12.18)
//! γ   = C_p / (C_p − R*),  C_p = (1 − x_v) (7/2) R* + x_v 4 R*
//! a   = (γ R* T / M)^½
//! ```
//!
//! Water vapour's molar heat capacity is taken as `4 R*`, the rigid nonlinear molecule
//! (33.26 J/(mol·K)); the ideal-gas value near 300 K is about 1% higher, which moves `a` by 0.009%
//! at 30 °C and saturation. Viscosity stays dry air's: by Wilke's mixing rule, water vapour
//! lowers it by 2.1% at 30 °C and saturation, which changes turbulent skin friction by about
//! 0.4%.
//!
//! Saturation vapour pressure over liquid water follows WMO-No. 8, *Guide to Instruments and
//! Methods of Observation*, Vol. I (2023), Annex 4.B, eq. 4.B.1, at every temperature: radiosonde
//! relative humidity is always reported relative to water, even below 0 °C.

use crate::air::AirState;
use crate::error::{AtmosError, finite, positive};
use crate::ussa76::{
    GAS_CONSTANT_J_PER_KMOL_K, SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL, sutherland_viscosity_pa_s,
};

/// Molar mass of water vapour `M_v`, kg/kmol (A. Picard et al., "Revised formula for the density
/// of moist air (CIPM-2007)", *Metrologia* 45, 149–155 (2008), section 2.1).
pub const WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL: f64 = 18.015_28;

/// 0 °C in kelvin.
const CELSIUS_ZERO_K: f64 = 273.15;

/// Saturation vapour pressure of pure water vapour over a plane surface of liquid water, Pa, at
/// temperature `temperature_k`. WMO-No. 8 (2023), Vol. I, Annex 4.B, eq. 4.B.1, with `t` in °C:
///
/// ```text
/// e_w(t) = 6.112 exp(17.62 t / (243.12 + t))  hPa
/// ```
///
/// WMO states it for −45 °C to 60 °C. Colder air holds so little vapour (1.9 Pa at −60 °C) that
/// continuing the formula below its range changes density negligibly. The formula vanishes as `t`
/// approaches −243.12 °C, so at and below that it returns zero.
pub fn saturation_vapour_pressure_pa(temperature_k: f64) -> f64 {
    let t = temperature_k - CELSIUS_ZERO_K;
    if t <= -243.12 {
        return 0.0;
    }
    611.2 * (17.62 * t / (243.12 + t)).exp()
}

/// The vapour pressure, Pa, of air at `temperature_k` with relative humidity
/// `relative_humidity` (a fraction, relative to liquid water): `e = U e_w(T)`.
///
/// # Errors
///
/// [`AtmosError::Domain`] if the temperature is not finite and positive, or the relative
/// humidity is outside `[0, 1]`.
pub fn vapour_pressure_pa(temperature_k: f64, relative_humidity: f64) -> Result<f64, AtmosError> {
    let t = positive("temperature (K)", temperature_k)?;
    let u = check_relative_humidity(relative_humidity)?;
    Ok(u * saturation_vapour_pressure_pa(t))
}

/// Checks a relative humidity fraction is in `[0, 1]`.
pub(crate) fn check_relative_humidity(relative_humidity: f64) -> Result<f64, AtmosError> {
    let u = finite("relative humidity", relative_humidity)?;
    if !(0.0..=1.0).contains(&u) {
        return Err(AtmosError::Domain {
            what: "relative humidity (fraction)",
            value: u,
        });
    }
    Ok(u)
}

/// The state of moist air at `temperature_k` and `pressure_pa` with water vapour at partial
/// pressure `vapour_pressure_pa`. The vapour mole fraction is capped at 1.
///
/// # Errors
///
/// [`AtmosError::Domain`] if the temperature or pressure is not finite and positive, or the
/// vapour pressure is negative or not finite.
pub fn moist_air(
    temperature_k: f64,
    pressure_pa: f64,
    vapour_pressure_pa: f64,
) -> Result<AirState, AtmosError> {
    let t = positive("temperature (K)", temperature_k)?;
    let p = positive("pressure (Pa)", pressure_pa)?;
    let e = finite("vapour pressure (Pa)", vapour_pressure_pa)?;
    if e < 0.0 {
        return Err(AtmosError::Domain {
            what: "vapour pressure (Pa)",
            value: e,
        });
    }
    Ok(moist_air_unchecked(t, p, e))
}

/// [`moist_air`] for inputs already checked.
pub(crate) fn moist_air_unchecked(t: f64, p: f64, e: f64) -> AirState {
    let x = (e / p).min(1.0);
    let molar_mass = (1.0 - x) * SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL
        + x * WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL;
    // Molar heat capacities in units of R*: 7/2 for dry air (γ = 1.4), 4 for water vapour.
    let cp = (1.0 - x) * 3.5 + x * 4.0;
    let gamma = cp / (cp - 1.0);
    AirState {
        temperature_k: t,
        pressure_pa: p,
        density_kg_m3: p * molar_mass / (GAS_CONSTANT_J_PER_KMOL_K * t),
        speed_of_sound_m_s: (gamma * GAS_CONSTANT_J_PER_KMOL_K * t / molar_mass).sqrt(),
        dynamic_viscosity_pa_s: sutherland_viscosity_pa_s(t),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::ussa76::Ussa76;

    /// Eq. 4.B.1 evaluated separately at a few temperatures (see
    /// `validation/oracles/atmosphere/moist_air.py`, which prints these).
    #[test]
    fn saturation_vapour_pressure_follows_wmo_4_b_1() {
        for (t_c, expected_hpa) in [(0.0, 6.112), (20.0, 23.325_960_2), (-40.0, 0.190_212_012)] {
            let e = saturation_vapour_pressure_pa(273.15 + t_c) / 100.0;
            assert!((e - expected_hpa).abs() < 1e-8 * expected_hpa, "{t_c}: {e}");
        }
        assert_eq!(saturation_vapour_pressure_pa(30.0), 0.0);
        assert!(saturation_vapour_pressure_pa(30.1) >= 0.0);
    }

    /// Dry air is the 1976 standard's air: same density and speed of sound below 80 km.
    #[test]
    fn dry_air_is_the_standard_atmosphere() {
        let standard = Ussa76::standard();
        for z in [0.0, 5_000.0, 30_000.0] {
            let reference = standard.sample(z).unwrap().air;
            let dry = moist_air(reference.temperature_k, reference.pressure_pa, 0.0).unwrap();
            assert!((dry.density_kg_m3 / reference.density_kg_m3 - 1.0).abs() < 1e-14);
            assert!((dry.speed_of_sound_m_s / reference.speed_of_sound_m_s - 1.0).abs() < 1e-14);
            assert_eq!(dry.dynamic_viscosity_pa_s, reference.dynamic_viscosity_pa_s);
        }
    }

    #[derive(Deserialize)]
    struct Fixture {
        cases: Vec<Case>,
    }

    #[derive(Deserialize)]
    struct Case {
        temperature_k: f64,
        pressure_pa: f64,
        relative_humidity: f64,
        cipm_2007_density_kg_m3: f64,
    }

    /// Humid air is lighter, and the ideal-mixture density agrees with the CIPM-2007 formula
    /// (with its real-gas compressibility) to 0.05% over CIPM-2007's range, 15–27 °C and
    /// 600–1100 hPa, from dry to saturated; the largest difference is 0.047%. Reference values
    /// from `validation/oracles/atmosphere/moist_air.py`.
    #[test]
    fn humid_density_agrees_with_cipm_2007() {
        let fixture: Fixture = serde_json::from_str(include_str!(
            "../../../validation/fixtures/atmosphere/cipm-2007-moist-air-density.json"
        ))
        .unwrap();
        assert!(fixture.cases.len() >= 12);
        for case in &fixture.cases {
            let e = vapour_pressure_pa(case.temperature_k, case.relative_humidity).unwrap();
            let air = moist_air(case.temperature_k, case.pressure_pa, e).unwrap();
            let error = air.density_kg_m3 / case.cipm_2007_density_kg_m3 - 1.0;
            assert!(
                error.abs() < 5e-4,
                "T = {} K, p = {} Pa, RH = {}: {error:e}",
                case.temperature_k,
                case.pressure_pa,
                case.relative_humidity
            );
        }
    }

    #[test]
    fn humidity_lowers_density_and_raises_the_speed_of_sound() {
        let (t, p) = (303.15, 101_325.0);
        let dry = moist_air(t, p, 0.0).unwrap();
        let wet = moist_air(t, p, vapour_pressure_pa(t, 1.0).unwrap()).unwrap();
        let density_change = wet.density_kg_m3 / dry.density_kg_m3 - 1.0;
        let sound_change = wet.speed_of_sound_m_s / dry.speed_of_sound_m_s - 1.0;
        // x_v = 42.4/1013.25 = 0.0419: density down by x_v (1 − M_v/M₀) = 1.6%, and the speed of
        // sound up by about 0.6%.
        assert!(
            (-0.0165..-0.0155).contains(&density_change),
            "{density_change}"
        );
        assert!((0.005..0.007).contains(&sound_change), "{sound_change}");
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        assert!(vapour_pressure_pa(300.0, 1.01).is_err());
        assert!(vapour_pressure_pa(300.0, -0.01).is_err());
        assert!(vapour_pressure_pa(0.0, 0.5).is_err());
        assert!(moist_air(300.0, 0.0, 0.0).is_err());
        assert!(moist_air(300.0, 1e5, -1.0).is_err());
        assert!(moist_air(f64::NAN, 1e5, 0.0).is_err());
        // Vapour pressure above the total pressure is capped at pure vapour.
        let steam = moist_air(400.0, 1_000.0, 5_000.0).unwrap();
        let expected = 1_000.0 * WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL
            / (GAS_CONSTANT_J_PER_KMOL_K * 400.0);
        assert!((steam.density_kg_m3 - expected).abs() < 1e-15);
    }
}
