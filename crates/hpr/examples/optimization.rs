//! Optimization: CMA-ES checked against four test functions whose minima are known, then the
//! nose ballast and body length that send a rocket to 3,048 m (10,000 ft), the winner checked by
//! flying it again.
//!
//! ```text
//! cargo run --example optimization -p hpr
//! ```
//!
//! The documentation site's page *Optimization* (`docs/optimization.md`) walks through it. What it
//! prints is kept next to it in `optimization.output.txt`, and CI checks that the two still agree
//! (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_analysis::optimize::Variable;
use hpr::hpr_analysis::optimize::benchmark::{ellipsoid, rosenbrock, rotated_ellipsoid, sphere};
use hpr::hpr_analysis::optimize::cmaes::Cmaes;
use hpr::hpr_sim::{Adaptive, FlightSettings, Method};
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

/// The apogee to hit, m above the pad: 10,000 ft.
const TARGET_M: f64 = 3048.0;

/// How close counts as a hit, m.
const HIT_M: f64 = 0.1;

fn main() -> Result<(), Box<dyn Error>> {
    let seed = 2026;
    test_functions(seed)?;
    println!();
    hit_the_target(seed)
}

/// A test function of ten variables.
type TestFunction = fn(&[f64]) -> f64;

/// CMA-ES on the four test functions, ten variables each, to within 10⁻¹⁰ of their minima.
fn test_functions(seed: u64) -> Result<(), Box<dyn Error>> {
    println!("CMA-ES on four test functions, 10 variables, run to f ≤ 1e-10 (seed {seed})");
    println!("function            start   evaluations   best value   distance to the minimum");
    let cases: [(&str, TestFunction, f64, f64); 4] = [
        ("sphere", sphere, 1.0, 0.0),
        ("ellipsoid", ellipsoid, 1.0, 0.0),
        ("rotated ellipsoid", rotated_ellipsoid, 1.0, 0.0),
        ("Rosenbrock", rosenbrock, 0.0, 1.0),
    ];
    for (name, f, start, minimum) in cases {
        let variables = (0..10)
            .map(|i| Variable::new(format!("x{i}"), start, 0.5))
            .collect::<Result<Vec<_>, _>>()?;
        let optimum = Cmaes::new(variables)?
            .with_target(1e-10)?
            .with_max_evaluations(100_000)?
            .minimize(seed, f)?;
        let distance = optimum
            .point
            .iter()
            .map(|x| (x - minimum).powi(2))
            .sum::<f64>()
            .sqrt();
        println!(
            "{name:<18} {start:>6.1} {:>13} {:>12.1e} {:>25.1e}",
            optimum.evaluations, optimum.value, distance
        );
    }
    Ok(())
}

/// A 66 mm rocket on a J760 with `ballast_kg` in its nose and a body tube `body_m` long.
fn rocket(ballast_kg: f64, body_m: f64) -> Result<Rocket, Box<dyn Error>> {
    let fiberglass = || material("fiberglass_filament_wound");
    let mut rocket = Rocket::new("66 mm on a J760", 0.066)?;
    rocket
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.33,
                0.0015,
                fiberglass()?,
            )
            .with_capped_shoulder(0.07, 0.0015),
        )?
        .add_tube(Tube::new(body_m, 0.0015, fiberglass()?))?
        .add_motor_tube(MotorTube::new(0.42, 0.054, 0.001, fiberglass()?).with_overhang_m(0.005))?
        .add_fins(
            Fins::new(
                3,
                FinPlanform::Trapezoidal {
                    root_chord_m: 0.14,
                    tip_chord_m: 0.05,
                    span_m: 0.06,
                    sweep_m: 0.08,
                },
                0.003175,
                material("fiberglass_g10")?,
            )
            .with_cross_section(FinCrossSection::Rounded),
        )?
        .add_mass(
            Mass::new(0.4, Position::Top { aft_offset_m: 0.1 })
                .packed(0.2, 0.06)
                .named("avionics and recovery"),
        )?
        .add_mass(
            Mass::new(ballast_kg, Position::Top { aft_offset_m: 0.0 })
                .packed(0.03, 0.06)
                .named("nose ballast"),
        )?
        .set_motor(Motor::from_catalog("J760")?)?
        .add_parachute(Device::new(
            "parachute",
            DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
            Trigger::Apogee,
        ));
    Ok(rocket)
}

