//! The export counts `cargo xtask ork` prints (M3.2a): each design written back out as a `.ork`
//! (`hpr_io::ork::export`), read again, and compared with the design it was written from.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use hpr_io::ork::{self, Attachment, Design, Document, Imported, SuppliedCurves, export};
use serde_json::{Value, json};

/// The parts of a design compared.
const PARTS: [&str; 5] = ["rocket", "motors", "recovery", "simulations", "extensions"];

/// The counts, summed over the designs.
#[derive(Debug, Default)]
pub(crate) struct ExportTally {
    /// The workspace root, and where to save each written file, if anywhere.
    root: PathBuf,
    out: Option<PathBuf>,
    designs: usize,
    same: usize,
    /// Designs that read back the same but for what the reader had assumed: a radius with nothing
    /// to take given OpenRocket's default (ADR-054), which the export states.
    assumed: usize,
    fixed_points: usize,
    /// The parts of the design that came back different, by part.
    differing: BTreeMap<String, usize>,
    /// Export warnings, by message.
    warnings: BTreeMap<String, usize>,
    /// Tags the original document holds more of than the written one, by name.
    unwritten: BTreeMap<String, usize>,
    /// Tags the written document holds more of than the original, by name.
    added: BTreeMap<String, usize>,
    failed: Vec<String>,
}

impl ExportTally {
    /// A tally that saves each written file under `out`, by the original's path from `root`.
    pub(crate) fn new(root: &Path, out: Option<&Path>) -> Self {
        Self {
            root: root.to_path_buf(),
            out: out.map(Path::to_path_buf),
            ..Self::default()
        }
    }

    /// Writes `design` out with `attachments`, reads it back with `curves`, and compares; returns
    /// the per-file detail.
    pub(crate) fn add(
        &mut self,
        name: &str,
        design: &Imported<Design>,
        document: &Document,
        attachments: &[Attachment],
        curves: &SuppliedCurves,
    ) -> Value {
        self.designs += 1;
        let written = match export::write(&design.value, attachments) {
            Ok(written) => written,
            Err(error) => {
                self.failed.push(name.to_owned());
                return json!({ "error": error.to_string() });
            }
        };
        if let Some(out) = &self.out {
            let relative = Path::new(name)
                .strip_prefix(&self.root)
                .unwrap_or(Path::new(name));
            let path = out.join(relative.to_string_lossy().replace('!', "/"));
            let saved = path
                .parent()
                .map_or(Ok(()), fs::create_dir_all)
                .and_then(|()| fs::write(&path, &written.value));
            if saved.is_err() {
                self.failed.push(name.to_owned());
            }
        }
        for warning in &written.warnings {
            *self.warnings.entry(warning.message.clone()).or_default() += 1;
        }
        let Ok(file) = ork::read(&written.value) else {
            self.failed.push(name.to_owned());
            return json!({ "error": "the written file does not read" });
        };
        let back = ork::design_with(&file.value, curves).value;
        let (unwritten, added) = tag_differences(document, &file.value.document);
        for (tag, count) in &unwritten {
            *self.unwritten.entry(tag.clone()).or_default() += count;
        }
        for (tag, count) in &added {
            *self.added.entry(tag.clone()).or_default() += count;
        }
        let differences = differences(&design.value, &back);
        if differences.is_empty() {
            self.same += 1;
        } else if assumed_radius(design, &differences) {
            self.assumed += 1;
        }
        for (part, _) in &differences {
            *self.differing.entry(part.clone()).or_default() += 1;
        }
        // Written again from what it read back, the file is the same, byte for byte.
        let again = export::write(&back, &file.value.attachments).map(|again| again.value);
        let fixed = again.as_ref() == Ok(&written.value);
        if fixed {
            self.fixed_points += 1;
        }
        json!({
            "same": differences.is_empty(),
            "differences": differences.iter().map(|(part, at)| format!("{part}{at}")).collect::<Vec<_>>(),
            "written_again_the_same": fixed,
            "warnings": written.warnings.iter().map(|w| format!("{}: {}", w.at, w.message)).collect::<Vec<_>>(),
            "tags_unwritten": unwritten,
            "tags_added": added,
        })
    }

    /// The counts, for the report's summary.
    pub(crate) fn summary(&self) -> Value {
        json!({
            "designs": self.designs,
            "read_back_the_same": self.same,
            "read_back_stating_an_assumed_radius": self.assumed,
            "written_again_the_same": self.fixed_points,
            "parts_differing": self.differing,
            "warnings": self.warnings,
            "tags_unwritten": self.unwritten,
            "tags_added": self.added,
            "failed": self.failed.len(),
        })
    }

