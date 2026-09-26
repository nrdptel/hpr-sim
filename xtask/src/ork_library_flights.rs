//! `cargo xtask ork-flights --library [--check]`: flies hpr on every configuration of the private
//! design library that OpenRocket flew and hpr flies, and reports the differences under
//! anonymised ids (M2.2e3, ADR-072).
//!
//! OpenRocket 24.12's flights of the library are in [`RECORD`], which `flights.py` writes under the
//! gitignored `corpus-out/` (M2.2e2, ADR-071): the designs are other people's. Each is flown by
//! [`fly_design`], exactly as the public report flies OpenRocket's examples, and compared by the
//! same definitions. A library file that is a public design, the same bytes or the same rocket
//! (its name and its configurations' ids, or those ids alone when OpenRocket made them at random)
//! saved again, is left to the public report, and a private design found twice is counted once
//! ([`select`]).
//!
//! What is committed, [`REPORT_JSON`] and [`REPORT_MD`], holds no file name, part name, motor or
//! value of a design. Each flight is an id, `C09/2` for the second configuration of the ninth
//! design in the order of their SHA-256, with its differences from OpenRocket's in per cent or
//! calibres, rounded to [`DIGITS`] decimals, masses to [`MASS_DIGITS`], so that no rounding
//! residue can give a value back; the class of its largest Mach number and of its launch site; how
//! early OpenRocket's parachute opened, to [`CHUTE_DIGITS`] decimals of a second; what hpr's design checks found, by kind; and the named causes of
//! [`crate::ork_flights`]. A value is never written, since with its difference it would give back
//! OpenRocket's. The summary is computed from those rows, and a test holds it to them in CI, where
//! the library is not.
//!
//! The report also counts the designs with a flight compared in all five of M2.2's spreads
//! (apogee, largest speed, margin, mass and centre of mass), beside the public report's count, as
//! M2.2's *done when* asks for at least [`BAR`] in all.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::ork_flights::{
    DRAG_OVERRIDE, EARLY_CHUTE, METRICS, NO_NAMED_CAUSE, fixed, fly_design,
    mass_and_cg_differences, read_json, same, signed, spread, summarise,
};
use crate::ork_supply::sha256;

/// OpenRocket's flights of the library (M2.2e2), gitignored.
pub(crate) const RECORD: &str = crate::ork_corpus_flights::RECORD;

/// Where the library is.
const LIBRARY: &str = "refs/loft-fixtures/";

/// The report, as data.
pub(crate) const REPORT_JSON: &str = "validation/reports/openrocket-library-flights.json";

/// The report, as a page.
pub(crate) const REPORT_MD: &str = "validation/reports/openrocket-library-flights.md";

/// The oracle scripts the record must have been written by, as its `inputs_sha256` names them.
const SCRIPTS: [(&str, &str); 3] = [
    ("flights.py", "validation/oracles/openrocket/flights.py"),
    ("events.py", "validation/oracles/openrocket/events.py"),
    (
        "geometry.py",
        "validation/oracles/rocketserializer/geometry.py",
    ),
];

/// The five spreads of M2.2's *done when*, by their keys in an anonymised row.
pub(crate) const FIVE: [&str; 5] = [
    "apogee_percent",
    "max_speed_percent",
    "margin_cal",
    "launch_mass_percent",
    "rod_clearance_cg_cal",
];

/// The designs M2.2's *done when* asks for, public and private together.
pub(crate) const BAR: usize = 20;

/// The decimals a published difference keeps.
pub(crate) const DIGITS: i32 = 6;

/// The decimals a published mass difference keeps, in per cent: fewer, since across a design's
/// configurations they would give the ratios of its launch masses.
pub(crate) const MASS_DIGITS: i32 = 3;

/// The decimals of how early OpenRocket's parachute opened, in seconds.
pub(crate) const CHUTE_DIGITS: i32 = 2;

/// The keys a flight's row holds, and nothing else.
const ROW_KEYS: [&str; 20] = [
    "flight",
    "aborted",
    "termination",
    "mach",
    "site",
    "apogee_percent",
    "apogee_outcome",
    "max_speed_percent",
    "max_speed_outcome",
    "margin_cal",
    "margin_outcome",
    "launch_mass_percent",
    "rod_clearance_mass_percent",
    "rod_clearance_cg_cal",
    "rod_clearance_cp_cal",
    "reference_same",
    "chute_early_s",
    "drag_overrides_not_applied",
    "without_the_overridden_parts",
    "design_checks",
];

/// Why `row` may not be published: a key not on [`ROW_KEYS`], or a number not rounded to its
/// digits. A difference of one rounding step would give back the value it was taken from.
pub(crate) fn unpublishable(row: &Value) -> Option<String> {
    let flight = row["flight"].as_str().unwrap_or("a row");
    for (key, value) in row.as_object()? {
        if !ROW_KEYS.contains(&key.as_str()) {
            return Some(format!("{flight} holds `{key}`, which is not on the list"));
        }
        let numbers: Vec<(&str, f64)> = match value {
            Value::Number(n) if key != "drag_overrides_not_applied" => {
                vec![(key.as_str(), n.as_f64()?)]
            }
            Value::Object(inner) => inner
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_f64()))
                .filter_map(|(k, v)| Some((k, v?)))
                .collect(),
            _ => Vec::new(),
        };
        for (key, number) in numbers {
            if rounded(Some(number), digits_of(key)) != json!(number) {
                return Some(format!("{flight}'s `{key}` is not rounded"));
            }
        }
    }
    None
}

/// The digits each number of a row is rounded to.
fn digits_of(key: &str) -> i32 {
    match key {
        "launch_mass_percent" | "rod_clearance_mass_percent" => MASS_DIGITS,
        "chute_early_s" => CHUTE_DIGITS,
        _ => DIGITS,
    }
}

pub(crate) fn run(check: bool) -> Result<(), String> {
    let root = crate::ork::root()?;
    let report = fly_library(&root)?;
    let page = page(&report);
    if check {
        let committed = read_json(&root.join(REPORT_JSON))?;
        let mut apart = Vec::new();
        same(&committed, &report, "", &mut apart);
        let committed_page = fs::read_to_string(root.join(REPORT_MD))
            .map_err(|error| format!("{REPORT_MD}: {error}"))?;
        if committed_page.replace("\r\n", "\n") != page {
            apart.push(format!("{REPORT_MD} differs from what the flights write"));
        }
        if !apart.is_empty() {
            return Err(format!(
                "the committed report is stale; run `cargo xtask ork-flights --library`:\n  {}",
                apart.join("\n  ")
            ));
        }
        println!("ork-flights --library: the committed report matches the flights");
    } else {
        let text = serde_json::to_string_pretty(&report).map_err(|error| error.to_string())? + "\n";
        fs::write(root.join(REPORT_JSON), text)
            .map_err(|error| format!("{REPORT_JSON}: {error}"))?;
        fs::write(root.join(REPORT_MD), &page).map_err(|error| format!("{REPORT_MD}: {error}"))?;
        println!("ork-flights --library: wrote {REPORT_MD} and {REPORT_JSON}");
    }
    print!("{}", summary_lines(&report));
    Ok(())
}

