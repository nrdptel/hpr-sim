# Exporting a flight

This page shows how to save a flight as files other programs read: a table of numbers over time
for a spreadsheet or a plotting tool (CSV or JSON), and a map of the flight path and the landing
(GeoJSON or KML, which Google Earth, QGIS and most web maps open). It flies the rocket of
[Getting started](getting-started.md) again and writes all four files. It needs the first page's
setup, and a little Rust.

> **The files are exact, the flight is not validated.** Every number in a file reads back to
> exactly the value the simulator computed, and tests check that. The flight itself is the first
> flight's, and [How far to trust it](getting-started.md#how-far-to-trust-it) on that page
> applies to it too.

## Run it

```bash
cargo run --example export_flight -p hpr-sim -- my-flight
```

This runs
[`crates/hpr-sim/examples/export_flight.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/export_flight.rs),
which writes four files into the folder `my-flight` (without a folder, into `hpr-sim-export` in
the system's temporary folder) and prints what it wrote:

<!-- quote: crates/hpr-sim/examples/export_flight.output.txt -->
```text
wrote 67 rows of 5 columns, a path of 67 points
  flight.csv
  flight.json
  flight.geojson
  flight.kml
landed at 32.99000° N, 106.96904° W, 90 m from the pad, at 58.9 s
```

CI runs it on macOS, Windows and Linux and fails if it prints anything else. The files hold every
number to its last digit, which may differ in the last place between operating systems, so the
program prints a rounded summary instead of the files.

## The four files

| file | what it holds | opens in |
|---|---|---|
| `flight.csv` | a header naming each column with its unit (`time_s`, `cg_east_m`, ...), then one line per recorded moment, lines ending in CRLF as RFC 4180 says | a spreadsheet, pandas, any plotting tool |
| `flight.json` | the same table as `{"columns": [...], "rows": [[...], ...]}` | any programming language |
| `flight.geojson` | the flight path as a line, and a point for each landing: the rocket's and any separated body's | QGIS, geojson.io, web maps |
| `flight.kml` | the same path and landing | Google Earth |

The tables hold whatever [channels](recording-a-trajectory.md#record-something-else) the
recorder kept, one row per recorded moment. The maps need the recorder to keep the time and the
centre of gravity's position (`Channel::Time` and `Channel::CgPosition`); they place each recorded
position on the Earth, with the same conversion as the landing point in
[Flight metrics](physics/metrics.md).

A map line in KML looks like this, one `longitude,latitude,height` per recorded moment (the digits
here are cut short):

```xml
<LineString>
<altitudeMode>absolute</altitudeMode>
<coordinates>
-106.97,32.99,1400.93591...
-106.97,32.98999...,1403.85869...
```

## Heights: two datums

A height needs something to count from, a datum. The two map formats use different ones, because
their standards say so:

- **GeoJSON** heights are above the WGS 84 ellipsoid, the smooth shape GPS uses
  ([ellipsoidal height](glossary.md#ellipsoidal-height);
  [RFC 7946](https://www.rfc-editor.org/rfc/rfc7946#section-4), section 4).
- **KML** heights with `altitudeMode` `absolute` are above sea level, which KML takes from the
  EGM96 geoid ([height above sea level](glossary.md#height-above-sea-level-msl);
  [OGC KML 2.2, 07-147r2](https://www.ogc.org/standard/kml/)).

Sea level sits above or below the ellipsoid by the
[geoid undulation](glossary.md#height-above-sea-level-msl) `N`, up to about 100 m. hpr has no
model of it, so it uses the value the flight was given (`Environment::with_geoid_undulation_m`,
zero unless set), the same for every point of the flight. The geoid's slope, about 5 cm per
kilometre and up to some 30 cm in mountains, moves it by centimetres to decimetres over a rocket's
few kilometres. For example, at a site where sea level
is 25 m below the ellipsoid (`N = −25` m), a point 1500 m above the ellipsoid is written as 1500 m
in GeoJSON and as 1525 m in KML. The first flight leaves `N` at zero, so its two files agree; at the real
site sea level is some tens of metres below the ellipsoid, so give `N` for heights you mean to
trust.

## How the files are checked

- **Numbers:** each is written in the shortest form that reads back to the same number, and the
  tests read every file back and compare it with the recording exactly, not within a tolerance. A
  value that isn't a finite number is refused with an error rather than written as `NaN` or
  `null`, which different programs read differently.
- **GeoJSON:** checked against the published GeoJSON schema
  ([geojson.org/schema](https://geojson.org/schema/FeatureCollection.json)), with longitude
  before latitude as the standard requires. A test also shows the check rejects a broken file.
- **KML:** parsed by a strict XML parser, and checked for the KML 2.2 namespace, the height mode
  and every coordinate.

The tests are in
[`crates/hpr-sim/src/export.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/src/export.rs).

## What it leaves out

