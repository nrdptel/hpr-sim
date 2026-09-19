//! Normal-force overrides (M1.8d): RASAero II's Calisto export read by
//! [`NormalForceTable::from_rasaero_csv`] and flown in place of hpr's normal force, written by
//! `cargo xtask aero` to `validation/fixtures/aero/normal-force-override.json`.
//!
//! - **The reading**: each angle of attack's column (how many Mach numbers, from where to where),
//!   and the 0° column's slope and centre of pressure at the Mach numbers M1.8a compared
//!   (`crate::aero_mach`). M1.8a read those from the export with its own parser; a test checks the
//!   two readings agree, so the library's reader is tested on the real export in CI without the
//!   export, which stays in `refs/` (ADR-009). The spread of the export's potential-flow slope
//!   between 2° and 4° is recorded too: the 0° slope rests on it being linear in the angle.
//! - **The flights**: Calisto from a 5.2 m rail at 85° in a 5 m/s wind across it, with hpr's own
//!   normal force, with the whole export's, and with the 0° column at the compared Mach numbers
//!   only. The last needs nothing outside the repository, so a test flies it again in CI.

use std::fs;
use std::path::Path;

use hpr_aero::{NormalForceColumn, NormalForceTable};
use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_core::interp::{Extrapolation, Interpolation, Table1D};
use hpr_design::Rocket;
use hpr_sim::{
    Environment, EventKind, FlightSettings, FlightStep, Observer, Rail, SimError, Simulation,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::aero_mach::{CALISTO_DESIGN, CALISTO_MACHS, EXPORT};

pub const FIXTURE: &str = "validation/fixtures/aero/normal-force-override.json";

/// The flight's setup, as the fixture states it.
const SETUP: &str = "Calisto (the design M1.8a compared) on its own motor, from a 5.2 m rail at \
                     85 degrees heading north at Spaceport America (32.990254 N, 106.974998 W, \
                     1400 m), in the 1976 standard atmosphere with a constant 5 m/s wind from \
                     the west, hpr's own drag; the largest Mach number is taken at the \
                     integrator's step ends.";

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
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "RASAero II's Calisto export read by hpr_aero::NormalForceTable::from_rasaero_csv \
                 (slopes per radian on the export's reference area, taken as the rocket's; \
                 centres of pressure m aft of the nose tip), and Calisto flown with hpr's own \
                 normal force and with the export's. compared_rows hold the 0 degree column \
                 at the Mach numbers of normal-force-vs-mach.json, which must match its \
                 reference values. potential_slope_spread is the largest relative difference \
                 of the export's CN Potential over the angle between 2 and 4 degrees, over \
                 every Mach number both have.",
        "source": "RASAero II export for Calisto with the 2018 notebook's fins, RocketPy's first \
                   commit's data/calisto/CD Test.CSV (da91db9e, 2018). A code, not a \
                   measurement.",
        "source_sha256": Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "design": CALISTO_DESIGN,
        "columns": columns,
        "potential_slope_spread": potential_slope_spread(text)?,
        "compared_rows": rows,
        "setup": SETUP,
        "flights": [
            fly(root, "hpr's own", None)?,
            fly(root, "the RASAero II export", Some(table))?,
            fly(root, "the export's 0 degree column at the compared Mach numbers", Some(compared))?,
        ],
    }))
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

/// The largest relative difference between the export's `CN Potential/α` at 2° and at 4°, over
/// the Mach numbers both angles have, read here apart from the library's reader.
fn potential_slope_spread(text: &str) -> Result<f64, String> {
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
    let (mach_col, alpha_col, potential_col) =
        (column("Mach")?, column("Alpha")?, column("CN Potential")?);
    let (mut two, mut four) = (Vec::new(), Vec::new());
    for line in lines.filter(|l| !l.trim().is_empty()) {
        let cells: Vec<&str> = line.split(',').collect();
        let get = |i: usize| -> Result<f64, String> {
            cells
                .get(i)
                .and_then(|c| c.trim().parse::<f64>().ok())
                .ok_or(format!("{EXPORT}: unreadable row `{line}`"))
        };
        let (mach, alpha, potential) = (get(mach_col)?, get(alpha_col)?, get(potential_col)?);
        if alpha == 2.0 {
            two.push((mach, potential / 2.0_f64.to_radians()));
        } else if alpha == 4.0 {
            four.push((mach, potential / 4.0_f64.to_radians()));
        }
    }
    let mut spread: f64 = 0.0;
    let mut matched = 0;
    for (mach, slope) in &four {
        if let Some((_, at_two)) = two.iter().find(|(m, _)| m == mach) {
            spread = spread.max((slope / at_two - 1.0).abs());
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

/// The largest Mach number at the integrator's step ends.
#[derive(Default)]
struct Peaks {
    max_mach: f64,
}

impl Observer for Peaks {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        self.max_mach = self.max_mach.max(step.sample(step.end_s())?.mach);
        Ok(())
    }
}

/// Calisto flown as [`SETUP`] says, with `table` in place of hpr's normal force when given.
pub fn fly(root: &Path, label: &str, table: Option<NormalForceTable>) -> Result<Value, String> {
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
    let mut simulation = Simulation::new(
        &rocket,
        "example",
        environment,
        rail,
        FlightSettings::default(),
    )
    .map_err(|x| e(&x))?;
    if let Some(table) = table {
        simulation = simulation.with_normal_force_table(table);
    }
    let mut peaks = Peaks::default();
    let result = simulation.run(&mut peaks).map_err(|x| e(&x))?;
    let at = |kind| {
        result
            .event(kind)
            .map(|event| event.sample.clone())
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
        let recorded = &committed["flights"][2];
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
        // Against hpr's own normal force, the table moves the flight.
        let own = &committed["flights"][0];
        assert_ne!(own["apogee_m"], recorded["apogee_m"]);
    }
}