/// A design file found in a record, as [`select`] tells a copy from a new design.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    /// The file's SHA-256.
    pub sha: String,
    /// The name the file gives its rocket.
    pub name: Option<String>,
    /// The ids of the configurations OpenRocket flew, lowercase.
    pub configurations: BTreeSet<String>,
}

/// OpenRocket's id for a configuration whose own id was not a UUID: derived from that id, so two
/// designs can share it. Seen, not documented: 7 of the public record's 63 configuration ids
/// start so, among them the examples' shared ones, and every other id in both records is a
/// version-4 UUID.
const DERIVED_ID: &str = "00000000-0000-0000-";

/// Whether an id is a random (version-4) UUID, as OpenRocket makes for a new configuration.
fn random_uuid(id: &str) -> bool {
    let bytes = id.as_bytes();
    !id.starts_with(DERIVED_ID)
        && bytes.len() == 36
        && bytes.iter().enumerate().all(|(at, byte)| match at {
            8 | 13 | 18 | 23 => *byte == b'-',
            14 => *byte == b'4',
            19 => matches!(byte, b'8' | b'9' | b'a' | b'b'),
            _ => byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase(),
        })
}

impl Candidate {
    /// The rocket, as two readings of it share: its name with its configurations' ids, when it has
    /// both.
    fn named(&self) -> Option<(String, BTreeSet<String>)> {
        let name = self.name.as_deref().filter(|name| !name.is_empty())?;
        (!self.configurations.is_empty()).then(|| (name.to_owned(), self.configurations.clone()))
    }

    /// The rocket by its configurations' ids alone, when every one is a random UUID OpenRocket
    /// made: a rocket renamed keeps them.
    fn configured(&self) -> Option<BTreeSet<String>> {
        let own =
            !self.configurations.is_empty() && self.configurations.iter().all(|id| random_uuid(id));
        own.then(|| self.configurations.clone())
    }
}

/// The public designs, by file and by rocket.
#[derive(Debug, Default)]
pub(crate) struct Known {
    files: BTreeSet<String>,
    named: BTreeSet<(String, BTreeSet<String>)>,
    configured: BTreeSet<BTreeSet<String>>,
}

impl Known {
    fn add(&mut self, candidate: &Candidate) {
        self.files.insert(candidate.sha.clone());
        self.named.extend(candidate.named());
        self.configured.extend(candidate.configured());
    }

    fn holds(&self, candidate: &Candidate) -> bool {
        self.files.contains(&candidate.sha)
            || candidate.named().is_some_and(|r| self.named.contains(&r))
            || candidate
                .configured()
                .is_some_and(|c| self.configured.contains(&c))
    }
}

/// Which candidates are private designs, in the order of their SHA-256, and how many were left out
/// as public or as a second reading of one kept.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Selection {
    pub kept: Vec<usize>,
    pub public: usize,
    pub duplicates: usize,
}

/// Selects the private designs among `candidates`. A candidate is public when its file or its
/// rocket is a public design's, and a duplicate when its file or its rocket is one already kept.
/// A rocket is its name with its configurations' ids, or those ids alone when OpenRocket made
/// them all (a rocket renamed keeps them). The name alone is never enough: OpenRocket gives every
/// new rocket the same default name.
pub(crate) fn select(candidates: &[Candidate], public: &Known) -> Selection {
    let mut order: Vec<usize> = (0..candidates.len()).collect();
    order.sort_by(|a, b| candidates[*a].sha.cmp(&candidates[*b].sha));
    let mut seen = Known::default();
    let mut selection = Selection {
        kept: Vec::new(),
        public: 0,
        duplicates: 0,
    };
    for index in order {
        let candidate = &candidates[index];
        if public.holds(candidate) {
            selection.public += 1;
        } else if seen.holds(candidate) {
            selection.duplicates += 1;
        } else {
            seen.add(candidate);
            selection.kept.push(index);
        }
    }
    selection
}

