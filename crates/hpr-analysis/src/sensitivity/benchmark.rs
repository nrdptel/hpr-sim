//! Test functions whose Sobol' indices are known in closed form, to check an analysis against.
//!
//! **Guide:** [Sensitivity analysis][guide]'s *Checked against* section shows both methods on
//! them.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/sensitivity.html#checked-against
//!
//! - [`Ishigami`]: `y = sin x₁ + a sin² x₂ + b x₃⁴ sin x₁` on `[−π, π]³`. It is nonlinear and
//!   not additive: `x₃` does nothing alone (`S₃ = 0`) but a quarter of the variance with `x₁`.
//! - [`SobolG`]: `y = Πᵢ (|4xᵢ − 2| + aᵢ)/(1 + aᵢ)` on `[0, 1]^k`. Each `aᵢ ≥ 0` sets how much
//!   factor `i` matters, from most at `aᵢ = 0` to almost nothing at `aᵢ = 99`.
//!
//! Each evaluates a point given as a slice, the form [`super::morris`] and [`super::sobol`] pass,
//! and gives `NaN` for a point of the wrong length, which an analysis then refuses.

use std::f64::consts::PI;

use serde::{Deserialize, Serialize};

use super::Factor;
use crate::error::AnalysisError;

/// Ishigami and Homma's function, `y = sin x₁ + a sin² x₂ + b x₃⁴ sin x₁`, each `xᵢ` uniform on
/// `[−π, π]`.
///
/// Its mean is `a/2` and its variance and the parts of it are (I. M. Sobol' and Y. L. Levitan,
/// "On the use of variance reducing multipliers in Monte Carlo computations of a global
/// sensitivity index", *Computer Physics Communications* 117, 52–61, 1999,
/// <https://doi.org/10.1016/S0010-4655(98)00156-8>, p. 57, §5; A. Saltelli and others, *Global
/// Sensitivity Analysis: The Primer*, Wiley, 2008, pp. 179–182)
///
/// - `V = 1/2 + a²/8 + bπ⁴/5 + b²π⁸/18`,
/// - `V₁ = (1 + bπ⁴/5)²/2`, `V₂ = a²/8`, `V₃ = 0`,
/// - `V₁₃ = (1/18 − 1/50) b²π⁸ = 8b²π⁸/225`, every other part zero,
///
/// so `Sᵢ = Vᵢ/V`, `S_T₁ = (V₁ + V₁₃)/V`, `S_T₂ = V₂/V` and `S_T₃ = V₁₃/V`. The usual
/// parameters, [`Ishigami::STANDARD`], are `a = 7`, `b = 0.1`. T. Ishigami and T. Homma, "An
/// importance quantification technique in uncertainty analysis for computer models", *Proceedings
/// of ISUMA '90*, 398–403, 1990, <https://doi.org/10.1109/ISUMA.1990.151285>, introduced it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Ishigami {
    /// The weight of `sin² x₂`.
    pub a: f64,
    /// The weight of `x₃⁴ sin x₁`.
    pub b: f64,
}

impl Ishigami {
    /// `a = 7`, `b = 0.1`.
    pub const STANDARD: Self = Self { a: 7.0, b: 0.1 };

    /// Its three factors, `x1` to `x3`, each on `[−π, π]`.
    ///
    /// # Errors
    ///
    /// None in practice; [`Factor::new`]'s, as the type requires.
    pub fn factors() -> Result<Vec<Factor>, AnalysisError> {
        ["x1", "x2", "x3"]
            .into_iter()
            .map(|name| Factor::new(name, -PI, PI))
            .collect()
    }

    /// `y` at `x = [x₁, x₂, x₃]`; `NaN` for a slice of another length.
    pub fn evaluate(&self, x: &[f64]) -> f64 {
        let &[x1, x2, x3] = x else {
            return f64::NAN;
        };
        let s2 = x2.sin();
        x1.sin() + self.a * s2 * s2 + self.b * x3.powi(4) * x1.sin()
    }

