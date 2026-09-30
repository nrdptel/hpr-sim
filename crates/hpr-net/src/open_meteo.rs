//! Weather from Open-Meteo: a launch site's forecast (or a past day's archived forecast) on
//! pressure levels, turned into a [`SoundingProfile`].
//!
//! [Open-Meteo](https://open-meteo.com/en/docs) serves numerical weather models' output as JSON.
//! Two of its APIs carry winds on pressure levels, which a rocket needs above the surface:
//!
//! - the forecast API (`api.open-meteo.com/v1/forecast`), about 16 days ahead and 3 months back;
//! - the historical-forecast API (`historical-forecast-api.open-meteo.com/v1/forecast`), the same
//!   forecasts archived, for a past launch. (Its ERA5 archive API has no pressure levels.)
//!
//! An [`OpenMeteoRequest`] asks for the two whole hours around the launch time, in UTC, with
//! these hourly variables:
//!
//! | variable | unit asked for | where |
//! |---|---|---|
//! | `temperature_2m`, `relative_humidity_2m` | °C, % | 2 m above the ground |
//! | `surface_pressure` | hPa | the ground |
//! | `wind_speed_10m`, `wind_direction_10m` | m/s, ° from | 10 m above the ground |
//! | `temperature_<p>hPa`, `relative_humidity_<p>hPa` | °C, % | each of [`PRESSURE_LEVELS_HPA`] |
//! | `wind_speed_<p>hPa`, `wind_direction_<p>hPa` | m/s, ° from | each level |
//! | `geopotential_height_<p>hPa` | geopotential m | each level |
//!
//! [`OpenMeteoProfile::parse`] reads the answer, refusing any unit but those, and interpolates
//! linearly in time between the two hours (the wind by its east and north components). It keeps:
//!
//! - **The surface** at the response's `elevation`: the surface pressure there, with the 2 m
//!   temperature and humidity and the 10 m wind. Placing the 10 m wind at the ground makes it the
//!   wind on the launch rail, as [Loft lesson L6][l6] asks, rather than a step at the lowest
//!   pressure level.
//! - **Each pressure level above the ground.** The models report every level, including those
//!   below the ground at a high site, where the values are extrapolated. A level is dropped when
//!   its pressure is not below the surface pressure or its height is not above the elevation.
//!   A level with no data at either hour is dropped too, and so is one whose relative humidity is
//!   outside 0 to 100%. [`OpenMeteoProfile::dropped`] lists each, with its reason.
//!
//! Heights are read as geopotential metres, as the weather models define them, and converted to
//! geometric heights at the response's latitude with WMO-No. 8 eq. 12.16
//! ([`hpr_atmos::profile::geometric_from_wmo_geopotential_m`]), as the [atmosphere page][atmos]
//! explains. Open-Meteo's documentation calls the variable an altitude above sea level; the
//! recorded answers' layer thicknesses bear out geopotential metres (the hypsometric check in
//! `tests/open_meteo.rs`). Directions are the meteorological convention, the direction the wind
//! blows from, clockwise from north. Relative humidity is taken as over liquid water, which is
//! what [`SoundingLevel`] means by it. A model that reports it over ice at cold levels shifts the
//! air's density there by `0.378 (e_w − e_i)/p`: at most 27 Pa of vapour pressure (near −12 °C),
//! so under 0.03% at 400 hPa. Higher up the air is colder and the gap smaller: about 6 Pa at
//! −40 °C, 0.08% even at 30 hPa.
//!
//! [`fetch`] asks a [`Client`] for the URL, so the answer comes from the cache when it can, and
//! offline from the cache only; an answer that doesn't parse is never cached. The data is licensed
//! CC BY 4.0: show [`ATTRIBUTION`] (it is on every [`Fetched`]) wherever the weather is shown.
//!
//! **How far to trust it:** the profile gives back every level it keeps as recorded (the tests);
//! how good the forecast is depends on the weather model, and nothing here measures that. The
//! [guide page][guide] says more.
//!
//! ```
//! use hpr_atmos::WindInterpolation;
//! use hpr_net::open_meteo::OpenMeteoProfile;
//!
//! // A response recorded from the historical-forecast API: Spaceport America, 2025-06-21,
//! // 15:00 and 16:00 UTC. Ask for 15:30.
//! let body = include_bytes!("../tests/fixtures/replay/open-meteo-historical.json");
//! let profile = OpenMeteoProfile::parse(body, 1_750_519_800)?;
//! assert_eq!(profile.dropped.len(), 5); // 1000 to 900 hPa lie below the 1,400 m ground.
//! let air = profile.sounding(WindInterpolation::SpeedDirection)?;
//! let at_5_km = air.sample(5_000.0)?.air;
//! assert!((at_5_km.pressure_pa - 55_000.0).abs() < 1_000.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [l6]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l6
//! [atmos]: https://nrdptel.github.io/hpr-sim/physics/atmosphere.html
//! [guide]: https://nrdptel.github.io/hpr-sim/weather.html

