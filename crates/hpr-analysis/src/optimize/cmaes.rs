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
//! bound is reached only slowly this way; for a bound that binds, leave the variable unbounded on
//! that side and write the bound as a constraint.
//!
//! # Integer variables
//!
//! An integer variable ([`Variable::integer`]) is drawn as a real number like the others, and the
//! model is given the whole number nearest the draw, clamped to its bounds
//! ([`Variable::encode`]); the update learns from the real draws. Left at that, the spread in an
//! integer variable would shrink until every draw gave the same whole number and that variable
//! stopped moving, wherever it was. CMA-ES with margin (R. Hamano, S. Saito, M. Nomura and
//! S. Shirakawa, "CMA-ES with Margin: Lower-Bounding Marginal Probability for Mixed-Integer
//! Black-Box Optimization", GECCO 2022, <https://arxiv.org/abs/2205.13482>, §4 and Algorithm 1,
//! pp. 5–6 and 10) prevents it: after each update it keeps at least a chance `α = 1/(n λ)` that a
//! draw lands on another value, by moving the mean towards a threshold or by stretching that
//! variable's draws by a factor `A` ([`Run::margin_scale`]):
//!
//! ```text
//! xₖ = m + σ S A yₖ                                    (S the steps, A diagonal, 1 if continuous)
//! s = σ A S √C_jj                                       (variable j's spread)
//! at an end value, threshold ℓ:  m ← ℓ + sign(m − ℓ) min(|m − ℓ|, Φ⁻¹(1 − α) s)      eq. (13)
//! inside, thresholds ℓ₋ < m ≤ ℓ₊:
//!   p₋ = Φ((ℓ₋ − m)/s),  p₊ = Φ((m − ℓ₊)/s),  p₀ = 1 − p₋ − p₊                  eqs. (17)–(19)
//!   p′ = max(α/2, p),  p″ = p′ + (1 − p′₋ − p′₊ − p₀)(p′ − α/2)/(p′₋ + p′₊ + p₀ − 3α/2)
//!   χ = Φ⁻¹(1 − p″):  m ← (ℓ₋ χ₊ + ℓ₊ χ₋)/(χ₋ + χ₊),  A ← (ℓ₊ − ℓ₋)/((χ₋ + χ₊) σ S √C_jj)   (24)
//! ```
//!
//! The thresholds lie halfway between neighbouring whole numbers; the mean, `σ` and `C` are the
//! updated ones and `A` the old one in `s`. The paths and `C` never see the correction. An
//! integer variable's draws are never redrawn for its bounds, which its encoding enforces. The
//! paper's α is the default here; its Figure 4 (p. 7) finds the method works across a range
//! about it. `Φ` is the standard normal distribution function.
//!
//! # Constraints
//!
//! Other constraints go through [`Run::tell_constrained`], which ranks candidates by Deb's
//! feasibility rules ([`Evaluation`]): any candidate that keeps every constraint ranks ahead of
//! any that doesn't, and those that don't rank by how far they break them. Infeasible candidates
//! are evaluated and ranked, not drawn again, so the distribution can sit across a constraint's
//! edge and close in on a minimum that lies on it.
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
//! using the point; under constraints, check [`Optimum::violation`] too, which is above zero if
//! no candidate kept them all.

use serde::{Deserialize, Serialize};

use hpr_core::random::SeededRng;

use super::eigen::symmetric_eigen;
use super::{Evaluation, Variable, check_variables, normal};
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
    /// The best point evaluated, one value per variable, in the variables' order, as the model
    /// was given it (an integer variable's value encoded).
    pub point: Vec<f64>,
    /// The model's value there: `+∞` if no candidate gave a finite value (serialized as none, a
    /// JSON `null`).
    #[serde(with = "infinity_as_none")]
    pub value: f64,
    /// Its constraint violation ([`Evaluation::violation`]): zero if it keeps every constraint,
    /// and always zero for a run told plain values; `+∞` if every candidate failed (serialized as
    /// none).
    #[serde(default, with = "infinity_as_none")]
    pub violation: f64,
    /// Which evaluation found it, counted from 1.
    pub evaluation: usize,
    /// How many evaluations the run made.
    pub evaluations: usize,
    /// How many generations it ran.
    pub generations: usize,
    /// The distribution's final mean, not encoded: an integer variable's may lie between whole
    /// numbers or outside its bounds ([`Variable::encode`] gives the value it stands for).
    pub mean: Vec<f64>,
    /// The distribution's largest standard deviation in any one variable, in that variable's
    /// units: the largest `σ √Cᵢᵢ` times the variable's step and its margin scale `Aᵢ`; `+∞`
    /// once the step size has overflowed (serialized as none, a JSON `null`). An integer variable
    /// at an inner value keeps it large enough for the margin (its `Aᵢ` grows), so then it
    /// doesn't shrink to zero; at an end value only the mean moves, and it may.
    #[serde(with = "infinity_as_none")]
    pub spread: f64,
    /// Why it stopped.
    pub stop: Stop,
}

