//! A flight log read on its own: the invented PerfectFlite log the tests use, its readings printed
//! with where each came from, or why it was withheld. No design file, no simulation.
//!
//! It uses the workspace crate `hpr-flightdata`, which doesn't pull in the simulator, and `serde`
//! and `serde_json` to print each code as the JSON output spells it.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example read_a_log -p hpr-flightdata
//! ```
//!
//! The guide's page *Reading a flight log* (`docs/reading-a-flight-log.md`) quotes it and what it
//! prints, which is kept next to it in `read_a_log.output.txt`; CI checks that the two still agree
//! (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_flightdata::perfectflite::{self, FOOT_M};
use hpr_flightdata::readings::{self, Reading};

/// The invented log: a Pnut's file of a flight made up for the tests.
const LOG: &str = include_str!("../../../validation/fixtures/logs/synthetic-pnut.pf2");

/// A code as the JSON output spells it, such as `no_accelerometer`.
fn code(value: impl serde::Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|json| json.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// A reading's value as text, or why it was withheld.
fn show<T>(reading: &Reading<T>, value: impl Fn(&T) -> String) -> String {
    match reading {
        Reading::Read(read) => value(read),
        Reading::Withheld(withheld) => {
            format!("withheld ({}): {}", code(withheld.reason), withheld.detail)
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let log = perfectflite::read(LOG)?;
    println!("{}: {} samples", log.logger, log.time_s.len());
    if let Some(stated) = log.stated.apogee_m {
        println!("it states an apogee of {:.0} ft", stated / FOOT_M);
    }

    let read = readings::read(&log);
    println!();
    println!(
        "liftoff           {}",
        show(&read.liftoff, |liftoff| format!("{:.2} s", liftoff.time_s))
    );
    println!(
        "apogee            {}",
        show(&read.apogee, |apogee| format!(
            "{:.1} m ({:.0} ft) at {:.2} s, source: {}",
            apogee.altitude_m,
            apogee.altitude_m / FOOT_M,
            apogee.time_s,
            code(apogee.source)
        ))
    );
    println!(
        "highest sample    {}",
        show(&read.apogee, |apogee| format!(
            "{:.1} m at {:.2} s",
            apogee.highest_sample.altitude_m, apogee.highest_sample.time_s
        ))
    );
    println!(
        "top speed         {}",
        show(&read.max_speed, |speed| format!(
            "{:.1} m/s at {:.2} s, source: {}",
            speed.speed_m_s,
            speed.time_s,
            code(speed.source)
        ))
    );
    println!(
        "top acceleration  {}",
        show(&read.max_acceleration, |top| format!(
            "{:.1} m/s²",
            top.acceleration_m_s2
        ))
    );
    println!(
        "landing           {}",
        show(&read.landing, |landing| format!(
            "{:.2} s; down from apogee at {:.1} m/s on average",
            landing.time_s, landing.mean_descent_rate_m_s
        ))
    );
    Ok(())
}
