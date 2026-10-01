//! Summaries of a sample of numbers: the spread of a Monte Carlo run's apogees, say.
//!
//! A [`Distribution`] keeps every value, sorted, and the number of samples that were tried, so a
//! sample that failed or never gave a value (a flight with no apogee) is counted, not dropped:
//! its share is [`Distribution::missing`] of [`Distribution::attempted`], and a probability is
//! reported as the bounds those unknowns allow ([`Distribution::share_at_least`]).
//!
//! - **Mean and standard deviation** are taken on the values shifted by the smallest one, the
//!   standard deviation by the two-pass formula with `n − 1` (T. F. Chan, G. H. Golub and R. J.
//!   LeVeque, "Algorithms for computing the sample variance: analysis and recommendations", *The
//!   American Statistician* 37(3), 242–247, 1983, <https://doi.org/10.2307/2683386>). Shifting
//!   by a value of the sample keeps the sums small, and makes a sample of equal values give that
//!   value and a deviation of exactly zero.
//! - **Quantiles** are Hyndman and Fan's definition 7, linear between order statistics, the
//!   default of R and NumPy: with the values sorted `x₀ ≤ … ≤ xₙ₋₁` and `h = (n − 1) p`,
//!   `Q(p) = x⌊h⌋ + (h − ⌊h⌋)(x⌊h⌋₊₁ − x⌊h⌋)` (R. J. Hyndman and Y. Fan, "Sample quantiles in
//!   statistical packages", *The American Statistician* 50(4), 361–365, 1996,
//!   <https://doi.org/10.2307/2684934>).
//!
//! Every sum runs over the sorted values in order, so a summary is bit-for-bit the same however
//! the values were computed, in parallel or not.

use serde::{Deserialize, Serialize};

use crate::error::AnalysisError;

/// The values a sample of runs gave, sorted, and how many runs were tried.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Distribution {
    attempted: usize,
    sorted: Vec<f64>,
}

/// The usual numbers of a [`Distribution`], for a report.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// The runs tried.
    pub attempted: usize,
    /// The runs that gave a value.
    pub count: usize,
    /// The mean, `None` with no values.
    pub mean: Option<f64>,
    /// The sample standard deviation (`n − 1`), `None` with fewer than two values.
    pub standard_deviation: Option<f64>,
    /// The smallest value.
    pub min: Option<f64>,
    /// The 5th percentile ([`Distribution::quantile`]).
    pub p05: Option<f64>,
    /// The median.
    pub p50: Option<f64>,
    /// The 95th percentile.
    pub p95: Option<f64>,
    /// The largest value.
    pub max: Option<f64>,
}

/// Bounds on the share of all the runs tried whose value passed a test: `low` counts a run with
/// no value as failing it, `high` as passing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Share {
    /// The runs with a value that passed, over every run tried.
    pub low: f64,
    /// The same, with every run that gave no value counted as passing.
    pub high: f64,
}

