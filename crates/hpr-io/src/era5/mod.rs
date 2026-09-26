//! ERA5 pressure-level files: the atmosphere over a launch site at launch time, as a
//! [`SoundingProfile`].
//!
//! ERA5 is ECMWF's global reanalysis, hourly on a 0.25° grid, distributed by the Copernicus
//! Climate Data Store (Hersbach et al., "The ERA5 global reanalysis", *Q. J. R. Meteorol. Soc.*
//! 146 (2020) 1999–2049). A pressure-level file holds, on each pressure level `p`, the
//! geopotential `z` (m² s⁻²), temperature `t` (K) and the wind's east and north components `u`
//! and `v` (m s⁻¹), on a grid of latitude, longitude and time. This module reads such a file in
//! the netCDF classic formats ([`crate::netcdf`]); a netCDF-4 file from the current Climate Data
//! Store is converted first ([`crate::netcdf::CONVERSION`]).
//!
//! **At the site**, each value on each level is interpolated bilinearly in latitude and
//! longitude (in degrees) between the four grid points around the site, as RocketPy 1.13 does
//! (`rocketpy/tools.py`, `bilinear_interpolation`, MIT):
//!
//! ```text
//! f = [f₁₁ (x₂ − x)(y₂ − y) + f₂₁ (x − x₁)(y₂ − y) + f₁₂ (x₂ − x)(y − y₁) + f₂₂ (x − x₁)(y − y₁)]
//!     / ((x₂ − x₁)(y₂ − y₁))
//! ```
//!
//! with `x` the latitude and `y` the longitude.
//!
//! **At launch time**, the two hourly fields around it are weighted linearly in time; a launch
//! on the hour takes that hour's field alone. RocketPy takes the nearest hour instead.
//!
//! **Heights.** ERA5's geopotential height is `Z = z/g₀`, with `g₀ = 9.80665 m s⁻²` fixed in
//! ECMWF's model. "Geometric height is not represented in ERA5", and ECMWF suggests
//! `h = R Z/(R − Z)`, "neglecting horizontal variations in the Earth's gravitational
//! acceleration" (ECMWF Knowledge Base, "ERA5: compute pressure and geopotential on model levels,
//! geopotential height and geometric height", captured 2026-09-26). RocketPy does that. hpr
//! instead reads `Z` as a WMO geopotential height, as for any sounding, and takes the geometric
//! height by WMO-No. 8 eqs. 12.15–12.16 at the site's latitude
//! ([`geometric_from_wmo_geopotential_m`]), the relation [`SoundingProfile`] inverts.
//!
//! Both are approximations. Suppose the model's ground lies at true height `h_s` with geopotential
//! `g₀ h_s`, as reading surface geopotential over `g₀` as the ground's height takes it (ECMWF does
//! not say how the model builds it, so this is an assumption), and gravity above it falls off as
//! WMO's formula has it. To first order hpr's reading is then off by `h_s (g₀/γ_s − 1) + h_s²/R`,
//! the same at every height, and ECMWF's by `h_s²/R − (h − h_s)(g₀/γ_s − 1)`, which grows with the
//! height above the model's ground (plus `h²(1/R_e − 1/R)` when ECMWF's radius `R_e` is not WMO's
//! `R`). Neither is always the smaller. Exactly, with RocketPy's `R_e` (the WGS 84 ellipsoid's
//! distance from the centre at the site): for a model ground at 407 m at 47.2° N hpr's is −0.04 m.
//! For one 1400 m up at 33° N hpr's is 1.88 m, and ECMWF's is 0.31 m at the ground and −3.07 m
//! 3 km above it; ECMWF's is the smaller up to 1.95 km above that ground.
//! hpr keeps WMO's because it is the rule [`SoundingProfile`] uses for every sounding, so a level's
//! geopotential round-trips. The two readings differ by `g₀/γ_s(φ) − 1` of the height, −1.58e-4 at
//! 47.21° N and +3.43e-4 at 41.78° N (−0.69 m at 4.4 km and +1.45 m at 4.2 km on the tests' files).
//!
//! **What it leaves out.** Humidity is not read, so the air is dry: at 20 °C and 50% relative
//! humidity dry air is about 0.4% denser than the real air. Between and beyond the levels the
//! profile is [`SoundingProfile`]'s: hydrostatic between levels and the offset standard atmosphere
//! above the highest level, where RocketPy holds every value at the end level's.

