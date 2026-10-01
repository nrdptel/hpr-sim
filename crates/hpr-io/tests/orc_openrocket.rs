//! The bundled `.orc` catalogue against OpenRocket 24.12's own reading of it.
//!
//! `tests/fixtures/orc/openrocket-presets.json` is every part OpenRocket's preset loader returns
//! for each bundled file, every value it holds (`validation/oracles/openrocket/orc_presets.py`),
//! and each part's material densities as OpenRocket reads the file with every stated mass taken
//! out. hpr must read the same parts in the same order, with every value equal to the bit, except
//! for the departures `hpr_io::orc`'s documentation names, each counted here and each shown to
//! have its cause: OpenRocket's ounce (the same count of OpenRocket's ounces), its display names
//! for two makers, the density it derives from a solid part's stated mass (hpr's equals
//! OpenRocket's once the mass is taken out, on all 207; on the 54 parts of a simple solid, the
//! derived density times the volume is the mass), and three materials the files name but don't
//! define (`None` here, zero there).

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::collections::BTreeMap;
use std::f64::consts::PI;

use hpr_io::orc::{BUNDLED_FILES, MaterialKind, MaterialRef, Part, PartKind, Shape, read};
use serde_json::Value;
use sha2::Digest;

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
    /// Of those, the parts of a simple solid (tube-like parts, bulkheads, filled conical nose
    /// cones and transitions) whose replaced density times the part's volume is the stated mass.
    derived_checked: usize,
    /// Materials named but not defined: `None` here, zero in OpenRocket.
    undefined: usize,
    /// Numbers compared and equal to the bit.
    equal: usize,
    /// Defined materials whose density equals OpenRocket's once stated masses are taken out.
    unweighed_equal: usize,
}

impl Departures {
    /// Every number compared: equal, or a counted departure.
    fn numbers(&self) -> usize {
        self.equal + self.ounce_masses + self.derived_densities + self.undefined
    }
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
        _ => panic!("{at}: a shape this test doesn't know"),
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

/// The material named under `key` against OpenRocket's. `index` is its place in the record's
/// `DensitiesWithoutMass`; `volume_m3` is the part's volume where it is a simple solid.
#[allow(
    clippy::too_many_arguments,
    reason = "one call per material, each argument a different fact about it"
)]
fn material(
    departures: &mut Departures,
    record: &Value,
    key: &str,
    index: usize,
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
        _ => panic!("{at}: a kind this test doesn't know"),
    };
    assert_eq!(theirs["kind"], kind, "{at}: {key}");
    let density = number(theirs, "density");
    // The probes' records have no reading without the mass: only the bundled files' do.
    let unweighed = record.get("DensitiesWithoutMass").map(|densities| {
        densities[index]
            .as_f64()
            .unwrap_or_else(|| panic!("{at}: {key}'s density without the mass"))
    });
    let Some(value) = ours.density else {
        assert_eq!(density, 0.0, "{at}: {key}");
        assert!(
            unweighed.is_none_or(|unweighed| unweighed == 0.0),
            "{at}: {key}"
        );
        departures.undefined += 1;
        return;
    };
    // With the stated masses taken out, OpenRocket's density is hpr's, every time.
    if let Some(unweighed) = unweighed {
        assert_eq!(value, unweighed, "{at}: {key} without the mass");
        departures.unweighed_equal += 1;
    }
    // With them, it differs exactly where a solid part states its mass: the cause, both ways.
    let derives = ours.kind == MaterialKind::Bulk && part.mass_kg.is_some();
    assert_eq!(
        value != density,
        derives,
        "{at}: {key}: {value} against {density}"
    );
    if !derives {
        departures.equal += 1;
        return;
    }
    departures.derived_densities += 1;
    if let Some(volume) = volume_m3 {
        // Against OpenRocket's own reading of the mass, in its ounce where stated in one. The
        // largest gap measured is 5.9e-16 of the mass, a few steps of the last digit.
        let theirs_kg = density * volume;
        let stated = number(record, "Mass");
        assert!(
            ((theirs_kg - stated) / stated).abs() < 1e-15,
            "{at}: {theirs_kg} kg against {stated} kg"
        );
        departures.derived_checked += 1;
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
        // The file OpenRocket read is the one bundled, byte for byte.
        let digest = sha2::Sha256::digest(text.as_bytes());
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(file["sha256"], hex.as_str(), "{name}");
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
    println!(
        "{departures:#?}\nnumbers compared: {}",
        departures.numbers()
    );
    // The counts the guide's format page and ADR-132 give.
    assert_eq!(departures.equal, 17_911);
    assert_eq!(departures.numbers(), 18_306);
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
    assert_eq!(departures.derived_checked, 54);
    assert_eq!(departures.undefined, 3);
    assert_eq!(departures.unweighed_equal, 3594 - 3);
}

