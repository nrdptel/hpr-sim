//! A rocket built from catalogue parts, LOC Precision's 2.56 in airframe from the parts catalogue
//! OpenRocket ships, then weighed part by part and flown.
//!
//! ```text
//! cargo run --example catalog_rocket -p hpr
//! ```
//!
//! The documentation site's page *The builder* (`docs/the-builder.md`) walks through it. What it
//! prints is kept next to it in `catalog_rocket.output.txt`, and CI checks that the two still
//! agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_io::orc::{self, PartKind};
use hpr::rocket::{Fins, Fitting, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinPlanform, Flight, Motor, Position, Rocket,
    Trigger,
};

/// The one part in the bundled catalogue that `maker` numbers `number`.
fn part(maker: &str, number: &str) -> Result<&'static orc::Part, Box<dyn Error>> {
    match orc::bundled().find(maker, number).as_slice() {
        [part] => Ok(part),
        found => Err(format!("{maker} {number}: {} parts, not one", found.len()).into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let loc = "LOC Precision";
    let nose = part(loc, "PNC-2.56")?;
    let tube = part(loc, "BT-2.56")?;
    let motor_tube = part(loc, "BT-1.52, MMT-1.52")?;
    let ring = part(loc, "CR-2.56-38mm")?;
    let chute = part(loc, "LP-36-2022")?;
    let PartKind::Parachute(canopy) = &chute.kind else {
        return Err("LP-36-2022 is a parachute".into());
    };

    let motor = Motor::from_catalog("H170")?.with_delay_s(10.0)?;
    // The rocket's diameter is the tube's; the nose states its own, the same.
    let PartKind::BodyTube(airframe) = &tube.kind else {
        return Err("BT-2.56 is a body tube".into());
    };
    let mut rocket = Rocket::new("LOC 2.56 in from the catalogue", airframe.outer_diameter_m)?;
    rocket
        // The polypropylene ogive and its shoulder, as the catalogue gives them.
        .add_nose(Nose::from_catalog(nose)?)?
        // The 30 in body tube, whole.
        .add_tube(Tube::from_catalog(tube)?)?
        // The 34 in, 38 mm motor tube, cut to 12 in, flush with the body tube's aft end.
        .add_motor_tube(MotorTube::from_catalog(motor_tube)?.with_length_m(0.3048))?
        // Two plywood centering rings on it: one at its aft end, one 11 in further forward.
        .add_fitting(Fitting::from_catalog(ring)?)?
        .add_fitting(Fitting::from_catalog(ring)?.at(Position::Bottom { aft_offset_m: -0.2794 }))?
        // The catalogue has no fins: three of 1/8 in birch plywood, made to measure.
        .add_fins(Fins::new(
            3,
            FinPlanform::Trapezoidal {
                root_chord_m: 0.2,
                tip_chord_m: 0.08,
                span_m: 0.11,
                sweep_m: 0.12,
            },
            0.003175,
            material("birch_plywood")?,
        ))?
        // The 36 in nylon parachute, packed 25 cm below the top of the tube. As a fitting it is
        // its weight; its drag is the recovery device below, the catalogue's diameter.
        .add_fitting(Fitting::from_catalog(chute)?.at(Position::Top { aft_offset_m: 0.25 }))?
        .set_motor(motor)?
        .add_parachute(Device::new(
            "LP-36-2022",
            DeviceDrag::canopy(CanopyType::FlatCircular, canopy.diameter_m),
            Trigger::MotorDelay { motor: 0 },
        ));

    println!(
        "{}, with motor {}",
        rocket.design().name,
        rocket.configuration_id().ok_or("no motor")?
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    // Each part as laid out: its name (the catalogue's maker and number) and its mass.
    println!("part                                     mass (g)");
    let layout = rocket.design().layout()?;
    let mut names = Vec::new();
    for stage in &rocket.design().stages {
        for component in &stage.components {
            names.push((&component.id, &component.name));
            names.extend(
                component
                    .children
                    .iter()
                    .map(|child| (&child.id, &child.name)),
            );
        }
    }
    for placed in &layout.components {
        // The part's name where it has one, its id where not (the fins).
        let name = names
            .iter()
            .find(|(id, name)| **id == placed.id && !name.is_empty())
            .map_or(placed.id.as_str(), |(_, name)| name.as_str());
        println!("{name:<40} {:>8.1}", 1000.0 * placed.own.mass_kg);
    }
    println!(
        "{:<40} {:>8.1}",
        "structure",
        1000.0 * layout.structure.mass_kg
    );
    let liftoff = rocket.mass_properties(0.0)?;
    let margin = rocket.margin(0.0, 0.3)?;
    println!();
    println!(
        "At liftoff: {:.3} kg, centre of gravity {:.3} m from the nose,",
        liftoff.mass_kg, -liftoff.cg_m.z
    );
    println!(
        "stability margin {:.2} calibres at Mach 0.3.",
        margin.margin_cal.ok_or("no margin")?
    );

    // Spaceport America, in calm air, from a 1.8 m vertical rail.
    let environment = Environment::new(32.99, -106.97, 1400.0)?;
    let flight = Flight::builder(&rocket, &environment, 1.8).fly()?;
    println!();
    println!(
        "Rail exit:  {:.1} m/s",
        flight.rail_exit_speed_m_s().ok_or("no rail exit")?
    );
    println!(
        "Apogee:     {:.1} m above the pad, at {:.2} s",
        flight.apogee_m().ok_or("no apogee")?,
        flight.apogee_time_s().ok_or("no apogee")?
    );
    println!(
        "Top speed:  {:.0} m/s (Mach {:.2})",
        flight.max_speed_m_s().ok_or("no top speed")?,
        flight.max_mach().ok_or("no top speed")?
    );
    let landing = flight.landing().ok_or("no landing")?;
    println!(
        "Landing:    at {:.1} m/s, at {:.1} s",
        landing.descent_rate_m_s, landing.time_s
    );
    Ok(())
}
