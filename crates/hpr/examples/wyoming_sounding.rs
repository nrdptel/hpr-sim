//! A weather-balloon sounding from the University of Wyoming's archive, and a flight through it
//! beside the same flight in the standard atmosphere with no wind.
//!
//! Run it from anywhere in the repository (it needs the `net` feature):
//!
//! ```text
//! cargo run --example wyoming_sounding -p hpr --features net
//! ```
//!
//! The documentation site's *Weather-balloon soundings* page (`docs/soundings.md`) walks through
//! it. What it prints is kept next to it in `wyoming_sounding.output.txt`, and CI checks that the
//! two still agree (`cargo xtask examples --check`).
//!
//! It never uses the network: a stand-in transport answers with the two versions of the Santa
//! Teresa, New Mexico, sounding of 21 June 2025, 12 UTC, recorded from the archive for the tests.

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
use hpr_net::wyoming::{self, WyomingRequest, WyomingVersion};
use hpr_net::{Cache, Client, Mode, Transport};
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};

/// Answers with the recorded sounding, the BUFR file or the coded message as the URL asks. A real
/// program uses `hpr_net::Http::new()` here, which fetches from the archive.
struct Recorded;

impl Transport for Recorded {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let body: &[u8] = if url.ends_with("src=BUFR") {
            include_bytes!("../../hpr-net/tests/fixtures/replay/wyoming-72364-bufr.csv")
        } else {
            include_bytes!("../../hpr-net/tests/fixtures/replay/wyoming-72364-fm35.csv")
        };
        Ok(body.to_vec())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. A client: the transport, a cache folder, and online mode. A real program would keep its
    //    cache in `Cache::platform_dir()`, so the sounding is there offline next time.
    let folder = std::env::temp_dir().join(format!("hpr-wyoming-{}", std::process::id()));
    let client = Client::new(Recorded, Cache::new(&folder), Mode::Online);

    // 2. A launch at 2025-06-21 15:30 UTC had the 12 UTC sounding from Santa Teresa (station
    //    72364), first as the coded message, then as the BUFR file.
    let launch_s = 1_750_519_800;
    let request = WyomingRequest::latest_before("72364", launch_s);
    let mut detailed = request.clone();
    detailed.version = WyomingVersion::Bufr;
    let now_s = 1_790_000_000;
    let (coded, fetched) = wyoming::fetch(&client, &request, now_s)?;
    let (bufr, _) = wyoming::fetch(&client, &detailed, now_s)?;
    std::fs::remove_dir_all(&folder).ok();

    println!("Santa Teresa, New Mexico (72364), 2025-06-21 12 UTC");
    println!("{}", fetched.attribution);
    // `Fetched`: from the transport, not the cache. A second call within the day says `Cached`.
    println!("freshness: {:?}", fetched.freshness);
    let release_min = (request.time_unix_s - coded.release_unix_s) / 60;
    println!("released {release_min} minutes before 12 UTC");
    for (name, sounding) in [("coded message", &coded), ("BUFR file", &bufr)] {
        println!(
            "{name}: {} levels kept, {} left out",
            sounding.levels.len(),
            sounding.dropped.len()
        );
    }
    println!();
    println!("level (hPa)   height (m)   temperature (°C)   humidity (%)   wind (m/s)   from (°)");
    let ground = coded.levels[0];
    let standard_levels = [85_000.0, 70_000.0, 50_000.0, 25_000.0, 10_000.0];
    for (i, level) in coded.levels.iter().enumerate() {
        if i > 0 && !standard_levels.contains(&level.pressure_pa) {
            continue;
        }
        println!(
            "{:>11.0} {:>12.0} {:>18.1} {:>14.0} {:>12.1} {:>10.0}{}",
            level.pressure_pa / 100.0,
            level.height_msl_m,
            level.temperature_k - 273.15,
            level.relative_humidity * 100.0,
            level.wind_speed_m_s,
            level.wind_direction_from_rad.to_degrees(),
            if i == 0 { "   the ground" } else { "" },
        );
    }

    // 3. The atmosphere as a flight uses it: hydrostatic between levels, with humidity, and the
    //    wind's speed and direction interpolated in height.
    let sounding = coded.sounding(WindInterpolation::SpeedDirection)?;
    let standard = Ussa76::standard();
    let pad_msl_m = ground.height_msl_m;
    println!();
    println!("height above the pad (m)   pressure (hPa)   temperature (°C)   density (kg/m³)");
    for above_m in [0.0, 1_000.0, 3_000.0] {
        for (name, air) in [
            ("sounding", sounding.air(pad_msl_m + above_m)?.air),
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

    // 4. RocketPy's Calisto flown from the station in each, without its parachutes.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
    ))?;
    // The BUFR file again, without its rows between the ground and the coded message's second row
    // (1,438 m of geopotential height): the two versions differ most there.
    let second_row_m = coded.levels[1].geopotential_height_m;
    let mut bufr_above = bufr.clone();
    bufr_above
        .levels
        .retain(|l| l == &bufr.levels[0] || l.geopotential_height_m >= second_row_m);
    let mut environments = Vec::new();
    for (name, balloon) in [
        ("coded message", &coded),
        ("BUFR file", &bufr),
        ("BUFR above 1,438 m", &bufr_above),
    ] {
        let air = balloon.sounding(WindInterpolation::SpeedDirection)?;
        let wind = air.wind().ok_or("the sounding has no wind")?.clone();
        // Each flight starts on its own sounding's ground. Heights above sea level are taken as
        // heights above the ellipsoid (geoid undulation 0).
        let ground = balloon.levels[0];
        let site =
            Geodetic::from_degrees(coded.latitude_deg, coded.longitude_deg, ground.height_msl_m)?;
        environments.push((name, Environment::new(Earth::wgs84(site)?, air, wind)));
    }
    let site = Geodetic::from_degrees(coded.latitude_deg, coded.longitude_deg, pad_msl_m)?;
    let calm = Environment::new(Earth::wgs84(site)?, standard, ConstantWind::calm());
    println!();
    println!("Calisto to apogee      apogee (m above the pad)   east of the pad (m)   north (m)");
    for (name, environment) in environments {
        fly(&rocket, name, environment)?;
    }
    fly(&rocket, "standard, calm", calm)?;
    Ok(())
}

/// Flies `rocket` to apogee in `environment` and prints the apogee and where it is: east and north
/// of the pad, negative for west and south.
fn fly(rocket: &Rocket, name: &str, environment: Environment) -> Result<(), Box<dyn Error>> {
    let simulation = Simulation::new(
        rocket,
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
    println!(
        "{name:<22} {:>26.1} {:>21.1} {:>11.1}",
        apogee.height_above_ground_m,
        tidy(apogee.cg_enu_m.x),
        tidy(apogee.cg_enu_m.y)
    );
    Ok(())
}

/// `x` rounded to 0.1, with a rounded `-0.0` printed as `0.0`.
fn tidy(x: f64) -> f64 {
    let rounded = (x * 10.0).round() / 10.0;
    if rounded == 0.0 { 0.0 } else { rounded }
}
