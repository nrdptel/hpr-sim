//! Drag against Mach (M1.8b): hpr's zero-lift drag against the Arcas Robin wind-tunnel models'
//! measured axial force, written by `cargo xtask aero` to
//! `validation/fixtures/aero/drag-vs-mach.json`.
//!
//! The reference is NASA's half-scale Arcas Robin (TN D-4013, Mach 0.6 to 1.2; TN D-4014, Mach 1.5
//! to 4.63), read from the reports' plots into `validation/fixtures/aero/arcas-robin-wind-tunnel.json`
//! with the normal force (M1.8a). The models sat on a sting, so their base pressure isn't a free
//! flight's, and both reports take the base apart: TN D-4013 plots the axial force "corrected for
//! base axial force" (`C_A,corr`), and TN D-4014 the axial force and the balance chamber's
//! (`C_A,c`). The comparison is of the forebody, then: hpr's `C_D0` less its base drag (friction,
//! pressure and parasitic drag), at the tunnels' Reynolds number, fins on and fins off.
//!
//! Target, set before measuring: within 10%, M1.8's tolerance for drag. A miss is reported and
//! explained, not hidden; `hpr_aero`'s test pins the set of misses.
//!
//! The same fixture compares hpr's whole `C_D0`, base included, with MIL-HDBK-762's sample drag
//! calculation (Table 5-4) for the rocket of its Fig. 5-155, term by term (M1.8b2, ADR-029): a
//! calculation with every input known, transcribed into
//! `validation/fixtures/aero/mil-hdbk-762-sample-drag.json`, not a measurement.
//!
//! And it compares hpr's boattail pressure drag and base pressure behind a boattail with measured
//! conical boattails, and the boattail chart with Jack's second-order theory (M1.8b3, ADR-030),
//! transcribed into `validation/fixtures/aero/measured-boattails.json`.

use std::fs;
use std::path::Path;

use hpr_aero::{AeroModel, Boattail, DragConditions, Flow};
use hpr_design::Rocket;
use serde_json::{Value, json};

pub const FIXTURE: &str = "validation/fixtures/aero/drag-vs-mach.json";
const WIND_TUNNEL: &str = crate::aero_mach::WIND_TUNNEL;
const TARGET: f64 = 0.10;

/// Both tunnels ran at 3.0 × 10⁶ per foot (TN D-4013 p. 1; TN D-4014 p. 1 and table I).
pub const REYNOLDS_PER_M: f64 = 3.0e6 / 0.3048;

/// The fixture, from the committed designs and the wind-tunnel reference.
pub fn generate(root: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(BOATTAILS)).map_err(|e| format!("{BOATTAILS}: {e}"))?;
    let references: Value = serde_json::from_str(&text).map_err(|e| format!("{BOATTAILS}: {e}"))?;
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "hpr's forebody drag, C_D0 less the base drag (friction + pressure + parasitic, on \
                 the maximum cross-section), against each reference's forebody axial force, at \
                 zero angle of attack and the reference's Reynolds number. error is hpr's over the \
                 reference's, minus 1. Bands are Niskanen 2009 Table 3.1's. hpr's base drag is \
                 given for information: the tunnels' base pressure is the sting's, not a free \
                 flight's.",
        "target_rel": TARGET,
        "reynolds_per_m": REYNOLDS_PER_M,
        "references": wind_tunnel(root)?,
        "calculations_note": "hpr's C_D0, base included, against MIL-HDBK-762's sample drag \
                              calculation (Table 5-4) for the rocket of its Fig. 5-155, at the \
                              Reynolds number per metre the table gives for each Mach number, term \
                              by term: friction (body and fins), the nose's wave drag against \
                              hpr's nose pressure drag, the fins' pressure drag, and the body's \
                              jet-off base drag. A calculation with every input known, not a \
                              measurement. The handbook's fins are single wedges, sharp at the \
                              leading edge, a section hpr doesn't have; the design takes square \
                              edges, so the fins' pressure drag (hpr's square edges; the \
                              handbook's wave and trailing-edge base drag) is recorded but left \
                              out of the comparison: compared is each total less it, and error is \
                              hpr's compared over the handbook's, minus 1.",
        "calculations": [handbook(root)?],
        "boattails_note": "hpr's boattail pressure drag on the boattail's fore (cylinder) area \
                           (hpr_aero::afterbody) against measured conical boattails behind a \
                           cylinder, jet off, at zero angle of attack \
                           (validation/fixtures/aero/measured-boattails.json). hpr_attached is \
                           the attached-flow value before the separation blend (from Mach 1); \
                           error is hpr's \
                           over the measured value, minus 1.",
        "boattails": boattail_rows(&references, "boattails", boattail_drag)?,
        "base_pressures_note": "hpr's base-pressure ratio k = C_p,bt/C_p,cyl behind a boattail \
                                (hpr_aero::afterbody::Boattail::base_pressure_ratio) against the \
                                measured ratio, each report's boattail base pressure over its own \
                                cylinder's. base_cd_difference is (k_hpr − k_measured) times the \
                                measured cylinder's −C_p,b times the base's area over the \
                                cylinder's: the error in base drag on the cylinder's area.",
        "base_pressures": boattail_rows(&references, "base_pressures", base_pressure)?,
        "second_order_theory_note": "hpr's attached boattail pressure drag (the chart held to the \
                                     2D limit) against Jack's second-order theory (NACA TN 2972, \
                                     Fig. 3) at each legible point; x is the chart's abscissa \
                                     √(M² − 1)/(2 l/d₁), inside the chart up to 1.4.",
        "second_order_theory": boattail_rows(&references, "second_order_theory", theory)?,
    }))
}

