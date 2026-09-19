//! A blunt or vertical nose tip faster than sound: modified Newtonian pressures on the cap, handed
//! over to the second-order shock-expansion method ([`crate::shock_expansion`]) where the surface
//! slope falls to the steepest wedge an attached shock can turn. After C. M. Jackson Jr.,
//! W. C. Sawyer and R. S. Smith, *A Method for Determining Surface Pressures on Blunt Bodies of
//! Revolution at Small Angles of Attack in Supersonic Flow*, NASA TN D-4865 (1968) ([J68]).
//!
//! **Flown faster than sound** since the milestone
//! [M1.8e7](https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8e7), for noses whose
//! tip is vertical (power-series noses with `n < 1`, Haack series, elliptical noses) through
//! [`crate::model::SupersonicBody`]. The guide's
//! [Blunt tips](https://nrdptel.github.io/hpr-sim/physics/aero.html#blunt-tips) explains it and
//! how it was checked.
//!
//! ```
//! use hpr_aero::blunt_tip::{handover_angle_rad, newtonian_loading, pitot_pressure_ratio};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Behind a normal shock at Mach 2 the pitot pressure is 5.640 times the free stream's
//! // (NACA Report 1135, Table II).
//! assert!((pitot_pressure_ratio(2.0)? - 5.6404).abs() < 1e-4);
//! // The cap hands over where its slope falls to the steepest wedge an attached shock turns
//! // at Mach 1.5, 12.1°.
//! assert!((handover_angle_rad(1.5)?.to_degrees() - 12.11).abs() < 0.01);
//! // Where the cap's slope is 45° its loading is C_p,max/2, 0.829 at Mach 2.
//! let loading = newtonian_loading(2.0, 45f64.to_radians())?;
//! assert!((loading - 0.8286).abs() < 1e-4);
//! # Ok(())
//! # }
//! ```
//!
//! **The cap.** Near a blunt tip the shock stands off the body and the flow behind it is subsonic,
//! where the shock-expansion method, which marches supersonic flow from a pointed tip, can't
//! start. The report's cap is modified Newtonian (eq. 1, p. 5):
//!
//! `p_s/p₀ = (p_t2/p₀ − 1) sin²δ + 1`,
//!
//! with `δ` the surface's slope to the wind and `p_t2` the pitot pressure behind a normal shock
//! (the Rayleigh pitot formula, NACA Report 1135, 1953, eq. 100, p. 619):
//!
//! `p_t2/p₀ = [(γ + 1)M²/2]^(γ/(γ − 1)) · [(γ + 1)/(2γM² − (γ − 1))]^(1/(γ − 1))`.
//!
//! In coefficient form `C_p = C_p,max sin²δ`, `C_p,max = (p_t2/p₀ − 1)/(γM²/2)`.
//!
//! **The handover.** The shock-expansion method starts "at the point where the surface slope is
//! the same as that required for shock attachment to a two-dimensional wedge at the free-stream
//! Mach number", chosen "simply because it gave the best agreement with the available data in the
//! low supersonic-speed range" (p. 5): the wedge's largest deflection `δ_max`, from the shock angle
//! of NACA Report 1135 eq. 168 (p. 624),
//!
//! `sin²θ = [(γ + 1)M²/4 − 1 + √((γ + 1)((γ + 1)M⁴/16 + (γ − 1)M²/2 + 1))]/(γM²)`,
//!
//! turned into a deflection by eq. 138 (p. 621), `tan δ = 2 cot θ (M² sin²θ − 1)/(2 + M²(γ + 1 −
//! 2 sin²θ))`. It is 12.1° at Mach 1.5 and 22.97° at Mach 2. hpr hands over at the lesser of
//! `δ_max` and 24°, the steepest tangent cone of TN 3527's Fig. 2 ([`MAX_HANDOVER_RAD`]): the
//! method needs that cone's normal-force slope (the report's own cone tables stop at 30°, p. 31,
//! short of its handover from Mach 2.96 up). From about Mach 2.1 up the cap therefore reaches
//! further aft than the report's.
//!
//! **The flow behind it: hpr's choice, not the report's.** hpr starts TN 3527's march at the
//! handover as the method starts at a pointed vertex: with the flow on the cone tangent to the
//! body there (Taylor–Maccoll), that cone's loading `tan δ (dC_N/dα)_tc`, and no pressure gradient
//! (TN 3527 sketch (a), p. 6). The report starts it from the Newtonian pressure and Mach number
//! instead (eq. 2 and p. 5). Read that way at `α → 0`, the march on the committed Arcas Robin nose
//! reduces elements from Mach 2.96 (issue #81), so its answer changes as elements are added, and
//! fails from Mach 3.96, where the Newtonian pressure at the handover lies below the tangent
//! cone's; the tangent cone's start holds to Mach 5 and converges. [`crate::shock_expansion::HandoverStart`] keeps the report's start to compare,
//! and the decision record on it, [ADR-038][adr-038], gives both readings' numbers.
//!
//! **The loading at `α → 0`.** On the cap, TN 3527's loading form ([`crate::shock_expansion`]),
//! whose `C_Nα = (2π/A_ref) ∫ Λ r dx` makes `Λ` half the windward meridian's `∂C_p/∂α`, takes
//! the wind's slope `δ + α cos φ` in `C_p = C_p,max sin²δ`: `Λ = C_p,max sin δ cos δ`
//! ([`newtonian_loading`]). A hemisphere then carries `C_Nα = C_p,max/2`, its Newtonian drag turned
//! into the body's axes, as it must. Behind the handover the loading is the method's.
//!
//! **What it covers.** The report checked spherical caps only: a sphere-cone (a 0.175-diameter
//! nose radius on an 11.5° cone) and a sphere on a flared body, from Mach 1.50 to 4.63 and up to
//! 12°, calling its results "adequate engineering estimates … except where flow separation or
//! detached secondary shock waves are present" (p. 13). A power-series, Haack or elliptical nose
//! has no sphere at its tip; applying the slope rule to it is an extrapolation, stated. The
//! wedge's deflection, the pitot pressure and the Newtonian pressure are exact for a perfect gas
//! with `γ = 1.4`; the loading is only as good as Newtonian theory on the cap.
//!
//! [J68]: https://ntrs.nasa.gov/citations/19690000884
//! [adr-038]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-038-blunt-and-vertical-nose-tips-faster-than-sound-by-a-newtonian-cap-the-method-started-from-the-tangent-cone-2026-09-19

