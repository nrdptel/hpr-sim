//! Fin sets: Barrowman's subsonic normal-force slope and centre of pressure, with the
//! Prandtl–Glauert factor, the fin-count and roll terms, and fin–body interference.
//!
//! - **One fin** (Diederich's planform correlation as Barrowman applies it; Barrowman 1967
//!   eq. 3-6, Niskanen 2009 eq. 3.40):
//!   `(C_Nα)₁ = 2π (s²/A_ref) / (1 + √(1 + (β s² / (A_fin cos Γ_c))²))`, `β = √(1 − M²)`,
//!   with `s` the span from the body surface, `A_fin` one fin's area and `Γ_c` the mid-chord
//!   sweep. At `M = 0` it is Barrowman 1966 eq. 50 (eq. 57 for a trapezoid, where
//!   `s²/(A_fin cos Γ_c) = 2ℓ/(c_r + c_t)`). At `M → 1` it tends to `π s²/A_ref`.
//! - **Mean aerodynamic chord** (Niskanen eq. 3.30–3.32): `c̄ = (1/A)∫c² dy`,
//!   `y_MAC = (1/A)∫y c dy`, `x_MAC,LE = (1/A)∫x_LE c dy`, and the centre of pressure at the
//!   quarter chord `X_f = x_MAC,LE + c̄/4`, fixed through subsonic flow (Barrowman 1967 p. 6). For a
//!   trapezoid these give Barrowman 1966 eq. 76a (Niskanen eq. 3.34); for an ellipse on its root
//!   chord `X_f = (½ − 2/(3π)) c_r`.
//! - **Freeform fins** (Niskanen pp. 27–29): the chord runs from the leading edge to the trailing
//!   edge, so the gap of a jagged edge counts toward the centre of pressure but not toward the
//!   area in `(C_Nα)₁`; `Γ_c` is the span average of the angle between the mid-chord points.
//! - **N fins** (Niskanen eq. 3.51–3.53, OpenRocket technical documentation 13.05 eq. 3.54): a fin
//!   at angle `Λ` to the lateral airflow adds `(C_Nα)₁ sin² Λ` in the plane of the flow, and
//!   `Σ sin² Λ_k = N/2` for three or more even fins. Fin–fin interference scales 5, 6, 7 and 8
//!   fins by 0.948, 0.913, 0.854 and 0.810: six and eight fins give 1.37 and 1.62 times four fins
//!   (MIL-HDBK-762(MI) p. 5-24), five and seven are interpolated. More than eight fins have no
//!   source and are refused.
//! - **Fin–body interference** (Barrowman 1966 eq. 77, Niskanen eq. 3.56):
//!   `K_T(B) = 1 + r_t/(s + r_t)`, with `r_t` the body radius at the fins.
//!
//! See `docs/physics/aero.md`.

use std::f64::consts::{PI, TAU};

use hpr_design::FinPlanform;
use serde::{Deserialize, Serialize};

use crate::error::{AeroError, check_dimension, check_mach};

/// A fin's aerodynamic geometry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FinGeometry {
    /// Span from the root (body surface) to the tip, m.
    pub span_m: f64,
    /// Area of one side of one fin, m²; the area in the normal-force slope.
    pub area_m2: f64,
    /// Mid-chord sweep `Γ_c`, rad, positive with the tip aft.
    pub midchord_sweep_rad: f64,
    /// Leading-edge sweep `Γ_L`, rad, positive with the tip aft; the span average of the edge's
    /// angle when it is curved or kinked (Niskanen 2009 eq. 3.91).
    pub leading_edge_sweep_rad: f64,
    /// Length of the mean aerodynamic chord `c̄`, m.
    pub mac_length_m: f64,
    /// Leading edge of the mean aerodynamic chord, m aft of the root leading edge.
    pub mac_leading_edge_m: f64,
    /// Spanwise station of the mean aerodynamic chord, m from the root.
    pub mac_span_m: f64,
}

