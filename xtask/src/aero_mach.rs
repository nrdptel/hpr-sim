//! The normal force against Mach (M1.8a): hpr's small-angle `C_Nα` and centre of pressure against
//! two references, written by `cargo xtask aero` to
//! `validation/fixtures/aero/normal-force-vs-mach.json`.
//!
//! - **RASAero II's Calisto export** (`refs/rocketpy-history/calisto-cd-test-2018.csv`, pinned in
//!   `validation/refs.lock.toml`): its potential-flow normal force at 2° over the angle, and its
//!   CP at 0°. Its `CNalpha (0 to 4 deg)` column is the secant slope to 4° and includes, past
//!   Mach 1, a viscous cross-flow term that hpr's small-angle slope leaves out (hpr's body lift
//!   grows as `sin² α`). The export stays in `refs/`; the fixture holds its values at the compared
//!   Mach numbers only, as `cargo xtask aero` does for the drag curves (ADR-009).
//! - **The Arcas Robin wind-tunnel model** (NASA TN D-4013 and TN D-4014): the measurements read
//!   from the reports' plots into `validation/fixtures/aero/arcas-robin-wind-tunnel.json`, a
//!   committed reference, and the design `cargo xtask designs` builds from it.
//!
//! Targets, set before measuring (ROADMAP M1.8a): CP within 0.5 calibers and `C_Nα` within 15%.
//! A miss is reported and explained, not hidden; `hpr_aero`'s test pins the set of misses.

use std::fs;
use std::path::Path;

use hpr_aero::{AeroModel, Flow};
use hpr_design::Rocket;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const FIXTURE: &str = "validation/fixtures/aero/normal-force-vs-mach.json";
pub const WIND_TUNNEL: &str = "validation/fixtures/aero/arcas-robin-wind-tunnel.json";
const EXPORT: &str = "refs/rocketpy-history/calisto-cd-test-2018.csv";
const CALISTO_DESIGN: &str = "rocketpy-calisto-tests-motor-at-minus-1.373.json";
const INCH: f64 = 0.0254;
const CP_TARGET_CALIBERS: f64 = 0.5;
const CN_ALPHA_TARGET: f64 = 0.15;

/// The Mach numbers compared against the export: its rows from 0.1 to 2.0, closer together
/// through the transonic region.
const CALISTO_MACHS: &[f64] = &[
    0.1, 0.3, 0.5, 0.7, 0.8, 0.9, 0.95, 1.0, 1.05, 1.1, 1.2, 1.3, 1.5, 1.75, 2.0,
];

/// Niskanen 2009 Table 3.1's regions: subsonic to 0.8, transonic to 1.2, supersonic beyond.
fn band(mach: f64) -> &'static str {
    if mach <= 0.8 {
        "subsonic"
    } else if mach < 1.2 {
        "transonic"
    } else {
        "supersonic"
    }
}

/// The fixture, from the committed designs, the wind-tunnel reference and the export in `refs/`.
pub fn generate(root: &Path) -> Result<Value, String> {
    let calisto = calisto(root)?;
    let tunnel = wind_tunnel(root)?;
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "hpr's small-angle normal-force slope (per radian, on its reference area) and \
                 centre of pressure (m aft of the nose tip) at alpha -> 0, against each \
                 reference's. cn_alpha_error is hpr's over the reference's, minus 1; \
                 cp_error_calibers is hpr's CP less the reference's, over the reference \
                 diameter, positive aft. Bands are Niskanen 2009 Table 3.1's.",
        "targets": {
            "cp_calibers": CP_TARGET_CALIBERS,
            "cn_alpha_rel": CN_ALPHA_TARGET,
        },
        "references": [calisto, tunnel],
    }))
}

/// One compared row.
fn row(
    mach: f64,
    cn_alpha: f64,
    cp_m: f64,
    model: &AeroModel,
    diameter_m: f64,
) -> Result<Value, String> {
    let (hpr_cn_alpha, hpr_cp_m) = hpr_at(model, mach)?;
    let cn_alpha_error = hpr_cn_alpha / cn_alpha - 1.0;
    let cp_error_calibers = (hpr_cp_m - cp_m) / diameter_m;
    Ok(json!({
        "mach": mach,
        "band": band(mach),
        "reference_cn_alpha_per_rad": cn_alpha,
        "reference_cp_m": cp_m,
        "hpr_cn_alpha_per_rad": hpr_cn_alpha,
        "hpr_cp_m": hpr_cp_m,
        "cn_alpha_error": cn_alpha_error,
        "cp_error_calibers": cp_error_calibers,
        "within_targets": cn_alpha_error.abs() <= CN_ALPHA_TARGET
            && cp_error_calibers.abs() <= CP_TARGET_CALIBERS,
    }))
}

