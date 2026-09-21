//! hpr's structure mass, centre of mass and inertia held to OpenRocket's (M2.2a, ADR-060).
//!
//! `validation/oracles/openrocket/mass.py` asks OpenRocket 24.12 for each design's structure:
//! every stage, no motors, after OpenRocket has settled the design by saving it. This compares
//! `Layout::structure` with it: the mass, the centre of mass's station as a share of the rocket's
//! length, the roll inertia, and the pitch inertia (the mean of the two transverse ones, which does
//! not depend on how either program turns its axes about the rocket's). Which of OpenRocket's
//! inertias is which is measured on a probe tube worked out by hand, which the record carries.

use std::collections::{BTreeMap, BTreeSet};

use hpr_design::tree::{Component, Layout, Rocket};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::ork_geometry::key;

/// OpenRocket run on Loft's public demo designs, committed.
#[cfg(test)]
const PUBLIC: &str = "validation/fixtures/ork/openrocket-mass-loft-demo.json";

/// The same run over the reference library and the jar's examples, which stays out of the
/// repository (CLAUDE.md rule 4).
pub(crate) const LIBRARY: &str = "corpus-out/openrocket-mass.json";

/// The quantities compared, in the order they are printed.
pub(crate) const QUANTITIES: [&str; 4] = ["mass", "cg", "roll_inertia", "pitch_inertia"];

/// The thresholds a design is held to, set before any design was measured (ADR-060): a design
/// outside either needs a written hypothesis.
pub(crate) const MASS_WITHIN: f64 = 0.01;
pub(crate) const CG_WITHIN: f64 = 0.01;

/// hpr's structure against OpenRocket's, each difference relative: mass and inertias to
/// OpenRocket's value, the centre of mass's station to the rocket's length.
pub(crate) fn differences(layout: &Layout, openrocket: &Value) -> Option<[f64; 4]> {
    let structure = &layout.structure;
    let mass = openrocket["mass_kg"].as_f64()?;
    let cm_x = openrocket["cm_x_m"].as_f64()?;
    let roll = openrocket["ixx"].as_f64()?;
    let pitch = (openrocket["iyy"].as_f64()? + openrocket["izz"].as_f64()?) / 2.0;
    // hpr's body `+z` points at the nose from an origin at the tip (`docs/physics/frames.md`),
    // so a station aft of the tip is `-z`.
    let station = -structure.cg_m.z;
    let inertia = structure.inertia_kg_m2;
    let hpr_pitch = (inertia.x_axis.x + inertia.y_axis.y) / 2.0;
    Some([
        (structure.mass_kg - mass) / mass,
        (station - cm_x) / layout.length_m,
        (inertia.z_axis.z - roll) / roll,
        (hpr_pitch - pitch) / pitch,
    ])
}

/// A spread of relative differences: how many within 0.1% and 1%, the median and the worst.
fn spread(values: &[f64]) -> String {
    if values.is_empty() {
        return "none".to_owned();
    }
    let mut sizes: Vec<f64> = values.iter().map(|v| v.abs()).collect();
    sizes.sort_by(f64::total_cmp);
    let within = |bound: f64| sizes.iter().filter(|size| **size <= bound).count();
    let worst = values
        .iter()
        .copied()
        .max_by(|a, b| a.abs().total_cmp(&b.abs()))
        .unwrap_or_default();
    format!(
        "{} within 0.1%, {} within 1% of {}; median {:.3}%, worst {:+.3}%",
        within(0.001),
        within(0.01),
        sizes.len(),
        100.0 * sizes[sizes.len() / 2],
        100.0 * worst
    )
}

/// The counts, summed over the designs.
#[derive(Debug, Default)]
pub(crate) struct MassTally {
    /// The record, by design, when the library run has been made.
    record: Option<BTreeMap<String, Value>>,
    /// The probe's mapping failures: an OpenRocket inertia that is not the one worked by hand.
    probe: Vec<String>,
    /// Files the record holds, and how many OpenRocket opened.
    recorded: [usize; 2],
    /// Differences for every design compared, and for the first design of each content.
    all: Vec<[f64; 4]>,
    distinct: Vec<[f64; 4]>,
    seen: BTreeSet<String>,
    /// Differences for designs hpr reads reduced (pods and parallel stages kept, not modelled).
    reduced: Vec<[f64; 4]>,
    /// Designs outside a threshold: the file, its differences, the start of its content hash (how
    /// a private design is named in print), and its parts that differ most.
    outside: Vec<(String, [f64; 4], String, Value)>,
    refused: usize,
    stale: Vec<String>,
    missing: Vec<String>,
}

