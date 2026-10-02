//! Optimization: the design variables that make a model's output as small as it can be.
//!
//! **Guide:** [Optimization][guide] runs the optimizer on test functions whose minima are known,
//! then finds the ballast and body length that send a rocket to 3,048 m, chooses a motor and a
//! catalogue nose cone for it, traces a rocket's trade-off between apogee and stability, and says
//! how far to trust it.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/optimization.html
//!
//! - [`Variable`]: one number the optimizer may change, with where it starts, the size of its
//!   first steps, and optional bounds. An integer variable ([`Variable::integer`]) takes whole
//!   numbers only: a count, or a choice from a list, such as a motor or a catalogue part.
//! - [`cmaes`]: the covariance matrix adaptation evolution strategy (CMA-ES), which samples
//!   candidates around a mean, keeps the better half, and learns from them which way, and how
//!   far, to step next. It needs only the output's ranking, no derivatives, so it suits flights,
//!   whose outputs are noisy in their last digits.
//! - [`nsga2`]: NSGA-II, a genetic algorithm for two or more goals at once (apogee against
//!   stability, say), which finds the *Pareto front*: the designs where one goal can only be
//!   bettered by giving up another.
//! - [`Evaluation`]: a value and a constraint violation, for a model with constraints, ranked
//!   by Deb's feasibility rules ([`cmaes::Run::tell_constrained`]).
//! - [`benchmark`]: test functions with known minima, and test problems with known fronts, which
//!   the tests hold the optimizers to.
//!
//! A model is minimized; to maximize an output, minimize its negative. To hit a target, minimize
//! the squared miss, as the guide's example does.
//!
//! # Reproducibility
//!
//! A run is drawn from a seed. Each CMA-ES candidate has its own random stream
//! ([`SeededRng::for_stream`](hpr_core::random::SeededRng::for_stream)), keyed by the seed, its
//! generation and its place in the generation, and each NSGA-II generation has one stream, so a run is bit
//! for bit the same every time on one platform, however its candidates are evaluated.
//!
//! # Left out
//!
//! Bayesian optimization and optimizing a Monte Carlo run's statistics are later increments of [M6.2, the optimization milestone][roadmap].
//!
//! [roadmap]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m6-2

pub mod benchmark;
pub mod cmaes;
mod eigen;
mod normal;
pub mod nsga2;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::AnalysisError;

/// The most variables an optimizer takes. CMA-ES holds an `n × n` covariance and decomposes it
/// each generation, `O(n³)` work; 200 variables is far more than a rocket design has.
pub const MAX_VARIABLES: usize = 200;

/// The largest whole number an integer variable's bound may be, `2⁵²`: every whole number up to
/// it, and every threshold halfway between two, is an `f64`.
const MAX_WHOLE: f64 = 4_503_599_627_370_496.0;

/// The whole number nearest `x`, a tie going to the lower, and 0 rather than −0. Exact: below
/// 2⁵² `⌊x⌋ + 0.5` is an `f64`, and from there on `x` is whole (`x − 0.5` or `x − ⌊x⌋` would
/// round).
pub(crate) fn nearest_whole(x: f64) -> f64 {
    let floor = x.floor();
    let nearest = if x > floor + 0.5 { floor + 1.0 } else { floor };
    nearest + 0.0
}

/// Checks an integer variable's bounds: whole numbers within `±2⁵²`.
fn check_whole(low: f64, high: f64) -> Result<(), AnalysisError> {
    let whole = |x: f64| x.fract() == 0.0 && x.abs() <= MAX_WHOLE;
    if !whole(low) {
        return Err(AnalysisError::Domain {
            what: "integer variable's low bound (a whole number within ±2⁵²)",
            value: low,
        });
    }
    if !whole(high) {
        return Err(AnalysisError::Domain {
            what: "integer variable's high bound (a whole number within ±2⁵²)",
            value: high,
        });
    }
    Ok(())
}

/// A number the optimizer may change: its name, where it starts, the size of its first steps,
/// the range it must stay in, and whether it takes only whole numbers. It serializes as its
/// fields (an unbounded side as `null`; `integer` only when true, and read as `false` when
/// absent), and reads back through [`Variable::new`], [`Variable::within`] and [`Variable::integer`]'s checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "VariableData", into = "VariableData")]
pub struct Variable {
    name: String,
    start: f64,
    step: f64,
    low: f64,
    high: f64,
    integer: bool,
}

