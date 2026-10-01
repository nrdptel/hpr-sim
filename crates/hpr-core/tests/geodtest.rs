//! Geodesics against Karney's test set (M5.3c1).
//!
//! C. F. F. Karney, *Test set for geodesics*, doi:10.5281/zenodo.32156 (CC0): 500,000 WGS 84
//! geodesics, each line `φ₁ λ₁ α₁ φ₂ λ₂ α₂ s₁₂ σ₁₂ m₁₂ S₁₂` (degrees, metres), computed in high
//! precision from the exact `φ₁`, `α₁` and `s₁₂`. The lines come in nine runs of kinds; see
//! [`CATEGORIES`]. Every 500th line is committed as `tests/fixtures/geodtest-every-500th.dat`;
//! the whole file is read from `refs/sources/geodtest/GeodTest.dat` where `cargo xtask refs
//! fetch` has put it.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests read the fetched set and write the report; not the pure core"
)]
#![allow(
    clippy::expect_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;
use std::path::PathBuf;

use hpr_core::DVec3;
use hpr_core::geodesy::{Ellipsoid, Geodetic, ecef_from_enu_rotation};

const WGS84: Ellipsoid = Ellipsoid::WGS84;

/// The test set's kinds, by their first line (0-based), from its description.
const CATEGORIES: [(usize, &str); 9] = [
    (0, "random"),
    (100_000, "nearly antipodal"),
    (150_000, "short distances"),
    (200_000, "one end near a pole"),
    (250_000, "both ends near opposite poles"),
    (300_000, "nearly meridional"),
    (350_000, "nearly equatorial"),
    (400_000, "between vertices"),
    (450_000, "ending close to vertices"),
];

fn category(line: usize) -> usize {
    CATEGORIES
        .iter()
        .rposition(|&(first, _)| line >= first)
        .unwrap_or(0)
}

struct Case {
    line: usize,
    lat1: f64,
    lon1: f64,
    azi1: f64,
    lat2: f64,
    lon2: f64,
    azi2: f64,
    s12: f64,
    m12: f64,
}

fn parse(text: &str, line_of: impl Fn(usize) -> usize) -> Vec<Case> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .enumerate()
        .map(|(k, l)| {
            let v: Vec<f64> = l
                .split_whitespace()
                .map(|x| x.parse().expect("a number"))
                .collect();
            assert_eq!(v.len(), 10, "ten fields: {l}");
            Case {
                line: line_of(k),
                lat1: v[0],
                lon1: v[1],
                azi1: v[2],
                lat2: v[3],
                lon2: v[4],
                azi2: v[5],
                s12: v[6],
                m12: v[8],
            }
        })
        .collect()
}

/// An azimuth difference wrapped into `[−π, π]`, rad.
fn azimuth_difference(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(TAU);
    if d > PI { d - TAU } else { d }
}

/// The unit vector of travel at `point` heading `azimuth_rad`, in ECEF.
fn heading(point: Geodetic, azimuth_rad: f64) -> DVec3 {
    let enu = ecef_from_enu_rotation(point);
    enu.x_axis * azimuth_rad.sin() + enu.y_axis * azimuth_rad.cos()
}

/// The angle between two headings, each at its own point, rad. Near a pole an azimuth turns
/// through large angles as the point moves; the heading itself does not.
fn heading_difference(a: Geodetic, azimuth_a: f64, b: Geodetic, azimuth_b: f64) -> f64 {
    let (u, v) = (heading(a, azimuth_a), heading(b, azimuth_b));
    2.0 * ((u - v).length() / 2.0).asin()
}

/// Karney's bound on round-off in both problems on WGS 84 (Karney 2013 §7, page 10), m.
const KARNEY_BOUND_M: f64 = 15e-9;

