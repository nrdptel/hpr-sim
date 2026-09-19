//! Body lift: the viscous crossflow term of Jorgensen's method for bodies of revolution at an
//! angle of attack (L. H. Jorgensen, NASA TR R-474, 1977), and Galejs's constant it replaces.
//!
//! At an angle of attack `α` the air crosses the body sideways at `V sin α`, separates behind it as
//! it would behind a cylinder in a cross-wind, and pushes it with the drag of that crossflow:
//!
//! `C_N = η C_dn (A_p/A_r) sin² α` (TR R-474 eq. 2.12, printed p. 10),
//!
//! with `A_p` the body's planform (side-view) area, `A_r` the reference area, `C_dn` the
//! crossflow drag coefficient of an infinitely long circular cylinder and `η` the ratio of a
//! finite cylinder's crossflow drag to an infinite one's. Both depend on the crossflow Mach number
//! `M_n = M sin α` (eq. 2.3, p. 8); `η` also on the body's length over its diameter. The force acts
//! at the planform's centroid (eq. 2.21, p. 13). hpr takes each factor from Jorgensen's figures,
//! read by hand from the page images:
//!
//! - **`C_dn`** ([`CROSSFLOW_DRAG`], Fig. 1, printed p. 75) below the critical crossflow Reynolds
//!   number, where "C_dn = 1.2" at low `M_n` (p. 15). From `M_n` 0.6 to 1.2 it takes the filled
//!   points "extrapolated from data obtained in Ames 2' × 2' wind tunnel", the values Fig. 6 was
//!   divided by (below); past 1.4, the faired curve through the experiments, to 4.8.
//! - **`η` against length over diameter** ([`ETA_BY_FINENESS`], Fig. 4, printed p. 77): the
//!   circular cylinder at a crossflow Reynolds number of 88,000, measured "only at very low
//!   subsonic Mach numbers" (p. 17).
//! - **`η` against `M_n`** ([`ETA_BY_CROSSFLOW_MACH`], Fig. 6, printed p. 78): Jorgensen's `η C_dn`
//!   back-computed from the measured normal force of two bodies of fineness 10 and 12 at 45° to
//!   60° (his Fig. 5), divided by Fig. 1's `C_dn`, at the eleven crossflow Mach numbers from 0.4
//!   to 1.6 he computed; below 0.4 it runs to Fig. 4's value for those bodies. He uses Figs. 5
//!   and 6 "in lieu of better information" (p. 18); past 1.6, `η` "probably can be assumed to be
//!   unity" (p. 17), and hpr holds the last point, 0.984.
//!
//! **Combining the two `η`s, a judgement.** Fig. 6 holds for bodies of fineness 10 to 12 only.
//! For another fineness `f`, hpr scales Fig. 6's `η` by how much longer or shorter Fig. 4 makes
//! the body, and lets that scaling fade as the crossflow speeds up, by the share `s` Fig. 6's own
//! bodies have risen toward 1:
//!
//! `η(f, M_n) = η₆(M_n) [η₄(f) + (1 − η₄(f)) r] / [η₆(0) + (1 − η₆(0)) r]`,
//!
//! `s = [η₆(M_n) − η₆(0)] / [1 − η₆(0)]` and `r` its running maximum over `[0, M_n]`, with
//! `η₆(0)` = 0.69, midway between Fig. 6's starting points for fineness 10 and 12. `r` never
//! falls back: Fig. 6 dips at `M_n` = 1 only because Jorgensen divided by Fig. 1's peak there, not
//! because the body's length counts again. The rule gives Fig. 6 back for a body of fineness about
//! 10.6 (where this reading of Fig. 4 gives 0.69), Fig. 4 at `M_n = 0` for any fineness, and
//! Fig. 5's `η C_dn` for every fineness once `M_n` passes 0.8, where Fig. 6 reaches 0.99. Where
//! `r = s`, below `M_n` 0.8, it equals `η₄ + (1 − η₄) s`.
//!
//! **Sampling, not smoothing.** Fig. 1's `C_dn` peaks at `M_n` ≈ 0.96 and Fig. 6's `η` dips at
//! 1.0; each is steep there. hpr samples both at Fig. 6's points and interpolates each linearly
//! between them, so their product is Jorgensen's own `η C_dn` at those points (his Fig. 5, within
//! the reading, test `the_product_follows_figure_5`) and moves smoothly between them, instead of
//! multiplying two steep curves read separately.
//!
//! **Left out.** Past the critical crossflow Reynolds number (about 2 × 10⁵, Fig. 2, p. 76) a
//! cylinder's `C_dn` falls to "between about 0.15 and 0.30" at low `M_n` (p. 15); Jorgensen
//! computes that only for illustration, with nothing to check it against (p. 27), and hpr leaves
//! it out. hpr's potential-flow term stays its own (`sin α`, slender-body theory or TN 3527's
//! method), not Jorgensen's `sin 2α cos(α/2)`.
//!
//! **Galejs's constant** ([`BodyLift::Galejs`]): hpr's body lift until the milestone that sized it
//! ([M1.8e6](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e6)) was
//! `K (A_plan/A_ref) sin² α` with `K` = 1.1 at every Mach number (R. Galejs, *Wind Instability*,
//! after Hoerner; Niskanen 2009 eq. 3.26), kept to reproduce earlier results.
//!
//! See `docs/physics/aero.md` (*Body lift*).

