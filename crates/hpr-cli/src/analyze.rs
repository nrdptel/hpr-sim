//! `hpr analyze`: a flight log's readings, taken from the log alone, with no design file and no
//! simulation (ADR-046).
//!
//! It reads the log with `hpr_flightdata`'s reader for its format and prints what
//! `hpr_flightdata::readings` takes from it: liftoff, apogee, the top speed, the top acceleration
//! and landing, each with where it came from, or withheld with the reason the log can't support
//! it. What the file states about itself, such as the logger's own apogee, is printed beside
//! them, never in place of them.

use std::path::Path;

use hpr::hpr_flightdata::log::{FlightLog, LogFormat};
use hpr::hpr_flightdata::perfectflite::{self, FOOT_M};
use hpr::hpr_flightdata::readings::{self, Reading, Readings, Reason, Source};

use crate::output::{
    Analyze, AnalyzeMethod, AnalyzedLog, ApogeeReading, HighestSample, LandingReading,
    LiftoffReading, LogFormatName, LogReading, LoggerStated, MaxAccelerationReading,
    MaxSpeedReading, ReadingSource, WithheldReading, WithheldReason,
};
use crate::{Failure, Out};

/// `hpr analyze`'s arguments.
#[derive(Debug, clap::Args)]
pub struct AnalyzeArgs {
    /// The flight log: a PerfectFlite .pf2
    pub log: String,
}

/// Runs `hpr analyze`.
pub(crate) fn run(args: &AnalyzeArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    let log = read_log(&args.log)?;
    let read = readings::read(&log);
    let document = document(&args.log, &log, &read);
    let lines = text_lines(&document);
    to.emit(&document, |out| {
        lines.iter().try_for_each(|line| writeln!(out, "{line}"))
    })
}

/// Reads a flight log in a format hpr knows, by its extension or its first line.
fn read_log(path: &str) -> Result<FlightLog, Failure> {
    let bytes = std::fs::read(path).map_err(|error| Failure::Input(format!("{path}: {error}")))?;
    let unknown = || {
        Failure::Input(format!(
            "{path}: hpr analyze reads PerfectFlite .pf2 logs so far, and this isn't one; other \
             loggers' files arrive with milestone M7.1"
        ))
    };
    // Only the comments of a `.pf2` can hold anything but ASCII, and hpr doesn't use them: a
    // comment in another encoding mustn't refuse the flight.
    let text = String::from_utf8_lossy(&bytes);
    let pf2 = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pf2"));
    let names_perfectflite = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.to_ascii_lowercase().contains("perfectflite"));
    if !(pf2 || names_perfectflite) {
        return Err(unknown());
    }
    perfectflite::read(&text).map_err(|error| Failure::Input(format!("{path}: {error}")))
}

/// The output document.
fn document(path: &str, log: &FlightLog, read: &Readings) -> Analyze {
    Analyze {
        log: AnalyzedLog {
            path: path.to_owned(),
            format: match log.format {
                LogFormat::PerfectFlitePf2 => LogFormatName::PerfectFlitePf2,
                _ => LogFormatName::Other,
            },
            logger: log.logger.clone(),
            serial_number: log.serial_number.clone(),
            firmware: log.firmware.clone(),
            flight_number: log.flight_number,
            samples: log.time_s.len(),
            first_time_s: log.time_s.first().copied().unwrap_or(0.0),
            last_time_s: log.time_s.last().copied().unwrap_or(0.0),
            notes: log.notes.clone(),
        },
        stated: LoggerStated {
            apogee_m: log.stated.apogee_m,
            ground_elevation_msl_m: log.stated.ground_elevation_msl_m,
        },
        method: AnalyzeMethod {
            sample_interval_s: read.sample_interval_s,
            median_window_s: read.median_window_s,
            peak_bound_m: read.peak_bound_m,
            altitude_resolution_m: log.format.altitude_resolution_m(),
            pad_altitude_m: read.pad_altitude_m,
        },
        liftoff: reading(&read.liftoff, |liftoff| LiftoffReading {
            time_s: liftoff.time_s,
            source: source(liftoff.source),
        }),
        apogee: reading(&read.apogee, |apogee| ApogeeReading {
            time_s: apogee.time_s,
            time_after_liftoff_s: apogee.time_after_liftoff_s,
            altitude_m: apogee.altitude_m,
            is_floor: apogee.is_floor,
            highest_sample: HighestSample {
                time_s: apogee.highest_sample.time_s,
                altitude_m: apogee.highest_sample.altitude_m,
            },
            source: source(apogee.source),
        }),
        max_speed: reading(&read.max_speed, |speed| MaxSpeedReading {
            speed_m_s: speed.speed_m_s,
            time_s: speed.time_s,
            altitude_m: speed.altitude_m,
            source: source(speed.source),
        }),
        max_acceleration: reading(&read.max_acceleration, |acceleration| {
            MaxAccelerationReading {
                acceleration_m_s2: acceleration.acceleration_m_s2,
                time_s: acceleration.time_s,
            }
        }),
        landing: reading(&read.landing, |landing| LandingReading {
            time_s: landing.time_s,
            flight_time_s: landing.flight_time_s,
            descent_time_s: landing.descent_time_s,
            mean_descent_rate_m_s: landing.mean_descent_rate_m_s,
            source: source(landing.source),
        }),
    }
}