const BOATTAILS: &str = "validation/fixtures/aero/measured-boattails.json";

/// A boattail of fore diameter 1 from a reference row: its length ratio `l/d₁` and diameter ratio
/// `d₂/d₁`, or its area ratio and half-angle.
fn boattail_of(row: &Value) -> Result<Boattail, String> {
    let (length, ratio) = match (row["length_ratio"].as_f64(), row["diameter_ratio"].as_f64()) {
        (Some(l), Some(r)) => (l, r),
        _ => {
            let a = row["area_ratio"]
                .as_f64()
                .ok_or(format!("{BOATTAILS}: a row without its geometry"))?;
            let theta = row["half_angle_deg"]
                .as_f64()
                .ok_or(format!("{BOATTAILS}: a row without its geometry"))?
                .to_radians();
            let r = a.sqrt();
            ((1.0 - r) / (2.0 * theta.tan()), r)
        }
    };
    Boattail::new(length, 1.0, ratio).map_err(|e| format!("{BOATTAILS}: {e}"))
}

fn number(row: &Value, key: &str) -> Result<f64, String> {
    row[key]
        .as_f64()
        .ok_or(format!("{BOATTAILS}: a row without `{key}`"))
}

/// Each reference row with hpr's values added by `add`.
fn boattail_rows(
    references: &Value,
    section: &str,
    add: fn(&Value, &Boattail) -> Result<Value, String>,
) -> Result<Value, String> {
    let mut rows = Vec::new();
    for row in references[section]["rows"]
        .as_array()
        .ok_or(format!("{BOATTAILS} has no `{section}`"))?
    {
        let boattail = boattail_of(row)?;
        let mut out = row.clone();
        let extra = add(row, &boattail)?;
        if let (Some(out), Some(extra)) = (out.as_object_mut(), extra.as_object()) {
            out.insert("length_ratio".into(), json!(boattail.length_ratio));
            out.insert("area_ratio".into(), json!(boattail.area_ratio));
            out.insert(
                "half_angle_deg".into(),
                json!(boattail.half_angle_rad.to_degrees()),
            );
            for (k, v) in extra {
                out.insert(k.clone(), v.clone());
            }
        }
        rows.push(out);
    }
    Ok(json!({ "source": references[section]["source"], "rows": rows }))
}

fn boattail_drag(row: &Value, boattail: &Boattail) -> Result<Value, String> {
    let mach = number(row, "mach")?;
    let hpr = boattail
        .pressure_drag_coefficient(mach)
        .map_err(|e| e.to_string())?;
    // The attached drag is defined from Mach 1.
    let attached = if mach >= hpr_aero::afterbody::SUPERSONIC_MACH {
        json!(
            boattail
                .attached_pressure_drag(mach)
                .map_err(|e| e.to_string())?
        )
    } else {
        Value::Null
    };
    Ok(json!({
        "hpr": hpr,
        "hpr_attached": attached,
        "separation_weight": boattail.separation_weight,
        "error": hpr / number(row, "cd")? - 1.0,
    }))
}

fn base_pressure(row: &Value, boattail: &Boattail) -> Result<Value, String> {
    let mach = number(row, "mach")?;
    let (cylinder, behind) = (number(row, "cylinder_cp")?, number(row, "boattail_cp")?);
    let measured = behind / cylinder;
    let hpr = boattail
        .base_pressure_ratio(mach, boattail.area_ratio)
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "k_measured": measured,
        "k_hpr": hpr,
        "base_cd_difference": (hpr - measured) * -cylinder * boattail.area_ratio,
    }))
}

