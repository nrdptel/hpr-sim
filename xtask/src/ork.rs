//! `cargo xtask ork [--dir <path>]…`: reads every `.ork` file in the reference library and
//! reports what came back.
//!
//! The corpus is private (`refs/loft-fixtures` holds other people's designs, CLAUDE.md rule 4), so
//! the per-file detail — names, versions, warnings, errors — goes to `corpus-out/ork-survey.json`,
//! which is gitignored. What this prints is counts: how many files read, how they were packaged,
//! which schema versions they claim, and how many warnings of each kind they raised. Those counts
//! are the part that may be published.
//!
//! With no `--dir`, it reads every `.ork` under `refs/` and the example designs inside the pinned
//! OpenRocket jar (`datafiles/examples/`), which is a zip like any other. It fails when any file
//! cannot be read, which is how milestone M3.1a's "every `.ork` opens" is checked.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use hpr_design::tree::{Component, Part, PlacedComponent};
use hpr_io::ork::{self, ATTACHED_TAGS, AXIAL_OFFSET, Dimension, INSTANCE_COUNT, WarningKind};
use serde_json::{Value, json};

pub const USAGE: &str = "\
  ork [--dir <path>]…      Read every .ork file in the reference library and print how many
                           opened, in which container and schema version. Per-file detail
                           (private corpus) goes to corpus-out/ork-survey.json.";

/// Where the per-file detail goes. Gitignored: it names files in a private corpus.
const REPORT: &str = "corpus-out/ork-survey.json";

/// The pinned OpenRocket jar, whose `datafiles/examples/` entries are designs in their own right.
const JAR: &str = "refs/openrocket/OpenRocket-24.12.jar";

/// The directories read when none are given.
const DEFAULT_DIRS: [&str; 1] = ["refs"];

/// What OpenRocket 24.12 resolved for every body radius of the files its probe ran
/// (`validation/oracles/openrocket/automatic_radius.py --library`, ADR-054).
const OPENROCKET_RADII: &str = "validation/fixtures/ork/openrocket-automatic-radius.json";

/// How `hpr_io::ork::rocket` begins the warning for a radius it gave OpenRocket's default radius;
/// a test below holds the two together, so a reworded warning cannot quietly count as none.
const DEFAULT_RADIUS: &str = "an automatic radius with no fixed radius anywhere along its chain";

/// How `hpr_io::ork::rocket` ends the warning for a document with no design in it, whether it has
/// no `<rocket>` or one holding nothing; held to the importer by the same test.
const NO_DESIGN: &str = "so the document holds no design";

/// Files in the reference library that are not well-formed XML, so no reader can open them.
///
/// Both are hand-written fixtures for Loft's browser tests, and both close a `<databranch>` with
/// `</flightdata>`. Python's expat refuses them at the same lines, which is the second opinion
/// this list rests on. They are counted apart rather than skipped, and each row carries the error
/// it must still give: if one of them ever reads, or fails differently, this command says so,
/// because then the list has gone stale.
const NOT_WELL_FORMED: [(&str, &str, &str); 2] = [
    (
        "refs/fusionspace-loft/e2e/fixtures/logged-sample.ork",
        "a <databranch> closed with </flightdata> (line 150)",
        "expected 'databranch' tag, not 'flightdata' at 150:",
    ),
    (
        "refs/fusionspace-loft/e2e/fixtures/fallback-canopy-cd.ork",
        "a <databranch> closed with </flightdata> (line 153)",
        "expected 'databranch' tag, not 'flightdata' at 153:",
    ),
];

pub fn run(args: &[String]) -> Result<(), String> {
    let root = root()?;
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut rest = args.iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--dir" => dirs.push(PathBuf::from(
                rest.next().ok_or_else(|| format!("usage:\n{USAGE}"))?,
            )),
            _ => return Err(format!("usage:\n{USAGE}")),
        }
    }

    let mut files = Vec::new();
    if dirs.is_empty() {
        for dir in DEFAULT_DIRS {
            collect(&root.join(dir), &mut files)?;
        }
        let jar = root.join(JAR);
        if jar.is_file() {
            files.extend(examples_in_jar(&jar)?);
        }
    } else {
        for dir in &dirs {
            collect(dir, &mut files)?;
        }
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    if files.is_empty() {
        return Err(
            "no .ork files found: the reference library is fetched by `cargo xtask refs fetch`"
                .to_owned(),
        );
    }
    // The OpenRocket fixture names designs in the library; a run over other directories need not
    // reach any of them.
    let library = dirs.is_empty() && root.join(JAR).is_file();
    report(&root, &files, library)
}

/// The tags that hold an angle. A `.ork` writes them in degrees and says so nowhere, so this is
/// the measurement that settles it: an angle larger than 2π is more than a whole turn, which no
/// component is written at.
const ANGLE_TAGS: [&str; 5] = [
    "angleoffset",
    "rotation",
    "radialdirection",
    "cant",
    "clusterrotation",
];

/// Counts the angles written in a document: how many, how many are not zero, and how many of
/// those are larger than 2π.
fn angles(element: &ork::Element, counts: &mut [usize; 3]) {
    if ANGLE_TAGS.contains(&element.name.as_str())
        && let Ok(value) = element.text().trim().parse::<f64>()
        && value.is_finite()
    {
        counts[0] += 1;
        if value != 0.0 {
            counts[1] += 1;
            if value.abs() > std::f64::consts::TAU {
                counts[2] += 1;
            }
        }
    }
    for child in element.elements() {
        angles(child, counts);
    }
}