use std::f64::consts::TAU;

use hpr_atmos::profile::geometric_from_wmo_geopotential_m;
use hpr_atmos::wind::WindInterpolation;
use hpr_atmos::{AtmosError, SoundingLevel, SoundingProfile};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::netcdf::{NetCdf, NetCdfError, Variable};

/// ERA5's `g₀`, which divides geopotential into geopotential height, m s⁻².
pub const ERA5_GRAVITY_M_S2: f64 = 9.80665;

/// Why an ERA5 profile could not be read.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum Era5Error {
    /// The netCDF file could not be read.
    #[error(transparent)]
    NetCdf(#[from] NetCdfError),
    /// A variable the profile needs is not in the file.
    #[error("the file has no `{name}` variable")]
    MissingVariable {
        /// The name, or the alternatives, looked for.
        name: String,
    },
    /// A variable's dimensions are not the ones ERA5 writes.
    #[error("`{variable}` has dimensions {found:?}; expected {expected}")]
    Dimensions {
        /// The variable.
        variable: String,
        /// Its dimensions: the first eight, then how many more there are.
        found: Vec<String>,
        /// What was expected.
        expected: String,
    },
    /// A variable's units are not the ones expected.
    #[error("`{variable}` is in `{units}`; expected {expected}")]
    Units {
        /// The variable.
        variable: String,
        /// Its units attribute, empty if absent.
        units: String,
        /// What was expected.
        expected: String,
    },
    /// The time variable's units or calendar cannot be read.
    #[error("the time axis cannot be read: {reason}")]
    Time {
        /// Why.
        reason: String,
    },
    /// The launch time is outside the file's times.
    #[error(
        "the launch time ({requested} s after 1970) is outside the file's times ({first} to {last})"
    )]
    OutsideTimes {
        /// The launch time, seconds since 1970-01-01T00:00Z.
        requested: f64,
        /// The first time in the file.
        first: f64,
        /// The last time in the file.
        last: f64,
    },
    /// The site is outside the file's grid.
    #[error("the site's {axis} ({value}°) is outside the file's grid ({first}° to {last}°)")]
    OutsideGrid {
        /// `latitude` or `longitude`.
        axis: &'static str,
        /// The site's coordinate.
        value: f64,
        /// The grid's first value.
        first: f64,
        /// The grid's last value.
        last: f64,
    },
    /// A coordinate axis has no values.
    #[error("the `{axis}` axis is empty")]
    EmptyAxis {
        /// The axis.
        axis: String,
    },
    /// A coordinate value is missing (a fill value).
    #[error("`{axis}` is missing its value at index {index}")]
    MissingCoordinate {
        /// The axis.
        axis: String,
        /// The position along it.
        index: usize,
    },
    /// A coordinate axis is not strictly monotonic.
    #[error("the `{axis}` axis is not strictly monotonic")]
    NotMonotonic {
        /// The axis.
        axis: String,
    },
    /// A value the profile needs is missing (a fill value) in the file.
    #[error("`{variable}` is missing at {pressure_pa} Pa near the site")]
    MissingValue {
        /// The variable.
        variable: String,
        /// The level.
        pressure_pa: f64,
    },
    /// A request value is not usable.
    #[error("{what} is {value}, which is not usable")]
    Domain {
        /// What the value is.
        what: &'static str,
        /// The value.
        value: f64,
    },
    /// The atmosphere could not be built from the levels.
    #[error(transparent)]
    Atmos(#[from] AtmosError),
}

/// An instant in Coordinated Universal Time, as seconds since 1970-01-01T00:00:00Z (leap seconds
/// not counted, as in POSIX time).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct UtcTime {
    unix_s: f64,
}

