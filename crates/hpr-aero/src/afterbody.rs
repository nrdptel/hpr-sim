//! The afterbody faster than sound: a boattail's own pressure drag, and the base pressure behind
//! it.
//!
//! A boattail is a transition that narrows toward the tail. Below Mach 0.8 it keeps Niskanen's
//! rule, a share of the base drag on its decrease in area (Niskanen 2009 eq. 3.88,
//! [`crate::drag::boattail_factor`]). Faster than sound the air expands around the boattail's
//! shoulder, its pressure falls below the free stream's, and it pulls back on the boattail: a
//! wave drag the rule doesn't have. Behind the boattail the base's pressure rises, which lowers the
//! base drag. On the boattail's fore (cylinder) area `A₁ = π d₁²/4`, for a boattail of length `l`
//! from diameter `d₁` to `d₂`, area ratio `a = (d₂/d₁)²` and half-angle
//! `θ = atan((d₁ − d₂)/(2l))`:
//!
//! - **Attached flow, from Mach 1** ([`Boattail::attached_pressure_drag`]): MIL-HDBK-762's chart
//!   for conical boattails (Fig. 5-122, printed p. 5-187), `y = 4 C_D (l/d₁)²` against
//!   `x = √(M² − 1)/(2 l/d₁)` for `a` from 0.25 to 0.80, read into [`conical_boattail_chart`].
//!   The handbook cites no source for it; its values agree with Jack's second-order theory
//!   (NACA TN 2972, 1953) within −10% to +8% for `a` up to 0.6 (`validation/fixtures/aero/`
//!   `measured-boattails.json`). It is held to the **2D limit** `C_PM = −C_p,PM(M, θ)(1 − a)`:
//!   the pressure behind a Prandtl–Meyer expansion through `θ`
//!   ([`expansion_pressure_coefficient`]) over the whole annulus. On an axisymmetric boattail the
//!   pressure recovers aft of the shoulder, so the drag stays below that limit and approaches it
//!   as the boattail gets short against `β d₁` (the quasi-cylinder solution's recovery falls as
//!   `1/x`). Past the chart's end at `x = 1.4`, the drag closes the chart's gap to the limit as
//!   `1/x`: `C_D = [1 − (1 − r) 1.4/x] C_PM(M)`, with `r` the chart's share of the limit at
//!   `x = 1.4` (at most 1).
//! - **Separation** ([`Boattail::separation_weight`]): steep boattails separate. Cubbage's
//!   boattails (NACA RM L57B21, 1957, Mach 0.6–1.28) stay attached at 16° and separate completely
//!   by 30°, and a separated boattail sees about a cylinder's base pressure. Between 16° and 30°
//!   the drag moves linearly in `θ` from the attached value to the base drag coefficient on the
//!   annulus, `C_D,base(M)(1 − a)`.
//! - **Through Mach 1**: MIL-HDBK-762 finds no method for boattails at transonic speeds and
//!   advises extrapolating the supersonic drag "to peak value at a Mach number range of 1.0 ≤ M∞
//!   ≤ 1.2, with a sharp reduction to a lower value at subsonic speeds" (p. 5-47). The chart's
//!   near-sonic end is not used: from Mach 1 to 1.2 the drag is held at its Mach 1.2 value,
//!   below Mach 1 it falls on a straight line to the rule's value at Mach 0.9, and the rule
//!   holds below. Cubbage's measured boattails stay near their subsonic drag through Mach 0.9,
//!   are half-way up by 0.92 to 0.96, peak at Mach 1.0 to 1.1, and at 1.2 are 0.83 to 0.90 of
//!   that peak, so holding the Mach 1.2 value reads 10% to 20% under the peak.
//! - **The base behind a boattail** ([`boattail_base_pressure_ratio`]): MIL-HDBK-762 Fig. 5-141
//!   (printed p. 5-210, after Rubin, Brazzel and Henderson, 1970) correlates a boattail's base
//!   pressure with a cylinder's at Mach 2.5 to 3.5 as `p_cyl/p_bt = 0.442 + 0.558 a_b`, with
//!   `a_b` the base's area over the cylinder's. hpr takes the cylinder's pressure from Love's
//!   correlation (Fig. 5-139, printed p. 5-208; NACA TN 3819) and scales its own base drag by the
//!   ratio of the two coefficients, `k = C_p,bt/C_p,cyl`. Below Mach 2.5, where the correlation
//!   over-predicts the relief of the measured bases, `k` is held at its Mach 2.5 value, which
//!   matches Cortright and Schroeder's at Mach 1.91 and de Moraes and Nowitzky's at 1.59; below
//!   Mach 1 it returns to 1 by Mach 0.9, where the base drag is Niskanen's again. A separated
//!   boattail gives no relief (weight as above).
//!
//! A shoulder right behind a boattail, rising no higher than the boattail fell (a lip at its end),
//! sits in its wake and has no pressure drag ([`crate::drag::ComponentDragTerms::in_wake_of`]). How well
//! each piece agrees with the measurements is in the guide.
//!
//! Calisto's boattail, 0.472 calibres long from `a = 1` to 0.469 (18.4°), at Mach 1.5: the chart
//! gives 0.219 and the 2D limit 0.211, so the attached drag is 0.211; separation takes it 17% of
//! the way to the base's 0.088, to 0.190.
//!
//! ```
//! use hpr_aero::afterbody::Boattail;
//!
//! let calisto = Boattail::new(0.06, 0.127, 0.087)?;
//! assert!((calisto.attached_pressure_drag(1.5)? - 0.211).abs() < 5e-4);
//! assert!((calisto.pressure_drag_coefficient(1.5)? - 0.190).abs() < 5e-4);
//! # Ok::<(), hpr_aero::AeroError>(())
//! ```
//!
//! See [Boattails faster than sound][guide] in the guide.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/physics/aero.html#boattails-faster-than-sound

