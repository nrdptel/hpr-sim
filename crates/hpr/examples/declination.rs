//! Magnetic declination from the World Magnetic Model (WMM2025) at four launch sites on
//! 2026-06-20: the declination, the model's estimate of its error and how fast it drifts, the dip
//! and strength of the field, the compass zone, and the true bearing of a launch rail aimed along
//! a compass's north.
//!
//! It uses the workspace crate `hpr-core`; a program of your own outside this repository depends
//! on that one.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example declination -p hpr
//! ```
//!
//! The documentation site's magnetic-field page (`docs/physics/magnetic.md`) quotes what it
//! prints, which is kept next to it in `declination.output.txt`; CI checks that the two still
//! agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_core::geodesy::Geodetic;
use hpr_core::magnetic::{WMM2025, decimal_year};

fn main() -> Result<(), Box<dyn Error>> {
    // Rounded public coordinates; heights are above the WGS 84 ellipsoid, as the model takes them.
    let sites = [
        ("Spaceport America, New Mexico", 32.99, -106.97, 1_380.0),
        ("Black Rock Desert, Nevada", 40.87, -119.06, 1_170.0),
        ("Lucerne Dry Lake, California", 34.50, -116.95, 840.0),
        ("Andøya, Norway", 69.29, 16.02, 45.0),
    ];
    let year = decimal_year(2026, 6, 20)?;
    println!("{} on 2026-06-20 (decimal year {year:.4})", WMM2025.name());
    println!();
    println!(
        "{:<30} {:>7} {:>6} {:>10} {:>7} {:>7} {:>7}  zone",
        "site", "D (°)", "± (°)", "dD (°/yr)", "I (°)", "F (nT)", "GV (°)"
    );
    for (name, lat, lon, height_m) in sites {
        let field = WMM2025.field(Geodetic::from_degrees(lat, lon, height_m)?, year)?;
        let grid = field
            .grid_variation_rad
            .map_or_else(|| "-".to_owned(), |gv| format!("{:.2}", gv.to_degrees()));
        println!(
            "{name:<30} {:>7.2} {:>6.2} {:>10.3} {:>7.2} {:>7.0} {:>7}  {:?}",
            field.declination_rad.to_degrees(),
            field.declination_uncertainty_rad().to_degrees(),
            field.declination_rate_rad_per_year.to_degrees(),
            field.inclination_rad.to_degrees(),
            field.total_nt,
            grid,
            field.compass_zone(),
        );
    }

    let (name, lat, lon, height_m) = sites[0];
    let field = WMM2025.field(Geodetic::from_degrees(lat, lon, height_m)?, year)?;
    let rail_true = field.true_from_magnetic_rad(0.0).to_degrees();
    println!();
    println!(
        "At {name}, a rail aimed at magnetic north points {:.2}° {} of true north.",
        rail_true.abs(),
        if rail_true >= 0.0 { "east" } else { "west" }
    );
    Ok(())
}