/// hpr's small-angle slope and CP at `mach`, straight into the wind.
pub fn hpr_at(model: &AeroModel, mach: f64) -> Result<(f64, f64), String> {
    let force = model
        .normal_force(&Flow::axial(mach))
        .map_err(|e| format!("Mach {mach}: {e}"))?;
    let cp = force
        .cp_station_m
        .ok_or(format!("Mach {mach}: no normal-force slope, so no CP"))?;
    Ok((force.slope_per_rad, cp))
}

fn load_design(root: &Path, name: &str) -> Result<(Rocket, AeroModel), String> {
    let path = root.join("validation/designs").join(name);
    let text = fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
    let rocket: Rocket = serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))?;
    let layout = rocket.layout().map_err(|e| format!("{name}: {e}"))?;
    let model = AeroModel::new(&layout).map_err(|e| format!("{name}: {e}"))?;
    Ok((rocket, model))
}

fn calisto(root: &Path) -> Result<Value, String> {
    let (_, model) = load_design(root, CALISTO_DESIGN)?;
    let diameter_m = (4.0 * model.reference_area_m2() / std::f64::consts::PI).sqrt();
    let bytes = fs::read(root.join(EXPORT))
        .map_err(|e| format!("{EXPORT}: {e} (run `cargo xtask refs fetch`)"))?;
    let text = std::str::from_utf8(&bytes).map_err(|e| format!("{EXPORT}: {e}"))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or(format!("{EXPORT} is empty"))?
        .split(',')
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|h| h.trim() == name)
            .ok_or(format!("{EXPORT} has no column `{name}`"))
    };
    let (mach_col, alpha_col, potential_col, cp_col) = (
        column("Mach")?,
        column("Alpha")?,
        column("CN Potential")?,
        column("CP")?,
    );
    // (Mach, alpha) -> (CN potential, CP in inches).
    let mut table = Vec::new();
    for line in lines {
        let cells: Vec<&str> = line.split(',').collect();
        let get = |i: usize| -> Result<f64, String> {
            cells
                .get(i)
                .and_then(|c| c.trim().parse::<f64>().ok())
                .ok_or(format!("{EXPORT}: unreadable row `{line}`"))
        };
        table.push((
            get(mach_col)?,
            get(alpha_col)?,
            get(potential_col)?,
            get(cp_col)?,
        ));
    }
    let find = |mach: f64, alpha: f64| {
        table
            .iter()
            .find(|r| (r.0 - mach).abs() < 1e-9 && r.1 == alpha)
            .ok_or(format!("{EXPORT} has no row at Mach {mach}, alpha {alpha}"))
    };
    let mut rows = Vec::new();
    for &mach in CALISTO_MACHS {
        let two = find(mach, 2.0)?;
        let zero = find(mach, 0.0)?;
        let cn_alpha = two.2 / 2.0_f64.to_radians();
        rows.push(row(mach, cn_alpha, zero.3 * INCH, &model, diameter_m)?);
    }
    Ok(json!({
        "id": "calisto-rasaero-ii",
        "design": CALISTO_DESIGN,
        "reference_diameter_m": diameter_m,
        "source": "RASAero II export for Calisto with the 2018 notebook's fins, RocketPy's first \
                   commit's data/calisto/CD Test.CSV (da91db9e, 2018): `CN Potential` at 2 \
                   degrees over the angle, and `CP` at 0 degrees, converted from inches. A code, \
                   not a measurement.",
        "source_sha256": Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "rows": rows,
    }))
}

fn wind_tunnel(root: &Path) -> Result<Value, String> {
    let path = root.join(WIND_TUNNEL);
    let text = fs::read_to_string(&path).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let reference: Value =
        serde_json::from_str(&text).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let design = reference["design"]
        .as_str()
        .ok_or(format!("{WIND_TUNNEL} names no design"))?;
    let (_, model) = load_design(root, design)?;
    let diameter_m = (4.0 * model.reference_area_m2() / std::f64::consts::PI).sqrt();
    let points = reference["measurements"]
        .as_array()
        .ok_or(format!("{WIND_TUNNEL} has no measurements"))?;
    let mut rows = Vec::new();
    for point in points {
        let get = |key: &str| {
            point[key]
                .as_f64()
                .ok_or(format!("{WIND_TUNNEL}: a measurement without `{key}`"))
        };
        let mut compared = row(
            get("mach")?,
            get("cn_alpha_per_rad")?,
            get("cp_m")?,
            &model,
            diameter_m,
        )?;
        compared["source"] = point["source"].clone();
        rows.push(compared);
    }
    Ok(json!({
        "id": reference["id"],
        "design": design,
        "reference_diameter_m": diameter_m,
        "source": reference["source"],
        "rows": rows,
    }))
}
