//! Every part of the bundled `.orc` catalogue built with the builder, against what OpenRocket
//! 24.12 builds from it.
//!
//! `tests/fixtures/orc/openrocket-built.json` is each part OpenRocket's preset loader returns for
//! each bundled file, applied to a new component of its kind
//! (`validation/oracles/openrocket/orc_built.py`): the component's mass and centre of mass, and
//! the dimensions the file leaves unsaid that OpenRocket chose. Each part is built here with its
//! `from_catalog` and added to a rocket, and its mass and centre as laid out are held to
//! OpenRocket's. The thresholds were set before measuring: a nose cone's or transition's mass
//! within 1e-3 of OpenRocket's and its centre within 1e-3 of its length, OpenRocket integrating
//! their volumes numerically; every other part within 1e-12, its sizes taken as they are.
//!
//! The departures `hpr::rocket::catalog` names are counted, each shown to have its cause:
//! a hollow part's shoulders (OpenRocket's weigh nothing; here they have the part's wall, and the
//! difference is their closed-form mass), the one streamer that states a mass (OpenRocket leaves
//! it unused), masses stated in ounces (OpenRocket's ounce), and the parts the builder refuses (a
//! material the file doesn't define; a tube or ring no narrower inside than out, which OpenRocket
//! weighs as zero).

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "tests stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::collections::BTreeMap;
use std::f64::consts::PI;

use hpr::hpr_design::PlacedComponent;
use hpr::hpr_io::orc::{BUNDLED_FILES, Part, PartKind, Shape, read};
use hpr::rocket::{Fitting, Nose, Transition, Tube, material};
use hpr::{CatalogProblem, Error, Rocket};
use serde_json::Value;

/// OpenRocket's ounce, kg, against the exact one (ADR-132 §4).
const OPENROCKET_OUNCE_KG: f64 = 0.028_349_523_1;
const OUNCE_KG: f64 = 0.028_349_523_125;

/// The thresholds, set before measuring: relative in mass, a fraction of the part's length in
/// its centre.
const REVOLVED: f64 = 1e-3;
const EXACT: f64 = 1e-12;

fn fixture() -> Value {
    let text = include_str!("fixtures/orc/openrocket-built.json");
    serde_json::from_str(text).expect("the fixture is JSON")
}

/// Largest differences found and departures counted, by group.
#[derive(Default, Debug)]
struct Survey {
    /// Parts built and held to OpenRocket, by kind.
    held: BTreeMap<&'static str, usize>,
    /// Largest relative mass difference, by group (`revolved`, `exact`).
    mass: BTreeMap<&'static str, f64>,
    /// Largest centre difference as a fraction of length, by group.
    centre: BTreeMap<&'static str, f64>,
    /// Hollow parts whose shoulders weigh their closed-form mass more than OpenRocket's.
    hollow_shoulders: usize,
    /// Hollow, shouldered parts stating their mass: the same mass, the centre not compared.
    hollow_shoulders_stated: usize,
    /// Streamers weighing their stated mass, where OpenRocket weighs their material.
    stated_streamers: usize,
    /// Stated masses that differ by OpenRocket's ounce alone.
    ounce_masses: usize,
    /// Parts refused for a material their file doesn't define.
    undefined_materials: usize,
    /// Tube-like parts refused for a bore not narrower than the outside.
    no_bore: usize,
    /// Hollow elliptical nose cones whose wall hpr weighs as the ellipse's inner parallel curve
    /// gives it, and OpenRocket more than the threshold lighter.
    ellipsoid_shells: usize,
    /// Hollow elliptical nose cones checked against the parallel curve, all of them.
    ellipsoid_shells_checked: usize,
    /// Filled conical nose cones held to their closed-form mass.
    cones_closed_form: usize,
    /// Parts outside a threshold, each with its numbers.
    misses: Vec<String>,
}

fn kind_name(part: &Part) -> &'static str {
    match &part.kind {
        PartKind::BodyTube(_) => "body tube",
        PartKind::TubeCoupler(_) => "tube coupler",
        PartKind::EngineBlock(_) => "engine block",
        PartKind::CenteringRing(_) => "centering ring",
        PartKind::LaunchLug(_) => "launch lug",
        PartKind::Bulkhead(_) => "bulkhead",
        PartKind::NoseCone(_) => "nose cone",
        PartKind::Transition(_) => "transition",
        PartKind::Parachute(_) => "parachute",
        PartKind::Streamer(_) => "streamer",
        _ => panic!("a kind this test doesn't know"),
    }
}

