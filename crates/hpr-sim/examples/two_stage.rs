//! A two-stage flight: the booster burns out, the stages come apart half a second later, and the
//! sustainer lights a second after the booster's burnout and flies on while the booster tumbles
//! back to the ground.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example two_stage -p hpr-sim
//! ```
//!
//! The documentation site's staging page (`docs/physics/staging.md`) walks through it. What it
//! prints is kept next to it in `two_stage.output.txt`, and CI checks that the two still agree
//! (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_core::geodesy::Geodetic;
use hpr_design::{Ignition, Rocket};
use hpr_sim::{
    CanopyType, Device, DeviceDrag, Environment, EventKind, FlightSettings, Rail, Separation,
    Simulation, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket: a synthetic two-stage design, a 54 mm sustainer (stage 0, with the nose) on a
    // 75 mm booster (stage 1). Its configuration "j760-i175" puts a J760 in the booster and an
    // I175 in the sustainer.
    let mut rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/synthetic-two-stage-75mm-54mm.json"
    ))?;
    // The design file lights both motors at launch. Light the sustainer 1 s after the booster
    // burns out instead.
    let configuration = rocket
        .configurations
        .iter_mut()
        .find(|configuration| configuration.id == "j760-i175")
        .ok_or("no configuration j760-i175")?;
    let sustainer_motor = configuration
        .motors
        .iter_mut()
        .find(|motor| motor.mount == "sustainer-motor-mount")
        .ok_or("no sustainer motor")?;
    sustainer_motor.ignition = Ignition::Burnout {
        mount: "booster-motor-mount".to_owned(),
        delay_s: 1.0,
    };

    // Calm air over a site in New Mexico 1,400 m up, and a 6 m vertical rail.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let simulation = Simulation::new(
        &rocket,
        "j760-i175",
        Environment::standard(site)?,
        Rail::vertical(6.0),
        FlightSettings::default(),
    )?;
    let booster = simulation
        .assembly()
        .motors
        .iter()
        .position(|motor| motor.mount == "booster-motor-mount")
        .ok_or("no booster motor")?;

    // Recovery: a 1.2 m parachute on the sustainer at its apogee, and the booster tumbling from
    // its own apogee on its own stage's geometry. The stages come apart 0.5 s after the booster
    // burns out, at the boundary after stage 0.
    let tumble = DeviceDrag::tumbling_stages(simulation.assembly(), (1, 1))?;
    let simulation = simulation
        .with_recovery(vec![
            Device::new(
                "parachute",
                DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
                Trigger::Apogee,
            ),
            Device::new("booster tumble", tumble, Trigger::Apogee).on_body(1),
        ])?
        .with_separation(Separation::new(
            Trigger::Burnout {
                motor: booster,
                delay_s: 0.5,
            },
            0,
        ))?;

    let flight = simulation.run(&mut ())?;

    println!("A 54 mm sustainer on a 75 mm booster, J760 then I175, calm air");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("event                   time (s)   CG height (m)   speed (m/s)   mass (kg)");
    for event in &flight.events {
        let name = match event.kind {
            EventKind::Liftoff => "liftoff".to_owned(),
            EventKind::RailExit => "rail exit".to_owned(),
            EventKind::Separation => "separation".to_owned(),
            EventKind::Ignition(_) => "sustainer lights".to_owned(),
            EventKind::Burnout => "sustainer burnout".to_owned(),
            EventKind::Apogee => "apogee".to_owned(),
            EventKind::Trigger(i) => format!("{} charge", simulation.recovery()[i].name),
            EventKind::Deployment(i) => format!("{} opens", simulation.recovery()[i].name),
            EventKind::GroundHit => "sustainer lands".to_owned(),
            other => format!("{other:?}"),
        };
        let sample = &event.sample;
        // The flight ends a micrometre below the ground: print that as 0 (`+ 0.0` turns a -0
        // into 0).
        println!(
            "{name:<23} {:>8.2} {:>15.1} {:>13.1} {:>11.3}",
            sample.time_s,
            sample.height_above_ground_m.max(0.0) + 0.0,
            sample.cg_velocity_enu_m_s.length(),
            sample.mass_kg,
        );
    }

    println!();
    for body in &flight.bodies {
        println!(
            "The booster ({:.3} kg) leaves at {:.2} s and {:.1} m, and lands at {:.2} s at \
             {:.1} m/s.",
            body.mass_kg,
            body.start_sample.time_s,
            body.start_sample.height_above_ground_m,
            body.final_sample.time_s,
            body.final_sample.cg_velocity_enu_m_s.length(),
        );
    }
    Ok(())
}
