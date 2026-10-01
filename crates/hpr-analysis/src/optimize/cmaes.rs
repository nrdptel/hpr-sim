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
//! about 85 variables at the default population.
//!
//! # Starting point and scaling
//!
//! The strategy works in each [`Variable`] divided by its step. There it starts as the tutorial's
//! Figure 6 does, with `σ = 1`, `C = I` and both paths zero, at the variables' starts. So a
//! variable in metres and another in kilograms each start with the steps the caller gave them,
//! and the condition number the run stops at is the tutorial's. This is the tutorial's advice for
//! variables whose search intervals differ: "a scaling of the variables should be applied"
//! (Figure 6's footnote, p. 29). [`Run::covariance`] is `C` in these scaled variables.
//!
//! # Bounds
//!
//! A candidate outside its variables' bounds is drawn again, from its own stream, until it falls
//! inside: the second of the two methods the tutorial gives for a best point strictly inside the
//! feasible region ("re-sampling any infeasible solution x until it become feasible", B.5,
//! p. 34). No candidate is repaired onto a bound, which the tutorial advises against. If any one
//! candidate of a generation is still outside after [`MAX_DRAWS`] tries, the run ends
//! ([`Stop::Bounds`]). The chance that a draw falls inside halves with each variable whose mean
//! sits on a bound, so with many variables near their bounds this comes soon. A best point *on* a
//! bound is reached only slowly this way; bounds that bind are left to the constraint handling of
//! a later increment.
//!
//! # Stopping
//!
//! A run stops at the first of: a target value reached ([`Stop::Target`]), the evaluations used
//! up ([`Stop::Evaluations`]), the distribution's spread and its evolution path below a tolerance
//! in every variable ([`Stop::TolX`]), the best values of the last `10 + ⌈30 n/λ⌉` generations
//! and every value of the last one all within a tolerance ([`Stop::TolFun`]), the covariance's
//! condition number above 10¹⁴ or a step or candidate that has overflowed ([`Stop::Condition`]), or a
//! candidate that can't be drawn inside the bounds ([`Stop::Bounds`]). These are the tutorial's
//! TolX, TolFun and ConditionCov (B.3, pp. 33–34), with its suggested 10⁻¹² for both tolerances,
//! TolX's taken in the scaled variables, so as a fraction of each one's step. Its NoEffectAxis,
//! NoEffectCoord, Stagnation and TolXUp tests are left out: a run that diverges ends at
//! [`Stop::Condition`], or at the evaluation cap.
//!
//! # A run with no finite value
//!
//! If every candidate so far has given `+∞` (every flight failed, say), the run goes on, ranking
//! them in their order, and its [`Optimum`]'s value is `+∞`. Check [`Optimum::value`] before
//! using the point.

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

/// The optimizer's settings: the variables, the population, and when to stop. It serializes as
/// its fields, and reads back through the same checks as [`Cmaes::new`] and its `with_` methods.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CmaesData")]
pub struct Cmaes {
    variables: Vec<Variable>,
    population: usize,
    max_evaluations: usize,
    target: Option<f64>,
    tolerance_x: f64,
    tolerance_value: f64,
}

/// The serialized form of a [`Cmaes`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CmaesData {
    variables: Vec<Variable>,
    population: usize,
    max_evaluations: usize,
    target: Option<f64>,
    tolerance_x: f64,
    tolerance_value: f64,
}

impl TryFrom<CmaesData> for Cmaes {
    type Error = AnalysisError;

    fn try_from(data: CmaesData) -> Result<Self, AnalysisError> {
        let cmaes = Cmaes::new(data.variables)?
            .with_population(data.population)?
            .with_max_evaluations(data.max_evaluations)?
            .with_tolerance_x(data.tolerance_x)?
            .with_tolerance_value(data.tolerance_value)?;
        match data.target {
            Some(target) => cmaes.with_target(target),
            None => Ok(cmaes),
        }
    }
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
    /// The covariance's condition number passed [`MAX_CONDITION`], or the step size or a
    /// candidate overflowed: the run has diverged, or its distribution has degenerated.
    Condition,
    /// A candidate of a generation didn't fall inside the bounds in [`MAX_DRAWS`] tries.
    Bounds,
}

