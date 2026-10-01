//! M6.1c: Morris screening and Sobol' indices against test functions with closed forms.
//!
//! Each estimate is held to its known value within four of its own standard errors, and, over
//! many seeds, the standard errors are held to the spread the estimates really have: so "within
//! sampling error" means what it says.

use std::f64::consts::PI;

use hpr_analysis::sensitivity::benchmark::{Ishigami, SobolG};
use hpr_analysis::sensitivity::morris::{ElementaryEffects, Morris};
use hpr_analysis::sensitivity::sobol::Sobol;

/// g-function parameters spanning the four classes Marrel and others (2008) name, as the SFU
/// Virtual Library of Simulation Experiments quotes them: `aᵢ = 0` very important, 1 relatively
/// important, 9 non-important, 99 non-significant; 4.5 between the middle two.
const G_A: [f64; 8] = [0.0, 1.0, 4.5, 9.0, 99.0, 99.0, 99.0, 99.0];

/// The mean and the sample standard deviation of `xs`.
fn mean_sd(xs: &[f64]) -> (f64, f64) {
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    let sd = (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
    (mean, sd)
}

#[test]
fn sobol_indices_of_ishigami_match_the_closed_form() {
    let f = Ishigami::STANDARD;
    let indices = Sobol::new(Ishigami::factors().unwrap(), 1 << 15)
        .unwrap()
        .indices(2026, |x| f.evaluate(x))
        .unwrap();
    let (first, total) = (f.first_order(), f.total());
    for (i, index) in indices.factors.iter().enumerate() {
        let z1 = (index.first_order - first[i]) / index.first_order_standard_error;
        let zt = (index.total - total[i]) / index.total_standard_error;
        assert!(z1.abs() < 4.0 && zt.abs() < 4.0, "{index:?}: z {z1}, {zt}");
        assert!(index.first_order_standard_error < 0.01 && index.total_standard_error < 0.01);
    }
    assert!((indices.mean - f.mean()).abs() < 0.05);
    assert!((indices.variance - f.variance()).abs() / f.variance() < 0.02);
}

#[test]
fn sobol_indices_of_the_g_function_match_the_closed_form() {
    let g = SobolG::new(G_A.to_vec()).unwrap();
    let indices = Sobol::new(g.factors().unwrap(), 1 << 14)
        .unwrap()
        .indices(2026, |x| g.evaluate(x))
        .unwrap();
    let (first, total) = (g.first_order(), g.total());
    for (i, index) in indices.factors.iter().enumerate() {
        let z1 = (index.first_order - first[i]) / index.first_order_standard_error;
        let zt = (index.total - total[i]) / index.total_standard_error;
        assert!(z1.abs() < 4.0 && zt.abs() < 4.0, "{index:?}: z {z1}, {zt}");
    }
}

/// Over many seeds, `(estimate − known) / standard error` should have mean 0 and standard
/// deviation 1. With `seeds` independent runs the sample mean's own error is `1/√seeds` and the
/// standard deviation's about `1/√(2 seeds)`; four of each are allowed. The check must also have
/// the power to see standard errors 15% too small: the same `z`s divided by 0.85 must fail it.
fn assert_calibrated(what: &str, zs: &[f64]) {
    let calibrated = |zs: &[f64]| {
        let n = zs.len() as f64;
        let (mean, sd) = mean_sd(zs);
        mean.abs() < 4.0 / n.sqrt() && (sd - 1.0).abs() < 4.0 / (2.0 * n).sqrt()
    };
    let (mean, sd) = mean_sd(zs);
    assert!(
        calibrated(zs),
        "{what}: z mean {mean}, standard deviation {sd}"
    );
    let too_small: Vec<f64> = zs.iter().map(|z| z / 0.85).collect();
    assert!(
        !calibrated(&too_small),
        "{what}: can't see 15% too small (sd {sd})"
    );
}

#[test]
fn sobol_standard_errors_are_the_spread_of_the_estimates() {
    let f = Ishigami::STANDARD;
    let sobol = Sobol::new(Ishigami::factors().unwrap(), 2000).unwrap();
    let (first, total) = (f.first_order(), f.total());
    let seeds = 1000;
    let mut z = (0..6)
        .map(|_| Vec::with_capacity(seeds))
        .collect::<Vec<_>>();
    for seed in 0..seeds as u64 {
        let indices = sobol.indices(seed, |x| f.evaluate(x)).unwrap();
        for (i, index) in indices.factors.iter().enumerate() {
            z[i].push((index.first_order - first[i]) / index.first_order_standard_error);
            z[3 + i].push((index.total - total[i]) / index.total_standard_error);
        }
    }
    for (i, zs) in z.iter().enumerate() {
        let what = if i < 3 { "first order" } else { "total" };
        assert_calibrated(&format!("Ishigami {what} x{}", i % 3 + 1), zs);
    }
}

#[test]
fn sobol_standard_errors_of_the_g_function_are_the_spread_of_the_estimates() {
    let g = SobolG::new(G_A[..4].to_vec()).unwrap();
    let sobol = Sobol::new(g.factors().unwrap(), 2000).unwrap();
    let (first, total) = (g.first_order(), g.total());
    let seeds = 500;
    let mut z = (0..8)
        .map(|_| Vec::with_capacity(seeds))
        .collect::<Vec<_>>();
    for seed in 0..seeds as u64 {
        let indices = sobol.indices(seed, |x| g.evaluate(x)).unwrap();
        for (i, index) in indices.factors.iter().enumerate() {
            z[i].push((index.first_order - first[i]) / index.first_order_standard_error);
            z[4 + i].push((index.total - total[i]) / index.total_standard_error);
        }
    }
    for (i, zs) in z.iter().enumerate() {
        let what = if i < 4 { "first order" } else { "total" };
        assert_calibrated(&format!("g {what} x{}", i % 4 + 1), zs);
    }
}

/// Ishigami's elementary effects at p = 4 in closed form: the grid is `−π, −π/3, π/3, π`, so
/// `Δ = 2/3` of the range, and each step goes from `−π` to `π/3` or from `−π/3` to `π`.
///
/// - `x₁`: `sin` rises `√3/2` on either step, so `d₁ = (√3/2)(1 + b x₃⁴)/Δ`, with `x₃⁴` either
///   `π⁴` or `π⁴/81`: always positive.
/// - `x₂`: `sin²` rises `3/4` on one step and falls `3/4` on the other: `d₂ = ±9a/8`.
/// - `x₃`: `x₃⁴` changes by `∓80π⁴/81`, times `b sin x₁`, which is 0 at `±π` and `±√3/2` at
///   `±π/3`: half the effects are zero, the rest `±20√3 bπ⁴/27`.
fn ishigami_population(f: Ishigami) -> [(f64, f64, f64); 3] {
    let (a, b) = (f.a, f.b);
    let c = 3.0 * 3.0_f64.sqrt() / 4.0;
    let pi4 = PI.powi(4);
    let d3 = 20.0 * 3.0_f64.sqrt() * b * pi4 / 27.0;
    [
        (
            c * (1.0 + b * pi4 * (1.0 + 1.0 / 81.0) / 2.0),
            c * (1.0 + b * pi4 * (1.0 + 1.0 / 81.0) / 2.0),
            c * b * pi4 * (1.0 - 1.0 / 81.0) / 2.0,
        ),
        (0.0, 9.0 * a / 8.0, 9.0 * a / 8.0),
        (0.0, d3 / 2.0, d3 / 2.0_f64.sqrt()),
    ]
}

/// The g function's elementary effects at p = 4 in closed form: `|4x − 2|` is `2, 2/3, 2/3, 2`
/// on the grid, so each step changes `gᵢ` by `±(4/3)/(1 + aᵢ)` and `dᵢ = ±2/(1 + aᵢ) Πⱼ≠ᵢ gⱼ`,
/// with the sign independent of the other factors: `μ = 0`, `μ* = 2/(1 + aᵢ) Πⱼ≠ᵢ E[gⱼ]` and
/// `σ = 2/(1 + aᵢ) √(Πⱼ≠ᵢ E[gⱼ²])`, the means over the four levels.
fn g_population(a: &[f64]) -> Vec<(f64, f64, f64)> {
    let mean = |aj: f64| (4.0 / 3.0 + aj) / (1.0 + aj);
    let square =
        |aj: f64| ((2.0 + aj).powi(2) + (2.0 / 3.0 + aj).powi(2)) / 2.0 / (1.0 + aj).powi(2);
    (0..a.len())
        .map(|i| {
            let others = |m: &dyn Fn(f64) -> f64| -> f64 {
                a.iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, &aj)| m(aj))
                    .product()
            };
            let scale = 2.0 / (1.0 + a[i]);
            (0.0, scale * others(&mean), scale * others(&square).sqrt())
        })
        .collect()
}

