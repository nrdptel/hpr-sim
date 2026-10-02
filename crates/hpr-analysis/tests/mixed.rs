//! CMA-ES with integer variables held to the known minima of three mixed-integer test functions,
//! and to `cmaes.CMAwM`, an outside implementation of CMA-ES with margin, run on the same
//! functions from the same starts.
//!
//! The fixture `tests/fixtures/cmawm/cmawm.json` is written by
//! `validation/oracles/cmawm/cmawm_runs.py` (cmaes 0.13.1, MIT, adapted from the method's authors'
//! code). Its "positive" runs have the active weights off, as ours are. Different random numbers
//! make the two runs differ point by point, so they are compared by their evaluation counts'
//! medians, as `tests/optimize.rs` compares the continuous runs with pycma's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::collections::BTreeMap;

use hpr_analysis::optimize::Variable;
use hpr_analysis::optimize::benchmark::mixed::{ellipsoid_int, sphere_int, sphere_one_max};
use hpr_analysis::optimize::cmaes::{Cmaes, Optimum, Stop};
use serde::Deserialize;

const TARGET: f64 = 1e-10;

#[derive(Deserialize)]
struct Fixture {
    settings: Settings,
    cases: BTreeMap<String, Case>,
}

#[derive(Deserialize)]
struct Settings {
    ftarget: f64,
    sigma0: f64,
    evaluations_per_dimension: usize,
    integer_range: [f64; 2],
}

#[derive(Deserialize)]
struct Case {
    function: String,
    dimension: usize,
    continuous: usize,
    discrete: String,
    variants: BTreeMap<String, Variant>,
}

#[derive(Deserialize)]
struct Variant {
    runs: Vec<Run>,
    summary: Summary,
}

#[derive(Deserialize)]
struct Run {
    seed: u64,
    initial_mean: Vec<f64>,
    success: bool,
}

#[derive(Deserialize)]
struct Summary {
    median: f64,
    successes: usize,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("fixtures/cmawm/cmawm.json")).unwrap()
}

/// The median of whole-number counts.
fn median(mut counts: Vec<usize>) -> f64 {
    counts.sort_unstable();
    let n = counts.len();
    // Casts: counts are far below 2⁵³.
    if n % 2 == 1 {
        counts[n / 2] as f64
    } else {
        (counts[n / 2 - 1] + counts[n / 2]) as f64 / 2.0
    }
}

/// Runs one case from each of the oracle's starts, with the oracle's settings: steps of `σ₀`,
/// the integer variables within the fixture's range (or 0 and 1), the continuous ones unbounded.
fn run_case(settings: &Settings, case: &Case) -> Vec<Optimum> {
    let f: fn(&[f64]) -> f64 = match case.function.as_str() {
        "sphere_int" => sphere_int,
        "ellipsoid_int" => ellipsoid_int,
        "sphere_one_max" => sphere_one_max,
        other => panic!("unknown function {other}"),
    };
    let [low, high] = match case.discrete.as_str() {
        "integer" => settings.integer_range,
        "binary" => [0.0, 1.0],
        other => panic!("unknown discrete kind {other}"),
    };
    let start = &case.variants["positive"].runs;
    start
        .iter()
        .map(|run| {
            let variables = run
                .initial_mean
                .iter()
                .enumerate()
                .map(|(i, &x)| {
                    let v = Variable::new(format!("x{i}"), x, settings.sigma0).unwrap();
                    if i < case.continuous {
                        v
                    } else {
                        v.within(low, high).unwrap().integer().unwrap()
                    }
                })
                .collect();
            Cmaes::new(variables)
                .unwrap()
                .with_target(settings.ftarget)
                .unwrap()
                .with_max_evaluations(case.dimension * settings.evaluations_per_dimension)
                .unwrap()
                .minimize(run.seed, f)
                .unwrap()
        })
        .collect()
}

/// Every run of a case reaches `f ≤ 10⁻¹⁰`, at the known minimum: each integer variable exactly
/// at its best value (0, or 1 for the binaries of SphereOneMax), each continuous one within
/// 10⁻⁵ of 0 (the functions' smallest coefficient is 1, so `f ≤ 10⁻¹⁰` puts it there). Their
/// median evaluations are within 25% of the outside implementation's, the bound
/// `tests/optimize.rs` holds the continuous runs to; the oracle reached the target in all 20 runs
/// of each case too.
fn check(name: &str) {
    let fixture = fixture();
    assert_eq!(fixture.settings.ftarget, TARGET);
    assert_eq!(fixture.cases.len(), 6, "a case each below");
    let case = &fixture.cases[name];
    let reference = &case.variants["positive"];
    assert_eq!(reference.runs.len(), 20);
    assert!(reference.runs.iter().all(|r| r.success));
    assert_eq!(reference.summary.successes, 20);
    let best_integer = if case.discrete == "binary" { 1.0 } else { 0.0 };
    let runs = run_case(&fixture.settings, case);
    for (run, optimum) in reference.runs.iter().zip(&runs) {
        assert_eq!(optimum.stop, Stop::Target, "seed {}", run.seed);
        assert!(optimum.value <= TARGET, "seed {}", run.seed);
        for (i, x) in optimum.point.iter().enumerate() {
            if i < case.continuous {
                assert!(x.abs() <= 1e-5, "seed {}: x{i} = {x}", run.seed);
            } else {
                assert_eq!(*x, best_integer, "seed {}: x{i}", run.seed);
            }
        }
    }
    let ours = median(runs.iter().map(|o| o.evaluations).collect());
    let theirs = reference.summary.median;
    // `cargo test --test mixed -- --nocapture` shows the comparison the guide quotes.
    println!("{name}: median evaluations {ours}, cmaes.CMAwM's {theirs}");
    assert!(
        (ours - theirs).abs() <= 0.25 * theirs,
        "{name}: median evaluations {ours}, cmaes.CMAwM's {theirs}"
    );
}

#[test]
fn sphere_int_10() {
    check("sphere_int_10");
}

#[test]
fn sphere_int_20() {
    check("sphere_int_20");
}

#[test]
fn ellipsoid_int_10() {
    check("ellipsoid_int_10");
}

#[test]
fn ellipsoid_int_20() {
    check("ellipsoid_int_20");
}

#[test]
fn sphere_one_max_10() {
    check("sphere_one_max_10");
}

#[test]
fn sphere_one_max_20() {
    check("sphere_one_max_20");
}
