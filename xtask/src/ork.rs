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

/// How `hpr_io::ork::rocket` begins the warning for a radius it gave OpenRocket's default radius;
/// a test below holds the two together, so a reworded warning cannot quietly count as none.
const DEFAULT_RADIUS: &str = "an automatic radius with no fixed radius anywhere along its chain";

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
    report(&root, &files)
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

fn report(root: &Path, files: &[Case]) -> Result<(), String> {
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
                let holds_design = read
                    .value
                    .document
                    .root
                    .child("rocket")
                    .and_then(|rocket| rocket.child("subcomponents"))
                    .is_some_and(|parts| parts.elements().next().is_some());
                if !holds_design {
                    no_design += 1;
                }
                let laid_out = match spine.value.layout() {
                    // A document with nothing in its `rocket` is not a design that failed; it is
                    // not a design, and is counted as such rather than as a failure.
                    Err(_) if !holds_design => Some("holds no design".to_owned()),
                    Ok(layout) => {
                        spines_laid_out += 1;
                        // A structural part that weighs nothing is almost always a reading gone
                        // wrong somewhere upstream, and it is silent by nature: the design lays
                        // out, the report is written, and the mass is simply missing.
                        for placed in &layout.components {
                            if placed.own.mass_kg == 0.0 {
                                *weightless.entry(placed.part.kind_name()).or_default() += 1;
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
    let summary = json!({
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
        "designs": read_count - no_design,
        "designs_laid_out": spines_laid_out,
        "documents_holding_no_design": no_design,
        "radii_given_openrocket_default": to_value(&defaulted),
        "designs_with_radii_given_openrocket_default": designs_defaulted,
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
        "  documents that hold no design (a `rocket` with no stage or component of any kind): \
         {no_design}"
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

    if !stale.is_empty() {
        return Err(format!(
            "the list of files that are not well-formed XML is out of date:\n  {}",
            stale.join("\n  ")
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

    /// The survey counts the radii given OpenRocket's default by the start of their warning; this
    /// holds that start to what the importer actually says.
    #[test]
    fn default_radius_warning_is_the_one_counted() {
        let xml = br#"<openrocket version="1.10" creator="test"><rocket><name>R</name>
            <subcomponents><stage><name>S</name><subcomponents>
            <bodytube><name>T</name><length>0.3</length><thickness>0.001</thickness>
            <radius>auto</radius></bodytube>
            </subcomponents></stage></subcomponents></rocket></openrocket>"#;
        let read = ork::read(xml).unwrap();
        let spine = ork::rocket(&read.value.document);
        let counted = spine
            .warnings
            .iter()
            .filter(|warning| warning.message.starts_with(DEFAULT_RADIUS))
            .count();
        assert_eq!(counted, 1, "{:?}", spine.warnings);
    }
}
