//! Blunt and vertical nose tips faster than sound (M1.8e7, ADR-038): hpr's Newtonian cap ahead of
//! the second-order shock-expansion method against its two checks, written by `cargo xtask aero`
//! to `validation/fixtures/aero/blunt-tips.json`.
//!
//! - **TN D-4865's sphere-cone** (model 1: a 0.175-diameter nose radius on an 11.5° cone; the
//!   readings are committed in `tn-d-4865-sphere-cone.json`): hpr's `C_Nα` and centre of pressure
//!   at `α → 0` against the report's measured `C_N` and `C_m`, each fitted with a straight line
//!   over its plotted angles (0° to 12°), with the report's own method beside them.
//! - **The Arcas Robin's committed nose** (a power series with a vertical tip) on the cylinder
//!   and the boattail, the lip left off, through the flight's path (hpr's current body model),
//!   against TN D-4014's body alone as `arcas-robin-crossflow.json` fits it, beside the secant
//!   ogive fitted to the report's coordinates, which M1.8e6 compared.
//!
//! No targets: the fixture shows where hpr stands.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::blunt_tip::handover_angle_rad;
use hpr_aero::shock_expansion::{
    BodySegment, DEFAULT_ELEMENTS_PER_CURVE, HandoverStart, ShockExpansionBody,
};
use hpr_aero::{AeroModel, BodyModel};
use hpr_design::{NoseShape, Part, Profile, Rocket};
use serde_json::{Value, json};

use crate::aero_body::{
    ARCAS_NOSE_R_IN, INCH, arcas_model, arcas_model_without_lip, arcas_nose_ratio, flight_bodies,
};
use crate::aero_crossflow::{CP_DEG, MOMENT};
use crate::aero_mach::{WIND_TUNNEL, hpr_force, slope};

pub const FIXTURE: &str = "validation/fixtures/aero/blunt-tips.json";

/// The committed readings of TN D-4865's model 1.
pub const READINGS: &str = "validation/fixtures/aero/tn-d-4865-sphere-cone.json";

fn read(root: &Path, name: &str) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))
}

