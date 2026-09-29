# Reading a flight log

This page is for anyone with an altimeter's log who wants to know what their rocket did: how high
it went, how fast it climbed, when it landed. hpr reads the log on its own. It needs no design
file and runs no simulation, so it works whatever the rocket was designed in, or if it was never
designed on a computer at all.

**What works today:** logs from PerfectFlite altimeters (the Pnut, the StratoLogger and the
StratoLoggerCF), in their `.pf2` format. Other loggers come with
[M7.1](decisions-and-roadmap.md#m7-1).

**How far to trust it:** each reading comes within a few tenths of a metre and a few hundredths
of a second of the truth on an invented flight whose every number is known. On one real flight it
reads 1,010 ft where the altimeter states 1,009 ft. The rules behind each reading are on
[Flight-log readings](physics/log-readings.md), with what they were checked against. A reading
the log can't support is left out and says why, rather than printed as a number.

## From the command line

```bash
hpr analyze flight.pf2
```

prints the readings as text, or as one JSON document with `--json`.
[`hpr analyze`](cli.md#hpr-analyze) shows its output on an example log and lists the fields.

## From a program

The library is `hpr_flightdata`. It reads a log's text into a record in SI units, then takes the
readings from that record. It doesn't depend on the simulator, so a program that only reads logs
doesn't build one. This program reads the invented log the tests use:

<!-- quote: crates/hpr-flightdata/examples/read_a_log.rs -->
```rust
//! A flight log read on its own: the invented PerfectFlite log the tests use, its readings printed
//! with where each came from, or why it was withheld. No design file, no simulation.
//!
//! It uses only the workspace crate `hpr-flightdata`; a program of your own depends on that one
//! crate, which doesn't pull in the simulator.
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

/// A reading's value as text, or why it was withheld.
fn show<T>(reading: &Reading<T>, value: impl Fn(&T) -> String) -> String {
    match reading {
        Reading::Read(read) => value(read),
        Reading::Withheld(withheld) => {
            format!("withheld ({:?}): {}", withheld.reason, withheld.detail)
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
            "{:.1} m ({:.0} ft) at {:.2} s, source: {:?}",
            apogee.altitude_m,
            apogee.altitude_m / FOOT_M,
            apogee.time_s,
            apogee.source
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
            "{:.1} m/s at {:.2} s, source: {:?}",
            speed.speed_m_s, speed.time_s, speed.source
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
```

It prints:

<!-- quote: crates/hpr-flightdata/examples/read_a_log.output.txt -->
```text
PerfectFlite Pnut: 984 samples
it states an apogee of 1281 ft

liftoff           0.55 s
apogee            390.1 m (1280 ft) at 10.28 s, source: Barometer
highest sample    400.5 m at 11.35 s
top speed         79.9 m/s at 2.10 s, source: LoggerSpeedFromBarometer
top acceleration  withheld (NoAccelerometer): a PerfectFlite logger has no accelerometer; hpr doesn't difference the altitude twice to make one, as its one-foot steps would read as spikes of many g
landing           45.85 s; down from apogee at 10.9 m/s on average
```

## What the readings say

The log is of a flight invented for the tests
([the flight](physics/log-readings.md#checked-against)): 80 m/s at burnout, 2.1 s after the
start of the log, and a coast with no drag to 390.3 m (1,280.5 ft) at 10.26 s.

- **Liftoff**, 0.55 s: the last sample before the altitude shows the rocket moving. The rocket
  left the pad at 0.50 s, but its first 0.15 m rounds to 0 ft.
- **Apogee**, 390.1 m (1,280 ft) at 10.28 s: the top of the altitude after a 0.3 s
  [running median](glossary.md#running-median). It is 0.17 m below the true apogee: the file
  rounds to whole feet.
- **The highest sample**, 400.5 m, a second after apogee, is the ejection charge's pressure
  pulse, not the rocket. The median sets it aside.
- **The top speed**, 79.9 m/s at 2.10 s: the altimeter's own speed column, which it works out
  from its barometer. The true speed at burnout is 80.0 m/s; the file rounds to whole feet per
  second.
- **The top acceleration** is withheld: a PerfectFlite has no accelerometer.
- **Landing**, 45.85 s: the first sample within 2 m of the pad. The rocket touches down 0.29 s
  later, at 6 m/s under its main.
- **The mean descent rate**, 10.9 m/s: the height lost from apogee to landing over the time
  taken, drogue and main together.

What the file states about itself, such as the altimeter's own apogee of 1,281 ft, is kept in
`log.stated`, beside hpr's readings and never in their place.

## What it doesn't do yet

- Read other loggers' files ([M7.1](decisions-and-roadmap.md#m7-1)).
- Split the descent into the drogue's and the main's rates, find the deployments, or give the
  Mach number and dynamic pressure ([M7.2](decisions-and-roadmap.md#m7-2)).
- Compare a flight with its simulation ([M7.3](decisions-and-roadmap.md#m7-3)).
- Correct a barometric altitude for the day's air. The altitude is the altimeter's own
  conversion, which assumes a standard atmosphere
  ([Barometric altimeter](glossary.md#barometric-altimeter)).
