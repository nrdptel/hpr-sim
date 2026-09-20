//! A conical boattail's share of the normal force faster than sound, measured: W. D. Washington
//! and W. Pettis Jr., *Boattail Effects on Static Stability at Small Angles of Attack*, U.S. Army
//! Missile Command report RD-TM-68-5 (1968; DTIC AD-695658, approved for public release).
//!
//! Washington and Pettis mounted a model's aft section on its own balance and measured it with a
//! boattail and as a plain cylinder of the same length, at Mach 1.75 to 4.5 (models T0 to T4), and
//! a whole model with and without a boattail (No. 1) from Mach 0.8 to 4.5, Mach 0.8 to 1.5 of it
//! in a second tunnel (pp. 1 to 2). The
//! boattail's increment, `ΔC_Nα = C_Nα(with) − C_Nα(without)` (eq. 1), correlates with
//!
//! `ΔC_Nα / [1 − (D_B/D)²] = F(√|M² − 1| / (L_B/D))` (Fig. 5, printed p. 8),
//!
//! per degree on the cylinder's area `πD²/4`, with `D` the cylinder's diameter, `D_B` the
//! boattail's base diameter and `L_B` its length. Its centre of pressure is "approximately 50
//! percent of its length" (abstract), from about 43% at Mach 2 to 64% at 4.5 (Fig. 6, printed
//! p. 9). hpr reads both figures by hand from the page images ([`WP_PARAMETERS`],
//! [`WP_SLOPE_PER_DEG`], [`WP_CENTRE_MACHS`], [`WP_CENTRE_FRACTION`]).
//!
//! **What it covers.** The models were conical boattails "with diameter ratios of 0.72 to 0.86
//! and angles from 4 to 10 degrees" (p. 1), 0.82 to 1.18 diameters long, behind a cylinder; the
//! correlation's points reach the peak near zero argument, with the next near 0.3 and the rest
//! out to about 5.4; the curve is drawn to 6.1. Past 6.1 hpr
//! holds its end, an extrapolation, as is any boattail steeper, shorter, **longer** or narrower
//! than those tested: a long one reads the curve near zero argument, which comes from the
//! report's lowest supersonic runs. The data scatter about the curve by up to about 15%.
//! Slender-body theory gives a boattail `2[(D_B/D)² − 1]` per radian at any Mach number (Munk's
//! value, which the report plots for comparison at subsonic speeds, p. 3). The measured curve
//! runs from 0.23 to 1.58 times it, crossing at an argument of 0.635; it is 0.24 to 0.47 of it at
//! the Arcas Robin's Mach numbers.
//!
//! See `docs/physics/aero.md` (*The body faster than sound in a flight*).

use std::f64::consts::PI;

use crate::error::{AeroError, check_dimension};

/// The argument of Fig. 5's correlation, `√(M² − 1)/(L_B/D)`, at [`WP_SLOPE_PER_DEG`]'s rows.
pub const WP_PARAMETERS: [f64; 24] = [
    0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0,
    4.5, 5.0, 5.5, 6.0, 6.1,
];

/// `ΔC_Nα / [1 − (D_B/D)²]`, per degree on the cylinder's area, at [`WP_PARAMETERS`]: RD-TM-68-5
/// Fig. 5, the supersonic side of the faired curve, read by tracing it on a 300-dpi scan to about
/// ±0.0003 per degree (±0.02 per radian). At 0 (Mach 1) the curve peaks near −0.055, past the
/// axis's last tick; from there to about 1.2 it follows model No. 1's points, the report's lowest
/// supersonic runs. Their Mach numbers are not read off here: the report gives model No. 1 as
/// Mach 0.8 to 4.5, 0.8 to 1.5 of it in the AEDC tunnel (p. 1).
pub const WP_SLOPE_PER_DEG: [f64; 24] = [
    -0.05500, -0.05291, -0.05080, -0.04818, -0.04486, -0.04100, -0.03681, -0.03131, -0.02531,
    -0.02186, -0.01985, -0.01876, -0.01726, -0.01614, -0.01429, -0.01299, -0.01181, -0.01089,
    -0.01013, -0.00946, -0.00886, -0.00841, -0.00820, -0.00816,
];

