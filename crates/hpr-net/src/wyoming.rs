//! Weather-balloon soundings from the University of Wyoming's radiosonde archive, turned into a
//! [`SoundingProfile`].
//!
//! A radiosonde is an instrument package carried up by a weather balloon, usually at 00 and
//! 12 UTC, from hundreds of stations worldwide. It measures pressure, temperature and humidity,
//! and its drift gives the wind. The [University of Wyoming][uwyo] serves the archive of these
//! soundings. It is a measurement, not a forecast, but only at the station and the time of the
//! flight, which can be a hundred kilometres and some hours from a launch.
//!
//! A [`WyomingRequest`] names a station (its WMO number, such as `72364` for Santa Teresa, New
//! Mexico) and the sounding's nominal hour, and asks for the comma-separated text, one row per
//! level, with these columns (among others) in these units:
//!
//! | column | unit | read as |
//! |---|---|---|
//! | `time` | `YYYY-MM-DD HH:MM:SS` UTC | the release time, from the first row |
//! | `latitude`, `longitude` | degrees | the release point, from the first row |
//! | `pressure_hPa` | hPa | pressure |
//! | `geopotential height_m` | geopotential m | height above sea level |
//! | `temperature_C` | °C | temperature |
//! | `relative humidity_%` | % | over liquid water (the file has `humidity wrt ice_%` too) |
//! | `wind direction_degree`, `wind speed_m/s` | °, m/s | the wind, the direction it blows from |
//!
//! A column in any other unit is refused, not converted. Two versions of most soundings are
//! served ([`WyomingVersion`]): the coded message stations send (WMO FM 35, "TEMP"), with the
//! standard pressure levels and the significant levels between them, about 200 rows; and the
//! BUFR file (WMO's binary format, as the archive decodes it), a row a second, about 6,000.
//!
//! [`WyomingSounding::parse`] first checks each row against the last row that fit: a row with a
//! pressure, height and temperature **fits** when its height above that row is the layer's
//! thickness by the hypsometric equation (from the two pressures and virtual temperatures, WMO-No.
//! 8 eqs. 12.17 and 12.18), within 5% ([`THICKNESS_SHARE`]) plus what rounding the pressures can
//! move it plus 30 m ([`THICKNESS_SLACK_M`]). The rows that fit form a chain from the ground; a row
//! missing only its wind or humidity still joins it, so a gap in the wind doesn't widen the layer
//! checked. A row that fits is still the odd one out, and doesn't join, if the next row misses it
//! but fits the chain's end better than it does. A row that misses the end but agrees with the
//! next row (which misses the end too), and fits one of the 10 chain rows before the end better than the end does, joins in place of the
//! rows after that one, which are the odd ones out. A row that doesn't join, or is taken out, is
//! left out ([`DropReason::Thickness`]); in the recordings every row fits, to within 1 m beyond
//! rounding. Then it keeps:
//!
//! - **The ground**, the first row: the pressure, temperature, humidity and wind at the station
//!   when the balloon was released. Nothing before it checks it, so three rows after it that miss
//!   it while fitting each other refuse the answer ([`WyomingError::GroundMisfit`]).
//! - **One row of each run of rows that fit with the same pressure**, the middle one. BUFR's
//!   pressures are rounded to 0.1 hPa, and high up the balloon climbs tens of metres while the
//!   pressure falls that much, so runs of rows share a pressure; the rounded value is the pressure
//!   at about the middle of its run.
//! - **Each such row that lies above the last row kept**, higher and at a lower pressure.
//!
//! A row missing a value (the last row often has no wind) is dropped, and so is one with a value
//! no air on Earth has ([`DropReason::OutOfRange`] lists the bounds). [`WyomingSounding::dropped`]
//! lists each row left out, with its reason.
//!
//! So a row with a bad pressure or height, a pressure missing a digit, say, is left out, not kept
//! to hide the good rows after it; rows that fall or stay at one height, a balloon coming down,
//! fit and are left out however many. More than [`MAX_MISFITS`] consecutive rows that don't join
//! the chain refuse the answer ([`WyomingError::Misfit`]): the chain's end, or all of them, are
//! wrong (a block of heights 1 km off), or a long run of rows with no temperature leaves a layer
//! too thick for its two ends' temperatures to give. Not caught: a wrong wind, humidity or
//! temperature (the check doesn't use the wind, humidity moves it by a few percent, and on layers
//! under about 100 m any temperature within the bounds fits), a height error within the
//! allowance, a row whose pressure and height are both wrong yet fit each other, and a bad ground
//! with fewer than three rows after it. A row kept a little too high leaves out the good rows just
//! above it, which now lie below it: in the tests, at most 2 other levels of a coded message and
//! 7 other rows of a BUFR file.
//!
//! Heights are geopotential metres (the column says so), converted to geometric heights at the
//! first row's latitude with WMO-No. 8 (2023) eqs. 12.15 and 12.16
//! ([`hpr_atmos::profile::geometric_from_wmo_geopotential_m`]); the [atmosphere page][atmos]
//! explains why. That is the latitude the profile uses for its hydrostatics. The balloon drifts;
//! converting at the latitude it reached instead would move a height by about 0.8 m per degree
//! of drift at 10 km, 2.5 m at 30 km. A relative humidity above 100%, which radiosondes report in
//! cloud, is kept as recorded and taken as 100% in [`WyomingSounding::sounding`], as the
//! [atmosphere's decision record][adr-004] asks.
//!
//! [`fetch`] asks a [`Client`] for the URL, so the answer comes from the cache when it can, and
//! offline from the cache only; an answer that doesn't parse is never cached. Show
//! [`ATTRIBUTION`] (it is on every [`Fetched`]) wherever the sounding is shown.
//!
//! **How far to trust it:** the profile gives back every row it keeps as recorded (the tests). A
//! radiosonde's own errors are small next to how far the air can change between the station and
//! the launch, and nothing here measures that. The [guide page][guide] says more.
//!
//! ```
//! use hpr_atmos::WindInterpolation;
//! use hpr_net::wyoming::WyomingSounding;
//!
//! // Santa Teresa, New Mexico, 21 June 2025, 12 UTC: the coded message's 228 rows.
//! let body = include_bytes!("../tests/fixtures/replay/wyoming-72364-fm35.csv");
//! let sounding = WyomingSounding::parse(body)?;
//! assert_eq!(sounding.levels.len(), 227); // the last row has no wind
//! let air = sounding.sounding(WindInterpolation::SpeedDirection)?;
//! let at_5_km = air.sample(5_000.0)?.air;
//! // Between the rows at 570 hPa (4,852 m) and 557 hPa (5,035 m) of geopotential height.
//! assert!((at_5_km.pressure_pa - 55_950.0).abs() < 100.0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [uwyo]: https://weather.uwyo.edu/upperair/sounding.shtml
//! [adr-004]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-004-atmosphere-wind-turbulence-and-the-seeded-generator-2026-09-17
//! [atmos]: https://nrdptel.github.io/hpr-sim/physics/atmosphere.html
//! [guide]: https://nrdptel.github.io/hpr-sim/soundings.html