fn reading<A, B>(reading: &Reading<A>, value: impl FnOnce(&A) -> B) -> LogReading<B> {
    match reading {
        Reading::Read(read) => LogReading::Read(value(read)),
        Reading::Withheld(withheld) => LogReading::Withheld(WithheldReading {
            reason: match withheld.reason {
                Reason::TooShort => WithheldReason::TooShort,
                Reason::NoClimb => WithheldReason::NoClimb,
                Reason::StartsOffThePad => WithheldReason::StartsOffThePad,
                Reason::EndsBeforeLanding => WithheldReason::EndsBeforeLanding,
                Reason::FasterThanFreeFall => WithheldReason::FasterThanFreeFall,
                Reason::NoSpeedColumn => WithheldReason::NoSpeedColumn,
                Reason::ImplausibleSpeed => WithheldReason::ImplausibleSpeed,
                Reason::NoisySpeed => WithheldReason::NoisySpeed,
                Reason::SpeedPeakAtLiftoff => WithheldReason::SpeedPeakAtLiftoff,
                Reason::NoAccelerometer => WithheldReason::NoAccelerometer,
                Reason::Needs => WithheldReason::Needs,
                Reason::BadRecord => WithheldReason::BadRecord,
                _ => WithheldReason::Other,
            },
            detail: withheld.detail.clone(),
        }),
    }
}

fn source(source: Source) -> ReadingSource {
    match source {
        Source::Barometer => ReadingSource::Barometer,
        Source::LoggerSpeedFromBarometer => ReadingSource::LoggerSpeedFromBarometer,
        _ => ReadingSource::Other,
    }
}

/// A height or a length in metres, with feet.
fn metres(value: f64) -> String {
    format!("{value:.1} m ({:.0} ft)", value / FOOT_M)
}

/// A speed in metres per second, with feet per second.
fn speed(value: f64) -> String {
    format!("{value:.1} m/s ({:.0} ft/s)", value / FOOT_M)
}