use std::f64::consts::TAU;

use hpr_atmos::profile::geometric_from_wmo_geopotential_m;
use hpr_atmos::{AtmosError, SoundingLevel, SoundingProfile, WindInterpolation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Client, Fetched, NetError, Source, Transport};

/// The pressure levels asked for, hPa: all 19 that Open-Meteo's forecast APIs serve.
pub const PRESSURE_LEVELS_HPA: [u32; 19] = [
    1000, 975, 950, 925, 900, 850, 800, 700, 600, 500, 400, 300, 250, 200, 150, 100, 70, 50, 30,
];

/// The credit Open-Meteo's licence (CC BY 4.0) asks for, shown wherever its data is shown.
pub const ATTRIBUTION: &str = "Weather data by Open-Meteo.com (CC BY 4.0)";

/// The hourly surface variables asked for, with the units the parser requires.
const SURFACE_VARIABLES: [(&str, &str); 5] = [
    ("temperature_2m", "°C"),
    ("relative_humidity_2m", "%"),
    ("surface_pressure", "hPa"),
    ("wind_speed_10m", "m/s"),
    ("wind_direction_10m", "°"),
];

/// The hourly variables asked for on each pressure level, as prefixes of `_<p>hPa`, with the
/// units the parser requires.
const LEVEL_VARIABLES: [(&str, &str); 5] = [
    ("temperature", "°C"),
    ("relative_humidity", "%"),
    ("wind_speed", "m/s"),
    ("wind_direction", "°"),
    ("geopotential_height", "m"),
];

/// Seconds in an hour: the forecasts' time step.
const HOUR_S: i64 = 3_600;

/// The first second of the year 10000, past which a date has no four-digit year.
const YEAR_10000_S: i64 = 253_402_300_800;

/// Which Open-Meteo API to ask.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpenMeteoApi {
    /// The forecast API: about 16 days ahead and 3 months back. A copy stays fresh for an hour.
    Forecast,
    /// The historical-forecast API: past forecasts, archived. A copy stays fresh for 30 days.
    HistoricalForecast,
}

impl OpenMeteoApi {
    /// The API's endpoint on Open-Meteo's own servers.
    #[must_use]
    pub fn endpoint(self) -> &'static str {
        match self {
            Self::Forecast => "https://api.open-meteo.com/v1/forecast",
            Self::HistoricalForecast => {
                "https://historical-forecast-api.open-meteo.com/v1/forecast"
            }
        }
    }

    /// How long a cached answer counts as fresh, s. Models run every hour to every six hours, so a
    /// forecast is refetched after an hour; an archived forecast changes little once written.
    #[must_use]
    pub fn ttl_s(self) -> u64 {
        match self {
            Self::Forecast => 3_600,
            Self::HistoricalForecast => 30 * 86_400,
        }
    }
}

/// What to ask Open-Meteo for: a place, a time and an API.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenMeteoRequest {
    /// Latitude, degrees north, in `[-90, 90]`.
    pub latitude_deg: f64,
    /// Longitude, degrees east, in `[-180, 180]`.
    pub longitude_deg: f64,
    /// The launch time, seconds since the Unix epoch (UTC), from 1970 to the year 9999.
    pub time_unix_s: i64,
    /// The API asked.
    pub api: OpenMeteoApi,
    /// A weather model by Open-Meteo's name (for example `"gfs_seamless"`); `None` lets it choose
    /// the best for the place. Letters, digits and `_` only.
    pub model: Option<String>,
    /// Another server's endpoint in place of [`OpenMeteoApi::endpoint`], for a self-hosted
    /// Open-Meteo; `None` for Open-Meteo's own. It must hold no `?` or `#`.
    pub endpoint: Option<String>,
}

