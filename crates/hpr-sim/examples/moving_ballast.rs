//! Ballast that slides aft during the coast: 200 g moved 0.3 m toward the tail over one second,
//! starting 5 s after launch. The rocket's centre of mass follows it aft, so its stability margin
//! falls. The program prints the mass properties and the static margin before, during and after
//! the move beside the hand calculation, then compares the apogee with the ballast held still.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example moving_ballast -p hpr-sim
//! ```
//!
//! The documentation site's page on moving mass (`docs/physics/moving-mass.md`) walks through
//! it. What it prints is kept next to it in `moving_ballast.output.txt`, and CI checks that the
//! two still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_core::geodesy::Geodetic;
use hpr_design::{Part, Position, Rocket};
use hpr_sim::{
    Environment, EventKind, FlightMetrics, FlightSettings, MassShift, Rail, Simulation, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket: the synthetic 54 mm single-stage design on an I175, which burns for 2.5 s.
    let mut rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/synthetic-54mm-three-fin.json"
    ))?;
    // 200 g of ballast in the airframe, a cylinder 50 mm long and 15 mm in radius on the axis,
    // 0.1 m aft of the airframe's forward end. It is an ordinary mass component, made here from
    // the design's altimeter.
    let airframe = rocket.stages[0]
        .components
        .iter_mut()
        .find(|component| component.id == "sustainer-airframe")
        .ok_or("no airframe")?;
    let mut ballast = airframe
        .children
        .iter()
        .find(|child| child.id == "altimeter")
        .cloned()
        .ok_or("no altimeter")?;
    ballast.id = "ballast".to_owned();
    if let Part::MassComponent(mass) = &mut ballast.part {
        mass.mass_kg = 0.2;
        mass.packing.length_m = 0.05;
        mass.packing.radius_m = 0.015;
    }
    ballast.position = Some(Position::Top { aft_offset_m: 0.1 });
    airframe.children.push(ballast);

    // Calm air over a site in New Mexico 1,400 m up, and a 3 m vertical rail.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let still = Simulation::new(
        &rocket,
        "i175",
        Environment::standard(site)?,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?;

    // The move: 0.3 m aft over 1 s, starting 5 s after launch.
    let (travel_m, start_s, duration_s) = (0.3, 5.0, 1.0);
    let moving = Simulation::new(
        &rocket,
        "i175",
        Environment::standard(site)?,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?
    .with_shifts(vec![MassShift::new(
        Trigger::Time { time_s: start_s },
        "ballast",
        travel_m,
        duration_s,
    )])?;
    let mut metrics = FlightMetrics::new();
    let flight = moving.run(&mut metrics)?;

    println!("A 54 mm rocket on an I175 whose 200 g of ballast slides 0.3 m aft from 5 s to 6 s");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();

    // The mass properties the flight had, with the centre of mass as a station aft of the nose
    // tip, and the moment of inertia about a transverse axis through it.
    println!("time (s)   mass (kg)   centre of mass (m aft of tip)   transverse inertia (kg m²)");
    for t_s in [4.0, 5.25, 5.5, 5.75, 7.0] {
        let mass = moving.mass_properties(&flight, t_s);
        println!(
            "{t_s:>8.2}   {:>9.4}   {:>29.4}   {:>26.5}",
            mass.mass_kg, -mass.cg_m.z, mass.inertia_kg_m2.x_axis.x
        );
    }

    // By hand: the ballast, m, moving Δ aft in a rocket of mass M moves the centre mΔ/M aft, and
    // the static margin by −mΔ/(M d) calibres.
    let before = moving.mass_properties(&flight, 4.0);
    let after = moving.mass_properties(&flight, 7.0);
    let margin_at = |t_s: f64| {
        metrics
            .stability()
            .iter()
            .rfind(|sample| sample.time_s <= t_s)
            .and_then(|sample| {
                sample
                    .static_margin
                    .margin_cal
                    .map(|margin| (margin, sample.reference_diameter_m))
            })
    };
    let (margin_before, d_m) = margin_at(4.9).ok_or("no margin before the move")?;
    let (margin_after, _) = margin_at(f64::INFINITY).ok_or("no margin after the move")?;
    let hand_travel_m = 0.2 * travel_m / before.mass_kg;
    println!();
    println!(
        "centre of mass moves   {:.4} m aft   by hand mΔ/M = 0.2 × 0.3 / {:.4} = {hand_travel_m:.4} m",
        before.cg_m.z - after.cg_m.z,
        before.mass_kg
    );
    println!(
        "static margin          {margin_before:.3} cal before, {margin_after:.3} after, a change of {:.3}",
        margin_after - margin_before
    );
    println!(
        "                       by hand −mΔ/(M d) = −{hand_travel_m:.4} / {d_m:.4} = {:.3} cal",
        -hand_travel_m / d_m
    );

    // The apogee, with the ballast moving and held still.
    let still_flight = still.run(&mut ())?;
    let apogee = |result: &hpr_sim::FlightResult| {
        result
            .event(EventKind::Apogee)
            .map(|event| event.sample.height_above_ground_m)
    };
    println!();
    println!(
        "apogee                 {:.1} m with the ballast moving, {:.1} m held still",
        apogee(&flight).ok_or("no apogee")?,
        apogee(&still_flight).ok_or("no apogee")?
    );
    Ok(())
}