use std::f64::consts::TAU;

use hpr_atmos::moist::{WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL, saturation_vapour_pressure_pa};
use hpr_atmos::profile::geometric_from_wmo_geopotential_m;
use hpr_atmos::ussa76::{DRY_AIR_GAS_CONSTANT_J_PER_KG_K, SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL};
use hpr_atmos::{AtmosError, SoundingLevel, SoundingProfile, WindInterpolation};
use hpr_core::gravity::STANDARD_GRAVITY_MPS2;
use serde::{Deserialize, Serialize};

use crate::civil::{date_hour, unix_day_start};
use crate::{Client, Fetched, NetError, Source, Transport};

/// The archive's address.
pub const ENDPOINT: &str = "https://weather.uwyo.edu/wsgi/sounding";

/// The credit shown wherever a sounding is shown. The archive states no licence; the soundings
/// are the stations' observations, which weather services exchange freely (WMO Resolution 40).
pub const ATTRIBUTION: &str = "Sounding from the University of Wyoming's radiosonde archive";

/// How long after its nominal hour a sounding may still be filling in, s: a day. The archive's
/// copy can grow for some hours after the flight, as a station's later messages arrive.
pub const SETTLE_S: u64 = 86_400;

/// How long an answer stays fresh while its sounding may still be filling in, s: an hour.
pub const YOUNG_TTL_S: u64 = 3_600;

/// How long an answer fetched after its sounding settled stays fresh, s: 30 days.
pub const SETTLED_TTL_S: u64 = 30 * 86_400;

/// The most consecutive rows that may fail to join the chain of rows that fit. More means that row, or all of
/// them, are wrong (a block of heights 1 km off), and the answer is refused. In a BUFR file 10
/// rows are about 10 s of the balloon's climb, some 50 m; in a coded message they can span
/// kilometres.
pub const MAX_MISFITS: usize = 10;

/// The share of a layer's hypsometric thickness a row's height may miss it by, beyond rounding
/// and [`THICKNESS_SLACK_M`]: 5%, for a layer whose inner rows have no temperature, which the mean
/// of its two ends' temperatures gives less well. A judgment, not a measurement: no row in the
/// recordings needs any share.
pub const THICKNESS_SHARE: f64 = 0.05;

/// The metres a row's height may miss its layer's thickness by, beyond the share and the pressures'
/// rounding: 30 m. A coded message's heights from 500 hPa up are rounded to 10 m; a height 30 m
/// off misplaces its level by as much as a pressure error of 0.33% to 0.55% (the air's scale
/// height, the climb over which pressure falls by a factor of e, is 5.5 to 9 km), so the check is
/// for gross errors, not for these.
pub const THICKNESS_SLACK_M: f64 = 30.0;

/// The most rows an answer may have. The archive's BUFR files have about 6,000.
pub const MAX_ROWS: usize = 100_000;

/// Bounds outside which a value is impossible on Earth, and its row is dropped as out of range:
/// pressure (the highest sea-level pressure recorded is about 1,084 hPa; 0.1 hPa is about 65 km up
/// in the 1976 standard atmosphere, above the height bound), temperature, geopotential height (the
/// Dead Sea's shore is at about −430 m) and wind speed.
const MIN_PRESSURE_HPA: f64 = 0.1;
const MAX_PRESSURE_HPA: f64 = 1_200.0;
const MIN_TEMPERATURE_C: f64 = -150.0;
const MAX_TEMPERATURE_C: f64 = 80.0;
const MIN_HEIGHT_M: f64 = -1_000.0;
const MAX_HEIGHT_M: f64 = 60_000.0;
const MAX_WIND_M_S: f64 = 300.0;

/// The most columns a header may have. The archive's has 13.
const MAX_COLUMNS: usize = 64;

/// The columns read, with the units the parser requires.
const COLUMNS: [(Column, &str, &str); 9] = [
    (Column::Time, "time", ""),
    (Column::Longitude, "longitude", ""),
    (Column::Latitude, "latitude", ""),
    (Column::Pressure, "pressure", "hPa"),
    (Column::Height, "geopotential height", "m"),
    (Column::Temperature, "temperature", "C"),
    (Column::Humidity, "relative humidity", "%"),
    (Column::Direction, "wind direction", "degree"),
    (Column::Speed, "wind speed", "m/s"),
];

/// Seconds in an hour.
const HOUR_S: i64 = 3_600;
/// The first second of the year 10000, past which a date has no four-digit year.
const YEAR_10000_S: i64 = 253_402_300_800;
/// The longest station name or quoted text an error repeats, in characters.
const QUOTE_CHARS: usize = 40;

/// Which version of a sounding to ask for.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WyomingVersion {
    /// The coded message the station sends (WMO FM 35, "TEMP"): the standard pressure levels and
    /// the significant levels between them, about 200 rows, pressures to 1 hPa (0.1 hPa above
    /// 100 hPa).
    Fm35,
    /// The BUFR file, where the station sends one: a row a second, about 6,000, pressures to
    /// 0.1 hPa.
    Bufr,
}

impl WyomingVersion {
    /// The URL's name for it.
    fn query(self) -> &'static str {
        match self {
            Self::Fm35 => "FM35",
            Self::Bufr => "BUFR",
        }
    }
}

/// What to ask the archive for: a station, the sounding's nominal hour and its version.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WyomingRequest {
    /// The station's WMO number (such as `72364`), or the name the archive knows it by.
    pub station: String,
    /// The sounding's nominal time, a whole UTC hour (usually 00 or 12), seconds since the Unix
    /// epoch. The balloon is released about an hour before.
    pub time_unix_s: i64,
    /// Which version to ask for.
    pub version: WyomingVersion,
    /// Another server with the same interface, instead of [`ENDPOINT`]: an address with no query
    /// (`?`) or fragment (`#`), to which the request's own query is added.
    pub endpoint: Option<String>,
}

impl WyomingRequest {
    /// A request for `station`'s coded-message sounding at the whole hour `time_unix_s`.
    #[must_use]
    pub fn new(station: impl Into<String>, time_unix_s: i64) -> Self {
        Self {
            station: station.into(),
            time_unix_s,
            version: WyomingVersion::Fm35,
            endpoint: None,
        }
    }

    /// A request for the latest 00 or 12 UTC sounding at or before `launch_unix_s` (seconds since
    /// the Unix epoch): the one a launch at that time would have had. Near its nominal hour a
    /// sounding may still be filling in; [`WyomingRequest::source`] keeps such an answer fresh
    /// for an hour only.
    #[must_use]
    pub fn latest_before(station: impl Into<String>, launch_unix_s: i64) -> Self {
        let half_day_s = 12 * HOUR_S;
        let time_unix_s = launch_unix_s
            .div_euclid(half_day_s)
            .saturating_mul(half_day_s);
        Self::new(station, time_unix_s)
    }

