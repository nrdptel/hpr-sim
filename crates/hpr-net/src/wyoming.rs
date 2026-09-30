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
//! [`WyomingSounding::parse`] keeps:
//!
//! - **The ground**, the first row: the pressure, temperature, humidity and wind at the station
//!   when the balloon was released.
//! - **One row of each run with the same pressure**, the middle one. BUFR's pressures are rounded
//!   to 0.1 hPa, and high up the balloon climbs tens of metres while the pressure falls that much,
//!   so runs of rows share a pressure; the rounded value is the pressure at about the middle of
//!   its run.
//! - **Each such row above the last row kept**, higher and at a lower pressure.
//!
//! A row missing a value (the last row often has no wind) is dropped, and so is one with a value
//! no air on Earth has ([`DropReason::OutOfRange`] lists the bounds). [`WyomingSounding::dropped`]
//! lists each row left out, with its reason.
//!
//! Rows below the last row kept since it (a run of one pressure counting once) that climb among
//! themselves refuse the answer when more than [`MAX_NOT_ABOVE`] of them do, each higher and at a
//! lower pressure than the highest before it: a grossly bad row was kept (a pressure missing a
//! digit, say), and good rows after it are being dropped. Rows that fall or float, a balloon coming
//! down, are only left out, however many. A bad row that still lies between its neighbours is not
//! caught: nothing checks a layer's thickness against its temperature.
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

use hpr_atmos::profile::geometric_from_wmo_geopotential_m;
use hpr_atmos::{AtmosError, SoundingLevel, SoundingProfile, WindInterpolation};
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

/// The most rows below the last row kept since it that may climb among themselves, each higher and
/// at a lower pressure than the highest before it. More means a bad row was kept and good rows
/// after it are being dropped, so the answer is refused.
pub const MAX_NOT_ABOVE: usize = 10;

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
    /// Another row of its run with the same pressure (or the ground's pressure) was kept.
    SamePressure,
    /// Its height is not above the last row kept, or its pressure not below it.
    NotAbove,
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
    /// - [`WyomingError::NotRising`] when more than [`MAX_NOT_ABOVE`] rows below the last row kept
    ///   since it climb among themselves.
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

        // Every row read: its level, or why it has none.
        let mut read: Vec<(usize, Result<WyomingLevel, DropReason>)> = Vec::new();
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
            read.push((line_no, level));
        }
        let Some((_, Ok(ground))) = read.first() else {
            return Err(WyomingError::NoGround { line: first + 1 });
        };
        let ground = *ground;

        // Pick the middle row of each run of complete rows with the same pressure; a run at the
        // ground's pressure keeps none.
        let mut choice: Vec<(usize, Result<WyomingLevel, DropReason>)> =
            Vec::with_capacity(read.len());
        let mut run: Vec<(usize, WyomingLevel)> = Vec::new();
        let close =
            |run: &mut Vec<(usize, WyomingLevel)>,
             choice: &mut Vec<(usize, Result<WyomingLevel, DropReason>)>| {
                let middle = run.len().saturating_sub(1) / 2;
                let at_ground = run
                    .first()
                    .is_some_and(|r| r.1.pressure_pa == ground.pressure_pa);
                for (k, (line, level)) in run.drain(..).enumerate() {
                    let kept = k == middle && !at_ground;
                    choice.push((
                        line,
                        if kept {
                            Ok(level)
                        } else {
                            Err(DropReason::SamePressure)
                        },
                    ));
                }
            };
        for &(line, level) in &read[1..] {
            match level {
                Ok(level) => {
                    if run
                        .last()
                        .is_some_and(|r| r.1.pressure_pa != level.pressure_pa)
                    {
                        close(&mut run, &mut choice);
                    }
                    run.push((line, level));
                }
                Err(reason) => choice.push((line, Err(reason))),
            }
        }
        close(&mut run, &mut choice);
        choice.sort_by_key(|c| c.0);

        let mut levels = vec![ground];
        let mut dropped = Vec::new();
        // The last row kept's line.
        let mut last_kept = read[0].0;
        // The rows below it since.
        let mut below: Option<Below> = None;
        for (line, level) in choice {
            let reason = match level {
                Ok(level) => {
                    let under = levels[levels.len() - 1];
                    if level.height_msl_m > under.height_msl_m
                        && level.pressure_pa < under.pressure_pa
                    {
                        if let Some(below) = below.take() {
                            below.check(last_kept)?;
                        }
                        levels.push(level);
                        last_kept = line;
                        continue;
                    }
                    match &mut below {
                        Some(below) => below.add(level),
                        None => {
                            below = Some(Below {
                                first: line,
                                top: level,
                                climbs: 1,
                                count: 1,
                            });
                        }
                    }
                    DropReason::NotAbove
                }
                Err(reason) => reason,
            };
            dropped.push(DroppedLevel { line, reason });
        }
        // Rows below the last row kept at the end are a balloon falling or floating, unless they
        // climb among themselves: then the row kept before them was bad, and the balloon burst
        // before it got back above it.
        if let Some(below) = below {
            below.check(last_kept)?;
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
    /// Too many rows below the last row kept since it climb among themselves: that row was likely
    /// bad.
    #[error(
        "the Wyoming answer stops rising after line {after}: {count} rows from line {line} on lie \
         below it, and rise again"
    )]
    NotRising {
        /// The last row kept before them, likely the bad one, counting the header as line 1.
        after: usize,
        /// The first row below it.
        line: usize,
        /// How many rows below it there are since it.
        count: usize,
    },
}

/// The rows below the last row kept since it.
struct Below {
    /// The first one's line.
    first: usize,
    /// The highest of them so far, by the keep rule: each row higher and at a lower pressure
    /// than the one before it replaces it.
    top: WyomingLevel,
    /// How many rows climbed that way, the first included.
    climbs: usize,
    /// How many there are.
    count: usize,
}

impl Below {
    fn add(&mut self, level: WyomingLevel) {
        self.count += 1;
        if level.height_msl_m > self.top.height_msl_m && level.pressure_pa < self.top.pressure_pa {
            self.top = level;
            self.climbs += 1;
        }
    }

    /// Refuses the answer when more than [`MAX_NOT_ABOVE`] of the rows climb: good rows after a
    /// bad one keep rising, while a balloon falling or floating gets nowhere.
    fn check(&self, after: usize) -> Result<(), WyomingError> {
        if self.climbs > MAX_NOT_ABOVE {
            return Err(WyomingError::NotRising {
                after,
                line: self.first,
                count: self.count,
            });
        }
        Ok(())
    }
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
