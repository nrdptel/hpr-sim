//! The standard normal distribution's cumulative distribution function `Φ` and its inverse, for
//! the integer variables' margin ([`super::cmaes`]).
//!
//! `Φ(x) = erfc(−x/√2)/2`, with `erfc` from the `libm` crate (a port of musl's, itself
//! FreeBSD's), which keeps the lower tail's relative accuracy where `1 − erf` would cancel.
//! `Φ⁻¹` is M. J. Wichura's algorithm AS 241, "The Percentage Points of the Normal
//! Distribution", *Applied Statistics* 37(3), 477–484 (1988), <https://doi.org/10.2307/2347330>:
//! its `PPND16`, three rational functions of degree 7, accurate to about 1 part in 10¹⁶.

/// `Φ(x)`, the chance that a standard normal draw is at most `x`.
pub(crate) fn cdf(x: f64) -> f64 {
    0.5 * libm::erfc(-x / std::f64::consts::SQRT_2)
}

/// The numerator's and denominator's coefficients of AS 241's three rational functions, highest
/// power first, as the paper prints them; each denominator's constant term is 1.
const CENTRAL: ([f64; 8], [f64; 7]) = (
    [
        2.509_080_928_730_122_7e3,
        3.343_057_558_358_812_8e4,
        6.726_577_092_700_870_1e4,
        4.592_195_393_154_987_1e4,
        1.373_169_376_550_946_1e4,
        1.971_590_950_306_551_4e3,
        1.331_416_678_917_843_8e2,
        3.387_132_872_796_366_6,
    ],
    [
        5.226_495_278_852_854_6e3,
        2.872_908_573_572_194_3e4,
        3.930_789_580_009_271_1e4,
        2.121_379_430_158_659_6e4,
        5.394_196_021_424_751_1e3,
        6.871_870_074_920_579_1e2,
        4.231_333_070_160_091_1e1,
    ],
);

/// AS 241's function for `√(−ln min(p, 1 − p))` from 1.6 to 5.
const INTERMEDIATE: ([f64; 8], [f64; 7]) = (
    [
        7.745_450_142_783_414_1e-4,
        2.272_384_498_926_918_5e-2,
        2.417_807_251_774_506_1e-1,
        1.270_458_252_452_368_4,
        3.647_848_324_763_204_6,
        5.769_497_221_460_691_4,
        4.630_337_846_156_545_3,
        1.423_437_110_749_683_6,
    ],
    [
        1.050_750_071_644_416_8e-9,
        5.475_938_084_995_344_9e-4,
        1.519_866_656_361_645_7e-2,
        1.481_039_764_274_800_7e-1,
        6.897_673_349_851_000_0e-1,
        1.676_384_830_183_803_8,
        2.053_191_626_637_758_8,
    ],
);

/// AS 241's function for `√(−ln min(p, 1 − p))` past 5.
const TAIL: ([f64; 8], [f64; 7]) = (
    [
        2.010_334_399_292_288_1e-7,
        2.711_555_568_743_487_6e-5,
        1.242_660_947_388_078_4e-3,
        2.653_218_952_657_612_3e-2,
        2.965_605_718_285_048_9e-1,
        1.784_826_539_917_291_3,
        5.463_784_911_164_114_4,
        6.657_904_643_501_103_8,
    ],
    [
        2.044_263_103_389_939_8e-15,
        1.421_511_758_316_445_9e-7,
        1.846_318_317_510_054_7e-5,
        7.868_691_311_456_132_6e-4,
        1.487_536_129_085_061_5e-2,
        1.369_298_809_227_358_1e-1,
        5.998_322_065_558_879_4e-1,
    ],
);

/// A rational function by Horner's rule: `numerator(r)/denominator(r)`, the denominator's
/// constant term 1.
fn rational((numerator, denominator): &([f64; 8], [f64; 7]), r: f64) -> f64 {
    let top = numerator.iter().fold(0.0, |acc, c| acc * r + c);
    let bottom = denominator.iter().fold(0.0, |acc, c| acc * r + c) * r + 1.0;
    top / bottom
}

