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
        assert_eq!(
            serde_json::from_slice::<Value>(&first.stdout).unwrap(),
            document
        );
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
        ["motors", "list", "--manufacturer", "CTI"],
        ["motors", "list", "--manufacturer", "ces"],
    ] {
        let document = json_error(&args, 1, "input");
        assert_eq!(document["error"]["command"], "motors");
        let output = hpr(&args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(text(&output.stderr).starts_with("error: --"));
    }
}

/// A maker is named as the catalog spells it, or by its full name, in any case: the guide's
/// examples.
#[test]
fn a_maker_is_named_by_abbreviation_or_full_name() {
    let listed = |maker: &str| {
        let document = json(
            &["motors", "list", "--manufacturer", maker],
            0,
            "motors-list.schema.json",
        );
        document["motors"].as_array().unwrap().clone()
    };
    let cesaroni = listed("cesaroni");
    assert!(!cesaroni.is_empty());
    assert!(
        cesaroni
            .iter()
            .all(|m| m["manufacturer_abbrev"] == "Cesaroni")
    );
    assert_eq!(listed("Cesaroni Technology"), cesaroni);
    assert_eq!(listed("AEROTECH").len(), listed("AeroTech").len());
    let refused = hpr(&["motors", "list", "--manufacturer", "CTI"]);
    let message = text(&refused.stderr);
    assert!(
        message.contains("use one of AMW, AeroTech, Cesaroni"),
        "{message}"
    );
}

