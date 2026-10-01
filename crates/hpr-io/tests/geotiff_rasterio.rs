//! `hpr_io::geotiff` against rasterio 1.5.2 (GDAL 3.12.2), the outside reader (M5.3c2).
//!
//! `validation/oracles/geotiff/dem.py` cut the fixtures from a public-domain USGS 3DEP tile and
//! wrote rasterio's reading of them, and of the whole tile, to `fixtures/geotiff/rasterio.json`.
//! Every fixture must read to the same size, corner, pixel size, raster type, CRS, scale, offset,
//! vertical unit and nodata value; to the same correctly rounded sums over every pixel; and, at each sampled
//! point, to the same pixel and the same value. The whole tile is checked where `refs/` has it.

#![allow(
    clippy::disallowed_methods,
    reason = "the tests read the fixtures and the fetched tile; not the pure core"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::path::{Path, PathBuf};

use hpr_io::geotiff::{ElevationRaster, GeoTiffError, Pixel, RasterType};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Deserialize)]
struct Oracle {
    rasterio: String,
    gdal: String,
    readings: Vec<Reading>,
}

#[derive(Deserialize)]
struct Reading {
    file: String,
    sha256: String,
    width: u32,
    height: u32,
    transform: [f64; 6],
    area_or_point: Option<String>,
    nodata: Option<f64>,
    horizontal_epsg: Option<u16>,
    vertical_unit_m: Option<f64>,
    vertical_epsg: Option<u16>,
    scale: f64,
    offset: f64,
    /// The band's unit type as GDAL reports it: a vertical CRS's unit, or GDAL_METADATA's.
    units: Option<String>,
    sum: f64,
    weighted_sum: f64,
    nodata_count: usize,
    /// `[lon, lat, row, col, value]`, the value a number, `null` (nodata) or `"outside"`.
    points: Vec<(f64, f64, i64, i64, Value)>,
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn oracle() -> Oracle {
    let text = include_str!("fixtures/geotiff/rasterio.json");
    serde_json::from_str(text).unwrap()
}

/// The correctly rounded sum of `terms`, as Python's `math.fsum` gives it: J. R. Shewchuk's
/// exact partials (*Adaptive precision floating-point arithmetic and fast robust geometric
/// predicates*, Discrete Comput. Geom. 18, 1997, 305-363), then the round-half-even correction
/// of the last step, written from that algorithm.
fn fsum(terms: impl Iterator<Item = f64>) -> f64 {
    let mut partials: Vec<f64> = Vec::new();
    for mut x in terms {
        let mut i = 0;
        for j in 0..partials.len() {
            let mut y = partials[j];
            if x.abs() < y.abs() {
                std::mem::swap(&mut x, &mut y);
            }
            let hi = x + y;
            let lo = y - (hi - x);
            if lo != 0.0 {
                partials[i] = lo;
                i += 1;
            }
            x = hi;
        }
        partials.truncate(i);
        partials.push(x);
    }
    let Some(mut hi) = partials.pop() else {
        return 0.0;
    };
    let mut lo = 0.0;
    while let Some(y) = partials.pop() {
        let x = hi;
        hi = x + y;
        let yr = hi - x;
        lo = y - yr;
        if lo != 0.0 {
            break;
        }
    }
    if let Some(&next) = partials.last()
        && ((lo < 0.0 && next < 0.0) || (lo > 0.0 && next > 0.0))
    {
        let y = lo * 2.0;
        let x = hi + y;
        if y == x - hi {
            hi = x;
        }
    }
    hi
}

/// Checks one file against rasterio's reading; returns the points compared, how many of them
/// are on the raster, and how many of those are nodata.
fn check(bytes: &[u8], reading: &Reading) -> (usize, usize, usize) {
    let name = &reading.file;
    assert_eq!(
        hex(&Sha256::digest(bytes)),
        reading.sha256,
        "{name}: not the file read"
    );
    let raster = ElevationRaster::parse(bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
    let info = raster.info();
    let t = reading.transform;
    assert_eq!(
        (info.width, info.height),
        (reading.width, reading.height),
        "{name}"
    );
    assert_eq!(
        (t[1], t[3]),
        (0.0, 0.0),
        "{name}: rasterio's transform is rotated"
    );
    // The corner and pixel size bit for bit: they place every pixel.
    assert_eq!(
        info.corner_longitude_deg.to_bits(),
        t[2].to_bits(),
        "{name}"
    );
    assert_eq!(info.corner_latitude_deg.to_bits(), t[5].to_bits(), "{name}");
    assert_eq!(info.pixel_longitude_deg.to_bits(), t[0].to_bits(), "{name}");
    assert_eq!(info.pixel_latitude_deg.to_bits(), t[4].to_bits(), "{name}");
    let point = reading.area_or_point.as_deref() == Some("Point");
    assert_eq!(
        info.raster_type == RasterType::PixelIsPoint,
        point,
        "{name}"
    );
    assert_eq!(
        Some(info.geographic_crs_epsg),
        reading.horizontal_epsg,
        "{name}"
    );
    assert_eq!(info.vertical_crs_epsg, reading.vertical_epsg, "{name}");
    assert_eq!(
        (info.scale, info.offset),
        (reading.scale, reading.offset),
        "{name}"
    );
    let unit = info.vertical_unit.metres();
    if let Some(m) = reading.vertical_unit_m {
        // The WKT prints the unit to 15 significant digits.
        assert!(
            (unit - m).abs() <= 1e-15 * m,
            "{name}: {unit} m against {m} m"
        );
    }
    // GDAL's names for the units, exactly: the international foot is 0.3048 m, the US survey
    // foot 1200/3937 m.
    let units = reading.units.as_deref().map(|u| match u {
        "metre" => 1.0,
        "ft" => 0.3048,
        "US survey foot" => 1200.0 / 3937.0,
        other => panic!("{name}: GDAL's unit {other:?} is not one the test knows"),
    });
    if let Some(m) = units {
        assert_eq!(unit, m, "{name}");
    }
    let stated = reading.vertical_unit_m.is_some() || units.is_some();
    assert_eq!(info.vertical_unit_stated, stated, "{name}");
    if !stated {
        assert_eq!(unit, 1.0, "{name}");
    }
    assert_eq!(info.nodata, reading.nodata, "{name}");

    let values = raster.values().unwrap();
    let nodata = values.iter().filter(|v| v.is_nan()).count();
    assert_eq!(nodata, reading.nodata_count, "{name}");
    let sum = fsum(values.iter().copied().filter(|v| !v.is_nan()));
    #[expect(clippy::cast_precision_loss, reason = "indices below 2^53, as numpy's")]
    let weighted = fsum(
        values
            .iter()
            .enumerate()
            .filter(|(_, v)| !v.is_nan())
            .map(|(i, v)| v * (i + 1) as f64),
    );
    assert_eq!(
        sum.to_bits(),
        reading.sum.to_bits(),
        "{name}: {sum} against {}",
        reading.sum
    );
    assert_eq!(
        weighted.to_bits(),
        reading.weighted_sum.to_bits(),
        "{name}: {weighted} against {}",
        reading.weighted_sum
    );

    // Read together, each tile decoded once (the whole tile's 2,000 points one by one take
    // minutes in a debug build); a few read alone, which must agree.
    let latlon: Vec<(f64, f64)> = reading.points.iter().map(|p| (p.1, p.0)).collect();
    let together = raster.values_at(&latlon);
    let mut inside = 0;
    for (k, (&(lon, lat, row, col, ref value), read)) in
        reading.points.iter().zip(together).enumerate()
    {
        let at = format!("{name} at ({lat}, {lon})");
        let pixel = raster.info().pixel_of(lat, lon);
        if k % 97 == 0 {
            assert_eq!(raster.value_at(lat, lon), read, "{at}: alone and together");
        }
        if value.as_str() == Some("outside") {
            assert_eq!(pixel, None, "{at}: rasterio has row {row}, column {col}");
            assert!(
                matches!(read, Err(GeoTiffError::Outside { .. })),
                "{at}: {read:?}"
            );
            continue;
        }
        inside += 1;
        let expected = Pixel {
            row: u32::try_from(row).unwrap(),
            col: u32::try_from(col).unwrap(),
        };
        assert_eq!(pixel, Some(expected), "{at}: rasterio's pixel");
        let read = read.unwrap_or_else(|e| panic!("{at}: {e}"));
        assert_eq!(read, value.as_f64(), "{at}");
        if k % 97 == 0 {
            // GDAL's scale and offset, then the unit (held to GDAL's above, metres where it has
            // none).
            let height = raster.height_at(lat, lon).unwrap();
            let theirs = read.map(|v| (v * reading.scale + reading.offset) * unit);
            assert_eq!(height, theirs, "{at}");
        }
    }
    let nodata = reading.points.iter().filter(|p| p.4.is_null()).count();
    (reading.points.len(), inside, nodata)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn fixtures_read_as_rasterio_reads_them() {
    let oracle = oracle();
    assert_eq!(
        (oracle.rasterio.as_str(), oracle.gdal.as_str()),
        ("1.5.2", "3.12.2")
    );
    let dir = root().join("crates/hpr-io/tests/fixtures/geotiff");
    let (mut files, mut points, mut inside, mut nodata) = (0, 0, 0, 0);
    for reading in oracle
        .readings
        .iter()
        .filter(|r| !r.file.starts_with("refs/"))
    {
        let bytes = std::fs::read(dir.join(&reading.file)).unwrap();
        let (p, i, n) = check(&bytes, reading);
        assert!(i > p / 2, "{}: {i} of {p} inside", reading.file);
        (files, points, inside, nodata) = (files + 1, points + p, inside + i, nodata + n);
    }
    // The counts the guide's tables quote: 7 files, 2800 places, 2368 on a raster, 9 of them
    // on nodata.
    assert_eq!((files, points, inside, nodata), (7, 2800, 2368, 9));
}

/// The whole USGS tile (44.7 MB, `cargo xtask refs fetch`): 13 million pixels' sums and 2,000
/// points. Passes without checking where `refs/` lacks it, as in CI.
#[test]
fn whole_usgs_tile_reads_as_rasterio_reads_it() {
    let oracle = oracle();
    let reading = oracle
        .readings
        .iter()
        .find(|r| r.file.starts_with("refs/"))
        .unwrap();
    let Ok(bytes) = std::fs::read(root().join(&reading.file)) else {
        eprintln!("{} is not in refs/; skipped", reading.file);
        return;
    };
    // 2000 places, 1696 on the tile, none on nodata.
    assert_eq!(check(&bytes, reading), (2000, 1696, 0));
}

#[test]
fn fsum_rounds_as_python_does() {
    // CPython's own examples of `math.fsum`.
    assert_eq!(
        fsum([1e-16, 1.0, 1e16].into_iter()),
        1.000_000_000_000_000_2e16
    );
    assert_eq!(fsum([0.1; 10].into_iter()), 1.0);
    assert_eq!(
        fsum([1e100, 1.0, -1e100, 1e-100, 1e50, -1.0, -1e50].into_iter()),
        1e-100
    );
}
