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

/// Test functions for global optimization with few evaluations, for [`ego`](super::ego): the
/// Branin and Hartmann functions of L. C. W. Dixon and G. P. Szegö, "The global optimisation
/// problem: an introduction", in *Towards Global Optimisation 2*, North-Holland, 1–15 (1978),
/// as D. R. Jones, M. Schonlau and W. J. Welch, "Efficient global optimization of expensive
/// black-box functions", *Journal of Global Optimization* 13, 455–492 (1998),
/// <https://doi.org/10.1023/A:1008306431147>, print them in their Table 1 and run EGO on them.
/// Each has several local minima.
///
/// | Function | Variables | Minimum |
/// |---|---|---|
/// | [`branin`](global::branin) | `x₀` in `[−5, 10]`, `x₁` in `[0, 15]` | `5/(4π) ≈ 0.397887` at three points ([`BRANIN_MINIMA`](global::BRANIN_MINIMA)) |
/// | [`hartmann3`](global::hartmann3) | 3, each in `[0, 1]` | `≈ −3.86278` ([`HARTMANN3_MINIMUM`](global::HARTMANN3_MINIMUM)) |
/// | [`hartmann6`](global::hartmann6) | 6, each in `[0, 1]` | `≈ −3.32237` ([`HARTMANN6_MINIMUM`](global::HARTMANN6_MINIMUM)) |
pub mod global {
    use std::f64::consts::PI;

    /// Branin's minimum, `s t = 10/(8π) = 5/(4π)`: the squared term vanishes and `cos x₀ = −1`.
    pub const BRANIN_MINIMUM: f64 = 5.0 / (4.0 * PI);

    /// Branin's three minima: `x₀ = −π, π, 3π`, where `cos x₀ = −1`, and `x₁` the root of its
    /// squared term, `b x₀² − c x₀ + r` (12.275, 2.275 and 2.475).
    pub const BRANIN_MINIMA: [[f64; 2]; 3] = [[-PI, 12.275], [PI, 2.275], [3.0 * PI, 2.475]];

    /// Branin's function of two variables, `x₀` in `[−5, 10]` and `x₁` in `[0, 15]`:
    /// `f = (x₁ − b x₀² + c x₀ − r)² + s (1 − t) cos x₀ + s`, with `b = 5.1/(4π²)`, `c = 5/π`,
    /// `r = 6`, `s = 10` and `t = 1/(8π)`.
    pub fn branin(x: &[f64]) -> f64 {
        let (b, c, r, s, t) = (5.1 / (4.0 * PI * PI), 5.0 / PI, 6.0, 10.0, 1.0 / (8.0 * PI));
        let square = x[1] - b * x[0] * x[0] + c * x[0] - r;
        square * square + s * (1.0 - t) * x[0].cos() + s
    }

    /// The Hartmann 3 function's least value, as printed to six figures.
    pub const HARTMANN3_MINIMUM: f64 = -3.86278;

    /// The Hartmann 6 function's least value, as printed to six figures.
    pub const HARTMANN6_MINIMUM: f64 = -3.32237;

    /// The Hartmann functions' weights `αᵢ`.
    const ALPHA: [f64; 4] = [1.0, 1.2, 3.0, 3.2];

    /// `f = −Σᵢ αᵢ exp(−Σⱼ aᵢⱼ (xⱼ − pᵢⱼ)²)`, the Hartmann functions' form.
    fn hartmann<const N: usize>(x: &[f64], a: &[[f64; N]; 4], p: &[[f64; N]; 4]) -> f64 {
        -(0..4)
            .map(|i| {
                let exponent: f64 = (0..N)
                    .map(|j| a[i][j] * (x[j] - p[i][j]) * (x[j] - p[i][j]))
                    .sum();
                ALPHA[i] * (-exponent).exp()
            })
            .sum::<f64>()
    }

