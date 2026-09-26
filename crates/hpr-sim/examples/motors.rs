//! Motors: the thrust curves that come with hpr-sim, and a motor read from a RASP `.eng` file
//! like the ones ThrustCurve.org serves, put in a rocket's motor mount and flown.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example motors -p hpr-sim
//! ```
//!
//! The documentation site's *Solid motors* page (`docs/physics/motor.md`, "Using a motor") walks
//! through it. What it prints is kept next to it in `motors.output.txt`, and CI checks that the
//! two still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_core::geodesy::Geodetic;
use hpr_design::{Configuration, Ignition, MountedMotor, Rocket};
use hpr_motor::catalog::MotorType;
use hpr_motor::{Catalog, ImpulseClass, SolidMotor, eng};
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation, Termination};

fn main() -> Result<(), Box<dyn Error>> {
    // 1. The motors that come built in. Size and loaded mass are ThrustCurve.org's catalog values;
    //    impulse, class, average thrust and burn time are worked out here from each motor's curve.
    let catalog = Catalog::bundled()?;
    println!("{} motors come built in:", catalog.motors.len());
    println!();
    let heading = [
        [
            "", "", "", "", "dia", "length", "loaded", "total", "average", "burn",
        ],
        [
            "designation",
            "maker",
            "type",
            "class",
            "mm",
            "mm",
            "mass g",
            "impulse N·s",
            "thrust N",
            "time s",
        ],
    ];
    for [a, b, c, d, e, f, g, h, i, j] in heading {
        println!("{a:<12} {b:<8}  {c:<10}  {d:<5} {e:>5} {f:>7} {g:>9} {h:>11} {i:>9} {j:>7}");
    }
    for entry in &catalog.motors {
        let motor = entry.bundled_motor()?;
        let curve = motor.curve();
        let kind = match entry.motor_type {
            MotorType::SingleUse => "single-use",
            MotorType::Reload => "reload",
            _ => "other",
        };
        let class = ImpulseClass::from_total_impulse(curve.total_impulse_ns())?;
        // An entry the catalog gives no loaded mass for would show a dash; every bundled one has
        // a loaded mass.
        let loaded_g = entry
            .total_mass_g
            .map_or_else(|| "-".to_owned(), |mass_g| format!("{mass_g:.1}"));
        println!(
            "{:<12} {:<8}  {kind:<10}  {:<5} {:>5} {:>7} {loaded_g:>9} {:>11.1} {:>9.1} {:>7.2}",
            entry.designation,
            entry.manufacturer_abbrev,
            class.label(),
            entry.diameter_mm,
            entry.length_mm,
            curve.total_impulse_ns(),
            curve.average_thrust_n(),
            curve.burn_time_s(),
        );
    }

    // 2. A motor from a file. This is one of the bundled files, built into the program so that it
    //    runs anywhere. For a file you downloaded, read its text instead with
    //    `let text = std::fs::read_to_string("path/to/your-motor.eng")?;`
    let text = include_str!("../../hpr-motor/data/thrustcurve/curves/5f4294d20002e90000000863.eng");
    let parsed = eng::parse(text)?;
    for warning in &parsed.warnings {
        println!("warning, line {}: {}", warning.line, warning.message);
    }
    let [entry] = &parsed.value.entries[..] else {
        return Err("expected one motor in the file".into());
    };
    // The file gives the size in millimetres and the masses in kilograms; the motor takes metres
    // and kilograms. Only the size needs converting, and the motor can't tell if it isn't: its
    // units check looks at the impulse and the propellant mass, not the size.
    let diameter_m = entry.diameter_mm * 1e-3;
    let length_m = entry.length_mm * 1e-3;
    let motor = SolidMotor::from_envelope(
        entry.thrust_curve()?,
        diameter_m,
        length_m,
        entry.propellant_mass_kg,
        entry.total_mass_kg,
    )?;
    let curve = motor.curve();
    println!();
    println!(
        "From the .eng file: {} by {}, {} mm by {} mm, {:.0} g loaded, {:.0} g of propellant",
        entry.name,
        entry.manufacturer,
        entry.diameter_mm,
        entry.length_mm,
        entry.total_mass_kg * 1e3,
        entry.propellant_mass_kg * 1e3,
    );
    println!(
        "  {:.1} N·s (class {}), {:.1} N average over {:.2} s, {:.1} N peak",
        curve.total_impulse_ns(),
        ImpulseClass::from_total_impulse(curve.total_impulse_ns())?,
        curve.average_thrust_n(),
        curve.burn_time_s(),
        curve.peak_thrust_n(),
    );
    println!(
        "  effective exhaust velocity {:.0} m/s",
        motor.exhaust_velocity_m_s()
    );

    // 3. Into a rocket: a new configuration that puts the motor in the design's motor mount. The
    //    rocket is Valetudo, the rocket of `first_flight.rs`. Its mount is 43 mm inside, so this
    //    38 mm motor fits: the design checks compare the case diameter given here with the bore.
    let mut rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;
    rocket.configurations.push(Configuration {
        id: "from-file".to_owned(),
        name: format!("{} from its .eng file", entry.name),
        motors: vec![MountedMotor {
            mount: "motor-mount".to_owned(),
            designation: entry.name.clone(),
            diameter_m,
            length_m,
            motor,
            // No ejection delay is chosen: this flight carries no parachutes.
            delay: None,
            ignition: Ignition::Launch,
            failed_tubes: Vec::new(),
        }],
    });

    // Fly it, straight up from a 3 m rail in calm air, with no parachutes.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let simulation = Simulation::new(
        &rocket,
        "from-file",
        Environment::standard(site)?,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?;
    let flight = simulation.run(&mut ())?;
    if flight.termination != Termination::GroundHit {
        return Err(format!("the flight ended with {:?}", flight.termination).into());
    }
    let sample = |kind| {
        flight
            .event(kind)
            .map(|event| event.sample)
            .ok_or(format!("the flight has no {kind:?}"))
    };
    let (liftoff, rail_exit, apogee) = (
        sample(EventKind::Liftoff)?,
        sample(EventKind::RailExit)?,
        sample(EventKind::Apogee)?,
    );
    println!();
    println!(
        "Valetudo on the {}: {:.2} kg at liftoff",
        entry.name, liftoff.mass_kg
    );
    println!(
        "  leaves the 3 m rail at {:.1} m/s",
        rail_exit.cg_velocity_enu_m_s.length()
    );
    println!(
        "  apogee {:.1} m above the pad, at {:.2} s",
        apogee.height_above_ground_m, apogee.time_s,
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    Ok(())
}
