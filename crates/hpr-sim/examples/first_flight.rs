//! A first flight: a rocket with a drogue and a main parachute, flown from the launch rail to the
//! ground, with a summary of what happened.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example first_flight -p hpr-sim
//! ```
//!
//! The documentation site's *Getting started* page (`docs/getting-started.md`) walks through it.
//! What it prints is kept next to it in `first_flight.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::ConstantWind;
use hpr_core::DVec3;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::{
    CanopyType, Device, DeviceDrag, Environment, EventKind, FlightSettings, FlightStep, Observer,
    Rail, SimError, Simulation, Termination, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket: Valetudo, which Projeto Jupiter (University of São Paulo) flew in 2019 and
    // RocketPy uses as an example. The design file describes its parts, materials and motor; the
    // motor keeps the example's size and mass, with a bundled K400C thrust curve.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;

    // Where and in what weather: a site in New Mexico 1,400 m up, the 1976 US Standard
    // Atmosphere, and a steady wind from the west (270°) at every height.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let wind_speed_m_s = 5.0;
    let wind_from_deg = 270.0_f64;
    let environment = Environment::standard(site)?.with_wind(ConstantWind::new(
        wind_speed_m_s,
        wind_from_deg.to_radians(),
    )?);

    // A vertical launch rail.
    let rail_length_m = 3.0;
    let rail = Rail::vertical(rail_length_m);

    // The flight: the design's configuration "example" (its K400C), the default settings, and
    // dual deployment. The drogue opens half a second after apogee; the main opens a second after
    // the rocket falls past 150 m above the ground, and the drogue stays attached.
    let simulation = Simulation::new(
        &rocket,
        "example",
        environment,
        rail,
        FlightSettings::default(),
    )?
    .with_recovery(vec![
        Device::new(
            "drogue",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.6),
            Trigger::Apogee,
        )
        .with_lag_s(0.5),
        Device::new(
            "main",
            DeviceDrag::canopy(CanopyType::FlatCircular, 2.4),
            Trigger::Altitude {
                height_above_ground_m: 150.0,
            },
        )
        .with_lag_s(1.0),
    ])?;

    // Fly it. `TopSpeed` (below) watches every step of the flight for its fastest moment.
    let mut top = TopSpeed::default();
    let flight = simulation.run(&mut top)?;
    if flight.termination != Termination::GroundHit {
        return Err(format!("the flight ended with {:?}", flight.termination).into());
    }

    let liftoff = flight
        .event(EventKind::Liftoff)
        .ok_or("the rocket never lifted off")?
        .sample;
    println!(
        "Valetudo on a K400C: {:.2} kg at liftoff, a {rail_length_m} m rail, {wind_speed_m_s} m/s \
         of wind from {wind_from_deg}°",
        liftoff.mass_kg
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("event                time (s)   CG height (m)   speed (m/s)");
    for event in &flight.events {
        let name = match event.kind {
            EventKind::Liftoff => "liftoff".to_owned(),
            EventKind::RailExit => "rail exit".to_owned(),
            EventKind::Burnout => "burnout".to_owned(),
            EventKind::Apogee => "apogee".to_owned(),
            EventKind::Trigger(i) => format!("{} charge fires", simulation.recovery()[i].name),
            EventKind::Deployment(i) => format!("{} opens", simulation.recovery()[i].name),
            EventKind::GroundHit => "landing".to_owned(),
            other => format!("{other:?}"),
        };
        let sample = &event.sample;
        // Heights are the centre of gravity's above the pad. The flight ends where it reaches the
        // ground, found to within a micrometre just below it: print that as 0 (`+ 0.0` turns a
        // -0 into 0).
        println!(
            "{name:<20} {:>8.2} {:>15.1} {:>13.1}",
            sample.time_s,
            sample.height_above_ground_m.max(0.0) + 0.0,
            sample.cg_velocity_enu_m_s.length(),
        );
    }

    let apogee = flight
        .event(EventKind::Apogee)
        .ok_or("the flight has no apogee")?
        .sample;
    let rail_exit = flight
        .event(EventKind::RailExit)
        .ok_or("the flight never left the rail")?
        .sample;
    let landing = flight.final_sample;
    let (apogee_distance_m, apogee_bearing_deg) = from_pad(apogee.cg_enu_m);
    let (landing_distance_m, landing_bearing_deg) = from_pad(landing.cg_enu_m);

    println!();
    println!(
        "Apogee:     {:.1} m ({:.0} ft) above the pad, {apogee_distance_m:.1} m from it at a \
         bearing of {apogee_bearing_deg:.0}°, at {:.2} s",
        apogee.height_above_ground_m,
        apogee.height_above_ground_m / METRES_PER_FOOT,
        apogee.time_s,
    );
    println!(
        "Top speed:  {:.1} m/s (Mach {:.2}), at {:.1} s",
        top.speed_m_s, top.mach, top.time_s,
    );
    println!(
        "Rail exit:  {:.1} m/s",
        rail_exit.cg_velocity_enu_m_s.length()
    );
    println!(
        "Landing:    {landing_distance_m:.1} m from the pad at a bearing of \
         {landing_bearing_deg:.0}°, falling at {:.1} m/s, at {:.2} s",
        -landing.vertical_speed_m_s, landing.time_s,
    );
    Ok(())
}

/// The international foot.
const METRES_PER_FOOT: f64 = 0.3048;

/// Where `position` is from the pad: its distance over the ground, m, and its bearing, clockwise
/// from north, degrees. The launch frame's axes point east, north and up from the pad
/// (`docs/physics/frames.md`).
fn from_pad(position: DVec3) -> (f64, f64) {
    let (east_m, north_m) = (position.x, position.y);
    let bearing_deg = east_m.atan2(north_m).to_degrees().rem_euclid(360.0);
    (east_m.hypot(north_m), bearing_deg)
}

/// The fastest moment of a flight, relative to the ground.
///
/// An [`Observer`] sees every accepted step of the integration as the flight runs. This one looks
/// at the end of each step. Looking at 200 points inside every step instead raises the top speed
/// by 0.005 m/s, and moves its time by 0.013 s, since the speed is flat near its peak. So the
/// speed can read low by one in its last printed digit, and the time is printed to 0.1 s.
#[derive(Debug, Default)]
struct TopSpeed {
    speed_m_s: f64,
    mach: f64,
    time_s: f64,
}

impl Observer for TopSpeed {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let sample = step.sample(step.end_s())?;
        let speed_m_s = sample.cg_velocity_enu_m_s.length();
        if speed_m_s > self.speed_m_s {
            *self = Self {
                speed_m_s,
                mach: sample.mach,
                time_s: sample.time_s,
            };
        }
        Ok(())
    }
}
