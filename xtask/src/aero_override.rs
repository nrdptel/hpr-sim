//! Normal-force overrides (M1.8d): RASAero II's Calisto export read by
//! [`NormalForceTable::from_rasaero_csv`] and flown in place of hpr's normal force, written by
//! `cargo xtask aero` to `validation/fixtures/aero/normal-force-override.json`.
//!
//! - **The reading**: each angle of attack's column (how many Mach numbers, from where to where);
//!   every row of the export read again here, apart from the library, and the largest relative
//!   difference from the library's table (its `CN` and `CP` at each row's Mach number and angle);
//!   a hash of the whole table; and the 0° column at the Mach numbers M1.8a compared
//!   (`crate::aero_mach`), which a test checks against M1.8a's own reading in CI. The export stays
//!   in `refs/` (ADR-009), so the rest is checked when `cargo xtask aero --check` runs with it, as
//!   `aero::tests` does locally; CI has no `refs/`.
//! - **What the export says about its angles**: the spread of its potential-flow slope between 2°
//!   and 4° (the 0° slope rests on it being linear in the angle), and how its viscous part grows
//!   from 2° to 4° at a few Mach numbers (4 would be `α²`).
//! - **The flights**: Calisto from a 5.2 m rail at 85° in a 5 m/s wind across it, with hpr's own
//!   normal force; with hpr's own as a table at the export's angles and Mach numbers (the control:
//!   what the table's method alone changes); with the whole export; and with the 0° column at the
//!   compared Mach numbers only, which needs nothing outside the repository, so a test flies it
//!   again in CI. Each flight records how long it flew past 4°, the export's last angle.

use std::fs;
use std::path::Path;

