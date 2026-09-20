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

use std::collections::BTreeMap;
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use hpr_io::ork::{self, WarningKind};
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

/// Files in the reference library that are not well-formed XML, so no reader can open them.
///
/// Both are hand-written fixtures for Loft's browser tests, and both close a `<databranch>` with
/// `</flightdata>`. Python's expat refuses them at the same line, which is the second opinion this
/// list rests on. They are counted apart rather than skipped: if one of them ever reads, or fails
/// for some other reason, this command says so, because then the list has gone stale.
const NOT_WELL_FORMED: [(&str, &str); 2] = [
    (
        "refs/fusionspace-loft/e2e/fixtures/logged-sample.ork",
        "a <databranch> closed with </flightdata> (line 150)",
    ),
    (
        "refs/fusionspace-loft/e2e/fixtures/fallback-canopy-cd.ork",
        "a <databranch> closed with </flightdata> (line 153)",
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

/// One file to read: how to name it, and its bytes.
type Case = (String, Vec<u8>);

fn report(root: &Path, files: &[Case]) -> Result<(), String> {
    let mut containers: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut versions: BTreeMap<String, usize> = BTreeMap::new();
    let mut creators: BTreeMap<String, usize> = BTreeMap::new();
    let mut kinds: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut messages: BTreeMap<String, usize> = BTreeMap::new();
    let mut attachments: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = 0usize;
    let mut not_round_tripped = 0usize;
    let mut known_bad = 0usize;
    let mut stale = Vec::new();
    let mut detail = Vec::new();

    for (name, bytes) in files {
        let excuse = NOT_WELL_FORMED
            .iter()
            .find(|(path, _)| name.replace('\\', "/").ends_with(path))
            .map(|(_, reason)| *reason);
        match ork::read(bytes) {
            Ok(read) => {
                if let Some(reason) = excuse {
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
                    "container": read.value.container.as_str(),
                    "version": read.value.document.version.to_string(),
                    "creator": read.value.document.creator,
                    "design_entry": read.value.design_entry,
                    "elements": count_elements(&read.value.document.root),
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
            Err(error) => match excuse {
                Some(reason) if matches!(error, ork::OrkError::Xml { .. }) => {
                    known_bad += 1;
                    detail.push(json!({
                        "file": name,
                        "not_well_formed": reason,
                        "error": error.to_string(),
                    }));
                }
                Some(reason) => {
                    stale.push(format!(
                        "{name} is listed as {reason} but failed for another reason: {error}"
                    ));
                    failures += 1;
                    detail.push(json!({ "file": name, "error": error.to_string() }));
                }
                None => {
                    failures += 1;
                    detail.push(json!({ "file": name, "error": error.to_string() }));
                }
            },
        }
    }

    let summary = json!({
        "files": files.len(),
        "read": files.len() - failures,
        "failed": failures,
        "not_well_formed_xml": known_bad,
        "not_round_tripped": not_round_tripped,
        "containers": to_value(&containers),
        "versions": to_value(&versions),
        "creators": to_value(&creators),
        "warnings": to_value(&kinds),
        "warning_messages": to_value(&messages),
        "attachment_kinds": to_value(&attachments),
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
