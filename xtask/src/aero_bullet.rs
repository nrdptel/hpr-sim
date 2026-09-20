//! M1.8e's 15% bullet, judged (M1.8e9, ADR-040): the Arcas Robin's body alone, as the committed
//! designs fly it, against NASA TN D-4014's fins-off measurement, with the gap split between the
//! attached-flow slope and the curvature body lift adds. Written by `cargo xtask aero` to
//! `validation/fixtures/aero/arcas-robin-body-gap.json`.
//!
//! - **The target** (ROADMAP M1.8e): the body alone within 15% at every Mach number from 1.5, and
//!   both configurations' whole-rocket `C_Nα` within 15% at Mach 3.96 and 4.63.
//! - **How it is judged** (ADR-036): hpr's `C_N` at the tunnel's plotted angles, fitted with a
//!   straight line as the measurement is.
//! - **The split.** A fitted slope is the slope at `α → 0` plus what the curve's bend adds over
//!   the plotted angles. Each side is reported: hpr's `α → 0` share is the shock-expansion
//!   method's, and the measurement's is `arcas-robin-gap.json`'s `α |α|` fit (M1.8e5). What is
//!   left is the curvature, which for hpr is body lift.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::BodyModel;
use serde_json::{Value, json};

use crate::aero_body::{ARCAS_NOSE_R_IN, INCH, arcas_model, flight_bodies};
use crate::aero_gap::FIXTURE as GAP;
use crate::aero_mach::{WIND_TUNNEL, hpr_force, slope};

pub const FIXTURE: &str = "validation/fixtures/aero/arcas-robin-body-gap.json";

/// The bullet's bound on the body alone and on both configurations at Mach 3.96 and 4.63.
pub const TARGET: f64 = 0.15;

fn read(root: &Path, name: &str) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))
}