fn assert_moments(what: &str, got: &ElementaryEffects, (mu, mu_star, sigma): (f64, f64, f64)) {
    let close = |x: f64, y: f64| (x - y).abs() <= 1e-12 * (1.0 + y.abs());
    assert!(
        close(got.mean, mu)
            && close(got.mean_absolute, mu_star)
            && close(got.standard_deviation, sigma),
        "{what}: {got:?} against ({mu}, {mu_star}, {sigma})"
    );
}

#[test]
fn morris_population_is_the_closed_form() {
    let f = Ishigami::STANDARD;
    let morris = Morris::new(Ishigami::factors().unwrap(), 4, 2).unwrap();
    let population = morris.population(|x| f.evaluate(x)).unwrap();
    for (i, (got, want)) in population.iter().zip(ishigami_population(f)).enumerate() {
        assert_eq!(got.count, 32);
        assert_moments(&format!("Ishigami x{}", i + 1), got, want);
    }
    let g = SobolG::new(G_A.to_vec()).unwrap();
    let morris = Morris::new(g.factors().unwrap(), 4, 2).unwrap();
    let population = morris.population(|x| g.evaluate(x)).unwrap();
    for (i, (got, want)) in population.iter().zip(g_population(&G_A)).enumerate() {
        assert_eq!(got.count, 4_usize.pow(8) / 2);
        assert_moments(&format!("g x{}", i + 1), got, want);
    }
}

