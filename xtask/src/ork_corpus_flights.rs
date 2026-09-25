//! `cargo xtask ork-flights --corpus`: counts OpenRocket's flights of the private design library
//! (M2.2e2, ADR-070).
//!
//! `flights.py` writes OpenRocket 24.12's calm-air flight of every motor configuration of every
//! design under `refs/` to [`RECORD`], which is gitignored: the designs are other people's, and a
//! flight's numbers can identify one. This prints only counts, which may be published, split by
//! where each design came from: the private library `refs/loft-fixtures/`, the examples inside
//! OpenRocket's jar, and the rest of `refs/`. A file found twice (the same SHA-256) is counted
//! once per place it was found, and the distinct files are counted too.

use std::collections::BTreeMap;
use std::fs;

use serde_json::Value;

/// OpenRocket's flights of everything under `refs/` and the jar's examples.
pub(crate) const RECORD: &str = "corpus-out/openrocket-flights.json";

/// Where a design in the record came from.
fn source(file: &str) -> &'static str {
    if file.starts_with("refs/loft-fixtures/") {
        "refs/loft-fixtures"
    } else if file.contains(".jar!") {
        "OpenRocket's examples"
    } else {
        "elsewhere under refs/"
    }
}

/// The counts for one source.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    /// Design files found.
    pub designs: usize,
    /// Distinct files among them, by SHA-256.
    pub distinct: usize,
    /// Files OpenRocket refused to open.
    pub refused_designs: usize,
    /// Files the driver failed on (a script fault, not a measurement).
    pub driver_errors: usize,
    /// Configurations declared by the files OpenRocket opened.
    pub configurations: usize,
    /// Of those, configurations with no motor, which cannot fly.
    pub no_motor: usize,
    /// Configurations OpenRocket refused to simulate.
    pub refused_flights: usize,
    /// Configurations flown whose run OpenRocket aborted (a `SIM_ABORT` event).
    pub aborted: usize,
    /// Configurations flown to the end.
    pub completed: usize,
    /// Of those completed, flights with a stability margin at rod clearance.
    pub with_margin: usize,
    /// Designs with at least one configuration flown to the end.
    pub designs_completed: usize,
}

/// Counts the record's designs and flights by [`source`].
pub(crate) fn count(record: &Value) -> Result<BTreeMap<&'static str, Counts>, String> {
    let designs = record["designs"]
        .as_array()
        .ok_or("the record has no `designs` list")?;
    let mut counts: BTreeMap<&'static str, Counts> = BTreeMap::new();
    let mut digests: BTreeMap<&'static str, Vec<&str>> = BTreeMap::new();
    for design in designs {
        let file = design["file"]
            .as_str()
            .ok_or("a design in the record has no `file`")?;
        let place = source(file);
        let tally = counts.entry(place).or_default();
        tally.designs += 1;
        if let Some(digest) = design["sha256"].as_str() {
            digests.entry(place).or_default().push(digest);
        }
        if design.get("driver_error").is_some() {
            tally.driver_errors += 1;
            continue;
        }
        if design.get("refused").is_some() {
            tally.refused_designs += 1;
            continue;
        }
        let flights = design["flights"]
            .as_array()
            .ok_or_else(|| format!("{file}: opened, but no `flights` list"))?;
        let mut any_completed = false;
        for flight in flights {
            tally.configurations += 1;
            if flight["has_motors"] != Value::Bool(true) {
                tally.no_motor += 1;
            } else if flight.get("refused").is_some() {
                tally.refused_flights += 1;
            } else if flight["aborted"] == Value::Bool(true) {
                tally.aborted += 1;
            } else {
                tally.completed += 1;
                any_completed = true;
                if flight["rod_clearance"]["stability_cal"].is_number() {
                    tally.with_margin += 1;
                }
            }
        }
        if any_completed {
            tally.designs_completed += 1;
        }
    }
    for (place, mut found) in digests {
        found.sort_unstable();
        found.dedup();
        if let Some(tally) = counts.get_mut(place) {
            tally.distinct = found.len();
        }
    }
    Ok(counts)
}

/// The counts as lines of text, one block per source.
pub(crate) fn lines(counts: &BTreeMap<&'static str, Counts>) -> String {
    let mut out = String::new();
    for (place, c) in counts {
        out += &format!(
            "{place}:\n  designs: {} files ({} distinct), {} refused by OpenRocket, {} driver errors\n  \
             configurations: {} declared, {} with no motor, {} refused, {} aborted, {} flown to the end \
             ({} with a margin at rod clearance)\n  designs with a flight to the end: {}\n",
            c.designs,
            c.distinct,
            c.refused_designs,
            c.driver_errors,
            c.configurations,
            c.no_motor,
            c.refused_flights,
            c.aborted,
            c.completed,
            c.with_margin,
            c.designs_completed,
        );
    }
    out
}

pub(crate) fn run() -> Result<(), String> {
    let root = crate::ork::root()?;
    let path = root.join(RECORD);
    let text = fs::read_to_string(&path).map_err(|error| {
        format!(
            "{RECORD}: {error}; write it with `refs/venv/bin/python \
             validation/oracles/openrocket/flights.py {RECORD} refs --jar`"
        )
    })?;
    let record: Value =
        serde_json::from_str(&text).map_err(|error| format!("{RECORD}: {error}"))?;
    print!("{}", lines(&count(&record)?));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn each_outcome_is_counted_once_under_its_source() {
        let record = json!({"designs": [
            {"file": "refs/loft-fixtures/a.ork", "sha256": "x", "flights": [
                {"has_motors": false},
                {"has_motors": true, "refused": "no motor data"},
                {"has_motors": true, "aborted": true},
                {"has_motors": true, "aborted": false, "rod_clearance": {"stability_cal": 1.5}},
                {"has_motors": true, "aborted": false, "rod_clearance": {"stability_cal": null}},
            ]},
            {"file": "refs/loft-fixtures/copy/a.ork", "sha256": "x", "flights": []},
            {"file": "refs/loft-fixtures/b.ork", "sha256": "y", "refused": "bad file"},
            {"file": "refs/loft-fixtures/c.ork", "sha256": "z", "driver_error": "boom"},
            {"file": "refs/openrocket/OpenRocket-24.12.jar!datafiles/examples/e.ork",
             "sha256": "e", "flights": [{"has_motors": true, "aborted": true}]},
            {"file": "refs/samples/s.ork", "sha256": "s", "flights": []},
        ]});
        let counts = count(&record).unwrap();
        assert_eq!(
            counts["refs/loft-fixtures"],
            Counts {
                designs: 4,
                distinct: 3,
                refused_designs: 1,
                driver_errors: 1,
                configurations: 5,
                no_motor: 1,
                refused_flights: 1,
                aborted: 1,
                completed: 2,
                with_margin: 1,
                designs_completed: 1,
            }
        );
        assert_eq!(counts["OpenRocket's examples"].aborted, 1);
        assert_eq!(counts["OpenRocket's examples"].designs_completed, 0);
        assert_eq!(counts["elsewhere under refs/"].designs, 1);
        let text = lines(&counts);
        assert!(text.contains("configurations: 5 declared, 1 with no motor, 1 refused, 1 aborted"));
        assert!(!text.contains("a.ork"), "no file name is printed");
    }
}