use hpr_aero::{AeroModel, Flow, NormalForceColumn, NormalForceTable};
use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_core::interp::{Extrapolation, Interpolation, Table1D};
use hpr_design::Rocket;
use hpr_sim::{
    Environment, EventKind, FlightSettings, FlightStep, Observer, Phase, Rail, SimError, Simulation,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::aero_mach::{CALISTO_DESIGN, CALISTO_MACHS, EXPORT};

pub const FIXTURE: &str = "validation/fixtures/aero/normal-force-override.json";

/// The flight's setup, as the fixture states it.
const SETUP: &str = "Calisto (the design M1.8a compared) on its own motor, from a 5.2 m rail at \
                     85 degrees heading north at Spaceport America (32.990254 N, 106.974998 W, \
                     1400 m), in the 1976 standard atmosphere with a constant 5 m/s wind from \
                     the west, hpr's own drag; the largest Mach number, and the time in free \
                     flight before apogee past 4 degrees (the export's last angle), are taken at \
                     the integrator's step ends.";

/// The export's last angle of attack, rad: past it the table continues by its assumption.
const LAST_ANGLE_RAD: f64 = 4.0 * std::f64::consts::PI / 180.0;

/// The fixture, from the committed Calisto design and the export in `refs/`.
pub fn generate(root: &Path) -> Result<Value, String> {
    let bytes = fs::read(root.join(EXPORT))
        .map_err(|e| format!("{EXPORT}: {e} (run `cargo xtask refs fetch`)"))?;
    let text = std::str::from_utf8(&bytes).map_err(|e| format!("{EXPORT}: {e}"))?;
    let table = NormalForceTable::from_rasaero_csv(text).map_err(|e| format!("{EXPORT}: {e}"))?;
    let columns: Vec<Value> = table
        .columns()
        .iter()
        .map(|column| {
            let machs = column.slope_per_rad.xs();
            json!({
                "alpha_deg": (column.alpha_rad.to_degrees() * 1e9).round() / 1e9,
                "mach_numbers": machs.len(),
                "mach_from": machs.first(),
                "mach_to": machs.last(),
            })
        })
        .collect();
    let zero = table
        .columns()
        .first()
        .filter(|column| column.alpha_rad == 0.0)
        .ok_or(format!("{EXPORT} has no rows at 0 degrees"))?;
    let mut rows = Vec::new();
    for &mach in CALISTO_MACHS {
        let read = |t: &Table1D| t.eval(mach).map_err(|e| e.to_string());
        rows.push(json!({
            "mach": mach,
            "slope_per_rad": read(&zero.slope_per_rad)?,
            "cp_station_m": read(&zero.cp_station_m)?,
        }));
    }
    let compared = compared_table(&rows)?;
    let export = export_rows(text)?;
    let reread = reread(&table, &export)?;
    let control_machs: Vec<f64> = zero
        .slope_per_rad
        .xs()
        .iter()
        .copied()
        .filter(|&m| m < hpr_aero::NORMAL_FORCE_MACH_LIMIT)
        .collect();
    let control = own_table(root, &[0.0, 2.0, 4.0], &control_machs)?;
    let table_json = serde_json::to_string(&table).map_err(|e| e.to_string())?;
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "RASAero II's Calisto export read by hpr_aero::NormalForceTable::from_rasaero_csv \
                 (slopes per radian on RASAero II's reference, the largest body section, which \
                 is Calisto's; centres of pressure m aft of the nose tip), and Calisto flown \
                 four ways. compared_rows hold the 0 degree column at the Mach numbers of \
                 normal-force-vs-mach.json, which must match its reference values. reread is \
                 every row of the export at a positive angle read again apart from the library: \
                 the largest relative difference of the table's CN and CP from the row's. \
                 table_sha256 hashes the table as serialized. potential_slope_spread is the \
                 largest relative difference of the export's CN Potential over the angle between \
                 2 and 4 degrees; viscous_growth is CN Viscous at 4 degrees over 2 degrees (4 for \
                 a part in the square of the angle).",
        "source": "RASAero II export for Calisto with the 2018 notebook's fins, RocketPy's first \
                   commit's data/calisto/CD Test.CSV (da91db9e, 2018). A code, not a \
                   measurement.",
        "source_sha256": sha256(&bytes),
        "design": CALISTO_DESIGN,
        "columns": columns,
        "reread": reread,
        "table_sha256": sha256(table_json.as_bytes()),
        "potential_slope_spread": potential_slope_spread(&export)?,
        "viscous_growth": viscous_growth(&export, &[1.0, 1.1, 1.5, 2.0, 3.0, 4.0])?,
        "compared_rows": rows,
        "setup": SETUP,
        "flights": [
            fly(root, "hpr's own", None)?,
            fly(root, "hpr's own as a table at the export's angles and Mach numbers", Some(control))?,
            fly(root, "the RASAero II export", Some(table))?,
            fly(root, "the export's 0 degree column at the compared Mach numbers", Some(compared))?,
        ],
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A table of one column at 0° from the fixture's `compared_rows`.
pub fn compared_table(rows: &[Value]) -> Result<NormalForceTable, String> {
    let field = |row: &Value, key: &str| {
        row[key]
            .as_f64()
            .ok_or(format!("a compared row has no `{key}`"))
    };
    let (mut machs, mut slopes, mut cps) = (Vec::new(), Vec::new(), Vec::new());
    for row in rows {
        machs.push(field(row, "mach")?);
        slopes.push(field(row, "slope_per_rad")?);
        cps.push(field(row, "cp_station_m")?);
    }
    let table = |ys| {
        Table1D::new(
            machs.clone(),
            ys,
            Interpolation::Linear,
            Extrapolation::Clamp,
        )
        .map_err(|e| e.to_string())
    };
    NormalForceTable::new(vec![NormalForceColumn::new(
        0.0,
        table(slopes)?,
        table(cps)?,
    )])
    .map_err(|e| e.to_string())
}

/// One row of the export as read here: Mach, the angle in degrees, `CN`, `CN Potential`,
/// `CN Viscous` and `CP` in inches.
struct ExportRow {
    mach: f64,
    alpha_deg: f64,
    cn: f64,
    potential: f64,
    viscous: f64,
    cp_in: f64,
}

/// Every row of the export, read with a plain split, apart from the library's reader.
fn export_rows(text: &str) -> Result<Vec<ExportRow>, String> {
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or(format!("{EXPORT} is empty"))?
        .split(',')
        .map(str::trim)
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|h| *h == name)
            .ok_or(format!("{EXPORT} has no column `{name}`"))
    };
    let columns = [
        column("Mach")?,
        column("Alpha")?,
        column("CN")?,
        column("CN Potential")?,
        column("CN Viscous")?,
        column("CP")?,
    ];
    let mut rows = Vec::new();
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let cells: Vec<&str> = line.split(',').collect();
        let mut values = [0.0; 6];
        for (value, &index) in values.iter_mut().zip(&columns) {
            *value = cells
                .get(index)
                .and_then(|c| c.trim().parse::<f64>().ok())
                .ok_or(format!("{EXPORT}: unreadable row `{line}`"))?;
        }
        let [mach, alpha_deg, cn, potential, viscous, cp_in] = values;
        rows.push(ExportRow {
            mach,
            alpha_deg,
            cn,
            potential,
            viscous,
            cp_in,
        });
    }
    Ok(rows)
}