    /// Its mean, `a/2`.
    pub fn mean(&self) -> f64 {
        0.5 * self.a
    }

    /// Its variance, `V`.
    pub fn variance(&self) -> f64 {
        let (a, b) = (self.a, self.b);
        0.5 + a * a / 8.0 + b * PI.powi(4) / 5.0 + b * b * PI.powi(8) / 18.0
    }

    /// `V₁₃`, the part of the variance `x₁` and `x₃` cause together.
    fn v13(&self) -> f64 {
        8.0 * self.b * self.b * PI.powi(8) / 225.0
    }

    /// The first-order indices, `S₁`, `S₂`, `S₃`.
    pub fn first_order(&self) -> [f64; 3] {
        let v = self.variance();
        let v1 = 0.5 * (1.0 + self.b * PI.powi(4) / 5.0).powi(2);
        let v2 = self.a * self.a / 8.0;
        [v1 / v, v2 / v, 0.0]
    }

    /// The total indices, `S_T₁`, `S_T₂`, `S_T₃`.
    pub fn total(&self) -> [f64; 3] {
        let [s1, s2, _] = self.first_order();
        let s13 = self.v13() / self.variance();
        [s1 + s13, s2, s13]
    }
}

/// Sobol's g function, `y = Πᵢ gᵢ(xᵢ)` with `gᵢ = (|4xᵢ − 2| + aᵢ)/(1 + aᵢ)`, each `xᵢ` uniform
/// on `[0, 1]`.
///
/// Each `gᵢ` has mean 1 and variance `Vᵢ = 1/(3 (1 + aᵢ)²)`, so (A. Saltelli and others, 2010,
/// the module [`super::sobol`]'s reference, Appendix A, p. 268, eqs. (31) and (32))
///
/// - `V = Πᵢ (1 + Vᵢ) − 1`, computed as `exp(Σᵢ ln(1 + Vᵢ)) − 1` with `ln_1p` and `exp_m1`, so
///   that a large `aᵢ` (a tiny `Vᵢ`) isn't lost in the `1 +`,
/// - `Sᵢ = Vᵢ/V`, and
/// - `S_Tᵢ = Vᵢ Πⱼ≠ᵢ (1 + Vⱼ) / V`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SobolGData")]
pub struct SobolG {
    a: Vec<f64>,
}

/// The serialized form of a [`SobolG`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SobolGData {
    a: Vec<f64>,
}

impl TryFrom<SobolGData> for SobolG {
    type Error = AnalysisError;

    fn try_from(data: SobolGData) -> Result<Self, AnalysisError> {
        Self::new(data.a)
    }
}

impl SobolG {
    /// The function with one factor for each of `a`.
    ///
    /// # Errors
    ///
    /// - [`AnalysisError::TooFew`] for an empty `a`.
    /// - [`AnalysisError::Domain`] for an `aᵢ` that isn't finite and at least zero.
    pub fn new(a: Vec<f64>) -> Result<Self, AnalysisError> {
        if a.is_empty() {
            return Err(AnalysisError::TooFew {
                what: "g function's factors",
                count: 0,
                minimum: 1,
            });
        }
        if let Some(&value) = a.iter().find(|&&ai| !(ai.is_finite() && ai >= 0.0)) {
            return Err(AnalysisError::Domain {
                what: "g function's a (finite, at least 0)",
                value,
            });
        }
        Ok(Self { a })
    }

    /// The `aᵢ`.
    pub fn a(&self) -> &[f64] {
        &self.a
    }

    /// Its factors, `x1` to `xk`, each on `[0, 1]`.
    ///
    /// # Errors
    ///
    /// None in practice; [`Factor::new`]'s, as the type requires.
    pub fn factors(&self) -> Result<Vec<Factor>, AnalysisError> {
        (1..=self.a.len())
            .map(|i| Factor::new(format!("x{i}"), 0.0, 1.0))
            .collect()
    }

