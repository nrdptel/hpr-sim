//! Weather from NOAA's GFS and RAP forecasts: a small GRIB2 cut from NOMADS' grib filter around a
//! launch site, turned into a [`SoundingProfile`].
//!
//! NOAA's National Centers for Environmental Prediction (NCEP) run two models whose output
//! [NOMADS](https://nomads.ncep.noaa.gov) serves through a *grib filter*, a web form that cuts
//! chosen variables, levels and a latitude/longitude box out of a forecast file:
//!
//! | model | grid | runs | forecast hours | pressure levels asked for |
//! |---|---|---|---|---|
//! | [`NomadsModel::Gfs`], the Global Forecast System | 0.25° latitude/longitude | every 6 h | every hour to 120, every third to 384 | [`GFS_LEVELS_HPA`], 1000 to 10 hPa |
//! | [`NomadsModel::Rap`], the Rapid Refresh | 13 km Lambert conformal, the contiguous U.S. and nearby Canada and Mexico | every hour | to 21; to 51 from the 03, 09, 15 and 21 UTC runs | [`RAP_LEVELS_HPA`], 1000 to 100 hPa every 25 |
//!
//! A [`NomadsRequest`] names the model, the run (its start, the *cycle*), the forecast hour and the
//! site; [`NomadsRequest::url`] refuses a run or an hour the model doesn't have. Its URL asks for
//! a box 0.3° each way around the site, which holds the four grid points around it: 9 GFS points
//! or about 25 RAP points, 147 or 192 fields (the ground's skin temperature, `TMP` at the surface,
//! comes too and is not used), about 28 or 40 KB. It asks for:
//!
//! | variable | GRIB2 parameter (discipline 0) | where |
//! |---|---|---|
//! | `HGT`, geopotential height, gpm | category 3, number 5 | the ground (the model's terrain) and each level |
//! | `PRES`, pressure, Pa | 3, 0 | the ground |
//! | `TMP`, temperature, K | 0, 0 | 2 m above the ground and each level |
//! | `RH`, relative humidity, % | 1, 1 | 2 m above the ground and each level |
//! | `UGRD`, `VGRD`, wind components, m/s | 2, 2 and 2, 3 | 10 m above the ground and each level |
//!
//! GRIB2's code table 4.2 fixes each parameter's unit, so there are no units to check.
//! [`NomadsProfile::parse`] decodes the cut with [`hpr_io::grib2`] and interpolates every field
//! bilinearly, in the grid's own indices, between the four grid points around the site. It keeps:
//!
//! - **The ground**, at the model's terrain height there, with the surface pressure, the 2 m
//!   temperature and humidity, and the 10 m wind, as [`crate::open_meteo`] does (so the wind on
//!   the rail is the 10 m wind, [Loft lesson L6][l6]).
//! - **Each pressure level above the ground**: a level whose pressure is not below the surface
//!   pressure, or whose height is not above the ground, is dropped, since the models extrapolate
//!   beneath the terrain. So is one without every variable at every surrounding point.
//!   [`NomadsProfile::dropped`] lists each with its reason.
//!
//! The ground test is made at the site only: a kept level can take weight from a grid point where
//! it is underground, and so from the model's extrapolation there. In the recorded RAP cut, 850 hPa
//! takes 13% of its weight from a point whose ground is at 845.8 hPa, about 0.02 K; in steep
//! terrain it can be more. A cut is refused when the four points' weighted position is not within
//! 1 km of the site (bilinear weights put it within a metre), which catches a grid whose numbers are
//! self-consistent but wrong, and when it holds more than [`MAX_FIELDS`] fields.
//!
//! **Winds along the grid.** RAP gives its winds along the Lambert grid's axes, not east and north
//! (GRIB2 flag table 3.3, bit 5). They are turned to east and north by the angle between the
//! grid's `y` axis and true north at the site, `θ = n (λ − λ₀)`
//! ([`hpr_io::grib2::Grid::earth_relative_wind`]); at Spaceport America, 12° west of RAP's central
//! meridian, `θ` is −5.06°. The components are interpolated first and turned once, at the site:
//! across one 13 km cell `θ` changes by about 0.06°.
//!
//! **Heights** are geopotential metres, which is what GRIB2 defines `HGT` in, converted to
//! geometric heights at the site's latitude with WMO-No. 8 eq. 12.16
//! ([`hpr_atmos::profile::geometric_from_wmo_geopotential_m`]), as for Open-Meteo. The terrain
//! height is also given in gpm and converted the same way: at 1,400 m and 33° N the two differ by
//! 1.9 m, so if a model's terrain is really a geometric height the ground here sits that far high
//! ([ADR-119][adr-119]'s caveat, from the Open-Meteo
//! source). **Relative humidity** is
//! taken as over liquid water, which [`SoundingLevel`] means. Whether NCEP's models report it
//! over ice at cold levels is not settled here; if they do, the density there shifts by under
//! 0.1% (the bound on [`crate::open_meteo`]). A humidity above 100% is kept as recorded and
//! clamped to 100% in [`NomadsProfile::sounding`], as [ADR-004][adr-004] (the atmosphere's
//! design) asks of imported humidity.
//!
//! [`fetch`] asks a [`Client`] for the URL, so the answer comes from the cache when it can, and
//! offline from the cache only; only an answer that decodes into a profile for the run and hour
//! asked is cached. A run's files don't change once written, so a copy stays fresh 30 days (NOMADS
//! keeps only recent runs: on 2026-09-30 its filters listed 10 days of GFS and 2 of RAP). The data is a U.S. government work, free of
//! copyright; [`ATTRIBUTION`] credits it.
//!
//! **How far to trust it:** the decoder gives every value and grid point ecCodes does (the tests),
//! and the profile gives back the interpolated values at every level it keeps. How good a forecast
//! is depends on the model, and nothing here measures that. The [guide page][guide] says more.
//!
//! ```
//! use hpr_atmos::WindInterpolation;
//! use hpr_net::nomads::NomadsProfile;
//!
//! // A cut recorded from GFS's run of 2026-09-30 00 UTC, hour 18, around Spaceport America.
//! let body = include_bytes!("../tests/fixtures/replay/nomads-gfs.grib2");
//! let profile = NomadsProfile::parse(body, 32.99, -106.97)?;
//! assert_eq!(profile.dropped.len(), 6); // 1000 to 850 hPa lie below the 1,400 m ground.
//! let air = profile.sounding(WindInterpolation::SpeedDirection)?;
//! let at_5_km = air.sample(5_000.0)?.air;
//! assert!((at_5_km.pressure_pa - 55_000.0).abs() < 1_000.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [l6]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l6
//! [guide]: https://nrdptel.github.io/hpr-sim/nomads.html
//! [adr-119]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-119-m52-split-open-meteos-pressure-levels-as-a-sounding-2026-09-30
//! [adr-004]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-004-atmosphere-wind-turbulence-and-the-seeded-generator-2026-09-17

