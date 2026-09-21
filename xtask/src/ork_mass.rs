//! hpr's structure mass, centre of mass and inertia held to OpenRocket's (M2.2a, ADR-060).
//!
//! `validation/oracles/openrocket/mass.py` asks OpenRocket 24.12 for each design's structure:
//! every stage, no motors, after OpenRocket has settled the design by saving it. This compares
//! `Layout::structure` with it: the mass, the centre of mass's station as a share of the rocket's
//! length, the roll inertia, and the pitch inertia (the mean of the two transverse ones, which does
//! not depend on how either program turns its axes about the rocket's). Which of OpenRocket's
//! inertias is which is measured on a probe tube worked out by hand, which the record carries.

use std::collections::{BTreeMap, BTreeSet};

use hpr_design::tree::{Component, Layout, Part, Rocket};
use hpr_validate::openrocket::openrocket_fin_set_roll_kg_m2;
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

/// hpr's roll inertia with OpenRocket's fin rule in place of hpr's exact integral, against
/// OpenRocket's (ADR-062).
#[derive(Debug, Clone, Copy)]
pub(crate) struct RollUnderRule {
    /// hpr's less OpenRocket's, relative to OpenRocket's.
    pub(crate) apart: f64,
    /// hpr's less OpenRocket's, kg·m².
    pub(crate) apart_kg_m2: f64,
    /// Whether a fin set kept hpr's own: tube fins, which the rule does not cover, or a fin set
    /// with no OpenRocket part to take the mass from, by id or by name.
    pub(crate) unpaired_fins: bool,
}

/// hpr's roll inertia with each fin set given OpenRocket's rule ([`openrocket_fin_set_roll_kg_m2`],
/// inferred from OpenRocket's output) on OpenRocket's own mass for it in place of hpr's: what is
/// left of the roll inertia's difference once that rule, and the way OpenRocket weighs a fin
/// section or fillets, are set aside. OpenRocket's mass is paired by id. An older file writes no
/// ids, so there a fin set takes the mass of OpenRocket's set of the same name nearest its own, if
/// within 30% (the most a section moves it is 24%, an airfoil's `0.85 / 0.6851`).
pub(crate) fn roll_under_openrocket_fins(
    rocket: &Rocket,
    layout: &Layout,
    openrocket: &Value,
    their_parts: &Value,
) -> Option<RollUnderRule> {
    fn names<'a>(components: &'a [Component], into: &mut BTreeMap<&'a str, &'a str>) {
        for component in components {
            into.insert(component.id.as_str(), component.name.as_str());
            names(&component.children, into);
        }
    }
    let mut name_of = BTreeMap::new();
    for stage in &rocket.stages {
        names(&stage.components, &mut name_of);
    }
    let theirs: Vec<&Value> = their_parts
        .as_array()
        .into_iter()
        .flatten()
        .filter(|part| {
            part["class"]
                .as_str()
                .is_some_and(|c| c.ends_with("FinSet"))
        })
        .collect();
    let by_name = |name: &str, ours_kg: f64| -> Option<f64> {
        theirs
            .iter()
            .filter(|part| part["name"].as_str() == Some(name))
            .filter_map(|part| part["mass_kg"].as_f64())
            .min_by(|a, b| (a - ours_kg).abs().total_cmp(&(b - ours_kg).abs()))
            .filter(|theirs_kg| (theirs_kg - ours_kg).abs() <= 0.3 * theirs_kg.abs())
    };
    let roll = openrocket["ixx"].as_f64()?;
    let mut ours = layout.structure.inertia_kg_m2.z_axis.z;
    let mut unpaired_fins = false;
    for placed in &layout.components {
        match &placed.part {
            Part::TubeFinSet(_) => unpaired_fins = true,
            Part::FinSet(fins) => {
                let mass_kg = theirs
                    .iter()
                    .find(|part| part["id"].as_str() == Some(placed.id.as_str()))
                    .and_then(|part| part["mass_kg"].as_f64())
                    .or_else(|| by_name(name_of.get(placed.id.as_str())?, placed.own.mass_kg));
                let rule = mass_kg
                    .zip(placed.body_radius_m)
                    .and_then(|(mass_kg, radius_m)| {
                        openrocket_fin_set_roll_kg_m2(fins, radius_m, mass_kg)
                    });
                match rule {
                    Some(rule) => ours += rule - placed.own.inertia_kg_m2.z_axis.z,
                    None => unpaired_fins = true,
                }
            }
            _ => {}
        }
    }
    Some(RollUnderRule {
        apart: (ours - roll) / roll,
        apart_kg_m2: ours - roll,
        unpaired_fins,
    })
}

