//! The normal force against Mach (M1.8a): hpr's small-angle `C_Nα` and centre of pressure against
//! two references, written by `cargo xtask aero` to
//! `validation/fixtures/aero/normal-force-vs-mach.json`.
//!
//! - **RASAero II's Calisto export** (`refs/rocketpy-history/calisto-cd-test-2018.csv`, pinned in
//!   `validation/refs.lock.toml`): its potential-flow normal force at 2° over the angle, and its
//!   CP at 0°. Its `CNalpha (0 to 4 deg)` column is the secant slope to 4° and includes, from
//!   Mach 0.95, a viscous cross-flow term that hpr's small-angle slope leaves out (hpr's body
//!   lift grows as `sin² α`). The export stays in `refs/`; the fixture holds its values at the compared
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
pub const EXPORT: &str = "refs/rocketpy-history/calisto-cd-test-2018.csv";
pub const CALISTO_DESIGN: &str = "rocketpy-calisto-tests-motor-at-minus-1.373.json";
const INCH: f64 = 0.0254;
const CP_TARGET_CALIBERS: f64 = 0.5;
const CN_ALPHA_TARGET: f64 = 0.15;

/// The Mach numbers compared against the export: its rows from 0.1 to 2.0, closer together
/// through the transonic region.
pub const CALISTO_MACHS: &[f64] = &[
    0.1, 0.3, 0.5, 0.7, 0.8, 0.9, 0.95, 1.0, 1.05, 1.1, 1.2, 1.3, 1.5, 1.75, 2.0,
];

/// Niskanen 2009 Table 3.1's regions: subsonic to 0.8, transonic to 1.2, supersonic beyond.
pub fn band(mach: f64) -> &'static str {
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
    let mut references = vec![calisto(root)?];
    references.extend(wind_tunnel(root)?);
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "hpr's normal-force slope (per radian, on its reference area) and centre of \
                 pressure (m aft of the nose tip) against each reference's, measured the same \
                 way (each reference says how). cn_alpha_error is hpr's over the reference's, \
                 minus 1; cp_error_calibers is hpr's CP less the reference's, over the reference \
                 diameter, positive aft. Bands are Niskanen 2009 Table 3.1's.",
        "targets": {
            "cp_calibers": CP_TARGET_CALIBERS,
            "cn_alpha_rel": CN_ALPHA_TARGET,
        },
        "references": references,
    }))
}