use std::collections::HashMap;
use std::f64::consts::TAU;

use hpr_atmos::profile::geometric_from_wmo_geopotential_m;
use hpr_atmos::{AtmosError, SoundingLevel, SoundingProfile, WindInterpolation};
use hpr_io::grib2::{self, Field, Grib2Error, Grid, Projection};
use serde::{Deserialize, Serialize};

use crate::civil::{date_hour, unix_day_start};
use crate::{Client, Fetched, NetError, Source, Transport};

/// NOMADS' grib filters, whose scripts a request's URL names.
pub const ENDPOINT: &str = "https://nomads.ncep.noaa.gov/cgi-bin";

/// The credit shown with the data. NCEP's forecasts are U.S. government works, not under
/// copyright; the credit says where they came from.
pub const ATTRIBUTION: &str = "Forecast data from NOAA/NCEP (GFS, RAP), via NOMADS";

/// GFS's pressure levels asked for, hPa: every level it has from 1000 to 10 hPa (about 31 km).
pub const GFS_LEVELS_HPA: [u32; 28] = [
    1000, 975, 950, 925, 900, 850, 800, 750, 700, 650, 600, 550, 500, 450, 400, 350, 300, 250, 200,
    150, 100, 70, 50, 40, 30, 20, 15, 10,
];

/// RAP's pressure levels asked for, hPa: all of them, 1000 to 100 hPa every 25 (about 16 km).
pub const RAP_LEVELS_HPA: [u32; 37] = [
    1000, 975, 950, 925, 900, 875, 850, 825, 800, 775, 750, 725, 700, 675, 650, 625, 600, 575, 550,
    525, 500, 475, 450, 425, 400, 375, 350, 325, 300, 275, 250, 225, 200, 175, 150, 125, 100,
];

/// The box asked for reaches this far each way from the site, degrees: more than a GFS cell
/// (0.25°) and, at the latitudes RAP covers, a rotated 13 km RAP cell.
const BOX_DEG: f64 = 0.3;

/// The most fields a cut may hold: a real one has 147 (GFS) or 192 (RAP). It bounds the work a
/// hostile answer can ask for.
pub const MAX_FIELDS: usize = 1_000;

/// How far the four grid points' weighted position may be from the site, m: bilinear weights on a
/// grid place it within a metre; farther means the grid is not what it says.
const PLACE_TOLERANCE_M: f64 = 1_000.0;