use serde::{Deserialize, Serialize};

use crate::body::BODY_LIFT_K;
use crate::error::AeroError;

/// The crossflow Mach numbers `M_n = M sin α` of [`CROSSFLOW_DRAG`].
pub const CROSSFLOW_DRAG_MACHS: [f64; 26] = [
    0.0, 0.2, 0.3, 0.35, 0.4, 0.45, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.4, 1.5, 1.6, 1.8,
    2.0, 2.4, 2.8, 3.2, 3.6, 4.0, 4.4, 4.8,
];

/// A circular cylinder's crossflow drag coefficient `C_dn` at [`CROSSFLOW_DRAG_MACHS`], below the
/// critical crossflow Reynolds number: NASA TR R-474, Fig. 1 (printed p. 75), read by hand to
/// about ±0.01. To 0.2, the "C_dn = 1.2" of p. 15; to 0.5, the curve through Lindsey's points;
/// from 0.6 to 1.2, the filled points extrapolated from the Ames 2' × 2' tunnel; from 1.4, the
/// curve through the supersonic experiments. Held past 4.8.
pub const CROSSFLOW_DRAG: [f64; 26] = [
    1.20, 1.20, 1.21, 1.237, 1.271, 1.305, 1.334, 1.458, 1.552, 1.515, 1.560, 1.985, 1.785, 1.676,
    1.555, 1.530, 1.489, 1.440, 1.403, 1.363, 1.336, 1.320, 1.305, 1.285, 1.272, 1.266,
];

/// The crossflow Mach numbers of [`ETA_BY_CROSSFLOW_MACH`]: `M_n` = 0 and Fig. 6's eleven
/// computed points.
pub const ETA_MACHS: [f64; 12] = [0.0, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1, 1.2, 1.4, 1.6];

/// Jorgensen's `η` against the crossflow Mach number for bodies of fineness 10 and 12, at
/// [`ETA_MACHS`]: NASA TR R-474, Fig. 6 (printed p. 78), the circles "computed from figures 1 and
/// 5", read by hand to about ±0.005. At 0, [`ETA_REFERENCE`]. Held past 1.6.
pub const ETA_BY_CROSSFLOW_MACH: [f64; 12] = [
    ETA_REFERENCE,
    0.717,
    0.804,
    0.815,
    0.845,
    0.994,
    0.979,
    0.769,
    0.910,
    0.937,
    0.985,
    0.984,
];

/// Fig. 6's `η` at `M_n = 0`: 0.69, midway between its square and diamond there, about 0.68 and
/// 0.70, which Jorgensen takes from Fig. 4 for its two bodies of fineness 10 and 12 (this module's
/// own reading of Fig. 4, [`ETA_BY_FINENESS`], gives 0.685 and 0.701).
pub const ETA_REFERENCE: f64 = 0.69;

/// The fineness ratios (length over diameter) of [`ETA_BY_FINENESS`].
pub const ETA_FINENESS: [f64; 12] = [
    2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0,
];

/// A finite circular cylinder's crossflow drag over an infinite one's, `η`, at [`ETA_FINENESS`],
/// at very low crossflow Mach number: NASA TR R-474, Fig. 4 (printed p. 77), the curve for a
/// circular cylinder at a crossflow Reynolds number of 88,000 (from Goldstein), read by hand to
/// about ±0.005. Held outside 2 to 40.
pub const ETA_BY_FINENESS: [f64; 12] = [
    0.577, 0.607, 0.643, 0.668, 0.685, 0.701, 0.724, 0.753, 0.775, 0.795, 0.805, 0.815,
];