    /// The Hartmann 3 function, each variable in `[0, 1]`; its least value is about −3.86278,
    /// near `(0.114614, 0.555649, 0.852547)`. `p₃₀` is 0.0381 here; some listings print 0.03815,
    /// which is far from the minimum and doesn't move it.
    pub fn hartmann3(x: &[f64]) -> f64 {
        const A: [[f64; 3]; 4] = [
            [3.0, 10.0, 30.0],
            [0.1, 10.0, 35.0],
            [3.0, 10.0, 30.0],
            [0.1, 10.0, 35.0],
        ];
        const P: [[f64; 3]; 4] = [
            [0.3689, 0.1170, 0.2673],
            [0.4699, 0.4387, 0.7470],
            [0.1091, 0.8732, 0.5547],
            [0.0381, 0.5743, 0.8828],
        ];
        hartmann(x, &A, &P)
    }

    /// The Hartmann 6 function, each variable in `[0, 1]`; its least value is about −3.32237,
    /// near `(0.20169, 0.150011, 0.476874, 0.275332, 0.311652, 0.6573)`.
    pub fn hartmann6(x: &[f64]) -> f64 {
        const A: [[f64; 6]; 4] = [
            [10.0, 3.0, 17.0, 3.5, 1.7, 8.0],
            [0.05, 10.0, 17.0, 0.1, 8.0, 14.0],
            [3.0, 3.5, 1.7, 10.0, 17.0, 8.0],
            [17.0, 8.0, 0.05, 10.0, 0.1, 14.0],
        ];
        const P: [[f64; 6]; 4] = [
            [0.1312, 0.1696, 0.5569, 0.0124, 0.8283, 0.5886],
            [0.2329, 0.4135, 0.8307, 0.3736, 0.1004, 0.9991],
            [0.2348, 0.1451, 0.3522, 0.2883, 0.3047, 0.6650],
            [0.4047, 0.8828, 0.8732, 0.5743, 0.1091, 0.0381],
        ];
        hartmann(x, &A, &P)
    }
}

/// Two-goal test problems with known Pareto fronts, for [`nsga2`](super::nsga2): ZDT1, ZDT2 and
/// ZDT3 of E. Zitzler, K. Deb and L. Thiele, "Comparison of multiobjective evolutionary
/// algorithms: empirical results", *Evolutionary Computation* 8(2), 173–195 (2000),
/// <https://doi.org/10.1162/106365600568202>, §4, eqs. (7)–(9) (pp. 177–178), each of `n` variables in `[0, 1]`
/// (the paper's `n = 30`):
///
/// `f₁ = x₀`, `g = 1 + 9 Σᵢ₌₁ⁿ⁻¹ xᵢ / (n − 1)`, `f₂ = g h(f₁, g)`.
///
/// The front is `g = 1`, every variable but the first zero, where `f₂ = h(f₁, 1)`:
///
/// | Problem | `h(f₁, g)` | The front |
/// |---|---|---|
/// | ZDT1 | `1 − √(f₁/g)` | `f₂ = 1 − √f₁`, convex, `f₁` from 0 to 1 |
/// | ZDT2 | `1 − (f₁/g)²` | `f₂ = 1 − f₁²`, concave |
/// | ZDT3 | `1 − √(f₁/g) − (f₁/g) sin(10π f₁)` | five separate pieces of `f₂ = 1 − √f₁ − f₁ sin(10π f₁)` ([`zdt::ZDT3_PIECES`]) |
pub mod zdt {
    /// One of the three problems.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    #[non_exhaustive]
    pub enum Zdt {
        /// ZDT1: a convex front.
        One,
        /// ZDT2: a concave front.
        Two,
        /// ZDT3: a front in five pieces.
        Three,
    }