/// A run's file doesn't change once written.
const TTL_S: u64 = 30 * 86_400;

const HOUR_S: i64 = 3_600;

/// The first second of the year 10000.
const YEAR_10000_S: i64 = 253_402_300_800;

/// GRIB2 parameters by (category, number) in discipline 0 (code table 4.2).
const TMP: (u8, u8) = (0, 0);
const RH: (u8, u8) = (1, 1);
const UGRD: (u8, u8) = (2, 2);
const VGRD: (u8, u8) = (2, 3);
const PRES: (u8, u8) = (3, 0);
const HGT: (u8, u8) = (3, 5);

/// Fixed surface types (code table 4.5).
const GROUND: u8 = 1;
const ISOBARIC: u8 = 100;
const ABOVE_GROUND: u8 = 103;

/// Which NCEP model to ask for.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NomadsModel {
    /// The Global Forecast System on its 0.25° grid (`gfs.tHHz.pgrb2.0p25.fFFF`).
    Gfs,
    /// The Rapid Refresh on its 13 km grid 130 over the contiguous United States
    /// (`rap.tHHz.awp130pgrbfFF.grib2`).
    Rap,
}

impl NomadsModel {
    /// Hours between runs: 6 for GFS (00, 06, 12, 18 UTC), 1 for RAP.
    #[must_use]
    pub fn cycle_hours(self) -> i64 {
        match self {
            Self::Gfs => 6,
            Self::Rap => 1,
        }
    }

    /// Whether a run starting at `cycle_hour_utc` has a forecast `forecast_hour` hours on: GFS's
    /// every hour to 120, then every third hour to 384; RAP's every hour to 21, and to 51 from its
    /// 03, 09, 15 and 21 UTC runs.
    #[must_use]
    pub fn has_forecast_hour(self, cycle_hour_utc: u32, forecast_hour: u32) -> bool {
        match self {
            Self::Gfs => {
                forecast_hour <= 120 || (forecast_hour <= 384 && forecast_hour.is_multiple_of(3))
            }
            Self::Rap => forecast_hour <= 21 || (forecast_hour <= 51 && cycle_hour_utc % 6 == 3),
        }
    }

    /// The pressure levels asked for, hPa, highest pressure first.
    #[must_use]
    pub fn levels_hpa(self) -> &'static [u32] {
        match self {
            Self::Gfs => &GFS_LEVELS_HPA,
            Self::Rap => &RAP_LEVELS_HPA,
        }
    }

    fn script(self) -> &'static str {
        match self {
            Self::Gfs => "filter_gfs_0p25.pl",
            Self::Rap => "filter_rap.pl",
        }
    }
}

/// What to ask NOMADS for: a model's run, a forecast hour and a site.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NomadsRequest {
    /// The site's latitude, degrees north, in `[-90, 90]`.
    pub latitude_deg: f64,
    /// The site's longitude, degrees east, in `[-180, 180]`.
    pub longitude_deg: f64,
    /// The model.
    pub model: NomadsModel,
    /// The run's start (its cycle), seconds since the Unix epoch (UTC): a whole multiple of
    /// [`NomadsModel::cycle_hours`], from 1970 to the year 9999.
    pub cycle_unix_s: i64,
    /// Hours after the cycle, one the run has ([`NomadsModel::has_forecast_hour`]).
    pub forecast_hour: u32,
    /// Another server's address in place of [`ENDPOINT`], such as a mirror; `None` for NOMADS.
    /// It must hold no `?` or `#`.
    pub endpoint: Option<String>,
}

impl NomadsRequest {
    /// A request to NOMADS.
    #[must_use]
    pub fn new(
        latitude_deg: f64,
        longitude_deg: f64,
        model: NomadsModel,
        cycle_unix_s: i64,
        forecast_hour: u32,
    ) -> Self {
        Self {
            latitude_deg,
            longitude_deg,
            model,
            cycle_unix_s,
            forecast_hour,
            endpoint: None,
        }
    }

    /// The time the forecast is for, the cycle plus the forecast hour, s since the Unix epoch.
    #[must_use]
    pub fn valid_unix_s(&self) -> i64 {
        self.cycle_unix_s
            .saturating_add(i64::from(self.forecast_hour) * HOUR_S)
    }