use serde::Serialize;

use crate::drag::{base_drag_coefficient, boattail_factor, check_mach_any};
use crate::error::{AeroError, check_dimension};

/// The ratio of specific heats of air, `γ = 1.4`.
const GAMMA: f64 = 1.4;

/// `√((γ + 1)/(γ − 1))`, `√6` for `γ = 1.4`.
const PM_K: f64 = 2.449_489_742_783_178;

/// The largest turning angle of a Prandtl–Meyer expansion from Mach 1, to a vacuum:
/// `(π/2)(√((γ + 1)/(γ − 1)) − 1)`, 130.45° (NACA Report 1135, 1953, eq. 172, p. 626).
pub const MAX_TURNING_RAD: f64 = std::f64::consts::FRAC_PI_2 * (PM_K - 1.0);

/// The boattail half-angle up to which the flow stays attached: 16°, Cubbage's steepest attached
/// boattail (NACA RM L57B21).
pub const SEPARATION_ONSET_RAD: f64 = 16.0 * std::f64::consts::PI / 180.0;

/// The boattail half-angle from which the flow is separated: 30°, Cubbage's shallowest separated
/// boattail (NACA RM L57B21).
pub const SEPARATION_COMPLETE_RAD: f64 = 30.0 * std::f64::consts::PI / 180.0;

/// The lowest Mach number of MIL-HDBK-762 Fig. 5-141's correlation, 2.5; below it the base
/// pressure ratio is held.
pub const BASE_RELIEF_MACH: f64 = 2.5;

/// Where a boattail's drag starts its transonic rise and its base its relief: Mach 0.9, below
/// which Cubbage's boattails keep their subsonic drag (NACA RM L57B21).
pub const TRANSONIC_ONSET_MACH: f64 = 0.9;

/// Where the transonic rise ends: from Mach 1 a boattail takes its supersonic drag (held to
/// [`SUPERSONIC_MODEL_MACH`]'s value) and its base the supersonic relief.
pub const SUPERSONIC_MACH: f64 = 1.0;

/// The lowest Mach number at which the supersonic boattail drag is evaluated, 1.2, the top of
/// MIL-HDBK-762's "peak value" range (p. 5-47); from Mach 1 to 1.2 its value there is held.
pub const SUPERSONIC_MODEL_MACH: f64 = 1.2;

/// The chart's abscissae `x = √(M² − 1)/(2 l/d₁)` at which [`CHART_Y`] was read.
pub const CHART_X: [f64; 20] = [
    0.06, 0.08, 0.1, 0.125, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1,
    1.2, 1.3, 1.4,
];

/// The chart's curves, by area ratio `(d₂/d₁)²`.
pub const CHART_AREA_RATIOS: [f64; 8] = [0.25, 0.30, 0.35, 0.40, 0.50, 0.60, 0.70, 0.80];

/// MIL-HDBK-762 Fig. 5-122 (printed p. 5-187, PDF p. 425), "Wave-Drag Coefficient of Conical
/// Boattails at Supersonic Speeds": `y = 4 C_D (l/d₁)²` on `π d₁²/4`, one row per
/// [`CHART_AREA_RATIOS`] at [`CHART_X`]. Read from a 200-dpi render with the grid fitted for tilt,
/// each curve fitted in `ln y` against `ln x` and checked on an overlay: ±(0.005 + 2%), the lines
/// being about 0.017 thick. The 0.80 curve's reading rises by 0.001 past `x = 1.2`, where the
/// printed line doesn't; it is held at 0.0332. The chart starts at `x = 0.05`, where the readings
/// hook; they start at 0.06.
pub const CHART_Y: [[f64; 20]; 8] = [
    [
        2.1326, 1.9357, 1.7839, 1.6375, 1.5231, 1.3518, 1.2251, 1.1244, 1.0402, 0.9677, 0.8467,
        0.7477, 0.6642, 0.5925, 0.5302, 0.4757, 0.4277, 0.3852, 0.3475, 0.3140,
    ],
    [
        1.8266, 1.6867, 1.5666, 1.4424, 1.3406, 1.1822, 1.0627, 0.9673, 0.8884, 0.8213, 0.7118,
        0.6250, 0.5538, 0.4941, 0.4431, 0.3990, 0.3606, 0.3270, 0.2972, 0.2708,
    ],
    [
        1.5427, 1.4371, 1.3387, 1.2330, 1.1445, 1.0054, 0.9003, 0.8169, 0.7484, 0.6905, 0.5972,
        0.5241, 0.4648, 0.4153, 0.3733, 0.3371, 0.3057, 0.2781, 0.2538, 0.2322,
    ],
    [
        1.2846, 1.1867, 1.1073, 1.0249, 0.9557, 0.8441, 0.7567, 0.6857, 0.6265, 0.5762, 0.4952,
        0.4324, 0.3822, 0.3412, 0.3071, 0.2782, 0.2535, 0.2322, 0.2136, 0.1973,
    ],
    [
        0.8662, 0.7911, 0.7271, 0.6625, 0.6112, 0.5347, 0.4798, 0.4378, 0.4042, 0.3762, 0.3316,
        0.2968, 0.2684, 0.2444, 0.2238, 0.2057, 0.1897, 0.1754, 0.1626, 0.1510,
    ],
    [
        0.5522, 0.5015, 0.4652, 0.4292, 0.3993, 0.3509, 0.3129, 0.2822, 0.2569, 0.2358, 0.2028,
        0.1783, 0.1596, 0.1450, 0.1333, 0.1238, 0.1161, 0.1096, 0.1042, 0.0996,
    ],
    [
        0.3266, 0.2894, 0.2697, 0.2526, 0.2384, 0.2137, 0.1920, 0.1732, 0.1569, 0.1430, 0.1209,
        0.1046, 0.0924, 0.0832, 0.0762, 0.0708, 0.0666, 0.0633, 0.0608, 0.0590,
    ],
    [
        0.1610, 0.1445, 0.1385, 0.1337, 0.1287, 0.1167, 0.1038, 0.0918, 0.0812, 0.0723, 0.0587,
        0.0494, 0.0431, 0.0388, 0.0360, 0.0343, 0.0334, 0.0332, 0.0332, 0.0332,
    ],
];

