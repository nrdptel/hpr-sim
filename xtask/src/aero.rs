//! `cargo xtask aero [--check]`: compares hpr's drag with the drag curves that ship with
//! RocketPy's example rockets, at Mach 0.3 (M1.5b) and every 0.05 from Mach 0.1 to 2.0 by band
//! (M1.8b2), and writes the derived numbers to
//! `validation/fixtures/aero/rocketpy-drag-curves.json`; compares hpr's normal force against
//! Mach with two references ([`crate::aero_mach`]), written to
//! `validation/fixtures/aero/normal-force-vs-mach.json`; and compares its drag against Mach with
//! NASA's Arcas Robin wind tunnel and MIL-HDBK-762's sample calculation ([`crate::aero_drag`]),
//! written to `validation/fixtures/aero/drag-vs-mach.json`.
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
  aero [--check]           Compare hpr's drag with the drag curves of RocketPy's example
                           rockets (from refs/rocketpy) at Mach 0.3 and from Mach 0.1 to 2.0,
                           and write validation/fixtures/aero/rocketpy-drag-curves.json;
                           compare its normal force against Mach with RASAero II's Calisto
                           export and the Arcas Robin wind tunnel and write
                           validation/fixtures/aero/normal-force-vs-mach.json; compare its
                           drag against Mach with the Arcas Robin wind tunnel and
                           MIL-HDBK-762's sample calculation and write
                           validation/fixtures/aero/drag-vs-mach.json. --check fails if a
                           committed fixture differs instead of writing.";

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
    /// The last Mach number of the curve the sweep uses: the curve's end, or where a hand-edited
    /// table stops being a drag curve.
    usable_to_mach: Option<f64>,
    /// Why the sweep stops before the curve's end, when it does.
    usable_note: Option<&'static str>,
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
        usable_to_mach: None,
        usable_note: None,
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
        usable_to_mach: None,
        usable_note: None,
    },
    Case {
        id: "juno-iii-power-off",
        design: "rocketpy-juno-iii.json",
        curve: "data/rockets/juno3/drag_curve.csv",
        thrusting: false,
        variant_of: None,
        origin: "labelled RASAero II by RocketPy's Juno III notebook; a 3-decimal, hand-edited \
                 table (broken above Mach 1), used for power-off and power-on",
        usable_to_mach: Some(0.92),
        usable_note: Some(
            "from Mach 0.93 to 1.0 the table climbs a constant 0.072 per 0.01 and then drops to \
             0.001: hand-edited, not a drag curve",
        ),
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
        usable_to_mach: None,
        usable_note: None,
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
        usable_to_mach: None,
        usable_note: None,
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
        usable_to_mach: None,
        usable_note: None,
    },
    Case {
        id: "valetudo-power-on",
        design: "rocketpy-valetudo.json",
        curve: "data/rockets/valetudo/Cd_PowerOn_RASAero.csv",
        thrusting: true,
        variant_of: None,
        origin: "the power-on companion of the Valetudo table: 0.003 to 0.010 below it from Mach \
                 0.02 (0.098 above at Mach 0.01)",
        usable_to_mach: None,
        usable_note: None,
    },
];

pub fn run(args: &[String]) -> Result<(), String> {
    let check = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err(format!("usage:\n{USAGE}")),
    };
    let root = crate::designs::root()?;
    for (name, fixture) in [
        (FIXTURE, generate(&root)?),
        (
            crate::aero_mach::FIXTURE,
            crate::aero_mach::generate(&root)?,
        ),
        (
            crate::aero_drag::FIXTURE,
            crate::aero_drag::generate(&root)?,
        ),
    ] {
        let path = root.join(name);
        if check {
            let committed = fs::read_to_string(&path).map_err(|e| format!("{name}: {e}"))?;
            let committed: Value =
                serde_json::from_str(&committed).map_err(|e| format!("{name}: {e}"))?;
            if !crate::designs::same(&committed, &fixture) {
                return Err(format!("{name} differs from `cargo xtask aero`"));
            }
            println!("{name} matches its references");
            continue;
        }
        let text = serde_json::to_string_pretty(&fixture).map_err(|e| e.to_string())? + "\n";
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        fs::write(&path, text).map_err(|e| format!("{name}: {e}"))?;
        println!("wrote {name}");
    }
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
        let drag = hpr_drag(&rocket, MACH, speed / nu, case.thrusting)
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
        let sweep = sweep(case, &rocket, |mach| {
            let at = table.lookup(mach).map_err(|e| e.to_string())?;
            Ok(at.extrapolated.is_none().then_some(at.value))
        })?;
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
            "sweep": sweep,
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
        "sweep_note": "Each case's sweep compares hpr's C_D0 with the curve every 0.05 from Mach \
                       0.1 to 2.0, where the curve reaches without extrapolation (to \
                       usable_to_mach), each at USSA76 sea level's Reynolds number for its Mach \
                       number. relative_error is hpr's over the curve's, minus 1; bands are \
                       Niskanen 2009 Table 3.1's (subsonic to 0.8, transonic below 1.2, \
                       supersonic from 1.2).",
        "cases": cases,
    }))
}

/// Mach numbers of the sweep: every 0.05 from 0.1 to 2.0, M1.8's range for drag.
fn sweep_machs() -> impl Iterator<Item = f64> {
    (2..=40).map(|i| f64::from(i) / 20.0)
}