/// The table's `CN` and `CP` at every row's Mach number and positive angle, against the row's:
/// the largest relative differences, and how many rows.
fn reread(table: &NormalForceTable, rows: &[ExportRow]) -> Result<Value, String> {
    let (mut cn, mut cp, mut count) = (0.0_f64, 0.0_f64, 0);
    for row in rows.iter().filter(|r| r.alpha_deg > 0.0) {
        let lookup = table
            .lookup(row.mach, row.alpha_deg.to_radians())
            .map_err(|e| e.to_string())?;
        cn = cn.max((lookup.coefficient / row.cn - 1.0).abs());
        cp = cp.max((lookup.cp_station_m / (row.cp_in * 0.0254) - 1.0).abs());
        count += 1;
    }
    Ok(json!({ "rows": count, "cn_max_rel": cn, "cp_max_rel": cp }))
}

/// The largest relative difference between the export's `CN Potential/α` at 2° and at 4°, over
/// the Mach numbers both angles have.
fn potential_slope_spread(rows: &[ExportRow]) -> Result<f64, String> {
    let at = |alpha: f64| rows.iter().filter(move |r| r.alpha_deg == alpha);
    let mut spread: f64 = 0.0;
    let mut matched = 0;
    for four in at(4.0) {
        if let Some(two) = at(2.0).find(|r| r.mach == four.mach) {
            let (slope_two, slope_four) = (two.potential / 2.0, four.potential / 4.0);
            spread = spread.max((slope_four / slope_two - 1.0).abs());
            matched += 1;
        }
    }
    if matched == 0 {
        return Err(format!(
            "{EXPORT} has no Mach number at both 2 and 4 degrees"
        ));
    }
    Ok(spread)
}

/// `CN Viscous` at 4° over 2° at each of `machs`.
fn viscous_growth(rows: &[ExportRow], machs: &[f64]) -> Result<Vec<Value>, String> {
    machs
        .iter()
        .map(|&mach| {
            let at = |alpha: f64| {
                rows.iter()
                    .find(|r| r.alpha_deg == alpha && (r.mach - mach).abs() < 1e-9)
                    .map(|r| r.viscous)
                    .ok_or(format!(
                        "{EXPORT} has no row at Mach {mach}, {alpha} degrees"
                    ))
            };
            Ok(json!({ "mach": mach, "ratio": at(4.0)? / at(2.0)? }))
        })
        .collect()
}

