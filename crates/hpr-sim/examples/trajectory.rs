//! A trajectory to plot: the flight of `first_flight.rs`, recorded every 5 s and at every event,
//! printed as CSV (comma-separated values), which a spreadsheet or a plotting tool reads.
//!
//! Run it from anywhere in the repository, and save what it prints to a file:
//!
//! ```text
//! cargo run --example trajectory -p hpr-sim > trajectory.csv
//! ```
//!
//! The documentation site's *Recording a trajectory* page (`docs/recording-a-trajectory.md`)
//! walks through it. What it prints is kept next to it in `trajectory.output.txt`, and CI checks
//! that the two still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::{
    CanopyType, Channel, Device, DeviceDrag, Environment, FlightSettings, Rail, Recorder,
    Simulation, Termination, Trigger,
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

    // What to keep, and how often: the time, the height above the pad, the vertical speed, the
    // airspeed and where the centre of gravity is (metres east, north and up of the pad), every
    // 5 s. A recorder also keeps a row at every event, such as burnout or a parachute opening.
    // For a smooth plot, use `Some(0.1)`; `None` keeps a row at the end of every step.
    let mut recorder = Recorder::new(
        vec![
            Channel::Time,
            Channel::HeightAboveGround,
            Channel::VerticalSpeed,
            Channel::Airspeed,
            Channel::CgPosition,
        ],
        Some(5.0),
    )?;
    let flight = simulation.run(&mut recorder)?;
    if flight.termination != Termination::GroundHit {
        return Err(format!("the flight ended with {:?}", flight.termination).into());
    }

    // One header line with each column's name and unit, then one line per row: the time to
    // 0.001 s, the rest to 0.1.
    println!("{}", recorder.columns().join(","));
    for row in recorder.rows() {
        let fields: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(column, &value)| rounded(value, if column == 0 { 3 } else { 1 }))
            .collect();
        println!("{}", fields.join(","));
    }
    Ok(())
}

/// `value` to `decimals` places, without the minus sign of a value that rounds to zero. Some values
/// that are zero in principle come out a hair below it: the vertical speed at ignition and at
/// apogee, the landing height, which is found just below the ground, and the landing's `cg_up_m`,
/// under a millimetre below the pad's level because the ground curves away.
fn rounded(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    match text.strip_prefix('-') {
        Some(unsigned) if unsigned.chars().all(|c| c == '0' || c == '.') => unsigned.to_owned(),
        _ => text,
    }
}