/// The lowercase ids of the configurations a record's flights of one design are for.
fn configuration_ids(recorded: &[Value]) -> BTreeSet<String> {
    recorded
        .iter()
        .filter_map(|flight| flight["configuration"].as_str())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Flies the library's configurations and builds the anonymised report. No error names a file.
fn fly_library(root: &Path) -> Result<Value, String> {
    let record = read_json(&root.join(RECORD)).map_err(|_| {
        format!(
            "{RECORD} is missing or unreadable; write it with `refs/venv/bin/python \
             validation/oracles/openrocket/flights.py {RECORD} refs --jar`"
        )
    })?;
    for (name, path) in SCRIPTS {
        let bytes = fs::read(root.join(path)).map_err(|error| format!("{path}: {error}"))?;
        if record["inputs_sha256"][name].as_str() != Some(&sha256(&bytes)) {
            return Err(format!(
                "{RECORD} was not written by the current {path}; run it again"
            ));
        }
    }
    let jar = crate::ork_flights::JAR;
    let jar_bytes = fs::read(root.join(jar)).map_err(|error| format!("{jar}: {error}"))?;
    if record["jar_sha256"].as_str() != Some(&sha256(&jar_bytes)) {
        return Err(format!("{RECORD} was not written with {jar}; run it again"));
    }
    let supply = crate::ork_supply::Supply::load(root, true)?;
    if !supply.is_present() {
        return Err(format!(
            "{} is missing: run validation/oracles/openrocket/motor_database.py (ADR-067)",
            crate::ork_supply::RECORD
        ));
    }
    if let Some(failure) = supply.failure() {
        return Err(format!("the supplied curves fail their checks: {failure}"));
    }
    let entries: Vec<&Value> = record["designs"]
        .as_array()
        .ok_or_else(|| format!("{RECORD} has no `designs` list"))?
        .iter()
        .filter(|entry| {
            entry["file"]
                .as_str()
                .is_some_and(|file| file.starts_with(LIBRARY))
        })
        .collect();
    let in_library = entries.len();
    let mut candidates = Vec::new();
    let mut inputs = Vec::new();
    let mut not_opened = 0;
    for entry in entries {
        if entry["driver_error"].is_string() {
            return Err(format!(
                "flights.py failed on a library file; fix it and write {RECORD} again"
            ));
        }
        // A file OpenRocket refused has no flights to compare.
        let Some(recorded) = entry["flights"].as_array() else {
            if entry["refused"].is_string() {
                not_opened += 1;
                continue;
            }
            return Err(format!("{RECORD} holds a library file with no flights"));
        };
        let file = entry["file"].as_str().unwrap_or_default();
        let sha = entry["sha256"].as_str().unwrap_or_default();
        let bytes =
            fs::read(root.join(file)).map_err(|_| "a library file cannot be read".to_owned())?;
        if sha256(&bytes) != sha {
            return Err("a library file is not the file the record flew (SHA-256 differs)".into());
        }
        candidates.push(Candidate {
            sha: sha.to_owned(),
            name: rocket_name(&bytes),
            configurations: configuration_ids(recorded),
        });
        inputs.push((file, &recorded[..], bytes));
    }
    let selection = select(&candidates, &public_designs_known(root)?);
    let kept_shas: Vec<&str> = selection
        .kept
        .iter()
        .map(|&k| candidates[k].sha.as_str())
        .collect();
    let mut rows = Vec::new();
    let mut not_flown = Vec::new();
    let mut flights = Vec::new();
    for (index, &k) in selection.kept.iter().enumerate() {
        let id = format!("C{:02}", index + 1);
        let (file, recorded, bytes) = &inputs[k];
        let label = |place: usize, _: &Value| format!("{id}/{place}");
        let flown = fly_design(file, &id, bytes, recorded, &supply, label, true)?;
        for flight in &flown.flights {
            let mut row = anonymised(flight);
            row["site"] = json!(site_class(recorded, &flight["configuration"]));
            rows.push(row);
        }
        for entry in &flown.not_flown {
            not_flown.push(json!({
                "flight": entry["motors"],
                "aborted": entry["aborted"],
                "why": entry["why"],
            }));
        }
        flights.extend(flown.flights);
    }
    if let Some(why) = rows.iter().find_map(unpublishable) {
        return Err(format!("a row may not be published: {why}"));
    }
    // The outcomes and definitions come from the flights; every spread from the rows, as a
    // reader of the committed rows would compute it.
    let summary = summarise(&flights, &not_flown);
    let spreads = spreads_of(&rows);
    let mut metrics = summary["metrics"].clone();
    for (_, key) in METRICS {
        metrics[key]["scored_by_cause"] = spreads["metrics"][key].clone();
    }
    let public = public_designs(root)?;
    let library = designs_with_all_five(&rows);
    let same_reference = rows.iter().filter(|r| r["reference_same"] == true).count();
    Ok(json!({
        "generated_by": "cargo xtask ork-flights --library",
        "record": RECORD,
        "reference": {
            "tool": "OpenRocket",
            "version": record["openrocket"],
            "jar_sha256": record["jar_sha256"],
        },
        "ids_sha256": sha256(kept_shas.join("\n").as_bytes()),
        "designs": {
            "in_library": in_library,
            "not_opened": not_opened,
            "public": selection.public,
            "duplicates": selection.duplicates,
            "private": selection.kept.len(),
        },
        "summary": {
            "flown": rows.len(),
            "not_flown": not_flown.len(),
            "apogee_over_5_percent": over_5_percent(&rows),
            "reference_same": same_reference,
            "metrics": metrics,
            "mass_and_cg": spreads["mass_and_cg"],
            "designs_with_all_five": library,
            "public_designs_with_all_five": public,
            "together": library + public,
            "bar": BAR,
            "bar_met": library + public >= BAR,
        },
        "flights": rows,
        "not_flown": not_flown,
    }))
}

/// The public record's designs, by file and by rocket.
fn public_designs_known(root: &Path) -> Result<Known, String> {
    let record = read_json(&root.join(crate::ork_flights::RECORD))?;
    let jar = root.join(crate::ork_flights::JAR);
    let examples = crate::ork::examples_in_jar(&jar)?;
    let mut known = Known::default();
    for entry in record["designs"].as_array().into_iter().flatten() {
        let Some(file) = entry["file"].as_str() else {
            continue;
        };
        let bytes = match file.split_once('!') {
            Some((_, inside)) => examples
                .iter()
                .find(|(name, _)| name.ends_with(&format!("!{inside}")))
                .map(|(_, bytes)| bytes.clone())
                .ok_or_else(|| format!("{inside} is not in the jar"))?,
            None => fs::read(root.join(file)).map_err(|error| format!("{file}: {error}"))?,
        };
        known.add(&Candidate {
            sha: entry["sha256"].as_str().unwrap_or_default().to_owned(),
            name: rocket_name(&bytes),
            configurations: configuration_ids(
                entry["flights"].as_array().map_or(&[][..], |f| &f[..]),
            ),
        });
    }
    Ok(known)
}

/// The name a `.ork` gives its rocket, when it reads.
fn rocket_name(bytes: &[u8]) -> Option<String> {
    let read = hpr_io::ork::read(bytes).ok()?;
    let rocket = read.value.document.root.children_named("rocket").next()?;
    Some(rocket.child("name")?.text().trim().to_owned())
}

/// Whether the recorded flight of `configuration` launched at sea level or above it.
fn site_class(recorded: &[Value], configuration: &Value) -> &'static str {
    let altitude = recorded
        .iter()
        .find(|flight| &flight["configuration"] == configuration)
        .and_then(|flight| flight["conditions"]["launch_altitude_m"].as_f64());
    match altitude {
        Some(0.0) => "sea_level",
        Some(_) => "above_sea_level",
        None => "unknown",
    }
}

/// `value` rounded to `digits` decimals, with no negative zero.
pub(crate) fn rounded(value: Option<f64>, digits: i32) -> Value {
    value.filter(|v| v.is_finite()).map_or(Value::Null, |v| {
        let scale = 10f64.powi(digits);
        let r = (v * scale).round() / scale;
        json!(if r == 0.0 { 0.0 } else { r })
    })
}

