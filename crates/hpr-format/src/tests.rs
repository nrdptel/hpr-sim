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
        // `.ork` → document → `.ork` writes the `.ork` the design itself writes, and that `.ork`
        // reads back as the design first read.
        let direct = ork::export::write(&read.design(), &read.attachments())
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
            "extensions"
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
    let mut flown = 0;
    for (name, bytes) in PUBLIC {
        let design = flyable(bytes);
        let provenance = Provenance::hpr(Some(Source::of(SourceFormat::Ork, bytes)));
        let text = to_json(&DesignFile::new(design.clone(), provenance)).unwrap();
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

/// Loft lesson L57's trap, one step on: a `.ork` that embeds its motor's curve, taken through the
/// document and written back out, still carries the curve, and flies on it with no curve supplied.
#[test]
fn an_embedded_curve_travels_with_the_document() {
    let (_, bytes) = PUBLIC[13];
    let design = ork::design(&ork::read(bytes).unwrap().value).value;
    let motor = &design.motors.configurations[0].motors[0];
    let entry = format!("thrustcurves/{}.rse", motor.digest.clone().unwrap());
    let rse = INVENTED_RSE
        .replace("{DIA}", &format!("{}", motor.diameter_m.unwrap() * 1e3))
        .replace("{LEN}", &format!("{}", motor.length_m.unwrap() * 1e3));
    let attachment = Attachment {
        name: entry.clone(),
        bytes: rse.clone().into_bytes(),
    };
    let archive = ork::export::write(&design, &[attachment]).unwrap().value;
    let read = document(&archive);
    let Curve::Embedded { text, .. } = &read.motors.configurations[0].motors[0].curve else {
        panic!("{:?}", read.motors.configurations[0].motors[0].curve);
    };
    assert_eq!(*text, rse);
    assert_eq!(read.attachments().len(), 1);
    let back = from_json(&to_json(&read).unwrap()).unwrap();
    let written = back.to_ork().unwrap().value;
    let file = ork::read(&written).unwrap().value;
    assert_eq!(file.attachment(&entry).unwrap().bytes, rse.as_bytes());
    let again = ork::design(&file).value;
    assert_eq!(again, read.design());
    let before = apogees(&read.design());
    assert_eq!(before.len(), 1);
    assert_eq!(apogees(&again), before);
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