    /// The URL: `…?datetime=YYYY-MM-DD%20HH:00:00&id=<station>&type=TEXT:CSV&src=<version>`.
    ///
    /// # Errors
    /// [`WyomingError::Request`] when the station is empty, longer than 16 characters or not
    /// letters and digits, the time is not a whole hour from 1970 to 9999, or the endpoint is
    /// empty or has a `?` or `#`.
    pub fn url(&self) -> Result<String, WyomingError> {
        let refuse = |what, value: &str| WyomingError::Request {
            what,
            value: cut(value),
        };
        let station = &self.station;
        if station.is_empty()
            || station.len() > 16
            || !station.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return Err(refuse("station", station));
        }
        if !(0..YEAR_10000_S).contains(&self.time_unix_s) || self.time_unix_s % HOUR_S != 0 {
            return Err(refuse("time (s since 1970)", &self.time_unix_s.to_string()));
        }
        let endpoint = match &self.endpoint {
            Some(endpoint) if endpoint.is_empty() || endpoint.contains(['?', '#']) => {
                return Err(refuse("endpoint", endpoint));
            }
            Some(endpoint) => endpoint.as_str(),
            None => ENDPOINT,
        };
        let (year, month, day, hour) = date_hour(self.time_unix_s);
        Ok(format!(
            "{endpoint}?datetime={year:04}-{month:02}-{day:02}%20{hour:02}:00:00&id={station}\
             &type=TEXT:CSV&src={}",
            self.version.query()
        ))
    }

    /// The cache's view of the source at `now_s` (seconds since the Unix epoch): the archive's
    /// name, [`ATTRIBUTION`], and how long a cached answer stays fresh.
    ///
    /// Until [`SETTLE_S`] after the nominal hour the sounding may still be filling in, so an
    /// answer stays fresh for [`YOUNG_TTL_S`]. After that, only an answer fetched after the
    /// sounding settled is fresh, for up to [`SETTLED_TTL_S`]: a copy fetched while it was young
    /// is fetched again online (and still served offline, marked stale).
    #[must_use]
    pub fn source(&self, now_s: u64) -> Source {
        let settled_s = u64::try_from(self.time_unix_s)
            .unwrap_or(0)
            .saturating_add(SETTLE_S);
        let ttl_s = if now_s < settled_s {
            YOUNG_TTL_S
        } else {
            // Fresh when `now − fetched < now − settled`, that is, fetched after it settled.
            (now_s - settled_s).min(SETTLED_TTL_S)
        };
        Source {
            name: "University of Wyoming soundings".to_owned(),
            attribution: ATTRIBUTION.to_owned(),
            ttl_s,
        }
    }
}

/// One level of a sounding, as recorded, in SI units.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WyomingLevel {
    /// Pressure, Pa.
    pub pressure_pa: f64,
    /// Geopotential height above sea level as recorded, geopotential m.
    pub geopotential_height_m: f64,
    /// Geometric height above sea level, m (WMO-No. 8 eqs. 12.15 and 12.16 at the first row's
    /// latitude).
    pub height_msl_m: f64,
    /// Temperature, K.
    pub temperature_k: f64,
    /// Relative humidity over liquid water as recorded, a fraction; it can exceed 1 in cloud.
    pub relative_humidity: f64,
    /// Wind speed, m/s.
    pub wind_speed_m_s: f64,
    /// Direction the wind blows from, clockwise from true north, rad, in `[0, 2π)`.
    pub wind_direction_from_rad: f64,
}

/// Why a row was left out of the profile.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropReason {
    /// A value is missing: the pressure, height, temperature, humidity, or either half of the
    /// wind.
    NoData,
    /// A value is out of range: a pressure below 0.1 hPa or above 1,200 hPa, a temperature
    /// outside −150 to 80 °C, a height outside −1 to 60 km, a wind speed below zero or above
    /// 300 m/s, a relative humidity below zero, a direction outside 0° to 360°, or a height
    /// with no geometric height.
    OutOfRange,
    /// It is not the middle row of its run of rows that fit with the same pressure, or the run is
    /// at the ground's pressure.
    SamePressure,
    /// Its height is not above the last row kept, or its pressure not below it.
    NotAbove,
    /// Its height above the last row that fit is not the thickness the two rows' pressures and
    /// temperatures give (see [`THICKNESS_SHARE`]), or the rows around it fit each other better
    /// than they fit it: its pressure, height or temperature is likely wrong. Rarely, a good row
    /// beside a bad one that only just fits is left out in its place.
    Thickness,
}

/// A row left out of the profile, and why.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DroppedLevel {
    /// Its line in the answer, counting the header as line 1.
    pub line: usize,
    /// Why it was dropped.
    pub reason: DropReason,
}

/// A sounding read from the archive's answer: the ground and the levels above it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WyomingSounding {
    /// Where the balloon was released, degrees north, from the first row: the station, or the
    /// sonde's own position at release in a BUFR file.
    pub latitude_deg: f64,
    /// Where the balloon was released, degrees east, from the first row.
    pub longitude_deg: f64,
    /// When the balloon was released, seconds since the Unix epoch (UTC), from the first row.
    pub release_unix_s: i64,
    /// The levels kept, the ground first.
    pub levels: Vec<WyomingLevel>,
    /// The rows left out, in order.
    pub dropped: Vec<DroppedLevel>,
}

