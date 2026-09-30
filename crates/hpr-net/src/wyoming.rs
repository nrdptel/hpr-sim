//! Weather-balloon soundings from the University of Wyoming's radiosonde archive, turned into a
//! [`SoundingProfile`].
//!
//! A radiosonde is an instrument package carried up by a weather balloon, usually at 00 and
//! 12 UTC, from about 800 stations worldwide. It measures pressure, temperature and humidity, and
//! its drift gives the wind. The [University of Wyoming][uwyo] serves the archive of these
//! soundings. It is a measurement, not a forecast, but only at the station and the time of the
//! flight, often a hundred kilometres and some hours from a launch.
//!
//! A [`WyomingRequest`] names a station (its WMO number, such as `72364` for Santa Teresa, New
//! Mexico) and the sounding's nominal hour, and asks for the comma-separated text, one row per
//! level, with these columns (among others) in these units:
//!
//! | column | unit | read as |
//! |---|---|---|
//! | `time` | `YYYY-MM-DD HH:MM:SS` UTC | the release time, from the first row |
//! | `latitude`, `longitude` | degrees | the station, from the first row |
//! | `pressure_hPa` | hPa | pressure |
//! | `geopotential height_m` | geopotential m | height above sea level |
//! | `temperature_C` | °C | temperature |
//! | `relative humidity_%` | % | over liquid water (the file has `humidity wrt ice_%` too) |
//! | `wind direction_degree`, `wind speed_m/s` | °, m/s | the wind, the direction it blows from |
//!
//! A column in any other unit is refused, not converted. Two versions of most soundings are
//! served ([`WyomingSource`]): the coded message stations send (FM 35, "TEMP"), with the
//! standard pressure levels and the significant levels between them, about 200 rows; and the
//! BUFR file, a row a second, about 6,000.
//!
//! [`WyomingSounding::parse`] keeps:
//!
//! - **The ground**, the first row: the station's pressure, temperature, humidity and wind.
//! - **Each row above it** with every value given, whose height is above the last row kept and
//!   whose pressure is below it. BUFR's pressures are rounded to 0.1 hPa, so near the top many
//!   rows repeat the pressure below them; they are dropped. A row missing a value (the last row
//!   often has no wind) is dropped too, and so is one whose relative humidity is below zero.
//!   [`WyomingSounding::dropped`] lists each, with its reason.
//!
//! Heights are geopotential metres (the column says so), converted to geometric heights at the
//! station's latitude with WMO-No. 8 eq. 12.16
//! ([`hpr_atmos::profile::geometric_from_wmo_geopotential_m`]); the [atmosphere page][atmos]
//! explains why. The balloon drifts, but converting at the station's latitude is the same
//! latitude the profile uses for its hydrostatics. A relative humidity above 100%, which
//! radiosondes report in cloud, is kept as recorded and taken as 100% in
//! [`WyomingSounding::sounding`], as the [atmosphere's decision record][adr-004] asks.
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
//! [guide]: https://nrdptel.github.io/hpr-sim/weather.html

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

/// How long an answer stays fresh: a day. A sounding doesn't change once flown, but the archive's
/// copy can fill in for some hours after it, as a station's later messages arrive.
pub const TTL_S: u64 = 86_400;

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
pub enum WyomingSource {
    /// The coded message the station sends (WMO FM 35, "TEMP"): the standard pressure levels and
    /// the significant levels between them, about 200 rows, pressures to 1 hPa (0.1 hPa above
    /// 100 hPa).
    Fm35,
    /// The BUFR file, where the station sends one: a row a second, about 6,000, pressures to
    /// 0.1 hPa.
    Bufr,
}

impl WyomingSource {
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
    pub source: WyomingSource,
    /// Another server with the same interface, instead of [`ENDPOINT`].
    pub endpoint: Option<String>,
}

impl WyomingRequest {
    /// A request for `station`'s coded-message sounding at the whole hour `time_unix_s`.
    #[must_use]
    pub fn new(station: impl Into<String>, time_unix_s: i64) -> Self {
        Self {
            station: station.into(),
            time_unix_s,
            source: WyomingSource::Fm35,
            endpoint: None,
        }
    }

    /// A request for the latest 00 or 12 UTC sounding at or before `launch_unix_s`: the one a
    /// launch at that time would have had.
    #[must_use]
    pub fn latest_before(station: impl Into<String>, launch_unix_s: i64) -> Self {
        let half_day_s = 12 * HOUR_S;
        Self::new(station, launch_unix_s.div_euclid(half_day_s) * half_day_s)
    }

