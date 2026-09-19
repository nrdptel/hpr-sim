//! The body faster than sound (M1.8e1): hpr's second-order shock-expansion `C_Nα` and centre of
//! pressure (`hpr_aero::shock_expansion`) against two references, written by `cargo xtask aero`
//! to `validation/fixtures/aero/shock-expansion.json`.
//!
//! - **NACA TN 3527's Tables I and II** (Syvertson and Dennis 1956), transcribed into
//!   `validation/fixtures/aero/tn3527-bodies.json`: cone- and tangent-ogive-cylinders of nose
//!   fineness 3, 5 and 7 with cylinders of 0 to 10 calibers, Mach 3 to 6.28. Each row gives the
//!   report's own second-order values and its measurements at `α = 0`. Targets, set before
//!   measuring (ROADMAP M1.8e1): within 0.05 per radian and 0.1 calibers of the report's
//!   second-order values, and within its stated ±0.2 per radian and ±0.2 calibers of its
//!   measurements. `hpr_aero`'s test pins the set of misses.
//! - **The Arcas Robin wind-tunnel model** (NASA TN D-4014): its nose and cylinder, and the same
//!   with its 15° boattail by the report's footnote 8, against the measured body alone, fitted as
//!   `cargo xtask aero` fits it for M1.8a. Reported, with no target: the measurement includes
//!   the boattail's lip and crossflow at the plotted angles, which the method leaves out.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::shock_expansion::{BodySegment, DEFAULT_ELEMENTS_PER_CURVE, ShockExpansionBody};
use hpr_design::{NoseShape, Profile};
use serde_json::{Value, json};

use crate::aero_mach::{WIND_TUNNEL, slope};

pub const FIXTURE: &str = "validation/fixtures/aero/shock-expansion.json";
pub const TABLES: &str = "validation/fixtures/aero/tn3527-bodies.json";
const INCH: f64 = 0.0254;

/// The targets set before measuring: against the report's second-order values, per radian and
/// calibers; against its measurements, per radian and calibers (its stated accuracy, p. 1).
pub const TARGETS: [f64; 4] = [0.05, 0.1, 0.2, 0.2];

/// The Arcas Robin's nose, TN D-4014 Fig. 1(a) (page 10 of the PDF): stations and radii, inches
/// from the tip (also in `arcas-robin-wind-tunnel.json`'s geometry).
pub const ARCAS_NOSE_X_IN: [f64; 9] = [0.0, 2.375, 3.375, 4.375, 5.375, 6.375, 7.375, 8.375, 9.375];
pub const ARCAS_NOSE_R_IN: [f64; 9] = [0.0, 0.414, 0.554, 0.688, 0.804, 0.908, 0.988, 1.062, 1.125];

