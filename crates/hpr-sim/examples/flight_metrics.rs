//! What a flight comes to: its peaks, its stability margins, the ejection delay that would fire at
//! apogee, and where it landed, from `hpr_sim::metrics`.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example flight_metrics -p hpr-sim
//! ```
//!
//! The documentation site's *Flight metrics* page (`docs/physics/metrics.md`) walks through it.
//! What it prints is kept next to it in `flight_metrics.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::metrics::{Peak, optimum_delays};
use hpr_sim::{
    CanopyType, Device, DeviceDrag, Environment, FlightMetrics, FlightSettings, Rail, Simulation,
    Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket and its weather, as on the Getting started page: Valetudo on a K400C, at a site
    // in New Mexico 1,400 m up, with 5 m/s of wind from the west.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let environment =
        Environment::standard(site)?.with_wind(ConstantWind::new(5.0, 270.0_f64.to_radians())?);

    // One parachute, fired 6 s after the motor burns out (a guess at the motor's delay), which
    // opens fully half a second after its charge.
    let simulation = Simulation::new(
        &rocket,
        "example",
        environment,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?
    .with_recovery(vec![
        Device::new(
            "main",
            DeviceDrag::canopy(CanopyType::FlatCircular, 1.5),
            Trigger::Burnout {
                motor: 0,
                delay_s: 6.0,
            },
        )
        .with_lag_s(0.5),
    ])?;

    // Fly it with a `FlightMetrics` watching, then sum the flight up.
    let mut metrics = FlightMetrics::new();
    let flight = simulation.run(&mut metrics)?;
    let summary = metrics.summary(&flight, simulation.environment())?;

    println!("Valetudo on a K400C, a 1.5 m parachute fired 6 s after burnout, open 0.5 s later");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    let launch_m = summary.launch_height_m.ok_or("no flight")?;
    println!(
        "Heights are the centre of mass's above the launch site; it starts {launch_m:.2} m up."
    );
    match summary.apogee {
        Some(apogee) => println!(
            "Apogee:              {:7.1} m above the site ({:.1} m of climb) at {:.2} s",
            apogee.height_above_ground_m,
            apogee.gain_m.ok_or("no launch height")?,
            apogee.time_s
        ),
        None => println!("Apogee:              none"),
    }
    let peak = |name: &str, peak: Option<Peak>, unit: &str, digits: usize| match peak {
        Some(p) => println!(
            "{name:<21}{:>7.digits$}{unit} at {:.2} s, {:.0} m up",
            p.value, p.time_s, p.height_above_ground_m
        ),
        None => println!("{name:<21}none"),
    };
    peak("Top speed:", summary.max_speed_m_s, " m/s", 1);
    peak("Top Mach number:", summary.max_mach, "", 3);
    peak("Max q:", summary.max_dynamic_pressure_pa, " Pa", 0);
    peak(
        "Boost acceleration:",
        summary.max_acceleration_m_s2,
        " m/s²",
        1,
    );
    peak(
        "Opening shock:",
        summary.max_descent_acceleration_m_s2,
        " m/s²",
        1,
    );
    println!();

    let exit = summary.rail_exit_stability.ok_or("no rail exit")?;
    let exit_speed = summary.rail_exit_speed_m_s.ok_or("no rail exit")?;
    println!(
        "Rail exit:            {:.1} m/s at {:.2} s, {:.1}° off the oncoming air",
        exit_speed.value,
        exit_speed.time_s,
        exit.flight_margin.angle_of_attack_rad.to_degrees()
    );
    let cal = |margin: Option<f64>| margin.map_or("none".to_owned(), |m| format!("{m:.2} cal"));
    println!(
        "At rail exit ({:.2} s): static margin {}, flight margin {} at Mach {:.3}",
        exit.time_s,
        cal(exit.static_margin.margin_cal),
        cal(exit.flight_margin.margin_cal),
        exit.flight_margin.mach
    );
    peak(
        "Least static margin:",
        summary.min_static_margin_cal,
        " cal",
        2,
    );
    peak(
        "Least flight margin:",
        summary.min_flight_margin_cal,
        " cal",
        2,
    );
    println!();

    // The delay that would fire the charge at apogee, from a flight with the charge held.
    for best in optimum_delays(&simulation)?.unwrap_or_default() {
        println!(
            "Optimum delay:        {:.1} s after burnout at {:.2} s (flown: 6.0 s), for an apogee \
             of {:.1} m at {:.2} s",
            best.delay_s, best.burnout_s, best.apogee_height_above_ground_m, best.apogee_s
        );
    }
    // Within 5 cm is none at all: the wind here is due east, and a digit's sign on a
    // millimetre would differ between platforms.
    let tidy = |x: f64| if x.abs() < 0.05 { 0.0 } else { x };
    match summary.landing {
        Some(landing) => println!(
            "Landing:              {:.6}° N, {:.6}° W: {:.1} m east and {:.1} m north of the \
             site, at {:.1} m/s, at {:.2} s",
            landing.latitude_deg,
            -landing.longitude_deg,
            tidy(landing.east_m),
            tidy(landing.north_m),
            landing.ground_hit_speed_m_s,
            landing.time_s
        ),
        None => println!("Landing:              none ({:?})", summary.termination),
    }
    Ok(())
}
