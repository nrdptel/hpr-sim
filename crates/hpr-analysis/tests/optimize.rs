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
use hpr_analysis::optimize::cmaes::{Cmaes, Optimum, Stop};
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
        .with_tolerance_x(0.0)
        .unwrap()
        .with_tolerance_value(0.0)
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
    let ours = cmaes(1.0).start(1).unwrap().parameters().clone();
    assert_eq!(ours.weights.len(), 5, "λ = 10");
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

/// `a b` for `n × n` row-major matrices.
fn multiply(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            c[i * n + j] = (0..n).map(|k| a[i * n + k] * b[k * n + j]).sum();
        }
    }
    c
}

/// The inverse of an `n × n` row-major matrix, by Gauss–Jordan elimination with partial pivoting.
fn inverse(a: &[f64], n: usize) -> Vec<f64> {
    let mut m = a.to_vec();
    let mut inv = vec![0.0; n * n];
    for i in 0..n {
        inv[i * n + i] = 1.0;
    }
    for col in 0..n {
        let pivot = (col..n)
            .max_by(|&i, &j| m[i * n + col].abs().total_cmp(&m[j * n + col].abs()))
            .unwrap();
        for k in 0..n {
            m.swap(col * n + k, pivot * n + k);
            inv.swap(col * n + k, pivot * n + k);
        }
        let p = m[col * n + col];
        for k in 0..n {
            m[col * n + k] /= p;
            inv[col * n + k] /= p;
        }
        for row in (0..n).filter(|&r| r != col) {
            let f = m[row * n + col];
            for k in 0..n {
                m[row * n + k] -= f * m[col * n + k];
                inv[row * n + k] -= f * inv[col * n + k];
            }
        }
    }
    inv
}

/// `A^(−1/2)` of a symmetric positive definite matrix by the Denman–Beavers iteration
/// (`Y₀ = A`, `Z₀ = I`, `Y ← (Y + Z⁻¹)/2`, `Z ← (Z + Y⁻¹)/2`; `Z → A^(−1/2)`): an algorithm
/// independent of the Jacobi eigen-decomposition the optimizer uses.
fn inverse_square_root(a: &[f64], n: usize) -> Vec<f64> {
    let mut y = a.to_vec();
    let mut z = vec![0.0; n * n];
    for i in 0..n {
        z[i * n + i] = 1.0;
    }
    for _ in 0..60 {
        let y_inv = inverse(&y, n);
        let z_inv = inverse(&z, n);
        y = y.iter().zip(&z_inv).map(|(a, b)| (a + b) / 2.0).collect();
        z = z.iter().zip(&y_inv).map(|(a, b)| (a + b) / 2.0).collect();
    }
    // Check: Z A Z = I.
    let check = multiply(&multiply(&z, a, n), &z, n);
    for i in 0..n {
        for j in 0..n {
            let expected = if i == j { 1.0 } else { 0.0 };
            assert!((check[i * n + j] - expected).abs() < 1e-12, "{check:?}");
        }
    }
    z
}

