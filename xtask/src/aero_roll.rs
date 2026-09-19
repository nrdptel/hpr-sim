//! Roll against Mach (M1.8c): hpr's roll forcing against the Arcas Robin's measured roll
//! effectiveness, and its roll damping against the Basic Finner's, written by `cargo xtask aero`
//! to `validation/fixtures/aero/roll-vs-mach.json`.
//!
//! - **The Arcas Robin wind-tunnel models** (NASA TN D-4014 Fig. 14): `C_lδ`, the rolling moment
//!   per degree of fin cant on the body's cross-section and diameter, read into
//!   `validation/fixtures/aero/arcas-robin-wind-tunnel.json` at `α` = −4°, 0° and +4°; compared at
//!   0°, where hpr's model applies.
//! - **The Basic Finner** (Barrowman 1967 Figs. 5-6 and 5-7): its roll damping `C_lp` measured
//!   in a wind tunnel from Mach 1.5 to 3, and Barrowman's computed value at Mach 0.07, read into
//!   `validation/fixtures/aero/basic-finner-roll-damping.json`.
//!
//! The roadmap sets no target for either (M1.8c: "compared"); each error is reported as it is.

use std::f64::consts::PI;
use std::fs;
use std::path::Path;

use hpr_aero::{AeroModel, FinAero, roll_damping_interference};
use hpr_design::{FinPlanform, Rocket};
use serde_json::{Value, json};

pub const FIXTURE: &str = "validation/fixtures/aero/roll-vs-mach.json";
pub const BASIC_FINNER: &str = "validation/fixtures/aero/basic-finner-roll-damping.json";

fn read(root: &Path, name: &str) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(name)).map_err(|e| format!("{name}: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))
}

fn number(value: &Value, what: &str) -> Result<f64, String> {
    value
        .as_f64()
        .ok_or_else(|| format!("{what} is not a number"))
}

/// The fixture, from the committed designs and the two committed references.
pub fn generate(root: &Path) -> Result<Value, String> {
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "hpr's roll forcing C_l_delta (per degree of cant, the rolling moment the cant \
                 drives, positive, on the body's cross-section and diameter) against the Arcas \
                 Robin's measured roll effectiveness at an angle of attack of 0, and hpr's roll \
                 damping C_lp (per unit of p d/(2V)) against the Basic Finner's. error is hpr's \
                 over the reference's, minus 1. The roadmap sets no target (M1.8c).",
        "arcas_robin": arcas_robin(root)?,
        "basic_finner": basic_finner(root)?,
    }))
}

/// Each Arcas Robin configuration's `C_lδ` at `α = 0` against hpr's.
fn arcas_robin(root: &Path) -> Result<Vec<Value>, String> {
    let tunnel = read(root, crate::aero_mach::WIND_TUNNEL)?;
    let configurations = tunnel["configurations"]
        .as_array()
        .ok_or("the wind tunnel has no `configurations`")?;
    let mut out = Vec::new();
    for configuration in configurations {
        let id = configuration["id"]
            .as_str()
            .ok_or("a configuration without `id`")?;
        let design = configuration["design"]
            .as_str()
            .ok_or("a configuration without `design`")?;
        let rocket: Rocket =
            serde_json::from_value(read(root, &format!("validation/designs/{design}"))?)
                .map_err(|e| format!("{design}: {e}"))?;
        let layout = rocket.layout().map_err(|e| format!("{design}: {e}"))?;
        let model = AeroModel::new(&layout).map_err(|e| format!("{design}: {e}"))?;
        let [set] = model.fin_sets() else {
            return Err(format!("{design}: one fin set expected"));
        };
        let d = model.reference_diameter_m();
        let mut rows = Vec::new();
        let curves = configuration["roll_effectiveness"]
            .as_array()
            .ok_or_else(|| format!("{id}: no `roll_effectiveness`"))?;
        for curve in curves {
            let mach = number(&curve["mach"], "a roll curve's `mach`")?;
            let points = curve["alpha_deg_c_l_delta_per_deg"]
                .as_array()
                .ok_or_else(|| format!("{id} at Mach {mach}: no points"))?;
            let at_zero = points
                .iter()
                .find(|p| p[0].as_f64().is_some_and(|a| a.abs() < 0.5))
                .ok_or_else(|| format!("{id} at Mach {mach}: no reading at 0 deg"))?;
            let measured = number(&at_zero[1], "a roll reading")?;
            let fin = set
                .fin
                .roll(mach, set.body_radius_m, d)
                .map_err(|e| format!("{id} at Mach {mach}: {e}"))?;
            let per_rad =
                f64::from(set.count) * fin.forcing_per_rad * set.roll_forcing_interference;
            let hpr = per_rad * PI / 180.0;
            let error = hpr / measured - 1.0;
            println!(
                "{id:<20} Mach {mach:<5} C_l_delta {hpr:.4}/deg against {measured:.4} ({:+.1}%)",
                100.0 * error
            );
            rows.push(json!({
                "mach": mach,
                "band": crate::aero_mach::band(mach),
                "alpha_deg": at_zero[0],
                "reference_c_l_delta_per_deg": measured,
                "hpr_c_l_delta_per_deg": hpr,
                "hpr_without_body_interference_per_deg": hpr / set.roll_forcing_interference,
                "error": error,
            }));
        }
        out.push(json!({
            "id": id,
            "design": design,
            "reference": "TN D-4014 Fig. 14",
            "fin_count": set.count,
            "roll_forcing_interference": set.roll_forcing_interference,
            "roll_damping_interference": set.roll_damping_interference,
            "rows": rows,
        }));
    }
    Ok(out)
}

