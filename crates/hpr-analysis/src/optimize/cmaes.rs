//! The covariance matrix adaptation evolution strategy (CMA-ES).
//!
//! Each generation draws `λ` candidates from a normal distribution about a mean `m`, with an
//! overall step size `σ` and a covariance `C` that sets the steps' shape. The model ranks them;
//! the mean moves to a weighted average of the best `μ = ⌊λ/2⌋`. The step size grows when
//! successive moves point the same way and shrinks when they cancel (*cumulative step-size
//! adaptation*), and the covariance learns the directions the good steps took (the *rank-one* and
//! *rank-μ* updates). Only the ranking matters, so the strategy is unchanged by any increasing
//! transformation of the output, and by any rotation of the variables once `C` has adapted.
//!
//! The algorithm and its default parameters are those of N. Hansen, "The CMA Evolution Strategy:
//! A Tutorial", arXiv:1604.00772v2 (2023), <https://arxiv.org/abs/1604.00772>: Appendix A's
//! summary (Figure 6, eqs. (38) to (47), pp. 28–29) with Table 1's default parameters (eqs. (48)
//! to (58), p. 31), and with the weights after the `μ`-th zero, as in the tutorial's own code
//! ("This code does not implement negative weights, that is, wᵢ = 0 for i > µ in Table 1",
//! p. 36): the original strategy, not the *active* one. Equations, for a generation `g` counted
//! from 0:
//!
//! ```text
//! yₖ = B D zₖ,  zₖ ~ N(0, I),  xₖ = m + σ yₖ                        (sampling, k = 1 … λ)
//! ⟨y⟩ = Σᵢ wᵢ yᵢ:λ                                                  (the μ best, ranked)
//! m ← m + c_m σ ⟨y⟩
//! p_σ ← (1 − c_σ) p_σ + √(c_σ (2 − c_σ) μ_eff) C^(−1/2) ⟨y⟩
//! σ ← σ exp((c_σ/d_σ) (‖p_σ‖/E‖N(0, I)‖ − 1))
//! h_σ = 1 if ‖p_σ‖/√(1 − (1 − c_σ)^(2(g+1))) < (1.4 + 2/(n + 1)) E‖N(0, I)‖, else 0
//! p_c ← (1 − c_c) p_c + h_σ √(c_c (2 − c_c) μ_eff) ⟨y⟩
//! C ← (1 + c₁ δ(h_σ) − c₁ − c_μ Σwⱼ) C + c₁ p_c p_cᵀ + c_μ Σᵢ wᵢ yᵢ:λ yᵢ:λᵀ
//! ```
//!
//! with `δ(h_σ) = (1 − h_σ) c_c (2 − c_c)` and `E‖N(0, I)‖ ≈ √n (1 − 1/(4n) + 1/(21n²))`.
//! `C = B D² Bᵀ` is decomposed every generation, by Jacobi's method; the tutorial allows putting
//! it off for up to `1/(10 n (c₁ + c_μ))` generations (B.2, p. 33), which is under one for up to
//! about 25 variables at the default population.
//!
//! # Starting point and scaling
//!
//! The run starts at each [`Variable`]'s start, with `σ = 1` and `C` diagonal, its entries the
//! squares of the variables' steps: "different search intervals ∆sᵢ for different variables can
//! be reflected by a different initialization of C, in that the diagonal elements of C obey
//! cᵢᵢ = (∆sᵢ)²" (the tutorial, Figure 6's footnote, p. 29). It is the tutorial's `C = I` in
//! variables divided by their steps, so a variable in metres and another in kilograms each start
//! with the steps the caller gave them. The tutorial adds that the steps "should not disagree by
//! several orders of magnitude"; give the variables units that keep them comparable.
//!
//! # Bounds
//!
//! A candidate outside its variables' bounds is drawn again, from its own stream, until it falls
//! inside: the second of the two methods the tutorial gives for a best point strictly inside the
//! feasible region ("re-sampling any infeasible solution x until it become feasible", B.5,
//! p. 34). It does not repair a candidate onto a bound, which the tutorial advises against. A generation that can't draw a candidate inside in [`MAX_DRAWS`] tries ends
//! the run ([`Stop::Bounds`]). A best point *on* a bound is reached only slowly this way; bounds
//! that bind are left to the constraint handling of a later increment.
//!
//! # Stopping
//!
//! A run stops at the first of: a target value reached ([`Stop::Target`]), the evaluations used
//! up ([`Stop::Evaluations`]), the distribution's spread and its evolution path below a tolerance
//! in every variable ([`Stop::TolX`]), the best values of the last `10 + ⌈30 n/λ⌉` generations
//! and every value of the last one all within a tolerance ([`Stop::TolFun`]), the covariance's
//! condition number above 10¹⁴ ([`Stop::Condition`]), or no candidate inside the bounds
//! ([`Stop::Bounds`]). These are the tutorial's TolX, TolFun and ConditionCov (B.3, pp. 33–34),
//! with its suggested 10⁻¹² for both tolerances, TolX's taken relative to the initial steps.

