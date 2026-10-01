//! Sobol' indices: the share of the output's variance each factor causes, alone and in all.
//!
//! **Guide:** [Sensitivity analysis][guide]'s *Sobol' indices* section.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/sensitivity.html#sobol-indices
//!
//! # The indices
//!
//! With the factors independent, the output's variance `V = Var(Y)` splits into the parts each
//! factor causes alone, each pair together, and so on. Factor `i`'s *first-order* index is the
//! share it causes alone, `Sᵢ = Var(E[Y | Xᵢ]) / V`; its *total* index is the share it has any
//! part in, `S_Tᵢ = E[Var(Y | X₋ᵢ)] / V = 1 − Var(E[Y | X₋ᵢ]) / V`, with `X₋ᵢ` every factor but
//! `i`. `Sᵢ ≤ S_Tᵢ`, and the gap is the share of `i`'s interactions with the others. A factor
//! whose total index is near zero can be left at its nominal value. I. M. Sobol', "Global
//! sensitivity indices for nonlinear mathematical models and their Monte Carlo estimates",
//! *Mathematics and Computers in Simulation* 55, 271–280, 2001,
//! <https://doi.org/10.1016/S0378-4754(00)00270-6>, defines both.
//!
//! # The estimates
//!
//! Two independent samples of `N` rows, `A` and `B`, each row every factor drawn uniformly over
//! its range, and for each factor `i` a third, `A_B⁽ⁱ⁾`: `A` with its column `i` taken from `B`.
//! The model runs at each row of all `k + 2`, `N (k + 2)` runs in all, and with `f` its output
//!
//! - `Vᵢ ≈ (1/N) Σⱼ f(B)ⱼ (f(A_B⁽ⁱ⁾)ⱼ − f(A)ⱼ)`, and
//! - `V_Tᵢ ≈ (1/(2N)) Σⱼ (f(A)ⱼ − f(A_B⁽ⁱ⁾)ⱼ)²` (Jansen's),
//!
//! with `V` the variance of the `2N` outputs of `A` and `B` together, so `Sᵢ = Vᵢ/V` and
//! `S_Tᵢ = V_Tᵢ/V`. Taking `V` from both samples, not `A`'s alone, is more accurate (A. Saltelli
//! and others, *Global Sensitivity Analysis: The Primer*, Wiley, 2008, p. 166).
//!
//! A. Saltelli, P. Annoni, I. Azzini, F. Campolongo, M. Ratto and S. Tarantola, "Variance based
//! sensitivity analysis of model output. Design and estimator for the total sensitivity index",
//! *Computer Physics Communications* 181, 259–270, 2010,
//! <https://doi.org/10.1016/j.cpc.2009.09.018>, give both (their Table 2, p. 262, rows (b) and
//! (f)). They call Jansen's the best practice so far for the total index (p. 262), and recommend
//! (b) for the first order, as its design holds more useful points (p. 263).
//!
//! The outputs are first shifted by their mean over `A` and `B`. That changes no index. Sobol'
//! (2001, p. 277, remark 3) advises it against a loss of accuracy when the mean is large; it also
//! keeps the first-order estimate's own variance, which has a term in the mean squared, from
//! growing with the output's mean, as an apogee's would.
//!
//! # Sampling error
//!
//! Each row `j` is drawn independently, so every estimate is a smooth function of means over
//! rows, and its standard error follows from the central limit theorem by the delta method: with
//! `ψⱼ` row `j`'s first-order change to the estimate (its *influence*), the standard error is
//! `√(Σⱼ ψⱼ² / (N (N − 1)))`. For `Sᵢ = P/V`, with `pⱼ` row `j`'s term of `Vᵢ`, `qⱼ` and `mⱼ` the
//! means of its two outputs' squares and of the two outputs, and `D` the mean of
//! `f(A_B⁽ⁱ⁾) − f(A)` (the shift's effect),
//!
//! `ψⱼ = ((pⱼ − P) − D (mⱼ − M)) / V − Sᵢ ((qⱼ − Q) − 2 M (mⱼ − M)) / V`,
//!
//! and the same for `S_Tᵢ` with Jansen's terms and no `D`. A unit test checks it against the
//! gradient of each index over the raw row means, and the tests check, over 1,000 seeds of
//! Ishigami's function and 500 of the g function, that it is the spread the estimates really
//! have.

