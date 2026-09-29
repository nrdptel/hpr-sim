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
const COMMITTED_SCHEMA: &str = include_str!("../../../schema/format/hpr-design-0.1.schema.json");

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
        "schema/format/hpr-design-0.1.schema.json is stale: run `cargo xtask format`"
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
        "{\n  \"format\": \"hpr-design\",\n  \"version\": \"0.1\",\n  \"provenance\": {\n    \
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
            "attachments"
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
        read.attachments,
        [
            AttachedFile {
                name: "decals/invented.png".to_owned(),
                content: Content::Base64("iVBORw0KGgoA//6A".to_owned()),
            },
            AttachedFile {
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

/// What the attachments must hold to, beyond their types: base64 that decodes, one name each, and
/// every embedded curve's entry.
#[test]
fn attachments_that_do_not_hold_together_are_refused() {
    let (archive, entry, _) = with_attachments();
    let text = to_json(&document(&archive)).unwrap();
    let swap = |from: &str, to: &str| {
        assert!(text.contains(from), "{from}");
        from_json(&text.replacen(from, to, 1))
            .unwrap_err()
            .to_string()
    };
    assert!(swap("iVBORw0KGgoA//6A", "iVBORw0KGgoA//6").contains("decals/invented.png"));
    assert!(swap("decals/invented.png", &entry).contains("two attachments are named"));
    for name in ["rocket.ork", "decals/", "decals\\", ""] {
        let refused = swap("\"decals/invented.png\"", &format!("{name:?}"));
        assert!(refused.contains("which a .ork can't hold"), "{refused}");
    }
    assert!(
        swap(
            &format!("\"name\": \"{entry}\""),
            "\"name\": \"elsewhere.rse\""
        )
        .contains("which the attachments don't hold")
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
    let newer = swap("\"version\": \"0.1\"", "\"version\": \"0.2\"");
    assert_eq!(
        newer,
        FormatError::Unsupported {
            found: Version { major: 0, minor: 2 },
            supported: VERSION
        }
    );
    assert!(newer.to_string().ends_with("written by a newer program"));
    let older = swap("\"version\": \"0.1\"", "\"version\": \"0.0\"");
    assert!(!older.to_string().contains("newer"));
    for bad in ["01.1", "0.01", "1", "0.1.0", "+0.1", " 0.1", "0.x", ""] {
        let refused = swap("\"version\": \"0.1\"", &format!("\"version\": \"{bad}\""));
        assert!(
            matches!(refused, FormatError::Invalid(_)),
            "{bad:?}: {refused}"
        );
    }
    assert!(matches!(
        swap("\"version\": \"0.1\"", "\"version\": 0.1"),
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
    read.version = Version { major: 0, minor: 2 };
    assert!(matches!(
        to_json(&read),
        Err(FormatError::Unsupported { found, .. }) if found == read.version
    ));
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

/// A smoke test of many attachments: a forged document with 30,000 reads back whole.
#[test]
fn many_attachments_are_read() {
    let mut read = document(PUBLIC[0].1);
    read.attachments = (0..30_000)
        .map(|index| AttachedFile {
            name: format!("decals/{index}.png"),
            content: Content::Base64("AA==".to_owned()),
        })
        .collect();
    let text = to_json(&read).unwrap();
    assert_eq!(from_json(&text).unwrap().attachments.len(), 30_000);
}