use serde::{Deserialize, Serialize};

use hpr_core::random::SeededRng;

use super::eigen::symmetric_eigen;
use super::{Variable, check_variables};
use crate::error::AnalysisError;

/// The most times one candidate is drawn again to fall inside the bounds.
pub const MAX_DRAWS: usize = 1000;

/// The largest population a run takes.
pub const MAX_POPULATION: usize = 1 << 16;

/// The covariance's largest condition number before a run stops: its axes' lengths then differ
/// by 10⁷, about where rounding in `f64` starts to blur the shortest.
pub const MAX_CONDITION: f64 = 1e14;

/// The optimizer's settings: the variables, the population, and when to stop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cmaes {
    variables: Vec<Variable>,
    population: usize,
    max_evaluations: usize,
    target: Option<f64>,
    tolerance_x: f64,
    tolerance_value: f64,
}

/// Why a run stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Stop {
    /// A value at or below the target was found.
    Target,
    /// The evaluations allowed were used up.
    Evaluations,
    /// The distribution's spread, and its evolution path, fell below the tolerance in every
    /// variable: the run has converged in the variables.
    TolX,
    /// The values stopped changing by more than the tolerance: converged in the output.
    TolFun,
    /// The covariance's condition number passed [`MAX_CONDITION`].
    Condition,
    /// No candidate of a generation fell inside the bounds in [`MAX_DRAWS`] tries.
    Bounds,
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
    /// How many generations it ran.
    pub generations: usize,
    /// The distribution's final mean.
    pub mean: Vec<f64>,
    /// The final step size, times the square root of the covariance's largest eigenvalue: the
    /// distribution's largest standard deviation, in the variables' units.
    pub spread: f64,
    /// Why it stopped.
    pub stop: Stop,
}

