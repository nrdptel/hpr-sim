//! The flight record every log reader produces: a clock, the channels the logger recorded on it,
//! and what the file states about itself.
//!
//! Units are SI, converted from the file's own at reading: metres, metres per second, kelvin,
//! volts. A gap in a channel, such as a row that left a column out, is `NaN`.

use serde::{Deserialize, Serialize};

/// A flight log, read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlightLog {
    /// The file format it was read from.
    pub format: LogFormat,
    /// The logger, as the file names it, such as `PerfectFlite Pnut`.
    pub logger: String,
    /// The logger's serial number, as the file states it.
    pub serial_number: Option<String>,
    /// The logger's firmware version, as the file states it.
    pub firmware: Option<String>,
    /// The flight's number in the logger's memory, as the file states it.
    pub flight_number: Option<u32>,
    /// What the file states about the flight: the logger's own figures, kept beside hpr's
    /// readings and never in place of them.
    pub stated: Stated,
    /// Each sample's time, s, from the logger's own zero. Strictly increasing.
    pub time_s: Vec<f64>,
    /// The barometric altitude, m, as the logger converts its pressure, above the logger's own
    /// zero: for a PerfectFlite, its reading on the pad.
    pub altitude_m: Vec<f64>,
    /// The vertical speed, m/s, up positive, as the logger computes it; `None` if the file has
    /// no such column. A barometric logger works it out from its own altitude.
    pub vertical_speed_m_s: Option<Vec<f64>>,
    /// The logger's temperature sensor, K; `None` if the file has no such column. It sits in the
    /// electronics bay, so it reads the bay, not the air.
    pub temperature_k: Option<Vec<f64>>,
    /// The logger's battery voltage, V; `None` if the file has no such column.
    pub battery_v: Option<Vec<f64>>,
    /// What the reader noticed and worked around, such as a sample count that differs from the
    /// one the file states.
    pub notes: Vec<String>,
}

/// A flight log's file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum LogFormat {
    /// PerfectFlite's `.pf2`, written by the Pnut, the StratoLogger and the StratoLoggerCF.
    PerfectFlitePf2,
}

impl LogFormat {
    /// The format's usual file extension, with its dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::PerfectFlitePf2 => ".pf2",
        }
    }
}

/// What a flight log states about the flight, in SI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Stated {
    /// The apogee the logger computed, m above its own zero; `None` if the file states none, or
    /// states something that isn't a height, such as a PerfectFlite's `PWRLOSS`.
    pub apogee_m: Option<f64>,
    /// The launch site's elevation, m above mean sea level, as the logger states it.
    pub ground_elevation_msl_m: Option<f64>,
    /// How many samples the file says it holds.
    pub samples: Option<usize>,
}
