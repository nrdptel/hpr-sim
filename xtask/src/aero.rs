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

/// A comparison: an hpr design, a RocketPy drag curve, and what the curve is.
struct Case {
    id: &'static str,
    design: &'static str,
    curve: &'static str,
    thrusting: bool,
    origin: &'static str,
}

const CASES: &[Case] = &[
    Case {
        id: "calisto-power-off",
        design: "rocketpy-calisto-tests-motor-at-minus-1.373.json",
        curve: "data/rockets/calisto/powerOffDragCurve.csv",
        thrusting: false,
        origin: "RASAero II export: the alpha-0 `CD Power-Off` column of RocketPy's first commit's \
                 `data/calisto/CD Test.CSV` (da91db9e, 2018), rounded to 9 decimals and cut to \
                 Mach 2; that export's power-on column equals its power-off column",
    },
    Case {
        id: "calisto-getting-started-power-off",
        design: "rocketpy-calisto-getting-started-motor-at-minus-1.255.json",
        curve: "data/rockets/calisto/powerOffDragCurve.csv",
        thrusting: false,
        origin: "the same RASAero II export, for the larger fins RocketPy's getting-started \
                 example gives Calisto (tip chord 0.06 m, span 0.11 m, against the 2018 \
                 notebook's 0.04 m and 0.10 m)",
    },
    Case {
        id: "juno-iii-power-off",
        design: "rocketpy-juno-iii.json",
        curve: "data/rockets/juno3/drag_curve.csv",
        thrusting: false,
        origin: "labelled RASAero II by RocketPy's Juno III notebook; a 3-decimal, hand-edited \
                 table (broken above Mach 1), used for power-off and power-on",
    },
    Case {
        id: "valetudo-power-off",
        design: "rocketpy-valetudo.json",
        curve: "data/rockets/valetudo/Cd_PowerOff_RASAero.csv",
        thrusting: false,
        origin: "labelled RASAero by its file name; a 3-decimal, hand-edited table with no input \
                 file, 1.44 times the OpenRocket export for the same rocket in RocketPy's \
                 RocketPaper repository at Mach 0.3",
    },
    Case {
        id: "valetudo-power-on",
        design: "rocketpy-valetudo.json",
        curve: "data/rockets/valetudo/Cd_PowerOn_RASAero.csv",
        thrusting: true,
        origin: "the power-on companion of the Valetudo table, 0.003 to 0.009 below it",
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
    let commit = git_head(&checkout)?;
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
        let curve_text = String::from_utf8(bytes.clone()).map_err(|e| e.to_string())?;
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
            "thrusting": case.thrusting,
            "hpr_cd0": drag,
            "relative_error": error,
        }));
    }
    Ok(json!({
        "generator": "cargo xtask aero",
        "note": "Derived numbers only: RocketPy's curves are read from refs/rocketpy and never \
                 committed. relative_error is hpr's C_D0 over the curve's value at the Mach \
                 number, minus 1; the curve interpolates linearly.",
        "rocketpy_commit": commit,
        "mach": MACH,
        "atmosphere": "USSA76 at sea level",
        "reynolds_per_m": speed / nu,
        "tolerance_rel": TOLERANCE,
        "cases": cases,
    }))
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
    let drag = model
        .drag(
            &Flow::axial(MACH),
            &DragConditions::from_air(speed, nu, motor_area),
        )
        .map_err(|e| e.to_string())?;
    Ok(drag.zero_lift_coefficient)
}

fn git_head(checkout: &Path) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(checkout)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{}: not a git checkout (run `cargo xtask refs fetch`)",
            checkout.display()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
