//! `cargo xtask aero [--check]`: compares hpr's subsonic drag with the drag curves that ship with
//! RocketPy's example rockets, and writes the derived numbers to
//! `validation/fixtures/aero/rocketpy-drag-curves.json`.
//!
//! RocketPy's data files carry their own terms (`THIRD-PARTY-NOTICES.md`), so the curves are read
//! from the `refs/rocketpy` checkout (`cargo xtask refs fetch`) and never committed. The fixture
//! holds only what is derived from them: hpr's `C_D0` at the comparison Mach number, its relative
//! error against the curve there, and each curve's sha256. `hpr_aero`'s tests recompute hpr's
//! values from the committed designs and check the recorded errors, so CI needs no `refs/`.
//! `--check` recomputes everything from `refs/` and fails if the committed fixture differs.

use std::fs;
use std::path::Path;

use hpr_aero::{AeroModel, DragConditions, Flow, parse_mach_csv};
use hpr_atmos::Ussa76;
use hpr_design::Rocket;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const USAGE: &str = "\
  aero [--check]           Compare hpr's drag at Mach 0.3 with the drag curves of RocketPy's
                           example rockets (from refs/rocketpy) and write
                           validation/fixtures/aero/rocketpy-drag-curves.json. --check fails
                           if the committed fixture differs instead of writing.";

const FIXTURE: &str = "validation/fixtures/aero/rocketpy-drag-curves.json";
const CHECKOUT: &str = "refs/rocketpy";
const MACH: f64 = 0.3;
const TOLERANCE: f64 = 0.10;

/// A comparison: an hpr design, a RocketPy drag curve, and what the curve is. `variant_of` names
/// the case whose curve a variant shares; variants are reported but aren't separate evidence.
struct Case {
    id: &'static str,
    design: &'static str,
    curve: &'static str,
    thrusting: bool,
    variant_of: Option<&'static str>,
    origin: &'static str,
}

/// Every RocketPy example whose drag curve is labelled RASAero: power-off, and power-on wherever
/// the example has a separate power-on curve that differs from its power-off curve. Calisto's
/// power-on file is byte-identical to its power-off file, and Juno III uses one file for both.
const CASES: &[Case] = &[
    Case {
        id: "calisto-power-off",
        design: "rocketpy-calisto-tests-motor-at-minus-1.373.json",
        curve: "data/rockets/calisto/powerOffDragCurve.csv",
        thrusting: false,
        variant_of: None,
        origin: "RASAero II export: the alpha-0 `CD Power-Off` column of RocketPy's first commit's \
                 `data/calisto/CD Test.CSV` (da91db9e, 2018), rounded to 9 decimals and cut to \
                 Mach 2; the export's power-on column equals its power-off column. The design has \
                 the 2018 notebook's fins; its rail buttons come from RocketPy's calisto_robust \
                 test fixture, not the notebook",
    },
    Case {
        id: "calisto-getting-started-power-off",
        design: "rocketpy-calisto-getting-started-motor-at-minus-1.255.json",
        curve: "data/rockets/calisto/powerOffDragCurve.csv",
        thrusting: false,
        variant_of: Some("calisto-power-off"),
        origin: "the same RASAero II export, for the larger NACA 0012 fins RocketPy's \
                 getting-started example gives Calisto (tip chord 0.06 m, span 0.11 m, against \
                 the 2018 notebook's 0.04 m and 0.10 m)",
    },
    Case {
        id: "juno-iii-power-off",
        design: "rocketpy-juno-iii.json",
        curve: "data/rockets/juno3/drag_curve.csv",
        thrusting: false,
        variant_of: None,
        origin: "labelled RASAero II by RocketPy's Juno III notebook; a 3-decimal, hand-edited \
                 table (broken above Mach 1), used for power-off and power-on",
    },
    Case {
        id: "cavour-power-off",
        design: "rocketpy-cavour.json",
        curve: "data/rockets/polito/drag_coefficient_power_off.csv",
        thrusting: false,
        variant_of: None,
        origin: "labelled RASAero II by RocketPy's Cavour notebook; a 3-decimal table from Mach \
                 0.082 to 0.895 with 22 extra rows repeating 13 Mach numbers up to 0.107, 7 of \
                 them with values 0.001 apart",
    },
    Case {
        id: "cavour-power-on",
        design: "rocketpy-cavour.json",
        curve: "data/rockets/polito/drag_coefficient_power_on.csv",
        thrusting: true,
        variant_of: None,
        origin: "the power-on companion of the Cavour table, from Mach 0.011 to 0.923: within the \
                 tables' 0.001 rounding of the power-off table from Mach 0.16 up (0.0001 lower at \
                 Mach 0.3), and 0.001 to 0.013 lower below Mach 0.16. Their uneven Mach spacing \
                 suggests samples along a flight rather than a Mach sweep (unconfirmed)",
    },
    Case {
        id: "valetudo-power-off",
        design: "rocketpy-valetudo.json",
        curve: "data/rockets/valetudo/Cd_PowerOff_RASAero.csv",
        thrusting: false,
        variant_of: None,
        origin: "labelled RASAero by its file name; a 3-decimal, hand-edited table with no input \
                 file, 1.44 times the OpenRocket export for the same rocket in RocketPy's \
                 RocketPaper repository at Mach 0.3",
    },
    Case {
        id: "valetudo-power-on",
        design: "rocketpy-valetudo.json",
        curve: "data/rockets/valetudo/Cd_PowerOn_RASAero.csv",
        thrusting: true,
        variant_of: None,
        origin: "the power-on companion of the Valetudo table: 0.003 to 0.010 below it from Mach \
                 0.02 (0.098 above at Mach 0.01)",
    },
];