/// Love's correlation of turbulent base pressure behind cylinders, `−C_p,b` against Mach number
/// (MIL-HDBK-762 Fig. 5-139, printed p. 5-208, the solid line, after NACA TN 3819), read at
/// Mach 2.5 to 5: ±0.001 (±0.0015 at Mach 3, where symbols hide the line).
pub const LOVE_BASE_PRESSURE: [(f64, f64); 6] = [
    (2.5, 0.1195),
    (3.0, 0.097),
    (3.5, 0.0805),
    (4.0, 0.067),
    (4.5, 0.0565),
    (5.0, 0.0475),
];

/// The Prandtl–Meyer function `ν(M) = √((γ + 1)/(γ − 1)) atan √((γ − 1)(M² − 1)/(γ + 1)) −
/// atan √(M² − 1)`, rad: the angle through which a flow at Mach 1 turns, expanding, to reach `M`
/// (NACA Report 1135, 1953, eq. 171c, p. 626).
///
/// # Errors
///
/// [`AeroError::Domain`] below Mach 1 or for a non-finite Mach number.
pub fn prandtl_meyer_angle(mach: f64) -> Result<f64, AeroError> {
    if !(mach.is_finite() && mach >= 1.0) {
        return Err(AeroError::Domain {
            what: "Mach number of a Prandtl–Meyer expansion",
            value: mach,
        });
    }
    Ok(prandtl_meyer(mach))
}

fn prandtl_meyer(mach: f64) -> f64 {
    let b = (mach * mach - 1.0).sqrt();
    PM_K * (b / PM_K).atan() - b.atan()
}

/// The Mach number whose Prandtl–Meyer angle is `nu_rad`, for `0 ≤ ν < ν_max`
/// ([`MAX_TURNING_RAD`]): Newton's method on `ν(M)`, kept inside a bracket, to 1e-13 in `ν`.
fn inverse_prandtl_meyer(nu_rad: f64) -> f64 {
    if nu_rad <= 0.0 {
        return 1.0;
    }
    // A bracket [lo, hi] with ν(lo) ≤ ν < ν(hi).
    let (mut lo, mut hi) = (1.0, 2.0);
    while prandtl_meyer(hi) < nu_rad {
        lo = hi;
        hi *= 2.0;
    }
    let mut mach = 0.5 * (lo + hi);
    for _ in 0..100 {
        let f = prandtl_meyer(mach) - nu_rad;
        if f.abs() < 1e-13 {
            break;
        }
        if f > 0.0 {
            hi = mach;
        } else {
            lo = mach;
        }
        let m2 = mach * mach;
        let slope = (m2 - 1.0).sqrt() / (mach * (1.0 + 0.5 * (GAMMA - 1.0) * m2));
        let newton = mach - f / slope;
        mach = if newton > lo && newton < hi && slope > 0.0 {
            newton
        } else {
            0.5 * (lo + hi)
        };
    }
    mach
}