/// How far the roll inertia may be from OpenRocket's, once OpenRocket's fin rule is in hpr's place,
/// before a design needs a cause: 1%, as for the mass (ADR-062).
pub(crate) const ROLL_WITHIN: f64 = 0.01;

/// Whether a stage, or a part with parts inside it, overrides the mass of what it covers: there hpr
/// scales the inertia of everything covered and OpenRocket that of the overriding part alone, a
/// departure ADR-061 keeps. On a lone part the two agree, so its own override is not counted.
fn covering_mass_override(rocket: &Rocket) -> bool {
    fn any(components: &[Component]) -> bool {
        components.iter().any(|component| {
            (component.overrides.mass_kg.is_some()
                && component.overrides_include_children
                && !component.children.is_empty())
                || any(&component.children)
        })
    }
    rocket
        .stages
        .iter()
        .any(|stage| stage.overrides.mass_kg.is_some() || any(&stage.components))
}

/// The median of sorted values: the middle one, or the mean of the two middle ones.
fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[n / 2 - 1] + sorted[n / 2]) / 2.0
    }
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
        100.0 * median(&sizes),
        100.0 * worst
    )
}

/// The causes a design outside a threshold is traced to, each by what hpr says when it reads the
/// file, in the order they are printed. M2.2a's first two, a shoulder written with no wall and a
/// part written with no material, are gone: hpr now reads both as OpenRocket does (ADR-061).
pub(crate) const CAUSES: [&str; 4] = [
    "a cluster of motor tubes, read as one tube",
    "fin fillets, left out",
    "parts hpr keeps unread (a reduced design)",
    "a stage OpenRocket's configuration switches off",
];

/// The causes a design shows, from the warnings hpr raised reading it, whether it is reduced, and
/// whether OpenRocket weighs fewer stages than hpr. The words are the importer's; a test holds
/// them to what it says.
fn causes(warnings: &[&str], reduced: bool, stages_apart: bool) -> Vec<&'static str> {
    let said = |words: &str| warnings.iter().any(|warning| warning.contains(words));
    let found = [said(CLUSTER), said(FILLETS), reduced, stages_apart];
    CAUSES
        .iter()
        .zip(found)
        .filter_map(|(cause, found)| found.then_some(*cause))
        .collect()
}

/// The causes a roll inertia outside [`ROLL_WITHIN`] is traced to, once OpenRocket's fin rule is in
/// hpr's place.
pub(crate) const ROLL_CAUSES: [&str; 4] = [
    "a mass override covering parts inside (ADR-061)",
    "parts hpr keeps unread (a reduced design)",
    "fins the rule was not given OpenRocket's mass for (tube fins, or no id to pair)",
    "packed parts hpr weighs as point masses, which account for the gap (ADR-062)",
];

/// The most roll inertia hpr's packed point masses can lack, kg·m²: each parachute, streamer, shock
/// cord or mass component with mass but no roll inertia in hpr, spread over a packing as wide as the
/// tube it sits in (at least the 12.5 mm OpenRocket gives a packing that writes none), `m R²/2`.
/// OpenRocket gives such a part its packing's roll inertia, where hpr has a point mass: a packing
/// that writes no radius, or a mass override on a part that weighs nothing (ADR-062's probes).
fn point_mass_bound_kg_m2(layout: &Layout) -> f64 {
    layout
        .components
        .iter()
        .filter(|placed| {
            matches!(
                placed.part,
                Part::Parachute(_)
                    | Part::Streamer(_)
                    | Part::ShockCord(_)
                    | Part::MassComponent(_)
            ) && placed.own.mass_kg > 0.0
                && placed.own.inertia_kg_m2.z_axis.z == 0.0
        })
        .map(|placed| {
            let mut up = placed.parent;
            let mut radius_m = 0.0;
            while let Some(index) = up {
                if let Some(radius) = layout.components[index].part.fore_radius_m() {
                    radius_m = radius;
                    break;
                }
                up = layout.components[index].parent;
            }
            let radius_m = f64::max(radius_m, 0.0125);
            placed.own.mass_kg * radius_m * radius_m / 2.0
        })
        .sum()
}

/// The causes of a roll inertia outside [`ROLL_WITHIN`] with OpenRocket's fin rule in hpr's place.
/// A point mass is a cause only when hpr's shortfall is no more than it could account for.
fn roll_causes(
    rocket: &Rocket,
    layout: &Layout,
    reduced: bool,
    under_rule: &RollUnderRule,
) -> Vec<&'static str> {
    let bound = point_mass_bound_kg_m2(layout);
    let found = [
        covering_mass_override(rocket),
        reduced,
        under_rule.unpaired_fins,
        under_rule.apart_kg_m2 < 0.0 && -under_rule.apart_kg_m2 <= bound,
    ];
    ROLL_CAUSES
        .iter()
        .zip(found)
        .filter_map(|(cause, found)| found.then_some(*cause))
        .collect()
}