impl FinGeometry {
    /// The geometry of a planform (Niskanen 2009 eq. 3.30–3.34; see the module docs).
    ///
    /// # Errors
    ///
    /// The planform's own validation errors, and [`AeroError::Domain`] for a fin without area.
    pub fn from_planform(planform: &FinPlanform) -> Result<Self, AeroError> {
        planform.validate()?;
        match *planform {
            FinPlanform::Trapezoidal {
                root_chord_m: c_r,
                tip_chord_m: c_t,
                span_m: s,
                sweep_m: x_t,
            } => {
                let sum = c_r + c_t;
                let y_mac = s / 3.0 * (c_r + 2.0 * c_t) / sum;
                Ok(Self {
                    span_m: s,
                    area_m2: 0.5 * s * sum,
                    midchord_sweep_rad: (x_t + 0.5 * c_t - 0.5 * c_r).atan2(s),
                    leading_edge_sweep_rad: x_t.atan2(s),
                    mac_length_m: 2.0 / 3.0 * (c_r * c_r + c_r * c_t + c_t * c_t) / sum,
                    mac_leading_edge_m: x_t * y_mac / s,
                    mac_span_m: y_mac,
                })
            }
            FinPlanform::Elliptical {
                root_chord_m: c_r,
                span_m: s,
            } => {
                let mac = 8.0 * c_r / (3.0 * PI);
                Ok(Self {
                    span_m: s,
                    area_m2: 0.25 * PI * c_r * s,
                    midchord_sweep_rad: 0.0,
                    leading_edge_sweep_rad: elliptical_leading_edge_sweep(0.5 * c_r / s),
                    mac_length_m: mac,
                    mac_leading_edge_m: 0.5 * (c_r - mac),
                    mac_span_m: 4.0 * s / (3.0 * PI),
                })
            }
            FinPlanform::Freeform { ref points_m } => Self::freeform(planform, points_m),
            _ => Err(AeroError::Unsupported("this fin planform".to_owned())),
        }
    }

    /// A freeform outline, integrated band by band between vertex heights. Edges of a simple
    /// polygon don't cross, so inside a band the leading and trailing edges are single straight
    /// edges: every integrand is a quadratic in `y`, and the three-point Gauss rule is exact.
    fn freeform(planform: &FinPlanform, points: &[[f64; 2]]) -> Result<Self, AeroError> {
        let mut heights: Vec<f64> = points.iter().map(|p| p[1]).collect();
        heights.sort_by(f64::total_cmp);
        heights.dedup();
        let span = planform.span_m();
        // Gauss–Legendre nodes and weights on [−1, 1].
        let nodes = [
            (-(0.6f64).sqrt(), 5.0 / 9.0),
            (0.0, 8.0 / 9.0),
            ((0.6f64).sqrt(), 5.0 / 9.0),
        ];
        let mut chords = Vec::new();
        let (mut area, mut filled, mut c2, mut yc, mut xc, mut sweep, mut le_sweep) =
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        for band in heights.windows(2) {
            let (lo, hi) = (band[0], band[1]);
            // Vertex heights a few rounding steps apart (a tip given in inches and in metres) make
            // a band too thin for its Gauss points to land inside it. Its share of any integral is
            // below 1e-12 of the fin's.
            if hi - lo <= 1e-12 * span {
                continue;
            }
            let half = 0.5 * (hi - lo);
            let mid = 0.5 * (hi + lo);
            let mut mids = [0.0; 2];
            let mut leading = [0.0; 2];
            for (k, &(t, w)) in nodes.iter().enumerate() {
                let y = mid + half * t;
                planform.chords_at(y, &mut chords);
                let (Some(first), Some(last)) = (chords.first(), chords.last()) else {
                    // A valid outline has chords at every height inside its span.
                    return Err(AeroError::Domain {
                        what: "freeform fin height without a chord",
                        value: y,
                    });
                };
                let (le, te) = (first.0, last.1);
                let c = te - le;
                let wh = w * half;
                area += wh * chords.iter().map(|(a, b)| b - a).sum::<f64>();
                filled += wh * c;
                c2 += wh * c * c;
                yc += wh * y * c;
                xc += wh * le * c;
                if k != 1 {
                    mids[k / 2] = 0.5 * (le + te);
                    leading[k / 2] = le;
                }
            }
            // The mid-chord line is straight in the band: its angle from the outer two nodes.
            let dy = 2.0 * half * (0.6f64).sqrt();
            sweep += (hi - lo) * (mids[1] - mids[0]).atan2(dy);
            le_sweep += (hi - lo) * (leading[1] - leading[0]).atan2(dy);
        }
        if !(area > 0.0 && filled > 0.0 && span > 0.0) {
            return Err(AeroError::Domain {
                what: "fin area",
                value: area,
            });
        }
        Ok(Self {
            span_m: span,
            area_m2: area,
            midchord_sweep_rad: sweep / span,
            leading_edge_sweep_rad: le_sweep / span,
            mac_length_m: c2 / filled,
            mac_leading_edge_m: xc / filled,
            mac_span_m: yc / filled,
        })
    }