/// The Mach numbers of [`WP_CENTRE_FRACTION`].
pub const WP_CENTRE_MACHS: [f64; 8] = [1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 4.8];

/// The boattail's centre of pressure as a share of its length from its fore end, at
/// [`WP_CENTRE_MACHS`]: the middle of RD-TM-68-5 Fig. 6's band (printed p. 9), read by tracing
/// its two edges to about ±0.01; the band is about ±0.03 wide. Held outside Mach 1.5 to 4.8.
pub const WP_CENTRE_FRACTION: [f64; 8] = [0.430, 0.430, 0.449, 0.479, 0.524, 0.580, 0.643, 0.684];

/// Linear interpolation in `xs` (increasing), holding the end values outside them.
fn held_linear(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let x = x.clamp(xs[0], xs[xs.len() - 1]);
    let i = xs.partition_point(|&c| c <= x).clamp(1, xs.len() - 1);
    let w = (x - xs[i - 1]) / (xs[i] - xs[i - 1]);
    (1.0 - w) * ys[i - 1] + w * ys[i]
}

/// Fig. 5's argument, `√(M² − 1)/(L_B/D)`, for a boattail `length_over_diameter` long in its fore
/// diameters at Mach `mach` (at least 1).
pub fn wp_parameter(mach: f64, length_over_diameter: f64) -> f64 {
    (mach * mach - 1.0).max(0.0).sqrt() / length_over_diameter
}

/// A conical boattail's normal-force slope increment at `α → 0`, per radian on the area
/// `π fore_radius_m²`, at Mach `mach` (at least 1): negative, from Fig. 5's correlation
/// ([`WP_SLOPE_PER_DEG`]).
///
/// # Errors
///
/// [`AeroError::Domain`] for a Mach number below 1 or not finite, a length that isn't finite and
/// positive, an aft radius that isn't below the fore radius (not a boattail) or is negative, or
/// dimensions whose ratios overflow.
pub fn wp_slope(
    mach: f64,
    fore_radius_m: f64,
    aft_radius_m: f64,
    length_m: f64,
) -> Result<f64, AeroError> {
    if !(mach.is_finite() && mach >= 1.0) {
        return Err(AeroError::Domain {
            what: "Mach number for the boattail correlation",
            value: mach,
        });
    }
    check_dimension("boattail length", length_m, false)?;
    check_dimension("boattail fore radius", fore_radius_m, false)?;
    if !(aft_radius_m.is_finite() && aft_radius_m >= 0.0 && aft_radius_m < fore_radius_m) {
        return Err(AeroError::Domain {
            what: "boattail aft radius",
            value: aft_radius_m,
        });
    }
    let ratio = aft_radius_m / fore_radius_m;
    let per_deg = held_linear(
        &WP_PARAMETERS,
        &WP_SLOPE_PER_DEG,
        wp_parameter(mach, length_m / (2.0 * fore_radius_m)),
    );
    let slope = per_deg * (180.0 / PI) * (1.0 - ratio * ratio);
    // Finite inputs whose ratios overflow (an enormous Mach number over an enormous length) leave
    // no parameter to read the curve at.
    if slope.is_finite() {
        Ok(slope)
    } else {
        Err(AeroError::Domain {
            what: "boattail correlation parameter",
            value: wp_parameter(mach, length_m / (2.0 * fore_radius_m)),
        })
    }
}