impl Cmaes {
    /// The optimizer over `variables`, with the tutorial's default population
    /// `λ = 4 + ⌊3 ln n⌋`, at most 10,000 evaluations, no target, and tolerances of 10⁻¹² (in
    /// the variables, as a fraction of each one's step; and in the output, in its own units).
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] for no variables, [`AnalysisError::Count`] for more than
    /// [`MAX_VARIABLES`](super::MAX_VARIABLES), and [`AnalysisError::DuplicateVariable`] for two
    /// with one name.
    pub fn new(variables: Vec<Variable>) -> Result<Self, AnalysisError> {
        check_variables(&variables)?;
        let n = variables.len() as f64;
        // Cast: 4 + ⌊3 ln n⌋ is at most 19 for n ≤ 200.
        let population = 4 + (3.0 * n.ln()).floor() as usize;
        Ok(Self {
            variables,
            population,
            max_evaluations: 10_000,
            target: None,
            tolerance_x: 1e-12,
            tolerance_value: 1e-12,
        })
    }

    /// The same, with `λ` candidates a generation. A larger population searches more widely,
    /// which helps on outputs with many local minima, at more evaluations a generation.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::TooFew`] for fewer than 2, [`AnalysisError::Count`] for more than
    /// [`MAX_POPULATION`].
    pub fn with_population(mut self, lambda: usize) -> Result<Self, AnalysisError> {
        if lambda < 2 {
            return Err(AnalysisError::TooFew {
                what: "population",
                count: lambda,
                minimum: 2,
            });
        }
        if lambda > MAX_POPULATION {
            return Err(AnalysisError::Count {
                what: "population",
                count: lambda,
                limit: MAX_POPULATION,
            });
        }
        self.population = lambda;
        Ok(self)
    }

    /// The same, stopping once `max` evaluations have been made (it finishes the generation, so
    /// it may make up to `λ − 1` more).
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
    /// [`AnalysisError::Domain`] for a NaN target.
    pub fn with_target(mut self, target: f64) -> Result<Self, AnalysisError> {
        if target.is_nan() {
            return Err(AnalysisError::Domain {
                what: "target",
                value: target,
            });
        }
        self.target = Some(target);
        Ok(self)
    }

    /// The same, with the tolerances that end a converged run: `x`, as a fraction of each
    /// variable's step, and `value`, in the output's units. Zero turns a test off.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a tolerance that is negative or not finite.
    pub fn with_tolerances(mut self, x: f64, value: f64) -> Result<Self, AnalysisError> {
        for (what, t) in [("x tolerance", x), ("value tolerance", value)] {
            if !(t.is_finite() && t >= 0.0) {
                return Err(AnalysisError::Domain { what, value: t });
            }
        }
        self.tolerance_x = x;
        self.tolerance_value = value;
        Ok(self)
    }

    /// The variables.
    pub fn variables(&self) -> &[Variable] {
        &self.variables
    }

    /// The population, `λ`.
    pub fn population(&self) -> usize {
        self.population
    }

    /// Starts a run from `seed`, with its first generation drawn. Evaluate the candidates, give
    /// their values to [`Run::tell`], and repeat until it returns why it stopped.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::OutOfBounds`] if a first candidate can't be drawn inside the bounds.
    pub fn start(&self, seed: u64) -> Result<Run, AnalysisError> {
        let mut run = Run::new(self, seed);
        if !run.draw() {
            return Err(AnalysisError::OutOfBounds { draws: MAX_DRAWS });
        }
        Ok(run)
    }

    /// Minimizes `model` from `seed`, evaluating each candidate in turn.
    ///
    /// # Errors
    ///
    /// [`Cmaes::start`]'s and [`Run::tell`]'s.
    pub fn minimize(
        &self,
        seed: u64,
        mut model: impl FnMut(&[f64]) -> f64,
    ) -> Result<Optimum, AnalysisError> {
        let mut run = self.start(seed)?;
        loop {
            let values: Vec<f64> = run.candidates().iter().map(|x| model(x)).collect();
            if let Some(optimum) = run.tell(&values)? {
                return Ok(optimum);
            }
        }
    }
}

/// The strategy's parameters for `n` variables and population `λ`: the tutorial's Table 1 with
/// the negative weights set to zero.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameters {
    /// `μ = ⌊λ/2⌋`, the candidates that move the mean.
    pub mu: usize,
    /// `wᵢ ∝ ln((λ + 1)/2) − ln i`, `i = 1 … μ`, summing to 1.
    pub weights: Vec<f64>,
    /// `μ_eff = 1/Σwᵢ²`, the selection's variance-effective size.
    pub mu_eff: f64,
    /// `c_σ = (μ_eff + 2)/(n + μ_eff + 5)`.
    pub c_sigma: f64,
    /// `d_σ = 1 + 2 max(0, √((μ_eff − 1)/(n + 1)) − 1) + c_σ`.
    pub d_sigma: f64,
    /// `c_c = (4 + μ_eff/n)/(n + 4 + 2 μ_eff/n)`.
    pub c_c: f64,
    /// `c₁ = 2/((n + 1.3)² + μ_eff)`.
    pub c_1: f64,
    /// `c_μ = min(1 − c₁, 2 (1/4 + μ_eff + 1/μ_eff − 2)/((n + 2)² + μ_eff))`.
    pub c_mu: f64,
    /// `E‖N(0, I)‖ ≈ √n (1 − 1/(4n) + 1/(21n²))`.
    pub chi_n: f64,
}