/// A design whose roll inertia is outside [`ROLL_WITHIN`] even with OpenRocket's fin rule.
#[derive(Debug)]
struct RollOutside {
    label: String,
    hash: String,
    apart: f64,
    causes: Vec<&'static str>,
}

/// How `hpr_io::ork` words the warnings `causes` looks for.
const CLUSTER: &str = "a cluster of motor tubes is read as the one tube";
const FILLETS: &str = "the fillets along the fin roots were dropped";

/// How a design is named in print: by its file when it is public (the jar's examples, Loft's own
/// repository, the parts catalogue, and the repository's own fixtures), and otherwise only by the
/// start of its content hash, so that a private file's name never leaves the machine.
fn label(file: &str, hash: &str) -> String {
    let public = [
        "datafiles/examples/",
        "refs/fusionspace-loft/",
        "refs/openrocket-database/",
        "validation/fixtures/",
    ];
    if public.iter().any(|prefix| file.starts_with(prefix)) {
        file.rsplit('/').next().unwrap_or(file).to_owned()
    } else {
        format!("private {}", hash.chars().take(8).collect::<String>())
    }
}

/// A design outside a threshold, as printed.
#[derive(Debug)]
struct Outside {
    label: String,
    hash: String,
    found: [f64; 4],
    causes: Vec<&'static str>,
    parts: Value,
}

