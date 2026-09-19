//! The Arcas Robin's supersonic body gap, source by source (M1.8e5): the measured body alone
//! (NASA TN D-4014, fins off) against hpr's body in a flight, and the size of each candidate
//! cause that hpr can compute, written by `cargo xtask aero` to
//! `validation/fixtures/aero/arcas-robin-gap.json`. `docs/research/body-supersonic-gap.md`
//! ranks the candidates from it.
//!
//! - **The fit's angles.** The measured slope (M1.8a's) is a straight line fitted through `C_N`
//!   plotted from about −5° to +4°. Crossflow lift grows as `α |α|`, so it steepens that line.
//!   The fixture fits the same points with a crossflow term, `C_N = a + b α + c α |α|`
//!   (Jorgensen, NASA TR R-474, eq. 2.12, where the term is `η C_dn (A_p/A_r) sin² α`), to separate
//!   the slope at `α → 0` from the curvature. Standard errors take each point's stated reading
//!   accuracy as independent.
//! - **hpr at the same angles.** hpr's bodies alone through the flight's path
//!   (`AeroModel::components`), the nose the fitted secant ogive and the boattail behind the
//!   cylinder, fitted through the same angles as the measurement. The difference from its slope
//!   at `α → 0` is hpr's body lift, `K (A_plan/A_ref) sin² α` with `K` = 1.1, at those angles.
//! - **The lip** behind the boattail: slender-body theory's share, `2 ΔA / A_ref`.
//! - **Issue #81:** how many of the nose's elements the method reduces at each Mach number.
//!
//! No targets: the fixture sizes causes, it doesn't pass or fail anything.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::BODY_LIFT_K;
use hpr_aero::shock_expansion::cone_normal_force_slope;
use serde_json::{Value, json};

use crate::aero_body::{
    ARCAS_NOSE_R_IN, ARCAS_NOSE_X_IN, INCH, arcas_body, arcas_model, arcas_nose_ratio,
    flight_bodies,
};
use crate::aero_mach::{WIND_TUNNEL, hpr_force, slope};

pub const FIXTURE: &str = "validation/fixtures/aero/arcas-robin-gap.json";

/// Each figure's reading accuracy for `C_N` at a plotted symbol, from the wind-tunnel fixture's
/// `reading`: TN D-4014 Fig. 5 (the short model) and Fig. 6 (the long).
pub const READING: [(&str, f64); 2] = [("arcas-robin-short", 0.01), ("arcas-robin-long", 0.013)];

/// Galejs's range for body lift's `K` ("1.0 to 1.5", *Wind Instability*, p. 1).
pub const GALEJS_K: [f64; 2] = [1.0, 1.5];

/// The measured points kept for the inner straight line: `|α|` under this, degrees: each plot's
/// middle five points.
pub const INNER_DEG: f64 = 3.0;

/// Jorgensen's `η` (NASA TR R-474, Fig. 4, printed p. 77), a finite cylinder's crossflow drag over
/// an infinite one's, read at each model's length over its diameter: 18.2 and 23.8.
pub const JORGENSEN_ETA: [(&str, f64); 2] =
    [("arcas-robin-short", 0.74), ("arcas-robin-long", 0.77)];

/// Jorgensen's `C_dn` for laminar separation below the critical Reynolds number, "C_dn = 1.2"
/// (NASA TR R-474, printed p. 15).
pub const JORGENSEN_C_DN: f64 = 1.2;

/// The Mach numbers of Butler, Sears and Pallas's supersonic tables (AFATL-TR-77-8, 1977,
/// Table 3(a) to (d), printed pp. 10 to 13).
pub const AFATL_MACHS: [f64; 4] = [1.5, 2.0, 3.0, 4.0];

/// Their `C_Nα` per degree at [`AFATL_MACHS`] of a 4-caliber tangent ogive on the 9-caliber
/// midsection and 1-caliber afterbody (M9 A17), sharp (nose N22) and truncated to a hemispherical
/// tip of 0.25 of its base radius (N23, 4.15 in long against 4.80).
pub const AFATL_PER_DEG: [[f64; 2]; 4] = [
    [0.048, 0.048],
    [0.053, 0.053],
    [0.060, 0.059],
    [0.063, 0.057],
];

/// N23's tip radius over its base radius (AFATL-TR-77-8's nose table).
pub const AFATL_TIP_RATIO: f64 = 0.25;

/// The Arcas Robin's tip radius, in (TN D-4014 Fig. 1(a); the wind-tunnel fixture's geometry).
pub const ARCAS_TIP_RADIUS_IN: f64 = 0.062;

/// The exponents `n` that scale AFATL's change to the Arcas Robin's tip by `(ratio / 0.25)ⁿ`: an
/// assumption, as no source measures a tip this small. The square gives the smaller loss.
pub const BLUNT_EXPONENTS: [i32; 2] = [2, 1];

/// The Mach numbers of [`SIMS_SLOPES`]' rows.
pub const SIMS_MACHS: [f64; 5] = [1.5, 1.75, 2.0, 2.5, 3.0];

/// The half-angles of [`SIMS_SLOPES`]' columns, degrees. The Arcas Robin nose's tangent cones run
/// from its tip's 10.76° down to its base, so 12.5° covers them.
pub const SIMS_ANGLES_DEG: [f64; 5] = [2.5, 5.0, 7.5, 10.0, 12.5];

