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

use crate::ork_supply::sha256;

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

/// Where the guide explains the check, and the roadmap's row for M3.2.
const GUIDE: &str = "https://nrdptel.github.io/hpr-sim/format/ork.html#checked-in-openrocket";
const ROADMAP_ROW: &str = "https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m3-2";

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
    /// Designs with at least one configuration compared.
    pub designs_compared: usize,
    /// Of those, the ones flown to an apogee both ways.
    pub compared: usize,
    /// Of those, the ones whose apogees are the same number.
    pub identical: usize,
    /// Of those compared, the ones within [`TOLERANCE_PERCENT`].
    pub within: usize,
    /// Configurations flown to no apogee either way, alike: no motor both times, or a flight
    /// OpenRocket aborted both times for the same cause.
    pub flown_neither: usize,
    /// Configurations flown one way and not the other, left unflown for different reasons, or
    /// found one way only.
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
        self.designs_compared += other.designs_compared;
        self.compared += other.compared;
        self.identical += other.identical;
        self.within += other.within;
        self.flown_neither += other.flown_neither;
        self.unmatched += other.unmatched;
        self.largest_percent = self.largest_percent.max(other.largest_percent);
    }

    /// Whether M3.2's two bullets hold: every export opens where its original does and is refused
    /// alike where it does not, and every configuration flies to within the tolerance of the
    /// original's apogee.
    pub(crate) fn met(&self) -> bool {
        self.exports_opened == self.originals_opened
            && self.only_exports_opened == 0
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

/// How a configuration's flight came out.
#[derive(Debug, PartialEq)]
enum Outcome {
    /// Flown to this apogee, in metres.
    Apogee(f64),
    /// Not flown: it has no motor.
    NoMotor,
    /// Aborted, with the causes OpenRocket gave.
    Aborted(Vec<Value>),
    /// Refused, or anything else the record says: compared as it stands.
    Other(Value),
}

/// A design's configurations, by id, and how OpenRocket's flight of each came out.
fn outcomes(design: &Value) -> BTreeMap<String, Outcome> {
    design["flights"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|flight| {
            let id = flight["configuration"].as_str()?.to_owned();
            let apogee = flight["summary"]["max_altitude_m"].as_f64();
            let outcome = match (
                flight.get("refused"),
                flight["has_motors"].as_bool(),
                flight["aborted"].as_bool(),
            ) {
                (None, Some(true), Some(false)) if apogee.is_some() => {
                    Outcome::Apogee(apogee.unwrap_or_default())
                }
                (None, Some(false), _) => Outcome::NoMotor,
                (None, Some(true), Some(true)) => Outcome::Aborted(
                    flight["events"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|event| event["type"] == "SIM_ABORT")
                        .map(|event| event["cause"].clone())
                        .collect(),
                ),
                _ => Outcome::Other(flight.get("refused").cloned().unwrap_or(Value::Null)),
            };
            Some((id, outcome))
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
/// `unread` says, of an original with no export, whether hpr cannot read it.
pub(crate) fn compare(
    originals: &Value,
    exports: &Value,
    mut unread: impl FnMut(&str) -> bool,
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
        // hpr writes only what it reads; a file it can't read has no export (NOT_WELL_FORMED),
        // which `unread` confirms.
        let Some(export) = written.remove(file) else {
            if opened(original) {
                return Err(format!(
                    "design #{index} ({}): OpenRocket opened it, but it has no export flight",
                    crate::ork_corpus_flights::source(file)
                ));
            }
            if !unread(file) {
                return Err(format!(
                    "design #{index} ({}): hpr reads it, but it has no export flight",
                    crate::ork_corpus_flights::source(file)
                ));
            }
            tally.unread += 1;
            continue;
        };
        let export = &export;
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
        let (before, after) = (outcomes(original), outcomes(export));
        let compared = tally.compared;
        for (id, outcome) in &before {
            tally.configurations += 1;
            match (outcome, after.get(id)) {
                (Outcome::Apogee(before), Some(Outcome::Apogee(after))) => {
                    // An apogee at or below the launch has no difference in percent to take.
                    let percent = if before == after {
                        0.0
                    } else if *before > 0.0 {
                        100.0 * ((after - before) / before).abs()
                    } else {
                        tally.unmatched += 1;
                        continue;
                    };
                    tally.compared += 1;
                    if before == after {
                        tally.identical += 1;
                    }
                    if percent <= TOLERANCE_PERCENT {
                        tally.within += 1;
                    }
                    tally.largest_percent = tally.largest_percent.max(round(percent));
                }
                (Outcome::NoMotor | Outcome::Aborted(_) | Outcome::Other(_), Some(other))
                    if outcome == other =>
                {
                    tally.flown_neither += 1;
                }
                _ => tally.unmatched += 1,
            }
        }
        if tally.compared > compared {
            tally.designs_compared += 1;
        }
        tally.unmatched += after.keys().filter(|id| !before.contains_key(*id)).count();
    }
    if !written.is_empty() {
        return Err(format!(
            "{EXPORTS} flew {} design(s) {ORIGINALS} does not have: fly the originals again",
            written.len()
        ));
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
    current(&root, &originals)?;
    let mut sources = Sources::new(&root);
    fresh(&root, &originals, &exports, &mut sources)?;
    let unread = |file: &str| {
        sources
            .bytes(file)
            .is_ok_and(|bytes| hpr_io::ork::read(&bytes).is_err())
    };
    let tallies = compare(&originals, &exports, unread)?;
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

/// OpenRocket's flight scripts, whose SHA-256 each record holds.
const SCRIPTS: [(&str, &str); 3] = [
    ("flights.py", "validation/oracles/openrocket/flights.py"),
    ("events.py", "validation/oracles/openrocket/events.py"),
    (
        "geometry.py",
        "validation/oracles/rocketserializer/geometry.py",
    ),
];

/// Fails unless the records were flown with today's scripts and the pinned jar.
fn current(root: &Path, record: &Value) -> Result<(), String> {
    for (name, path) in SCRIPTS {
        let bytes = fs::read(root.join(path)).map_err(|error| format!("{path}: {error}"))?;
        if record["inputs_sha256"][name].as_str() != Some(&sha256(&bytes)) {
            return Err(format!(
                "the records were not flown by the current {path}: fly both again"
            ));
        }
    }
    let jar = crate::ork::JAR;
    let bytes = fs::read(root.join(jar)).map_err(|error| format!("{jar}: {error}"))?;
    if record["jar_sha256"].as_str() != Some(&sha256(&bytes)) {
        return Err(format!(
            "the records were not flown with {jar}: fly both again"
        ));
    }
    Ok(())
}

/// The originals' bytes, by their key in [`ORIGINALS`]: a file under the root, or an entry of
/// the jar, read once.
struct Sources {
    root: std::path::PathBuf,
    jar: Option<BTreeMap<String, Vec<u8>>>,
}

impl Sources {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            jar: None,
        }
    }

    fn bytes(&mut self, key: &str) -> Result<Vec<u8>, String> {
        let Some((_, entry)) = key.rsplit_once('!') else {
            return fs::read(self.root.join(key)).map_err(|_| "not under refs/".to_owned());
        };
        let entries = match &mut self.jar {
            Some(entries) => entries,
            None => self.jar.insert(
                crate::ork::examples_in_jar(&self.root.join(crate::ork::JAR))?
                    .into_iter()
                    .filter_map(|(name, bytes)| Some((name.rsplit_once('!')?.1.to_owned(), bytes)))
                    .collect(),
            ),
        };
        entries
            .get(entry)
            .cloned()
            .ok_or_else(|| "not in the jar".to_owned())
    }
}

/// Fails unless both records flew today's files: each original is the file [`ORIGINALS`] flew,
/// and each design [`EXPORTS`] flew is the file hpr writes from it today, as `cargo xtask ork
/// --export` writes it. No error names a file.
fn fresh(
    root: &Path,
    originals: &Value,
    exports: &Value,
    sources: &mut Sources,
) -> Result<(), String> {
    for (index, design) in originals["designs"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let file = design["file"].as_str().unwrap_or_default();
        let bytes = sources
            .bytes(file)
            .map_err(|why| format!("original #{index}: {why}"))?;
        if design["sha256"].as_str() != Some(&sha256(&bytes)) {
            return Err(format!(
                "original #{index} is not the file {ORIGINALS} flew: fly the originals again"
            ));
        }
    }
    let supply = crate::ork_supply::Supply::load(root, true)?;
    for (index, design) in exports["designs"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let file = design["file"].as_str().unwrap_or_default();
        let original = original_of(file).unwrap_or_default();
        let bytes = sources
            .bytes(&original)
            .map_err(|why| format!("export #{index}: its original is {why}"))?;
        let read = hpr_io::ork::read(&bytes)
            .map_err(|_| format!("export #{index}: its original no longer reads"))?;
        let design_now = hpr_io::ork::design_with(&read.value, supply.curves()).value;
        let written = hpr_io::ork::export::write(&design_now, &read.value.attachments)
            .map_err(|_| format!("export #{index}: its original no longer writes"))?;
        if design["sha256"].as_str() != Some(&sha256(&written.value)) {
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
         is within {TOLERANCE_PERCENT}% of the original's; *identical* means the same apogee to \
         the last bit. Both flights are OpenRocket's, so this checks the written file, not hpr's \
         physics. The designs are private, so only counts are published. What the check is for, \
         and how to run it: [Checked in OpenRocket]({GUIDE}).\n",
        report["openrocket"].as_str().unwrap_or("?"),
        report["seed"]
    );
    let _ = writeln!(
        out,
        "| source | designs | originals opened | exports opened | designs compared | configurations | flown neither way | compared | identical | within {TOLERANCE_PERCENT}% | unmatched | largest difference |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (name, tally) in sources.iter().chain([(&"total".to_owned(), &total)]) {
        let _ = writeln!(
            out,
            "| {name} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {}% |",
            tally.designs,
            tally.originals_opened,
            tally.exports_opened,
            tally.designs_compared,
            tally.configurations,
            tally.flown_neither,
            tally.compared,
            tally.identical,
            tally.within,
            tally.unmatched,
            tally.largest_percent
        );
    }
    let refused = total.designs.saturating_sub(total.originals_opened);
    let mut outcomes = Vec::new();
    for (count, what) in [
        (total.unread, "hpr can't read, so it writes nothing"),
        (
            total.refused_both,
            "OpenRocket refuses as exported with the original's error",
        ),
        (
            total.refused_differently,
            "OpenRocket refuses as exported with another error",
        ),
        (total.only_exports_opened, "OpenRocket opens as exported"),
    ] {
        if count > 0 {
            outcomes.push(format!("{count} {what}"));
        }
    }
    if refused > 0 {
        let _ = writeln!(
            out,
            "\nOpenRocket did not open {refused} design(s) as written: of those, {}.",
            outcomes.join("; ")
        );
    }
    let _ = writeln!(
        out,
        "\nA design is compared when at least one of its configurations flies both ways. A \
         configuration is flown neither way when it has no motor, or OpenRocket aborted it for the \
         same cause, both times. One flown only one way, left unflown for different reasons, or \
         found only one way, is unmatched."
    );
    let _ = writeln!(
        out,
        "\nThe bar, from the roadmap's [M3.2]({ROADMAP_ROW}) (writing `.ork` files): every export \
         opens where its original does and is refused for the same reason where it does not, and \
         every configuration flown both ways is within {TOLERANCE_PERCENT}%. **{}.**",
        if total.met() { "Met" } else { "Not met" }
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

    /// The committed page is the committed report's, the total is the sum of its sources, it
    /// meets M3.2's bar, and it was flown by the committed scripts: CI has no records to make it
    /// from, so it holds it to itself.
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
            assert!(tally.designs_compared <= tally.exports_opened);
        }
        assert!(total.met(), "{total:?}");
        assert!(total.compared > 0 && total.largest_percent <= TOLERANCE_PERCENT);
        assert_eq!(report["tolerance_percent"], json!(TOLERANCE_PERCENT));
        for (name, path) in SCRIPTS {
            let bytes = fs::read(root.join(path)).expect("the script");
            assert_eq!(
                report["inputs_sha256"][name].as_str(),
                Some(sha256(&bytes).as_str()),
                "{path} changed since the report's flights: fly both records again"
            );
        }
    }

    fn flight(id: &str, apogee: f64) -> Value {
        json!({"configuration": id, "has_motors": true, "aborted": false,
               "summary": {"max_altitude_m": apogee}})
    }

    fn unpowered(id: &str) -> Value {
        json!({"configuration": id, "has_motors": false})
    }

    fn aborted(id: &str, cause: &str) -> Value {
        json!({"configuration": id, "has_motors": true, "aborted": true,
               "events": [{"type": "SIM_ABORT", "cause": cause}],
               "summary": {"max_altitude_m": 10.0}})
    }

    fn design(file: &str, flights: Vec<Value>) -> Value {
        json!({"file": file, "flights": flights})
    }

    fn refused(file: &str, why: &str) -> Value {
        json!({"file": file, "refused": why})
    }

    fn exported(file: &str) -> String {
        format!("{EXPORT_DIR}/{}", file.replace('!', "/"))
    }

    /// One original and its export, compared on their own: the tally of their source.
    fn one(original: Value, export: Value) -> Tally {
        let file = original["file"].as_str().expect("a file").to_owned();
        let mut export = export;
        export["file"] = json!(exported(&file));
        let tallies = compare(
            &json!({"designs": [original]}),
            &json!({"designs": [export]}),
            |_| false,
        )
        .expect("compared");
        tallies.into_values().next().expect("one source")
    }

    const FILE: &str = "refs/loft-fixtures/a.ork";

    /// Two records of invented designs: one flown the same and a hair apart, one past the bar, one
    /// refused both ways alike and one not, one with a configuration only its export has; each
    /// lands where it should.
    #[test]
    fn records_compare_configuration_by_configuration() {
        let originals = json!({"designs": [
            design("refs/loft-fixtures/a.ork", vec![flight("1", 100.0), flight("2", 200.0)]),
            design("refs/other/b.ork", vec![flight("1", 100.0)]),
            refused("refs/other/c.ork", "no"),
            refused("refs/other/e.ork", "no"),
            design("refs/openrocket/OpenRocket-24.12.jar!datafiles/examples/d.ork", vec![flight("1", 50.0)]),
        ]});
        let exports = json!({"designs": [
            design("corpus-out/ork-export/refs/loft-fixtures/a.ork", vec![flight("1", 100.0), flight("2", 200.4)]),
            design("corpus-out/ork-export/refs/other/b.ork", vec![flight("1", 101.0)]),
            refused("corpus-out/ork-export/refs/other/c.ork", "no"),
            refused("corpus-out/ork-export/refs/other/e.ork", "other"),
            design("corpus-out/ork-export/refs/openrocket/OpenRocket-24.12.jar/datafiles/examples/d.ork", vec![flight("1", 50.0), flight("9", 1.0)]),
        ]});
        let tallies = compare(&originals, &exports, |_| false).expect("compared");
        let library = &tallies["refs/loft-fixtures"];
        assert_eq!(
            (library.compared, library.identical, library.within),
            (2, 1, 2)
        );
        assert_eq!(
            (library.designs_compared, library.largest_percent),
            (1, 0.2)
        );
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

    /// A configuration left unflown counts as flown neither way only when both flights say why
    /// alike; otherwise it is unmatched, and the bar fails.
    #[test]
    fn unflown_configurations_must_be_unflown_alike() {
        let alike = one(
            design(FILE, vec![unpowered("1"), aborted("2", "tumbled")]),
            design("", vec![unpowered("1"), aborted("2", "tumbled")]),
        );
        assert_eq!((alike.flown_neither, alike.unmatched), (2, 0));
        assert!(alike.met());
        for export in [
            vec![unpowered("1")],
            vec![aborted("1", "another cause")],
            vec![flight("1", 10.0)],
            vec![],
        ] {
            let apart = one(
                design(FILE, vec![aborted("1", "tumbled")]),
                design("", export),
            );
            assert_eq!((apart.flown_neither, apart.unmatched), (0, 1));
            assert!(!apart.met());
        }
    }

    /// The bar's edge: 0.5% passes, a hair more fails; an apogee at the launch compares only to
    /// itself.
    #[test]
    fn the_tolerance_is_inclusive_and_a_zero_apogee_is_not_divided_by() {
        let at = |after: f64| {
            one(
                design(FILE, vec![flight("1", 200.0)]),
                design("", vec![flight("1", after)]),
            )
        };
        assert!(at(201.0).met());
        assert!(!at(201.0 + 1e-9).met());
        let zero = |after: f64| {
            one(
                design(FILE, vec![flight("1", 0.0)]),
                design("", vec![flight("1", after)]),
            )
        };
        let same = zero(0.0);
        assert_eq!(
            (same.compared, same.identical, same.largest_percent),
            (1, 1, 0.0)
        );
        let apart = zero(1.0);
        assert_eq!((apart.compared, apart.unmatched), (0, 1));
        assert!(!apart.met());
    }

    /// Each way a design can open or not: the bar fails unless the export does as the original.
    #[test]
    fn exports_open_as_their_originals_do() {
        let opens = design(FILE, vec![flight("1", 10.0)]);
        assert!(one(opens.clone(), opens.clone()).met());
        let lost = one(opens.clone(), refused("", "no"));
        assert_eq!((lost.originals_opened, lost.exports_opened), (1, 0));
        assert!(!lost.met());
        let gained = one(refused(FILE, "no"), opens);
        assert_eq!(gained.only_exports_opened, 1);
        assert!(!gained.met());
        let alike = one(refused(FILE, "no"), refused("", "no"));
        assert_eq!(alike.refused_both, 1);
        assert!(alike.met());
        assert!(!one(refused(FILE, "no"), refused("", "other")).met());
    }

    /// A design missing from one record: an original hpr can't read is counted as such, and any
    /// other gap is an error, whichever record has it.
    #[test]
    fn a_design_missing_from_one_record_is_explained_or_refused() {
        let originals = json!({"designs": [refused(FILE, "malformed")]});
        let none = json!({"designs": []});
        let unread = compare(&originals, &none, |_| true).expect("compared");
        assert_eq!(unread["refs/loft-fixtures"].unread, 1);
        assert!(unread["refs/loft-fixtures"].met());
        assert!(compare(&originals, &none, |_| false).is_err());
        let opened = json!({"designs": [design(FILE, vec![])]});
        assert!(compare(&opened, &none, |_| true).is_err());
        let extra = json!({"designs": [design(&exported(FILE), vec![])]});
        assert!(compare(&none, &extra, |_| true).is_err());
    }

    /// An export's path leads back to its original's key, a jar entry's `!` included.
    #[test]
    fn export_paths_lead_back_to_their_originals() {
        for file in [
            FILE,
            "refs/openrocket/OpenRocket-24.12.jar!datafiles/examples/d.ork",
        ] {
            assert_eq!(original_of(&exported(file)).as_deref(), Some(file));
        }
        assert_eq!(original_of("elsewhere/a.ork"), None);
    }
}