/// One flight as the report may publish it: differences, never values.
pub(crate) fn anonymised(flight: &Value) -> Value {
    let metrics = &flight["metrics"];
    let [launch, clearance, cg] = mass_and_cg_differences(flight);
    let at = &flight["at_rod_clearance"];
    let (hpr, openrocket) = (&at["hpr"], &at["openrocket"]);
    let reference = openrocket["reference_length_m"].as_f64();
    let cp = hpr["cp_from_nose_m"]
        .as_f64()
        .zip(openrocket["cp_from_nose_m"].as_f64())
        .zip(reference)
        .map(|((h, o), r)| (h - o) / r);
    let reference_same = hpr["reference_length_m"]
        .as_f64()
        .zip(reference)
        .is_some_and(|(h, o)| (h - o).abs() <= 1e-9 * o.abs());
    let outcome = |key: &str| metrics[key]["outcome"]["outcome"].clone();
    let probe = &flight["without_the_overridden_parts"];
    let aborted = flight["aborted"] == true;
    let kept = |value: Option<f64>| if aborted { None } else { value };
    let mut findings: Vec<&str> = flight["design_errors_accepted"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|finding| finding["kind"].as_str())
        .collect();
    findings.sort_unstable();
    findings.dedup();
    json!({
        "flight": flight["motors"],
        "aborted": aborted,
        "termination": flight["termination"],
        "mach": mach_class(flight["max_mach_openrocket"].as_f64()),
        "apogee_percent": rounded(metrics["apogee_m"]["relative_percent"].as_f64(), DIGITS),
        "apogee_outcome": outcome("apogee_m"),
        "max_speed_percent": rounded(metrics["max_speed_m_s"]["relative_percent"].as_f64(), DIGITS),
        "max_speed_outcome": outcome("max_speed_m_s"),
        "margin_cal": rounded(metrics["rod_clearance_margin_cal"]["difference"].as_f64(), DIGITS),
        "margin_outcome": outcome("rod_clearance_margin_cal"),
        "launch_mass_percent": rounded(kept(launch), MASS_DIGITS),
        "rod_clearance_mass_percent": rounded(kept(clearance), MASS_DIGITS),
        "rod_clearance_cg_cal": rounded(kept(cg), DIGITS),
        "rod_clearance_cp_cal": rounded(kept(cp), DIGITS),
        "reference_same": reference_same,
        "chute_early_s": rounded(flight["deployed_before_apogee_s"].as_f64(), CHUTE_DIGITS),
        "drag_overrides_not_applied": flight["drag_overrides_not_applied"]
            .as_array()
            .map(Vec::len),
        "without_the_overridden_parts": if probe.is_null() {
            Value::Null
        } else {
            json!({
                "apogee_percent": rounded(probe["apogee_m"]["relative_percent"].as_f64(), DIGITS),
                "max_speed_percent":
                    rounded(probe["max_speed_m_s"]["relative_percent"].as_f64(), DIGITS),
            })
        },
        "design_checks": findings,
    })
}

/// OpenRocket's largest Mach number of a flight, as a class.
fn mach_class(mach: Option<f64>) -> &'static str {
    match mach {
        Some(m) if m < 0.8 => "subsonic",
        Some(m) if m <= 1.2 => "transonic",
        Some(_) => "supersonic",
        None => "unknown",
    }
}

/// The cause an anonymised row's difference in the metric `key` is summarised under, as
/// [`crate::ork_flights`] names it.
pub(crate) fn row_cause(row: &Value, key: &str) -> &'static str {
    if key == "margin_cal" {
        NO_NAMED_CAUSE
    } else if !row["drag_overrides_not_applied"].is_null() {
        DRAG_OVERRIDE
    } else if key == "apogee_percent" && !row["chute_early_s"].is_null() {
        EARLY_CHUTE
    } else {
        NO_NAMED_CAUSE
    }
}

/// The rows whose apogee is more than 5% from OpenRocket's.
fn over_5_percent(rows: &[Value]) -> usize {
    rows.iter()
        .filter(|row| {
            row["apogee_percent"]
                .as_f64()
                .is_some_and(|p| p.abs() > 5.0)
        })
        .count()
}

/// The designs, by the id before the `/`, with a flight compared in all five spreads.
pub(crate) fn designs_with_all_five(rows: &[Value]) -> usize {
    rows.iter()
        .filter(|row| FIVE.iter().all(|key| row[*key].is_f64()))
        .filter_map(|row| row["flight"].as_str()?.split('/').next())
        .collect::<BTreeSet<_>>()
        .len()
}

/// The public report's designs with a flight compared in all five spreads.
pub(crate) fn public_designs(root: &Path) -> Result<usize, String> {
    let report = read_json(&root.join(crate::ork_flights::REPORT_JSON))?;
    let empty = Vec::new();
    let rows: Vec<Value> = report["flights"]
        .as_array()
        .unwrap_or(&empty)
        .iter()
        .map(|flight| {
            let mut row = anonymised(flight);
            row["flight"] = json!(format!(
                "{}/",
                flight["design"].as_str().unwrap_or_default()
            ));
            row
        })
        .collect();
    Ok(designs_with_all_five(&rows))
}

/// The spreads of the rows: each metric by cause, and the mass and centre of mass over the
/// flights not aborted.
pub(crate) fn spreads_of(rows: &[Value]) -> Value {
    let mut metrics = serde_json::Map::new();
    for ((_, key), row_key) in
        METRICS
            .iter()
            .zip(["apogee_percent", "max_speed_percent", "margin_cal"])
    {
        let mut groups: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for row in rows {
            if let Some(value) = row[row_key].as_f64() {
                groups
                    .entry(row_cause(row, row_key))
                    .or_default()
                    .push(value);
            }
        }
        let groups: serde_json::Map<String, Value> = groups
            .iter()
            .map(|(cause, values)| ((*cause).to_owned(), spread(values)))
            .collect();
        metrics.insert((*key).to_owned(), Value::Object(groups));
    }
    let column = |key: &str| -> Vec<f64> {
        rows.iter()
            .filter(|row| row["aborted"] != true)
            .filter_map(|row| row[key].as_f64())
            .collect()
    };
    json!({
        "metrics": metrics,
        "mass_and_cg": {
            "launch_mass_percent": spread(&column("launch_mass_percent")),
            "rod_clearance_mass_percent": spread(&column("rod_clearance_mass_percent")),
            "rod_clearance_cg_cal": spread(&column("rod_clearance_cg_cal")),
        },
    })
}

