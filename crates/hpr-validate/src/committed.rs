//! The committed reports, held to a run and to the accepted census: the check `cargo xtask
//! validate --check` and `hpr validate` both make, so neither can pass where the other fails.
//!
//! [`check`] takes a run of every case ([`crate::run_lock`]) and fails unless
//!
//! - every scored metric is within its tolerance;
//! - the run reproduces the committed report, `validation/reports/latest.{md,json}`, to the
//!   digits the platforms share ([`Report::reproduces`]); and
//! - the committed reports hold to the accepted census, `validation/reports/census.json`, and
//!   every file written from it is what it writes ([`check_census`]; the census's decision record,
//!   [ADR-084][adr-084]).
//!
//! The census is checked whether or not the run reproduces, so a report that fails both says so.
//!
//! [adr-084]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-084-the-accuracy-census-the-reports-numbers-held-to-the-ones-accepted-2026-09-26

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::census::{self, Accepted, Census, Change, Reports};
use crate::real_flight::RealFlightReport;
use crate::report::Report;
use crate::run::ValidateError;

/// The harness's committed report, JSON.
pub const LATEST_JSON: &str = "validation/reports/latest.json";
/// The harness's committed report, Markdown.
pub const LATEST_MD: &str = "validation/reports/latest.md";
/// The accepted census.
pub const CENSUS_JSON: &str = "validation/reports/census.json";
/// Its page.
pub const CENSUS_MD: &str = "validation/reports/census.md";
/// Its page on GitHub, which the tables link to: the README and the site read it at one address.
pub const CENSUS_URL: &str =
    "https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/census.md";
/// Where the badges go.
pub const BADGES: &str = "docs/images";
/// The files that show the census table, between [`BEGIN`] and [`END`].
pub const TABLES: [&str; 2] = ["README.md", "docs/accuracy.md"];
/// The line that opens the census table in each of [`TABLES`].
pub const BEGIN: &str = "<!-- census: written by `cargo xtask census --accept` from \
                         validation/reports/census.json; do not edit -->";
/// The line that closes it.
pub const END: &str = "<!-- census: end -->";

/// The committed reports against the accepted census.
#[derive(Debug, Clone, PartialEq)]
pub struct CensusCheck {
    /// The accepted census's rows.
    pub rows: usize,
    /// The rows that differ from it.
    pub changes: Vec<Change>,
    /// The files written from it whose committed text isn't what it writes, each with why.
    pub stale: Vec<String>,
}

impl CensusCheck {
    /// Whether nothing differs and nothing is stale.
    pub fn holds(&self) -> bool {
        self.changes.is_empty() && self.stale.is_empty()
    }

    /// What `cargo xtask census --check` prints: a line per change and per stale file, and a
    /// summary.
    pub fn lines(&self) -> Vec<String> {
        if self.holds() {
            return vec![format!(
                "census: the committed reports hold to the accepted census ({} rows)",
                self.rows
            )];
        }
        let mut lines = change_lines(&self.changes);
        lines.extend(self.stale.iter().map(|line| format!("census: {line}")));
        lines
    }

    /// Why the check fails, or `None` if it [holds](Self::holds).
    pub fn problem(&self) -> Option<String> {
        let worse = self
            .changes
            .iter()
            .filter(|change| change.is_worse())
            .count();
        let mut problems = Vec::new();
        if !self.changes.is_empty() {
            problems.push(format!(
                "{} row(s) differ from the accepted census, {worse} of them for the worse: a \
                 regression beyond its slack fails here. If the change is meant, accept it in \
                 writing: `cargo xtask census --accept --reason <why>`, and commit {CENSUS_JSON}",
                self.changes.len()
            ));
        }
        if !self.stale.is_empty() {
            problems.push(format!(
                "{} output(s) are stale: run `cargo xtask census --accept`",
                self.stale.len()
            ));
        }
        (!problems.is_empty()).then(|| problems.join("; "))
    }
}

/// A line per change, and a summary: what `cargo xtask census` prints.
pub fn change_lines(changes: &[Change]) -> Vec<String> {
    if changes.is_empty() {
        return vec!["census: no row differs from the accepted census".to_owned()];
    }
    let mut lines: Vec<String> = changes
        .iter()
        .map(|change| format!("census: {}", change.describe()))
        .collect();
    lines.push(format!(
        "census: {} row(s) differ from the accepted census, {} for the worse",
        changes.len(),
        changes.iter().filter(|change| change.is_worse()).count()
    ));
    lines
}

/// Why a run doesn't reproduce the committed report.
#[derive(Debug)]
pub enum NotReproduced {
    /// The committed report couldn't be read.
    Unreadable(ValidateError),
    /// It was read, and differs from the run: the first difference.
    Differs(String),
}

/// A run held to the committed reports.
#[derive(Debug)]
pub struct Checked {
    /// The scored metrics outside their tolerance.
    pub failed: usize,
    /// Whether the run reproduces the committed report.
    pub reproduced: Result<(), NotReproduced>,
    /// The census check, or why it couldn't be taken.
    pub census: Result<CensusCheck, ValidateError>,
}