    /// Centre of pressure at the quarter mean aerodynamic chord, m aft of the root leading edge
    /// (Niskanen 2009, `X_f = x_MAC,LE + 0.25 c̄`; Barrowman 1966 eq. 76a for a trapezoid).
    pub fn centre_of_pressure_m(&self) -> f64 {
        self.mac_leading_edge_m + 0.25 * self.mac_length_m
    }

    /// Normal-force slope of one fin, per radian of the angle between the flow and the fin
    /// (Barrowman 1967 eq. 3-6; Niskanen 2009 eq. 3.40).
    ///
    /// # Errors
    ///
    /// [`AeroError::Mach`] outside `[0, 1)`, and [`AeroError::Domain`] for a non-positive
    /// reference area.
    pub fn single_fin_slope(&self, reference_area_m2: f64, mach: f64) -> Result<f64, AeroError> {
        check_mach(mach)?;
        check_dimension("reference area", reference_area_m2, false)?;
        Ok(self.slope_at((1.0 - mach * mach).sqrt(), reference_area_m2))
    }

    /// [`FinGeometry::single_fin_slope`] at the Prandtl–Glauert factor `beta`, unchecked.
    pub(crate) fn slope_at(&self, beta: f64, reference_area_m2: f64) -> f64 {
        let s2 = self.span_m * self.span_m;
        let f = beta * s2 / (self.area_m2 * self.midchord_sweep_rad.cos());
        TAU * s2 / reference_area_m2 / (1.0 + (1.0 + f * f).sqrt())
    }
}

/// The span-averaged leading-edge angle of an elliptical fin whose root chord is `2k` spans.
///
/// The leading edge `x = (c_r/2)(1 − √(1 − η²))`, `η = y/s`, has the angle
/// `Γ(η) = atan(k η/√(1 − η²))`. Integrating by parts with `η = sin t`,
/// `∫₀¹ Γ dη = π/2 − ∫₀¹ k du/(k² + (1 − k²)u²)`, which is `π/2 − acos(k)/√(1 − k²)` for `k < 1`,
/// `π/2 − 1` at `k = 1`, and `π/2 − acosh(k)/√(k² − 1)` for `k > 1`.
fn elliptical_leading_edge_sweep(k: f64) -> f64 {
    let d = 1.0 - k * k;
    let integral = if d.abs() < 1e-6 {
        // Series about k = 1 in d = 1 − k², the same on both sides: 1 + d/6 + 3d²/40 + ….
        1.0 + d / 6.0 + 0.075 * d * d
    } else if d > 0.0 {
        k.acos() / d.sqrt()
    } else {
        k.acosh() / (-d).sqrt()
    };
    std::f64::consts::FRAC_PI_2 - integral
}

