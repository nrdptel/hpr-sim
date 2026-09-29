use hpr_core::geodesy::Geodetic;
use hpr_io::ork::{CaseSize, SuppliedCurves};
use hpr_motor::catalog::Catalog;
use hpr_sim::{Environment, EventKind, FlightSettings, Rail, Simulation};

use super::*;

/// The public `.ork` designs committed under `validation/fixtures/ork/`: Loft's seven demonstration
/// designs and the OpenRocket probes of M2.2e4 and M2.2e5.
const PUBLIC: [(&str, &[u8]); 17] = [
    (
        "demo-boattail",
        include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-boattail.ork"),
    ),
    (
        "demo-dual-deploy",
        include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-dual-deploy.ork"),
    ),
    (
        "demo-multi-config",
        include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-multi-config.ork"),
    ),
    (
        "demo-payload-separation",
        include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-payload-separation.ork"),
    ),
    (
        "demo-quirks",
        include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-quirks.ork"),
    ),
    (
        "demo-single-deploy",
        include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-single-deploy.ork"),
    ),
    (
        "demo-stable",
        include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-stable.ork"),
    ),
    (
        "pods-bodies-3",
        include_bytes!("../../../validation/fixtures/ork/pod-flights/pods-bodies-3.ork"),
    ),
    (
        "pods-fins-2",
        include_bytes!("../../../validation/fixtures/ork/pod-flights/pods-fins-2.ork"),
    ),
    (
        "pods-fins-tail-4",
        include_bytes!("../../../validation/fixtures/ork/pod-flights/pods-fins-tail-4.ork"),
    ),
    (
        "pods-motors-2",
        include_bytes!("../../../validation/fixtures/ork/pod-flights/pods-motors-2.ork"),
    ),
    (
        "pods-none",
        include_bytes!("../../../validation/fixtures/ork/pod-flights/pods-none.ork"),
    ),
    (
        "pods-winglets-2",
        include_bytes!("../../../validation/fixtures/ork/pod-flights/pods-winglets-2.ork"),
    ),
    (
        "rod-5-north",
        include_bytes!("../../../validation/fixtures/ork/rod-flights/rod-5-north.ork"),
    ),
    (
        "rod-10-east",
        include_bytes!("../../../validation/fixtures/ork/rod-flights/rod-10-east.ork"),
    ),
    (
        "rod-10-southwest",
        include_bytes!("../../../validation/fixtures/ork/rod-flights/rod-10-southwest.ork"),
    ),
    (
        "rod-20-east",
        include_bytes!("../../../validation/fixtures/ork/rod-flights/rod-20-east.ork"),
    ),
];

/// The committed schema.
const COMMITTED_SCHEMA: &str = include_str!("../../../schema/format/hpr-design-0.2.schema.json");

fn validator() -> jsonschema::Validator {
    let schema: serde_json::Value = serde_json::from_str(COMMITTED_SCHEMA).unwrap();
    jsonschema::validator_for(&schema).unwrap()
}

