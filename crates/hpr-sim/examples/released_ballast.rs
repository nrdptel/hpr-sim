//! Ballast dropped during the coast: 200 g let go 5 s after launch, tumbling down on its own while
//! the rocket coasts on to its apogee and comes down under a drogue. The program prints the
//! rocket's mass properties and static margin before and after the release beside the hand
//! calculation, how the momentum divides, the apogee against a flight that keeps the ballast, and
//! where each lands.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example released_ballast -p hpr-sim
//! ```
//!
//! The documentation site's page on released mass (`docs/physics/released-mass.md`) walks through
//! it. What it prints is kept next to it in `released_ballast.output.txt`, and CI checks that the
//! two still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_core::geodesy::Geodetic;
use hpr_design::{Part, Position, Rocket};
use hpr_sim::{
    Device, DeviceDrag, Environment, EventKind, FlightMetrics, FlightResult, FlightSettings,
    MassRelease, Rail, Simulation, Trigger,
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

    // Calm air over a site in New Mexico 1,400 m up, a 3 m vertical rail, and a drogue with a
    // drag area of 0.3 m² that opens at apogee.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let simulation = |releases: Vec<MassRelease>| -> Result<Simulation, Box<dyn Error>> {
        Ok(Simulation::new(
            &rocket,
            "i175",
            Environment::standard(site)?,
            Rail::vertical(3.0),
            FlightSettings::default(),
        )?
        .with_recovery(vec![Device::new(
            "drogue",
            DeviceDrag::DragArea { cd_s_m2: 0.3 },
            Trigger::Apogee,
        )])?
        .with_releases(releases)?)
    };
    let kept = simulation(Vec::new())?.run(&mut ())?;
    // The ballast's drag area tumbling: the tumble model's body term, 0.56 times its side
    // profile, 50 mm long by 30 mm across (the Recovery page's Tumble section).
    let tumbling_m2 = 0.56 * 0.05 * 0.03;
    let sim = simulation(vec![MassRelease::new(
        Trigger::Time { time_s: 5.0 },
        "ballast",
        tumbling_m2,
    )])?;
    let mut metrics = FlightMetrics::new();
    let dropped = sim.run(&mut metrics)?;

    println!("A 54 mm rocket on an I175 that drops its 200 g of ballast 5 s after launch");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();

    // The mass properties the flight had, with the centre of mass as a station aft of the nose
    // tip, and the moment of inertia about a transverse axis through it.
    println!("time (s)   mass (kg)   centre of mass (m aft of tip)   transverse inertia (kg m²)");
    for t_s in [4.0, 6.0] {
        let mass = sim.mass_properties(&dropped, t_s);
        println!(
            "{t_s:>8.2}   {:>9.4}   {:>29.4}   {:>26.5}",
            mass.mass_kg, -mass.cg_m.z, mass.inertia_kg_m2.x_axis.x
        );
    }
    // By hand: the ballast, m at station c, leaving a rocket of mass M with its centre at station
    // cg leaves the rest's centre at (M cg − m c)/(M − m).
    let before = sim.mass_properties(&dropped, 4.0);
    let after = sim.mass_properties(&dropped, 6.0);
    let (_, placed) = sim.assembly().layout.find("ballast").ok_or("no ballast")?;
    let (big_m, cg, c) = (before.mass_kg, -before.cg_m.z, -placed.with_children.cg_m.z);
    println!(
        "by hand                {:.4}   (M cg − m c)/(M − m) = ({big_m:.4} × {cg:.4} − 0.2 × {c:.4}) / {:.4} = {:.4}",
        big_m - 0.2,
        big_m - 0.2,
        (big_m * cg - 0.2 * c) / (big_m - 0.2)
    );

    // The static margin just before and after: the centre of mass moving aft by Δ takes Δ/d
    // calibres off it, with d the body's diameter.
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
    let (margin_before, d_m) = margin_at(4.99).ok_or("no margin before the release")?;
    let (margin_after, _) = margin_at(5.5).ok_or("no margin after the release")?;
    let aft_m = before.cg_m.z - after.cg_m.z;
    println!();
    println!(
        "static margin          {margin_before:.3} cal before, {margin_after:.3} after, a change of {:.3}",
        margin_after - margin_before
    );
    println!(
        "                       by hand −Δ/d = −{aft_m:.5} / {d_m:.5} = {:.3} cal",
        -aft_m / d_m
    );

    // How the momentum divides as the ballast leaves: every point of the airframe moves at
    // v_O + ω × r, the rest's centre and the ballast's with it.
    let event = dropped
        .event(EventKind::MassRelease(0))
        .ok_or("no release")?
        .sample;
    let part = &dropped.released[0];
    let state = event.state;
    let rest_velocity = state.velocity_enu_m_s
        + state
            .unit_attitude()
            .mul_vec3(state.body_rate_rad_s.cross(after.cg_m));
    println!();
    println!(
        "momentum at 5 s        {:.4} kg m/s up: the rest carries {:.4}, the ballast {:.4}",
        event.cg_velocity_enu_m_s.z * big_m,
        rest_velocity.z * after.mass_kg,
        part.start_sample.cg_velocity_enu_m_s.z * part.start_sample.mass_kg
    );

    // The apogee, with the ballast dropped and kept.
    let apogee = |result: &FlightResult| {
        result
            .event(EventKind::Apogee)
            .map(|event| event.sample.height_above_ground_m)
    };
    println!();
    println!(
        "apogee                 {:.1} m with the ballast dropped, {:.1} m kept",
        apogee(&dropped).ok_or("no apogee")?,
        apogee(&kept).ok_or("no apogee")?
    );
    let ballast_apogee = part
        .events
        .iter()
        .find(|event| event.kind == EventKind::Apogee)
        .ok_or("the ballast left climbing, so it has an apogee")?;
    println!(
        "the ballast's apogee   {:.1} m at {:.2} s",
        ballast_apogee.sample.height_above_ground_m, ballast_apogee.sample.time_s
    );

    // Where each lands, and how fast.
    let landing = |time_s: f64, speed_m_s: f64, what: &str| {
        println!("{what:<22} lands at {time_s:.1} s, at {speed_m_s:.2} m/s");
    };
    println!();
    landing(
        dropped.final_sample.time_s,
        dropped.final_sample.cg_velocity_enu_m_s.length(),
        "the rocket",
    );
    landing(
        part.final_sample.time_s,
        part.final_sample.cg_velocity_enu_m_s.length(),
        "the ballast",
    );
    landing(
        kept.final_sample.time_s,
        kept.final_sample.cg_velocity_enu_m_s.length(),
        "the rocket, kept",
    );
    Ok(())
}