impl UtcTime {
    /// The instant `seconds` after 1970-01-01T00:00:00Z.
    ///
    /// # Errors
    ///
    /// [`Era5Error::Domain`] if `seconds` is not finite.
    pub fn from_unix_seconds(seconds: f64) -> Result<Self, Era5Error> {
        if !seconds.is_finite() {
            return Err(Era5Error::Domain {
                what: "the time (s)",
                value: seconds,
            });
        }
        Ok(UtcTime { unix_s: seconds })
    }

    /// The instant at a date and time of the Gregorian calendar, in UTC.
    ///
    /// # Errors
    ///
    /// [`Era5Error::Domain`] for a month outside 1 to 12, a day not in the month, an hour past
    /// 23, a minute past 59, or a second outside `[0, 60)`.
    pub fn from_civil(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: f64,
    ) -> Result<Self, Era5Error> {
        let bad = |what, value: f64| Err(Era5Error::Domain { what, value });
        if !(1..=12).contains(&month) {
            return bad("the month", f64::from(month));
        }
        if day == 0 || day > days_in_month(year, month) {
            return bad("the day of the month", f64::from(day));
        }
        if hour > 23 {
            return bad("the hour", f64::from(hour));
        }
        if minute > 59 {
            return bad("the minute", f64::from(minute));
        }
        if !(0.0..60.0).contains(&second) {
            return bad("the second", second);
        }
        let days = days_from_civil(i64::from(year), month, day);
        let whole = days * 86_400 + i64::from(hour) * 3600 + i64::from(minute) * 60;
        // Exact: every whole second of five million years fits in an f64's 53 bits.
        Ok(UtcTime {
            unix_s: whole as f64 + second,
        })
    }

    /// Seconds since 1970-01-01T00:00:00Z.
    pub fn unix_seconds(self) -> f64 {
        self.unix_s
    }
}

impl TryFrom<f64> for UtcTime {
    type Error = Era5Error;

    fn try_from(seconds: f64) -> Result<Self, Era5Error> {
        UtcTime::from_unix_seconds(seconds)
    }
}

impl From<UtcTime> for f64 {
    fn from(time: UtcTime) -> f64 {
        time.unix_s
    }
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 to a date of the proleptic Gregorian calendar: H. Hinnant,
/// "chrono-Compatible Low-Level Date Algorithms", `days_from_civil`.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y.rem_euclid(400);
    let m = i64::from(month);
    let day_of_year = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Where and when to read the atmosphere.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Era5Request {
    /// Geodetic latitude of the site, degrees north.
    pub latitude_deg: f64,
    /// Longitude of the site, degrees east (either `[-180, 180]` or `[0, 360)`).
    pub longitude_deg: f64,
    /// The launch time.
    pub time: UtcTime,
}

/// One pressure level at the site and time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Era5Level {
    /// The level's pressure, Pa.
    pub pressure_pa: f64,
    /// Geopotential height `Z = z/g₀`, gpm.
    pub geopotential_height_m: f64,
    /// Geometric height above mean sea level from `Z` at the site's latitude (WMO-No. 8), m.
    pub height_msl_m: f64,
    /// Temperature, K.
    pub temperature_k: f64,
    /// The wind's east component `u`, m/s.
    pub wind_east_m_s: f64,
    /// The wind's north component `v`, m/s.
    pub wind_north_m_s: f64,
}

/// The atmosphere over a site at a time, read from an ERA5 pressure-level file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era5Profile {
    /// What was asked for.
    pub request: Era5Request,
    /// The file's times used and their weights: one on the hour, else the two around it.
    pub times: Vec<(UtcTime, f64)>,
    /// The levels, lowest (highest pressure) first.
    pub levels: Vec<Era5Level>,
    /// Variables on the same grid that were not read, such as humidity.
    pub unread: Vec<String>,
}

/// The time variable's and pressure variable's names: the current Climate Data Store's first,
/// then the older one's.
const TIME_NAMES: [&str; 2] = ["valid_time", "time"];
const LEVEL_NAMES: [&str; 2] = ["pressure_level", "level"];

fn find<'a>(file: &'a NetCdf, names: &[&str]) -> Result<&'a Variable, Era5Error> {
    names
        .iter()
        .find_map(|name| file.variable(name))
        .ok_or_else(|| Era5Error::MissingVariable {
            name: names.join("` or `"),
        })
}

