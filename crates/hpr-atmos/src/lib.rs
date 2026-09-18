//! Atmosphere and wind models: the US Standard Atmosphere 1976, ISA offsets, custom soundings, wind
//! profiles and turbulence.
//!
//! **Guide:** [Atmosphere][guide-atmosphere], [Wind][guide-wind] and
//! [Turbulence][guide-turbulence]: the models, their sources, how well they are validated and what
//! they leave out.
//!
//! [guide-atmosphere]: https://nrdptel.github.io/hpr-sim/physics/atmosphere.html
//! [guide-wind]: https://nrdptel.github.io/hpr-sim/physics/wind.html
//! [guide-turbulence]: https://nrdptel.github.io/hpr-sim/physics/turbulence.html
//!
//! - [`ussa76`]: the 1976 standard from −5 to 86 km, with temperature offsets and launch-site
//!   anchoring.
//! - [`moist`]: saturation vapour pressure and humid-air density and speed of sound.
//! - [`profile`]: atmospheres from soundings and forecasts, and WMO geopotential heights.
//! - [`wind`]: constant, power-law, logarithmic and layered mean winds.
//! - [`dryden`]: seeded Dryden turbulence.
//!
//! Every model is queried with **geometric height above mean sea level**; the flight engine
//! converts from its ellipsoidal heights first. Samples report whether a model extrapolated.

pub mod air;
pub mod dryden;
pub mod error;
pub mod moist;
pub mod profile;
pub mod ussa76;
pub mod wind;

pub use air::{AirSample, AirState, Atmosphere, AtmosphereModel};
pub use dryden::{DrydenGenerator, DrydenParameters, GustField, GustSample, TurbulenceSeverity};
pub use error::AtmosError;
pub use profile::{SoundingLevel, SoundingProfile};
pub use ussa76::Ussa76;
pub use wind::{
    ConstantWind, LayeredWind, LogLawWind, PowerLawWind, Wind, WindInterpolation, WindLevel,
    WindModel, WindSample,
};
