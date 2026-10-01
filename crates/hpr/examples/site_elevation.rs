//! A launch site's ground elevation from Open-Meteo: three places looked up in one request, the
//! same lookup again offline from the cache, the standard atmosphere at each ground, and a
//! flight's environment placed on the first.
//!
//! Run it from anywhere in the repository (it needs the `net` feature):
//!
//! ```text
//! cargo run --example site_elevation -p hpr --features net
//! ```
//!
//! The documentation site's *A launch site's elevation* page (`docs/elevation.md`) walks through
//! it. What it prints is kept next to it in `site_elevation.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).
//!
//! It never uses the network: a stand-in transport answers with a response recorded from
//! Open-Meteo's elevation API on 1 October 2026.

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

use hpr_atmos::{Atmosphere, Ussa76};
use hpr_core::geodesy::Geodetic;
use hpr_net::elevation::{self, ElevationRequest, Place};
use hpr_net::{Cache, Client, Mode, Transport};
use hpr_sim::Environment;

/// Answers every URL with the recorded response. A real program uses `hpr_net::Http::new()`
/// here, which fetches from Open-Meteo.
struct Recorded;

impl Transport for Recorded {
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        let body =
            include_bytes!("../../hpr-net/tests/fixtures/replay/open-meteo-elevation-three.json");
        Ok(body.to_vec())
    }
}

/// A transport with no network, as on a field with no signal.
struct NoSignal;

impl Transport for NoSignal {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        Err(format!("no network for {url}"))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let names = [
        "Spaceport America, New Mexico",
        "the Dead Sea's shore",
        "the open Atlantic",
    ];
    let places = vec![
        Place::new(32.99, -106.97),
        Place::new(31.5, 35.5),
        Place::new(0.0, -30.0),
    ];
    let request = ElevationRequest::new(places.clone());

    // 1. Online: the heights are fetched and cached. A real program would keep its cache in
    //    `Cache::platform_dir()`, so the heights are there offline next time.
    let folder = std::env::temp_dir().join(format!("hpr-elevation-{}", std::process::id()));
    let online = Client::new(Recorded, Cache::new(&folder), Mode::Online);
    let now_s = 1_790_812_800;
    let (heights, fetched) = elevation::fetch(&online, &request, now_s)?;

    // 2. Offline, a day later: the same lookup comes from the cache, and the transport is never
    //    asked.
    let offline = Client::new(NoSignal, Cache::new(&folder), Mode::Offline);
    let (again, cached) = elevation::fetch(&offline, &request, now_s + 86_400)?;
    std::fs::remove_dir_all(&folder).ok();

    println!("{}", fetched.attribution);
    println!(
        "first lookup: {:?}; offline a day later: {:?}, the same heights: {}",
        fetched.freshness,
        cached.freshness,
        again == heights
    );
    println!();

    // 3. The standard atmosphere at each ground, beside sea level's.
    let standard = Ussa76::standard();
    let sea = standard.air(0.0)?.air;
    println!(
        "{:<30} {:>9} {:>14} {:>16} {:>14}",
        "place", "lat (°)", "ground (m)", "pressure (hPa)", "density / sea"
    );
    for ((name, place), height_m) in names.iter().zip(&places).zip(&heights) {
        let air = standard.air(*height_m)?.air;
        println!(
            "{name:<30} {:>9.2} {height_m:>14.0} {:>16.1} {:>14.3}",
            place.latitude_deg,
            air.pressure_pa / 100.0,
            air.density_kg_m3 / sea.density_kg_m3,
        );
    }

    // 4. A flight's environment on the first ground. The height is above mean sea level and a
    //    site takes its height above the WGS 84 ellipsoid; with no geoid model, give the same
    //    number and leave the geoid undulation at 0, so the atmosphere sees the right height.
    let site = Geodetic::from_degrees(places[0].latitude_deg, places[0].longitude_deg, heights[0])?;
    let environment = Environment::standard(site)?;
    println!();
    println!(
        "A flight from {} starts {:.0} m above sea level, at {:.1} hPa.",
        names[0],
        environment.site().height_m - environment.geoid_undulation_m,
        environment
            .atmosphere
            .air(environment.site().height_m - environment.geoid_undulation_m)?
            .air
            .pressure_pa
            / 100.0,
    );
    Ok(())
}
