//! ERA5 weather: the atmosphere over a launch site at launch time, read from an ERA5
//! pressure-level file, and a flight through it beside the same flight in the standard atmosphere
//! with no wind.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example era5_weather -p hpr
//! ```
//!
//! The documentation site's *ERA5 weather files* page (`docs/format/era5.md`) walks through
//! it. What it prints is kept next to it in `era5_weather.output.txt`, and CI checks that
//! the two still agree (`cargo xtask examples --check`).
//!
//! The file is a small cut of the ERA5 file RocketPy ships for Bella Lui's flight (EPFL, 22
//! February 2020, Kaltbrunn, Switzerland). It contains modified Copernicus Climate Change Service
//! information 2020.

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::wind::WindInterpolation;
use hpr_atmos::{Atmosphere, ConstantWind, Ussa76};
use hpr_core::DVec3;
use hpr_core::earth::Earth;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_io::era5::{Era5Profile, Era5Request, UtcTime, direction_from_rad};
use hpr_io::netcdf::NetCdf;
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Read the file. A program would read it from disk; this one carries it inside.
    let bytes = include_bytes!("../../../validation/fixtures/weather/era5/bella-lui.nc");
    let file = NetCdf::parse(bytes)?;

    // 2. The atmosphere over the pad at 13:00 UTC, when Bella Lui flew.
    let (latitude_deg, longitude_deg, pad_msl_m) = (47.213476, 9.003336, 407.0);
    let request = Era5Request {
        latitude_deg,
        longitude_deg,
        time: UtcTime::from_civil(2020, 2, 22, 13, 0, 0.0)?,
    };
    let profile = Era5Profile::read(&file, request)?;

    println!("ERA5 over {latitude_deg}° N, {longitude_deg}° E at 2020-02-22 13:00 UTC");
    println!();
    println!("level (hPa)   height (m)   temperature (°C)   wind (m/s)   from (°)");
    for level in profile.levels.iter().filter(|l| l.height_msl_m < 2500.0) {
        let speed = level.wind_east_m_s.hypot(level.wind_north_m_s);
        let from = direction_from_rad(level.wind_east_m_s, level.wind_north_m_s).to_degrees();
        println!(
            "{:>11.0} {:>12.0} {:>18.1} {:>12.1} {:>10.0}",
            level.pressure_pa / 100.0,
            level.height_msl_m,
            level.temperature_k - 273.15,
            speed,
            from,
        );
    }

    // 3. The atmosphere as a flight uses it: hydrostatic between levels, and the wind's east and
    //    north parts interpolated in height, as RocketPy does.
    let sounding = profile.sounding(WindInterpolation::Components)?;
    let standard = Ussa76::standard();
    println!();
    println!("height above the pad (m)   pressure (hPa)   temperature (°C)   density (kg/m³)");
    for above_m in [0.0, 500.0] {
        for (name, air) in [
            ("ERA5", sounding.air(pad_msl_m + above_m)?.air),
            ("standard", standard.air(pad_msl_m + above_m)?.air),
        ] {
            println!(
                "{above_m:>6.0} {name:<17} {:>16.1} {:>18.1} {:>17.4}",
                air.pressure_pa / 100.0,
                air.temperature_k - 273.15,
                air.density_kg_m3,
            );
        }
    }

    // 4. Bella Lui (RocketPy's example, K-class motor) flown in both, without its parachute.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-bella-lui.json"
    ))?;
    // Heights above sea level are taken as heights above the ellipsoid (geoid undulation 0).
    let site = Geodetic::from_degrees(latitude_deg, longitude_deg, pad_msl_m)?;
    let wind = sounding.wind().ok_or("the profile has no wind")?.clone();
    let era5 = Environment::new(Earth::wgs84(site)?, sounding.clone(), wind);
    let calm = Environment::new(Earth::wgs84(site)?, standard, ConstantWind::calm());
    println!();
    println!("Bella Lui to apogee   apogee (m above the pad)   drift at apogee (m)");
    for (name, environment) in [("ERA5", era5), ("standard, calm", calm)] {
        let simulation = Simulation::new(
            &rocket,
            "example",
            environment,
            Rail::vertical(4.2),
            FlightSettings::default(),
        )?;
        let flight = simulation.run(&mut ())?;
        let apogee = flight
            .event(EventKind::Apogee)
            .ok_or("the flight has no apogee")?
            .sample;
        let drift = DVec3::new(apogee.cg_enu_m.x, apogee.cg_enu_m.y, 0.0).length();
        println!(
            "{name:<20} {:>26.1} {:>21.1}",
            apogee.height_above_ground_m, drift
        );
    }
    println!();
    println!("This design flies a stand-in motor; Bella Lui's real flight, on its own K828FJ, is");
    println!("compared on the accuracy page.");
    Ok(())
}