impl WyomingSounding {
    /// Reads the archive's comma-separated answer.
    ///
    /// # Errors
    /// - [`WyomingError::Missing`] when a column is absent (an answer that is not the archive's
    ///   text has no header to find), and [`WyomingError::Units`] when one is in another unit.
    /// - [`WyomingError::Row`] when the header has more than 64 columns or a column twice, there
    ///   are more than [`MAX_ROWS`] rows, a row has the wrong number of fields, or a field is not
    ///   a number (or, in the first row, the time is not a date).
    /// - [`WyomingError::NoGround`] when there is no row, or the first is missing a value or has
    ///   one out of range.
    /// - [`WyomingError::GroundMisfit`] when three rows after the ground miss it while fitting
    ///   each other.
    /// - [`WyomingError::Misfit`] when more than [`MAX_MISFITS`] consecutive rows fail to join
    ///   the chain of rows that fit.
    pub fn parse(body: &[u8]) -> Result<Self, WyomingError> {
        let text = std::str::from_utf8(body).map_err(|_| WyomingError::Missing {
            field: "header".to_owned(),
        })?;
        let mut lines = text.lines().enumerate();
        let header = lines
            .next()
            .map(|(_, line)| line)
            .ok_or_else(|| WyomingError::Missing {
                field: "header".to_owned(),
            })?;
        let names: Vec<&str> = header.splitn(MAX_COLUMNS + 1, ',').map(str::trim).collect();
        if names.len() > MAX_COLUMNS {
            return Err(WyomingError::Row {
                line: 1,
                reason: format!("more than {MAX_COLUMNS} columns"),
            });
        }
        let mut index = [0_usize; COLUMNS.len()];
        for &(column, name, unit) in &COLUMNS {
            index[column as usize] = find_column(&names, name, unit)?;
        }
        let col = |column: Column| index[column as usize];

        let mut rows = lines.filter(|(_, line)| !line.trim().is_empty()).peekable();
        let Some(&(first, first_line)) = rows.peek() else {
            return Err(WyomingError::NoGround { line: 2 });
        };
        let fields = split_row(first_line, names.len(), first + 1)?;
        let position = |column, what| number(&fields, col(column), first + 1, what);
        let (Some(latitude_deg), Some(longitude_deg)) = (
            position(Column::Latitude, "latitude")?,
            position(Column::Longitude, "longitude")?,
        ) else {
            return Err(WyomingError::NoGround { line: first + 1 });
        };
        let time = fields[col(Column::Time)];
        let release_unix_s = parse_time(time).ok_or_else(|| WyomingError::Row {
            line: first + 1,
            reason: format!("the time {:?} is not a date", cut(time)),
        })?;
        let latitude_rad = latitude_deg.to_radians();

        // Every row read: its line, its pressure, height and temperature when it has them, and
        // its level, or why it has none.
        let mut read: Vec<(usize, Option<Thermo>, Result<WyomingLevel, DropReason>)> = Vec::new();
        for (i, line) in rows {
            let line_no = i + 1;
            if read.len() == MAX_ROWS {
                return Err(WyomingError::Row {
                    line: line_no,
                    reason: format!("more than {MAX_ROWS} rows"),
                });
            }
            let fields = split_row(line, names.len(), line_no)?;
            let value = |column: Column, what: &str| number(&fields, col(column), line_no, what);
            let values = (
                value(Column::Pressure, "pressure")?,
                value(Column::Height, "height")?,
                value(Column::Temperature, "temperature")?,
                value(Column::Humidity, "humidity")?,
                value(Column::Speed, "wind speed")?,
                value(Column::Direction, "wind direction")?,
            );
            let level = match values {
                (Some(p), Some(z), Some(t), Some(rh), Some(speed), Some(direction)) => {
                    level(p, z, t, rh, speed, direction, latitude_rad)
                }
                _ => Err(DropReason::NoData),
            };
            if read.is_empty() && level.is_err() {
                return Err(WyomingError::NoGround { line: line_no });
            }
            let thermo = match values {
                (Some(p), Some(z), Some(t), rh, _, _) => thermo(p, z, t, rh),
                _ => None,
            };
            read.push((line_no, thermo, level));
        }
        let Some(&(ground_line, Some(_), Ok(ground))) = read.first() else {
            return Err(WyomingError::NoGround { line: first + 1 });
        };
        // An answer whose pressures of 100 hPa or more are all whole numbers is a coded message's,
        // rounded there to 1 hPa; the rest are to 0.1 hPa.
        let coarse = read.iter().all(|(_, thermo, _)| {
            thermo
                .is_none_or(|t| t.pressure_pa < 10_000.0 || (t.pressure_pa / 100.0).fract() == 0.0)
        });

        // Which rows fit: each row with a pressure, height and temperature is checked against the
        // chain's end, the last row that fit (the ground at first).
        let with_thermo: Vec<(usize, Thermo)> = read
            .iter()
            .enumerate()
            .filter_map(|(i, (_, thermo, _))| thermo.map(|t| (i, t)))
            .collect();
        let mut fit = vec![false; read.len()];
        let mut chain = vec![with_thermo[0]];
        // The rows since the chain's end that didn't join it: the first's line and how many.
        let mut misfits: Option<(usize, usize)> = None;
        for (k, &(i, thermo)) in with_thermo.iter().enumerate().skip(1) {
            let next = |ahead: usize| with_thermo.get(k + ahead).map(|&(_, t)| t);
            let end = chain[chain.len() - 1].1;
            let joins = if fits(&end, &thermo, coarse) {
                // It fits, but the next row fits the end and not it, and the end better than it
                // does: it is the odd one out.
                !next(1).is_some_and(|n| {
                    !fits(&thermo, &n, coarse)
                        && miss_share(&end, &n, coarse) < miss_share(&end, &thermo, coarse)
                })
            } else if next(1).is_some_and(|n| fits(&thermo, &n, coarse) && !fits(&end, &n, coarse))
            {
                // It misses the end, and the next row agrees with it: the end, and the rows of
                // the chain back to one it fits better than the end does, are the odd ones out.
                let back = chain.len().saturating_sub(MAX_MISFITS + 1);
                if let Some(j) = (back..chain.len() - 1).rev().find(|&j| {
                    let share = miss_share(&chain[j].1, &thermo, coarse);
                    share <= 1.0 && share < miss_share(&chain[j].1, &end, coarse)
                }) {
                    for (dropped, _) in chain.drain(j + 1..) {
                        fit[dropped] = false;
                    }
                    true
                } else if chain.len() == 1
                    && next(2).is_some_and(|n2| {
                        next(1).is_some_and(|n1| {
                            fits(&n1, &n2, coarse)
                                && !fits(&end, &n1, coarse)
                                && !fits(&end, &n2, coarse)
                        })
                    })
                {
                    // Nothing before the ground checks it: three rows that miss it while fitting
                    // each other say it is the odd one out, and it can't be left out.
                    return Err(WyomingError::GroundMisfit {
                        line: ground_line,
                        first: read[i].0,
                    });
                } else {
                    false
                }
            } else {
                false
            };
            if joins {
                chain.push((i, thermo));
                fit[i] = true;
                misfits = None;
            } else {
                let (first, count) = misfits.get_or_insert((read[i].0, 0));
                *count += 1;
                if *count > MAX_MISFITS {
                    return Err(WyomingError::Misfit {
                        after: read[chain[chain.len() - 1].0].0,
                        line: *first,
                    });
                }
            }
        }
        fit[0] = true;

        // Of each run of complete rows that fit with the same pressure, all but the middle one;
        // a run at the ground's pressure, all of it. Other rows between them don't end a run.
        let complete = |i: usize| match read[i].2 {
            Ok(level) if fit[i] => Some(level),
            _ => None,
        };
        let mut same_pressure = vec![false; read.len()];
        let mut start = 1;
        while start < read.len() {
            let Some(first) = complete(start) else {
                start += 1;
                continue;
            };
            let mut members = vec![start];
            let mut next = start + 1;
            while next < read.len() {
                match complete(next) {
                    Some(level) if level.pressure_pa == first.pressure_pa => members.push(next),
                    Some(_) => break,
                    None => {}
                }
                next += 1;
            }
            let middle = (members.len() - 1) / 2;
            let at_ground = first.pressure_pa == ground.pressure_pa;
            for (k, &i) in members.iter().enumerate() {
                same_pressure[i] = k != middle || at_ground;
            }
            start = members[members.len() - 1] + 1;
        }

        let mut levels = vec![ground];
        let mut dropped = Vec::new();
        for (i, &(line, thermo, level)) in read.iter().enumerate().skip(1) {
            let reason = match level {
                _ if thermo.is_some() && !fit[i] => DropReason::Thickness,
                Err(reason) => reason,
                Ok(_) if same_pressure[i] => DropReason::SamePressure,
                Ok(level) => {
                    let under = levels[levels.len() - 1];
                    if level.height_msl_m > under.height_msl_m
                        && level.pressure_pa < under.pressure_pa
                    {
                        levels.push(level);
                        continue;
                    }
                    DropReason::NotAbove
                }
            };
            dropped.push(DroppedLevel { line, reason });
        }
        Ok(Self {
            latitude_deg,
            longitude_deg,
            release_unix_s,
            levels,
            dropped,
        })
    }