/// The serialized form of a [`Variable`].
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VariableData {
    name: String,
    start: f64,
    step: f64,
    low: Option<f64>,
    high: Option<f64>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    integer: bool,
}

impl TryFrom<VariableData> for Variable {
    type Error = AnalysisError;

    fn try_from(data: VariableData) -> Result<Self, AnalysisError> {
        let variable = Self::new(data.name, data.start, data.step)?.within(
            data.low.unwrap_or(f64::NEG_INFINITY),
            data.high.unwrap_or(f64::INFINITY),
        )?;
        if data.integer {
            variable.integer()
        } else {
            Ok(variable)
        }
    }
}

impl From<Variable> for VariableData {
    fn from(v: Variable) -> Self {
        Self {
            name: v.name,
            start: v.start,
            step: v.step,
            low: v.low.is_finite().then_some(v.low),
            high: v.high.is_finite().then_some(v.high),
            integer: v.integer,
        }
    }
}

impl Variable {
    /// A variable named `name`, starting at `start`, with no bounds. `step` is the size of the
    /// optimizer's first steps in it: about a quarter to a third of the range its best value is
    /// expected in, in the variable's own units.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a `start` that isn't finite, or a `step` that isn't finite
    /// and positive.
    pub fn new(name: impl Into<String>, start: f64, step: f64) -> Result<Self, AnalysisError> {
        if !start.is_finite() {
            return Err(AnalysisError::Domain {
                what: "variable's start",
                value: start,
            });
        }
        if !(step.is_finite() && step > 0.0) {
            return Err(AnalysisError::Domain {
                what: "variable's step (finite, positive)",
                value: step,
            });
        }
        Ok(Self {
            name: name.into(),
            start,
            step,
            low: f64::NEG_INFINITY,
            high: f64::INFINITY,
            integer: false,
        })
    }

    /// The same variable, held to `[low, high]`. Either bound may be infinite, for none.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a NaN bound, a `high` not above `low`, a start outside
    /// the range, or for an integer variable bounds that aren't whole numbers within `±2⁵²`.
    pub fn within(mut self, low: f64, high: f64) -> Result<Self, AnalysisError> {
        if low.is_nan() || low == f64::INFINITY {
            return Err(AnalysisError::Domain {
                what: "variable's low bound",
                value: low,
            });
        }
        if high.is_nan() || high <= low {
            return Err(AnalysisError::Domain {
                what: "variable's high bound (above the low one)",
                value: high,
            });
        }
        if !(low..=high).contains(&self.start) {
            return Err(AnalysisError::Domain {
                what: "variable's start (within its bounds)",
                value: self.start,
            });
        }
        if self.integer {
            check_whole(low, high)?;
        }
        self.low = low;
        self.high = high;
        Ok(self)
    }

    /// The same variable, taking only the whole numbers from its low bound to its high one: a
    /// count, or the place of a choice in a list (a motor, a catalogue part). The optimizer still
    /// draws it as a real number, and the model is given the whole number nearest the draw,
    /// clamped to the bounds ([`Variable::encode`]); [`cmaes`] keeps every value within reach by
    /// the *margin* of CMA-ES with margin. Its step is the size of the first steps, in whole
    /// numbers: 1 is a good start for a handful of choices. Its start may lie between two whole
    /// numbers, as the draws are centred on it.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for bounds that aren't whole numbers within `±2⁵²`: set them
    /// first with [`Variable::within`], which checks them again if called after.
    pub fn integer(mut self) -> Result<Self, AnalysisError> {
        check_whole(self.low, self.high)?;
        self.integer = true;
        Ok(self)
    }

    /// Whether it takes only whole numbers ([`Variable::integer`]).
    pub fn is_integer(&self) -> bool {
        self.integer
    }

    /// The value the model is given for a draw `x`: `x` itself, or for an integer variable the
    /// whole number nearest `x`, a draw halfway between two going to the lower, clamped to the
    /// bounds. These are the *encoding* of R. Hamano et al., "CMA-ES with Margin", GECCO 2022
    /// (arXiv:2205.13482, §4.1, p. 5), with thresholds halfway between neighbouring values. NaN
    /// stays NaN (a run stops before it would draw one).
    pub fn encode(&self, x: f64) -> f64 {
        if self.integer {
            // `+ 0.0` turns a bound written −0 into 0.
            nearest_whole(x).clamp(self.low, self.high) + 0.0
        } else {
            x
        }
    }

    /// Its name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Where it starts.
    pub fn start(&self) -> f64 {
        self.start
    }