/// `[α°, value]` pairs as radians and values.
fn pairs(value: &Value, key: &str) -> Result<(Vec<f64>, Vec<f64>), String> {
    value[key]
        .as_array()
        .ok_or(format!("no `{key}`"))?
        .iter()
        .map(|p| match (p[0].as_f64(), p[1].as_f64()) {
            (Some(a), Some(c)) => Ok((a.to_radians(), c)),
            _ => Err(format!("a malformed `{key}` point")),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|p| p.into_iter().unzip())
}

pub fn generate(root: &Path) -> Result<Value, String> {
    Ok(json!({
        "generator": "cargo xtask aero (xtask/src/aero_blunt.rs)",
        "note": "Blunt and vertical nose tips faster than sound (M1.8e7): TN D-4865's Newtonian cap \
                 and handover ahead of TN 3527's method, which starts behind it from the tangent \
                 cone's flow. sphere_cone: TN D-4865's model 1 (readings in \
                 tn-d-4865-sphere-cone.json), hpr's C_N_alpha on the base area and centre of \
                 pressure from the nose tip in base diameters at alpha -> 0, against the measured \
                 C_N and C_m fitted with a straight line (with an intercept) over the plotted \
                 angles 0 to 12 deg, and the report's own method fitted the same way through the \
                 origin's zero. arcas_robin: the committed design's power-series nose, cylinder and \
                 boattail, the lip left off, through the flight's path with hpr's current body \
                 model, fitted at the tunnel's plotted angles as arcas-robin-crossflow.json fits \
                 them (per radian on the body's cross-section), beside the fitted secant ogive's \
                 (M1.8e6's comparison) and the slopes at alpha -> 0. Errors are hpr's over the \
                 measured, minus 1; centre-of-pressure errors hpr's minus the measured, calibers. \
                 No targets.",
        "sphere_cone": sphere_cone_rows(root)?,
        "arcas_robin": arcas_robin(root)?,
        "starts": starts(root)?,
    }))
}

/// The Mach numbers the two starts are compared at: the tunnel's, Mach 3.5 between them, and 5.
pub const START_MACHS: [f64; 8] = [1.5, 1.8, 2.3, 2.96, 3.5, 3.96, 4.63, 5.0];

/// The two starts behind the cap ([`HandoverStart`]) on the Arcas Robin's committed nose and the
/// short model's cylinder at `α → 0`, per radian on its cross-section, with the default elements
/// and four times as many: the slope and how many elements the march reduces (issue #81), or why
/// it fails.
fn starts(root: &Path) -> Result<Value, String> {
    let design = "wind-tunnel-arcas-robin-short.json";
    let text = fs::read_to_string(root.join("validation/designs").join(design))
        .map_err(|e| format!("{design}: {e}"))?;
    let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{design}: {e}"))?;
    let stage = rocket.stages.first().ok_or(format!("{design}: no stage"))?;
    let (Some(Part::NoseCone(nose)), Some(Part::BodyTube(tube))) = (
        stage.components.first().map(|c| &c.part),
        stage.components.get(1).map(|c| &c.part),
    ) else {
        return Err(format!("{design}: not a nose and a body tube"));
    };
    let profile = nose.profile().map_err(|e| e.to_string())?;
    let area = PI * nose.base_radius_m * nose.base_radius_m;
    let mut rows = Vec::new();
    for mach in START_MACHS {
        let mut row = serde_json::Map::new();
        row.insert("mach".into(), json!(mach));
        for (name, start) in [
            ("tangent_cone", HandoverStart::TangentCone),
            ("newtonian", HandoverStart::Newtonian),
        ] {
            let mut by_count = serde_json::Map::new();
            for elements in [DEFAULT_ELEMENTS_PER_CURVE, 4 * DEFAULT_ELEMENTS_PER_CURVE] {
                let body = ShockExpansionBody::new(
                    &[
                        BodySegment::Profile { profile },
                        BodySegment::Cylinder {
                            length_m: tube.length_m,
                            radius_m: tube.outer_radius_m,
                        },
                    ],
                    elements,
                )
                .map_err(|e| e.to_string())?
                .with_handover_start(start);
                let value = match (body.slope(mach, area), body.reduced_elements(mach)) {
                    (Ok(slope), Ok(reduced)) => json!({
                        "c_n_alpha_per_rad": slope.slope_per_rad,
                        "cp_calibers": slope.centre_of_pressure_m / (2.0 * nose.base_radius_m),
                        "reduced_elements": reduced,
                    }),
                    (Err(e), _) | (_, Err(e)) => json!({ "fails": e.to_string() }),
                };
                by_count.insert(elements.to_string(), value);
            }
            row.insert(name.into(), Value::Object(by_count));
        }
        rows.push(Value::Object(row));
    }
    Ok(json!({
        "design": design,
        "body": "the committed nose and the short model's cylinder, nothing aft",
        "rows": rows,
    }))
}

/// TN D-4865's model 1 as hpr builds it, one base diameter across: the sphere's cap to its
/// tangent with the cone, and the cone to the base.
pub fn sphere_cone_body(readings: &Value) -> Result<ShockExpansionBody, String> {
    let model = &readings["model"];
    let radius = model["nose_radius_over_base_diameter"]
        .as_f64()
        .ok_or("no nose radius")?;
    let half_angle = model["cone_half_angle_deg"]
        .as_f64()
        .ok_or("no cone half-angle")?
        .to_radians();
    let tangent_x = radius * (1.0 - half_angle.sin());
    let tangent_r = radius * half_angle.cos();
    let cone = Profile::transition(
        NoseShape::Conical {},
        (0.5 - tangent_r) / half_angle.tan(),
        tangent_r,
        0.5,
        false,
    )
    .map_err(|e| e.to_string())?;
    ShockExpansionBody::new(
        &[
            BodySegment::SphericalCap {
                radius_m: radius,
                length_m: tangent_x,
            },
            BodySegment::Profile { profile: cone },
        ],
        DEFAULT_ELEMENTS_PER_CURVE,
    )
    .map_err(|e| e.to_string())
}

fn sphere_cone_rows(root: &Path) -> Result<Value, String> {
    let readings = read(root, READINGS)?;
    let body = sphere_cone_body(&readings)?;
    let length = readings["reference"]["length_over_base_diameter"]
        .as_f64()
        .ok_or(format!("{READINGS}: no reference length"))?;
    let area = 0.25 * PI;
    let mut rows = Vec::new();
    for row in readings["rows"]
        .as_array()
        .ok_or(format!("{READINGS}: no rows"))?
    {
        let mach = row["mach"].as_f64().ok_or("a row without `mach`")?;
        let (n_a, n_c) = pairs(row, "alpha_deg_c_n")?;
        let (m_a, m_c) = pairs(row, "alpha_deg_c_m")?;
        let measured = slope(&n_a, &n_c);
        // C_m about the nose tip on the body length, nose-up positive: the centre of pressure
        // sits `−C_m l / C_N` aft of the tip.
        let measured_cp = -slope(&m_a, &m_c) * length / measured;
        // The report's method passes through zero at zero angle.
        let (mut t_a, mut t_n) = pairs(row, "report_theory_alpha_deg_c_n")?;
        let (_, mut t_m) = pairs(row, "report_theory_alpha_deg_c_m")?;
        t_a.insert(0, 0.0);
        t_n.insert(0, 0.0);
        t_m.insert(0, 0.0);
        let theory = slope(&t_a, &t_n);
        let theory_cp = -slope(&t_a, &t_m) * length / theory;
        let hpr = body
            .slope(mach, area)
            .map_err(|e| format!("model 1 at Mach {mach}: {e}"))?;
        let newtonian = body
            .clone()
            .with_handover_start(HandoverStart::Newtonian)
            .slope(mach, area)
            .map_err(|e| format!("model 1 at Mach {mach}, the Newtonian start: {e}"))?;
        let handover_x = body
            .handover_m(mach)
            .map_err(|e| e.to_string())?
            .ok_or("model 1 has no handover")?;
        let handover_deg = handover_angle_rad(mach)
            .map_err(|e| e.to_string())?
            .to_degrees();
        rows.push(json!({
            "mach": mach,
            "measured": { "c_n_alpha_per_rad": measured, "cp_calibers": measured_cp },
            "report_method": {
                "c_n_alpha_per_rad": theory,
                "cp_calibers": theory_cp,
                "c_n_alpha_error": theory / measured - 1.0,
            },
            "hpr": {
                "c_n_alpha_per_rad": hpr.slope_per_rad,
                "cp_calibers": hpr.centre_of_pressure_m,
                "c_n_alpha_error": hpr.slope_per_rad / measured - 1.0,
                "cp_error_calibers": hpr.centre_of_pressure_m - measured_cp,
                "handover_deg": handover_deg,
                "handover_calibers": handover_x,
            },
            "hpr_newtonian_start": {
                "c_n_alpha_per_rad": newtonian.slope_per_rad,
                "cp_calibers": newtonian.centre_of_pressure_m,
                "c_n_alpha_error": newtonian.slope_per_rad / measured - 1.0,
            },
        }));
    }
    Ok(json!(rows))
}

/// Where the committed nose's cap hands over at `mach`, as its radius over the base's.
fn handover_radius_ratio(root: &Path, design: &str, mach: f64) -> Result<f64, String> {
    let text = fs::read_to_string(root.join("validation/designs").join(design))
        .map_err(|e| format!("{design}: {e}"))?;
    let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{design}: {e}"))?;
    let Some(Part::NoseCone(nose)) = rocket
        .stages
        .first()
        .and_then(|s| s.components.first())
        .map(|c| &c.part)
    else {
        return Err(format!("{design}: its first component isn't a nose cone"));
    };
    let profile = nose.profile().map_err(|e| e.to_string())?;
    let body = ShockExpansionBody::new(&[BodySegment::Profile { profile }], 10)
        .map_err(|e| e.to_string())?;
    let x = body
        .handover_m(mach)
        .map_err(|e| e.to_string())?
        .ok_or(format!("{design}: its nose has no handover"))?;
    Ok(profile.radius_m(x) / nose.base_radius_m)
}

fn arcas_robin(root: &Path) -> Result<Value, String> {
    let reference = read(root, WIND_TUNNEL)?;
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
        let design = configuration["design"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no design"))?;
        let committed = arcas_model_without_lip(root, design, BodyModel::CURRENT)?;
        let fitted = arcas_model(root, design, Some(ratio), true, BodyModel::CURRENT)?;
        let models: [(&str, &AeroModel); 2] = [("committed", &committed), ("fitted", &fitted)];
        if committed.supersonic_body().is_none() {
            return Err(format!("{id}: the committed nose doesn't fly the method"));
        }
        let moment_center_m = moment["moment_center_in"][id]
            .as_f64()
            .ok_or(format!("{MOMENT}: no moment center for {id}"))?
            * INCH;
        let moment_curves = moment["configurations"]
            .as_array()
            .and_then(|c| c.iter().find(|c| c["id"] == id))
            .and_then(|c| c["cm_fins_off"].as_array())
            .ok_or(format!("{MOMENT} has no curves for {id}"))?;
        let mut rows = Vec::new();
        for curve in configuration["cn_alpha_fins_off"]
            .as_array()
            .ok_or(format!("{WIND_TUNNEL}: {id} has no fins-off curves"))?
        {
            let mach = curve["mach"].as_f64().ok_or("a curve without `mach`")?;
            if mach < 1.5 {
                continue;
            }
            let low: Vec<(f64, f64)> = curve["alpha_deg_c_n"]
                .as_array()
                .ok_or("a curve without `alpha_deg_c_n`")?
                .iter()
                .map(|p| {
                    (
                        p[0].as_f64().unwrap_or(f64::NAN),
                        p[1].as_f64().unwrap_or(f64::NAN),
                    )
                })
                .collect();
            if low.iter().any(|p| !(p.0.is_finite() && p.1.is_finite())) {
                return Err(format!(
                    "{WIND_TUNNEL}: {id} at Mach {mach} has a bad point"
                ));
            }
            let alphas: Vec<f64> = low.iter().map(|p| p.0.to_radians()).collect();
            let measured: Vec<f64> = low.iter().map(|p| p.1).collect();
            let measured_slope = slope(&alphas, &measured);
            // The body's centre of pressure at low angles, as arcas-robin-crossflow.json takes it.
            let m_points: Vec<(f64, f64)> = moment_curves
                .iter()
                .find(|c| c["mach"].as_f64() == Some(mach))
                .and_then(|c| c["alpha_deg_c_m"].as_array())
                .ok_or(format!("{MOMENT}: no curve for {id} at Mach {mach}"))?
                .iter()
                .map(|p| {
                    (
                        p[0].as_f64().unwrap_or(f64::NAN),
                        p[1].as_f64().unwrap_or(f64::NAN),
                    )
                })
                .collect();
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
            let mut hpr = serde_json::Map::new();
            for (name, model) in models {
                let scale = model.reference_area_m2() / area;
                let c_n: Vec<f64> = low
                    .iter()
                    .map(|p| hpr_force(model, mach, p.0, true).map(|f| scale * f.0))
                    .collect::<Result<_, _>>()?;
                let fitted_slope = slope(&alphas, &c_n);
                let (_, at_zero, _) = flight_bodies(model, mach, area)?;
                let forces: Vec<(f64, f64)> = low
                    .iter()
                    .filter(inner)
                    .map(|p| hpr_force(model, mach, p.0, true))
                    .collect::<Result<_, _>>()?;
                let (normal, moments): (Vec<f64>, Vec<f64>) = forces.into_iter().unzip();
                let cp = slope(&n_a, &moments) / slope(&n_a, &normal) / diameter_m;
                if !(fitted_slope.is_finite() && at_zero.is_finite() && cp.is_finite()) {
                    return Err(format!("{id} at Mach {mach}: {name} isn't a number"));
                }
                hpr.insert(
                    name.to_string(),
                    json!({
                        "fitted_c_n_alpha": fitted_slope,
                        "fitted_c_n_alpha_error": fitted_slope / measured_slope - 1.0,
                        "zero_alpha_c_n_alpha": at_zero,
                        "cp_calibers": cp,
                        "cp_error_calibers": cp - measured_cp,
                    }),
                );
            }
            rows.push(json!({
                "mach": mach,
                "measured": {
                    "fitted_c_n_alpha": measured_slope,
                    "cp_calibers": measured_cp,
                    "points": [low.len(), n_a.len()],
                },
                "hpr": hpr,
                "handover_radius_ratio": handover_radius_ratio(root, design, mach)?,
            }));
        }
        configurations.push(json!({
            "id": id,
            "design": design,
            "rows": rows,
        }));
    }
    Ok(json!(configurations))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// A number as the guide writes it: `digits` decimals, a Unicode minus.
    fn num(x: f64, digits: usize) -> String {
        format!("{x:.digits$}").replace('-', "−")
    }

    /// A signed percentage as the guide writes it.
    fn pct(x: f64) -> String {
        format!("{:+.1}%", 100.0 * x).replace('-', "−")
    }

    fn f(value: &Value, pointer: &str) -> f64 {
        value
            .pointer(pointer)
            .and_then(Value::as_f64)
            .unwrap_or_else(|| panic!("no {pointer}"))
    }

    /// `docs/physics/aero.md`'s *Blunt tips* holds the fixture's three tables, cell by cell.
    #[test]
    fn the_guide_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
        let fixture = read(&root, FIXTURE).unwrap();
        let guide = fs::read_to_string(root.join("docs/physics/aero.md")).unwrap();
        let mut rows = Vec::new();
        for r in fixture["sphere_cone"].as_array().unwrap() {
            rows.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                f(r, "/mach"),
                num(f(r, "/measured/c_n_alpha_per_rad"), 3),
                num(f(r, "/report_method/c_n_alpha_per_rad"), 3),
                num(f(r, "/hpr/c_n_alpha_per_rad"), 3),
                pct(f(r, "/hpr/c_n_alpha_error")),
                num(f(r, "/hpr_newtonian_start/c_n_alpha_per_rad"), 3),
                pct(f(r, "/hpr_newtonian_start/c_n_alpha_error")),
                num(f(r, "/measured/cp_calibers"), 2),
                num(f(r, "/hpr/cp_calibers"), 2),
            ));
        }
        for c in fixture["arcas_robin"].as_array().unwrap() {
            let model = c["id"].as_str().unwrap().rsplit('-').next().unwrap();
            for r in c["rows"].as_array().unwrap() {
                rows.push(format!(
                    "| {model} | {} | {} | {} | {} | {} | {} | {} |",
                    f(r, "/mach"),
                    num(f(r, "/measured/fitted_c_n_alpha"), 3),
                    num(f(r, "/hpr/committed/fitted_c_n_alpha"), 3),
                    pct(f(r, "/hpr/committed/fitted_c_n_alpha_error")),
                    num(f(r, "/hpr/fitted/fitted_c_n_alpha"), 3),
                    pct(f(r, "/hpr/fitted/fitted_c_n_alpha_error")),
                    num(f(r, "/handover_radius_ratio"), 3),
                ));
            }
        }
        let cell = |v: &Value| match v.get("c_n_alpha_per_rad").and_then(Value::as_f64) {
            None => "fails".to_owned(),
            Some(slope) => {
                let reduced = v["reduced_elements"].as_u64().unwrap();
                if reduced > 0 {
                    format!("{} ({reduced} reduced)", num(slope, 3))
                } else {
                    num(slope, 3)
                }
            }
        };
        for r in fixture["starts"]["rows"].as_array().unwrap() {
            rows.push(format!(
                "| {} | {} | {} | {} | {} |",
                f(r, "/mach"),
                cell(&r["tangent_cone"]["10"]),
                cell(&r["tangent_cone"]["40"]),
                cell(&r["newtonian"]["10"]),
                cell(&r["newtonian"]["40"]),
            ));
        }
        // The report's own method's range, which the section's opening quotes.
        let errors: Vec<f64> = fixture["sphere_cone"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| f(r, "/report_method/c_n_alpha_error"))
            .collect();
        let (low, high) = errors
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), &e| {
                (a.min(e), b.max(e))
            });
        rows.push(format!("{} to {}", pct(low), pct(high)));
        assert_eq!(rows.len(), 6 + 11 + START_MACHS.len() + 1);
        for row in rows {
            assert!(guide.contains(&row), "aero.md doesn't have the row `{row}`");
        }
    }
}