/// The part built with the builder, laid out: the component made from it. A transition follows
/// a tube of its fore diameter; a fitting goes on a tube wider than it.
fn build(part: &Part) -> Result<PlacedComponent, Error> {
    let kraft = material("kraft_phenolic")?;
    let mut rocket;
    match &part.kind {
        PartKind::NoseCone(_) => {
            rocket = Rocket::new("catalogue", 0.1)?;
            rocket.add_nose(Nose::from_catalog(part)?)?;
        }
        PartKind::BodyTube(_) => {
            rocket = Rocket::new("catalogue", 0.1)?;
            rocket.add_tube(Tube::from_catalog(part)?)?;
        }
        PartKind::Transition(transition) => {
            rocket = Rocket::new("catalogue", transition.fore_outer_diameter_m)?;
            rocket.add_tube(Tube::new(0.1, 0.001, kraft))?;
            rocket.add_transition(Transition::from_catalog(part)?)?;
        }
        _ => {
            rocket = Rocket::new("catalogue", 0.5)?;
            rocket.add_tube(Tube::new(1.0, 0.001, kraft))?;
            rocket.add_fitting(Fitting::from_catalog(part)?)?;
        }
    }
    let layout = rocket.design().layout()?;
    let index = match &part.kind {
        PartKind::NoseCone(_) | PartKind::BodyTube(_) => 0,
        // After the tube and its children.
        _ => layout.components.len() - 1,
    };
    Ok(layout.components[index].clone())
}

/// The part's length along the axis, m, shoulders included.
fn length_m(part: &Part) -> f64 {
    match &part.kind {
        PartKind::BodyTube(t)
        | PartKind::TubeCoupler(t)
        | PartKind::EngineBlock(t)
        | PartKind::CenteringRing(t)
        | PartKind::LaunchLug(t) => t.length_m,
        PartKind::Bulkhead(b) => b.length_m,
        PartKind::NoseCone(n) => n.length_m + n.shoulder_length_m,
        PartKind::Transition(t) => t.length_m + t.fore_shoulder_length_m + t.aft_shoulder_length_m,
        PartKind::Parachute(_) | PartKind::Streamer(_) => 1.0,
        _ => panic!("a kind this test doesn't know"),
    }
}

/// A hollow part's shoulders' mass in closed form, kg: each a tube of the part's wall (or
/// solid where that is thicker than its radius) and material. Zero for a filled part or one
/// with no shoulder.
fn hollow_shoulders_kg(part: &Part) -> f64 {
    let (shoulders, filled, thickness_m, material) = match &part.kind {
        PartKind::NoseCone(n) => (
            vec![(n.shoulder_diameter_m, n.shoulder_length_m)],
            n.filled,
            n.thickness_m,
            &n.material,
        ),
        PartKind::Transition(t) => (
            vec![
                (t.fore_shoulder_diameter_m, t.fore_shoulder_length_m),
                (t.aft_shoulder_diameter_m, t.aft_shoulder_length_m),
            ],
            t.filled,
            t.thickness_m,
            &t.material,
        ),
        _ => return 0.0,
    };
    if filled == Some(true) {
        return 0.0;
    }
    let wall = thickness_m.expect("a hollow part gives its wall");
    let density = material
        .density
        .expect("a built part's material is defined");
    shoulders
        .into_iter()
        .filter(|&(d, l)| d > 0.0 && l > 0.0)
        .map(|(d, l)| {
            let r = 0.5 * d;
            let inner = r - wall.min(r);
            PI * (r * r - inner * inner) * l * density
        })
        .sum()
}

/// A tube-like part whose bore is no narrower than its outside.
fn no_bore(part: &Part) -> bool {
    match &part.kind {
        PartKind::TubeCoupler(t)
        | PartKind::EngineBlock(t)
        | PartKind::CenteringRing(t)
        | PartKind::LaunchLug(t)
        | PartKind::BodyTube(t) => t.inner_diameter_m >= t.outer_diameter_m,
        _ => false,
    }
}

