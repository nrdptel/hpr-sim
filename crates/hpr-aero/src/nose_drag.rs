//! Pressure drag of noses, shoulders and steps at every Mach number: Niskanen's semi-empirical
//! method (2009 §3.4.3, eq. 3.86–3.87, and appendix B), with Stoney's measured curves for the
//! shapes that have no closed form.
//!
//! A nose, or a shoulder (a transition that widens toward the tail), drags on its increase in area
//! with a coefficient `(C_D•)_p(M)` in three parts:
//!
//! - **At rest**, `(C_D•)_p,0 = 0.8 sin² φ` (eq. 3.86), with `φ` the joint angle at the aft end
//!   ([`crate::drag::joint_pressure_drag_coefficient`]): the separation drag of a joint that isn't
//!   smooth.
//! - **From a lower bound `M_L`**, appendix B's transonic and supersonic value `C_T(M)`, which
//!   depends on the shape and the fineness ratio `f = l/(d_aft − d_fore)` (a nose's length over
//!   its base diameter; a shoulder's length over its rise in diameter, so a cone and a conical
//!   shoulder of the same surface angle drag alike):
//!
//!   | shape | `C_T(M)` | `M_L` |
//!   |---|---|---|
//!   | a step (no length), a body's bare front face | the blunt cylinder, `0.85 q_stag/q` (eq. B.2) | 0.8 |
//!   | cone | eq. B.4–B.6, a cubic between Mach 1 and 1.3 ([`cone_pressure_drag_coefficient`]) | 1 |
//!   | ogive | the cone of the same length and diameter times `0.72 (κ − ½)² + 0.82` (eq. B.8) | 1 |
//!   | elliptical, power series, parabolic series, Haack series | Stoney's fineness-3 curves, scaled to `f` by eq. B.9 | where the curves start |
//!
//! - **Between Mach 0 and `M_L`**, eq. 3.87: `a M^b + (C_D•)_p,0`, with `a` and `b` fitting the
//!   value and slope of `C_T` at `M_L` ([`subsonic_pressure_drag_coefficient`]).
//!
//! Niskanen p. 48 treats shoulders "similar to nose cones" at all speeds and calls the result
//! "somewhat dubious at supersonic velocities"; a step is a shoulder of zero length, fineness 0.
//! See [Drag through Mach 1][guide] in the guide and the decision record [ADR-028][adr-028].
//!
//! A 5:1 von Kármán nose at Mach 1.5, the guide's worked example: Stoney's 3:1 curve gives 0.0893,
//! scaled by eq. B.9 to 0.0407 on the base area, where a 5:1 cone drags 0.0653.
//!
//! ```
//! use hpr_aero::nose_drag::{PressureDragCurve, cone_pressure_drag_coefficient};
//! use hpr_design::NoseShape;
//!
//! let von_karman = PressureDragCurve::new(NoseShape::VON_KARMAN, 5.0, 0.0)?;
//! assert!((von_karman.coefficient(1.5)? - 0.0407).abs() < 5e-5);
//! assert!((cone_pressure_drag_coefficient(5.0, 1.5)? - 0.0653).abs() < 5e-5);
//! # Ok::<(), hpr_aero::AeroError>(())
//! ```
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/physics/aero.html#drag-through-mach-1
//! [adr-028]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-028-drag-through-mach-1-niskanens-appendix-b-stoneys-curves-and-the-arcas-robins-axial-force-2026-09-18

use hpr_design::NoseShape;
use serde::Serialize;

use crate::drag::{
    SUBSONIC_MACH_LIMIT, check_mach_any, joint_pressure_drag_coefficient, stagnation_ratio,
};
use crate::error::{AeroError, check_dimension};

/// Where eq. B.4 takes over from the cubic join for cones: Mach 1.3 (Niskanen 2009 p. 107,
/// "M ≳ 1.3").
pub const CONE_SUPERSONIC_MACH: f64 = 1.3;

/// The ratio of specific heats of air in eq. B.5, `γ = 1.4`.
const GAMMA: f64 = 1.4;

/// `ln 4`, for eq. B.9's `log₄(f + 1)`.
const LN_4: f64 = std::f64::consts::LN_2 * 2.0;

/// The slope of the blunt cylinder's coefficient `0.85 q_stag/q` in Mach (eq. B.1–B.2
/// differentiated).
fn stagnation_drag_slope(mach: f64) -> f64 {
    0.85 * if mach < 1.0 {
        0.5 * mach + 0.1 * mach * mach * mach
    } else {
        let i = 1.0 / mach;
        let i3 = i * i * i;
        1.52 * i3 - 0.664 * i3 * i * i - 0.21 * i3 * i3 * i
    }
}

/// A cubic Hermite segment from `(x0, y0)` with slope `m0` to `(x1, y1)` with slope `m1`: the
/// value and the slope at `x`.
fn hermite(x0: f64, y0: f64, m0: f64, x1: f64, y1: f64, m1: f64, x: f64) -> (f64, f64) {
    let h = x1 - x0;
    let t = (x - x0) / h;
    let (t2, t3) = (t * t, t * t * t);
    let value = (2.0 * t3 - 3.0 * t2 + 1.0) * y0
        + (t3 - 2.0 * t2 + t) * h * m0
        + (3.0 * t2 - 2.0 * t3) * y1
        + (t3 - t2) * h * m1;
    let slope = ((6.0 * t2 - 6.0 * t) * y0 + (6.0 * t - 6.0 * t2) * y1) / h
        + (3.0 * t2 - 4.0 * t + 1.0) * m0
        + (3.0 * t2 - 2.0 * t) * m1;
    (value, slope)
}

/// A cone's transonic and supersonic pressure drag on its base area, from Mach 1, and its slope in
/// Mach, with `s = sin ε` the sine of its half-angle (Niskanen 2009 appendix B.2, after Hoerner
/// pp. 16-18 to 16-20):
///
/// ```text
/// C(1)  = s                                         (eq. B.6)
/// C′(1) = 4/(γ + 1) (1 − C(1)/2)                    (eq. B.5)
/// C(M)  = 2.1 s² + 0.5 s/√(M² − 1),   M ≥ 1.3       (eq. B.4)
/// ```
///
/// and between Mach 1 and 1.3 the cubic that meets both ends' values and slopes ("polynomial
/// interpolation with the boundary conditions from equations (B.4), (B.5) and (B.6)"; four
/// conditions, so a cubic).
fn cone_transonic(s: f64, mach: f64) -> (f64, f64) {
    let supersonic = |m: f64| {
        let root = (m * m - 1.0).sqrt();
        (
            2.1 * s * s + 0.5 * s / root,
            -0.5 * s * m / (root * root * root),
        )
    };
    if mach >= CONE_SUPERSONIC_MACH {
        return supersonic(mach);
    }
    let slope_at_1 = 4.0 / (GAMMA + 1.0) * (1.0 - 0.5 * s);
    let (c13, slope13) = supersonic(CONE_SUPERSONIC_MACH);
    hermite(1.0, s, slope_at_1, CONE_SUPERSONIC_MACH, c13, slope13, mach)
}

/// Eq. 3.87's fit below `M_L`: how the pressure drag rises from its value at rest to the
/// transonic method's value at `M_L`.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Fit {
    /// `a Mᵇ` added to the value at rest (eq. 3.87), held as `Δ (M/M_L)ᵇ`, which is the same with
    /// `a = Δ/M_Lᵇ` and stays within `[0, Δ]` where `M_Lᵇ` would underflow.
    Power {
        /// `Δ = C_T(M_L) − (C_D•)_p,0`, the rise.
        delta: f64,
        /// `b`.
        b: f64,
    },
    /// `Δ (M/M_L)²` added to the value at rest, where eq. 3.87 has no solution (hpr's choice).
    Quadratic {
        /// `Δ = C_T(M_L) − (C_D•)_p,0`.
        delta: f64,
    },
}

impl Fit {
    /// Eq. 3.87's `a` and `b` from the rise `delta = C_T(M_L) − (C_D•)_p,0` and the slope
    /// `slope = C_T′(M_L)`: `b = C_T′(M_L) M_L/Δ`, `a = Δ/M_L^b`, so that `a M^b` meets both at
    /// `M_L`. The power meets Niskanen's conditions (non-decreasing, zero slope at rest) only for a
    /// rise with `b > 1`; otherwise the quadratic.
    fn new(delta: f64, slope: f64, mach_low: f64) -> Self {
        let b = slope * mach_low / delta;
        if delta > 0.0 && b > 1.0 {
            Self::Power { delta, b }
        } else {
            Self::Quadratic { delta }
        }
    }

