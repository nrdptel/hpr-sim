//! The bundled `.orc` catalogue against OpenRocket 24.12's own reading of it.
//!
//! `tests/fixtures/orc/openrocket-presets.json` is every part OpenRocket's preset loader returns
//! for each bundled file, every value it holds (`validation/oracles/openrocket/orc_presets.py`).
//! hpr must read the same parts in the same order, with every value equal to the bit, except for
//! the three departures `hpr_io::orc`'s documentation names, each counted here and each shown to
//! have the cause it is given: OpenRocket's ounce, its display names for two makers, and the
//! density it derives from a solid part's stated mass. Three materials the files name but don't
//! define are `None` here and zero there.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::collections::BTreeMap;
use std::f64::consts::PI;

use hpr_io::orc::{BUNDLED_FILES, MaterialKind, MaterialRef, Part, PartKind, Shape, read};
use serde_json::Value;

/// OpenRocket's ounce, kg, as its readings show it: 0.0283495231, against the exact
/// 0.028349523125.
const OPENROCKET_OUNCE_KG: f64 = 0.028_349_523_1;
const OUNCE_KG: f64 = 0.028_349_523_125;

#[derive(Default, Debug)]
struct Departures {
    /// Masses that differ by OpenRocket's ounce alone.
    ounce_masses: usize,
    /// Parts whose maker OpenRocket names otherwise: (the file's, OpenRocket's) → parts.
    makers: BTreeMap<(String, String), usize>,
    /// Bulk parts stating a mass whose density OpenRocket replaced.
    derived_densities: usize,
    /// Of those, the tube-like parts and bulkheads whose replaced density times the part's volume
    /// was checked to give the stated mass.
    derived_checked: usize,
    /// Materials named but not defined: `None` here, zero in OpenRocket.
    undefined: usize,
    /// Values compared and equal.
    equal: usize,
}

fn number(record: &Value, key: &str) -> f64 {
    record[key]
        .as_f64()
        .unwrap_or_else(|| panic!("OpenRocket's record has a number `{key}`: {record}"))
}

fn same(departures: &mut Departures, record: &Value, key: &str, ours: f64, at: &str) {
    assert_eq!(ours, number(record, key), "{at}: {key}");
    departures.equal += 1;
}

fn shape(record: &Value, ours: Shape, at: &str) {
    let name = match ours {
        Shape::Conical => "CONICAL",
        Shape::Ogive => "OGIVE",
        Shape::Ellipsoid => "ELLIPSOID",
        Shape::Parabolic => "PARABOLIC",
        Shape::Haack => "HAACK",
        Shape::Power => "POWER",
    };
    assert_eq!(record["Shape"], name, "{at}");
}

fn filled(record: &Value, ours: Option<bool>, at: &str) {
    assert_eq!(record.get("Filled").and_then(Value::as_bool), ours, "{at}");
}

fn thickness(departures: &mut Departures, record: &Value, ours: Option<f64>, at: &str) {
    match ours {
        Some(value) => same(departures, record, "Thickness", value, at),
        None => assert!(record.get("Thickness").is_none(), "{at}"),
    }
}

/// The material against OpenRocket's; `volume_m3` is the part's solid volume where it is simple
/// enough to check a derived density with.
fn material(
    departures: &mut Departures,
    record: &Value,
    key: &str,
    ours: &MaterialRef,
    part: &Part,
    volume_m3: Option<f64>,
    at: &str,
) {
    let theirs = &record[key];
    assert_eq!(theirs["name"], ours.name.as_str(), "{at}: {key}");
    let kind = match ours.kind {
        MaterialKind::Bulk => "BULK",
        MaterialKind::Surface => "SURFACE",
        MaterialKind::Line => "LINE",
    };
    assert_eq!(theirs["kind"], kind, "{at}: {key}");
    let density = number(theirs, "density");
    match ours.density {
        None => {
            assert_eq!(density, 0.0, "{at}: {key}");
            departures.undefined += 1;
        }
        Some(value) if ours.kind == MaterialKind::Bulk && part.mass_kg.is_some() => {
            // OpenRocket gives the part its stated mass by changing the density.
            assert_ne!(value, density, "{at}: {key}");
            departures.derived_densities += 1;
            if let Some(volume) = volume_m3 {
                // Against OpenRocket's own reading of the mass, in its ounce where stated in one.
                let theirs_kg = density * volume;
                let stated = number(record, "Mass");
                assert!(
                    ((theirs_kg - stated) / stated).abs() < 1e-12,
                    "{at}: {theirs_kg} kg against {stated} kg"
                );
                departures.derived_checked += 1;
            }
        }
        Some(value) => {
            assert_eq!(value, density, "{at}: {key}");
            departures.equal += 1;
        }
    }
}

