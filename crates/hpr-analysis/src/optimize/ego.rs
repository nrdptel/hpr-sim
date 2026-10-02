//! Efficient global optimization (EGO): minimizing a model that is slow to evaluate in few
//! evaluations, by fitting a surrogate to the points evaluated so far and evaluating next where
//! the surrogate expects the most improvement.
//!
//! The method is D. R. Jones, M. Schonlau and W. J. Welch, "Efficient global optimization of
//! expensive black-box functions", *Journal of Global Optimization* 13, 455–492 (1998),
//! <https://doi.org/10.1023/A:1008306431147>:
//!
//! 1. Evaluate the model at an initial design that spreads over the variables' box: a Latin
//!    hypercube (each variable's range cut into `n` equal slices, one point in each), the most
//!    spread out (largest least distance between two points) of [`DESIGNS`] drawn. Its size is
//!    10 points per variable by default, the rule of J. L. Loeppky, J. Sacks and W. J. Welch,
//!    "Choosing the sample size of a computer experiment: a practical guide", *Technometrics*
//!    51(4), 366–376 (2009), <https://doi.org/10.1198/TECH.2009.08040>.
//! 2. Fit a *kriging* surrogate (a Gaussian process): the model's values are taken as a
//!    constant mean `μ` plus a correlated deviation of variance `σ²`, the correlation of two
//!    points `x`, `x'` being `exp(−Σₖ θₖ (xₖ − x'ₖ)²)` with the variables scaled to `[0, 1]`
//!    (Jones et al. eq. (1) with `pₖ = 2`). `μ`, `σ²` and each `θₖ` are those of most
//!    likelihood (eqs. (2)–(4)): `μ` and `σ²` in closed form, the `θₖ` by [`cmaes`](super::cmaes)
//!    over `log₁₀ θₖ` in `[−3, 3]`.
//! 3. The surrogate predicts the model at a point, `ŷ`, with a standard error `s` (eqs. (7),
//!    (9)), zero at an evaluated point and growing away from them. The *expected improvement*
//!    over the best value so far `f_min` is (eq. (15))
//!
//!    `E[I] = (f_min − ŷ) Φ((f_min − ŷ)/s) + s φ((f_min − ŷ)/s)`,
//!
//!    `Φ` and `φ` the standard normal distribution and density. It is large where `ŷ` is low
//!    (exploiting what is known) and where `s` is large (exploring what isn't).
//! 4. Evaluate the model where the expected improvement is largest, found by CMA-ES started
//!    from the best of [`SEARCH_POINTS`] random points per variable and from the best point so
//!    far, refit, and repeat until the budget is spent, the target is met, or the largest
//!    expected improvement falls below a tolerance.
//!
//! The values are standardized (their mean taken off, divided by their standard deviation)
//! before fitting, which changes neither the predictions nor where the improvement is largest.
//! The correlation matrix gets [`NUGGET`] added to its diagonal so its Cholesky factorization
//! stays defined as points crowd together near a minimum.
//!
//! # Reproducibility
//!
//! A run's random numbers (the designs, the search's points, and each CMA-ES run's seed) come
//! from streams keyed by the seed and the evaluation's number
//! ([`SeededRng::for_stream`]), so a run is bit for bit the same every time on one platform.

use hpr_core::random::SeededRng;
use serde::{Deserialize, Serialize};

use super::cmaes::Cmaes;
use super::{MAX_VARIABLES, Variable, normal};
use crate::error::AnalysisError;

/// How many Latin hypercube designs are drawn for the initial design; the most spread out is
/// kept.
pub const DESIGNS: usize = 100;

/// How many random points per variable the search for the largest expected improvement starts
/// from.
pub const SEARCH_POINTS: usize = 200;

/// Added to the correlation matrix's diagonal, in units of `σ²`: about the square of
/// `10⁻⁴`, the smallest relative scatter the surrogate is allowed to see between two values.
pub const NUGGET: f64 = 1e-8;