/// Every generation of a run, recomputed from its candidates by a separate dense implementation
/// of the tutorial's Figure 6 and Table 1, in the variables divided by their steps: the mean,
/// `σ` and `C` agree to rounding. The steps differ by a factor of 200, and the model is linear,
/// so the step-size path grows until `h_σ` switches the rank-one update's path off; the test
/// checks both branches are taken.
#[test]
fn every_generation_follows_the_tutorials_update() {
    let n = 3;
    let steps = [0.5, 2.0, 0.01];
    let starts = [1.0, -3.0, 0.02];
    let variables = (0..n)
        .map(|i| Variable::new(format!("x{i}"), starts[i], steps[i]).unwrap())
        .collect();
    let cmaes = Cmaes::new(variables).unwrap();
    let lambda = cmaes.population();
    let model = |x: &[f64]| x[0] / 0.5 + x[1] / 2.0 + x[2] / 0.01;
    let mut run = cmaes.start(2026).unwrap();

    // Table 1, written out again.
    let nf = n as f64;
    let mu = lambda / 2;
    let raw: Vec<f64> = (1..=mu)
        .map(|i| ((lambda as f64 + 1.0) / 2.0).ln() - (i as f64).ln())
        .collect();
    let total: f64 = raw.iter().sum();
    let w: Vec<f64> = raw.iter().map(|r| r / total).collect();
    let mu_eff = total * total / raw.iter().map(|r| r * r).sum::<f64>();
    let cs = (mu_eff + 2.0) / (nf + mu_eff + 5.0);
    let ds = 1.0 + 2.0 * (((mu_eff - 1.0) / (nf + 1.0)).sqrt() - 1.0).max(0.0) + cs;
    let cc = (4.0 + mu_eff / nf) / (nf + 4.0 + 2.0 * mu_eff / nf);
    let c1 = 2.0 / ((nf + 1.3).powi(2) + mu_eff);
    let cmu =
        (1.0 - c1).min(2.0 * (0.25 + mu_eff + 1.0 / mu_eff - 2.0) / ((nf + 2.0).powi(2) + mu_eff));
    let chi = nf.sqrt() * (1.0 - 1.0 / (4.0 * nf) + 1.0 / (21.0 * nf * nf));

    let mut m = starts.to_vec();
    let mut sigma = 1.0;
    let mut c = vec![0.0; n * n];
    for i in 0..n {
        c[i * n + i] = 1.0;
    }
    let mut ps = vec![0.0; n];
    let mut pc = vec![0.0; n];
    let mut branches = [false, false];
    for g in 0..12 {
        let x = run.candidates().to_vec();
        let f: Vec<f64> = x.iter().map(|xi| model(xi)).collect();
        let mut order: Vec<usize> = (0..lambda).collect();
        order.sort_by(|&a, &b| f[a].total_cmp(&f[b]));
        // Each candidate's step in the scaled variables.
        let y: Vec<Vec<f64>> = x
            .iter()
            .map(|xi| {
                (0..n)
                    .map(|j| (xi[j] - m[j]) / (sigma * steps[j]))
                    .collect()
            })
            .collect();
        let mut yw = vec![0.0; n];
        for (wi, &k) in w.iter().zip(&order) {
            for j in 0..n {
                yw[j] += wi * y[k][j];
            }
        }
        for j in 0..n {
            m[j] += sigma * steps[j] * yw[j];
        }
        let c_inv_sqrt = inverse_square_root(&c, n);
        for i in 0..n {
            let whitened: f64 = (0..n).map(|j| c_inv_sqrt[i * n + j] * yw[j]).sum();
            ps[i] = (1.0 - cs) * ps[i] + (cs * (2.0 - cs) * mu_eff).sqrt() * whitened;
        }
        let norm = ps.iter().map(|v| v * v).sum::<f64>().sqrt();
        let h = norm / (1.0 - (1.0 - cs).powi(2 * (g + 1))).sqrt() < (1.4 + 2.0 / (nf + 1.0)) * chi;
        branches[usize::from(h)] = true;
        let hf = if h { 1.0 } else { 0.0 };
        for i in 0..n {
            pc[i] = (1.0 - cc) * pc[i] + hf * (cc * (2.0 - cc) * mu_eff).sqrt() * yw[i];
        }
        let delta = (1.0 - hf) * cc * (2.0 - cc);
        let mut next = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                let rank_mu: f64 = w
                    .iter()
                    .zip(&order)
                    .map(|(wi, &k)| wi * y[k][i] * y[k][j])
                    .sum();
                next[i * n + j] = (1.0 + c1 * delta - c1 - cmu) * c[i * n + j]
                    + c1 * pc[i] * pc[j]
                    + cmu * rank_mu;
            }
        }
        c = next;
        sigma *= ((cs / ds) * (norm / chi - 1.0)).exp();

        assert!(run.tell(&f).unwrap().is_none(), "generation {g}");
        for j in 0..n {
            let scale = m[j].abs().max(sigma * steps[j]);
            assert!(
                (run.mean()[j] - m[j]).abs() <= 1e-12 * scale,
                "generation {g}, mean {j}: {} vs {}",
                run.mean()[j],
                m[j]
            );
        }
        assert!(
            (run.sigma() / sigma - 1.0).abs() <= 1e-12,
            "generation {g}: σ"
        );
        let largest = c.iter().fold(0.0f64, |a, b| a.max(b.abs()));
        for (k, (a, b)) in run.covariance().iter().zip(&c).enumerate() {
            assert!(
                (a - b).abs() <= 1e-12 * largest,
                "generation {g}, C[{k}]: {a} vs {b}"
            );
        }
    }
    assert_eq!(branches, [true, true], "h_σ = 0 and h_σ = 1 both taken");
}