    /// `y` at `x`; `NaN` for a slice whose length isn't the number of factors.
    pub fn evaluate(&self, x: &[f64]) -> f64 {
        if x.len() != self.a.len() {
            return f64::NAN;
        }
        x.iter()
            .zip(&self.a)
            .map(|(&xi, &ai)| ((4.0 * xi - 2.0).abs() + ai) / (1.0 + ai))
            .product()
    }

    /// Its mean, 1.
    pub fn mean(&self) -> f64 {
        1.0
    }

    /// Each factor's part of the variance alone, `Vᵢ = 1/(3 (1 + aᵢ)²)`.
    fn parts(&self) -> Vec<f64> {
        self.a
            .iter()
            .map(|ai| 1.0 / (3.0 * (1.0 + ai) * (1.0 + ai)))
            .collect()
    }

    /// Its variance, `V = Πᵢ (1 + Vᵢ) − 1`.
    pub fn variance(&self) -> f64 {
        self.log_product().exp_m1()
    }

    /// `ln Πᵢ (1 + Vᵢ)`.
    fn log_product(&self) -> f64 {
        self.parts().iter().map(|v| v.ln_1p()).sum()
    }

    /// The first-order indices, `Sᵢ = Vᵢ/V`.
    pub fn first_order(&self) -> Vec<f64> {
        let v = self.variance();
        self.parts().iter().map(|vi| vi / v).collect()
    }

