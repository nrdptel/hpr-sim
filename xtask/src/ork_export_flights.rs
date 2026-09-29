//! `cargo xtask ork-export-flights [--check]`: OpenRocket 24.12 flying each design of the reference
//! corpus as hpr writes it back out, against its flight of the original (M3.2b, ADR-110).
//!
//! Two records written by `flights.py` are compared, configuration by configuration: [`ORIGINALS`],
//! OpenRocket's flights of every `.ork` under `refs/` and of the examples in its jar, and
//! [`EXPORTS`], its flights of the same designs read by hpr and written again
//! (`hpr_io::ork::export`), as `cargo xtask ork --export` saves them under [`EXPORT_DIR`]. Both
//! records are gitignored: the designs are other people's. What is committed, [`REPORT`] and its
//! page, holds counts by where the designs came from and the largest difference, never a file.
//!
//! The export record is checked to be of the files hpr writes now: each design is read and written
//! again here, and the written file's SHA-256 must be the one the record flew. Both records must
//! come from the same jar and the same scripts.
//!
//! With `--check` the report is regenerated and compared with the committed one, which fails when
//! either record, the writer, or the report has moved.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

pub const USAGE: &str = "\
  ork-export-flights [--check]
                           Compare OpenRocket's flights of the reference corpus's designs with
                           its flights of hpr's export of them (0.5% of the apogee); write
                           validation/reports/openrocket-export-flights.{json,md}, counts only.";

/// OpenRocket's flights of the originals: `flights.py corpus-out/openrocket-flights.json refs --jar`.
const ORIGINALS: &str = "corpus-out/openrocket-flights.json";

/// OpenRocket's flights of the exports: `flights.py corpus-out/openrocket-export-flights.json
/// corpus-out/ork-export`.
const EXPORTS: &str = "corpus-out/openrocket-export-flights.json";

/// Where `cargo xtask ork --export` saves the written files, by each original's path.
const EXPORT_DIR: &str = "corpus-out/ork-export";

/// The committed report and its page.
const REPORT: &str = "validation/reports/openrocket-export-flights.json";
const PAGE: &str = "validation/reports/openrocket-export-flights.md";

/// M3.2's bar: the export's apogee within this share of the original's, in percent.
pub(crate) const TOLERANCE_PERCENT: f64 = 0.5;

/// Digits kept of a published percentage.
const DIGITS: i32 = 6;

/// What one source's designs came to.
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub(crate) struct Tally {
    /// Designs in the originals' record.
    pub designs: usize,
    /// Of those, the ones OpenRocket opened.
    pub originals_opened: usize,
    /// Of the designs whose original it opened, the ones whose export it opened too.
    pub exports_opened: usize,
    /// Designs whose export it opened though it refused the original.
    pub only_exports_opened: usize,
    /// Designs it refused both as written and as hpr wrote them again, for the same reason.
    pub refused_both: usize,
    /// Designs it refused both ways, but for different reasons.
    pub refused_differently: usize,
    /// Designs hpr does not read, so has no export of; OpenRocket refused each of them too.
    pub unread: usize,
    /// Configurations of the designs opened both ways.
    pub configurations: usize,
    /// Of those, the ones flown to an apogee both ways.
    pub compared: usize,
    /// Of those, the ones whose apogees are the same number.
    pub identical: usize,
    /// Of those compared, the ones within [`TOLERANCE_PERCENT`].
    pub within: usize,
    /// Configurations flown to no apogee either way: no motor, or a flight OpenRocket aborted.
    pub flown_neither: usize,
    /// Configurations flown one way and not the other, or found one way only.
    pub unmatched: usize,
    /// The largest difference of the export's apogee from the original's, in percent of it.
    pub largest_percent: f64,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.designs += other.designs;
        self.originals_opened += other.originals_opened;
        self.exports_opened += other.exports_opened;
        self.only_exports_opened += other.only_exports_opened;
        self.refused_both += other.refused_both;
        self.refused_differently += other.refused_differently;
        self.unread += other.unread;
        self.configurations += other.configurations;
        self.compared += other.compared;
        self.identical += other.identical;
        self.within += other.within;
        self.flown_neither += other.flown_neither;
        self.unmatched += other.unmatched;
        self.largest_percent = self.largest_percent.max(other.largest_percent);
    }

    /// Whether M3.2's two bullets hold: every export opens where its original does, and every
    /// configuration flies to within the tolerance of the original's apogee.
    pub(crate) fn met(&self) -> bool {
        self.exports_opened == self.originals_opened
            && self.refused_differently == 0
            && self.unmatched == 0
            && self.within == self.compared
    }
}