    /// The fit's value and slope at `mach`, below `mach_low`, above the value at rest.
    fn eval(self, mach: f64, mach_low: f64) -> (f64, f64) {
        match self {
            Self::Power { delta, b } => {
                if mach == 0.0 {
                    // The slope at rest is never asked for (only at and above `M_L`).
                    (0.0, 0.0)
                } else {
                    let power = delta * (mach / mach_low).powf(b);
                    (power, b * power / mach)
                }
            }
            Self::Quadratic { delta } => {
                let t = mach / mach_low;
                (delta * t * t, 2.0 * delta * t / mach_low)
            }
        }
    }
}

/// Niskanen's eq. 3.87 between Mach 0 and the transonic method's lower bound `M_L`:
/// `(C_D•)_p = a M^b + (C_D•)_p,0`, with `a` and `b` "computed to fit the drag coefficient and
/// its derivative at the lower bound of the transonic method" (Niskanen 2009 p. 48):
/// `b = C_T′(M_L) M_L/Δ` and `a = Δ/M_L^b`, where `Δ = C_T(M_L) − (C_D•)_p,0`.
///
/// Niskanen assumes the curve "non-decreasing in the subsonic region" with zero slope at rest,
/// which needs `Δ > 0` and a positive slope (and `b > 1` for the zero slope). Where `Δ ≤ 0` or the
/// slope isn't positive, no `a M^b` meets both conditions, and hpr uses `(C_D•)_p,0 + Δ (M/M_L)²`
/// instead: continuous in value, flat at rest, with a kink at `M_L` ([ADR-028][adr-028]). It
/// arises only for small coefficients: a joint that isn't smooth on a shape whose measured curve
/// is still near 0 at `M_L`.
///
/// [adr-028]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-028-drag-through-mach-1-niskanens-appendix-b-stoneys-curves-and-the-arcas-robins-axial-force-2026-09-18
///
/// `c_rest` is `(C_D•)_p,0`, `c_low` and `slope_low` the value and slope at `mach_low`.
///
/// # Errors
///
/// [`AeroError::Domain`] for a Mach number outside `[0, mach_low]`, a non-positive `mach_low` or
/// non-finite coefficients.
pub fn subsonic_pressure_drag_coefficient(
    c_rest: f64,
    c_low: f64,
    slope_low: f64,
    mach_low: f64,
    mach: f64,
) -> Result<f64, AeroError> {
    check_dimension("transonic lower bound", mach_low, false)?;
    for (what, value) in [
        ("pressure drag at rest", c_rest),
        ("pressure drag at the lower bound", c_low),
        ("pressure drag slope at the lower bound", slope_low),
    ] {
        if !value.is_finite() {
            return Err(AeroError::Domain { what, value });
        }
    }
    if !(0.0..=mach_low).contains(&mach) {
        return Err(AeroError::Domain {
            what: "Mach number below the transonic lower bound",
            value: mach,
        });
    }
    Ok(c_rest
        + Fit::new(c_low - c_rest, slope_low, mach_low)
            .eval(mach, mach_low)
            .0)
}

/// The pressure drag of a cone nose of fineness ratio `f` (length over base diameter) at any
/// Mach number, on its base area (Niskanen 2009 eq. 3.86–3.87 and B.3–B.6):
///
/// - below Mach 1, eq. 3.87 from `0.8 sin² ε` at rest to the transonic method at Mach 1;
/// - `sin ε` at Mach 1 (eq. B.6), with slope `4/(γ + 1) (1 − sin ε/2)` (eq. B.5);
/// - between Mach 1 and 1.3, the cubic that meets both ends' values and slopes;
/// - from Mach 1.3, `2.1 sin² ε + 0.5 sin ε/√(M² − 1)` (eq. B.4),
///
/// with the half-angle `ε` from `tan ε = 1/(2f)` (eq. B.3). The joint angle of a cone meeting its
/// tube is `ε`, so its value at rest is eq. 3.86's `0.8 sin² ε`. A 3:1 cone: 0.0216 at rest,
/// 0.1644 at Mach 1, 0.1557 at Mach 1.3, 0.1042 at Mach 2.
///
/// Below fineness 1 the closed form passes a flat face's drag as the cone flattens, so there hpr
/// scales between a flat face at fineness 0 and this curve at fineness 1, as eq. B.9 does, from
/// Mach 0.8 ([`PressureDragCurve::new`]): a cone of fineness 0.5 gets 0.647 at Mach 1, not
/// `sin ε` = 0.707.
///
/// # Errors
///
/// [`AeroError::Domain`] for a non-positive or non-finite fineness ratio, or a negative or
/// non-finite Mach number.
pub fn cone_pressure_drag_coefficient(fineness_ratio: f64, mach: f64) -> Result<f64, AeroError> {
    check_dimension("cone fineness ratio", fineness_ratio, false)?;
    PressureDragCurve::new(
        NoseShape::Conical {},
        fineness_ratio,
        (0.5 / fineness_ratio).atan(),
    )?
    .coefficient(mach)
}

/// Eq. B.8's ratio of an ogive's pressure drag to that of the cone with the same length and base
/// diameter, at transonic and supersonic speeds: `0.72 (κ − ½)² + 0.82` (Niskanen 2009 p. 110),
/// with `κ = ρ_t/ρ` the tangent ogive's arc radius over the ogive's (0 for a cone, 1 for a tangent
/// ogive). It is 1 at both ends and 0.82 at `κ = ½`, after NAVWEPS Report 1488 p. 239: the best
/// ogive drags "consistently 18% less" than the cone at Mach 1.6 to 2.5 and fineness 2 to 3.5.
///
/// # Errors
///
/// [`AeroError::Domain`] for `κ` outside `[0, 1]`: a bulged secant ogive (`κ > 1`, arc radius
/// below the tangent ogive's) is outside the fit.
pub fn ogive_pressure_drag_factor(kappa: f64) -> Result<f64, AeroError> {
    if !(0.0..=1.0).contains(&kappa) {
        return Err(AeroError::Domain {
            what: "ogive κ (tangent-ogive radius over arc radius)",
            value: kappa,
        });
    }
    let d = kappa - 0.5;
    Ok(0.72 * d * d + 0.82)
}

/// Eq. B.9's fineness-ratio scaling, for shapes measured at fineness 3 (Niskanen 2009 p. 110):
///
/// ```text
/// (C_D•)_p = C₀ (C₃/C₀)^log₄(f + 1)
/// ```
///
/// the curve `a/(f + 1)^b` (eq. B.7) through the blunt cylinder `C₀` at fineness 0 (eq. B.2) and
/// the measured `C₃` at fineness 3. Stoney's report "suggests that the effects of fineness ratio
/// and Mach number may be separated" (p. 108).
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite `C₃` or fineness ratio, or a non-positive
/// `C₀`.
pub fn fineness_scaled_pressure_drag(
    c3: f64,
    c0: f64,
    fineness_ratio: f64,
) -> Result<f64, AeroError> {
    check_dimension("fineness-3 pressure drag", c3, true)?;
    check_dimension("blunt-cylinder pressure drag", c0, false)?;
    check_dimension("fineness ratio", fineness_ratio, true)?;
    Ok(c0 * (c3 / c0).powf((fineness_ratio + 1.0).ln() / LN_4))
}

/// A nose shape Stoney measured at fineness 3 (NASA TR R-100, 1961, Figure 12, printed p. 16),
/// whose pressure-drag curve hpr carries as digitized points.
///
/// Niskanen 2009 p. 108 names these nine; hpr's shapes between them are interpolated in their
/// parameter ([`PressureDragCurve::new`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum StoneyNose {
    /// Power series `x^¼`.
    PowerQuarter,
    /// Power series `x^½`.
    PowerHalf,
    /// Power series `x^¾`.
    PowerThreeQuarters,
    /// Half (`K′ = ½`) parabola.
    ParabolaHalf,
    /// Three-quarter (`K′ = ¾`) parabola.
    ParabolaThreeQuarters,
    /// Full (`K′ = 1`) parabola.
    Parabola,
    /// Ellipsoid (hpr's elliptical nose).
    Ellipsoid,
    /// L-V Haack (`C = ⅓`).
    LvHaack,
    /// Von Kármán (LD-Haack, `C = 0`).
    VonKarman,
}

