//! Adaptive numerical integration of vector-valued functions over a finite interval.
//!
//! [`integrate`] applies the 15-point Gauss–Kronrod rule (G7K15) on each subinterval and bisects
//! the subinterval with the largest error estimate until every component meets its tolerance. It
//! is QUADPACK's globally adaptive `QAG` strategy (R. Piessens, E. de Doncker-Kapenga,
//! C. Überhuber and D. Kahaner, *QUADPACK: A Subroutine Package for Automatic Integration*,
//! Springer, 1983, §2.2 and §3.3) without its extrapolation step, with the rule's nodes and
//! weights from QUADPACK's `qk15` (public domain).
//!
//! On `[a, b]` with centre `c` and half-width `h`, the Kronrod estimate and its embedded Gauss
//! estimate are
//!
//! ```text
//! K = h Σ_{i=0}^{14} w_i f(c + h x_i),        G = h Σ_{j=0}^{6} v_j f(c + h x_{2j+1})
//! ```
//!
//! `K` is exact for polynomials up to degree 22 and `G` up to degree 13. The error estimate of a
//! subinterval is `|K − G|`, which overestimates the error of `K` for smooth integrands. The
//! integral converges when, for every component `k`,
//!
//! ```text
//! Σ_intervals |K_k − G_k| ≤ max(absolute, relative · |Σ_intervals K_k|)
//! ```
//!
//! Bisection copes with integrable endpoint singularities such as `√x` or `x^(−1/2)` and with
//! kinks inside the interval, at the cost of more subintervals; callers should split the interval
//! at kinks they know about. `docs/physics/quadrature.md` has the tests that pin the rule.

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Gauss–Kronrod nodes on `[0, 1)`, largest first; the rule is symmetric about 0, which is the
/// last node. Nodes 1, 3 and 5 are the positive Gauss–Legendre nodes of order 7.
#[expect(
    clippy::excessive_precision,
    reason = "the published digits, which round to the nearest f64"
)]
const NODES: [f64; 8] = [
    0.991_455_371_120_812_639_206_854_697_526_329,
    0.949_107_912_342_758_524_526_189_684_047_851,
    0.864_864_423_359_769_072_789_712_788_640_926,
    0.741_531_185_599_394_439_863_864_773_280_788,
    0.586_087_235_467_691_130_294_144_845_693_013,
    0.405_845_151_377_397_166_906_606_412_076_961,
    0.207_784_955_007_898_467_600_689_403_773_245,
    0.0,
];

/// Kronrod weights for [`NODES`].
#[expect(
    clippy::excessive_precision,
    reason = "the published digits, which round to the nearest f64"
)]
const KRONROD_WEIGHTS: [f64; 8] = [
    0.022_935_322_010_529_224_963_732_008_058_970,
    0.063_092_092_629_978_553_290_700_663_189_204,
    0.104_790_010_322_250_183_839_876_322_541_518,
    0.140_653_259_715_525_918_745_189_590_510_238,
    0.169_004_726_639_267_902_826_583_426_598_550,
    0.190_350_578_064_785_409_913_256_402_421_014,
    0.204_432_940_075_298_892_414_161_999_234_649,
    0.209_482_141_084_727_828_012_999_174_891_714,
];

/// Gauss weights for nodes 1, 3, 5 and 7 of [`NODES`].
#[expect(
    clippy::excessive_precision,
    reason = "the published digits, which round to the nearest f64"
)]
const GAUSS_WEIGHTS: [f64; 4] = [
    0.129_484_966_168_869_693_270_611_432_679_082,
    0.279_705_391_489_276_667_901_467_771_423_780,
    0.381_830_050_505_118_944_950_369_775_488_975,
    0.417_959_183_673_469_387_755_102_040_816_327,
];

/// When an adaptive integral stops.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Tolerance {
    /// Relative error bound on each component, as a fraction of its magnitude.
    pub relative: f64,
    /// Absolute error bound on each component, for components whose value is near zero.
    pub absolute: f64,
    /// The most subintervals to use before giving up.
    pub max_intervals: usize,
}

impl Default for Tolerance {
    /// `1e-12` relative and absolute, with up to 4000 subintervals: tight enough for mass
    /// properties computed from integrands scaled to order one.
    fn default() -> Self {
        Self {
            relative: 1e-12,
            absolute: 1e-12,
            max_intervals: 4000,
        }
    }
}

