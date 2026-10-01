//! Motor stock and prices from motor.fusionspace.co: the in-stock list fetched and cached, read
//! again offline, and the L motors in stock listed cheapest first, by the price of one motor.
//!
//! Run it from anywhere in the repository (it needs the `net` feature):
//!
//! ```text
//! cargo run --example motor_stock -p hpr --features net
//! ```
//!
//! The documentation site's *Motor stock and prices* page (`docs/motor-stock.md`) walks through
//! it. What it prints is kept next to it in `motor_stock.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).
//!
//! It never uses the network: a stand-in transport answers with the files recorded from the
//! motor finder's API on 1 October 2026, at 07:07 UTC. Stock and prices change by the hour.

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

use hpr_net::motor_finder::{self, Endpoint};
use hpr_net::{Cache, Client, Mode, Transport};

/// Answers the two URLs the example asks for with their recorded files. A real program uses
/// `hpr_net::Http::new()` here, which fetches from motor.fusionspace.co.
struct Recorded;

impl Transport for Recorded {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        if url == Endpoint::Meta.url() {
            Ok(
                include_bytes!("../../hpr-net/tests/fixtures/replay/motor-finder-meta.json")
                    .to_vec(),
            )
        } else if url == Endpoint::InStock.url() {
            let body =
                include_bytes!("../../hpr-net/tests/fixtures/replay/motor-finder-in-stock.json");
            Ok(body.to_vec())
        } else {
            Err(format!("no recording of {url}"))
        }
    }
}

/// A transport with no network, as on a field with no signal.
struct NoSignal;

impl Transport for NoSignal {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        Err(format!("no network for {url}"))
    }
}

/// Cents as dollars: `26099` as `$260.99`.
fn dollars(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Online: when the files were built, then the in-stock list, both fetched and cached. A
    //    real program would keep its cache in `Cache::platform_dir()`, so the list is there
    //    offline next time.
    let folder = std::env::temp_dir().join(format!("hpr-motor-stock-{}", std::process::id()));
    let online = Client::new(Recorded, Cache::new(&folder), Mode::Online);
    let now_s = 1_790_839_800; // 2026-10-01 07:30 UTC
    let (meta, _) = motor_finder::fetch_meta(&online, now_s)?;
    let (in_stock, fetched) = motor_finder::fetch_in_stock(&online, now_s)?;

    // 2. Offline, two hours later: the same list comes from the cache, now stale (older than an
    //    hour), and the transport is never asked.
    let offline = Client::new(NoSignal, Cache::new(&folder), Mode::Offline);
    let (again, cached) = motor_finder::fetch_in_stock(&offline, now_s + 7_200)?;
    std::fs::remove_dir_all(&folder).ok();

    println!("{}", fetched.attribution);
    println!(
        "built {}: {} motors listed, {} in stock, from {} vendors",
        meta.generated_at, meta.counts.motors, meta.counts.in_stock, meta.counts.vendors
    );
    println!(
        "first read: {:?}; offline two hours later: {:?}, the same list: {}",
        fetched.freshness,
        cached.freshness,
        again == in_stock
    );
    println!();

    // 3. The L motors in stock, cheapest first by the price of one motor; an offer whose vendor
    //    shows no price goes last.
    let mut l_class: Vec<_> = in_stock
        .motors
        .iter()
        .filter(|m| m.impulse_class == "L")
        .filter_map(|m| Some((m, m.cheapest_in_stock.as_ref()?)))
        .collect();
    l_class.sort_by_key(|(m, offer)| {
        let unit = offer.unit_price_cents;
        (unit.is_none(), unit, m.designation.clone())
    });
    println!(
        "{} L motors in stock; the five cheapest by the price of one motor:",
        l_class.len()
    );
    println!(
        "{:<20} {:<12} {:>9} {:>14} {:>11}  {:<24} {:>7}",
        "manufacturer", "motor", "dia (mm)", "impulse (N·s)", "one motor", "cheapest at", "vendors"
    );
    for (motor, offer) in l_class.iter().take(5) {
        println!(
            "{:<20} {:<12} {:>9.0} {:>14.1} {:>11}  {:<24} {:>7}",
            motor.manufacturer,
            motor.designation,
            motor.diameter_mm,
            motor.total_impulse_ns.unwrap_or(f64::NAN),
            offer
                .unit_price_cents
                .map_or_else(|| "no price".to_owned(), dollars),
            offer.vendor,
            motor.in_stock_vendor_count,
        );
    }
    Ok(())
}