/// The summary, as lines for the terminal and the page.
fn summary_lines(report: &Value) -> String {
    let summary = &report["summary"];
    let designs = &report["designs"];
    let line = |label: &str, spread: &Value, unit: &str, digits: usize, verb: &str| {
        format!(
            "- {label}: {} {verb}, median {}{unit}, mean absolute {}{unit}, from {}{unit} to {}{unit}\n",
            spread["count"],
            signed(&spread["median"], digits),
            fixed(&spread["mean_absolute"], digits),
            signed(&spread["min"], digits),
            signed(&spread["max"], digits),
        )
    };
    let mut out = format!(
        "- designs: {} files in the library; {} OpenRocket did not open, {} public designs (the \
         same file, or an edited copy of the same rocket) left to the public report, {} found twice; \
         {} private designs, {} with a flight compared in all five spreads\n\
         - with the public report's {}, {} designs in all, against [M2.2][m2-2]'s bar of {} ({})\n\
         - configurations flown: {} ({} OpenRocket flew are not flown by hpr); apogee more than 5% \
         from OpenRocket's: {}; the same reference diameter as OpenRocket's: {}\n",
        designs["in_library"],
        designs["not_opened"],
        designs["public"],
        designs["duplicates"],
        designs["private"],
        summary["designs_with_all_five"],
        summary["public_designs_with_all_five"],
        summary["together"],
        summary["bar"],
        if summary["bar_met"] == true {
            "met"
        } else {
            "not met"
        },
        summary["flown"],
        summary["not_flown"],
        summary["apogee_over_5_percent"],
        summary["reference_same"],
    );
    let empty = serde_json::Map::new();
    for (key, label, unit, digits) in [
        ("apogee_m", "apogee", "%", 2),
        ("max_speed_m_s", "largest speed", "%", 2),
        (
            "rod_clearance_margin_cal",
            "margin at rod clearance",
            " cal",
            4,
        ),
    ] {
        let groups = summary["metrics"][key]["scored_by_cause"]
            .as_object()
            .unwrap_or(&empty);
        for (cause, spread) in groups {
            out.push_str(&line(
                &format!("{label}, {cause}"),
                spread,
                unit,
                digits,
                "scored",
            ));
        }
    }
    let mass_and_cg = &summary["mass_and_cg"];
    for (key, label, unit, digits) in [
        ("launch_mass_percent", "mass at launch", "%", 3),
        (
            "rod_clearance_mass_percent",
            "mass at rod clearance",
            "%",
            3,
        ),
        (
            "rod_clearance_cg_cal",
            "centre of mass at rod clearance",
            " cal",
            4,
        ),
    ] {
        out.push_str(&line(label, &mass_and_cg[key], unit, digits, "compared"));
    }
    out
}

/// A difference cell, or the outcome when there is no number.
fn cell(row: &Value, key: &str, outcome: Option<&str>, digits: usize, unit: &str) -> String {
    match row[key].as_f64() {
        Some(value) => format!("{value:+.digits$}{unit}"),
        None => outcome
            .and_then(|outcome| row[outcome].as_str())
            .unwrap_or("—")
            .to_owned(),
    }
}