/// The keys OpenRocket holds for a part of `kind`, all of which `compare` compares.
fn keys(kind: &str) -> &'static [&'static str] {
    match kind {
        "BODY_TUBE" | "TUBE_COUPLER" | "ENGINE_BLOCK" | "CENTERING_RING" | "LAUNCH_LUG" => {
            &["OuterDiameter", "InnerDiameter", "Length", "Thickness"]
        }
        "BULK_HEAD" => &["OuterDiameter", "Length", "Filled"],
        "NOSE_CONE" => &[
            "Shape",
            "Length",
            "AftOuterDiameter",
            "AftShoulderDiameter",
            "AftShoulderLength",
            "Filled",
            "Thickness",
        ],
        "TRANSITION" => &[
            "Shape",
            "Length",
            "ForeOuterDiameter",
            "ForeShoulderDiameter",
            "ForeShoulderLength",
            "AftOuterDiameter",
            "AftShoulderDiameter",
            "AftShoulderLength",
            "Filled",
            "Thickness",
        ],
        "PARACHUTE" => &[
            "Diameter",
            "Sides",
            "LineCount",
            "LineLength",
            "LineMaterial",
        ],
        "STREAMER" => &["Length", "Width", "Thickness"],
        other => panic!("OpenRocket's kind `{other}`"),
    }
}