#[test]
fn every_part_reads_as_openrocket_reads_it() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/orc/openrocket-presets.json"))
        .expect("the fixture is JSON");
    assert_eq!(fixture["openrocket"], "24.12");
    let files = fixture["files"].as_array().expect("files");
    let names: Vec<_> = files.iter().map(|file| file["file"].as_str()).collect();
    let bundled: Vec<_> = BUNDLED_FILES.iter().map(|(name, _)| Some(*name)).collect();
    assert_eq!(
        names, bundled,
        "the fixture covers every bundled file, in order"
    );

    let mut departures = Departures::default();
    let mut parts = 0;
    for ((name, text), file) in BUNDLED_FILES.iter().zip(files) {
        let read = read(text, name).expect("every bundled file reads");
        let records = file["parts"].as_array().expect("parts");
        assert_eq!(read.catalog.parts.len(), records.len(), "{name}");
        for (part, record) in read.catalog.parts.iter().zip(records) {
            let at = format!("{name}: {} {}", part.manufacturer, part.part_number);
            compare(&mut departures, part, record, &at);
            parts += 1;
        }
    }
    assert_eq!(parts, 3449);
    println!("{departures:#?}");
    assert_eq!(departures.ounce_masses, 185);
    assert_eq!(
        departures.makers,
        BTreeMap::from([
            (
                ("LOC Precision".to_owned(), "LOC/Precision".to_owned()),
                113
            ),
            (
                (
                    "Public Missiles".to_owned(),
                    "Public Missiles, Ltd.".to_owned()
                ),
                139
            ),
        ])
    );
    assert_eq!(departures.derived_densities, 207);
    assert_eq!(departures.derived_checked, 11);
    assert_eq!(departures.undefined, 3);
}