/// The pressure coefficient behind a two-dimensional isentropic (Prandtl–Meyer) expansion of a
/// flow at Mach `mach ≥ 1` through `turn_rad`: `M₂` from `ν(M₂) = ν(M) + θ`, then
/// `C_p = (p₂/p − 1)/(γ M²/2)` with `p₂/p = [(1 + (γ−1)M²/2)/(1 + (γ−1)M₂²/2)]^(γ/(γ−1))`
/// (NACA Report 1135: the pressures from eq. 44, the dynamic pressure `γ p M²/2` from eq. 31b,
/// p. 616). Past the largest turning angle the flow reaches a vacuum, `C_p = −2/(γ M²)`.
///
/// # Errors
///
/// [`AeroError::Domain`] below Mach 1, for a non-finite Mach number, or a turning angle outside
/// `[0, π/2]`.
pub fn expansion_pressure_coefficient(mach: f64, turn_rad: f64) -> Result<f64, AeroError> {
    let nu = prandtl_meyer_angle(mach)?;
    if !(0.0..=std::f64::consts::FRAC_PI_2).contains(&turn_rad) {
        return Err(AeroError::Domain {
            what: "expansion turning angle",
            value: turn_rad,
        });
    }
    let q = 0.5 * GAMMA * mach * mach;
    let target = nu + turn_rad;
    if target >= MAX_TURNING_RAD {
        return Ok(-1.0 / q);
    }
    let m2 = inverse_prandtl_meyer(target);
    let h = 0.5 * (GAMMA - 1.0);
    let ratio = ((1.0 + h * mach * mach) / (1.0 + h * m2 * m2)).powf(GAMMA / (GAMMA - 1.0));
    Ok((ratio - 1.0) / q)
}

/// A curve's value at `x`: log-log between the readings, held below the first.
fn chart_curve(row: &[f64; 20], x: f64) -> f64 {
    if x <= CHART_X[0] {
        return row[0];
    }
    let last = CHART_X.len() - 1;
    if x >= CHART_X[last] {
        return row[last];
    }
    // `CHART_X[0] < x < CHART_X[last]`, so the partition point is between 1 and `last`.
    let i = CHART_X.partition_point(|&c| c <= x) - 1;
    let t = (x / CHART_X[i]).ln() / (CHART_X[i + 1] / CHART_X[i]).ln();
    (row[i].ln() * (1.0 - t) + row[i + 1].ln() * t).exp()
}

/// MIL-HDBK-762 Fig. 5-122's `y = 4 C_D (l/d₁)²` for a conical boattail of area ratio
/// `a = (d₂/d₁)²` at `x = √(M² − 1)/(2 l/d₁)` from 0 to 1.4 ([`CHART_Y`]): log-log in `x` between
/// the readings, held below `x = 0.06`, and linear in `a` between the curves. Past the chart's
/// last curve, `a > 0.8`, it goes to 0 at `a = 1` as `(1 − √a)²` (linear theory's pressure is
/// proportional to the surface slope, so at a fixed length the drag goes as the slope squared);
/// below its first, `a < 0.25`, it continues the straight line through the 0.25 and 0.30 curves.
///
/// # Errors
///
/// [`AeroError::Domain`] for `x` outside `[0, 1.4]` or `a` outside `[0, 1]`.
pub fn conical_boattail_chart(x: f64, area_ratio: f64) -> Result<f64, AeroError> {
    if !(0.0..=CHART_X[CHART_X.len() - 1]).contains(&x) {
        return Err(AeroError::Domain {
            what: "boattail chart abscissa",
            value: x,
        });
    }
    if !(0.0..=1.0).contains(&area_ratio) {
        return Err(AeroError::Domain {
            what: "boattail area ratio",
            value: area_ratio,
        });
    }
    let curve = |i: usize| chart_curve(&CHART_Y[i], x);
    let n = CHART_AREA_RATIOS.len();
    let (first, last) = (CHART_AREA_RATIOS[0], CHART_AREA_RATIOS[n - 1]);
    Ok(if area_ratio >= last {
        let slope = (1.0 - area_ratio.sqrt()) / (1.0 - last.sqrt());
        curve(n - 1) * slope * slope
    } else if area_ratio <= first {
        let (y0, y1) = (curve(0), curve(1));
        y0 + (y0 - y1) * (first - area_ratio) / (CHART_AREA_RATIOS[1] - first)
    } else {
        // `first < a < last`, so the partition point is between 1 and `n − 1`.
        let i = CHART_AREA_RATIOS.partition_point(|&c| c <= area_ratio) - 1;
        let t =
            (area_ratio - CHART_AREA_RATIOS[i]) / (CHART_AREA_RATIOS[i + 1] - CHART_AREA_RATIOS[i]);
        curve(i) * (1.0 - t) + curve(i + 1) * t
    })
}

/// Love's `−C_p,b` for a cylinder at `mach`, linear between [`LOVE_BASE_PRESSURE`]'s readings and
/// held at their ends.
fn love_base_pressure(mach: f64) -> f64 {
    let points = &LOVE_BASE_PRESSURE;
    let (first, last) = (points[0], points[points.len() - 1]);
    if mach <= first.0 {
        return first.1;
    }
    if mach >= last.0 {
        return last.1;
    }
    let i = points.partition_point(|p| p.0 <= mach) - 1;
    let ((m0, c0), (m1, c1)) = (points[i], points[i + 1]);
    c0 + (c1 - c0) * (mach - m0) / (m1 - m0)
}