impl Distribution {
    /// The distribution of `values`, from `attempted` runs (those that gave no value make up the
    /// difference).
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::Domain`] for a value that isn't finite.
    /// - [`AnalysisError::Count`] for more values than runs.
    pub fn new(values: Vec<f64>, attempted: usize) -> Result<Self, AnalysisError> {
        if values.len() > attempted {
            return Err(AnalysisError::Count {
                what: "values in a distribution, against the runs tried",
                count: values.len(),
                limit: attempted,
            });
        }
        if let Some(&bad) = values.iter().find(|v| !v.is_finite()) {
            return Err(AnalysisError::Domain {
                what: "value in a distribution",
                value: bad,
            });
        }
        let mut sorted = values;
        sorted.sort_by(f64::total_cmp);
        Ok(Self { attempted, sorted })
    }

    /// The runs tried.
    pub fn attempted(&self) -> usize {
        self.attempted
    }

    /// The runs that gave a value.
    pub fn count(&self) -> usize {
        self.sorted.len()
    }

    /// The runs that gave no value: failed, or without the quantity asked for.
    pub fn missing(&self) -> usize {
        self.attempted - self.sorted.len()
    }

    /// The values, smallest first.
    pub fn sorted(&self) -> &[f64] {
        &self.sorted
    }

    /// The smallest value, `None` with no values.
    pub fn min(&self) -> Option<f64> {
        self.sorted.first().copied()
    }

    /// The largest value, `None` with no values.
    pub fn max(&self) -> Option<f64> {
        self.sorted.last().copied()
    }

    /// The mean, `x₀ + Σ(xᵢ − x₀)/n` with `x₀` the smallest value; `None` with no values.
    pub fn mean(&self) -> Option<f64> {
        let shift = self.min()?;
        Some(shift + self.shifted_mean(shift))
    }

    /// The sample standard deviation, `√(Σ(dᵢ − d̄)²/(n − 1))` with `dᵢ = xᵢ − x₀`; `None` with
    /// fewer than two values.
    pub fn standard_deviation(&self) -> Option<f64> {
        if self.sorted.len() < 2 {
            return None;
        }
        let shift = self.min()?;
        let mean = self.shifted_mean(shift);
        let sum_squares: f64 = self
            .sorted
            .iter()
            .map(|&x| {
                let d = (x - shift) - mean;
                d * d
            })
            .sum();
        // Cast: a count of values is far below 2⁵³.
        Some((sum_squares / (self.sorted.len() - 1) as f64).sqrt())
    }

    /// The mean of the values less `shift`.
    fn shifted_mean(&self, shift: f64) -> f64 {
        let sum: f64 = self.sorted.iter().map(|&x| x - shift).sum();
        // Cast: a count of values is far below 2⁵³.
        sum / self.sorted.len() as f64
    }

    /// The `p` quantile by Hyndman and Fan's definition 7 (the module's docs); `None` with no
    /// values.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::Domain`] for `p` outside `[0, 1]`.
    pub fn quantile(&self, p: f64) -> Result<Option<f64>, AnalysisError> {
        if !(0.0..=1.0).contains(&p) {
            return Err(AnalysisError::Domain {
                what: "quantile probability",
                value: p,
            });
        }
        let Some(&last) = self.sorted.last() else {
            return Ok(None);
        };
        // Cast: a count of values is far below 2⁵³, and `h` lies in `[0, n − 1]`.
        let h = (self.sorted.len() - 1) as f64 * p;
        let below = h.floor();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = below as usize;
        let Some(&above) = self.sorted.get(index + 1) else {
            return Ok(Some(last));
        };
        let low = self.sorted[index];
        Ok(Some(low + (h - below) * (above - low)))
    }

    /// Bounds on the share of the runs tried whose value is at least `threshold` ([`Share`]).
    /// With no runs tried, both bounds are zero.
    pub fn share_at_least(&self, threshold: f64) -> Share {
        if self.attempted == 0 {
            return Share {
                low: 0.0,
                high: 0.0,
            };
        }
        let passed = self.sorted.len() - self.sorted.partition_point(|&x| x < threshold);
        // Cast: counts far below 2⁵³.
        let attempted = self.attempted as f64;
        Share {
            low: passed as f64 / attempted,
            high: (passed + self.missing()) as f64 / attempted,
        }
    }

    /// The usual numbers, for a report: [`Summary`].
    pub fn summary(&self) -> Summary {
        // The probabilities are in [0, 1].
        let quantile = |p: f64| self.quantile(p).ok().flatten();
        Summary {
            attempted: self.attempted,
            count: self.count(),
            mean: self.mean(),
            standard_deviation: self.standard_deviation(),
            min: self.min(),
            p05: quantile(0.05),
            p50: quantile(0.5),
            p95: quantile(0.95),
            max: self.max(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_values_give_their_value_and_no_spread() {
        // A sum of 0.1s divided by their count isn't 0.1 in floating point; shifted by the
        // smallest value, it is.
        let d = Distribution::new(vec![0.1; 7], 7).unwrap();
        assert_eq!(d.mean(), Some(0.1));
        assert_eq!(d.standard_deviation(), Some(0.0));
        assert_eq!(d.quantile(0.37).unwrap(), Some(0.1));
    }

    #[test]
    fn moments_and_quantiles_by_hand() {
        let d = Distribution::new(vec![4.0, 1.0, 3.0, 2.0], 6).unwrap();
        assert_eq!(d.sorted(), &[1.0, 2.0, 3.0, 4.0]);
        assert_eq!((d.count(), d.missing(), d.attempted()), (4, 2, 6));
        assert_eq!(d.mean(), Some(2.5));
        // Σ(x − 2.5)² = 5, over n − 1 = 3.
        assert_eq!(d.standard_deviation(), Some((5.0_f64 / 3.0).sqrt()));
        // h = 3p: p = 0.5 is halfway from 2 to 3; p = 0.9 is 0.7 of the way from 3 to 4. NumPy's
        // `np.quantile([1, 2, 3, 4], [0, 0.5, 0.9, 1])` gives the same: 1, 2.5, 3.7, 4.
        assert_eq!(d.quantile(0.0).unwrap(), Some(1.0));
        assert_eq!(d.quantile(0.5).unwrap(), Some(2.5));
        assert!((d.quantile(0.9).unwrap().unwrap() - 3.7).abs() < 1e-15);
        assert_eq!(d.quantile(1.0).unwrap(), Some(4.0));
        let summary = d.summary();
        assert_eq!((summary.min, summary.max), (Some(1.0), Some(4.0)));
        assert_eq!(summary.p50, Some(2.5));
    }

    #[test]
    fn a_share_is_bounded_by_the_runs_with_no_value() {
        // Two of six runs gave nothing: at least 2/6 and at most 4/6 reached 3.
        let d = Distribution::new(vec![1.0, 2.0, 3.0, 4.0], 6).unwrap();
        let share = d.share_at_least(3.0);
        assert_eq!((share.low, share.high), (2.0 / 6.0, 4.0 / 6.0));
        let all = Distribution::new(vec![1.0, 2.0], 2)
            .unwrap()
            .share_at_least(1.0);
        assert_eq!((all.low, all.high), (1.0, 1.0));
        let none = Distribution::new(vec![], 0).unwrap().share_at_least(1.0);
        assert_eq!((none.low, none.high), (0.0, 0.0));
    }

    #[test]
    fn empty_and_single_samples() {
        let empty = Distribution::new(vec![], 3).unwrap();
        assert_eq!(empty.mean(), None);
        assert_eq!(empty.quantile(0.5).unwrap(), None);
        assert_eq!(empty.missing(), 3);
        let one = Distribution::new(vec![2.0], 1).unwrap();
        assert_eq!(one.mean(), Some(2.0));
        assert_eq!(one.standard_deviation(), None);
        assert_eq!(one.quantile(0.3).unwrap(), Some(2.0));
    }

    #[test]
    fn bad_inputs_are_refused() {
        assert!(matches!(
            Distribution::new(vec![1.0, f64::NAN], 2),
            Err(AnalysisError::Domain { what: "value in a distribution", value }) if value.is_nan()
        ));
        assert!(matches!(
            Distribution::new(vec![1.0, 2.0], 1),
            Err(AnalysisError::Count {
                count: 2,
                limit: 1,
                ..
            })
        ));
        let d = Distribution::new(vec![1.0], 1).unwrap();
        for p in [-0.1, 1.1, f64::NAN] {
            assert!(matches!(
                d.quantile(p),
                Err(AnalysisError::Domain { what: "quantile probability", value })
                    if value.to_bits() == p.to_bits()
            ));
        }
    }
}
