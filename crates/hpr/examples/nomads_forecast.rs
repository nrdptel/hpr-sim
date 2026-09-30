//! NOAA's GFS and RAP forecasts over a launch site, cut from NOMADS' grib filter, and a flight
//! through each beside the same flight in the standard atmosphere with no wind.
//!
//! Run it from anywhere in the repository (it needs the `net` feature):
//!
//! ```text
//! cargo run --example nomads_forecast -p hpr --features net
//! ```
//!
//! The documentation site's *NOAA forecasts: GFS and RAP* page (`docs/nomads.md`) walks through
//! it. What it prints is kept next to it in `nomads_forecast.output.txt`, and CI checks that the
//! two still agree (`cargo xtask examples --check`).
//!
//! It never uses the network: a stand-in transport answers with two cuts recorded from NOMADS for
//! Spaceport America, both for 2026-09-30 18 UTC: the GFS run of 00 UTC at hour 18 and the RAP run
//! of 12 UTC at hour 6. Forecast data from NOAA/NCEP, U.S. government works.

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
use hpr_atmos::{ConstantWind, Ussa76};
use hpr_core::earth::Earth;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_net::nomads::{self, NomadsModel, NomadsProfile, NomadsRequest};
use hpr_net::{Cache, Client, Mode, Transport};
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};

/// Answers with the recorded RAP cut when the URL names RAP's grib filter, and with the GFS cut
/// otherwise. A real program uses `hpr_net::Http::new()` here, which fetches from NOMADS.
struct Recorded;

impl Transport for Recorded {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        let body: &[u8] = if url.contains("filter_rap.pl") {
            include_bytes!("../../hpr-net/tests/fixtures/replay/nomads-rap.grib2")
        } else {
            include_bytes!("../../hpr-net/tests/fixtures/replay/nomads-gfs.grib2")
        };
        Ok(body.to_vec())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. A client: the transport, a cache folder, and online mode. A real program would keep its
    //    cache in `Cache::platform_dir()`, so the forecast is there offline next time.
    let folder = std::env::temp_dir().join(format!("hpr-nomads-{}", std::process::id()));
    let client = Client::new(Recorded, Cache::new(&folder), Mode::Online);

    // 2. Spaceport America's launch area, and a launch at 2026-09-30 18 UTC. GFS runs every 6
    //    hours: its 00 UTC run, 18 hours ahead. RAP runs every hour: its 12 UTC run, 6 hours ahead.
    let (latitude_deg, longitude_deg) = (32.99, -106.97);
    let gfs = NomadsRequest::new(
        latitude_deg,
        longitude_deg,
        NomadsModel::Gfs,
        1_790_726_400,
        18,
    );
    let rap = NomadsRequest::new(
        latitude_deg,
        longitude_deg,
        NomadsModel::Rap,
        1_790_769_600,
        6,
    );
    let now_s = 1_790_800_000;
    let (gfs_profile, fetched) = nomads::fetch(&client, &gfs, now_s)?;
    let (rap_profile, _) = nomads::fetch(&client, &rap, now_s)?;
    std::fs::remove_dir_all(&folder).ok();

    println!("Spaceport America, 2026-09-30 18 UTC");
    println!("{}", fetched.attribution);
    // `Fetched`: from the transport, not the cache. A second call within 30 days says `Cached`.
    println!("freshness: {:?}", fetched.freshness);
    let models = [("GFS", &gfs_profile), ("RAP", &rap_profile)];
    for (name, profile) in models {
        println!(
            "{name}: run of {:02} UTC, hour {}: {} levels kept, {} left out",
            (profile.cycle_unix_s % 86_400) / 3_600,
            (profile.valid_unix_s - profile.cycle_unix_s) / 3_600,
            profile.levels.len(),
            profile.dropped.len()
        );
    }
    // RAP gives its winds along its grid's axes; they were turned to east and north.
    println!(
        "RAP's winds turned by {:.2}° to east and north",
        rap_profile.wind_turn_rad.to_degrees()
    );

    let standard_levels = [85_000.0, 70_000.0, 50_000.0, 25_000.0, 10_000.0];
    for (name, profile) in models {
        println!();
        println!(
            "{name:<5} level (hPa)   height (m)   temperature (°C)   humidity (%)   wind (m/s)   from (°)"
        );
        let s = &profile.surface;
        row(
            s.pressure_pa,
            s.height_msl_m,
            s.temperature_k,
            s.relative_humidity,
            (s.wind_east_m_s, s.wind_north_m_s),
            "   the ground",
        );
        for level in profile
            .levels
            .iter()
            .filter(|l| standard_levels.contains(&l.pressure_pa))
        {
            row(
                level.pressure_pa,
                level.height_msl_m,
                level.temperature_k,
                level.relative_humidity,
                (level.wind_east_m_s, level.wind_north_m_s),
                "",
            );
        }
    }

    // 3. RocketPy's Calisto flown from the site in each forecast, without its parachutes.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
    ))?;
    println!();
    println!("Calisto to apogee      apogee (m above the pad)   east of the pad (m)   north (m)");
    for (name, profile) in models {
        fly(&rocket, name, environment(profile)?)?;
    }
    // The standard atmosphere, calm, from GFS's ground.
    let site = Geodetic::from_degrees(
        latitude_deg,
        longitude_deg,
        gfs_profile.surface.height_msl_m,
    )?;
    let calm = Environment::new(
        Earth::wgs84(site)?,
        Ussa76::standard(),
        ConstantWind::calm(),
    );
    fly(&rocket, "standard, calm", calm)?;
    Ok(())
}

/// Prints one row of a level table: pressure, height, temperature, humidity, and the wind's speed
/// and the direction it blows from, clockwise from north.
fn row(
    pressure_pa: f64,
    height_msl_m: f64,
    temperature_k: f64,
    relative_humidity: f64,
    (east_m_s, north_m_s): (f64, f64),
    note: &str,
) {
    let from_deg = (-east_m_s).atan2(-north_m_s).to_degrees().rem_euclid(360.0);
    println!(
        "{:>17.0} {:>12.0} {:>18.1} {:>14.0} {:>12.1} {:>10.0}{note}",
        pressure_pa / 100.0,
        height_msl_m,
        temperature_k - 273.15,
        relative_humidity * 100.0,
        east_m_s.hypot(north_m_s),
        from_deg,
    );
}

/// The forecast as a flight's environment: the atmosphere and its wind, with the pad on the
/// model's ground. Heights above sea level are taken as heights above the ellipsoid (geoid
/// undulation 0).
fn environment(profile: &NomadsProfile) -> Result<Environment, Box<dyn Error>> {
    let air = profile.sounding(WindInterpolation::SpeedDirection)?;
    let wind = air.wind().ok_or("the forecast has no wind")?.clone();
    let site = Geodetic::from_degrees(
        profile.latitude_deg,
        profile.longitude_deg,
        profile.surface.height_msl_m,
    )?;
    Ok(Environment::new(Earth::wgs84(site)?, air, wind))
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
