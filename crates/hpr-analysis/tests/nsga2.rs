//! NSGA-II held to the known Pareto fronts of ZDT1, ZDT2 and ZDT3, and to pymoo's NSGA-II run on
//! the same problems with the same settings.
//!
//! The fixture `tests/fixtures/nsga2/pymoo.json` is written by
//! `validation/oracles/nsga2/pymoo_runs.py` (pymoo 0.6.2 with Deb et al.'s 2002 settings: 100
//! designs, 250 generations, SBX η = 20 with probability 0.9, polynomial mutation η = 20 at
//! 1/n, tournaments by rank and crowding, seeds 1 to 20). Different random numbers make the two
//! runs differ point by point, so they are compared by their fronts' distances from the true
//! front:
//!
//! - the generational distance (GD), the mean over a run's front of each design's distance to
//!   the true front, the curve itself ([`Zdt::distance_to_front`]): how close the front lies;
//! - the inverted generational distance (IGD), the mean over points of the true front of the
//!   distance to the nearest design: how close, and how evenly the front covers it. The points
//!   are at 1,000 evenly spaced `f₁`, those off ZDT3's five pieces left out (265 remain).
//!
//! The rules were set from pymoo's runs before hpr's were measured: every one of hpr's 20 runs
//! within twice pymoo's worst, and hpr's median no more than 25% above pymoo's (the bound
//! ADR-138 set on CMA-ES's evaluations against pycma's). Review then made the median rule
//! two-sided, within a factor of 1.25 either way (at most 20% below): on ZDT every variable but
//! the first is best at its low bound, so an operator biased toward the bounds passes a
//! one-sided rule.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]
#![allow(
    clippy::print_stderr,
    reason = "the medians and worst distances the guide quotes, shown with `--nocapture`"
)]

use std::collections::BTreeMap;

use hpr_analysis::optimize::Variable;
use hpr_analysis::optimize::benchmark::zdt::Zdt;
use hpr_analysis::optimize::nsga2::{
    Front, Nsga2, generational_distance, inverted_generational_distance,
};
use serde::Deserialize;

const N: usize = 30;
const SEEDS: std::ops::RangeInclusive<u64> = 1..=20;
const REFERENCE_POINTS: usize = 1000;

#[derive(Deserialize)]
struct Fixture {
    population: usize,
    generations: usize,
    seeds: Vec<u64>,
    problems: BTreeMap<String, Problem>,
}

#[derive(Deserialize)]
struct Problem {
    reference: Vec<Vec<f64>>,
    runs: Vec<PymooRun>,
}

#[derive(Deserialize)]
struct PymooRun {
    seed: u64,
    front: Vec<Vec<f64>>,
    gd: f64,
}

fn fixture() -> Fixture {
    let text = include_str!("fixtures/nsga2/pymoo.json");
    serde_json::from_str(text).expect("the committed fixture parses")
}

/// The settings pymoo ran with: 30 variables in `[0, 1]` and the paper's defaults.
fn nsga2() -> Nsga2 {
    let variables = (0..N)
        .map(|i| {
            Variable::new(format!("x{i}"), 0.5, 0.25)
                .unwrap()
                .within(0.0, 1.0)
                .unwrap()
        })
        .collect();
    Nsga2::new(variables, 2).unwrap()
}

fn goals(front: &Front) -> Vec<Vec<f64>> {
    front.members.iter().map(|m| m.objectives.clone()).collect()
}

/// The mean distance from a front's designs to the true front.
fn gd(problem: Zdt, front: &[Vec<f64>]) -> f64 {
    front
        .iter()
        .map(|f| problem.distance_to_front(f))
        .sum::<f64>()
        / front.len() as f64
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let n = values.len();
    0.5 * (values[(n - 1) / 2] + values[n / 2])
}

fn largest(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
}