pub fn run(args: &[String]) -> Result<(), String> {
    let check = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err(format!("usage:\n{USAGE}")),
    };
    let root = crate::designs::root()?;
    let fixture = generate(&root)?;
    let path = root.join(FIXTURE);
    if check {
        let committed = fs::read_to_string(&path).map_err(|e| format!("{FIXTURE}: {e}"))?;
        let committed: Value =
            serde_json::from_str(&committed).map_err(|e| format!("{FIXTURE}: {e}"))?;
        if !crate::designs::same(&committed, &fixture) {
            return Err(format!("{FIXTURE} differs from `cargo xtask aero`"));
        }
        println!("{FIXTURE} matches {} curves in {CHECKOUT}", CASES.len());
        return Ok(());
    }
    let text = serde_json::to_string_pretty(&fixture).map_err(|e| e.to_string())? + "\n";
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    fs::write(&path, text).map_err(|e| format!("{FIXTURE}: {e}"))?;
    println!("wrote {FIXTURE}");
    Ok(())
}

/// The fixture, computed from the committed designs and the curves in `refs/rocketpy`.
fn generate(root: &Path) -> Result<Value, String> {
    let checkout = root.join(CHECKOUT);
    let commit = crate::refs::git::clean_head(&checkout)?;
    let air = Ussa76::standard()
        .sample(0.0)
        .map_err(|e| e.to_string())?
        .air;
    let speed = MACH * air.speed_of_sound_m_s;
    let nu = air.kinematic_viscosity_m2_s();
    let mut cases = Vec::new();
    for case in CASES {
        let design_path = root.join("validation/designs").join(case.design);
        let text = fs::read_to_string(&design_path).map_err(|e| format!("{}: {e}", case.design))?;
        let rocket: Rocket =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", case.design))?;
        let drag = hpr_drag(&rocket, speed, nu, case.thrusting)
            .map_err(|e| format!("{}: {e}", case.id))?;
        let curve_path = checkout.join(case.curve);
        let bytes = fs::read(&curve_path).map_err(|e| {
            format!(
                "{}: {e} (run `cargo xtask refs fetch` for the RocketPy checkout)",
                curve_path.display()
            )
        })?;
        let curve_text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
        let (curve_text, dropped) = first_row_per_mach(curve_text);
        let table =
            parse_mach_csv(&curve_text, None).map_err(|e| format!("{}: {e}", case.curve))?;
        let reference = table.lookup(MACH).map_err(|e| e.to_string())?;
        if reference.extrapolated.is_some() {
            return Err(format!("{}: Mach {MACH} is outside the curve", case.curve));
        }
        let error = drag / reference.value - 1.0;
        println!(
            "{:<36} hpr C_D0 {drag:.4}, {:+.1}% from the curve",
            case.id,
            100.0 * error
        );
        cases.push(json!({
            "id": case.id,
            "design": case.design,
            "curve": case.curve,
            "curve_sha256": Sha256::digest(&bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            "origin": case.origin,
            "variant_of": case.variant_of,
            "thrusting": case.thrusting,
            "curve_rows_dropped": dropped,
            "curve_cd0": reference.value,
            "hpr_cd0": drag,
            "relative_error": error,
        }));
    }
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "Derived numbers only: RocketPy's curves are read from refs/rocketpy and never \
                 committed. curve_cd0 is a curve's linear interpolation at the Mach number, after \
                 dropping rows that repeat the previous row's Mach number (curve_rows_dropped), \
                 and relative_error is hpr_cd0 over it, minus 1.",
        "rocketpy_commit": commit,
        "mach": MACH,
        "atmosphere": "USSA76 at sea level",
        "reynolds_per_m": speed / nu,
        "tolerance_rel": TOLERANCE,
        "cases": cases,
    }))
}