/// What a run found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Optimum {
    /// The best point evaluated, one value per variable, in the variables' order.
    pub point: Vec<f64>,
    /// The model's value there: `+∞` if no candidate gave a finite value (serialized as none, a
    /// JSON `null`).
    #[serde(with = "infinity_as_none")]
    pub value: f64,
    /// Which evaluation found it, counted from 1.
    pub evaluation: usize,
    /// How many evaluations the run made.
    pub evaluations: usize,
    /// How many generations it ran.
    pub generations: usize,
    /// The distribution's final mean.
    pub mean: Vec<f64>,
    /// The distribution's largest standard deviation in any one variable, in that variable's
    /// units: the largest `σ √Cᵢᵢ` times the variable's step; `+∞` once the step size has
    /// overflowed (serialized as none, a JSON `null`).
    #[serde(with = "infinity_as_none")]
    pub spread: f64,
    /// Why it stopped.
    pub stop: Stop,
}

/// The serialized form of a number that is finite or `+∞`, the only infinity an [`Optimum`]
/// holds: an option, none for `+∞`, as JSON has no infinity.
mod infinity_as_none {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S: Serializer>(x: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        let value = (*x != f64::INFINITY).then_some(*x);
        value.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
        Ok(Option::<f64>::deserialize(deserializer)?.unwrap_or(f64::INFINITY))
    }
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
    /// [`AnalysisError::Domain`] for a target that isn't finite: an infinite one would be met by
    /// a generation whose every candidate failed.
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

    /// The same, ending a run once the distribution's spread, and its evolution path, are below
    /// `tolerance` times each variable's step in every variable ([`Stop::TolX`]). Zero turns the
    /// test off.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a tolerance that is negative or not finite.
    pub fn with_tolerance_x(mut self, tolerance: f64) -> Result<Self, AnalysisError> {
        self.tolerance_x = check_tolerance("x tolerance", tolerance)?;
        Ok(self)
    }

