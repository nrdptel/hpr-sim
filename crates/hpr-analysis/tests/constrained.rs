//! CMA-ES under constraints, ranked by Deb's feasibility rules, held to three problems whose
//! minima lie on their constraints' edges and are known in closed form: the sphere with `x₀ ≥ 1`
//! and the tangent problem (`Σ xᵢ ≥ n`), both in 10 variables, and CEC 2006's g06, whose minimum
//! is where two circles cross.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use hpr_analysis::optimize::benchmark::constrained::{G06_X0, g06, g06_x1, sphere_above, tangent};
use hpr_analysis::optimize::cmaes::{Cmaes, Optimum};
use hpr_analysis::optimize::{Evaluation, Variable};

const N: usize = 10;
/// Every run's best value is held to the minimum to this, relative.
const TOLERANCE: f64 = 1e-10;
const SEEDS: std::ops::RangeInclusive<u64> = 1..=20;

fn runs(variables: Vec<Variable>, f: fn(&[f64]) -> Evaluation) -> Vec<Optimum> {
    let cmaes = Cmaes::new(variables)
        .unwrap()
        .with_max_evaluations(100_000)
        .unwrap();
    SEEDS
        .map(|seed| cmaes.minimize_constrained(seed, f).unwrap())
        .collect()
}

/// Ten unbounded variables starting at 3, where both 10-variable problems' constraints are kept.
fn ten() -> Vec<Variable> {
    (0..N)
        .map(|i| Variable::new(format!("x{i}"), 3.0, 1.0).unwrap())
        .collect()
}

fn assert_minimum(name: &str, runs: &[Optimum], minimum: f64, point: &[f64]) {
    for (seed, run) in SEEDS.zip(runs) {
        assert_eq!(
            run.violation, 0.0,
            "{name}, seed {seed}: keeps every constraint"
        );
        let error = ((run.value - minimum) / minimum).abs();
        assert!(
            error < TOLERANCE,
            "{name}, seed {seed}: {} against {minimum} ({error:.1e})",
            run.value
        );
        let distance = run
            .point
            .iter()
            .zip(point)
            .map(|(x, p)| (x - p).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(
            distance < 1e-4,
            "{name}, seed {seed}: {distance:.1e} from the minimum"
        );
        // Re-evaluated, the winner gives what the run reported.
        let again = match name {
            "sphere_above" => sphere_above(&run.point),
            "tangent" => tangent(&run.point),
            _ => g06(&run.point),
        };
        assert_eq!(again.value, run.value);
        assert!(again.is_feasible());
    }
}

#[test]
fn sphere_above_reaches_its_minimum_on_the_constraint() {
    let mut point = vec![0.0; N];
    point[0] = 1.0;
    assert_minimum("sphere_above", &runs(ten(), sphere_above), 1.0, &point);
}

#[test]
fn tangent_problem_reaches_its_minimum_on_a_slanted_constraint() {
    let point = vec![1.0; N];
    assert_minimum("tangent", &runs(ten(), tangent), N as f64, &point);
}

#[test]
fn g06_reaches_the_circles_crossing() {
    let variables = vec![
        Variable::new("x0", 20.0, 2.0)
            .unwrap()
            .within(13.0, 100.0)
            .unwrap(),
        Variable::new("x1", 5.0, 2.0)
            .unwrap()
            .within(0.0, 100.0)
            .unwrap(),
    ];
    let point = [G06_X0, g06_x1()];
    let minimum = g06(&point).value;
    // CEC 2006's published minimum, to the digits it gives.
    assert!((minimum - -6961.813_875_580_15).abs() < 1e-6, "{minimum}");
    assert_minimum("g06", &runs(variables, g06), minimum, &point);
}

/// The infeasible start: from x = 0, every candidate of the first generations breaks
/// `x₀ ≥ 1`, and the violation alone must lead the run to the feasible side.
#[test]
fn a_run_started_outside_finds_the_feasible_side() {
    let variables = (0..N)
        .map(|i| Variable::new(format!("x{i}"), -5.0, 1.0).unwrap())
        .collect();
    let mut point = vec![0.0; N];
    point[0] = 1.0;
    assert_minimum("sphere_above", &runs(variables, sphere_above), 1.0, &point);
}
