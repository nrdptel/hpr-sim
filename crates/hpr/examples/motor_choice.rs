//! Which motor: one rocket flown on each 29 mm motor in the built-in catalog, with the ejection
//! delay each one needs.
//!
//! ```text
//! cargo run --example motor_choice -p hpr
//! ```
//!
//! The documentation site's page *The builder* (`docs/the-builder.md`) quotes what it prints,
//! which is kept in `motor_choice.output.txt` and checked in CI (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_sim::metrics::optimum_delays;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, Flight, Motor, NoseShape,
    Position, Rocket, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket of `build_and_fly`, without its motor for now.
    let mut rocket = Rocket::new("My 29 mm rocket", 0.0563)?;
    rocket
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.22,
                0.0015,
                material("abs")?,
            )
            .with_shoulder(0.06, 0.0015, true),
        )?
        .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
        .add_motor_tube(
            MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?).with_overhang_m(0.005),
        )?
        .add_fins(
            Fins::trapezoidal(
                3,
                [0.1, 0.04, 0.045, 0.05],
                0.003175,
                material("birch_plywood")?,
            )
            .with_cross_section(FinCrossSection::Rounded),
        )?
        .add_mass(Mass::new(0.2, Position::Top { aft_offset_m: 0.07 }).packed(0.15, 0.05))?
        // This time the parachute opens at apogee, whatever the motor's delay.
        .add_parachute(Device::new(
            "parachute",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
            Trigger::Apogee,
        ));
    let environment = Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(5.0, 270.0)?;

    println!("My 29 mm rocket from a 1.8 m vertical rail, in 5 m/s of wind from the west");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("motor        liftoff  margin  rail exit   apogee  top speed  best delay");
    println!("               (kg)   (cal)     (m/s)      (m)     (m/s)        (s)");
    // The two 29 mm motors with a bundled thrust curve. `set_motor` swaps the motor in the tube.
    for name in ["F52C", "168H54-10A"] {
        rocket.set_motor(Motor::from_catalog(name)?)?;
        let mass_kg = rocket.mass_properties(0.0)?.mass_kg;
        let margin_cal = rocket.static_margin_cal(0.0, 0.3)?;
        let launch = Flight::builder(&rocket, &environment, 1.8);
        let flight = launch.fly()?;
        // The best delay: from burnout to apogee, with the parachute held. The builder has no
        // method for it, but hands over the simulation it flies, and `hpr_sim` has one.
        let delays = optimum_delays(&launch.simulation()?)?.ok_or("no apogee")?;
        let best_s = delays.first().ok_or("no burnout before apogee")?.delay_s;
        println!(
            "{name:<12} {mass_kg:>7.3} {margin_cal:>7.2} {:>10.1} {:>8.1} {:>10.0} {best_s:>11.1}",
            flight.rail_exit_speed_m_s().ok_or("no rail exit")?,
            flight.apogee_m().ok_or("no apogee")?,
            flight.max_speed_m_s().ok_or("no top speed")?,
        );
    }
    Ok(())
}