/// The report as a page.
pub(crate) fn page(report: &Value) -> String {
    let summary = &report["summary"];
    let empty = Vec::new();
    let rows = report["flights"].as_array().unwrap_or(&empty);
    let flown_designs = rows
        .iter()
        .filter_map(|row| row["flight"].as_str()?.split('/').next())
        .collect::<BTreeSet<_>>()
        .len();
    let mut out = String::new();
    out.push_str("# hpr against OpenRocket 24.12's flights of the private design library\n\n");
    let widest = rows
        .iter()
        .filter_map(|row| row["margin_cal"].as_f64())
        .fold(f64::NEG_INFINITY, f64::max);
    let margin = if widest > 0.0 {
        format!(
            "hpr's stability margin is larger than OpenRocket's by up to {widest:.4} calibres, so \
             it can call a rocket more stable than OpenRocket does"
        )
    } else {
        "hpr's stability margin is nowhere larger than OpenRocket's".to_owned()
    };
    out.push_str(&format!(
        "This report compares hpr's flights of {flown_designs} of the library's {} private \
         designs with OpenRocket 24.12's, and publishes only the differences. It is a \
         code-to-code comparison with no target: agreeing with OpenRocket is not agreeing with a \
         real flight. {margin} \
         (open leads: [#172](https://github.com/nrdptel/hpr-sim/issues/172) and \
         [#186](https://github.com/nrdptel/hpr-sim/issues/186)). With the [public \
         report](openrocket-flights.md) it covers {} designs of the {} that [M2.2][m2-2] (the \
         OpenRocket comparison) asks for: {}. Nobody without the private library can fly these \
         again; CI checks only that this report adds up and names nothing of a design.\n\n",
        report["designs"]["private"],
        summary["together"],
        summary["bar"],
        if summary["bar_met"] == true {
            "met"
        } else {
            "not met"
        },
    ));
    out.push_str(&format!(
        "- Written by `cargo xtask ork-flights --library` ([M2.2e3][m2-2e3], hpr's flights of \
         the library; decision [ADR-072][adr-072]) from OpenRocket's flights of it \
         ([M2.2e2][m2-2e2]), flown and compared as in the public report, by the same \
         definitions. The explanation is on the [documentation site][site].\n\
         - A design is an id, `C01` onwards, in the order of its file's SHA-256 hash, and a \
         flight is the design's id and the configuration's place in the file: `C09/2` is the \
         second configuration of `C09`. The ids hold while the library's files do; `ids_sha256` \
         in the report's JSON changes when they would move.\n\
         - Differences (Δ) are hpr less OpenRocket: apogee, largest speed and mass in per cent \
         of OpenRocket's; the stability margin, the centre of mass (CG) and the centre of \
         pressure (CP) at rod clearance in OpenRocket's calibres (its reference diameter), the CG \
         and CP positive when hpr's is further aft. A positive margin Δ means hpr calls the \
         rocket more stable; margin Δ = CP Δ − CG Δ when the reference diameters agree. The \
         *five spreads* are apogee, largest speed, margin, mass and CG.\n\
         - A flight is *scored* in a spread when both programs have the number; a *named cause* \
         is a known difference in how the two flights were set up that can move a number, and \
         flights with one get their own summary line, for that spread only. The *reference \
         parachute* is OpenRocket's.\n\
         - *Mach* is the class of OpenRocket's largest Mach number: subsonic below 0.8, \
         transonic to 1.2, supersonic above. *Site* is whether the launch site is at sea level.\n\
         - No value of a design is written: with its difference it would give back \
         OpenRocket's. The JSON holds differences to {DIGITS} decimals (masses to {MASS_DIGITS}), \
         so no rounding residue gives a value back, and the tables here show fewer; the one time \
         given is how early OpenRocket's parachute opened, to {CHUTE_DIGITS} decimals of a \
         second.\n\n\
         [m2-2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2\n\
         [m2-2e3]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e3\n\
         [m2-2e2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2e2\n\
         [adr-072]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-072-hprs-flights-of-the-private-library-under-anonymised-ids-2026-09-25\n\
         [site]: https://nrdptel.github.io/hpr-sim/format/ork.html#hprs-flights-of-the-private-designs\n\n",
    ));
    out.push_str(&summary_lines(report));
    out.push_str(
        "\n| flight | Mach | site | apogee Δ | max speed Δ | margin Δ (cal) | CG Δ (cal) \
         | CP Δ (cal) | launch mass Δ | rod-clearance mass Δ | named cause |\n",
    );
    out.push_str("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for row in rows {
        let mut causes = Vec::new();
        if let Some(early) = row["chute_early_s"].as_f64() {
            causes.push(format!("chute {early:.2} s early"));
        }
        if let Some(parts) = row["drag_overrides_not_applied"].as_u64() {
            let probe = &row["without_the_overridden_parts"];
            let mut cause = format!(
                "{parts} part{} with a drag override",
                if parts == 1 { "" } else { "s" }
            );
            if let (Some(apogee), Some(speed)) = (
                probe["apogee_percent"].as_f64(),
                probe["max_speed_percent"].as_f64(),
            ) {
                cause.push_str(&format!(" (without them {apogee:+.2}% and {speed:+.2}%)"));
            }
            causes.push(cause);
        }
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            row["flight"].as_str().unwrap_or_default(),
            row["mach"].as_str().unwrap_or_default(),
            row["site"].as_str().unwrap_or_default().replace('_', " "),
            cell(row, "apogee_percent", Some("apogee_outcome"), 2, "%"),
            cell(row, "max_speed_percent", Some("max_speed_outcome"), 2, "%"),
            cell(row, "margin_cal", Some("margin_outcome"), 4, ""),
            cell(row, "rod_clearance_cg_cal", None, 4, ""),
            cell(row, "rod_clearance_cp_cal", None, 4, ""),
            cell(row, "launch_mass_percent", None, 3, "%"),
            cell(row, "rod_clearance_mass_percent", None, 3, "%"),
            causes.join("; "),
        ));
    }
    let mut legend = Vec::new();
    if rows.iter().any(|row| !row["chute_early_s"].is_null()) {
        legend.push(
            "*Chute s early*: OpenRocket's parachute opened that long before the apogee of the \
             same flight with nothing deployed; hpr flies no parachute from a `.ork` yet. An \
             early parachute lowers OpenRocket's apogee, so it can explain an apogee Δ above \
             zero, not one below."
                .to_owned(),
        );
    }
    if rows
        .iter()
        .any(|row| !row["drag_overrides_not_applied"].is_null())
    {
        legend.push(
            "*A drag override*: parts that state their own drag coefficient, which hpr reads but \
             does not apply ([#165](https://github.com/nrdptel/hpr-sim/issues/165)); where every \
             such part states zero, the same flight by hpr with those parts removed gives the \
             apogee and largest speed differences in brackets, a probe, not the override."
                .to_owned(),
        );
    }
    if !legend.is_empty() {
        out.push('\n');
        out.push_str(&legend.join(" "));
        out.push('\n');
    }
    let checked: Vec<&Value> = rows
        .iter()
        .filter(|row| {
            row["design_checks"]
                .as_array()
                .is_some_and(|c| !c.is_empty())
        })
        .collect();
    if !checked.is_empty() {
        out.push_str(
            "\nWhat hpr's [design checks](https://nrdptel.github.io/hpr-sim/physics/design.html#checks) \
             object to, by kind, in flights flown anyway as OpenRocket flies them:\n\n\
             | flight | findings |\n|---|---|\n",
        );
        for row in checked {
            let kinds: Vec<&str> = row["design_checks"]
                .as_array()
                .unwrap_or(&empty)
                .iter()
                .filter_map(Value::as_str)
                .collect();
            out.push_str(&format!(
                "| {} | `{}` |\n",
                row["flight"].as_str().unwrap_or_default(),
                kinds.join("`, `")
            ));
        }
    }
    let not_flown = report["not_flown"].as_array().unwrap_or(&empty);
    if !not_flown.is_empty() {
        let mut why: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for entry in not_flown {
            why.entry(entry["why"].as_str().unwrap_or_default())
                .or_default()
                .push(entry["flight"].as_str().unwrap_or_default());
        }
        out.push_str(
            "\nConfigurations OpenRocket flew that hpr does not fly here, by reason:\n\n\
             | why | how many | flights |\n|---|---:|---|\n",
        );
        for (why, ids) in why {
            out.push_str(&format!("| {why} | {} | {} |\n", ids.len(), ids.join(", ")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed() -> Value {
        let root = crate::ork::root().unwrap();
        read_json(&root.join(REPORT_JSON)).unwrap()
    }

    #[test]
    fn the_summary_is_the_committed_rows() {
        let report = committed();
        let rows = report["flights"].as_array().unwrap();
        let summary = &report["summary"];
        let spreads = spreads_of(rows);
        for (_, key) in METRICS {
            assert_eq!(
                spreads["metrics"][key], summary["metrics"][key]["scored_by_cause"],
                "{key}"
            );
        }
        assert_eq!(spreads["mass_and_cg"], summary["mass_and_cg"]);
        assert_eq!(summary["flown"], rows.len());
        assert_eq!(
            summary["not_flown"],
            report["not_flown"].as_array().unwrap().len()
        );
        assert_eq!(summary["apogee_over_5_percent"], over_5_percent(rows));
        assert_eq!(
            summary["designs_with_all_five"],
            designs_with_all_five(rows)
        );
        let same = rows.iter().filter(|r| r["reference_same"] == true).count();
        assert_eq!(summary["reference_same"], same);
        for (key, outcome) in [
            ("apogee_m", "apogee_outcome"),
            ("max_speed_m_s", "max_speed_outcome"),
            ("rod_clearance_margin_cal", "margin_outcome"),
        ] {
            let mut counts: BTreeMap<String, u64> = BTreeMap::new();
            for row in rows {
                *counts
                    .entry(row[outcome].as_str().unwrap_or("missing").to_owned())
                    .or_default() += 1;
            }
            assert_eq!(summary["metrics"][key]["outcomes"], json!(counts), "{key}");
        }
        let designs = &report["designs"];
        let parts: u64 = ["not_opened", "public", "duplicates", "private"]
            .iter()
            .map(|key| designs[*key].as_u64().unwrap())
            .sum();
        assert_eq!(designs["in_library"], parts);
    }

    #[test]
    fn the_count_toward_twenty_designs_is_the_two_reports() {
        // M2.2's *done when* asks for at least 20 designs with an error distribution of apogee,
        // largest speed, margin, mass and centre of mass. The report counts them from both
        // reports and says whether the bar is met; M2.2e5 is met only when it is (ADR-072).
        let root = crate::ork::root().unwrap();
        let report = committed();
        let public = public_designs(&root).unwrap();
        let library = designs_with_all_five(report["flights"].as_array().unwrap());
        let summary = &report["summary"];
        assert_eq!(summary["public_designs_with_all_five"], public);
        assert_eq!(summary["together"], public + library);
        assert_eq!(summary["bar"], BAR);
        assert_eq!(summary["bar_met"], public + library >= BAR);
    }

    /// Every word a row may hold, by key: ids aside, a closed list. A design's own words are in
    /// none of them.
    fn allowed(key: &str) -> Vec<String> {
        use crate::ork_flights::*;
        use hpr_io::ork::NotFlown;
        let words: &[&str] = match key {
            "termination" => &[
                "ground_hit",
                "no_liftoff",
                "stalled_on_rail",
                "time_cap",
                "step_limit",
                "separated",
            ],
            "apogee_outcome" | "max_speed_outcome" | "margin_outcome" => {
                &["scored", "withheld", "failed"]
            }
            "mach" => &["subsonic", "transonic", "supersonic", "unknown"],
            "site" => &["sea_level", "above_sea_level", "unknown"],
            "design_checks" => &[
                "motor_wider_than_mount",
                "motor_outside_mount",
                "motor_past_mount_top",
                "attachment_off_body",
                "attachment_past_body_end",
                "part_outside_rocket",
                "internal_part_past_parent_end",
                "internal_part_wider_than_parent",
                "cluster_tubes_overlap",
                "ring_overlaps_inner_tube",
                "centre_outside_rocket",
                "radius_step",
                "no_nose_cone",
            ],
            "why" => {
                let importer = [
                    NotFlown::UnreadMotor,
                    NotFlown::NoMotor,
                    NotFlown::InactiveStage,
                    NotFlown::NoCurve,
                    NotFlown::NoSize,
                    NotFlown::IgnitionNotFlown,
                    NotFlown::AirframeNotAsWritten,
                    NotFlown::SeparationNotFlown,
                ]
                .map(crate::ork_motors::not_flown);
                return importer
                    .iter()
                    .flat_map(|why| [(*why).to_owned(), format!("{RENAMED}{why}")])
                    .chain(
                        [
                            NO_SUCH_CONFIGURATION,
                            ROD_NOT_VERTICAL,
                            WIND,
                            NOT_STANDARD_AIR,
                            CURVE_BY_NAME,
                            NOT_PLACED,
                            FLIGHT_FAILED,
                            REFUSED,
                        ]
                        .map(str::to_owned),
                    )
                    .collect();
            }
            _ => &[],
        };
        words.iter().map(|w| (*w).to_owned()).collect()
    }

    #[test]
    fn the_report_names_no_file_part_or_motor() {
        let report = committed();
        let text = fs::read_to_string(crate::ork::root().unwrap().join(REPORT_MD)).unwrap();
        assert!(!text.contains("refs/") && !text.contains("loft-fixtures"));
        let id = |s: &str| {
            let (design, place) = s.split_once('/').unwrap_or((s, ""));
            design.len() == 3
                && design.starts_with('C')
                && design[1..].chars().all(|c| c.is_ascii_digit())
                && !place.is_empty()
                && place.chars().all(|c| c.is_ascii_digit())
        };
        let rows = report["flights"].as_array().unwrap();
        for row in rows.iter().chain(report["not_flown"].as_array().unwrap()) {
            assert!(id(row["flight"].as_str().unwrap()), "{}", row["flight"]);
            for (key, value) in row.as_object().unwrap() {
                let words: Vec<&str> = match value {
                    Value::String(s) if key != "flight" => vec![s],
                    Value::Array(items) => items.iter().map(|i| i.as_str().unwrap()).collect(),
                    Value::Object(inner) => {
                        assert!(inner.values().all(|v| v.is_number() || v.is_null()));
                        continue;
                    }
                    _ => continue,
                };
                let allowed = allowed(key);
                for word in words {
                    assert!(
                        allowed.iter().any(|a| a == word),
                        "`{key}` holds a word not on its list: {word}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_committed_row_may_be_published() {
        let report = committed();
        for row in report["flights"].as_array().unwrap() {
            assert_eq!(unpublishable(row), None);
        }
        for row in report["not_flown"].as_array().unwrap() {
            let keys: Vec<&str> = row
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(keys, ["aborted", "flight", "why"]);
        }
    }

    #[test]
    fn a_row_with_a_value_or_a_residue_may_not_be_published() {
        let row = json!({"flight": "C01/1", "apogee_percent": -1.25, "launch_mass_percent": 0.004});
        assert_eq!(unpublishable(&row), None);
        let mut value = row.clone();
        value["apogee_m"] = json!(1234.5);
        assert!(unpublishable(&value).unwrap().contains("apogee_m"));
        let mut residue = row.clone();
        residue["launch_mass_percent"] = json!(3.0e-14);
        assert!(
            unpublishable(&residue)
                .unwrap()
                .contains("launch_mass_percent")
        );
        let mut mass = row.clone();
        mass["launch_mass_percent"] = json!(0.0042);
        assert!(unpublishable(&mass).is_some());
        let mut nested = row;
        nested["without_the_overridden_parts"] = json!({"apogee_percent": 0.123_456_7});
        assert!(unpublishable(&nested).is_some());
    }

    #[test]
    fn rounding_drops_a_residue_and_a_negative_zero() {
        assert_eq!(rounded(Some(3.0e-14), DIGITS), json!(0.0));
        assert_eq!(rounded(Some(-1e-12), DIGITS), json!(0.0));
        assert_eq!(rounded(Some(-0.004_218_7), DIGITS), json!(-0.004_219));
        assert_eq!(rounded(Some(2.345), 2), json!(2.35));
        assert_eq!(rounded(Some(f64::NAN), DIGITS), Value::Null);
        assert_eq!(rounded(None, DIGITS), Value::Null);
    }

    #[test]
    fn a_row_keeps_differences_and_drops_values() {
        let flight = json!({
            "motors": "C01/2", "aborted": false, "termination": "ground_hit",
            "max_mach_openrocket": 0.95, "deployed_before_apogee_s": 0.123_456,
            "drag_overrides_not_applied": ["Tail cone"],
            "without_the_overridden_parts": {
                "apogee_m": {"relative_percent": -0.5}, "max_speed_m_s": {"relative_percent": 0.1}
            },
            "design_errors_accepted": [{"kind": "b", "component": "x"}, {"kind": "a"}, {"kind": "b"}],
            "metrics": {
                "apogee_m": {"openrocket": 100.0, "hpr": 90.0, "difference": -10.0,
                             "relative_percent": -10.0, "outcome": {"outcome": "scored"}},
                "max_speed_m_s": {"openrocket": 50.0, "hpr": 51.0, "difference": 1.0,
                                  "relative_percent": 2.0, "outcome": {"outcome": "scored"}},
                "rod_clearance_margin_cal": {"openrocket": 2.0, "hpr": 1.9, "difference": -0.1,
                                             "relative_percent": -5.0,
                                             "outcome": {"outcome": "scored"}},
            },
            "launch_mass_kg": {"openrocket": 2.0, "hpr": 2.02},
            "at_rod_clearance": {
                "openrocket": {"mass_kg": 1.0, "cg_from_nose_m": 0.5, "cp_from_nose_m": 0.7,
                               "reference_length_m": 0.1},
                "hpr": {"mass_kg": 0.99, "cg_from_nose_m": 0.51, "cp_from_nose_m": 0.72,
                        "reference_length_m": 0.1},
            },
        });
        let row = anonymised(&flight);
        assert_eq!(row["mach"], "transonic");
        assert_eq!(row["apogee_percent"], -10.0);
        assert_eq!(row["margin_cal"], -0.1);
        assert_eq!(row["launch_mass_percent"], 1.0);
        assert_eq!(row["rod_clearance_mass_percent"], -1.0);
        assert_eq!(row["rod_clearance_cg_cal"], 0.1);
        assert_eq!(row["rod_clearance_cp_cal"], 0.2);
        assert_eq!(row["reference_same"], true);
        assert_eq!(row["chute_early_s"], 0.12);
        assert_eq!(row["drag_overrides_not_applied"], 1);
        assert_eq!(row["without_the_overridden_parts"]["apogee_percent"], -0.5);
        assert_eq!(row["design_checks"], json!(["a", "b"]));
        assert_eq!(row_cause(&row, "apogee_percent"), DRAG_OVERRIDE);
        assert_eq!(row_cause(&row, "margin_cal"), NO_NAMED_CAUSE);
        // No value of the design survives.
        let text = row.to_string();
        for value in ["100.0", "90.0", "\"Tail cone\"", "\"x\"", "0.95", "0.72"] {
            assert!(!text.contains(value), "{value} in {text}");
        }
        // An aborted flight's mass is not compared, as in the public report.
        let mut aborted = flight.clone();
        aborted["aborted"] = json!(true);
        assert!(anonymised(&aborted)["launch_mass_percent"].is_null());
    }

    #[test]
    fn the_mach_classes_have_their_edges() {
        assert_eq!(mach_class(Some(0.799)), "subsonic");
        assert_eq!(mach_class(Some(0.8)), "transonic");
        assert_eq!(mach_class(Some(1.2)), "transonic");
        assert_eq!(mach_class(Some(1.201)), "supersonic");
        assert_eq!(mach_class(None), "unknown");
    }

    #[test]
    fn the_site_is_the_recorded_flights() {
        let recorded = [
            json!({"configuration": "a", "conditions": {"launch_altitude_m": 0.0}}),
            json!({"configuration": "b", "conditions": {"launch_altitude_m": 1400.0}}),
        ];
        assert_eq!(site_class(&recorded, &json!("a")), "sea_level");
        assert_eq!(site_class(&recorded, &json!("b")), "above_sea_level");
        assert_eq!(site_class(&recorded, &json!("c")), "unknown");
    }

    #[test]
    fn designs_are_counted_once_and_only_with_all_five() {
        let full = |id: &str| {
            let mut row = json!({"flight": id});
            for key in FIVE {
                row[key] = json!(0.5);
            }
            row
        };
        let mut missing = full("C03/1");
        missing["margin_cal"] = Value::Null;
        let rows = [full("C01/1"), full("C01/2"), full("C02/1"), missing];
        assert_eq!(designs_with_all_five(&rows), 2);
    }

    fn candidate(sha: &str, name: &str, configurations: &[&str]) -> Candidate {
        Candidate {
            sha: sha.to_owned(),
            name: Some(name.to_owned()),
            configurations: configurations.iter().map(|c| (*c).to_owned()).collect(),
        }
    }

    #[test]
    fn a_copy_is_its_file_or_its_name_with_its_configurations() {
        let mut public = Known::default();
        public.add(&candidate("p1", "Example", &["x", "y"]));
        public.add(&candidate("p2", "Rocket", &["z"]));
        let candidates = [
            candidate("d", "Mine", &["m"]),          // private
            candidate("p1", "Example", &["x", "y"]), // the very file
            candidate("c", "Example", &["x", "y"]),  // the same rocket saved again
            candidate("b", "Rocket", &["q"]),        // the default name alone: private
            candidate("a", "Example", &["x"]),       // a name with other configurations: private
            candidate("e", "Mine", &["m"]),          // `d` saved again: counted once
            candidate("d", "Other", &["n"]),         // `d`'s very file twice
        ];
        let selection = select(&candidates, &public);
        // Kept in SHA-256 order: `a`, `b`, `d` (the first `d`).
        assert_eq!(selection.kept, vec![4, 3, 0]);
        assert_eq!(selection.public, 2);
        assert_eq!(selection.duplicates, 2);
        // A rocket renamed keeps OpenRocket's configuration ids, and is still public; ids OpenRocket
        // derived from a design's own are not enough without the name.
        let uuid = |n: u8| format!("{n:08x}-1111-4111-8111-111111111111");
        let derived = |n: u8| format!("{DERIVED_ID}ffff-{n:012x}");
        let mut known = Known::default();
        known.add(&candidate("p", "Example", &[&uuid(1), &uuid(2)]));
        known.add(&candidate("q", "Other", &[&derived(1)]));
        let renamed = candidate("r", "Mine", &[&uuid(2), &uuid(1)]);
        let shared = candidate("s", "Mine", &[&derived(1)]);
        let selection = select(&[renamed, shared], &known);
        assert_eq!((selection.kept, selection.public), (vec![1], 1));
        // A rocket with no name, or no configurations, is matched by its file only.
        let unnamed = Candidate {
            name: None,
            ..candidate("f", "", &["x", "y"])
        };
        let bare = candidate("g", "Example", &[]);
        let selection = select(&[unnamed, bare], &public);
        assert_eq!(selection.kept, vec![0, 1]);
    }

    #[test]
    fn only_a_random_uuid_is_a_rocket_by_its_ids_alone() {
        assert!(random_uuid("0f1e2d3c-4b5a-4968-8776-a5b4c3d2e1f0"));
        assert!(random_uuid("0f1e2d3c-4b5a-4968-b776-a5b4c3d2e1f0"));
        assert!(!random_uuid("00000000-0000-0000-ffff-000000000001"));
        assert!(!random_uuid("0f1e2d3c-4b5a-1968-8776-a5b4c3d2e1f0"));
        assert!(!random_uuid("0f1e2d3c-4b5a-4968-c776-a5b4c3d2e1f0"));
        assert!(!random_uuid("0f1e2d3c-4b5a-4968-8776-a5b4c3d2e1fg"));
        assert!(!random_uuid("0f1e2d3c_4b5a-4968-8776-a5b4c3d2e1f0"));
        assert!(!random_uuid("0F1E2D3C-4B5A-4968-8776-A5B4C3D2E1F0"));
        assert!(!random_uuid("0f1e2d3c-4b5a-4968-8776-a5b4c3d2e1f"));
    }
}