/// How a body's crossflow lift is sized: its `C_N = factor · (A_plan/A_ref) sin² α`. In JSON,
/// `{"kind": "jorgensen"}` or `{"kind": "galejs", "k": 1.1}`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum BodyLift {
    /// Jorgensen's `η C_dn` ([`crossflow_factor`]), from the body's fineness and the crossflow
    /// Mach number: hpr's model since body lift was sized ([M1.8e6](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e6)).
    Jorgensen {},
    /// Galejs's constant `K` at every Mach number: hpr's model before, with `k` =
    /// [`BODY_LIFT_K`] (1.1). Galejs gives 1.0 to 1.5.
    Galejs {
        /// `K`, dimensionless.
        k: f64,
    },
}

impl Default for BodyLift {
    /// Jorgensen's, hpr's current model.
    fn default() -> Self {
        Self::JORGENSEN
    }
}

impl BodyLift {
    /// Jorgensen's `η C_dn`, hpr's current model.
    pub const JORGENSEN: Self = Self::Jorgensen {};

    /// hpr's model before Jorgensen's: Galejs's `K` = [`BODY_LIFT_K`].
    pub const GALEJS: Self = Self::Galejs { k: BODY_LIFT_K };

    /// The factor on `(A_plan/A_ref) sin² α` for a body of fineness `fineness` (length over
    /// diameter) at crossflow Mach number `crossflow_mach` (`M sin α`).
    pub fn factor(&self, fineness: f64, crossflow_mach: f64) -> f64 {
        match *self {
            Self::Jorgensen {} => crossflow_factor(fineness, crossflow_mach),
            Self::Galejs { k } => k,
        }
    }

    /// Checks the model's own number.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a `K` that isn't finite and non-negative.
    pub fn validate(&self) -> Result<(), AeroError> {
        match *self {
            Self::Jorgensen {} => Ok(()),
            Self::Galejs { k } if k.is_finite() && k >= 0.0 => Ok(()),
            Self::Galejs { k } => Err(AeroError::Domain {
                what: "body-lift K",
                value: k,
            }),
        }
    }
}

/// Linear interpolation in `xs` (increasing), holding the end values outside them.
fn held_linear(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let x = x.clamp(xs[0], xs[xs.len() - 1]);
    let i = xs.partition_point(|&c| c <= x).clamp(1, xs.len() - 1);
    let w = (x - xs[i - 1]) / (xs[i] - xs[i - 1]);
    (1.0 - w) * ys[i - 1] + w * ys[i]
}

/// A circular cylinder's crossflow drag coefficient `C_dn` at crossflow Mach number
/// `crossflow_mach`, below the critical Reynolds number ([`CROSSFLOW_DRAG`]). A negative or NaN
/// input reads as 0.
pub fn crossflow_drag(crossflow_mach: f64) -> f64 {
    held_linear(
        &CROSSFLOW_DRAG_MACHS,
        &CROSSFLOW_DRAG,
        finite_or_zero(crossflow_mach),
    )
}

/// Fig. 4's `η` for a body of fineness `fineness`, at low crossflow Mach number
/// ([`ETA_BY_FINENESS`]).
pub fn crossflow_eta_low(fineness: f64) -> f64 {
    held_linear(&ETA_FINENESS, &ETA_BY_FINENESS, finite_or_zero(fineness))
}

/// `η` for a body of fineness `fineness` at crossflow Mach number `crossflow_mach`: Fig. 6's value
/// scaled by Fig. 4's for the body's length, the scaling fading as Fig. 6 rises toward 1 (see the
/// module's *Combining the two `η`s*).
pub fn crossflow_eta(fineness: f64, crossflow_mach: f64) -> f64 {
    eta_from_low(crossflow_eta_low(fineness), crossflow_mach)
}

/// [`crossflow_eta`] from Fig. 4's `low` for the body.
fn eta_from_low(low: f64, crossflow_mach: f64) -> f64 {
    let n = ETA_MACHS.len();
    let m = finite_or_zero(crossflow_mach).clamp(ETA_MACHS[0], ETA_MACHS[n - 1]);
    // The rows at or below `m`: at least the first, since `m` is at least its Mach number.
    let below = ETA_MACHS.partition_point(|&c| c <= m).clamp(1, n);
    let i = below.clamp(1, n - 1);
    let w = (m - ETA_MACHS[i - 1]) / (ETA_MACHS[i] - ETA_MACHS[i - 1]);
    let eta6 = (1.0 - w) * ETA_BY_CROSSFLOW_MACH[i - 1] + w * ETA_BY_CROSSFLOW_MACH[i];
    // How far Fig. 6's `η` has risen toward 1 by `m`, never falling back: Fig. 6's dip at
    // `M_n` = 1 comes from dividing by Fig. 1's peak there, not from the body's length, so the
    // length's effect doesn't return with it.
    let risen = RISEN_SHARE[below - 1].max(share(eta6));
    eta6 * (low + (1.0 - low) * risen) / (ETA_REFERENCE + (1.0 - ETA_REFERENCE) * risen)
}