/// The range of `log₁₀ θₖ` the likelihood is maximized over: correlation lengths from about
/// 0.03 to 30 times a variable's range.
const LOG_THETA: (f64, f64) = (-3.0, 3.0);

/// The optimizer's settings: the variables, the initial design's size, and when to stop.
///
/// Branin's function from seed 1, in 50 evaluations, the initial design's 20 included:
///
/// ```
/// use hpr_analysis::optimize::Variable;
/// use hpr_analysis::optimize::benchmark::global::{BRANIN_MINIMUM, branin};
/// use hpr_analysis::optimize::ego::Ego;
///
/// let variables = vec![
///     Variable::new("x0", 2.5, 3.0)?.within(-5.0, 10.0)?,
///     Variable::new("x1", 7.5, 3.0)?.within(0.0, 15.0)?,
/// ];
/// let optimum = Ego::new(variables)?.with_max_evaluations(50)?.minimize(1, branin)?;
/// assert_eq!(optimum.evaluations, 50);
/// assert!(optimum.value < 1.01 * BRANIN_MINIMUM);
/// # Ok::<(), hpr_analysis::AnalysisError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ego {
    variables: Vec<Variable>,
    initial: usize,
    max_evaluations: usize,
    target: Option<f64>,
    tolerance_improvement: f64,
}

/// Why a run stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Stop {
    /// A value at or below the target was found.
    Target,
    /// The evaluations allowed were used up.
    Evaluations,
    /// The largest expected improvement fell below the tolerance.
    Improvement,
}

/// What a run found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Optimum {
    /// The best point evaluated, one value per variable, in the variables' order.
    pub point: Vec<f64>,
    /// The model's value there.
    pub value: f64,
    /// Which evaluation found it, counted from 1.
    pub evaluation: usize,
    /// How many evaluations the run made.
    pub evaluations: usize,
    /// Why it stopped.
    pub stop: Stop,
}

impl Ego {
    /// The optimizer for `variables`, each with two finite bounds and continuous: EGO searches
    /// the box they make, so a variable's start and step aren't used. The initial design has 10
    /// points per variable, and a run stops after 20 evaluations per variable.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] for no variables, [`AnalysisError::Count`] for more than
    /// [`MAX_VARIABLES`], and [`AnalysisError::Domain`] for a variable with an infinite bound or
    /// an integer one.
    pub fn new(variables: Vec<Variable>) -> Result<Self, AnalysisError> {
        if variables.is_empty() {
            return Err(AnalysisError::TooFew {
                what: "variables",
                count: 0,
                minimum: 1,
            });
        }
        if variables.len() > MAX_VARIABLES {
            return Err(AnalysisError::Count {
                what: "variables",
                count: variables.len(),
                limit: MAX_VARIABLES,
            });
        }
        for v in &variables {
            if !(v.low.is_finite() && v.high.is_finite()) {
                return Err(AnalysisError::Domain {
                    what: "EGO variable's bounds (both finite)",
                    value: if v.low.is_finite() { v.high } else { v.low },
                });
            }
            if v.integer {
                return Err(AnalysisError::Domain {
                    what: "EGO variable (continuous only)",
                    value: v.low,
                });
            }
        }
        let n = variables.len();
        Ok(Self {
            variables,
            initial: 10 * n,
            max_evaluations: 20 * n,
            target: None,
            tolerance_improvement: 0.0,
        })
    }

    /// The same, with an initial design of `points` points.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] for fewer than 2, as the likelihood needs a spread of values.
    pub fn with_initial(mut self, points: usize) -> Result<Self, AnalysisError> {
        if points < 2 {
            return Err(AnalysisError::TooFew {
                what: "initial design's points",
                count: points,
                minimum: 2,
            });
        }
        self.initial = points;
        Ok(self)
    }