impl Parameters {
    /// The parameters for `n` variables and `lambda` candidates a generation.
    pub fn new(n: usize, lambda: usize) -> Self {
        let nf = n as f64;
        let mu = lambda / 2;
        let half = (lambda as f64 + 1.0) / 2.0;
        let raw: Vec<f64> = (1..=mu).map(|i| half.ln() - (i as f64).ln()).collect();
        let sum: f64 = raw.iter().sum();
        let weights: Vec<f64> = raw.iter().map(|w| w / sum).collect();
        let mu_eff = 1.0 / weights.iter().map(|w| w * w).sum::<f64>();
        let c_sigma = (mu_eff + 2.0) / (nf + mu_eff + 5.0);
        let d_sigma = 1.0 + 2.0 * (((mu_eff - 1.0) / (nf + 1.0)).sqrt() - 1.0).max(0.0) + c_sigma;
        let c_c = (4.0 + mu_eff / nf) / (nf + 4.0 + 2.0 * mu_eff / nf);
        // α_cov = 2.
        let c_1 = 2.0 / ((nf + 1.3).powi(2) + mu_eff);
        let c_mu = (1.0 - c_1).min(
            2.0 * (0.25 + mu_eff + 1.0 / mu_eff - 2.0) / ((nf + 2.0).powi(2) + 2.0 * mu_eff / 2.0),
        );
        let chi_n = nf.sqrt() * (1.0 - 1.0 / (4.0 * nf) + 1.0 / (21.0 * nf * nf));
        Self {
            mu,
            weights,
            mu_eff,
            c_sigma,
            d_sigma,
            c_c,
            c_1,
            c_mu,
            chi_n,
        }
    }
}

/// A run in progress: its distribution, the current generation's candidates, and the best point
/// so far.
#[derive(Debug, Clone)]
pub struct Run {
    variables: Vec<Variable>,
    seed: u64,
    lambda: usize,
    max_evaluations: usize,
    target: Option<f64>,
    tolerance_x: f64,
    tolerance_value: f64,
    p: Parameters,
    n: usize,
    mean: Vec<f64>,
    sigma: f64,
    p_sigma: Vec<f64>,
    p_c: Vec<f64>,
    /// `C`, row-major.
    c: Vec<f64>,
    /// `B`, the eigenvectors of `C` as columns, row-major.
    b: Vec<f64>,
    /// `D`, the square roots of `C`'s eigenvalues.
    d: Vec<f64>,
    generation: usize,
    evaluations: usize,
    candidates: Vec<Vec<f64>>,
    /// Each candidate's step `y = B D z`.
    steps: Vec<Vec<f64>>,
    best: Option<(Vec<f64>, f64, usize)>,
    /// The best value of each generation, oldest first.
    history: Vec<f64>,
    stop: Option<Stop>,
}

impl Run {
    fn new(cmaes: &Cmaes, seed: u64) -> Self {
        let n = cmaes.variables.len();
        let mut c = vec![0.0; n * n];
        let mut b = vec![0.0; n * n];
        for (i, v) in cmaes.variables.iter().enumerate() {
            c[i * n + i] = v.step() * v.step();
            b[i * n + i] = 1.0;
        }
        Self {
            seed,
            lambda: cmaes.population,
            max_evaluations: cmaes.max_evaluations,
            target: cmaes.target,
            tolerance_x: cmaes.tolerance_x,
            tolerance_value: cmaes.tolerance_value,
            p: Parameters::new(n, cmaes.population),
            n,
            mean: cmaes.variables.iter().map(Variable::start).collect(),
            sigma: 1.0,
            p_sigma: vec![0.0; n],
            p_c: vec![0.0; n],
            c,
            b,
            d: cmaes.variables.iter().map(Variable::step).collect(),
            generation: 0,
            evaluations: 0,
            candidates: Vec::new(),
            steps: Vec::new(),
            best: None,
            history: Vec::new(),
            stop: None,
            variables: cmaes.variables.clone(),
        }
    }

    /// The strategy's parameters.
    pub fn parameters(&self) -> &Parameters {
        &self.p
    }

    /// The current generation's candidates, `λ` points, each one value per variable. Empty once
    /// the run has stopped.
    pub fn candidates(&self) -> &[Vec<f64>] {
        &self.candidates
    }

