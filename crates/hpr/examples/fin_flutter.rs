//! Fin flutter by NACA TN 4197's criterion: a 54 mm rocket's fins in five materials with a cited
//! shear modulus. For each: the dynamic pressure at eq. 18's flutter speed, that speed at sea level
//! and 3 km above it, the ratio of that speed to the airspeed at the flight's peak dynamic pressure,
//! and Martin's own figure 3 reading at the launch site's pressure.
//!
//! It uses the workspace crates `hpr-sim`, `hpr-design`, `hpr-atmos` and `hpr-core`, and
//! `serde_json` to read the design; a program of your own outside this repository depends on those
//! five.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example fin_flutter -p hpr
//! ```
//!
//! The documentation site's fin-flutter page (`docs/physics/flutter.md`) quotes what it prints,
//! which is kept next to it in `fin_flutter.output.txt`; CI checks that the two still agree
//! (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::{Atmosphere, Ussa76};
use hpr_core::geodesy::Geodetic;
use hpr_design::{Component, FinSet, Part, Rocket, materials};
use hpr_sim::flutter::FIGURE_3_BAND;
use hpr_sim::{Environment, FlightMetrics, FlightSettings, FlutterPanel, Rail, Simulation};

/// The repository's synthetic 54 mm rocket: three 3.2 mm fins, 150 mm root, 60 mm tip, 70 mm span.
const DESIGN: &str = include_str!("../../../validation/designs/synthetic-54mm-three-fin.json");

/// The first fin set among `components` and their children.
fn fins(components: &[Component]) -> Option<&FinSet> {
    components
        .iter()
        .find_map(|component| match &component.part {
            Part::FinSet(set) => Some(set),
            _ => fins(&component.children),
        })
}

fn main() -> Result<(), Box<dyn Error>> {
    let rocket: Rocket = serde_json::from_str(DESIGN)?;
    let set = rocket
        .stages
        .iter()
        .find_map(|stage| fins(&stage.components))
        .ok_or("the design has no fin set")?;
    let panel = FlutterPanel::of_fins(set)?;
    println!(
        "Panel: aspect ratio {:.3}, taper ratio {:.3}, thickness ratio {:.4}",
        panel.aspect_ratio(),
        panel.taper_ratio(),
        panel.thickness_ratio()
    );

    // Its flight on an I175, for the peak dynamic pressure, from a site 200 m above the ellipsoid.
    let site_height_m = 200.0;
    let simulation = Simulation::new(
        &rocket,
        "i175",
        Environment::standard(Geodetic::from_degrees(35.0, -106.0, site_height_m)?)?,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?;
    let mut metrics = FlightMetrics::new();
    let result = simulation.run(&mut metrics)?;
    let summary = metrics.summary(&result, simulation.environment())?;
    let peak = summary
        .max_dynamic_pressure_pa
        .ok_or("the rocket never flew")?;
    println!(
        "Max q: {:.0} Pa at {:.2} s, {:.0} m above the pad",
        peak.value, peak.time_s, peak.height_above_ground_m
    );
    if let (Some(speed), Some(mach)) = (summary.max_speed_m_s, summary.max_mach) {
        println!(
            "Top speed: {:.0} m/s at {:.2} s; top Mach number: {:.2} at {:.2} s",
            speed.value, speed.time_s, mach.value, mach.time_s
        );
    }

    // The flutter speed at sea level and at 3 km in the standard atmosphere.
    let atmosphere = Ussa76::standard();
    let sea_level = atmosphere.air(0.0)?.air;
    let high = atmosphere.air(3000.0)?.air;
    // Martin's figure 3 reading at the launch site's pressure (taking its height as above sea level).
    let launch = atmosphere.air(site_height_m)?.air;
    println!(
        "Martin's band (figure 3): D/G_E from {} to {}",
        FIGURE_3_BAND[0], FIGURE_3_BAND[1]
    );
    println!();
    println!(
        "material            G (GPa)   q_f (kPa)   V_f 0 m (m/s)   V_f 3 km (m/s)   V_f/V at max q   D/G_E"
    );
    for id in [
        "aluminum_6061",
        "carbon_fiber",
        "birch_plywood",
        "basswood",
        "balsa",
    ] {
        let g = materials::shear_modulus(id)
            .ok_or("no shear modulus")?
            .shear_modulus_pa;
        let q_f = panel.flutter_dynamic_pressure_pa(g)?;
        let v_0 =
            panel.flutter_speed_m_s(g, sea_level.pressure_pa, sea_level.speed_of_sound_m_s)?;
        let v_3 = panel.flutter_speed_m_s(g, high.pressure_pa, high.speed_of_sound_m_s)?;
        let margin = panel.margin(g, &summary)?.ok_or("the rocket never flew")?;
        let ratio = panel.figure_3_ratio(g, launch.pressure_pa)?;
        println!(
            "{id:<18}{:>9.3}{:>12.1}{:>16.0}{:>17.0}{:>17.2}{:>8.3}",
            g / 1e9,
            q_f / 1e3,
            v_0,
            v_3,
            margin.speed_ratio,
            ratio
        );
    }
    Ok(())
}