/// A hollow elliptical nose cone's body (its shoulder aside), kg, by the inner parallel curve of
/// its ellipse; `None` for any other part.
///
/// The profile is the quarter ellipse `x = L(1 − cos θ)`, `r = R sin θ`, `θ` from the tip to the
/// base, `x` aft of the tip. A wall of thickness `t` normal to it, with `t` under the ellipse's
/// least radius of curvature `R²/L` (when `L ≥ R`), is the region between it and the curve `t`
/// inside it along the normal, which runs from `(t, 0)` at the tip to `(L, R − t)` at the base.
/// The wall's volume is the half ellipsoid's `2πR²L/3` less the volume that curve encloses,
/// `π ∫ r_i² dx_i`, here by Simpson's rule in `θ` over 20,000 intervals.
fn ellipsoid_shell_kg(part: &Part) -> Option<f64> {
    let PartKind::NoseCone(nose) = &part.kind else {
        return None;
    };
    if nose.shape != Shape::Ellipsoid || nose.filled == Some(true) {
        return None;
    }
    let (l, r) = (nose.length_m, 0.5 * nose.outer_diameter_m);
    let t = nose.thickness_m.expect("a hollow part gives its wall");
    assert!(l >= r && t < r * r / l, "the parallel curve is regular");
    let integrand = |theta: f64| {
        let (s, c) = theta.sin_cos();
        let n = (l * l * s * s + r * r * c * c).sqrt();
        let dn = (l * l - r * r) * s * c / n;
        // The inner curve and its rate along θ.
        let ri = r * s - t * l * s / n;
        let dxi = l * s + t * r * (-s * n - c * dn) / (n * n);
        PI * ri * ri * dxi
    };
    let steps = 20_000;
    let h = 0.5 * PI / f64::from(steps);
    let mut sum = integrand(0.0) + integrand(0.5 * PI);
    for i in 1..steps {
        sum += if i % 2 == 1 { 4.0 } else { 2.0 } * integrand(f64::from(i) * h);
    }
    let cavity = sum * h / 3.0;
    let density = nose
        .material
        .density
        .expect("a built part's material is defined");
    Some((2.0 * PI * r * r * l / 3.0 - cavity) * density)
}

/// A filled conical nose cone's mass, kg, from its closed-form volume, `πR²L/3` and its solid
/// shoulder's `πr²l`; `None` for any other part.
fn filled_cone_kg(part: &Part) -> Option<f64> {
    let PartKind::NoseCone(nose) = &part.kind else {
        return None;
    };
    if nose.shape != Shape::Conical || nose.filled != Some(true) {
        return None;
    }
    let r = 0.5 * nose.outer_diameter_m;
    let s = 0.5 * nose.shoulder_diameter_m;
    let density = nose.material.density?;
    Some(PI * (r * r * nose.length_m / 3.0 + s * s * nose.shoulder_length_m) * density)
}

fn is_conical(part: &Part) -> bool {
    match &part.kind {
        PartKind::NoseCone(n) => n.shape == Shape::Conical,
        PartKind::Transition(t) => t.shape == Shape::Conical,
        _ => false,
    }
}

fn revolved(part: &Part) -> bool {
    matches!(part.kind, PartKind::NoseCone(_) | PartKind::Transition(_))
}

fn worst(map: &mut BTreeMap<&'static str, f64>, group: &'static str, value: f64) {
    let entry = map.entry(group).or_insert(0.0);
    *entry = entry.max(value);
}

