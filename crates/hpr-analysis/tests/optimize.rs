//! CMA-ES held to the known minima of four test functions, and to pycma, the reference
//! implementation by the method's author, run on the same functions from the same starts.
//!
//! The fixture `tests/fixtures/cmaes/pycma.json` is written by
//! `validation/oracles/cmaes/pycma_runs.py` (pycma 4.5.0, its active weights off, every stop but
//! the target and the evaluation cap off, seeds 1 to 20). Different random numbers make the two
//! runs differ point by point, so they are compared by their evaluation counts' medians.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::collections::BTreeMap;

use hpr_analysis::optimize::Variable;
use hpr_analysis::optimize::benchmark::{ellipsoid, rosenbrock, rotated_ellipsoid, sphere};
use hpr_analysis::optimize::cmaes::{Cmaes, Optimum, Parameters, Stop};
use serde::Deserialize;

const N: usize = 10;
const TARGET: f64 = 1e-10;
const MAX_EVALUATIONS: usize = 100_000;
const SEEDS: std::ops::RangeInclusive<u64> = 1..=20;

#[derive(Deserialize)]
struct Fixture {
    dimension: usize,
    sigma0: f64,
    ftarget: f64,
    max_evaluations: usize,
    parameters: PycmaParameters,
    runs: BTreeMap<String, Vec<PycmaRun>>,
}

#[derive(Deserialize)]
struct PycmaParameters {
    weights: Vec<f64>,
    mueff: f64,
    c1: f64,
    cmu: f64,
    cc: f64,
    cs: f64,
    damps: f64,
    chi_n: f64,
}

#[derive(Deserialize)]
struct PycmaRun {
    evaluations: usize,
    reached: bool,
}

fn fixture() -> Fixture {
    let text = include_str!("fixtures/cmaes/pycma.json");
    serde_json::from_str(text).expect("the committed fixture parses")
}

/// The settings pycma ran with: ten variables starting at `start`, steps of 0.5, a target of
/// 10⁻¹⁰, 100,000 evaluations, and no other stop.
fn cmaes(start: f64) -> Cmaes {
    let variables = (0..N)
        .map(|i| Variable::new(format!("x{i}"), start, 0.5).unwrap())
        .collect();
    Cmaes::new(variables)
        .unwrap()
        .with_target(TARGET)
        .unwrap()
        .with_max_evaluations(MAX_EVALUATIONS)
        .unwrap()
        .with_tolerances(0.0, 0.0)
        .unwrap()
}

fn runs(start: f64, f: fn(&[f64]) -> f64) -> Vec<Optimum> {
    let cmaes = cmaes(start);
    SEEDS.map(|seed| cmaes.minimize(seed, f).unwrap()).collect()
}

fn median(mut counts: Vec<usize>) -> f64 {
    counts.sort_unstable();
    let n = counts.len();
    if n % 2 == 1 {
        counts[n / 2] as f64
    } else {
        (counts[n / 2 - 1] + counts[n / 2]) as f64 / 2.0
    }
}

fn distance(x: &[f64], to: f64) -> f64 {
    x.iter().map(|xi| (xi - to).powi(2)).sum::<f64>().sqrt()
}

/// Our median evaluations and pycma's.
fn against_pycma(name: &str, ours: &[Optimum]) -> (f64, f64) {
    let fixture = fixture();
    let theirs = median(
        fixture.runs[name]
            .iter()
            .map(|run| run.evaluations)
            .collect(),
    );
    let ours = median(ours.iter().map(|o| o.evaluations).collect());
    (ours, theirs)
}

/// Checks that our median evaluations are within 25% of pycma's.
fn assert_near_pycma(name: &str, ours: &[Optimum]) {
    let (ours, theirs) = against_pycma(name, ours);
    let ratio = ours / theirs;
    assert!(
        (0.75..=1.25).contains(&ratio),
        "{name}: median evaluations {ours}, pycma's {theirs}"
    );
}

#[test]
fn fixture_matches_these_settings() {
    let fixture = fixture();
    assert_eq!(fixture.dimension, N);
    assert_eq!(fixture.sigma0, 0.5);
    assert_eq!(fixture.ftarget, TARGET);
    assert_eq!(fixture.max_evaluations, MAX_EVALUATIONS);
    for (name, runs) in &fixture.runs {
        assert_eq!(runs.len(), 20, "{name}");
    }
}