    /// The grib filter's URL for the cut.
    ///
    /// # Errors
    /// [`NomadsError::Request`] when a field is outside the range its doc gives.
    pub fn url(&self) -> Result<String, NomadsError> {
        let refuse = |what: &'static str, value: String| NomadsError::Request { what, value };
        if !(-90.0..=90.0).contains(&self.latitude_deg) {
            return Err(refuse("latitude (deg)", self.latitude_deg.to_string()));
        }
        if !(-180.0..=180.0).contains(&self.longitude_deg) {
            return Err(refuse("longitude (deg)", self.longitude_deg.to_string()));
        }
        let cycle_s = self.model.cycle_hours() * HOUR_S;
        if !(0..YEAR_10000_S).contains(&self.cycle_unix_s) || self.cycle_unix_s % cycle_s != 0 {
            return Err(refuse(
                "cycle (s since 1970)",
                self.cycle_unix_s.to_string(),
            ));
        }
        let (_, _, _, cycle_hour) = date_hour(self.cycle_unix_s);
        // `date_hour` gives an hour of the day, 0 to 23.
        let cycle_hour = u32::try_from(cycle_hour).unwrap_or(0);
        if !self.model.has_forecast_hour(cycle_hour, self.forecast_hour) {
            return Err(refuse("forecast hour", self.forecast_hour.to_string()));
        }
        let endpoint = match &self.endpoint {
            Some(endpoint) if endpoint.is_empty() || endpoint.contains(['?', '#']) => {
                return Err(refuse("endpoint", endpoint.clone()));
            }
            Some(endpoint) => endpoint.as_str(),
            None => ENDPOINT,
        };
        let (year, month, day, hour) = date_hour(self.cycle_unix_s);
        let date = format!("{year:04}{month:02}{day:02}");
        let (dir, file) = match self.model {
            NomadsModel::Gfs => (
                format!("%2Fgfs.{date}%2F{hour:02}%2Fatmos"),
                format!("gfs.t{hour:02}z.pgrb2.0p25.f{:03}", self.forecast_hour),
            ),
            NomadsModel::Rap => (
                format!("%2Frap.{date}"),
                format!("rap.t{hour:02}z.awp130pgrbf{:02}.grib2", self.forecast_hour),
            ),
        };
        let mut url = format!(
            "{endpoint}/{}?dir={dir}&file={file}&var_HGT=on&var_PRES=on&var_RH=on&var_TMP=on\
             &var_UGRD=on&var_VGRD=on&lev_surface=on&lev_2_m_above_ground=on\
             &lev_10_m_above_ground=on",
            self.model.script()
        );
        for p in self.model.levels_hpa() {
            url.push_str(&format!("&lev_{p}_mb=on"));
        }
        let (lat, lon) = (self.latitude_deg, self.longitude_deg);
        url.push_str(&format!(
            "&subregion=&toplat={:.2}&leftlon={:.2}&rightlon={:.2}&bottomlat={:.2}",
            (lat + BOX_DEG).min(90.0),
            lon - BOX_DEG,
            lon + BOX_DEG,
            (lat - BOX_DEG).max(-90.0),
        ));
        Ok(url)
    }

    /// The [`Source`] a [`Client`] caches this request's answer under: NOMADS, [`ATTRIBUTION`],
    /// 30 days.
    #[must_use]
    pub fn source(&self) -> Source {
        Source {
            name: "NOMADS".to_owned(),
            attribution: ATTRIBUTION.to_owned(),
            ttl_s: TTL_S,
        }
    }
}

/// The ground under the forecast, at the site.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NomadsSurface {
    /// The model's terrain height as the cut gives it, gpm.
    pub geopotential_height_m: f64,
    /// Its geometric height above mean sea level, m (WMO-No. 8 eq. 12.16).
    pub height_msl_m: f64,
    /// Surface pressure, Pa.
    pub pressure_pa: f64,
    /// Temperature 2 m above the ground, K.
    pub temperature_k: f64,
    /// Relative humidity 2 m above the ground, a fraction, as recorded (it may pass 1).
    pub relative_humidity: f64,
    /// The 10 m wind's east component, m/s.
    pub wind_east_m_s: f64,
    /// The 10 m wind's north component, m/s.
    pub wind_north_m_s: f64,
}

/// One pressure level above the ground, at the site.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NomadsLevel {
    /// The level's pressure, Pa.
    pub pressure_pa: f64,
    /// Its geopotential height as the cut gives it, gpm.
    pub geopotential_height_m: f64,
    /// Its geometric height above mean sea level, m (WMO-No. 8 eq. 12.16).
    pub height_msl_m: f64,
    /// Temperature, K.
    pub temperature_k: f64,
    /// Relative humidity, a fraction, as recorded (it may pass 1).
    pub relative_humidity: f64,
    /// The wind's east component, m/s.
    pub wind_east_m_s: f64,
    /// The wind's north component, m/s.
    pub wind_north_m_s: f64,
}

/// Why a pressure level was left out of the profile.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropReason {
    /// Its pressure is not below the surface pressure, or its height is not above the ground.
    BelowGround,
    /// A variable is not in the cut at this level, or has no value at a grid point around the
    /// site.
    NoData,
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