impl MassTally {
    /// The tally for a survey of the library, with the library's record if it has been written.
    pub(crate) fn load(root: &std::path::Path, library: bool) -> Result<Self, String> {
        let path = root.join(LIBRARY);
        if !library || !path.is_file() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path).map_err(|error| format!("{LIBRARY}: {error}"))?;
        let record: Value =
            serde_json::from_str(&text).map_err(|error| format!("{LIBRARY}: {error}"))?;
        Ok(Self::of(&record))
    }

    fn of(record: &Value) -> Self {
        let designs: BTreeMap<String, Value> = record["designs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|design| Some((key(design["file"].as_str()?), design.clone())))
            .collect();
        let opened = designs
            .values()
            .filter(|design| design["opens"] == true)
            .count();
        Self {
            probe: probe_mismatches(&record["probe"]),
            recorded: [designs.len(), opened],
            record: Some(designs),
            ..Self::default()
        }
    }

    /// Compares one design the survey laid out, when the record holds it, and returns the detail.
    pub(crate) fn add(
        &mut self,
        name: &str,
        bytes: &[u8],
        rocket: &Rocket,
        layout: &Layout,
        reduced: bool,
    ) -> Value {
        let Some(designs) = &mut self.record else {
            return Value::Null;
        };
        let Some(design) = designs.remove(&key(name)) else {
            self.missing.push(key(name));
            return Value::Null;
        };
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if design["sha256"].as_str() != Some(digest.as_str()) {
            self.stale.push(key(name));
            return Value::Null;
        }
        if design["opens"] != true {
            self.refused += 1;
            return json!({ "openrocket_refuses": design["refused"] });
        }
        let Some(found) = differences(layout, &design["structure"]) else {
            self.stale
                .push(format!("{}: no structure in the record", key(name)));
            return Value::Null;
        };
        self.all.push(found);
        if self.seen.insert(digest) {
            self.distinct.push(found);
        }
        if reduced {
            self.reduced.push(found);
        }
        let parts = parts(rocket, layout, &design["parts"]);
        if found[0].abs() > MASS_WITHIN || found[1].abs() > CG_WITHIN {
            let hash = design["sha256"].as_str().unwrap_or_default();
            self.outside.push((
                key(name),
                found,
                hash.chars().take(8).collect(),
                parts.clone(),
            ));
        }
        json!({
            "parts": parts,
            "reduced": reduced,
            "differences": QUANTITIES.iter().zip(found).map(|(q, d)| (q.to_string(), json!(d)))
                .collect::<BTreeMap<_, _>>(),
            "openrocket": design["structure"],
        })
    }

    /// Notes a design the survey read but could not lay out: not compared, and not missing.
    pub(crate) fn not_laid_out(&mut self, name: &str) {
        if let Some(designs) = &mut self.record {
            designs.remove(&key(name));
        }
    }

    pub(crate) fn print(&self) {
        if self.record.is_none() {
            println!(
                "  structure mass against OpenRocket 24.12: not run; \
                 validation/oracles/openrocket/mass.py writes {LIBRARY}"
            );
            return;
        }
        println!(
            "  structure mass, centre of mass and inertia against OpenRocket 24.12 ({LIBRARY}): {} \
             file(s) recorded, {} opened by OpenRocket; {} design(s) compared, {} distinct by \
             content, {} of them reduced",
            self.recorded[0],
            self.recorded[1],
            self.all.len(),
            self.distinct.len(),
            self.reduced.len()
        );
        for (index, quantity) in QUANTITIES.iter().enumerate() {
            let column = |rows: &[[f64; 4]]| rows.iter().map(|row| row[index]).collect::<Vec<_>>();
            println!(
                "    {quantity}: {}; each file once: {}; reduced designs: {}",
                spread(&column(&self.all)),
                spread(&column(&self.distinct)),
                spread(&column(&self.reduced))
            );
        }
        println!(
            "    designs outside 1% in mass or 1% of length in centre of mass: {}, each with the \
             parts that differ most (a part hpr does not have shows as `none`):",
            self.outside.len()
        );
        // A public design is named; one from the private library, by the start of its hash.
        for (file, found, hash, parts) in &self.outside {
            let label = if file.starts_with("refs/loft-fixtures/")
                || file.starts_with("refs/debrief-fixtures/")
            {
                format!("private {hash}")
            } else {
                file.rsplit('/').next().unwrap_or(file).to_owned()
            };
            let top: Vec<String> = parts
                .as_array()
                .into_iter()
                .flatten()
                .take(3)
                .filter(|part| part["apart_kg"].as_f64().is_some_and(|kg| kg.abs() >= 1e-4))
                .map(|part| match part["hpr_kg"].as_f64() {
                    Some(_) => format!(
                        "{} {:+.4} kg",
                        part["class"].as_str().unwrap_or("?"),
                        part["apart_kg"].as_f64().unwrap_or_default()
                    ),
                    None => format!(
                        "{} none in hpr ({:.4} kg in OpenRocket)",
                        part["class"].as_str().unwrap_or("?"),
                        part["openrocket_kg"].as_f64().unwrap_or_default()
                    ),
                })
                .collect();
            println!(
                "      {label}: mass {:+.2}%, centre of mass {:+.2}% of length; {}",
                100.0 * found[0],
                100.0 * found[1],
                top.join(", ")
            );
        }
    }

    /// The designs outside a threshold, for the per-file report.
    pub(crate) fn outside(&self) -> Value {
        json!(
            self.outside
                .iter()
                .map(|(file, found, hash, parts)| json!({
                    "file": file, "sha256": hash, "differences": found, "parts": parts,
                }))
                .collect::<Vec<_>>()
        )
    }

    /// Why the survey fails on the mass record, if it does: a record out of date, or a probe whose
    /// inertias are not the ones worked by hand. A design outside a threshold does not fail it;
    /// it needs a hypothesis, which is written, not computed.
    pub(crate) fn failure(&self) -> Option<String> {
        let left = self.record.as_ref()?;
        let mut problems = self.probe.clone();
        problems.extend(
            self.stale
                .iter()
                .map(|file| format!("{file} has changed since mass.py ran")),
        );
        problems.extend(
            self.missing
                .iter()
                .map(|file| format!("{file} lays out but is not in {LIBRARY}: run mass.py again")),
        );
        problems.extend(
            left.iter()
                .filter(|(_, design)| design["opens"] == true)
                .map(|(file, _)| {
                    format!("{file} is in {LIBRARY} but the survey did not lay it out")
                }),
        );
        (!problems.is_empty()).then(|| {
            format!(
                "the OpenRocket mass record does not hold:\n  {}",
                problems.join("\n  ")
            )
        })
    }
}

