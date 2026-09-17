//! The state of the air at a point, and the [`Atmosphere`] trait every atmosphere model implements.

use std::fmt;

use hpr_core::interp::Side;
use serde::{Deserialize, Serialize};

use crate::error::AtmosError;
use crate::profile::SoundingProfile;
use crate::ussa76::Ussa76;

/// Thermodynamic and transport properties of the air at one point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AirState {
    /// Kinetic temperature `T`, K.
    pub temperature_k: f64,
    /// Static pressure `p`, Pa.
    pub pressure_pa: f64,
    /// Density `ρ`, kg/m³ (of the moist air, where a model carries humidity).
    pub density_kg_m3: f64,
    /// Speed of sound `a`, m/s.
    pub speed_of_sound_m_s: f64,
    /// Dynamic viscosity `μ`, Pa·s.
    pub dynamic_viscosity_pa_s: f64,
}

impl AirState {
    /// Kinematic viscosity `ν = μ/ρ`, m²/s. Infinite where the density is zero.
    pub fn kinematic_viscosity_m2_s(&self) -> f64 {
        self.dynamic_viscosity_pa_s / self.density_kg_m3
    }
}

/// An [`AirState`] and whether the model extrapolated beyond its data or its defined range to
/// produce it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AirSample {
    /// The air at the requested height.
    pub air: AirState,
    /// `Some` when the height was below or above the range the model is defined or tabulated
    /// over; `None` inside it, ends included.
    pub extrapolated: Option<Side>,
}

/// An atmosphere model: the air as a function of height.
///
/// **Height datum.** Every atmosphere is queried with the **geometric height above mean sea
/// level**, in metres. The flight engine's heights are ellipsoidal (`docs/physics/frames.md`), so
/// it subtracts the geoid undulation `N` at the site first: `H = h − N`.
pub trait Atmosphere: fmt::Debug + Send + Sync {
    /// The air at geometric height `height_msl_m` above mean sea level.
    ///
    /// # Errors
    ///
    /// [`AtmosError::Domain`] if the height is not finite, or is outside the heights the model
    /// can represent at all (as opposed to heights it extrapolates to, which it flags).
    fn air(&self, height_msl_m: f64) -> Result<AirSample, AtmosError>;
}

/// Any of the atmosphere models, tagged by `model` when serialized.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "snake_case")]
#[non_exhaustive]
pub enum AtmosphereModel {
    /// [`Ussa76`], optionally offset.
    Standard(Ussa76),
    /// [`SoundingProfile`], boxed because it is far larger than the standard.
    Sounding(Box<SoundingProfile>),
}

impl Default for AtmosphereModel {
    fn default() -> Self {
        AtmosphereModel::Standard(Ussa76::standard())
    }
}

impl Atmosphere for AtmosphereModel {
    fn air(&self, height_msl_m: f64) -> Result<AirSample, AtmosError> {
        match self {
            AtmosphereModel::Standard(model) => model.sample(height_msl_m),
            AtmosphereModel::Sounding(model) => model.sample(height_msl_m),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atmosphere_models_round_trip_through_json() {
        let standard = AtmosphereModel::Standard(Ussa76::with_offset(10.0, 99_000.0).unwrap());
        let json = serde_json::to_string(&standard).unwrap();
        assert_eq!(
            json,
            r#"{"model":"standard","temperature_offset_k":10.0,"sea_level_pressure_pa":99000.0}"#
        );
        assert_eq!(
            serde_json::from_str::<AtmosphereModel>(&json).unwrap(),
            standard
        );
        let sounding = r#"{"model":"sounding","latitude_rad":0.6,"levels":[
            {"height_msl_m":0.0,"temperature_k":290.0,"pressure_pa":100000.0}]}"#;
        let model: AtmosphereModel = serde_json::from_str(sounding).unwrap();
        assert!(matches!(model, AtmosphereModel::Sounding(_)));
        let air = model.air(0.0).unwrap().air;
        assert_eq!(air.pressure_pa, 100_000.0);
        assert_eq!(
            AtmosphereModel::default().air(0.0).unwrap(),
            Ussa76::standard().sample(0.0).unwrap()
        );
    }
}
