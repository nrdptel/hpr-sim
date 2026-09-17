//! The state of the air at a point, and the [`Atmosphere`] trait every atmosphere model implements.

use std::fmt;

use hpr_core::interp::Side;
use serde::{Deserialize, Serialize};

use crate::error::AtmosError;

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