/// The serialized form of a number that is finite or `+∞`, the only infinity an [`Optimum`]
/// holds: an option, none for `+∞`, as JSON has no infinity. A NaN or `−∞` is refused, as
/// it would read back as `+∞`.
pub(crate) mod infinity_as_none {
    use serde::ser::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(crate) fn serialize<S: Serializer>(x: &f64, serializer: S) -> Result<S::Ok, S::Error> {
        if x.is_nan() || *x == f64::NEG_INFINITY {
            return Err(S::Error::custom(format!("{x} is neither finite nor +∞")));
        }
        let value = (*x != f64::INFINITY).then_some(*x);
        value.serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
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

    /// Minimizes `model` under constraints from `seed`, evaluating each candidate in turn: the
    /// model gives each candidate's [`Evaluation`], and [`Run::tell_constrained`] ranks them.
    /// If no candidate keeps every constraint, the optimum's [`violation`](Optimum::violation)
    /// is above zero: check it before using the point.
    ///
    /// # Errors
    ///
    /// [`Cmaes::start`]'s and [`Run::tell_constrained`]'s.
    pub fn minimize_constrained(
        &self,
        seed: u64,
        mut model: impl FnMut(&[f64]) -> Evaluation,
    ) -> Result<Optimum, AnalysisError> {
        let mut run = self.start(seed)?;
        loop {
            let evaluations: Vec<Evaluation> = run.candidates().iter().map(|x| model(x)).collect();
            if let Some(optimum) = run.tell_constrained(&evaluations)? {
                return Ok(optimum);
            }
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
    /// The margin's diagonal `A`: each candidate's draw is `m + σ S A y`. 1 for a continuous
    /// variable, always.
    a: Vec<f64>,
    /// The margin `α = 1/(n λ)`: the least chance an integer variable's draw has of falling on a
    /// value next to its mean's.
    alpha: f64,
    generation: usize,
    evaluations: usize,
    candidates: Vec<Vec<f64>>,
    /// Each candidate's step `y = B D z`, in the scaled variables.
    steps: Vec<Vec<f64>>,
    best: Option<(Vec<f64>, Evaluation, usize)>,
    /// The best evaluation of each generation, oldest first.
    history: Vec<Evaluation>,
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
            a: vec![1.0; n],
            // Casts: n ≤ 200 and λ ≤ 2¹⁶, exact in f64.
            alpha: 1.0 / (n as f64 * cmaes.population as f64),
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

    /// The current generation's candidates, `λ` points, each one value per variable, as the model
    /// is to be given them: an integer variable's value is encoded, a whole number within its
    /// bounds. Empty once the run has stopped.
    pub fn candidates(&self) -> &[Vec<f64>] {
        &self.candidates
    }

    /// The generations finished so far.
    pub fn generation(&self) -> usize {
        self.generation
    }

    /// The distribution's mean. An integer variable's is a real number, not encoded, and may lie
    /// outside its bounds.
    pub fn mean(&self) -> &[f64] {
        &self.mean
    }

    /// The step size `σ`.
    pub fn sigma(&self) -> f64 {
        self.sigma
    }

    /// The covariance `C`, row-major, of the variables divided by their steps: the distribution
    /// of draws about the mean has covariance `σ² S A C A S`, with `S` the steps and `A`
    /// [`Run::margin_scale`] on diagonals.
    pub fn covariance(&self) -> &[f64] {
        &self.c
    }

    /// The margin's diagonal `A`, one entry per variable: an integer variable's draws spread
    /// `A` times as far as `σ` and `C` alone would spread them ([`Variable::integer`]). 1 for
    /// every continuous variable.
    pub fn margin_scale(&self) -> &[f64] {
        &self.a
    }

    /// The margin `α = 1/(n λ)`: the least chance, each generation, that an integer variable's
    /// draw falls on a value other than the one its mean is nearest (at least `α/2` on each side
    /// when the mean's value has neighbours on both).
    pub fn integer_margin(&self) -> f64 {
        self.alpha
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
        let evaluations: Vec<Evaluation> =
            values.iter().map(|&v| Evaluation::feasible(v)).collect();
        self.tell_constrained(&evaluations)
    }

    /// [`Run::tell`] with each candidate's constraint violation: candidates are ranked by Deb's
    /// feasibility rules ([`Evaluation::rank`]), the best point is the best by the same rules,
    /// and a target counts only for a point that keeps every constraint. The stop on values
    /// ([`Stop::TolFun`]) looks only at generations whose best keeps them, and at the values of
    /// this generation's candidates that keep them.
    ///
    /// Told only violations of zero, a run is bit for bit [`Run::tell`]'s.
    ///
    /// # Errors
    ///
    /// [`Run::tell`]'s, and [`AnalysisError::Domain`] for a violation that is negative or NaN
    /// (`+∞` is a failed candidate's, [`Evaluation::failed`]).
    pub fn tell_constrained(
        &mut self,
        evaluations: &[Evaluation],
    ) -> Result<Option<Optimum>, AnalysisError> {
        let values: Vec<f64> = evaluations.iter().map(|e| e.value).collect();
        if values.len() != self.candidates.len() || self.candidates.is_empty() {
            return Err(AnalysisError::Length {
                what: "values, against the generation's candidates",
                length: values.len(),
                expected: self.candidates.len(),
            });
        }
        if let Some(e) = evaluations
            .iter()
            .find(|e| e.violation.is_nan() || e.violation < 0.0)
        {
            return Err(AnalysisError::Domain {
                what: "constraint violation (must not be negative or NaN)",
                value: e.violation,
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
        order.sort_by(|&i, &j| evaluations[i].rank(&evaluations[j]));
        let first = order[0];
        if self
            .best
            .as_ref()
            .is_none_or(|(_, e, _)| evaluations[first].beats(e))
        {
            self.best = Some((
                self.candidates[first].clone(),
                evaluations[first],
                self.evaluations + first + 1,
            ));
        }
        self.evaluations += self.lambda;
        self.update(&order);
        self.generation += 1;
        self.history.push(evaluations[first]);
        let feasible: Vec<f64> = evaluations
            .iter()
            .filter(|e| e.is_feasible())
            .map(|e| e.value)
            .collect();
        self.stop = self.check_stop(&feasible);
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
        self.correct_margin();
    }

    /// The margin of CMA-ES with margin (R. Hamano, S. Saito, M. Nomura, S. Shirakawa, GECCO
    /// 2022, arXiv:2205.13482, §4.2 to 4.4, eqs. (12) to (24), pp. 5–6, and Algorithm 1, p. 10):
    /// for each integer variable, moves the mean and stretches `A` so that a draw keeps at least
    /// a chance `α` of leaving the mean's value, after the update of `m`, `σ` and `C`, and before
    /// the next draw. The paths and `C` never see it.
    fn correct_margin(&mut self) {
        let n = self.n;
        let alpha = self.alpha;
        for j in 0..n {
            let variable = &self.variables[j];
            if !variable.is_integer() {
                continue;
            }
            // The draw's spread in this variable, in its own units.
            let spread_unit = self.sigma * self.scale[j] * self.c[j * n + j].sqrt();
            let spread = self.a[j] * spread_unit;
            if !(spread > 0.0 && spread.is_finite()) {
                // σ or C has collapsed or overflowed: the run stops at `Stop::Condition`.
                continue;
            }
            let m = self.mean[j];
            // The thresholds sit halfway between neighbouring values.
            let first = variable.low() + 0.5;
            let last = variable.high() - 0.5;
            if m <= first || m > last {
                // At an end value (and always for two values), eq. (13): keep the mean within
                // Φ⁻¹(1 − α) spreads of the threshold, so the far side keeps a chance α. A is
                // unchanged, eq. (14).
                let threshold = if m <= first { first } else { last };
                // `−Φ⁻¹(α)`: `1 − α` would round first.
                let reach = -normal::quantile(alpha) * spread;
                if (m - threshold).abs() <= reach {
                    continue;
                }
                let mut mean = threshold + reach.copysign(m - threshold);
                // The sum rounds; never land farther from the threshold than `reach`.
                if (mean - threshold).abs() > reach {
                    mean = if mean > threshold {
                        mean.next_down()
                    } else {
                        mean.next_up()
                    };
                }
                self.mean[j] = mean;
                continue;
            }
            // Inside, eqs. (15) to (24): the thresholds either side of the mean.
            let value = super::nearest_whole(m);
            let (low, up) = (value - 0.5, value + 0.5);
            let p_low = normal::cdf((low - m) / spread);
            let p_up = normal::cdf((m - up) / spread);
            let half = alpha / 2.0;
            if p_low >= half && p_up >= half {
                // Eqs. (20) to (24) would give back the same mean and A.
                continue;
            }
            let p_mid = 1.0 - p_low - p_up;
            // Eqs. (20) to (23): each side at least α/2, the excess taken back in proportion.
            let (q_low, q_up) = (p_low.max(half), p_up.max(half));
            let excess = 1.0 - q_low - q_up - p_mid;
            let share = q_low + q_up + p_mid - 3.0 * half;
            let r_low = q_low + excess * (q_low - half) / share;
            let r_up = q_up + excess * (q_up - half) / share;
            // Eq. (24): the mean and A that give the sides these chances.
            // `Φ⁻¹(1 − r) = −Φ⁻¹(r)`, without rounding `1 − r` first.
            let chi_low = -normal::quantile(r_low);
            let chi_up = -normal::quantile(r_up);
            let mean = (low * chi_up + up * chi_low) / (chi_low + chi_up);
            let a = (up - low) / ((chi_low + chi_up) * spread_unit);
            if mean.is_finite() && a.is_finite() && a > 0.0 {
                self.mean[j] = mean;
                self.a[j] = a;
            }
        }
    }

    /// Why the run should stop now, if it should; `values` are this generation's feasible ones.
    fn check_stop(&self, values: &[f64]) -> Option<Stop> {
        let n = self.n;
        if let (Some(target), Some((_, best, _))) = (self.target, &self.best)
            && best.is_feasible()
            && best.value <= target
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
            (self.sigma * self.c[i * n + i].sqrt() * self.scale[i] * self.a[i]).is_finite()
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
        if self.history.len() >= window && !values.is_empty() {
            let bests = &self.history[self.history.len() - window..];
            if !bests.iter().all(Evaluation::is_feasible) {
                return None;
            }
            let recent = bests.iter().map(|e| &e.value).chain(values);
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
                    x[i] = self.mean[i] + self.sigma * self.scale[i] * self.a[i] * y[i];
                }
                if !x.iter().all(|xi| xi.is_finite()) {
                    drawn = Some(Stop::Condition);
                    break;
                }
                // An integer variable's draw is never outside: its encoding clamps it.
                if x.iter()
                    .zip(&self.variables)
                    .all(|(xi, v)| v.is_integer() || v.contains(*xi))
                {
                    drawn = None;
                    self.candidates.push(
                        x.iter()
                            .zip(&self.variables)
                            .map(|(xi, v)| v.encode(*xi))
                            .collect(),
                    );
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
            .map(|i| self.sigma * self.c[i * n + i].sqrt() * self.scale[i] * self.a[i])
            .map(|t| if t.is_nan() { f64::INFINITY } else { t })
            .fold(0.0, f64::max);
        // A run is only told after a generation is evaluated, so there is a best point; the
        // default is never used.
        let (point, best, evaluation) =
            self.best
                .clone()
                .unwrap_or((Vec::new(), Evaluation::feasible(f64::INFINITY), 0));
        Optimum {
            point,
            value: best.value,
            violation: best.violation,
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
    use crate::optimize::benchmark::constrained::sphere_above;
    use crate::optimize::benchmark::sphere;

    /// Told violations of zero, a constrained run is the plain one, bit for bit. (`tell` goes
    /// through `tell_constrained`; `tests/optimize.rs` pins that path to pycma and to a dense
    /// recomputation.)
    #[test]
    fn zero_violations_change_nothing() {
        let cmaes = Cmaes::new(variables(5, 1.0, 0.5)).unwrap();
        let plain = cmaes.minimize(7, sphere).unwrap();
        let constrained = cmaes
            .minimize_constrained(7, |x| Evaluation::feasible(sphere(x)))
            .unwrap();
        assert_eq!(plain, constrained);
    }

    #[test]
    fn violations_must_not_be_negative_or_nan() {
        let cmaes = Cmaes::new(variables(2, 1.0, 0.5)).unwrap();
        for bad in [-1e-300, f64::NAN, f64::NEG_INFINITY] {
            let mut run = cmaes.start(1).unwrap();
            let told: Vec<Evaluation> = run
                .candidates()
                .iter()
                .map(|_| Evaluation {
                    value: 1.0,
                    violation: bad,
                })
                .collect();
            match run.tell_constrained(&told) {
                Err(AnalysisError::Domain { what, value }) => {
                    assert!(what.starts_with("constraint violation"));
                    assert!(value.total_cmp(&bad).is_eq());
                }
                other => panic!("{bad}: {other:?}"),
            }
        }
    }

    /// A NaN or −∞ isn't written as JSON's `null`, which reads back as a failure's `+∞`.
    #[test]
    fn json_refuses_what_would_read_back_as_a_failure() {
        for bad in [f64::NAN, f64::NEG_INFINITY] {
            let e = Evaluation {
                value: 1.0,
                violation: bad,
            };
            assert!(serde_json::to_string(&e).is_err(), "{bad}");
            assert!(
                serde_json::to_string(&Evaluation::feasible(bad)).is_err(),
                "{bad}"
            );
        }
    }

    /// Failed candidates rank last: a run where every one fails reports `+∞` for both, and
    /// survives JSON; a run where some fail finds the minimum among the rest.
    #[test]
    fn failed_candidates_rank_last() {
        let cmaes = Cmaes::new(variables(3, 3.0, 1.0))
            .unwrap()
            .with_max_evaluations(300)
            .unwrap();
        let none = cmaes
            .minimize_constrained(5, |_| Evaluation::failed())
            .unwrap();
        assert_eq!((none.value, none.violation), (f64::INFINITY, f64::INFINITY));
        let json = serde_json::to_string(&none).unwrap();
        assert_eq!(serde_json::from_str::<Optimum>(&json).unwrap(), none);
        // Every candidate with x₁ < 0 fails; the minimum (1, 0, 0) is on that edge.
        let cmaes = Cmaes::new(variables(3, 3.0, 1.0))
            .unwrap()
            .with_max_evaluations(20_000)
            .unwrap();
        let some = cmaes
            .minimize_constrained(5, |x| {
                if x[1] < 0.0 {
                    Evaluation::failed()
                } else {
                    sphere_above(x)
                }
            })
            .unwrap();
        assert_eq!(some.violation, 0.0);
        assert!(some.point[1] >= 0.0);
        assert!((some.value - 1.0).abs() < 1e-8, "{}", some.value);
    }

    /// A target counts only for a point that keeps the constraints: an infeasible candidate
    /// far below the target doesn't stop the run.
    #[test]
    fn a_target_needs_a_feasible_point() {
        let cmaes = Cmaes::new(variables(3, 0.0, 1.0))
            .unwrap()
            .with_target(1.5)
            .unwrap();
        let optimum = cmaes.minimize_constrained(3, sphere_above).unwrap();
        assert_eq!(optimum.stop, Stop::Target);
        assert_eq!(optimum.violation, 0.0);
        assert!(optimum.value <= 1.5 && optimum.point[0] >= 1.0);
    }

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

    /// After every generation, each integer variable's next draw keeps the margin: at least `α`
    /// of falling past the threshold nearest an end value's mean (eq. (13)), and at least `α/2`
    /// on each side of an inner value (eqs. (20), (21)), with the spread `σ A s √C_jj` the draw
    /// uses. The run mixes an end-bound binary, an integer from −10 to 10 driven to an end, one
    /// driven to the middle, and two continuous variables.
    #[test]
    fn integer_draws_keep_the_margin() {
        let variables = vec![
            Variable::new("x0", 1.0, 0.5).unwrap(),
            Variable::new("x1", -2.0, 0.5).unwrap(),
            Variable::new("binary", 0.5, 1.0)
                .unwrap()
                .within(0.0, 1.0)
                .unwrap()
                .integer()
                .unwrap(),
            Variable::new("to an end", 3.0, 1.0)
                .unwrap()
                .within(-10.0, 10.0)
                .unwrap()
                .integer()
                .unwrap(),
            Variable::new("inner", -4.0, 2.0)
                .unwrap()
                .within(-10.0, 10.0)
                .unwrap()
                .integer()
                .unwrap(),
        ];
        let f = |x: &[f64]| {
            x[0] * x[0] + x[1] * x[1] + (1.0 - x[2]) + (x[3] - 10.0).abs() + (x[4] - 2.0).powi(2)
        };
        let mut run = Cmaes::new(variables).unwrap().start(3).unwrap();
        let n = 5;
        let alpha = run.integer_margin();
        assert_eq!(alpha, 1.0 / (5.0 * 8.0));
        let mut generations = 0;
        let mut corrected = 0;
        loop {
            let values: Vec<f64> = run.candidates().iter().map(|x| f(x)).collect();
            for x in run.candidates() {
                assert!(x[2] == 0.0 || x[2] == 1.0);
                assert!(x[3].fract() == 0.0 && (-10.0..=10.0).contains(&x[3]));
            }
            let done = run.tell(&values).unwrap();
            if done.is_some() {
                break;
            }
            generations += 1;
            for (j, (low, high)) in [(2, (0.0, 1.0)), (3, (-10.0, 10.0)), (4, (-10.0, 10.0))] {
                let step = run.variables[j].step();
                let spread =
                    run.sigma() * run.margin_scale()[j] * step * run.covariance()[j * n + j].sqrt();
                let m = run.mean()[j];
                let (first, last) = (low + 0.5, high - 0.5);
                // Φ and Φ⁻¹ are good to a few parts in 10¹⁶, and a relative error `δ` in a
                // tail's `x` moves the tail by about `x² δ` of itself; `x` here is under 3.
                let floor = |p: f64| p * (1.0 - 1e-13);
                if m <= first || m > last {
                    let threshold = if m <= first { first } else { last };
                    let far = normal::cdf(-(m - threshold).abs() / spread);
                    assert!(
                        far >= floor(alpha),
                        "gen {generations} x{j}: {far} < {alpha}"
                    );
                } else {
                    let value = crate::optimize::nearest_whole(m);
                    let below = normal::cdf((value - 0.5 - m) / spread);
                    let above = normal::cdf((m - value - 0.5) / spread);
                    assert!(
                        below >= floor(alpha / 2.0),
                        "gen {generations} x{j}: {below}"
                    );
                    assert!(
                        above >= floor(alpha / 2.0),
                        "gen {generations} x{j}: {above}"
                    );
                }
                if run.margin_scale()[j] != 1.0 {
                    corrected += 1;
                }
            }
            assert_eq!(&run.margin_scale()[..2], &[1.0, 1.0]);
        }
        // The run went long enough for the margin to bind: σ shrank and A grew.
        assert!(generations > 50, "{generations}");
        assert!(corrected > 0);
    }

    /// The correction at a set state against the paper's equations evaluated in 40-digit
    /// arithmetic (mpmath's `ncdf` and `erfinv`), with `n = 2`, `λ = 6`, so `α = 1/12`: an inner
    /// mean whose lower side has less than `α/2` and upper side more, an end-value mean too far
    /// from its threshold, and an inner mean with both sides above `α/2`, which is left alone.
    #[test]
    #[expect(
        clippy::excessive_precision,
        reason = "17 digits of each 40-digit reference, which round to the nearest f64"
    )]
    fn margin_correction_matches_the_equations() {
        let variables = vec![
            Variable::new("x", 0.0, 1.0).unwrap(),
            Variable::new("k", 0.0, 1.0)
                .unwrap()
                .within(-10.0, 10.0)
                .unwrap()
                .integer()
                .unwrap(),
        ];
        let fresh = || {
            let mut run = Cmaes::new(variables.clone()).unwrap().start(1).unwrap();
            run.c = vec![1.0, 0.0, 0.0, 1.0];
            run
        };
        assert_eq!(fresh().integer_margin(), 1.0 / 12.0);
        let close = |got: f64, want: f64| (got - want).abs() <= 1e-14 * want.abs();

        // Inside, eqs. (17) to (24): spread 2 × 0.125, `P(below 1.5) = 6.9e-4`,
        // `P(above 2.5) = 0.212`; the lower side is lifted to α/2 and the upper gives way.
        let mut run = fresh();
        (run.mean[1], run.sigma, run.a[1]) = (2.3, 0.125, 2.0);
        run.correct_margin();
        assert!(
            close(run.mean[1], 2.176_899_187_754_804_1),
            "{}",
            run.mean[1]
        );
        assert!(close(run.a[1], 3.127_161_079_343_547_5), "{}", run.a[1]);
        assert_eq!((run.mean[0], run.a[0]), (0.0, 1.0));

        // At the low end, eq. (13): the mean is drawn to `Φ⁻¹(1 − α)` spreads of −9.5.
        let mut run = fresh();
        (run.mean[1], run.sigma) = (-9.8, 0.1);
        run.correct_margin();
        assert!(
            close(run.mean[1], -9.638_299_412_710_063_8),
            "{}",
            run.mean[1]
        );
        assert_eq!(run.a[1], 1.0);

        // An end mean never lands farther from its threshold than the reach, though
        // `threshold ± reach` rounds outward for some spreads: those are stepped back an ulp.
        let reach_per_spread = -normal::quantile(1.0 / 12.0);
        let mut stepped = 0;
        for k in 0..2000 {
            let sigma = 0.01 + 1e-4 * f64::from(k);
            for (m, threshold) in [(-9.9, -9.5), (9.9, 9.5)] {
                let mut run = fresh();
                (run.mean[1], run.sigma) = (m, sigma);
                run.correct_margin();
                let reach = reach_per_spread * sigma;
                let gap = (run.mean[1] - threshold).abs();
                assert!(gap <= reach, "σ = {sigma}: {gap} > {reach}");
                assert!(reach - gap <= 4.0 * f64::EPSILON * 10.0, "σ = {sigma}");
                if (threshold + reach.copysign(m - threshold) - threshold).abs() > reach {
                    stepped += 1;
                }
            }
        }
        assert!(stepped > 0, "no spread rounded outward");

        // A mean just above −0.5 stands for 0: its lower side is lifted, and it stays above.
        let mut run = fresh();
        (run.mean[1], run.sigma) = (-0.499_999_999_999_999_94, 0.05);
        run.correct_margin();
        assert!(run.mean[1] > -0.5 && run.a[1] > 1.0, "{}", run.mean[1]);

        // Both sides above α/2, or an end mean already near its threshold: nothing moves.
        for (m, sigma) in [(2.3, 1.0), (-9.6, 0.1)] {
            let mut run = fresh();
            (run.mean[1], run.sigma) = (m, sigma);
            run.correct_margin();
            assert_eq!((run.mean[1], run.a[1]), (m, 1.0));
        }
    }

    /// An integer variable's draws are encoded before the model sees them; the best point is
    /// encoded too, and the run ends at the integer minimum exactly.
    #[test]
    fn integer_runs_reach_whole_minima() {
        let variables = vec![
            Variable::new("x", 2.5, 1.0).unwrap(),
            Variable::new("k", 2.3, 1.0)
                .unwrap()
                .within(-10.0, 10.0)
                .unwrap()
                .integer()
                .unwrap(),
        ];
        let optimum = Cmaes::new(variables)
            .unwrap()
            .with_target(1e-12)
            .unwrap()
            .minimize(5, |x| x[0] * x[0] + (x[1] - 3.0).powi(2))
            .unwrap();
        assert_eq!(optimum.stop, Stop::Target);
        assert_eq!(optimum.point[1], 3.0);
    }
}
