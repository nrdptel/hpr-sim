//! The `hpr` binary run as a user runs it: exit codes, standard output and standard error, and
//! every `--json` document checked against its committed schema in `schema/cli/`.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests read the committed schemas and write motor files; not the pure core"
)]
#![allow(
    clippy::unwrap_used,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::path::{Path, PathBuf};
use std::process::Output;

use clap::ValueEnum;
use clap_complete::Shell;
use hpr::hpr_motor::Catalog;
use hpr_cli::motors::MotorFile;
use hpr_cli::registry::{self, Availability, PLANNED};
use serde_json::Value;

/// The repository's root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A bundled curve file, by its catalog path.
fn curve_file(name: &str) -> String {
    root()
        .join("crates/hpr-motor/data/thrustcurve")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

/// Runs `hpr` with `args`.
fn hpr(args: &[&str]) -> Output {
    assert_cmd::cargo::cargo_bin_cmd!("hpr")
        .args(args)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Checks `document` against the committed schema `name`.
fn validate(name: &str, document: &Value) {
    let path = root().join("schema/cli").join(name);
    let schema: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(document)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "{name}: {errors:?}\n{document:#}");
}

/// Runs `hpr` with `--json` added, expects `code`, an empty standard error and one document on
/// standard output that the schema `name` accepts, and returns the document.
fn json(args: &[&str], code: i32, schema: &str) -> Value {
    let mut args = args.to_vec();
    args.push("--json");
    let output = hpr(&args);
    assert_eq!(output.status.code(), Some(code), "{args:?}: {output:?}");
    assert!(
        output.stderr.is_empty(),
        "{args:?}: {}",
        text(&output.stderr)
    );
    let document: Value = serde_json::from_slice(&output.stdout).unwrap();
    validate(schema, &document);
    document
}

/// Runs `hpr` without `--json`, expects success and nothing on standard error, and returns the
/// text.
fn text_ok(args: &[&str]) -> String {
    let output = hpr(args);
    assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
    assert!(
        output.stderr.is_empty(),
        "{args:?}: {}",
        text(&output.stderr)
    );
    text(&output.stdout)
}

/// An error document of `kind`, from a run with `--json`.
fn json_error(args: &[&str], code: i32, kind: &str) -> Value {
    let document = json(args, code, "error.schema.json");
    assert_eq!(document["error"]["kind"], kind, "{document:#}");
    document
}

/// Every command registered before its milestone refuses with status 3 and names that
/// milestone, whatever arguments it is given, as text on standard error or as a JSON document.
#[test]
fn every_planned_command_refuses_with_its_milestone() {
    let planned: Vec<_> = registry::commands()
        .into_iter()
        .filter(|c| matches!(c.availability, Some(Availability::Planned { .. })))
        .collect();
    assert_eq!(planned.len(), PLANNED.len());
    for (name, milestone) in PLANNED {
        let output = hpr(&[name, "design.ork", "--rail", "2"]);
        assert_eq!(output.status.code(), Some(3), "{name}: {output:?}");
        assert!(output.stdout.is_empty(), "{name}");
        let message = text(&output.stderr);
        assert!(
            message.starts_with(&format!("error: hpr {name} is not available yet"))
                && message.contains(milestone),
            "{message}"
        );
        let document = json_error(&[name, "design.ork", "--rail", "2"], 3, "not_available");
        assert_eq!(document["error"]["command"], name);
        assert_eq!(document["error"]["milestone"], milestone);
        // `--json` first, before the command, works the same.
        let first = hpr(&["--json", name]);
        assert_eq!(first.status.code(), Some(3));
        assert_eq!(serde_json::from_slice::<Value>(&first.stdout).unwrap(), {
            let mut alone = document.clone();
            alone["error"]["command"] = Value::from(name);
            alone
        });
    }
}

/// `hpr motors list` lists the bundled catalog in its order, with its stated figures, and each
/// filter narrows it.
#[test]
fn motors_list_is_the_catalog() {
    let catalog = Catalog::bundled().unwrap();
    let all = json(&["motors", "list"], 0, "motors-list.schema.json");
    let motors = all["motors"].as_array().unwrap();
    assert_eq!(motors.len(), catalog.motors.len());
    for (listed, motor) in motors.iter().zip(&catalog.motors) {
        assert_eq!(listed["designation"], motor.designation.as_str());
        assert_eq!(
            listed["impulse_class"],
            motor.impulse_class.label().as_str()
        );
        assert_eq!(listed["total_impulse_ns"], motor.total_impulse_ns);
        assert_eq!(listed["burn_time_s"], motor.burn_time_s);
        assert_eq!(listed["diameter_mm"], motor.diameter_mm);
    }
    assert_eq!(
        all["catalog"]["captured"],
        catalog.snapshot.captured.as_str()
    );

    let class_j = json(
        &["motors", "list", "--class", "j"],
        0,
        "motors-list.schema.json",
    );
    let class_j = class_j["motors"].as_array().unwrap();
    let expected = catalog
        .motors
        .iter()
        .filter(|m| m.impulse_class.label() == "J")
        .count();
    assert!(expected > 0);
    assert_eq!(class_j.len(), expected);
    assert!(class_j.iter().all(|m| m["impulse_class"] == "J"));

    let wide = json(
        &["motors", "list", "--diameter", "54"],
        0,
        "motors-list.schema.json",
    );
    let wide = wide["motors"].as_array().unwrap();
    assert!(!wide.is_empty());
    assert!(wide.iter().all(|m| m["diameter_mm"] == 54.0));

    let loki = json(
        &["motors", "list", "--manufacturer", "LOKI", "--class", "J"],
        0,
        "motors-list.schema.json",
    );
    let loki = loki["motors"].as_array().unwrap();
    assert_eq!(loki.len(), 1);
    assert_eq!(loki[0]["manufacturer_abbrev"], "Loki");

    let none = json(
        &["motors", "list", "--class", "Z"],
        0,
        "motors-list.schema.json",
    );
    assert!(none["motors"].as_array().unwrap().is_empty());

    let table = text_ok(&["motors", "list", "--class", "J"]);
    assert!(table.starts_with(&format!("{expected} motors from ThrustCurve.org")));
    assert!(table.contains("designation"));
    assert_eq!(table.lines().count(), 3 + expected);
}

/// A filter that can't mean anything is refused rather than matching nothing.
#[test]
fn a_bad_filter_is_an_input_error() {
    for args in [
        ["motors", "list", "--class", "JJ"],
        ["motors", "list", "--diameter", "-3"],
        ["motors", "list", "--diameter", "NaN"],
    ] {
        let document = json_error(&args, 1, "input");
        assert_eq!(document["error"]["command"], "motors");
        let output = hpr(&args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(text(&output.stderr).starts_with("error: --"));
    }
}

/// `hpr motors show` reports the library's own figures for a catalog motor's bundled curve, and
/// the catalog's stated ones beside them.
#[test]
fn motors_show_reports_the_librarys_figures() {
    let catalog = Catalog::bundled().unwrap();
    let motor = catalog.find("1266J760-19A").next().unwrap();
    let solid = motor.bundled_motor().unwrap();
    let curve = solid.curve();
    for name in ["1266J760-19A", "j760", "J 760"] {
        let document = json(&["motors", "show", name], 0, "motors-show.schema.json");
        let shown = &document["motors"][0];
        assert_eq!(document["motors"].as_array().unwrap().len(), 1);
        assert_eq!(shown["name"], "1266J760-19A");
        assert_eq!(shown["source"]["kind"], "catalog");
        assert_eq!(shown["total_impulse_ns"], curve.total_impulse_ns());
        assert_eq!(shown["average_thrust_n"], curve.average_thrust_n());
        assert_eq!(shown["peak_thrust_n"], curve.peak_thrust_n());
        assert_eq!(shown["burn_time_s"], curve.burn_time_s());
        assert_eq!(shown["burn_start_s"], curve.burn_window_s().0);
        assert_eq!(shown["burn_end_s"], curve.burn_window_s().1);
        assert_eq!(shown["curve_end_s"], curve.end_time_s());
        assert_eq!(shown["impulse_class"], "J");
        assert_eq!(
            shown["propellant_mass_kg"],
            solid.propellant_initial_mass_kg()
        );
        assert_eq!(shown["diameter_m"], 0.054);
        assert_eq!(shown["stated"]["total_impulse_ns"], motor.total_impulse_ns);
        assert_eq!(shown["stated"]["burn_time_s"], motor.burn_time_s);
        assert_eq!(
            shown["delays"].as_array().unwrap().len(),
            motor.delays().delays.len()
        );
        assert_eq!(
            shown["delays"][0],
            serde_json::json!({"kind": "seconds", "value": 9.0})
        );
    }
    let lines = text_ok(&["motors", "show", "J760"]);
    assert!(lines.starts_with("1266J760-19A (Cesaroni Technology), from the bundled catalog\n"));
    assert!(lines.contains("  ThrustCurve.org  J class, 1265.7 N·s"));
}

/// A name several catalog motors share shows each of them.
#[test]
fn a_shared_name_shows_every_match() {
    let catalog = Catalog::bundled().unwrap();
    let expected: Vec<&str> = catalog
        .find("I175")
        .map(|m| m.designation.as_str())
        .collect();
    assert_eq!(expected.len(), 2);
    let document = json(&["motors", "show", "I175"], 0, "motors-show.schema.json");
    let names: Vec<&str> = document["motors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, expected);
}

/// Every motor file format the command table claims is read, from a bundled curve file, to the
/// figures the catalog's own reading of that file gives.
#[test]
fn every_motor_file_format_is_read() {
    let catalog = Catalog::bundled().unwrap();
    for format in MotorFile::ALL {
        let motor = catalog
            .motors
            .iter()
            .find(|m| m.curves[0].file.ends_with(format.extension()))
            .unwrap();
        let path = curve_file(&motor.curves[0].file);
        assert_eq!(MotorFile::of(&path), Some(format));
        let document = json(&["motors", "show", &path], 0, "motors-show.schema.json");
        let shown = &document["motors"][0];
        assert_eq!(shown["source"]["kind"], "file");
        assert_eq!(shown["source"]["path"], path.as_str());
        assert_eq!(shown["source"]["format"], &format.extension()[1..]);
        assert!(shown["stated"].is_null());
        let curve = motor.bundled_motor().unwrap();
        assert_eq!(shown["total_impulse_ns"], curve.curve().total_impulse_ns());
        assert_eq!(shown["burn_time_s"], curve.curve().burn_time_s());
        let lines = text_ok(&["motors", "show", &path]);
        assert!(lines.contains(&format!("from {path}\n")), "{lines}");
    }
}

/// A `.eng` file with two motors shows both, and a `0` delay is shown with its warning.
#[test]
fn a_file_with_two_motors_shows_both_with_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("two.ENG");
    // Invented motors: a triangle and a rectangle of thrust, with round numbers.
    std::fs::write(
        &path,
        "; two test motors\n\
         T10 24 70 0-4 0.01 0.03 Test\n\
         0.0 0.0\n0.5 20.0\n1.0 0.0\n;\n\
         T20 24 70 P 0.02 0.04 Test\n\
         0.0 20.0\n1.0 20.0\n1.0 0.0\n;\n",
    )
    .unwrap();
    let path = path.to_string_lossy().into_owned();
    let document = json(&["motors", "show", &path], 0, "motors-show.schema.json");
    let motors = document["motors"].as_array().unwrap();
    assert_eq!(motors.len(), 2);
    assert_eq!(motors[0]["name"], "T10");
    assert_eq!(motors[0]["total_impulse_ns"], 10.0);
    assert_eq!(motors[0]["peak_thrust_n"], 20.0);
    assert_eq!(
        motors[0]["delays"][0],
        serde_json::json!({"kind": "zero_or_plugged"})
    );
    assert_eq!(motors[1]["name"], "T20");
    assert_eq!(motors[1]["total_impulse_ns"], 20.0);
    assert_eq!(motors[1]["impulse_class"], "D");
    assert_eq!(
        motors[1]["delays"][0],
        serde_json::json!({"kind": "plugged"})
    );
    let warnings = document["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w["motor"] == "T10" && w["kind"] == "unusual"),
        "{warnings:?}"
    );
    let lines = text_ok(&["motors", "show", &path]);
    assert!(lines.contains("\nT20 (Test), from "));
    assert!(lines.contains("\nwarning: T10: "), "{lines}");
}

/// A motor that can't be found or read is an input error, with nothing on standard output.
#[test]
fn a_missing_or_unreadable_motor_is_an_input_error() {
    let dir = tempfile::tempdir().unwrap();
    let broken = dir.path().join("broken.rse");
    std::fs::write(&broken, "<engine-database><engine-list>").unwrap();
    let not_text = dir.path().join("binary.eng");
    std::fs::write(&not_text, [0xff, 0xfe, 0x00]).unwrap();
    let missing = dir.path().join("missing.eng");
    for motor in [
        "Z99999",
        &broken.to_string_lossy(),
        &not_text.to_string_lossy(),
        &missing.to_string_lossy(),
    ] {
        let document = json_error(&["motors", "show", motor], 1, "input");
        assert_eq!(document["error"]["command"], "motors");
        let output = hpr(&["motors", "show", motor]);
        assert_eq!(output.status.code(), Some(1), "{motor}");
        assert!(output.stdout.is_empty(), "{motor}");
        assert!(text(&output.stderr).starts_with("error: "), "{motor}");
    }
}

/// `hpr completions` writes a script for every shell clap_complete knows, naming the commands.
#[test]
fn completions_for_every_shell() {
    for shell in Shell::value_variants() {
        let name = shell.to_string();
        let script = text_ok(&["completions", &name]);
        assert!(
            script.contains("motors") && script.contains("completions"),
            "{name}"
        );
        let document = json(&["completions", &name], 0, "completions.schema.json");
        assert_eq!(document["shell"], name.as_str());
        assert_eq!(document["script"], script.as_str());
    }
}

/// A wrong command line exits with 2: clap's text on standard error, or an error document.
#[test]
fn usage_errors_exit_with_2() {
    for args in [
        vec!["motors", "shwo"],
        vec!["motors", "show"],
        vec!["motors", "list", "--diameter", "wide"],
        vec!["launch"],
        vec!["completions", "cmd.exe"],
        vec![],
    ] {
        let output = hpr(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        assert!(!output.stderr.is_empty(), "{args:?}");
        let document = json_error(&args, 2, "usage");
        assert!(document["error"]["command"].is_null());
    }
}

/// Help and version are answers, not errors: standard output, status 0, even with `--json`.
#[test]
fn help_and_version_print_text() {
    // clap's own `help` command takes command names only, not `--json`.
    assert!(!text_ok(&["help"]).is_empty());
    for args in [["--help"], ["--version"]] {
        let help = text_ok(&args);
        assert!(!help.is_empty());
        let with_json = hpr(&[args[0], "--json"]);
        assert_eq!(with_json.status.code(), Some(0));
        assert_eq!(text(&with_json.stdout), help);
    }
    let help = text_ok(&["--help"]);
    for command in registry::commands() {
        assert!(
            help.contains(&format!("  {} ", command.name)),
            "{}",
            command.name
        );
    }
    assert!(text_ok(&["--version"]).starts_with(&format!("hpr {}", env!("CARGO_PKG_VERSION"))));
}

/// The committed schemas are the ones the output types generate, and no others.
#[test]
fn the_committed_schemas_are_generated() {
    let generated = hpr_cli::schemas();
    let mut committed: Vec<String> = std::fs::read_dir(root().join("schema/cli"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    committed.sort();
    let names: Vec<&str> = generated.iter().map(|(name, _)| *name).collect();
    assert_eq!(committed, names);
    for (name, text) in generated {
        let file = std::fs::read_to_string(root().join("schema/cli").join(name)).unwrap();
        assert_eq!(file, text, "{name} is stale: run `cargo xtask cli`");
    }
}