    /// The same, stopping once `max` evaluations have been made, the initial design's included.
    /// A budget smaller than the initial design evaluates the design whole and stops.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] for none.
    pub fn with_max_evaluations(mut self, max: usize) -> Result<Self, AnalysisError> {
        if max == 0 {
            return Err(AnalysisError::TooFew {
                what: "evaluations",
                count: 0,
                minimum: 1,
            });
        }
        self.max_evaluations = max;
        Ok(self)
    }

    /// The same, stopping once a value at or below `target` is found.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a target that isn't finite.
    pub fn with_target(mut self, target: f64) -> Result<Self, AnalysisError> {
        if !target.is_finite() {
            return Err(AnalysisError::Domain {
                what: "target (finite)",
                value: target,
            });
        }
        self.target = Some(target);
        Ok(self)
    }

    /// The same, stopping once the largest expected improvement found is below `tolerance`, in
    /// the model's units. Jones et al. stop at 1% of the best value's size. Zero, the default,
    /// turns the test off.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a tolerance that is negative or not finite.
    pub fn with_tolerance_improvement(mut self, tolerance: f64) -> Result<Self, AnalysisError> {
        if !(tolerance.is_finite() && tolerance >= 0.0) {
            return Err(AnalysisError::Domain {
                what: "improvement tolerance",
                value: tolerance,
            });
        }
        self.tolerance_improvement = tolerance;
        Ok(self)
    }

    /// The variables.
    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    /// Minimizes `model` from `seed`, evaluating one point at a time.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Output`] for a value that isn't finite (the surrogate can't take it),
    /// with the evaluation's index counted from 0, and [`cmaes`](super::cmaes)'s errors from the
    /// fits and searches.
    pub fn minimize(
        &self,
        seed: u64,
        mut model: impl FnMut(&[f64]) -> f64,
    ) -> Result<Optimum, AnalysisError> {
        let n = self.variables.len();
        let mut points: Vec<Vec<f64>> = Vec::new();
        let mut values: Vec<f64> = Vec::new();
        let mut best: Option<usize> = None;
        let mut evaluate = |unit: Vec<f64>,
                            points: &mut Vec<Vec<f64>>,
                            values: &mut Vec<f64>,
                            best: &mut Option<usize>|
         -> Result<(), AnalysisError> {
            let value = model(&self.to_variables(&unit));
            if !value.is_finite() {
                return Err(AnalysisError::Output {
                    index: values.len(),
                    value,
                });
            }
            if best.is_none_or(|b| value < values[b]) {
                *best = Some(values.len());
            }
            points.push(unit);
            values.push(value);
            Ok(())
        };
        let finish = |points: &[Vec<f64>], values: &[f64], best: usize, stop: Stop| Optimum {
            point: self.to_variables(&points[best]),
            value: values[best],
            evaluation: best + 1,
            evaluations: values.len(),
            stop,
        };

        let mut rng = SeededRng::for_stream(seed, &[0]);
        for unit in latin_hypercube(self.initial, n, &mut rng) {
            evaluate(unit, &mut points, &mut values, &mut best)?;
            let b = best.unwrap_or(0);
            if self.target.is_some_and(|t| values[b] <= t) {
                return Ok(finish(&points, &values, b, Stop::Target));
            }
            if values.len() >= self.max_evaluations {
                return Ok(finish(&points, &values, b, Stop::Evaluations));
            }
        }

        let mut log_theta = vec![0.0; n];
        loop {
            // Invariant: the initial design has at least 2 points, all evaluated.
            let b = best.unwrap_or(0);
            let mut rng = SeededRng::for_stream(seed, &[1, values.len() as u64]);
            let (mean, sd) = mean_and_sd(&values);
            let standard: Vec<f64> = values.iter().map(|v| (v - mean) / sd).collect();
            log_theta = fit_log_theta(&points, &standard, &log_theta, rng.next_u64())?;
            let theta: Vec<f64> = log_theta.iter().map(|l| 10f64.powf(*l)).collect();
            let Some(kriging) = Kriging::fit(&points, &standard, &theta) else {
                // No factorization even at the nugget: points coincide beyond its help.
                return Ok(finish(&points, &values, b, Stop::Improvement));
            };
            let f_min = standard[b];
            let (next, improvement) = kriging.most_improving(f_min, &points[b], &mut rng)?;
            if improvement * sd < self.tolerance_improvement {
                return Ok(finish(&points, &values, b, Stop::Improvement));
            }
            evaluate(next, &mut points, &mut values, &mut best)?;
            let b = best.unwrap_or(0);
            if self.target.is_some_and(|t| values[b] <= t) {
                return Ok(finish(&points, &values, b, Stop::Target));
            }
            if values.len() >= self.max_evaluations {
                return Ok(finish(&points, &values, b, Stop::Evaluations));
            }
        }
    }