/// A converged integral.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Integral<const N: usize> {
    /// The integral of each component.
    #[serde(with = "serde_arrays")]
    pub value: [f64; N],
    /// The summed error estimate of each component.
    #[serde(with = "serde_arrays")]
    pub error: [f64; N],
    /// The number of subintervals used.
    pub intervals: usize,
}

/// Serializes fixed-size arrays of any length as sequences (serde's derive covers only 0 to 32).
mod serde_arrays {
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer, const N: usize>(
        values: &[f64; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        values.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<[f64; N], D::Error> {
        let values = Vec::<f64>::deserialize(deserializer)?;
        let got = values.len();
        values
            .try_into()
            .map_err(|_| D::Error::custom(format!("expected {N} values, got {got}")))
    }
}

/// One subinterval with its Kronrod estimate and error estimate.
#[derive(Debug, Clone, Copy)]
struct Piece<const N: usize> {
    a: f64,
    b: f64,
    value: [f64; N],
    error: [f64; N],
}

/// Applies G7K15 on `[a, b]`.
fn rule<const N: usize, F>(f: &mut F, a: f64, b: f64) -> Result<Piece<N>, CoreError>
where
    F: FnMut(f64) -> [f64; N],
{
    let centre = 0.5 * (a + b);
    let half = 0.5 * (b - a);
    let mut kronrod = [0.0; N];
    let mut gauss = [0.0; N];
    let mut add = |x: f64, kw: f64, gw: f64| -> Result<(), CoreError> {
        let y = f(x);
        for k in 0..N {
            if !y[k].is_finite() {
                return Err(CoreError::QuadratureNotFinite { x });
            }
            kronrod[k] += kw * y[k];
            gauss[k] += gw * y[k];
        }
        Ok(())
    };
    add(centre, KRONROD_WEIGHTS[7], GAUSS_WEIGHTS[3])?;
    for (i, (&node, &kw)) in NODES.iter().zip(&KRONROD_WEIGHTS).take(7).enumerate() {
        let gw = if i % 2 == 1 {
            GAUSS_WEIGHTS[i / 2]
        } else {
            0.0
        };
        add(centre - half * node, kw, gw)?;
        add(centre + half * node, kw, gw)?;
    }
    let mut value = [0.0; N];
    let mut error = [0.0; N];
    for k in 0..N {
        value[k] = half * kronrod[k];
        error[k] = (half * (kronrod[k] - gauss[k])).abs();
    }
    Ok(Piece { a, b, value, error })
}

/// Integrates the vector-valued `f` over `[a, b]` to `tolerance`.
///
/// `a > b` integrates backwards (the result changes sign) and `a == b` gives zeros. Integrands
/// whose components differ in scale should be scaled to order one first, so one absolute
/// tolerance suits them all.
///
/// # Errors
///
/// - [`CoreError::Domain`] if `a` or `b` is not finite, or the tolerance is negative or NaN, or
///   allows no subinterval.
/// - [`CoreError::QuadratureNotFinite`] if `f` returns NaN or an infinity (for example at an
///   endpoint singularity the rule's nodes happen to reach; the nodes never include the ends).
/// - [`CoreError::QuadratureDidNotConverge`] if `tolerance.max_intervals` subintervals don't meet
///   the tolerance.
pub fn integrate<const N: usize, F>(
    mut f: F,
    a: f64,
    b: f64,
    tolerance: Tolerance,
) -> Result<Integral<N>, CoreError>
where
    F: FnMut(f64) -> [f64; N],
{
    for (what, value) in [
        ("integration lower limit", a),
        ("integration upper limit", b),
    ] {
        if !value.is_finite() {
            return Err(CoreError::Domain { what, value });
        }
    }
    for (what, value) in [
        ("relative tolerance", tolerance.relative),
        ("absolute tolerance", tolerance.absolute),
    ] {
        if value.is_nan() || value < 0.0 {
            return Err(CoreError::Domain { what, value });
        }
    }
    if tolerance.max_intervals == 0 {
        return Err(CoreError::Domain {
            what: "maximum number of subintervals",
            value: 0.0,
        });
    }
    if a == b {
        return Ok(Integral {
            value: [0.0; N],
            error: [0.0; N],
            intervals: 1,
        });
    }
    // Roundoff sets a floor under any requested relative tolerance.
    let relative = tolerance.relative.max(50.0 * f64::EPSILON);
    let mut pieces = vec![rule(&mut f, a, b)?];
    loop {
        let mut total = [0.0; N];
        let mut error = [0.0; N];
        for piece in &pieces {
            for k in 0..N {
                total[k] += piece.value[k];
                error[k] += piece.error[k];
            }
        }
        let bound: [f64; N] =
            std::array::from_fn(|k| tolerance.absolute.max(relative * total[k].abs()));
        if (0..N).all(|k| error[k] <= bound[k]) {
            return Ok(Integral {
                value: total,
                error,
                intervals: pieces.len(),
            });
        }
        // The subinterval contributing most to the worst component's excess.
        let score = |piece: &Piece<N>| {
            (0..N)
                .map(|k| piece.error[k] / bound[k].max(f64::MIN_POSITIVE))
                .fold(0.0, f64::max)
        };
        let worst = (0..pieces.len())
            .max_by(|&i, &j| score(&pieces[i]).total_cmp(&score(&pieces[j])))
            .unwrap_or(0);
        let piece = pieces[worst];
        let middle = 0.5 * (piece.a + piece.b);
        let tiny = middle == piece.a || middle == piece.b;
        if pieces.len() >= tolerance.max_intervals || tiny {
            let worst_component = (0..N)
                .max_by(|&i, &j| {
                    (error[i] / bound[i].max(f64::MIN_POSITIVE))
                        .total_cmp(&(error[j] / bound[j].max(f64::MIN_POSITIVE)))
                })
                .unwrap_or(0);
            return Err(CoreError::QuadratureDidNotConverge {
                component: worst_component,
                value: total.get(worst_component).copied().unwrap_or(0.0),
                error: error.get(worst_component).copied().unwrap_or(0.0),
                intervals: pieces.len(),
            });
        }
        pieces[worst] = rule(&mut f, piece.a, middle)?;
        pieces.push(rule(&mut f, middle, piece.b)?);
    }
}

/// Integrates the scalar `f` over `[a, b]` to `tolerance`; see [`integrate`].
///
/// # Errors
///
/// As [`integrate`].
pub fn integrate_scalar<F>(mut f: F, a: f64, b: f64, tolerance: Tolerance) -> Result<f64, CoreError>
where
    F: FnMut(f64) -> f64,
{
    integrate(|x| [f(x)], a, b, tolerance).map(|integral| integral.value[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Legendre polynomial `P_n(x)` by the three-term recurrence.
    fn legendre(n: usize, x: f64) -> f64 {
        let (mut p0, mut p1) = (1.0, x);
        for k in 1..n {
            let k = k as f64;
            let p2 = ((2.0 * k + 1.0) * x * p1 - k * p0) / (k + 1.0);
            p0 = p1;
            p1 = p2;
        }
        if n == 0 { p0 } else { p1 }
    }

    #[test]
    fn the_nodes_and_weights_are_the_published_rule() {
        // The Gauss nodes are the roots of P_7, and both weight sets integrate 1 to 2.
        for j in [1, 3, 5, 7] {
            assert!(legendre(7, NODES[j]).abs() < 1e-15, "node {j}");
        }
        let kronrod_sum: f64 = 2.0 * KRONROD_WEIGHTS[..7].iter().sum::<f64>() + KRONROD_WEIGHTS[7];
        let gauss_sum: f64 = 2.0 * GAUSS_WEIGHTS[..3].iter().sum::<f64>() + GAUSS_WEIGHTS[3];
        assert!((kronrod_sum - 2.0).abs() < 1e-15);
        assert!((gauss_sum - 2.0).abs() < 1e-15);
    }

    #[test]
    fn one_panel_is_exact_to_the_rules_degrees() {
        // On [-1, 1]: ∫ x^k = 2/(k+1) for even k and 0 for odd k.
        for degree in 0..=24 {
            let exact = if degree % 2 == 0 {
                2.0 / (degree as f64 + 1.0)
            } else {
                0.0
            };
            let piece = rule(&mut |x: f64| [x.powi(degree)], -1.0, 1.0).unwrap();
            let kronrod_error = (piece.value[0] - exact).abs();
            if degree <= 22 {
                assert!(
                    kronrod_error < 2e-16,
                    "K15 degree {degree}: {kronrod_error}"
                );
            } else if degree % 2 == 0 {
                // Odd powers vanish by symmetry at any degree.
                assert!(kronrod_error > 1e-12, "K15 is not exact at degree {degree}");
            }
            // The Gauss estimate is K minus the signed error, so check it through the error.
            if degree <= 13 {
                assert!(
                    piece.error[0] < 2e-16,
                    "G7 degree {degree}: {}",
                    piece.error[0]
                );
            } else if degree % 2 == 0 {
                assert!(piece.error[0] > 1e-8, "G7 is not exact at degree {degree}");
            }
        }
        // An asymmetric interval: ∫_1^3 x^22 dx = (3^23 − 1)/23.
        let piece = rule(&mut |x: f64| [x.powi(22)], 1.0, 3.0).unwrap();
        let exact = (3f64.powi(23) - 1.0) / 23.0;
        assert!((piece.value[0] / exact - 1.0).abs() < 1e-14);
    }

    #[test]
    fn smooth_singular_and_kinked_integrands_converge() {
        /// An integrand with its limits and exact integral.
        type Case<'a> = (&'a dyn Fn(f64) -> f64, f64, f64, f64);
        let tol = Tolerance::default();
        let cases: [Case; 6] = [
            (&|x: f64| x.exp(), 0.0, 1.0, std::f64::consts::E - 1.0),
            (&|x: f64| x.sin(), 0.0, std::f64::consts::PI, 2.0),
            // Endpoint singularities of the kinds nose-cone profiles produce.
            (&|x: f64| x.sqrt(), 0.0, 1.0, 2.0 / 3.0),
            (&|x: f64| 1.0 / x.sqrt(), 0.0, 1.0, 2.0),
            (&|x: f64| x.powf(0.3), 0.0, 2.0, 2f64.powf(1.3) / 1.3),
            // A kink away from the subdivision points.
            (&|x: f64| (x - 0.3).abs(), 0.0, 1.0, 0.5 * (0.09 + 0.49)),
        ];
        for (i, (f, a, b, exact)) in cases.iter().enumerate() {
            let value = integrate_scalar(f, *a, *b, tol).unwrap();
            let relative = ((value - exact) / exact).abs();
            assert!(
                relative < 1e-11,
                "case {i}: {value} vs {exact} ({relative:e})"
            );
        }
    }

    #[test]
    fn components_converge_together_and_limits_can_be_reversed() {
        let integral = integrate(
            |x: f64| [1.0, x, x * x, (x * 10.0).sin()],
            0.0,
            2.0,
            Tolerance::default(),
        )
        .unwrap();
        let exact = [2.0, 2.0, 8.0 / 3.0, (1.0 - 20f64.cos()) / 10.0];
        for (k, want) in exact.iter().enumerate() {
            assert!((integral.value[k] - want).abs() < 1e-12, "component {k}");
            assert!(integral.error[k] <= 1e-12f64.max(1e-12 * want.abs()));
        }
        let backwards = integrate_scalar(|x| x * x, 2.0, 0.0, Tolerance::default()).unwrap();
        assert!((backwards + 8.0 / 3.0).abs() < 1e-14);
        assert_eq!(
            integrate(|_| [1.0, 2.0], 1.5, 1.5, Tolerance::default())
                .unwrap()
                .value,
            [0.0, 0.0]
        );
    }

    #[test]
    fn bad_inputs_and_hard_integrands_are_errors() {
        let tol = Tolerance::default();
        assert!(matches!(
            integrate_scalar(|x| x, f64::NAN, 1.0, tol),
            Err(CoreError::Domain { .. })
        ));
        assert!(matches!(
            integrate_scalar(
                |x| x,
                0.0,
                1.0,
                Tolerance {
                    relative: -1.0,
                    ..tol
                }
            ),
            Err(CoreError::Domain { .. })
        ));
        assert!(matches!(
            integrate_scalar(|x| if x > 0.5 { f64::NAN } else { x }, 0.0, 1.0, tol),
            Err(CoreError::QuadratureNotFinite { .. })
        ));
        // 1/x on (0, 1] diverges, so bisection runs out of subintervals.
        let limited = Tolerance {
            max_intervals: 50,
            ..tol
        };
        assert!(matches!(
            integrate_scalar(|x| 1.0 / x, 0.0, 1.0, limited),
            Err(CoreError::QuadratureDidNotConverge { intervals: 50, .. })
        ));
    }

    #[test]
    fn a_result_serializes_as_plain_arrays() {
        let integral = integrate(|x| [x, 1.0], 0.0, 1.0, Tolerance::default()).unwrap();
        let json = serde_json::to_string(&integral).unwrap();
        let back: Integral<2> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, integral);
        assert!(serde_json::from_str::<Integral<3>>(&json).is_err());
    }
}