/// A grid point the profile is interpolated from.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GridPoint {
    /// Its index in the cut's grid.
    pub index: u64,
    /// Latitude, degrees north.
    pub latitude_deg: f64,
    /// Longitude, degrees east, in `[0, 360)`.
    pub longitude_deg: f64,
    /// Its bilinear weight; the four sum to 1.
    pub weight: f64,
}

/// A NOMADS cut read at a site: the ground and the pressure levels above it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NomadsProfile {
    /// The site's latitude, degrees north.
    pub latitude_deg: f64,
    /// The site's longitude, degrees east.
    pub longitude_deg: f64,
    /// The run's start, s since the Unix epoch (UTC).
    pub cycle_unix_s: i64,
    /// The time the forecast is for, s since the Unix epoch (UTC).
    pub valid_unix_s: i64,
    /// The cut's grid.
    pub grid: Grid,
    /// The four grid points around the site, with their weights.
    pub grid_points: [GridPoint; 4],
    /// The angle the winds were turned by, from the grid's axes to east and north, rad; 0 when
    /// the cut gives them east and north.
    pub wind_turn_rad: f64,
    /// The ground.
    pub surface: NomadsSurface,
    /// The pressure levels above the ground, lowest first.
    pub levels: Vec<NomadsLevel>,
    /// The pressure levels left out, highest pressure first.
    pub dropped: Vec<DroppedLevel>,
}

/// What a field is: its parameter (category, number), the surface type it is on, and the
/// surface's value as bits (so it can key a map).
type Key = ((u8, u8), u8, Option<u64>);

fn key(parameter: (u8, u8), surface: u8, value: Option<f64>) -> Key {
    (parameter, surface, value.map(f64::to_bits))
}

/// The cut's fields by what they are, with the bilinear weights at the site.
struct Cut<'a> {
    fields: HashMap<Key, Field<'a>>,
    corners: [(u64, f64); 4],
}

impl Cut<'_> {
    fn get(&self, parameter: (u8, u8), surface: u8, value: f64) -> Option<&Field<'_>> {
        self.fields.get(&key(parameter, surface, Some(value)))
    }

    /// The field's value at the site, or `None` when a grid point with weight has none.
    fn at(&self, field: &Field<'_>) -> Result<Option<f64>, Grib2Error> {
        let mut sum = 0.0;
        for &(index, weight) in &self.corners {
            if weight == 0.0 {
                continue;
            }
            match field.value(index)? {
                Some(v) => sum += weight * v,
                None => return Ok(None),
            }
        }
        Ok(Some(sum))
    }

    fn value(
        &self,
        parameter: (u8, u8),
        surface: u8,
        value: f64,
    ) -> Result<Option<f64>, Grib2Error> {
        match self.get(parameter, surface, value) {
            Some(field) => self.at(field),
            None => Ok(None),
        }
    }
}

