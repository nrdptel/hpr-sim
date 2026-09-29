//! Sizing fins: the same rocket built with fins of five spans, weighed, checked for stability
//! and flown in a wind, to see what a bigger fin buys and costs.
//!
//! ```text
//! cargo run --example fin_sizing -p hpr
//! ```
//!
//! The documentation site's page *The builder* (`docs/the-builder.md`) walks through it. What it
//! prints is kept next to it in `fin_sizing.output.txt`, and CI checks that the two still agree
//! (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

/// The rocket of the `build_and_fly` example on an H54, with fins `span_m` tall, and its
/// parachute opened at apogee rather than by the motor's charge, so the charge doesn't cut the
/// climb short. The fins' other sizes stay as they are, so a taller fin is also a larger and
/// heavier one.
fn rocket(span_m: f64) -> Result<Rocket, hpr::Error> {
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
                    span_m,
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
    Ok(rocket)
}

fn main() -> Result<(), Box<dyn Error>> {
    // Spaceport America, 1,400 m up, in calm air and in 5 m/s of wind from the west, from a
    // vertical 1.8 m rail.
    let calm = Environment::new(32.99, -106.97, 1400.0)?;
    let windy = calm.clone().with_constant_wind(5.0, 270.0)?;

    println!("My 54 mm rocket on an H54, fins of five spans, in calm air and a 5 m/s west wind");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("fin span  liftoff  margin   apogee (m)       in the wind: drift (m)");
    println!("    (mm)     (kg)   (cal)   calm   wind   apogee upwind  landing downwind");
    for span_m in [0.025, 0.035, 0.045, 0.055, 0.065] {
        let rocket = rocket(span_m)?;
        // The margin at liftoff, found at Mach 0.3 with the air along the axis.
        let margin = rocket.margin(0.0, 0.3)?;
        let margin_cal = margin.margin_cal.ok_or("no margin")?;
        let mass_kg = rocket.mass_properties(0.0)?.mass_kg;
        let span_mm = span_m * 1000.0;
        // The usual rule of thumb asks for at least one calibre. With less, a gust can leave the
        // rocket pointing away from its path, or tumbling; don't fly it.
        if margin_cal < 1.0 {
            println!("{span_mm:>8.0} {mass_kg:>8.3} {margin_cal:>7.2}   too little margin to fly");
            continue;
        }
        let still = Flight::builder(&rocket, &calm, 1.8).fly()?;
        let flight = Flight::builder(&rocket, &windy, 1.8).fly()?;
        // Where the apogee is over the ground: the launch frame's x points east.
        let apogee = flight
            .result()
            .event(hpr::hpr_sim::EventKind::Apogee)
            .ok_or("no apogee")?;
        println!(
            "{:>8.0} {:>8.3} {:>7.2} {:>6.0} {:>6.0} {:>15.0} {:>17.0}",
            span_mm,
            mass_kg,
            margin_cal,
            still.apogee_m().ok_or("no apogee")?,
            flight.apogee_m().ok_or("no apogee")?,
            -apogee.sample.cg_enu_m.x,
            flight.landing().ok_or("no landing")?.east_m,
        );
    }
    Ok(())
}