/// The curve's text with every row whose Mach number repeats the previous row's dropped, and how
/// many were dropped. `parse_mach_csv` refuses a repeated Mach number with another value; Cavour's
/// curve rounds its Mach numbers to three decimals and repeats 13 of them up to Mach 0.107 (7 with
/// values 0.001 apart), far from the comparison. Unparseable rows are kept for the parser to report.
fn first_row_per_mach(text: &str) -> (String, usize) {
    let mut kept = String::new();
    let mut last: Option<String> = None;
    let mut dropped = 0;
    for line in text.lines() {
        let mach = line.split(',').next().map(|m| m.trim().to_owned());
        if mach.is_some() && !line.trim().is_empty() && mach == last {
            dropped += 1;
            continue;
        }
        if !line.trim().is_empty() {
            last = mach;
        }
        kept.push_str(line);
        kept.push('\n');
    }
    (kept, dropped)
}

/// hpr's zero-lift drag coefficient for `rocket`'s first configuration, with its motors thrusting
/// when `thrusting`.
fn hpr_drag(rocket: &Rocket, speed: f64, nu: f64, thrusting: bool) -> Result<f64, String> {
    let layout = rocket.layout().map_err(|e| e.to_string())?;
    let model = AeroModel::new(&layout).map_err(|e| e.to_string())?;
    let motor_area: f64 = if thrusting {
        rocket
            .configurations
            .first()
            .ok_or("the design has no configuration")?
            .motors
            .iter()
            .map(|m| 0.25 * std::f64::consts::PI * m.diameter_m * m.diameter_m)
            .sum()
    } else {
        0.0
    };
    let conditions = if thrusting {
        DragConditions::thrusting(speed / nu, motor_area)
    } else {
        DragConditions::coasting(speed / nu)
    };
    let drag = model
        .drag(&Flow::axial(MACH), &conditions)
        .map_err(|e| e.to_string())?;
    Ok(drag.zero_lift_coefficient)
}

#[cfg(test)]
mod tests {
    /// With `refs/rocketpy` fetched, the committed fixture matches the curves and the designs.
    /// Without the checkout (as in CI) there is nothing to compare; `hpr_aero`'s test still checks
    /// the fixture against the committed designs.
    #[test]
    fn fixture_matches_the_curves_when_the_checkout_is_present() {
        let root = crate::designs::root().unwrap();
        if !root.join(super::CHECKOUT).join(".git").exists() {
            eprintln!(
                "{} is not fetched; skipping the comparison",
                super::CHECKOUT
            );
            return;
        }
        super::run(&["--check".to_owned()]).unwrap();
    }
}
