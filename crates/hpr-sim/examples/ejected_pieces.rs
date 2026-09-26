//! A rocket that comes apart into three pieces: its nose cone is pushed off at apogee, and a
//! payload leaves the airframe at 300 m. Each piece comes down under its own parachute and lands
//! somewhere of its own.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example ejected_pieces -p hpr-sim
//! ```
//!
//! The documentation site's recovery page (`docs/physics/recovery.md`, *Ejected pieces*) walks
//! through it. What it prints is kept next to it in `ejected_pieces.output.txt`, and CI checks
//! that the two still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_design::{Part, Position, Rocket};
use hpr_sim::{
    CanopyType, Device, DeviceDrag, Ejection, Environment, EventKind, FlightSettings, Rail,
    Simulation, Trigger, terminal_speed_m_s,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket: the synthetic 54 mm single-stage design on an I175.
    let mut rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/synthetic-54mm-three-fin.json"
    ))?;
    // Carry a 250 g payload in the airframe, 0.15 m aft of its forward end. It is an ordinary
    // mass component, made here from the design's altimeter.
    let airframe = rocket.stages[0]
        .components
        .iter_mut()
        .find(|component| component.id == "sustainer-airframe")
        .ok_or("no airframe")?;
    let mut payload = airframe
        .children
        .iter()
        .find(|child| child.id == "altimeter")
        .cloned()
        .ok_or("no altimeter")?;
    payload.id = "payload".to_owned();
    if let Part::MassComponent(mass) = &mut payload.part {
        mass.mass_kg = 0.25;
    }
    payload.position = Some(Position::Top { aft_offset_m: 0.15 });
    airframe.children.push(payload);

    // A 4 m/s wind from the west over a site in New Mexico 1,400 m up, and a 3 m vertical rail.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let wind = ConstantWind::new(4.0, 270_f64.to_radians())?;
    let simulation = Simulation::new(
        &rocket,
        "i175",
        Environment::standard(site)?.with_wind(wind),
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?;

    // Two partings. At apogee the airframe parts at the joint aft of the nose cone: the nose
    // cone keeps body 0 and the airframe becomes body 1. At 300 m on the way down the payload
    // leaves the airframe as body 2. Each body has its own parachute.
    let at_300_m = Trigger::Altitude {
        height_above_ground_m: 300.0,
    };
    let canopy = |name: &str, diameter_m: f64, trigger: Trigger, body: usize| {
        Device::new(
            name,
            DeviceDrag::canopy(CanopyType::FlatCircular, diameter_m),
            trigger,
        )
        .on_body(body)
    };
    let simulation = simulation
        .with_recovery(vec![
            canopy("nose cone chute", 0.45, Trigger::Apogee, 0),
            canopy("airframe chute", 0.9, Trigger::Apogee, 1),
            canopy("payload chute", 0.6, at_300_m, 2),
        ])?
        .with_ejections(vec![
            Ejection::aft_of(Trigger::Apogee, "nose"),
            Ejection::payload(at_300_m, "payload"),
        ])?;

    let flight = simulation.run(&mut ())?;

    println!("A 54 mm rocket on an I175 in a 4 m/s west wind, ejecting its nose cone and a payload");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    for event in &flight.events {
        let name = match event.kind {
            EventKind::Liftoff => "liftoff".to_owned(),
            EventKind::RailExit => "rail exit".to_owned(),
            EventKind::Burnout => "burnout".to_owned(),
            EventKind::Apogee => "apogee".to_owned(),
            EventKind::Ejection(0) => "nose cone ejected".to_owned(),
            EventKind::Trigger(i) => format!("{} charge", simulation.recovery()[i].name),
            EventKind::Deployment(i) => format!("{} opens", simulation.recovery()[i].name),
            other => format!("{other:?}"),
        };
        println!(
            "{name:<26} {:>7.2} s {:>8.1} m",
            event.sample.time_s, event.sample.height_above_ground_m
        );
    }

    println!();
    let names = ["nose cone", "airframe", "payload"];
    // Each piece's terminal speed, `√(2 m g/(ρ C_D S))`, in the air and gravity at the ground,
    // to set beside the speed it lands at.
    let environment = simulation.environment();
    let density_kg_m3 = environment.atmosphere.air(site.height_m)?.air.density_kg_m3;
    println!(
        "body         mass (kg)   flies from (s)   lands (s)   at (m/s)   terminal (m/s)   east (m)   north (m)"
    );
    for (body, name) in flight.bodies.iter().zip(names) {
        let landing = body.final_sample;
        let drag_area_m2 = simulation.recovery()[body.body].drag.drag_area_m2();
        let gravity_m_s2 = environment
            .earth
            .gravity_enu_mps2(landing.cg_enu_m)?
            .length();
        println!(
            "{} {name:<10} {:>9.3} {:>16.2} {:>11.2} {:>10.2} {:>16.2} {:>10.1} {:>11.1}",
            body.body,
            body.mass_kg,
            body.start_sample.time_s,
            landing.time_s,
            -landing.vertical_speed_m_s,
            terminal_speed_m_s(body.mass_kg, drag_area_m2, density_kg_m3, gravity_m_s2),
            landing.cg_enu_m.x,
            landing.cg_enu_m.y,
        );
    }
    let airframe = flight.bodies.get(1).ok_or("no airframe")?;
    let parting = airframe
        .event(EventKind::Ejection(1))
        .ok_or("the payload never left")?
        .sample;
    println!();
    println!(
        "The payload leaves at {:.2} s: the airframe's body goes from {:.3} kg to {:.3} kg.",
        parting.time_s, parting.mass_kg, airframe.mass_kg
    );
    Ok(())
}