    /// The `f₁` ranges of ZDT3's five pieces of front. Along `f₂ = 1 − √f₁ − f₁ sin(10π f₁)`,
    /// a point is on the front only if `f₂` is below its value at every smaller `f₁`: each piece
    /// ends at a local minimum of the curve (where its slope is zero), and the next starts where
    /// the curve, falling again, first drops below that minimum. These were solved to 40 digits
    /// (mpmath 1.3.0's `findroot`), here rounded to the nearest `f64`, and agree with the
    /// ten-digit ranges published for ZDT3; the tests check each end's equation.
    pub const ZDT3_PIECES: [(f64, f64); 5] = [
        (0.0, 0.083_001_534_926_911_63),
        (0.182_228_728_029_399_77, 0.257_762_363_387_830_2),
        (0.409_313_674_808_656_8, 0.453_882_104_088_830_2),
        (0.618_396_794_439_265_8, 0.652_511_703_804_662_5),
        (0.823_331_798_326_632_7, 0.851_832_865_436_413_9),
    ];

    /// The whole range of `f₁`, ZDT1's and ZDT2's one piece of front.
    const WHOLE: [(f64, f64); 1] = [(0.0, 1.0)];

    /// `g = 1 + 9 Σᵢ₌₁ⁿ⁻¹ xᵢ / (n − 1)`; 1 for a single variable.
    fn g(x: &[f64]) -> f64 {
        let rest = x.get(1..).unwrap_or(&[]);
        if rest.is_empty() {
            return 1.0;
        }
        // Cast: at most 200 variables.
        1.0 + 9.0 * rest.iter().sum::<f64>() / rest.len() as f64
    }

    impl Zdt {
        /// `h(f₁, g)`.
        fn h(self, f1: f64, g: f64) -> f64 {
            let r = f1 / g;
            match self {
                Self::One => 1.0 - r.sqrt(),
                Self::Two => 1.0 - r * r,
                Self::Three => 1.0 - r.sqrt() - r * (10.0 * std::f64::consts::PI * f1).sin(),
            }
        }

        /// The problem's two goals at `x`, `[f₁, f₂]`; `x₀` is taken as 0 if `x` is empty.
        pub fn evaluate(self, x: &[f64]) -> Vec<f64> {
            let f1 = x.first().copied().unwrap_or(0.0);
            let g = g(x);
            vec![f1, g * self.h(f1, g)]
        }

        /// The curve the front lies on, `f₂ = h(f₁, 1)`, at `f₁`.
        pub fn front_f2(self, f1: f64) -> f64 {
            self.h(f1, 1.0)
        }

        /// The `f₁` ranges of the front's pieces: one for ZDT1 and ZDT2, five for ZDT3.
        pub fn pieces(self) -> &'static [(f64, f64)] {
            match self {
                Self::One | Self::Two => &WHOLE,
                Self::Three => &ZDT3_PIECES,
            }
        }

        /// `points` points of the front, `f₁` evenly spaced from 0 to 1, those outside ZDT3's
        /// pieces left out: a reference for
        /// [`inverted_generational_distance`](crate::optimize::nsga2::inverted_generational_distance).
        pub fn reference(self, points: usize) -> Vec<Vec<f64>> {
            // Cast: a count of points, exact in f64 below 2⁵³.
            let last = points.saturating_sub(1).max(1) as f64;
            (0..points)
                .map(|i| i as f64 / last)
                .filter(|&f1| {
                    self.pieces()
                        .iter()
                        .any(|&(lo, hi)| (lo..=hi).contains(&f1))
                })
                .map(|f1| vec![f1, self.front_f2(f1)])
                .collect()
        }