/// Holds one part to OpenRocket's record of it, or counts its departure.
fn compare(survey: &mut Survey, part: &Part, record: &Value, at: &str) {
    let theirs_kg = record
        .get("override_mass_kg")
        .unwrap_or(&record["mass_kg"])
        .as_f64()
        .expect("a mass");
    let theirs_cg_m = record["cg_m"].as_f64().expect("a centre");
    let built = match build(part) {
        Ok(built) => built,
        Err(Error::Catalog {
            problem: CatalogProblem::UndefinedMaterial(_),
            ..
        }) => {
            // OpenRocket weighs the material as zero: the part's material, or a parachute's
            // lines.
            survey.undefined_materials += 1;
            return;
        }
        Err(Error::Design(_)) if no_bore(part) => {
            // OpenRocket weighs it as zero.
            assert_eq!(theirs_kg, 0.0, "{at}");
            survey.no_bore += 1;
            return;
        }
        Err(error) => panic!("{at}: the builder refused it: {error}"),
    };
    let ours_kg = built.own.mass_kg;
    // OpenRocket's centre is metres aft of the component's fore end; ours is a station.
    let ours_cg_m = -built.own.cg_m.z - built.fore_station_m;
    let (group, tolerance) = if revolved(part) {
        ("revolved", REVOLVED)
    } else {
        ("exact", EXACT)
    };
    let length = length_m(part);
    let shoulders_kg = hollow_shoulders_kg(part);

    if let Some(stated_kg) = part.mass_kg {
        // The stated mass is the component's override: it weighs that.
        assert!(
            (ours_kg - stated_kg).abs() <= EXACT * stated_kg,
            "{at}: {ours_kg} kg built, {stated_kg} kg stated"
        );
        if matches!(part.kind, PartKind::Streamer(_)) {
            let PartKind::Streamer(streamer) = &part.kind else {
                unreachable!()
            };
            let density = streamer.material.density.expect("defined");
            let area_kg = density * streamer.length_m * streamer.width_m;
            assert!(
                (theirs_kg - area_kg).abs() <= EXACT * area_kg,
                "{at}: OpenRocket weighs the material, {area_kg} kg; it has {theirs_kg} kg"
            );
            assert!((ours_kg - theirs_kg).abs() > 1e-3 * theirs_kg);
            survey.stated_streamers += 1;
            return;
        }
        let ratio = theirs_kg / ours_kg;
        if (ratio - 1.0).abs() <= EXACT {
            worst(&mut survey.mass, "stated", (ratio - 1.0).abs());
        } else {
            assert!(
                (ratio - OPENROCKET_OUNCE_KG / OUNCE_KG).abs() <= EXACT,
                "{at}: {theirs_kg} kg in OpenRocket, {ours_kg} kg here, not an ounce apart"
            );
            survey.ounce_masses += 1;
        }
        if shoulders_kg > 0.0 {
            // The same mass spread over a heavier shoulder: the centre isn't OpenRocket's.
            survey.hollow_shoulders_stated += 1;
            *survey.held.entry(kind_name(part)).or_default() += 1;
            return;
        }
    } else {
        let relative = (ours_kg - shoulders_kg - theirs_kg).abs() / theirs_kg;
        if let Some(shell_kg) = ellipsoid_shell_kg(part) {
            // hpr's wall is every point within its thickness of the surface: for an ellipse,
            // the region between it and its inner parallel curve, integrated here on its own.
            let body_kg = ours_kg - shoulders_kg;
            assert!(
                (body_kg - shell_kg).abs() <= 1e-9 * shell_kg,
                "{at}: {body_kg} kg here, {shell_kg} kg by the parallel curve"
            );
            survey.ellipsoid_shells_checked += 1;
            if relative > tolerance {
                assert!(
                    theirs_kg < body_kg,
                    "{at}: OpenRocket's shell is the lighter"
                );
                worst(&mut survey.mass, "ellipsoid shells", relative);
                survey.ellipsoid_shells += 1;
                if shoulders_kg > 0.0 {
                    survey.hollow_shoulders += 1;
                }
                *survey.held.entry(kind_name(part)).or_default() += 1;
                return;
            }
        }
        if relative > tolerance {
            survey.misses.push(format!(
                "{at}: {ours_kg} kg here less {shoulders_kg} kg of hollow shoulders, {theirs_kg} \
                 kg in OpenRocket: {relative:e} apart"
            ));
        }
        worst(&mut survey.mass, group, relative);
        if revolved(part) && is_conical(part) && shoulders_kg == 0.0 {
            worst(&mut survey.mass, "revolved cones", relative);
        }
        if let Some(cone_kg) = filled_cone_kg(part) {
            // A cone's volume is closed-form: hpr's is it, so a difference is OpenRocket's.
            assert!(
                (ours_kg - cone_kg).abs() <= EXACT * cone_kg,
                "{at}: {ours_kg} kg here, {cone_kg} kg by the cone's and cylinder's volumes"
            );
            survey.cones_closed_form += 1;
        }
        if shoulders_kg > 0.0 {
            assert!(ours_kg - theirs_kg > 0.5 * shoulders_kg, "{at}");
            survey.hollow_shoulders += 1;
            *survey.held.entry(kind_name(part)).or_default() += 1;
            return;
        }
    }
    // A parachute's or streamer's centre is where it is packed, not in the catalogue.
    if !matches!(part.kind, PartKind::Parachute(_) | PartKind::Streamer(_)) {
        let off = (ours_cg_m - theirs_cg_m).abs() / length;
        if off > tolerance {
            survey.misses.push(format!(
                "{at}: centre {ours_cg_m} m here, {theirs_cg_m} m in OpenRocket: {off:e} of \
                 {length} m"
            ));
        }
        worst(&mut survey.centre, group, off);
    }
    *survey.held.entry(kind_name(part)).or_default() += 1;
}