fn compare(departures: &mut Departures, part: &Part, record: &Value, at: &str) {
    // Every key OpenRocket holds is one this compares.
    let compared = [
        "Type",
        "Legacy",
        "Manufacturer",
        "PartNo",
        "Description",
        "Mass",
        "Material",
        "LineMaterial",
        "OuterDiameter",
        "InnerDiameter",
        "Length",
        "Thickness",
        "Filled",
        "Shape",
        "AftOuterDiameter",
        "AftShoulderDiameter",
        "AftShoulderLength",
        "ForeOuterDiameter",
        "ForeShoulderDiameter",
        "ForeShoulderLength",
        "Diameter",
        "Sides",
        "LineCount",
        "LineLength",
        "Width",
    ];
    for key in record.as_object().expect("a part is an object").keys() {
        assert!(
            compared.contains(&key.as_str()),
            "{at}: OpenRocket's `{key}`"
        );
    }
    assert_eq!(record["Legacy"], false, "{at}");
    assert_eq!(record["PartNo"], part.part_number.as_str(), "{at}");
    assert_eq!(record["Description"], part.description.as_str(), "{at}");
    let maker = record["Manufacturer"].as_str().expect("a maker");
    if maker != part.manufacturer {
        *departures
            .makers
            .entry((part.manufacturer.clone(), maker.to_owned()))
            .or_default() += 1;
    }
    match (part.mass_kg, record.get("Mass").and_then(Value::as_f64)) {
        (None, None) => {}
        (Some(ours), Some(theirs)) if ours == theirs => departures.equal += 1,
        (Some(ours), Some(theirs)) => {
            // Stated in ounces: the same number of OpenRocket's ounces.
            let ounces = ours / OUNCE_KG;
            assert!(
                ((ounces * OPENROCKET_OUNCE_KG - theirs) / theirs).abs() < 1e-15,
                "{at}: {ours} kg against {theirs} kg"
            );
            departures.ounce_masses += 1;
        }
        (ours, theirs) => panic!("{at}: mass {ours:?} against {theirs:?}"),
    }
    let tube_volume =
        |outer: f64, inner: f64, length: f64| PI / 4.0 * (outer * outer - inner * inner) * length;
    match &part.kind {
        PartKind::BodyTube(tube)
        | PartKind::TubeCoupler(tube)
        | PartKind::EngineBlock(tube)
        | PartKind::CenteringRing(tube)
        | PartKind::LaunchLug(tube) => {
            let kind = match &part.kind {
                PartKind::BodyTube(_) => "BODY_TUBE",
                PartKind::TubeCoupler(_) => "TUBE_COUPLER",
                PartKind::EngineBlock(_) => "ENGINE_BLOCK",
                PartKind::CenteringRing(_) => "CENTERING_RING",
                _ => "LAUNCH_LUG",
            };
            assert_eq!(record["Type"], kind, "{at}");
            same(
                departures,
                record,
                "OuterDiameter",
                tube.outer_diameter_m,
                at,
            );
            same(
                departures,
                record,
                "InnerDiameter",
                tube.inner_diameter_m,
                at,
            );
            same(departures, record, "Length", tube.length_m, at);
            same(departures, record, "Thickness", tube.thickness_m(), at);
            let volume = tube_volume(tube.outer_diameter_m, tube.inner_diameter_m, tube.length_m);
            material(
                departures,
                record,
                "Material",
                &tube.material,
                part,
                Some(volume),
                at,
            );
        }
        PartKind::Bulkhead(bulkhead) => {
            assert_eq!(record["Type"], "BULK_HEAD", "{at}");
            same(
                departures,
                record,
                "OuterDiameter",
                bulkhead.outer_diameter_m,
                at,
            );
            same(departures, record, "Length", bulkhead.length_m, at);
            filled(record, bulkhead.filled, at);
            let volume = tube_volume(bulkhead.outer_diameter_m, 0.0, bulkhead.length_m);
            material(
                departures,
                record,
                "Material",
                &bulkhead.material,
                part,
                Some(volume),
                at,
            );
        }
        PartKind::NoseCone(nose) => {
            assert_eq!(record["Type"], "NOSE_CONE", "{at}");
            shape(record, nose.shape, at);
            same(departures, record, "Length", nose.length_m, at);
            same(
                departures,
                record,
                "AftOuterDiameter",
                nose.outer_diameter_m,
                at,
            );
            same(
                departures,
                record,
                "AftShoulderDiameter",
                nose.shoulder_diameter_m,
                at,
            );
            same(
                departures,
                record,
                "AftShoulderLength",
                nose.shoulder_length_m,
                at,
            );
            filled(record, nose.filled, at);
            thickness(departures, record, nose.thickness_m, at);
            material(
                departures,
                record,
                "Material",
                &nose.material,
                part,
                None,
                at,
            );
        }
        PartKind::Transition(transition) => {
            assert_eq!(record["Type"], "TRANSITION", "{at}");
            shape(record, transition.shape, at);
            same(departures, record, "Length", transition.length_m, at);
            same(
                departures,
                record,
                "ForeOuterDiameter",
                transition.fore_outer_diameter_m,
                at,
            );
            same(
                departures,
                record,
                "ForeShoulderDiameter",
                transition.fore_shoulder_diameter_m,
                at,
            );
            same(
                departures,
                record,
                "ForeShoulderLength",
                transition.fore_shoulder_length_m,
                at,
            );
            same(
                departures,
                record,
                "AftOuterDiameter",
                transition.aft_outer_diameter_m,
                at,
            );
            same(
                departures,
                record,
                "AftShoulderDiameter",
                transition.aft_shoulder_diameter_m,
                at,
            );
            same(
                departures,
                record,
                "AftShoulderLength",
                transition.aft_shoulder_length_m,
                at,
            );
            filled(record, transition.filled, at);
            thickness(departures, record, transition.thickness_m, at);
            material(
                departures,
                record,
                "Material",
                &transition.material,
                part,
                None,
                at,
            );
        }
        PartKind::Parachute(parachute) => {
            assert_eq!(record["Type"], "PARACHUTE", "{at}");
            same(departures, record, "Diameter", parachute.diameter_m, at);
            same(
                departures,
                record,
                "LineLength",
                parachute.line_length_m,
                at,
            );
            assert_eq!(record["Sides"], parachute.sides, "{at}");
            assert_eq!(record["LineCount"], parachute.line_count, "{at}");
            material(
                departures,
                record,
                "Material",
                &parachute.material,
                part,
                None,
                at,
            );
            match &parachute.line_material {
                Some(line) => {
                    material(departures, record, "LineMaterial", line, part, None, at);
                }
                None => assert!(record.get("LineMaterial").is_none(), "{at}"),
            }
        }
        PartKind::Streamer(streamer) => {
            assert_eq!(record["Type"], "STREAMER", "{at}");
            same(departures, record, "Length", streamer.length_m, at);
            same(departures, record, "Width", streamer.width_m, at);
            same(departures, record, "Thickness", streamer.thickness_m, at);
            material(
                departures,
                record,
                "Material",
                &streamer.material,
                part,
                None,
                at,
            );
        }
    }
}