/// One compared row: the reference's slope and CP against hpr's.
fn compare(
    mach: f64,
    (cn_alpha, cp_m): (f64, f64),
    (hpr_cn_alpha, hpr_cp_m): (f64, f64),
    diameter_m: f64,
) -> Value {
    let cn_alpha_error = hpr_cn_alpha / cn_alpha - 1.0;
    let cp_error_calibers = (hpr_cp_m - cp_m) / diameter_m;
    json!({
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
    })
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

/// The least-squares slope of `ys` against `xs`, with an intercept.
pub fn slope(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let sxy: f64 = xs.iter().zip(ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let sxx: f64 = xs.iter().map(|x| (x - mx) * (x - mx)).sum();
    sxy / sxx
}

/// hpr's normal-force coefficient and its moment about the nose tip (per unit dynamic pressure
/// and reference area, m) at `alpha_deg`, of the whole rocket or of its bodies alone, extended
/// oddly to negative angles. Four fins make both independent of roll.
pub fn hpr_force(
    model: &AeroModel,
    mach: f64,
    alpha_deg: f64,
    bodies_only: bool,
) -> Result<(f64, f64), String> {
    let flow = Flow::new(mach, alpha_deg.abs().to_radians(), 0.0);
    let parts = model
        .components(&flow)
        .map_err(|e| format!("Mach {mach}: {e}"))?;
    let kept = if bodies_only {
        &parts[..model.bodies().len()]
    } else {
        &parts[..]
    };
    let sign = alpha_deg.signum();
    Ok((
        sign * kept.iter().map(|p| p.normal_force.coefficient).sum::<f64>(),
        sign * kept.iter().map(|p| p.normal_force.moment_m).sum::<f64>(),
    ))
}

/// The angles of attack at which the reports give the CP, "low angles": −2° to 2°.
pub const CP_ANGLES_DEG: [f64; 5] = [-2.0, -1.0, 0.0, 1.0, 2.0];

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
    let (secant_col, secant_cp_col) = (
        column("CNalpha (0 to 4 deg) (per rad)")?,
        column("CP (0 to 4 deg)")?,
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
            get(secant_col)?,
            get(secant_cp_col)?,
        ));
    }
    let find = |mach: f64, alpha: f64| {
        table
            .iter()
            .find(|r| (r.0 - mach).abs() < 1e-9 && r.1 == alpha)
            .ok_or(format!("{EXPORT} has no row at Mach {mach}, alpha {alpha}"))
    };
    let mut rows = Vec::new();
    // The same rows against the export's other pair of columns, its secant slope and CP to 4°,
    // which carry its viscous crossflow lift. Only the summary is committed: the fixture holds
    // the export's values at the compared Mach numbers (ADR-009, ADR-027), and this is a second
    // reading of the same rows, so it is reported as counts and one row's error.
    let mut secant_rows_from_0_8 = 0;
    let mut secant_within_from_0_8 = 0;
    let mut secant_at_mach_2 = 0.0;
    let mut smallest_margin: Option<(f64, f64)> = None;
    for &mach in CALISTO_MACHS {
        let two = find(mach, 2.0)?;
        let zero = find(mach, 0.0)?;
        let cn_alpha = two.2 / 2.0_f64.to_radians();
        let hpr = hpr_at(&model, mach)?;
        rows.push(compare(mach, (cn_alpha, zero.3 * INCH), hpr, diameter_m));
        let secant = compare(mach, (zero.4, zero.5 * INCH), hpr, diameter_m);
        let (error, margin) = (
            secant["cn_alpha_error"].as_f64().unwrap_or(f64::NAN),
            CP_TARGET_CALIBERS
                - secant["cp_error_calibers"]
                    .as_f64()
                    .unwrap_or(f64::NAN)
                    .abs(),
        );
        if (mach - 2.0).abs() < 1e-9 {
            secant_at_mach_2 = error;
        }
        if mach >= 0.8 {
            secant_rows_from_0_8 += 1;
            if secant["within_targets"].as_bool().unwrap_or(false) {
                secant_within_from_0_8 += 1;
                if smallest_margin.is_none_or(|(_, m)| margin < m) {
                    smallest_margin = Some((mach, margin));
                }
            }
        }
    }
    let (tightest_mach, tightest_margin) = smallest_margin.ok_or(format!(
        "{EXPORT}: no row from Mach 0.8 passes on the secant columns"
    ))?;
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
        "secant_comparison": json!({
            "note": "The same comparison against the export's `CNalpha (0 to 4 deg) (per rad)` \
                     and `CP (0 to 4 deg)` columns, its secant slope and centre of pressure to 4 \
                     degrees, which carry a viscous crossflow term hpr's small-angle slope leaves \
                     out. Summary only: the rows themselves would commit a second reading of the \
                     export (ADR-009, ADR-027).",
            "rows_from_mach_0_8": secant_rows_from_0_8,
            "rows_within_targets_from_mach_0_8": secant_within_from_0_8,
            "tightest_pass_mach": tightest_mach,
            "tightest_pass_cp_margin_calibers": tightest_margin,
            "mach_2_cn_alpha_error": secant_at_mach_2,
        }),
    }))
}