    /// A point of the unit box in the variables' units.
    fn to_variables(&self, unit: &[f64]) -> Vec<f64> {
        self.variables
            .iter()
            .zip(unit)
            .map(|(v, u)| (v.low + u * (v.high - v.low)).clamp(v.low, v.high))
            .collect()
    }
}

/// The values' mean and standard deviation (1 if they are all equal).
fn mean_and_sd(values: &[f64]) -> (f64, f64) {
    // Cast: a count of evaluations.
    let count = values.len() as f64;
    let mean = values.iter().sum::<f64>() / count;
    let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / count;
    let sd = variance.sqrt();
    (mean, if sd > 0.0 { sd } else { 1.0 })
}

/// `points` points of a Latin hypercube in the unit box of `n` variables: each variable's range
/// cut into `points` equal slices and each slice holding one point, at a uniform place in it.
/// The design with the largest least distance between two points of [`DESIGNS`] drawn.
fn latin_hypercube(points: usize, n: usize, rng: &mut SeededRng) -> Vec<Vec<f64>> {
    let mut best: (f64, Vec<Vec<f64>>) = (f64::NEG_INFINITY, Vec::new());
    for _ in 0..DESIGNS {
        let mut design = vec![vec![0.0; n]; points];
        for k in 0..n {
            // A Fisher–Yates shuffle of the slices.
            let mut slices: Vec<usize> = (0..points).collect();
            for i in (1..points).rev() {
                // Cast: `uniform() < 1`, so the product is below `i + 1`.
                let j = (rng.uniform() * (i + 1) as f64) as usize;
                slices.swap(i, j);
            }
            for (row, slice) in design.iter_mut().zip(slices) {
                // Cast: counts of points.
                row[k] = (slice as f64 + rng.uniform()) / points as f64;
            }
        }
        let mut least = f64::INFINITY;
        for i in 0..points {
            for j in 0..i {
                least = least.min(squared_distance(&design[i], &design[j]));
            }
        }
        if least > best.0 {
            best = (least, design);
        }
    }
    best.1
}

/// `Σ (aₖ − bₖ)²`.
fn squared_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