    /// The size of its first steps.
    pub fn step(&self) -> f64 {
        self.step
    }

    /// Its low bound, `−∞` for none.
    pub fn low(&self) -> f64 {
        self.low
    }

    /// Its high bound, `+∞` for none.
    pub fn high(&self) -> f64 {
        self.high
    }

    /// Whether `x` is within its bounds.
    pub fn contains(&self, x: f64) -> bool {
        (self.low..=self.high).contains(&x)
    }
}

/// What a model gives for one candidate under constraints: its value, and by how much it breaks
/// the constraints, zero if it keeps them all.
///
/// Candidates are ranked by K. Deb's feasibility rules ("An efficient constraint handling method
/// for genetic algorithms", *Computer Methods in Applied Mechanics and Engineering* 186(2–4),
/// 311–338 (2000), <https://doi.org/10.1016/S0045-7825(99)00389-8>, §3): a candidate that keeps
/// every constraint beats one that doesn't; of two that keep them, the smaller value wins; of
/// two that don't, the smaller violation wins (here ties in violation go to the smaller value).
/// No penalty weight is needed, as values and violations are never compared with each other.
/// The rules rank, and CMA-ES uses only ranks.
///
/// A candidate the model can't evaluate (a flight that fails) is `value` and `violation` both
/// `+∞`: it ranks behind every other. Both serialize `+∞` as none, a JSON `null`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Evaluation {
    /// The model's value.
    #[serde(with = "cmaes::infinity_as_none")]
    pub value: f64,
    /// The total violation, `Σ max(0, gⱼ)` over constraints written `gⱼ ≤ 0`: zero if the
    /// candidate keeps them all.
    #[serde(with = "cmaes::infinity_as_none")]
    pub violation: f64,
}

impl Evaluation {
    /// A value with no constraints to break.
    pub const fn feasible(value: f64) -> Self {
        Self {
            value,
            violation: 0.0,
        }
    }

    /// A candidate the model can't evaluate: value and violation both `+∞`, behind every other.
    pub const fn failed() -> Self {
        Self {
            value: f64::INFINITY,
            violation: f64::INFINITY,
        }
    }

    /// A value under constraints `gⱼ(x) ≤ 0`, given as the numbers `gⱼ`: the violation is
    /// `Σ max(0, gⱼ)`, Deb's (2000) overall violation. Deb divides each constraint by a constant
    /// so that they count alike (a margin in calibers and a speed in m/s, say); do the same before
    /// passing them in. A NaN `gⱼ` gives a NaN violation, which [`cmaes::Run::tell_constrained`]
    /// refuses.
    pub fn constrained(value: f64, constraints: &[f64]) -> Self {
        // Folded from +0: an empty f64 sum is −0, which `total_cmp` would rank first.
        let violation = constraints
            .iter()
            .map(|&g| if g.is_nan() || g > 0.0 { g } else { 0.0 })
            .fold(0.0, |total, g| total + g);
        Self { value, violation }
    }

    /// Whether the candidate keeps every constraint.
    pub fn is_feasible(&self) -> bool {
        self.violation == 0.0
    }

    /// Deb's rules as an ordering: [`Less`](std::cmp::Ordering::Less) if `self` ranks ahead of
    /// `other`. Violation first, with `−0` equal to `0`, then value by [`f64::total_cmp`]
    /// (CMA-ES's own ranking of plain values).
    pub fn rank(&self, other: &Self) -> std::cmp::Ordering {
        let violation = if self.violation == other.violation {
            std::cmp::Ordering::Equal
        } else {
            self.violation.total_cmp(&other.violation)
        };
        violation.then(self.value.total_cmp(&other.value))
    }

    /// Whether `self` is strictly better than `other` by Deb's rules, comparing as `<` does, so
    /// `−0` and `0` tie.
    pub(crate) fn beats(&self, other: &Self) -> bool {
        self.violation < other.violation
            || (self.violation == other.violation && self.value < other.value)
    }
}