fn compare(departures: &mut Departures, part: &Part, record: &Value, at: &str) {
    // Every key OpenRocket holds for the part's kind is one compared below.
    let common = [
        "Type",
        "Legacy",
        "Manufacturer",
        "PartNo",
        "Description",
        "Mass",
        "Material",
        "DensitiesWithoutMass",
    ];
    let kind = record["Type"].as_str().expect("a kind");
    for key in record.as_object().expect("a part is an object").keys() {
        assert!(
            common.contains(&key.as_str()) || keys(kind).contains(&key.as_str()),
            "{at}: OpenRocket's `{key}` on a {kind}"
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
            // Stated in ounces: the same number of OpenRocket's ounces. The largest gap measured
            // is 2.0e-16.
            let ounces = ours / OUNCE_KG;
            assert!(
                ((ounces * OPENROCKET_OUNCE_KG - theirs) / theirs).abs() < 1e-15,
                "{at}: {ours} kg against {theirs} kg"
            );
            departures.ounce_masses += 1;
        }
        (ours, theirs) => panic!("{at}: mass {ours:?} against {theirs:?}"),
    }
    let cylinder = |diameter: f64, length: f64| PI / 4.0 * diameter * diameter * length;
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
            // π/4 (D² − d²) L, the squares' difference taken first: two cylinders' difference
            // loses digits on a thin wall.
            let (outer, inner) = (tube.outer_diameter_m, tube.inner_diameter_m);
            let volume = PI / 4.0 * (outer * outer - inner * inner) * tube.length_m;
            let material_ref = &tube.material;
            material(
                departures,
                record,
                "Material",
                0,
                material_ref,
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
            let volume = cylinder(bulkhead.outer_diameter_m, bulkhead.length_m);
            let material_ref = &bulkhead.material;
            material(
                departures,
                record,
                "Material",
                0,
                material_ref,
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
            // A filled cone and a solid shoulder: πr²L/3 + πr_s²L_s.
            let volume = (nose.shape == Shape::Conical && nose.filled == Some(true)).then(|| {
                cylinder(nose.outer_diameter_m, nose.length_m) / 3.0
                    + cylinder(nose.shoulder_diameter_m, nose.shoulder_length_m)
            });
            material(
                departures,
                record,
                "Material",
                0,
                &nose.material,
                part,
                volume,
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
            // A filled frustum, πL(R₁² + R₁R₂ + R₂²)/3, and two solid shoulders.
            let solid = transition.shape == Shape::Conical && transition.filled == Some(true);
            let volume = solid.then(|| {
                let (fore, aft) = (
                    transition.fore_outer_diameter_m / 2.0,
                    transition.aft_outer_diameter_m / 2.0,
                );
                PI * transition.length_m * (fore * fore + fore * aft + aft * aft) / 3.0
                    + cylinder(
                        transition.fore_shoulder_diameter_m,
                        transition.fore_shoulder_length_m,
                    )
                    + cylinder(
                        transition.aft_shoulder_diameter_m,
                        transition.aft_shoulder_length_m,
                    )
            });
            let material_ref = &transition.material;
            material(
                departures,
                record,
                "Material",
                0,
                material_ref,
                part,
                volume,
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
            let canopy = &parachute.material;
            material(departures, record, "Material", 0, canopy, part, None, at);
            match &parachute.line_material {
                Some(line) => {
                    material(departures, record, "LineMaterial", 1, line, part, None, at);
                }
                None => assert!(record.get("LineMaterial").is_none(), "{at}"),
            }
        }
        PartKind::Streamer(streamer) => {
            assert_eq!(record["Type"], "STREAMER", "{at}");
            same(departures, record, "Length", streamer.length_m, at);
            same(departures, record, "Width", streamer.width_m, at);
            same(departures, record, "Thickness", streamer.thickness_m, at);
            let fabric = &streamer.material;
            material(departures, record, "Material", 0, fabric, part, None, at);
        }
        _ => panic!("{at}: a kind this test doesn't know"),
    }
}

/// What hpr does with a probe, against OpenRocket's reading of it.
enum Probe {
    /// The same parts and values, to the bit.
    Same,
    /// The same, but for one value in a unit whose factor OpenRocket rounds: hpr's is the exact
    /// definition, within 2e-9 of OpenRocket's and not equal to it.
    Rounded(&'static str),
    /// OpenRocket reads the one part (`in/64` as inches, `ten` as zero, a material of the wrong
    /// kind with a density of zero, `BT<b>-</b>20` as `20`); hpr leaves it out with a warning.
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
        ("unreadable length", Probe::LeftOut),
        ("unknown shape", Probe::Refused),
        ("material twice", Probe::Same),
        ("second components", Probe::Same),
        ("second materials", Probe::Same),
        ("element in number", Probe::LeftOut),
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
        [23, 6, 4, 4]
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
                // Reading only the last `<Materials>`, both leave the first's material undefined.
                let undefined = usize::from(*name == "second materials");
                assert_eq!(
                    (
                        departures.ounce_masses,
                        departures.derived_densities,
                        departures.undefined
                    ),
                    (0, 0, undefined),
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
                // The largest gap measured is 1.6e-9.
                assert!(((ours - theirs_value) / ours).abs() < 2e-9, "{name}");
            }
            Probe::MaterialRefused => {
                assert!(theirs.is_none(), "{name}: {probe}");
                let messages: Vec<_> = read.warnings.iter().map(|w| w.message.as_str()).collect();
                assert_eq!(
                    messages,
                    [
                        "`oz/in` is not a unit of line density this reads; it was left out",
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
                // What OpenRocket made of the sizes hpr won't read.
                let length = theirs
                    .and_then(|parts| parts.first())
                    .and_then(|part| part["Length"].as_f64());
                match *name {
                    "length in/64" => assert_eq!(length, Some(3.0 * 0.0254), "{name}"),
                    "unreadable length" => assert_eq!(length, Some(0.0), "{name}"),
                    "wrong kind" => {
                        let material = theirs.and_then(|parts| parts.first()).map(|part| {
                            (
                                part["Material"]["kind"].clone(),
                                part["Material"]["density"].clone(),
                            )
                        });
                        assert_eq!(
                            material,
                            Some((Value::from("SURFACE"), Value::from(0.0))),
                            "{name}"
                        );
                    }
                    "element in number" => {
                        let number = theirs
                            .and_then(|parts| parts.first())
                            .map(|part| &part["PartNo"]);
                        assert_eq!(number, Some(&Value::from("20")), "{name}");
                    }
                    _ => {}
                }
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