/// Each of OpenRocket's parts beside hpr's part of the same id, largest difference first: where a
/// design's difference comes from. A part hpr has no component for (a pod, a part left out) is
/// listed with hpr's mass as `null`. hpr's is the part's own mass after the overrides that cover it
/// alone; an override of a part and everything on it is compared on the assembly (`with_children`),
/// as OpenRocket reports the overriding part's mass for the whole assembly.
fn parts(rocket: &Rocket, layout: &Layout, parts: &Value) -> Value {
    // hpr's parts by name, in its own mass. An older file writes no ids, and OpenRocket then makes
    // up random ones, so there a part is found by name: the parts of one name are compared as a
    // group, the sum of OpenRocket's against the sum of hpr's.
    // Only hpr's parts no id of OpenRocket's matches take part in the groups.
    let matched: BTreeSet<&str> = parts
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|part| part["id"].as_str())
        .filter(|id| layout.find(id).is_some())
        .collect();
    let mut ours_named: BTreeMap<&str, (usize, f64)> = BTreeMap::new();
    fn visit<'a>(
        component: &'a Component,
        layout: &Layout,
        matched: &BTreeSet<&str>,
        named: &mut BTreeMap<&'a str, (usize, f64)>,
    ) {
        if !matched.contains(component.id.as_str()) {
            let entry = named.entry(component.name.as_str()).or_default();
            entry.0 += 1;
            entry.1 += layout
                .find(&component.id)
                .map_or(0.0, |(_, placed)| placed.own.mass_kg);
        }
        for child in &component.children {
            visit(child, layout, matched, named);
        }
    }
    for stage in &rocket.stages {
        for component in &stage.components {
            visit(component, layout, &matched, &mut ours_named);
        }
    }
    let row = |name: &Value, class: &Value, count: usize, theirs: f64, ours: Option<f64>| {
        let apart = ours.map_or(theirs, |ours| ours - theirs);
        (
            apart.abs(),
            json!({
                "name": name, "class": class, "count": count,
                "openrocket_kg": theirs, "hpr_kg": ours, "apart_kg": apart,
            }),
        )
    };
    let mut rows: Vec<(f64, Value)> = Vec::new();
    let mut by_name: BTreeMap<String, (Value, usize, f64)> = BTreeMap::new();
    for part in parts.as_array().into_iter().flatten().filter(|part| {
        !matches!(
            part["class"].as_str(),
            Some("Rocket" | "AxialStage" | "ParallelStage" | "PodSet")
        )
    }) {
        let theirs = part["mass_kg"].as_f64().unwrap_or_default();
        match part["id"].as_str().and_then(|id| layout.find(id)) {
            Some((_, placed)) => rows.push(row(
                &part["name"],
                &part["class"],
                1,
                theirs,
                Some(placed.own.mass_kg),
            )),
            None => {
                let name = part["name"].as_str().unwrap_or_default().to_owned();
                let entry = by_name
                    .entry(name)
                    .or_insert_with(|| (part["class"].clone(), 0, 0.0));
                entry.1 += 1;
                entry.2 += theirs;
            }
        }
    }
    for (name, (class, count, theirs)) in by_name {
        // Only a whole group is compared: hpr's parts of that name that no id already matched.
        let ours = ours_named
            .get(name.as_str())
            .filter(|(ours_count, _)| *ours_count == count)
            .map(|(_, kg)| *kg);
        rows.push(row(&json!(name), &class, count, theirs, ours));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    json!(rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>())
}