/// Runs hpr from seeds 1 to 20 on `problem`, and holds its fronts to pymoo's.
fn check(name: &str, problem: Zdt) {
    let fixture = fixture();
    assert_eq!(fixture.population, 100);
    assert_eq!(fixture.generations, 250);
    assert_eq!(fixture.seeds, SEEDS.collect::<Vec<_>>());
    let pymoo = &fixture.problems[name];
    let reference = problem.reference(REFERENCE_POINTS);

    // The metric: pymoo's GD of its own fronts from its own 500 points, to rounding.
    for run in &pymoo.runs {
        let ours = generational_distance(&run.front, &pymoo.reference);
        assert!(
            (ours - run.gd).abs() <= 1e-12 * run.gd,
            "{name} seed {}: GD {ours} against pymoo's {}",
            run.seed,
            run.gd
        );
    }

    let pymoo_gd: Vec<f64> = pymoo.runs.iter().map(|r| gd(problem, &r.front)).collect();
    let pymoo_igd: Vec<f64> = pymoo
        .runs
        .iter()
        .map(|r| inverted_generational_distance(&r.front, &reference))
        .collect();
    let optimizer = nsga2();
    let (mut hpr_gd, mut hpr_igd) = (Vec::new(), Vec::new());
    for seed in SEEDS {
        let front = optimizer.minimize(seed, |x| problem.evaluate(x)).unwrap();
        assert_eq!(front.evaluations, 25_000);
        assert!(front.is_feasible());
        let points = goals(&front);
        hpr_gd.push(gd(problem, &points));
        hpr_igd.push(inverted_generational_distance(&points, &reference));
    }
    let report = |what: &str, hpr: &[f64], pymoo: &[f64]| {
        eprintln!(
            "{name} {what}: hpr median {:.5e}, largest {:.5e}; pymoo median {:.5e}, largest {:.5e}",
            median(hpr.to_vec()),
            largest(hpr),
            median(pymoo.to_vec()),
            largest(pymoo)
        );
    };
    report("GD", &hpr_gd, &pymoo_gd);
    report("IGD", &hpr_igd, &pymoo_igd);
    for (what, hpr, pymoo) in [("GD", &hpr_gd, &pymoo_gd), ("IGD", &hpr_igd, &pymoo_igd)] {
        let bound = 2.0 * largest(pymoo);
        for (seed, value) in SEEDS.zip(hpr.iter()) {
            assert!(
                *value <= bound,
                "{name} seed {seed}: {what} {value:.3e} above twice pymoo's worst, {bound:.3e}"
            );
        }
        // Two-sided: a front much closer than pymoo's is as suspect as one much further, as on
        // these problems every variable but the first is best at its low bound, and an operator
        // biased toward the bounds would look better than it is.
        let (ours, theirs) = (median(hpr.clone()), median(pymoo.clone()));
        assert!(
            ours <= 1.25 * theirs && ours >= theirs / 1.25,
            "{name}: median {what} {ours:.3e} not within a factor of 1.25 of pymoo's {theirs:.3e}"
        );
    }
}

#[test]
fn zdt1_front_as_close_as_pymoos() {
    check("zdt1", Zdt::One);
}

#[test]
fn zdt2_front_as_close_as_pymoos() {
    check("zdt2", Zdt::Two);
}

#[test]
fn zdt3_front_as_close_as_pymoos() {
    check("zdt3", Zdt::Three);
}

/// A run is bit for bit the same from the same seed, and differs from another seed.
#[test]
fn a_seed_gives_the_same_front() {
    let optimizer = nsga2().with_generations(20).unwrap();
    let a = optimizer.minimize(7, |x| Zdt::Three.evaluate(x)).unwrap();
    let b = optimizer.minimize(7, |x| Zdt::Three.evaluate(x)).unwrap();
    assert_eq!(a, b);
    let c = optimizer.minimize(8, |x| Zdt::Three.evaluate(x)).unwrap();
    assert_ne!(a, c);
}