fn theory(row: &Value, boattail: &Boattail) -> Result<Value, String> {
    let mach = number(row, "mach")?;
    let hpr = boattail
        .attached_pressure_drag(mach)
        .map_err(|e| e.to_string())?;
    Ok(json!({
        "x": (mach * mach - 1.0).sqrt() / (2.0 * boattail.length_ratio),
        "hpr": hpr,
        "error": hpr / number(row, "cd")? - 1.0,
    }))
}

const HANDBOOK: &str = "validation/fixtures/aero/mil-hdbk-762-sample-drag.json";

/// hpr's drag, term by term, against MIL-HDBK-762's sample calculation (Table 5-4).
fn handbook(root: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(root.join(HANDBOOK)).map_err(|e| format!("{HANDBOOK}: {e}"))?;
    let reference: Value = serde_json::from_str(&text).map_err(|e| format!("{HANDBOOK}: {e}"))?;
    let design = "mil-hdbk-762-sample-rocket.json";
    let model = load_model(root, design)?;
    let fin_ids: Vec<&str> = model.fin_sets().iter().map(|f| f.id.as_str()).collect();
    let mut rows = Vec::new();
    for row in reference["rows"]
        .as_array()
        .ok_or(format!("{HANDBOOK} has no rows"))?
    {
        let get = |key: &str| {
            row[key]
                .as_f64()
                .ok_or(format!("{HANDBOOK}: a row without `{key}`"))
        };
        let mach = get("mach")?;
        let conditions = DragConditions::coasting(get("reynolds_per_m")?);
        let flow = Flow::axial(mach);
        let total = model
            .drag(&flow, &conditions)
            .map_err(|e| format!("Mach {mach}: {e}"))?;
        let parts = model
            .buildup_components(&flow, &conditions)
            .map_err(|e| format!("Mach {mach}: {e}"))?;
        let (mut nose, mut fins, mut other) = (0.0, 0.0, 0.0);
        for part in &parts {
            if part.id == "nose" {
                nose += part.drag.pressure;
            } else if fin_ids.contains(&part.id.as_str()) {
                fins += part.drag.pressure;
            } else {
                other += part.drag.pressure + part.drag.parasitic;
            }
        }
        let reference_total = get("total_jet_off")?;
        let reference_fins = get("fins")?;
        let reference_compared = reference_total - reference_fins;
        if !(reference_compared.is_finite() && reference_compared > 0.0) {
            return Err(format!(
                "{HANDBOOK} at Mach {mach}: a total without its fins of {reference_compared}"
            ));
        }
        let hpr = total.zero_lift_coefficient;
        let hpr_compared = hpr - fins;
        let error = hpr_compared / reference_compared - 1.0;
        rows.push(json!({
            "mach": mach,
            "band": crate::aero_mach::band(mach),
            "reynolds_per_m": get("reynolds_per_m")?,
            "reference": {
                "friction": get("friction")?,
                "nose": row["nose_wave"].as_f64().unwrap_or(0.0),
                "fins": reference_fins,
                "base": get("base")?,
                "total": reference_total,
            },
            "hpr": {
                "friction": total.friction,
                "nose": nose,
                "fins": fins,
                "base": total.base,
                "other": other,
                "total": hpr,
            },
            "compared": {
                "reference": reference_compared,
                "hpr": hpr_compared,
            },
            "error": error,
            "within_target": error.abs() <= TARGET,
        }));
    }
    Ok(json!({
        "id": "mil-hdbk-762-sample",
        "design": design,
        "source": reference["source"],
        "rows": rows,
    }))
}

/// hpr's forebody drag at one Mach number, each part on the reference area.
pub struct Forebody {
    /// Friction, pressure and parasitic drag: `C_D0` less the base drag.
    pub total: f64,
    /// Its friction.
    pub friction: f64,
    /// Its pressure drag.
    pub pressure: f64,
    /// The base drag it leaves out.
    pub base: f64,
    /// Each kept component's forebody drag, by id.
    pub parts: serde_json::Map<String, Value>,
}