/// Fin–body interference factor `K_T(B) = 1 + r_t/(s + r_t)` (Barrowman 1966 eq. 77; Niskanen 2009
/// eq. 3.56), with `s` the span from the body surface and `r_t` the body radius at the fins.
///
/// # Errors
///
/// [`AeroError::Domain`] for a non-positive span or a negative body radius.
pub fn interference_factor(span_m: f64, body_radius_m: f64) -> Result<f64, AeroError> {
    check_dimension("fin span", span_m, false)?;
    check_dimension("body radius at the fins", body_radius_m, true)?;
    Ok(1.0 + body_radius_m / (span_m + body_radius_m))
}

/// Fin–fin interference factor for `count` fins in one set (OpenRocket technical documentation
/// 13.05 eq. 3.54, from MIL-HDBK-762(MI) p. 5-24): 1 up to four fins, then 0.948, 0.913, 0.854
/// and 0.810.
///
/// # Errors
///
/// [`AeroError::Domain`] for no fins or more than eight: the documentation's 0.750 for more than
/// eight fins has no data behind it.
pub fn fin_count_factor(count: u32) -> Result<f64, AeroError> {
    match count {
        1..=4 => Ok(1.0),
        5 => Ok(0.948),
        6 => Ok(0.913),
        7 => Ok(0.854),
        8 => Ok(0.810),
        _ => Err(AeroError::Domain {
            what: "fin count (1 to 8 have a normal-force model)",
            value: f64::from(count),
        }),
    }
}

/// `Σ sin² Λ_k` over `count` evenly spaced fins, where `Λ_k` is the angle from the lateral airflow
/// to fin `k` (Niskanen 2009 eq. 3.51–3.53). The first fin is at `base_angle_rad` and the airflow at
/// `flow_roll_rad`, both from `x_B` toward `y_B`. Three or more fins give exactly `N/2` at any roll.
pub fn roll_sum(count: u32, base_angle_rad: f64, flow_roll_rad: f64) -> f64 {
    if count >= 3 {
        return 0.5 * f64::from(count);
    }
    (0..count)
        .map(|k| {
            let lambda = base_angle_rad + TAU * f64::from(k) / f64::from(count) - flow_roll_rad;
            lambda.sin().powi(2)
        })
        .sum()
}