impl NomadsProfile {
    /// Reads a NOMADS cut at a site, interpolating bilinearly between the four grid points around
    /// it.
    ///
    /// # Errors
    /// - [`NomadsError::Grib`] when the body is not GRIB2 the decoder reads.
    /// - [`NomadsError::Mixed`] when its fields differ in grid, cycle or forecast time, and
    ///   [`NomadsError::Duplicate`] when a variable is given twice at one level.
    /// - [`NomadsError::Outside`] when the site is not inside the cut's grid.
    /// - [`NomadsError::NoSurface`] when a ground value is not in the cut or has no value around
    ///   the site, and [`NomadsError::Height`] when a height is beyond what the geopotential
    ///   conversion takes.
    pub fn parse(body: &[u8], latitude_deg: f64, longitude_deg: f64) -> Result<Self, NomadsError> {
        let all: Vec<Field<'_>> = grib2::parse(body)?
            .into_iter()
            .filter(|f| f.discipline == 0)
            .collect();
        if all.len() > MAX_FIELDS {
            return Err(NomadsError::TooManyFields { count: all.len() });
        }
        let Some(first) = all.first() else {
            return Err(NomadsError::NoSurface {
                what: "any meteorological field",
            });
        };
        let grid = first.grid;
        let reference = first.reference_time;
        let lead_s = first
            .product
            .forecast_time_s()
            .ok_or(NomadsError::TimeUnit {
                code: first.product.time_unit,
            })?;
        let mut fields = HashMap::with_capacity(all.len());
        for field in all {
            if field.grid != grid {
                return Err(NomadsError::Mixed { what: "grid" });
            }
            if field.reference_time != reference {
                return Err(NomadsError::Mixed { what: "cycle" });
            }
            if field.product.forecast_time_s() != Some(lead_s) {
                return Err(NomadsError::Mixed {
                    what: "forecast time",
                });
            }
            let p = field.product;
            let k = key((p.category, p.number), p.surface.kind, p.surface.value);
            if fields.insert(k, field).is_some() {
                return Err(NomadsError::Duplicate {
                    category: p.category,
                    number: p.number,
                    surface: p.surface.kind,
                });
            }
        }
        let cycle_unix_s = unix_day_start(
            reference.year.into(),
            reference.month.into(),
            reference.day.into(),
        )
        .ok_or(NomadsError::Mixed { what: "cycle date" })?
            + i64::from(reference.hour) * HOUR_S
            + i64::from(reference.minute) * 60
            + i64::from(reference.second);
        let corners = corners(&grid, latitude_deg, longitude_deg)?;
        let grid_points = corners.map(|(index, weight)| {
            let (lat, lon) = grid.point_deg(index).unwrap_or((f64::NAN, f64::NAN));
            GridPoint {
                index,
                latitude_deg: lat,
                longitude_deg: lon,
                weight,
            }
        });
        // The points' weighted position must be the site: a grid whose numbers are self-consistent
        // but wrong (its projection or its steps) would otherwise weight points far away.
        let unit = |lat: f64, lon: f64| {
            let (lat, lon) = (lat.to_radians(), lon.to_radians());
            [lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()]
        };
        let site = unit(latitude_deg, longitude_deg);
        let mut blend = [0.0; 3];
        for g in &grid_points {
            let u = unit(g.latitude_deg, g.longitude_deg);
            for (b, c) in blend.iter_mut().zip(u) {
                *b += g.weight * c;
            }
        }
        let chord = blend
            .iter()
            .zip(site)
            .map(|(b, s)| (b - s).powi(2))
            .sum::<f64>()
            .sqrt();
        // The mean radius the WMO height conversion uses; the check needs no better.
        // A NaN position (a point off the grid's projection) fails too.
        let near = chord * 6_371_000.0 < PLACE_TOLERANCE_M;
        if !near {
            return Err(NomadsError::Outside {
                latitude_deg,
                longitude_deg,
            });
        }
        let cut = Cut { fields, corners };
        let lat_rad = latitude_deg.to_radians();
        let height = |gpm: f64| {
            geometric_from_wmo_geopotential_m(gpm, lat_rad).map_err(|e| NomadsError::Height {
                geopotential_m: gpm,
                reason: e.to_string(),
            })
        };
        let wind_turn_rad = if grid.winds_grid_relative {
            grid.north_to_grid_y_rad(longitude_deg)
        } else {
            0.0
        };
        let ground = |parameter, surface, value, what| {
            cut.value(parameter, surface, value)?
                .ok_or(NomadsError::NoSurface { what })
        };
        let surface_gpm = ground(HGT, GROUND, 0.0, "terrain height")?;
        let (east, north) = grid.earth_relative_wind(
            ground(UGRD, ABOVE_GROUND, 10.0, "10 m wind, u")?,
            ground(VGRD, ABOVE_GROUND, 10.0, "10 m wind, v")?,
            longitude_deg,
        );
        let surface = NomadsSurface {
            geopotential_height_m: surface_gpm,
            height_msl_m: height(surface_gpm)?,
            pressure_pa: ground(PRES, GROUND, 0.0, "surface pressure")?,
            temperature_k: ground(TMP, ABOVE_GROUND, 2.0, "2 m temperature")?,
            relative_humidity: ground(RH, ABOVE_GROUND, 2.0, "2 m relative humidity")? / 100.0,
            wind_east_m_s: east,
            wind_north_m_s: north,
        };
        // Every isobaric level with a temperature, highest pressure first.
        let mut pressures: Vec<f64> = cut
            .fields
            .values()
            .filter(|f| (f.product.category, f.product.number) == TMP)
            .filter(|f| f.product.surface.kind == ISOBARIC)
            .filter_map(|f| f.product.surface.value)
            .collect();
        pressures.sort_by(|a, b| b.total_cmp(a));
        let mut levels = Vec::new();
        let mut dropped = Vec::new();
        for p in pressures {
            let read = |parameter| cut.value(parameter, ISOBARIC, p);
            let (Some(t), Some(gpm), Some(rh), Some(u), Some(v)) =
                (read(TMP)?, read(HGT)?, read(RH)?, read(UGRD)?, read(VGRD)?)
            else {
                dropped.push(DroppedLevel {
                    pressure_pa: p,
                    reason: DropReason::NoData,
                });
                continue;
            };
            let height_msl_m = height(gpm)?;
            if p >= surface.pressure_pa || height_msl_m <= surface.height_msl_m {
                dropped.push(DroppedLevel {
                    pressure_pa: p,
                    reason: DropReason::BelowGround,
                });
                continue;
            }
            let (east, north) = grid.earth_relative_wind(u, v, longitude_deg);
            levels.push(NomadsLevel {
                pressure_pa: p,
                geopotential_height_m: gpm,
                height_msl_m,
                temperature_k: t,
                relative_humidity: rh / 100.0,
                wind_east_m_s: east,
                wind_north_m_s: north,
            });
        }
        Ok(Self {
            latitude_deg,
            longitude_deg,
            cycle_unix_s,
            valid_unix_s: cycle_unix_s + lead_s,
            grid,
            grid_points,
            wind_turn_rad,
            surface,
            levels,
            dropped,
        })
    }