/// The boattail's centre of pressure at Mach `mach`, as a share of its length from its fore end
/// ([`WP_CENTRE_FRACTION`]).
pub fn wp_centre_fraction(mach: f64) -> f64 {
    let m = if mach.is_nan() {
        WP_CENTRE_MACHS[0]
    } else {
        mach
    };
    held_linear(&WP_CENTRE_MACHS, &WP_CENTRE_FRACTION, m)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INCH: f64 = 0.0254;

    #[test]
    fn overflowing_ratios_are_refused() {
        // Finite inputs, but `√(M² − 1)` and `L/D` both overflow and their ratio is NaN.
        assert!(matches!(
            wp_slope(1e200, 1e-300, 0.0, 1e300),
            Err(AeroError::Domain { .. })
        ));
    }

    #[test]
    fn the_tables_are_well_formed() {
        assert!(WP_PARAMETERS.windows(2).all(|w| w[0] < w[1]));
        // The curve rises (toward zero) all the way from its peak at Mach 1.
        assert!(WP_SLOPE_PER_DEG.windows(2).all(|w| w[0] < w[1]));
        assert!(WP_CENTRE_MACHS.windows(2).all(|w| w[0] < w[1]));
        assert!(WP_CENTRE_FRACTION.windows(2).all(|w| w[0] <= w[1]));
        // Munk's (slender-body) value, −2 per radian, is −0.0349 per degree: the dashed line
        // on Fig. 5 at the subsonic side. The supersonic curve falls below it in size from
        // just past Mach 1.
        let munk = -2.0 * PI / 180.0;
        assert!((munk - -0.034_91).abs() < 1e-5);
        assert!(WP_SLOPE_PER_DEG[0] < munk && WP_SLOPE_PER_DEG[7] > munk);
    }

    /// The Arcas Robin's 15° boattail (TN D-4014 Fig. 1(a): 2.25 in to 1.308 in across over
    /// 1.757 in) at the report's Mach numbers, pinned, and between the two theories M1.8e5's
    /// research note brackets it with: footnote 8's −0.177 to −0.026 and slender-body theory's
    /// −1.324 per radian on the body's cross-section.
    #[test]
    fn the_arcas_robins_boattail_lies_between_the_two_theories() {
        let (fore, aft, length) = (1.125 * INCH, 0.654 * INCH, 1.757 * INCH);
        let slender = 2.0 * ((aft / fore).powi(2) - 1.0);
        assert!((slender - -1.324).abs() < 2e-3);
        let pinned = [
            (1.5, -0.6219),
            (1.8, -0.5538),
            (2.3, -0.4791),
            (2.96, -0.4092),
            (3.96, -0.3403),
            (4.63, -0.3144),
        ];
        for (mach, want) in pinned {
            let got = wp_slope(mach, fore, aft, length).unwrap();
            assert!((got - want).abs() < 1e-4, "Mach {mach}: {got}");
            assert!(got > slender && got < -0.177, "Mach {mach}: {got}");
        }
    }

    #[test]
    fn it_is_continuous_and_held_past_the_curve() {
        let (fore, aft) = (0.05, 0.04);
        // From Mach 1.2, where hpr's supersonic join starts. At Mach 1 itself the argument
        // `√(M² − 1)` rises with infinite slope, so a step of 1e-9 moves it by about 4e-5.
        for mach in [1.2, 1.5, 2.0, 3.0, 4.0, 4.99] {
            let a = wp_slope(mach, fore, aft, 0.1).unwrap();
            let b = wp_slope(mach + 1e-9, fore, aft, 0.1).unwrap();
            assert!((a - b).abs() < 1e-8, "Mach {mach}");
        }
        // A short boattail at Mach 5: past the curve's end, held.
        let short = wp_slope(5.0, fore, aft, 0.005).unwrap();
        let end = -0.00816 * (180.0 / PI) * (1.0 - 0.64);
        assert!((short - end).abs() < 1e-12);
        assert_eq!(wp_centre_fraction(1.2), 0.430);
        assert_eq!(wp_centre_fraction(6.0), 0.684);
    }

    #[test]
    fn it_refuses_what_isnt_a_supersonic_boattail() {
        assert!(wp_slope(0.9, 0.05, 0.04, 0.1).is_err());
        assert!(wp_slope(f64::NAN, 0.05, 0.04, 0.1).is_err());
        assert!(wp_slope(2.0, 0.05, 0.05, 0.1).is_err());
        assert!(wp_slope(2.0, 0.05, 0.06, 0.1).is_err());
        assert!(wp_slope(2.0, 0.05, -0.01, 0.1).is_err());
        assert!(wp_slope(2.0, 0.05, 0.04, 0.0).is_err());
        // To a point: the full base area, still negative and finite.
        assert!(wp_slope(2.0, 0.05, 0.0, 0.1).unwrap() < 0.0);
    }
}