/// `Φ⁻¹(p)`, the `x` with `Φ(x) = p`: `−∞` at 0, `+∞` at 1, NaN outside `[0, 1]`.
pub(crate) fn quantile(p: f64) -> f64 {
    if !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    let q = p - 0.5;
    if q.abs() <= 0.425 {
        return q * rational(&CENTRAL, 0.180_625 - q * q);
    }
    let tail = if q < 0.0 { p } else { 1.0 - p };
    if tail == 0.0 {
        return if q < 0.0 {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    let r = (-tail.ln()).sqrt();
    let x = if r <= 5.0 {
        rational(&INTERMEDIATE, r - 1.6)
    } else {
        rational(&TAIL, r - 5.0)
    };
    if q < 0.0 { -x } else { x }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The paper's "hash sums", printed to check a transcription: each function's 15
    /// coefficients written as `d.ddd… × 10ᵏ`, their mantissas `d.ddd…` added (the denominator's
    /// constant 1 left out): 55.8831928806149014439, 49.3320650330161028904 and
    /// 47.5258331754928967163.
    #[test]
    fn coefficients_match_the_papers_hash_sums() {
        let mantissa = |c: &f64| c / 10f64.powf(c.log10().floor());
        let sum = |(top, bottom): &([f64; 8], [f64; 7])| {
            top.iter().map(mantissa).sum::<f64>() + bottom.iter().map(mantissa).sum::<f64>()
        };
        for (function, hash) in [
            (&CENTRAL, 55.883_192_880_614_901_443_9),
            (&INTERMEDIATE, 49.332_065_033_016_102_890_4),
            (&TAIL, 47.525_833_175_492_896_716_3),
        ] {
            let got = sum(function);
            assert!((got - hash).abs() <= 1e-13 * hash, "{got} against {hash}");
        }
    }

    /// Quantiles to 17 digits, solved for in 40-digit arithmetic (mpmath's `findroot` on
    /// `ncdf`) at the `f64` nearest each `p`.
    #[test]
    fn known_quantiles() {
        for (p, x) in [
            (0.5, 0.0),
            (0.975, 1.959_963_984_540_053_9),
            (0.025, -1.959_963_984_540_054_2),
            (0.841_344_746_068_542_9, 0.999_999_999_999_999_8),
            (1e-10, -6.361_340_902_404_056_2),
            (1e-300, -37.047_096_299_361_199),
            (0.999, 3.090_232_306_167_813_5),
        ] {
            let got = quantile(p);
            assert!(
                (got - x).abs() <= 1e-14 * x.abs().max(1.0),
                "Φ⁻¹({p}) = {got}, not {x}"
            );
        }
        assert_eq!(quantile(0.0), f64::NEG_INFINITY);
        assert_eq!(quantile(1.0), f64::INFINITY);
        assert!(
            quantile(-1e-300).is_nan() && quantile(1.5).is_nan() && quantile(f64::NAN).is_nan()
        );
    }

    /// `Φ(Φ⁻¹(p)) = p` across the range, in the lower tail to its relative accuracy.
    #[test]
    fn cdf_inverts_the_quantile() {
        for k in 1..2000 {
            let p = f64::from(k) / 2000.0;
            let back = cdf(quantile(p));
            assert!((back - p).abs() <= 4e-16, "Φ(Φ⁻¹({p})) = {back}");
        }
        // In the tail a relative error δ in `x` moves `Φ(x)` by about `x² δ` of itself.
        for e in 1..=300 {
            let p = 10f64.powi(-e);
            let x = quantile(p);
            let back = cdf(x);
            assert!(
                (back - p).abs() <= 1e-15 * (1.0 + x * x) * p,
                "Φ(Φ⁻¹({p})) = {back}"
            );
        }
        assert_eq!(cdf(0.0), 0.5);
        assert!((cdf(1.0) - 0.841_344_746_068_542_9).abs() <= 2e-16);
    }
}