impl OpenMeteoRequest {
    /// A request to Open-Meteo's own servers, with the model left to them.
    #[must_use]
    pub fn new(latitude_deg: f64, longitude_deg: f64, time_unix_s: i64, api: OpenMeteoApi) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
            time_unix_s,
            api,
            model: None,
            endpoint: None,
        }
    }

    /// The URL that asks for the two whole hours around the launch time: the hour at or before it
    /// and the one after.
    ///
    /// # Errors
    /// [`OpenMeteoError::Request`] when a field is outside the range its doc gives.
    pub fn url(&self) -> Result<String, OpenMeteoError> {
        let refuse = |what: &'static str, value: String| OpenMeteoError::Request { what, value };
        if !(-90.0..=90.0).contains(&self.latitude_deg) {
            return Err(refuse("latitude (deg)", self.latitude_deg.to_string()));
        }
        if !(-180.0..=180.0).contains(&self.longitude_deg) {
            return Err(refuse("longitude (deg)", self.longitude_deg.to_string()));
        }
        // The hour after the launch hour must still have a four-digit year.
        if !(0..YEAR_10000_S - HOUR_S).contains(&self.time_unix_s) {
            return Err(refuse("time (s since 1970)", self.time_unix_s.to_string()));
        }
        let endpoint = match &self.endpoint {
            Some(endpoint) if endpoint.is_empty() || endpoint.contains(['?', '#']) => {
                return Err(refuse("endpoint", endpoint.clone()));
            }
            Some(endpoint) => endpoint.as_str(),
            None => self.api.endpoint(),
        };
        let mut variables: Vec<String> = SURFACE_VARIABLES
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect();
        for p in PRESSURE_LEVELS_HPA {
            variables.extend(
                LEVEL_VARIABLES
                    .iter()
                    .map(|(name, _)| format!("{name}_{p}hPa")),
            );
        }
        let start = self.time_unix_s.div_euclid(HOUR_S) * HOUR_S;
        let mut url = format!(
            "{endpoint}?latitude={}&longitude={}&hourly={}&wind_speed_unit=ms&timeformat=unixtime\
             &timezone=GMT&start_hour={}&end_hour={}",
            self.latitude_deg,
            self.longitude_deg,
            variables.join(","),
            iso_hour(start),
            iso_hour(start + HOUR_S),
        );
        if let Some(model) = &self.model {
            let plain = |c: char| c.is_ascii_alphanumeric() || c == '_';
            if model.is_empty() || !model.chars().all(plain) {
                return Err(refuse("model", model.clone()));
            }
            url.push_str("&models=");
            url.push_str(model);
        }
        Ok(url)
    }

    /// The [`Source`] a [`Client`] caches this request's answer under: Open-Meteo, its
    /// attribution, and the API's [`OpenMeteoApi::ttl_s`].
    #[must_use]
    pub fn source(&self) -> Source {
        Source {
            name: "Open-Meteo".to_owned(),
            attribution: ATTRIBUTION.to_owned(),
            ttl_s: self.api.ttl_s(),
        }
    }
}

/// The ground under the forecast, at the launch time.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OpenMeteoSurface {
    /// The response's `elevation`: the ground's height above mean sea level, m.
    pub height_msl_m: f64,
    /// Surface pressure, Pa.
    pub pressure_pa: f64,
    /// Temperature 2 m above the ground, K.
    pub temperature_k: f64,
    /// Relative humidity 2 m above the ground, a fraction.
    pub relative_humidity: f64,
    /// Wind speed 10 m above the ground, m/s.
    pub wind_speed_m_s: f64,
    /// Direction the 10 m wind blows from, clockwise from true north, rad in `[0, 2π)`.
    pub wind_direction_from_rad: f64,
}

/// One pressure level above the ground, at the launch time.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OpenMeteoLevel {
    /// The level's pressure, Pa.
    pub pressure_pa: f64,
    /// Its geopotential height as the response gives it, gpm.
    pub geopotential_height_m: f64,
    /// Its geometric height above mean sea level, m (WMO-No. 8 eq. 12.16).
    pub height_msl_m: f64,
    /// Temperature, K.
    pub temperature_k: f64,
    /// Relative humidity, a fraction.
    pub relative_humidity: f64,
    /// Wind speed, m/s.
    pub wind_speed_m_s: f64,
    /// Direction the wind blows from, clockwise from true north, rad in `[0, 2π)`.
    pub wind_direction_from_rad: f64,
}

/// Why a pressure level was left out of the profile.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropReason {
    /// Its pressure is not below the surface pressure, or its height is not above the ground.
    BelowGround,
    /// A value is missing (`null`) at one of the two hours.
    NoData,
    /// Its relative humidity is outside 0 to 100%, which a sounding refuses.
    Humidity,
}