    /// The profile as an atmosphere with its wind: the ground, then each level above it, with
    /// relative humidity clamped to `[0, 1]`.
    ///
    /// # Errors
    /// What [`SoundingProfile::new`] refuses: heights not increasing, pressures not decreasing
    /// upward, or a value out of range.
    pub fn sounding(
        &self,
        wind_interpolation: WindInterpolation,
    ) -> Result<SoundingProfile, AtmosError> {
        let s = &self.surface;
        let level = |height_msl_m, temperature_k, pressure_pa, rh: f64, east: f64, north: f64| {
            let (speed, from) = speed_direction(east, north);
            SoundingLevel {
                height_msl_m,
                temperature_k,
                pressure_pa: Some(pressure_pa),
                relative_humidity: Some(rh.clamp(0.0, 1.0)),
                wind_speed_m_s: Some(speed),
                wind_direction_from_rad: Some(from),
            }
        };
        let levels = std::iter::once(level(
            s.height_msl_m,
            s.temperature_k,
            s.pressure_pa,
            s.relative_humidity,
            s.wind_east_m_s,
            s.wind_north_m_s,
        ))
        .chain(self.levels.iter().map(|l| {
            level(
                l.height_msl_m,
                l.temperature_k,
                l.pressure_pa,
                l.relative_humidity,
                l.wind_east_m_s,
                l.wind_north_m_s,
            )
        }))
        .collect();
        SoundingProfile::new(levels, self.latitude_deg.to_radians(), wind_interpolation)
    }
}

/// A wind's speed and the direction it blows from, clockwise from north in `[0, 2π)`, from its
/// east and north components.
fn speed_direction(east: f64, north: f64) -> (f64, f64) {
    let from = (-east).atan2(-north).rem_euclid(TAU);
    // `rem_euclid` of a tiny negative angle rounds to 2π itself.
    (east.hypot(north), if from >= TAU { 0.0 } else { from })
}

/// The four grid points around a place, `(i, j)`, `(i+1, j)`, `(i, j+1)`, `(i+1, j+1)`, with their
/// bilinear weights.
fn corners(
    grid: &Grid,
    latitude_deg: f64,
    longitude_deg: f64,
) -> Result<[(u64, f64); 4], NomadsError> {
    let outside = || NomadsError::Outside {
        latitude_deg,
        longitude_deg,
    };
    let (fi, fj) = grid.index_at(latitude_deg, longitude_deg);
    let (ni, nj) = (f64::from(grid.ni), f64::from(grid.nj));
    // The last row or column is a lower corner's `+1`, so a place on it takes the cell before.
    if !(fi >= 0.0 && fi <= ni - 1.0 && fj >= 0.0 && fj <= nj - 1.0) || ni < 2.0 || nj < 2.0 {
        return Err(outside());
    }
    let i0 = fi.floor().min(ni - 2.0);
    let j0 = fj.floor().min(nj - 2.0);
    let (wi, wj) = (fi - i0, fj - j0);
    // In range by the checks above, so the casts are exact.
    let index = |i: f64, j: f64| (i as u64) + u64::from(grid.ni) * (j as u64);
    Ok([
        (index(i0, j0), (1.0 - wi) * (1.0 - wj)),
        (index(i0 + 1.0, j0), wi * (1.0 - wj)),
        (index(i0, j0 + 1.0), (1.0 - wi) * wj),
        (index(i0 + 1.0, j0 + 1.0), wi * wj),
    ])
}