/// The base pressure behind a boattail over a cylinder's, as a ratio of pressure coefficients
/// `k = C_p,bt/C_p,cyl` that scales the base drag, for a base of area ratio `a_b` (its area over
/// the boattail's fore area) and an attached boattail (MIL-HDBK-762 Fig. 5-141, printed p. 5-210):
///
/// - From Mach 2.5, `p_bt = p_cyl/(0.442 + 0.558 a_b)` with the cylinder's `p_cyl/p =
///   1 + C_p,cyl γM²/2` from Love's correlation ([`LOVE_BASE_PRESSURE`]), and
///   `k = (1 − p_bt/p)/(1 − p_cyl/p)`, not below 0 (no measured base pressure is above the free
///   stream's).
/// - From Mach 1 to 2.5, `k` at Mach 2.5.
/// - From Mach 0.9 to 1, a straight line from 1 to that value; 1 below.
///
/// # Errors
///
/// [`AeroError::Domain`] for a negative or non-finite Mach number, or `a_b` outside `[0, 1]`.
pub fn boattail_base_pressure_ratio(mach: f64, base_area_ratio: f64) -> Result<f64, AeroError> {
    check_mach_any(mach)?;
    if !(0.0..=1.0).contains(&base_area_ratio) {
        return Err(AeroError::Domain {
            what: "base area ratio",
            value: base_area_ratio,
        });
    }
    let supersonic = |m: f64| {
        let cylinder = love_base_pressure(m);
        let p_cyl = 1.0 - cylinder * 0.5 * GAMMA * m * m;
        let p_bt = p_cyl / (0.442 + 0.558 * base_area_ratio);
        ((1.0 - p_bt) / (1.0 - p_cyl)).max(0.0)
    };
    Ok(if mach <= TRANSONIC_ONSET_MACH {
        1.0
    } else if mach < SUPERSONIC_MACH {
        let k = supersonic(BASE_RELIEF_MACH);
        1.0 + (k - 1.0) * (mach - TRANSONIC_ONSET_MACH) / (SUPERSONIC_MACH - TRANSONIC_ONSET_MACH)
    } else {
        supersonic(mach.max(BASE_RELIEF_MACH))
    })
}

/// A boattail's geometry and the terms of its pressure drag that don't depend on the Mach number.
/// Coefficients are on its fore area `π d₁²/4`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Boattail {
    /// Length `l`, m.
    pub length_m: f64,
    /// Fore diameter `d₁`, m.
    pub fore_diameter_m: f64,
    /// Aft diameter `d₂`, m.
    pub aft_diameter_m: f64,
    /// Area ratio `a = (d₂/d₁)²`.
    pub area_ratio: f64,
    /// Length over fore diameter, `l/d₁`.
    pub length_ratio: f64,
    /// Half-angle of the cone through the same ends, `θ = atan((d₁ − d₂)/(2l))`, rad.
    pub half_angle_rad: f64,
    /// Niskanen's boattail factor (eq. 3.88), used below Mach 0.8.
    pub rule_factor: f64,
    /// The share of the separated value in the supersonic drag and base pressure:
    /// ([`SEPARATION_ONSET_RAD`], [`SEPARATION_COMPLETE_RAD`]) mapped linearly to (0, 1).
    pub separation_weight: f64,
    /// The chart's share of the 2D limit at its end, `x = 1.4`, at most 1: `r` in the drag past
    /// the chart.
    pub chart_end_ratio: f64,
}

impl Boattail {
    /// A boattail of `length_m` narrowing from `fore_diameter_m` to `aft_diameter_m`, compared as
    /// the cone through the same ends. A curved boattail drags at least as much as that cone
    /// (Jack found the cone's wave drag the smallest of conical, tangent-parabolic and
    /// secant-parabolic boattails of the same length and area ratio, NACA TN 2972 p. 1), so hpr
    /// under-predicts it.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a non-positive or non-finite length or fore diameter, a negative
    /// aft diameter, or diameters that don't decrease.
    pub fn new(
        length_m: f64,
        fore_diameter_m: f64,
        aft_diameter_m: f64,
    ) -> Result<Self, AeroError> {
        check_dimension("boattail length", length_m, false)?;
        let rule_factor = boattail_factor(length_m, fore_diameter_m, aft_diameter_m)?;
        let ratio = aft_diameter_m / fore_diameter_m;
        let length_ratio = length_m / fore_diameter_m;
        let half_angle_rad = ((fore_diameter_m - aft_diameter_m) / (2.0 * length_m)).atan();
        let separation_weight = ((half_angle_rad - SEPARATION_ONSET_RAD)
            / (SEPARATION_COMPLETE_RAD - SEPARATION_ONSET_RAD))
            .clamp(0.0, 1.0);
        let mut boattail = Self {
            length_m,
            fore_diameter_m,
            aft_diameter_m,
            area_ratio: ratio * ratio,
            length_ratio,
            half_angle_rad,
            rule_factor,
            separation_weight,
            chart_end_ratio: 1.0,
        };
        // The Mach number at which `x = 1.4`: `√(M² − 1) = 2.8 l/d₁`.
        let end = CHART_X[CHART_X.len() - 1];
        let beta = 2.0 * end * length_ratio;
        let mach_end = (1.0 + beta * beta).sqrt();
        let chart = boattail.chart_drag(end)?;
        boattail.chart_end_ratio = (chart / boattail.expansion_limit(mach_end)?).min(1.0);
        Ok(boattail)
    }

    /// The chart's `C_D` at `x`.
    fn chart_drag(&self, x: f64) -> Result<f64, AeroError> {
        let l = self.length_ratio;
        Ok(conical_boattail_chart(x, self.area_ratio)? / (4.0 * l * l))
    }

