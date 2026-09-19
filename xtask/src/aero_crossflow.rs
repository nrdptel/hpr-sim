//! The Arcas Robin's body faster than sound with M1.8e6's crossflow and boattail (ADR-037): hpr's
//! bodies through the flight's path against the measured body alone (NASA TN D-4014, fins off),
//! written by `cargo xtask aero` to `validation/fixtures/aero/arcas-robin-crossflow.json`.
//!
//! - **The body.** As M1.8e5's (`aero_gap`): the nose as the secant ogive fitted to the report's
//!   coordinates, the cylinder and the 15° boattail, without the lip, which the shock-expansion
//!   method can't take.
//! - **Four body models.** hpr's before M1.8e6 (Galejs's `K` = 1.1 and TN 3527 footnote 8's
//!   boattail), each of the two changes alone, and hpr's current (Jorgensen's crossflow and
//!   Washington and Pettis's boattail), so each change's share of the difference shows.
//! - **As the tunnel measures** (ADR-036): each model's `C_N` at the plotted angles from about −5°
//!   to +4°, fitted with a straight line as M1.8a fits the measurement; the slopes at `α → 0`
//!   beside them.
//! - **The high angles.** At each point the report plots above +4° (to about 21°, committed in
//!   `arcas-robin-high-alpha.json`), the measured `C_N` against the models', the crossflow Mach
//!   number `M sin α`, hpr's crossflow factor `η C_dn`, and the factor that would put hpr on the
//!   measured point with its potential-flow part as it is.
//! - **The boattail's share** at `α → 0`: footnote 8's, the current, and slender-body theory's.
//!
//! No targets: M1.8e8 carries M1.8e's 15% bullet. The fixture shows where hpr stands.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::supersonic_boattail::wp_slope;
use hpr_aero::{AeroModel, BodyLift, BodyModel, Flow, SupersonicBoattail};
use serde_json::{Value, json};

use crate::aero_body::{ARCAS_NOSE_R_IN, INCH, arcas_model, arcas_nose_ratio, flight_bodies};
use crate::aero_gap::{READING, slope_error};
use crate::aero_mach::{WIND_TUNNEL, hpr_force, slope};

pub const FIXTURE: &str = "validation/fixtures/aero/arcas-robin-crossflow.json";

/// The committed readings of the fins-off normal force above +4°.
pub const HIGH_ALPHA: &str = "validation/fixtures/aero/arcas-robin-high-alpha.json";

/// The committed readings of the fins-off pitching moment.
pub const MOMENT: &str = "validation/fixtures/aero/arcas-robin-fins-off-moment.json";

/// The angles, degrees, over which the body's centre of pressure is taken: both lines fitted
/// through the points with `|α|` up to this, the low angles the tunnel plotted (six or seven).
pub const CP_DEG: f64 = 4.5;

/// The body models compared, by name.
pub const MODELS: [(&str, BodyModel); 4] = [
    ("before", BodyModel::BEFORE_M1_8E6),
    (
        "crossflow_only",
        BodyModel::BEFORE_M1_8E6.with_body_lift(BodyLift::JORGENSEN),
    ),
    (
        "boattail_only",
        BodyModel::BEFORE_M1_8E6.with_supersonic_boattail(SupersonicBoattail::WashingtonPettis),
    ),
    ("current", BodyModel::CURRENT),
];

/// The boattail's index among the body's components: nose, cylinder, boattail.
const BOATTAIL: usize = 2;

fn read(root: &Path, name: &str) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))
}

fn points(curve: &Value, key: &str) -> Result<Vec<(f64, f64)>, String> {
    curve[key]
        .as_array()
        .ok_or(format!("a curve without `{key}`"))?
        .iter()
        .map(|p| Some((p[0].as_f64()?, p[1].as_f64()?)))
        .collect::<Option<_>>()
        .ok_or_else(|| "an unreadable point".to_string())
}