fn units(variable: &Variable) -> String {
    variable
        .attribute("units")
        .and_then(|a| a.values.text())
        .unwrap_or("")
        .trim()
        .to_owned()
}

fn check_units(variable: &Variable, accepted: &[&str]) -> Result<(), Era5Error> {
    let found = units(variable);
    if accepted.contains(&found.as_str()) {
        Ok(())
    } else {
        Err(Era5Error::Units {
            variable: variable.name.clone(),
            units: found,
            expected: format!("`{}`", accepted.join("` or `")),
        })
    }
}

/// A variable's dimension names, for an error: the first few and a count of the rest, so a file
/// that names one long dimension on many axes can't make its error text grow as the square of its
/// size.
fn names_of(variable: &Variable) -> Vec<String> {
    const SHOWN: usize = 8;
    let mut names: Vec<String> = variable
        .dimensions
        .iter()
        .take(SHOWN)
        .map(|d| d.to_string())
        .collect();
    if let Some(rest) = variable
        .dimensions
        .len()
        .checked_sub(SHOWN)
        .filter(|&n| n > 0)
    {
        names.push(format!("and {rest} more"));
    }
    names
}

/// A one-dimensional coordinate variable's unpacked values.
fn axis(variable: &Variable) -> Result<Vec<f64>, Era5Error> {
    // A coordinate variable lies along the dimension of its own name (CF Conventions 1.11 §1.2),
    // so the data variables' dimensions of that name are indexed by it.
    if !matches!(&variable.dimensions[..], [only] if **only == *variable.name) {
        return Err(Era5Error::Dimensions {
            variable: variable.name.clone(),
            found: names_of(variable),
            expected: format!("[\"{}\"]", variable.name),
        });
    }
    if variable.values.is_empty() {
        return Err(Era5Error::EmptyAxis {
            axis: variable.name.clone(),
        });
    }
    let packing = variable.packing()?;
    (0..variable.values.len())
        .map(|index| {
            variable
                .values
                .get(index)
                .and_then(|stored| packing.unpack(stored))
                .ok_or_else(|| Era5Error::MissingCoordinate {
                    axis: variable.name.clone(),
                    index,
                })
        })
        .collect()
}

/// Two indices around `x` on a strictly monotonic axis and their weights' numerators and common
/// denominator: `f(x) = (a₁ f[i₁] + a₂ f[i₂])/span`. A point on the axis takes that point alone.
fn bracket(values: &[f64], x: f64) -> Option<(usize, usize, f64, f64, f64)> {
    if let Some(i) = values.iter().position(|&v| v == x) {
        return Some((i, i, 1.0, 0.0, 1.0));
    }
    values.windows(2).enumerate().find_map(|(i, pair)| {
        let (x1, x2) = (pair[0], pair[1]);
        let inside = (x1 < x && x < x2) || (x2 < x && x < x1);
        inside.then(|| (i, i + 1, x2 - x, x - x1, x2 - x1))
    })
}

fn strictly_monotonic(values: &[f64]) -> bool {
    values.windows(2).all(|w| w[0] < w[1]) || values.windows(2).all(|w| w[0] > w[1])
}