/// hpr's `C_D0` against the curve at every Mach number of [`sweep_machs`] the curve reaches, at
/// sea level, and the errors by band. `curve` gives the curve's value at a Mach number, `None`
/// outside it.
fn sweep(
    case: &Case,
    rocket: &Rocket,
    curve: impl Fn(f64) -> Result<Option<f64>, String>,
) -> Result<Value, String> {
    let air = Ussa76::standard()
        .sample(0.0)
        .map_err(|e| e.to_string())?
        .air;
    let nu = air.kinematic_viscosity_m2_s();
    let usable = case.usable_to_mach.unwrap_or(f64::INFINITY);
    let mut rows = Vec::new();
    let mut bands: Vec<(&str, Vec<f64>)> = Vec::new();
    for mach in sweep_machs().filter(|&m| m <= usable) {
        let Some(reference) = curve(mach)? else {
            // The rows run without a gap from Mach 0.1 (the tests check it): a curve that starts
            // late or has a hole is refused here, where the cause is clear.
            if rows.is_empty() {
                return Err(format!("{}: the curve doesn't reach Mach {mach}", case.id));
            }
            break;
        };
        if !(reference.is_finite() && reference > 0.0) {
            return Err(format!(
                "{}: the curve gives {reference} at Mach {mach}",
                case.id
            ));
        }
        let reynolds_per_m = mach * air.speed_of_sound_m_s / nu;
        let drag = hpr_drag(rocket, mach, reynolds_per_m, case.thrusting)
            .map_err(|e| format!("{} at Mach {mach}: {e}", case.id))?;
        let error = drag / reference - 1.0;
        let band = crate::aero_mach::band(mach);
        match bands.last_mut() {
            Some((name, errors)) if *name == band => errors.push(error),
            _ => bands.push((band, vec![error])),
        }
        rows.push(json!({
            "mach": mach,
            "band": band,
            "hpr_cd0": drag,
            "relative_error": error,
            "within_target": error.abs() <= TOLERANCE,
        }));
    }
    let bands: Vec<Value> = bands
        .iter()
        .map(|(band, errors)| {
            let min = errors.iter().copied().fold(f64::INFINITY, f64::min);
            let max = errors.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let rms = (errors.iter().map(|e| e * e).sum::<f64>() / errors.len() as f64).sqrt();
            json!({
                "band": band,
                "rows": errors.len(),
                "within_target": errors.iter().filter(|e| e.abs() <= TOLERANCE).count(),
                "min_error": min,
                "max_error": max,
                "rms_error": rms,
            })
        })
        .collect();
    for band in &bands {
        println!(
            "  {:<10} {:>2} Mach numbers, {:>2} within {:.0}%, {:+.1}% to {:+.1}%",
            band["band"].as_str().unwrap_or_default(),
            band["rows"],
            band["within_target"],
            100.0 * TOLERANCE,
            100.0 * band["min_error"].as_f64().unwrap_or(f64::NAN),
            100.0 * band["max_error"].as_f64().unwrap_or(f64::NAN),
        );
    }
    Ok(json!({
        "usable_to_mach": case.usable_to_mach,
        "usable_note": case.usable_note,
        "bands": bands,
        "rows": rows,
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

/// hpr's zero-lift drag coefficient for `rocket`'s first configuration at `mach` and
/// `reynolds_per_m`, with its motors thrusting when `thrusting`.
fn hpr_drag(
    rocket: &Rocket,
    mach: f64,
    reynolds_per_m: f64,
    thrusting: bool,
) -> Result<f64, String> {
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
        DragConditions::thrusting(reynolds_per_m, motor_area)
    } else {
        DragConditions::coasting(reynolds_per_m)
    };
    let drag = model
        .drag(&Flow::axial(mach), &conditions)
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

    /// The predicted-mode case files of NDRT 2020 and Bella Lui (ADR-023) explain their results
    /// by hpr's coasting `C_D0` at Mach 0.3, sea level, against their examples' constant drags,
    /// 0.44 and 0.43. Those two examples ship no curve for `cargo xtask aero` to record, so the
    /// quoted values are pinned here, computed as it computes the others.
    #[test]
    #[expect(
        clippy::approx_constant,
        reason = "NDRT's 0.318 is a drag coefficient, not an approximation of 1/π"
    )]
    fn the_drags_the_predicted_cases_quote_are_hpr_s() {
        let root = crate::designs::root().unwrap();
        let air = super::Ussa76::standard().sample(0.0).unwrap().air;
        let speed = super::MACH * air.speed_of_sound_m_s;
        let nu = air.kinematic_viscosity_m2_s();
        for (design, quoted) in [
            ("rocketpy-ndrt-2020-nose-to-tail", 0.318),
            ("rocketpy-bella-lui", 0.424),
        ] {
            let text =
                std::fs::read_to_string(root.join(format!("validation/designs/{design}.json")))
                    .unwrap();
            let rocket: super::Rocket = serde_json::from_str(&text).unwrap();
            let drag = super::hpr_drag(&rocket, super::MACH, speed / nu, false).unwrap();
            assert!(
                (drag - quoted).abs() < 5e-4,
                "{design}: hpr's C_D0 is {drag:.4}, where its predicted case quotes {quoted}"
            );
        }
    }
}