/// The largest errors in one kind of geodesic, m.
#[derive(Default, Clone, Copy)]
struct Errors {
    count: usize,
    /// Inverse: `|s₁₂ − s₁₂,ref|`.
    inverse_distance_m: f64,
    /// Inverse: how far the direct problem from point 1, with the inverse's `α₁` and `s₁₂`,
    /// lands from point 2.
    inverse_landing_m: f64,
    /// Inverse: the larger azimuth error times `|m₁₂|`, the sideways miss it stands for at the
    /// other end.
    inverse_azimuth_m: f64,
    /// Inverse: lines whose azimuth error times `|m₁₂|` passes [`KARNEY_BOUND_M`], each within
    /// the turn a one-ulp move of an input gives (see [`input_turn_rad`]).
    inverse_azimuth_ill_conditioned: usize,
    /// Direct: the end point's distance from the reference's.
    direct_position_m: f64,
    /// Direct: the angle between the computed and reference headings at the end, times `|m₁₂|`.
    direct_azimuth_m: f64,
}

/// The largest turn of either inverse azimuth when one input coordinate moves by one ulp, rad.
/// The test set's points are rounded to `f64` on reading, by up to half an ulp; where an azimuth
/// turns further than its error under such a move, the error is the input's, not the method's.
fn input_turn_rad(c: &Case) -> f64 {
    let nominal = solve_inverse(c.lat1, c.lat2, c.lon2);
    let step = |x: f64, by: i64| f64::from_bits(x.to_bits().wrapping_add_signed(by));
    let mut turn = 0.0_f64;
    for by in [-1, 1] {
        for (lat1, lat2, lon2) in [
            (step(c.lat1, by), c.lat2, c.lon2),
            (c.lat1, step(c.lat2, by), c.lon2),
            (c.lat1, c.lat2, step(c.lon2, by)),
        ] {
            let moved = solve_inverse(lat1, lat2, lon2);
            turn = turn
                .max(azimuth_difference(moved.0, nominal.0).abs())
                .max(azimuth_difference(moved.1, nominal.1).abs());
        }
    }
    turn
}

/// The inverse's azimuths from `(lat1, 0)` to `(lat2, lon2)`, degrees in, rad out.
fn solve_inverse(lat1: f64, lat2: f64, lon2: f64) -> (f64, f64) {
    let p1 = Geodetic::from_degrees(lat1, 0.0, 0.0).expect("a point");
    let p2 = Geodetic::from_degrees(lat2, lon2, 0.0).expect("a point");
    let g = WGS84.geodesic_inverse(p1, p2);
    (g.initial_azimuth_rad, g.final_azimuth_rad)
}

fn measure(cases: &[Case]) -> Vec<Errors> {
    let mut errors = vec![Errors::default(); CATEGORIES.len()];
    for c in cases {
        assert_eq!(
            c.lon1, 0.0,
            "line {}: the set starts every geodesic at λ₁ = 0",
            c.line
        );
        let p1 = Geodetic::from_degrees(c.lat1, c.lon1, 0.0).expect("a point");
        let p2 = Geodetic::from_degrees(c.lat2, c.lon2, 0.0).expect("a point");
        let e = &mut errors[category(c.line)];
        e.count += 1;

        let inverse = WGS84.geodesic_inverse(p1, p2);
        e.inverse_distance_m = e.inverse_distance_m.max((inverse.distance_m - c.s12).abs());
        let back = WGS84
            .geodesic_direct(p1, inverse.initial_azimuth_rad, inverse.distance_m)
            .expect("finite");
        let landing = WGS84.ecef_from_geodetic(back.end) - WGS84.ecef_from_geodetic(p2);
        e.inverse_landing_m = e.inverse_landing_m.max(landing.length());
        let da1 = azimuth_difference(inverse.initial_azimuth_rad, c.azi1.to_radians()).abs();
        let da2 = azimuth_difference(inverse.final_azimuth_rad, c.azi2.to_radians()).abs();
        let azimuth_miss = da1.max(da2) * c.m12.abs();
        e.inverse_azimuth_m = e.inverse_azimuth_m.max(azimuth_miss);
        if azimuth_miss > KARNEY_BOUND_M {
            let turn = input_turn_rad(c);
            assert!(
                da1.max(da2) <= turn,
                "line {}: inverse azimuth off by {:.3e} rad ({:.2} nm), more than a one-ulp \
                 move of an input turns it ({turn:.3e} rad)",
                c.line + 1,
                da1.max(da2),
                azimuth_miss * 1e9,
            );
            e.inverse_azimuth_ill_conditioned += 1;
        }

        let direct = WGS84
            .geodesic_direct(p1, c.azi1.to_radians(), c.s12)
            .expect("finite");
        let miss = WGS84.ecef_from_geodetic(direct.end) - WGS84.ecef_from_geodetic(p2);
        e.direct_position_m = e.direct_position_m.max(miss.length());
        let turn = heading_difference(
            direct.end,
            direct.final_azimuth_rad,
            p2,
            c.azi2.to_radians(),
        );
        e.direct_azimuth_m = e.direct_azimuth_m.max(turn * c.m12.abs());
    }
    errors
}