/// What hpr does with a probe, against OpenRocket's reading of it.
enum Probe {
    /// The same parts and values, to the bit.
    Same,
    /// The same, but for one value in a unit whose factor OpenRocket rounds: hpr's is the exact
    /// definition, within 1e-8 of OpenRocket's and not equal to it.
    Rounded(&'static str),
    /// OpenRocket reads the one part (as inches, or with a density of zero); hpr leaves it out
    /// with a warning.
    LeftOut,
    /// OpenRocket refuses the whole file; hpr leaves the one part out with a warning.
    Refused,
    /// OpenRocket refuses the whole file over a material's unit; hpr leaves the material out and
    /// reads the part with no density for it, warning of both.
    MaterialRefused,
}

#[test]
fn probes_read_as_openrocket_reads_them() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/orc/openrocket-presets.json"))
        .expect("the fixture is JSON");
    let expected = [
        ("bulk kg/m3", Probe::Same),
        ("bulk g/cm3", Probe::Same),
        ("bulk kg/dm3", Probe::Same),
        ("bulk lb/ft3", Probe::Rounded("Material")),
        ("surface kg/m2", Probe::Same),
        ("surface g/m2", Probe::Same),
        ("surface g/cm2", Probe::Same),
        ("surface oz/in2", Probe::Rounded("Material")),
        ("surface oz/ft2", Probe::Rounded("Material")),
        ("surface lb/ft2", Probe::Rounded("Material")),
        ("line kg/m", Probe::Same),
        ("line g/m", Probe::Same),
        ("line g/cm", Probe::Same),
        ("line oz/ft", Probe::Rounded("LineMaterial")),
        ("line oz/in", Probe::MaterialRefused),
        ("length m", Probe::Same),
        ("length cm", Probe::Same),
        ("length mm", Probe::Same),
        ("length in", Probe::Same),
        ("length ft", Probe::Same),
        ("length in/64", Probe::LeftOut),
        ("length furlong", Probe::Refused),
        ("mass kg", Probe::Same),
        ("mass g", Probe::Same),
        ("mass oz", Probe::Rounded("Mass")),
        ("mass lb", Probe::Same),
        ("no units", Probe::Same),
        ("wrong kind", Probe::LeftOut),
        ("unknown field", Probe::Same),
        ("unknown part", Probe::Same),
        ("no length", Probe::Refused),
        ("unknown shape", Probe::Refused),
    ];
    let probes = fixture["probes"].as_array().expect("probes");
    let names: Vec<_> = probes.iter().map(|probe| probe["probe"].as_str()).collect();
    let listed: Vec<_> = expected.iter().map(|(name, _)| Some(*name)).collect();
    assert_eq!(names, listed, "every probe has an expectation");
    let tally = |want: fn(&Probe) -> bool| expected.iter().filter(|(_, probe)| want(probe)).count();
    // The counts the guide's format page and ADR-132 give.
    assert_eq!(
        [
            tally(|probe| matches!(probe, Probe::Same)),
            tally(|probe| matches!(probe, Probe::Rounded(_))),
            tally(|probe| matches!(probe, Probe::Refused | Probe::MaterialRefused)),
            tally(|probe| matches!(probe, Probe::LeftOut)),
        ],
        [20, 6, 4, 2]
    );
    for (probe, (name, expectation)) in probes.iter().zip(&expected) {
        let text = probe["text"].as_str().expect("a probe's text");
        let read = read(text, name).expect("a probe reads");
        let theirs = probe["parts"].as_array();
        match expectation {
            Probe::Same => {
                let theirs = theirs.unwrap_or_else(|| panic!("{name}: OpenRocket refused it"));
                assert_eq!(read.catalog.parts.len(), theirs.len(), "{name}");
                let mut departures = Departures::default();
                for (part, record) in read.catalog.parts.iter().zip(theirs) {
                    compare(&mut departures, part, record, name);
                }
                assert_eq!(
                    (
                        departures.ounce_masses,
                        departures.derived_densities,
                        departures.undefined
                    ),
                    (0, 0, 0),
                    "{name}"
                );
                assert!(departures.makers.is_empty(), "{name}");
            }
            Probe::Rounded(key) => {
                let theirs = theirs.unwrap_or_else(|| panic!("{name}: OpenRocket refused it"));
                assert_eq!((read.catalog.parts.len(), theirs.len()), (1, 1), "{name}");
                let part = &read.catalog.parts[0];
                let theirs_value = if *key == "Mass" {
                    number(&theirs[0], "Mass")
                } else {
                    number(&theirs[0][*key], "density")
                };
                let ours = match (*key, &part.kind) {
                    ("Mass", _) => part.mass_kg,
                    ("Material", PartKind::BodyTube(tube)) => tube.material.density,
                    ("Material", PartKind::Parachute(chute)) => chute.material.density,
                    ("LineMaterial", PartKind::Parachute(chute)) => {
                        chute.line_material.as_ref().and_then(|line| line.density)
                    }
                    _ => None,
                }
                .unwrap_or_else(|| panic!("{name}: a value"));
                assert_ne!(ours, theirs_value, "{name}");
                assert!(((ours - theirs_value) / ours).abs() < 1e-8, "{name}");
            }
            Probe::MaterialRefused => {
                assert!(theirs.is_none(), "{name}: {probe}");
                let messages: Vec<_> = read.warnings.iter().map(|w| w.message.as_str()).collect();
                assert_eq!(
                    messages,
                    [
                        "`oz/in` is not a unit of line density the format has; it was left out",
                        "its material `L` is not defined in this file, so it has no density",
                    ],
                    "{name}"
                );
                let [part] = read.catalog.parts.as_slice() else {
                    panic!("{name}: one part")
                };
                let PartKind::Parachute(chute) = &part.kind else {
                    panic!("{name}: a parachute")
                };
                assert_eq!(
                    chute.line_material.as_ref().map(|line| line.density),
                    Some(None)
                );
            }
            Probe::LeftOut | Probe::Refused => {
                match expectation {
                    Probe::Refused => assert!(theirs.is_none(), "{name}: {probe}"),
                    _ => assert_eq!(theirs.map(Vec::len), Some(1), "{name}"),
                }
                assert!(read.catalog.parts.is_empty(), "{name}");
                assert_eq!(read.warnings.len(), 1, "{name}: {:?}", read.warnings);
                assert!(
                    read.warnings[0].message.ends_with("the part was left out"),
                    "{name}: {:?}",
                    read.warnings
                );
            }
        }
    }
}