    /// The URL: `…?datetime=YYYY-MM-DD%20HH:00:00&id=<station>&type=TEXT:CSV&src=<source>`.
    ///
    /// # Errors
    /// [`WyomingError::Request`] when the station is empty, longer than 16 characters or not
    /// letters and digits, or the time is not a whole hour from 1970 to 9999.
    pub fn url(&self) -> Result<String, WyomingError> {
        let station = &self.station;
        if station.is_empty()
            || station.len() > 16
            || !station.bytes().all(|b| b.is_ascii_alphanumeric())
        {
            return Err(WyomingError::Request {
                what: "station",
                value: cut(station),
            });
        }
        if !(0..YEAR_10000_S).contains(&self.time_unix_s) || self.time_unix_s % HOUR_S != 0 {
            return Err(WyomingError::Request {
                what: "time",
                value: self.time_unix_s.to_string(),
            });
        }
        let (year, month, day, hour) = date_hour(self.time_unix_s);
        let endpoint = self.endpoint.as_deref().unwrap_or(ENDPOINT);
        Ok(format!(
            "{endpoint}?datetime={year:04}-{month:02}-{day:02}%20{hour:02}:00:00&id={station}\
             &type=TEXT:CSV&src={}",
            self.source.query()
        ))
    }

    /// The cache's view of the source: the archive's name, [`ATTRIBUTION`] and [`TTL_S`].
    #[must_use]
    pub fn source(&self) -> Source {
        Source {
            name: "University of Wyoming soundings".to_owned(),
            attribution: ATTRIBUTION.to_owned(),
            ttl_s: TTL_S,
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
    /// Geometric height above sea level, m (WMO-No. 8 eq. 12.16 at the station's latitude).
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
    /// Its height is not above the last row kept, or its pressure not below it.
    NotAbove,
    /// Its relative humidity is below zero.
    Humidity,
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
    /// The station's latitude, degrees north, from the first row.
    pub latitude_deg: f64,
    /// The station's longitude, degrees east, from the first row.
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
    /// - [`WyomingError::Row`] when a row has the wrong number of fields, or a field is not a
    ///   number (or, in the first row, the time is not a date).
    /// - [`WyomingError::NoGround`] when there is no row, or the first is missing a value or has
    ///   a relative humidity below zero.
    /// - [`WyomingError::Atmos`] when a geopotential height has no geometric height.
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
        let names: Vec<&str> = header.split(',').map(str::trim).collect();
        let mut index = [0_usize; COLUMNS.len()];
        for (slot, &(_, name, unit)) in index.iter_mut().zip(&COLUMNS) {
            *slot = find_column(&names, name, unit)?;
        }
        let col = |column: Column| index[column as usize];

        let mut rows = lines.filter(|(_, line)| !line.trim().is_empty()).peekable();
        let Some(&(first, first_line)) = rows.peek() else {
            return Err(WyomingError::NoGround { line: 2 });
        };
        let fields = split_row(first_line, names.len(), first + 1)?;
        let latitude_deg = number(&fields, col(Column::Latitude), first + 1, "latitude")?
            .ok_or(WyomingError::NoGround { line: first + 1 })?;
        let longitude_deg = number(&fields, col(Column::Longitude), first + 1, "longitude")?
            .ok_or(WyomingError::NoGround { line: first + 1 })?;
        let release_unix_s =
            parse_time(fields[col(Column::Time)]).ok_or_else(|| WyomingError::Row {
                line: first + 1,
                reason: format!(
                    "the time {:?} is not a date",
                    cut(fields[col(Column::Time)])
                ),
            })?;
        let latitude_rad = latitude_deg.to_radians();

        let mut levels: Vec<WyomingLevel> = Vec::new();
        let mut dropped = Vec::new();
        for (i, line) in rows {
            let line_no = i + 1;
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
            let ground = levels.is_empty();
            let drop = |reason| {
                if ground {
                    Err(WyomingError::NoGround { line: line_no })
                } else {
                    Ok(DroppedLevel {
                        line: line_no,
                        reason,
                    })
                }
            };
            let (
                Some(pressure_hpa),
                Some(geopotential_height_m),
                Some(temperature_c),
                Some(humidity_pct),
                Some(wind_speed_m_s),
                Some(direction_deg),
            ) = values
            else {
                dropped.push(drop(DropReason::NoData)?);
                continue;
            };
            let height_msl_m =
                geometric_from_wmo_geopotential_m(geopotential_height_m, latitude_rad)?;
            let pressure_pa = pressure_hpa * 100.0;
            if let Some(below) = levels.last()
                && (height_msl_m <= below.height_msl_m || pressure_pa >= below.pressure_pa)
            {
                dropped.push(drop(DropReason::NotAbove)?);
                continue;
            }
            if humidity_pct < 0.0 {
                dropped.push(drop(DropReason::Humidity)?);
                continue;
            }
            levels.push(WyomingLevel {
                pressure_pa,
                geopotential_height_m,
                height_msl_m,
                temperature_k: temperature_c + 273.15,
                relative_humidity: humidity_pct / 100.0,
                wind_speed_m_s,
                wind_direction_from_rad: wrap_direction(direction_deg.to_radians()),
            });
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
    /// What [`SoundingProfile::new`] refuses, such as a temperature at or below 0 K or a
    /// negative wind speed.
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
/// The answer comes from the client's cache while fresh ([`TTL_S`]); offline, from the cache
/// only. The [`Fetched`] says which, carries [`ATTRIBUTION`] and holds the body. Only an answer
/// that parses and makes a sounding is cached ([`Client::fetch_checked`]): one that doesn't never
/// takes a good copy's place, and online a stale good copy is returned instead, with the reason.
/// A sounding the archive doesn't have is an HTTP 404, which the transport reports.
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
    let fetched = client.fetch_checked(&request.source(), &url, now_s, check)?;
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
        /// Its value.
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
        /// The unit it came in.
        found: String,
        /// The unit required.
        expected: &'static str,
    },
    /// A row can't be read.
    #[error("line {line} of the Wyoming answer: {reason}")]
    Row {
        /// The line, counting the header as line 1.
        line: usize,
        /// What is wrong with it.
        reason: String,
    },
    /// There is no ground: no row, or a first row missing a value or with a negative humidity.
    #[error("the Wyoming answer has no usable ground level (line {line})")]
    NoGround {
        /// The line of the first row, counting the header as line 1.
        line: usize,
    },
    /// A height or level was refused by the atmosphere.
    #[error(transparent)]
    Atmos(#[from] AtmosError),
}

/// A column the parser reads, as an index into [`COLUMNS`].
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

/// The index of the column `name` (a header is `name_unit`, or `name` alone), checking its unit.
fn find_column(
    names: &[&str],
    name: &'static str,
    unit: &'static str,
) -> Result<usize, WyomingError> {
    for (i, header) in names.iter().enumerate() {
        let (found_name, found_unit) = header.rsplit_once('_').unwrap_or((header, ""));
        if found_name == name {
            if found_unit != unit {
                return Err(WyomingError::Units {
                    field: name,
                    found: cut(found_unit),
                    expected: unit,
                });
            }
            return Ok(i);
        }
    }
    Err(WyomingError::Missing {
        field: name.to_owned(),
    })
}

/// A row's fields, which must number as many as the header's.
fn split_row(line: &str, columns: usize, line_no: usize) -> Result<Vec<&str>, WyomingError> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != columns {
        return Err(WyomingError::Row {
            line: line_no,
            reason: format!("{} fields, not {columns}", fields.len()),
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
    fn url_names_the_hour_station_and_source() -> Result<(), WyomingError> {
        let mut request = WyomingRequest::new("72364", 1_750_507_200);
        assert_eq!(
            request.url()?,
            "https://weather.uwyo.edu/wsgi/sounding?datetime=2025-06-21%2012:00:00&id=72364\
             &type=TEXT:CSV&src=FM35"
        );
        request.source = WyomingSource::Bufr;
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
        for time in [at + 1, at + 1_800, -HOUR_S, YEAR_10000_S, i64::MAX] {
            bad(WyomingRequest::new("72364", time), "time");
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

    #[test]
    fn source_keeps_a_sounding_a_day() {
        let source = WyomingRequest::new("72364", 0).source();
        assert_eq!(
            (source.ttl_s, source.attribution.as_str()),
            (TTL_S, ATTRIBUTION)
        );
        assert_eq!(TTL_S, 86_400);
    }
}