/// The fixture, from the committed designs and wind-tunnel references.
pub fn generate(root: &Path) -> Result<Value, String> {
    let reference = read(root, WIND_TUNNEL)?;
    let high = read(root, HIGH_ALPHA)?;
    let moment = read(root, MOMENT)?;
    let (ratio, _) = arcas_nose_ratio()?;
    let diameter_m = 2.0 * ARCAS_NOSE_R_IN[8] * INCH;
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
        let design = configuration["design"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no design"))?;
        let models: Vec<(&str, AeroModel)> = MODELS
            .iter()
            .map(|(name, body_model)| {
                arcas_model(root, design, Some(ratio), true, *body_model).map(|m| (*name, m))
            })
            .collect::<Result<_, _>>()?;
        let current = &models[3].1;
        let scale = current.reference_area_m2() / area;
        let planform: f64 = scale
            * current
                .bodies()
                .iter()
                .map(|b| b.planform_ratio)
                .sum::<f64>();
        let moment_center_m = moment["moment_center_in"][id]
            .as_f64()
            .ok_or(format!("{MOMENT}: no moment center for {id}"))?
            * INCH;
        let moment_curves = moment["configurations"]
            .as_array()
            .and_then(|c| c.iter().find(|c| c["id"] == id))
            .and_then(|c| c["cm_fins_off"].as_array())
            .ok_or(format!("{MOMENT} has no curves for {id}"))?;
        let high_curves = high["configurations"]
            .as_array()
            .and_then(|c| c.iter().find(|c| c["id"] == id))
            .and_then(|c| c["cn_fins_off"].as_array())
            .ok_or(format!("{HIGH_ALPHA} has no curves for {id}"))?;
        let mut rows = Vec::new();
        for curve in configuration["cn_alpha_fins_off"]
            .as_array()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no fins-off curves"))?
        {
            let mach = curve["mach"].as_f64().ok_or("a curve without `mach`")?;
            if mach < 1.5 {
                continue;
            }
            let low = points(curve, "alpha_deg_c_n")?;
            let alphas: Vec<f64> = low.iter().map(|p| p.0.to_radians()).collect();
            let measured: Vec<f64> = low.iter().map(|p| p.1).collect();
            let fitted = slope(&alphas, &measured);
            let mut hpr = serde_json::Map::new();
            let mut zero = serde_json::Map::new();
            for (name, model) in &models {
                let c_n: Vec<f64> = low
                    .iter()
                    .map(|p| hpr_force(model, mach, p.0, true).map(|f| scale * f.0))
                    .collect::<Result<_, _>>()?;
                let hpr_fitted = slope(&alphas, &c_n);
                let (_, at_zero, _) = flight_bodies(model, mach, area)?;
                if !(hpr_fitted.is_finite() && at_zero.is_finite()) {
                    return Err(format!("{id} at Mach {mach}: {name} isn't a number"));
                }
                hpr.insert(
                    name.to_string(),
                    json!({
                        "fitted_c_n_alpha": hpr_fitted,
                        "fitted_c_n_alpha_error": hpr_fitted / fitted - 1.0,
                    }),
                );
                zero.insert(name.to_string(), json!(at_zero));
            }
            // The body's centre of pressure at low angles, calibers from the nose tip: the
            // measured from the fitted slopes of C_m about the moment center and of C_N (nose-up
            // C_m positive), hpr's from its C_N and moment about the tip at the same angles.
            let m_points = moment_curves
                .iter()
                .find(|c| c["mach"].as_f64() == Some(mach))
                .map(|c| points(c, "alpha_deg_c_m"))
                .ok_or(format!("{MOMENT}: no curve for {id} at Mach {mach}"))??;
            let inner = |p: &&(f64, f64)| p.0.abs() <= CP_DEG;
            let (n_a, n_c): (Vec<f64>, Vec<f64>) = low
                .iter()
                .filter(inner)
                .map(|p| (p.0.to_radians(), p.1))
                .unzip();
            let (m_a, m_c): (Vec<f64>, Vec<f64>) = m_points
                .iter()
                .filter(inner)
                .map(|p| (p.0.to_radians(), p.1))
                .unzip();
            let measured_cp = moment_center_m / diameter_m - slope(&m_a, &m_c) / slope(&n_a, &n_c);
            let mut cp = serde_json::Map::new();
            for (name, model) in &models {
                let forces: Vec<(f64, f64)> = low
                    .iter()
                    .filter(inner)
                    .map(|p| hpr_force(model, mach, p.0, true))
                    .collect::<Result<_, _>>()?;
                let (c_n, moments): (Vec<f64>, Vec<f64>) = forces.into_iter().unzip();
                cp.insert(
                    name.to_string(),
                    json!(slope(&n_a, &moments) / slope(&n_a, &c_n) / diameter_m),
                );
            }
            // The boattail's share at alpha -> 0, per radian on the body's cross-section.
            let share = |model: &AeroModel| -> Result<f64, String> {
                let s = model
                    .supersonic_body()
                    .ok_or(format!("{id}: the method doesn't cover the body"))?;
                let (slope, _) = s
                    .share(BOATTAIL, mach)
                    .ok_or(format!("{id} at Mach {mach}: no boattail share"))?;
                Ok(scale * slope)
            };
            let slender = scale * current.bodies()[BOATTAIL].slope_per_rad;
            let geometry = &current.bodies()[BOATTAIL].geometry;
            let fore_radius_m = (geometry.fore_area_m2 / PI).sqrt();
            let increment = wp_slope(
                mach,
                fore_radius_m,
                (geometry.aft_area_m2 / PI).sqrt(),
                geometry.length_m,
            )
            .map_err(|e| format!("{id} at Mach {mach}: {e}"))?
                * geometry.fore_area_m2
                / area;
            // The high angles.
            let mut high_points = Vec::new();
            if let Some(high_curve) = high_curves
                .iter()
                .find(|c| c["mach"].as_f64() == Some(mach))
            {
                for (alpha_deg, c_n) in points(high_curve, "alpha_deg_c_n")? {
                    let alpha = alpha_deg.to_radians();
                    let flow = Flow::new(mach, alpha, 0.0);
                    let factor = current.body_lift_factor(&flow);
                    let at = |model: &AeroModel| -> Result<f64, String> {
                        Ok(scale * hpr_force(model, mach, alpha_deg, true)?.0)
                    };
                    let (before, now) = (at(&models[0].1)?, at(current)?);
                    let lift = planform * alpha.sin().powi(2);
                    // The factor that would put hpr on the measured point, its potential-flow
                    // part as it is.
                    let needed = (c_n - (now - factor * lift)) / lift;
                    high_points.push(json!({
                        "alpha_deg": alpha_deg,
                        "measured_c_n": c_n,
                        "before_c_n": before,
                        "current_c_n": now,
                        "before_error": before / c_n - 1.0,
                        "current_error": now / c_n - 1.0,
                        "crossflow_mach": mach * alpha.sin(),
                        "current_factor": factor,
                        "needed_factor": needed,
                    }));
                }
            }
            rows.push(json!({
                "mach": mach,
                "measured": {
                    "fitted_c_n_alpha": fitted,
                    "fitted_standard_error": slope_error(&alphas, reading),
                    "points": low.len(),
                },
                "hpr": Value::Object(hpr),
                "zero_alpha_c_n_alpha": Value::Object(zero),
                "boattail": {
                    "footnote_8": share(&models[0].1)?,
                    "current": share(current)?,
                    "washington_pettis_increment": increment,
                    "slender_body": slender,
                },
                "high_alpha": high_points,
                "centre_of_pressure_calibers": {
                    "measured": measured_cp,
                    "points": [n_a.len(), m_a.len()],
                    "hpr": Value::Object(cp),
                },
            }));
        }
        configurations.push(json!({
            "id": id,
            "design": design,
            "fineness": current.fineness(),
            "planform_per_rad2": planform,
            "rows": rows,
        }));
    }
    Ok(json!({
        "generator": "cargo xtask aero (xtask/src/aero_crossflow.rs)",
        "note": "The Arcas Robin's body alone faster than sound (NASA TN D-4014, fins off) against \
                 hpr's bodies through the flight's path: the nose as the secant ogive fitted to the \
                 report's coordinates, the cylinder and the 15 deg boattail, without the lip. Four \
                 body models: before M1.8e6 (Galejs's K = 1.1, TN 3527 footnote 8), the crossflow \
                 change alone (Jorgensen, NASA TR R-474), the boattail change alone (Washington and \
                 Pettis, RD-TM-68-5) and both (current). Slopes per radian on the body's \
                 cross-section, fitted at the tunnel's plotted angles as M1.8a fits the \
                 measurement (ADR-036); the slopes at alpha -> 0 beside them. High angles from \
                 arcas-robin-high-alpha.json. No targets.",
        "configurations": configurations,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed fixture is what the generator writes; CI has no `refs/`, so this test keeps
    /// it current there.
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

    /// A signed percentage as the guide writes it: one decimal, a Unicode minus.
    fn pct(x: f64) -> String {
        format!("{:+.1}%", 100.0 * x).replace('-', "−")
    }

    /// `docs/physics/aero.md` quotes the fixture: its like-for-like table cell by cell, the
    /// boattail's shares, and the ranges over the high angles.
    #[test]
    fn the_guide_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
        let fixture = read(&root, FIXTURE).unwrap();
        let guide = fs::read_to_string(root.join("docs/physics/aero.md")).unwrap();
        let joined = guide.split_whitespace().collect::<Vec<_>>().join(" ");
        let (mut fitted, mut high) = (Vec::new(), Vec::new());
        let mut bands = [[f64::MAX, f64::MIN, f64::MAX, f64::MIN, 0.0]; 3];
        let (mut within, mut count) = (0, 0);
        let (mut shares, mut increments) = (Vec::new(), Vec::new());
        for configuration in fixture["configurations"].as_array().unwrap() {
            let short = configuration["id"] == "arcas-robin-short";
            let model = if short { "short" } else { "long" };
            for row in configuration["rows"].as_array().unwrap() {
                let f = |pointer: &str| row.pointer(pointer).unwrap().as_f64().unwrap();
                let cell = |name: &str| {
                    format!(
                        "{:.2} ({})",
                        f(&format!("/hpr/{name}/fitted_c_n_alpha")),
                        pct(f(&format!("/hpr/{name}/fitted_c_n_alpha_error")))
                    )
                };
                let line = format!(
                    "| {model} | {} | {:.2} ± {:.2} | {} | {} | {} | {} | {:.2} |",
                    row["mach"].as_f64().unwrap(),
                    f("/measured/fitted_c_n_alpha"),
                    f("/measured/fitted_standard_error"),
                    cell("before"),
                    cell("crossflow_only"),
                    cell("boattail_only"),
                    cell("current"),
                    f("/zero_alpha_c_n_alpha/current"),
                );
                assert!(
                    guide.contains(&line),
                    "aero.md doesn't have the row `{line}`"
                );
                fitted.push(f("/hpr/current/fitted_c_n_alpha_error"));
                if short {
                    shares.push(-f("/boattail/current"));
                    increments.push(f("/boattail/washington_pettis_increment"));
                }
                for point in row["high_alpha"].as_array().unwrap() {
                    let p = |key: &str| point[key].as_f64().unwrap();
                    let band = match p("crossflow_mach") {
                        m if m < 0.45 => 0,
                        m if m < 0.95 => 1,
                        _ => 2,
                    };
                    let b = &mut bands[band];
                    b[0] = b[0].min(p("current_error"));
                    b[1] = b[1].max(p("current_error"));
                    b[2] = b[2].min(p("before_error"));
                    b[3] = b[3].max(p("before_error"));
                    b[4] += 1.0;
                    count += 1;
                    within += usize::from(p("current_error").abs() <= 0.15);
                    high.push(p("alpha_deg"));
                }
            }
        }
        let range = |xs: &[f64]| {
            (
                xs.iter().cloned().fold(f64::MAX, f64::min),
                xs.iter().cloned().fold(f64::MIN, f64::max),
            )
        };
        let (lo, hi) = range(&fitted);
        assert!(joined.contains(&format!("reads {} to {}", pct(lo), pct(hi))));
        assert!(joined.contains(&format!("At its {count} angles from")));
        assert!(joined.contains(&format!("{within} are within 15%")));
        let (a, b) = range(&high);
        assert!(joined.contains(&format!("from {a:.1}° to {b:.1}°")));
        for (band, name) in bands
            .iter()
            .zip(["under 0.45", "0.45 to 0.95", "0.95 and over"])
        {
            let line = format!(
                "| {name} | {} | {} to {} | {} to {} |",
                band[4],
                pct(band[0]),
                pct(band[1]),
                pct(band[2]),
                pct(band[3])
            );
            assert!(
                guide.contains(&line),
                "aero.md doesn't have the row `{line}`"
            );
        }
        let (a, b) = range(&shares);
        assert!(joined.contains(&format!("{a:.2} to {b:.2} per radian off")));
        // The body's centre of pressure, row by row, and the ranges the text quotes.
        let signed = |x: f64| format!("{x:+.2}").replace('-', "−");
        let mut errors = [[f64::MAX, f64::MIN]; 4];
        for configuration in fixture["configurations"].as_array().unwrap() {
            let short = configuration["id"] == "arcas-robin-short";
            for row in configuration["rows"].as_array().unwrap() {
                let cp = &row["centre_of_pressure_calibers"];
                let measured = cp["measured"].as_f64().unwrap();
                let (before, now) = (
                    cp["hpr"]["before"].as_f64().unwrap(),
                    cp["hpr"]["current"].as_f64().unwrap(),
                );
                let line = format!(
                    "| {} | {} | {measured:.2} | {before:.2} ({}) | {now:.2} ({}) |",
                    if short { "short" } else { "long" },
                    row["mach"].as_f64().unwrap(),
                    signed(before - measured),
                    signed(now - measured),
                );
                assert!(
                    guide.contains(&line),
                    "aero.md doesn't have the row `{line}`"
                );
                let k = if short { 0 } else { 1 };
                for (j, e) in [(k, before - measured), (k + 2, now - measured)] {
                    errors[j] = [errors[j][0].min(e), errors[j][1].max(e)];
                }
            }
        }
        let quote = format!(
            "hpr put it {:.2} to {:.2} calibres aft of the tunnel's on the short model and {:.2} \
             to {:.2} on the long; now {} to {} and {} to {}",
            errors[0][0],
            errors[0][1],
            errors[1][0],
            errors[1][1],
            signed(errors[2][0]),
            signed(errors[2][1]),
            signed(errors[3][0]),
            signed(errors[3][1]),
        );
        assert!(joined.contains(&quote), "aero.md doesn't say `{quote}`");
        // The guide's table of Washington and Pettis's increments on the short model.
        for increment in &increments {
            assert!(
                joined.contains(&format!("{increment:.3}").replace('-', "−")),
                "aero.md doesn't quote {increment:.3}"
            );
        }
    }

    /// The current model is the library's default, and the first is M1.8e5's.
    #[test]
    fn the_models_are_the_ones_named() {
        assert_eq!(MODELS[3].1, BodyModel::default());
        assert_eq!(MODELS[0].1, BodyModel::BEFORE_M1_8E6);
    }

    /// `wind_response.py` flies hpr's body lift in RocketPy (ADR-026, ADR-037) from its own copy
    /// of Jorgensen's tables: they are the library's.
    #[test]
    fn the_wind_oracle_uses_the_librarys_tables() {
        use hpr_aero::crossflow as c;
        let root = crate::designs::root().unwrap();
        let script =
            fs::read_to_string(root.join("validation/oracles/rocketpy/wind_response.py")).unwrap();
        let list = |name: &str| -> Vec<f64> {
            let start = script
                .find(&format!("\n{name} = ["))
                .unwrap_or_else(|| panic!("no {name}"));
            let body = &script[start + name.len() + 5..];
            body[..body.find(']').unwrap()]
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(|v| v.parse().unwrap())
                .collect()
        };
        assert_eq!(list("CROSSFLOW_DRAG_MACHS"), c::CROSSFLOW_DRAG_MACHS);
        assert_eq!(list("CROSSFLOW_DRAG"), c::CROSSFLOW_DRAG);
        assert_eq!(list("ETA_MACHS"), c::ETA_MACHS);
        assert_eq!(list("ETA_BY_CROSSFLOW_MACH"), c::ETA_BY_CROSSFLOW_MACH);
        assert_eq!(list("ETA_FINENESS"), c::ETA_FINENESS);
        assert_eq!(list("ETA_BY_FINENESS"), c::ETA_BY_FINENESS);
        assert!(script.contains(&format!("\nETA_REFERENCE = {}\n", c::ETA_REFERENCE)));
        assert!(script.contains(&format!("\nBODY_LIFT_K = {}\n", hpr_aero::BODY_LIFT_K)));
    }
}
