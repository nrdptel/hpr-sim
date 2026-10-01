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