/// The fixture, from the committed tables and wind-tunnel reference.
pub fn generate(root: &Path) -> Result<Value, String> {
    let tables: Value = serde_json::from_str(
        &fs::read_to_string(root.join(TABLES)).map_err(|e| format!("{TABLES}: {e}"))?,
    )
    .map_err(|e| format!("{TABLES}: {e}"))?;
    let rows = tables["rows"]
        .as_array()
        .ok_or(format!("{TABLES} has no rows"))?;
    let mut out = Vec::new();
    for row in rows {
        let text = |key: &str| row[key].as_str().ok_or(format!("{TABLES}: no `{key}`"));
        let number = |key: &str| row[key].as_f64().ok_or(format!("{TABLES}: no `{key}`"));
        let (nose, fineness, mach, afterbody) = (
            text("nose")?,
            number("fineness")?,
            number("mach")?,
            number("afterbody_calibers")?,
        );
        let body = tn3527_body(nose, fineness, afterbody)?;
        let hpr = match body.slope(mach, 0.25 * PI) {
            Ok(hpr) => hpr,
            Err(e) => {
                out.push(json!({
                    "nose": nose,
                    "fineness": fineness,
                    "mach": mach,
                    "afterbody_calibers": afterbody,
                    "refused": e.to_string(),
                }));
                continue;
            }
        };
        let compare = |value: f64, reference: &Value, target: f64| -> Value {
            match reference.as_f64() {
                Some(r) => json!({
                    "reference": r,
                    "error": value - r,
                    "within": (value - r).abs() <= target + 1e-12,
                }),
                None => Value::Null,
            }
        };
        let [so_cn, so_cp, ex_cn, ex_cp] = TARGETS;
        out.push(json!({
            "nose": nose,
            "fineness": fineness,
            "mach": mach,
            "afterbody_calibers": afterbody,
            "hpr": {
                "c_n_alpha": hpr.slope_per_rad,
                "cp_calibers": hpr.centre_of_pressure_m,
            },
            "second_order": {
                "c_n_alpha": compare(hpr.slope_per_rad, &row["c_n_alpha"]["second_order"], so_cn),
                "cp_calibers": compare(
                    hpr.centre_of_pressure_m,
                    &row["cp"]["second_order"],
                    so_cp
                ),
            },
            "experiment": {
                "c_n_alpha": compare(hpr.slope_per_rad, &row["c_n_alpha"]["experiment"], ex_cn),
                "cp_calibers": compare(
                    hpr.centre_of_pressure_m,
                    &row["cp"]["experiment"],
                    ex_cp
                ),
            },
        }));
    }
    let [so_cn, so_cp, ex_cn, ex_cp] = TARGETS;
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "hpr's second-order shock-expansion slope (per radian, on the base area) and \
                 centre of pressure (calibers from the vertex) at alpha -> 0, with the tangent body \
                 hpr uses by default, against NACA TN 3527's Tables I and II \
                 (tn3527-bodies.json), and the Arcas Robin's nose and cylinder against TN \
                 D-4014's measured body alone. error is hpr's value less the reference's.",
        "elements_per_curve": DEFAULT_ELEMENTS_PER_CURVE,
        "targets": {
            "second_order_c_n_alpha": so_cn,
            "second_order_cp_calibers": so_cp,
            "experiment_c_n_alpha": ex_cn,
            "experiment_cp_calibers": ex_cp,
        },
        "tn3527": out,
        "arcas_robin": arcas_robin(root)?,
    }))
}

/// A TN 3527 body of unit diameter: a cone or tangent ogive of `fineness` calibers and a
/// cylinder of `afterbody` calibers.
pub fn tn3527_body(
    nose: &str,
    fineness: f64,
    afterbody: f64,
) -> Result<ShockExpansionBody, String> {
    let shape = match nose {
        "cone" => NoseShape::Conical {},
        "ogive" => NoseShape::TANGENT_OGIVE,
        other => return Err(format!("{TABLES}: unknown nose `{other}`")),
    };
    let profile = Profile::nose(shape, fineness, 0.5).map_err(|e| e.to_string())?;
    let mut segments = vec![BodySegment::Profile { profile }];
    if afterbody > 0.0 {
        segments.push(BodySegment::Cylinder {
            length_m: afterbody,
            radius_m: 0.5,
        });
    }
    ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).map_err(|e| e.to_string())
}

/// The secant ogive through the Arcas Robin's tip and base nearest the report's coordinates: the
/// arc radius ratio minimising the squared radius misses (golden-section search), and the rms
/// miss, inches.
pub fn arcas_nose_ratio() -> Result<(f64, f64), String> {
    let length = ARCAS_NOSE_X_IN[8] * INCH;
    let radius = ARCAS_NOSE_R_IN[8] * INCH;
    let misses = |ratio: f64| -> Result<f64, String> {
        let nose = Profile::nose(
            NoseShape::Ogive {
                radius_ratio: ratio,
            },
            length,
            radius,
        )
        .map_err(|e| e.to_string())?;
        Ok(ARCAS_NOSE_X_IN
            .iter()
            .zip(ARCAS_NOSE_R_IN)
            .map(|(x, r)| (nose.radius_m(x * INCH) / INCH - r).powi(2))
            .sum())
    };
    let golden = 0.5 * (5.0_f64.sqrt() - 1.0);
    let (mut a, mut b) = (1.0, 5.0);
    for _ in 0..100 {
        let c = b - golden * (b - a);
        let d = a + golden * (b - a);
        if misses(c)? < misses(d)? {
            b = d;
        } else {
            a = c;
        }
    }
    let ratio = 0.5 * (a + b);
    Ok((
        ratio,
        (misses(ratio)? / ARCAS_NOSE_X_IN.len() as f64).sqrt(),
    ))
}

