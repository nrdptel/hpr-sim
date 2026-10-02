//! Test functions with known minima, for checking an optimizer.
//!
//! Each is a test function of CMA-ES's authors, from N. Hansen, S. D. Müller and P. Koumoutsakos,
//! "Reducing the time complexity of the derandomized evolution strategy with covariance matrix
//! adaptation (CMA-ES)", *Evolutionary Computation* 11(1), 1–18 (2003),
//! <https://doi.org/10.1162/106365603321828970>, Table 1 (printed p. 7), with the variables
//! numbered from 0 here. Their minima are known exactly, so a run's best point and value can be
//! held to them; that paper runs each to `f = 10⁻¹⁰`, as the tests do.
//!
//! | Function | What it tests | Minimum |
//! |---|---|---|
//! | [`sphere`] | the step size alone | 0 at `x = 0` |
//! | [`ellipsoid`] | coefficients from 1 to 10⁶, so the distribution must grow 1,000 times longer one way than the other | 0 at `x = 0` |
//! | [`rotated_ellipsoid`] | the same along axes that aren't the variables' | 0 at `x = 0` |
//! | [`rosenbrock`] | following a long, curved valley | 0 at `x = 1` |
//!
//! [`constrained`] holds three problems whose minima lie on their constraints' edges.
//! [`mixed`] holds three whose second half of variables take only whole numbers.

/// The ellipsoid's condition number: the ratio of its largest curvature to its smallest.
pub const ELLIPSOID_CONDITION: f64 = 1e6;

/// The sphere, `f(x) = Σ xᵢ²`.
pub fn sphere(x: &[f64]) -> f64 {
    x.iter().map(|xi| xi * xi).sum()
}

/// The ellipsoid, `f(x) = Σ 10^(6 i/(n − 1)) xᵢ²` for `i = 0 … n − 1`: its curvatures run from 1
/// to [`ELLIPSOID_CONDITION`]. With one variable it is the sphere.
pub fn ellipsoid(x: &[f64]) -> f64 {
    let n = x.len();
    if n < 2 {
        return sphere(x);
    }
    let denominator = (n - 1) as f64;
    x.iter()
        .enumerate()
        .map(|(i, xi)| ELLIPSOID_CONDITION.powf(i as f64 / denominator) * xi * xi)
        .sum()
}

/// The ellipsoid turned so that its axes aren't the variables': `ellipsoid(H x)`, with `H` the
/// reflection `I − 2 v vᵀ/(vᵀv)`, `vᵢ = i + 1`. `H` is orthogonal, so the minimum is still 0 at
/// `x = 0`, but no variable can be stepped alone to reach it. CMA-ES is invariant under rotations
/// and reflections of the variables, so it should take about as many evaluations as on
/// [`ellipsoid`].
pub fn rotated_ellipsoid(x: &[f64]) -> f64 {
    let vv: f64 = (1..=x.len()).map(|i| (i * i) as f64).sum();
    let vx: f64 = x
        .iter()
        .enumerate()
        .map(|(i, xi)| (i + 1) as f64 * xi)
        .sum();
    let factor = 2.0 * vx / vv;
    let hx: Vec<f64> = x
        .iter()
        .enumerate()
        .map(|(i, xi)| xi - factor * (i + 1) as f64)
        .collect();
    ellipsoid(&hx)
}