    /// The 2D limit `C_PM = −C_p,PM(M, θ)(1 − a)` at `mach ≥ 1`: a Prandtl–Meyer expansion's
    /// pressure over the whole annulus.
    ///
    /// # Errors
    ///
    /// As [`expansion_pressure_coefficient`].
    pub fn expansion_limit(&self, mach: f64) -> Result<f64, AeroError> {
        Ok(-expansion_pressure_coefficient(mach, self.half_angle_rad)? * (1.0 - self.area_ratio))
    }

    /// The attached boattail's pressure drag at `mach ≥ 1`: the chart held to the 2D limit, and
    /// past the chart's end the limit less the chart's share of it closing as `1/x` (module
    /// docs).
    ///
    /// # Errors
    ///
    /// As [`expansion_pressure_coefficient`].
    pub fn attached_pressure_drag(&self, mach: f64) -> Result<f64, AeroError> {
        let limit = self.expansion_limit(mach)?;
        let x = (mach * mach - 1.0).sqrt() / (2.0 * self.length_ratio);
        let end = CHART_X[CHART_X.len() - 1];
        Ok(if x <= end {
            self.chart_drag(x)?.min(limit)
        } else {
            (1.0 - (1.0 - self.chart_end_ratio) * end / x) * limit
        })
    }

    /// The boattail's pressure drag on its fore area at any Mach number (module docs): Niskanen's
    /// rule to Mach 0.9, a straight line to Mach 1, the Mach 1.2 value held to Mach 1.2, and from
    /// there the attached drag blended toward the separated value.
    ///
    /// # Errors
    ///
    /// [`AeroError::Domain`] for a negative or non-finite Mach number.
    pub fn pressure_drag_coefficient(&self, mach: f64) -> Result<f64, AeroError> {
        check_mach_any(mach)?;
        let annulus = 1.0 - self.area_ratio;
        let supersonic = |m: f64| -> Result<f64, AeroError> {
            let separated = base_drag_coefficient(m)? * annulus;
            let w = self.separation_weight;
            Ok((1.0 - w) * self.attached_pressure_drag(m)? + w * separated)
        };
        if mach <= TRANSONIC_ONSET_MACH {
            Ok(self.rule_factor * base_drag_coefficient(mach)? * annulus)
        } else if mach < SUPERSONIC_MACH {
            let low = self.rule_factor * base_drag_coefficient(TRANSONIC_ONSET_MACH)? * annulus;
            let high = supersonic(SUPERSONIC_MODEL_MACH)?;
            let t = (mach - TRANSONIC_ONSET_MACH) / (SUPERSONIC_MACH - TRANSONIC_ONSET_MACH);
            Ok(low + (high - low) * t)
        } else {
            supersonic(mach.max(SUPERSONIC_MODEL_MACH))
        }
    }