/// Counts every tag inside a `<subcomponents>` that no milestone reads yet.
fn off_the_spine(element: &ork::Element, counts: &mut BTreeMap<String, usize>) {
    const ON_THE_SPINE: [&str; 4] = ["stage", "nosecone", "bodytube", "transition"];
    if element.name == "subcomponents" {
        for child in element.elements() {
            if !ON_THE_SPINE.contains(&child.name.as_str())
                && !ATTACHED_TAGS.contains(&child.name.as_str())
            {
                *counts.entry(child.name.clone()).or_default() += 1;
            }
        }
    }
    for child in element.elements() {
        off_the_spine(child, counts);
    }
}

/// Counts a component and everything attached to it, by kind and by automatic dimension.
fn count_tree(
    component: &Component,
    parts: &mut BTreeMap<&'static str, usize>,
    autos: &mut BTreeMap<&'static str, usize>,
) -> usize {
    *parts.entry(component.part.kind_name()).or_default() += 1;
    for dimension in &component.auto {
        *autos.entry(dimension.name()).or_default() += 1;
    }
    1 + component
        .children
        .iter()
        .map(|child| count_tree(child, parts, autos))
        .sum::<usize>()
}

/// Every automatic dimension a file cached a number with, as `(component id, tag, cached)`.
///
/// This is the only oracle available offline for the resolution rules: `auto 0.025` is the answer
/// OpenRocket itself last worked out, so a reader that resolves the dimension from the neighbours
/// can be held to it. Only a component the file gives an `<id>` can be matched back, and only the
/// 104 of 413 automatic dimensions that cache anything say a number at all.
fn cached_dimensions(element: &ork::Element, found: &mut Vec<(String, String, f64)>) {
    let id = element.child("id").map(|id| id.text().trim().to_owned());
    if let Some(id) = id.filter(|id| !id.is_empty()) {
        for child in element.elements() {
            if let Some(Dimension::Automatic {
                cached: Some(value),
            }) = Dimension::parse(&child.text())
            {
                found.push((id.clone(), child.name.clone(), value));
            }
        }
    }
    for child in element.elements() {
        cached_dimensions(child, found);
    }
}

/// What the layout resolved the dimension `tag` to on the placed component `placed`.
fn resolved_dimension(placed: &PlacedComponent, tag: &str) -> Option<f64> {
    Some(match (tag, &placed.part) {
        ("aftradius", Part::NoseCone(nose)) => nose.base_radius_m,
        ("aftradius", Part::Transition(t)) => t.aft_radius_m,
        ("foreradius", Part::Transition(t)) => t.fore_radius_m,
        ("aftshoulderradius", Part::NoseCone(nose)) => nose.shoulder.as_ref()?.outer_radius_m,
        ("aftshoulderradius", Part::Transition(t)) => t.aft_shoulder.as_ref()?.outer_radius_m,
        ("foreshoulderradius", Part::Transition(t)) => t.fore_shoulder.as_ref()?.outer_radius_m,
        ("radius", Part::BodyTube(tube)) => tube.outer_radius_m,
        ("radius", Part::TubeFinSet(fins)) => fins.outer_radius_m,
        ("radius", Part::LaunchLug(lug)) => lug.outer_radius_m,
        ("outerradius", Part::InnerTube(tube)) => tube.outer_radius_m,
        ("outerradius", Part::CenteringRing(ring)) => ring.outer_radius_m,
        ("innerradius", Part::CenteringRing(ring)) => ring.inner_radius_m,
        ("packedradius", Part::MassComponent(p)) => p.packing.radius_m,
        ("packedradius", Part::Parachute(p)) => p.packing.radius_m,
        ("packedradius", Part::Streamer(p)) => p.packing.radius_m,
        ("packedradius", Part::ShockCord(p)) => p.packing.radius_m,
        _ => return None,
    })
}

/// One file to read: how to name it, and its bytes.
type Case = (String, Vec<u8>);