/// hpr's own normal force for Calisto, as the flight sums it (fin sets' force times `sin α/α`,
/// ADR-011), as a table at `alphas_deg` and `machs`.
fn own_table(root: &Path, alphas_deg: &[f64], machs: &[f64]) -> Result<NormalForceTable, String> {
    let aero: AeroModel = simulation(root, "the control")?.aero().clone();
    let mut columns = Vec::new();
    for &alpha_deg in alphas_deg {
        let alpha = alpha_deg.to_radians();
        let (mut slopes, mut cps) = (Vec::new(), Vec::new());
        for &mach in machs {
            if alpha == 0.0 {
                let force = aero
                    .normal_force(&Flow::axial(mach))
                    .map_err(|e| e.to_string())?;
                slopes.push(force.slope_per_rad);
                cps.push(force.cp_station_m.ok_or("no centre of pressure")?);
                continue;
            }
            let (mut force, mut moment) = (0.0, 0.0);
            let components = aero
                .components(&Flow::new(mach, alpha, 0.0))
                .map_err(|e| e.to_string())?;
            for (index, component) in components.iter().enumerate() {
                let scale = if index >= aero.bodies().len() {
                    alpha.sin() / alpha
                } else {
                    1.0
                };
                force += component.normal_force.coefficient * scale;
                moment += component.normal_force.moment_m * scale;
            }
            slopes.push(force / alpha);
            cps.push(moment / force);
        }
        let table = |ys| {
            Table1D::new(
                machs.to_vec(),
                ys,
                Interpolation::Linear,
                Extrapolation::Clamp,
            )
            .map_err(|e| e.to_string())
        };
        columns.push(NormalForceColumn::new(alpha, table(slopes)?, table(cps)?));
    }
    NormalForceTable::new(columns).map_err(|e| e.to_string())
}

/// The largest Mach number, and the free-flight time before apogee past the export's last angle,
/// at the integrator's step ends.
#[derive(Default)]
struct Peaks {
    max_mach: f64,
    past_last_angle_s: f64,
}

impl Observer for Peaks {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let sample = step.sample(step.end_s())?;
        self.max_mach = self.max_mach.max(sample.mach);
        if sample.phase == Phase::Free
            && sample.vertical_speed_m_s > 0.0
            && sample.angle_of_attack_rad > LAST_ANGLE_RAD
        {
            self.past_last_angle_s += step.end_s() - step.start_s();
        }
        Ok(())
    }
}

/// Calisto set up as [`SETUP`] says.
fn simulation(root: &Path, label: &str) -> Result<Simulation, String> {
    let path = root.join("validation/designs").join(CALISTO_DESIGN);
    let text = fs::read_to_string(&path).map_err(|e| format!("{CALISTO_DESIGN}: {e}"))?;
    let rocket: Rocket =
        serde_json::from_str(&text).map_err(|e| format!("{CALISTO_DESIGN}: {e}"))?;
    let e = |e: &dyn std::fmt::Display| format!("{label}: {e}");
    let site = Geodetic::from_degrees(32.990254, -106.974998, 1400.0).map_err(|x| e(&x))?;
    let wind = ConstantWind::new(5.0, 270_f64.to_radians()).map_err(|x| e(&x))?;
    let environment = Environment::standard(site)
        .map_err(|x| e(&x))?
        .with_wind(wind);
    let rail = Rail {
        length_m: 5.2,
        azimuth_rad: 0.0,
        elevation_rad: 85_f64.to_radians(),
        roll_rad: 0.0,
        friction_coefficient: 0.0,
    };
    Simulation::new(
        &rocket,
        "example",
        environment,
        rail,
        FlightSettings::default(),
    )
    .map_err(|x| e(&x))
}