/// The counts, summed over the designs.
#[derive(Debug, Default)]
pub(crate) struct MassTally {
    /// The record, by design, when the library run has been made.
    record: Option<BTreeMap<String, Value>>,
    /// Files the record holds, and how many OpenRocket opened.
    recorded: [usize; 2],
    /// Differences for every design compared, and for the first design of each content.
    all: Vec<[f64; 4]>,
    distinct: Vec<[f64; 4]>,
    /// The roll inertia's difference with OpenRocket's fin rule, for every design and each content.
    roll_under_rule: [Vec<f64>; 2],
    /// Designs whose roll inertia is outside [`ROLL_WITHIN`] even so, each with its causes, of
    /// [`ROLL_CAUSES`].
    roll_outside: Vec<RollOutside>,
    seen: BTreeSet<String>,
    /// Differences for designs hpr reads reduced (pods and parallel stages kept, not modelled).
    reduced: Vec<[f64; 4]>,
    /// Designs outside a threshold.
    outside: Vec<Outside>,
    refused: usize,
    /// Problems that fail the survey: a record out of date or incomplete, a design missing from
    /// it, one OpenRocket opens that hpr does not lay out, and an error in the script itself.
    problems: Vec<String>,
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
        let mut problems = probe_mismatches(&record["probe"]);
        for (file, design) in &designs {
            if let Some(error) = design["driver_error"].as_str() {
                let hash = design["sha256"].as_str().unwrap_or_default();
                problems.push(format!("mass.py failed on {}: {error}", label(file, hash)));
            }
        }
        Self {
            recorded: [designs.len(), opened],
            record: Some(designs),
            problems,
            ..Self::default()
        }
    }

    /// Compares one design the survey laid out, when the record holds it, and returns the detail.
    /// `warnings` are what hpr said reading it, which name the cause of a difference.
    pub(crate) fn add(
        &mut self,
        name: &str,
        bytes: &[u8],
        rocket: &Rocket,
        layout: &Layout,
        reduced: bool,
        warnings: &[&str],
    ) -> Value {
        let Some(designs) = &mut self.record else {
            return Value::Null;
        };
        let digest: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let named = label(&key(name), &digest);
        let Some(design) = designs.remove(&key(name)) else {
            self.problems.push(format!(
                "{named} lays out but is not in {LIBRARY}: run mass.py again"
            ));
            return Value::Null;
        };
        if design["sha256"].as_str() != Some(digest.as_str()) {
            self.problems
                .push(format!("{named} has changed since mass.py ran"));
            return Value::Null;
        }
        if design["driver_error"].is_string() {
            return Value::Null;
        }
        if design["opens"] != true {
            self.refused += 1;
            return json!({ "openrocket_refuses": design["refused"] });
        }
        // A record without OpenRocket's parts, or with some it could not name, was written by an
        // older script or went wrong: it would leave the trace below empty without a sound.
        let (Some(found), Some(_)) = (
            differences(layout, &design["structure"]),
            design["parts"].as_array().filter(|parts| !parts.is_empty()),
        ) else {
            self.problems.push(format!(
                "{named}: the record has no structure or no parts; run mass.py again"
            ));
            return Value::Null;
        };
        if design["parts_skipped"].as_u64() != Some(0) {
            self.problems.push(format!(
                "{named}: mass.py could not name some of OpenRocket's parts"
            ));
        }
        let under_rule =
            roll_under_openrocket_fins(rocket, layout, &design["structure"], &design["parts"]);
        let under_rule = under_rule.unwrap_or(RollUnderRule {
            apart: f64::NAN,
            apart_kg_m2: f64::NAN,
            unpaired_fins: false,
        });
        self.all.push(found);
        self.roll_under_rule[0].push(under_rule.apart);
        if self.seen.insert(digest.clone()) {
            self.distinct.push(found);
            self.roll_under_rule[1].push(under_rule.apart);
        }
        if reduced {
            self.reduced.push(found);
        }
        // OpenRocket's own count, since it counts a parallel stage that hpr keeps unread.
        let stages_apart = design["stages"]["active"] != design["stages"]["total"];
        // A difference that is not a number counts as outside.
        let beyond =
            |difference: f64, within: f64| difference.is_nan() || difference.abs() > within;
        let parts = parts(rocket, layout, &design["parts"]);
        if beyond(under_rule.apart, ROLL_WITHIN) {
            self.roll_outside.push(RollOutside {
                label: named.clone(),
                hash: digest.clone(),
                apart: under_rule.apart,
                causes: roll_causes(rocket, layout, reduced, &under_rule),
            });
        }
        if beyond(found[0], MASS_WITHIN) || beyond(found[1], CG_WITHIN) {
            self.outside.push(Outside {
                label: named,
                hash: digest,
                found,
                causes: causes(warnings, reduced, stages_apart),
                parts: parts.clone(),
            });
        }
        json!({
            "parts": parts,
            "reduced": reduced,
            "differences": QUANTITIES.iter().zip(found).map(|(q, d)| (q.to_string(), json!(d)))
                .collect::<BTreeMap<_, _>>(),
            "roll_inertia_with_openrocket_fins": under_rule.apart,
            "openrocket": design["structure"],
        })
    }

    /// Notes a design the survey read but could not lay out: not compared. One OpenRocket opens
    /// fails the survey, since it would otherwise leave the comparison without a sound.
    pub(crate) fn not_laid_out(&mut self, name: &str) {
        if let Some(designs) = &mut self.record
            && let Some(design) = designs.remove(&key(name))
            && design["opens"] == true
        {
            let hash = design["sha256"].as_str().unwrap_or_default();
            self.problems.push(format!(
                "{} opens in OpenRocket but does not lay out in hpr",
                label(&key(name), hash)
            ));
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
            "    roll_inertia with OpenRocket's fin rule in place of hpr's (ADR-062): {}; each file \
             once: {}",
            spread(&self.roll_under_rule[0]),
            spread(&self.roll_under_rule[1])
        );
        let roll_distinct: BTreeSet<&str> =
            self.roll_outside.iter().map(|o| o.hash.as_str()).collect();
        let by_roll_cause = |cause: &str| {
            let hashes: BTreeSet<&str> = self
                .roll_outside
                .iter()
                .filter(|o| o.causes.contains(&cause))
                .map(|o| o.hash.as_str())
                .collect();
            hashes.len()
        };
        println!(
            "    roll inertia outside {}% even with OpenRocket's fin rule: {} ({} distinct by \
             content); by distinct content: {}",
            100.0 * ROLL_WITHIN,
            self.roll_outside.len(),
            roll_distinct.len(),
            ROLL_CAUSES
                .iter()
                .map(|cause| format!("{cause} {}", by_roll_cause(cause)))
                .collect::<Vec<_>>()
                .join("; ")
        );
        for outside in &self.roll_outside {
            println!(
                "      {}: roll inertia {:+.2}%; causes: {}",
                outside.label,
                100.0 * outside.apart,
                if outside.causes.is_empty() {
                    "NONE".to_owned()
                } else {
                    outside.causes.join("; ")
                }
            );
        }
        let distinct: BTreeSet<&str> = self.outside.iter().map(|o| o.hash.as_str()).collect();
        println!(
            "    designs outside 1% in mass or 1% of length in centre of mass: {} ({} distinct by \
             content), each with the causes hpr warned of and the parts that differ most:",
            self.outside.len(),
            distinct.len()
        );
        for outside in &self.outside {
            let top: Vec<String> = outside
                .parts
                .as_array()
                .into_iter()
                .flatten()
                .take(3)
                .filter(|part| part["apart_kg"].as_f64().is_some_and(|kg| kg.abs() >= 1e-4))
                .map(|part| {
                    let class = part["class"].as_str().unwrap_or("?");
                    match (part["hpr_kg"].as_f64(), part["openrocket_kg"].as_f64()) {
                        (Some(_), Some(_)) => format!(
                            "{class} {:+.4} kg",
                            part["apart_kg"].as_f64().unwrap_or_default()
                        ),
                        (None, Some(theirs)) => {
                            format!("{class} none in hpr ({theirs:.4} kg in OpenRocket)")
                        }
                        (Some(ours), None) => {
                            format!("{class} none in OpenRocket ({ours:.4} kg in hpr)")
                        }
                        (None, None) => class.to_owned(),
                    }
                })
                .collect();
            println!(
                "      {}: mass {:+.2}%, centre of mass {:+.2}% of length; causes: {}; parts: {}",
                outside.label,
                100.0 * outside.found[0],
                100.0 * outside.found[1],
                if outside.causes.is_empty() {
                    "NONE".to_owned()
                } else {
                    outside.causes.join("; ")
                },
                top.join(", ")
            );
        }
        // Each cause counted once per distinct design content, as the guide's table counts them.
        let mut by_cause: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for outside in &self.outside {
            for cause in &outside.causes {
                by_cause.entry(cause).or_default().insert(&outside.hash);
            }
        }
        println!(
            "    causes, by distinct content: {}",
            CAUSES
                .iter()
                .map(|cause| format!("{cause} {}", by_cause.get(cause).map_or(0, BTreeSet::len)))
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    /// The designs outside a threshold, for the per-file report.
    pub(crate) fn outside(&self) -> Value {
        json!(
            self.outside
                .iter()
                .map(|outside| json!({
                    "label": outside.label, "sha256": outside.hash, "differences": outside.found,
                    "causes": outside.causes, "parts": outside.parts,
                }))
                .collect::<Vec<_>>()
        )
    }

    /// Why the survey fails on the mass record, if it does: a record out of date or incomplete, a
    /// probe whose inertias are not the ones worked by hand, a design OpenRocket opens that hpr
    /// does not lay out or compare, a design outside a threshold with no cause hpr warned of, and a
    /// roll inertia outside [`ROLL_WITHIN`] with OpenRocket's fin rule and no cause.
    pub(crate) fn failure(&self) -> Option<String> {
        let left = self.record.as_ref()?;
        let mut problems = self.problems.clone();
        problems.extend(
            left.iter()
                .filter(|(_, design)| design["opens"] == true)
                .map(|(file, design)| {
                    let hash = design["sha256"].as_str().unwrap_or_default();
                    format!(
                        "{} is in {LIBRARY} but the survey did not read it",
                        label(file, hash)
                    )
                }),
        );
        problems.extend(
            self.roll_outside
                .iter()
                .filter(|outside| outside.causes.is_empty())
                .map(|outside| {
                    format!(
                        "{}'s roll inertia is {:+.3}% from OpenRocket's with its fin rule, with \
                         no cause",
                        outside.label,
                        100.0 * outside.apart
                    )
                }),
        );
        problems.extend(
            self.outside
                .iter()
                .filter(|outside| outside.causes.is_empty())
                .map(|outside| {
                    format!(
                        "{} is outside a threshold with no cause hpr warned of",
                        outside.label
                    )
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
/// design's difference comes from.
///
/// - An override that covers a part and everything on it is compared on the assembly: OpenRocket
///   reports the overriding part's mass as the override, so hpr's is its `with_children`, and the
///   parts under it are left out of the ranking.
/// - An older file writes no ids, and OpenRocket then makes up random ones; there the parts of one
///   name are compared as a group, the sum of OpenRocket's against the sum of hpr's.
/// - A part only OpenRocket has shows hpr's mass as `null`, and one only hpr has shows
///   OpenRocket's as `null`; `apart_kg` is always hpr's less OpenRocket's.
fn parts(rocket: &Rocket, layout: &Layout, parts: &Value) -> Value {
    let rows_in: Vec<&Value> = parts
        .as_array()
        .into_iter()
        .flatten()
        .filter(|part| {
            !matches!(
                part["class"].as_str(),
                Some("Rocket" | "AxialStage" | "ParallelStage" | "PodSet")
            )
        })
        .collect();
    // The parts whose override covers the parts on them, and the parts it covers.
    let assemblies: BTreeSet<&str> = rows_in
        .iter()
        .filter_map(|part| {
            let by = part["overridden_by"].as_str()?;
            (Some(by) != part["id"].as_str()).then_some(by)
        })
        .collect();
    let covered = |part: &Value| {
        part["overridden_by"]
            .as_str()
            .is_some_and(|by| Some(by) != part["id"].as_str())
    };
    let matched: BTreeSet<&str> = rows_in
        .iter()
        .filter_map(|part| part["id"].as_str())
        .filter(|id| layout.find(id).is_some())
        .collect();
    // hpr's parts no id matched, by name, in their own mass.
    let mut ours_named: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    fn visit<'a>(
        component: &'a Component,
        matched: &BTreeSet<&str>,
        named: &mut BTreeMap<&'a str, Vec<&'a str>>,
    ) {
        if !matched.contains(component.id.as_str()) {
            named
                .entry(component.name.as_str())
                .or_default()
                .push(component.id.as_str());
        }
        for child in &component.children {
            visit(child, matched, named);
        }
    }
    for stage in &rocket.stages {
        for component in &stage.components {
            visit(component, &matched, &mut ours_named);
        }
    }
    let own = |id: &str| {
        layout
            .find(id)
            .map_or(0.0, |(_, placed)| placed.own.mass_kg)
    };
    let row =
        |name: &Value, class: &Value, count: usize, theirs: Option<f64>, ours: Option<f64>| {
            let apart = ours.unwrap_or_default() - theirs.unwrap_or_default();
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
    for part in rows_in.iter().filter(|part| !covered(part)) {
        let theirs = part["mass_kg"].as_f64().unwrap_or_default();
        let id = part["id"].as_str().unwrap_or_default();
        match layout.find(id) {
            Some((_, placed)) => {
                let ours = if assemblies.contains(id) {
                    placed.with_children.mass_kg
                } else {
                    placed.own.mass_kg
                };
                rows.push(row(
                    &part["name"],
                    &part["class"],
                    1,
                    Some(theirs),
                    Some(ours),
                ));
            }
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
    let mut grouped: BTreeSet<String> = BTreeSet::new();
    for (name, (class, count, theirs)) in by_name {
        // Only a whole group is compared: as many of hpr's parts of that name as OpenRocket's.
        let ours = ours_named
            .get(name.as_str())
            .filter(|ids| ids.len() == count)
            .map(|ids| ids.iter().map(|id| own(id)).sum::<f64>());
        if ours.is_some() {
            grouped.insert(name.clone());
        }
        rows.push(row(&json!(name), &class, count, Some(theirs), ours));
    }
    // hpr's parts OpenRocket has none of, by id or by a whole group of their name.
    for (name, ids) in &ours_named {
        if grouped.contains(*name) {
            continue;
        }
        for id in ids {
            if let Some((_, placed)) = layout.find(id) {
                rows.push(row(
                    &json!(name),
                    &json!(placed.part.kind_name()),
                    1,
                    None,
                    Some(placed.own.mass_kg),
                ));
            }
        }
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
    fn demo_tally() -> MassTally {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let record = record();
        let mut tally = MassTally::of(&record);
        for design in record["designs"].as_array().unwrap() {
            let file = design["file"].as_str().unwrap();
            let bytes = std::fs::read(root.join(file)).unwrap();
            let read = ork::read(&bytes).unwrap();
            let whole = ork::design(&read.value);
            let layout = whole.value.rocket.layout().unwrap();
            let said: Vec<&str> = whole
                .warnings
                .iter()
                .map(|warning| warning.message.as_str())
                .collect();
            tally.add(
                file,
                &bytes,
                &whole.value.rocket,
                &layout,
                whole.value.is_reduced(),
                &said,
            );
        }
        tally
    }

    /// Six designs open in OpenRocket (demo-quirks does not). Their mass, centre of mass and pitch
    /// inertia are OpenRocket's within 0.1%; their roll inertia is not, by the amounts pinned here
    /// (sign included). With OpenRocket's fin rule on its own fin mass in place of hpr's exact
    /// integral, five are within 5e-6 and the sixth, whose fins are elliptical, 0.093% apart:
    /// OpenRocket's 30-sided ellipse has less area, which the rule's `hₑ` takes, and with it the
    /// two are 1.5e-6 apart (`hpr_validate::openrocket`'s test of it; ADR-062). Every part of each
    /// is matched by id and within 0.3 g of OpenRocket's.
    #[test]
    fn openrocket_structure_on_the_loft_demos() {
        let tally = demo_tally();
        assert_eq!(tally.failure(), None);
        assert_eq!((tally.all.len(), tally.refused), (6, 1));
        assert!(tally.outside.is_empty());
        let roll: Vec<f64> = tally
            .all
            .iter()
            .map(|found| (found[2] * 1e4).round() / 1e4)
            .collect();
        assert_eq!(roll, [-0.0264, 0.0306, 0.0383, 0.0337, 0.0383, 0.0116]);
        let under_rule: Vec<f64> = tally.roll_under_rule[0]
            .iter()
            .map(|found| (found * 1e5).round() / 1e5)
            .collect();
        assert_eq!(under_rule, [0.00093, 0.0, 0.0, 0.0, 0.0, 0.0]);
        assert!(tally.roll_outside.is_empty());
        for found in &tally.all {
            assert!(found[0].abs() < 1e-3 && found[1].abs() < 1e-3, "{found:?}");
            assert!(found[3].abs() < 1e-3, "{found:?}");
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let record = record();
        for design in record["designs"].as_array().unwrap() {
            if design["opens"] != true {
                continue;
            }
            let bytes = std::fs::read(root.join(design["file"].as_str().unwrap())).unwrap();
            let read = ork::read(&bytes).unwrap();
            let whole = ork::design(&read.value);
            let layout = whole.value.rocket.layout().unwrap();
            let rows = parts(&whole.value.rocket, &layout, &design["parts"]);
            let rows = rows.as_array().unwrap();
            // One row for each of OpenRocket's parts other than the rocket and its stages.
            let theirs = design["parts"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|part| !matches!(part["class"].as_str(), Some("Rocket" | "AxialStage")))
                .count();
            assert!(theirs > 0 && rows.len() == theirs, "{rows:?}");
            for row in rows {
                assert_eq!(row["count"], 1, "{row}");
                assert!(
                    row["hpr_kg"].is_f64() && row["openrocket_kg"].is_f64(),
                    "{row}"
                );
                assert!(row["apart_kg"].as_f64().unwrap().abs() < 3e-4, "{row}");
            }
        }
    }

    /// A tube whose override covers everything on it: OpenRocket reports the override as the
    /// tube's mass, so hpr's assembly is what it is compared with, and the parts under it are not
    /// ranked on their own.
    #[test]
    fn an_assembly_override_is_compared_as_the_assembly() {
        let xml = br#"<openrocket version="1.10" creator="test"><rocket><name>R</name>
            <subcomponents><stage><name>S</name><id>s</id><subcomponents>
            <bodytube><name>T</name><id>t</id><length>0.5</length><thickness>0.002</thickness>
            <radius>0.04</radius><overridemass>1.0</overridemass>
            <overridesubcomponentsmass>true</overridesubcomponentsmass><subcomponents>
            <masscomponent><name>M</name><id>m</id><length>0.05</length>
            <radius>0.02</radius><mass>0.3</mass></masscomponent>
            </subcomponents></bodytube></subcomponents></stage></subcomponents></rocket></openrocket>"#;
        let read = ork::read(xml).unwrap();
        let spine = ork::rocket(&read.value.document);
        let layout = spine.value.layout().unwrap();
        let record = json!([
            { "id": "t", "name": "T", "class": "BodyTube", "mass_kg": 1.0, "overridden_by": "t" },
            { "id": "m", "name": "M", "class": "MassComponent", "mass_kg": 0.3, "overridden_by": "t" },
        ]);
        let rows = parts(&spine.value, &layout, &record);
        let rows = rows.as_array().unwrap();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0]["name"], "T");
        assert!(
            rows[0]["apart_kg"].as_f64().unwrap().abs() < 1e-9,
            "{rows:?}"
        );
    }

    /// The survey fails on a record written by an older script, on a design missing from it, on a
    /// probe whose inertias are swapped, and on a design outside a threshold with no warned cause;
    /// and a cause is read from the importer's own words.
    /// A committed probe of `openrocket-conventions.json`: hpr's rocket and layout of it, and
    /// OpenRocket's record.
    fn conventions_probe(question: &str) -> (Rocket, Layout, Value) {
        let text = include_str!("../../validation/fixtures/ork/openrocket-conventions.json");
        let record: Value = serde_json::from_str(text).unwrap();
        let probe = record["probes"][question].clone();
        let read = ork::read(probe["document"].as_str().unwrap().as_bytes()).unwrap();
        let rocket = ork::design(&read.value).value.rocket;
        let layout = rocket.layout().unwrap();
        (rocket, layout, probe)
    }

    /// The roll causes are measured on OpenRocket's probes, not assumed: which overrides cover the
    /// parts inside; OpenRocket's fin rule on its own fin mass closes an airfoil probe, and a fin
    /// set with nothing to pair keeps hpr's; and a packed point mass is a cause only for a gap of
    /// its sign and no larger than it can account for.
    #[test]
    fn roll_causes_are_measured_not_assumed() {
        for (question, covering) in [
            ("a mass override on a tube and the part inside", true),
            ("a mass override on a tube, not the part inside", false),
            ("a mass override on the part inside only", false),
            ("a mass override on the stage", true),
        ] {
            let (rocket, _, _) = conventions_probe(question);
            assert_eq!(covering_mass_override(&rocket), covering, "{question}");
        }

        let (rocket, layout, probe) = conventions_probe("a tube and a fin set of airfoil section");
        let paired =
            roll_under_openrocket_fins(&rocket, &layout, &probe["structure"], &probe["parts"])
                .unwrap();
        assert!(
            paired.apart.abs() < 1e-12 && !paired.unpaired_fins,
            "{paired:?}"
        );
        let unpaired =
            roll_under_openrocket_fins(&rocket, &layout, &probe["structure"], &json!([])).unwrap();
        assert!(
            unpaired.unpaired_fins && unpaired.apart < -0.02,
            "{unpaired:?}"
        );
        assert_eq!(
            roll_causes(&rocket, &layout, false, &unpaired),
            [ROLL_CAUSES[2]]
        );

        let question = "a tube and a parachute of no canopy, under a mass override";
        let (rocket, layout, probe) = conventions_probe(question);
        // OpenRocket spreads the 30 g over the 20 mm packing; hpr's bound is the 50 mm tube's.
        let bound = point_mass_bound_kg_m2(&layout);
        assert!(
            (bound - 0.03 * 0.05 * 0.05 / 2.0).abs() < 1e-15,
            "{bound:e}"
        );
        let under =
            roll_under_openrocket_fins(&rocket, &layout, &probe["structure"], &probe["parts"])
                .unwrap();
        assert!(
            (under.apart_kg_m2 + 0.03 * 0.02 * 0.02 / 2.0).abs() < 1e-15,
            "{under:?}"
        );
        assert_eq!(
            roll_causes(&rocket, &layout, false, &under),
            [ROLL_CAUSES[3]]
        );
        let wrong_sign = RollUnderRule {
            apart_kg_m2: -under.apart_kg_m2,
            ..under
        };
        let too_big = RollUnderRule {
            apart_kg_m2: -2.0 * bound,
            ..under
        };
        for gap in [wrong_sign, too_big] {
            assert!(
                roll_causes(&rocket, &layout, false, &gap).is_empty(),
                "{gap:?}"
            );
        }
        let (_, plain, _) = conventions_probe("a tube and a parachute");
        assert_eq!(point_mass_bound_kg_m2(&plain), 0.0);

        let tally = MassTally {
            record: Some(BTreeMap::new()),
            roll_outside: vec![RollOutside {
                label: "probe".to_owned(),
                hash: "0".to_owned(),
                apart: 0.02,
                causes: Vec::new(),
            }],
            ..MassTally::default()
        };
        assert!(tally.failure().unwrap().contains("with no cause"));
    }

    #[test]
    fn a_record_that_does_not_hold_fails() {
        let mut stale = record();
        for design in stale["designs"].as_array_mut().unwrap() {
            design.as_object_mut().unwrap().remove("parts");
        }
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let file = "validation/fixtures/ork/loft-demo/demo-stable.ork";
        let bytes = std::fs::read(root.join(file)).unwrap();
        let read = ork::read(&bytes).unwrap();
        let whole = ork::design(&read.value);
        let layout = whole.value.rocket.layout().unwrap();
        let mut tally = MassTally::of(&stale);
        tally.add(file, &bytes, &whole.value.rocket, &layout, false, &[]);
        assert!(
            tally
                .failure()
                .unwrap()
                .contains("no structure or no parts")
        );

        let mut tally = MassTally::of(&json!({ "probe": record()["probe"], "designs": [] }));
        tally.add(file, &bytes, &whole.value.rocket, &layout, false, &[]);
        assert!(tally.failure().unwrap().contains("is not in"));

        let mut swapped = record();
        swapped["probe"]["openrocket"]["ixx"] = swapped["probe"]["openrocket"]["iyy"].clone();
        assert!(!probe_mismatches(&swapped["probe"]).is_empty());

        assert_eq!(causes(&[], false, false), Vec::<&str>::new());
        let tally = MassTally {
            record: Some(BTreeMap::new()),
            outside: vec![Outside {
                label: "a design".to_owned(),
                hash: String::new(),
                found: [0.5, 0.0, 0.0, 0.0],
                causes: Vec::new(),
                parts: json!([]),
            }],
            ..MassTally::default()
        };
        assert!(tally.failure().unwrap().contains("no cause hpr warned of"));
        assert_eq!(
            causes(
                &["a cluster of motor tubes is read as the one tube, so its mass is too"],
                false,
                false
            ),
            [CAUSES[0]]
        );
    }
}