    /// The sounding as an atmosphere with its wind: every level kept, each with its pressure,
    /// temperature, relative humidity (above 100% taken as 100%) and wind.
    ///
    /// # Errors
    /// What [`SoundingProfile::new`] refuses. [`WyomingSounding::parse`] keeps only levels within
    /// physical bounds, and [`fetch`] checks the profile too, so an answer that fails here is
    /// never cached.
    pub fn sounding(
        &self,
        wind_interpolation: WindInterpolation,
    ) -> Result<SoundingProfile, AtmosError> {
        let levels = self
            .levels
            .iter()
            .map(|l| SoundingLevel {
                height_msl_m: l.height_msl_m,
                temperature_k: l.temperature_k,
                pressure_pa: Some(l.pressure_pa),
                relative_humidity: Some(l.relative_humidity.min(1.0)),
                wind_speed_m_s: Some(l.wind_speed_m_s),
                wind_direction_from_rad: Some(l.wind_direction_from_rad),
            })
            .collect();
        SoundingProfile::new(levels, self.latitude_deg.to_radians(), wind_interpolation)
    }
}

/// Fetches `request` through `client` and reads it.
///
/// The answer comes from the client's cache while fresh ([`WyomingRequest::source`] at `now_s`,
/// seconds since the Unix epoch); offline, from the cache only. The [`Fetched`] says which,
/// carries [`ATTRIBUTION`] and holds the body. Only an answer that parses and makes a sounding is
/// cached ([`Client::fetch_checked`]): one that doesn't never takes a good copy's place, and
/// online a stale good copy is returned instead, with the reason. A sounding the archive doesn't
/// have is an HTTP error (404; 400 for a BUFR file a station doesn't send), which the transport
/// reports.
///
/// # Errors
/// [`WyomingError::Request`] for a bad request; [`WyomingError::Net`] when the fetch fails, or
/// with [`NetError::Refused`] naming what [`WyomingSounding::parse`] or
/// [`WyomingSounding::sounding`] refused when the only answer there is doesn't pass them.
pub fn fetch<T: Transport>(
    client: &Client<T>,
    request: &WyomingRequest,
    now_s: u64,
) -> Result<(WyomingSounding, Fetched), WyomingError> {
    let url = request.url()?;
    let check = |body: &[u8]| {
        // A sounding the profile would refuse is refused here too, so it is never cached.
        let sounding = WyomingSounding::parse(body).map_err(|e| e.to_string())?;
        sounding
            .sounding(WindInterpolation::SpeedDirection)
            .map(drop)
            .map_err(|e| e.to_string())
    };
    let fetched = client.fetch_checked(&request.source(now_s), &url, now_s, check)?;
    let sounding = WyomingSounding::parse(&fetched.body)?;
    Ok((sounding, fetched))
}

/// Why a Wyoming request or answer was refused.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum WyomingError {
    /// A request field is outside its range.
    #[error("the request's {what} is out of range: {value}")]
    Request {
        /// The field.
        what: &'static str,
        /// Its value, cut to 40 characters.
        value: String,
    },
    /// The fetch failed.
    #[error(transparent)]
    Net(#[from] NetError),
    /// A column is absent.
    #[error("the Wyoming answer has no {field} column")]
    Missing {
        /// The column, or `header` when there is no header line.
        field: String,
    },
    /// A column is in a unit other than the one required.
    #[error("the Wyoming answer gives {field} in {found:?}, not {expected:?}")]
    Units {
        /// The column.
        field: &'static str,
        /// The unit it came in, cut to 40 characters.
        found: String,
        /// The unit required.
        expected: &'static str,
    },
    /// A line can't be read.
    #[error("line {line} of the Wyoming answer: {reason}")]
    Row {
        /// The line, counting the header as line 1.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// There is no ground: no row, or a first row missing a value or with one out of range.
    #[error("the Wyoming answer has no usable ground level (line {line})")]
    NoGround {
        /// The line of the first row, counting the header as line 1.
        line: usize,
    },
    /// Three rows after the ground miss it while fitting each other: the ground is taken as wrong,
    /// and it can't be left out. Three bad rows right after a good ground that agree with each
    /// other look the same.
    #[error(
        "the Wyoming answer's ground (line {line}) doesn't fit the rows after it, which fit each \
         other (from line {first})"
    )]
    GroundMisfit {
        /// The ground's line, counting the header as line 1.
        line: usize,
        /// The first of the three rows.
        first: usize,
    },
    /// More than [`MAX_MISFITS`] consecutive rows fail to join the chain of rows that fit: its
    /// end, or all of them, are wrong, or rows with no temperature leave a layer too thick to check.
    #[error(
        "the Wyoming answer's line {after} and the more than {MAX_MISFITS} rows from line {line} \
         on don't fit each other: the hypsometric thickness between them is wrong"
    )]
    Misfit {
        /// The last row that fit, counting the header as line 1.
        after: usize,
        /// The first row after it that doesn't fit it.
        line: usize,
    },
}

/// A row's pressure, geopotential height and virtual temperature: what the thickness check needs.
#[derive(Clone, Copy)]
struct Thermo {
    pressure_pa: f64,
    geopotential_height_m: f64,
    virtual_temperature_k: f64,
}

/// A row's [`Thermo`], when its pressure, height and temperature are within the bounds; with no
/// humidity, or one below zero, the air is taken as dry.
fn thermo(
    pressure_hpa: f64,
    geopotential_height_m: f64,
    temperature_c: f64,
    humidity_pct: Option<f64>,
) -> Option<Thermo> {
    if !(MIN_PRESSURE_HPA..=MAX_PRESSURE_HPA).contains(&pressure_hpa)
        || !(MIN_HEIGHT_M..=MAX_HEIGHT_M).contains(&geopotential_height_m)
        || !(MIN_TEMPERATURE_C..=MAX_TEMPERATURE_C).contains(&temperature_c)
    {
        return None;
    }
    let pressure_pa = pressure_hpa * 100.0;
    let humidity = humidity_pct.filter(|h| *h >= 0.0).unwrap_or(0.0) / 100.0;
    Some(Thermo {
        pressure_pa,
        geopotential_height_m,
        virtual_temperature_k: virtual_temperature_k(temperature_c + 273.15, humidity, pressure_pa),
    })
}