/// Calisto flown as [`SETUP`] says, with `table` in place of hpr's normal force when given.
pub fn fly(root: &Path, label: &str, table: Option<NormalForceTable>) -> Result<Value, String> {
    let e = |e: &dyn std::fmt::Display| format!("{label}: {e}");
    let mut simulation = simulation(root, label)?;
    if let Some(table) = table {
        simulation = simulation
            .with_normal_force_table(table)
            .map_err(|x| e(&x))?;
    }
    let mut peaks = Peaks::default();
    let result = simulation.run(&mut peaks).map_err(|x| e(&x))?;
    let at = |kind| {
        result
            .event(kind)
            .map(|event| event.sample)
            .ok_or(format!("{label}: the flight has no {kind:?}"))
    };
    let (apogee, burnout) = (at(EventKind::Apogee)?, at(EventKind::Burnout)?);
    Ok(json!({
        "normal_force": label,
        "apogee_m": apogee.height_above_ground_m,
        "apogee_time_s": apogee.time_s,
        "apogee_east_m": apogee.cg_enu_m.x,
        "apogee_north_m": apogee.cg_enu_m.y,
        "burnout_angle_of_attack_deg": burnout.angle_of_attack_rad.to_degrees(),
        "max_mach": peaks.max_mach,
        "past_4_degrees_s": peaks.past_last_angle_s,
    }))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    fn fixture(name: &str) -> Value {
        let root = crate::designs::root().unwrap();
        serde_json::from_str(&std::fs::read_to_string(root.join(name)).unwrap()).unwrap()
    }

    fn close(got: f64, want: f64, rel: f64, what: &str) {
        assert!(
            (got - want).abs() <= rel * want.abs(),
            "{what}: {got} against {want}"
        );
    }

    /// The library's reader and M1.8a's own parser give the same slope and centre of pressure at
    /// 0° at every Mach number M1.8a compared: the reading tested on the Calisto export without
    /// the export.
    #[test]
    fn the_reader_agrees_with_m1_8a_on_the_calisto_export() {
        let ours = fixture(super::FIXTURE);
        let theirs = fixture(crate::aero_mach::FIXTURE);
        let reference = theirs["references"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == "calisto-rasaero-ii")
            .unwrap();
        assert_eq!(ours["source_sha256"], reference["source_sha256"]);
        let rows = ours["compared_rows"].as_array().unwrap();
        let theirs_rows = reference["rows"].as_array().unwrap();
        assert_eq!(rows.len(), theirs_rows.len());
        for (row, other) in rows.iter().zip(theirs_rows) {
            let mach = row["mach"].as_f64().unwrap();
            assert_eq!(other["mach"].as_f64().unwrap(), mach);
            let what = format!("Mach {mach}");
            close(
                row["slope_per_rad"].as_f64().unwrap(),
                other["reference_cn_alpha_per_rad"].as_f64().unwrap(),
                1e-12,
                &what,
            );
            close(
                row["cp_station_m"].as_f64().unwrap(),
                other["reference_cp_m"].as_f64().unwrap(),
                1e-12,
                &what,
            );
        }
    }

    /// The flight on the compared rows needs nothing outside the repository: flown again here,
    /// it lands where the fixture says, within the 1e-7 by which whole flights may differ across
    /// platforms (M2.1b2).
    #[test]
    fn the_compared_rows_fly_as_recorded() {
        let root = crate::designs::root().unwrap();
        let committed = fixture(super::FIXTURE);
        let table = super::compared_table(committed["compared_rows"].as_array().unwrap()).unwrap();
        let recorded = &committed["flights"][3];
        let flown = super::fly(
            &root,
            recorded["normal_force"].as_str().unwrap(),
            Some(table),
        )
        .unwrap();
        for key in ["apogee_m", "apogee_time_s", "max_mach"] {
            close(
                flown[key].as_f64().unwrap(),
                recorded[key].as_f64().unwrap(),
                1e-7,
                key,
            );
        }
        // Where it lands, within a millimetre.
        for key in ["apogee_east_m", "apogee_north_m"] {
            let (got, want) = (
                flown[key].as_f64().unwrap(),
                recorded[key].as_f64().unwrap(),
            );
            assert!((got - want).abs() < 1e-3, "{key}: {got} against {want}");
        }
        // It isn't hpr's own flight: the table moved it by metres.
        let own = &committed["flights"][0];
        let moved =
            flown["apogee_east_m"].as_f64().unwrap() - own["apogee_east_m"].as_f64().unwrap();
        assert!(moved.abs() > 1.0, "{moved}");
    }
}