/// The `log₁₀ θₖ` of most likelihood, found by CMA-ES started from `start`.
fn fit_log_theta(
    points: &[Vec<f64>],
    values: &[f64],
    start: &[f64],
    seed: u64,
) -> Result<Vec<f64>, AnalysisError> {
    let variables = start
        .iter()
        .enumerate()
        .map(|(k, s)| {
            Variable::new(format!("log10 theta {k}"), *s, 1.0)?.within(LOG_THETA.0, LOG_THETA.1)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let n = start.len();
    let optimum = Cmaes::new(variables)?
        .with_max_evaluations(100 * n)?
        .with_tolerance_x(1e-3)?
        .with_tolerance_value(1e-6)?
        .minimize(seed, |log_theta| {
            let theta: Vec<f64> = log_theta.iter().map(|l| 10f64.powf(*l)).collect();
            Kriging::fit(points, values, &theta).map_or(f64::INFINITY, |k| -k.log_likelihood)
        })?;
    // A fit that failed everywhere keeps the start.
    Ok(if optimum.value.is_finite() {
        optimum.point
    } else {
        start.to_vec()
    })
}

/// A kriging surrogate fitted to points of the unit box and their (standardized) values.
#[derive(Debug, Clone)]
struct Kriging<'a> {
    points: &'a [Vec<f64>],
    theta: &'a [f64],
    /// The Cholesky factor `L` of the correlation matrix `R` (nugget included), row by row.
    factor: Vec<f64>,
    /// `μ̂ = 1ᵀR⁻¹y / 1ᵀR⁻¹1`.
    mu: f64,
    /// `σ̂² = (y − 1μ̂)ᵀ R⁻¹ (y − 1μ̂) / n`.
    sigma2: f64,
    /// `R⁻¹ (y − 1μ̂)`.
    weights: Vec<f64>,
    /// `R⁻¹ 1`.
    inverse_ones: Vec<f64>,
    /// `1ᵀ R⁻¹ 1`.
    ones_inverse_ones: f64,
    /// The concentrated log-likelihood, `−(n/2) ln σ̂² − ½ ln |R|` (Jones et al. eq. (4)
    /// without its constant).
    log_likelihood: f64,
}

impl<'a> Kriging<'a> {
    /// The surrogate with correlation parameters `theta`, or `None` if the correlation matrix
    /// can't be factorized or the fit isn't finite.
    fn fit(points: &'a [Vec<f64>], values: &[f64], theta: &'a [f64]) -> Option<Self> {
        let m = points.len();
        let mut factor = vec![0.0; m * m];
        for i in 0..m {
            for j in 0..i {
                factor[i * m + j] = correlation(&points[i], &points[j], theta);
            }
            factor[i * m + i] = 1.0 + NUGGET;
        }
        cholesky(&mut factor, m)?;
        let inverse_ones = solve(&factor, m, &vec![1.0; m]);
        let ones_inverse_ones: f64 = inverse_ones.iter().sum();
        let inverse_values = solve(&factor, m, values);
        let mu = inverse_values.iter().sum::<f64>() / ones_inverse_ones;
        let weights: Vec<f64> = inverse_values
            .iter()
            .zip(&inverse_ones)
            .map(|(v, o)| v - mu * o)
            .collect();
        // Cast: a count of points.
        let sigma2 = values
            .iter()
            .zip(&weights)
            .map(|(y, w)| (y - mu) * w)
            .sum::<f64>()
            / m as f64;
        let log_determinant: f64 = (0..m).map(|i| 2.0 * factor[i * m + i].ln()).sum();
        // Cast: a count of points.
        let log_likelihood = -0.5 * m as f64 * sigma2.ln() - 0.5 * log_determinant;
        (sigma2 > 0.0 && log_likelihood.is_finite()).then_some(Self {
            points,
            theta,
            factor,
            mu,
            sigma2,
            weights,
            inverse_ones,
            ones_inverse_ones,
            log_likelihood,
        })
    }

    /// The prediction `ŷ` at `x` and its standard error `s` (Jones et al. eqs. (7) and (9)):
    /// `ŷ = μ̂ + rᵀR⁻¹(y − 1μ̂)` and
    /// `s² = σ̂² (1 − rᵀR⁻¹r + (1 − 1ᵀR⁻¹r)²/(1ᵀR⁻¹1))`, `r` the correlations of `x` with the
    /// points.
    fn predict(&self, x: &[f64]) -> (f64, f64) {
        let m = self.points.len();
        let r: Vec<f64> = self
            .points
            .iter()
            .map(|p| correlation(x, p, self.theta))
            .collect();
        let prediction = self.mu + dot(&r, &self.weights);
        let half = forward(&self.factor, m, &r);
        let ones = 1.0 - dot(&self.inverse_ones, &r);
        let variance =
            self.sigma2 * (1.0 - dot(&half, &half) + ones * ones / self.ones_inverse_ones);
        (prediction, variance.max(0.0).sqrt())
    }

    /// The expected improvement at `x` over `f_min`.
    fn expected_improvement(&self, x: &[f64], f_min: f64) -> f64 {
        let (prediction, error) = self.predict(x);
        expected_improvement(f_min - prediction, error)
    }

    /// The point of the unit box with the largest expected improvement over `f_min`, and that
    /// improvement: CMA-ES from the best of [`SEARCH_POINTS`] per variable random points and
    /// from `best`, the best point so far, the larger kept.
    fn most_improving(
        &self,
        f_min: f64,
        best: &[f64],
        rng: &mut SeededRng,
    ) -> Result<(Vec<f64>, f64), AnalysisError> {
        let n = best.len();
        let mut start = (f64::NEG_INFINITY, best.to_vec());
        for _ in 0..SEARCH_POINTS * n {
            let x: Vec<f64> = (0..n).map(|_| rng.uniform()).collect();
            let improvement = self.expected_improvement(&x, f_min);
            if improvement > start.0 {
                start = (improvement, x);
            }
        }
        let mut found = (f64::NEG_INFINITY, best.to_vec());
        for from in [start.1, best.to_vec()] {
            let variables = from
                .iter()
                .enumerate()
                .map(|(k, s)| Variable::new(format!("x{k}"), *s, 0.1)?.within(0.0, 1.0))
                .collect::<Result<Vec<_>, _>>()?;
            let optimum = Cmaes::new(variables)?
                .with_max_evaluations(200 * n)?
                .with_tolerance_x(1e-9)?
                .minimize(rng.next_u64(), |x| -self.expected_improvement(x, f_min))?;
            if -optimum.value > found.0 {
                found = (-optimum.value, optimum.point);
            }
        }
        Ok((found.1, found.0))
    }
}

/// `E[I] = d Φ(d/s) + s φ(d/s)` for an improvement `d = f_min − ŷ` expected with standard error
/// `s`; `max(d, 0)` where `s` is zero.
fn expected_improvement(d: f64, s: f64) -> f64 {
    if s > 0.0 {
        let z = d / s;
        let density = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
        (d * normal::cdf(z) + s * density).max(0.0)
    } else {
        d.max(0.0)
    }
}

/// `exp(−Σₖ θₖ (aₖ − bₖ)²)`.
fn correlation(a: &[f64], b: &[f64], theta: &[f64]) -> f64 {
    let exponent: f64 = a
        .iter()
        .zip(b)
        .zip(theta)
        .map(|((x, y), t)| t * (x - y) * (x - y))
        .sum();
    (-exponent).exp()
}

/// `Σ aᵢ bᵢ`.
fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Factorizes the symmetric matrix whose lower triangle `a` holds (`m × m`, row by row) as
/// `L Lᵀ`, leaving `L` in the lower triangle and zeros above; `None` if it isn't positive
/// definite.
fn cholesky(a: &mut [f64], m: usize) -> Option<()> {
    for j in 0..m {
        let mut diagonal = a[j * m + j];
        for k in 0..j {
            diagonal -= a[j * m + k] * a[j * m + k];
        }
        if diagonal.is_nan() || diagonal <= 0.0 {
            return None;
        }
        let pivot = diagonal.sqrt();
        a[j * m + j] = pivot;
        for i in j + 1..m {
            let mut sum = a[i * m + j];
            for k in 0..j {
                sum -= a[i * m + k] * a[j * m + k];
            }
            a[i * m + j] = sum / pivot;
        }
        for k in j + 1..m {
            a[j * m + k] = 0.0;
        }
    }
    Some(())
}

/// `L⁻¹ b`, `L` lower triangular.
fn forward(factor: &[f64], m: usize, b: &[f64]) -> Vec<f64> {
    let mut x = b.to_vec();
    for i in 0..m {
        for k in 0..i {
            x[i] -= factor[i * m + k] * x[k];
        }
        x[i] /= factor[i * m + i];
    }
    x
}

/// `(L Lᵀ)⁻¹ b`.
fn solve(factor: &[f64], m: usize, b: &[f64]) -> Vec<f64> {
    let mut x = forward(factor, m, b);
    for i in (0..m).rev() {
        for k in i + 1..m {
            x[i] -= factor[k * m + i] * x[k];
        }
        x[i] /= factor[i * m + i];
    }
    x
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests stop at the failure")]

    use super::*;

    /// The surrogate goes through its points (to the nugget's size) with no error there.
    #[test]
    fn kriging_interpolates() {
        let points: Vec<Vec<f64>> = (0..8)
            .map(|i| vec![f64::from(i) / 7.0, (f64::from(i) * 0.37).fract()])
            .collect();
        let values: Vec<f64> = points.iter().map(|p| (3.0 * p[0]).sin() + p[1]).collect();
        let theta = [2.0, 3.0];
        let kriging = Kriging::fit(&points, &values, &theta).unwrap();
        for (p, v) in points.iter().zip(&values) {
            let (prediction, error) = kriging.predict(p);
            assert!((prediction - v).abs() < 1e-6, "{prediction} against {v}");
            assert!(error < 1e-3, "{error}");
        }
        let (_, between) = kriging.predict(&[0.5, 0.9]);
        assert!(between > 1e-3, "{between}");
    }

    /// The likelihood is the closed form on two points, where `R = [[1 + δ, ρ], [ρ, 1 + δ]]`.
    #[test]
    fn likelihood_two_points() {
        let points = vec![vec![0.0], vec![0.5]];
        let values = [1.0, -1.0];
        let theta = [2.0];
        let kriging = Kriging::fit(&points, &values, &theta).unwrap();
        let rho = (-0.5f64).exp();
        let a = 1.0 + NUGGET;
        // `R⁻¹ = [[a, −ρ], [−ρ, a]]/(a² − ρ²)`: `μ̂ = 0` by symmetry, `σ̂² = yᵀR⁻¹y/2`.
        let determinant = a * a - rho * rho;
        let sigma2 = (2.0 * a + 2.0 * rho) / determinant / 2.0;
        assert!(kriging.mu.abs() < 1e-15);
        assert!((kriging.sigma2 - sigma2).abs() < 1e-14 * sigma2);
        let expected = -sigma2.ln() - 0.5 * determinant.ln();
        assert!((kriging.log_likelihood - expected).abs() < 1e-14);
    }

    /// `E[I]` at `d = 0` is `s φ(0)`; far below, `d`; far above, nearly 0.
    #[test]
    fn expected_improvement_values() {
        let s = 0.3;
        let at_zero = expected_improvement(0.0, s);
        assert!((at_zero - s / (2.0 * std::f64::consts::PI).sqrt()).abs() < 1e-16);
        assert!((expected_improvement(5.0, s) - 5.0).abs() < 1e-12);
        assert!(expected_improvement(-5.0, s) < 1e-30);
        assert_eq!(expected_improvement(0.25, 0.0), 0.25);
        assert_eq!(expected_improvement(-0.25, 0.0), 0.0);
    }

    /// Each variable's slices hold one point each.
    #[test]
    fn latin_hypercube_fills_every_slice() {
        let mut rng = SeededRng::for_stream(7, &[0]);
        let design = latin_hypercube(12, 3, &mut rng);
        for k in 0..3 {
            // Cast: within [0, 12).
            let mut slices: Vec<usize> = design.iter().map(|p| (p[k] * 12.0) as usize).collect();
            slices.sort_unstable();
            assert_eq!(slices, (0..12).collect::<Vec<_>>());
        }
    }

    /// Unbounded and integer variables are refused.
    #[test]
    fn refuses_unbounded_and_integer() {
        let open = Variable::new("x", 0.0, 1.0).unwrap();
        assert!(matches!(
            Ego::new(vec![open.clone()]),
            Err(AnalysisError::Domain { what, .. }) if what.starts_with("EGO variable's bounds")
        ));
        let whole = open.within(0.0, 4.0).unwrap().integer().unwrap();
        assert!(matches!(
            Ego::new(vec![whole]),
            Err(AnalysisError::Domain { what, .. }) if what.starts_with("EGO variable (continuous")
        ));
    }
}