/// Each wind-tunnel configuration's rows: at every Mach number both reports give, the least-squares
/// slope of the plotted `C_N` against the angle of attack, fins on at 0° and fins off, and the
/// plotted CP, against hpr's `C_N` fitted at the same angles and hpr's CP from its moment and
/// normal force fitted over −2° to 2°.
fn wind_tunnel(root: &Path) -> Result<Vec<Value>, String> {
    let path = root.join(WIND_TUNNEL);
    let text = fs::read_to_string(&path).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let reference: Value =
        serde_json::from_str(&text).map_err(|e| format!("{WIND_TUNNEL}: {e}"))?;
    let configurations = reference["configurations"]
        .as_array()
        .ok_or(format!("{WIND_TUNNEL} has no configurations"))?;
    let mut out = Vec::new();
    for configuration in configurations {
        let text_of = |key: &str| {
            configuration[key]
                .as_str()
                .ok_or(format!("{WIND_TUNNEL}: a configuration without `{key}`"))
        };
        let design = text_of("design")?;
        let (_, model) = load_design(root, design)?;
        let diameter_m = (4.0 * model.reference_area_m2() / std::f64::consts::PI).sqrt();
        let length_m = configuration["length_m"]
            .as_f64()
            .ok_or(format!("{WIND_TUNNEL}: a configuration without `length_m`"))?;
        let series = |key: &str| {
            configuration[key]
                .as_array()
                .ok_or(format!("{WIND_TUNNEL}: a configuration without `{key}`"))
        };
        let slopes =
            |key: &str, bodies_only: bool| -> Result<Vec<(f64, f64, f64, Value)>, String> {
                series(key)?
                    .iter()
                    .map(|curve| {
                        let mach = curve["mach"].as_f64().ok_or("a curve without `mach`")?;
                        let points: Vec<(f64, f64)> = curve["alpha_deg_c_n"]
                            .as_array()
                            .ok_or("a curve without points")?
                            .iter()
                            .map(|p| Some((p[0].as_f64()?, p[1].as_f64()?)))
                            .collect::<Option<_>>()
                            .ok_or("an unreadable point")?;
                        let alphas: Vec<f64> = points.iter().map(|p| p.0.to_radians()).collect();
                        let measured: Vec<f64> = points.iter().map(|p| p.1).collect();
                        let hpr: Vec<f64> = points
                            .iter()
                            .map(|p| hpr_force(&model, mach, p.0, bodies_only).map(|f| f.0))
                            .collect::<Result<_, _>>()?;
                        Ok((
                            mach,
                            slope(&alphas, &measured),
                            slope(&alphas, &hpr),
                            curve["source"].clone(),
                        ))
                    })
                    .collect()
            };
        let fins_on = slopes("cn_alpha", false)?;
        let fins_off = slopes("cn_alpha_fins_off", true)?;
        let mut rows = Vec::new();
        for cp in series("cp")? {
            let get = |key: &str| {
                cp[key]
                    .as_f64()
                    .ok_or(format!("{WIND_TUNNEL}: a CP without `{key}`"))
            };
            let mach = get("mach")?;
            let &(_, measured, hpr, ref source) = fins_on
                .iter()
                .find(|s| s.0 == mach)
                .ok_or(format!("{WIND_TUNNEL}: no C_N curve at Mach {mach}"))?;
            let forces: Vec<(f64, f64)> = CP_ANGLES_DEG
                .iter()
                .map(|&a| hpr_force(&model, mach, a, false))
                .collect::<Result<_, _>>()?;
            let alphas: Vec<f64> = CP_ANGLES_DEG.iter().map(|a| a.to_radians()).collect();
            let normal: Vec<f64> = forces.iter().map(|f| f.0).collect();
            let moment: Vec<f64> = forces.iter().map(|f| f.1).collect();
            let hpr_cp = slope(&alphas, &moment) / slope(&alphas, &normal);
            let mut row = compare(
                mach,
                (measured, 0.01 * get("percent_length")? * length_m),
                (hpr, hpr_cp),
                diameter_m,
            );
            row["cp_uncertainty_calibers"] =
                json!(0.01 * get("uncertainty_percent")? * length_m / diameter_m);
            row["cn_alpha_source"] = source.clone();
            row["cp_source"] = cp["source"].clone();
            if let Some(&(_, body, hpr_body, ref body_source)) =
                fins_off.iter().find(|s| s.0 == mach)
            {
                row["reference_body_cn_alpha_per_rad"] = json!(body);
                row["hpr_body_cn_alpha_per_rad"] = json!(hpr_body);
                row["body_source"] = body_source.clone();
            }
            rows.push(row);
        }
        out.push(json!({
            "id": configuration["id"],
            "design": design,
            "reference_diameter_m": diameter_m,
            "source": format!(
                "{} C_N_alpha: the least-squares slope, with an intercept, of the plotted C_N \
                 against the angle of attack, fins at 0 degrees, and hpr's C_N fitted at the same \
                 angles; the body's the same with the fins off. CP: the plotted CP at low angles \
                 of attack, and hpr's from its moment and normal force fitted over -2 to 2 \
                 degrees. A measurement.",
                reference["source"].as_str().unwrap_or_default()
            ),
            "rows": rows,
        }));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture(root: &Path) -> Value {
        let text = fs::read_to_string(root.join(FIXTURE)).unwrap();
        serde_json::from_str(&text).unwrap()
    }

    /// A signed percentage as the guide writes it: one decimal, a Unicode minus.
    fn pct(x: f64) -> String {
        format!("{:+.1}%", 100.0 * x).replace('-', "−")
    }

    /// A signed CP difference in calibres as the guide writes it: two decimals, a Unicode minus.
    fn cal(x: f64) -> String {
        format!("{x:+.2}").replace('-', "−")
    }

    /// `docs/physics/aero.md` quotes this fixture: the summary table's ranges and counts, the two
    /// Calisto rows the prose names, and the secant comparison's summary. Without this the
    /// largest table in the guide could drift from the fixture unnoticed, which is how
    /// [M1.8e9](https://github.com/nrdptel/hpr-sim/pull/103) found three stale numbers.
    #[test]
    fn the_guide_quotes_the_fixture() {
        let root = crate::designs::root().unwrap();
        let fixture = fixture(&root);
        let guide = fs::read_to_string(root.join("docs/physics/aero.md")).unwrap();
        // Sentences wrap across lines; the table's rows don't.
        let joined = guide.split_whitespace().collect::<Vec<_>>().join(" ");
        let groups: [(&str, &str, &[f64]); 6] = [
            ("arcas-robin-long", "Arcas, long, 0.6 and 0.8", &[0.6, 0.8]),
            (
                "arcas-robin-long",
                "Arcas, long, 0.9 to 1.2",
                &[0.9, 1.0, 1.2],
            ),
            (
                "arcas-robin-long",
                "Arcas, long, 1.8 to 2.96",
                &[1.8, 2.3, 2.96],
            ),
            (
                "arcas-robin-long",
                "Arcas, long, 3.96 and 4.63",
                &[3.96, 4.63],
            ),
            (
                "calisto-rasaero-ii",
                "Calisto against RASAero II, 0.1 to 0.7",
                &[0.1, 0.3, 0.5, 0.7],
            ),
            (
                "calisto-rasaero-ii",
                "Calisto against RASAero II, 0.8 to 2.0",
                &[0.8, 0.9, 0.95, 1.0, 1.05, 1.1, 1.2, 1.3, 1.5, 1.75, 2.0],
            ),
        ];
        for (id, label, machs) in groups {
            let rows = fixture["references"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == id)
                .unwrap()["rows"]
                .as_array()
                .unwrap();
            let pick = |mach: f64| {
                rows.iter()
                    .find(|r| (r["mach"].as_f64().unwrap() - mach).abs() < 1e-9)
                    .unwrap_or_else(|| panic!("{id} has no row at Mach {mach}"))
            };
            let cn: Vec<f64> = machs
                .iter()
                .map(|m| pick(*m)["cn_alpha_error"].as_f64().unwrap())
                .collect();
            let cp: Vec<f64> = machs
                .iter()
                .map(|m| pick(*m)["cp_error_calibers"].as_f64().unwrap())
                .collect();
            let within = machs
                .iter()
                .filter(|m| pick(**m)["within_targets"].as_bool().unwrap())
                .count();
            let span = |values: &[f64], write: &dyn Fn(f64) -> String| {
                let low = values.iter().copied().fold(f64::MAX, f64::min);
                let high = values.iter().copied().fold(f64::MIN, f64::max);
                if values.len() == 2 {
                    format!("{}, {}", write(values[0]), write(values[1]))
                } else {
                    format!("{} to {}", write(low), write(high))
                }
            };
            let line = format!(
                "| {label} | {} | {} | {within} of {} |",
                span(&cn, &pct),
                span(&cp, &cal),
                machs.len()
            );
            assert!(
                guide.contains(&line),
                "aero.md doesn't have the row `{line}`"
            );
        }
        // The two Calisto rows the prose names, and the secant comparison it warns with.
        let calisto = fixture["references"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "calisto-rasaero-ii")
            .unwrap();
        let at = |mach: f64| {
            calisto["rows"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| (r["mach"].as_f64().unwrap() - mach).abs() < 1e-9)
                .unwrap()["cn_alpha_error"]
                .as_f64()
                .unwrap()
        };
        let sentence = format!(
            "Mach 1.5 reads {} and Mach 2 {}",
            pct(at(1.5)),
            pct(at(2.0))
        );
        assert!(
            joined.contains(&sentence),
            "aero.md doesn't say `{sentence}`"
        );
        let secant = &calisto["secant_comparison"];
        let claim = format!(
            "{} of the {} rows from Mach 0.8 are within the targets",
            secant["rows_within_targets_from_mach_0_8"], secant["rows_from_mach_0_8"]
        );
        assert!(joined.contains(&claim), "aero.md doesn't say `{claim}`");
        let margin = format!(
            "Mach {} by {:.5} calibres",
            secant["tightest_pass_mach"].as_f64().unwrap(),
            secant["tightest_pass_cp_margin_calibers"].as_f64().unwrap()
        );
        assert!(joined.contains(&margin), "aero.md doesn't say `{margin}`");
        let mach_2 = format!(
            "Mach 2 is {}",
            pct(secant["mach_2_cn_alpha_error"].as_f64().unwrap())
        );
        assert!(joined.contains(&mach_2), "aero.md doesn't say `{mach_2}`");
    }
}