/// Every bound but the inverse's azimuths, which [`measure`] checks line by line.
fn assert_within_karney_bound(errors: &[Errors]) {
    for ((_, name), e) in CATEGORIES.iter().zip(errors) {
        for (what, value) in [
            ("inverse distance", e.inverse_distance_m),
            ("inverse landing", e.inverse_landing_m),
            ("direct position", e.direct_position_m),
            ("direct azimuth", e.direct_azimuth_m),
        ] {
            assert!(
                value <= KARNEY_BOUND_M,
                "{name}: {what} off by {:.2} nm, past Karney's 15 nm",
                value * 1e9
            );
        }
    }
}

/// The report: one row per kind, errors in nanometres.
fn table(errors: &[Errors]) -> String {
    let mut out = String::from(
        "| Kind | Lines | Inverse `s₁₂` | Inverse landing | Inverse azimuth × `m₁₂` | Over 15 nm, ill-conditioned | Direct position | Direct azimuth × `m₁₂` |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for ((_, name), e) in CATEGORIES.iter().zip(errors) {
        writeln!(
            out,
            "| {name} | {} | {:.2} | {:.2} | {:.2} | {} | {:.2} | {:.2} |",
            e.count,
            e.inverse_distance_m * 1e9,
            e.inverse_landing_m * 1e9,
            e.inverse_azimuth_m * 1e9,
            e.inverse_azimuth_ill_conditioned,
            e.direct_position_m * 1e9,
            e.direct_azimuth_m * 1e9,
        )
        .expect("writing to a string");
    }
    out
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every 500th line of the set, committed: CI's check.
#[test]
fn every_500th_line_is_within_karneys_bound() {
    let text = include_str!("fixtures/geodtest-every-500th.dat");
    let cases = parse(text, |k| k * 500);
    assert_eq!(cases.len(), 1_000);
    let errors = measure(&cases);
    assert_within_karney_bound(&errors);
    for ((first, name), e) in CATEGORIES.iter().zip(&errors) {
        let next = CATEGORIES
            .iter()
            .map(|c| c.0)
            .find(|&n| n > *first)
            .unwrap_or(500_000);
        assert_eq!(e.count, (next - first) / 500, "{name}");
    }
}

/// The whole set where fetched, against the committed report `validation/reports/geodesics.md`'s
/// table. `HPR_WRITE_GEODESICS=1` rewrites the table.
#[test]
fn the_whole_test_set_where_fetched() {
    let path = root().join("refs/sources/geodtest/GeodTest.dat");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("skipped: {} not fetched", path.display());
        return;
    };
    let cases = parse(&text, |k| k);
    assert_eq!(cases.len(), 500_000);
    let errors = measure(&cases);
    assert_within_karney_bound(&errors);

    let report_path = root().join("validation/reports/geodesics.md");
    let report = std::fs::read_to_string(&report_path).expect("the committed report");
    let (start, end) = (
        report.find(TABLE_START).expect("the table's start marker") + TABLE_START.len(),
        report.find(TABLE_END).expect("the table's end marker"),
    );
    let measured = table(&errors);
    if std::env::var_os("HPR_WRITE_GEODESICS").is_some() {
        let rewritten = format!("{}\n{measured}{}", &report[..start], &report[end..]);
        std::fs::write(&report_path, rewritten).expect("writing the report");
    } else {
        assert_eq!(
            report[start..end].trim(),
            measured.trim(),
            "rerun with HPR_WRITE_GEODESICS=1"
        );
    }
}

const TABLE_START: &str = "<!-- table: written by crates/hpr-core/tests/geodtest.rs -->";
const TABLE_END: &str = "<!-- end of table -->";