/// The original's key in [`ORIGINALS`] for a design in [`EXPORTS`]: the path under
/// [`EXPORT_DIR`], with the jar's `!` back where `--export` made it a directory.
fn original_of(export: &str) -> Option<String> {
    let path = export.strip_prefix(EXPORT_DIR)?.strip_prefix('/')?;
    Some(path.replacen(".jar/", ".jar!", 1))
}

/// A design's configurations, by id: the apogee OpenRocket flew it to, if it flew to one.
fn apogees(design: &Value) -> BTreeMap<String, Option<f64>> {
    design["flights"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|flight| {
            let id = flight["configuration"].as_str()?.to_owned();
            let flown = flight.get("refused").is_none()
                && flight["has_motors"].as_bool() == Some(true)
                && flight["aborted"].as_bool() == Some(false);
            let apogee = flown
                .then(|| flight["summary"]["max_altitude_m"].as_f64())
                .flatten();
            Some((id, apogee))
        })
        .collect()
}

/// Whether OpenRocket opened a design: the record holds its flights, not a refusal or a fault.
fn opened(design: &Value) -> bool {
    design.get("refused").is_none()
        && design.get("driver_error").is_none()
        && design["flights"].is_array()
}

/// Why OpenRocket did not open a design, as the record says.
fn refusal(design: &Value) -> &Value {
    design
        .get("refused")
        .or_else(|| design.get("driver_error"))
        .unwrap_or(&Value::Null)
}

/// Compares the two records, by the source each design came from.
pub(crate) fn compare(
    originals: &Value,
    exports: &Value,
) -> Result<BTreeMap<String, Tally>, String> {
    let list = |record: &Value, name: &str| -> Result<Vec<Value>, String> {
        record["designs"]
            .as_array()
            .cloned()
            .ok_or_else(|| format!("{name} has no `designs` list"))
    };
    let mut written: BTreeMap<String, Value> = BTreeMap::new();
    for design in list(exports, EXPORTS)? {
        let file = design["file"].as_str().unwrap_or_default();
        let original = original_of(file)
            .ok_or_else(|| format!("{EXPORTS} flew a file outside {EXPORT_DIR}"))?;
        written.insert(original, design);
    }
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for (index, original) in list(originals, ORIGINALS)?.iter().enumerate() {
        let file = original["file"]
            .as_str()
            .ok_or_else(|| format!("design #{index} of {ORIGINALS} has no `file`"))?;
        let tally = tallies
            .entry(crate::ork_corpus_flights::source(file).to_owned())
            .or_default();
        tally.designs += 1;
        // hpr writes only what it reads; a file it can't read has no export (NOT_WELL_FORMED).
        let Some(export) = written.get(file) else {
            if opened(original) {
                return Err(format!(
                    "design #{index} ({}): OpenRocket opened it, but it has no export flight",
                    crate::ork_corpus_flights::source(file)
                ));
            }
            tally.unread += 1;
            continue;
        };
        match (opened(original), opened(export)) {
            (true, true) => {
                tally.originals_opened += 1;
                tally.exports_opened += 1;
            }
            (true, false) => tally.originals_opened += 1,
            (false, true) => {
                tally.only_exports_opened += 1;
                continue;
            }
            (false, false) => {
                if refusal(original) == refusal(export) {
                    tally.refused_both += 1;
                } else {
                    tally.refused_differently += 1;
                }
                continue;
            }
        }
        if !opened(export) {
            continue;
        }
        let (before, after) = (apogees(original), apogees(export));
        for (id, apogee) in &before {
            tally.configurations += 1;
            match (apogee, after.get(id)) {
                (Some(before), Some(Some(after))) => {
                    tally.compared += 1;
                    if before == after {
                        tally.identical += 1;
                    }
                    let percent = 100.0 * ((after - before) / before).abs();
                    if percent <= TOLERANCE_PERCENT {
                        tally.within += 1;
                    }
                    tally.largest_percent = tally.largest_percent.max(round(percent));
                }
                (None, Some(None)) => tally.flown_neither += 1,
                _ => tally.unmatched += 1,
            }
        }
        tally.unmatched += after.keys().filter(|id| !before.contains_key(*id)).count();
    }
    Ok(tallies)
}