/// The share of the way from Fig. 6's low-speed `η` to 1: `(η − η₆(0))/(1 − η₆(0))`.
const fn share(eta: f64) -> f64 {
    (eta - ETA_REFERENCE) / (1.0 - ETA_REFERENCE)
}

/// The running maximum of [`share`] over Fig. 6's rows up to each: how far its `η` has risen by
/// then, never falling back.
const RISEN_SHARE: [f64; ETA_BY_CROSSFLOW_MACH.len()] = {
    let mut risen = [0.0; ETA_BY_CROSSFLOW_MACH.len()];
    let mut best = 0.0;
    let mut i = 0;
    while i < ETA_BY_CROSSFLOW_MACH.len() {
        let s = share(ETA_BY_CROSSFLOW_MACH[i]);
        if s > best {
            best = s;
        }
        risen[i] = best;
        i += 1;
    }
    risen
};

/// Jorgensen's `η C_dn` for a body of fineness `fineness` at crossflow Mach number
/// `crossflow_mach`: the factor on `(A_plan/A_ref) sin² α` in its body lift.
pub fn crossflow_factor(fineness: f64, crossflow_mach: f64) -> f64 {
    crossflow_factor_from_eta_low(crossflow_eta_low(fineness), crossflow_mach)
}

/// [`crossflow_factor`] from the body's Fig. 4 `η` ([`crossflow_eta_low`]), which a model
/// computes once.
pub fn crossflow_factor_from_eta_low(eta_low: f64, crossflow_mach: f64) -> f64 {
    eta_from_low(eta_low, crossflow_mach) * crossflow_drag(crossflow_mach)
}

