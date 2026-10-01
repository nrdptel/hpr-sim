//! Sensitivity analysis: which of a model's uncertain inputs move its output most.
//!
//! **Guide:** [Sensitivity analysis][guide] runs both methods on test functions whose answers are
//! known, then on a rocket's apogee, and says how far to trust them.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/sensitivity.html
//!
//! Two methods, both over *factors*: inputs each spread evenly between a low and a high value
//! ([`Factor`]), independent of each other.
//!
//! - [`morris`]: Morris's screening. A few cheap runs, stepping one factor at a time along random
//!   paths through a grid, rank the factors by how much a step in each moves the output. It
//!   finds the factors that don't matter, at about ten runs per factor.
//! - [`sobol`]: Sobol' indices. Many runs split the output's variance among the factors: the
//!   share each causes alone (its *first-order* index) and the share it has any part in, with
//!   the others (its *total* index). It costs hundreds of runs per factor.
//! - [`benchmark`]: two test functions whose indices are known in closed form, Ishigami and
//!   Homma's and Sobol's g, which the tests hold both methods to.
//!
//! Each method lays out its points first (a *design*), takes the model's output at each, in the
//! design's order, and then analyses them. The model can be anything: a flight, flown with the
//! factors set into its inputs, or a function. So the runs can be made however the caller likes,
//! on many threads or on many machines; each method also has a shortcut that takes a closure.
//!
//! # Reproducibility
//!
//! A design is drawn from a seed. Each Morris path and each Sobol' sample row has its own random
//! stream ([`SeededRng::for_stream`](hpr_core::random::SeededRng::for_stream)), keyed by the
//! seed and its index, so path or row `k` is the same however many are drawn. Every sum runs in
//! the design's order, so on one platform an analysis is bit for bit the same every time.
//!
//! # Left out
//!
//! Factors are uniform and independent: no other distributions, no correlations. A normal input
//! can be given as a range about its mean (say ±2 standard deviations), which spreads it more
//! evenly than it is. Sobol' samples are pseudo-random, not quasi-random. Second-order Sobol'
//! indices, and Campolongo's choice of the most spread-out Morris paths among many, are not
//! computed.

pub mod benchmark;
pub mod morris;
pub mod sobol;

use serde::{Deserialize, Serialize};

use crate::error::AnalysisError;

/// An uncertain input, spread evenly between `low` and `high`. It serializes as its three
/// fields, and reads back through [`Factor::new`]'s checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "FactorData")]
pub struct Factor {
    name: String,
    low: f64,
    high: f64,
}

/// The serialized form of a [`Factor`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FactorData {
    name: String,
    low: f64,
    high: f64,
}

impl TryFrom<FactorData> for Factor {
    type Error = AnalysisError;

    fn try_from(data: FactorData) -> Result<Self, AnalysisError> {
        Self::new(data.name, data.low, data.high)
    }
}

impl Factor {
    /// A factor named `name`, spread evenly over `[low, high]`.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for a bound that isn't finite, or a `high` not above `low`.
    pub fn new(name: impl Into<String>, low: f64, high: f64) -> Result<Self, AnalysisError> {
        if !low.is_finite() {
            return Err(AnalysisError::Domain {
                what: "factor's low value",
                value: low,
            });
        }
        if !(high.is_finite() && high > low) {
            return Err(AnalysisError::Domain {
                what: "factor's high value (finite, above the low one)",
                value: high,
            });
        }
        Ok(Self {
            name: name.into(),
            low,
            high,
        })
    }

    /// Its name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The low end of its range.
    pub fn low(&self) -> f64 {
        self.low
    }

    /// The high end of its range.
    pub fn high(&self) -> f64 {
        self.high
    }

    /// The value a fraction `u` of the way from `low` to `high`: `low + u (high − low)`.
    pub fn at(&self, u: f64) -> f64 {
        self.low + u * (self.high - self.low)
    }
}

/// Checks that there is at least one factor.
fn check_factors(factors: &[Factor]) -> Result<(), AnalysisError> {
    if factors.is_empty() {
        return Err(AnalysisError::TooFew {
            what: "factors",
            count: 0,
            minimum: 1,
        });
    }
    Ok(())
}

/// Checks that `outputs` has one finite value for each of a design's `points`.
fn check_outputs(outputs: &[f64], points: usize) -> Result<(), AnalysisError> {
    if outputs.len() != points {
        return Err(AnalysisError::Length {
            what: "outputs, against the design's points",
            length: outputs.len(),
            expected: points,
        });
    }
    if let Some((index, &value)) = outputs.iter().enumerate().find(|(_, y)| !y.is_finite()) {
        return Err(AnalysisError::Output { index, value });
    }
    Ok(())
}

/// `a × b` for a design's size, refused if it overflows.
fn product(what: &'static str, a: usize, b: usize) -> Result<usize, AnalysisError> {
    a.checked_mul(b).ok_or(AnalysisError::Count {
        what,
        count: a,
        limit: usize::MAX / b.max(1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_factor_maps_the_unit_interval_onto_its_range() {
        let f = Factor::new("drag", 0.9, 1.1).unwrap();
        assert_eq!(f.at(0.0), 0.9);
        assert!((f.at(1.0) - 1.1).abs() < 1e-15);
        assert!((f.at(0.5) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn a_factor_refuses_an_empty_or_reversed_range() {
        let refused = |low: f64, high: f64| match Factor::new("x", low, high) {
            Err(AnalysisError::Domain { what, value }) => (what, value),
            other => panic!("{low}, {high}: {other:?}"),
        };
        assert_eq!(refused(f64::NAN, 1.0).0, "factor's low value");
        let (what, value) = refused(1.0, 1.0);
        assert_eq!(what, "factor's high value (finite, above the low one)");
        assert_eq!(value, 1.0);
        assert_eq!(refused(1.0, 0.0).1, 0.0);
        assert!(refused(0.0, f64::INFINITY).1.is_infinite());
    }

    #[test]
    fn a_factor_reads_back_through_its_checks() {
        let f = Factor::new("wind", 0.0, 8.0).unwrap();
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(json, r#"{"name":"wind","low":0.0,"high":8.0}"#);
        assert_eq!(serde_json::from_str::<Factor>(&json).unwrap(), f);
        assert!(serde_json::from_str::<Factor>(r#"{"name":"w","low":2.0,"high":1.0}"#).is_err());
    }

    #[test]
    fn outputs_must_match_the_design_and_be_finite() {
        assert!(check_outputs(&[1.0, 2.0], 2).is_ok());
        match check_outputs(&[1.0], 2) {
            Err(AnalysisError::Length {
                length, expected, ..
            }) => assert_eq!((length, expected), (1, 2)),
            other => panic!("{other:?}"),
        }
        match check_outputs(&[1.0, f64::NAN, f64::INFINITY], 3) {
            Err(AnalysisError::Output { index, value }) => {
                assert_eq!(index, 1);
                assert!(value.is_nan());
            }
            other => panic!("{other:?}"),
        }
    }
}