/// Table 1's weights, `μ_eff`, `c₁`, `c_μ` and `c_c` are pycma's to rounding. pycma's `c_σ`
/// (`(μ_eff + 2)/(n + μ_eff + 3)`), and so its `d_σ`, differ from Table 1's by design, and it
/// takes `E‖N(0, I)‖` exactly where the tutorial approximates it.
#[test]
fn parameters_agree_with_pycma_where_both_follow_table_1() {
    let ours = Parameters::new(N, 10);
    let theirs = fixture().parameters;
    let close = |a: f64, b: f64| (a / b - 1.0).abs() <= 1e-15;
    assert_eq!(ours.weights.len(), theirs.weights.len());
    for (a, b) in ours.weights.iter().zip(&theirs.weights) {
        assert!(close(*a, *b), "weight {a} vs {b}");
    }
    assert!(close(ours.mu_eff, theirs.mueff), "{}", ours.mu_eff);
    assert!(close(ours.c_1, theirs.c1), "{}", ours.c_1);
    assert!(close(ours.c_mu, theirs.cmu), "{}", ours.c_mu);
    assert!(close(ours.c_c, theirs.cc), "{}", ours.c_c);
    // The approximation of E‖N(0, I)‖ is good to about 10⁻⁴ at n = 10.
    assert!((ours.chi_n / theirs.chi_n - 1.0).abs() < 2e-4);
    // Table 1's c_σ = (μ_eff + 2)/(n + μ_eff + 5) is smaller than pycma's.
    assert!(ours.c_sigma < theirs.cs && ours.d_sigma < theirs.damps);
}

#[test]
fn sphere_reaches_its_minimum_from_every_seed() {
    let runs = runs(1.0, sphere);
    for (seed, o) in SEEDS.zip(&runs) {
        assert_eq!(o.stop, Stop::Target, "seed {seed}");
        assert!(o.value <= TARGET);
        // f ≤ 10⁻¹⁰ puts x within 10⁻⁵ of 0.
        assert!(distance(&o.point, 0.0) <= 1e-5, "seed {seed}");
    }
    assert_near_pycma("sphere", &runs);
}

#[test]
fn ellipsoid_and_its_rotation_reach_their_minimum_alike() {
    let plain = runs(1.0, ellipsoid);
    let turned = runs(1.0, rotated_ellipsoid);
    for (seed, (a, b)) in SEEDS.zip(plain.iter().zip(&turned)) {
        assert_eq!(a.stop, Stop::Target, "seed {seed}");
        assert_eq!(b.stop, Stop::Target, "seed {seed}");
        // The smallest curvature is 1, so f ≤ 10⁻¹⁰ puts x within 10⁻⁵ of 0.
        assert!(distance(&a.point, 0.0) <= 1e-5, "seed {seed}");
        assert!(distance(&b.point, 0.0) <= 1e-5, "seed {seed}");
    }
    assert_near_pycma("ellipsoid", &plain);
    assert_near_pycma("rotated_ellipsoid", &turned);
    // Invariance under a rotation of the variables: about as many evaluations either way.
    let plain = median(plain.iter().map(|o| o.evaluations).collect());
    let turned = median(turned.iter().map(|o| o.evaluations).collect());
    assert!((turned / plain - 1.0).abs() <= 0.1, "{plain} vs {turned}");
}

#[test]
fn rosenbrock_reaches_its_global_minimum_from_most_seeds() {
    let runs = runs(0.0, rosenbrock);
    let mut global = 0;
    for (seed, o) in SEEDS.zip(&runs) {
        if o.stop == Stop::Target {
            global += 1;
            assert!(
                distance(&o.point, 1.0) <= 1e-4,
                "seed {seed}: {:?}",
                o.point
            );
        } else {
            // The local minimum near (−1, 1, …, 1), where f ≈ 3.987: the run converges there
            // until the evaluations run out or the covariance's condition passes 10¹⁴ (a stop
            // pycma's runs had turned off).
            assert!(
                matches!(o.stop, Stop::Evaluations | Stop::Condition),
                "seed {seed}: {:?}",
                o.stop
            );
            assert!(
                (o.point[0] + 1.0).abs() < 0.05,
                "seed {seed}: {:?}",
                o.point
            );
            assert!((3.9..4.0).contains(&o.value), "seed {seed}: {}", o.value);
        }
    }
    let pycma = fixture().runs["rosenbrock"]
        .iter()
        .filter(|r| r.reached)
        .count();
    assert!(
        global >= 17,
        "global minimum from {global} of 20 seeds, pycma's from {pycma}"
    );
    assert_near_pycma("rosenbrock", &runs);
}