pub fn generate(root: &Path) -> Result<Value, String> {
    let reference = read(root, WIND_TUNNEL)?;
    let gap = read(root, GAP)?;
    let diameter_m = 2.0 * ARCAS_NOSE_R_IN[8] * INCH;
    let area = 0.25 * PI * diameter_m * diameter_m;
    let mut configurations = Vec::new();
    let mut outside = Vec::new();
    for configuration in reference["configurations"]
        .as_array()
        .ok_or(format!("{WIND_TUNNEL} has no configurations"))?
    {
        let id = configuration["id"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: a configuration without `id`"))?;
        let design = configuration["design"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no design"))?;
        let model = arcas_model(root, design, None, false, BodyModel::CURRENT)?;
        let scale = model.reference_area_m2() / area;
        let gap_rows = gap["configurations"]
            .as_array()
            .and_then(|c| c.iter().find(|c| c["id"] == id))
            .and_then(|c| c["rows"].as_array())
            .ok_or(format!("{GAP} has no rows for {id}"))?;
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
                .map(|p| match (p[0].as_f64(), p[1].as_f64()) {
                    (Some(a), Some(c)) => Ok((a, c)),
                    _ => Err("a malformed point".to_owned()),
                })
                .collect::<Result<_, _>>()?;
            let alphas: Vec<f64> = points.iter().map(|p| p.0.to_radians()).collect();
            // Two distinct angles, or the least-squares slope is 0/0 and the fixture would
            // commit nulls where its numbers belong.
            if alphas.iter().all(|a| *a == alphas[0]) {
                return Err(format!(
                    "{WIND_TUNNEL}: {id} at Mach {mach} plots fewer than two distinct angles"
                ));
            }
            let measured = slope(&alphas, &points.iter().map(|p| p.1).collect::<Vec<_>>());
            let c_n: Vec<f64> = points
                .iter()
                .map(|p| hpr_force(&model, mach, p.0, true).map(|f| scale * f.0))
                .collect::<Result<_, _>>()?;
            let fitted = slope(&alphas, &c_n);
            // hpr's slope at `α → 0`: body lift vanishes there, so this is the method's own —
            // as long as the method covers every body. Both committed designs fly it to their
            // base since M1.8e8; if a switch (issue #87) ever drops a segment from the run, the
            // column would quietly become part slender-body theory, so refuse that here.
            let (covered, zero_alpha, _) = flight_bodies(&model, mach, area)?;
            if (covered - zero_alpha).abs() > 1e-12 * zero_alpha.abs() {
                return Err(format!(
                    "{id} at Mach {mach}: the method covers {covered} per rad of the body's \
                     {zero_alpha}, so the zero-alpha column is not the method's own"
                ));
            }
            let gap_row = gap_rows
                .iter()
                .find(|r| r["mach"].as_f64() == Some(mach))
                .ok_or(format!("{GAP}: no row for {id} at Mach {mach}"))?;
            let measured_zero = gap_row["measured"]["zero_alpha_c_n_alpha"]
                .as_f64()
                .ok_or("no measured zero-alpha slope")?;
            let measured_zero_error = gap_row["measured"]["zero_alpha_standard_error"]
                .as_f64()
                .ok_or("no measured zero-alpha standard error")?;
            let mach_over_fineness = gap_row["mach_over_nose_fineness"]
                .as_f64()
                .ok_or("no Mach over nose fineness")?;
            // How tightly the fit's two terms trade off: near −1 the measurement cannot tell a
            // low slope at `α → 0` from a high curvature, so the split below is soft.
            let correlation = gap_row["measured"]["zero_alpha_crossflow_correlation"]
                .as_f64()
                .ok_or("no zero-alpha/crossflow correlation")?;
            let error = fitted / measured - 1.0;
            if error.abs() > TARGET {
                outside.push(format!("{id}@{mach}"));
            }
            rows.push(json!({
                "mach": mach,
                "mach_over_nose_fineness": mach_over_fineness,
                "measured": {
                    "fitted_c_n_alpha": measured,
                    "zero_alpha_c_n_alpha": measured_zero,
                    "zero_alpha_standard_error": measured_zero_error,
                    "zero_alpha_crossflow_correlation": correlation,
                    "curvature": measured - measured_zero,
                },
                "hpr": {
                    "fitted_c_n_alpha": fitted,
                    "zero_alpha_c_n_alpha": zero_alpha,
                    "curvature": fitted - zero_alpha,
                },
                "fitted_error": error,
                "within_target": error.abs() <= TARGET,
                "zero_alpha_error": zero_alpha / measured_zero - 1.0,
                "zero_alpha_ratio": zero_alpha / measured_zero,
                "zero_alpha_standard_errors": (zero_alpha - measured_zero) / measured_zero_error,
                "zero_alpha_share_of_gap": (zero_alpha - measured_zero) / (fitted - measured),
                "curvature_ratio": (fitted - zero_alpha) / (measured - measured_zero),
            }));
        }
        configurations.push(json!({ "id": id, "design": design, "rows": rows }));
    }
    Ok(json!({
        "generator": "cargo xtask aero (xtask/src/aero_bullet.rs)",
        "note": "M1.8e's 15% bullet for the Arcas Robin's body alone, judged as ADR-036 judges the \
                 tunnel: the committed design's bodies through the flight's path, their C_N at the \
                 plotted angles fitted with a straight line as TN D-4014's fins-off measurement \
                 is, per radian on the body's cross-section. fitted_error is hpr's over the \
                 measured, minus 1, and within_target is |fitted_error| <= 0.15. The split: \
                 zero_alpha_c_n_alpha is hpr's slope where body lift vanishes, the \
                 shock-expansion method's own, against the measurement's alpha|alpha| fit from \
                 arcas-robin-gap.json (M1.8e5); curvature is what the rest of the plotted angles \
                 add, body lift for hpr, and curvature_ratio is hpr's over the measured's. \
                 zero_alpha_standard_errors is how many of the measurement's own standard errors \
                 separate the two zero-alpha slopes, and zero_alpha_share_of_gap how much of the \
                 fitted gap sits there; the share means little where the fitted gap is small, so \
                 read it on the rows outside the target. The split is soft on the measurement's \
                 side: zero_alpha_crossflow_correlation is how the fit's two terms trade off, \
                 near -1, so a low slope at alpha to 0 forces a high curvature and the curvature \
                 carries at least the zero-alpha standard error. mach_over_nose_fineness is the Mach \
                 number over the nose's fineness, which TN 3527 states its method for from 0.4 \
                 to 2.",
        "target": TARGET,
        "configurations": configurations,
        "outside_the_target": outside,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bullet's judgement, pinned: which rows are outside 15%, and that the whole rocket is
    /// within it at Mach 3.96 and 4.63 on both configurations (the bullet's second half).
    #[test]
    fn the_bullet_is_judged_the_same_way_every_time() {
        let root = crate::designs::root().unwrap();
        let fixture = read(&root, FIXTURE).unwrap();
        let outside: Vec<&str> = fixture["outside_the_target"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(
            outside,
            [
                "arcas-robin-short@1.5",
                "arcas-robin-short@1.8",
                "arcas-robin-short@2.3",
                "arcas-robin-short@2.96",
                "arcas-robin-long@1.8",
                "arcas-robin-long@2.3",
            ]
        );
        // Where the gap is, as the guide and ADR-040 state it: on every row outside, the two
        // slopes at α → 0 are within 1.5 of the measurement's own standard errors — a weak test,
        // its errors being a seventh to a fifth of the slope — and most of the gap is in the
        // curvature, except on the short model at Mach 2.96, where most of it is the method's own
        // slope. Both halves are pinned, so neither claim can go stale quietly.
        let mut most_in_curvature = 0;
        let mut counterexamples = Vec::new();
        for configuration in fixture["configurations"].as_array().unwrap() {
            for row in configuration["rows"].as_array().unwrap() {
                if row["within_target"].as_bool().unwrap() {
                    continue;
                }
                let sigmas = row["zero_alpha_standard_errors"].as_f64().unwrap();
                assert!(
                    sigmas.abs() <= 1.5,
                    "{}: {sigmas} standard errors",
                    row["mach"]
                );
                // The curvature carries the rest of the gap, `1 − share`: on the short model at
                // Mach 1.8 it carries more than all of it, the slope at α → 0 reading low.
                let share = row["zero_alpha_share_of_gap"].as_f64().unwrap();
                if share < 0.5 {
                    most_in_curvature += 1;
                } else {
                    counterexamples.push((
                        configuration["id"].as_str().unwrap().to_owned(),
                        row["mach"].as_f64().unwrap(),
                        (share * 100.0).round(),
                    ));
                }
            }
        }
        assert_eq!(most_in_curvature, 5);
        assert_eq!(
            counterexamples,
            [("arcas-robin-short".to_owned(), 2.96, 77.0)],
            "the rows where the method's own slope carries most of the gap"
        );
        // The bullet's other half: both configurations within 15% at Mach 3.96 and 4.63. Match
        // the two Arcas references by name, so a reference added later can't be swept in, and
        // count the rows, so the half can't pass by checking nothing.
        let whole = read(&root, crate::aero_mach::FIXTURE).unwrap();
        let mut checked = 0;
        for reference in whole["references"].as_array().unwrap() {
            let id = reference["id"].as_str().unwrap();
            if !["arcas-robin-short", "arcas-robin-long"].contains(&id) {
                continue;
            }
            for row in reference["rows"].as_array().unwrap() {
                let mach = row["mach"].as_f64().unwrap();
                if mach < 3.9 {
                    continue;
                }
                let error = row["cn_alpha_error"].as_f64().unwrap();
                assert!(error.abs() <= TARGET, "{id} at Mach {mach}: {error}");
                checked += 1;
            }
        }
        assert_eq!(checked, 4, "Mach 3.96 and 4.63 on both configurations");
    }

    #[test]
    fn committed_fixture_is_current() {
        let root = crate::designs::root().unwrap();
        let committed = read(&root, FIXTURE).unwrap();
        let fresh = generate(&root).unwrap();
        assert!(
            crate::designs::same(&committed, &fresh),
            "{FIXTURE} differs from `cargo xtask aero`: {}",
            crate::designs::difference(&committed, &fresh).unwrap_or_default()
        );
    }
}