/// Seconds per unit and the epoch, in seconds after 1970, of CF time units
/// `"<unit> since <date>[ <time>][ <zone>]"` (CF Conventions 1.11 §4.4).
fn time_units(units: &str, calendar: Option<&str>) -> Result<(f64, f64), Era5Error> {
    let bad = |reason: String| Era5Error::Time { reason };
    let calendar = calendar.map(str::to_ascii_lowercase);
    let proleptic = match calendar.as_deref() {
        None | Some("standard" | "gregorian") => false,
        Some("proleptic_gregorian") => true,
        Some(other) => return Err(bad(format!("calendar `{other}` is not read"))),
    };
    let mut words = units.split_whitespace();
    let unit = words.next().unwrap_or("").to_ascii_lowercase();
    let seconds = match unit.as_str() {
        "seconds" | "second" | "secs" | "sec" | "s" => 1.0,
        "minutes" | "minute" | "mins" | "min" => 60.0,
        "hours" | "hour" | "hrs" | "hr" | "h" => 3600.0,
        "days" | "day" | "d" => 86_400.0,
        _ => return Err(bad(format!("units `{units}` are not a time since a date"))),
    };
    if words.next().map(str::to_ascii_lowercase).as_deref() != Some("since") {
        return Err(bad(format!("units `{units}` are not a time since a date")));
    }
    let rest: Vec<&str> = words.collect();
    let (date, clock, zone) = match rest.as_slice() {
        [date] => match date.split_once('T') {
            Some((d, t)) => (d.to_owned(), t.to_owned(), None),
            None => ((*date).to_owned(), "0:0:0".to_owned(), None),
        },
        [date, clock] => ((*date).to_owned(), (*clock).to_owned(), None),
        [date, clock, zone] => ((*date).to_owned(), (*clock).to_owned(), Some(*zone)),
        _ => return Err(bad(format!("units `{units}` have no reference date"))),
    };
    let (clock, zone) = match clock.strip_suffix('Z') {
        Some(clock) => (clock.to_owned(), Some("Z")),
        None => (clock, zone),
    };
    if !matches!(
        zone,
        None | Some("UTC" | "Z" | "+00:00" | "+0000" | "+00" | "00:00")
    ) {
        return Err(bad(format!("reference time zone in `{units}` is not UTC")));
    }
    let number = |text: &str| text.parse::<f64>().ok();
    let date: Vec<Option<f64>> = date.splitn(3, '-').map(number).collect();
    let clock: Vec<Option<f64>> = clock.splitn(3, ':').map(number).collect();
    let (Some(year), Some(month), Some(day)) = (
        date.first().copied().flatten(),
        date.get(1).copied().flatten(),
        date.get(2).copied().flatten(),
    ) else {
        return Err(bad(format!(
            "reference date in `{units}` is not YYYY-MM-DD"
        )));
    };
    let field = |i: usize| clock.get(i).copied().unwrap_or(Some(0.0));
    let (Some(hour), Some(minute), Some(second)) = (field(0), field(1), field(2)) else {
        return Err(bad(format!("reference time in `{units}` is not hh:mm:ss")));
    };
    let whole = |x: f64| x.fract() == 0.0 && x.abs() < 1e6;
    if ![year, month, day, hour, minute].into_iter().all(whole) {
        return Err(bad(format!(
            "reference date in `{units}` is not whole numbers"
        )));
    }
    // The standard calendar is Julian before 1582-10-15; only the Gregorian part is read.
    if !proleptic && (year, month, day) < (1582.0, 10.0, 15.0) {
        return Err(bad(format!(
            "reference date in `{units}` falls in the standard calendar's Julian part"
        )));
    }
    let epoch = UtcTime::from_civil(
        year as i32,
        month as u32,
        day as u32,
        hour as u32,
        minute as u32,
        second,
    )
    .map_err(|e| bad(format!("reference date in `{units}`: {e}")))?;
    Ok((seconds, epoch.unix_seconds()))
}

/// The dimension positions of a four-dimensional data variable: time, level, latitude,
/// longitude.
fn positions(variable: &Variable, names: [&str; 4]) -> Result<[usize; 4], Era5Error> {
    let error = || Era5Error::Dimensions {
        variable: variable.name.clone(),
        found: names_of(variable),
        expected: format!("{names:?} in any order"),
    };
    if variable.dimensions.len() != 4 {
        return Err(error());
    }
    let mut out = [0; 4];
    for (slot, name) in out.iter_mut().zip(names) {
        *slot = variable
            .dimensions
            .iter()
            .position(|d| **d == *name)
            .ok_or_else(error)?;
    }
    Ok(out)
}

