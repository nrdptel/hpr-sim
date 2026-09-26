//! A flight written out for other programs: the flight of `first_flight.rs`, recorded every 1 s
//! and at every event, saved as CSV, JSON and Parquet tables and as a GeoJSON and a KML map of its
//! path and landing.
//!
//! Run it from anywhere in the repository, naming the folder to write the five files to. Parquet
//! is an optional feature of the library, so the command turns it on:
//!
//! ```text
//! cargo run --example export_flight -p hpr-sim --features parquet -- my-flight
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

    // The path on the Earth, then the five files. Each function returns the file's contents:
    // text, or bytes for Parquet, which is a binary format.
    let track = export::track(&recorder, simulation.environment())?;
    let files = [
        ("flight.csv", export::csv(&recorder)?.into_bytes()),
        ("flight.json", export::json(&recorder)?.into_bytes()),
        ("flight.parquet", export::parquet(&recorder)?),
        (
            "flight.geojson",
            export::geojson(&track, &summary)?.into_bytes(),
        ),
        (
            "flight.kml",
            export::kml(&track, &summary, "Valetudo, K400C")?.into_bytes(),
        ),
    ];
    let folder = match std::env::args_os().nth(1) {
        Some(folder) => PathBuf::from(folder),
        None => std::env::temp_dir().join("hpr-sim-export"),
    };
    std::fs::create_dir_all(&folder)?;
    for (name, contents) in &files {
        std::fs::write(folder.join(name), contents)?;
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