/// Whether `upper`'s geopotential height above `lower` (below it, if negative) is the thickness
/// the hypsometric equation gives the layer between their pressures, with the layer's mean virtual
/// temperature `T̄_v` (WMO-No. 8 (2023), Vol. I, eqs. 12.17 and 12.18):
///
/// ```text
/// ΔZ = (R_d T̄_v / g₀) ln(p_bottom / p_top)
/// ```
///
/// It may miss by [`THICKNESS_SHARE`] of `|ΔZ|`, plus what rounding each pressure can move it
/// (half its step, over the pressure, times `R_d T̄_v / g₀`), plus [`THICKNESS_SLACK_M`]. The
/// step is 1 hPa for a pressure of 100 hPa or more in a `coarse` answer (a coded message's), else
/// 0.1 hPa.
fn fits(lower: &Thermo, upper: &Thermo, coarse: bool) -> bool {
    miss_share(lower, upper, coarse) <= 1.0
}

/// How far `upper`'s height misses the thickness from `lower`, as a share of what [`fits`] allows:
/// at most 1 when it fits; infinite when it can't be worked out.
fn miss_share(lower: &Thermo, upper: &Thermo, coarse: bool) -> f64 {
    let scale_height_m = DRY_AIR_GAS_CONSTANT_J_PER_KG_K
        * 0.5
        * (lower.virtual_temperature_k + upper.virtual_temperature_k)
        / STANDARD_GRAVITY_MPS2;
    let thickness_m = scale_height_m * (lower.pressure_pa / upper.pressure_pa).ln();
    let half_step_pa = |p: f64| {
        if coarse && p >= 10_000.0 { 50.0 } else { 5.0 }
    };
    let rounding_m = scale_height_m
        * (half_step_pa(lower.pressure_pa) / lower.pressure_pa
            + half_step_pa(upper.pressure_pa) / upper.pressure_pa);
    let miss_m = upper.geopotential_height_m - lower.geopotential_height_m - thickness_m;
    let allowance_m = THICKNESS_SHARE * thickness_m.abs() + rounding_m + THICKNESS_SLACK_M;
    let share = miss_m.abs() / allowance_m;
    if share.is_finite() {
        share
    } else {
        f64::INFINITY
    }
}

/// The virtual temperature, K, of air at `temperature_k` with relative humidity `humidity` (a
/// fraction over liquid water, taken as at most 1) and pressure `pressure_pa`, as
/// `hpr_atmos::moist` has it: `T_v = T / (1 − x_v (1 − M_v/M₀))`, with the vapour's share of the
/// pressure `x_v = e/p` at most 1.
fn virtual_temperature_k(temperature_k: f64, humidity: f64, pressure_pa: f64) -> f64 {
    let vapour_pa = humidity.min(1.0) * saturation_vapour_pressure_pa(temperature_k);
    let share = (vapour_pa / pressure_pa).min(1.0);
    let ratio = WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL / SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL;
    temperature_k / (1.0 - share * (1.0 - ratio))
}

/// A column the parser reads, as an index into the parser's table of column positions.
#[derive(Clone, Copy)]
enum Column {
    Time,
    Longitude,
    Latitude,
    Pressure,
    Height,
    Temperature,
    Humidity,
    Direction,
    Speed,
}

/// A row's values as a level, or why they make none.
fn level(
    pressure_hpa: f64,
    geopotential_height_m: f64,
    temperature_c: f64,
    humidity_pct: f64,
    wind_speed_m_s: f64,
    direction_deg: f64,
    latitude_rad: f64,
) -> Result<WyomingLevel, DropReason> {
    let temperature_k = temperature_c + 273.15;
    if !(MIN_PRESSURE_HPA..=MAX_PRESSURE_HPA).contains(&pressure_hpa)
        || !(MIN_TEMPERATURE_C..=MAX_TEMPERATURE_C).contains(&temperature_c)
        || !(MIN_HEIGHT_M..=MAX_HEIGHT_M).contains(&geopotential_height_m)
        || humidity_pct < 0.0
        || !(0.0..=MAX_WIND_M_S).contains(&wind_speed_m_s)
        || !(0.0..=360.0).contains(&direction_deg)
    {
        return Err(DropReason::OutOfRange);
    }
    let height_msl_m = geometric_from_wmo_geopotential_m(geopotential_height_m, latitude_rad)
        .map_err(|_| DropReason::OutOfRange)?;
    Ok(WyomingLevel {
        pressure_pa: pressure_hpa * 100.0,
        geopotential_height_m,
        height_msl_m,
        temperature_k,
        relative_humidity: humidity_pct / 100.0,
        wind_speed_m_s,
        wind_direction_from_rad: wrap_direction(direction_deg.to_radians()),
    })
}

/// The index of the column `name` (a header is `name_unit`, or `name` alone), checking its unit
/// and that it appears once.
fn find_column<'a>(
    names: &[&'a str],
    name: &'static str,
    unit: &'static str,
) -> Result<usize, WyomingError> {
    let split = |header: &'a str| header.rsplit_once('_').unwrap_or((header, ""));
    let mut found = names
        .iter()
        .enumerate()
        .filter(|(_, header)| split(header).0 == name);
    let Some((i, header)) = found.next() else {
        return Err(WyomingError::Missing {
            field: name.to_owned(),
        });
    };
    if found.next().is_some() {
        return Err(WyomingError::Row {
            line: 1,
            reason: format!("two {name} columns"),
        });
    }
    let found_unit = split(header).1;
    if found_unit != unit {
        return Err(WyomingError::Units {
            field: name,
            found: cut(found_unit),
            expected: unit,
        });
    }
    Ok(i)
}

/// A row's fields, which must number as many as the header's. At most one more than that is
/// split off, so a hostile row costs no more than a good one.
fn split_row(line: &str, columns: usize, line_no: usize) -> Result<Vec<&str>, WyomingError> {
    let fields: Vec<&str> = line.splitn(columns + 1, ',').map(str::trim).collect();
    if fields.len() != columns {
        let found = if fields.len() > columns {
            format!("more than {columns}")
        } else {
            fields.len().to_string()
        };
        return Err(WyomingError::Row {
            line: line_no,
            reason: format!("{found} fields, not {columns}"),
        });
    }
    Ok(fields)
}

/// A field as a finite number, or `None` when it is empty.
fn number(
    fields: &[&str],
    index: usize,
    line_no: usize,
    what: &str,
) -> Result<Option<f64>, WyomingError> {
    let text = fields[index];
    if text.is_empty() {
        return Ok(None);
    }
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => Ok(Some(value)),
        _ => Err(WyomingError::Row {
            line: line_no,
            reason: format!("the {what} {:?} is not a number", cut(text)),
        }),
    }
}