impl Checked {
    /// Whether every metric passed, the report reproduces, and the census holds.
    pub fn passed(&self) -> bool {
        self.problems().is_empty()
    }

    /// Why the check fails, in the order `cargo xtask validate --check` gives them; empty when
    /// it passes.
    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        match &self.census {
            Ok(census) => problems.extend(census.problem()),
            Err(error) => problems.push(error.to_string()),
        }
        if self.failed != 0 {
            problems.push(format!("{} metric(s) outside tolerance", self.failed));
        }
        match &self.reproduced {
            Ok(()) => {}
            Err(NotReproduced::Unreadable(error)) => problems.push(error.to_string()),
            Err(NotReproduced::Differs(what)) => problems.push(format!(
                "the committed report is not this run's ({what}); run `cargo xtask validate` \
                 and commit validation/reports/latest.{{md,json}}"
            )),
        }
        problems
    }

    /// What `cargo xtask validate --check` prints after the run's summary: the census's lines,
    /// then whether the report reproduces, when every metric passed and it does.
    pub fn lines(&self) -> Vec<String> {
        let mut lines = match &self.census {
            Ok(census) => census.lines(),
            Err(_) => Vec::new(),
        };
        if self.failed == 0 && self.reproduced.is_ok() {
            lines.push("validate: the committed report reproduces".to_owned());
        }
        lines
    }
}

/// Holds `report`, a run of every case, to the reports committed under `root`.
pub fn check(root: &Path, report: &Report) -> Checked {
    let reproduced = match (read(root, LATEST_MD), read(root, LATEST_JSON)) {
        (Ok(markdown), Ok(json)) => report
            .reproduces(&markdown, &json)
            .map_err(|error| NotReproduced::Differs(error.to_string())),
        (Err(error), _) | (_, Err(error)) => Err(NotReproduced::Unreadable(error)),
    };
    Checked {
        failed: report.failures().len(),
        reproduced,
        census: check_census(root),
    }
}

/// The committed reports under `root` against the accepted census, and the files written from it.
///
/// # Errors
///
/// A report or the census that can't be read, a census that can't be taken, or a table file
/// without its census block; a missing census is [`ValidateError::Case`].
pub fn check_census(root: &Path) -> Result<CensusCheck, ValidateError> {
    let changes = census_changes(root)?;
    let accepted = read_accepted(root)?.ok_or_else(|| {
        ValidateError::Case(format!(
            "{CENSUS_JSON} is missing; run `cargo xtask census --accept --reason <why>`"
        ))
    })?;
    let stale = stale(census_outputs(root, &accepted)?, |path| {
        std::fs::read_to_string(root.join(path))
    });
    Ok(CensusCheck {
        rows: accepted.census.rows.len(),
        changes,
        stale,
    })
}

/// The outputs whose committed text, as `read` gives it, isn't what the census writes.
pub fn stale(
    outputs: Vec<(PathBuf, String)>,
    read: impl Fn(&Path) -> std::io::Result<String>,
) -> Vec<String> {
    outputs
        .into_iter()
        .filter_map(|(path, text)| match read(&path) {
            Ok(committed) if committed == text => None,
            Ok(_) => Some(format!(
                "{} is not what the accepted census writes",
                path.display()
            )),
            Err(error) => Some(format!("{}: {error}", path.display())),
        })
        .collect()
}

/// The differences between the committed reports and the accepted census; every row is added
/// when none is accepted yet.
///
/// # Errors
///
/// As [`take_census`] and [`read_accepted`].
pub fn census_changes(root: &Path) -> Result<Vec<Change>, ValidateError> {
    let now = take_census(root)?;
    let accepted = read_accepted(root)?;
    let empty = Census {
        slack_share: census::SLACK_SHARE,
        references: now.references.clone(),
        rows: Vec::new(),
    };
    Ok(census::compare(
        accepted
            .as_ref()
            .map_or(&empty, |accepted| &accepted.census),
        &now,
    ))
}

/// Every file the accepted census writes, with its text, by its path from `root`.
///
/// Everything is rendered from the accepted rows, its summaries computed again, so a summary
/// edited by hand, or one an older version of the code wrote, shows as a stale `census.json`.
///
/// # Errors
///
/// A table file that can't be read or has no single census block.
pub fn census_outputs(
    root: &Path,
    accepted: &Accepted,
) -> Result<Vec<(PathBuf, String)>, ValidateError> {
    let accepted = accepted.refreshed();
    let json = serde_json::to_string_pretty(&accepted).map_err(|source| ValidateError::Json {
        path: CENSUS_JSON.to_owned(),
        source: Box::new(source),
    })?;
    let mut files = vec![
        (PathBuf::from(CENSUS_JSON), format!("{json}\n")),
        (PathBuf::from(CENSUS_MD), accepted.to_markdown()),
    ];
    for (name, svg) in census::badges(&accepted.summaries) {
        files.push((Path::new(BADGES).join(name), svg));
    }
    let block = format!(
        "{BEGIN}\n\n{}\n{END}",
        census::table(&accepted.summaries, Some(CENSUS_URL))
    );
    for path in TABLES {
        let text = read(root, path)?;
        files.push((
            PathBuf::from(path),
            splice(&text, &block, path).map_err(ValidateError::Case)?,
        ));
    }
    Ok(files)
}