/// A cone's `(dC_N/dα)` at `α = 0`, per radian on its base area: J. L. Sims, *Tables for
/// Supersonic Flow Around Right Circular Cones at Small Angle of Attack*, NASA SP-3007 (1964),
/// Table 2, printed p. 20, the same theory as TN 3527's Fig. 2 (Stone's, which Sims says gives
/// expressions "identical to those found by Kopal", p. 7; Fig. 2 plots Kopal's tables).
pub const SIMS_SLOPES: [[f64; 5]; 5] = [
    [1.9770483, 1.9336127, 1.8821566, 1.8280992, 1.7735619],
    [1.9731742, 1.9258447, 1.8737533, 1.8224578, 1.7730646],
    [1.9690287, 1.9180672, 1.8661051, 1.8183423, 1.7742540],
    [1.9602967, 1.9036740, 1.8548454, 1.8161039, 1.7820973],
    [1.9514206, 1.8917127, 1.8493550, 1.8202950, 1.7941549],
];

/// Below Mach 3 hpr holds Fig. 2's Mach 3 curve. The ratio of Sims's slope at a Mach number to
/// the slope hpr holds (its reading of Fig. 2 at Mach 3), over his tabulated angles and the rows at or around `mach` (and 1 at a
/// cone of 0°): its least and greatest, or `[1, 1]` from Mach 3; `None` below his first row or
/// for a Mach number that isn't a number. The body's lift is a sum of its tangent cones' slopes
/// with positive weights (TN 3527 eq. 19), so replacing each held slope by Sims's scales the
/// nose's share by a ratio within these.
pub fn fig2_ratio_range(mach: f64) -> Option<[f64; 2]> {
    let last = SIMS_MACHS.len() - 1;
    if mach.is_nan() || mach < SIMS_MACHS[0] {
        return None;
    }
    if mach >= SIMS_MACHS[last] {
        return Some([1.0, 1.0]);
    }
    // `SIMS_MACHS[0] <= mach < SIMS_MACHS[last]`, so `above` is 0 only at the first row.
    let above = SIMS_MACHS.partition_point(|&m| m < mach);
    let below = if SIMS_MACHS[above] == mach {
        above
    } else {
        above - 1
    };
    let mut range = [1.0_f64, 1.0_f64];
    for row in [below, above] {
        for (slope, angle) in SIMS_SLOPES[row].iter().zip(SIMS_ANGLES_DEG) {
            // hpr's own Mach 3 reading of Fig. 2, the slope it holds; Sims's angles are all within
            // Fig. 2's 24°, so the lookup can't fail.
            let held = cone_normal_force_slope(3.0, angle.to_radians()).ok()?;
            let ratio = slope / held;
            range = [range[0].min(ratio), range[1].max(ratio)];
        }
    }
    Some(range)
}

/// AFATL's relative change in `C_Nα` from the sharp nose to the blunt one: the larger (more
/// negative) at its Mach numbers at or around `mach`, its Mach 4 value past Mach 4; `None` below
/// Mach 1.5 or for a Mach number that isn't a number.
pub fn afatl_change(mach: f64) -> Option<f64> {
    if mach.is_nan() || mach < AFATL_MACHS[0] {
        return None;
    }
    let change = |i: usize| AFATL_PER_DEG[i][1] / AFATL_PER_DEG[i][0] - 1.0;
    let last = AFATL_MACHS.len() - 1;
    let above = AFATL_MACHS.partition_point(|&m| m < mach).min(last);
    let below = if AFATL_MACHS[above] <= mach || above == 0 {
        above
    } else {
        above - 1
    };
    Some(change(below).min(change(above)))
}

/// The least-squares fit of `ys` on three columns `basis(x)`, each point's reading error
/// independent with standard deviation `reading`: the coefficients, their standard errors, and
/// the correlation between the second and third coefficients' errors.
pub fn fit3(
    xs: &[f64],
    ys: &[f64],
    basis: impl Fn(f64) -> [f64; 3],
    reading: f64,
) -> Result<([f64; 3], [f64; 3], f64), String> {
    if xs.len() != ys.len() {
        return Err(format!("{} angles for {} readings", xs.len(), ys.len()));
    }
    let mut normal = [[0.0; 3]; 3];
    let mut rhs = [0.0; 3];
    for (x, y) in xs.iter().zip(ys) {
        let b = basis(*x);
        for i in 0..3 {
            rhs[i] += b[i] * y;
            for j in 0..3 {
                normal[i][j] += b[i] * b[j];
            }
        }
    }
    let inverse = invert3(&normal).ok_or("a singular fit: too few distinct angles")?;
    let mut coefficients = [0.0; 3];
    let mut errors = [0.0; 3];
    for i in 0..3 {
        coefficients[i] = (0..3).map(|j| inverse[i][j] * rhs[j]).sum();
        errors[i] = reading * inverse[i][i].sqrt();
    }
    let correlation = inverse[1][2] / (inverse[1][1] * inverse[2][2]).sqrt();
    Ok((coefficients, errors, correlation))
}

/// The inverse of a normal-equations matrix (symmetric, positive semi-definite) by its adjugate,
/// or `None` if it is singular. Its determinant is at most the product of its diagonal
/// (Hadamard), so their ratio says how independent the columns are whatever their units.
fn invert3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let cofactor = |i: usize, j: usize| {
        let (r0, r1) = ((i + 1) % 3, (i + 2) % 3);
        let (c0, c1) = ((j + 1) % 3, (j + 2) % 3);
        m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
    };
    let determinant: f64 = (0..3).map(|j| m[0][j] * cofactor(0, j)).sum();
    // `partial_cmp` so that a NaN determinant is singular too.
    let floor = 1e-12 * m[0][0] * m[1][1] * m[2][2];
    if determinant.partial_cmp(&floor) != Some(std::cmp::Ordering::Greater) {
        return None;
    }
    let mut inverse = [[0.0; 3]; 3];
    for (i, row) in inverse.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = cofactor(j, i) / determinant;
        }
    }
    Some(inverse)
}