#[test]
fn morris_screening_matches_the_closed_form() {
    let f = Ishigami::STANDARD;
    let screening = Morris::new(Ishigami::factors().unwrap(), 4, 1000)
        .unwrap()
        .screen(2026, |x| f.evaluate(x))
        .unwrap();
    let g = SobolG::new(G_A.to_vec()).unwrap();
    let g_screening = Morris::new(g.factors().unwrap(), 4, 1000)
        .unwrap()
        .screen(2026, |x| g.evaluate(x))
        .unwrap();
    let pairs = screening
        .effects
        .iter()
        .zip(ishigami_population(f))
        .chain(g_screening.effects.iter().zip(g_population(&G_A)));
    for (got, (mu, mu_star, sigma)) in pairs {
        // μ*'s error is the run's own; μ's is σ/√r, with σ known.
        let r = got.count as f64;
        if got.mean_absolute_standard_error > 1e-12 {
            let z = (got.mean_absolute - mu_star) / got.mean_absolute_standard_error;
            assert!(z.abs() < 4.0, "{got:?}: μ* z {z}");
        } else {
            // Every |effect| equal up to rounding: Ishigami's x₂, whose error is ~1e-16.
            assert!((got.mean_absolute - mu_star).abs() < 1e-12, "{got:?}");
        }
        assert!((got.mean - mu).abs() < 4.0 * sigma / r.sqrt(), "{got:?}");
        assert!(
            (got.standard_deviation - sigma).abs() < 0.1 * sigma,
            "{got:?}"
        );
    }
}

#[test]
fn morris_standard_errors_are_the_spread_of_the_estimates() {
    let g = SobolG::new(G_A[..4].to_vec()).unwrap();
    let morris = Morris::new(g.factors().unwrap(), 4, 50).unwrap();
    let known = g_population(&G_A[..4]);
    let seeds = 1000;
    let mut z = (0..4)
        .map(|_| Vec::with_capacity(seeds))
        .collect::<Vec<_>>();
    for seed in 0..seeds as u64 {
        let screening = morris.screen(seed, |x| g.evaluate(x)).unwrap();
        for (i, e) in screening.effects.iter().enumerate() {
            z[i].push((e.mean_absolute - known[i].1) / e.mean_absolute_standard_error);
        }
    }
    for (i, zs) in z.iter().enumerate() {
        assert_calibrated(&format!("g μ* x{}", i + 1), zs);
    }
}

#[test]
fn morris_ranks_the_g_function_as_its_total_indices() {
    // Campolongo and others' claim, by experiment: μ* orders factors as S_T does. On the g
    // function with distinct aᵢ, twenty paths already order the four that matter.
    let g = SobolG::new(G_A.to_vec()).unwrap();
    let screening = Morris::new(g.factors().unwrap(), 4, 20)
        .unwrap()
        .screen(2026, |x| g.evaluate(x))
        .unwrap();
    let total = g.total();
    let mu_star: Vec<f64> = screening.effects.iter().map(|e| e.mean_absolute).collect();
    for i in 0..3 {
        assert!(total[i] > total[i + 1]);
        assert!(mu_star[i] > mu_star[i + 1], "{mu_star:?}");
    }
    // The four at a = 99 hardly matter: their known μ* is about a tenth of x₄'s (2/100 against
    // 2/10, times their other factors' means); a fifth allows for twenty paths' error.
    let known = g_population(&G_A);
    for (i, &negligible) in mu_star.iter().enumerate().skip(4) {
        assert!(known[i].1 < 0.11 * known[3].1);
        assert!(negligible < 0.2 * mu_star[3], "{mu_star:?}");
    }
}

#[test]
fn the_same_seed_gives_the_same_numbers_bit_for_bit() {
    let f = Ishigami::STANDARD;
    let sobol = Sobol::new(Ishigami::factors().unwrap(), 300).unwrap();
    assert_eq!(
        sobol.indices(5, |x| f.evaluate(x)).unwrap(),
        sobol.indices(5, |x| f.evaluate(x)).unwrap()
    );
    let morris = Morris::new(Ishigami::factors().unwrap(), 4, 30).unwrap();
    assert_eq!(
        morris.screen(5, |x| f.evaluate(x)).unwrap(),
        morris.screen(5, |x| f.evaluate(x)).unwrap()
    );
}