impl StoneyNose {
    /// Every curve.
    pub const ALL: &'static [Self] = &[
        Self::PowerQuarter,
        Self::PowerHalf,
        Self::PowerThreeQuarters,
        Self::ParabolaHalf,
        Self::ParabolaThreeQuarters,
        Self::Parabola,
        Self::Ellipsoid,
        Self::LvHaack,
        Self::VonKarman,
    ];

    /// The digitized points `(M, C_D,N)`, in increasing Mach number: the nose's pressure drag on
    /// its base area at fineness 3.
    pub fn points(self) -> &'static [(f64, f64)] {
        match self {
            Self::PowerQuarter => stoney::POWER_QUARTER,
            Self::PowerHalf => stoney::POWER_HALF,
            Self::PowerThreeQuarters => stoney::POWER_THREE_QUARTERS,
            Self::ParabolaHalf => stoney::PARABOLA_HALF,
            Self::ParabolaThreeQuarters => stoney::PARABOLA_THREE_QUARTERS,
            Self::Parabola => stoney::PARABOLA,
            Self::Ellipsoid => stoney::ELLIPSOID,
            Self::LvHaack => stoney::LV_HAACK,
            Self::VonKarman => stoney::VON_KARMAN,
        }
    }

    /// Where the points were read: Figure 12's panel and the curve.
    pub fn source(self) -> &'static str {
        match self {
            Self::PowerQuarter => stoney::POWER_QUARTER_SOURCE,
            Self::PowerHalf => stoney::POWER_HALF_SOURCE,
            Self::PowerThreeQuarters => stoney::POWER_THREE_QUARTERS_SOURCE,
            Self::ParabolaHalf => stoney::PARABOLA_HALF_SOURCE,
            Self::ParabolaThreeQuarters => stoney::PARABOLA_THREE_QUARTERS_SOURCE,
            Self::Parabola => stoney::PARABOLA_SOURCE,
            Self::Ellipsoid => stoney::ELLIPSOID_SOURCE,
            Self::LvHaack => stoney::LV_HAACK_SOURCE,
            Self::VonKarman => stoney::VON_KARMAN_SOURCE,
        }
    }

    /// The first Mach number of the curve.
    pub fn first_mach(self) -> f64 {
        self.points()[0].0
    }

    /// The pressure drag at fineness 3 and `mach`, on the base area, and its slope in Mach:
    /// linear between the points, the right-hand slope at a point, and the last value held past
    /// the last point (slope 0). A curve that starts after Mach 0.8 (the x^¼ and the ellipsoid,
    /// from 1.2) is joined by a straight line to 0 at Mach 0.8, where every smooth 3:1 nose of
    /// panel (a) reads 0 (ADR-028); nothing is asked below Mach 0.8.
    fn value_and_slope(self, mach: f64) -> (f64, f64) {
        let points = self.points();
        let (first, last) = (points[0], points[points.len() - 1]);
        if mach < first.0 {
            if first.0 > SUBSONIC_MACH_LIMIT && mach > SUBSONIC_MACH_LIMIT {
                let slope = first.1 / (first.0 - SUBSONIC_MACH_LIMIT);
                return (slope * (mach - SUBSONIC_MACH_LIMIT), slope);
            }
            return (
                if first.0 > SUBSONIC_MACH_LIMIT {
                    0.0
                } else {
                    first.1
                },
                0.0,
            );
        }
        if mach >= last.0 {
            return (last.1, 0.0);
        }
        // The segment [i, i + 1] that holds `mach`, with `mach` at or past point `i`: the first
        // point is at or below `mach` (checked above, and every caller checks the Mach number is
        // finite), so the partition point is at least 1.
        let i = points.partition_point(|p| p.0 <= mach) - 1;
        let ((m0, c0), (m1, c1)) = (points[i], points[i + 1]);
        let slope = (c1 - c0) / (m1 - m0);
        (c0 + slope * (mach - m0), slope)
    }
}

/// A reference curve that eq. B.9's scaling and the interpolation between shapes run between.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Reference {
    /// The blunt cylinder (fineness 0, and a power series of exponent 0): eq. B.2.
    Blunt,
    /// A cone of `fineness_ratio` times `factor`, its whole curve: eq. 3.87 from `rest` below
    /// Mach 1, eq. B.4–B.6 above.
    Cone {
        /// The fineness ratio.
        fineness_ratio: f64,
        /// Eq. B.8's ogive factor, 1 for a cone.
        factor: f64,
        /// The value at rest.
        rest: f64,
    },
    /// Stoney's measured curve at fineness 3.
    Stoney(StoneyNose),
}

impl Reference {
    /// The 3:1 cone, an end of the power and parabolic series' interpolations, at rest
    /// `0.8 sin² ε` with `sin ε = 1/√37` (`tan ε = 1/6`).
    const CONE_3: Self = Self::Cone {
        fineness_ratio: 3.0,
        factor: 1.0,
        rest: 0.8 / 37.0,
    };

    fn value_and_slope(self, mach: f64) -> (f64, f64) {
        match self {
            Self::Blunt => (0.85 * stagnation_ratio(mach), stagnation_drag_slope(mach)),
            Self::Cone {
                fineness_ratio,
                factor,
                rest,
            } => PressureDragCurve::from_transonic(
                rest,
                Transonic::cone(fineness_ratio, factor),
                1.0,
            )
            .value_and_slope(mach),
            Self::Stoney(nose) => nose.value_and_slope(mach),
        }
    }
}

/// A transonic and supersonic method, from its lower bound `M_L` up.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Transonic {
    /// The blunt cylinder, `0.85 q_stag/q` (eq. B.2).
    Blunt,
    /// A cone of half-angle `ε`, times an ogive's factor (eq. B.4–B.6, B.8).
    Cone {
        /// `sin ε`.
        sin_half_angle: f64,
        /// Eq. B.8's factor, 1 for a cone.
        factor: f64,
    },
    /// Reference curves, interpolated in the shape's parameter and scaled from their fineness
    /// `f_ref` to the shape's `f` by eq. B.9.
    Scaled {
        /// The curve at the lower parameter.
        lower: Reference,
        /// The curve at the upper parameter.
        upper: Reference,
        /// The weight of `upper`, in `[0, 1]`.
        weight: f64,
        /// `ln(f + 1)/ln(f_ref + 1)`: `log₄(f + 1)` for Stoney's fineness 3.
        exponent: f64,
    },
}

impl Transonic {
    /// A cone of fineness ratio `f`, `tan ε = 1/(2f)` (eq. B.3), times `factor`.
    fn cone(fineness_ratio: f64, factor: f64) -> Self {
        let tan = 0.5 / fineness_ratio;
        Self::Cone {
            sin_half_angle: tan / (1.0 + tan * tan).sqrt(),
            factor,
        }
    }

    fn value_and_slope(self, mach: f64) -> (f64, f64) {
        match self {
            Self::Blunt => Reference::Blunt.value_and_slope(mach),
            Self::Cone {
                sin_half_angle,
                factor,
            } => {
                let (c, slope) = cone_transonic(sin_half_angle, mach);
                (factor * c, factor * slope)
            }
            Self::Scaled {
                lower,
                upper,
                weight,
                exponent,
            } => {
                let (lo, lo_slope) = lower.value_and_slope(mach);
                let (hi, hi_slope) = upper.value_and_slope(mach);
                let c3 = lo + weight * (hi - lo);
                let c3_slope = lo_slope + weight * (hi_slope - lo_slope);
                let (c0, c0_slope) = Reference::Blunt.value_and_slope(mach);
                if c3 <= 0.0 {
                    return (0.0, 0.0);
                }
                let c = c0 * (c3 / c0).powf(exponent);
                (
                    c,
                    c * ((1.0 - exponent) * c0_slope / c0 + exponent * c3_slope / c3),
                )
            }
        }
    }
}

/// A nose's, shoulder's or step's pressure-drag coefficient against Mach number, on its increase
/// in area: the value at rest, eq. 3.87's fit, and appendix B's transonic method from `M_L`
/// (the module docs). It serializes what it was built from, not its internals.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PressureDragCurve {
    /// The nose or shoulder shape; `None` for a step or a bare front face.
    shape: Option<NoseShape>,
    /// The fineness ratio `l/(d_aft − d_fore)`; 0 for a step.
    fineness_ratio: f64,
    /// `(C_D•)_p,0`, eq. 3.86's value at rest.
    rest: f64,
    /// `M_L`, where the transonic method starts.
    mach_low: f64,
    /// Eq. 3.87's fit below `M_L`.
    #[serde(skip)]
    fit: Fit,
    /// The transonic method.
    #[serde(skip)]
    transonic: Transonic,
}

impl PressureDragCurve {
    /// The curve from its value at rest, its transonic method and `M_L`. With `M_L` 0 the method
    /// covers every Mach number, and the value at rest is its own.
    fn from_transonic(rest: f64, transonic: Transonic, mach_low: f64) -> Self {
        let (c_low, slope_low) = transonic.value_and_slope(mach_low);
        let rest = if mach_low == 0.0 { c_low } else { rest };
        Self {
            shape: None,
            fineness_ratio: 0.0,
            rest,
            mach_low,
            fit: Fit::new(c_low - rest, slope_low, mach_low),
            transonic,
        }
    }

    /// A step up in radius, or a body's bare front face: a flat face, the blunt cylinder's
    /// `0.85 q_stag/q` at every Mach number (eq. B.1–B.2), 0.85 at rest. Eq. 3.86 "does not take
    /// into account the effect of extremely blunt nose cones (length less than half of the
    /// diameter)" (Niskanen 2009 p. 47), and a step has no length.
    pub fn step() -> Self {
        Self::from_transonic(0.0, Transonic::Blunt, 0.0)
    }