use hpr_core::random::SeededRng;
use serde::{Deserialize, Serialize};

use super::{Factor, check_factors, check_outputs, check_size};
use crate::error::AnalysisError;

/// A Sobol' analysis: the factors and the number of rows `N` of each of the samples `A` and `B`.
/// It serializes as its fields, and reads back through [`Sobol::new`]'s checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SobolData")]
pub struct Sobol {
    factors: Vec<Factor>,
    rows: usize,
}

/// The serialized form of a [`Sobol`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SobolData {
    factors: Vec<Factor>,
    rows: usize,
}

impl TryFrom<SobolData> for Sobol {
    type Error = AnalysisError;

    fn try_from(data: SobolData) -> Result<Self, AnalysisError> {
        Self::new(data.factors, data.rows)
    }
}

/// The points of a Sobol' analysis, row after row: `A`'s row, `B`'s, then `A_B⁽ⁱ⁾`'s for each
/// factor `i` in order, `k + 2` points a row. It is not serialized: [`Sobol::design`] rebuilds
/// it, bit for bit, from the analysis and its seed.
#[derive(Debug, Clone, PartialEq)]
pub struct SobolDesign {
    factors: Vec<Factor>,
    /// `A`'s rows then `B`'s, each `k` fractions of the factors' ranges.
    a: Vec<f64>,
    b: Vec<f64>,
}

/// A factor's indices and their standard errors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SobolIndex {
    /// The factor's name.
    pub name: String,
    /// Its first-order index, `Sᵢ`.
    pub first_order: f64,
    /// The standard error of `Sᵢ`.
    pub first_order_standard_error: f64,
    /// Its total index, `S_Tᵢ`.
    pub total: f64,
    /// The standard error of `S_Tᵢ`.
    pub total_standard_error: f64,
}

/// What a Sobol' analysis found.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SobolIndices {
    /// The rows `N` of each sample.
    pub rows: usize,
    /// The mean output over `A` and `B`.
    pub mean: f64,
    /// The output's variance over `A` and `B`, `V`, with `2N` in the denominator.
    pub variance: f64,
    /// Each factor's indices, in the factors' order.
    pub factors: Vec<SobolIndex>,
}

