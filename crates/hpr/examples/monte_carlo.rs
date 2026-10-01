//! A Monte Carlo run: one rocket flown 200 times, its mass, drag, motor, wind and rail each
//! scattered a little every time, to see how far its apogee and landing spread.
//!
//! ```text
//! cargo run --example monte_carlo -p hpr
//! ```
//!
//! The documentation site's page *Monte Carlo dispersion* (`docs/monte-carlo.md`) walks through
//! it. What it prints is kept next to it in `monte_carlo.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_analysis::montecarlo::{Dispersion, MonteCarlo};
use hpr::hpr_analysis::statistics::Distribution;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

/// One row of the table: the nominal flight's value, then the spread of the run's.
fn row(name: &str, nominal: f64, spread: &Distribution) -> Result<(), Box<dyn Error>> {
    let summary = spread.summary();
    let number = |x: Option<f64>| x.ok_or("no values");
    println!(
        "{name:<21} {nominal:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>8.1}",
        number(summary.mean)?,
        number(summary.standard_deviation)?,
        number(summary.p05)?,
        number(summary.p50)?,
        number(summary.p95)?,
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket of the `build_and_fly` example on an H54, with its parachute opened at apogee.
    let mut rocket = Rocket::new("My 54 mm rocket", 0.0563)?;
    rocket
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.22,
                0.0015,
                material("abs")?,
            )
            .with_capped_shoulder(0.06, 0.0015),
        )?
        .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
        .add_motor_tube(
            MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?).with_overhang_m(0.005),
        )?
        .add_fins(
            Fins::new(
                3,
                FinPlanform::Trapezoidal {
                    root_chord_m: 0.1,
                    tip_chord_m: 0.04,
                    span_m: 0.045,
                    sweep_m: 0.05,
                },
                0.003175,
                material("birch_plywood")?,
            )
            .with_cross_section(FinCrossSection::Rounded),
        )?
        .add_mass(
            Mass::new(0.2, Position::Top { aft_offset_m: 0.07 })
                .packed(0.15, 0.05)
                .named("recovery bay"),
        )?
        .set_motor(Motor::from_catalog("H54")?)?
        .add_parachute(Device::new(
            "parachute",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
            Trigger::Apogee,
        ));

    // Spaceport America, 1,400 m up, a forecast of 4 m/s from the west, and the rail leaned 5°
    // into it.
    let environment = Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(4.0, 270.0)?;
    let launch = Flight::builder(&rocket, &environment, 1.8)
        .inclination_deg(85.0)
        .heading_deg(270.0);

    // How unsure each input is, as one standard deviation. The motor's 3% is within NFPA 1125's
    // ceiling of 6.7% for a motor type's certification firings; the rest are made up for the
    // example. The motor's ejection delay isn't scattered: the parachute opens at apogee.
    let dispersion = Dispersion {
        dry_mass_sd_fraction: 0.02,
        cg_sd_m: 0.005,
        drag_sd_fraction: 0.05,
        impulse_sd_fraction: 0.03,
        burn_time_sd_fraction: 0.02,
        wind_speed_sd_fraction: 0.25,
        wind_heading_sd_rad: 15_f64.to_radians(),
        rail_elevation_sd_rad: 1_f64.to_radians(),
        rail_azimuth_sd_rad: 2_f64.to_radians(),
        ..Dispersion::default()
    };
    let monte_carlo = MonteCarlo::new(launch.inputs()?, dispersion)?;
    let (seed, count) = (2026, 200);
    let run = monte_carlo.run(seed, count);
    let nominal = monte_carlo.nominal().fly()?;

    println!(
        "{} on a {}, from a 1.8 m rail at 85°, heading west into a 4 m/s west wind",
        rocket.design().name,
        rocket.configuration_id().ok_or("no motor")?
    );
    println!(
        "{count} flights, seed {seed}: {} failed",
        run.failed().count()
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("                       nominal     mean  std dev       5%   median      95%");
    let apogee = run.apogee()?;
    let nominal_apogee = nominal.apogee.as_ref().ok_or("no apogee")?;
    row("apogee (m)", nominal_apogee.height_above_ground_m, &apogee)?;
    let distance = run.distribution(|flight| flight.landing.as_ref().map(|l| l.distance_m))?;
    let nominal_landing = nominal.landing.as_ref().ok_or("no landing")?;
    row(
        "landing distance (m)",
        nominal_landing.distance_m,
        &distance,
    )?;
    let east = run.distribution(|flight| flight.landing.as_ref().map(|l| l.east_m))?;
    row("landing east (m)", nominal_landing.east_m, &east)?;
    println!();
    let share = apogee.share_at_least(1100.0)?.ok_or("no flights")?;
    println!("Reached 1,100 m: {:.1}% of the flights", 100.0 * share.low);
    // Why the mean apogee isn't the nominal one: the nominal flight with 5% less and 5% more
    // drag.
    let height = |drag_scale: f64| -> Result<f64, Box<dyn Error>> {
        let mut inputs = monte_carlo.nominal().clone();
        inputs.drag_scale = drag_scale;
        let flight = inputs.fly()?;
        Ok(flight.apogee.ok_or("no apogee")?.height_above_ground_m)
    };
    let nominal_m = nominal_apogee.height_above_ground_m;
    println!(
        "Apogee with 5% less drag: {:+.0} m; with 5% more: {:+.0} m",
        height(0.95)? - nominal_m,
        height(1.05)? - nominal_m
    );
    Ok(())
}