    /// The curve of a nose or shoulder of `shape`, fineness ratio `f = l/(d_aft − d_fore)` and
    /// joint angle `joint_angle_rad` at its aft end (eq. 3.86's `φ`).
    ///
    /// - A cone takes eq. B.4–B.6, and its value at rest from `φ` (for a cone nose, `φ = ε`).
    /// - An ogive takes the cone of the same fineness times eq. B.8's factor, `κ` the reciprocal of
    ///   its radius ratio.
    /// - The other shapes interpolate Stoney's fineness-3 curves linearly in their parameter, then
    ///   scale by eq. B.9 (Niskanen p. 108: "If data for a particular parameter value is missing,
    ///   interpolate"). A power series `xⁿ` runs through the blunt cylinder (`n = 0`), `x^¼`, `x^½`,
    ///   `x^¾` and the 3:1 cone (`n = 1`); a parabolic series through the cone (`K′ = 0`) and the
    ///   `½`, `¾` and full parabolas; a Haack series between von Kármán (`C = 0`) and L-V Haack
    ///   (`C = ⅓`). `M_L` is the first Mach number both ends' curves have.
    ///
    /// Cones and ogives below fineness 1 scale by eq. B.9's form between a flat face at fineness 0
    /// and their closed form at fineness 1, from Mach 0.8, because the closed form passes a flat
    /// face's drag as the cone flattens. A zero fineness ratio is a step:
    /// [`PressureDragCurve::step`] whatever the shape.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a negative or non-finite fineness ratio, a joint angle outside
    /// `[0, π/2]`, or a power-series exponent, parabolic parameter or Haack parameter outside
    /// `[0, 1]`, `[0, 1]` or `[0, ⅓]`; [`AeroError::Unsupported`] for an ogive whose radius ratio
    /// isn't a finite number of at least 1 (a bulged secant ogive has one below 1) or a Haack series
    /// above `C = ⅓`, where no data reaches, and for a shape this model doesn't know.
    pub fn new(
        shape: NoseShape,
        fineness_ratio: f64,
        joint_angle_rad: f64,
    ) -> Result<Self, AeroError> {
        let mut curve = Self::build(shape, fineness_ratio, joint_angle_rad)?;
        if fineness_ratio > 0.0 {
            curve.shape = Some(shape);
            curve.fineness_ratio = fineness_ratio;
        }
        Ok(curve)
    }

    /// [`PressureDragCurve::new`]'s curve, before it records its inputs.
    fn build(
        shape: NoseShape,
        fineness_ratio: f64,
        joint_angle_rad: f64,
    ) -> Result<Self, AeroError> {
        check_dimension("fineness ratio", fineness_ratio, true)?;
        let rest = joint_pressure_drag_coefficient(joint_angle_rad)?;
        if fineness_ratio == 0.0 {
            return Ok(Self::step());
        }
        let interpolated = |lower: Reference, upper: Reference, weight: f64| {
            let transonic = Transonic::Scaled {
                lower,
                upper,
                weight,
                exponent: (fineness_ratio + 1.0).ln() / LN_4,
            };
            Ok(Self::from_transonic(rest, transonic, SUBSONIC_MACH_LIMIT))
        };
        // Cones and ogives: the closed form from fineness 1; below it, at every Mach number,
        // eq. B.9's form between the flat face and the whole curve at fineness 1, since eq. B.4
        // passes the flat face as the cone flattens (ADR-028).
        let cone = |factor: f64| {
            if fineness_ratio >= 1.0 {
                Ok(Self::from_transonic(
                    rest,
                    Transonic::cone(fineness_ratio, factor),
                    1.0,
                ))
            } else {
                let transonic = Transonic::Scaled {
                    lower: Reference::Blunt,
                    upper: Reference::Cone {
                        fineness_ratio: 1.0,
                        factor,
                        rest,
                    },
                    weight: 1.0,
                    exponent: (fineness_ratio + 1.0).ln() / std::f64::consts::LN_2,
                };
                Ok(Self::from_transonic(rest, transonic, 0.0))
            }
        };
        use Reference::{Blunt, Stoney};
        let cone_3 = Reference::CONE_3;
        match shape {
            NoseShape::Conical {} => cone(1.0),
            NoseShape::Ogive { radius_ratio } => {
                if !(radius_ratio.is_finite() && radius_ratio >= 1.0) {
                    return Err(AeroError::Unsupported(format!(
                        "an ogive of radius ratio {radius_ratio}, not at least 1 (below 1 is a \
                         bulged secant ogive): Niskanen's \
                         eq. B.8 runs only from the cone to the tangent ogive"
                    )));
                }
                cone(ogive_pressure_drag_factor(1.0 / radius_ratio)?)
            }
            NoseShape::Elliptical {} => interpolated(
                Stoney(StoneyNose::Ellipsoid),
                Stoney(StoneyNose::Ellipsoid),
                0.0,
            ),
            NoseShape::PowerSeries { exponent: n } => {
                let (lower, upper, low, high) = if n < 0.25 {
                    (Blunt, Stoney(StoneyNose::PowerQuarter), 0.0, 0.25)
                } else if n < 0.5 {
                    (
                        Stoney(StoneyNose::PowerQuarter),
                        Stoney(StoneyNose::PowerHalf),
                        0.25,
                        0.5,
                    )
                } else if n < 0.75 {
                    (
                        Stoney(StoneyNose::PowerHalf),
                        Stoney(StoneyNose::PowerThreeQuarters),
                        0.5,
                        0.75,
                    )
                } else {
                    (Stoney(StoneyNose::PowerThreeQuarters), cone_3, 0.75, 1.0)
                };
                check_parameter("power series exponent", n, 0.0, 1.0)?;
                interpolated(lower, upper, (n - low) / (high - low))
            }
            NoseShape::ParabolicSeries { parameter: k } => {
                let (lower, upper, low, high) = if k < 0.5 {
                    (cone_3, Stoney(StoneyNose::ParabolaHalf), 0.0, 0.5)
                } else if k < 0.75 {
                    (
                        Stoney(StoneyNose::ParabolaHalf),
                        Stoney(StoneyNose::ParabolaThreeQuarters),
                        0.5,
                        0.75,
                    )
                } else {
                    (
                        Stoney(StoneyNose::ParabolaThreeQuarters),
                        Stoney(StoneyNose::Parabola),
                        0.75,
                        1.0,
                    )
                };
                check_parameter("parabolic series parameter", k, 0.0, 1.0)?;
                interpolated(lower, upper, (k - low) / (high - low))
            }
            NoseShape::Haack { parameter: c } => {
                if c > 1.0 / 3.0 {
                    return Err(AeroError::Unsupported(format!(
                        "a Haack series nose of C = {c}, above the L-V Haack's 1/3: Stoney's data \
                         stops there (Niskanen 2009 p. 103)"
                    )));
                }
                check_parameter("Haack series parameter", c, 0.0, 1.0 / 3.0)?;
                interpolated(
                    Stoney(StoneyNose::VonKarman),
                    Stoney(StoneyNose::LvHaack),
                    3.0 * c,
                )
            }
            // A new shape needs a pressure-drag decision.
            _ => Err(AeroError::Unsupported(
                "this nose shape (no transonic pressure-drag method)".to_owned(),
            )),
        }
    }

    /// Eq. 3.86's value at rest, `0.8 sin² φ`.
    pub fn rest_coefficient(&self) -> f64 {
        self.rest
    }

    /// `M_L`, where appendix B's transonic method takes over from eq. 3.87.
    pub fn transonic_lower_bound(&self) -> f64 {
        self.mach_low
    }

    /// The coefficient at `mach`, on the increase in area.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a negative or non-finite Mach number.
    pub fn coefficient(&self, mach: f64) -> Result<f64, AeroError> {
        check_mach_any(mach)?;
        Ok(self.value_and_slope(mach).0)
    }

    /// The value and slope at a checked Mach number.
    fn value_and_slope(&self, mach: f64) -> (f64, f64) {
        if mach < self.mach_low {
            let (value, slope) = self.fit.eval(mach, self.mach_low);
            (self.rest + value, slope)
        } else {
            self.transonic.value_and_slope(mach)
        }
    }
}

/// Checks a shape parameter against `[low, high]`.
fn check_parameter(what: &'static str, value: f64, low: f64, high: f64) -> Result<(), AeroError> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(AeroError::Domain { what, value })
    }
}