    /// The same, ending a run once its recent values all lie within `tolerance` of each other, in
    /// the output's units ([`Stop::TolFun`]). Zero turns the test off.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a tolerance that is negative or not finite.
    pub fn with_tolerance_value(mut self, tolerance: f64) -> Result<Self, AnalysisError> {
        self.tolerance_value = check_tolerance("value tolerance", tolerance)?;
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
    /// [`AnalysisError::OutOfBounds`] if a first candidate can't be drawn inside the bounds, and
    /// [`AnalysisError::Domain`] if one overflows (steps near `f64::MAX`).
    pub fn start(&self, seed: u64) -> Result<Run, AnalysisError> {
        let mut run = Run::new(self, seed);
        match run.draw() {
            None => Ok(run),
            Some(Stop::Bounds) => Err(AnalysisError::OutOfBounds { draws: MAX_DRAWS }),
            Some(_) => Err(AnalysisError::Domain {
                what: "first candidate (not finite: the steps are too large)",
                value: f64::INFINITY,
            }),
        }
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

/// A tolerance, checked to be finite and not negative.
fn check_tolerance(what: &'static str, tolerance: f64) -> Result<f64, AnalysisError> {
    if tolerance.is_finite() && tolerance >= 0.0 {
        Ok(tolerance)
    } else {
        Err(AnalysisError::Domain {
            what,
            value: tolerance,
        })
    }
}

/// The strategy's parameters for `n` variables and population `λ`: the tutorial's Table 1 with
/// the negative weights set to zero. [`Run::parameters`] gives a run's; they serialize, but
/// aren't read back, as nothing takes them in.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
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
    /// The parameters for `n ≥ 1` variables and `lambda ≥ 2` candidates a generation, as
    /// [`Cmaes`] checks them.
    pub(crate) fn new(n: usize, lambda: usize) -> Self {
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
    /// Each variable's step: the strategy works in the variables divided by these.
    scale: Vec<f64>,
    /// The mean, in the variables' own units.
    mean: Vec<f64>,
    sigma: f64,
    p_sigma: Vec<f64>,
    p_c: Vec<f64>,
    /// `C`, row-major, in the scaled variables.
    c: Vec<f64>,
    /// `B`, the eigenvectors of `C` as columns, row-major.
    b: Vec<f64>,
    /// `D`, the square roots of `C`'s eigenvalues.
    d: Vec<f64>,
    generation: usize,
    evaluations: usize,
    candidates: Vec<Vec<f64>>,
    /// Each candidate's step `y = B D z`, in the scaled variables.
    steps: Vec<Vec<f64>>,
    best: Option<(Vec<f64>, f64, usize)>,
    /// The best value of each generation, oldest first.
    history: Vec<f64>,
    stop: Option<Stop>,
}

impl Run {
    fn new(cmaes: &Cmaes, seed: u64) -> Self {
        let n = cmaes.variables.len();
        let mut identity = vec![0.0; n * n];
        for i in 0..n {
            identity[i * n + i] = 1.0;
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
            scale: cmaes.variables.iter().map(Variable::step).collect(),
            mean: cmaes.variables.iter().map(Variable::start).collect(),
            sigma: 1.0,
            p_sigma: vec![0.0; n],
            p_c: vec![0.0; n],
            c: identity.clone(),
            b: identity,
            d: vec![1.0; n],
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

    /// The covariance `C`, row-major, of the variables divided by their steps: the distribution
    /// of candidates about the mean has covariance `σ² S C S`, with `S` the steps on a diagonal.
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
        if self.stop.is_none() {
            self.stop = self.draw();
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
        // c_m = 1; the step is scaled back to the variables' units.
        for ((m, y), s) in self.mean.iter_mut().zip(&y_w).zip(&self.scale) {
            *m += self.sigma * s * y;
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
        // And in the variables' own units, where a step above 1 overflows first.
        let finite = (0..n).all(|i| {
            (self.sigma * self.c[i * n + i].sqrt() * self.scale[i]).is_finite()
                && self.mean[i].is_finite()
        });
        if !(d_min > 0.0
            && d_max / d_min <= MAX_CONDITION.sqrt()
            && (self.sigma * d_max).is_finite()
            && finite)
        {
            return Some(Stop::Condition);
        }
        // In the scaled variables, so the tolerance is a fraction of each variable's step.
        let converged_x = (0..n).all(|i| {
            self.sigma * self.c[i * n + i].sqrt() < self.tolerance_x
                && self.sigma * self.p_c[i].abs() < self.tolerance_x
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

    /// Draws the next generation's candidates, each from its own stream. Returns why the run must
    /// stop instead: [`Stop::Bounds`] if one can't be drawn inside the bounds, [`Stop::Condition`]
    /// if one overflows.
    fn draw(&mut self) -> Option<Stop> {
        let n = self.n;
        self.candidates.clear();
        self.steps.clear();
        // B D, once a generation.
        let mut bd = self.b.clone();
        for row in bd.chunks_exact_mut(n) {
            for (x, d) in row.iter_mut().zip(&self.d) {
                *x *= d;
            }
        }
        let mut z = vec![0.0; n];
        let mut y = vec![0.0; n];
        let mut x = vec![0.0; n];
        for k in 0..self.lambda {
            // Casts: a generation and a candidate's place are far below 2⁶⁴.
            let mut rng = SeededRng::for_stream(self.seed, &[self.generation as u64, k as u64]);
            let mut drawn = None;
            for _ in 0..MAX_DRAWS {
                z.iter_mut().for_each(|zi| *zi = rng.standard_normal());
                for (yi, row) in y.iter_mut().zip(bd.chunks_exact(n)) {
                    *yi = row.iter().zip(&z).map(|(a, b)| a * b).sum();
                }
                for i in 0..n {
                    x[i] = self.mean[i] + self.sigma * self.scale[i] * y[i];
                }
                if !x.iter().all(|xi| xi.is_finite()) {
                    drawn = Some(Stop::Condition);
                    break;
                }
                if x.iter().zip(&self.variables).all(|(xi, v)| v.contains(*xi)) {
                    drawn = None;
                    self.candidates.push(x.clone());
                    self.steps.push(y.clone());
                    break;
                }
                drawn = Some(Stop::Bounds);
            }
            if drawn.is_some() {
                self.candidates.clear();
                self.steps.clear();
                return drawn;
            }
        }
        None
    }

    /// What the run has found, stopped for `stop`.
    fn optimum(&mut self, stop: Stop) -> Optimum {
        self.candidates.clear();
        self.steps.clear();
        let n = self.n;
        // A NaN here can only come of an overflowed σ times a zero: count it as infinite, as
        // `f64::max` would drop it.
        let spread = (0..n)
            .map(|i| self.sigma * self.c[i * n + i].sqrt() * self.scale[i])
            .map(|t| if t.is_nan() { f64::INFINITY } else { t })
            .fold(0.0, f64::max);
        // A run is only told after a generation is evaluated, so there is a best point; the
        // default is never used.
        let (point, value, evaluation) = self.best.clone().unwrap_or_default();
        Optimum {
            point,
            value,
            evaluation,
            evaluations: self.evaluations,
            generations: self.generation,
            mean: self.mean.clone(),
            spread,
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
            .with_tolerance_x(1e-6)
            .unwrap()
            .with_tolerance_value(0.0)
            .unwrap()
            .minimize(1, sphere)
            .unwrap();
        assert_eq!(converged.stop, Stop::TolX);
        // σ √Cᵢᵢ below 10⁻⁶ in the scaled variables: 10⁻⁶ times the step, 0.5.
        assert!(converged.spread < 1e-6 * 0.5);
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
        assert!(cmaes.clone().with_target(f64::INFINITY).is_err());
        assert!(cmaes.clone().with_tolerance_x(-1.0).is_err());
        assert!(cmaes.clone().with_tolerance_value(f64::INFINITY).is_err());
        assert!(Cmaes::new(Vec::new()).is_err());
    }

    /// Settings read back through their checks: a round trip, and refusals.
    #[test]
    fn settings_serialize_and_are_checked_on_reading() {
        let cmaes = Cmaes::new(variables(2, 0.0, 1.0))
            .unwrap()
            .with_target(0.5)
            .unwrap();
        let json = serde_json::to_string(&cmaes).unwrap();
        assert_eq!(serde_json::from_str::<Cmaes>(&json).unwrap(), cmaes);
        for (field, bad) in [
            ("\"population\":6", "\"population\":1"),
            ("\"max_evaluations\":10000", "\"max_evaluations\":0"),
            ("\"tolerance_x\":1e-12", "\"tolerance_x\":-1.0"),
        ] {
            assert!(json.contains(field), "{json}");
            let edited = json.replace(field, bad);
            assert!(serde_json::from_str::<Cmaes>(&edited).is_err(), "{edited}");
        }
        let empty = json.replace(
            &json[json.find('[').unwrap()..=json.find(']').unwrap()],
            "[]",
        );
        assert!(serde_json::from_str::<Cmaes>(&empty).is_err(), "{empty}");
        let extra = json.replacen('{', "{\"extra\":1,", 1);
        assert!(serde_json::from_str::<Cmaes>(&extra).is_err());
    }

    /// The smallest populations, `λ` = 2 and 3 (`μ` = 1), still converge on the sphere.
    #[test]
    fn smallest_populations_run() {
        for lambda in [2, 3] {
            let optimum = Cmaes::new(variables(2, 1.0, 0.5))
                .unwrap()
                .with_population(lambda)
                .unwrap()
                .with_target(1e-10)
                .unwrap()
                .minimize(1, sphere)
                .unwrap();
            assert_eq!(optimum.stop, Stop::Target, "λ = {lambda}");
        }
    }

    /// A covariance stretched past 10¹⁴ ends the run: an ellipse whose curvatures differ by 10¹⁶.
    #[test]
    fn stops_at_the_condition_limit() {
        let optimum = Cmaes::new(variables(2, 1.0, 0.5))
            .unwrap()
            .with_tolerance_x(0.0)
            .unwrap()
            .with_tolerance_value(0.0)
            .unwrap()
            .minimize(1, |x| x[0] * x[0] + 1e16 * x[1] * x[1])
            .unwrap();
        assert_eq!(optimum.stop, Stop::Condition);
    }

    /// Steps 10⁸ apart, on a sphere scaled to match: the run works in the scaled variables, so it
    /// converges (a run on the unscaled covariance stopped at its condition limit after one
    /// generation), and the x tolerance is a fraction of each variable's own step.
    #[test]
    fn steps_far_apart_converge_alike() {
        let steps = [1e-4, 1e4];
        let vars = vec![
            Variable::new("small", steps[0], steps[0]).unwrap(),
            Variable::new("large", steps[1], steps[1]).unwrap(),
        ];
        let model = |x: &[f64]| (x[0] / 1e-4).powi(2) + (x[1] / 1e4).powi(2);
        let cmaes = Cmaes::new(vars)
            .unwrap()
            .with_tolerance_x(1e-6)
            .unwrap()
            .with_tolerance_value(0.0)
            .unwrap();
        let mut run = cmaes.start(1).unwrap();
        let optimum = loop {
            let values: Vec<f64> = run.candidates().iter().map(|x| model(x)).collect();
            if let Some(optimum) = run.tell(&values).unwrap() {
                break optimum;
            }
        };
        assert_eq!(optimum.stop, Stop::TolX);
        assert!(optimum.value < 1e-10, "{}", optimum.value);
        for i in 0..2 {
            let sd = run.sigma() * run.covariance()[i * 2 + i].sqrt();
            assert!(sd < 1e-6, "variable {i}: {sd} of its step");
        }
        assert!(optimum.spread < 1e-6 * steps[1]);
    }

    /// A run whose every value is `+∞` gives an infinity that reads back; so does an infinite
    /// spread. A run that diverges stops as `Condition`.
    #[test]
    fn infinite_results_read_back() {
        let cmaes = Cmaes::new(variables(1, 0.0, 1.0))
            .unwrap()
            .with_max_evaluations(50)
            .unwrap();
        let failed = cmaes.minimize(1, |_| f64::INFINITY).unwrap();
        assert_eq!(failed.value, f64::INFINITY);
        let json = serde_json::to_string(&failed).unwrap();
        assert!(json.contains("\"value\":null"), "{json}");
        assert_eq!(serde_json::from_str::<Optimum>(&json).unwrap(), failed);
        let overflowed = Optimum {
            spread: f64::INFINITY,
            ..failed.clone()
        };
        let json = serde_json::to_string(&overflowed).unwrap();
        assert!(json.contains("\"spread\":null"), "{json}");
        assert_eq!(serde_json::from_str::<Optimum>(&json).unwrap(), overflowed);
        // Unbounded below and a step above 1: a candidate overflows before σ does, and the run
        // says so as `Condition`, not `Bounds`.
        let vars = vec![Variable::new("x", 0.0, 1000.0).unwrap()];
        let diverged = Cmaes::new(vars)
            .unwrap()
            .with_max_evaluations(1_000_000)
            .unwrap()
            .minimize(7, |x| x[0])
            .unwrap();
        assert_eq!(diverged.stop, Stop::Condition);
    }

    /// Steps so large that a first candidate overflows are refused.
    #[test]
    fn overflowing_first_candidates_are_refused() {
        let vars = vec![Variable::new("x", 0.0, 1e308).unwrap()];
        let error = Cmaes::new(vars).unwrap().start(1).unwrap_err();
        assert!(matches!(
            error,
            AnalysisError::Domain { what, .. } if what.starts_with("first candidate")
        ));
    }
}