/// After `--`, `--json` is an argument, such as a motor's name, and the output stays text.
#[test]
fn json_after_a_double_dash_is_an_argument() {
    let output = hpr(&["motors", "show", "--", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(text(&output.stderr).contains("is called --json"));
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

/// The guide's numbers for how hpr's figures stand beside ThrustCurve.org's stated ones, over the
/// whole catalog: total impulse, average thrust and burn time within 1% (the catalog's selection
/// rule), and peak thrust from 16.7% below (Cesaroni 26E31-15A) to 2.1% above (Loki M1378LR).
#[test]
fn the_catalogs_stated_figures_are_as_the_guide_says() {
    let catalog = Catalog::bundled().unwrap();
    let mut peaks: Vec<(f64, String)> = Vec::new();
    for motor in &catalog.motors {
        let document = json(
            &["motors", "show", &motor.designation],
            0,
            "motors-show.schema.json",
        );
        let shown = document["motors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["name"] == motor.designation.as_str())
            .unwrap()
            .clone();
        let gap = |ours: &str, stated: &str| {
            let (ours, stated) = (
                shown[ours].as_f64().unwrap(),
                shown["stated"][stated].as_f64().unwrap(),
            );
            (ours - stated) / stated
        };
        for (ours, stated) in [
            ("total_impulse_ns", "total_impulse_ns"),
            ("average_thrust_n", "average_thrust_n"),
            ("burn_time_s", "burn_time_s"),
        ] {
            let gap = gap(ours, stated);
            assert!(gap.abs() <= 0.01, "{}: {ours} {gap}", motor.designation);
        }
        peaks.push((
            gap("peak_thrust_n", "max_thrust_n"),
            motor.designation.clone(),
        ));
    }
    assert_eq!(peaks.len(), 32);
    peaks.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (low, high) = (&peaks[0], &peaks[peaks.len() - 1]);
    assert_eq!(low.1, "26E31-15A");
    assert!((-0.1675..=-0.1665).contains(&low.0), "{low:?}");
    assert_eq!(high.1, "M1378LR");
    assert!((0.0205..=0.0215).contains(&high.0), "{high:?}");
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
        .filter(|name| name.ends_with(".schema.json"))
        .collect();
    committed.sort();
    let names: Vec<&str> = generated.iter().map(|(name, _)| *name).collect();
    assert_eq!(committed, names);
    for (name, text) in generated {
        let file = std::fs::read_to_string(root().join("schema/cli").join(name)).unwrap();
        assert_eq!(file, text, "{name} is stale: run `cargo xtask cli`");
    }
}

/// The repository's own pod probe with no pods: a single-stage `.ork`, public, with a 29 mm mount
/// and an AeroTech H128W that the bundled catalog doesn't hold.
const PROBE: &str = "validation/fixtures/ork/pod-flights/pods-none.ork";

/// A repository file's path, as a string for the command line.
fn repo_file(path: &str) -> String {
    root().join(path).to_string_lossy().into_owned()
}

/// `a` and `b` hold the same keys and the same numbers, bit for bit (`float_roundtrip` reads the
/// printed numbers back exactly).
fn same_bits(a: &Value, b: &Value, at: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            assert_eq!(x.to_bits(), y.to_bits(), "{at}: {x} != {y}");
        }
        (Value::Object(x), Value::Object(y)) => {
            let keys = |o: &serde_json::Map<String, Value>| o.keys().cloned().collect::<Vec<_>>();
            assert_eq!(keys(x), keys(y), "{at}");
            for (key, value) in x {
                same_bits(value, &y[key], &format!("{at}.{key}"));
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{at}");
            for (i, (x, y)) in x.iter().zip(y).enumerate() {
                same_bits(x, y, &format!("{at}[{i}]"));
            }
        }
        _ => assert_eq!(a, b, "{at}"),
    }
}

/// The library's flight of a design with `motor` put in the configuration `id`'s one mount, from
/// `builder`'s rail, recorded every 0.01 s as `hpr sim` records it.
fn library_flight(
    mut design: hpr::hpr_design::Rocket,
    id: &str,
    mount: &str,
    motor: Option<&hpr::Motor>,
    environment: &hpr::Environment,
    launch: impl Fn(hpr::FlightBuilder<'_>) -> hpr::FlightBuilder<'_>,
) -> (hpr::Flight, hpr::hpr_sim::Recorder) {
    use hpr::hpr_design::{Configuration, Ignition, MountedMotor};
    if let Some(motor) = motor {
        design.configurations.retain(|c| c.id != id);
        design.configurations.push(Configuration {
            id: id.to_owned(),
            name: String::new(),
            motors: vec![MountedMotor {
                mount: mount.to_owned(),
                designation: motor.designation().to_owned(),
                diameter_m: motor.diameter_m(),
                length_m: motor.length_m(),
                motor: motor.solid_motor().clone(),
                delay: motor.delay(),
                ignition: Ignition::Launch,
                failed_tubes: Vec::new(),
            }],
        });
    }
    let rocket = hpr::Rocket::from_design(design, id).unwrap();
    let mut recorder = hpr::hpr_sim::Recorder::new(
        hpr::hpr_sim::Channel::ALL.to_vec(),
        Some(hpr_cli::sim::DEFAULT_INTERVAL_S),
    )
    .unwrap();
    let flight = launch(hpr::Flight::builder(
        &rocket,
        environment,
        hpr_cli::sim::DEFAULT_RAIL_LENGTH_M,
    ))
    .fly_with(&mut recorder)
    .unwrap();
    (flight, recorder)
}

/// The probe read as the library reads it, with its default configuration's id and mount.
fn probe_design() -> (hpr::hpr_design::Rocket, String, String) {
    let bytes = std::fs::read(repo_file(PROBE)).unwrap();
    let file = hpr::hpr_io::ork::read(&bytes).unwrap().value;
    let design = hpr::hpr_io::ork::design(&file).value;
    let configuration = design.motors.default_configuration().unwrap();
    let id = configuration.id.clone();
    let mount = configuration.motors[0].mount.clone();
    (design.rocket, id, mount)
}

/// Checks `document`'s summary and events against the library's flight, bit for bit.
fn same_flight(document: &Value, flight: &hpr::Flight) {
    let mut summary = serde_json::to_value(flight.summary()).unwrap();
    // The library names its variants as Rust does; the command's schema, in snake case.
    summary["termination"] = "ground_hit".into();
    assert_eq!(
        flight.result().termination,
        hpr::hpr_sim::Termination::GroundHit
    );
    same_bits(&document["summary"], &summary, "summary");
    let events = document["events"].as_array().unwrap();
    assert_eq!(events.len(), flight.result().events.len());
    for (printed, event) in events.iter().zip(&flight.result().events) {
        let sample = &event.sample;
        for (key, value) in [
            ("time_s", sample.time_s),
            ("height_above_ground_m", sample.height_above_ground_m),
            ("speed_m_s", sample.cg_velocity_enu_m_s.length()),
        ] {
            assert_eq!(
                printed[key].as_f64().unwrap().to_bits(),
                value.to_bits(),
                "{key}"
            );
        }
    }
}

/// M4.2b's bullet: a public `.ork` flown by `hpr sim` gives the library's flight bit for bit,
/// and its JSON validates. The probe's own motor has no bundled curve, so the catalog's H54 flies
/// in its mount, as `--motor H54` asks; the library is called as a program would call it.
#[test]
fn sim_flies_a_public_ork_as_the_library_does() {
    let folder = tempfile::tempdir().unwrap();
    let csv = folder.path().join("flight.csv");
    let csv_arg = csv.to_string_lossy().into_owned();
    let design = repo_file(PROBE);
    let document = json(
        &["sim", &design, "--motor", "H54", "--export", &csv_arg],
        0,
        "sim.schema.json",
    );

    let (rocket, id, mount) = probe_design();
    let motor = hpr::Motor::from_catalog("H54").unwrap();
    let environment = hpr::Environment::new(0.0, 0.0, 0.0).unwrap();
    let (flight, recorder) = library_flight(rocket, &id, &mount, Some(&motor), &environment, |b| b);
    same_flight(&document, &flight);
    // The recording, every value printed to its last bit.
    let written = std::fs::read_to_string(&csv).unwrap();
    assert_eq!(written, hpr::hpr_sim::export::csv(&recorder).unwrap());
    assert_eq!(document["exports"][0]["rows"], recorder.rows().len());
    assert_eq!(document["exports"][0]["format"], "csv");

    assert_eq!(document["design"]["file"], "pods-none.ork");
    assert_eq!(document["design"]["format"], "ork");
    assert_eq!(document["design"]["configuration"], id.as_str());
    assert_eq!(document["motors"][0]["designation"], "168H54-10A");
    assert_eq!(document["motors"][0]["mount"], mount.as_str());
    assert_eq!(document["motors"][0]["source"]["kind"], "catalog");
    assert_eq!(document["launch"]["rail_length_m"], 1.5);
    let notes = document["notes"].as_array().unwrap();
    assert!(
        notes[0].as_str().unwrap().contains("no recovery device"),
        "{notes:?}"
    );
    // The apogee is well above the rail, and the text says what the JSON says.
    let apogee = flight.apogee_m().unwrap();
    assert!(apogee > 100.0, "{apogee}");
    let text = text_ok(&["sim", &design, "--motor", "H54"]);
    assert!(
        text.contains(&format!(
            "\napogee                {apogee:.1} m above the site"
        )),
        "{text}"
    );
    assert!(
        text.contains("168H54-10A (from the bundled catalog)"),
        "{text}"
    );
}

/// A motor file flies as the library reads it: the F15's `.rse` and the H54's `.eng`, each in
/// the probe's mount, and every recording format is written.
#[test]
fn sim_flies_a_motor_file_as_the_library_does() {
    let (rocket, id, mount) = probe_design();
    let environment = hpr::Environment::new(0.0, 0.0, 0.0).unwrap();
    for (file, format) in [
        ("curves/5f923edb1bca5800041716ab.rse", "rse"),
        ("curves/5f4294d20002e90000000735.eng", "eng"),
    ] {
        let path = curve_file(file);
        let text = std::fs::read_to_string(&path).unwrap();
        let motor = match format {
            "rse" => hpr::Motor::from_rse(&text).unwrap(),
            _ => hpr::Motor::from_eng(&text).unwrap(),
        };
        let folder = tempfile::tempdir().unwrap();
        let exports: Vec<String> = ["f.csv", "f.json", "f.parquet", "f.geojson", "f.kml"]
            .iter()
            .map(|name| folder.path().join(name).to_string_lossy().into_owned())
            .collect();
        let design = repo_file(PROBE);
        let mut args = vec!["sim", design.as_str(), "--motor", path.as_str()];
        for export in &exports {
            args.extend(["--export", export.as_str()]);
        }
        let document = json(&args, 0, "sim.schema.json");
        let (flight, recorder) = library_flight(
            rocket.clone(),
            &id,
            &mount,
            Some(&motor),
            &environment,
            |b| b,
        );
        same_flight(&document, &flight);
        assert_eq!(document["motors"][0]["source"]["kind"], "file", "{file}");
        assert_eq!(document["motors"][0]["source"]["format"], format, "{file}");
        assert_eq!(document["motors"][0]["designation"], motor.designation());
        let written: Vec<&str> = document["exports"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["format"].as_str().unwrap())
            .collect();
        assert_eq!(written, ["csv", "json", "parquet", "geojson", "kml"]);
        assert_eq!(
            std::fs::read(&exports[2]).unwrap(),
            hpr::hpr_sim::export::parquet(&recorder).unwrap()
        );
        for export in &exports {
            assert!(!std::fs::read(export).unwrap().is_empty(), "{export}");
        }
    }
}

/// An hpr design file flies with its own motor, from a site, a leaning rail and a wind given on
/// the command line, as the library flies it with the same.
#[test]
fn sim_flies_an_hpr_design_from_a_given_launch() {
    let path = repo_file("validation/designs/synthetic-54mm-three-fin.json");
    let document = json(
        &[
            "sim",
            &path,
            "--latitude",
            "32.99",
            "--longitude",
            "-106.97",
            "--elevation",
            "1400",
            "--rail-length",
            "1.5",
            "--inclination",
            "85",
            "--heading",
            "90",
            "--wind",
            "5",
            "--wind-from",
            "-90",
        ],
        0,
        "sim.schema.json",
    );
    let design: hpr::hpr_design::Rocket =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    // A wind from -90°, which is 270°: a west wind.
    let environment = hpr::Environment::new(32.99, -106.97, 1400.0)
        .unwrap()
        .with_constant_wind(5.0, -90.0)
        .unwrap();
    let (flight, _) = library_flight(design.clone(), "i175", "", None, &environment, |b| {
        b.inclination_deg(85.0).heading_deg(90.0)
    });
    same_flight(&document, &flight);
    assert_eq!(document["design"]["format"], "hpr_json");
    assert_eq!(document["motors"][0]["source"]["kind"], "design");
    assert_eq!(document["motors"][0]["ignition"], "at launch");
    assert_eq!(document["motors"][0]["count"], 1);
    assert_eq!(document["launch"]["longitude_deg"], -106.97);
    assert_eq!(document["launch"]["wind_from_deg"], -90.0);
    // The rail leans east, downwind: the rocket climbs to apogee east of where the same rail
    // leaning west, into the wind, puts it. (Where it lands isn't asserted: with no recovery
    // device the descent isn't a prediction, #241.)
    let (upwind, _) = library_flight(design, "i175", "", None, &environment, |b| {
        b.inclination_deg(85.0).heading_deg(270.0)
    });
    let east_at_apogee = |flight: &hpr::Flight| {
        let mut events = flight.result().events.iter();
        let apogee = events
            .rfind(|e| e.kind == hpr::hpr_sim::EventKind::Apogee)
            .unwrap();
        apogee.sample.cg_enu_m.x
    };
    let (east, west) = (east_at_apogee(&flight), east_at_apogee(&upwind));
    assert!(east > west + 10.0, "{east} m, {west} m");
}

/// What `hpr sim` refuses, each with status 1 and an error document naming the reason.
#[test]
fn sim_refuses_what_it_cant_fly() {
    let folder = tempfile::tempdir().unwrap();
    let probe = repo_file(PROBE);
    let refused = |args: &[&str], reason: &str| {
        let document = json_error(args, 1, "input");
        let message = document["error"]["message"].as_str().unwrap().to_owned();
        assert!(message.contains(reason), "{args:?}: {message}");
        assert_eq!(document["error"]["command"], "sim");
        let output = hpr(args);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(text(&output.stderr).contains(reason));
    };
    // The file's own motor has no curve in the bundled catalog.
    refused(&["sim", &probe], "no thrust curve for H128W");
    refused(&["sim", &probe], "give a motor with --motor");
    refused(
        &["sim", &probe, "--config", "nope"],
        "no motor configuration `nope`",
    );
    refused(
        &["sim", &probe, "--motor", "H54", "--mount", "nope"],
        "no motor mount `nope`",
    );
    refused(
        &["sim", &probe, "--motor", "Z9999"],
        "no motor with a bundled thrust curve",
    );
    refused(
        &["sim", &probe, "--motor", "I175"],
        "matches several motors",
    );
    let text_export = folder.path().join("f.txt");
    refused(
        &[
            "sim",
            &probe,
            "--motor",
            "H54",
            "--export",
            &text_export.to_string_lossy(),
        ],
        "--export writes",
    );
    assert!(!text_export.exists());
    refused(&["sim", "missing.ork"], "missing.ork");
    refused(
        &["sim", "design.rkt"],
        "reads an OpenRocket .ork file or an hpr design file",
    );
    let not_json = folder.path().join("design.json");
    std::fs::write(&not_json, "{}").unwrap();
    refused(
        &["sim", &not_json.to_string_lossy()],
        "not an hpr design file",
    );
    refused(
        &[
            "sim",
            &repo_file("validation/fixtures/ork/loft-demo/demo-multi-config.ork"),
            "--motor",
            "H54",
        ],
        "--accept-design-errors",
    );

    // The two-stage design with its sustainer lit by the separation, which never comes.
    let path = repo_file("validation/designs/synthetic-two-stage-75mm-54mm.json");
    let mut design: hpr::hpr_design::Rocket =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    for motor in &mut design.configurations[0].motors {
        if motor.mount == "sustainer-motor-mount" {
            motor.ignition = hpr::hpr_design::Ignition::Separation { delay_s: 1.0 };
        }
    }
    let separated = folder.path().join("separated.json");
    std::fs::write(&separated, serde_json::to_string(&design).unwrap()).unwrap();
    refused(
        &["sim", &separated.to_string_lossy()],
        "at its stage's separation",
    );

    // The staging example's rocket: its booster drops away under power, which the library
    // flies and `hpr sim` doesn't yet, with its own motors or another.
    let example =
        std::fs::read_to_string(root().join("crates/hpr/examples/ork_two_stage.rs")).unwrap();
    let start = example.find("r#\"").unwrap() + 3;
    let end = example[start..].find("\"#").unwrap() + start;
    let staged = folder.path().join("two-stage.ork");
    std::fs::write(&staged, &example[start..end]).unwrap();
    let staged = staged.to_string_lossy().into_owned();
    refused(&["sim", &staged], "separates under power");
    refused(
        &["sim", &staged, "--motor", "F15"],
        "drops away under power",
    );

    // The probe with a parallel stage hpr doesn't read: its airframe isn't the file's, so no
    // motor flies it, though its configuration's first reason is the missing curve.
    let xml = std::fs::read_to_string(&probe).unwrap();
    let at = xml.find("<bodytube>").unwrap();
    let inside = xml[at..].find("<subcomponents>").unwrap() + at + "<subcomponents>".len();
    let boosters = format!(
        "{}<parallelstage><name>Boosters</name><id>boosters</id>\
         <instancecount>2</instancecount></parallelstage>{}",
        &xml[..inside],
        &xml[inside..]
    );
    let reduced = folder.path().join("reduced.ork");
    std::fs::write(&reduced, boosters).unwrap();
    let reduced = reduced.to_string_lossy().into_owned();
    refused(&["sim", &reduced], "no thrust curve for H128W");
    refused(
        &["sim", &reduced, "--motor", "H54"],
        "not read exactly as written",
    );
}

/// A design flown past its failed checks says so in its notes.
#[test]
fn sim_notes_accepted_design_errors() {
    let path = repo_file("validation/fixtures/ork/loft-demo/demo-multi-config.ork");
    let document = json(
        &["sim", &path, "--motor", "H54", "--accept-design-errors"],
        0,
        "sim.schema.json",
    );
    let notes: Vec<&str> = document["notes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|note| note.as_str().unwrap())
        .collect();
    assert!(
        notes
            .iter()
            .any(|note| note.contains("motor_wider_than_mount")),
        "{notes:?}"
    );
    assert!(
        notes
            .iter()
            .any(|note| note.contains("recovery device is not flown")),
        "{notes:?}"
    );
}

/// The probe's own motor, as written: an AeroTech H128W with OpenRocket's digest of its curve.
const PROBE_MOTOR: &str = "<manufacturer>AeroTech</manufacturer><designation>H128W</designation>\
                           <digest>501239de7374691072270406476cb243</digest>\
                           <diameter>0.029</diameter><length>0.194</length>";

/// The probe with `from` replaced by `to`, written to `folder` as `name`; its path.
fn probe_with(folder: &Path, name: &str, from: &str, to: &str) -> String {
    let xml = std::fs::read_to_string(repo_file(PROBE)).unwrap();
    assert_eq!(xml.matches(from).count(), 1, "{from}");
    let path = folder.join(name);
    std::fs::write(&path, xml.replacen(from, to, 1)).unwrap();
    path.to_string_lossy().into_owned()
}

/// A `.ork` flies its own configuration as the library flies it, when its motor is one the
/// catalog holds: the probe with the H54 written in place of its H128W.
#[test]
fn sim_flies_a_orks_own_motor_as_the_library_does() {
    let folder = tempfile::tempdir().unwrap();
    let path = probe_with(
        folder.path(),
        "own.ork",
        PROBE_MOTOR,
        "<manufacturer>Cesaroni Technology</manufacturer><designation>168H54-10A</designation>\
         <diameter>0.029</diameter><length>0.187</length>",
    );
    let document = json(&["sim", &path], 0, "sim.schema.json");
    let file = hpr::hpr_io::ork::read(&std::fs::read(&path).unwrap())
        .unwrap()
        .value;
    let design = hpr::hpr_io::ork::design(&file).value;
    let id = design.motors.default_configuration().unwrap().id.clone();
    let environment = hpr::Environment::new(0.0, 0.0, 0.0).unwrap();
    let (flight, _) = library_flight(design.rocket, &id, "", None, &environment, |b| b);
    same_flight(&document, &flight);
    assert_eq!(document["motors"][0]["source"]["kind"], "design");
    assert_eq!(document["motors"][0]["designation"], "168H54-10A");
    assert_eq!(document["design"]["configurations"][0]["flies"], true);
}

/// A `.ork` with no motor configuration flies `--motor` in its only mount, in a configuration
/// named after the motor, as the library flies it there.
#[test]
fn sim_flies_a_motor_in_a_file_with_no_configuration() {
    let folder = tempfile::tempdir().unwrap();
    let xml = std::fs::read_to_string(repo_file(PROBE)).unwrap();
    let cut = |xml: &str, open: &str, close: &str| {
        let start = xml.find(open).unwrap();
        let end = xml[start..].find(close).unwrap() + start + close.len();
        format!("{}{}", &xml[..start], &xml[end..])
    };
    let bare = cut(&xml, "<motor configid", "</motor>");
    let bare = cut(&bare, "<motorconfiguration ", "</motorconfiguration>");
    let path = folder.path().join("bare.ork");
    std::fs::write(&path, bare).unwrap();
    let path = path.to_string_lossy().into_owned();
    let document = json_error(&["sim", &path], 1, "input");
    let message = document["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("no motor configuration: give a motor with --motor"),
        "{message}"
    );

    let document = json(&["sim", &path, "--motor", "H54"], 0, "sim.schema.json");
    assert_eq!(document["design"]["configuration"], "168H54-10A");
    assert_eq!(document["design"]["configurations"], serde_json::json!([]));
    let (rocket, _, mount) = probe_design();
    let motor = hpr::Motor::from_catalog("H54").unwrap();
    let environment = hpr::Environment::new(0.0, 0.0, 0.0).unwrap();
    let (flight, _) = library_flight(
        rocket,
        "168H54-10A",
        &mount,
        Some(&motor),
        &environment,
        |b| b,
    );
    same_flight(&document, &flight);
}

/// What `--motor` refuses on top of the rest: a configuration that switches a stage off, a hybrid
/// motor, and a pod set's motors are counted, one per pod.
#[test]
fn sim_motor_refusals_and_counts() {
    let folder = tempfile::tempdir().unwrap();
    let off = probe_with(
        folder.path(),
        "off.ork",
        r#"<stage number="0" active="true"/>"#,
        r#"<stage number="0" active="false"/>"#,
    );
    let document = json_error(&["sim", &off, "--motor", "H54"], 1, "input");
    let message = document["error"]["message"].as_str().unwrap();
    assert!(message.contains("switches a stage off"), "{message}");
    let document = json_error(&["sim", &off], 1, "input");
    let message = document["error"]["message"].as_str().unwrap();
    assert!(message.contains("nor can it with --motor"), "{message}");

    let rse = std::fs::read_to_string(curve_file("curves/5f923edb1bca5800041716ab.rse")).unwrap();
    let at = rse.find("Type=\"").unwrap() + "Type=\"".len();
    let end = rse[at..].find('"').unwrap() + at;
    let hybrid = folder.path().join("hybrid.rse");
    std::fs::write(&hybrid, format!("{}hybrid{}", &rse[..at], &rse[end..])).unwrap();
    let probe = repo_file(PROBE);
    let document = json_error(
        &["sim", &probe, "--motor", &hybrid.to_string_lossy()],
        1,
        "input",
    );
    let message = document["error"]["message"].as_str().unwrap();
    assert!(message.contains("hybrid"), "{message}");

    let pods = repo_file("validation/fixtures/ork/pod-flights/pods-motors-2.ork");
    let document = json(&["sim", &pods, "--motor", "H54"], 0, "sim.schema.json");
    assert_eq!(document["motors"][0]["count"], 2);
    assert_eq!(document["motors"][0]["unlit"], 0);
}

/// `--export` never writes over a file the run reads, nor the same file twice, and a missing
/// folder or a bad interval is refused before the flight.
#[test]
fn sim_exports_are_checked_before_the_flight() {
    let folder = tempfile::tempdir().unwrap();
    let design = folder.path().join("design.json");
    std::fs::copy(
        repo_file("validation/designs/synthetic-54mm-three-fin.json"),
        &design,
    )
    .unwrap();
    let before = std::fs::read(&design).unwrap();
    let design = design.to_string_lossy().into_owned();
    let csv = folder.path().join("f.csv").to_string_lossy().into_owned();
    let lost = folder
        .path()
        .join("no/f.csv")
        .to_string_lossy()
        .into_owned();
    for (args, reason) in [
        (
            vec!["--export", design.as_str()],
            "this run reads that file",
        ),
        (
            vec!["--export", &csv, "--export", &csv],
            "names the file twice",
        ),
        (vec!["--export", &lost], "there is no folder"),
        (vec!["--interval", "0"], "--interval"),
    ] {
        let mut line = vec!["sim", design.as_str()];
        line.extend(args);
        let document = json_error(&line, 1, "input");
        let message = document["error"]["message"].as_str().unwrap();
        assert!(message.contains(reason), "{line:?}: {message}");
    }
    assert_eq!(std::fs::read(&design).unwrap(), before);
    assert!(!Path::new(&csv).exists());
}

/// The guide's launch section says the example rocket climbs about 8% higher from a 1,400 m site
/// than from sea level, and that 45° N moves its apogee by less than 0.2%.
#[test]
fn the_guides_launch_figures_hold() {
    let probe = repo_file(PROBE);
    let apogee = |extra: &[&str]| {
        let mut args = vec!["sim", probe.as_str(), "--motor", "H54"];
        args.extend(extra);
        json(&args, 0, "sim.schema.json")["summary"]["apogee"]["height_above_ground_m"]
            .as_f64()
            .unwrap()
    };
    let sea = apogee(&[]);
    let high = apogee(&["--elevation", "1400"]) / sea - 1.0;
    assert!((0.075..0.085).contains(&high), "{high}");
    let north = apogee(&["--latitude", "45"]) / sea - 1.0;
    assert!(north.abs() < 0.002, "{north}");
}