fn finite_or_zero(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn the_tables_are_well_formed() {
        for xs in [&CROSSFLOW_DRAG_MACHS[..], &ETA_MACHS, &ETA_FINENESS] {
            assert!(xs.windows(2).all(|w| w[0] < w[1]), "{xs:?}");
        }
        assert!(CROSSFLOW_DRAG.iter().all(|&c| (1.19..=2.0).contains(&c)));
        assert!(
            ETA_BY_CROSSFLOW_MACH
                .iter()
                .all(|&e| (0.68..1.0).contains(&e))
        );
        assert!(ETA_BY_FINENESS.windows(2).all(|w| w[0] < w[1]));
        // Every Fig. 6 point from 0.4 has a Fig. 1 value at the same crossflow Mach number, so
        // their product is Jorgensen's own there.
        for m in &ETA_MACHS[1..] {
            assert!(CROSSFLOW_DRAG_MACHS.contains(m), "{m}");
        }
        // From 0.6 to 1.2, where both curves are steep, Fig. 1 has no node but Fig. 6's: no
        // steep feature of one meets a straight line of the other.
        for m in CROSSFLOW_DRAG_MACHS
            .iter()
            .filter(|&&m| (0.6..=1.2).contains(&m))
        {
            assert!(ETA_MACHS.contains(m), "{m}");
        }
    }

    /// Jorgensen's Fig. 5 (printed p. 78), `η C_dn` back-computed from two bodies of fineness 10
    /// and 12 at 45° to 60°, read by hand from its faired curve. Fig. 6's points times Fig. 1's
    /// are his division of these, so the product at a body of Fig. 6's own fineness returns them
    /// within the two readings.
    #[test]
    fn the_product_follows_figure_5() {
        let fig5 = [
            (0.5, 1.08),
            (0.6, 1.20),
            (0.7, 1.33),
            (0.8, 1.52),
            (0.9, 1.55),
            (1.0, 1.52),
            (1.1, 1.63),
            (1.2, 1.61),
            (1.4, 1.51),
            (1.6, 1.46),
        ];
        // Fineness where Fig. 4 gives Fig. 6's own starting value, about 10.6.
        let f = 10.0 + 2.0 * (ETA_REFERENCE - 0.685) / (0.701 - 0.685);
        assert!((f - 10.625).abs() < 1e-12);
        for (m, want) in fig5 {
            let got = crossflow_factor(f, m);
            assert!(
                ((got - want) / want).abs() < 0.03,
                "M_n {m}: {got} against Fig. 5's {want}"
            );
        }
    }

    #[test]
    fn low_crossflow_mach_is_figure_4_times_1_2() {
        for (&f, &eta) in ETA_FINENESS.iter().zip(&ETA_BY_FINENESS) {
            assert!((crossflow_eta(f, 0.0) - eta).abs() < 1e-15);
            assert!((crossflow_factor(f, 0.0) - 1.2 * eta).abs() < 1e-15);
        }
        // Up to Mach 0.8, where Fig. 6 only rises, the rule is `η₄ + (1 − η₄) s`.
        for m in [0.1, 0.3, 0.45, 0.62, 0.79] {
            let s = (held_linear(&ETA_MACHS, &ETA_BY_CROSSFLOW_MACH, m) - ETA_REFERENCE)
                / (1.0 - ETA_REFERENCE);
            for f in [3.0, 18.2, 40.0] {
                let low = crossflow_eta_low(f);
                assert!((crossflow_eta(f, m) - (low + (1.0 - low) * s)).abs() < 1e-14);
            }
        }
        // The Arcas Robin's two models (fineness 18.2 and 23.8), 0.74 and 0.77 to the reading.
        assert!((crossflow_eta(18.2, 0.0) - 0.7426).abs() < 1e-4);
        assert!((crossflow_eta(23.8, 0.0) - 0.7697).abs() < 1e-4);
    }

    #[test]
    fn figure_6s_own_bodies_get_figure_6_back() {
        let f = 10.0 + 2.0 * (ETA_REFERENCE - 0.685) / (0.701 - 0.685);
        for (&m, &eta) in ETA_MACHS.iter().zip(&ETA_BY_CROSSFLOW_MACH) {
            assert!((crossflow_eta(f, m) - eta).abs() < 1e-12, "M_n {m}");
        }
    }

    #[test]
    fn ends_are_held() {
        assert_eq!(crossflow_drag(5.0), 1.266);
        assert_eq!(crossflow_drag(-0.1), 1.2);
        assert_eq!(crossflow_drag(f64::NAN), 1.2);
        assert_eq!(crossflow_eta(60.0, 0.0), 0.815);
        assert_eq!(crossflow_eta(1.0, 0.0), 0.577);
        assert_eq!(crossflow_eta(10.0, 3.0), crossflow_eta(10.0, 1.6));
    }

    #[test]
    fn galejs_is_the_old_constant() {
        assert_eq!(BodyLift::GALEJS.factor(7.0, 0.9), 1.1);
        assert_eq!(BodyLift::Galejs { k: 1.5 }.factor(30.0, 0.0), 1.5);
        assert!(BodyLift::Galejs { k: -1.0 }.validate().is_err());
        assert!(BodyLift::Galejs { k: f64::NAN }.validate().is_err());
        assert!(BodyLift::JORGENSEN.validate().is_ok());
        assert_eq!(BodyLift::default(), BodyLift::JORGENSEN);
    }

    /// The JSON form is a file format: each model round-trips, and a field a model doesn't have
    /// is refused, not dropped.
    #[test]
    fn body_lift_in_json() {
        for (model, text) in [
            (BodyLift::JORGENSEN, r#"{"kind":"jorgensen"}"#),
            (BodyLift::GALEJS, r#"{"kind":"galejs","k":1.1}"#),
        ] {
            assert_eq!(serde_json::to_string(&model).unwrap(), text);
            assert_eq!(serde_json::from_str::<BodyLift>(text).unwrap(), model);
        }
        for bad in [
            r#"{"kind":"jorgensen","k":1.5}"#,
            r#"{"kind":"galejs"}"#,
            r#"{"kind":"galejs","k":1.1,"eta":0.7}"#,
            r#"{"kind":"hoerner"}"#,
        ] {
            assert!(serde_json::from_str::<BodyLift>(bad).is_err(), "{bad}");
        }
    }

    proptest! {
        /// `η` stays within Fig. 4's lowest value and 1 and grows with fineness, and the factor
        /// is continuous: a step of 1e-9 in the crossflow Mach number moves it by no more than
        /// its steepest slope (under 20 per unit) allows. Once the crossflow passes Mach 0.8 the
        /// body's length no longer matters: every fineness is within 0.3% of Fig. 6's own bodies.
        #[test]
        fn eta_is_bounded_and_the_factor_continuous(f in 0.5f64..80.0, m in 0.0f64..5.0) {
            let eta = crossflow_eta(f, m);
            prop_assert!((0.577..1.0).contains(&eta));
            prop_assert!(crossflow_eta(f + 1.0, m) >= eta - 1e-15);
            let d = (crossflow_factor(f, m + 1e-9) - crossflow_factor(f, m)).abs();
            prop_assert!(d < 2e-8, "jump {d} at M_n {m}");
            if m >= 0.8 {
                let reference = crossflow_factor(10.625, m);
                prop_assert!((crossflow_factor(f, m) / reference - 1.0).abs() < 3e-3);
            }
        }
    }
}