/// A pressure level left out of the profile, and why.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DroppedLevel {
    /// The level's pressure, Pa.
    pub pressure_pa: f64,
    /// Why it was dropped.
    pub reason: DropReason,
}

/// An Open-Meteo response read at one time: the surface and the pressure levels above it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenMeteoProfile {
    /// The model grid point's latitude, degrees north (near the one asked for).
    pub latitude_deg: f64,
    /// The model grid point's longitude, degrees east.
    pub longitude_deg: f64,
    /// The time read, seconds since the Unix epoch (UTC).
    pub time_unix_s: i64,
    /// The hours interpolated between, with their weights, which sum to 1. One hour, weight 1,
    /// when the time is on the hour.
    pub hours: Vec<(i64, f64)>,
    /// The ground.
    pub surface: OpenMeteoSurface,
    /// The pressure levels above the ground, lowest first.
    pub levels: Vec<OpenMeteoLevel>,
    /// The pressure levels left out, highest pressure first.
    pub dropped: Vec<DroppedLevel>,
}

impl OpenMeteoProfile {
    /// Reads an Open-Meteo response at `time_unix_s`, interpolating linearly between the hours
    /// around it.
    ///
    /// # Errors
    /// - [`OpenMeteoError::Json`] when the body is not JSON, [`OpenMeteoError::Server`] when it is
    ///   Open-Meteo's error answer.
    /// - [`OpenMeteoError::Missing`] when a field or a variable asked for is absent, and
    ///   [`OpenMeteoError::Units`] when a variable is in a unit other than the one asked for.
    /// - [`OpenMeteoError::TimeOutside`] when the time is outside the response's hours.
    /// - [`OpenMeteoError::NoSurface`] when a surface value is missing at either hour, and
    ///   [`OpenMeteoError::OutOfRange`] when the surface humidity is outside 0 to 100%.
    /// - [`OpenMeteoError::Atmos`] when a geopotential height has no geometric height.
    pub fn parse(body: &[u8], time_unix_s: i64) -> Result<Self, OpenMeteoError> {
        let root: Value =
            serde_json::from_slice(body).map_err(|e| OpenMeteoError::Json(e.to_string()))?;
        if root.get("error").and_then(Value::as_bool) == Some(true) {
            let reason = root
                .get("reason")
                .and_then(Value::as_str)
                .unwrap_or("no reason given");
            return Err(OpenMeteoError::Server {
                reason: reason.to_owned(),
            });
        }
        let latitude_deg = number(&root, "latitude")?;
        let longitude_deg = number(&root, "longitude")?;
        let elevation_m = number(&root, "elevation")?;
        let units = root
            .get("hourly_units")
            .ok_or_else(|| missing("hourly_units"))?;
        let hourly = root.get("hourly").ok_or_else(|| missing("hourly"))?;
        let series = Series::new(hourly, units, time_unix_s)?;
        let latitude_rad = latitude_deg.to_radians();

        let surface_value = |name: &str, unit: &'static str| match series.value(name, unit)? {
            Some(value) => Ok(value),
            None => Err(OpenMeteoError::NoSurface {
                field: name.to_owned(),
            }),
        };
        // Name the missing one of the pair before reading them as a wind.
        surface_value("wind_speed_10m", "m/s")?;
        surface_value("wind_direction_10m", "°")?;
        let (wind_speed_m_s, wind_direction_from_rad) =
            match series.wind("wind_speed_10m", "wind_direction_10m")? {
                Some(wind) => wind,
                None => {
                    return Err(OpenMeteoError::NoSurface {
                        field: "wind_speed_10m".to_owned(),
                    });
                }
            };
        let surface = OpenMeteoSurface {
            height_msl_m: elevation_m,
            pressure_pa: surface_value("surface_pressure", "hPa")? * 100.0,
            temperature_k: celsius_to_kelvin(surface_value("temperature_2m", "°C")?),
            relative_humidity: surface_value("relative_humidity_2m", "%")? / 100.0,
            wind_speed_m_s,
            wind_direction_from_rad,
        };
        if !(0.0..=1.0).contains(&surface.relative_humidity) {
            return Err(OpenMeteoError::OutOfRange {
                field: "relative_humidity_2m".to_owned(),
                value: surface.relative_humidity * 100.0,
            });
        }