        /// The Euclidean distance from goals `f = [f₁, f₂]` to the front, the curve itself
        /// rather than points along it.
        ///
        /// Any point of the front bounds it, so the nearest point lies within `f₁ ± d₀` of `f₁`,
        /// where `d₀` is the distance to the front at `f₁` itself (or a piece's nearer end):
        /// that window of each piece is searched on a grid of 400 steps in `s` (`f₁ = s²` for
        /// ZDT1 and ZDT3, whose slope is infinite at 0; `f₁ = s` for ZDT2), and the best step
        /// refined by golden-section search over its two neighbours.
        pub fn distance_to_front(self, f: &[f64]) -> f64 {
            const STEPS: usize = 400;
            let (a, b) = (
                f.first().copied().unwrap_or(0.0),
                f.get(1).copied().unwrap_or(0.0),
            );
            let distance = |f1: f64| (a - f1).hypot(b - self.front_f2(f1));
            let squared = self != Self::Two;
            let to_f1 = |s: f64| if squared { s * s } else { s };
            let to_s = |f1: f64| if squared { f1.sqrt() } else { f1 };
            let d0 = self
                .pieces()
                .iter()
                .map(|&(lo, hi)| distance(a.clamp(lo, hi)))
                .fold(f64::INFINITY, f64::min);
            let mut best = d0;
            for &(lo, hi) in self.pieces() {
                let (left, right) = ((a - d0).max(lo), (a + d0).min(hi));
                if left > right {
                    continue;
                }
                let (s0, s1) = (to_s(left), to_s(right));
                // Cast: STEPS is small and exact in f64.
                let at = |k: usize| s0 + (s1 - s0) * k as f64 / STEPS as f64;
                let on = |s: f64| distance(to_f1(s));
                let (mut k_best, mut d_best) = (0, f64::INFINITY);
                for k in 0..=STEPS {
                    let d = on(at(k));
                    if d < d_best {
                        (k_best, d_best) = (k, d);
                    }
                }
                let (mut lo_s, mut hi_s) =
                    (at(k_best.saturating_sub(1)), at((k_best + 1).min(STEPS)));
                let ratio = (5.0_f64.sqrt() - 1.0) / 2.0;
                for _ in 0..100 {
                    let p = hi_s - ratio * (hi_s - lo_s);
                    let q = lo_s + ratio * (hi_s - lo_s);
                    if on(p) <= on(q) {
                        hi_s = q;
                    } else {
                        lo_s = p;
                    }
                }
                best = best.min(d_best).min(on(0.5 * (lo_s + hi_s)));
            }
            best
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        use std::f64::consts::PI;

        /// The slope of ZDT3's front curve, `dh/df₁ = −1/(2√f₁) − sin(10π f₁) − 10π f₁ cos(10π f₁)`.
        fn slope(f1: f64) -> f64 {
            -0.5 / f1.sqrt() - (10.0 * PI * f1).sin() - 10.0 * PI * f1 * (10.0 * PI * f1).cos()
        }

        /// Each piece of ZDT3's front ends where the curve's slope is zero, and the next starts
        /// at the same height, to rounding; the ends agree with the ten-digit ranges pymoo
        /// 0.6.2's `zdt.py` prints, but for its second start, 0.182228780, a digit short of
        /// 0.1822287280.
        #[test]
        fn zdt3_pieces_meet_their_equations() {
            let h = |f1| Zdt::Three.front_f2(f1);
            for (k, &(lo, hi)) in ZDT3_PIECES.iter().enumerate() {
                assert!(
                    slope(hi).abs() <= 1e-12,
                    "piece {k} end: slope {}",
                    slope(hi)
                );
                if k > 0 {
                    let previous = ZDT3_PIECES[k - 1].1;
                    assert!((h(lo) - h(previous)).abs() <= 1e-15, "piece {k} start");
                    assert!(slope(lo) < 0.0);
                }
            }
            let published = [
                (0.0, 0.083_001_534_9),
                (0.182_228_728_0, 0.257_762_363_4),
                (0.409_313_674_8, 0.453_882_104_1),
                (0.618_396_794_4, 0.652_511_703_8),
                (0.823_331_798_3, 0.851_832_865_4),
            ];
            for (&(lo, hi), (plo, phi)) in ZDT3_PIECES.iter().zip(published) {
                assert!((lo - plo).abs() <= 5e-11 && (hi - phi).abs() <= 5e-11);
            }
        }

        /// On a scan of 200,001 points of the curve and the pieces' ends, a point is a new
        /// lowest `f₂` exactly when it is inside a piece. (A piece's start ties the previous end's
        /// `f₂`, so is itself dominated: the ends only set the lowest.)
        #[test]
        fn zdt3_pieces_are_the_curves_undominated_points() {
            let ends: Vec<f64> = ZDT3_PIECES.iter().flat_map(|&(lo, hi)| [lo, hi]).collect();
            let mut scan: Vec<f64> = (0..=200_000).map(|i| f64::from(i) / 200_000.0).collect();
            scan.extend(&ends);
            scan.sort_by(f64::total_cmp);
            let mut lowest = f64::INFINITY;
            for f1 in scan {
                let f2 = Zdt::Three.front_f2(f1);
                if !ends.contains(&f1) {
                    let inside = ZDT3_PIECES.iter().any(|&(lo, hi)| (lo..=hi).contains(&f1));
                    assert_eq!(f2 < lowest, inside, "f₁ = {f1}");
                }
                lowest = lowest.min(f2);
            }
        }

        #[test]
        fn the_front_is_g_equal_to_one() {
            let mut x = vec![0.0; 30];
            for f1 in [0.0, 0.04, 0.2, 0.5, 0.83, 1.0] {
                x[0] = f1;
                for p in [Zdt::One, Zdt::Two, Zdt::Three] {
                    let f = p.evaluate(&x);
                    assert_eq!(f, vec![f1, p.front_f2(f1)]);
                }
            }
            x[5] = 0.29;
            // g = 1 + 9 × 0.29 / 29 = 1.09.
            let f = Zdt::Two.evaluate(&x);
            let r = 1.0 / 1.09;
            assert!((f[1] - 1.09 * (1.0 - r * r)).abs() <= 1e-15);
        }

        /// The distance to the front: zero on it; by hand for a point straight out from ZDT2's
        /// curve along its normal; and never more than a brute-force scan of 10⁶ points of it,
        /// nor less than that scan by more than its spacing could hide.
        #[test]
        fn distance_to_the_front() {
            for p in [Zdt::One, Zdt::Two, Zdt::Three] {
                for f1 in [0.0, 0.01, 0.25, 0.42, 0.65, 0.83] {
                    if p.pieces().iter().any(|&(lo, hi)| (lo..=hi).contains(&f1)) {
                        assert!(p.distance_to_front(&[f1, p.front_f2(f1)]) <= 1e-15);
                    }
                }
            }
            // ZDT2 at f₁ = 0.5: the curve's normal is (2 f₁, 1)/√(1 + 4 f₁²) = (1, 1)/√2.
            let d = 0.01;
            let out = [0.5 + d / 2f64.sqrt(), 0.75 + d / 2f64.sqrt()];
            assert!((Zdt::Two.distance_to_front(&out) - d).abs() <= 1e-12);
            let points = [
                [0.3, 0.5],
                [0.001, 1.2],
                [0.12, 0.9],
                [0.3, 0.3],
                [0.55, -0.1],
                [0.9, -0.5],
                [1.2, 0.1],
                [-0.1, 1.1],
            ];
            for p in [Zdt::One, Zdt::Two, Zdt::Three] {
                for f in points {
                    let mut scan = f64::INFINITY;
                    for &(lo, hi) in p.pieces() {
                        for i in 0..=1_000_000 {
                            let f1 = lo + (hi - lo) * f64::from(i) / 1e6;
                            scan = scan.min((f[0] - f1).hypot(f[1] - p.front_f2(f1)));
                        }
                    }
                    let d = p.distance_to_front(&f);
                    assert!(
                        d <= scan + 1e-15,
                        "{p:?} {f:?}: {d} above the scan's {scan}"
                    );
                    assert!(
                        scan - d <= 1e-6,
                        "{p:?} {f:?}: {d} far below the scan's {scan}"
                    );
                }
            }
        }

        #[test]
        fn reference_points_lie_on_the_front() {
            let r = Zdt::One.reference(1001);
            assert_eq!(r.len(), 1001);
            assert_eq!(r[500], vec![0.5, 1.0 - 0.5f64.sqrt()]);
            let r3 = Zdt::Three.reference(1001);
            assert!(r3.iter().all(|f| Zdt::Three.distance_to_front(f) <= 1e-15));
            // The pieces' f₁ lengths sum to 0.2657, so about a quarter of the points.
            assert_eq!(r3.len(), 265);
        }
    }
}