use crate::afterbody::GAMMA;
use crate::error::AeroError;

/// The steepest slope the cap hands over at: TN 3527 Fig. 2's steepest tangent cone, 24°, whose
/// normal-force slope the shock-expansion method's loading reads
/// ([`crate::shock_expansion::cone_normal_force_slope`]).
pub const MAX_HANDOVER_RAD: f64 = 24.0 * std::f64::consts::PI / 180.0;

/// The Mach number must be finite and above 1: the cap stands behind a normal shock.
fn check_mach(mach: f64) -> Result<(), AeroError> {
    if mach.is_finite() && mach > 1.0 {
        Ok(())
    } else {
        Err(AeroError::Domain {
            what: "Mach number of a blunt tip's cap",
            value: mach,
        })
    }
}

/// A slope to the wind within `[0, π/2]`.
fn check_slope(slope_rad: f64) -> Result<(), AeroError> {
    if slope_rad.is_finite() && (0.0..=std::f64::consts::FRAC_PI_2).contains(&slope_rad) {
        Ok(())
    } else {
        Err(AeroError::Domain {
            what: "surface slope on a blunt tip's cap",
            value: slope_rad,
        })
    }
}

/// The pitot pressure behind a normal shock over the free stream's, `p_t2/p₀`, at Mach `mach`
/// (the Rayleigh pitot formula, NACA Report 1135 eq. 100, p. 619; TN D-4865 eq. 1).
///
/// # Errors
///
/// [`AeroError::Domain`] for a Mach number that isn't finite and above 1.
pub fn pitot_pressure_ratio(mach: f64) -> Result<f64, AeroError> {
    check_mach(mach)?;
    Ok(pitot(mach))
}

fn pitot(mach: f64) -> f64 {
    let m2 = mach * mach;
    let g = GAMMA;
    (0.5 * (g + 1.0) * m2).powf(g / (g - 1.0))
        * ((g + 1.0) / (2.0 * g * m2 - (g - 1.0))).powf(1.0 / (g - 1.0))
}

/// The largest angle a two-dimensional wedge can turn the flow at Mach `mach` behind an attached
/// shock, rad: the shock angle of NACA Report 1135 eq. 168 (p. 624) put into eq. 138 (p. 621).
///
/// # Errors
///
/// [`AeroError::Domain`] for a Mach number that isn't finite and above 1.
pub fn wedge_detachment_angle_rad(mach: f64) -> Result<f64, AeroError> {
    check_mach(mach)?;
    let g = GAMMA;
    let m2 = mach * mach;
    let root = ((g + 1.0) * ((g + 1.0) * m2 * m2 / 16.0 + 0.5 * (g - 1.0) * m2 + 1.0)).sqrt();
    let sin2 = ((0.25 * (g + 1.0) * m2 - 1.0 + root) / (g * m2)).min(1.0);
    let cot = ((1.0 - sin2) / sin2).sqrt();
    let tan = 2.0 * cot * (m2 * sin2 - 1.0) / (2.0 + m2 * (g + 1.0 - 2.0 * sin2));
    Ok(tan.atan())
}