/// Where the probe's OpenRocket inertias are not the ones worked by hand: roll is `ixx` and the
/// rotational inertia, pitch is `iyy`, `izz` and the longitudinal inertia, all about the centre of
/// mass. This is what lets the comparison take `ixx` as roll.
fn probe_mismatches(probe: &Value) -> Vec<String> {
    let hand = &probe["by_hand"];
    let or = &probe["openrocket"];
    let pairs = [
        ("mass_kg", "mass_kg"),
        ("cm_x_m", "cm_x_m"),
        ("roll_inertia", "ixx"),
        ("roll_inertia", "rotational_inertia"),
        ("pitch_inertia", "iyy"),
        ("pitch_inertia", "izz"),
        ("pitch_inertia", "longitudinal_inertia"),
    ];
    pairs
        .iter()
        .filter(
            |(ours, theirs)| match (hand[*ours].as_f64(), or[*theirs].as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() > 1e-9 * a.abs(),
                _ => true,
            },
        )
        .map(|(ours, theirs)| format!("the probe's {theirs} is not its {ours} worked by hand"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use hpr_io::ork;
    use std::path::Path;

    fn record() -> Value {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        serde_json::from_str(&std::fs::read_to_string(root.join(PUBLIC)).unwrap()).unwrap()
    }

    /// OpenRocket's probe tube is the one worked by hand, so `ixx` is roll and `iyy`, `izz` pitch;
    /// and hpr, laying the same document out, gives the same numbers, which pins hpr's side of the
    /// mapping: a centre of mass at `z = -0.5`, roll about `z`.
    #[test]
    fn the_probe_tube_is_the_one_worked_by_hand() {
        let record = record();
        assert_eq!(probe_mismatches(&record["probe"]), Vec::<String>::new());
        let document = record["probe"]["document"].as_str().unwrap();
        let read = ork::read(document.as_bytes()).unwrap();
        let spine = ork::rocket(&read.value.document);
        let layout = spine.value.layout().unwrap();
        let found = differences(&layout, &record["probe"]["openrocket"]).unwrap();
        assert!(found.iter().all(|d| d.abs() < 1e-9), "{found:?}");
    }

    /// The committed record of OpenRocket run on Loft's seven public demo designs, held to hpr's
    /// layout of the same files.
    #[test]
    fn openrocket_structure_on_the_loft_demos() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let record = record();
        let mut tally = MassTally::of(&record);
        for design in record["designs"].as_array().unwrap() {
            let file = design["file"].as_str().unwrap();
            let bytes = std::fs::read(root.join(file)).unwrap();
            let read = ork::read(&bytes).unwrap();
            let whole = ork::design(&read.value);
            let layout = whole.value.rocket.layout().unwrap();
            tally.add(
                file,
                &bytes,
                &whole.value.rocket,
                &layout,
                whole.value.is_reduced(),
            );
        }
        assert_eq!(tally.failure(), None);
        // Six designs open in OpenRocket (demo-quirks does not). Their mass and centre of mass are
        // OpenRocket's within 0.1%, their pitch inertia within 0.1%; their roll inertia is not,
        // by 1.2% to 3.8% either way, which the guide reports as unexplained.
        assert_eq!((tally.all.len(), tally.refused), (6, 1));
        assert!(tally.outside.is_empty());
        for found in &tally.all {
            assert!(found[0].abs() < 1e-3 && found[1].abs() < 1e-3, "{found:?}");
            assert!(found[3].abs() < 1e-3, "{found:?}");
            assert!((0.01..0.04).contains(&found[2].abs()), "{found:?}");
        }
    }
}