/// What `hpr analyze` prints without `--json`.
fn text_lines(document: &Analyze) -> Vec<String> {
    let log = &document.log;
    let mut about = vec![log.logger.clone()];
    if let Some(serial) = &log.serial_number {
        about.push(format!("serial {serial}"));
    }
    if let Some(flight) = log.flight_number {
        about.push(format!("flight {flight}"));
    }
    // The file's name, as `hpr convert` prints it: the folder adds nothing a reader needs.
    let name = Path::new(&log.path).file_name().map_or_else(
        || log.path.clone(),
        |name| name.to_string_lossy().into_owned(),
    );
    let mut lines = vec![format!("{name}: {}", about.join(", "))];
    let method = &document.method;
    lines.push(match (method.sample_interval_s, method.median_window_s) {
        (Some(interval), Some(window)) => format!(
            "{} samples every {interval:.3} s, from {:.2} s to {:.2} s; heights from the \
             altitude after a {window:.2} s running median",
            log.samples, log.first_time_s, log.last_time_s
        ),
        _ => format!("{} samples", log.samples),
    });
    let mut stated = Vec::new();
    if let Some(apogee) = document.stated.apogee_m {
        stated.push(format!("apogee {}", metres(apogee)));
    }
    if let Some(ground) = document.stated.ground_elevation_msl_m {
        stated.push(format!(
            "ground elevation {} above sea level",
            metres(ground)
        ));
    }
    if !stated.is_empty() {
        lines.push(format!("the logger states: {}", stated.join("; ")));
    }
    lines.push(String::new());

    let row = |name: &str, value: String| format!("{name:<18}{value}");
    let withheld = |reading: &WithheldReading| format!("withheld: {}", reading.detail);
    lines.push(row(
        "liftoff",
        match &document.liftoff {
            LogReading::Read(liftoff) => format!("{:.2} s", liftoff.time_s),
            LogReading::Withheld(reading) => withheld(reading),
        },
    ));
    match &document.apogee {
        LogReading::Read(apogee) => {
            let mut value = format!("{} at {:.2} s", metres(apogee.altitude_m), apogee.time_s);
            if let Some(after) = apogee.time_after_liftoff_s {
                value.push_str(&format!(", {after:.2} s after liftoff"));
            }
            if apogee.is_floor {
                value.push_str("; the log ends at its peak, so the rocket may have gone higher");
            }
            lines.push(row("apogee", value));
            // Above what the median's own rounding of the peak and the altitude's resolution
            // explain, the median set a pulse aside.
            let highest = &apogee.highest_sample;
            let explained = method.peak_bound_m.unwrap_or(0.0) + method.altitude_resolution_m;
            if highest.altitude_m - apogee.altitude_m > explained {
                lines.push(row(
                    "",
                    format!(
                        "highest sample {} at {:.2} s, set aside by the median",
                        metres(highest.altitude_m),
                        highest.time_s
                    ),
                ));
            }
        }
        LogReading::Withheld(reading) => lines.push(row("apogee", withheld(reading))),
    }
    lines.push(row(
        "top speed",
        match &document.max_speed {
            LogReading::Read(top) => format!(
                "{} at {:.2} s, {} up: the logger's own, from its barometer",
                speed(top.speed_m_s),
                top.time_s,
                metres(top.altitude_m)
            ),
            LogReading::Withheld(reading) => withheld(reading),
        },
    ));
    lines.push(row(
        "top acceleration",
        match &document.max_acceleration {
            LogReading::Read(top) => {
                format!("{:.1} m/s² at {:.2} s", top.acceleration_m_s2, top.time_s)
            }
            LogReading::Withheld(reading) => withheld(reading),
        },
    ));
    lines.push(row(
        "landing",
        match &document.landing {
            LogReading::Read(landing) => {
                let mut value = format!(
                    "{:.2} s, {:.2} s after liftoff",
                    landing.time_s, landing.flight_time_s
                );
                value.push_str(&format!(
                    "; {:.2} s from apogee, at {} on average",
                    landing.descent_time_s,
                    speed(landing.mean_descent_rate_m_s)
                ));
                value
            }
            LogReading::Withheld(reading) => withheld(reading),
        },
    ));
    lines.extend(log.notes.iter().map(|note| format!("note: {note}")));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every reason and source the library has maps to the command's own, of the same name: a
    /// library variant added without a mapping would reach the JSON as `other`.
    #[test]
    fn every_library_reason_and_source_has_its_own_name() {
        for reason in Reason::ALL.iter().copied() {
            let withheld = Reading::<()>::Withheld(readings::Withheld {
                reason,
                detail: String::new(),
            });
            let LogReading::Withheld(mapped) = reading(&withheld, |()| ()) else {
                panic!("{reason:?} read");
            };
            assert_ne!(mapped.reason, WithheldReason::Other, "{reason:?}");
            assert_eq!(
                serde_json::to_value(mapped.reason).unwrap(),
                serde_json::to_value(reason).unwrap()
            );
        }
        for from in Source::ALL.iter().copied() {
            let mapped = source(from);
            assert_ne!(mapped, ReadingSource::Other, "{from:?}");
            assert_eq!(
                serde_json::to_value(mapped).unwrap(),
                serde_json::to_value(from).unwrap()
            );
        }
    }
}