        let mut levels = Vec::new();
        let mut dropped = Vec::new();
        for p in PRESSURE_LEVELS_HPA {
            let pressure_pa = f64::from(p) * 100.0;
            let name = |prefix: &str| format!("{prefix}_{p}hPa");
            let geopotential = series.value(&name("geopotential_height"), "m")?;
            let temperature = series.value(&name("temperature"), "°C")?;
            let humidity = series.value(&name("relative_humidity"), "%")?;
            let wind = series.wind(&name("wind_speed"), &name("wind_direction"))?;
            let (Some(geopotential_height_m), Some(temperature_c), Some(humidity), Some(wind)) =
                (geopotential, temperature, humidity, wind)
            else {
                dropped.push(DroppedLevel {
                    pressure_pa,
                    reason: DropReason::NoData,
                });
                continue;
            };
            let height_msl_m =
                geometric_from_wmo_geopotential_m(geopotential_height_m, latitude_rad)?;
            if pressure_pa >= surface.pressure_pa || height_msl_m <= surface.height_msl_m {
                dropped.push(DroppedLevel {
                    pressure_pa,
                    reason: DropReason::BelowGround,
                });
                continue;
            }
            if !(0.0..=100.0).contains(&humidity) {
                dropped.push(DroppedLevel {
                    pressure_pa,
                    reason: DropReason::Humidity,
                });
                continue;
            }
            levels.push(OpenMeteoLevel {
                pressure_pa,
                geopotential_height_m,
                height_msl_m,
                temperature_k: celsius_to_kelvin(temperature_c),
                relative_humidity: humidity / 100.0,
                wind_speed_m_s: wind.0,
                wind_direction_from_rad: wind.1,
            });
        }
        Ok(Self {
            latitude_deg,
            longitude_deg,
            time_unix_s,
            hours: series.hours(),
            surface,
            levels,
            dropped,
        })
    }

    /// The profile as an atmosphere with its wind: the surface, then each level above it, every
    /// one with its pressure, temperature, relative humidity and wind.
    ///
    /// # Errors
    /// What [`SoundingProfile::new`] refuses: heights not increasing, pressures not decreasing
    /// upward, or a value out of range (such as a relative humidity above 100%).
    pub fn sounding(
        &self,
        wind_interpolation: WindInterpolation,
    ) -> Result<SoundingProfile, AtmosError> {
        let s = &self.surface;
        let surface = SoundingLevel {
            height_msl_m: s.height_msl_m,
            temperature_k: s.temperature_k,
            pressure_pa: Some(s.pressure_pa),
            relative_humidity: Some(s.relative_humidity),
            wind_speed_m_s: Some(s.wind_speed_m_s),
            wind_direction_from_rad: Some(s.wind_direction_from_rad),
        };
        let levels = std::iter::once(surface)
            .chain(self.levels.iter().map(|l| SoundingLevel {
                height_msl_m: l.height_msl_m,
                temperature_k: l.temperature_k,
                pressure_pa: Some(l.pressure_pa),
                relative_humidity: Some(l.relative_humidity),
                wind_speed_m_s: Some(l.wind_speed_m_s),
                wind_direction_from_rad: Some(l.wind_direction_from_rad),
            }))
            .collect();
        SoundingProfile::new(levels, self.latitude_deg.to_radians(), wind_interpolation)
    }
}

/// Fetches `request` through `client` and reads it at the request's time.
///
/// The answer comes from the client's cache while fresh (see [`OpenMeteoRequest::source`]); offline,
/// from the cache only. The [`Fetched`] says which, carries [`ATTRIBUTION`] and holds the body.
/// Only an answer that parses is cached ([`Client::fetch_checked`]): one that doesn't never takes
/// a good copy's place, and online a stale good copy is returned instead, with the reason.
///
/// # Errors
/// [`OpenMeteoError::Request`] for a bad request; [`OpenMeteoError::Net`] when the fetch fails,
/// or with [`NetError::Refused`] naming what [`OpenMeteoProfile::parse`] or
/// [`OpenMeteoProfile::sounding`] refused when the only answer there is doesn't pass them.
pub fn fetch<T: Transport>(
    client: &Client<T>,
    request: &OpenMeteoRequest,
    now_s: u64,
) -> Result<(OpenMeteoProfile, Fetched), OpenMeteoError> {
    let url = request.url()?;
    let time_s = request.time_unix_s;
    let check = |body: &[u8]| {
        // A profile the sounding would refuse is refused here too, so it is never cached.
        let profile = OpenMeteoProfile::parse(body, time_s).map_err(|e| e.to_string())?;
        profile
            .sounding(WindInterpolation::SpeedDirection)
            .map(drop)
            .map_err(|e| e.to_string())
    };
    let fetched = client.fetch_checked(&request.source(), &url, now_s, check)?;
    let profile = OpenMeteoProfile::parse(&fetched.body, time_s)?;
    Ok((profile, fetched))
}