/// The Arcas Robin's body for the method, in metres: the fitted nose, the cylinder to
/// `cylinder_end_in`, and with `boattail` its 15° conical boattail (1.757 in long, to 1.308 in
/// across). The lip behind the boattail, a 57° flare, is past the method (its tangent cone is
/// past Fig. 2).
pub fn arcas_body(
    ratio: f64,
    cylinder_end_in: f64,
    boattail: bool,
) -> Result<ShockExpansionBody, String> {
    let radius = ARCAS_NOSE_R_IN[8] * INCH;
    let nose_length = ARCAS_NOSE_X_IN[8] * INCH;
    let nose = Profile::nose(
        NoseShape::Ogive {
            radius_ratio: ratio,
        },
        nose_length,
        radius,
    )
    .map_err(|e| e.to_string())?;
    let mut segments = vec![
        BodySegment::Profile { profile: nose },
        BodySegment::Cylinder {
            length_m: cylinder_end_in * INCH - nose_length,
            radius_m: radius,
        },
    ];
    if boattail {
        let tail = Profile::transition(
            NoseShape::Conical {},
            1.757 * INCH,
            radius,
            0.5 * 1.308 * INCH,
            false,
        )
        .map_err(|e| e.to_string())?;
        segments.push(BodySegment::Profile { profile: tail });
    }
    ShockExpansionBody::new(&segments, DEFAULT_ELEMENTS_PER_CURVE).map_err(|e| e.to_string())
}

fn arcas_robin(root: &Path) -> Result<Value, String> {
    let text =
        fs::read_to_string(root.join(WIND_TUNNEL)).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let reference: Value =
        serde_json::from_str(&text).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let (ratio, rms_in) = arcas_nose_ratio()?;
    let diameter = 2.0 * ARCAS_NOSE_R_IN[8] * INCH;
    let area = 0.25 * PI * diameter * diameter;
    let mut configurations = Vec::new();
    for configuration in reference["configurations"]
        .as_array()
        .ok_or(format!("{WIND_TUNNEL} has no configurations"))?
    {
        let id = configuration["id"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: a configuration without `id`"))?;
        let cylinder_end_in = reference["geometry"]["cylinder_ends_in"][id]
            .as_f64()
            .ok_or(format!("{WIND_TUNNEL}: no cylinder end for {id}"))?;
        let bare = arcas_body(ratio, cylinder_end_in, false)?;
        let tailed = arcas_body(ratio, cylinder_end_in, true)?;
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
                .map(|p| Some((p[0].as_f64()?.to_radians(), p[1].as_f64()?)))
                .collect::<Option<_>>()
                .ok_or("an unreadable point")?;
            let alphas: Vec<f64> = points.iter().map(|p| p.0).collect();
            let measured: Vec<f64> = points.iter().map(|p| p.1).collect();
            let measured = slope(&alphas, &measured);
            let entry = |body: &ShockExpansionBody| -> Result<Value, String> {
                let s = body
                    .slope(mach, area)
                    .map_err(|e| format!("{id} at Mach {mach}: {e}"))?;
                Ok(json!({
                    "c_n_alpha": s.slope_per_rad,
                    "cp_calibers": s.centre_of_pressure_m / diameter,
                    "c_n_alpha_error": s.slope_per_rad / measured - 1.0,
                }))
            };
            rows.push(json!({
                "mach": mach,
                "measured_c_n_alpha": measured,
                "nose_and_cylinder": entry(&bare)?,
                "with_boattail": entry(&tailed)?,
            }));
        }
        configurations.push(json!({
            "id": id,
            "cylinder_ends_in": cylinder_end_in,
            "rows": rows,
        }));
    }
    Ok(json!({
        "note": "The measured slope is the least-squares slope, with an intercept, of TN \
                 D-4014's fins-off C_N points (arcas-robin-wind-tunnel.json), as M1.8a fits it; \
                 it includes the lip and crossflow at the plotted angles. hpr's is the method's \
                 at alpha -> 0 on the maximum cross-section. c_n_alpha_error is hpr's over the \
                 measured, minus 1. No target (M1.8e2 flies it).",
        "nose": {
            "shape": "the secant ogive through the tip and base nearest TN D-4014 Fig. 1(a)'s \
                      coordinates",
            "radius_ratio": ratio,
            "rms_miss_in": rms_in,
            "vertex_half_angle_deg": arcas_body(ratio, 39.14, false)?.vertex_angle_rad().to_degrees(),
        },
        "configurations": configurations,
    }))
}