/// hpr's forebody drag at `mach`, with or without the fin sets.
pub fn hpr_forebody(model: &AeroModel, mach: f64, fins: bool) -> Result<Forebody, String> {
    let parts = model
        .buildup_components(
            &Flow::axial(mach),
            &DragConditions::coasting(REYNOLDS_PER_M),
        )
        .map_err(|e| format!("Mach {mach}: {e}"))?;
    let fin_ids: Vec<&str> = model.fin_sets().iter().map(|f| f.id.as_str()).collect();
    let (mut friction, mut pressure, mut parasitic, mut base) = (0.0, 0.0, 0.0, 0.0);
    let mut by_part = serde_json::Map::new();
    for part in parts
        .iter()
        .filter(|p| fins || !fin_ids.contains(&p.id.as_str()))
    {
        friction += part.drag.friction;
        pressure += part.drag.pressure;
        parasitic += part.drag.parasitic;
        base += part.drag.base;
        by_part.insert(
            part.id.clone(),
            json!(part.drag.friction + part.drag.pressure + part.drag.parasitic),
        );
    }
    Ok(Forebody {
        total: friction + pressure + parasitic,
        friction,
        pressure,
        base,
        parts: by_part,
    })
}

fn load_model(root: &Path, name: &str) -> Result<AeroModel, String> {
    let path = root.join("validation/designs").join(name);
    let text = fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
    let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))?;
    let layout = rocket.layout().map_err(|e| format!("{name}: {e}"))?;
    AeroModel::new(&layout).map_err(|e| format!("{name}: {e}"))
}

/// Each wind-tunnel configuration's rows: at every Mach number the reports give, fins on and off,
/// the measured forebody axial force against hpr's.
fn wind_tunnel(root: &Path) -> Result<Vec<Value>, String> {
    let text =
        fs::read_to_string(root.join(WIND_TUNNEL)).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let reference: Value =
        serde_json::from_str(&text).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let configurations = reference["configurations"]
        .as_array()
        .ok_or(format!("{WIND_TUNNEL} has no configurations"))?;
    let mut out = Vec::new();
    for configuration in configurations {
        let design = configuration["design"]
            .as_str()
            .ok_or(format!("{WIND_TUNNEL}: a configuration without `design`"))?;
        let model = load_model(root, design)?;
        let points = configuration["axial_force"].as_array().ok_or(format!(
            "{WIND_TUNNEL}: a configuration without `axial_force`"
        ))?;
        let mut rows = Vec::new();
        for point in points {
            let get = |key: &str| {
                point[key].as_f64().ok_or(format!(
                    "{WIND_TUNNEL}: an axial-force point without `{key}`"
                ))
            };
            let mach = get("mach")?;
            let fins = match point["fins"].as_str() {
                Some("on") => true,
                Some("off") => false,
                other => return Err(format!("{WIND_TUNNEL}: fins {other:?}, not on or off")),
            };
            let measured = get("forebody_c_a")?;
            let forebody = hpr_forebody(&model, mach, fins)?;
            let hpr = forebody.total;
            let error = hpr / measured - 1.0;
            rows.push(json!({
                "mach": mach,
                "band": crate::aero_mach::band(mach),
                "fins": point["fins"],
                "reference_forebody_c_a": measured,
                "reference_uncertainty": get("uncertainty")?,
                "hpr_forebody_c_d": hpr,
                "hpr_friction": forebody.friction,
                "hpr_pressure": forebody.pressure,
                "hpr_base": forebody.base,
                "hpr_forebody_by_part": forebody.parts,
                "error": error,
                "within_target": error.abs() <= TARGET,
                "source": point["source"],
            }));
        }
        out.push(json!({
            "id": configuration["id"],
            "design": design,
            "source": format!(
                "{} Forebody axial force at zero angle of attack, fins at 0 degrees or off: TN \
                 D-4013's C_A,corr (the axial force corrected for the base's), and TN D-4014's C_A \
                 less the balance chamber's C_A,c. A measurement.",
                reference["source"].as_str().unwrap_or_default()
            ),
            "rows": rows,
        }));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    /// The committed fixture is what the generator writes from the committed designs and
    /// references. Unlike `cargo xtask aero --check`, this needs nothing from `refs/`, so CI runs
    /// it: it catches a hand edit to any field, including the ones `hpr_aero`'s tests don't read.
    #[test]
    fn committed_fixture_matches_the_generator() {
        let root = crate::designs::root().unwrap();
        let committed: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(super::FIXTURE)).unwrap())
                .unwrap();
        let generated = super::generate(&root).unwrap();
        assert!(
            crate::designs::same(&committed, &generated),
            "{} differs from `cargo xtask aero`: {:?}",
            super::FIXTURE,
            crate::designs::difference(&committed, &generated)
        );
    }
}