/// The Basic Finner's `C_lp`: four square fins one diameter in chord and span on a body one
/// diameter across, so any diameter gives the same coefficients.
fn basic_finner(root: &Path) -> Result<Value, String> {
    let reference = read(root, BASIC_FINNER)?;
    let geometry = &reference["geometry"];
    let count = number(&geometry["fin_count"], "fin_count")?;
    let (c_r, c_t, s, sweep, r) = (
        number(&geometry["root_chord_calibers"], "root_chord_calibers")?,
        number(&geometry["tip_chord_calibers"], "tip_chord_calibers")?,
        number(&geometry["span_calibers"], "span_calibers")?,
        number(&geometry["leading_edge_sweep_calibers"], "sweep")?,
        number(&geometry["body_radius_at_fins_calibers"], "body radius")?,
    );
    let d = 1.0;
    let planform = FinPlanform::Trapezoidal {
        root_chord_m: c_r,
        tip_chord_m: c_t,
        span_m: s,
        sweep_m: sweep,
    };
    let fin = FinAero::new(&planform, 0.25 * PI * d * d).map_err(|e| e.to_string())?;
    let k_r = roll_damping_interference(s, r, c_t / c_r).map_err(|e| e.to_string())?;
    let rows_of = |key: &str| -> Result<Vec<Value>, String> {
        let points = reference[key]
            .as_array()
            .ok_or_else(|| format!("the Basic Finner has no `{key}`"))?;
        points
            .iter()
            .map(|p| {
                let mach = number(&p["mach"], "a Basic Finner `mach`")?;
                let c_lp = number(&p["c_lp"], "a Basic Finner `c_lp`")?;
                let hpr = count
                    * fin.roll(mach, r, d).map_err(|e| e.to_string())?.damping
                    * k_r;
                let error = hpr / c_lp - 1.0;
                println!(
                    "basic-finner {key:<22} Mach {mach:<5} C_lp {hpr:.3} against {c_lp:.3} ({:+.1}%)",
                    100.0 * error
                );
                Ok(json!({
                    "mach": mach,
                    "band": crate::aero_mach::band(mach),
                    "reference_c_lp": c_lp,
                    "hpr_c_lp": hpr,
                    "error": error,
                }))
            })
            .collect()
    };
    Ok(json!({
        "id": "basic-finner",
        "reference": "Barrowman 1967 Fig. 5-7",
        "roll_damping_interference": k_r,
        "wind_tunnel": rows_of("wind_tunnel_c_lp")?,
        "barrowman_theory": rows_of("barrowman_theory_c_lp")?,
    }))
}