    /// The total indices, `S_Tᵢ = Vᵢ Πⱼ≠ᵢ (1 + Vⱼ) / V`.
    pub fn total(&self) -> Vec<f64> {
        let log_all = self.log_product();
        let v = log_all.exp_m1();
        self.parts()
            .iter()
            .map(|vi| vi * (log_all - vi.ln_1p()).exp() / v)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ishigamis_indices_are_the_published_ones() {
        // Sobol' and Levitan's b = 0.05 case (p. 59, Example 6.3): S₁ = 0.219, S₂ = 0.687,
        // S₁₃ = 0.0946, to their printing.
        let f = Ishigami { a: 7.0, b: 0.05 };
        let [s1, s2, s3] = f.first_order();
        let [t1, _, t3] = f.total();
        assert!((s1 - 0.219).abs() < 5e-4, "{s1}");
        assert!((s2 - 0.687).abs() < 5e-4, "{s2}");
        assert_eq!(s3, 0.0);
        assert!((t3 - 0.0946).abs() < 5e-5, "{t3}");
        assert!((t1 - s1 - t3).abs() < 1e-15);
        // a = 7, b = 0.1: S₁ = 0.3139, S₂ = 0.4424 (Marrel and others, 2009, quoting them).
        let [s1, s2, _] = Ishigami::STANDARD.first_order();
        assert!((s1 - 0.3139).abs() < 5e-5 && (s2 - 0.4424).abs() < 5e-5);
        // The parts add up to the whole.
        let f = Ishigami::STANDARD;
        let [s1, s2, _] = f.first_order();
        let [_, _, t3] = f.total();
        assert!((s1 + s2 + t3 - 1.0).abs() < 1e-15);
    }

    #[test]
    fn ishigamis_mean_and_variance_are_its_integrals() {
        // Midpoint rule on a 120³ grid: smooth and periodic in x₁ and x₂, so it converges fast;
        // x₃⁴ is not periodic, so allow its O(h²) error.
        let f = Ishigami::STANDARD;
        let n = 120;
        let h = 2.0 * PI / f64::from(n);
        let (mut sum, mut squares) = (0.0, 0.0);
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    let x = |m: i32| -PI + (f64::from(m) + 0.5) * h;
                    let y = f.evaluate(&[x(i), x(j), x(k)]);
                    sum += y;
                    squares += y * y;
                }
            }
        }
        let count = f64::from(n).powi(3);
        let mean = sum / count;
        let variance = squares / count - mean * mean;
        assert!((mean - f.mean()).abs() < 1e-9, "{mean}");
        assert!(
            (variance - f.variance()).abs() / f.variance() < 1e-3,
            "{variance}"
        );
    }

    #[test]
    fn the_g_functions_indices_add_up_and_order_by_a() {
        let g = SobolG::new(vec![0.0, 1.0, 4.5, 9.0, 99.0, 99.0, 99.0, 99.0]).unwrap();
        let s = g.first_order();
        let t = g.total();
        assert!(s.iter().sum::<f64>() < 1.0);
        assert!(t.iter().sum::<f64>() > 1.0);
        for i in 0..8 {
            assert!(s[i] <= t[i]);
        }
        for i in 0..3 {
            assert!(t[i] > t[i + 1]);
        }
        // A huge a: its tiny part isn't lost in 1 + Vᵢ.
        let tiny = SobolG::new(vec![1e9]).unwrap();
        let part = 1.0 / (3.0 * (1.0 + 1e9) * (1.0 + 1e9));
        assert!((tiny.variance() / part - 1.0).abs() < 1e-12);
        assert!((tiny.first_order()[0] - 1.0).abs() < 1e-12);
        // One factor: all its variance is its own, 1/3 at a = 0.
        let one = SobolG::new(vec![0.0]).unwrap();
        assert!((one.variance() - 1.0 / 3.0).abs() < 1e-16);
        // (1 + 1/3) − 1 is not 1/3 in floating point, so V and Vᵢ differ in the last bit.
        assert!((one.first_order()[0] - 1.0).abs() < 1e-15);
        assert!((one.total()[0] - 1.0).abs() < 1e-15);
    }

    #[test]
    fn the_g_functions_mean_and_variance_are_its_integrals() {
        // Two factors on a 4000² midpoint grid: |4x − 2| is linear on each half, so the rule is
        // exact on each cell but the kink's, which falls on a cell edge.
        let g = SobolG::new(vec![0.5, 2.0]).unwrap();
        let n = 4000;
        let h = 1.0 / f64::from(n);
        let (mut sum, mut squares) = (0.0, 0.0);
        for i in 0..n {
            for j in 0..n {
                let y = g.evaluate(&[(f64::from(i) + 0.5) * h, (f64::from(j) + 0.5) * h]);
                sum += y;
                squares += y * y;
            }
        }
        let count = f64::from(n * n);
        let mean = sum / count;
        let variance = squares / count - mean * mean;
        assert!((mean - 1.0).abs() < 1e-12, "{mean}");
        assert!((variance - g.variance()).abs() < 1e-7, "{variance}");
    }

    #[test]
    fn a_point_of_the_wrong_length_is_nan_and_bad_parameters_are_refused() {
        assert!(Ishigami::STANDARD.evaluate(&[0.0, 0.0]).is_nan());
        let g = SobolG::new(vec![1.0, 2.0]).unwrap();
        assert!(g.evaluate(&[0.5]).is_nan());
        assert_eq!(g.evaluate(&[0.5, 0.5]), (1.0 / 2.0) * (2.0 / 3.0));
        assert!(matches!(
            SobolG::new(vec![]),
            Err(AnalysisError::TooFew {
                what: "g function's factors",
                count: 0,
                minimum: 1
            })
        ));
        match SobolG::new(vec![1.0, -0.5]) {
            Err(AnalysisError::Domain { value, .. }) => assert_eq!(value, -0.5),
            other => panic!("{other:?}"),
        }
        let refused = serde_json::from_str::<SobolG>(r#"{"a":[1.0,-1.0]}"#)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("g function's a"), "{refused}");
        assert_eq!(
            serde_json::from_str::<SobolG>(r#"{"a":[1.0,2.0]}"#).unwrap(),
            g
        );
    }
}