    /// The factor on the base drag of a base right behind this boattail, of area ratio `a_b` (the
    /// base's area over the boattail's fore area): [`boattail_base_pressure_ratio`], moved toward
    /// 1 by the separation weight.
    ///
    /// # Errors
    ///
    /// As [`boattail_base_pressure_ratio`].
    pub fn base_pressure_ratio(&self, mach: f64, base_area_ratio: f64) -> Result<f64, AeroError> {
        let k = boattail_base_pressure_ratio(mach, base_area_ratio)?;
        let w = self.separation_weight;
        Ok((1.0 - w) * k + w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(got: f64, want: f64, tol: f64, what: &str) {
        assert!((got - want).abs() <= tol, "{what}: {got} against {want}");
    }

    /// The Prandtl–Meyer function against NACA Report 1135's table (M = 2: 26.380°; M = 3:
    /// 49.757°), its limit (eq. 172: 130.45°), and the inverse.
    #[test]
    fn prandtl_meyer_matches_tables_and_inverts() {
        let deg = |r: f64| r.to_degrees();
        close(deg(prandtl_meyer_angle(1.0).unwrap()), 0.0, 1e-12, "ν(1)");
        close(deg(prandtl_meyer_angle(2.0).unwrap()), 26.380, 5e-4, "ν(2)");
        close(deg(prandtl_meyer_angle(3.0).unwrap()), 49.757, 5e-4, "ν(3)");
        close(deg(MAX_TURNING_RAD), 130.454, 5e-4, "ν_max");
        assert!(prandtl_meyer_angle(0.99).is_err());
        assert!(prandtl_meyer_angle(f64::NAN).is_err());
        for mach in [1.0, 1.001, 1.2, 2.0, 4.6, 20.0, 300.0] {
            let back = inverse_prandtl_meyer(prandtl_meyer(mach));
            close(back, mach, 1e-9 * mach, "inverse");
        }
    }

    /// Expansion pressure by hand: from Mach 1.5 through 15°, `ν` goes from 11.905° to 26.905°,
    /// `M₂` = 2.0191, `p₂/p = (1.45/1.8154)^3.5 = 0.4554` and `C_p = −0.3458`; a turn past the
    /// vacuum limit gives `−2/(γM²)`.
    #[test]
    fn expansion_pressure_by_hand_and_at_the_vacuum() {
        let cp = expansion_pressure_coefficient(1.5, 15f64.to_radians()).unwrap();
        close(cp, -0.3458, 5e-5, "Mach 1.5, 15°");
        close(
            expansion_pressure_coefficient(1.5, 0.0).unwrap(),
            0.0,
            1e-12,
            "no turn",
        );
        let vacuum = expansion_pressure_coefficient(4.0, std::f64::consts::FRAC_PI_2).unwrap();
        close(vacuum, -1.0 / (0.7 * 16.0), 1e-12, "vacuum");
        assert!(expansion_pressure_coefficient(1.5, -0.1).is_err());
    }

    /// The chart gives back its readings at the grid, is log-log between them, falls with `x`
    /// and with the area ratio, and goes to 0 at `a = 1`.
    #[test]
    fn chart_reproduces_its_readings_and_orders() {
        for (i, &a) in CHART_AREA_RATIOS.iter().enumerate() {
            for (j, &x) in CHART_X.iter().enumerate() {
                close(
                    conical_boattail_chart(x, a).unwrap(),
                    CHART_Y[i][j],
                    1e-12,
                    "grid",
                );
            }
        }
        // Halfway in ln x between 0.4 and 0.5 on the 0.50 curve: the geometric mean.
        let x = (0.4f64 * 0.5).sqrt();
        close(
            conical_boattail_chart(x, 0.5).unwrap(),
            (0.3762f64 * 0.3316).sqrt(),
            1e-12,
            "log-log",
        );
        close(
            conical_boattail_chart(0.0, 0.5).unwrap(),
            0.8662,
            1e-12,
            "held below 0.06",
        );
        close(
            conical_boattail_chart(0.7, 1.0).unwrap(),
            0.0,
            1e-12,
            "a = 1",
        );
        let mut previous = f64::INFINITY;
        for step in 0..=100 {
            let a = f64::from(step) / 100.0;
            let y = conical_boattail_chart(0.5, a).unwrap();
            assert!(y < previous || a == 0.0, "falls with a at {a}");
            previous = y;
        }
        assert!(conical_boattail_chart(1.41, 0.5).is_err());
        assert!(conical_boattail_chart(0.5, 1.01).is_err());
    }

    /// The module's worked example, Calisto's boattail at Mach 1.5, and the pieces of the drag:
    /// continuous at Mach 0.8 and 1, the rule below, never above the 2D limit, and the chart's
    /// own value inside it.
    #[test]
    fn calistos_boattail_by_hand() {
        let b = Boattail::new(0.06, 0.127, 0.087).unwrap();
        close(b.area_ratio, 0.469_28, 1e-5, "area ratio");
        close(b.half_angle_rad.to_degrees(), 18.435, 1e-3, "angle");
        close(b.separation_weight, (18.435 - 16.0) / 14.0, 1e-4, "weight");
        // x = √1.25/(2 × 0.472) = 1.183; the chart at a = 0.469: 0.219 on d₁.
        let x = 1.25f64.sqrt() / (2.0 * b.length_ratio);
        let chart =
            conical_boattail_chart(x, b.area_ratio).unwrap() / (4.0 * b.length_ratio.powi(2));
        close(chart, 0.2189, 5e-4, "chart");
        let limit = b.expansion_limit(1.5).unwrap();
        close(limit, 0.2112, 5e-4, "2D limit");
        close(
            b.attached_pressure_drag(1.5).unwrap(),
            limit.min(chart),
            1e-12,
            "attached",
        );
        let separated = 0.25 / 1.5 * (1.0 - b.area_ratio);
        let w = b.separation_weight;
        close(
            b.pressure_drag_coefficient(1.5).unwrap(),
            (1.0 - w) * limit.min(chart) + w * separated,
            1e-12,
            "blended",
        );
        let rule =
            |m: f64| b.rule_factor * base_drag_coefficient(m).unwrap() * (1.0 - b.area_ratio);
        close(
            b.pressure_drag_coefficient(0.5).unwrap(),
            rule(0.5),
            1e-12,
            "rule",
        );
        close(
            b.pressure_drag_coefficient(0.9).unwrap(),
            rule(0.9),
            1e-12,
            "rule to 0.9",
        );
        let peak = b.pressure_drag_coefficient(1.2).unwrap();
        close(
            b.pressure_drag_coefficient(1.0).unwrap(),
            peak,
            1e-12,
            "held from 1",
        );
        close(
            b.pressure_drag_coefficient(1.1).unwrap(),
            peak,
            1e-12,
            "held to 1.2",
        );
        close(
            b.pressure_drag_coefficient(0.95).unwrap(),
            0.5 * (rule(0.9) + peak),
            1e-12,
            "half-way at 0.95",
        );
        for (edge, what) in [
            (TRANSONIC_ONSET_MACH, "0.9"),
            (SUPERSONIC_MACH, "1"),
            (SUPERSONIC_MODEL_MACH, "1.2"),
        ] {
            let below = b.pressure_drag_coefficient(edge - 1e-9).unwrap();
            let above = b.pressure_drag_coefficient(edge + 1e-9).unwrap();
            close(below, above, 1e-6, what);
        }
        for step in 0..=400 {
            let mach = 1.0 + f64::from(step) / 100.0;
            let attached = b.attached_pressure_drag(mach).unwrap();
            assert!(
                attached <= b.expansion_limit(mach).unwrap() + 1e-12,
                "Mach {mach}"
            );
            assert!(attached > 0.0, "Mach {mach}");
        }
    }

    /// Past the chart's end the drag closes on the 2D limit as `1/x`, starting from the chart's
    /// value at `x = 1.4`: continuous there, rising toward the limit's share.
    #[test]
    fn past_the_chart_the_drag_closes_on_the_2d_limit() {
        // A long, gentle boattail reaches x = 1.4 early: 2 calibres to a = 0.5.
        let b = Boattail::new(0.2, 0.1, 0.1 * 0.5f64.sqrt()).unwrap();
        let mach_end = (1.0 + (2.8 * b.length_ratio).powi(2)).sqrt();
        let chart = b.chart_drag(1.4).unwrap();
        assert!(chart < b.expansion_limit(mach_end).unwrap());
        close(
            b.attached_pressure_drag(mach_end).unwrap(),
            chart,
            1e-9,
            "at the end",
        );
        close(
            b.attached_pressure_drag(mach_end + 1e-9).unwrap(),
            chart,
            1e-7,
            "just past",
        );
        let mut previous = 0.0;
        for mach in [6.0, 8.0, 12.0, 20.0] {
            let share = b.attached_pressure_drag(mach).unwrap() / b.expansion_limit(mach).unwrap();
            assert!(share > previous && share < 1.0, "Mach {mach}: {share}");
            previous = share;
        }
    }

    /// Separation: none to 16°, all from 30°, where a boattail drags like the base it uncovers
    /// and gives its base no relief.
    #[test]
    fn steep_boattails_separate() {
        let d = 0.1;
        let at = |deg: f64| {
            let l = (d - 0.06) / (2.0 * deg.to_radians().tan());
            Boattail::new(l, d, 0.06).unwrap()
        };
        close(at(15.0).separation_weight, 0.0, 0.0, "15°");
        close(at(16.0).separation_weight, 0.0, 1e-12, "16°");
        close(at(23.0).separation_weight, 0.5, 1e-12, "23°");
        let steep = at(45.0);
        close(steep.separation_weight, 1.0, 0.0, "45°");
        close(
            steep.pressure_drag_coefficient(2.0).unwrap(),
            0.125 * (1.0 - 0.36),
            1e-12,
            "separated",
        );
        close(
            steep.base_pressure_ratio(2.0, 0.36).unwrap(),
            1.0,
            0.0,
            "no relief",
        );
    }

    /// The base pressure ratio: 1 below Mach 0.8, held below 2.5, Fig. 5-141's line in pressure
    /// from 2.5 (at Mach 3, `p_cyl/p_bt = 0.442 + 0.558 a_b` exactly), 1 for a base as large as the
    /// cylinder, and none below 0.
    #[test]
    fn base_pressure_ratio_follows_fig_5_141() {
        let k = |m: f64, a: f64| boattail_base_pressure_ratio(m, a).unwrap();
        close(k(0.5, 0.469), 1.0, 0.0, "subsonic");
        close(k(0.9, 0.469), 1.0, 0.0, "to Mach 0.9");
        close(k(1.0, 0.469), k(2.5, 0.469), 0.0, "held");
        close(k(1.8, 0.469), k(2.5, 0.469), 0.0, "held");
        close(k(3.0, 1.0), 1.0, 1e-12, "no boattail");
        // At Mach 3, Love's cylinder gives −0.097, so p_cyl/p = 1 − 0.097 × 6.3.
        let p_cyl = 1.0 - 0.097 * 0.7 * 9.0;
        let a = 0.338;
        let p_bt = p_cyl / (0.442 + 0.558 * a);
        close(k(3.0, a), (1.0 - p_bt) / (1.0 - p_cyl), 1e-12, "Fig. 5-141");
        // Calisto's base at Mach 2.5: 0.616.
        close(k(2.5, 0.469), 0.6157, 5e-4, "Calisto");
        assert!(k(4.9, 0.0) >= 0.0);
        let mid = k(0.95, 0.469);
        close(mid, 0.5 * (1.0 + k(2.5, 0.469)), 1e-12, "join");
        assert!(boattail_base_pressure_ratio(2.0, 1.1).is_err());
    }

    /// Every boattail from 1° to 89°, `a` from 0 to 0.95, gives a finite, non-negative drag at
    /// every Mach number to 5, and a base ratio in `[0, 1]`.
    #[test]
    fn every_boattail_is_finite() {
        for deg in [1.0f64, 3.0, 8.0, 15.0, 16.0, 20.0, 30.0, 60.0, 89.0] {
            for a in [0.0f64, 0.05, 0.2, 0.25, 0.469, 0.8, 0.95] {
                let d2 = 0.1 * a.sqrt();
                let l = (0.1 - d2) / (2.0 * deg.to_radians().tan());
                let b = Boattail::new(l, 0.1, d2).unwrap();
                for step in 0..500 {
                    let mach = f64::from(step) / 100.0;
                    let c = b.pressure_drag_coefficient(mach).unwrap();
                    assert!(c.is_finite() && c >= 0.0, "{deg}° a {a} Mach {mach}: {c}");
                    let k = b.base_pressure_ratio(mach, a).unwrap();
                    assert!((0.0..=1.0).contains(&k), "{deg}° a {a} Mach {mach}: {k}");
                }
            }
        }
    }
}