/// The slope at which the cap hands over to the shock-expansion method at Mach `mach`, rad: the
/// lesser of the wedge's largest deflection ([`wedge_detachment_angle_rad`], TN D-4865 p. 5) and
/// [`MAX_HANDOVER_RAD`].
///
/// # Errors
///
/// As [`wedge_detachment_angle_rad`].
pub fn handover_angle_rad(mach: f64) -> Result<f64, AeroError> {
    Ok(wedge_detachment_angle_rad(mach)?.min(MAX_HANDOVER_RAD))
}

/// Modified Newtonian `C_p,max = (p_t2/p₀ − 1)/(γM²/2)` at Mach `mach`.
///
/// # Errors
///
/// As [`pitot_pressure_ratio`].
pub fn newtonian_pressure_coefficient_max(mach: f64) -> Result<f64, AeroError> {
    Ok((pitot_pressure_ratio(mach)? - 1.0) / (0.5 * GAMMA * mach * mach))
}

/// The cap's surface pressure over the free stream's where its slope to the wind is `slope_rad`
/// (TN D-4865 eq. 1): `(p_t2/p₀ − 1) sin²δ + 1`.
///
/// # Errors
///
/// [`AeroError::Domain`] for a Mach number that isn't finite and above 1, or a slope outside
/// `[0, π/2]`.
pub fn newtonian_pressure_ratio(mach: f64, slope_rad: f64) -> Result<f64, AeroError> {
    check_slope(slope_rad)?;
    let sin = slope_rad.sin();
    Ok((pitot_pressure_ratio(mach)? - 1.0) * sin * sin + 1.0)
}

/// The cap's surface Mach number where its pressure is `pressure_ratio` times the free stream's,
/// isentropic from the pitot pressure (TN D-4865 eq. 2); zero at the stagnation point. The
/// report's march starts from it ([`crate::shock_expansion::HandoverStart::Newtonian`]).
///
/// # Errors
///
/// [`AeroError::Domain`] for a Mach number that isn't finite and above 1, or a pressure that
/// isn't positive and at most the pitot pressure.
pub fn newtonian_surface_mach(mach: f64, pressure_ratio: f64) -> Result<f64, AeroError> {
    let pitot = pitot_pressure_ratio(mach)?;
    if !(pressure_ratio.is_finite() && pressure_ratio > 0.0 && pressure_ratio <= pitot) {
        return Err(AeroError::Domain {
            what: "surface pressure ratio on a blunt tip's cap",
            value: pressure_ratio,
        });
    }
    let g = GAMMA;
    let m2 = 2.0 / (g - 1.0) * ((pressure_ratio / pitot).powf(-(g - 1.0) / g) - 1.0);
    Ok(m2.max(0.0).sqrt())
}

/// The cap's loading at `α → 0` where its slope to the axis is `slope_rad`, `Λ = C_p,max sin δ
/// cos δ`: half the windward `∂C_p/∂α` of `C_p = C_p,max sin²(δ + α cos φ)`, in the units of
/// TN 3527's loading ([`crate::shock_expansion`]).
///
/// # Errors
///
/// As [`newtonian_pressure_ratio`].
pub fn newtonian_loading(mach: f64, slope_rad: f64) -> Result<f64, AeroError> {
    check_slope(slope_rad)?;
    Ok(newtonian_pressure_coefficient_max(mach)? * slope_rad.sin() * slope_rad.cos())
}

/// [`newtonian_loading`] from the slope `dr/dx` itself, which may be infinite (a vertical tip):
/// `sin δ cos δ = 1/(t + 1/t)` with `t = dr/dx`, zero at both ends. `c_p_max` is
/// [`newtonian_pressure_coefficient_max`].
pub(crate) fn newtonian_loading_at_slope(c_p_max: f64, slope: f64) -> f64 {
    let t = slope.abs();
    if t == 0.0 || t.is_infinite() {
        0.0
    } else {
        c_p_max / (t + 1.0 / t)
    }
}