/// Why an Open-Meteo request or response was refused.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum OpenMeteoError {
    /// A request field is outside its range.
    #[error("the request's {what} is out of range: {value}")]
    Request {
        /// The field.
        what: &'static str,
        /// Its value.
        value: String,
    },
    /// The fetch failed.
    #[error(transparent)]
    Net(#[from] NetError),
    /// The body is not JSON.
    #[error("the Open-Meteo response is not JSON: {0}")]
    Json(String),
    /// The body is Open-Meteo's error answer, from [`OpenMeteoProfile::parse`]. Open-Meteo sends
    /// it with an HTTP error status, which `Http` reports as [`NetError::Transport`] without the
    /// body; [`fetch`] reports any answer that doesn't parse as [`NetError::Refused`], with this
    /// error's message.
    #[error("Open-Meteo refused the request: {reason}")]
    Server {
        /// Its reason.
        reason: String,
    },
    /// A field or variable is absent, or not of the expected type.
    #[error("the Open-Meteo response has no usable {field}")]
    Missing {
        /// The field.
        field: String,
    },
    /// A variable is in a unit other than the one asked for.
    #[error("the Open-Meteo response gives {field} in {found:?}, not {expected:?}")]
    Units {
        /// The variable.
        field: String,
        /// The unit it came in.
        found: String,
        /// The unit asked for.
        expected: &'static str,
    },
    /// The time is outside the response's hours.
    #[error("{time_unix_s} s is outside the Open-Meteo response's hours, {first_s} to {last_s} s")]
    TimeOutside {
        /// The time asked for.
        time_unix_s: i64,
        /// The first hour in the response.
        first_s: i64,
        /// The last hour in the response.
        last_s: i64,
    },
    /// A surface value is outside its range at the launch time.
    #[error("the Open-Meteo response's {field} is out of range at the launch time: {value}")]
    OutOfRange {
        /// The variable.
        field: String,
        /// Its value, in the response's unit.
        value: f64,
    },
    /// A surface value is missing at the launch time.
    #[error("the Open-Meteo response has no {field} at the launch time")]
    NoSurface {
        /// The variable.
        field: String,
    },
    /// A height or level was refused by the atmosphere.
    #[error(transparent)]
    Atmos(#[from] AtmosError),
}

/// The `hourly` arrays of a response, read at one time.
struct Series<'a> {
    hourly: &'a Value,
    units: &'a Value,
    /// The index and time of the hour at or before the time, and of the hour after with its
    /// weight (the before hour's is one minus it); `after` is `None` when the time is on an hour.
    before: (usize, i64),
    after: Option<(usize, i64, f64)>,
}