/// `value` to [`DIGITS`] decimals, so a report is the same wherever it is made.
fn round(value: f64) -> f64 {
    let scale = 10f64.powi(DIGITS);
    (value * scale).round() / scale
}

pub fn run(args: &[String]) -> Result<(), String> {
    let check = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return Err(format!("usage:\n{USAGE}")),
    };
    let root = crate::ork::root()?;
    let load = |name: &str| -> Result<Value, String> {
        let path = root.join(name);
        let text = fs::read_to_string(&path).map_err(|error| {
            format!(
                "{}: {error}; run `cargo xtask ork --export {EXPORT_DIR}`, then flights.py for \
                 both records (see {PAGE})",
                path.display()
            )
        })?;
        serde_json::from_str(&text).map_err(|error| format!("{name}: {error}"))
    };
    let (originals, exports) = (load(ORIGINALS)?, load(EXPORTS)?);
    for field in ["jar_sha256", "inputs_sha256", "seed"] {
        if originals[field] != exports[field] {
            return Err(format!(
                "the two records differ in `{field}`: fly both with the same jar and scripts"
            ));
        }
    }
    fresh(&root, &exports)?;
    let tallies = compare(&originals, &exports)?;
    let report = report(&originals, &tallies);
    let json = format!(
        "{}\n",
        serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?
    );
    let page = page(&report)?;
    if check {
        for (name, text) in [(REPORT, &json), (PAGE, &page)] {
            let committed = fs::read_to_string(root.join(name)).unwrap_or_default();
            if committed.replace("\r\n", "\n") != *text {
                return Err(format!(
                    "{name} is not what the records give; run `cargo xtask ork-export-flights`"
                ));
            }
        }
    } else {
        fs::write(root.join(REPORT), &json).map_err(|error| format!("{REPORT}: {error}"))?;
        fs::write(root.join(PAGE), &page).map_err(|error| format!("{PAGE}: {error}"))?;
    }
    print!("{page}");
    let total: Tally = serde_json::from_value(report["total"].clone())
        .map_err(|error| format!("the report's total: {error}"))?;
    if total.met() {
        Ok(())
    } else {
        Err("M3.2's bar is not met: see the report".to_owned())
    }
}

/// Fails unless every design [`EXPORTS`] flew is the file hpr writes today: each original is read
/// and written again, and the SHA-256 compared. No error names a file.
fn fresh(root: &Path, exports: &Value) -> Result<(), String> {
    let mut jar: Option<BTreeMap<String, Vec<u8>>> = None;
    for (index, design) in exports["designs"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let file = design["file"].as_str().unwrap_or_default();
        let original = original_of(file).unwrap_or_default();
        let bytes = match original.split_once('!') {
            Some((_, entry)) => {
                let entries = match &mut jar {
                    Some(entries) => entries,
                    None => jar.insert(
                        crate::ork::examples_in_jar(&root.join(crate::ork::JAR))?
                            .into_iter()
                            .filter_map(|(name, bytes)| {
                                Some((name.split_once('!')?.1.to_owned(), bytes))
                            })
                            .collect(),
                    ),
                };
                entries
                    .get(entry)
                    .cloned()
                    .ok_or_else(|| format!("export #{index}: its original is not in the jar"))?
            }
            None => fs::read(root.join(&original))
                .map_err(|_| format!("export #{index}: its original is not under refs/"))?,
        };
        let read = hpr_io::ork::read(&bytes)
            .map_err(|_| format!("export #{index}: its original no longer reads"))?;
        let design_now = hpr_io::ork::design(&read.value).value;
        let written = hpr_io::ork::export::write(&design_now, &read.value.attachments)
            .map_err(|error| format!("export #{index}: {error}"))?;
        let digest: String = Sha256::digest(&written.value)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if design["sha256"].as_str() != Some(digest.as_str()) {
            return Err(format!(
                "export #{index} is not the file hpr writes today: run `cargo xtask ork --export \
                 {EXPORT_DIR}`, then flights.py for {EXPORTS}"
            ));
        }
    }
    Ok(())
}