    /// The generations finished so far.
    pub fn generation(&self) -> usize {
        self.generation
    }

    /// The distribution's mean.
    pub fn mean(&self) -> &[f64] {
        &self.mean
    }

    /// The step size `σ`.
    pub fn sigma(&self) -> f64 {
        self.sigma
    }

    /// The covariance `C`, row-major.
    pub fn covariance(&self) -> &[f64] {
        &self.c
    }

    /// Why the run stopped, or `None` while it runs.
    pub fn stopped(&self) -> Option<Stop> {
        self.stop
    }

    /// Takes the model's values at [`Run::candidates`], in their order, and moves the
    /// distribution. Returns what the run found once it stops, and `None` while it goes on, with
    /// the next generation's candidates drawn.
    ///
    /// A value may be `+∞`, for a candidate the model can't evaluate (a flight that fails, say):
    /// it ranks below every finite one.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Length`] for a number of values other than the candidates', and
    /// [`AnalysisError::Output`] for a NaN or `−∞`, with the index of its evaluation in the run.
    /// Telling a stopped run is [`AnalysisError::Length`] too, as it has no candidates.
    pub fn tell(&mut self, values: &[f64]) -> Result<Option<Optimum>, AnalysisError> {
        if values.len() != self.candidates.len() || self.candidates.is_empty() {
            return Err(AnalysisError::Length {
                what: "values, against the generation's candidates",
                length: values.len(),
                expected: self.candidates.len(),
            });
        }
        if let Some((k, &value)) = values
            .iter()
            .enumerate()
            .find(|(_, v)| v.is_nan() || **v == f64::NEG_INFINITY)
        {
            return Err(AnalysisError::Output {
                index: self.evaluations + k,
                value,
            });
        }
        // Rank: a stable sort, so ties keep the candidates' order.
        let mut order: Vec<usize> = (0..self.lambda).collect();
        order.sort_by(|&i, &j| values[i].total_cmp(&values[j]));
        let first = order[0];
        if self
            .best
            .as_ref()
            .is_none_or(|(_, v, _)| values[first] < *v)
        {
            self.best = Some((
                self.candidates[first].clone(),
                values[first],
                self.evaluations + first + 1,
            ));
        }
        self.evaluations += self.lambda;
        self.update(&order);
        self.generation += 1;
        self.history.push(values[first]);
        self.stop = self.check_stop(values);
        if self.stop.is_none() && !self.draw() {
            self.stop = Some(Stop::Bounds);
        }
        Ok(self.stop.map(|stop| self.optimum(stop)))
    }

    /// Moves the mean, the paths, the covariance and the step size: the tutorial's update.
    fn update(&mut self, order: &[usize]) {
        let n = self.n;
        let p = &self.p;
        let selected: Vec<&Vec<f64>> = order[..p.mu].iter().map(|&k| &self.steps[k]).collect();
        let mut y_w = vec![0.0; n];
        for (w, y) in p.weights.iter().zip(&selected) {
            for (a, yi) in y_w.iter_mut().zip(y.iter()) {
                *a += w * yi;
            }
        }
        // c_m = 1.
        for (m, y) in self.mean.iter_mut().zip(&y_w) {
            *m += self.sigma * y;
        }
        // C^(−1/2) ⟨y⟩ = B D⁻¹ Bᵀ ⟨y⟩.
        let bt_y: Vec<f64> = (0..n)
            .map(|k| (0..n).map(|i| self.b[i * n + k] * y_w[i]).sum::<f64>() / self.d[k])
            .collect();
        let whitened: Vec<f64> = (0..n)
            .map(|i| (0..n).map(|k| self.b[i * n + k] * bt_y[k]).sum())
            .collect();
        let cs = p.c_sigma;
        let scale_sigma = (cs * (2.0 - cs) * p.mu_eff).sqrt();
        for (ps, w) in self.p_sigma.iter_mut().zip(&whitened) {
            *ps = (1.0 - cs) * *ps + scale_sigma * w;
        }
        let norm_ps = self.p_sigma.iter().map(|x| x * x).sum::<f64>().sqrt();
        // The generation just finished is g + 1, counting from 1.
        let exponent = 2.0 * (self.generation as f64 + 1.0);
        let h_sigma = norm_ps / (1.0 - (1.0 - cs).powf(exponent)).sqrt()
            < (1.4 + 2.0 / (n as f64 + 1.0)) * p.chi_n;
        let cc = p.c_c;
        let scale_c = (cc * (2.0 - cc) * p.mu_eff).sqrt();
        for (pc, y) in self.p_c.iter_mut().zip(&y_w) {
            *pc = (1.0 - cc) * *pc + if h_sigma { scale_c * y } else { 0.0 };
        }
        let delta = if h_sigma { 0.0 } else { cc * (2.0 - cc) };
        // Σwⱼ = 1 with the negative weights zero.
        let keep = 1.0 + p.c_1 * delta - p.c_1 - p.c_mu;
        for i in 0..n {
            for j in 0..=i {
                let rank_mu: f64 = p
                    .weights
                    .iter()
                    .zip(&selected)
                    .map(|(w, y)| w * y[i] * y[j])
                    .sum();
                let value =
                    keep * self.c[i * n + j] + p.c_1 * self.p_c[i] * self.p_c[j] + p.c_mu * rank_mu;
                self.c[i * n + j] = value;
                self.c[j * n + i] = value;
            }
        }
        self.sigma *= ((cs / p.d_sigma) * (norm_ps / p.chi_n - 1.0)).exp();
        let (values, vectors) = symmetric_eigen(&self.c, n);
        self.b = vectors;
        self.d = values.iter().map(|v| v.max(0.0).sqrt()).collect();
    }