/// The rocket's flight from a 3 m rail, 85° above the horizon, into 5 m/s of wind from the west.
fn fly(ballast_kg: f64, body_m: f64, settings: FlightSettings) -> Result<Flight, Box<dyn Error>> {
    let rocket = rocket(ballast_kg, body_m)?;
    let environment = Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(5.0, 270.0)?;
    Ok(Flight::builder(&rocket, &environment, 3.0)
        .inclination_deg(85.0)
        .heading_deg(270.0)
        .settings(settings)
        .fly()?)
}

/// Finds the ballast and body length that hit [`TARGET_M`], then flies the winner again.
fn hit_the_target(seed: u64) -> Result<(), Box<dyn Error>> {
    let variables = vec![
        Variable::new("nose ballast (kg)", 0.3, 0.15)?.within(0.0, 1.0)?,
        Variable::new("body tube (m)", 1.0, 0.1)?.within(0.6, 1.4)?,
    ];
    // Minimize the squared miss; a hit within HIT_M stops the run.
    let optimizer = Cmaes::new(variables)?
        .with_target(HIT_M * HIT_M)?
        .with_max_evaluations(1_000)?;
    let start = fly(0.3, 1.0, FlightSettings::default())?
        .apogee_m()
        .ok_or("no apogee")?;
    let optimum = optimizer.minimize(seed, |x| {
        match fly(x[0], x[1], FlightSettings::default()) {
            Ok(flight) => flight
                .apogee_m()
                .map_or(f64::INFINITY, |apogee| (apogee - TARGET_M).powi(2)),
            // A design the simulator refuses ranks below every flight.
            Err(_) => f64::INFINITY,
        }
    })?;
    let [ballast_kg, body_m] = optimum.point[..] else {
        return Err("two variables expected".into());
    };
    println!("Hit 3,048 m with a 66 mm rocket on a J760 by its nose ballast and body length");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!("Start: 0.300 kg of ballast, a 1.000 m body: apogee {start:.1} m");
    println!(
        "Found: {ballast_kg:.3} kg of ballast, a {body_m:.3} m body, in {} flights ({} generations, stopped: {:?})",
        optimum.evaluations, optimum.generations, optimum.stop
    );

    // Fly the winner again, from a fresh build: the same apogee to the bit.
    let again = fly(ballast_kg, body_m, FlightSettings::default())?
        .apogee_m()
        .ok_or("no apogee")?;
    // And with the integrator's tolerances 100 times tighter: the hit is the flight's, not the
    // integrator's.
    let tight = FlightSettings {
        method: Method::DormandPrince54(Adaptive {
            relative_tolerance: 1e-10,
            absolute_tolerance: 1e-10,
            ..Adaptive::default()
        }),
        ..FlightSettings::default()
    };
    let tighter = fly(ballast_kg, body_m, tight)?
        .apogee_m()
        .ok_or("no apogee")?;
    println!(
        "Flown again: apogee {again:.3} m (miss {:+.3} m)",
        again - TARGET_M
    );
    println!(
        "Flown again, tolerances 100 times tighter: apogee {tighter:.3} m (miss {:+.3} m)",
        tighter - TARGET_M
    );
    if (again - TARGET_M).powi(2) != optimum.value {
        return Err("the winner flown again doesn't give the optimizer's apogee".into());
    }
    if (again - TARGET_M).abs() > HIT_M || (tighter - TARGET_M).abs() > HIT_M {
        return Err("the winner flown again misses the target".into());
    }
    Ok(())
}