/// The committed report: where the records came from, and the counts by source and in total.
fn report(originals: &Value, tallies: &BTreeMap<String, Tally>) -> Value {
    let mut total = Tally::default();
    for tally in tallies.values() {
        total.add(tally);
    }
    json!({
        "source": "cargo xtask ork-export-flights (M3.2b, ADR-110)",
        "openrocket": originals["openrocket"],
        "jar_sha256": originals["jar_sha256"],
        "inputs_sha256": originals["inputs_sha256"],
        "seed": originals["seed"],
        "tolerance_percent": TOLERANCE_PERCENT,
        "sources": tallies,
        "total": total,
    })
}

/// The report as a page.
pub(crate) fn page(report: &Value) -> Result<String, String> {
    let sources: BTreeMap<String, Tally> = serde_json::from_value(report["sources"].clone())
        .map_err(|error| format!("the report's sources: {error}"))?;
    let total: Tally = serde_json::from_value(report["total"].clone())
        .map_err(|error| format!("the report's total: {error}"))?;
    let mut out = String::new();
    let _ = writeln!(out, "# OpenRocket flying hpr's `.ork` export\n");
    let _ = writeln!(
        out,
        "Generated by `cargo xtask ork-export-flights` from two records of OpenRocket {}'s calm-air \
         flights (seed {}): of each design in the reference corpus as written, and of the same \
         design read by hpr and written again. A configuration passes when the export's apogee \
         is within {TOLERANCE_PERCENT}% of the original's. The designs are private, so only \
         counts are published.\n",
        report["openrocket"].as_str().unwrap_or("?"),
        report["seed"]
    );
    let _ = writeln!(
        out,
        "| source | designs | originals opened | exports opened | configurations | flown neither way | compared | identical | within {TOLERANCE_PERCENT}% | unmatched | largest difference |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|");
    for (name, tally) in sources.iter().chain([(&"total".to_owned(), &total)]) {
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {}% |",
            tally.designs,
            tally.originals_opened,
            tally.exports_opened,
            tally.configurations,
            tally.flown_neither,
            tally.compared,
            tally.identical,
            tally.within,
            tally.unmatched,
            tally.largest_percent
        );
    }
    let _ = writeln!(
        out,
        "\nOf the {} designs OpenRocket did not open as written, hpr does not read {}, so has none \
         to write; OpenRocket refused the export of {} for the reason it refused the original, of \
         {} for another, and opened {}. A configuration is flown \
         neither way when it has no motor or OpenRocket aborted its flight both times; one flown \
         only one way, or found only one way, is unmatched.",
        total.designs - total.originals_opened,
        total.unread,
        total.refused_both,
        total.refused_differently,
        total.only_exports_opened
    );
    let _ = writeln!(
        out,
        "\nM3.2's bar ({}): every export opens where its original does and is refused for the same \
         reason where it does not, and every configuration flown both ways is within \
         {TOLERANCE_PERCENT}%.",
        if total.met() { "met" } else { "not met" }
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed() -> Value {
        let root = crate::ork::root().expect("the workspace root");
        let text = fs::read_to_string(root.join(REPORT)).expect("the committed report");
        serde_json::from_str(&text).expect("JSON")
    }

    /// The committed page is the committed report's, the total is the sum of its sources, and it
    /// meets M3.2's bar: CI has no records to make them from, so it holds them to each other.
    #[test]
    fn the_committed_report_is_its_page_and_meets_the_bar() {
        let root = crate::ork::root().expect("the workspace root");
        let report = committed();
        let page_text = fs::read_to_string(root.join(PAGE)).expect("the committed page");
        assert_eq!(
            page(&report).expect("a page"),
            page_text.replace("\r\n", "\n")
        );
        let sources: BTreeMap<String, Tally> =
            serde_json::from_value(report["sources"].clone()).expect("sources");
        let mut sum = Tally::default();
        for tally in sources.values() {
            sum.add(tally);
        }
        let total: Tally = serde_json::from_value(report["total"].clone()).expect("total");
        assert_eq!(sum, total);
        for tally in sources.values().chain([&total]) {
            assert_eq!(
                tally.designs,
                tally.originals_opened
                    + tally.only_exports_opened
                    + tally.refused_both
                    + tally.refused_differently
                    + tally.unread
            );
            assert!(tally.configurations >= tally.compared + tally.flown_neither);
        }
        assert!(total.met(), "{total:?}");
        assert!(total.compared > 0 && total.largest_percent <= TOLERANCE_PERCENT);
        assert_eq!(report["tolerance_percent"], json!(TOLERANCE_PERCENT));
    }

    /// Two records of invented designs: one flown the same and a hair apart, one past the bar, one
    /// refused both ways alike and one not, one with a configuration only its export has; each
    /// lands where it should.
    #[test]
    fn records_compare_configuration_by_configuration() {
        let flight = |id: &str, apogee: f64| {
            json!({"configuration": id, "has_motors": true, "aborted": false,
                   "summary": {"max_altitude_m": apogee}})
        };
        let design = |file: &str, flights: Vec<Value>| json!({"file": file, "flights": flights});
        let refused = |file: &str| json!({"file": file, "refused": "no"});
        let refused_otherwise = |file: &str| json!({"file": file, "refused": "other"});
        let originals = json!({"designs": [
            design("refs/loft-fixtures/a.ork", vec![flight("1", 100.0), flight("2", 200.0)]),
            design("refs/other/b.ork", vec![flight("1", 100.0)]),
            refused("refs/other/c.ork"),
            refused("refs/other/e.ork"),
            design("refs/openrocket/OpenRocket-24.12.jar!datafiles/examples/d.ork", vec![flight("1", 50.0)]),
        ]});
        let exports = json!({"designs": [
            design("corpus-out/ork-export/refs/loft-fixtures/a.ork", vec![flight("1", 100.0), flight("2", 200.4)]),
            design("corpus-out/ork-export/refs/other/b.ork", vec![flight("1", 101.0)]),
            refused("corpus-out/ork-export/refs/other/c.ork"),
            refused_otherwise("corpus-out/ork-export/refs/other/e.ork"),
            design("corpus-out/ork-export/refs/openrocket/OpenRocket-24.12.jar/datafiles/examples/d.ork", vec![flight("1", 50.0), flight("9", 1.0)]),
        ]});
        let tallies = compare(&originals, &exports).expect("compared");
        let library = &tallies["refs/loft-fixtures"];
        assert_eq!(
            (library.compared, library.identical, library.within),
            (2, 1, 2)
        );
        assert_eq!(library.largest_percent, 0.2);
        let elsewhere = &tallies["elsewhere under refs/"];
        assert_eq!(
            (
                elsewhere.designs,
                elsewhere.originals_opened,
                elsewhere.within
            ),
            (3, 1, 0)
        );
        assert_eq!(
            (
                elsewhere.refused_both,
                elsewhere.refused_differently,
                elsewhere.unread
            ),
            (1, 1, 0)
        );
        assert_eq!(elsewhere.largest_percent, 1.0);
        let examples = &tallies["OpenRocket's examples"];
        assert_eq!((examples.identical, examples.unmatched), (1, 1));
        assert!(library.met() && !elsewhere.met() && !examples.met());
    }
}
