//! How much the drag matters: the rocket of `first_flight.rs`, from the same rail in the same wind,
//! flown with hpr's own drag and then with the drag coefficients of two other sources for it.
//!
//! ```text
//! cargo run --example drag_what_if -p hpr-sim
//! ```
//!
//! The *Getting started* page (`docs/getting-started.md`) quotes what it prints, which is kept in
//! `drag_what_if.output.txt` and checked in CI (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_aero::DragTable;
use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_core::interp::{Extrapolation, Interpolation, Table1D};
use hpr_design::Rocket;
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};

fn main() -> Result<(), Box<dyn Error>> {
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let environment =
        Environment::standard(site)?.with_wind(ConstantWind::new(5.0, 270_f64.to_radians())?);

    // The height of the apogee above the pad, m, with hpr's own drag or with `drag` in its place.
    let apogee_m = |drag: Option<DragTable>| -> Result<f64, Box<dyn Error>> {
        let mut simulation = Simulation::new(
            &rocket,
            "example",
            environment.clone(),
            Rail::vertical(3.0),
            FlightSettings::default(),
        )?;
        if let Some(table) = drag {
            simulation = simulation.with_drag_table(table);
        }
        let flight = simulation.run(&mut ())?;
        let apogee = flight
            .event(EventKind::Apogee)
            .ok_or("the flight has no apogee")?;
        Ok(apogee.sample.height_above_ground_m)
    };

    println!("Valetudo's apogee under three drag models, from a 3 m rail in 5 m/s of wind");
    println!();
    println!("drag coefficient                             apogee (m)");
    println!(
        "{:<44} {:>9.1}",
        "hpr's own, from the design",
        apogee_m(None)?
    );
    // A drag table gives the drag coefficient against Mach number, in place of hpr's own. Each of
    // these holds one source's value at Mach 0.3 at every Mach number (this flight peaks at
    // Mach 0.36), with the motor burning or not. hpr's own drag varies with Mach number and is
    // lower while the motor burns, but that matters little here: flown the same way, as a
    // constant table, hpr's own value at Mach 0.3 (0.5566, motor off) gives an apogee 0.5 m
    // lower than its own drag does. So the gaps below come almost entirely from the drag values.
    for (drag_coefficient, source) in [
        (0.728, "the rocket's OpenRocket file"),
        (1.05, "RocketPy's example curve"),
    ] {
        let table = Table1D::new(
            vec![0.0, 1.0],
            vec![drag_coefficient, drag_coefficient],
            Interpolation::Linear,
            Extrapolation::Clamp,
        )?;
        println!(
            "{:<44} {:>9.1}",
            format!("{drag_coefficient}, from {source}"),
            apogee_m(Some(DragTable::new(table, None)))?
        );
    }
    Ok(())
}