/// Rosenbrock's function, `f(x) = Σ [100 (xᵢ² − xᵢ₊₁)² + (1 − xᵢ)²]` for `i = 0 … n − 2`. Its
/// global minimum is 0 at `x = 1`. "For higher dimension, even function 8 f_Rosen has a local
/// minimum near y = (−1, 1, …, 1)ᵀ" (N. Hansen and A. Ostermeier, *Evolutionary Computation* 9(2), 159–195,
/// 2001, <https://doi.org/10.1162/106365601750190398>, footnote 18), and CMA-ES sometimes misses
/// the global one: in 1 to 3 of 20 runs at 4 to 16 variables in S. Kern, N. Hansen and
/// P. Koumoutsakos, "Local meta-models for optimization using evolution strategies", *PPSN IX*
/// (2006), Table 3.
pub fn rosenbrock(x: &[f64]) -> f64 {
    x.windows(2)
        .map(|w| 100.0 * (w[0] * w[0] - w[1]).powi(2) + (1.0 - w[0]).powi(2))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minima_are_zero_where_stated() {
        let zero = [0.0; 10];
        let one = [1.0; 10];
        assert_eq!(sphere(&zero), 0.0);
        assert_eq!(ellipsoid(&zero), 0.0);
        assert_eq!(rotated_ellipsoid(&zero), 0.0);
        assert_eq!(rosenbrock(&one), 0.0);
    }

    #[test]
    fn values_at_a_point_by_hand() {
        // Two variables: the ellipsoid's curvatures are 1 and 1e6.
        assert_eq!(ellipsoid(&[1.0, 1.0]), 1.0 + 1e6);
        assert_eq!(ellipsoid(&[3.0]), 9.0);
        // Rosenbrock at the origin: one term per pair, each 100·0 + 1.
        assert_eq!(rosenbrock(&[0.0; 10]), 9.0);
        // H with v = (1, 2) maps e₁ to e₁ − (2/5)(1, 2) = (0.6, −0.8).
        let turned = rotated_ellipsoid(&[1.0, 0.0]);
        let by_hand = 0.6f64.powi(2) + 1e6 * 0.8f64.powi(2);
        assert!((turned - by_hand).abs() <= 1e-9, "{turned} vs {by_hand}");
    }

    /// The reflection keeps lengths: on the sphere's terms (all curvatures 1) the turned and
    /// unturned values agree, checked through a one-variable-at-a-time identity.
    #[test]
    fn reflection_keeps_lengths() {
        let x = [0.3, -1.2, 2.5, 0.7];
        let vv = 30.0;
        let vx: f64 = x
            .iter()
            .enumerate()
            .map(|(i, xi)| (i + 1) as f64 * xi)
            .sum();
        let hx: Vec<f64> = x
            .iter()
            .enumerate()
            .map(|(i, xi)| xi - 2.0 * vx / vv * (i + 1) as f64)
            .collect();
        assert!((sphere(&hx) - sphere(&x)).abs() <= 1e-14);
    }
}

/// Constrained test problems with known minima on their constraints' edges, for
/// [`Run::tell_constrained`](super::cmaes::Run::tell_constrained). Each gives an
/// [`Evaluation`](super::Evaluation) with constraints written `g(x) ≤ 0`.
///
/// | Problem | Constraint | Minimum |
/// |---|---|---|
/// | [`constrained::sphere_above`] | `x₀ ≥ 1` | 1 at `x = (1, 0, …, 0)` |
/// | [`constrained::tangent`] | `Σ xᵢ ≥ n` | `n` at `x = 1` |
/// | [`constrained::g06`] | two circles | `(x₀* − 10)³ + (x₁* − 20)³` at their crossing |
pub mod constrained {
    use crate::optimize::Evaluation;

    /// The sphere `Σ xᵢ²` with `x₀ ≥ 1`, as `g = 1 − x₀ ≤ 0`. Its minimum is 1, at
    /// `x = (1, 0, …, 0)`: the constraint binds, as the sphere's own minimum breaks it.
    pub fn sphere_above(x: &[f64]) -> Evaluation {
        let g = 1.0 - x.first().copied().unwrap_or(0.0);
        Evaluation::constrained(super::sphere(x), &[g])
    }

    /// The tangent problem: the sphere with `Σ xᵢ ≥ n`, as `g = n − Σ xᵢ ≤ 0`, a constraint along
    /// no variable's axis. By symmetry and Lagrange's condition `2 xᵢ = λ`, the minimum is `n`, at
    /// `x = 1`.
    pub fn tangent(x: &[f64]) -> Evaluation {
        let n = x.len() as f64;
        let g = n - x.iter().sum::<f64>();
        Evaluation::constrained(super::sphere(x), &[g])
    }