    /// Prints the counts under the rest of the survey.
    pub(crate) fn print(&self) {
        println!(
            "  written back out as .ork: {} design(s); {} read back the same, {} stating a radius \
             the reader assumed; {} written again byte for byte; {} failed",
            self.designs,
            self.same,
            self.assumed,
            self.fixed_points,
            self.failed.len()
        );
        crate::ork::print_counts("parts read back differently", &self.differing);
        crate::ork::print_counts("export warnings", &self.warnings);
        crate::ork::print_counts("tags the original has more of", &self.unwritten);
        crate::ork::print_counts("tags the export has more of", &self.added);
    }

    /// Why the survey should fail: a design that does not come back as it went out.
    pub(crate) fn failure(&self) -> Option<String> {
        let apart = self.designs - self.same - self.assumed;
        let moved = self.designs - self.fixed_points;
        (apart + moved + self.failed.len() > 0).then(|| {
            format!(
                "{apart} design(s) read back from their export differently, {moved} were not \
                 written again byte for byte, and {} failed to write or read",
                self.failed.len()
            )
        })
    }
}

/// Whether the only differences are the ones a radius the reader assumed makes: the original
/// gave an automatic radius with nothing to take OpenRocket's default, with a warning that keeps
/// its configurations from flying (ADR-054, ADR-055); the export writes that radius as stated, so
/// the design read back has no warning, and flies.
fn assumed_radius(design: &Imported<Design>, differences: &[(String, String)]) -> bool {
    let warned = design
        .warnings
        .iter()
        .any(|warning| warning.message.starts_with(crate::ork::DEFAULT_RADIUS));
    let flown_only = differences.iter().all(|(part, at)| {
        (part == "rocket" && at.starts_with("/configurations"))
            || (part == "motors" && at.starts_with("/configurations/") && at.ends_with("/left_out"))
    });
    warned && flown_only
}

/// Each part of the design that differs, and the first place in it, as a JSON pointer.
fn differences(original: &Design, back: &Design) -> Vec<(String, String)> {
    if original == back {
        return Vec::new();
    }
    let (Ok(left), Ok(right)) = (serde_json::to_value(original), serde_json::to_value(back)) else {
        return vec![("design".to_owned(), String::new())];
    };
    // Each field of each part that differs, so one difference cannot hide another.
    let mut found = Vec::new();
    for part in PARTS {
        match (&left[part], &right[part]) {
            (Value::Object(l), Value::Object(r)) => {
                let keys: std::collections::BTreeSet<&String> = l.keys().chain(r.keys()).collect();
                for key in keys {
                    let (a, b) = (
                        l.get(key).unwrap_or(&Value::Null),
                        r.get(key).unwrap_or(&Value::Null),
                    );
                    if let Some(at) = pointer(a, b, format!("/{key}")) {
                        found.push((part.to_owned(), at));
                    }
                }
            }
            (l, r) => {
                if let Some(at) = pointer(l, r, String::new()) {
                    found.push((part.to_owned(), at));
                }
            }
        }
    }
    if found.is_empty() {
        // Equal as JSON but not as values: a difference JSON cannot show, such as -0 and 0.
        return vec![("design".to_owned(), " (equal as JSON)".to_owned())];
    }
    found
}

/// The first place `left` and `right` differ, as a JSON pointer below `at`.
fn pointer(left: &Value, right: &Value, at: String) -> Option<String> {
    match (left, right) {
        (Value::Object(l), Value::Object(r)) => {
            for key in l.keys().chain(r.keys()) {
                let (a, b) = (l.get(key), r.get(key));
                if a != b {
                    let here = format!("{at}/{key}");
                    return match (a, b) {
                        (Some(a), Some(b)) => pointer(a, b, here),
                        _ => Some(here),
                    };
                }
            }
            None
        }
        (Value::Array(l), Value::Array(r)) => {
            for (index, (a, b)) in l.iter().zip(r).enumerate() {
                if a != b {
                    return pointer(a, b, format!("{at}/{index}"));
                }
            }
            (l.len() != r.len()).then(|| format!("{at} (length {} vs {})", l.len(), r.len()))
        }
        _ => (left != right).then_some(at),
    }
}

/// How many more of each tag the original document holds than the written one, and the other
/// way round.
fn tag_differences(
    original: &Document,
    written: &Document,
) -> (BTreeMap<String, usize>, BTreeMap<String, usize>) {
    let (mut before, mut after) = (BTreeMap::new(), BTreeMap::new());
    count(&original.root, &mut before);
    count(&written.root, &mut after);
    let more = |a: &BTreeMap<String, usize>, b: &BTreeMap<String, usize>| {
        a.iter()
            .filter_map(|(tag, n)| {
                let m = b.get(tag).copied().unwrap_or(0);
                (*n > m).then(|| (tag.clone(), n - m))
            })
            .collect::<BTreeMap<_, _>>()
    };
    (more(&before, &after), more(&after, &before))
}

fn count(element: &ork::Element, counts: &mut BTreeMap<String, usize>) {
    *counts.entry(element.name.clone()).or_default() += 1;
    for child in element.elements() {
        count(child, counts);
    }
}
