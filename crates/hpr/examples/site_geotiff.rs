//! A launch site's height from an elevation file the user gives: a GeoTIFF cut from the USGS's
//! 1-arc-second terrain model around Spaceport America, its georeferencing, the height at three
//! places (one over a hole in the data, one off the file), the same site in a copy stored in US
//! survey feet, and the standard atmosphere at the height.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example site_geotiff -p hpr
//! ```
//!
//! The documentation site's *A launch site's elevation* page (`docs/elevation.md`) walks through
//! it. What it prints is kept next to it in `site_geotiff.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::{Atmosphere, Ussa76};
use hpr_io::geotiff::ElevationRaster;

fn main() -> Result<(), Box<dyn Error>> {
    // A real program reads the user's file: `let bytes = std::fs::read(path)?;`. These are two
    // copies of the same 70 by 50 pixels, cut from the public-domain USGS 3DEP tile n33w107.
    let metres = include_bytes!("../../hpr-io/tests/fixtures/geotiff/usgs-f32-lzw-fp-tiles.tif");
    let feet =
        include_bytes!("../../hpr-io/tests/fixtures/geotiff/usgs-u16-packbits-ftus-nodata.tif");

    // 1. Reading the tags: where the pixels lie, and what their values are.
    let raster = ElevationRaster::parse(metres)?;
    let info = raster.info();
    let bounds = info.bounds();
    println!(
        "{} by {} pixels of {:.3}\" by {:.3}\", {:?}, CRS EPSG:{}, values {:?} in {:?} (stated: {})",
        info.width,
        info.height,
        info.pixel_longitude_deg * 3600.0,
        -info.pixel_latitude_deg * 3600.0,
        info.raster_type,
        info.geographic_crs_epsg,
        info.sample,
        info.vertical_unit,
        info.vertical_unit_stated,
    );
    println!(
        "latitudes {:.5} to {:.5}, longitudes {:.5} to {:.5}",
        bounds.south_deg, bounds.north_deg, bounds.west_deg, bounds.east_deg
    );
    println!();

    // 2. Heights: each the value of the pixel the place falls in.
    let places = [
        ("Spaceport America, New Mexico", 32.99, -106.97),
        ("a hole in the data", 32.996, -106.9832),
        ("Albuquerque, New Mexico", 35.0844, -106.6504),
    ];
    println!("{:<30} {:>9} {:>11}  height", "place", "lat (°)", "lon (°)");
    for (name, lat, lon) in places {
        let height = match raster.height_at(lat, lon) {
            Ok(Some(m)) => format!("{m:.3} m"),
            Ok(None) => "no data".to_string(),
            Err(e) => format!("refused: {e}"),
        };
        println!("{name:<30} {lat:>9.4} {lon:>11.4}  {height}");
    }
    println!();

    // 3. The same site in the copy stored in whole US survey feet, read back in metres.
    let (lat, lon) = (32.99, -106.97);
    let in_feet = ElevationRaster::parse(feet)?;
    let raw = in_feet.value_at(lat, lon)?.unwrap_or(f64::NAN);
    let height = raster.height_at(lat, lon)?.unwrap_or(f64::NAN);
    let from_feet = in_feet.height_at(lat, lon)?.unwrap_or(f64::NAN);
    println!(
        "in feet: {raw} {:?} (vertical CRS EPSG:{}) = {from_feet:.3} m, {:+.3} m from the metres file",
        in_feet.info().vertical_unit,
        in_feet.info().vertical_crs_epsg.unwrap_or(0),
        from_feet - height,
    );

    // 4. The standard atmosphere there: the height starts a flight at the right air.
    let standard = Ussa76::standard();
    let sea = standard.air(0.0)?.air;
    let air = standard.air(height)?.air;
    println!(
        "at {height:.1} m: {:.2} hPa, density {:.3} of sea level's",
        air.pressure_pa / 100.0,
        air.density_kg_m3 / sea.density_kg_m3
    );
    Ok(())
}