/// The loading just behind a handover that holds still in the wind, `Λ = λ/(γM²)`, with
/// `λ = 2γp/sin 2μ` at the handover's pressure `pressure_ratio` (times the free stream's) and
/// surface Mach number `surface_mach`, and `M` the free stream's: the Prandtl–Meyer flow's
/// `∂p/∂ν` over the free stream's `γM²/2`, halved as TN 3527's loading is. It is TN D-4865's
/// equivalent bodies (eqs. 4a and 4b, the body turned about the sphere's centre) read at `α → 0`,
/// hpr's reading ([`crate::shock_expansion::HandoverStart::Newtonian`]).
///
/// # Errors
///
/// [`AeroError::Domain`] for a free-stream or surface Mach number that isn't finite and above 1,
/// or a pressure that isn't finite and positive.
pub fn handover_loading(
    mach: f64,
    pressure_ratio: f64,
    surface_mach: f64,
) -> Result<f64, AeroError> {
    check_mach(mach)?;
    if !(surface_mach.is_finite() && surface_mach > 1.0) {
        return Err(AeroError::Domain {
            what: "surface Mach number at a blunt tip's handover",
            value: surface_mach,
        });
    }
    if !(pressure_ratio.is_finite() && pressure_ratio > 0.0) {
        return Err(AeroError::Domain {
            what: "surface pressure ratio at a blunt tip's handover",
            value: pressure_ratio,
        });
    }
    let m2 = surface_mach * surface_mach;
    let lambda = GAMMA * pressure_ratio * m2 / (m2 - 1.0).sqrt();
    Ok(lambda / (GAMMA * mach * mach))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn close(got: f64, want: f64, tol: f64, what: &str) {
        assert!(
            (got - want).abs() <= tol,
            "{what}: got {got}, want {want} ± {tol}"
        );
    }

    /// NACA Report 1135, Table II (normal shock, γ = 1.4): `p_t2/p₁` at M = 1.5, 2, 3 and 5.
    #[test]
    fn the_pitot_pressure_is_rayleighs() {
        close(pitot_pressure_ratio(1.5).unwrap(), 3.413, 5e-4, "M 1.5");
        close(pitot_pressure_ratio(2.0).unwrap(), 5.640, 5e-4, "M 2");
        close(pitot_pressure_ratio(3.0).unwrap(), 12.06, 5e-3, "M 3");
        close(pitot_pressure_ratio(5.0).unwrap(), 32.65, 5e-3, "M 5");
        // At Mach 1 the shock vanishes: the pitot pressure is the isentropic total, 1.8929.
        close(pitot(1.0), 1.892_929, 1e-6, "M 1");
    }

    /// NACA Report 1135, chart 2: the largest wedge deflection is 12.1° at Mach 1.5, 22.97° at
    /// Mach 2, 34.07° at Mach 3 and 41.1° at Mach 5, rising to 45.6° as Mach → ∞.
    #[test]
    fn the_wedge_detaches_where_naca_1135_says() {
        let deg = |m: f64| wedge_detachment_angle_rad(m).unwrap().to_degrees();
        close(deg(1.5), 12.11, 0.01, "M 1.5");
        close(deg(2.0), 22.97, 0.01, "M 2");
        close(deg(3.0), 34.07, 0.01, "M 3");
        close(deg(5.0), 41.12, 0.02, "M 5");
        close(deg(1e6), 45.58, 0.01, "M → ∞");
        // Near Mach 1 it falls to zero like (M² − 1)^(3/2).
        assert!(deg(1.0 + 1e-9) < 1e-9);
        // It rises with Mach.
        let mut last = 0.0;
        for i in 1..400 {
            let d = deg(1.0 + 0.02 * f64::from(i));
            assert!(d > last);
            last = d;
        }
    }

    #[test]
    fn the_handover_is_capped_at_fig_2s_steepest_cone() {
        close(
            handover_angle_rad(1.5).unwrap(),
            wedge_detachment_angle_rad(1.5).unwrap(),
            0.0,
            "M 1.5",
        );
        assert_eq!(handover_angle_rad(3.0).unwrap(), MAX_HANDOVER_RAD);
    }

    #[test]
    fn newtonian_pressures() {
        // At the stagnation point the pitot pressure; at zero slope the free stream's.
        let pitot = pitot_pressure_ratio(2.0).unwrap();
        close(
            newtonian_pressure_ratio(2.0, FRAC_PI_2).unwrap(),
            pitot,
            1e-12,
            "p",
        );
        close(newtonian_pressure_ratio(2.0, 0.0).unwrap(), 1.0, 0.0, "p₀");
        // C_p,max at Mach 2: (5.6404 − 1)/2.8.
        close(
            newtonian_pressure_coefficient_max(2.0).unwrap(),
            (5.640_4 - 1.0) / 2.8,
            1e-4,
            "C_p,max",
        );
        assert!(matches!(
            newtonian_pressure_ratio(2.0, 2.0),
            Err(AeroError::Domain { .. })
        ));
        assert!(matches!(
            pitot_pressure_ratio(1.0),
            Err(AeroError::Domain { .. })
        ));
    }

    /// A hemisphere's Newtonian loading integrates to `C_Nα = C_p,max/2` on its base: the drag
    /// `C_p,max/2` of a Newtonian hemisphere turned into body axes (the force on a sphere passes
    /// through its centre, so it lies along the wind).
    #[test]
    fn a_hemisphere_carries_its_drag_turned() {
        let mach = 3.0;
        let c_p_max = newtonian_pressure_coefficient_max(mach).unwrap();
        // With r = sin θ, x = 1 − cos θ (unit radius) the slope is cot θ.
        let n = 20_000;
        let mut sum = 0.0;
        for i in 0..n {
            let theta = FRAC_PI_2 * (f64::from(i) + 0.5) / f64::from(n);
            let slope = 1.0 / theta.tan();
            let load = newtonian_loading_at_slope(c_p_max, slope);
            let r = theta.sin();
            // dx = sin θ dθ.
            sum += load * r * theta.sin() * FRAC_PI_2 / f64::from(n);
        }
        // C_Nα = (2π/π) ∫ Λ r dx on the unit base.
        close(2.0 * sum, 0.5 * c_p_max, 1e-8, "C_Nα");
        close(
            newtonian_loading(mach, 0.3).unwrap(),
            newtonian_loading_at_slope(c_p_max, 0.3f64.tan()),
            1e-15,
            "Λ",
        );
        assert_eq!(newtonian_loading_at_slope(c_p_max, f64::INFINITY), 0.0);
        assert_eq!(newtonian_loading_at_slope(c_p_max, 0.0), 0.0);
    }

    /// A Newtonian cone of half-angle δ carries `C_Nα = C_p,max cos²δ` on its base, the classical
    /// result; TN 3527's loading form gives it from `Λ = C_p,max sin δ cos δ`: `C_Nα = Λ/tan δ`.
    #[test]
    fn a_newtonian_cone_carries_c_p_max_cos_squared() {
        let delta = 0.2_f64;
        let load = newtonian_loading(2.5, delta).unwrap();
        let c_p_max = newtonian_pressure_coefficient_max(2.5).unwrap();
        close(
            load / delta.tan(),
            c_p_max * delta.cos().powi(2),
            1e-14,
            "cone",
        );
    }

    #[test]
    fn newtonian_surface_mach_numbers() {
        let pitot = pitot_pressure_ratio(2.0).unwrap();
        close(newtonian_surface_mach(2.0, pitot).unwrap(), 0.0, 1e-12, "M");
        // Expanded to the free stream's pressure through the normal shock's total, the surface
        // flow is slower than the free stream.
        let m = newtonian_surface_mach(2.0, 1.0).unwrap();
        assert!(m > 1.5 && m < 2.0, "{m}");
        assert!(matches!(
            newtonian_surface_mach(2.0, pitot * 1.01),
            Err(AeroError::Domain { .. })
        ));
    }

    /// Behind a handover fixed in the wind the loading is the Prandtl–Meyer flow's: a turn `dν`
    /// lowers the pressure by `λ dν`, checked by a finite difference of the expansion itself.
    #[test]
    fn the_handover_loading_is_the_expansions() {
        use crate::afterbody::{inverse_prandtl_meyer, prandtl_meyer};
        let mach = 2.3;
        let delta = handover_angle_rad(mach).unwrap();
        let p = newtonian_pressure_ratio(mach, delta).unwrap();
        let m = newtonian_surface_mach(mach, p).unwrap();
        let total = pitot_pressure_ratio(mach).unwrap();
        let static_at = |nu: f64| {
            let ms = inverse_prandtl_meyer(nu);
            total / (1.0 + 0.2 * ms * ms).powf(3.5)
        };
        let h = 1e-6;
        let nu = prandtl_meyer(m);
        // A turn smaller by ε raises the pressure by λε, C_p by 2λε/(γM²); Λ is half that.
        let dp = (static_at(nu - h) - static_at(nu + h)) / (2.0 * h);
        let want = dp / (GAMMA * mach * mach);
        close(
            handover_loading(mach, p, m).unwrap(),
            want,
            1e-6 * want,
            "Λ",
        );
        assert!(matches!(
            handover_loading(mach, p, 1.0),
            Err(AeroError::Domain { .. })
        ));
    }
}