/// The least-squares slope's standard error, each point's reading error independent with
/// standard deviation `reading`.
pub fn slope_error(xs: &[f64], reading: f64) -> f64 {
    let n = xs.len() as f64;
    let mean = xs.iter().sum::<f64>() / n;
    reading / xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>().sqrt()
}

/// The fixture, from the committed designs and wind-tunnel reference.
pub fn generate(root: &Path) -> Result<Value, String> {
    let text =
        fs::read_to_string(root.join(WIND_TUNNEL)).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let reference: Value =
        serde_json::from_str(&text).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let geometry = &reference["geometry"];
    let inches = |pointer: &str| {
        geometry
            .pointer(pointer)
            .and_then(Value::as_f64)
            .ok_or(format!("{WIND_TUNNEL}: no geometry{pointer}"))
    };
    let diameter_in = inches("/diameter_in")?;
    let boattail_in = inches("/boattail/aft_diameter_in")?;
    let lip_in = inches("/boattail/lip/aft_diameter_in")?;
    let lip = 2.0 * (lip_in.powi(2) - boattail_in.powi(2)) / diameter_in.powi(2);
    let boattail = 2.0 * (boattail_in.powi(2) - diameter_in.powi(2)) / diameter_in.powi(2);
    let nose_fineness = ARCAS_NOSE_X_IN[8] / (2.0 * ARCAS_NOSE_R_IN[8]);
    let (ratio, _) = arcas_nose_ratio()?;
    let diameter_m = 2.0 * ARCAS_NOSE_R_IN[8] * INCH;
    let area = 0.25 * PI * diameter_m * diameter_m;
    let mut configurations = Vec::new();
    let mut pooled = [0.0, 0.0];
    for configuration in reference["configurations"]
        .as_array()
        .ok_or(format!("{WIND_TUNNEL} has no configurations"))?
    {
        let id = configuration["id"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: a configuration without `id`"))?;
        let reading = READING
            .iter()
            .find(|(name, _)| *name == id)
            .map(|(_, u)| *u)
            .ok_or(format!("no reading accuracy for {id}"))?;
        let cylinder_end_in = geometry["cylinder_ends_in"][id]
            .as_f64()
            .ok_or(format!("{WIND_TUNNEL}: no cylinder end for {id}"))?;
        let design = configuration["design"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no design"))?;
        let model = arcas_model(root, design, Some(ratio), true)?;
        let scale = model.reference_area_m2() / area;
        let body_lift: f64 = scale * model.bodies().iter().map(|b| b.lift_factor).sum::<f64>();
        let method = arcas_body(ratio, cylinder_end_in, true)?;
        let bare = arcas_body(ratio, cylinder_end_in, false)?;
        let length_m = configuration["length_m"]
            .as_f64()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no length"))?;
        let eta = JORGENSEN_ETA
            .iter()
            .find(|(name, _)| *name == id)
            .map(|(_, e)| *e)
            .ok_or(format!("no Jorgensen eta for {id}"))?;
        let jorgensen_k = eta * JORGENSEN_C_DN;
        let mut rows = Vec::new();
        for curve in configuration["cn_alpha_fins_off"]
            .as_array()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no fins-off curves"))?
        {
            let mach = curve["mach"].as_f64().ok_or("a curve without `mach`")?;
            if mach < 1.5 {
                continue;
            }
            let points: Vec<(f64, f64)> = curve["alpha_deg_c_n"]
                .as_array()
                .ok_or("a curve without points")?
                .iter()
                .map(|p| Some((p[0].as_f64()?, p[1].as_f64()?)))
                .collect::<Option<_>>()
                .ok_or("an unreadable point")?;
            let alphas: Vec<f64> = points.iter().map(|p| p.0.to_radians()).collect();
            let measured: Vec<f64> = points.iter().map(|p| p.1).collect();
            let fitted = slope(&alphas, &measured);
            let (fit, [_, zero_alpha_error, curvature_error], correlation) =
                fit3(&alphas, &measured, |a| [1.0, a, a * a.abs()], reading)?;
            let [_, zero_alpha, curvature] = fit;
            let ([_, zero_alpha_cubic, _], _, _) =
                fit3(&alphas, &measured, |a| [1.0, a, a * a * a], reading)?;
            // The tunnel's slope at alpha -> 0 with its curvature held at Jorgensen's.
            let jorgensen_curvature = jorgensen_k * body_lift / BODY_LIFT_K;
            let without: Vec<f64> = alphas
                .iter()
                .zip(&measured)
                .map(|(a, c)| c - jorgensen_curvature * a * a.abs())
                .collect();
            let zero_alpha_jorgensen = slope(&alphas, &without);
            let (inner_alphas, inner_measured): (Vec<f64>, Vec<f64>) = alphas
                .iter()
                .zip(&measured)
                .filter(|(a, _)| a.to_degrees().abs() < INNER_DEG)
                .unzip();
            let inner = slope(&inner_alphas, &inner_measured);
            let (_, at_zero, _) = flight_bodies(&model, mach, area)?;
            let share = bare
                .slope(mach, area)
                .map_err(|e| format!("{id} at Mach {mach}: {e}"))?
                .slope_per_rad;
            let fig2 = fig2_ratio_range(mach)
                .ok_or(format!("Mach {mach} is below Sims's table"))?
                .map(|r| (r - 1.0) * share);
            let hpr: Vec<f64> = points
                .iter()
                .map(|p| hpr_force(&model, mach, p.0, true).map(|f| scale * f.0))
                .collect::<Result<_, _>>()?;
            let hpr_fitted = slope(&alphas, &hpr);
            // hpr's body lift alone at the plotted angles, fitted the same way: the part of its
            // fitted slope that crossflow adds.
            let lift: Vec<f64> = alphas
                .iter()
                .map(|a| body_lift * a.sin() * a.sin().abs())
                .collect();
            let crossflow = slope(&alphas, &lift);
            let gap = fitted - at_zero;
            let jorgensen_fitted = hpr_fitted - crossflow * (1.0 - jorgensen_k / BODY_LIFT_K);
            // The K that puts hpr's fitted slope within 15% of the measured, everything else as
            // it is: its body lift scales with K.
            let k_within = [0.85, 1.15]
                .map(|f| (f * fitted - (hpr_fitted - crossflow)) * BODY_LIFT_K / crossflow);
            // AFATL's change, the larger at its Mach numbers at or around the row's (its Mach 4
            // past that, an extrapolation), scaled to the Arcas Robin's tip.
            let tip = ARCAS_TIP_RADIUS_IN / ARCAS_NOSE_R_IN[8] / AFATL_TIP_RATIO;
            let afatl = afatl_change(mach).ok_or(format!("Mach {mach} is below AFATL's"))?;
            let blunt_tip = BLUNT_EXPONENTS.map(|n| afatl * tip.powi(n) * at_zero);
            let residuals: f64 = alphas
                .iter()
                .zip(&measured)
                .map(|(a, c)| {
                    ((c - fit[0] - zero_alpha * a - curvature * a * a.abs()) / reading).powi(2)
                })
                .sum();
            let numbers = [
                fitted,
                zero_alpha,
                zero_alpha_error,
                curvature,
                curvature_error,
                inner,
                at_zero,
                hpr_fitted,
                crossflow,
                share,
                fig2[0],
                fig2[1],
                residuals,
                blunt_tip[0],
                blunt_tip[1],
                correlation,
                zero_alpha_cubic,
                zero_alpha_jorgensen,
                jorgensen_fitted,
            ];
            if inner_alphas.len() < 2 || numbers.iter().any(|x| !x.is_finite()) {
                return Err(format!("{id} at Mach {mach}: a fit that isn't a number"));
            }
            pooled[0] += residuals;
            pooled[1] += (alphas.len() - 3) as f64;
            rows.push(json!({
                "mach": mach,
                "mach_over_nose_fineness": mach / nose_fineness,
                "measured": {
                    "fitted_c_n_alpha": fitted,
                    "fitted_standard_error": slope_error(&alphas, reading),
                    "zero_alpha_c_n_alpha": zero_alpha,
                    "zero_alpha_standard_error": zero_alpha_error,
                    "crossflow_per_rad2": curvature,
                    "crossflow_standard_error": curvature_error,
                    "crossflow_share": fitted - zero_alpha,
                    "implied_k": BODY_LIFT_K * curvature / body_lift,
                    "implied_k_standard_error": BODY_LIFT_K * curvature_error / body_lift,
                    "zero_alpha_crossflow_correlation": correlation,
                    "chi_square": residuals,
                    "degrees_of_freedom": alphas.len() - 3,
                    "zero_alpha_cubic_c_n_alpha": zero_alpha_cubic,
                    "inner_points": inner_alphas.len(),
                    "inner_c_n_alpha": inner,
                    "inner_standard_error": slope_error(&inner_alphas, reading),
                },
                "hpr": {
                    "zero_alpha_c_n_alpha": at_zero,
                    "fitted_c_n_alpha": hpr_fitted,
                    "fitted_c_n_alpha_error": hpr_fitted / fitted - 1.0,
                    "above_zero_alpha": at_zero - zero_alpha,
                    "above_inner": at_zero - inner,
                    "above_zero_alpha_cubic": at_zero - zero_alpha_cubic,
                    "k_within_15_percent": k_within,
                },
                "at_jorgensen_k": {
                    "hpr_fitted_c_n_alpha": jorgensen_fitted,
                    "hpr_fitted_c_n_alpha_error": jorgensen_fitted / fitted - 1.0,
                    "measured_zero_alpha_c_n_alpha": zero_alpha_jorgensen,
                    "hpr_above_zero_alpha": at_zero - zero_alpha_jorgensen,
                },
                "gap": gap,
                "sources": {
                    "crossflow": crossflow,
                    "crossflow_galejs_range": GALEJS_K.map(|k| crossflow * k / BODY_LIFT_K),
                    "lip": lip,
                    "nose_and_cylinder_c_n_alpha": share,
                    "fig2_below_mach_3": fig2,
                    "blunt_tip": blunt_tip,
                    "blunt_tip_extrapolated": mach > AFATL_MACHS[AFATL_MACHS.len() - 1],
                    "boattail_footnote_8": at_zero - share,
                    "boattail_slender_body": boattail,
                    "reduced_elements": method
                        .reduced_elements(mach)
                        .map_err(|e| format!("{id} at Mach {mach}: {e}"))?,
                },
                "remaining": gap - crossflow - lip,
            }));
        }
        configurations.push(json!({
            "id": id,
            "fineness": length_m / diameter_m,
            "jorgensen_k": jorgensen_k,
            "reading_c_n": reading,
            "body_lift_per_rad2": body_lift,
            "rows": rows,
        }));
    }
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "The Arcas Robin's body alone faster than sound (TN D-4014, fins off) against \
                 hpr's bodies in a flight: the nose as the fitted secant ogive of \
                 shock-expansion.json, the cylinder and the boattail, not the lip (M1.8e4's \
                 in_flight_with_boattail). All slopes per radian on the body's cross-section. \
                 measured.fitted_c_n_alpha is M1.8a's straight line through the plotted points; \
                 zero_alpha_c_n_alpha and crossflow_per_rad2 are b and c of C_N = a + b alpha + \
                 c alpha |alpha| through the same points (Jorgensen, NASA TR R-474, eq. 2.12), \
                 crossflow_share = fitted - zero_alpha the curvature's part of the straight \
                 line, and implied_k = 1.1 c / body_lift_per_rad2 the K that makes hpr's body \
                 lift that curvature; \
                 standard errors take each point's reading_c_n as an independent standard \
                 deviation. hpr.zero_alpha_c_n_alpha is its slope at alpha -> 0; \
                 hpr.fitted_c_n_alpha its bodies' C_N through the flight's path at the plotted \
                 angles, fitted the same way; above_zero_alpha and above_inner its slope at \
                 alpha -> 0 less measured.zero_alpha_c_n_alpha and measured.inner_c_n_alpha. gap = measured.fitted_c_n_alpha - \
                 hpr.zero_alpha_c_n_alpha. sources.crossflow is hpr's body lift, K (A_plan/A_ref) \
                 sin^2 alpha with K = 1.1, fitted at the plotted angles (body_lift_per_rad2 is K \
                 A_plan/A_ref); crossflow_galejs_range the same at K = 1.0 and 1.5. sources.lip is \
                 slender-body theory's share of the lip behind the boattail, 2 dA/A_ref. \
                 sources.reduced_elements counts the nose's elements the method reduces to the \
                 generalized method (issue #81). measured.inner_c_n_alpha is the straight line \
                 through the points within 3 deg only. sources.fig2_below_mach_3 bounds the \
                 change in the nose and cylinder's share (nose_and_cylinder_c_n_alpha, the \
                 method's, no boattail) if each tangent cone's slope, held at TN 3527 Fig. 2's \
                 Mach 3 curve below Mach 3, took Sims's value instead (NASA SP-3007 Table 2, \
                 p. 20): the least and greatest ratio of his slopes to the Mach 3 slopes hpr holds at his \
                 angles to 12.5 deg and his Mach numbers at or around the row's, less 1, times \
                 the share; zero from Mach 3. sources.blunt_tip is AFATL-TR-77-8's change from a \
                 sharp 4-caliber ogive to a tip of 0.25 of its radius (Table 3, the larger at its \
                 Mach numbers at or around the row's; blunt_tip_extrapolated past its Mach 4), \
                 scaled by (0.055/0.25)^n for n = 2 and 1 (an assumption), times hpr's slope at \
                 alpha -> 0. chi_square sums the alpha |alpha| fit's squared residuals over \
                 reading_c_n, on degrees_of_freedom. remaining = \
                 gap - crossflow - lip. zero_alpha_crossflow_correlation is the correlation of b's and c's \
                 errors; zero_alpha_cubic_c_n_alpha is b of C_N = a + b alpha + d alpha^3 \
                 instead, and hpr.above_zero_alpha_cubic hpr's slope less it; \
                 hpr.k_within_15_percent is the range of K that puts hpr's fitted slope within \
                 15% of the measured. at_jorgensen_k \
                 holds the curvature at Jorgensen's eta C_dn (jorgensen_k: eta from TR R-474 \
                 Fig. 4 at the model's fineness, C_dn = 1.2, p. 15): the tunnel's slope at \
                 alpha -> 0 with that curvature taken out, and hpr's fitted slope with its body \
                 lift scaled to that K. sources.boattail_footnote_8 is the boattail's share as \
                 hpr flies it (hpr at alpha -> 0 less nose_and_cylinder_c_n_alpha); \
                 boattail_slender_body is slender-body theory's, 2 (A_aft - A)/A_ref. No target.",
        "fig2_sims": {
            "machs": SIMS_MACHS,
            "angles_deg": SIMS_ANGLES_DEG,
            "slopes_per_rad": SIMS_SLOPES,
        },
        "nose_fineness": nose_fineness,
        "pooled_chi_square": pooled[0],
        "pooled_degrees_of_freedom": pooled[1],
        "configurations": configurations,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed fixture is what the generator writes from the committed designs and
    /// wind-tunnel reference. `cargo xtask aero --check` needs `refs/rocketpy` for the other
    /// fixtures and CI has none, so this test is what keeps this one current there.
    #[test]
    fn committed_fixture_is_current() {
        let root = crate::designs::root().unwrap();
        let committed: Value =
            serde_json::from_str(&fs::read_to_string(root.join(FIXTURE)).unwrap()).unwrap();
        let fresh = generate(&root).unwrap();
        assert!(
            crate::designs::same(&committed, &fresh),
            "{FIXTURE} differs from `cargo xtask aero`: {}",
            crate::designs::difference(&committed, &fresh).unwrap_or_default()
        );
    }

    /// [`READING`] and [`ARCAS_TIP_RADIUS_IN`] are the wind-tunnel reference's own.
    #[test]
    fn constants_match_the_wind_tunnel_reference() {
        let root = crate::designs::root().unwrap();
        let reference: Value =
            serde_json::from_str(&fs::read_to_string(root.join(WIND_TUNNEL)).unwrap()).unwrap();
        let reading = reference["reading"].as_str().unwrap();
        assert!(reading.contains("+-0.01 (TN D-4014 Fig. 5), +-0.013 (Fig. 6)"));
        assert_eq!(
            READING,
            [("arcas-robin-short", 0.01), ("arcas-robin-long", 0.013)]
        );
        let shape = reference["geometry"]["nose"]["shape"].as_str().unwrap();
        assert!(shape.contains(&format!("{ARCAS_TIP_RADIUS_IN}-in tip radius")));
    }

    /// Every range `docs/research/body-supersonic-gap.md` quotes: in the note, and from the
    /// committed fixture.
    #[test]
    fn the_research_note_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
        // Both notes, joined into one line, so a quote may wrap.
        let note = ["body-supersonic-gap.md", "body-supersonic-gap-sources.md"]
            .map(|name| fs::read_to_string(root.join("docs/research").join(name)).unwrap())
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let fixture: Value =
            serde_json::from_str(&fs::read_to_string(root.join(FIXTURE)).unwrap()).unwrap();
        let rows: Vec<&Value> = fixture["configurations"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|c| c["rows"].as_array().unwrap())
            .collect();
        assert_eq!(rows.len(), 11);
        let f = |row: &Value, pointer: &str| row.pointer(pointer).unwrap().as_f64().unwrap();
        let range = |pointer: &str, from_mach: f64| {
            rows.iter()
                .filter(|r| f(r, "/mach") >= from_mach)
                .map(|r| f(r, pointer))
                .fold([f64::INFINITY, f64::NEG_INFINITY], |[a, b], x| {
                    [a.min(x), b.max(x)]
                })
        };
        // Each range as the fixture rounds it, and the words the note quotes it in.
        let quoted = |[a, b]: [f64; 2], [lo, hi]: [f64; 2], within: f64, text: &str| {
            assert!(
                (a - lo).abs() <= within && (b - hi).abs() <= within,
                "{text}: {a} to {b}"
            );
            assert!(note.contains(text), "the note doesn't say `{text}`");
        };
        quoted(range("/gap", 0.0), [-0.18, 1.25], 0.005, "−0.18");
        quoted(
            range("/hpr/fitted_c_n_alpha_error", 0.0),
            [0.149, 0.732],
            0.0005,
            "15% to 73% high",
        );
        quoted(
            range("/sources/crossflow", 0.0),
            [1.40, 2.09],
            0.005,
            "1.40 to 2.09",
        );
        quoted(
            range("/measured/crossflow_share", 0.0),
            [0.09, 1.74],
            0.005,
            "0.09 to 1.74",
        );
        quoted(
            range("/measured/implied_k", 2.3),
            [0.66, 1.05],
            0.005,
            "0.66 to 1.05",
        );
        quoted(
            range("/measured/implied_k_standard_error", 2.3),
            [0.18, 0.23],
            0.005,
            "±0.18 to ±0.23",
        );
        quoted(
            range("/hpr/above_zero_alpha", 0.0),
            [0.06, 0.87],
            0.005,
            "0.06 to 0.87 above",
        );
        quoted(
            range("/hpr/above_zero_alpha_cubic", 0.0),
            [-0.35, 0.43],
            0.005,
            "between 0.35 below and 0.43 above",
        );
        let cubic: Vec<f64> = rows
            .iter()
            .map(|r| {
                f(r, "/measured/zero_alpha_cubic_c_n_alpha")
                    - f(r, "/measured/zero_alpha_c_n_alpha")
            })
            .collect();
        quoted(
            [
                cubic.iter().copied().fold(f64::INFINITY, f64::min),
                cubic.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ],
            [0.01, 0.61],
            0.005,
            "0.01 to 0.61 higher",
        );
        quoted(
            range("/measured/zero_alpha_crossflow_correlation", 0.0),
            [-0.96, -0.95],
            0.005,
            "−0.95 to −0.96",
        );
        quoted(
            range("/at_jorgensen_k/measured_zero_alpha_c_n_alpha", 0.0),
            [1.04, 2.98],
            0.005,
            "1.04 to 2.98",
        );
        quoted(
            range("/at_jorgensen_k/hpr_above_zero_alpha", 0.0),
            [0.37, 1.33],
            0.005,
            "0.37 to 1.33 below hpr's",
        );
        quoted(
            range("/at_jorgensen_k/hpr_fitted_c_n_alpha_error", 0.0),
            [0.082, 0.607],
            0.0005,
            "8.2% to 60.7% high",
        );
        quoted(
            range("/sources/boattail_footnote_8", 0.0),
            [-0.177, -0.026],
            0.0005,
            "−0.177 at Mach 1.5 to −0.026",
        );
        quoted(
            range("/sources/boattail_slender_body", 0.0),
            [-1.324, -1.324],
            0.0005,
            "= −1.324",
        );
        let spread: Vec<f64> = rows
            .iter()
            .map(|r| f(r, "/sources/boattail_footnote_8") - f(r, "/sources/boattail_slender_body"))
            .collect();
        quoted(
            [
                spread.iter().copied().fold(f64::INFINITY, f64::min),
                spread.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ],
            [1.15, 1.30],
            0.005,
            "1.15 to 1.30",
        );
        let fig2 = [
            range("/sources/fig2_below_mach_3/0", 0.0)[0],
            range("/sources/fig2_below_mach_3/1", 0.0)[1],
        ];
        quoted(fig2, [-0.031, 0.056], 0.0005, "−0.031 to +0.056");
        // The same as a share of the nose and cylinder's slope, at Mach 1.5 and 2.96.
        for (mach, [lo, hi], text) in [
            (1.5, [-1.1, 2.2], "−1.1% to +2.2%"),
            (2.96, [-0.6, 0.9], "−0.6% to +0.9%"),
        ] {
            let row = rows.iter().find(|r| f(r, "/mach") == mach).unwrap();
            let share = f(row, "/sources/nose_and_cylinder_c_n_alpha");
            quoted(
                [
                    100.0 * f(row, "/sources/fig2_below_mach_3/0") / share,
                    100.0 * f(row, "/sources/fig2_below_mach_3/1") / share,
                ],
                [lo, hi],
                0.05,
                text,
            );
        }
        quoted(range("/sources/lip", 0.0), [0.178, 0.178], 0.0005, "+0.178");
        quoted(
            range("/sources/reduced_elements", 0.0),
            [0.0, 0.0],
            0.0,
            "none on the Arcas Robin's body",
        );
        let blunt = [
            -range("/sources/blunt_tip/0", 3.0)[1],
            -range("/sources/blunt_tip/1", 3.0)[0],
        ];
        quoted(blunt, [0.015, 0.07], 0.001, "0.015 to 0.07");
        let below: Vec<&&Value> = rows.iter().filter(|r| f(r, "/mach") < 3.0).collect();
        let loss = |r: &Value, n: usize| -r["sources"]["blunt_tip"][n].as_f64().unwrap();
        let mid: Vec<[f64; 2]> = below
            .iter()
            .filter(|r| f(r, "/mach") > 2.0)
            .map(|r| [loss(r, 0), loss(r, 1)])
            .collect();
        quoted(
            [
                mid.iter().map(|m| m[0]).fold(f64::INFINITY, f64::min),
                mid.iter().map(|m| m[1]).fold(f64::NEG_INFINITY, f64::max),
            ],
            [0.002, 0.011],
            0.0005,
            "0.002 to 0.011 at 2.3 and 2.96",
        );
        assert!(
            below
                .iter()
                .filter(|r| f(r, "/mach") < 2.0)
                .all(|r| loss(r, 0) == 0.0 && loss(r, 1) == 0.0)
        );
        assert!(note.contains("nothing the table resolves at Mach 1.5 and 1.8"));
        // No single K puts all 11 rows within 15%.
        let (lo, hi) = (
            range("/hpr/k_within_15_percent/0", 0.0)[1],
            range("/hpr/k_within_15_percent/1", 0.0)[0],
        );
        quoted([hi, lo], [0.11, 0.32], 0.005, "at most 0.11");
        assert!(note.contains("at least 0.32"));
        // Only the Mach 4.63 rows extrapolate past AFATL's Mach 4.
        assert!(
            rows.iter()
                .all(|r| r["sources"]["blunt_tip_extrapolated"] == (f(r, "/mach") > 4.0))
        );
        // The fits' scatter against the reading error.
        let chi = [
            fixture["pooled_chi_square"].as_f64().unwrap(),
            fixture["pooled_degrees_of_freedom"].as_f64().unwrap(),
        ];
        quoted(chi, [43.6, 44.0], 0.05, "χ² 43.6 on 44 degrees of freedom");
        // The gap is smaller than the tunnel's own crossflow at every row.
        assert!(
            rows.iter()
                .all(|r| f(r, "/gap").abs() < f(r, "/measured/crossflow_share"))
        );
        // Jorgensen's eta C_dn (Fig. 4's eta at l/d 18.2 and 23.8, times C_dn = 1.2) lies within
        // 1.3 standard errors of each of the eight implied K from Mach 2.3; hpr's 1.1 lies above
        // all eight, by up to 2.2.
        let (mut checked, mut widest) = (0, 0.0_f64);
        for configuration in fixture["configurations"].as_array().unwrap() {
            let jorgensen = if configuration["id"] == "arcas-robin-short" {
                0.74 * 1.2
            } else {
                0.77 * 1.2
            };
            for row in configuration["rows"].as_array().unwrap() {
                if f(row, "/mach") < 2.3 {
                    continue;
                }
                let (k, e) = (
                    f(row, "/measured/implied_k"),
                    f(row, "/measured/implied_k_standard_error"),
                );
                assert!((k - jorgensen).abs() < 1.3 * e);
                assert!(k < BODY_LIFT_K);
                widest = widest.max((BODY_LIFT_K - k) / e);
                checked += 1;
            }
        }
        assert_eq!(checked, 8);
        assert!((widest - 2.2).abs() < 0.05, "{widest}");
        for text in ["within 1.3 standard errors", "by up to 2.2 standard errors"] {
            assert!(note.contains(text), "the note doesn't say `{text}`");
        }
        // The guide quotes the same ranges, each checked against the fixture above.
        let guide = fs::read_to_string(root.join("docs/physics/aero.md"))
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        for text in [
            "14.9% to 73.2% high",
            "1.40 to 2.09",
            "0.09 to 1.74",
            "from 0.66 to 1.05, each ±0.18 to ±0.23",
            "correlate at −0.95 to −0.96",
            "8.2% to 60.7% high",
            "−0.177 to −0.026, slender-body theory −1.324",
            "adds +0.178",
            "loses 0.015 to 0.07 past Mach 3",
            "−0.031 to +0.056",
        ] {
            assert!(guide.contains(text), "aero.md doesn't say `{text}`");
        }
    }

    /// Sims's Mach 3 slopes against hpr's reading of TN 3527's Fig. 2, which Sims's theory
    /// plots: within 0.003 from 5° to 12.5°, 0.01 at 2.5°.
    #[test]
    fn sims_agrees_with_hpr_s_fig2_at_mach_3() {
        for (angle, sims) in SIMS_ANGLES_DEG.iter().zip(SIMS_SLOPES[4]) {
            let hpr = hpr_aero::shock_expansion::cone_normal_force_slope(3.0, angle.to_radians())
                .unwrap();
            let within = if *angle < 5.0 { 0.01 } else { 0.003 };
            assert!((hpr - sims).abs() < within, "{angle}°: {hpr} {sims}");
        }
    }

    /// The boattail's share barely depends on the loading the cylinder carries to it:
    /// lengthening the Arcas Robin's cylinder from 39.14 in to 1000 in moves it by under 0.002
    /// per radian at every tunnel Mach number, so the nose's loading has died away before it.
    #[test]
    fn the_boattail_share_carries_almost_nothing_from_the_nose() {
        let (ratio, _) = arcas_nose_ratio().unwrap();
        let area = 0.25 * PI * (2.0 * ARCAS_NOSE_R_IN[8] * INCH).powi(2);
        let short = arcas_body(ratio, 39.14, true).unwrap();
        let long = arcas_body(ratio, 1000.0, true).unwrap();
        for mach in [1.5, 1.8, 2.3, 2.96, 3.96, 4.63] {
            let a = short.segment_slopes(mach, area).unwrap()[2].slope_per_rad;
            let b = long.segment_slopes(mach, area).unwrap()[2].slope_per_rad;
            assert!((a - b).abs() < 0.002, "Mach {mach}: {a} {b}");
        }
    }

    #[test]
    fn afatl_changes_take_the_larger_around_each_mach() {
        let close = |got: Option<f64>, want: f64| assert!((got.unwrap() - want).abs() < 1e-12);
        // Mach 1.5 and 2: no change the table resolves; Mach 3: 0.060 to 0.059; Mach 4: 0.063
        // to 0.057.
        close(afatl_change(1.5), 0.0);
        close(afatl_change(1.8), 0.0);
        close(afatl_change(2.3), 0.059 / 0.060 - 1.0);
        close(afatl_change(2.96), 0.059 / 0.060 - 1.0);
        close(afatl_change(3.96), 0.057 / 0.063 - 1.0);
        close(afatl_change(4.0), 0.057 / 0.063 - 1.0);
        close(afatl_change(4.63), 0.057 / 0.063 - 1.0);
        assert_eq!(afatl_change(1.49), None);
        assert_eq!(afatl_change(f64::NAN), None);
    }

    #[test]
    fn fig2_ratios_bracket_the_tabulated_rows() {
        let close = |got: Option<[f64; 2]>, want: [f64; 2]| {
            let got = got.unwrap();
            assert!(
                (got[0] - want[0]).abs() < 1e-6 && (got[1] - want[1]).abs() < 1e-6,
                "{got:?} {want:?}"
            );
        };
        // By hand, Table 2 over hpr's Mach 3 reading of Fig. 2 (1.942, 1.892, 1.8505, 1.820 and
        // 1.79325 at 2.5° to 12.5°): at Mach 1.5 its own row (12.5° and 5° set the ends); at 2.96
        // the rows for 2.5 and 3.0 (12.5° and 2.5° of the 2.5 row).
        close(fig2_ratio_range(1.5), [0.989_021, 1.021_994]);
        close(fig2_ratio_range(2.96), [0.993_781, 1.009_421]);
        // From Mach 3 nothing changes; below the table, or not a number, there is no answer.
        assert_eq!(fig2_ratio_range(3.0), Some([1.0, 1.0]));
        assert_eq!(fig2_ratio_range(4.63), Some([1.0, 1.0]));
        assert_eq!(fig2_ratio_range(1.49), None);
        assert_eq!(fig2_ratio_range(f64::NAN), None);
        // Sims's 10° cone at Mach 1.5 over Mach 3, straight from the table.
        assert!((SIMS_SLOPES[0][3] / SIMS_SLOPES[4][3] - 1.004_287).abs() < 1e-6);
    }

    #[test]
    fn fit3_recovers_an_exact_curve_and_its_errors_by_hand() {
        let xs: [f64; 7] = [-0.07, -0.035, -0.017, 0.0, 0.019, 0.037, 0.072];
        let ys: Vec<f64> = xs
            .iter()
            .map(|&a| 0.02 + 2.5 * a + 20.0 * a * a.abs())
            .collect();
        let (c, _, _) = fit3(&xs, &ys, |a| [1.0, a, a * a.abs()], 0.01).unwrap();
        for (got, want) in c.iter().zip([0.02, 2.5, 20.0]) {
            assert!((got - want).abs() < 1e-9, "{got} {want}");
        }
        // At -2 to 2 the normal matrix is [[5, 0, 0], [0, 10, 18], [0, 18, 34]], whose inverse's
        // diagonal is 1/5, 34/16 and 10/16.
        let xs = [-2.0, -1.0, 0.0, 1.0, 2.0];
        let ys = [0.0; 5];
        let (_, e, correlation) = fit3(&xs, &ys, |a| [1.0, a, a * a.abs()], 1.0).unwrap();
        // ... and whose off-diagonal, -18/16, over sqrt(34/16 · 10/16) is -18/sqrt(340).
        assert!((correlation + 18.0 / 340.0_f64.sqrt()).abs() < 1e-12);
        for (got, want) in e
            .iter()
            .zip([0.2_f64.sqrt(), 2.125_f64.sqrt(), 0.625_f64.sqrt()])
        {
            assert!((got - want).abs() < 1e-12, "{got} {want}");
        }
        // The slope's error alone is 1/sqrt(10), and an even column beside it doesn't change it.
        assert!((slope_error(&xs, 1.0) - 0.1_f64.sqrt()).abs() < 1e-15);
        let (_, even, _) = fit3(&xs, &ys, |a| [1.0, a, a * a], 1.0).unwrap();
        assert!((even[1] - 0.1_f64.sqrt()).abs() < 1e-12);
        // A repeated column is singular whatever its units, and a well-posed fit at small angles
        // is not.
        assert!(fit3(&xs, &ys, |a| [1.0, a, a], 0.01).is_err());
        assert!(fit3(&xs, &ys, |a| [1.0, 57.3 * a, 3.0 * 57.3 * a], 0.01).is_err());
        let small: Vec<f64> = xs.iter().map(|x| 0.1 * x * 0.001).collect();
        assert!(fit3(&small, &ys, |a| [1.0, a, a * a.abs()], 0.01).is_ok());
        assert!(fit3(&xs, &ys[..4], |a| [1.0, a, a * a.abs()], 0.01).is_err());
    }
}