/// Stoney's curves, read from NASA TR R-100 (1961), Figure 12 ("Pressure drag of noses of
/// fineness ratio 3"), printed p. 16 (PDF p. 20): `(M, C_D,N)`, the nose's pressure drag on its
/// base area, at the faired line's centre (600-dpi render, each panel's grid calibrated locally;
/// 2026-09-18, M1.8b1). Panel (a), Stoney's flight models, for the seven shapes it has; panel (b),
/// the wind tunnel of his ref. 30, for the x^¼ and the ellipsoid, which begin at Mach 1.2 there.
/// A curve ends at its last visible point, and [`super::StoneyNose`] holds its end value past it.
/// Configuration numbers are Stoney's (Fig. 9's key, PDF p. 19).
#[rustfmt::skip]
mod stoney {
    /// Stoney 1961, Fig. 12(a), flight models: x^3/4, configuration 61, the faired line from Mach 0.80 to its end at 1.967; leaving zero at Mach 0.870, peak 0.1185 at 1.057; read to +-0.0015 (at most +-0.0048).
    pub(super) const POWER_THREE_QUARTERS: &[(f64, f64)] = &[(0.8, 0.0), (0.85, 0.0), (0.87, 0.0), (0.9, 0.0131), (0.95, 0.0375), (1.0, 0.0848), (1.05, 0.1185), (1.057, 0.1185), (1.1, 0.1095), (1.15, 0.1089), (1.2, 0.109), (1.25, 0.1032), (1.3, 0.1001), (1.4, 0.097), (1.5, 0.0932), (1.6, 0.09), (1.8, 0.0847), (1.967, 0.0789)];
    pub(super) const POWER_THREE_QUARTERS_SOURCE: &str = "Stoney 1961, Fig. 12(a), flight models: x^3/4, configuration 61, the faired line from Mach 0.80 to its end at 1.967; leaving zero at Mach 0.870, peak 0.1185 at 1.057; read to +-0.0015 (at most +-0.0048)";
    /// Stoney 1961, Fig. 12(a), flight models: x^1/2, configuration 63, the faired line from Mach 0.80 to its end at 1.941; leaving zero at Mach 0.934; read to +-0.0014 (at most +-0.0023).
    pub(super) const POWER_HALF: &[(f64, f64)] = &[(0.8, 0.0), (0.85, 0.0), (0.9, 0.0), (0.934, 0.0), (0.95, 0.0069), (1.0, 0.046), (1.05, 0.0597), (1.1, 0.0573), (1.15, 0.0675), (1.2, 0.08), (1.25, 0.0853), (1.3, 0.0851), (1.4, 0.0847), (1.5, 0.0856), (1.6, 0.086), (1.8, 0.0885), (1.941, 0.0898)];
    pub(super) const POWER_HALF_SOURCE: &str = "Stoney 1961, Fig. 12(a), flight models: x^1/2, configuration 63, the faired line from Mach 0.80 to its end at 1.941; leaving zero at Mach 0.934; read to +-0.0014 (at most +-0.0023)";
    /// Stoney 1961, Fig. 12(a), flight models: parabola, configuration 59, the faired line from Mach 0.80 to its end at 1.975; leaving zero at Mach 0.952, peak 0.1162 at 1.165; read to +-0.0016 (at most +-0.0046).
    pub(super) const PARABOLA: &[(f64, f64)] = &[(0.8, 0.0), (0.85, 0.0), (0.9, 0.0), (0.95, 0.0), (0.952, 0.0), (1.0, 0.037), (1.05, 0.0898), (1.1, 0.1073), (1.15, 0.1149), (1.165, 0.1162), (1.2, 0.1161), (1.25, 0.1145), (1.3, 0.1134), (1.4, 0.1103), (1.5, 0.1078), (1.6, 0.1067), (1.8, 0.1062), (1.975, 0.1073)];
    pub(super) const PARABOLA_SOURCE: &str = "Stoney 1961, Fig. 12(a), flight models: parabola, configuration 59, the faired line from Mach 0.80 to its end at 1.975; leaving zero at Mach 0.952, peak 0.1162 at 1.165; read to +-0.0016 (at most +-0.0046)";
    /// Stoney 1961, Fig. 12(a), flight models: 3/4 parabola, configuration 62, the faired line from Mach 0.80 to its end at 1.968; leaving zero at Mach 0.902, peak 0.1078 at 1.136; read to +-0.0016 (at most +-0.0035).
    pub(super) const PARABOLA_THREE_QUARTERS: &[(f64, f64)] = &[(0.8, 0.0), (0.85, 0.0), (0.9, 0.0), (0.902, 0.0), (0.95, 0.0182), (1.0, 0.069), (1.05, 0.0889), (1.1, 0.1049), (1.136, 0.1078), (1.15, 0.1072), (1.2, 0.1036), (1.25, 0.0997), (1.3, 0.0932), (1.4, 0.0861), (1.5, 0.082), (1.6, 0.0817), (1.8, 0.0797), (1.968, 0.0814)];
    pub(super) const PARABOLA_THREE_QUARTERS_SOURCE: &str = "Stoney 1961, Fig. 12(a), flight models: 3/4 parabola, configuration 62, the faired line from Mach 0.80 to its end at 1.968; leaving zero at Mach 0.902, peak 0.1078 at 1.136; read to +-0.0016 (at most +-0.0035)";
    /// Stoney 1961, Fig. 12(a), flight models: 1/2 parabola, configuration 57, the faired line from Mach 0.80 to its end at 1.968; leaving zero at Mach 0.817, peak 0.1247 at 1.081; read to +-0.0016 (at most +-0.003).
    pub(super) const PARABOLA_HALF: &[(f64, f64)] = &[(0.8, 0.0), (0.817, 0.0), (0.85, 0.0057), (0.9, 0.0155), (0.95, 0.0403), (1.0, 0.094), (1.05, 0.1231), (1.081, 0.1247), (1.1, 0.1235), (1.15, 0.1149), (1.2, 0.1139), (1.25, 0.1068), (1.3, 0.1009), (1.4, 0.0936), (1.5, 0.0877), (1.6, 0.0854), (1.8, 0.0854), (1.968, 0.0864)];
    pub(super) const PARABOLA_HALF_SOURCE: &str = "Stoney 1961, Fig. 12(a), flight models: 1/2 parabola, configuration 57, the faired line from Mach 0.80 to its end at 1.968; leaving zero at Mach 0.817, peak 0.1247 at 1.081; read to +-0.0016 (at most +-0.003)";
    /// Stoney 1961, Fig. 12(a), flight models: Von Karman, configuration 58, the faired line from Mach 0.80 to its end at 1.994; leaving zero at Mach 0.915, peak 0.0898 at 1.553; read to +-0.0014 (at most +-0.0022).
    pub(super) const VON_KARMAN: &[(f64, f64)] = &[(0.8, 0.0), (0.85, 0.0), (0.9, 0.0), (0.915, 0.0), (0.95, 0.0077), (1.0, 0.0253), (1.05, 0.0581), (1.1, 0.0694), (1.15, 0.0734), (1.2, 0.0758), (1.25, 0.0779), (1.3, 0.0825), (1.4, 0.0879), (1.5, 0.0893), (1.553, 0.0898), (1.6, 0.0898), (1.8, 0.0869), (1.994, 0.0794)];
    pub(super) const VON_KARMAN_SOURCE: &str = "Stoney 1961, Fig. 12(a), flight models: Von Karman, configuration 58, the faired line from Mach 0.80 to its end at 1.994; leaving zero at Mach 0.915, peak 0.0898 at 1.553; read to +-0.0014 (at most +-0.0022)";
    /// Stoney 1961, Fig. 12(a), flight models: L-V Haack, configuration 60, the faired line from Mach 0.80 to its end at 1.977; leaving zero at Mach 0.915, peak 0.1172 at 1.617; read to +-0.0014 (at most +-0.0022).
    pub(super) const LV_HAACK: &[(f64, f64)] = &[(0.8, 0.0), (0.85, 0.0), (0.9, 0.0), (0.915, 0.0), (0.95, 0.0077), (1.0, 0.0253), (1.05, 0.065), (1.1, 0.0847), (1.15, 0.095), (1.2, 0.1002), (1.25, 0.1028), (1.3, 0.107), (1.4, 0.1135), (1.5, 0.1156), (1.6, 0.117), (1.617, 0.1172), (1.8, 0.1155), (1.977, 0.1115)];
    pub(super) const LV_HAACK_SOURCE: &str = "Stoney 1961, Fig. 12(a), flight models: L-V Haack, configuration 60, the faired line from Mach 0.80 to its end at 1.977; leaving zero at Mach 0.915, peak 0.1172 at 1.617; read to +-0.0014 (at most +-0.0022)";
    /// Stoney 1961, Fig. 12(b), wind tunnel (Stoney's ref. 30): x^1/4, the faired line from Mach 1.20 to its end at 3.587; read to +-0.0014 (at most +-0.003).
    pub(super) const POWER_QUARTER: &[(f64, f64)] = &[(1.2, 0.141), (1.25, 0.148), (1.3, 0.1558), (1.4, 0.1689), (1.5, 0.1809), (1.6, 0.1894), (1.8, 0.2051), (2.0, 0.2165), (2.4, 0.2331), (2.8, 0.2441), (3.2, 0.248), (3.587, 0.2491)];
    pub(super) const POWER_QUARTER_SOURCE: &str = "Stoney 1961, Fig. 12(b), wind tunnel (Stoney's ref. 30): x^1/4, the faired line from Mach 1.20 to its end at 3.587; read to +-0.0014 (at most +-0.003)";
    /// Stoney 1961, Fig. 12(b), wind tunnel (Stoney's ref. 30): ellipsoid, the faired line from Mach 1.20 to its end at 3.587; read to +-0.0014 (at most +-0.004).
    pub(super) const ELLIPSOID: &[(f64, f64)] = &[(1.2, 0.111), (1.25, 0.1298), (1.3, 0.14), (1.4, 0.1478), (1.5, 0.1509), (1.6, 0.1523), (1.8, 0.1552), (2.0, 0.1576), (2.4, 0.1601), (2.8, 0.1601), (3.2, 0.16), (3.587, 0.158)];
    pub(super) const ELLIPSOID_SOURCE: &str = "Stoney 1961, Fig. 12(b), wind tunnel (Stoney's ref. 30): ellipsoid, the faired line from Mach 1.20 to its end at 3.587; read to +-0.0014 (at most +-0.004)";
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = ((got - want) / want).abs();
        assert!(
            err <= rel,
            "{what}: got {got}, want {want}, rel err {err:e}"
        );
    }

    /// A 3:1 cone by hand from eq. 3.86–3.87 and B.3–B.6, and the cubic joining Mach 1 to 1.3
    /// smoothly (values and slopes agree on both sides of each join).
    #[test]
    fn a_cone_follows_appendix_b() {
        let s = 1.0 / 37f64.sqrt();
        let rest = 0.8 * s * s;
        let slope_at_1 = 4.0 / 2.4 * (1.0 - 0.5 * s);
        let b4 = |m: f64| 2.1 * s * s + 0.5 * s / (m * m - 1.0).sqrt();
        let cone = |m: f64| cone_pressure_drag_coefficient(3.0, m).unwrap();
        close(cone(0.0), rest, 1e-15, "at rest, 0.8 sin² ε");
        close(rest, 0.021_621_6, 1e-5, "0.0216");
        close(cone(1.0), s, 1e-15, "eq. B.6 at Mach 1");
        close(cone(1.3), b4(1.3), 1e-15, "eq. B.4 at 1.3");
        for m in [1.5, 2.0, 3.0, 4.99] {
            close(cone(m), b4(m), 1e-15, "eq. B.4");
        }
        close(cone(2.0), 0.104_215, 1e-5, "0.1042 at Mach 2");
        // Eq. 3.87 below Mach 1: b = C′(1)/(C(1) − C₀), a = C(1) − C₀.
        let b = slope_at_1 / (s - rest);
        close(b, 10.714, 1e-4, "b");
        for m in [0.3, 0.6, 0.9, 0.99] {
            close(
                cone(m),
                rest + (s - rest) * f64::powf(m, b),
                1e-13,
                "eq. 3.87",
            );
        }
        // Smooth at both joins.
        let curve =
            PressureDragCurve::new(NoseShape::Conical {}, 3.0, (1.0 / 6f64).atan()).unwrap();
        let h = 1e-7;
        for (m, want) in [
            (1.0, slope_at_1),
            (1.3, -0.5 * s * 1.3 / (0.69f64).powf(1.5)),
        ] {
            let c = |m: f64| curve.coefficient(m).unwrap();
            close((c(m) - c(m - h)) / h, want, 1e-5, "slope below a join");
            close((c(m + h) - c(m)) / h, want, 1e-5, "slope above a join");
            let below = curve.coefficient(m - 1e-12).unwrap();
            let above = curve.coefficient(m).unwrap();
            assert!((below - above).abs() < 1e-11, "value at Mach {m}");
        }
        assert_eq!(curve.transonic_lower_bound(), 1.0);
        close(curve.rest_coefficient(), rest, 1e-15, "rest");
    }

    /// Eq. B.8: 1 for a cone and a tangent ogive, 0.82 at `κ = ½`; outside `[0, 1]` refused, and a
    /// bulged ogive refused where the model is built.
    #[test]
    fn an_ogive_is_the_cone_times_eq_b8() {
        assert_eq!(ogive_pressure_drag_factor(0.0).unwrap(), 1.0);
        assert_eq!(ogive_pressure_drag_factor(1.0).unwrap(), 1.0);
        close(
            ogive_pressure_drag_factor(0.5).unwrap(),
            0.82,
            1e-15,
            "κ = ½",
        );
        for bad in [-0.1, 1.01, f64::NAN] {
            assert!(ogive_pressure_drag_factor(bad).is_err(), "{bad}");
        }
        let cone = PressureDragCurve::new(NoseShape::Conical {}, 4.0, 0.0).unwrap();
        let tangent = PressureDragCurve::new(NoseShape::TANGENT_OGIVE, 4.0, 0.0).unwrap();
        let secant =
            PressureDragCurve::new(NoseShape::Ogive { radius_ratio: 2.0 }, 4.0, 0.0).unwrap();
        for m in [1.0, 1.2, 2.0, 4.0] {
            close(
                tangent.coefficient(m).unwrap(),
                cone.coefficient(m).unwrap(),
                1e-15,
                "tangent",
            );
            close(
                secant.coefficient(m).unwrap(),
                0.82 * cone.coefficient(m).unwrap(),
                1e-14,
                "κ = ½",
            );
        }
        // With a smooth joint, nothing at rest.
        assert_eq!(tangent.coefficient(0.0).unwrap(), 0.0);
        let bulged = PressureDragCurve::new(NoseShape::Ogive { radius_ratio: 0.8 }, 4.0, 0.1);
        assert!(
            matches!(bulged, Err(AeroError::Unsupported(_))),
            "{bulged:?}"
        );
    }

    /// Eq. B.9 passes through the blunt cylinder at fineness 0 and the measured value at 3.
    #[test]
    fn eq_b9_runs_from_the_blunt_cylinder_to_fineness_3() {
        let (c3, c0) = (0.08, 1.4);
        close(
            fineness_scaled_pressure_drag(c3, c0, 3.0).unwrap(),
            c3,
            1e-15,
            "f = 3",
        );
        close(
            fineness_scaled_pressure_drag(c3, c0, 0.0).unwrap(),
            c0,
            1e-15,
            "f = 0",
        );
        let mut previous = c0;
        for f in [0.5, 1.0, 2.0, 3.0, 5.0, 10.0] {
            let c = fineness_scaled_pressure_drag(c3, c0, f).unwrap();
            assert!(c < previous, "falls with fineness: {c} at {f}");
            previous = c;
        }
        // a/(f + 1)^b through both (eq. B.7).
        let b = (c0 / c3).ln() / 4f64.ln();
        close(
            fineness_scaled_pressure_drag(c3, c0, 5.0).unwrap(),
            c0 / 6f64.powf(b),
            1e-14,
            "eq. B.7",
        );
        assert_eq!(fineness_scaled_pressure_drag(0.0, c0, 2.0).unwrap(), 0.0);
        for (c3, c0, f) in [(-0.01, 1.0, 3.0), (0.1, 0.0, 3.0), (0.1, 1.0, -1.0)] {
            assert!(fineness_scaled_pressure_drag(c3, c0, f).is_err());
        }
    }

    /// Eq. 3.87 meets the transonic value and slope at `M_L`; where it can't (no rise, or a
    /// falling curve), the quadratic meets the value.
    #[test]
    fn eq_3_87_meets_the_lower_bound() {
        let (rest, low, slope, m_l) = (0.02, 0.16, 1.5, 1.0);
        let fit = |m: f64| subsonic_pressure_drag_coefficient(rest, low, slope, m_l, m).unwrap();
        assert_eq!(fit(0.0), rest);
        close(fit(m_l), low, 1e-15, "value at M_L");
        close(
            (fit(m_l) - fit(m_l - 1e-7)) / 1e-7,
            slope,
            1e-5,
            "slope at M_L",
        );
        // No rise: the quadratic, from the value at rest to the value at M_L.
        let no_rise =
            |m: f64| subsonic_pressure_drag_coefficient(0.01, 0.005, 0.3, 0.8, m).unwrap();
        close(no_rise(0.4), 0.01 - 0.005 * 0.25, 1e-15, "quadratic");
        close(no_rise(0.8), 0.005, 1e-15, "quadratic at M_L");
        // Falling: the same.
        let falling = |m: f64| subsonic_pressure_drag_coefficient(0.0, 0.1, -0.2, 0.8, m).unwrap();
        close(falling(0.4), 0.025, 1e-15, "falling");
        for bad in [-0.1, 0.81, f64::NAN] {
            assert!(subsonic_pressure_drag_coefficient(rest, low, slope, 0.8, bad).is_err());
        }
        assert!(subsonic_pressure_drag_coefficient(rest, f64::NAN, slope, 0.8, 0.5).is_err());
        assert!(subsonic_pressure_drag_coefficient(rest, low, slope, 0.0, 0.0).is_err());
    }

    /// A step: the flat face, the blunt cylinder's `0.85 q_stag/q` at every Mach number (eq.
    /// B.1–B.2), 0.85 at rest, with its published jump at Mach 1.
    #[test]
    fn a_step_rises_to_the_blunt_cylinder() {
        let step = PressureDragCurve::step();
        let blunt = |m: f64| stagnation_drag_coefficient_for_test(m);
        assert_eq!(step.coefficient(0.0).unwrap(), 0.85);
        assert_eq!(step.rest_coefficient(), 0.85);
        close(
            step.coefficient(0.8).unwrap(),
            0.994_704,
            1e-12,
            "0.85 × 1.17024",
        );
        close(
            step.coefficient(0.3).unwrap(),
            0.85 * (1.0 + 0.0225 + 0.0081 / 40.0),
            1e-15,
            "0.3",
        );
        for m in [0.1, 0.5, 0.9, 0.999, 1.0, 2.0, 4.9] {
            close(
                step.coefficient(m).unwrap(),
                blunt(m),
                1e-15,
                "blunt cylinder",
            );
        }
        let mut previous = 0.85;
        for m in [0.1, 0.3, 0.5, 0.7, 0.8] {
            let c = step.coefficient(m).unwrap();
            assert!(c > previous, "rises: {c} at Mach {m}");
            previous = c;
        }
        // A zero-length shoulder of any shape is the step.
        for shape in [
            NoseShape::Conical {},
            NoseShape::VON_KARMAN,
            NoseShape::Elliptical {},
        ] {
            assert_eq!(
                PressureDragCurve::new(shape, 0.0, std::f64::consts::FRAC_PI_2).unwrap(),
                step
            );
        }
    }

    fn stagnation_drag_coefficient_for_test(mach: f64) -> f64 {
        crate::drag::stagnation_drag_coefficient(mach).unwrap()
    }

    /// Loft lesson L15 through Mach 1: a cone or ogive shoulder tends to the step as it shortens,
    /// and its blend below fineness 1 meets the closed form at 1.
    #[test]
    fn short_cones_tend_to_the_step_and_meet_the_closed_form_at_fineness_1() {
        let step = PressureDragCurve::step();
        for shape in [NoseShape::Conical {}, NoseShape::TANGENT_OGIVE] {
            let at = |f: f64| {
                let joint = (0.5 / f).atan();
                PressureDragCurve::new(shape, f, joint).unwrap()
            };
            for m in [0.0, 0.3, 0.79, 0.8, 0.95, 1.0, 1.2, 2.0, 4.9] {
                let near_zero = at(1e-9).coefficient(m).unwrap();
                close(near_zero, step.coefficient(m).unwrap(), 1e-7, "f → 0");
                let below = at(1.0 - 1e-10).coefficient(m).unwrap();
                let at_1 = at(1.0).coefficient(m).unwrap();
                close(below, at_1, 1e-8, "f → 1 from below");
            }
        }
    }

    /// Refusals: a Haack series past L-V Haack, shape parameters and fineness out of range, and
    /// a Mach number that isn't one.
    #[test]
    fn out_of_range_shapes_are_refused() {
        let haack = PressureDragCurve::new(NoseShape::Haack { parameter: 0.5 }, 3.0, 0.0);
        assert!(matches!(haack, Err(AeroError::Unsupported(_))), "{haack:?}");
        for (shape, f, joint) in [
            (NoseShape::Conical {}, -1.0, 0.1),
            (NoseShape::Conical {}, f64::NAN, 0.1),
            (NoseShape::Conical {}, 3.0, 2.0),
            (NoseShape::PowerSeries { exponent: 1.5 }, 3.0, 0.1),
            (NoseShape::ParabolicSeries { parameter: -0.1 }, 3.0, 0.1),
            (NoseShape::Haack { parameter: -0.1 }, 3.0, 0.0),
        ] {
            assert!(
                PressureDragCurve::new(shape, f, joint).is_err(),
                "{shape:?} {f} {joint}"
            );
        }
        let cone = PressureDragCurve::new(NoseShape::Conical {}, 3.0, 0.1).unwrap();
        for bad in [-0.1, f64::NAN, f64::INFINITY] {
            assert!(cone.coefficient(bad).is_err());
        }
        assert!(cone_pressure_drag_coefficient(0.0, 1.0).is_err());
    }

    /// Stoney's curves as carried: increasing Mach, non-negative, panel (a)'s from Mach 0.8 and
    /// panel (b)'s from 1.2; each shape at fineness 3 is its curve from `M_L` (eq. B.9 is the
    /// identity there), its end value held past its last point; shapes between measured ones
    /// interpolate, and the series' ends are the 3:1 cone and the blunt cylinder.
    #[test]
    fn stoney_curves_are_read_as_published() {
        for nose in StoneyNose::ALL {
            let points = nose.points();
            assert!(points.windows(2).all(|w| w[0].0 < w[1].0), "{nose:?}");
            assert!(points.iter().all(|p| p.1 >= 0.0 && p.1 < 0.3), "{nose:?}");
            let first = match nose {
                StoneyNose::PowerQuarter | StoneyNose::Ellipsoid => 1.2,
                _ => 0.8,
            };
            assert_eq!(nose.first_mach(), first, "{nose:?}");
            assert!(
                nose.source().starts_with("Stoney 1961, Fig. 12"),
                "{nose:?}"
            );
        }
        let at_3 = |shape: NoseShape| PressureDragCurve::new(shape, 3.0, 0.0).unwrap();
        let vk = at_3(NoseShape::VON_KARMAN);
        for &(m, c) in StoneyNose::VonKarman.points() {
            close(
                vk.coefficient(m).unwrap() + 1e-300,
                c + 1e-300,
                1e-12,
                "von Kármán",
            );
        }
        let (last_m, last_c) = *StoneyNose::VonKarman.points().last().unwrap();
        assert!(last_m < 2.0);
        close(
            vk.coefficient(4.0).unwrap(),
            last_c,
            1e-12,
            "held past the end",
        );
        close(
            vk.coefficient(1.5).unwrap(),
            0.0893,
            1e-12,
            "0.0893 at Mach 1.5",
        );
        let lv = at_3(NoseShape::LV_HAACK);
        close(
            lv.coefficient(1.5).unwrap(),
            0.1156,
            1e-12,
            "L-V Haack at 1.5",
        );
        let between = at_3(NoseShape::Haack {
            parameter: 1.0 / 6.0,
        });
        close(
            between.coefficient(1.5).unwrap(),
            0.5 * (0.0893 + 0.1156),
            1e-12,
            "C = 1/6",
        );
        // The series' ends: a power series of exponent 1 and a parabolic series of 0 are the 3:1
        // cone from Mach 0.8; exponent 0 would be the blunt cylinder.
        let cone = PressureDragCurve::new(NoseShape::Conical {}, 3.0, (1.0 / 6f64).atan()).unwrap();
        for shape in [
            NoseShape::PowerSeries { exponent: 1.0 },
            NoseShape::ParabolicSeries { parameter: 0.0 },
        ] {
            for m in [0.8, 0.9, 1.0, 1.2, 2.0, 4.0] {
                close(
                    at_3(shape).coefficient(m).unwrap(),
                    cone.coefficient(m).unwrap(),
                    1e-12,
                    "the 3:1 cone",
                );
            }
        }
        let blunt_ish = PressureDragCurve::new(NoseShape::PowerSeries { exponent: 0.05 }, 3.0, 0.0)
            .unwrap()
            .coefficient(2.0)
            .unwrap();
        let x_quarter = at_3(NoseShape::PowerSeries { exponent: 0.25 })
            .coefficient(2.0)
            .unwrap();
        close(x_quarter, 0.2165, 1e-12, "x^¼ at Mach 2");
        close(
            blunt_ish,
            0.8 * stagnation_drag_coefficient_for_test(2.0) + 0.2 * 0.2165,
            1e-12,
            "a fifth of the way from the blunt cylinder",
        );
        // The x^¼ and the ellipsoid start at Mach 1.2: a straight line joins them to 0 at Mach
        // 0.8, where the other smooth 3:1 noses read 0, so every Stoney shape starts at 0.8.
        let ellipse = at_3(NoseShape::Elliptical {});
        assert_eq!(ellipse.transonic_lower_bound(), 0.8);
        assert_eq!(vk.transonic_lower_bound(), 0.8);
        assert_eq!(ellipse.coefficient(0.6).unwrap(), 0.0);
        assert_eq!(ellipse.coefficient(0.8).unwrap(), 0.0);
        close(
            ellipse.coefficient(1.0).unwrap(),
            0.5 * 0.111,
            1e-12,
            "halfway up the line",
        );
        close(
            ellipse.coefficient(1.2).unwrap(),
            0.111,
            1e-12,
            "its first point",
        );
    }

    /// The guide's worked example (`docs/physics/aero.md`, *Drag through Mach 1*): at Mach 1.5 a
    /// 5:1 von Kármán nose drags 0.0407 on its base area, eq. B.9 from Stoney's 3:1 value 0.0893
    /// and the blunt cylinder's 1.3074 with the exponent `log₄ 6`; a 5:1 cone 0.0653 (eq. B.4).
    #[test]
    fn the_guides_worked_example() {
        let blunt = PressureDragCurve::step().coefficient(1.5).unwrap();
        close(blunt, 1.3074, 1e-4, "the blunt cylinder at Mach 1.5");
        let exponent = 6f64.ln() / 4f64.ln();
        close(exponent, 1.2925, 1e-4, "log₄ 6");
        let by_hand = blunt * (0.0893 / blunt).powf(exponent);
        let vk = PressureDragCurve::new(NoseShape::VON_KARMAN, 5.0, 0.0).unwrap();
        close(
            vk.coefficient(1.5).unwrap(),
            by_hand,
            1e-12,
            "5:1 von Kármán",
        );
        close(by_hand, 0.0407, 1e-3, "0.0407");
        let cone = cone_pressure_drag_coefficient(5.0, 1.5).unwrap();
        close(cone, 0.0653, 1e-3, "5:1 cone");
    }

    /// Niskanen's closed-form 3:1 cone (eq. 3.87 below Mach 1, eq. B.4–B.6 and the cubic above)
    /// against Stoney's measured 3:1 cone, configuration 56 of Figure 12(a) (read with the other
    /// curves, to ±0.0014): high through the whole rise, +87% at Mach 0.8 and +105% at 0.85, +49%
    /// at Mach 1 and +48% at 1.1, near the cubic's peak; +15% at 1.5 and +4% at the curve's end,
    /// 1.94. Documented in `docs/physics/aero.md`; the ogives inherit it.
    #[test]
    fn niskanens_cone_against_stoneys_measured_cone() {
        let stoney = [
            (0.8, 0.0186, 0.865),
            (0.85, 0.0228, 1.046),
            (0.9, 0.0366, 0.852),
            (0.95, 0.0673, 0.546),
            (1.0, 0.1102, 0.492),
            (1.1, 0.1580, 0.483),
            (1.2, 0.1378, 0.453),
            (1.5, 0.1136, 0.147),
            (1.8, 0.1044, 0.070),
            (1.937, 0.1022, 0.040),
        ];
        for (m, measured, error) in stoney {
            let hpr = cone_pressure_drag_coefficient(3.0, m).unwrap();
            let got = hpr / measured - 1.0;
            assert!(
                (got - error).abs() < 0.001,
                "Mach {m}: {got:+.4}, recorded {error:+.3}"
            );
        }
    }

    /// Code review: eq. 3.87's `a = Δ/M_Lᵇ` overflowed where `Δ` is small and `b` huge, and gave
    /// NaN below `M_L`. An x^0.868229375 nose at 3:1 with its own joint angle has `Δ` near 0; the
    /// curve now stays finite and between the value at rest and `C_T(M_L)` below `M_L`.
    #[test]
    fn eq_3_87_stays_finite_when_the_rise_is_tiny() {
        for n in [0.868229375, 0.8675, 0.869, 0.8695] {
            let joint = (n / 6.0f64).atan();
            let curve =
                PressureDragCurve::new(NoseShape::PowerSeries { exponent: n }, 3.0, joint).unwrap();
            let rest = curve.rest_coefficient();
            let at_l = curve.coefficient(curve.transonic_lower_bound()).unwrap();
            for m in [0.0, 0.1, 0.3, 0.5, 0.79, 0.8, 1.0, 1.1, 1.19, 1.5] {
                let c = curve.coefficient(m).unwrap();
                assert!(c.is_finite(), "n = {n}, Mach {m}: {c}");
                if m < curve.transonic_lower_bound() {
                    assert!(c >= rest.min(at_l) - 1e-15 && c <= rest.max(at_l) + 1e-15);
                }
            }
        }
        let tiny = subsonic_pressure_drag_coefficient(0.01, 0.01 + 1e-12, 1.0, 0.8, 0.5).unwrap();
        close(tiny, 0.01, 1e-9, "a rise of 1e-12");
    }

    /// Below fineness 1 a cone scales between a flat face and its fineness-1 closed form: at
    /// fineness 0.5 and Mach 1, 0.647 where `sin ε` would give 0.707.
    #[test]
    fn a_stubby_cone_takes_the_blend() {
        let c = cone_pressure_drag_coefficient(0.5, 1.0).unwrap();
        close(c, 0.647, 1e-3, "fineness 0.5 at Mach 1");
        assert!(
            c < std::f64::consts::FRAC_1_SQRT_2 - 0.05,
            "below sin ε = sin 45°"
        );
    }

    /// Physics review: a near-flat power series (x^0.05) has almost nothing at rest by eq. 3.86,
    /// which leaves bluntness out (Niskanen p. 47), and nearly the flat face's drag at Mach 0.8.
    /// No `a Mᵇ` with `b > 1` joins them, so the curve rises as `Δ (M/M_L)²`, flat at rest.
    #[test]
    fn a_near_flat_nose_rises_from_rest_without_a_jump() {
        let n = 0.05;
        let curve = PressureDragCurve::new(
            NoseShape::PowerSeries { exponent: n },
            3.0,
            (n / 6.0).atan(),
        )
        .unwrap();
        let rest = curve.rest_coefficient();
        let at_08 = curve.coefficient(0.8).unwrap();
        for m in [0.01, 0.1, 0.3, 0.6] {
            let want = rest + (at_08 - rest) * (m / 0.8) * (m / 0.8);
            close(curve.coefficient(m).unwrap(), want, 1e-12, "quadratic");
        }
        assert!(curve.coefficient(0.01).unwrap() < 1e-3);
        close(
            at_08,
            0.7958,
            1e-3,
            "0.80 at Mach 0.8, near the flat face's 0.9947",
        );
    }

    proptest::proptest! {
        /// Every shape at any fineness and joint angle gives a finite, non-negative coefficient
        /// from rest to Mach 5, and nothing jumps at `M_L`.
        #[test]
        fn every_shape_is_finite_and_non_negative(
            which in 0usize..6,
            parameter in 0.0f64..=1.0,
            fineness in 0.0f64..12.0,
            joint in 0.0f64..=std::f64::consts::FRAC_PI_2,
            mach in 0.0f64..5.0,
        ) {
            let shape = match which {
                0 => NoseShape::Conical {},
                1 => NoseShape::Ogive { radius_ratio: 1.0 + 10.0 * parameter },
                2 => NoseShape::Elliptical {},
                3 => NoseShape::PowerSeries { exponent: 0.05 + 0.95 * parameter },
                4 => NoseShape::ParabolicSeries { parameter },
                _ => NoseShape::Haack { parameter: parameter / 3.0 },
            };
            let curve = PressureDragCurve::new(shape, fineness, joint).unwrap();
            let c = curve.coefficient(mach).unwrap();
            proptest::prop_assert!(c.is_finite() && c >= 0.0, "{c}");
            let m_l = curve.transonic_lower_bound();
            let below = curve.coefficient(m_l * (1.0 - 1e-12)).unwrap();
            let at = curve.coefficient(m_l).unwrap();
            proptest::prop_assert!((below - at).abs() <= 1e-9 * (1.0 + at), "{below} {at}");
        }

        /// Physics review: the drag is continuous in the shape's parameter, across the measured
        /// shapes where the interpolation changes its ends (x^¼, x^½, x^¾; the ½ and ¾
        /// parabolas), at every Mach number.
        #[test]
        fn continuous_in_the_shape_parameter(
            which in 0usize..3,
            knot in 0usize..3,
            fineness in 0.5f64..8.0,
            mach in 0.0f64..5.0,
        ) {
            let shape = |p: f64| match which {
                0 => NoseShape::PowerSeries { exponent: p },
                1 => NoseShape::ParabolicSeries { parameter: p },
                _ => NoseShape::Haack { parameter: p / 3.0 },
            };
            let p = [0.25, 0.5, 0.75][knot];
            let at = |p: f64| {
                PressureDragCurve::new(shape(p), fineness, 0.0)
                    .unwrap()
                    .coefficient(mach)
                    .unwrap()
            };
            // Eq. B.9 raises the fineness-3 value to `log₄(f + 1)`, below 1 under fineness 3, so
            // the drag is continuous but steep where that value is near 0: the gap across the knot
            // shrinks with the step, and is small at a step of 1e-12 (a jump, like the 0.05 the
            // review found at n = ½, would not shrink).
            let gap = |step: f64| (at(p - step) - at(p + step)).abs();
            let (wide, narrow) = (gap(1e-6), gap(1e-12));
            proptest::prop_assert!(
                narrow <= wide + 1e-12 && narrow <= 0.01,
                "{shape:?} at Mach {mach}: gaps {wide} and {narrow}",
                shape = shape(p)
            );
        }
    }
}
