//! `hpr validate`: every validation case run, and held to the reports committed beside them.
//!
//! It makes the check `cargo xtask validate --check` makes, with the same function
//! ([`hpr_validate::committed::check`]), so it fails where that fails: a metric outside its
//! tolerance, a run that doesn't reproduce the committed report, or committed reports that don't
//! hold to the accepted accuracy census. It writes nothing. It needs a copy of the repository:
//! the cases, their references and the reports are files in it, not part of the tool.

use std::path::Path;

use hpr_validate::committed::{self, CensusCheck};
use hpr_validate::run_lock;
use hpr_validate::summary::{self, CaseSummary};

use crate::output::{CensusHeld, LargestDifference, Validate, ValidateCase, ValidateTotals};
use crate::{Failure, Out};

/// `hpr validate`'s arguments.
#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// The copy of the hpr-sim repository whose cases to run, the current folder by default; build
    /// hpr from the same commit
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub root: String,
}

/// The file a copy of the repository has, which the check names when it is missing.
const LOCK: &str = "validation/cases/lock.toml";

/// Runs `hpr validate`.
pub(crate) fn run(args: &ValidateArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    let root = Path::new(&args.root);
    if !root.join(LOCK).is_file() {
        return Err(Failure::Input(format!(
            "{}: not a copy of the hpr-sim repository, as it has no {LOCK}; run hpr validate \
             from the repository's folder, or give it with --root",
            args.root
        )));
    }
    let report = run_lock(root, false).map_err(|error| Failure::Input(error.to_string()))?;
    let checked = committed::check(root, &report);
    let problems = checked.problems();
    let totals = summary::totals(&report);
    let document = Validate {
        passed: problems.is_empty(),
        cases: summary::cases(&report).into_iter().map(case).collect(),
        totals: ValidateTotals {
            cases: totals.cases,
            metrics: totals.metrics,
            not_scored: totals.not_scored,
            predicted: totals.predicted,
            outside_target: totals.outside_target,
            failed: totals.failed,
        },
        reproduced: checked.reproduced.is_ok(),
        census: checked.census.as_ref().ok().map(census),
        problems,
    };
    let lines = text_lines(&report, &checked);
    let emitted = to.emit(&document, |out| {
        lines.iter().try_for_each(|line| writeln!(out, "{line}"))
    });
    // A failed check fails even when the output couldn't be written, such as to a closed pipe,
    // which is otherwise the reader's choice and no failure.
    if document.passed {
        emitted
    } else {
        Err(Failure::Checked(document.problems.join("; ")))
    }
}

/// What `cargo xtask validate --check` prints, line for line: the summary, then the check's lines.
fn text_lines(report: &hpr_validate::Report, checked: &committed::Checked) -> Vec<String> {
    let mut lines: Vec<String> = summary::cases(report)
        .iter()
        .map(ToString::to_string)
        .collect();
    lines.push(summary::totals(report).to_string());
    lines.extend(checked.lines());
    lines
}

fn case(summary: CaseSummary) -> ValidateCase {
    match summary {
        CaseSummary::Gap {
            case,
            metrics,
            refusal,
        } => ValidateCase::Gap {
            case,
            metrics,
            refusal,
        },
        CaseSummary::Predicted {
            case,
            metrics,
            within_target,
            largest,
        } => ValidateCase::Predicted {
            case,
            metrics,
            within_target,
            largest: largest.map(|(metric, relative)| LargestDifference { metric, relative }),
        },
        CaseSummary::Scored {
            case,
            metrics,
            worst_scored,
            not_scored,
            failed,
        } => ValidateCase::Scored {
            case,
            metrics,
            worst_scored,
            not_scored,
            failed,
        },
    }
}

fn census(check: &CensusCheck) -> CensusHeld {
    CensusHeld {
        rows: check.rows,
        changes: check
            .changes
            .iter()
            .map(|change| change.describe())
            .collect(),
        worse: check
            .changes
            .iter()
            .filter(|change| change.is_worse())
            .count(),
        stale: check.stale.clone(),
    }
}
