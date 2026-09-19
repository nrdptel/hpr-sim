//! The Arcas Robin's supersonic body gap, source by source (M1.8e5): the measured body alone
//! (NASA TN D-4014, fins off) against hpr's body in a flight, and the size of each candidate
//! cause that hpr can compute, written by `cargo xtask aero` to
//! `validation/fixtures/aero/arcas-robin-gap.json`. `docs/research/body-supersonic-gap.md`
//! ranks the candidates from it.
//!
//! - **The fit's angles.** The measured slope (M1.8a's) is a straight line fitted through `C_N`
//!   plotted from about −5° to +4°. Crossflow lift grows as `α |α|`, so it steepens that line.
//!   The fixture fits the same points with a crossflow term, `C_N = a + b α + c α |α|`
//!   (Jorgensen, NASA TR R-474, eq. 1, where the term is `η C_dc (A_p/A) sin² α`), to separate
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
use serde_json::{Value, json};

use crate::aero_body::{
    ARCAS_NOSE_R_IN, ARCAS_NOSE_X_IN, arcas_body, arcas_model, arcas_nose_ratio, flight_bodies,
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
/// his slope at Mach 3, over his tabulated angles and the rows at or around `mach` (and 1 at a
/// cone of 0°): its least and greatest, or `[1, 1]` from Mach 3. The body's lift is a sum of its
/// tangent cones' slopes with positive weights (TN 3527 eq. 19), so replacing each held slope by
/// Sims's scales the nose's share by a ratio within these.
pub fn fig2_ratio_range(mach: f64) -> [f64; 2] {
    let last = SIMS_MACHS.len() - 1;
    if mach >= SIMS_MACHS[last] {
        return [1.0, 1.0];
    }
    let above = SIMS_MACHS.partition_point(|&m| m < mach).min(last);
    let below = if SIMS_MACHS[above] == mach || above == 0 {
        above
    } else {
        above - 1
    };
    let mut range = [1.0_f64, 1.0_f64];
    for row in [below, above] {
        for (slope, at_3) in SIMS_SLOPES[row].iter().zip(SIMS_SLOPES[last]) {
            let ratio = slope / at_3;
            range = [range[0].min(ratio), range[1].max(ratio)];
        }
    }
    range
}

/// The least-squares fit of `ys` on three columns `basis(x)`, each point's reading error
/// independent with standard deviation `reading`: the coefficients and their standard errors.
pub fn fit3(
    xs: &[f64],
    ys: &[f64],
    basis: impl Fn(f64) -> [f64; 3],
    reading: f64,
) -> Result<([f64; 3], [f64; 3]), String> {
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
    Ok((coefficients, errors))
}

/// The inverse of a symmetric 3 × 3 matrix by its adjugate, or `None` if it is singular.
fn invert3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    let cofactor = |i: usize, j: usize| {
        let (r0, r1) = ((i + 1) % 3, (i + 2) % 3);
        let (c0, c1) = ((j + 1) % 3, (j + 2) % 3);
        m[r0][c0] * m[r1][c1] - m[r0][c1] * m[r1][c0]
    };
    let determinant: f64 = (0..3).map(|j| m[0][j] * cofactor(0, j)).sum();
    if determinant.abs() <= f64::EPSILON * m[0][0].abs().max(1.0).powi(3) {
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
    let nose_fineness = ARCAS_NOSE_X_IN[8] / (2.0 * ARCAS_NOSE_R_IN[8]);
    let (ratio, _) = arcas_nose_ratio()?;
    let diameter_m = 2.0 * ARCAS_NOSE_R_IN[8] * 0.0254;
    let area = 0.25 * PI * diameter_m * diameter_m;
    let mut configurations = Vec::new();
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
            let ([_, zero_alpha, curvature], [_, zero_alpha_error, curvature_error]) =
                fit3(&alphas, &measured, |a| [1.0, a, a * a.abs()], reading)?;
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
            let fig2 = fig2_ratio_range(mach).map(|r| (r - 1.0) * share);
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
                },
                "gap": gap,
                "sources": {
                    "crossflow": crossflow,
                    "crossflow_galejs_range": GALEJS_K.map(|k| crossflow * k / BODY_LIFT_K),
                    "lip": lip,
                    "nose_and_cylinder_c_n_alpha": share,
                    "fig2_below_mach_3": fig2,
                    "reduced_elements": method
                        .reduced_elements(mach)
                        .map_err(|e| format!("{id} at Mach {mach}: {e}"))?,
                },
                "remaining": gap - crossflow - lip,
            }));
        }
        configurations.push(json!({
            "id": id,
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
                 c alpha |alpha| through the same points (Jorgensen, NASA TR R-474, eq. 1), \
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
                 p. 20): the least and greatest ratio of his slopes to his Mach 3 slopes at his \
                 angles to 12.5 deg and his Mach numbers at or around the row's, less 1, times \
                 the share; zero from Mach 3. remaining = gap - crossflow - lip. No target.",
        "fig2_sims": {
            "machs": SIMS_MACHS,
            "angles_deg": SIMS_ANGLES_DEG,
            "slopes_per_rad": SIMS_SLOPES,
        },
        "nose_fineness": nose_fineness,
        "configurations": configurations,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed fixture is what the generator writes from the committed designs and
    /// wind-tunnel reference (no `refs/` needed), so CI catches a stale one.
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

    /// The ranges `docs/research/body-supersonic-gap.md` quotes, from the committed fixture.
    #[test]
    fn the_research_note_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
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
        let near = |[a, b]: [f64; 2], [lo, hi]: [f64; 2], within: f64, what: &str| {
            assert!(
                (a - lo).abs() <= within && (b - hi).abs() <= within,
                "{what}: {a} to {b}"
            );
        };
        near(range("/gap", 0.0), [-0.18, 1.25], 0.005, "gap");
        near(
            range("/hpr/fitted_c_n_alpha_error", 0.0),
            [0.149, 0.732],
            0.0005,
            "hpr fitted",
        );
        near(
            range("/sources/crossflow", 0.0),
            [1.40, 2.09],
            0.005,
            "hpr crossflow",
        );
        near(
            range("/measured/crossflow_share", 0.0),
            [0.09, 1.74],
            0.005,
            "tunnel crossflow",
        );
        near(
            range("/measured/implied_k", 2.3),
            [0.66, 1.05],
            0.005,
            "K from 2.3",
        );
        near(
            range("/measured/implied_k_standard_error", 2.3),
            [0.18, 0.23],
            0.005,
            "K errors",
        );
        near(
            range("/hpr/above_zero_alpha", 0.0),
            [0.06, 0.87],
            0.005,
            "above b",
        );
        near(
            range("/hpr/above_inner", 0.0),
            [-0.37, 0.36],
            0.005,
            "above inner",
        );
        near(
            range("/sources/fig2_below_mach_3/0", 0.0),
            [-0.033, 0.0],
            0.0005,
            "Fig. 2 low",
        );
        near(
            range("/sources/fig2_below_mach_3/1", 0.0),
            [0.0, 0.057],
            0.0005,
            "Fig. 2 high",
        );
        near(range("/sources/lip", 0.0), [0.178, 0.178], 0.0005, "lip");
        near(
            range("/sources/reduced_elements", 0.0),
            [0.0, 0.0],
            0.0,
            "#81",
        );
        // The gap is smaller than the tunnel's own crossflow at every row.
        assert!(
            rows.iter()
                .all(|r| f(r, "/gap").abs() < f(r, "/measured/crossflow_share"))
        );
        // Jorgensen's eta C_dn (Fig. 4's eta at l/d 18.2 and 23.8, times C_dn = 1.2) lies within
        // 1.3 standard errors of every implied K from Mach 2.3; hpr's 1.1 above all of them, by
        // up to 2.2.
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
                assert!(k < BODY_LIFT_K && (BODY_LIFT_K - k) < 2.2 * e);
            }
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

    #[test]
    fn fig2_ratios_bracket_the_tabulated_rows() {
        // From Mach 3 nothing changes; at a tabulated Mach number only its own row counts.
        assert_eq!(fig2_ratio_range(3.0), [1.0, 1.0]);
        assert_eq!(fig2_ratio_range(4.63), [1.0, 1.0]);
        let at = |row: usize| {
            SIMS_SLOPES[row]
                .iter()
                .zip(SIMS_SLOPES[4])
                .map(|(s, t)| s / t)
                .fold([1.0_f64, 1.0_f64], |r, x| [r[0].min(x), r[1].max(x)])
        };
        assert_eq!(fig2_ratio_range(1.5), at(0));
        assert_eq!(fig2_ratio_range(2.0), at(2));
        // Between rows, both rows' ratios.
        let (a, b) = (at(1), at(2));
        assert_eq!(fig2_ratio_range(1.8), [a[0].min(b[0]), a[1].max(b[1])]);
        // Sims's 10° cone at Mach 1.5 over Mach 3, straight from the table.
        assert!((SIMS_SLOPES[0][3] / SIMS_SLOPES[4][3] - 1.004_287).abs() < 1e-6);
    }

    #[test]
    fn fit3_recovers_an_exact_curve_and_scales_its_errors() {
        let xs: [f64; 7] = [-0.07, -0.035, -0.017, 0.0, 0.019, 0.037, 0.072];
        let ys: Vec<f64> = xs
            .iter()
            .map(|&a| 0.02 + 2.5 * a + 20.0 * a * a.abs())
            .collect();
        let (c, e) = fit3(&xs, &ys, |a| [1.0, a, a * a.abs()], 0.01).unwrap();
        for (got, want) in c.iter().zip([0.02, 2.5, 20.0]) {
            assert!((got - want).abs() < 1e-9, "{got} {want}");
        }
        let (_, twice) = fit3(&xs, &ys, |a| [1.0, a, a * a.abs()], 0.02).unwrap();
        for (a, b) in e.iter().zip(twice) {
            assert!((2.0 * a - b).abs() < 1e-15 * b);
        }
        // With the curvature column dropped (a repeated column), the fit is singular.
        assert!(fit3(&xs, &ys, |a| [1.0, a, a], 0.01).is_err());
        // The straight line's error: u / sqrt(sum (x - mean)^2).
        let n = xs.len() as f64;
        let mean = xs.iter().sum::<f64>() / n;
        let sxx: f64 = xs.iter().map(|x| (x - mean).powi(2)).sum();
        assert!((slope_error(&xs, 0.01) - 0.01 / sxx.sqrt()).abs() < 1e-15);
    }
}