fn report(root: &Path, files: &[Case], library: bool) -> Result<(), String> {
    let openrocket = openrocket_radii(root, &root.join(OPENROCKET_RADII))?;
    let mut radii_designs = 0usize;
    let mut radii_compared = 0usize;
    let mut radii_agreeing = 0usize;
    let mut radii_apart: Vec<Value> = Vec::new();
    let mut motor_tally = crate::ork_motors::MotorTally::default();
    let mut recovery_tally = crate::ork_recovery::RecoveryTally::default();
    let mut simulation_tally = crate::ork_simulations::SimulationTally::default();
    let mut extension_tally = crate::ork_extensions::ExtensionTally::default();
    let mut geometry = crate::ork_geometry::GeometryTally::load(root, library)?;
    let mut mass = crate::ork_mass::MassTally::load(root, library)?;
    // Per source (a directory under `refs/`, or the jar): files, read, laid out, holding no
    // design, and errors (not read, or read but not laid out).
    let mut sources: BTreeMap<String, [usize; 5]> = BTreeMap::new();
    let mut containers: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut versions: BTreeMap<String, usize> = BTreeMap::new();
    let mut creators: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut messages: BTreeMap<String, usize> = BTreeMap::new();
    let mut attachments: BTreeMap<String, usize> = BTreeMap::new();
    let mut depths: BTreeMap<usize, usize> = BTreeMap::new();
    let mut mixed: BTreeMap<String, usize> = BTreeMap::new();
    let mut files_with_mixed = 0usize;
    let mut largest_unpacked = 0usize;
    let mut automatic: BTreeMap<String, usize> = BTreeMap::new();
    let mut overrides: BTreeMap<String, usize> = BTreeMap::new();
    let mut both_names: BTreeMap<String, [usize; 3]> = BTreeMap::new();
    let mut tag_totals: BTreeMap<String, usize> = BTreeMap::new();
    let mut elements_with_both = 0usize;
    let mut stages_read = 0usize;
    let mut body_parts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut attached_parts: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut auto_marked: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut left_out: BTreeMap<String, usize> = BTreeMap::new();
    let mut spine_warning_kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut cached_checked = 0usize;
    let mut cached_agree = 0usize;
    let mut cached_by_tag: BTreeMap<String, [usize; 2]> = BTreeMap::new();
    let mut cached_unmatched = 0usize;
    let mut cached_apart: BTreeMap<String, usize> = BTreeMap::new();
    let mut angle_counts = [0usize; 3];
    let mut weightless: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut apart_detail: Vec<Value> = Vec::new();
    let mut off_spine: BTreeMap<String, usize> = BTreeMap::new();
    let mut spines_laid_out = 0usize;
    let mut no_design = 0usize;
    let mut defaulted: BTreeMap<String, usize> = BTreeMap::new();
    let mut designs_defaulted = 0usize;
    let mut spine_errors: BTreeMap<String, usize> = BTreeMap::new();
    let mut spine_warnings: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = 0usize;
    let mut not_round_tripped = 0usize;
    let mut known_bad = 0usize;
    let mut stale = Vec::new();
    let mut detail = Vec::new();

    for (name, bytes) in files {
        let slashed = name.replace('\\', "/");
        let source = sources.entry(source_of(root, name)).or_default();
        source[0] += 1;
        let excuse = NOT_WELL_FORMED
            .iter()
            .find(|(path, _, _)| {
                slashed
                    .strip_suffix(path)
                    .is_some_and(|before| before.is_empty() || before.ends_with('/'))
            })
            .map(|(_, reason, expected)| (*reason, *expected));
        match ork::read(bytes) {
            Ok(read) => {
                source[1] += 1;
                if let Some((reason, _)) = excuse {
                    stale.push(format!("{name} reads, though it is listed as {reason}"));
                }
                *containers.entry(read.value.container.as_str()).or_default() += 1;
                *versions
                    .entry(read.value.document.version.to_string())
                    .or_default() += 1;
                *creators
                    .entry(
                        read.value
                            .document
                            .creator
                            .clone()
                            .unwrap_or_else(|| "(none)".to_owned()),
                    )
                    .or_default() += 1;
                for warning in &read.warnings {
                    *kinds.entry(kind_name(warning.kind)).or_default() += 1;
                    *messages.entry(warning.message.clone()).or_default() += 1;
                }
                for attachment in &read.value.attachments {
                    *attachments.entry(extension(&attachment.name)).or_default() += 1;
                }
                walk(
                    &read.value.document.root,
                    &mut automatic,
                    &mut overrides,
                    &mut both_names,
                    &mut tag_totals,
                    &mut elements_with_both,
                );
                let spine = ork::rocket(&read.value.document);
                // The whole design, motors and all, whose first warnings are the spine's.
                let whole = ork::design(&read.value);
                let motors_here = motor_tally.add(&whole, spine.warnings.len());
                let recovery_here = recovery_tally.add(&whole);
                let simulations_here = simulation_tally.add(&whole);
                let extensions_here = extension_tally.add(&whole, &read.value.document);
                let mut defaulted_here = 0usize;
                for warning in &spine.warnings {
                    if warning.message.starts_with(DEFAULT_RADIUS) {
                        defaulted_here += 1;
                        let step = warning.at.rsplit('/').next().unwrap_or("?");
                        let tag = step.split_once('[').map_or(step, |(tag, _)| tag);
                        *defaulted.entry(tag.to_owned()).or_default() += 1;
                    }
                    *spine_warnings.entry(warning.message.clone()).or_default() += 1;
                    *spine_warning_kinds
                        .entry(kind_name(warning.kind))
                        .or_default() += 1;
                    if warning.kind == WarningKind::Skipped {
                        // The path's last step is `trapezoidfinset[3]`: the tag, and which one.
                        let step = warning.at.rsplit('/').next().unwrap_or("?");
                        let tag = step.split_once('[').map_or(step, |(tag, _)| tag);
                        if ATTACHED_TAGS.contains(&tag) {
                            *left_out.entry(tag.to_owned()).or_default() += 1;
                        }
                    }
                }
                stages_read += spine.value.stages.len();
                let mut components = 0usize;
                let mut attached = 0usize;
                for stage in &spine.value.stages {
                    for component in &stage.components {
                        components += 1;
                        *body_parts.entry(component.part.kind_name()).or_default() += 1;
                        for dimension in &component.auto {
                            *auto_marked.entry(dimension.name()).or_default() += 1;
                        }
                        for child in &component.children {
                            attached += count_tree(child, &mut attached_parts, &mut auto_marked);
                        }
                    }
                }
                off_the_spine(&read.value.document.root, &mut off_spine);
                angles(&read.value.document.root, &mut angle_counts);
                if defaulted_here > 0 {
                    designs_defaulted += 1;
                }
                let holds_design = !spine
                    .warnings
                    .iter()
                    .any(|warning| warning.message.ends_with(NO_DESIGN));
                if !holds_design {
                    no_design += 1;
                }
                let mut geometry_here = Value::Null;
                let mut mass_here = Value::Null;
                let laid_out = match spine.value.layout() {
                    // A document with nothing in its `rocket` is not a design that failed; it is
                    // not a design, and is counted as such rather than as a failure.
                    Err(_) if !holds_design => {
                        source[3] += 1;
                        Some("holds no design".to_owned())
                    }
                    Ok(layout) => {
                        spines_laid_out += 1;
                        source[2] += 1;
                        geometry_here =
                            geometry.add(&oracle_key(root, name), bytes, &spine.value, &layout);
                        let said: Vec<&str> = spine
                            .warnings
                            .iter()
                            .map(|warning| warning.message.as_str())
                            .collect();
                        mass_here = mass.add(
                            &oracle_key(root, name),
                            bytes,
                            &spine.value,
                            &layout,
                            whole.value.is_reduced(),
                            &said,
                        );
                        // A structural part that weighs nothing is almost always a reading gone
                        // wrong somewhere upstream, and it is silent by nature: the design lays
                        // out, the report is written, and the mass is simply missing.
                        for placed in &layout.components {
                            if placed.own.mass_kg == 0.0 {
                                *weightless.entry(placed.part.kind_name()).or_default() += 1;
                            }
                        }
                        // OpenRocket itself, run on the same file: every body radius it resolved,
                        // forward to aft, against the one hpr resolved.
                        if let Some(theirs) = openrocket.get(&oracle_key(root, name)) {
                            let ours: Vec<f64> = layout
                                .body()
                                .flat_map(|placed| body_radii(&placed.part))
                                .collect();
                            radii_designs += 1;
                            if ours.len() == theirs.len() {
                                for (index, (ours, theirs)) in ours.iter().zip(theirs).enumerate() {
                                    radii_compared += 1;
                                    if (ours - theirs).abs() <= 1e-9 * theirs.abs().max(1e-6) {
                                        radii_agreeing += 1;
                                    } else {
                                        radii_apart.push(json!({
                                            "file": oracle_key(root, name),
                                            "radius": index,
                                            "hpr": ours,
                                            "openrocket": theirs,
                                        }));
                                    }
                                }
                            } else {
                                radii_apart.push(json!({
                                    "file": oracle_key(root, name),
                                    "hpr_radii": ours.len(),
                                    "openrocket_radii": theirs.len(),
                                }));
                            }
                        }
                        // Every automatic dimension the file cached a number with is an answer
                        // OpenRocket worked out; hold the resolution to it.
                        let mut cached = Vec::new();
                        cached_dimensions(&read.value.document.root, &mut cached);
                        for (id, tag, value) in cached {
                            let seen = cached_by_tag.entry(tag.clone()).or_insert([0, 0]);
                            seen[0] += 1;
                            let Some((_, placed)) = layout.find(&id) else {
                                cached_unmatched += 1;
                                continue;
                            };
                            let Some(resolved) = resolved_dimension(placed, &tag) else {
                                cached_unmatched += 1;
                                continue;
                            };
                            cached_by_tag.entry(tag.clone()).or_insert([0, 0])[1] += 1;
                            cached_checked += 1;
                            let apart = (resolved - value).abs();
                            if apart <= 1e-9 * value.abs().max(1e-6) {
                                cached_agree += 1;
                            } else {
                                *cached_apart.entry(tag.clone()).or_default() += 1;
                                apart_detail.push(json!({
                                    "file": name,
                                    "id": id,
                                    "tag": tag,
                                    "cached": value,
                                    "resolved": resolved,
                                }));
                            }
                        }
                        None
                    }
                    Err(error) => {
                        source[4] += 1;
                        geometry.not_laid_out(&oracle_key(root, name));
                        mass.not_laid_out(&oracle_key(root, name));
                        let text = error.to_string();
                        *spine_errors.entry(text.clone()).or_default() += 1;
                        Some(text)
                    }
                };
                let unpacked = read.value.document.to_xml().len()
                    + read
                        .value
                        .attachments
                        .iter()
                        .map(|attachment| attachment.bytes.len())
                        .sum::<usize>();
                largest_unpacked = largest_unpacked.max(unpacked);
                let depth = depth_of(&read.value.document.root);
                *depths.entry(depth).or_default() += 1;
                let mut here = Vec::new();
                mixed_content(&read.value.document.root, "", &mut here);
                if !here.is_empty() {
                    files_with_mixed += 1;
                }
                for at in &here {
                    *mixed.entry(at.clone()).or_default() += 1;
                }
                let written = read.value.document.to_xml();
                let round_trip = match ork::Document::parse(&written) {
                    Ok(again) => again.value == read.value.document,
                    Err(_) => false,
                };
                if !round_trip {
                    not_round_tripped += 1;
                }
                detail.push(json!({
                    "file": name,
                    "round_trip": round_trip,
                    "spine": json!({
                        "holds_design": holds_design,
                        "radii_given_openrocket_default": defaulted_here,
                        "stages": spine.value.stages.len(),
                        "body_components": components,
                        "attached_parts": attached,
                        "laid_out": laid_out.is_none(),
                        "error": laid_out,
                        "warnings": spine.warnings.iter()
                            .map(|warning| json!({ "at": warning.at, "says": warning.message }))
                            .collect::<Vec<_>>(),
                    }),
                    "motors": motors_here,
                    "recovery": recovery_here,
                    "simulations": simulations_here,
                    "extensions": extensions_here,
                    "rocketserializer": geometry_here,
                    "openrocket_mass": mass_here,
                    "container": read.value.container.as_str(),
                    "version": read.value.document.version.to_string(),
                    "creator": read.value.document.creator,
                    "design_entry": read.value.design_entry,
                    "elements": count_elements(&read.value.document.root),
                    "depth": depth,
                    "unpacked_bytes": unpacked,
                    "mixed_content": here,
                    "attachments": read.value.attachments.iter()
                        .map(|attachment| json!({
                            "name": attachment.name,
                            "bytes": attachment.bytes.len(),
                        }))
                        .collect::<Vec<_>>(),
                    "warnings": read.warnings.iter()
                        .map(|warning| json!({
                            "at": warning.at,
                            "kind": kind_name(warning.kind),
                            "message": warning.message,
                        }))
                        .collect::<Vec<_>>(),
                }));
            }
            Err(error) => {
                source[4] += 1;
                let text = error.to_string();
                match excuse {
                    // Not merely "it failed with an XML error", which a later size or node limit
                    // could also raise: the error it is listed for, word for word.
                    Some((reason, expected)) if text.contains(expected) => {
                        known_bad += 1;
                        detail.push(json!({
                            "file": name,
                            "not_well_formed": reason,
                            "error": text,
                        }));
                    }
                    Some((reason, expected)) => {
                        stale.push(format!(
                            "{name} is listed as {reason}, whose error says `{expected}`, but it \
                             failed with: {text}"
                        ));
                        failures += 1;
                        detail.push(json!({ "file": name, "error": text }));
                    }
                    None => {
                        failures += 1;
                        detail.push(json!({ "file": name, "error": text }));
                    }
                }
            }
        }
    }

    let read_count = files.len() - failures - known_bad;
    let openrocket_body_radii = json!({
        "designs": radii_designs,
        "compared": radii_compared,
        "agreeing": radii_agreeing,
        "apart": radii_apart.clone(),
    });
    let mut summary = json!({
        "files": files.len(),
        "read": files.len() - failures - known_bad,
        "failed": failures,
        "not_well_formed_xml": known_bad,
        "not_round_tripped": not_round_tripped,
        "containers": to_value(&containers),
        "versions": to_value(&versions),
        "creators": to_value(&creators),
        "warnings": to_value(&kinds),
        "warning_messages": to_value(&messages),
        "attachment_kinds": to_value(&attachments),
        "depths": to_value(&depths),
        "mixed_content_elements": to_value(&mixed),
        "files_with_mixed_content": files_with_mixed,
        "largest_unpacked_bytes": largest_unpacked,
        "attached_parts": to_value(&attached_parts),
        "parts_left_out": to_value(&left_out),
        "design_warnings": to_value(&spine_warning_kinds),
        "parts_weighing_nothing": to_value(&weightless),
        "angles_written": angle_counts[0],
        "angles_not_zero": angle_counts[1],
        "angles_larger_than_tau": angle_counts[2],
        "cached_dimensions_checked": cached_checked,
        "cached_dimensions_unmatched": cached_unmatched,
        "cached_dimensions_by_tag": Value::Object(
            cached_by_tag
                .iter()
                .map(|(tag, [seen, checked])| {
                    (tag.clone(), json!({ "cached": seen, "checked": checked }))
                })
                .collect(),
        ),
        "cached_dimensions_agreeing": cached_agree,
        "cached_dimensions_apart": Value::Array(apart_detail.clone()),
        "automatic_dimensions": to_value(&automatic),
        "override_tags": to_value(&overrides),
        "elements_with_both_names": elements_with_both,
        "renamed_tag_totals": to_value(&tag_totals),
        "both_names": Value::Object(
            both_names
                .iter()
                .map(|(pair, [seen, text, frames])| {
                    (
                        pair.clone(),
                        json!({
                            "elements": seen,
                            "agreeing_on_text": text,
                            "differing_on_frame": frames,
                        }),
                    )
                })
                .collect(),
        ),
    });
    // Added one by one: a single `json!` this size passes the macro's recursion limit.
    summary["designs"] = json!(read_count - no_design);
    summary["designs_laid_out"] = json!(spines_laid_out);
    summary["documents_holding_no_design"] = json!(no_design);
    summary["radii_given_openrocket_default"] = to_value(&defaulted);
    summary["designs_with_radii_given_openrocket_default"] = json!(designs_defaulted);
    summary["openrocket_body_radii"] = openrocket_body_radii;
    summary["openrocket_mass_outside"] = mass.outside();
    summary["motors"] = motor_tally.summary();
    summary["recovery"] = recovery_tally.summary();
    summary["simulations"] = simulation_tally.summary();
    summary["extensions"] = extension_tally.summary();
    let path = root.join(REPORT);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let document = json!({ "summary": summary, "files": detail });
    fs::write(
        &path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&document).map_err(|error| error.to_string())?
        ),
    )
    .map_err(|error| format!("{}: {error}", path.display()))?;

    let read = files.len() - failures - known_bad;
    println!(
        "{} .ork files, {read} read, {} written and read back unchanged, {known_bad} not \
         well-formed XML, {failures} failed",
        files.len(),
        read - not_round_tripped
    );
    print_counts("containers", &containers);
    print_counts("schema versions", &versions);
    print_counts("warnings", &kinds);
    print_counts("attachments by extension", &attachments);
    print_counts("nesting depth", &depths);
    println!("  largest unpacked (document and attachments): {largest_unpacked} bytes");
    println!(
        "  designs: {spines_laid_out} of {} lay out, {stages_read} stage(s), \
         {} body component(s), {} attached part(s)",
        read - no_design,
        body_parts.values().sum::<usize>(),
        attached_parts.values().sum::<usize>()
    );
    println!(
        "  documents that hold no design (no `rocket`, or one with nothing in it): {no_design}"
    );
    // An import error is a file that does not read, or a design that reads but does not lay out;
    // warnings are not errors (they never stop an import).
    println!(
        "  imports by source, as files: read, laid out, holding no design, errors: {}",
        sources
            .iter()
            .map(|(source, [files, read, laid, empty, errors])| format!(
                "{source} {files}: {read}, {laid}, {empty}, {errors}"
            ))
            .collect::<Vec<_>>()
            .join("; ")
    );
    println!(
        "  automatic radii with no fixed radius along their chain, given OpenRocket's default \
         {} mm: {} in {designs_defaulted} design(s), on {}",
        ork::OPENROCKET_DEFAULT_RADIUS_M * 1e3,
        defaulted.values().sum::<usize>(),
        if defaulted.is_empty() {
            "nothing".to_owned()
        } else {
            defaulted
                .iter()
                .map(|(tag, count)| format!("{tag} {count}"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    );
    print_counts("body components", &body_parts);
    print_counts("attached parts", &attached_parts);
    print_counts("automatic dimensions marked", &auto_marked);
    print_counts("parts left out, by tag", &left_out);
    print_counts("parts that weigh nothing", &weightless);
    print_counts("warnings reading designs", &spine_warning_kinds);
    println!(
        "  angles: {} written, {} not zero, {} of those larger than 2π",
        angle_counts[0], angle_counts[1], angle_counts[2]
    );
    let by_tag = cached_by_tag
        .iter()
        .map(|(tag, [seen, checked])| format!("{tag} {checked}/{seen}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "  automatic dimensions against OpenRocket's cached answer: {cached_agree} of \
         {cached_checked} agree ({cached_unmatched} cached but not comparable)"
    );
    println!("  by tag, compared of cached: {by_tag}");
    println!(
        "  body radii against OpenRocket 24.12 run on the same file ({OPENROCKET_RADII}): \
         {radii_agreeing} of {radii_compared} agree, over {radii_designs} design(s){}",
        if radii_apart.is_empty() {
            String::new()
        } else {
            format!(
                "; apart: {}",
                radii_apart
                    .iter()
                    .map(
                        |apart| match (apart["hpr"].as_f64(), apart["openrocket"].as_f64()) {
                            (Some(ours), Some(theirs)) => format!(
                                "{} radius {}: hpr {ours} m, OpenRocket {theirs} m",
                                apart["file"].as_str().unwrap_or("?"),
                                apart["radius"]
                            ),
                            _ => format!(
                                "{}: {} radii in hpr, {} in OpenRocket",
                                apart["file"].as_str().unwrap_or("?"),
                                apart["hpr_radii"],
                                apart["openrocket_radii"]
                            ),
                        }
                    )
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        }
    );
    // The rules with no cached answer anywhere have no oracle at all, and that is worth saying.
    let never_cached: Vec<&str> = automatic
        .keys()
        .filter(|tag| !cached_by_tag.contains_key(*tag))
        .map(String::as_str)
        .collect();
    if !never_cached.is_empty() {
        println!(
            "  automatic dimensions that never cache a number, so no oracle reaches them: {}",
            never_cached.join(", ")
        );
    }
    if !cached_apart.is_empty() {
        print_counts("cached answers hpr resolves differently", &cached_apart);
    }
    print_counts("tags no milestone reads yet", &off_spine);
    motor_tally.print();
    recovery_tally.print();
    simulation_tally.print();
    extension_tally.print();
    geometry.print();
    mass.print();
    if !spine_errors.is_empty() {
        print_counts("designs that do not lay out", &spine_errors);
    }
    print_counts("automatic dimensions", &automatic);
    print_counts("override tags", &overrides);
    let names = both_names
        .iter()
        .map(|(pair, [seen, text, frames])| {
            format!("{pair} {seen} ({text} agree on text, {frames} differ on frame)")
        })
        .collect::<Vec<_>>()
        .join(", ");
    println!("  elements carrying both names of a pair: {elements_with_both}");
    println!("  by pair: {names}");
    print_counts("elements carrying each renamed tag", &tag_totals);
    println!(
        "  text beside child elements: {} element(s) in {files_with_mixed} file(s){}",
        mixed.values().sum::<usize>(),
        if mixed.is_empty() {
            String::new()
        } else {
            format!(
                ", at {}",
                mixed.keys().cloned().collect::<Vec<_>>().join(", ")
            )
        }
    );
    println!("per-file detail (names and all): {REPORT}");

    if let Some(failure) = motor_tally.failure() {
        return Err(failure);
    }
    if let Some(failure) = extension_tally.failure() {
        return Err(failure);
    }
    if let Some(failure) = geometry.failure() {
        return Err(failure);
    }
    if let Some(failure) = mass.failure() {
        return Err(failure);
    }
    // M3.1's first *done when*: the private design library and the jar's examples import with no
    // error at all. Held, not only printed.
    for source in [LIBRARY_SOURCE, EXAMPLES_SOURCE] {
        if let Some([_, _, _, _, errors]) = sources.get(source)
            && *errors > 0
        {
            return Err(format!(
                "{errors} file(s) in {source} do not import: they do not read or do not lay out; \
                 see {REPORT}"
            ));
        }
    }
    if !stale.is_empty() {
        return Err(format!(
            "the list of files that are not well-formed XML is out of date:\n  {}",
            stale.join("\n  ")
        ));
    }
    // The radii are held to OpenRocket's, not only printed beside them: a disagreement fails the
    // survey, and so does a library run that reached fewer of the designs OpenRocket was run on
    // than it names, which is what a path that no longer matches the fixture's keys looks like.
    if !radii_apart.is_empty() {
        return Err(format!(
            "{} of OpenRocket's body radii are not hpr's on the same file; see {REPORT}",
            radii_apart.len()
        ));
    }
    if library && radii_designs != openrocket.len() {
        return Err(format!(
            "{OPENROCKET_RADII} holds OpenRocket's radii for {} designs, but {radii_designs} were \
             compared: a file is missing, does not lay out, or is filed under another key",
            openrocket.len()
        ));
    }
    match (failures, not_round_tripped) {
        (0, 0) => Ok(()),
        (0, lost) => Err(format!(
            "{lost} of {} .ork files did not survive being written and read again; see {REPORT}",
            files.len()
        )),
        (failed, _) => Err(format!(
            "{failed} of {} .ork files could not be read; see {REPORT}",
            files.len()
        )),
    }
}

/// A body component's radii, forward to aft, in the order the OpenRocket probe records them: a nose
/// cone's base, a tube's outer radius, a transition's forward then aft radius.
fn body_radii(part: &Part) -> Vec<f64> {
    match part {
        Part::NoseCone(p) => vec![p.base_radius_m],
        Part::BodyTube(p) => vec![p.outer_radius_m],
        Part::Transition(p) => vec![p.fore_radius_m, p.aft_radius_m],
        _ => Vec::new(),
    }
}

/// The key the OpenRocket probe files a design under: its path from the repository root, or its
/// entry in the jar, whichever machine ran it. A name already relative to the root, as the
/// fixture's are, is its own key.
fn oracle_key(root: &Path, name: &str) -> String {
    let name = name.replace('\\', "/");
    if let Some((_, entry)) = name.rsplit_once('!') {
        return entry.trim_start_matches('/').to_owned();
    }
    let root = root.display().to_string().replace('\\', "/");
    name.strip_prefix(root.trim_end_matches('/'))
        .and_then(|rest| rest.strip_prefix('/'))
        .map_or_else(|| name.clone(), str::to_owned)
}

/// The source the private design library's files are counted under.
const LIBRARY_SOURCE: &str = "refs/loft-fixtures";

/// The source the jar's example designs are counted under.
const EXAMPLES_SOURCE: &str = "OpenRocket 24.12 examples";

/// Where a file came from, for the import counts: the jar, or the directory under `refs/` (or the
/// directory given) that holds it.
fn source_of(root: &Path, name: &str) -> String {
    let name = name.replace('\\', "/");
    if name.contains('!') {
        return EXAMPLES_SOURCE.to_owned();
    }
    let relative = oracle_key(root, &name);
    let mut steps = relative.split('/');
    match (steps.next(), steps.next()) {
        (Some("refs"), Some(dir)) => format!("refs/{dir}"),
        // A directory outside the repository, given with `--dir`, is its own source.
        (Some(first), _) if !first.is_empty() => first.to_owned(),
        _ => relative
            .rsplit_once('/')
            .map_or_else(|| relative.clone(), |(dir, _)| dir.to_owned()),
    }
}

/// Every body radius OpenRocket resolved in each file its probe opened, by [`oracle_key`].
fn openrocket_radii(root: &Path, path: &Path) -> Result<BTreeMap<String, Vec<f64>>, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let fixture: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let mut radii = BTreeMap::new();
    for run in fixture["library"].as_array().into_iter().flatten() {
        let (Some(file), Some(resolved)) = (run["file"].as_str(), run["resolved"].as_array())
        else {
            continue;
        };
        let values = resolved
            .iter()
            .flat_map(|component| {
                ["base", "outer", "fore", "aft"]
                    .into_iter()
                    .filter_map(|key| component[key].as_f64())
                    .collect::<Vec<_>>()
            })
            .collect();
        radii.insert(oracle_key(root, file), values);
    }
    Ok(radii)
}

fn print_counts<K: std::fmt::Display>(title: &str, counts: &BTreeMap<K, usize>) {
    if counts.is_empty() {
        println!("  {title}: none");
        return;
    }
    let line = counts
        .iter()
        .map(|(key, count)| format!("{key} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("  {title}: {line}");
}

fn to_value<K: std::fmt::Display>(counts: &BTreeMap<K, usize>) -> Value {
    Value::Object(
        counts
            .iter()
            .map(|(key, count)| (key.to_string(), json!(count)))
            .collect(),
    )
}

fn kind_name(kind: WarningKind) -> &'static str {
    match kind {
        WarningKind::Skipped => "skipped",
        WarningKind::Dropped => "dropped",
        WarningKind::Unusual => "unusual",
        _ => "other",
    }
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy().to_ascii_lowercase()))
        .unwrap_or_else(|| "(none)".to_owned())
}

fn count_elements(element: &ork::Element) -> usize {
    1 + element.elements().map(count_elements).sum::<usize>()
}

/// The tag pairs OpenRocket writes a value under two names, newest first.
/// The two the reader takes either name of, and the two it deliberately does not: the newer name
/// of each of those carries a `method` the older never does, which the counts here measure.
const NAME_PAIRS: [[&str; 2]; 5] = [
    AXIAL_OFFSET,
    INSTANCE_COUNT,
    // The angle, under its newer name and the two older ones a fin set and everything else use.
    // The reader takes either (ADR-053) because these agree on the *number* every time, which is
    // narrower than agreeing on the frame as well and is all an angle needs.
    ["angleoffset", "rotation"],
    ["angleoffset", "radialdirection"],
    ["radiusoffset", "radialposition"],
];

/// Counts, over the whole tree, the things a component reader has to get right: dimensions
/// OpenRocket works out for itself, overrides, and values written under two names at once.
fn walk(
    element: &ork::Element,
    automatic: &mut BTreeMap<String, usize>,
    overrides: &mut BTreeMap<String, usize>,
    both_names: &mut BTreeMap<String, [usize; 3]>,
    tag_totals: &mut BTreeMap<String, usize>,
    elements_with_both: &mut usize,
) {
    for child in element.elements() {
        if child.name.starts_with("override") {
            *overrides.entry(child.name.clone()).or_default() += 1;
        }
        // Read the child itself, not the first child of that name: an element may carry the same
        // tag twice, and counting the first one twice would be a wrong count.
        if Dimension::parse(&child.text()).is_some_and(Dimension::is_automatic) {
            *automatic.entry(child.name.clone()).or_default() += 1;
        }
    }
    // Each renamed tag is counted once for the element that carries it, not once per pair it
    // belongs to: `angleoffset` is half of two pairs, and counting it twice would double it.
    for name in NAME_PAIRS.iter().flatten().collect::<BTreeSet<_>>() {
        if element.child(name).is_some() {
            *tag_totals.entry((*name).to_owned()).or_default() += 1;
        }
    }
    let mut carries_a_pair = false;
    for pair in NAME_PAIRS {
        let [modern_name, legacy_name] = pair;
        // Seeded at zero whether or not this element has either, so that a pair never written
        // together is a measured zero rather than a missing row.
        let counts = both_names.entry(pair.join("/")).or_insert([0, 0, 0]);
        let (Some(modern), Some(legacy)) = (element.child(modern_name), element.child(legacy_name))
        else {
            continue;
        };
        carries_a_pair = true;
        counts[0] += 1;
        if modern.text().trim() == legacy.text().trim() {
            counts[1] += 1;
        }
        // The text is only half of it: `position` carries a `type` and `axialoffset` a `method`,
        // and the same number measured from two places is two different places. Counting the
        // differences, not the matches, keeps a pair that carries no frame at all out of it.
        if frame(modern) != frame(legacy) {
            counts[2] += 1;
        }
    }
    if carries_a_pair {
        *elements_with_both += 1;
    }
    for child in element.elements() {
        walk(
            child,
            automatic,
            overrides,
            both_names,
            tag_totals,
            elements_with_both,
        );
    }
}

/// The frame a placement tag names: `type` on the older tag, `method` on the newer.
fn frame(element: &ork::Element) -> Option<&str> {
    element
        .attribute("method")
        .or_else(|| element.attribute("type"))
}

/// How deeply the tree nests, counting the root as one level.
fn depth_of(element: &ork::Element) -> usize {
    1 + element.elements().map(depth_of).max().unwrap_or(0)
}

/// Elements holding text beside child elements, and the name of each, deepest name last.
fn mixed_content(element: &ork::Element, at: &str, found: &mut Vec<String>) {
    let here = if at.is_empty() {
        element.name.clone()
    } else {
        format!("{at}/{}", element.name)
    };
    let has_elements = element.elements().next().is_some();
    if has_elements && !element.text().trim().is_empty() {
        found.push(here.clone());
    }
    for child in element.elements() {
        mixed_content(child, &here, found);
    }
}

/// Every `.ork` under `dir`, deepest last, with its path relative to the repository root.
fn collect(dir: &Path, files: &mut Vec<Case>) -> Result<(), String> {
    if !dir.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect(&path, files)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ork"))
        {
            let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            files.push((path.display().to_string(), bytes));
        }
    }
    Ok(())
}

/// The example designs inside the OpenRocket jar. The jar is a zip, and its
/// `datafiles/examples/*.ork` entries are designs OpenRocket ships; reading them is not reading
/// its source (CLAUDE.md rule 3).
fn examples_in_jar(jar: &Path) -> Result<Vec<Case>, String> {
    let bytes = fs::read(jar).map_err(|error| format!("{}: {error}", jar.display()))?;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|error| format!("{}: {error}", jar.display()))?;
    let mut examples = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("{}: {error}", jar.display()))?;
        let name = entry.name().to_owned();
        if !(name.starts_with("datafiles/examples/") && name.to_ascii_lowercase().ends_with(".ork"))
        {
            continue;
        }
        let mut content = Vec::new();
        entry
            .read_to_end(&mut content)
            .map_err(|error| format!("{name}: {error}"))?;
        examples.push((format!("{}!{name}", jar.display()), content));
    }
    Ok(examples)
}

