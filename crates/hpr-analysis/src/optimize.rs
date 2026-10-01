//! Optimization: the design variables that make a model's output as small as it can be.
//!
//! **Guide:** [Optimization][guide] runs the optimizer on test functions whose minima are known,
//! then finds the ballast and body length that send a rocket to 3,048 m, and says how far to
//! trust it.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/optimization.html
//!
//! - [`Variable`]: one number the optimizer may change, with where it starts, the size of its
//!   first steps, and optional bounds.
//! - [`cmaes`]: the covariance matrix adaptation evolution strategy (CMA-ES), which samples
//!   candidates around a mean, keeps the better half, and learns from them which way, and how
//!   far, to step next. It needs only the output's ranking, no derivatives, so it suits flights,
//!   whose outputs are noisy in their last digits.
//! - [`benchmark`]: test functions with known minima, which the tests hold the optimizer to.
//!
//! A model is minimized; to maximize an output, minimize its negative. To hit a target, minimize
//! the squared miss, as the guide's example does.
//!
//! # Reproducibility
//!
//! A run is drawn from a seed. Each candidate has its own random stream
//! ([`SeededRng::for_stream`](hpr_core::random::SeededRng::for_stream)), keyed by the seed, its
//! generation and its place in the generation, so a run is bit for bit the same every time on
//! one platform, however its candidates are evaluated.
//!
//! # Left out
//!
//! Variables are continuous. Discrete choices (a motor, a catalogue part), constraints other than
//! bounds, several objectives at once, Bayesian optimization and optimizing a Monte Carlo run's
//! statistics are later increments of [M6.2, the optimization milestone][roadmap].
//!
//! [roadmap]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m6-2

pub mod benchmark;
pub mod cmaes;
mod eigen;

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::error::AnalysisError;

/// The most variables an optimizer takes. CMA-ES holds an `n × n` covariance and decomposes it
/// each generation, `O(n³)` work; 200 variables is far more than a rocket design has.
pub const MAX_VARIABLES: usize = 200;

/// A number the optimizer may change: its name, where it starts, the size of its first steps,
/// and the range it must stay in. It serializes as its five fields (an unbounded side as
/// `null`), and reads back through [`Variable::new`] and [`Variable::within`]'s checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "VariableData", into = "VariableData")]
pub struct Variable {
    name: String,
    start: f64,
    step: f64,
    low: f64,
    high: f64,
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
}

impl TryFrom<VariableData> for Variable {
    type Error = AnalysisError;

    fn try_from(data: VariableData) -> Result<Self, AnalysisError> {
        Self::new(data.name, data.start, data.step)?.within(
            data.low.unwrap_or(f64::NEG_INFINITY),
            data.high.unwrap_or(f64::INFINITY),
        )
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
        })
    }

    /// The same variable, held to `[low, high]`. Either bound may be infinite, for none.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a NaN bound, a `high` not above `low`, or a start outside
    /// the range.
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
        self.low = low;
        self.high = high;
        Ok(self)
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
        let bad = r#"{"name":"x","start":2.0,"step":0.1,"low":0.0,"high":1.0}"#;
        assert!(serde_json::from_str::<Variable>(bad).is_err());
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