/// The dimensions OpenRocket chose for what the file leaves unsaid are the ones the builder
/// chooses (`hpr::rocket::catalog`).
fn unsaid_as_builder_chooses(part: &Part, record: &Value, at: &str) {
    let flag = |key: &str| record[key].as_bool().expect("a flag");
    let value = |key: &str| record[key].as_f64().expect("a number");
    let (shape, filled) = match &part.kind {
        PartKind::NoseCone(n) => (n.shape, n.filled),
        PartKind::Transition(t) => (t.shape, t.filled),
        PartKind::Parachute(_) => {
            assert_eq!(
                value("CD"),
                0.8,
                "{at}: OpenRocket's parachute drag coefficient"
            );
            return;
        }
        _ => return,
    };
    let parameter = match shape {
        Shape::Ogive | Shape::Parabolic => 1.0,
        Shape::Power => 0.5,
        Shape::Conical | Shape::Ellipsoid | Shape::Haack => 0.0,
        _ => panic!("a shape this test doesn't know"),
    };
    assert_eq!(value("ShapeParameter"), parameter, "{at}: shape parameter");
    if let Some(clipped) = record.get("Clipped") {
        let ours = matches!(shape, Shape::Ellipsoid | Shape::Haack | Shape::Power);
        assert_eq!(clipped.as_bool(), Some(ours), "{at}: clipped");
    }
    assert_eq!(flag("Filled"), filled == Some(true), "{at}: filled");
    for key in ["AftShoulderCapped", "ForeShoulderCapped"] {
        if record.get(key).is_some() {
            assert!(!flag(key), "{at}: {key}");
        }
    }
    // OpenRocket's shoulder walls: a filled part's its radius (solid), whatever its length, a
    // hollow part's zero.
    for (key, diameter_m) in shoulder_diameters(part) {
        if let Some(thickness) = record.get(key) {
            let expected = if filled == Some(true) && diameter_m > 0.0 {
                0.5 * diameter_m
            } else {
                0.0
            };
            assert_eq!(thickness.as_f64(), Some(expected), "{at}: {key}");
        }
    }
}

fn shoulder_diameters(part: &Part) -> Vec<(&'static str, f64)> {
    match &part.kind {
        PartKind::NoseCone(n) => vec![("AftShoulderThickness", n.shoulder_diameter_m)],
        PartKind::Transition(t) => vec![
            ("ForeShoulderThickness", t.fore_shoulder_diameter_m),
            ("AftShoulderThickness", t.aft_shoulder_diameter_m),
        ],
        _ => Vec::new(),
    }
}