/// Fetches `request` through `client` and reads it at the request's site.
///
/// The answer comes from the client's cache while fresh; offline, from the cache only. Only an
/// answer that decodes, builds a sounding, and is for the cycle and forecast hour asked is cached
/// ([`Client::fetch_checked`]).
///
/// # Errors
/// [`NomadsError::Request`] for a bad request; [`NomadsError::Net`] when the fetch fails, or with
/// [`NetError::Refused`] naming what was refused when the only answer there is doesn't pass.
pub fn fetch<T: Transport>(
    client: &Client<T>,
    request: &NomadsRequest,
    now_s: u64,
) -> Result<(NomadsProfile, Fetched), NomadsError> {
    let url = request.url()?;
    let (lat, lon) = (request.latitude_deg, request.longitude_deg);
    let (cycle, valid) = (request.cycle_unix_s, request.valid_unix_s());
    let model = request.model;
    let check = |body: &[u8]| {
        let profile = NomadsProfile::parse(body, lat, lon).map_err(|e| e.to_string())?;
        let model_grid = match profile.grid.projection {
            Projection::LatLon { .. } => Some(NomadsModel::Gfs),
            Projection::LambertConformal { .. } => Some(NomadsModel::Rap),
            _ => None,
        };
        if model_grid != Some(model) {
            return Err(format!("the cut's grid is not {model:?}'s"));
        }
        if profile.cycle_unix_s != cycle || profile.valid_unix_s != valid {
            return Err(format!(
                "the cut is the run of {} s for {} s, not the run of {cycle} s for {valid} s",
                profile.cycle_unix_s, profile.valid_unix_s
            ));
        }
        profile
            .sounding(WindInterpolation::SpeedDirection)
            .map(drop)
            .map_err(|e| e.to_string())
    };
    let fetched = client.fetch_checked(&request.source(), &url, now_s, check)?;
    let profile = NomadsProfile::parse(&fetched.body, lat, lon)?;
    Ok((profile, fetched))
}

/// Why a NOMADS request or cut was refused.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum NomadsError {
    /// A request field is outside its range.
    #[error("the request's {what} is out of range: {value}")]
    Request {
        /// The field.
        what: &'static str,
        /// Its value.
        value: String,
    },
    /// The body is not GRIB2 the decoder reads.
    #[error(transparent)]
    Grib(#[from] Grib2Error),
    /// The cut's fields differ in something they must share.
    #[error("the cut's fields differ in {what}")]
    Mixed {
        /// What differs.
        what: &'static str,
    },
    /// The first field's forecast time is in a unit other than those
    /// [`hpr_io::grib2::Product::forecast_time_s`] converts.
    #[error("the cut's forecast time is in unit {code} of GRIB2 code table 4.4, which is not read")]
    TimeUnit {
        /// The unit's code.
        code: u8,
    },
    /// The cut holds more than [`MAX_FIELDS`] fields.
    #[error("the cut holds {count} fields, more than {MAX_FIELDS}")]
    TooManyFields {
        /// Its meteorological fields.
        count: usize,
    },
    /// A variable is given twice on one surface.
    #[error("parameter {category}.{number} is given twice on a surface of type {surface}")]
    Duplicate {
        /// Its category (code table 4.1).
        category: u8,
        /// Its number (code table 4.2).
        number: u8,
        /// The surface type (code table 4.5).
        surface: u8,
    },
    /// The site is not inside the cut's grid.
    #[error("the site ({latitude_deg}°, {longitude_deg}°) is not inside the cut's grid")]
    Outside {
        /// The site's latitude, degrees.
        latitude_deg: f64,
        /// The site's longitude, degrees.
        longitude_deg: f64,
    },
    /// A ground value is not in the cut, or has no value around the site.
    #[error("the cut has no {what} at the site")]
    NoSurface {
        /// What is missing.
        what: &'static str,
    },
    /// A height is beyond what the geopotential conversion takes.
    #[error("a geopotential height of {geopotential_m} gpm: {reason}")]
    Height {
        /// The height, gpm.
        geopotential_m: f64,
        /// Why it was refused.
        reason: String,
    },
    /// The fetch failed, or its answer was refused.
    #[error(transparent)]
    Net(#[from] NetError),
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
    )]

    use super::*;

    /// A humidity past saturation is kept in the profile as recorded and clamped in the sounding.
    #[test]
    fn humidity_past_saturation_is_clamped_in_the_sounding() {
        let body = include_bytes!("../tests/fixtures/replay/nomads-gfs.grib2");
        let mut profile = NomadsProfile::parse(body, 32.99, -106.97).unwrap();
        profile.surface.relative_humidity = 1.04;
        profile.levels[3].relative_humidity = 1.2;
        let air = profile.sounding(WindInterpolation::SpeedDirection).unwrap();
        let levels = air.levels();
        assert_eq!(levels[0].relative_humidity, Some(1.0));
        assert_eq!(levels[4].relative_humidity, Some(1.0));
        assert_eq!(
            levels[5].relative_humidity,
            Some(profile.levels[4].relative_humidity)
        );
        assert_eq!(profile.levels[3].relative_humidity, 1.2);
    }
}
