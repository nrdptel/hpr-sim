//! A rocket built with the builder API, part by part, then weighed, checked for stability and
//! flown in a wind under a parachute.
//!
//! ```text
//! cargo run --example build_and_fly -p hpr
//! ```
//!
//! The documentation site's page *The builder* (`docs/the-builder.md`) walks through it. What it
//! prints is kept next to it in `build_and_fly.output.txt`, and CI checks that the two still
//! agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_sim::EventKind;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // A Cesaroni H54 from the built-in catalog, with a 10 s delay. Its thrust curve ends, and its
    // propellant is spent, at `end_s`.
    let motor = Motor::from_catalog("H54")?.with_delay_s(10.0)?;
    let end_s = motor.solid_motor().curve().end_time_s();

    // The rocket: 56.3 mm across outside (a 54 mm airframe), its parts listed from the nose back.
    // Every part names its material, from the built-in list (`hpr_design::materials`), and its
    // wall.
    let mut rocket = Rocket::new("My 54 mm rocket", 0.0563)?;
    rocket
        // A 22 cm tangent ogive of ABS with a 1.5 mm wall, and a 6 cm shoulder, capped, that
        // fits inside the tube behind it.
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.22,
                0.0015,
                material("abs")?,
            )
            .with_capped_shoulder(0.06, 0.0015),
        )?
        // 90 cm of kraft phenolic tube with a 1.15 mm wall. What follows goes on or in it.
        .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
        // A 20 cm motor tube with a 29 mm bore, flush with the tube's aft end; the motor's
        // nozzle sticks out 5 mm past it.
        .add_motor_tube(
            MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?).with_overhang_m(0.005),
        )?
        // Three trapezoidal fins of 1/8 in birch plywood with rounded edges, flush with the aft
        // end. The tip's leading edge is 5 cm aft of the root's.
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
        // The parachute, shock cord and altimeter, as one 200 g mass packed in a cylinder 15 cm
        // long and 5 cm across, its top 7 cm below the top of the tube.
        .add_mass(
            Mass::new(0.2, Position::Top { aft_offset_m: 0.07 })
                .packed(0.15, 0.05)
                .named("recovery bay"),
        )?
        // The motor, in the motor tube.
        .set_motor(motor)?
        // A 90 cm parachute, opened by the ejection charge of the first motor, number 0.
        .add_parachute(Device::new(
            "parachute",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
            Trigger::MotorDelay { motor: 0 },
        ));

    // Weighed at liftoff and with the propellant spent. The centre of gravity is in the body
    // frame, where a point s metres aft of the nose tip is at z = -s; the centre of pressure is a
    // station, metres aft of the tip. Both are found at Mach 0.3, with the air along the axis.
    println!(
        "{} on a {}",
        rocket.design().name,
        rocket.configuration_id().ok_or("no motor")?
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("                                 liftoff     spent");
    let (liftoff, spent) = (rocket.margin(0.0, 0.3)?, rocket.margin(end_s, 0.3)?);
    let (full, empty) = (rocket.mass_properties(0.0)?, rocket.mass_properties(end_s)?);
    println!(
        "mass (kg)                        {:>7.3} {:>9.3}",
        full.mass_kg, empty.mass_kg
    );
    println!(
        "centre of gravity (m from nose)  {:>7.3} {:>9.3}",
        -full.cg_m.z, -empty.cg_m.z
    );
    let cp = |margin: &hpr::hpr_sim::metrics::Margin| margin.cp_station_m.ok_or("no normal force");
    println!(
        "centre of pressure (m from nose) {:>7.3} {:>9.3}",
        cp(&liftoff)?,
        cp(&spent)?
    );
    let calibres = |margin: &hpr::hpr_sim::metrics::Margin| margin.margin_cal.ok_or("no margin");
    println!(
        "stability margin (calibres)      {:>7.2} {:>9.2}",
        calibres(&liftoff)?,
        calibres(&spent)?
    );

    // Where: Spaceport America, 1,400 m up, with 5 m/s of wind from the west. Longitude is
    // positive east, so this one is negative.
    let environment = Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(5.0, 270.0)?;
    // The flight: from a 1.8 m rail, 85° above the horizon, leaning toward 270°, the west: 5°
    // into the wind.
    let flight = Flight::builder(&rocket, &environment, 1.8)
        .inclination_deg(85.0)
        .heading_deg(270.0)
        .fly()?;

    println!();
    println!("From a 1.8 m rail leaning 5° west, in 5 m/s of wind from the west:");
    let rail_exit_m_s = flight.rail_exit_speed_m_s().ok_or("no rail exit")?;
    println!("Rail exit:  {rail_exit_m_s:.1} m/s");
    let (apogee_m, apogee_s) = (
        flight.apogee_m().ok_or("no apogee")?,
        flight.apogee_time_s().ok_or("no apogee")?,
    );
    // Where the apogee is over the ground: the launch frame's x points east, y north.
    let over = flight
        .result()
        .event(EventKind::Apogee)
        .ok_or("no apogee")?
        .sample
        .cg_enu_m;
    println!(
        "Apogee:     {apogee_m:.1} m above the pad, at {apogee_s:.2} s, {:.0} m west of it",
        -over.x
    );
    let (speed_m_s, mach) = (
        flight.max_speed_m_s().ok_or("no top speed")?,
        flight.max_mach().ok_or("no top speed")?,
    );
    println!("Top speed:  {speed_m_s:.0} m/s (Mach {mach:.2})");
    let landing = flight.landing().ok_or("no landing")?;
    println!(
        "Landing:    {:.0} m east of the pad, at {:.1} m/s, at {:.1} s",
        landing.east_m, landing.descent_rate_m_s, landing.time_s,
    );
    Ok(())
}
