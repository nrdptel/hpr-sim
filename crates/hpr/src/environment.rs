//! Where and in what weather a rocket flies.

use hpr_atmos::{Atmosphere, ConstantWind, Wind};
use hpr_core::geodesy::Geodetic;

use crate::error::{Error, finite};

/// Where and in what weather a rocket flies: a launch site, an atmosphere and a wind.
///
/// [`Environment::new`] puts the site on the WGS 84 ellipsoid, with the 1976 US Standard
/// Atmosphere and no wind. The `with_` methods change the weather. For what this type doesn't
/// offer, such as a geoid undulation, build an [`hpr_sim::Environment`] and wrap it with
/// [`Environment::from_sim`].
///
/// ```
/// use hpr::Environment;
///
/// // Spaceport America, 1,400 m up, with 5 m/s of wind from the west.
/// let environment = Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(5.0, 270.0)?;
/// assert_eq!(environment.sim().site().height_m, 1400.0);
/// # Ok::<(), hpr::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Environment {
    sim: hpr_sim::Environment,
}

impl Environment {
    /// A launch site at latitude `latitude_deg` (degrees north), longitude `longitude_deg`
    /// (degrees east) and elevation `elevation_m`, with the 1976 US Standard Atmosphere and no
    /// wind.
    ///
    /// The elevation is taken as both the site's height above mean sea level, which the
    /// atmosphere and wind are read at, and its height above the WGS 84 ellipsoid, which the
    /// flight's position is measured from, as if the geoid undulation, the gap between the two,
    /// were zero; hpr has no geoid model. The air is read where you said; the site's gravity is
    /// off by the free-air gradient, about 3.1 µm/s² per metre of undulation. To give an
    /// undulation `N`, build an [`hpr_sim::Environment`] on the site's ellipsoidal height
    /// `h = H + N`, set `N` with [`hpr_sim::Environment::with_geoid_undulation_m`], and wrap it
    /// with [`Environment::from_sim`]; the air is then read at `H`. Heights in a flight's results
    /// are above the site. Longitude is positive east, so a site in the Americas has a negative
    /// one.
    ///
    /// # Errors
    ///
    /// [`Error::Core`] for a latitude outside `[−90°, 90°]` or a value that isn't finite.
    pub fn new(latitude_deg: f64, longitude_deg: f64, elevation_m: f64) -> Result<Self, Error> {
        let site = Geodetic::from_degrees(latitude_deg, longitude_deg, elevation_m)?;
        Ok(Self {
            sim: hpr_sim::Environment::standard(site)?,
        })
    }

    /// The same place and atmosphere, with `wind` in place of the wind: any [`Wind`], such as
    /// [`hpr_atmos`]'s power law, log law or layers by height, or a model of your own.
    #[must_use]
    pub fn with_wind(self, wind: impl Wind + 'static) -> Self {
        Self {
            sim: self.sim.with_wind(wind),
        }
    }

    /// The same place and atmosphere, with a wind of `speed_m_s` at every height, blowing from
    /// `from_deg`: the direction it comes from, clockwise from true north (270° is a west wind).
    ///
    /// # Errors
    ///
    /// [`Error::Domain`] for a direction that isn't finite; [`Error::Atmos`] for a negative or
    /// non-finite speed.
    pub fn with_constant_wind(self, speed_m_s: f64, from_deg: f64) -> Result<Self, Error> {
        let from_deg = finite("wind direction, degrees", from_deg)?;
        Ok(self.with_wind(ConstantWind::new(speed_m_s, from_deg.to_radians())?))
    }

    /// The same place and wind, with `atmosphere` in place of the standard one: any
    /// [`Atmosphere`], such as a balloon sounding's levels ([`hpr_atmos::SoundingProfile`]), or a
    /// model of your own.
    #[must_use]
    pub fn with_atmosphere(mut self, atmosphere: impl Atmosphere + 'static) -> Self {
        self.sim.atmosphere = std::sync::Arc::new(atmosphere);
        self
    }

    /// An environment built with [`hpr_sim`] directly.
    #[must_use]
    pub fn from_sim(sim: hpr_sim::Environment) -> Self {
        Self { sim }
    }

    /// The [`hpr_sim::Environment`] this wraps.
    #[must_use]
    pub fn sim(&self) -> &hpr_sim::Environment {
        &self.sim
    }
}