impl Sobol {
    /// An analysis of `factors` with `rows` rows in each of `A` and `B`.
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::TooFew`] with no factors, or fewer than 2 rows (a standard error needs
    ///   two).
    /// - [`AnalysisError::DuplicateFactor`] for two factors with one name.
    /// - [`AnalysisError::Count`] for a design of more than
    ///   [`MAX_DESIGN_POINTS`](super::MAX_DESIGN_POINTS) points or
    ///   [`MAX_DESIGN_VALUES`](super::MAX_DESIGN_VALUES) coordinates.
    pub fn new(factors: Vec<Factor>, rows: usize) -> Result<Self, AnalysisError> {
        check_factors(&factors)?;
        if rows < 2 {
            return Err(AnalysisError::TooFew {
                what: "Sobol' rows",
                count: rows,
                minimum: 2,
            });
        }
        let k = factors.len();
        check_size("Sobol' points", rows, k.saturating_add(2), k)?;
        Ok(Self { factors, rows })
    }

    /// The factors.
    pub fn factors(&self) -> &[Factor] {
        &self.factors
    }

    /// The rows `N` of each sample.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// The design seeded with `seed`. Row `j` draws from its own stream,
    /// [`SeededRng::for_stream`]`(seed, &[j])`, `k` uniform deviates for `A`'s row and then `k`
    /// for `B`'s, so it is the same however many rows are drawn.
    pub fn design(&self, seed: u64) -> SobolDesign {
        let k = self.factors.len();
        let mut a = Vec::with_capacity(self.rows * k);
        let mut b = Vec::with_capacity(self.rows * k);
        for j in 0..self.rows {
            // Cast: a row's index is far below 2⁶⁴.
            let mut rng = SeededRng::for_stream(seed, &[j as u64]);
            a.extend((0..k).map(|_| rng.uniform()));
            b.extend((0..k).map(|_| rng.uniform()));
        }
        SobolDesign {
            factors: self.factors.clone(),
            a,
            b,
        }
    }

    /// Draws the design seeded with `seed`, runs `model` at each of its points in order, and
    /// analyses the outputs ([`SobolDesign::analyse`]).
    ///
    /// # Errors
    ///
    /// As [`SobolDesign::analyse`].
    pub fn indices(
        &self,
        seed: u64,
        mut model: impl FnMut(&[f64]) -> f64,
    ) -> Result<SobolIndices, AnalysisError> {
        let design = self.design(seed);
        let outputs: Vec<f64> = design.points().iter().map(|x| model(x)).collect();
        design.analyse(&outputs)
    }
}

impl SobolDesign {
    /// The rows `N` of each sample.
    pub fn rows(&self) -> usize {
        self.a.len() / self.factors.len()
    }

    /// The number of points, `N (k + 2)`.
    pub fn len(&self) -> usize {
        self.rows() * (self.factors.len() + 2)
    }

    /// Whether there are no points; never, as an analysis has at least two rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The points, in physical units, row after row: `A`'s, `B`'s, then `A_B⁽ⁱ⁾`'s for each
    /// factor `i`.
    pub fn points(&self) -> Vec<Vec<f64>> {
        let k = self.factors.len();
        let physical = |unit: &[f64]| -> Vec<f64> {
            unit.iter()
                .zip(&self.factors)
                .map(|(&u, factor)| factor.at(u))
                .collect()
        };
        let mut points = Vec::with_capacity(self.len());
        for (a, b) in self.a.chunks_exact(k).zip(self.b.chunks_exact(k)) {
            let a = physical(a);
            let b = physical(b);
            for i in 0..k {
                let mut mixed = a.clone();
                mixed[i] = b[i];
                points.push(mixed);
            }
            // `A`'s and `B`'s rows go first: rotate them in front of the mixed ones.
            let first = points.len() - k;
            points.push(a);
            points.push(b);
            points[first..].rotate_right(2);
        }
        points
    }

    /// Each factor's indices from `outputs`, the model's output at each of
    /// [`SobolDesign::points`] in order (the module's docs give the estimates).
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::Length`] unless there is one output per point.
    /// - [`AnalysisError::Output`] for an output that isn't finite.
    /// - [`AnalysisError::Domain`] if the outputs of `A` and `B` are all the same, as with no
    ///   variance there is nothing to share out, or so spread that their variance overflows; or
    ///   if an index or its standard error overflows, from a mixed point's output far beyond the
    ///   others.
    pub fn analyse(&self, outputs: &[f64]) -> Result<SobolIndices, AnalysisError> {
        check_outputs(outputs, self.len())?;
        let k = self.factors.len();
        let rows: Vec<&[f64]> = outputs.chunks_exact(k + 2).collect();
        // Cast: a count of rows is far below 2⁵³.
        let n = rows.len() as f64;
        let shift = rows.iter().map(|r| r[0] + r[1]).sum::<f64>() / (2.0 * n);
        // Rows of shifted outputs: `A`'s, `B`'s, then the mixed ones.
        let rows: Vec<Vec<f64>> = rows
            .iter()
            .map(|r| r.iter().map(|y| y - shift).collect())
            .collect();
        let m: Vec<f64> = rows.iter().map(|r| 0.5 * (r[0] + r[1])).collect();
        let q: Vec<f64> = rows
            .iter()
            .map(|r| 0.5 * (r[0] * r[0] + r[1] * r[1]))
            .collect();
        let mean = |xs: &[f64]| xs.iter().sum::<f64>() / n;
        let big_m = mean(&m);
        let big_q = mean(&q);
        let variance = big_q - big_m * big_m;
        if !variance.is_finite() || variance <= 0.0 {
            return Err(AnalysisError::Domain {
                what: "output variance over A and B",
                value: variance,
            });
        }
        // Row j's influence on V, times −1/V: shared by every index.
        let on_variance: Vec<f64> = m
            .iter()
            .zip(&q)
            .map(|(&mj, &qj)| ((qj - big_q) - 2.0 * big_m * (mj - big_m)) / variance)
            .collect();
        let standard_error = |psi: &mut dyn Iterator<Item = f64>| {
            (psi.map(|x| x * x).sum::<f64>() / (n * (n - 1.0))).sqrt()
        };
        let factors: Vec<SobolIndex> = self
            .factors
            .iter()
            .enumerate()
            .map(|(i, factor)| {
                let first: Vec<f64> = rows.iter().map(|r| r[1] * (r[2 + i] - r[0])).collect();
                let differences: Vec<f64> = rows.iter().map(|r| r[2 + i] - r[0]).collect();
                let total: Vec<f64> = differences.iter().map(|d| 0.5 * d * d).collect();
                let big_p = mean(&first);
                let big_d = mean(&differences);
                let big_t = mean(&total);
                let s = big_p / variance;
                let st = big_t / variance;
                let first_order_standard_error =
                    standard_error(&mut first.iter().zip(&m).zip(&on_variance).map(
                        |((&pj, &mj), &v)| ((pj - big_p) - big_d * (mj - big_m)) / variance - s * v,
                    ));
                let total_standard_error = standard_error(
                    &mut total
                        .iter()
                        .zip(&on_variance)
                        .map(|(&tj, &v)| (tj - big_t) / variance - st * v),
                );
                SobolIndex {
                    name: factor.name().to_owned(),
                    first_order: s,
                    first_order_standard_error,
                    total: st,
                    total_standard_error,
                }
            })
            .collect();
        if let Some(value) = factors
            .iter()
            .flat_map(|f| {
                [
                    f.first_order,
                    f.first_order_standard_error,
                    f.total,
                    f.total_standard_error,
                ]
            })
            .find(|x| !x.is_finite())
        {
            return Err(AnalysisError::Domain {
                what: "Sobol' index or standard error (an output too large)",
                value,
            });
        }
        Ok(SobolIndices {
            rows: rows.len(),
            mean: shift + big_m,
            variance,
            factors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit_factors(k: usize) -> Vec<Factor> {
        (0..k)
            .map(|i| Factor::new(format!("x{i}"), 0.0, 1.0).unwrap())
            .collect()
    }

    #[test]
    fn an_analysis_refuses_what_it_cant_lay_out() {
        assert!(matches!(
            Sobol::new(Vec::new(), 10),
            Err(AnalysisError::TooFew {
                what: "factors",
                ..
            })
        ));
        assert!(matches!(
            Sobol::new(unit_factors(2), 1),
            Err(AnalysisError::TooFew {
                what: "Sobol' rows",
                count: 1,
                ..
            })
        ));
        assert!(matches!(
            Sobol::new(unit_factors(1), 1 << 61),
            Err(AnalysisError::Count {
                what: "Sobol' points",
                limit: crate::sensitivity::MAX_DESIGN_POINTS,
                ..
            })
        ));
        // 100 factors, 10,000 rows: 1,020,000 points, but 102,000,000 coordinates.
        assert!(matches!(
            Sobol::new(unit_factors(100), 10_000),
            Err(AnalysisError::Count {
                what: "design coordinates (points times factors)",
                count: 102_000_000,
                ..
            })
        ));
        let huge = r#"{"factors":[{"name":"x","low":0.0,"high":1.0}],"rows":2305843009213693952}"#;
        let refused = serde_json::from_str::<Sobol>(huge).unwrap_err().to_string();
        assert!(refused.contains("Sobol' points"), "{refused}");
    }

    #[test]
    fn each_row_holds_a_b_and_a_with_one_column_of_b() {
        let factors = vec![
            Factor::new("a", 0.0, 1.0).unwrap(),
            Factor::new("b", 10.0, 20.0).unwrap(),
            Factor::new("c", -1.0, 1.0).unwrap(),
        ];
        let design = Sobol::new(factors.clone(), 6).unwrap().design(5);
        let points = design.points();
        assert_eq!((points.len(), design.len(), design.rows()), (30, 30, 6));
        for row in points.as_chunks::<5>().0 {
            let (a, b) = (&row[0], &row[1]);
            for (x, f) in a.iter().chain(b).zip(factors.iter().cycle()) {
                assert!(*x >= f.low() && *x < f.high());
            }
            assert!(a.iter().zip(b).all(|(x, y)| x != y));
            for (i, mixed) in row[2..].iter().enumerate() {
                for j in 0..3 {
                    assert_eq!(mixed[j], if i == j { b[j] } else { a[j] });
                }
            }
        }
    }

    #[test]
    fn row_j_is_the_same_however_many_are_drawn() {
        let short = Sobol::new(unit_factors(3), 4).unwrap().design(9).points();
        let long = Sobol::new(unit_factors(3), 40).unwrap().design(9).points();
        assert_eq!(short[..], long[..short.len()]);
        let other = Sobol::new(unit_factors(3), 4).unwrap().design(10).points();
        assert_ne!(short, other);
    }

    #[test]
    fn an_additive_model_has_equal_first_order_and_total_indices() {
        // y = 2 x₀ + x₁ on [0, 1]²: shares 4/5 and 1/5, no interaction, so each pair of estimates
        // is equal row by row only in expectation; both converge on the shares.
        let indices = Sobol::new(unit_factors(2), 20_000)
            .unwrap()
            .indices(1, |x| 2.0 * x[0] + x[1])
            .unwrap();
        for (index, share) in indices.factors.iter().zip([0.8, 0.2]) {
            for (estimate, error) in [
                (index.first_order, index.first_order_standard_error),
                (index.total, index.total_standard_error),
            ] {
                assert!(error > 0.0 && error < 0.02, "{index:?}");
                assert!((estimate - share).abs() < 4.0 * error, "{index:?}");
            }
        }
        assert!((indices.mean - 1.5).abs() < 0.02);
        assert!((indices.variance - 5.0 / 12.0).abs() < 0.01);
    }

    #[test]
    fn adding_a_constant_changes_no_estimate_beyond_rounding() {
        let sobol = Sobol::new(unit_factors(2), 500).unwrap();
        let f = |x: &[f64]| x[0] * x[1] + x[0];
        let plain = sobol.indices(3, f).unwrap();
        let shifted = sobol.indices(3, |x| f(x) + 1.0e4).unwrap();
        for (a, b) in plain.factors.iter().zip(&shifted.factors) {
            assert!((a.first_order - b.first_order).abs() < 1e-9);
            assert!((a.first_order_standard_error - b.first_order_standard_error).abs() < 1e-9);
            assert!((a.total - b.total).abs() < 1e-9);
        }
        assert!((shifted.mean - plain.mean - 1.0e4).abs() < 1e-8);
    }

    /// The standard errors by another route: the gradient of each index as a function of raw
    /// (unshifted) row means, `Sᵢ = (R − M D)/(Q − M²)` with `R` the mean of `f(B)(f(A_B⁽ⁱ⁾) − f(A))`,
    /// and `S_Tᵢ = T/(Q − M²)`, times the rows' sample covariance: `√(gᵀ Σ g / N)`.
    #[test]
    fn standard_errors_are_the_delta_method_on_raw_row_means() {
        let f = |x: &[f64]| x[0] * x[1] + 2.0 * x[0] + x[2] * x[2] + 5.0;
        let design = Sobol::new(unit_factors(3), 1000).unwrap().design(4);
        let outputs: Vec<f64> = design.points().iter().map(|x| f(x)).collect();
        let indices = design.analyse(&outputs).unwrap();
        let rows: Vec<&[f64; 5]> = outputs.as_chunks::<5>().0.iter().collect();
        let n = rows.len() as f64;
        let mean = |v: &[f64]| v.iter().sum::<f64>() / n;
        let covariance = |a: &[f64], b: &[f64]| {
            let (ma, mb) = (mean(a), mean(b));
            a.iter()
                .zip(b)
                .map(|(x, y)| (x - ma) * (y - mb))
                .sum::<f64>()
                / (n - 1.0)
        };
        // gᵀ Σ g / N over the columns `zs`.
        let delta = |g: &[f64], zs: &[Vec<f64>]| {
            let mut var = 0.0;
            for (gi, zi) in g.iter().zip(zs) {
                for (gj, zj) in g.iter().zip(zs) {
                    var += gi * gj * covariance(zi, zj);
                }
            }
            (var / n).sqrt()
        };
        let m: Vec<f64> = rows.iter().map(|r| 0.5 * (r[0] + r[1])).collect();
        let q: Vec<f64> = rows
            .iter()
            .map(|r| 0.5 * (r[0] * r[0] + r[1] * r[1]))
            .collect();
        let (big_m, big_q) = (mean(&m), mean(&q));
        let v = big_q - big_m * big_m;
        assert!((v - indices.variance).abs() < 1e-12 * v);
        for (i, index) in indices.factors.iter().enumerate() {
            let d: Vec<f64> = rows.iter().map(|r| r[2 + i] - r[0]).collect();
            let r: Vec<f64> = rows.iter().zip(&d).map(|(row, d)| row[1] * d).collect();
            let t: Vec<f64> = d.iter().map(|d| 0.5 * d * d).collect();
            let (big_r, big_d, big_t) = (mean(&r), mean(&d), mean(&t));
            let s = (big_r - big_m * big_d) / v;
            let st = big_t / v;
            let g = [1.0 / v, -big_m / v, (2.0 * big_m * s - big_d) / v, -s / v];
            let zs = [r, d.clone(), m.clone(), q.clone()];
            let se = delta(&g, &zs);
            let gt = [1.0 / v, 2.0 * big_m * st / v, -st / v];
            let se_t = delta(&gt, &[t, m.clone(), q.clone()]);
            let close = |x: f64, y: f64| (x - y).abs() < 1e-9 * y.abs();
            assert!(close(index.first_order, s), "{index:?} against {s}");
            assert!(close(index.total, st), "{index:?} against {st}");
            assert!(
                close(index.first_order_standard_error, se),
                "{index:?} against {se}"
            );
            assert!(
                close(index.total_standard_error, se_t),
                "{index:?} against {se_t}"
            );
        }
    }

    #[test]
    fn outputs_whose_variance_overflows_are_refused() {
        let sobol = Sobol::new(unit_factors(2), 10).unwrap();
        let large = sobol.indices(1, |x| 1e150 * x[0]).unwrap();
        assert!(large.factors.iter().all(|f| f.first_order.is_finite()
            && f.total.is_finite()
            && f.first_order_standard_error.is_finite()
            && f.total_standard_error.is_finite()));
        match sobol.indices(1, |x| 1e160 * x[0]) {
            Err(AnalysisError::Domain { what, value }) => {
                assert_eq!(what, "output variance over A and B");
                assert!(value.is_infinite());
            }
            other => panic!("{other:?}"),
        }
        // A and B ordinary, a mixed point's output huge: the variance is 1, the total overflows.
        let design = Sobol::new(unit_factors(1), 2).unwrap().design(1);
        match design.analyse(&[1.0, -1.0, 1e160, -1.0, 1.0, -1e160]) {
            Err(AnalysisError::Domain { what, .. }) => {
                assert_eq!(what, "Sobol' index or standard error (an output too large)");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_constant_output_is_refused() {
        let sobol = Sobol::new(unit_factors(2), 10).unwrap();
        assert!(matches!(
            sobol.indices(1, |_| 3.0),
            Err(AnalysisError::Domain {
                what: "output variance over A and B",
                ..
            })
        ));
    }

    #[test]
    fn an_analysis_reads_back_through_its_checks() {
        let s = Sobol::new(unit_factors(2), 64).unwrap();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<Sobol>(&json).unwrap(), s);
        let few = json.replace("\"rows\":64", "\"rows\":1");
        let refused = serde_json::from_str::<Sobol>(&few).unwrap_err().to_string();
        assert!(refused.contains("Sobol' rows: 1 given"), "{refused}");
    }
}
