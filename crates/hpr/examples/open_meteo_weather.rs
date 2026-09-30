//! Open-Meteo weather: the forecast over a launch site at launch time, on pressure levels, and a
//! flight through it beside the same flight in the standard atmosphere with no wind.
//!
//! Run it from anywhere in the repository (it needs the `net` feature):
//!
//! ```text
//! cargo run --example open_meteo_weather -p hpr --features net
//! ```
//!
//! The documentation site's *Launch-day weather* page (`docs/weather.md`) walks through it. What
//! it prints is kept next to it in `open_meteo_weather.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).
//!
//! It never uses the network: a stand-in transport answers with a response recorded from
//! Open-Meteo's historical-forecast API for Spaceport America on 21 June 2025. Weather data by
//! Open-Meteo.com (CC BY 4.0).

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the example keeps its cache in a temporary folder"
)]
#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::wind::WindInterpolation;
use hpr_atmos::{Atmosphere, ConstantWind, Ussa76};
use hpr_core::earth::Earth;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_net::open_meteo::{self, DropReason, OpenMeteoApi, OpenMeteoRequest};
use hpr_net::{Cache, Client, Mode, Transport};
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};

/// Answers every URL with the recorded response. A real program uses `hpr_net::Http::new()`
/// here, which fetches from Open-Meteo.
struct Recorded;

impl Transport for Recorded {
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        let body = include_bytes!("../../hpr-net/tests/fixtures/replay/open-meteo-historical.json");
        Ok(body.to_vec())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. A client: the transport, a cache folder, and online mode. A real program would keep its
    //    cache in `Cache::platform_dir()`, so the weather is there offline next time.
    let folder = std::env::temp_dir().join(format!("hpr-open-meteo-{}", std::process::id()));
    let client = Client::new(Recorded, Cache::new(&folder), Mode::Online);

    // 2. Ask for 2025-06-21 15:30 UTC (9:30 local) over the launch area. The answer holds 15:00
    //    and 16:00; the profile is read halfway between.
    let (latitude_deg, longitude_deg) = (32.99, -106.97);
    let launch_s = 1_750_519_800;
    let request = OpenMeteoRequest::new(
        latitude_deg,
        longitude_deg,
        launch_s,
        OpenMeteoApi::HistoricalForecast,
    );
    let now_s = 1_790_000_000;
    let (profile, fetched) = open_meteo::fetch(&client, &request, now_s)?;
    std::fs::remove_dir_all(&folder).ok();

    println!(
        "Open-Meteo over {latitude_deg}° N, {}° W at 2025-06-21 15:30 UTC",
        -longitude_deg
    );
    println!("{}", fetched.attribution);
    // `Fetched`: from the transport, not the cache. A second call within the hour says `Cached`.
    println!("freshness: {:?}", fetched.freshness);
    println!();
    let s = &profile.surface;
    println!("level (hPa)   height (m)   temperature (°C)   humidity (%)   wind (m/s)   from (°)");
    println!(
        "{:>11.1} {:>12.0} {:>18.1} {:>14.0} {:>12.1} {:>10.0}   the ground",
        s.pressure_pa / 100.0,
        s.height_msl_m,
        s.temperature_k - 273.15,
        s.relative_humidity * 100.0,
        s.wind_speed_m_s,
        s.wind_direction_from_rad.to_degrees(),
    );
    for level in profile.levels.iter().filter(|l| l.height_msl_m < 6_000.0) {
        println!(
            "{:>11.0} {:>12.0} {:>18.1} {:>14.0} {:>12.1} {:>10.0}",
            level.pressure_pa / 100.0,
            level.height_msl_m,
            level.temperature_k - 273.15,
            level.relative_humidity * 100.0,
            level.wind_speed_m_s,
            level.wind_direction_from_rad.to_degrees(),
        );
    }
    let underground: Vec<String> = profile
        .dropped
        .iter()
        .filter(|d| d.reason == DropReason::BelowGround)
        .map(|d| format!("{:.0}", d.pressure_pa / 100.0))
        .collect();
    println!("Below the ground, left out: {} hPa", underground.join(", "));

    // 3. The atmosphere as a flight uses it: hydrostatic between levels, with humidity, and the
    //    wind's speed and direction interpolated in height.
    let sounding = profile.sounding(WindInterpolation::SpeedDirection)?;
    let standard = Ussa76::standard();
    let pad_msl_m = s.height_msl_m;
    println!();
    println!("height above the pad (m)   pressure (hPa)   temperature (°C)   density (kg/m³)");
    for above_m in [0.0, 1_000.0, 3_000.0] {
        for (name, air) in [
            ("Open-Meteo", sounding.air(pad_msl_m + above_m)?.air),
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

    // 4. RocketPy's Calisto flown in both, without its parachutes.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
    ))?;
    // Heights above sea level are taken as heights above the ellipsoid (geoid undulation 0).
    let site = Geodetic::from_degrees(latitude_deg, longitude_deg, pad_msl_m)?;
    let wind = sounding.wind().ok_or("the profile has no wind")?.clone();
    let forecast = Environment::new(Earth::wgs84(site)?, sounding.clone(), wind);
    let calm = Environment::new(Earth::wgs84(site)?, standard, ConstantWind::calm());
    println!();
    println!("Calisto to apogee      apogee (m above the pad)   east of the pad (m)   north (m)");
    for (name, environment) in [("Open-Meteo", forecast), ("standard, calm", calm)] {
        let simulation = Simulation::new(
            &rocket,
            "example",
            environment,
            Rail::vertical(5.2),
            FlightSettings::default(),
        )?;
        let flight = simulation.run(&mut ())?;
        let apogee = flight
            .event(EventKind::Apogee)
            .ok_or("the flight has no apogee")?
            .sample;
        // Where it is at apogee: east and north of the pad, negative for west and south.
        println!(
            "{name:<22} {:>26.1} {:>21.1} {:>11.1}",
            apogee.height_above_ground_m,
            tidy(apogee.cg_enu_m.x),
            tidy(apogee.cg_enu_m.y)
        );
    }
    Ok(())
}

/// `x` rounded to 0.1, with a rounded `-0.0` printed as `0.0`.
fn tidy(x: f64) -> f64 {
    let rounded = (x * 10.0).round() / 10.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}