/// Checks that there are between one and [`MAX_VARIABLES`] variables, and no two share a name.
fn check_variables(variables: &[Variable]) -> Result<(), AnalysisError> {
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
    let mut names = BTreeSet::new();
    for variable in variables {
        if !names.insert(variable.name()) {
            return Err(AnalysisError::DuplicateVariable(variable.name().to_owned()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_checks_its_numbers() {
        assert!(Variable::new("x", f64::NAN, 1.0).is_err());
        assert!(Variable::new("x", 0.0, 0.0).is_err());
        assert!(Variable::new("x", 0.0, f64::INFINITY).is_err());
        let x = Variable::new("x", 0.5, 0.1).unwrap();
        assert!(x.clone().within(1.0, 2.0).is_err(), "start outside");
        assert!(x.clone().within(1.0, 1.0).is_err(), "empty range");
        assert!(x.clone().within(f64::NAN, 1.0).is_err());
        assert!(x.clone().within(0.0, f64::NAN).is_err());
        assert!(x.clone().within(f64::INFINITY, f64::INFINITY).is_err());
        let x = x.within(0.0, f64::INFINITY).unwrap();
        assert!(x.contains(0.0) && x.contains(1e300) && !x.contains(-1e-300));
    }

    #[test]
    fn evaluations_rank_by_deb_rules() {
        use std::cmp::Ordering::{Greater, Less};
        let e = Evaluation::constrained(5.0, &[-1.0, 0.0, -3.0]);
        assert!(e.is_feasible());
        let broken = Evaluation::constrained(-100.0, &[0.25, -1.0, 0.5]);
        assert_eq!(broken.violation, 0.75);
        // Feasible beats infeasible, whatever the values.
        assert_eq!(e.rank(&broken), Less);
        // Two infeasible: the smaller violation, whatever the values.
        assert_eq!(broken.rank(&Evaluation::constrained(-1e9, &[1.0])), Less);
        // Two feasible: the smaller value.
        assert_eq!(e.rank(&Evaluation::feasible(4.0)), Greater);
        assert!(Evaluation::constrained(0.0, &[f64::NAN]).violation.is_nan());
        // No constraints is +0, not the −0 of an empty sum, so values decide.
        let none = Evaluation::constrained(10.0, &[]);
        assert!(none.violation.is_sign_positive());
        assert_eq!(none.rank(&Evaluation::feasible(1.0)), Greater);
        let negative_zero = Evaluation {
            value: 10.0,
            violation: -0.0,
        };
        assert_eq!(negative_zero.rank(&Evaluation::feasible(1.0)), Greater);
        // A failure ranks behind everything, and reads back from JSON.
        assert_eq!(broken.rank(&Evaluation::failed()), Less);
        let json = serde_json::to_string(&Evaluation::failed()).unwrap();
        assert_eq!(json, r#"{"value":null,"violation":null}"#);
        assert_eq!(
            serde_json::from_str::<Evaluation>(&json).unwrap(),
            Evaluation::failed()
        );
    }

    #[test]
    fn variable_serializes_unbounded_sides_as_null_and_rechecks_on_reading() {
        let x = Variable::new("ballast", 0.2, 0.1)
            .unwrap()
            .within(0.0, f64::INFINITY)
            .unwrap();
        let json = serde_json::to_string(&x).unwrap();
        assert_eq!(
            json,
            r#"{"name":"ballast","start":0.2,"step":0.1,"low":0.0,"high":null}"#
        );
        assert_eq!(serde_json::from_str::<Variable>(&json).unwrap(), x);
        // `integer` is written only when true, and may be written false.
        let explicit =
            r#"{"name":"ballast","start":0.2,"step":0.1,"low":0.0,"high":null,"integer":false}"#;
        assert_eq!(serde_json::from_str::<Variable>(explicit).unwrap(), x);
        let bad = r#"{"name":"x","start":2.0,"step":0.1,"low":0.0,"high":1.0}"#;
        assert!(serde_json::from_str::<Variable>(bad).is_err());
        let motor = Variable::new("motor", 2.0, 1.0)
            .unwrap()
            .within(0.0, 4.0)
            .unwrap()
            .integer()
            .unwrap();
        let json = serde_json::to_string(&motor).unwrap();
        assert_eq!(
            json,
            r#"{"name":"motor","start":2.0,"step":1.0,"low":0.0,"high":4.0,"integer":true}"#
        );
        assert_eq!(serde_json::from_str::<Variable>(&json).unwrap(), motor);
        // An integer variable's bounds are checked on reading too.
        let bad = r#"{"name":"k","start":0.0,"step":1.0,"low":-0.5,"high":3.0,"integer":true}"#;
        assert!(serde_json::from_str::<Variable>(bad).is_err());
    }

    #[test]
    fn integer_variables_need_whole_bounds() {
        let k = Variable::new("k", 1.5, 1.0).unwrap();
        for (low, high) in [
            (f64::NEG_INFINITY, 4.0),
            (0.0, f64::INFINITY),
            (0.5, 4.0),
            (0.0, 3.5),
            (-1e16, 4.0),
        ] {
            let err = k.clone().within(low, high).unwrap().integer().unwrap_err();
            let AnalysisError::Domain { what, .. } = err else {
                panic!("{low}, {high}: {err:?}");
            };
            assert!(what.starts_with("integer variable's"), "{what}");
            // Bounds set after `integer` are checked too.
            let err = k
                .clone()
                .within(0.0, 4.0)
                .unwrap()
                .integer()
                .unwrap()
                .within(low, high)
                .unwrap_err();
            let AnalysisError::Domain { what, .. } = err else {
                panic!("{low}, {high} after: {err:?}");
            };
            assert!(what.starts_with("integer variable's"), "{what}");
        }
        // A start between two whole numbers is allowed: the draws are centred on it.
        let k = k.within(0.0, 4.0).unwrap().integer().unwrap();
        assert!(k.is_integer() && !Variable::new("x", 0.0, 1.0).unwrap().is_integer());
    }

    /// The nearest whole number, halfway going to the lower, clamped to the bounds; a continuous
    /// variable's draw is its own value.
    #[test]
    fn integer_draws_encode_to_the_nearest_value() {
        let k = Variable::new("k", 0.0, 1.0)
            .unwrap()
            .within(-2.0, 3.0)
            .unwrap()
            .integer()
            .unwrap();
        for (x, value) in [
            (0.0, 0.0_f64),
            (0.5, 0.0),
            (0.500_000_000_000_1, 1.0),
            (-0.5, -1.0),
            (-0.499_999_999_999_9, 0.0),
            (1e-300, 0.0),
            (2.5, 2.0),
            (2.6, 3.0),
            (40.0, 3.0),
            (-1.5, -2.0),
            (-7.2, -2.0),
            (f64::MAX, 3.0),
            (-0.0, 0.0),
            (-0.3, 0.0),
            // `x + 1` would round to 0.5 here, a tie.
            (-0.499_999_999_999_999_94, 0.0),
            (0.499_999_999_999_999_94, 0.0),
        ] {
            // Bits, so that −0 isn't taken for 0.
            assert_eq!(k.encode(x).to_bits(), value.to_bits(), "{x}");
        }
        // Up to 2⁵², where `x − 0.5` would round to an even neighbour; past it, clamped.
        let wide = Variable::new("k", 0.0, 1.0)
            .unwrap()
            .within(-MAX_WHOLE, MAX_WHOLE)
            .unwrap()
            .integer()
            .unwrap();
        for (x, value) in [
            (MAX_WHOLE - 1.0, MAX_WHOLE - 1.0),
            (MAX_WHOLE - 0.5, MAX_WHOLE - 1.0),
            (MAX_WHOLE, MAX_WHOLE),
            (MAX_WHOLE + 1.0, MAX_WHOLE),
            (-MAX_WHOLE + 0.5, -MAX_WHOLE),
        ] {
            assert_eq!(wide.encode(x), value, "{x}");
        }
        for high in [MAX_WHOLE + 1.0, 2.0 * MAX_WHOLE] {
            let err = Variable::new("k", 0.0, 1.0)
                .unwrap()
                .within(0.0, high)
                .unwrap()
                .integer()
                .unwrap_err();
            let AnalysisError::Domain { what, value } = err else {
                panic!("{high}: {err:?}");
            };
            assert!(what.starts_with("integer variable's high bound"), "{what}");
            assert_eq!(value, high);
        }
        let x = Variable::new("x", 0.0, 1.0).unwrap();
        assert_eq!(x.encode(0.7), 0.7);
    }

    #[test]
    fn variables_need_distinct_names_and_a_count_in_range() {
        let x = Variable::new("x", 0.0, 1.0).unwrap();
        assert!(matches!(
            check_variables(&[]),
            Err(AnalysisError::TooFew {
                what: "variables",
                ..
            })
        ));
        assert!(matches!(
            check_variables(&[x.clone(), x.clone()]),
            Err(AnalysisError::DuplicateVariable(name)) if name == "x"
        ));
        let many = vec![x; MAX_VARIABLES + 1];
        assert!(matches!(
            check_variables(&many),
            Err(AnalysisError::Count {
                what: "variables",
                count,
                ..
            }) if count == MAX_VARIABLES + 1
        ));
    }
}