/// Every error the committed schema finds in `text`, as one line each.
fn schema_errors(validator: &jsonschema::Validator, text: &str) -> Vec<String> {
    let value: serde_json::Value = serde_json::from_str(text).unwrap();
    validator
        .iter_errors(&value)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

fn document(bytes: &[u8]) -> DesignFile {
    DesignFile::from_ork(bytes).unwrap().value
}

fn file_design(file: &ork::OrkFile) -> Design {
    ork::design(file).value
}

/// The document of `design`, read from `bytes`, with the file's own attachments.
fn document_of(design: Design, bytes: &[u8]) -> DesignFile {
    let provenance = Provenance::hpr(Some(Source::of(SourceFormat::Ork, bytes)));
    DesignFile::new(
        design,
        provenance,
        &ork::read(bytes).unwrap().value.attachments,
    )
}

#[test]
fn the_committed_schema_is_the_generated_one() {
    // `cargo xtask format` rewrites it.
    assert_eq!(
        COMMITTED_SCHEMA,
        schema_json(),
        "schema/format/hpr-design-0.2.schema.json is stale: run `cargo xtask format`"
    );
}

#[test]
fn every_public_design_round_trips_through_the_document() {
    let validator = validator();
    for (name, bytes) in PUBLIC {
        let read = document(bytes);
        let text = to_json(&read).unwrap();
        // The document follows the committed schema, reads back the same, and writes again the
        // same bytes.
        assert_eq!(
            schema_errors(&validator, &text),
            Vec::<String>::new(),
            "{name}"
        );
        let back = from_json(&text).unwrap();
        assert_eq!(back, read, "{name}");
        assert_eq!(to_json(&back).unwrap(), text, "{name}");
        // `.ork` → document → `.ork` writes the `.ork` hpr writes from the file itself, its
        // archive's other entries and all, and that `.ork` reads back as the design first read.
        let file = ork::read(bytes).unwrap().value;
        let direct = ork::export::write(&file_design(&file), &file.attachments)
            .unwrap()
            .value;
        let through = back.to_ork().unwrap().value;
        assert_eq!(through, direct, "{name}");
        let again = ork::design(&ork::read(&through).unwrap().value).value;
        assert_eq!(again, read.design(), "{name}");
    }
}

#[test]
fn the_provenance_names_the_program_and_the_source() {
    let (_, bytes) = PUBLIC[0];
    let read = document(bytes);
    assert_eq!(read.provenance.tool, "hpr-sim");
    assert_eq!(read.provenance.tool_version, env!("CARGO_PKG_VERSION"));
    let source = read.provenance.source.unwrap();
    assert_eq!(source.format, SourceFormat::Ork);
    // The SHA-256 of `demo-boattail.ork`, by `shasum -a 256`.
    assert_eq!(
        source.sha256,
        "cb96b09570a5a084dbc43bfeac2af9df449ed52f5fdd8fa991c11b94d212da88"
    );
    // The SHA-256 of no bytes, FIPS 180-4's known answer.
    assert_eq!(
        Source::of(SourceFormat::Ork, b"").sha256,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn the_text_is_canonical() {
    let (_, bytes) = PUBLIC[1];
    let read = document(bytes);
    let text = to_json(&read).unwrap();
    assert!(text.starts_with(
        "{\n  \"format\": \"hpr-design\",\n  \"version\": \"0.2\",\n  \"provenance\": {\n    \
         \"tool\": \"hpr-sim\","
    ));
    assert!(text.ends_with("}\n") && !text.ends_with("\n\n"));
    // The top-level keys, in order.
    let keys: Vec<_> = text
        .lines()
        .filter_map(|line| line.strip_prefix("  \""))
        .filter_map(|line| line.split_once('"').map(|(key, _)| key))
        .collect();
    assert_eq!(
        keys,
        [
            "format",
            "version",
            "provenance",
            "rocket",
            "motors",
            "recovery",
            "simulations",
            "extensions",
            "source_files"
        ]
    );
    // The same design always writes the same bytes.
    assert_eq!(to_json(&document(bytes)).unwrap(), text);
}

/// Flies every configuration of `design` that flies, calm and standard at sea level off a 1.5 m
/// vertical rail, the stack whole; returns each configuration's id and apogee above the start, m.
fn apogees(design: &Design) -> Vec<(String, f64)> {
    let site = Geodetic::from_degrees(0.0, 0.0, 0.0).unwrap();
    design
        .rocket
        .configurations
        .iter()
        .map(|configuration| {
            let environment = Environment::standard(site).unwrap();
            let settings = FlightSettings {
                accept_design_errors: true,
                ..FlightSettings::default()
            };
            let simulation = Simulation::new(
                &design.rocket,
                &configuration.id,
                environment,
                Rail::vertical(1.5),
                settings,
            )
            .unwrap();
            let result = simulation.run(&mut ()).unwrap();
            let apogee = result.event(EventKind::Apogee).unwrap();
            (
                configuration.id.clone(),
                apogee.sample.height_above_ground_m,
            )
        })
        .collect()
}

/// The bundled motor of `designation`, or the bundled motor nearest `diameter_m`.
fn stand_in(catalog: &Catalog, designation: &str, diameter_m: f64) -> hpr_motor::SolidMotor {
    let named = catalog
        .find(designation)
        .find_map(|entry| entry.bundled_motor().ok());
    let nearest = || {
        catalog
            .motors
            .iter()
            .filter_map(|entry| {
                let motor = entry.bundled_motor().ok()?;
                Some(((entry.diameter_mm * 1e-3 - diameter_m).abs(), motor))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, motor)| motor)
    };
    named.or_else(nearest).unwrap()
}

/// `bytes`' design, made to fly. None of the public designs carries a curve hpr flies on its own,
/// so bundled motors stand in for OpenRocket's database: each motor that names a curve's digest
/// is supplied the bundled motor of its designation, or the one nearest its diameter, in the case
/// the design gives it; and each configuration left out whose motors name no digest (Loft's
/// demonstrations) is added to the rocket with such motors, lit at launch. The comparison is of a
/// design with itself, so it needs a flight, not the design's own motor.
fn flyable(bytes: &[u8]) -> Design {
    let catalog = Catalog::bundled().unwrap();
    let file = ork::read(bytes).unwrap().value;
    let mut curves = SuppliedCurves::new("the bundled motors, standing in");
    for motor in ork::design(&file)
        .value
        .motors
        .configurations
        .iter()
        .flat_map(|configuration| &configuration.motors)
    {
        if let (Some(digest), Some(diameter_m), Some(length_m)) =
            (&motor.digest, motor.diameter_m, motor.length_m)
        {
            let solid = stand_in(&catalog, &motor.designation, diameter_m);
            let case = CaseSize {
                diameter_m,
                length_m,
            };
            curves.insert(digest.clone(), case, solid).unwrap();
        }
    }
    let mut design = ork::design_with(&file, &curves).value;
    let added: Vec<_> = design
        .motors
        .configurations
        .iter()
        .filter(|configuration| configuration.left_out.is_some())
        .filter(|configuration| {
            configuration
                .motors
                .iter()
                .all(|motor| motor.digest.is_none())
        })
        .filter_map(|configuration| {
            let motors = configuration
                .motors
                .iter()
                .map(|motor| {
                    let (diameter_m, length_m) = (motor.diameter_m?, motor.length_m?);
                    Some(hpr_design::MountedMotor {
                        mount: motor.mount.clone(),
                        designation: motor.designation.clone(),
                        diameter_m,
                        length_m,
                        motor: stand_in(&catalog, &motor.designation, diameter_m),
                        delay: motor.delay,
                        ignition: hpr_design::Ignition::Launch,
                        failed_tubes: Vec::new(),
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(hpr_design::Configuration {
                id: configuration.id.clone(),
                name: configuration.name.clone(),
                motors,
            })
        })
        .collect();
    design.rocket.configurations.extend(added);
    design
}

#[test]
fn a_design_read_from_its_document_flies_to_the_same_apogee() {
    let validator = validator();
    let mut flown = 0;
    for (name, bytes) in PUBLIC {
        let design = flyable(bytes);
        let text = to_json(&document_of(design.clone(), bytes)).unwrap();
        // Supplied curves and whole motors: the schema covers them too.
        assert_eq!(
            schema_errors(&validator, &text),
            Vec::<String>::new(),
            "{name}"
        );
        let back = from_json(&text).unwrap();
        let before = apogees(&design);
        // Read from the document, the design flies the same flight, bit for bit.
        assert_eq!(apogees(&back.design()), before, "{name}");
        // The `.ork` written from the document holds the design's motors and sizes but not its
        // bundled stand-ins, so the curves are supplied again the same way.
        let written = back.to_ork().unwrap().value;
        let again = flyable(&written);
        let after = apogees(&again);
        assert_eq!(after.len(), before.len(), "{name}");
        for ((id, apogee), (id_again, apogee_again)) in before.iter().zip(&after) {
            assert_eq!(id, id_again, "{name}");
            let relative = ((apogee_again - apogee) / apogee).abs();
            assert!(
                relative <= 1e-9,
                "{name} {id}: {apogee} m, then {apogee_again} m"
            );
        }
        flown += before.len();
    }
    // Every configuration of every public design: one each, and demo-multi-config's two.
    assert_eq!(flown, PUBLIC.len() + 1);
}

/// A thrust curve no catalog lists, for an embedded entry: 100 N for a second, ramped up over
/// 0.05 s and down over 0.1 s, so 102.5 N·s from 60 g of propellant. `{DIA}` and `{LEN}` are the
/// case's diameter and length, mm.
const INVENTED_RSE: &str = r#"<engine-database>
  <engine-list>
    <engine mfg="Nobody" code="H128W" Type="single-use" dia="{DIA}" len="{LEN}" initWt="150."
      propWt="60." delays="6" Itot="102.5" burn-time="1.1">
      <data>
        <eng-data t="0." f="0." m="60."/>
        <eng-data t="0.05" f="100." m="58."/>
        <eng-data t="1." f="100." m="3."/>
        <eng-data t="1.1" f="0." m="0."/>
      </data>
    </engine>
  </engine-list>
</engine-database>"#;

/// Bytes that are not UTF-8, standing in for a decal image: a PNG's signature and a few more.
const IMAGE: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0xff, 0xfe, 0x80,
];

/// A public design with an embedded thrust curve and a decal, archived: the design, the curve's
/// entry name and text.
fn with_attachments() -> (Vec<u8>, String, String) {
    let (_, bytes) = PUBLIC[13];
    let design = ork::design(&ork::read(bytes).unwrap().value).value;
    let motor = &design.motors.configurations[0].motors[0];
    let entry = format!("thrustcurves/{}.rse", motor.digest.clone().unwrap());
    let rse = INVENTED_RSE
        .replace("{DIA}", &format!("{}", motor.diameter_m.unwrap() * 1e3))
        .replace("{LEN}", &format!("{}", motor.length_m.unwrap() * 1e3));
    let attachments = [
        Attachment {
            name: "decals/invented.png".to_owned(),
            bytes: IMAGE.to_vec(),
        },
        Attachment {
            name: entry.clone(),
            bytes: rse.clone().into_bytes(),
        },
    ];
    let archive = ork::export::write(&design, &attachments).unwrap().value;
    (archive, entry, rse)
}

/// Loft lesson L57's trap, one step on: a `.ork` that embeds its motor's curve and a decal, taken
/// through the document and written back out, still carries both, and flies on the curve with none
/// supplied.
#[test]
fn a_files_other_entries_travel_with_the_document() {
    let (archive, entry, rse) = with_attachments();
    let read = document(&archive);
    assert!(matches!(
        read.motors.configurations[0].motors[0].curve,
        Curve::Embedded { .. }
    ));
    assert_eq!(
        read.source_files,
        [
            SourceFile {
                name: "decals/invented.png".to_owned(),
                content: Content::Base64("iVBORw0KGgoA//6A".to_owned()),
            },
            SourceFile {
                name: entry.clone(),
                content: Content::Text(rse.clone()),
            },
        ]
    );
    let text = to_json(&read).unwrap();
    assert_eq!(schema_errors(&validator(), &text), Vec::<String>::new());
    let back = from_json(&text).unwrap();
    let written = back.to_ork().unwrap().value;
    // The `.ork` hpr writes from the archive itself, byte for byte.
    let file = ork::read(&archive).unwrap().value;
    let direct = ork::export::write(&file_design(&file), &file.attachments)
        .unwrap()
        .value;
    assert_eq!(written, direct);
    let file = ork::read(&written).unwrap().value;
    assert_eq!(file.attachment(&entry).unwrap().bytes, rse.as_bytes());
    assert_eq!(file.attachment("decals/invented.png").unwrap().bytes, IMAGE);
    let again = ork::design(&file).value;
    assert_eq!(again, read.design());
    let before = apogees(&read.design());
    assert_eq!(before.len(), 1);
    assert_eq!(apogees(&again), before);
}

/// What the source files must hold to, beyond their types: base64 that decodes, one name each, and
/// every embedded curve's entry.
#[test]
fn source_files_that_do_not_hold_together_are_refused() {
    let (archive, entry, _) = with_attachments();
    let text = to_json(&document(&archive)).unwrap();
    let swap = |from: &str, to: &str| {
        assert!(text.contains(from), "{from}");
        from_json(&text.replacen(from, to, 1))
            .unwrap_err()
            .to_string()
    };
    assert!(swap("iVBORw0KGgoA//6A", "iVBORw0KGgoA//6").contains("decals/invented.png"));
    assert!(swap("decals/invented.png", &entry).contains("two source files are named"));
    for name in ["rocket.ork", "decals/", "decals\\", ""] {
        let refused = swap("\"decals/invented.png\"", &format!("{name:?}"));
        assert!(refused.contains("which a .ork can't hold"), "{refused}");
    }
    assert!(
        swap(
            &format!("\"name\": \"{entry}\""),
            "\"name\": \"elsewhere.rse\""
        )
        .contains("which the source files don't hold")
    );
}

/// A document whose rocket is RocketPy's Bella Lui, one of the validation designs: its motor has a
/// nozzle, whose reference pressure is required and `null`, and BATES grains.
fn with_a_nozzle() -> DesignFile {
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-bella-lui.json"
    ))
    .unwrap();
    let mut read = document(PUBLIC[0].1);
    read.rocket = rocket;
    read
}

#[test]
fn a_nozzle_follows_the_schema() {
    let text = to_json(&with_a_nozzle()).unwrap();
    assert_eq!(schema_errors(&validator(), &text), Vec::<String>::new());
    assert!(text.contains("\"reference_pressure_pa\": null"));
    assert!(text.contains("\"model\": \"grains\""));
}

/// The schema and the reader agree on which keys a document can leave out: each key of two rich
/// documents is taken out in turn, and the schema takes the document if and only if the reader
/// does.
#[test]
fn the_schema_and_the_reader_agree_on_every_key() {
    let validator = validator();
    let (archive, _, _) = with_attachments();
    let dual_deploy = to_json(&document_of(flyable(PUBLIC[1].1), PUBLIC[1].1)).unwrap();
    let mut checked = 0;
    for text in [
        to_json(&document(&archive)).unwrap(),
        dual_deploy,
        to_json(&with_a_nozzle()).unwrap(),
    ] {
        let whole: serde_json::Value = serde_json::from_str(&text).unwrap();
        let mut objects = vec![String::new()];
        while let Some(pointer) = objects.pop() {
            let Some(serde_json::Value::Object(object)) = whole.pointer(&pointer) else {
                if let Some(serde_json::Value::Array(items)) = whole.pointer(&pointer) {
                    objects.extend((0..items.len()).map(|index| format!("{pointer}/{index}")));
                }
                continue;
            };
            for key in object.keys() {
                let at = format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                objects.push(at.clone());
                let mut without = whole.clone();
                if let Some(serde_json::Value::Object(parent)) = without.pointer_mut(&pointer) {
                    parent.remove(key);
                }
                let spoiled = serde_json::to_string(&without).unwrap();
                let schema_takes = validator.is_valid(&without);
                let reader_takes = from_json(&spoiled).is_ok();
                assert_eq!(schema_takes, reader_takes, "without {at}");
                checked += 1;
            }
        }
    }
    assert!(checked > 500, "{checked} keys");
}

#[test]
fn another_format_or_version_is_refused_with_its_reason() {
    let (_, bytes) = PUBLIC[0];
    let text = to_json(&document(bytes)).unwrap();
    let swap = |from: &str, to: &str| {
        assert!(text.contains(from));
        from_json(&text.replacen(from, to, 1)).unwrap_err()
    };
    assert!(matches!(from_json("{"), Err(FormatError::Json(_))));
    assert_eq!(
        from_json("{}").unwrap_err(),
        FormatError::NotADesign {
            found: "missing".to_owned()
        }
    );
    assert_eq!(
        swap("\"format\": \"hpr-design\"", "\"format\": \"rocksim\""),
        FormatError::NotADesign {
            found: "\"rocksim\"".to_owned()
        }
    );
    let newer = swap("\"version\": \"0.2\"", "\"version\": \"0.3\"");
    assert_eq!(
        newer,
        FormatError::Unsupported {
            found: Version { major: 0, minor: 3 },
            supported: VERSION
        }
    );
    assert!(newer.to_string().ends_with("written by a newer program"));
    // 0.0 was never a version: nothing migrates from it.
    let older = swap("\"version\": \"0.2\"", "\"version\": \"0.0\"");
    assert_eq!(
        older.to_string(),
        "the document is version 0.0, and this reader takes version 0.2 and migrates from 0.1 on"
    );
    assert!(matches!(
        swap("\"version\": \"0.2\"", "\"version\": \"1.2\""),
        FormatError::Unsupported { .. }
    ));
    for bad in ["01.1", "0.01", "1", "0.1.0", "+0.1", " 0.1", "0.x", ""] {
        let refused = swap("\"version\": \"0.2\"", &format!("\"version\": \"{bad}\""));
        assert!(
            matches!(refused, FormatError::Invalid(_)),
            "{bad:?}: {refused}"
        );
    }
    assert!(matches!(
        swap("\"version\": \"0.2\"", "\"version\": 0.2"),
        FormatError::Invalid(_)
    ));
    // A key the version does not have, at the top, among the extensions or inside a part of the
    // design, is refused rather than dropped, by the reader and by the schema.
    let validator = validator();
    for (from, to) in [
        (
            "\"format\": \"hpr-design\",",
            "\"format\": \"hpr-design\",\n  \"colour\": 1,",
        ),
        (
            "\"extensions\": {",
            "\"extensions\": {\n    \"x-rocksim\": {},",
        ),
        (
            "\"manufacturer\": ",
            "\"colour\": 1,\n          \"manufacturer\": ",
        ),
    ] {
        let refused = swap(from, to);
        assert!(matches!(refused, FormatError::Invalid(_)), "{refused}");
        let spoiled = text.replacen(from, to, 1);
        assert!(!schema_errors(&validator, &spoiled).is_empty(), "{to}");
    }
}

#[test]
fn a_value_json_cannot_carry_is_refused_when_written() {
    let (_, bytes) = PUBLIC[0];
    let mut read = document(bytes);
    read.provenance.source = None;
    assert!(to_json(&read).is_ok());
    for value in [f64::NAN, f64::INFINITY] {
        read.motors.configurations[0].motors[0].diameter_m = Some(value);
        assert!(
            matches!(to_json(&read), Err(FormatError::NotRepresentable(_))),
            "{value}"
        );
    }
}

#[test]
fn a_document_of_another_version_is_not_written() {
    let (_, bytes) = PUBLIC[0];
    let mut read = document(bytes);
    for version in [
        Version { major: 0, minor: 1 },
        Version { major: 0, minor: 3 },
    ] {
        read.version = version;
        assert_eq!(
            to_json(&read).unwrap_err().to_string(),
            format!(
                "not a valid hpr design 0.2: its \"version\" is {version}, and this program \
                 writes only 0.2"
            )
        );
    }
}

#[test]
fn other_json_is_refused_as_what_it_is() {
    for (text, found) in [
        ("[]", "absent: the JSON is an array, not an object"),
        ("42", "absent: the JSON is a number, not an object"),
        ("{\"format\": 7}", "7"),
    ] {
        assert_eq!(
            from_json(text),
            Err(FormatError::NotADesign {
                found: found.to_owned()
            }),
            "{text}"
        );
    }
    // A long value is quoted cut short.
    let long = format!("{{\"format\": \"{}\"}}", "x".repeat(1000));
    let Err(FormatError::NotADesign { found }) = from_json(&long) else {
        panic!("{long}");
    };
    assert_eq!(found.chars().count(), 41, "{found}");
    // A byte-order mark before the document is not an error.
    let (_, bytes) = PUBLIC[0];
    let text = to_json(&document(bytes)).unwrap();
    assert_eq!(
        from_json(&format!("\u{feff}{text}")).unwrap(),
        from_json(&text).unwrap()
    );
}

/// The schema's names become type names in generated code (M3.3c), so none is a name schemars
/// made up by numbering a clash.
#[test]
fn every_schema_name_is_chosen() {
    let schema: serde_json::Value = serde_json::from_str(COMMITTED_SCHEMA).unwrap();
    let names: Vec<_> = schema["$defs"].as_object().unwrap().keys().collect();
    assert!(names.len() > 50, "{names:?}");
    for name in names {
        assert!(!name.ends_with(|c: char| c.is_ascii_digit()), "{name}");
    }
}

#[test]
fn a_version_reads_as_it_writes() {
    for (text, version) in [
        ("0.1", Version { major: 0, minor: 1 }),
        (
            "10.20",
            Version {
                major: 10,
                minor: 20,
            },
        ),
    ] {
        assert_eq!(Version::try_from(text.to_owned()), Ok(version));
        assert_eq!(String::from(version), text);
    }
    assert!(Version::try_from("4294967296.0".to_owned()).is_err());
}

/// The format page's example is the start of the stable trainer's document, as written today.
#[test]
fn the_pages_example_is_the_document() {
    let page = include_str!("../../../docs/format/hpr.md");
    let (_, after) = page.split_once("```json\n").unwrap();
    let (example, _) = after.split_once("```").unwrap();
    let (_, bytes) = PUBLIC[6];
    let text = to_json(&document(bytes)).unwrap();
    assert!(text.starts_with(example), "{example}");
    assert!(example.lines().count() > 20);
}

/// A smoke test of many source files: a forged document with 30,000 reads back whole.
#[test]
fn many_source_files_are_read() {
    let mut read = document(PUBLIC[0].1);
    read.source_files = (0..30_000)
        .map(|index| SourceFile {
            name: format!("decals/{index}.png"),
            content: Content::Base64("AA==".to_owned()),
        })
        .collect();
    let text = to_json(&read).unwrap();
    assert_eq!(from_json(&text).unwrap().source_files.len(), 30_000);
}

/// A document of version 0.1, written by hpr's M3.3a reader (the format's first step) from
/// `with_attachments`'s archive, before 0.2 renamed `attachments` to `source_files`.
const DOCUMENT_0_1: &str = include_str!("../fixtures/embedded-curve-0.1.hpr");

/// Version 0.1's schema, committed beside the current one.
const SCHEMA_0_1: &str = include_str!("../../../schema/format/hpr-design-0.1.schema.json");

/// A document of the older version reads as the current version, holding everything it held: the
/// migration renames one key and changes nothing else, the design flies on the curve it embeds,
/// and it writes the `.ork` it came from.
#[test]
fn a_document_of_an_older_version_is_migrated_to_the_current_one() {
    // The fixture is a real 0.1 document: 0.1's schema takes it, and the current one doesn't.
    let old: serde_json::Value = serde_json::from_str(DOCUMENT_0_1).unwrap();
    let schema_0_1: serde_json::Value = serde_json::from_str(SCHEMA_0_1).unwrap();
    let errors: Vec<String> = jsonschema::validator_for(&schema_0_1)
        .unwrap()
        .iter_errors(&old)
        .map(|error| error.to_string())
        .collect();
    assert_eq!(errors, Vec::<String>::new());
    assert!(!schema_errors(&validator(), DOCUMENT_0_1).is_empty());

    let opened = read_json(DOCUMENT_0_1).unwrap();
    assert_eq!(opened.written_as, Version { major: 0, minor: 1 });
    let document = opened.value;
    assert_eq!(document.version, VERSION);
    let text = to_json(&document).unwrap();
    assert_eq!(schema_errors(&validator(), &text), Vec::<String>::new());
    // Key for key, the old document with `attachments` renamed and the version set.
    let mut expected = old.clone();
    let object = expected.as_object_mut().unwrap();
    let files = object.remove("attachments").unwrap();
    assert_eq!(files.as_array().map(Vec::len), Some(2));
    object.insert("source_files".to_owned(), files);
    object.insert("version".to_owned(), serde_json::json!("0.2"));
    let written: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(written, expected);

    // The design as the current reader makes it from the same archive, and the same `.ork` back.
    // The provenance is the fixture's own: it names the program's version then, and the SHA-256
    // of the archive as the `.ork` writer wrote it then, which later versions of either change.
    let (archive, entry, rse) = with_attachments();
    let mut fresh = document_of(file_design(&ork::read(&archive).unwrap().value), &archive);
    fresh.provenance = document.provenance.clone();
    assert_eq!(document, fresh);
    let written = document.to_ork().unwrap().value;
    let file = ork::read(&written).unwrap().value;
    assert_eq!(file.attachment(&entry).unwrap().bytes, rse.as_bytes());
    let flown = apogees(&document.design());
    assert_eq!(flown.len(), 1);
    assert_eq!(flown, apogees(&fresh.design()));
}

/// A 0.1 document that doesn't hold what its migration needs is refused with the reason, as is
/// one of 0.1 whose content is wrong once migrated.
#[test]
fn an_older_document_that_does_not_migrate_is_refused() {
    let swap = |from: &str, to: &str| {
        assert!(DOCUMENT_0_1.contains(from), "{from}");
        from_json(&DOCUMENT_0_1.replacen(from, to, 1))
            .unwrap_err()
            .to_string()
    };
    assert_eq!(
        swap("\"attachments\":", "\"not_attachments\":"),
        "not a valid hpr design 0.2: as a 0.1 document, it has no \"attachments\""
    );
    assert_eq!(
        swap(
            "\"attachments\":",
            "\"source_files\": [],\n  \"attachments\":"
        ),
        "not a valid hpr design 0.2: as a 0.1 document, it has a \"source_files\", which 0.1 \
         doesn't define"
    );
    let unknown = swap(
        "\"format\": \"hpr-design\",",
        "\"format\": \"hpr-design\",\n  \"colour\": 1,",
    );
    assert!(
        unknown.contains("migrated from 0.1") && unknown.contains("colour"),
        "{unknown}"
    );
    // The rules beyond the schema hold after a migration too.
    let broken = swap("iVBORw0KGgoA//6A", "iVBORw0KGgoA//6");
    assert!(broken.contains("decals/invented.png"), "{broken}");
}

fn hprz_of(attachments: Vec<container::Entry>) -> container::Hprz {
    let (archive, _, _) = with_attachments();
    container::Hprz::new(document(&archive), attachments)
}

/// A `.hprz` holds a design and its attachments and reads back the same, byte for byte; its first
/// entry is the design's `.hpr` text; and its bytes depend only on what it holds.
#[test]
fn a_container_reads_back_the_same() {
    let attachments = vec![
        container::Entry::new(
            "logs/flight-1.csv",
            b"time_s,altitude_m\n0,0\n1,12.5\n".to_vec(),
        ),
        container::Entry::new("photos/pad.png", IMAGE.to_vec()),
        container::Entry::new("empty.txt", Vec::new()),
        container::Entry::new("r\u{e9}sultats/vol \u{2116}1.json", b"{}".to_vec()),
    ];
    let hprz = hprz_of(attachments);
    let bytes = container::write(&hprz).unwrap();
    let read = container::read(&bytes).unwrap();
    assert_eq!(read.written_as, VERSION);
    assert_eq!(read.value, hprz);
    assert_eq!(container::write(&read.value).unwrap(), bytes);
    // Unzipped by the zip reader alone: the design's text first, then each file, all deflated and
    // dated 1980-01-01.
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
    let names: Vec<String> = archive.file_names().map(str::to_owned).collect();
    assert_eq!(names.len(), 5);
    let mut first = archive.by_index(0).unwrap();
    assert_eq!(first.name(), container::DESIGN_ENTRY);
    assert_eq!(first.compression(), zip::CompressionMethod::Deflated);
    assert_eq!(first.last_modified(), Some(zip::DateTime::DEFAULT));
    let mut text = String::new();
    std::io::Read::read_to_string(&mut first, &mut text).unwrap();
    assert_eq!(text, to_json(&hprz.design).unwrap());
    // A container of no attachments is the design alone.
    let bare = hprz_of(Vec::new());
    assert_eq!(
        container::read(&container::write(&bare).unwrap())
            .unwrap()
            .value,
        bare
    );
}

/// A container whose design is of an older version reads as the current version.
#[test]
fn a_containers_older_design_is_migrated() {
    let zip = zip_of(&[
        ("design.hpr", DOCUMENT_0_1.as_bytes()),
        ("logs/a.csv", b"t\n"),
    ]);
    let read = container::read(&zip).unwrap();
    assert_eq!(read.written_as, Version { major: 0, minor: 1 });
    assert_eq!(read.value.design, from_json(DOCUMENT_0_1).unwrap());
    assert_eq!(
        read.value.attachments,
        [container::Entry::new("logs/a.csv", b"t\n".to_vec())]
    );
}

/// A zip archive of `entries`, in order, each deflated, as another program might write one.
fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    use std::io::Write as _;
    let options = zip::write::SimpleFileOptions::DEFAULT
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::DEFAULT);
    let mut archive = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        archive.start_file(*name, options).unwrap();
        archive.write_all(bytes).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

/// Names no container holds, written and read: each refused with its reason, on both sides.
#[test]
fn an_attachment_name_that_leaves_its_folder_is_refused() {
    let design = to_json(&document(PUBLIC[6].1)).unwrap();
    for (name, why) in [
        ("", "is empty"),
        ("design.hpr", "the design's entry"),
        ("DESIGN.HPR", "the design's entry"),
        ("../up.txt", "not a relative path"),
        ("logs/../../up.txt", "not a relative path"),
        ("/etc/passwd", "not a relative path"),
        ("logs//a.csv", "not a relative path"),
        ("./a.csv", "not a relative path"),
        ("logs/", "not a relative path"),
        ("C:/a.csv", "':'"),
        ("logs\\a.csv", "'\\\\'"),
        ("a\u{0}.csv", "'\\0'"),
        ("a\n.csv", "'\\n'"),
        ("a.csv.", "ending in `.` or a space"),
        ("logs /a.csv", "ending in `.` or a space"),
        ("NUL", "a Windows device's name"),
        ("con.txt", "a Windows device's name"),
        ("logs/COM1.csv", "a Windows device's name"),
        ("Lpt9", "a Windows device's name"),
        ("COM0", "a Windows device's name"),
        ("lpt\u{b9}.txt", "a Windows device's name"),
        ("NUL .txt", "a Windows device's name"),
        ("logs/CONIN$", "a Windows device's name"),
        (&format!("{}.txt", "x".repeat(252)), "longer than 255 bytes"),
    ] {
        let written = container::write(&hprz_of(vec![container::Entry::new(name, vec![1])]));
        let Err(FormatError::Container(message)) = written else {
            panic!("{name:?} written: {written:?}");
        };
        assert!(message.contains(why), "{name:?}: {message}");
        // An empty name, a folder's, or the design's own is not an entry the zip writer adds
        // beside a design, so only the rest are read back (a second design entry: below).
        if name.is_empty() || name.ends_with('/') || name == "design.hpr" {
            continue;
        }
        let zip = zip_of(&[("design.hpr", design.as_bytes()), (name, &[1])]);
        let read = container::read(&zip);
        let Err(FormatError::Container(message)) = read else {
            panic!("{name:?} read: {read:?}");
        };
        assert!(message.contains(why), "{name:?}: {message}");
    }
    // Two of one name, even by case alone.
    for (a, b) in [("logs/a.csv", "logs/a.csv"), ("Logs/A.csv", "logs/a.csv")] {
        let hprz = hprz_of(vec![
            container::Entry::new(a, vec![1]),
            container::Entry::new(b, vec![2]),
        ]);
        let message = container::write(&hprz).unwrap_err().to_string();
        assert!(message.contains("two attachments are named"), "{message}");
    }
    let zip = zip_of(&[
        ("design.hpr", design.as_bytes()),
        ("Logs/A.csv", &[1]),
        ("logs/a.csv", &[2]),
    ]);
    let message = container::read(&zip).unwrap_err().to_string();
    assert!(message.contains("two attachments are named"), "{message}");
    // Names that only look like a device's are taken.
    let names = [
        "console.txt",
        "com10.txt",
        "nul-report.csv",
        "lpt.csv",
        "logs/aux1.csv",
    ];
    let hprz = hprz_of(
        names
            .iter()
            .map(|name| container::Entry::new(*name, vec![1]))
            .collect(),
    );
    assert_eq!(
        container::read(&container::write(&hprz).unwrap())
            .unwrap()
            .value,
        hprz
    );
    // A name that is also another's folder, even by case alone, in either order.
    for (a, b) in [("logs", "logs/a.csv"), ("logs/a.csv", "Logs")] {
        let hprz = hprz_of(vec![
            container::Entry::new(a, vec![1]),
            container::Entry::new(b, vec![2]),
        ]);
        let message = container::write(&hprz).unwrap_err().to_string();
        assert!(
            message.contains("is also the folder of another"),
            "{message}"
        );
        let zip = zip_of(&[("design.hpr", design.as_bytes()), (a, &[1]), (b, &[2])]);
        let message = container::read(&zip).unwrap_err().to_string();
        assert!(
            message.contains("is also the folder of another"),
            "{message}"
        );
    }
    // A symbolic link is refused, not read as a file holding its target.
    let mut links = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    links
        .start_file("design.hpr", zip::write::SimpleFileOptions::DEFAULT)
        .unwrap();
    std::io::Write::write_all(&mut links, design.as_bytes()).unwrap();
    links
        .add_symlink("logs/link", "/etc", zip::write::SimpleFileOptions::DEFAULT)
        .unwrap();
    let zip = links.finish().unwrap().into_inner();
    let message = container::read(&zip).unwrap_err().to_string();
    assert!(
        message.contains("\"logs/link\" is a symbolic link"),
        "{message}"
    );
    // A name not marked as UTF-8 (a byte of the old DOS code page, which the zip reader turns into
    // another character) is refused rather than renamed.
    let mut dos = zip_of(&[("design.hpr", design.as_bytes()), ("x.csv", b"1")]);
    let at: Vec<usize> = (0..dos.len() - 5)
        .filter(|&i| &dos[i..i + 5] == b"x.csv")
        .collect();
    assert_eq!(at.len(), 2);
    for i in at {
        dos[i] = 0x82;
    }
    let message = container::read(&dos).unwrap_err().to_string();
    assert!(message.contains("isn't marked as UTF-8"), "{message}");
    // A folder's entry that holds bytes would be lost, so it is refused.
    let mut full = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    full.start_file("design.hpr", zip::write::SimpleFileOptions::DEFAULT)
        .unwrap();
    std::io::Write::write_all(&mut full, design.as_bytes()).unwrap();
    full.start_file("notes/", zip::write::SimpleFileOptions::DEFAULT)
        .unwrap();
    std::io::Write::write_all(&mut full, b"lost").unwrap();
    let zip = full.finish().unwrap().into_inner();
    let message = container::read(&zip).unwrap_err().to_string();
    assert!(
        message.contains("\"notes/\" is a folder's entry that is not an empty folder"),
        "{message}"
    );
    // Names that differ only by a letter's case, where Rust's lower case alone keeps them apart:
    // the Greek final sigma, and the Kelvin sign.
    for (a, b) in [
        ("\u{3b1}\u{3c3}.txt", "\u{391}\u{3a3}.txt"),
        ("\u{3b1}\u{3c2}.txt", "\u{3b1}\u{3c3}.txt"),
        ("\u{212a}.txt", "k.txt"),
    ] {
        let hprz = hprz_of(vec![
            container::Entry::new(a, vec![1]),
            container::Entry::new(b, vec![2]),
        ]);
        let message = container::write(&hprz).unwrap_err().to_string();
        assert!(
            message.contains("two attachments are named"),
            "{a} {b}: {message}"
        );
    }
    // A name of 255 bytes is taken.
    let longest = format!("{}.txt", "x".repeat(251));
    let hprz = hprz_of(vec![container::Entry::new(longest.as_str(), vec![1])]);
    assert_eq!(
        container::read(&container::write(&hprz).unwrap())
            .unwrap()
            .value,
        hprz
    );
    // A name of 30,000 folders is checked in memory of its own size, not one copy per folder (which
    // was about a gigabyte), whether it stands alone or beside its own first folder.
    let deep = format!("{}x", "a/".repeat(30_000));
    let hprz = hprz_of(vec![container::Entry::new(deep.as_str(), vec![1])]);
    assert_eq!(
        container::read(&container::write(&hprz).unwrap())
            .unwrap()
            .value,
        hprz
    );
    let hprz = hprz_of(vec![
        container::Entry::new(deep.as_str(), vec![1]),
        container::Entry::new("A", vec![2]),
    ]);
    let message = container::write(&hprz).unwrap_err().to_string();
    assert!(
        message.contains("\"A\" is also the folder of another"),
        "{message}"
    );
    // A name that sorts between a folder and its files is not taken for one of them.
    let names = ["logs", "logs-old/a.csv", "logs.csv"];
    let hprz = hprz_of(
        names
            .iter()
            .map(|name| container::Entry::new(*name, vec![1]))
            .collect(),
    );
    assert!(container::write(&hprz).is_ok());
    // A folder's own entry holds nothing and is passed over.
    let mut folders = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    folders
        .add_directory("logs/", zip::write::SimpleFileOptions::DEFAULT)
        .unwrap();
    folders
        .start_file("design.hpr", zip::write::SimpleFileOptions::DEFAULT)
        .unwrap();
    std::io::Write::write_all(&mut folders, design.as_bytes()).unwrap();
    let zip = folders.finish().unwrap().into_inner();
    assert_eq!(container::read(&zip).unwrap().value.attachments, []);
}

/// What isn't a container, or is a damaged or hostile one, is refused with the reason.
#[test]
fn a_damaged_container_is_refused() {
    let design = to_json(&document(PUBLIC[6].1)).unwrap();
    let refused = |bytes: &[u8]| match container::read(bytes) {
        Err(error) => error.to_string(),
        Ok(read) => panic!("read {} attachments", read.value.attachments.len()),
    };
    assert!(refused(b"").contains("don't start as one"));
    assert!(refused(design.as_bytes()).contains("don't start as one"));
    assert!(refused(b"PK\x03\x04 and then nothing").starts_with("not a valid .hprz"));
    assert!(refused(&zip_of(&[("logs/a.csv", b"t\n")])).contains("holds no \"design.hpr\""));
    assert!(refused(&zip_of(&[("design.hpr", &[0xff, 0xfe])])).contains("not UTF-8"));
    // The design is held to everything a `.hpr` is.
    let other = design.replacen("\"version\": \"0.2\"", "\"version\": \"0.3\"", 1);
    assert!(refused(&zip_of(&[("design.hpr", other.as_bytes())])).contains("newer program"));
    // Two entries of one name, which the zip writer won't make, so the second is renamed in the
    // bytes: the zip reader would keep one and drop the other, so the archive is refused. For a
    // second design entry and for an attachment's.
    for (keep, rename) in [("design.hpr", "design.hpx"), ("logs/a.csv", "logs/a.csx")] {
        let mut two = zip_of(&[
            ("design.hpr", design.as_bytes()),
            ("logs/a.csv", b"1\n"),
            (rename, b"2\n"),
        ]);
        let at: Vec<usize> = (0..two.len() - rename.len())
            .filter(|&i| &two[i..i + rename.len()] == rename.as_bytes())
            .collect();
        assert_eq!(
            at.len(),
            2,
            "its local header and its central directory record"
        );
        for i in at {
            two[i..i + rename.len()].copy_from_slice(keep.as_bytes());
        }
        assert_eq!(
            refused(&two),
            "not a valid .hprz: its central directory holds 3 entries, and the zip reader keeps \
             2: two have the same name, and one would be lost, or the directory is damaged"
        );
    }
    // A megabyte of zeros deflates to about a kilobyte; with a smaller budget it is refused
    // rather than read, whichever entry passes it.
    let zeros = vec![0u8; 1 << 20];
    let bomb = zip_of(&[("design.hpr", design.as_bytes()), ("zeros.bin", &zeros)]);
    assert!(bomb.len() < 20_000, "{}", bomb.len());
    let budget = design.len() as u64 + (1 << 20) - 1;
    let message = container::read_within(&bomb, budget)
        .unwrap_err()
        .to_string();
    assert!(message.contains("more than"), "{message}");
    assert!(container::read_within(&bomb, budget + 1).is_ok());
    let message = container::read_within(&bomb, 10).unwrap_err().to_string();
    assert!(message.contains("more than 10 bytes"), "{message}");
    // A name is refused before anything is decompressed: with the budget spent on the design,
    // the name's reason still comes first.
    let named = zip_of(&[("design.hpr", design.as_bytes()), ("../zeros.bin", &zeros)]);
    let message = container::read_within(&named, 10).unwrap_err().to_string();
    assert!(message.contains("not a relative path"), "{message}");
    // Nor does a container hold more than it can be read back with: the writer refuses one past
    // the limit before compressing anything.
    let text = to_json(&hprz_of(Vec::new()).design).unwrap();
    let over = container::MAX_UNPACKED_BYTES as usize - text.len() + 1;
    let hprz = hprz_of(vec![container::Entry::new("big.bin", vec![0; over])]);
    let message = container::write(&hprz).unwrap_err().to_string();
    assert!(
        message.contains(&format!(
            "it would hold {} bytes unpacked",
            container::MAX_UNPACKED_BYTES + 1
        )),
        "{message}"
    );
}

/// A design the format refuses to write is refused in a container too.
#[test]
fn a_container_of_a_design_that_cannot_be_written_is_refused() {
    let mut hprz = hprz_of(Vec::new());
    hprz.design.motors.configurations[0].motors[0].diameter_m = Some(f64::NAN);
    assert!(matches!(
        container::write(&hprz),
        Err(FormatError::NotRepresentable(_))
    ));
}

/// A public design's XML with a parallel stage hpr doesn't read put inside its first body tube, so
/// its airframe is not read exactly as written, as raw XML, which the `.ork` reader takes.
fn with_a_parallel_stage() -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(PUBLIC[6].1)).unwrap();
    let mut xml = String::new();
    std::io::Read::read_to_string(&mut archive.by_name("rocket.ork").unwrap(), &mut xml).unwrap();
    let at = xml.find("<bodytube>").unwrap();
    let inside = xml[at..].find("<subcomponents>").unwrap() + at + "<subcomponents>".len();
    format!(
        "{}<parallelstage><name>Boosters</name><id>boosters</id>\
         <instancecount>2</instancecount></parallelstage>{}",
        &xml[..inside],
        &xml[inside..]
    )
    .into_bytes()
}

/// The provenance says why a `.ork`'s airframe was not read exactly as written, as the `.ork`
/// reader does, and says nothing when it was.
#[test]
fn the_provenance_says_when_the_airframe_was_not_read_as_written() {
    let airframe = |document: &DesignFile| {
        document
            .provenance
            .source
            .as_ref()
            .unwrap()
            .airframe_not_as_written
            .clone()
    };
    for (name, bytes) in PUBLIC {
        let file = ork::read(bytes).unwrap().value;
        assert_eq!(
            airframe(&document(bytes)),
            ork::airframe_not_as_written(&file),
            "{name}"
        );
    }
    let reduced = with_a_parallel_stage();
    let read = document(&reduced);
    let why = airframe(&read).unwrap();
    assert!(why.starts_with("1 `parallelstage` were left out"), "{why}");
    let text = to_json(&read).unwrap();
    assert!(text.contains("\"airframe_not_as_written\": \"1 `parallelstage`"));
    assert_eq!(schema_errors(&validator(), &text), Vec::<String>::new());
    assert_eq!(from_json(&text).unwrap(), read);
}

/// 0.1 didn't record the airframe's reason, so the migration takes it from the first
/// configuration left out for it, when the source is a `.ork`; and refuses a 0.1 document that
/// already has one.
#[test]
fn the_migration_recovers_the_airframes_reason_from_the_configurations() {
    let reduced = document(&with_a_parallel_stage());
    let why = reduced
        .provenance
        .source
        .clone()
        .unwrap()
        .airframe_not_as_written
        .unwrap();
    // Its one configuration is left out first for want of a curve, so as 0.1 held it, nothing
    // says whether the airframe was read as written: the migration says it can't be known, which
    // `hpr sim` refuses another motor on, as it does the `.ork`.
    let left_out = reduced.motors.configurations[0].left_out.as_ref().unwrap();
    assert_eq!(left_out.why, ork::NotFlown::NoCurve);
    let as_0_1 = |document: &DesignFile| {
        let mut value = serde_json::to_value(document).unwrap();
        let object = value.as_object_mut().unwrap();
        object.insert("version".to_owned(), serde_json::json!("0.1"));
        let files = object.remove("source_files").unwrap();
        object.insert("attachments".to_owned(), files);
        object["provenance"]["source"]
            .as_object_mut()
            .unwrap()
            .remove("airframe_not_as_written");
        value
    };
    let migrated_reason = |value: &serde_json::Value| {
        from_json(&value.to_string())
            .unwrap()
            .provenance
            .source
            .unwrap()
            .airframe_not_as_written
    };
    let old = as_0_1(&reduced);
    let errors: Vec<String> = jsonschema::validator_for(&serde_json::from_str(SCHEMA_0_1).unwrap())
        .unwrap()
        .iter_errors(&old)
        .map(|error| error.to_string())
        .collect();
    assert_eq!(errors, Vec::<String>::new());
    let migrated = from_json(&old.to_string()).unwrap();
    let mut expected = reduced.clone();
    expected
        .provenance
        .source
        .as_mut()
        .unwrap()
        .airframe_not_as_written = Some(migrate::UNKNOWN.to_owned());
    assert_eq!(migrated, expected);
    // With no configuration at all, the same.
    let mut value = old.clone();
    value["motors"]["configurations"] = serde_json::json!([]);
    assert_eq!(migrated_reason(&value).as_deref(), Some(migrate::UNKNOWN));
    // Had it a curve, the configuration would be left out for the airframe: the reason is taken
    // from there.
    let mut value = old.clone();
    value["motors"]["configurations"][0]["left_out"] = serde_json::json!({
        "why": "airframe_not_as_written",
        "message": format!("the airframe was not read exactly as written: {why}"),
    });
    assert_eq!(migrated_reason(&value).as_deref(), Some(why.as_str()));
    // A configuration that flies, or is left out only for its separation, which the reader asks
    // after the airframe, shows the airframe was read as written.
    for left_out in [
        serde_json::Value::Null,
        serde_json::json!({ "why": "separation_not_flown", "message": "x" }),
    ] {
        let mut value = old.clone();
        value["motors"]["configurations"][0]["left_out"] = left_out.clone();
        assert_eq!(migrated_reason(&value), None, "{left_out}");
    }
    // Nothing is recorded for a source that isn't a `.ork`.
    let mut value = old.clone();
    value["provenance"]["source"]["format"] = serde_json::json!("hpr_design");
    assert_eq!(migrated_reason(&value), None);
    // A 0.1 document that has one is not a 0.1 document.
    let mut value = as_0_1(&reduced);
    value["provenance"]["source"]["airframe_not_as_written"] = serde_json::json!("x");
    assert_eq!(
        from_json(&value.to_string()).unwrap_err().to_string(),
        "not a valid hpr design 0.2: as a 0.1 document, its source has an \
         \"airframe_not_as_written\", which 0.1 doesn't define"
    );
}

/// On every public design, the migration from 0.1 works out whether the airframe was read as
/// written as the `.ork` reader says, or marks it unknown, and is never wrong: as `cargo xtask ork`
/// holds the private designs to. The reduced design adds one whose airframe wasn't, and the
/// archive with an embedded curve one whose configuration flies.
#[test]
fn the_migration_is_never_wrong_about_a_public_designs_airframe() {
    // Found the same, marked unknown though read as written, marked unknown though not.
    let mut counts = [0usize; 3];
    let reduced = with_a_parallel_stage();
    let (embedded, _, _) = with_attachments();
    let designs = PUBLIC.iter().map(|(name, bytes)| (*name, *bytes)).chain([
        ("reduced", reduced.as_slice()),
        ("embedded", embedded.as_slice()),
    ]);
    for (name, bytes) in designs {
        let document = document(bytes);
        let truth = ork::airframe_not_as_written(&ork::read(bytes).unwrap().value);
        let mut value = serde_json::to_value(&document).unwrap();
        let object = value.as_object_mut().unwrap();
        object.insert("version".to_owned(), serde_json::json!("0.1"));
        let files = object.remove("source_files").unwrap();
        object.insert("attachments".to_owned(), files);
        object["provenance"]["source"]
            .as_object_mut()
            .unwrap()
            .remove("airframe_not_as_written");
        let migrated = from_json(&value.to_string()).unwrap();
        let reason = migrated.provenance.source.unwrap().airframe_not_as_written;
        match (truth.as_deref(), reason.as_deref()) {
            (truth, reason) if truth == reason => counts[0] += 1,
            (_, Some(migrate::UNKNOWN)) => counts[usize::from(truth.is_some()) + 1] += 1,
            (truth, reason) => panic!("{name}: {truth:?}, migrated as {reason:?}"),
        }
    }
    // With no motor catalog here, a public design's configurations are all left out for want of a
    // curve, before the airframe is asked after, so only the embedded curve's design shows it.
    assert_eq!(counts, [1, PUBLIC.len() - 1, 2]);
}