    /// Problem g06 of the CEC 2006 constrained benchmark (J. J. Liang et al., "Problem
    /// definitions and evaluation criteria for the CEC 2006 special session on constrained
    /// real-parameter optimization", Nanyang Technological University (2006)), two variables:
    /// `f = (x₀ − 10)³ + (x₁ − 20)³`, with `g₁ = 100 − (x₀ − 5)² − (x₁ − 5)² ≤ 0` (outside one
    /// circle) and `g₂ = (x₀ − 6)² + (x₁ − 5)² − 82.81 ≤ 0` (inside another), and bounds
    /// `13 ≤ x₀ ≤ 100`, `0 ≤ x₁ ≤ 100` for the caller to set. The minimum is where the circles
    /// cross: subtracting the two edges gives `2 x₀ − 11 = 17.19`, so [`G06_X0`], and
    /// [`g06_x1`] below the centres.
    pub fn g06(x: &[f64]) -> Evaluation {
        let (a, b) = (
            x.first().copied().unwrap_or(0.0),
            x.get(1).copied().unwrap_or(0.0),
        );
        let f = (a - 10.0).powi(3) + (b - 20.0).powi(3);
        let g1 = 100.0 - (a - 5.0).powi(2) - (b - 5.0).powi(2);
        let g2 = (a - 6.0).powi(2) + (b - 5.0).powi(2) - 82.81;
        Evaluation::constrained(f, &[g1, g2])
    }

    /// g06's minimum's first coordinate, `x₀* = 14.095`.
    pub const G06_X0: f64 = 14.095;

    /// g06's minimum's second coordinate, `x₁* = 5 − √(100 − (x₀* − 5)²)`.
    pub fn g06_x1() -> f64 {
        5.0 - (100.0 - (G06_X0 - 5.0).powi(2)).sqrt()
    }
}

/// Test functions of continuous and integer variables together: those of R. Hamano, S. Saito,
/// M. Nomura and S. Shirakawa, "CMA-ES with Margin: Lower-Bounding Marginal Probability for
/// Mixed-Integer Black-Box Optimization", GECCO 2022, <https://arxiv.org/abs/2205.13482>, §5.1
/// (p. 7). The first `⌊n/2⌋` variables are continuous and the rest integer
/// ([`Variable::integer`](super::Variable::integer)); each function is given the values the
/// optimizer encodes, whole numbers in the integer variables.
///
/// | Function | Integer variables | Minimum |
/// |---|---|---|
/// | [`sphere_int`](mixed::sphere_int) | from −10 to 10 | 0 at `x = 0` |
/// | [`ellipsoid_int`](mixed::ellipsoid_int) | from −10 to 10, with the largest coefficients | 0 at `x = 0` |
/// | [`sphere_one_max`](mixed::sphere_one_max) | 0 or 1 | 0 at continuous 0, integer 1 |
pub mod mixed {
    use super::{ellipsoid, sphere};

    /// SphereInt, `f(x) = Σ xᵢ²` over every variable: [`sphere`].
    pub fn sphere_int(x: &[f64]) -> f64 {
        sphere(x)
    }

    /// EllipsoidInt, `f(x) = Σ (1000^(i/(n − 1)) xᵢ)²`: [`ellipsoid`], whose coefficients are the
    /// same, so the integer variables, the last, have the largest.
    pub fn ellipsoid_int(x: &[f64]) -> f64 {
        ellipsoid(x)
    }

    /// SphereOneMax, `f(x) = Σ xᵢ² + n_b − Σ x_k`, the first sum over the first `⌊n/2⌋`
    /// variables (continuous) and the second over the `n_b` others (0 or 1).
    pub fn sphere_one_max(x: &[f64]) -> f64 {
        let (continuous, binary) = x.split_at(x.len() / 2);
        // Cast: at most 200 variables.
        sphere(continuous) + binary.len() as f64 - binary.iter().sum::<f64>()
    }
}