- **Parquet**, a compact table format for large runs such as the planned Monte Carlo, comes in
  [M1.10c2](decisions-and-roadmap.md#m1-10c2).
- **A path over the antimeridian** (±180° longitude) is not cut in two as RFC 7946 asks, so a map
  would draw it the long way round the globe.
- **Landing heights:** a landing is a point on the ground, without a height.
- **A summary file:** the flight's peaks and margins ([Flight metrics](physics/metrics.md)) are
  not in these files. A `FlightSummary` converts to JSON on its own with `serde_json::to_string`,
  but that path writes a value that isn't finite as `null` rather than refusing it.

The choices are recorded in
[ADR-079: exports as text built in the core](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-079-exports-as-text-built-in-the-core-heights-on-each-formats-own-datum-2026-09-26).

## The program

This is the whole program, line for line the file CI runs. Everything above the recorder is the
first flight's setup, which [Getting started](getting-started.md#the-program-step-by-step)
explains step by step.

<!-- quote: crates/hpr-sim/examples/export_flight.rs -->
```rust
//! A flight written out for other programs: the flight of `first_flight.rs`, recorded every 1 s
//! and at every event, saved as CSV and JSON tables and as a GeoJSON and a KML map of its path and
//! landing.
//!
//! Run it from anywhere in the repository, naming the folder to write the four files to:
//!
//! ```text
//! cargo run --example export_flight -p hpr-sim -- my-flight
//! ```
//!
//! Without a folder it writes them to `hpr-sim-export` in the system's temporary folder. The
//! documentation site's *Exporting a flight* page (`docs/exporting-a-flight.md`) walks through it.
//! What it prints is kept next to it in `export_flight.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]
#![allow(
    clippy::disallowed_methods,
    reason = "the library builds each file's text without I/O; this program writes the files"
)]

use std::error::Error;
use std::path::PathBuf;

use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::{
    CanopyType, Channel, Device, DeviceDrag, Environment, FlightMetrics, FlightSettings, Rail,
    Recorder, Simulation, Termination, Trigger, export,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The first flight: Valetudo on a K400C, from a 3 m vertical rail in New Mexico, in 5 m/s of
    // wind from the west, with a drogue at apogee and a main at 150 m.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let environment =
        Environment::standard(site)?.with_wind(ConstantWind::new(5.0, 270_f64.to_radians())?);
    let simulation = Simulation::new(
        &rocket,
        "example",
        environment,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?
    .with_recovery(vec![
        Device::new(
            "drogue",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.6),
            Trigger::Apogee,
        )
        .with_lag_s(0.5),
        Device::new(
            "main",
            DeviceDrag::canopy(CanopyType::FlatCircular, 2.4),
            Trigger::Altitude {
                height_above_ground_m: 150.0,
            },
        )
        .with_lag_s(1.0),
    ])?;

    // Two watchers on one flight: the metrics, for the landing, and a recorder of the time, the
    // height above the pad and the centre of gravity's position, which a map needs.
    let recorder = Recorder::new(
        vec![
            Channel::Time,
            Channel::HeightAboveGround,
            Channel::CgPosition,
        ],
        Some(1.0),
    )?;
    let mut watchers = (FlightMetrics::new(), recorder);
    let flight = simulation.run(&mut watchers)?;
    if flight.termination != Termination::GroundHit {
        return Err(format!("the flight ended with {:?}", flight.termination).into());
    }
    let (metrics, recorder) = watchers;
    let summary = metrics.summary(&flight, simulation.environment())?;

    // The path on the Earth, then the four files. Each function returns the file's text.
    let track = export::track(&recorder, simulation.environment())?;
    let files = [
        ("flight.csv", export::csv(&recorder)?),
        ("flight.json", export::json(&recorder)?),
        ("flight.geojson", export::geojson(&track, &summary)?),
        (
            "flight.kml",
            export::kml(&track, &summary, "Valetudo, K400C")?,
        ),
    ];
    let folder = match std::env::args_os().nth(1) {
        Some(folder) => PathBuf::from(folder),
        None => std::env::temp_dir().join("hpr-sim-export"),
    };
    std::fs::create_dir_all(&folder)?;
    for (name, text) in &files {
        std::fs::write(folder.join(name), text)?;
    }

    // What was written, and where it landed, to five decimal places of a degree (about a metre).
    // The site is west of Greenwich, so the longitude is printed as degrees west.
    println!(
        "wrote {} rows of {} columns, a path of {} points",
        recorder.rows().len(),
        recorder.columns().len(),
        track.len()
    );
    for (name, _) in &files {
        println!("  {name}");
    }
    if let Some(landing) = summary.landing {
        println!(
            "landed at {:.5}° N, {:.5}° W, {:.0} m from the pad, at {:.1} s",
            landing.latitude_deg, -landing.longitude_deg, landing.distance_m, landing.time_s
        );
    }
    Ok(())
}
```