#[test]
fn every_catalogue_part_weighs_what_openrocket_builds() {
    let fixture = fixture();
    let files = fixture["files"].as_array().expect("files");
    let names: Vec<_> = files.iter().map(|file| file["file"].as_str()).collect();
    let bundled: Vec<_> = BUNDLED_FILES.iter().map(|(name, _)| Some(*name)).collect();
    assert_eq!(
        names, bundled,
        "the fixture covers every bundled file, in order"
    );
    let mut survey = Survey::default();
    let mut parts = 0;
    let mut stated = 0;
    for ((name, text), file) in BUNDLED_FILES.iter().zip(files) {
        let read = read(text, name).expect("every bundled file reads");
        let records = file["parts"].as_array().expect("parts");
        assert_eq!(read.catalog.parts.len(), records.len(), "{name}: parts");
        for (index, (part, record)) in read.catalog.parts.iter().zip(records).enumerate() {
            let at = format!("{name} part {index} ({})", part.part_number);
            assert_eq!(
                record["PartNo"].as_str(),
                Some(part.part_number.as_str()),
                "{at}"
            );
            unsaid_as_builder_chooses(part, record, &at);
            compare(&mut survey, part, record, &at);
            parts += 1;
            stated += usize::from(part.mass_kg.is_some());
        }
    }
    println!("{parts} parts: {survey:#?}");
    let held: usize = survey.held.values().sum();
    assert_eq!(parts, 3_449);
    assert_eq!(stated, 229, "parts stating their mass");
    assert_eq!(
        held + survey.undefined_materials + survey.no_bore + survey.stated_streamers,
        parts
    );
    assert!(survey.misses.is_empty(), "{:#?}", survey.misses);
    // The largest differences, as `docs/the-builder.md` and ADR-133 quote them: the revolved
    // parts' to two digits, the rest by a bound, their last bits being the platform's.
    let printed = |value: f64| format!("{value:.1e}");
    assert_eq!(printed(survey.mass["revolved"]), "6.3e-4", "{survey:#?}");
    assert_eq!(printed(survey.centre["revolved"]), "7.0e-5", "{survey:#?}");
    assert_eq!(
        printed(survey.mass["ellipsoid shells"]),
        "4.8e-3",
        "{survey:#?}"
    );
    assert_eq!(
        printed(survey.mass["revolved cones"]),
        "6.8e-7",
        "{survey:#?}"
    );
    assert!(survey.mass["exact"] <= 1e-14, "{survey:#?}");
    assert!(survey.centre["exact"] <= 1e-13, "{survey:#?}");
    assert!(survey.mass["stated"] <= 1e-15, "{survey:#?}");
    assert_eq!(survey.ellipsoid_shells_checked, 6, "{survey:#?}");
    assert_eq!(survey.cones_closed_form, 85, "{survey:#?}");
    assert_eq!(survey.ellipsoid_shells, 3, "{survey:#?}");
    assert_eq!(survey.hollow_shoulders, 67, "{survey:#?}");
    assert_eq!(survey.hollow_shoulders_stated, 74, "{survey:#?}");
    assert_eq!(survey.ounce_masses, 185, "{survey:#?}");
    assert_eq!(survey.undefined_materials, 3, "{survey:#?}");
    assert_eq!(survey.no_bore, 3, "{survey:#?}");
    assert_eq!(survey.stated_streamers, 1, "{survey:#?}");
}

#[test]
fn probes_weigh_what_openrocket_builds() {
    let fixture = fixture();
    let probes = fixture["probes"].as_array().expect("probes");
    let mut survey = Survey::default();
    for probe in probes {
        let label = probe["probe"].as_str().expect("a label");
        let read = read(probe["text"].as_str().expect("text"), label).expect("each probe reads");
        let records = probe["parts"].as_array().expect("parts");
        assert_eq!(read.catalog.parts.len(), records.len(), "{label}");
        for (part, record) in read.catalog.parts.iter().zip(records) {
            unsaid_as_builder_chooses(part, record, label);
            compare(&mut survey, part, record, label);
        }
    }
    println!("{survey:#?}");
    assert_eq!(survey.held.values().sum::<usize>(), 6);
}

#[test]
fn a_part_of_another_kind_is_refused() {
    let catalog = hpr::hpr_io::orc::bundled();
    let tube = catalog
        .parts
        .iter()
        .find(|part| matches!(part.kind, PartKind::BodyTube(_)))
        .expect("a body tube");
    let Err(Error::Catalog {
        problem: CatalogProblem::Kind { found, builder },
        ..
    }) = Nose::from_catalog(tube)
    else {
        panic!("a body tube isn't a nose cone");
    };
    assert_eq!((found, builder), ("body tube", "Nose::from_catalog"));
    assert!(Fitting::from_catalog(tube).is_err());
}