/// `YYYY-MM-DD HH:MM:SS` as seconds since the Unix epoch, UTC.
fn parse_time(text: &str) -> Option<i64> {
    let (date, time) = text.split_once(' ')?;
    let mut date = date.split('-');
    let mut time = time.split(':');
    let next = |parts: &mut std::str::Split<'_, char>, digits: usize| {
        let part = parts.next()?;
        (part.len() == digits && part.bytes().all(|b| b.is_ascii_digit()))
            .then(|| part.parse::<i64>().ok())
            .flatten()
    };
    let (year, month, day) = (
        next(&mut date, 4)?,
        next(&mut date, 2)?,
        next(&mut date, 2)?,
    );
    let (hour, minute, second) = (
        next(&mut time, 2)?,
        next(&mut time, 2)?,
        next(&mut time, 2)?,
    );
    if date.next().is_some() || time.next().is_some() || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    Some(unix_day_start(year, month, day)? + hour * HOUR_S + minute * 60 + second)
}

/// Text for an error message, cut to [`QUOTE_CHARS`] characters.
fn cut(text: &str) -> String {
    let mut cut: String = text.chars().take(QUOTE_CHARS).collect();
    if text.chars().nth(QUOTE_CHARS).is_some() {
        cut.push('…');
    }
    cut
}

