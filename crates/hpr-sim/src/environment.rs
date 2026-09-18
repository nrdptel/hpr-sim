//! The flight's surroundings: the Earth model with the launch site, the atmosphere and the wind.

use std::sync::Arc;

use hpr_atmos::{Atmosphere, AtmosphereModel, ConstantWind, Wind};
use hpr_core::earth::Earth;
use hpr_core::geodesy::Geodetic;

use crate::error::SimError;

/// The Earth, atmosphere and wind a flight sees.
///
/// Heights: the state and the Earth model use the launch frame and ellipsoidal heights. The
/// atmosphere and wind take height above mean sea level, `H = h − N`, with the geoid undulation
/// `N` at the site given here (hpr has no geoid model; `docs/physics/geodesy.md`). The ground is
/// the ellipsoidal height of the site. Wind vectors are taken in the launch frame's axes.
#[derive(Debug, Clone)]
pub struct Environment {
    /// The Earth: the launch site and frame, gravity and rotation.
    pub earth: Earth,
    /// The geoid undulation `N` at the site, m (`h = H + N`).
    pub geoid_undulation_m: f64,
    /// The atmosphere, by height above mean sea level. Shared, so environments clone cheaply
    /// across threads.
    pub atmosphere: Arc<dyn Atmosphere>,
    /// The wind, by height above mean sea level, in launch-frame axes.
    pub wind: Arc<dyn Wind>,
}

impl Environment {
    /// An environment from its parts, with `N = 0`.
    pub fn new(
        earth: Earth,
        atmosphere: impl Atmosphere + 'static,
        wind: impl Wind + 'static,
    ) -> Self {
        Self {
            earth,
            geoid_undulation_m: 0.0,
            atmosphere: Arc::new(atmosphere),
            wind: Arc::new(wind),
        }
    }

    /// WGS 84 at `site` (ellipsoidal height), the 1976 standard atmosphere and no wind.
    ///
    /// # Errors
    ///
    /// [`SimError::Core`] for an invalid site.
    pub fn standard(site: Geodetic) -> Result<Self, SimError> {
        Ok(Self::new(
            Earth::wgs84(site)?,
            AtmosphereModel::default(),
            ConstantWind::calm(),
        ))
    }

    /// The same environment with geoid undulation `N`, m.
    #[must_use]
    pub fn with_geoid_undulation_m(mut self, undulation_m: f64) -> Self {
        self.geoid_undulation_m = undulation_m;
        self
    }

    /// The same environment with `wind` in place of its wind.
    #[must_use]
    pub fn with_wind(mut self, wind: impl Wind + 'static) -> Self {
        self.wind = Arc::new(wind);
        self
    }

    /// The launch site.
    #[must_use]
    pub fn site(&self) -> Geodetic {
        self.earth.frame().origin()
    }
}