impl<'a> Series<'a> {
    fn new(hourly: &'a Value, units: &'a Value, time_unix_s: i64) -> Result<Self, OpenMeteoError> {
        let unit = units.get("time").and_then(Value::as_str);
        if unit != Some("unixtime") {
            return Err(OpenMeteoError::Units {
                field: "time".to_owned(),
                found: unit.unwrap_or("nothing").to_owned(),
                expected: "unixtime",
            });
        }
        let times = hourly
            .get("time")
            .and_then(Value::as_array)
            .ok_or_else(|| missing("hourly.time"))?
            .iter()
            .map(|t| t.as_i64().ok_or_else(|| missing("hourly.time")))
            .collect::<Result<Vec<_>, _>>()?;
        // Times from 1970 to the year 9999 keep every difference below far from overflow.
        if times.iter().any(|t| !(0..=YEAR_10000_S).contains(t)) {
            return Err(missing("hourly.time from 1970 to 9999"));
        }
        if times.windows(2).any(|w| w[1] <= w[0]) {
            return Err(missing("hourly.time in increasing order"));
        }
        let (Some(&first_s), Some(&last_s)) = (times.first(), times.last()) else {
            return Err(missing("hourly.time"));
        };
        if !(first_s..=last_s).contains(&time_unix_s) {
            return Err(OpenMeteoError::TimeOutside {
                time_unix_s,
                first_s,
                last_s,
            });
        }
        // The last hour at or before the time; it exists because the time is not before the first.
        let i = times.partition_point(|&t| t <= time_unix_s) - 1;
        let after = if times[i] == time_unix_s {
            None
        } else {
            // Not on the last hour, so another follows.
            let t1 = times[i + 1];
            #[expect(
                clippy::cast_precision_loss,
                reason = "hour spans and offsets are far below 2^52 s"
            )]
            let weight = (time_unix_s - times[i]) as f64 / (t1 - times[i]) as f64;
            Some((i + 1, t1, weight))
        };
        Ok(Self {
            hourly,
            units,
            before: (i, times[i]),
            after,
        })
    }

    fn hours(&self) -> Vec<(i64, f64)> {
        match self.after {
            None => vec![(self.before.1, 1.0)],
            Some((_, t1, w)) => vec![(self.before.1, 1.0 - w), (t1, w)],
        }
    }

    /// A variable's values at the one or two hours, after checking its unit; `None` when either
    /// is `null`.
    fn raw(&self, name: &str, unit: &'static str) -> Result<Option<(f64, f64)>, OpenMeteoError> {
        let found = self.units.get(name).and_then(Value::as_str);
        if found != Some(unit) {
            return Err(OpenMeteoError::Units {
                field: name.to_owned(),
                found: found.unwrap_or("nothing").to_owned(),
                expected: unit,
            });
        }
        let values = self
            .hourly
            .get(name)
            .and_then(Value::as_array)
            .ok_or_else(|| missing(name))?;
        let at = |i: usize| -> Result<Option<f64>, OpenMeteoError> {
            match values.get(i) {
                Some(Value::Null) => Ok(None),
                Some(v) => v.as_f64().map(Some).ok_or_else(|| missing(name)),
                None => Err(missing(name)),
            }
        };
        let a = at(self.before.0)?;
        let b = match self.after {
            Some((j, _, _)) => at(j)?,
            None => a,
        };
        Ok(a.zip(b))
    }

    /// A variable at the time, linear between the hours.
    fn value(&self, name: &str, unit: &'static str) -> Result<Option<f64>, OpenMeteoError> {
        Ok(self.raw(name, unit)?.map(|(a, b)| match self.after {
            None => a,
            // Exact when a == b, and never outside [a, b]: 100% humidity stays 100%.
            Some((_, _, w)) => a + w * (b - a),
        }))
    }

    /// A wind's speed (m/s) and direction from (rad in `[0, 2π)`) at the time. Between hours its
    /// east and north components are interpolated, so a veering wind passes through the shorter
    /// arc and a reversing one through calm.
    fn wind(&self, speed: &str, direction: &str) -> Result<Option<(f64, f64)>, OpenMeteoError> {
        let (Some(s), Some(d)) = (self.raw(speed, "m/s")?, self.raw(direction, "°")?) else {
            return Ok(None);
        };
        Ok(Some(match self.after {
            None => (s.0, wrap_direction(d.0.to_radians())),
            Some((_, _, w)) => {
                let (d0, d1) = (d.0.to_radians(), d.1.to_radians());
                // The components of the velocity the wind blows toward, east and north.
                let east = (1.0 - w) * -s.0 * d0.sin() + w * -s.1 * d1.sin();
                let north = (1.0 - w) * -s.0 * d0.cos() + w * -s.1 * d1.cos();
                let speed = east.hypot(north);
                let from = if speed > 0.0 {
                    wrap_direction((-east).atan2(-north))
                } else {
                    0.0
                };
                (speed, from)
            }
        }))
    }
}

fn missing(field: &str) -> OpenMeteoError {
    OpenMeteoError::Missing {
        field: field.to_owned(),
    }
}

fn number(root: &Value, field: &str) -> Result<f64, OpenMeteoError> {
    root.get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| missing(field))
}

/// An angle in `[0, 2π)`. `rem_euclid` alone returns 2π for a tiny negative angle, such as the
/// `−2.4e-16` that `atan2` gives for a wind from 360°.
fn wrap_direction(angle_rad: f64) -> f64 {
    let wrapped = angle_rad.rem_euclid(TAU);
    if wrapped >= TAU { 0.0 } else { wrapped }
}

fn celsius_to_kelvin(celsius: f64) -> f64 {
    celsius + 273.15
}