/// An angle in `[0, 2π)`. `rem_euclid` alone returns 2π for a tiny negative angle.
fn wrap_direction(angle_rad: f64) -> f64 {
    let wrapped = angle_rad.rem_euclid(TAU);
    if wrapped >= TAU { 0.0 } else { wrapped }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row at `pressure_hpa`, `height_m` and `virtual_k`.
    fn at(pressure_hpa: f64, height_m: f64, virtual_k: f64) -> Thermo {
        Thermo {
            pressure_pa: pressure_hpa * 100.0,
            geopotential_height_m: height_m,
            virtual_temperature_k: virtual_k,
        }
    }

    /// Dry air at 288.15 K from 1000 to 900 hPa: `R_d T / g₀ = 8,434.5 m` and the thickness
    /// 888.665 m. Rounded to 1 hPa the pressures can move it 8.903 m, so the allowance is
    /// 44.433 + 8.903 + 30 = 83.336 m; rounded to 0.1 hPa (1000.5 to 900.5 hPa, thickness
    /// 888.197 m), 75.300 m. Worked out by hand from the equation, not by the code.
    #[test]
    fn a_layer_fits_within_its_allowance() {
        let lower = at(1_000.0, 0.0, 288.15);
        for (thickness, allowance, coarse) in [(888.665, 83.336, true), (888.197, 75.300, false)] {
            let (lower, upper_hpa) = if coarse {
                (lower, 900.0)
            } else {
                (at(1_000.5, 0.0, 288.15), 900.5)
            };
            for sign in [1.0, -1.0] {
                let height = |miss: f64| thickness + sign * miss;
                let inside = at(upper_hpa, height(allowance - 0.01), 288.15);
                let outside = at(upper_hpa, height(allowance + 0.01), 288.15);
                assert!(fits(&lower, &inside, coarse), "{coarse} {sign}");
                assert!(!fits(&lower, &outside, coarse), "{coarse} {sign}");
            }
        }
        // A coarse answer's step is 1 hPa only at 100 hPa or more: at 50 to 45 hPa, 0.1 hPa.
        let high = at(50.0, 20_000.0, 288.15);
        let thickness = 8_434.49 * (50.0_f64 / 45.0).ln();
        let allowance = 0.05 * thickness + 8_434.49 * (5.0 / 5_000.0 + 5.0 / 4_500.0) + 30.0;
        let upper = at(45.0, 20_000.0 + thickness + allowance + 0.1, 288.15);
        assert!(!fits(&high, &upper, true));
        // The layer's mean: 288.15 K below and 278.15 K above give 8,288.16 m and 873.245 m, with
        // an allowance of 82.411 m.
        let cooler = |height: f64| at(900.0, height, 278.15);
        assert!(fits(&lower, &cooler(873.245 + 82.40), true));
        assert!(!fits(&lower, &cooler(873.245 + 82.42), true));
        // Downward, as a falling balloon's rows are: the same layer, the other way.
        assert!(fits(&at(900.0, 888.665, 288.15), &lower, true));
    }

    /// The guide's worked example, from the coded message's rows at 557 hPa (5,035 m, −1.7 °C,
    /// 79%) and 549 hPa (−2.5 °C, 81%): `T_v` 272.238 and 271.420 K, `R_d T̄_v / g₀` 7,956.79 m,
    /// thickness 115.109 m and allowance 5.755 + 14.389 + 30 = 50.145 m, by hand. Taken as dry,
    /// the thickness would be 0.33 m less.
    #[test]
    fn the_guides_example_fits_as_worked() -> Result<(), &'static str> {
        let lower = thermo(557.0, 5_035.0, -1.7, Some(79.0)).ok_or("lower")?;
        assert!((lower.virtual_temperature_k - 272.238).abs() < 1e-3);
        let upper = |height: f64| thermo(549.0, height, -2.5, Some(81.0)).ok_or("upper");
        assert!((upper(0.0)?.virtual_temperature_k - 271.420).abs() < 1e-3);
        for sign in [1.0, -1.0] {
            let height = |miss: f64| 5_035.0 + 115.109 + sign * miss;
            assert!(fits(&lower, &upper(height(50.135))?, true), "{sign}");
            assert!(!fits(&lower, &upper(height(50.155))?, true), "{sign}");
        }
        // A 557 hPa row with a digit lost, 57 hPa at 5,035 m, misses the 570 hPa row at 4,852 m by
        // 18.4 km.
        let below = thermo(570.0, 4_852.0, -0.5, Some(75.0)).ok_or("below")?;
        let lost = thermo(57.0, 5_035.0, -1.7, Some(79.0)).ok_or("lost")?;
        assert!(!fits(&below, &lost, true));
        Ok(())
    }

    /// Saturated air at 30 °C: `T_v` is 308.081 K at 1000 hPa and 308.638 K at 900 hPa (WMO-No. 8
    /// eq. 12.18 with eq. 4.B.1's 4,233.8 Pa), by hand. A humidity above 100% is taken as 100%;
    /// vapour at more than the air's pressure counts as all of it, so `T_v` stays finite.
    #[test]
    fn virtual_temperature_follows_the_humidity() {
        for (pressure_pa, expected) in [(100_000.0, 308.081), (90_000.0, 308.638)] {
            let t_v = virtual_temperature_k(303.15, 1.0, pressure_pa);
            assert!((t_v - expected).abs() < 1e-3, "{t_v}");
            assert_eq!(virtual_temperature_k(303.15, 1.03, pressure_pa), t_v);
        }
        assert_eq!(virtual_temperature_k(288.15, 0.0, 100_000.0), 288.15);
        let ratio =
            WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL / SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL;
        let all_vapour = virtual_temperature_k(333.15, 1.0, 1_000.0);
        assert!((all_vapour - 333.15 / ratio).abs() < 1e-9, "{all_vapour}");
    }

    #[test]
    fn values_at_the_bounds_are_kept_and_past_them_dropped() {
        let at = |p: f64, z: f64, t: f64, speed: f64| level(p, z, t, 50.0, speed, 180.0, 0.5);
        for (p, z, t, speed) in [
            (0.1, 5_000.0, -10.0, 10.0),
            (1_200.0, 5_000.0, -10.0, 10.0),
            (500.0, -1_000.0, -10.0, 10.0),
            (500.0, 60_000.0, -10.0, 10.0),
            (500.0, 5_000.0, -150.0, 10.0),
            (500.0, 5_000.0, 80.0, 10.0),
            (500.0, 5_000.0, -10.0, 0.0),
            (500.0, 5_000.0, -10.0, 300.0),
        ] {
            assert!(at(p, z, t, speed).is_ok(), "{p} {z} {t} {speed}");
        }
        for (p, z, t, speed) in [
            (0.099, 5_000.0, -10.0, 10.0),
            (1_200.1, 5_000.0, -10.0, 10.0),
            (500.0, -1_000.1, -10.0, 10.0),
            (500.0, 60_000.1, -10.0, 10.0),
            (500.0, 5_000.0, -150.1, 10.0),
            (500.0, 5_000.0, 80.1, 10.0),
            (500.0, 5_000.0, -10.0, -0.1),
            (500.0, 5_000.0, -10.0, 300.1),
        ] {
            assert_eq!(
                at(p, z, t, speed),
                Err(DropReason::OutOfRange),
                "{p} {z} {t} {speed}"
            );
        }
    }

    #[test]
    fn url_names_the_hour_station_and_version() -> Result<(), WyomingError> {
        let mut request = WyomingRequest::new("72364", 1_750_507_200);
        assert_eq!(
            request.url()?,
            "https://weather.uwyo.edu/wsgi/sounding?datetime=2025-06-21%2012:00:00&id=72364\
             &type=TEXT:CSV&src=FM35"
        );
        request.version = WyomingVersion::Bufr;
        request.endpoint = Some("http://127.0.0.1:8080/s".to_owned());
        assert_eq!(
            request.url()?,
            "http://127.0.0.1:8080/s?datetime=2025-06-21%2012:00:00&id=72364&type=TEXT:CSV\
             &src=BUFR"
        );
        Ok(())
    }

    #[test]
    fn url_refuses_each_bad_field() {
        let bad = |request: WyomingRequest, field: &str| {
            assert!(
                matches!(request.url(), Err(WyomingError::Request { what, .. }) if what == field),
                "{request:?}"
            );
        };
        let at = 1_750_507_200;
        for station in ["", "72 364", "72364&x=1", "Ω", "12345678901234567"] {
            bad(WyomingRequest::new(station, at), "station");
        }
        for time in [
            at + 1,
            at + 1_800,
            -HOUR_S,
            YEAR_10000_S,
            i64::MAX,
            i64::MIN,
        ] {
            bad(WyomingRequest::new("72364", time), "time (s since 1970)");
        }
        for endpoint in ["", "https://proxy/s?key=abc", "https://proxy/s#top"] {
            let mut request = WyomingRequest::new("72364", at);
            request.endpoint = Some(endpoint.to_owned());
            bad(request, "endpoint");
        }
        assert!(WyomingRequest::new("1234567890123456", at).url().is_ok());
        assert!(
            WyomingRequest::new("72364", YEAR_10000_S - HOUR_S)
                .url()
                .is_ok()
        );
    }

    #[test]
    fn latest_before_picks_the_last_00_or_12_utc() {
        let noon = 1_750_507_200; // 2025-06-21 12:00 UTC
        for (launch, expected) in [
            (noon, noon),
            (noon + 1, noon),
            (noon + 12 * HOUR_S - 1, noon),
            (noon + 12 * HOUR_S, noon + 12 * HOUR_S),
            (noon - 1, noon - 12 * HOUR_S),
        ] {
            assert_eq!(
                WyomingRequest::latest_before("72364", launch).time_unix_s,
                expected
            );
        }
        // The extremes saturate rather than overflow, and the URL refuses them.
        for launch in [i64::MIN, i64::MAX] {
            let request = WyomingRequest::latest_before("72364", launch);
            assert!(request.url().is_err(), "{request:?}");
        }
    }

    #[test]
    fn times_parse_strictly() {
        assert_eq!(parse_time("2025-06-21 11:02:00"), Some(1_750_503_720));
        assert_eq!(parse_time("1970-01-01 00:00:00"), Some(0));
        for bad in [
            "",
            "2025-06-21",
            "2025-06-21T11:02:00",
            "2025-06-21 11:02",
            "2025-06-21 11:02:00:00",
            "2025-6-21 11:02:00",
            "2025-06-21 24:00:00",
            "2025-06-21 11:60:00",
            "2025-06-21 11:02:60",
            "2025-02-29 11:02:00",
            "2025-06-21 +1:02:00",
            "+025-06-21 11:02:00",
        ] {
            assert_eq!(parse_time(bad), None, "{bad}");
        }
    }

    #[test]
    fn quotes_are_cut() {
        assert_eq!(cut("m/s"), "m/s");
        assert_eq!(cut(&"x".repeat(QUOTE_CHARS)), "x".repeat(QUOTE_CHARS));
        let long = cut(&"x".repeat(1_000));
        assert_eq!(long.chars().count(), QUOTE_CHARS + 1);
        assert!(long.ends_with('…'));
    }

    /// Young, an answer is fresh for an hour; settled, only one fetched after it settled, for up
    /// to 30 days.
    #[test]
    fn freshness_follows_the_soundings_age() {
        let noon = 1_750_507_200_u64;
        let request = WyomingRequest::new("72364", 1_750_507_200);
        let source = request.source(noon + 3_600);
        assert_eq!(
            (source.ttl_s, source.attribution.as_str()),
            (YOUNG_TTL_S, ATTRIBUTION)
        );
        let settled = noon + SETTLE_S;
        assert_eq!(request.source(settled - 1).ttl_s, YOUNG_TTL_S);
        assert_eq!(request.source(settled).ttl_s, 0);
        assert_eq!(request.source(settled + 7_200).ttl_s, 7_200);
        assert_eq!(request.source(settled + 365 * 86_400).ttl_s, SETTLED_TTL_S);
        // A request before 1970, which `url` refuses, still has a source.
        assert_eq!(
            WyomingRequest::new("72364", -1).source(0).ttl_s,
            YOUNG_TTL_S
        );
    }
}