/// `text` with what lies from [`BEGIN`] to [`END`] replaced by `block`.
///
/// # Errors
///
/// No block, one without its end, or two blocks, each naming `path`.
pub fn splice(text: &str, block: &str, path: &str) -> Result<String, String> {
    let start = text
        .find(BEGIN)
        .ok_or_else(|| format!("{path} has no census block: add the line `{BEGIN}` and `{END}`"))?;
    let end = text[start..]
        .find(END)
        .map(|at| start + at + END.len())
        .ok_or_else(|| format!("{path}: the census block has no `{END}`"))?;
    if text[end..].contains(BEGIN) {
        return Err(format!("{path} has two census blocks"));
    }
    Ok(format!("{}{block}{}", &text[..start], &text[end..]))
}

/// The accepted census, or `None` if none is committed.
///
/// # Errors
///
/// A census file that can't be read or isn't one.
pub fn read_accepted(root: &Path) -> Result<Option<Accepted>, ValidateError> {
    if root.join(CENSUS_JSON).exists() {
        read_json(root, CENSUS_JSON).map(Some)
    } else {
        Ok(None)
    }
}

/// The census of the reports committed under `root`.
///
/// # Errors
///
/// A report that can't be read or parsed, or a census that can't be taken from them.
pub fn take_census(root: &Path) -> Result<Census, ValidateError> {
    let harness: Report = read_json(root, LATEST_JSON)?;
    let real: RealFlightReport = read_json(root, "validation/reports/real-flights.json")?;
    let examples: Value = read_json(root, "validation/reports/openrocket-flights.json")?;
    let library: Value = read_json(root, "validation/reports/openrocket-library-flights.json")?;
    Census::take(Reports {
        harness: &harness,
        real_flights: &real,
        openrocket_examples: &examples,
        openrocket_library: &library,
    })
    .map_err(|error| ValidateError::Case(error.to_string()))
}

fn read(root: &Path, relative: &str) -> Result<String, ValidateError> {
    let path = root.join(relative);
    std::fs::read_to_string(&path).map_err(|source| ValidateError::Io {
        what: "reading",
        path: path.display().to_string(),
        source,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(
    root: &Path,
    relative: &str,
) -> Result<T, ValidateError> {
    serde_json::from_str(&read(root, relative)?).map_err(|source| ValidateError::Json {
        path: root.join(relative).display().to_string(),
        source: Box::new(source),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A census check of 7 rows, none changed, with `stale` stale files.
    fn census(stale: usize) -> CensusCheck {
        CensusCheck {
            rows: 7,
            changes: Vec::new(),
            stale: vec!["README.md is not what the accepted census writes".to_owned(); stale],
        }
    }

    /// The reasons, in xtask's order, and the lines after the summary.
    #[test]
    fn a_check_says_each_reason_once_in_order() {
        let passed = Checked {
            failed: 0,
            reproduced: Ok(()),
            census: Ok(census(0)),
        };
        assert!(passed.passed());
        assert_eq!(
            passed.lines(),
            [
                "census: the committed reports hold to the accepted census (7 rows)",
                "validate: the committed report reproduces"
            ]
        );
        let failed = Checked {
            failed: 2,
            reproduced: Err(NotReproduced::Differs("latest.json: x".to_owned())),
            census: Ok(census(1)),
        };
        assert_eq!(
            failed.problems(),
            [
                "1 output(s) are stale: run `cargo xtask census --accept`",
                "2 metric(s) outside tolerance",
                "the committed report is not this run's (latest.json: x); run `cargo xtask \
                 validate` and commit validation/reports/latest.{md,json}"
            ]
        );
        assert_eq!(
            failed.lines(),
            [
                "census: no row differs from the accepted census",
                "census: README.md is not what the accepted census writes"
            ]
        );
        // An unreadable report is its own reason, not a report to regenerate; a census that can't
        // be taken prints no census line.
        let unread = Checked {
            failed: 0,
            reproduced: Err(NotReproduced::Unreadable(ValidateError::Case(
                "reading x".into(),
            ))),
            census: Err(ValidateError::Case("no census".into())),
        };
        assert_eq!(unread.problems(), ["no census", "reading x"]);
        assert!(unread.lines().is_empty());
    }

    #[test]
    fn a_block_is_replaced_whole_and_only_once() {
        let text = format!("before\n{BEGIN}\nold\n{END}\nafter\n");
        let block = format!("{BEGIN}\nnew\n{END}");
        assert_eq!(
            splice(&text, &block, "f").unwrap(),
            format!("before\n{BEGIN}\nnew\n{END}\nafter\n")
        );
        assert!(
            splice("no block", &block, "f")
                .unwrap_err()
                .contains("no census block")
        );
        assert!(
            splice(&format!("{BEGIN}\nold"), &block, "f")
                .unwrap_err()
                .contains("no `")
        );
        let twice = format!("{text}{text}");
        assert!(splice(&twice, &block, "f").unwrap_err().contains("two"));
    }
}