fn root() -> Result<PathBuf, String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("no workspace root above {}", manifest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The survey counts the radii given OpenRocket's default, and the documents with no design,
    /// by the words of their warnings; this holds those words to what the importer actually says,
    /// and the default's warning to the tag it is about, which the survey tallies by.
    #[test]
    fn the_warnings_the_survey_counts_are_the_ones_raised() {
        let xml = br#"<openrocket version="1.10" creator="test"><rocket><name>R</name>
            <subcomponents><stage><name>S</name><subcomponents>
            <bodytube><name>T</name><length>0.3</length><thickness>0.001</thickness>
            <radius>auto</radius></bodytube>
            </subcomponents></stage></subcomponents></rocket></openrocket>"#;
        let read = ork::read(xml).unwrap();
        let spine = ork::rocket(&read.value.document);
        let counted: Vec<&str> = spine
            .warnings
            .iter()
            .filter(|warning| warning.message.starts_with(DEFAULT_RADIUS))
            .map(|warning| warning.at.as_str())
            .collect();
        assert_eq!(
            counted,
            ["openrocket/rocket/stage[0]/bodytube[0]"],
            "{:?}",
            spine.warnings
        );
        assert!(
            spine
                .warnings
                .iter()
                .all(|w| !w.message.ends_with(NO_DESIGN))
        );

        for xml in [
            &br#"<openrocket version="1.10" creator="test"><rocket><name>R</name></rocket>
                </openrocket>"#[..],
            &br#"<openrocket version="1.10" creator="test"></openrocket>"#[..],
        ] {
            let read = ork::read(xml).unwrap();
            let spine = ork::rocket(&read.value.document);
            let none = spine
                .warnings
                .iter()
                .filter(|warning| warning.message.ends_with(NO_DESIGN))
                .count();
            assert_eq!(none, 1, "{:?}", spine.warnings);
        }
    }
}