    /// Why the run should stop now, if it should.
    fn check_stop(&self, values: &[f64]) -> Option<Stop> {
        let n = self.n;
        if let (Some(target), Some((_, best, _))) = (self.target, &self.best)
            && *best <= target
        {
            return Some(Stop::Target);
        }
        if self.evaluations >= self.max_evaluations {
            return Some(Stop::Evaluations);
        }
        let d_max = self.d.iter().copied().fold(0.0, f64::max);
        let d_min = self.d.iter().copied().fold(f64::INFINITY, f64::min);
        // Written so that a NaN in D or σ stops the run too.
        if !(d_min > 0.0 && d_max / d_min <= MAX_CONDITION.sqrt() && self.sigma.is_finite()) {
            return Some(Stop::Condition);
        }
        let converged_x = (0..n).all(|i| {
            let tolerance = self.tolerance_x * self.variables[i].step();
            self.sigma * self.c[i * n + i].sqrt() < tolerance
                && self.sigma * self.p_c[i].abs() < tolerance
        });
        if converged_x {
            return Some(Stop::TolX);
        }
        // Cast: ⌈30 n/λ⌉ is small; n ≤ 200 and λ ≥ 2.
        let window = 10 + (30.0 * n as f64 / self.lambda as f64).ceil() as usize;
        if self.history.len() >= window {
            let recent = self.history[self.history.len() - window..]
                .iter()
                .chain(values);
            let (low, high) = recent.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| {
                (lo.min(v), hi.max(v))
            });
            if high - low < self.tolerance_value {
                return Some(Stop::TolFun);
            }
        }
        None
    }

    /// Draws the next generation's candidates, each from its own stream; `false` if one can't be
    /// drawn inside the bounds.
    fn draw(&mut self) -> bool {
        let n = self.n;
        self.candidates.clear();
        self.steps.clear();
        for k in 0..self.lambda {
            // Casts: a generation and a candidate's place are far below 2⁶⁴.
            let mut rng = SeededRng::for_stream(self.seed, &[self.generation as u64, k as u64]);
            let mut drawn = None;
            for _ in 0..MAX_DRAWS {
                let z: Vec<f64> = (0..n).map(|_| rng.standard_normal()).collect();
                let y: Vec<f64> = (0..n)
                    .map(|i| (0..n).map(|j| self.b[i * n + j] * self.d[j] * z[j]).sum())
                    .collect();
                let x: Vec<f64> = self
                    .mean
                    .iter()
                    .zip(&y)
                    .map(|(m, yi)| m + self.sigma * yi)
                    .collect();
                if x.iter().zip(&self.variables).all(|(xi, v)| v.contains(*xi)) {
                    drawn = Some((x, y));
                    break;
                }
            }
            let Some((x, y)) = drawn else {
                self.candidates.clear();
                self.steps.clear();
                return false;
            };
            self.candidates.push(x);
            self.steps.push(y);
        }
        true
    }

    /// What the run has found, stopped for `stop`.
    fn optimum(&mut self, stop: Stop) -> Optimum {
        self.candidates.clear();
        self.steps.clear();
        let d_max = self.d.iter().copied().fold(0.0, f64::max);
        // A run is only told after a generation is evaluated, so there is a best point.
        let (point, value, evaluation) = self.best.clone().unwrap_or_default();
        Optimum {
            point,
            value,
            evaluation,
            evaluations: self.evaluations,
            generations: self.generation,
            mean: self.mean.clone(),
            spread: self.sigma * d_max,
            stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::benchmark::sphere;

    fn variables(n: usize, start: f64, step: f64) -> Vec<Variable> {
        (0..n)
            .map(|i| Variable::new(format!("x{i}"), start, step).unwrap())
            .collect()
    }

    /// Table 1 at n = 10, λ = 10, worked by hand: μ = 5, w' = ln 5.5 − ln i.
    #[test]
    fn parameters_follow_table_1() {
        let p = Parameters::new(10, 10);
        assert_eq!(p.mu, 5);
        let raw: Vec<f64> = (1..=5).map(|i| 5.5f64.ln() - (i as f64).ln()).collect();
        let sum: f64 = raw.iter().sum();
        for (w, r) in p.weights.iter().zip(&raw) {
            assert!((w - r / sum).abs() <= 1e-16);
        }
        assert!((p.weights.iter().sum::<f64>() - 1.0).abs() <= 1e-15);
        let mu_eff = sum * sum / raw.iter().map(|r| r * r).sum::<f64>();
        assert!((p.mu_eff - mu_eff).abs() <= 1e-13);
        assert!((p.c_sigma - (mu_eff + 2.0) / (15.0 + mu_eff)).abs() <= 1e-16);
        assert!((p.c_1 - 2.0 / (11.3f64.powi(2) + mu_eff)).abs() <= 1e-16);
        // μ_eff < 11 here, so d_σ's max term is zero.
        assert!((p.d_sigma - (1.0 + p.c_sigma)).abs() <= 1e-16);
    }

    /// Ranking is all that matters: a run on `f` and on `exp(f)` take the same steps.
    #[test]
    fn invariant_under_an_increasing_transformation() {
        let cmaes = Cmaes::new(variables(4, 1.0, 0.5))
            .unwrap()
            .with_max_evaluations(400)
            .unwrap();
        let a = cmaes.minimize(3, sphere).unwrap();
        let b = cmaes.minimize(3, |x| sphere(x).sqrt().exp()).unwrap();
        assert_eq!(a.point, b.point);
        assert_eq!(a.mean, b.mean);
        assert_eq!(a.evaluations, b.evaluations);
    }

    /// The same seed gives the same run bit for bit; another seed another run.
    #[test]
    fn a_seed_fixes_the_run() {
        let cmaes = Cmaes::new(variables(3, 2.0, 1.0)).unwrap();
        let a = cmaes.minimize(11, sphere).unwrap();
        let b = cmaes.minimize(11, sphere).unwrap();
        let c = cmaes.minimize(12, sphere).unwrap();
        assert_eq!(a, b);
        assert_ne!(a.point, c.point);
    }

    /// Bounds hold every candidate, and a run whose best point is inside still finds it.
    #[test]
    fn bounds_hold_every_candidate() {
        let vars: Vec<Variable> = (0..3)
            .map(|i| {
                Variable::new(format!("x{i}"), 1.5, 1.0)
                    .unwrap()
                    .within(-0.5, 2.0)
                    .unwrap()
            })
            .collect();
        let cmaes = Cmaes::new(vars).unwrap().with_target(1e-12).unwrap();
        let mut run = cmaes.start(5).unwrap();
        let optimum = loop {
            for x in run.candidates() {
                assert!(x.iter().all(|xi| (-0.5..=2.0).contains(xi)), "{x:?}");
            }
            let values: Vec<f64> = run.candidates().iter().map(|x| sphere(x)).collect();
            if let Some(optimum) = run.tell(&values).unwrap() {
                break optimum;
            }
        };
        assert_eq!(optimum.stop, Stop::Target);
        assert!(optimum.value <= 1e-12);
    }

    /// Steps far larger than the bounds can't draw a first candidate: an error, not a hang.
    #[test]
    fn steps_too_large_for_the_bounds_are_refused() {
        let vars: Vec<Variable> = (0..4)
            .map(|i| {
                Variable::new(format!("x{i}"), 0.0, 1e6)
                    .unwrap()
                    .within(-1e-6, 1e-6)
                    .unwrap()
            })
            .collect();
        let error = Cmaes::new(vars).unwrap().start(1).unwrap_err();
        assert!(matches!(error, AnalysisError::OutOfBounds { draws } if draws == MAX_DRAWS));
    }

    /// A NaN is refused with its evaluation's index; `+∞` ranks last; a wrong count is refused.
    #[test]
    fn values_are_checked() {
        let cmaes = Cmaes::new(variables(2, 1.0, 0.5)).unwrap();
        let mut run = cmaes.start(1).unwrap();
        let lambda = run.candidates().len();
        assert_eq!(lambda, 6);
        let mut values = vec![1.0; lambda];
        values[4] = f64::NAN;
        assert!(matches!(
            run.tell(&values),
            Err(AnalysisError::Output { index: 4, .. })
        ));
        values[4] = f64::NEG_INFINITY;
        assert!(run.tell(&values).is_err());
        assert!(matches!(
            run.tell(&values[..3]),
            Err(AnalysisError::Length { length: 3, .. })
        ));
        values[4] = f64::INFINITY;
        assert!(run.tell(&values).unwrap().is_none());
        assert_eq!(run.generation(), 1);
        let values = vec![1.0; lambda];
        assert!(matches!(run.tell(&values), Ok(None)));
        // The second generation's evaluations are counted from the first's end.
        let mut values = vec![1.0; lambda];
        values[0] = f64::NAN;
        assert!(matches!(
            run.tell(&values),
            Err(AnalysisError::Output { index, .. }) if index == 2 * lambda
        ));
    }

    /// A flat output ends the run by the value tolerance; a run out of evaluations says so.
    #[test]
    fn stops_are_reported() {
        let cmaes = Cmaes::new(variables(2, 1.0, 0.5)).unwrap();
        let flat = cmaes.minimize(1, |_| 7.0).unwrap();
        assert_eq!(flat.stop, Stop::TolFun);
        // Window 10 + ⌈60/6⌉ = 20 generations of history, checked from the 20th.
        assert_eq!(flat.generations, 20);
        let short = cmaes
            .clone()
            .with_max_evaluations(30)
            .unwrap()
            .minimize(1, sphere)
            .unwrap();
        assert_eq!(short.stop, Stop::Evaluations);
        assert_eq!(short.evaluations, 30);
        let converged = cmaes
            .with_tolerances(1e-6, 0.0)
            .unwrap()
            .minimize(1, sphere)
            .unwrap();
        assert_eq!(converged.stop, Stop::TolX);
        assert!(converged.spread < 1e-6 * 0.5 * 10.0);
    }

    /// Settings are checked.
    #[test]
    fn settings_are_checked() {
        let cmaes = Cmaes::new(variables(2, 0.0, 1.0)).unwrap();
        assert_eq!(cmaes.population(), 6);
        assert!(cmaes.clone().with_population(1).is_err());
        assert!(cmaes.clone().with_population(MAX_POPULATION + 1).is_err());
        assert!(cmaes.clone().with_max_evaluations(0).is_err());
        assert!(cmaes.clone().with_target(f64::NAN).is_err());
        assert!(cmaes.clone().with_tolerances(-1.0, 0.0).is_err());
        assert!(cmaes.clone().with_tolerances(0.0, f64::INFINITY).is_err());
        assert!(Cmaes::new(Vec::new()).is_err());
    }
}