/// `YYYY-MM-DDTHH:00` for a whole hour since the Unix epoch, UTC. The date is Hinnant's
/// `civil_from_days` (<https://howardhinnant.github.io/date_algorithms.html>, public domain),
/// for days on or after 1970-01-01.
fn iso_hour(unix_s: i64) -> String {
    let days = unix_s.div_euclid(86_400);
    let hour = unix_s.rem_euclid(86_400) / HOUR_S;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:00")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_hour_matches_known_dates() {
        assert_eq!(iso_hour(0), "1970-01-01T00:00");
        assert_eq!(iso_hour(1_750_518_000), "2025-06-21T15:00");
        // A leap day, and the last hour of a century that is a leap year.
        assert_eq!(iso_hour(951_782_400), "2000-02-29T00:00");
        assert_eq!(iso_hour(978_303_600), "2000-12-31T23:00");
        assert_eq!(iso_hour(YEAR_10000_S - HOUR_S), "9999-12-31T23:00");
    }

    #[test]
    fn url_asks_for_the_two_hours_around_the_time() -> Result<(), OpenMeteoError> {
        let mut request = OpenMeteoRequest::new(
            32.99,
            -106.97,
            1_750_519_800,
            OpenMeteoApi::HistoricalForecast,
        );
        let url = request.url()?;
        assert!(url.starts_with(
            "https://historical-forecast-api.open-meteo.com/v1/forecast?latitude=32.99\
             &longitude=-106.97&hourly=temperature_2m,relative_humidity_2m,surface_pressure,"
        ));
        assert!(url.ends_with(
            "&wind_speed_unit=ms&timeformat=unixtime&timezone=GMT\
             &start_hour=2025-06-21T15:00&end_hour=2025-06-21T16:00"
        ));
        // 5 surface variables and 5 on each of 19 levels.
        let hourly = url
            .split("hourly=")
            .nth(1)
            .and_then(|s| s.split('&').next());
        assert_eq!(hourly.map(|h| h.split(',').count()), Some(100));

        request.model = Some("gfs_seamless".into());
        request.endpoint = Some("http://127.0.0.1:8080/v1/forecast".into());
        let url = request.url()?;
        assert!(url.starts_with("http://127.0.0.1:8080/v1/forecast?latitude="));
        assert!(url.ends_with("&models=gfs_seamless"));
        Ok(())
    }

    #[test]
    fn url_refuses_each_bad_field() {
        let good = OpenMeteoRequest::new(32.99, -106.97, 0, OpenMeteoApi::Forecast);
        let cases: [(&str, OpenMeteoRequest); 9] = [
            (
                "latitude (deg)",
                OpenMeteoRequest {
                    latitude_deg: 90.5,
                    ..good.clone()
                },
            ),
            (
                "latitude (deg)",
                OpenMeteoRequest {
                    latitude_deg: f64::NAN,
                    ..good.clone()
                },
            ),
            (
                "longitude (deg)",
                OpenMeteoRequest {
                    longitude_deg: -181.0,
                    ..good.clone()
                },
            ),
            (
                "time (s since 1970)",
                OpenMeteoRequest {
                    time_unix_s: -1,
                    ..good.clone()
                },
            ),
            (
                "time (s since 1970)",
                OpenMeteoRequest {
                    time_unix_s: YEAR_10000_S - HOUR_S,
                    ..good.clone()
                },
            ),
            (
                "model",
                OpenMeteoRequest {
                    model: Some("gfs&x=1".into()),
                    ..good.clone()
                },
            ),
            (
                "endpoint",
                OpenMeteoRequest {
                    endpoint: Some("http://h/v1#x".into()),
                    ..good.clone()
                },
            ),
            (
                "endpoint",
                OpenMeteoRequest {
                    endpoint: Some("http://h/?a".into()),
                    ..good.clone()
                },
            ),
            (
                "endpoint",
                OpenMeteoRequest {
                    endpoint: Some(String::new()),
                    ..good.clone()
                },
            ),
        ];
        for (field, request) in cases {
            match request.url() {
                Err(OpenMeteoError::Request { what, .. }) => assert_eq!(what, field),
                other => panic!("{field}: {other:?}"),
            }
        }
        assert!(good.url().is_ok());
    }

    #[test]
    fn sources_keep_forecasts_an_hour_and_archives_a_month() {
        let forecast = OpenMeteoRequest::new(0.0, 0.0, 0, OpenMeteoApi::Forecast).source();
        let archive = OpenMeteoRequest::new(0.0, 0.0, 0, OpenMeteoApi::HistoricalForecast).source();
        assert_eq!((forecast.ttl_s, archive.ttl_s), (3_600, 2_592_000));
        assert_eq!(forecast.attribution, ATTRIBUTION);
    }
}
