//! Geodesics against Karney's test set (M5.3c1).
//!
//! C. F. F. Karney, *Test set for geodesics*, doi:10.5281/zenodo.32156 (CC0): 500,000 WGS 84
//! geodesics, each line `φ₁ λ₁ α₁ φ₂ λ₂ α₂ s₁₂ σ₁₂ m₁₂ S₁₂` (degrees, metres), computed in high
//! precision from the exact `φ₁`, `α₁` and `s₁₂`. The lines come in nine runs of kinds; see
//! [`CATEGORIES`]. Committed for CI: every 500th line, `tests/fixtures/geodtest-every-500th.dat`,
//! and the 21 lines with two equally short geodesics, `tests/fixtures/geodtest-mirror-lines.dat`.
//! The whole file is read from `refs/sources/geodtest/GeodTest.dat` where `cargo xtask refs fetch`
//! has put it.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests read the fetched set and write the report; not the pure core"
)]
#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
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

/// The cases in `text`, `#` lines skipped. A line of 11 fields starts with its line number in the
/// whole set, counted from 1; otherwise `line_of` numbers the `k`th case from 0.
fn parse(text: &str, line_of: impl Fn(usize) -> usize) -> Vec<Case> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .enumerate()
        .map(|(k, l)| {
            let mut v: Vec<f64> = l
                .split_whitespace()
                .map(|x| x.parse().expect("a number"))
                .collect();
            let line = match v.len() {
                10 => line_of(k),
                11 => v.remove(0) as usize - 1,
                n => panic!("{n} fields: {l}"),
            };
            Case {
                line,
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
    u.cross(v).length().atan2(u.dot(v))
}

/// Karney's bound on round-off in both problems on WGS 84 (Karney 2013 §7, page 10), m.
const KARNEY_BOUND_M: f64 = 15e-9;

/// A kind whose largest `|m₁₂|` is below this has no inverse azimuth measure: an azimuth error
/// times `m₁₂` reads zero whatever the error, m.
const M12_MEASURABLE_M: f64 = 1e-3;

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
    /// other end. On a mirror line (see [`is_mirror`]), the smaller of the errors against the
    /// set's pair and the swapped pair.
    inverse_azimuth_m: f64,
    /// The largest `|m₁₂|` in the kind.
    largest_m12_m: f64,
    /// Mirror lines, and on how many the inverse's azimuths are nearer the swapped pair.
    mirror_lines: usize,
    mirror_swapped: usize,
    /// Direct: the end point's distance from the reference's.
    direct_position_m: f64,
    /// Direct: the angle between the computed and reference headings at the end, as directions
    /// in space, times the equatorial radius `a`.
    direct_heading_m: f64,
}

/// A line whose ends read as `f64` with `φ₂ = −φ₁` exactly and `α₁ ≠ α₂`: two geodesics of the
/// same length join them, the second with `α₁` and `α₂` swapped (GeographicLib's `GeodSolve`
/// manual, *Multiple solutions*), and the set holds one of them.
fn is_mirror(c: &Case) -> bool {
    c.lat2 == -c.lat1 && c.azi1 != c.azi2
}

/// Every per-line error must be a number: `f64::max` would drop a NaN silently.
fn finite(value: f64, what: &str, c: &Case) -> f64 {
    assert!(value.is_finite(), "line {}: {what} is {value}", c.line + 1);
    value
}

fn measure(cases: &[Case]) -> Vec<Errors> {
    let a = WGS84.semi_major_axis_m();
    let mut errors = vec![Errors::default(); CATEGORIES.len()];
    for c in cases {
        assert_eq!(
            c.lon1,
            0.0,
            "line {}: the set starts every geodesic at λ₁ = 0",
            c.line + 1
        );
        let p1 = Geodetic::from_degrees(c.lat1, c.lon1, 0.0).expect("a point");
        let p2 = Geodetic::from_degrees(c.lat2, c.lon2, 0.0).expect("a point");
        let e = &mut errors[category(c.line)];
        e.count += 1;
        e.largest_m12_m = e.largest_m12_m.max(c.m12.abs());

        let inverse = WGS84.geodesic_inverse(p1, p2).expect("a geodesic");
        let distance = finite(
            (inverse.distance_m - c.s12).abs(),
            "the inverse's distance",
            c,
        );
        e.inverse_distance_m = e.inverse_distance_m.max(distance);
        let back = WGS84
            .geodesic_direct(p1, inverse.initial_azimuth_rad, inverse.distance_m)
            .expect("finite")
            .end(0.0)
            .expect("a point");
        let landing = (WGS84.ecef_from_geodetic(back) - WGS84.ecef_from_geodetic(p2)).length();
        e.inverse_landing_m = e.inverse_landing_m.max(finite(landing, "the landing", c));
        let pair_error = |azi1: f64, azi2: f64| {
            let da1 = azimuth_difference(inverse.initial_azimuth_rad, azi1.to_radians()).abs();
            let da2 = azimuth_difference(inverse.final_azimuth_rad, azi2.to_radians()).abs();
            let da1 = finite(da1, "the inverse's first azimuth", c);
            let da2 = finite(da2, "the inverse's second azimuth", c);
            da1.max(da2) * c.m12.abs()
        };
        let mut azimuth_miss = pair_error(c.azi1, c.azi2);
        if is_mirror(c) {
            e.mirror_lines += 1;
            let swapped = pair_error(c.azi2, c.azi1);
            if swapped < azimuth_miss {
                e.mirror_swapped += 1;
                azimuth_miss = swapped;
            }
        }
        e.inverse_azimuth_m = e.inverse_azimuth_m.max(azimuth_miss);

        let direct = WGS84
            .geodesic_direct(p1, c.azi1.to_radians(), c.s12)
            .expect("finite");
        let end = direct.end(0.0).expect("a point");
        let miss = (WGS84.ecef_from_geodetic(end) - WGS84.ecef_from_geodetic(p2)).length();
        e.direct_position_m = e.direct_position_m.max(finite(miss, "the direct's end", c));
        let turn = heading_difference(end, direct.final_azimuth_rad, p2, c.azi2.to_radians());
        e.direct_heading_m = e
            .direct_heading_m
            .max(finite(turn * a, "the direct's heading", c));
    }
    errors
}

/// Every measure, every kind, within Karney's 15 nm.
fn assert_within_karney_bound(errors: &[Errors]) {
    for ((_, name), e) in CATEGORIES.iter().zip(errors) {
        for (what, value) in [
            ("inverse distance", e.inverse_distance_m),
            ("inverse landing", e.inverse_landing_m),
            ("inverse azimuth", e.inverse_azimuth_m),
            ("direct position", e.direct_position_m),
            ("direct heading", e.direct_heading_m),
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
        "| Kind | Lines | Inverse `s₁₂` | Inverse landing | Inverse azimuth × `m₁₂` | Mirror lines (swapped) | Direct position | Direct heading × `a` |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for ((_, name), e) in CATEGORIES.iter().zip(errors) {
        let azimuth = if e.largest_m12_m < M12_MEASURABLE_M {
            format!("not measured (`m₁₂` ≤ {:.0e} m)", e.largest_m12_m)
        } else {
            format!("{:.2}", e.inverse_azimuth_m * 1e9)
        };
        writeln!(
            out,
            "| {name} | {} | {:.2} | {:.2} | {azimuth} | {} ({}) | {:.2} | {:.2} |",
            e.count,
            e.inverse_distance_m * 1e9,
            e.inverse_landing_m * 1e9,
            e.mirror_lines,
            e.mirror_swapped,
            e.direct_position_m * 1e9,
            e.direct_heading_m * 1e9,
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

/// The 21 mirror lines, committed: either pair of azimuths, 4 nearer the swapped pair.
#[test]
fn the_mirror_lines_are_within_karneys_bound() {
    let cases = parse(include_str!("fixtures/geodtest-mirror-lines.dat"), |_| {
        unreachable!("every mirror line carries its number")
    });
    assert_eq!(cases.len(), 21);
    assert!(cases.iter().all(is_mirror));
    let errors = measure(&cases);
    assert_within_karney_bound(&errors);
    let swapped: usize = errors.iter().map(|e| e.mirror_swapped).sum();
    assert_eq!(swapped, 4);
}

/// The build that wrote the committed table. The cells are a few ulps of Earth-centred
/// coordinates and rest on the platform's `sin` and `cos`: a release build on macOS moves three by
/// up to 1.83 nm, and other platforms are unmeasured, so there the test holds only the 15 nm bound.
const REPORT_BUILD: &str = "a debug build on macos aarch64";

fn this_build() -> String {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    format!(
        "a {profile} build on {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// The whole set where fetched: every line within the bound, and on [`REPORT_BUILD`] the table
/// in `validation/reports/geodesics.md` as committed. `HPR_WRITE_GEODESICS=1` rewrites the table.
#[test]
fn the_whole_test_set_where_fetched() {
    let path = root().join("refs/sources/geodtest/GeodTest.dat");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("skipped: {} not fetched", path.display());
        return;
    };
    let cases = parse(&text, |k| k);
    assert_eq!(cases.len(), 500_000);
    assert_eq!(cases.iter().filter(|c| is_mirror(c)).count(), 21);
    let errors = measure(&cases);
    assert_within_karney_bound(&errors);

    let report_path = root().join("validation/reports/geodesics.md");
    let report = std::fs::read_to_string(&report_path).expect("the committed report");
    let start = report.find(TABLE_START).expect("the table's start marker") + TABLE_START.len();
    let end = start
        + report[start..]
            .find(TABLE_END)
            .expect("the table's end marker");
    let measured = table(&errors);
    if std::env::var_os("HPR_WRITE_GEODESICS").is_some() {
        assert_eq!(
            this_build(),
            REPORT_BUILD,
            "write the table from {REPORT_BUILD}"
        );
        let rewritten = format!("{}\n{measured}{}", &report[..start], &report[end..]);
        std::fs::write(&report_path, rewritten).expect("writing the report");
    } else if this_build() == REPORT_BUILD {
        assert_eq!(
            report[start..end].trim(),
            measured.trim(),
            "rerun with HPR_WRITE_GEODESICS=1"
        );
    } else {
        eprintln!(
            "{} is not {REPORT_BUILD}: the table is not compared\n{measured}",
            this_build()
        );
    }
}

const TABLE_START: &str = "<!-- table: written by crates/hpr-core/tests/geodtest.rs -->";
const TABLE_END: &str = "<!-- end of table -->";