impl Era5Profile {
    /// Reads the atmosphere at `request` from an ERA5 pressure-level file.
    ///
    /// # Errors
    ///
    /// [`Era5Error`]: a variable missing or with other dimensions or units, a time axis whose
    /// units or calendar cannot be read, a site or time outside the file, a missing value around
    /// the site, or a non-finite coordinate.
    pub fn read(file: &NetCdf, request: Era5Request) -> Result<Self, Era5Error> {
        for (what, value) in [
            ("the latitude (deg)", request.latitude_deg),
            ("the longitude (deg)", request.longitude_deg),
        ] {
            if !value.is_finite() {
                return Err(Era5Error::Domain { what, value });
            }
        }
        if request.latitude_deg.abs() > 90.0 {
            return Err(Era5Error::Domain {
                what: "the latitude (deg)",
                value: request.latitude_deg,
            });
        }

        // Time.
        let time = find(file, &TIME_NAMES)?;
        let calendar = time.attribute("calendar").and_then(|a| a.values.text());
        let (unit_s, epoch_s) = time_units(&units(time), calendar)?;
        let times: Vec<f64> = axis(time)?
            .into_iter()
            .map(|t| epoch_s + t * unit_s)
            .collect();
        if !times.windows(2).all(|w| w[0] < w[1]) {
            return Err(Era5Error::NotMonotonic {
                axis: time.name.clone(),
            });
        }
        let t = request.time.unix_seconds();
        // `axis` refuses an empty axis.
        let (first, last) = (times[0], times[times.len() - 1]);
        let Some((t1, t2, a1, a2, span)) = bracket(&times, t) else {
            return Err(Era5Error::OutsideTimes {
                requested: t,
                first,
                last,
            });
        };
        let mut time_weights = vec![(t1, a1 / span)];
        if t2 != t1 {
            time_weights.push((t2, a2 / span));
        }

        // Levels.
        let level = find(file, &LEVEL_NAMES)?;
        let level_scale = match units(level).as_str() {
            "millibars" | "millibar" | "mbar" | "hPa" => 100.0,
            "Pa" => 1.0,
            other => {
                return Err(Era5Error::Units {
                    variable: level.name.clone(),
                    units: other.to_owned(),
                    expected: "`hPa`, `millibars` or `Pa`".into(),
                });
            }
        };
        let pressures: Vec<f64> = axis(level)?.iter().map(|p| p * level_scale).collect();

        // The site on the grid.
        let latitude = find(file, &["latitude"])?;
        let longitude = find(file, &["longitude"])?;
        check_units(latitude, &["degrees_north"])?;
        check_units(longitude, &["degrees_east"])?;
        let lats = axis(latitude)?;
        let lons = axis(longitude)?;
        for (name, values) in [("latitude", &lats), ("longitude", &lons)] {
            if !strictly_monotonic(values) {
                return Err(Era5Error::NotMonotonic { axis: name.into() });
            }
        }
        let outside = |axis, value, values: &[f64]| Era5Error::OutsideGrid {
            axis,
            value,
            first: values.first().copied().unwrap_or(f64::NAN),
            last: values.last().copied().unwrap_or(f64::NAN),
        };
        let x = request.latitude_deg;
        let (i1, i2, xa, xb, xspan) =
            bracket(&lats, x).ok_or_else(|| outside("latitude", x, &lats))?;
        // The site's longitude, turned by whole turns into the grid's range.
        let (low, high) = lons
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(l, h), &v| {
                (l.min(v), h.max(v))
            });
        let y = [0.0, -360.0, 360.0]
            .into_iter()
            .map(|turn| request.longitude_deg + turn)
            .find(|y| (low..=high).contains(y))
            .ok_or_else(|| outside("longitude", request.longitude_deg, &lons))?;
        let (j1, j2, ya, yb, yspan) =
            bracket(&lons, y).ok_or_else(|| outside("longitude", y, &lons))?;