/// `Σ sin(φ − θ_k) cos(φ − θ_k)` over `count` evenly spaced fins at `θ_k` in a lateral airflow at
/// `φ`: the side-force share, perpendicular to the flow's plane. Each fin sees the local angle
/// `α sin Λ_k` (Niskanen 2009 eq. 3.50) and pushes along its own normal; eq. 3.51 keeps the part of
/// that push in the flow's plane, `sin² Λ_k`, and this is the part across it. The sum vanishes for
/// three or more fins; for one or two it doesn't: two fins at 45° to the flow push along their
/// common normal, `√2` times their in-plane share. Derived here from eq. 3.50; Niskanen drops it.
pub fn side_sum(count: u32, base_angle_rad: f64, flow_roll_rad: f64) -> f64 {
    if count >= 3 {
        return 0.0;
    }
    (0..count)
        .map(|k| {
            let lambda = base_angle_rad + TAU * f64::from(k) / f64::from(count) - flow_roll_rad;
            -lambda.sin() * lambda.cos()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::f64::consts::FRAC_1_SQRT_2;

    use super::*;

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        let err = if want == 0.0 {
            got.abs()
        } else {
            ((got - want) / want).abs()
        };
        assert!(
            err <= rel,
            "{what}: got {got}, want {want}, rel err {err:e}"
        );
    }

    fn trapezoid(c_r: f64, c_t: f64, s: f64, x_t: f64) -> FinPlanform {
        FinPlanform::Trapezoidal {
            root_chord_m: c_r,
            tip_chord_m: c_t,
            span_m: s,
            sweep_m: x_t,
        }
    }

    /// Barrowman 1966 eq. 57, for four fins with `N/2` applied by hand: `8 (s/d)² / (1 + √(1 +
    /// (2ℓ/(c_r + c_t))²))` per fin, with `ℓ` the mid-chord line's length.
    fn eq57(c_r: f64, c_t: f64, s: f64, x_t: f64, d: f64) -> f64 {
        let ell = (s * s + (x_t + 0.5 * c_t - 0.5 * c_r).powi(2)).sqrt();
        8.0 * (s / d).powi(2) / (1.0 + (1.0 + (2.0 * ell / (c_r + c_t)).powi(2)).sqrt())
    }

    /// Loft lesson L8: fin-set slopes don't grow linearly past four fins. Six and eight fins give
    /// 1.37 and 1.62 times four (MIL-HDBK-762(MI) p. 5-24); five and seven sit between.
    #[test]
    fn six_fin_cna_applies_fin_count_factor() {
        let set = |n: u32| roll_sum(n, 0.3, 1.1) * fin_count_factor(n).unwrap();
        close(set(6) / set(4), 1.37, 5e-4, "six over four");
        close(set(8) / set(4), 1.62, 1e-12, "eight over four");
        close(set(5), 2.37, 1e-12, "five");
        // Niskanen's thesis prints three figures: 3.5 × 0.854 = 2.989.
        close(set(7), 2.99, 5e-4, "seven");
        assert!(set(6) < 1.5 * set(4));
        for n in [1, 2, 3, 4] {
            assert_eq!(fin_count_factor(n).unwrap(), 1.0);
        }
        assert!(fin_count_factor(0).is_err());
        assert!(fin_count_factor(9).is_err());
    }

    /// Loft lesson L10: an elliptical fin's mid-chord line is straight along the root's middle, so
    /// `Γ_c = 0` and its slope uses its own area, not an equal-area trapezoid's sweep.
    #[test]
    fn elliptical_fin_cna_uses_zero_midchord_sweep() {
        let (c_r, s, d) = (0.1, 0.06, 0.05);
        let a_ref = 0.25 * PI * d * d;
        let ellipse = FinGeometry::from_planform(&FinPlanform::Elliptical {
            root_chord_m: c_r,
            span_m: s,
        })
        .unwrap();
        assert_eq!(ellipse.midchord_sweep_rad, 0.0);
        let area = 0.25 * PI * c_r * s;
        let f = s * s / area;
        let want = TAU * s * s / a_ref / (1.0 + (1.0 + f * f).sqrt());
        close(
            ellipse.single_fin_slope(a_ref, 0.0).unwrap(),
            want,
            1e-15,
            "slope",
        );

        // Loft's equal-area trapezoid (tip 2A/s − c_r, sweep 0) moves the mid-chord and is 1.3%
        // low.
        let loft =
            FinGeometry::from_planform(&trapezoid(c_r, 2.0 * area / s - c_r, s, 0.0)).unwrap();
        let ratio = loft.single_fin_slope(a_ref, 0.0).unwrap() / want;
        assert!((0.985..0.99).contains(&ratio), "{ratio}");

        // A 2000-gon ellipse converges on the same sweep, slope and CP.
        let n = 2000;
        let points: Vec<[f64; 2]> = (0..=n)
            .map(|i| {
                let t = PI * f64::from(i) / f64::from(n);
                let x = 0.5 * c_r * (1.0 - t.cos());
                [x, s * t.sin()]
            })
            .map(|[x, y]| [x, if y.abs() < 1e-15 { 0.0 } else { y }])
            .collect();
        let polygon =
            FinGeometry::from_planform(&FinPlanform::Freeform { points_m: points }).unwrap();
        assert!(
            polygon.midchord_sweep_rad.abs() < 1e-12,
            "{}",
            polygon.midchord_sweep_rad
        );
        close(
            polygon.single_fin_slope(a_ref, 0.0).unwrap(),
            want,
            1e-5,
            "polygon slope",
        );
        close(
            polygon.centre_of_pressure_m(),
            ellipse.centre_of_pressure_m(),
            1e-5,
            "polygon CP",
        );
    }

    /// Trapezoids match Barrowman 1966's closed forms (eq. 57 slope, eq. 76a CP), and the same
    /// outline as a freeform polygon matches them to round-off.
    #[test]
    fn trapezoid_matches_barrowman_and_its_polygon() {
        let (c_r, c_t, s, x_t, d) = (3.0, 2.0, 1.5, 1.5, 0.976);
        let a_ref = 0.25 * PI * d * d;
        let fin = FinGeometry::from_planform(&trapezoid(c_r, c_t, s, x_t)).unwrap();
        close(
            fin.single_fin_slope(a_ref, 0.0).unwrap(),
            eq57(c_r, c_t, s, x_t, d),
            1e-15,
            "eq. 57",
        );
        let eq76a = x_t / 3.0 * (c_r + 2.0 * c_t) / (c_r + c_t)
            + (c_r + c_t - c_r * c_t / (c_r + c_t)) / 6.0;
        close(fin.centre_of_pressure_m(), eq76a, 1e-15, "eq. 76a");
        // Testbed II's hand value, X_F = 1.333 in (NARAM-8 p. 43).
        close(fin.centre_of_pressure_m(), 1.333, 3e-4, "Testbed II X_F");

        let polygon = FinGeometry::from_planform(&FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [x_t, s], [x_t + c_t, s], [c_r, 0.0]],
        })
        .unwrap();
        for (got, want, what) in [
            (polygon.area_m2, fin.area_m2, "area"),
            (polygon.midchord_sweep_rad, fin.midchord_sweep_rad, "sweep"),
            (polygon.mac_length_m, fin.mac_length_m, "MAC"),
            (polygon.mac_leading_edge_m, fin.mac_leading_edge_m, "MAC LE"),
            (polygon.mac_span_m, fin.mac_span_m, "MAC span"),
        ] {
            close(got, want, 1e-13, what);
        }
        // A pointed (delta) fin and a rectangle.
        let delta = FinGeometry::from_planform(&trapezoid(0.2, 0.0, 0.1, 0.2)).unwrap();
        close(
            delta.centre_of_pressure_m(),
            0.2 / 3.0 * 1.0 + 0.2 / 6.0,
            1e-15,
            "delta CP",
        );
        let rectangle = FinGeometry::from_planform(&trapezoid(0.1, 0.1, 0.05, 0.0)).unwrap();
        assert_eq!(rectangle.midchord_sweep_rad, 0.0);
        close(
            rectangle.centre_of_pressure_m(),
            0.025,
            1e-15,
            "rectangle CP",
        );
    }

    /// A jagged freeform fin: the notch counts toward the CP's chord but not the slope's area
    /// (Niskanen 2009 pp. 27–28).
    #[test]
    fn jagged_fin_fills_its_gap_for_the_cp_only() {
        // A 0.1 × 0.1 square with a 0.04-wide, 0.05-deep slot cut into its tip.
        let points = vec![
            [0.0, 0.0],
            [0.0, 0.1],
            [0.03, 0.1],
            [0.03, 0.05],
            [0.07, 0.05],
            [0.07, 0.1],
            [0.1, 0.1],
            [0.1, 0.0],
        ];
        let fin = FinGeometry::from_planform(&FinPlanform::Freeform { points_m: points }).unwrap();
        close(
            fin.area_m2,
            0.01 - 0.04 * 0.05,
            1e-14,
            "area without the slot",
        );
        close(fin.mac_length_m, 0.1, 1e-14, "filled chord");
        close(fin.mac_leading_edge_m, 0.0, 1e-14, "LE");
        close(fin.mac_span_m, 0.05, 1e-14, "filled centroid span");
        assert_eq!(fin.midchord_sweep_rad, 0.0);
    }

    /// Prandtl–Glauert: the slope rises with Mach, equals Barrowman 1967 eq. 3-6 written in the
    /// aspect ratio `AR = 2s²/A_fin`, and tends to `π s²/A_ref` as `M → 1`.
    #[test]
    fn fin_slope_follows_prandtl_glauert() {
        let fin = FinGeometry::from_planform(&trapezoid(0.12, 0.06, 0.08, 0.05)).unwrap();
        let a_ref = 0.25 * PI * 0.1 * 0.1;
        let mut last = 0.0;
        for mach in [0.0, 0.2, 0.5, 0.8, 0.95, 0.999] {
            let slope = fin.single_fin_slope(a_ref, mach).unwrap();
            assert!(slope > last, "{mach}");
            last = slope;
            let beta = (1.0 - mach * mach).sqrt();
            let ar = 2.0 * fin.span_m * fin.span_m / fin.area_m2;
            let eq36 = TAU * ar * (fin.area_m2 / a_ref)
                / (2.0 + (4.0 + (beta * ar / fin.midchord_sweep_rad.cos()).powi(2)).sqrt());
            close(slope, eq36, 1e-14, "eq. 3-6");
        }
        let near_one = fin.single_fin_slope(a_ref, 1.0 - 1e-12).unwrap();
        close(
            near_one,
            PI * fin.span_m * fin.span_m / a_ref,
            1e-5,
            "M → 1",
        );
        assert!(fin.single_fin_slope(a_ref, 1.0).is_err());
        assert!(fin.single_fin_slope(a_ref, -0.1).is_err());
        assert!(fin.single_fin_slope(a_ref, f64::NAN).is_err());
    }

    /// Interference at its limits (no body: 1; a vanishing span: 2), and the roll sum: `N/2` for
    /// three or more fins at any roll, `sin²` terms for one and two.
    #[test]
    fn interference_and_roll_limits() {
        assert_eq!(interference_factor(0.1, 0.0).unwrap(), 1.0);
        close(
            interference_factor(1e-12, 0.05).unwrap(),
            2.0,
            1e-10,
            "tiny span",
        );
        close(
            interference_factor(1.5, 0.368).unwrap(),
            1.197,
            1e-3,
            "Testbed II K",
        );
        assert!(interference_factor(0.0, 0.05).is_err());
        assert!(interference_factor(0.1, -0.01).is_err());

        for n in 3..=8 {
            for roll in [0.0, 0.1, 0.7, 2.0, -3.0] {
                let direct: f64 = (0..n)
                    .map(|k| {
                        (0.2 + TAU * f64::from(k) / f64::from(n) - roll)
                            .sin()
                            .powi(2)
                    })
                    .sum();
                close(roll_sum(n, 0.2, roll), direct, 1e-14, "direct sum");
            }
        }
        for n in 3..=8 {
            assert_eq!(side_sum(n, 0.2, 0.9), 0.0);
        }
        // Two fins along x_B, flow at 45°: in-plane and side shares of 1 each, a push along y_B.
        close(side_sum(2, 0.0, PI / 4.0), 1.0, 1e-15, "two fins, side");
        close(roll_sum(2, 0.0, PI / 4.0), 1.0, 1e-15, "two fins, in plane");
        assert!(side_sum(2, 0.0, 0.0).abs() < 1e-15 && side_sum(2, 0.0, PI / 2.0).abs() < 1e-15);
        assert_eq!(roll_sum(1, 0.0, 0.0), 0.0);
        close(
            roll_sum(1, 0.0, PI / 2.0),
            1.0,
            1e-15,
            "one fin across the flow",
        );
        close(
            roll_sum(2, 0.0, PI / 2.0),
            2.0,
            1e-15,
            "two fins across the flow",
        );
        close(roll_sum(2, 0.0, PI / 4.0), 1.0, 1e-15, "two fins at 45°");
    }

    /// A tip whose two vertices are a few rounding steps apart (3.5 in written in metres and
    /// converted from inches) is still one tip: the sliver band between them is skipped.
    #[test]
    fn nearly_level_tip_vertices_are_one_tip() {
        let tip = 3.5 * 0.0254;
        assert_ne!(tip, 0.0889);
        let nudged = FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [0.03, 0.0889], [0.06, tip], [0.1, 0.0]],
        };
        let level = FinPlanform::Freeform {
            points_m: vec![[0.0, 0.0], [0.03, 0.0889], [0.06, 0.0889], [0.1, 0.0]],
        };
        let (a, b) = (
            FinGeometry::from_planform(&nudged).unwrap(),
            FinGeometry::from_planform(&level).unwrap(),
        );
        close(a.area_m2, b.area_m2, 1e-12, "area");
        close(
            a.centre_of_pressure_m(),
            b.centre_of_pressure_m(),
            1e-12,
            "CP",
        );
        close(a.midchord_sweep_rad, b.midchord_sweep_rad, 1e-12, "sweep");
    }

    proptest::proptest! {
        /// Four-point outlines, with the tip vertices nudged by up to three rounding steps: the
        /// area and the area centroid's span match hpr-design's own quadrature of the planform.
        #[test]
        fn freeform_integrals_match_the_design_quadrature(
            x1 in -0.05f64..0.15,
            tip in 0.0f64..0.1,
            y1 in 0.01f64..0.2,
            y2_ratio in 0.5f64..1.5,
            root in 0.02f64..0.3,
            steps in -3i32..=3,
            level in proptest::bool::ANY,
        ) {
            let mut y2 = if level { y1 } else { y1 * y2_ratio };
            for _ in 0..steps.unsigned_abs() {
                y2 = if steps > 0 { y2.next_up() } else { y2.next_down() };
            }
            let planform = FinPlanform::Freeform {
                points_m: vec![[0.0, 0.0], [x1, y1], [x1 + tip, y2], [root, 0.0]],
            };
            proptest::prop_assume!(planform.validate().is_ok());
            let reference = planform.geometry().unwrap();
            let fin = FinGeometry::from_planform(&planform).unwrap();
            proptest::prop_assert!((fin.area_m2 / reference.area_m2 - 1.0).abs() < 1e-9);
            proptest::prop_assert!(
                (fin.mac_span_m / reference.centroid_span_m - 1.0).abs() < 1e-9
            );
        }
    }

    /// The roll and side sums rebuild the direct per-fin vector sum: fin `k` at `θ_k` pushes along
    /// its normal `n_k = (−sin θ_k, cos θ_k)` in proportion to the air crossing it, `sin(φ − θ_k)`,
    /// and `Σ sin(φ − θ_k) n_k = roll_sum · ŵ + side_sum · (z_B × ŵ)` with `ŵ = (cos φ, sin φ)`
    /// (`frames.md`, force directions). Two fins along `x_B` in a 45° flow push along `+y_B`.
    #[test]
    fn roll_and_side_sums_rebuild_the_per_fin_vector() {
        for n in 1..=8u32 {
            for base in [0.0, 0.7, 1.0] {
                for roll in [0.0, 0.4, PI / 4.0, 2.5, -1.2] {
                    let direct = (0..n).fold([0.0, 0.0], |acc, k| {
                        let theta = base + TAU * f64::from(k) / f64::from(n);
                        let push = (roll - theta).sin();
                        [acc[0] - push * theta.sin(), acc[1] + push * theta.cos()]
                    });
                    let (c_n, c_y) = (roll_sum(n, base, roll), side_sum(n, base, roll));
                    let rebuilt = [
                        c_n * roll.cos() - c_y * roll.sin(),
                        c_n * roll.sin() + c_y * roll.cos(),
                    ];
                    for (got, want) in rebuilt.iter().zip(direct) {
                        assert!(
                            (got - want).abs() < 1e-14,
                            "{n} {base} {roll}: {rebuilt:?} {direct:?}"
                        );
                    }
                }
            }
        }
        let (c_n, c_y) = (roll_sum(2, 0.0, PI / 4.0), side_sum(2, 0.0, PI / 4.0));
        let push = [
            c_n * FRAC_1_SQRT_2 - c_y * FRAC_1_SQRT_2,
            c_n * FRAC_1_SQRT_2 + c_y * FRAC_1_SQRT_2,
        ];
        assert!(
            push[0].abs() < 1e-15 && (push[1] - 2.0 * FRAC_1_SQRT_2).abs() < 1e-15,
            "{push:?}"
        );
    }
}