        let names = [
            time.name.as_str(),
            level.name.as_str(),
            "latitude",
            "longitude",
        ];
        // `f` at every level: bilinear at each time used, weighted in time.
        let field = |name: &str, accepted: &[&str]| -> Result<Vec<f64>, Era5Error> {
            let variable = find(file, &[name])?;
            check_units(variable, accepted)?;
            let at = positions(variable, names)?;
            let packing = variable.packing()?;
            let mut out = Vec::with_capacity(pressures.len());
            for (k, &pressure_pa) in pressures.iter().enumerate() {
                let value = |ti: usize, i: usize, j: usize| -> Result<f64, Era5Error> {
                    let mut index = [0u64; 4];
                    for (slot, n) in at.into_iter().zip([ti, k, i, j]) {
                        index[slot] = n as u64;
                    }
                    variable
                        .offset(&index)
                        .and_then(|o| variable.values.get(o))
                        .and_then(|stored| packing.unpack(stored))
                        .ok_or_else(|| Era5Error::MissingValue {
                            variable: name.to_owned(),
                            pressure_pa,
                        })
                };
                let mut total = 0.0;
                for &(ti, weight) in &time_weights {
                    let f11 = value(ti, i1, j1)?;
                    let f21 = value(ti, i2, j1)?;
                    let f12 = value(ti, i1, j2)?;
                    let f22 = value(ti, i2, j2)?;
                    let f = (f11 * xa * ya + f21 * xb * ya + f12 * xa * yb + f22 * xb * yb)
                        / (xspan * yspan);
                    total += weight * f;
                }
                out.push(total);
            }
            Ok(out)
        };
        let z = field("z", &["m**2 s**-2", "m2 s-2"])?;
        let t_k = field("t", &["K"])?;
        let u = field("u", &["m s**-1", "m s-1"])?;
        let v = field("v", &["m s**-1", "m s-1"])?;

        let latitude_rad = request.latitude_deg.to_radians();
        let mut levels = Vec::with_capacity(pressures.len());
        for k in 0..pressures.len() {
            let geopotential_height_m = z[k] / ERA5_GRAVITY_M_S2;
            levels.push(Era5Level {
                pressure_pa: pressures[k],
                geopotential_height_m,
                height_msl_m: geometric_from_wmo_geopotential_m(
                    geopotential_height_m,
                    latitude_rad,
                )?,
                temperature_k: t_k[k],
                wind_east_m_s: u[k],
                wind_north_m_s: v[k],
            });
        }
        levels.sort_by(|a, b| b.pressure_pa.total_cmp(&a.pressure_pa));

        let unread = file
            .variables
            .iter()
            .filter(|var| var.dimensions.len() == 4 && !["z", "t", "u", "v"].contains(&&*var.name))
            .map(|var| var.name.clone())
            .collect();
        let times = time_weights
            .into_iter()
            .map(|(i, w)| (UtcTime { unix_s: times[i] }, w))
            .collect();
        Ok(Era5Profile {
            request,
            times,
            levels,
            unread,
        })
    }

    /// The profile as an atmosphere: every level's height, temperature, pressure and wind, dry,
    /// at the site's latitude, with the wind interpolated as `wind_interpolation`.
    ///
    /// # Errors
    ///
    /// [`AtmosError`] as [`SoundingProfile::new`] raises it, for example for heights that do not
    /// increase from level to level.
    pub fn sounding(
        &self,
        wind_interpolation: WindInterpolation,
    ) -> Result<SoundingProfile, AtmosError> {
        let levels = self
            .levels
            .iter()
            .map(|level| SoundingLevel {
                height_msl_m: level.height_msl_m,
                temperature_k: level.temperature_k,
                pressure_pa: Some(level.pressure_pa),
                relative_humidity: None,
                wind_speed_m_s: Some(level.wind_east_m_s.hypot(level.wind_north_m_s)),
                wind_direction_from_rad: Some(direction_from_rad(
                    level.wind_east_m_s,
                    level.wind_north_m_s,
                )),
            })
            .collect();
        SoundingProfile::new(
            levels,
            self.request.latitude_deg.to_radians(),
            wind_interpolation,
        )
    }
}

/// The direction a wind of east and north components `u` and `v` blows from, clockwise from true
/// north, in `[0, 2π)`; 0 for a calm.
pub fn direction_from_rad(u: f64, v: f64) -> f64 {
    if u == 0.0 && v == 0.0 {
        return 0.0;
    }
    let angle = (-u).atan2(-v).rem_euclid(TAU);
    // `+ 0.0` turns a -0 (a wind from due north) into 0.
    if angle >= TAU { 0.0 } else { angle + 0.0 }
}

#[cfg(test)]
mod tests;
