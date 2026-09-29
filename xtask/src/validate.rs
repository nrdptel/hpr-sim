//! `cargo xtask validate [--fast]`: runs the validation cases and writes the reports.
//!
//! The cases live in `validation/cases/`, their references in `validation/fixtures/`, and the
//! reports go to `validation/reports/latest.md` and `latest.json`. The command exits non-zero when
//! a metric is outside its tolerance, when the lock names a case that is not there, when a
//! reference value has no provenance, or when a metric has no tolerance
//! (`docs/VALIDATION.md`, ADR-015).
//!
//! `--fast` leaves out the cases the lock marks slow. It never leaves out a case silently: it
//! says so on the console, its report says so too, and it writes `latest-fast.{md,json}` rather
//! than the committed report, so a partial run can never stand in for the whole suite's record.
//!
//! `--check` runs every case, writes nothing, and fails unless the run reproduces the committed
//! report, to the digits the platforms share (`Report::reproduces`). CI runs it on macOS, Windows
//! and Linux, so a change that moves a number cannot merge without the report that says so. It
//! then holds the committed reports to the accepted accuracy census (`crate::census`, ADR-084), so
//! a regenerated report can't carry a regression in either: that needs the census accepted again,
//! with a written reason.

use std::path::Path;

use hpr_validate::{Report, committed, run_lock, summary};

pub const USAGE: &str = "  validate [--fast|--check]
                           Run the validation cases and write
                           validation/reports/latest.{md,json}. --fast leaves out the cases the
                           lock marks slow and writes latest-fast.{md,json} instead. --check
                           writes nothing and fails unless the committed report reproduces and
                           holds to the accepted census (`census --check`).";

/// Runs the suite and writes the reports.
pub fn run(args: &[String]) -> Result<(), String> {
    let mut fast = false;
    let mut check = false;
    for arg in args {
        match arg.as_str() {
            "--fast" => fast = true,
            "--check" => check = true,
            other => return Err(format!("unknown argument `{other}`\n\n{USAGE}")),
        }
    }
    if fast && check {
        return Err(
            "`--check` compares the whole suite with its committed report, so it cannot be `--fast`"
                .to_owned(),
        );
    }
    let root = crate::designs::root()?;
    let report = run_lock(&root, fast).map_err(|error| error.to_string())?;
    if check {
        print_summary(&report);
        return check_committed(&root, &report);
    }
    let written = write_reports(&root, &report)?;
    print_summary(&report);
    println!("validate: wrote validation/reports/{written}.md and {written}.json");
    if !report.fast {
        // Not a failure here: the report is new, and the census says what it changed. `--check`
        // fails until the change is accepted.
        // A census that can't be taken is printed, not returned, so the suite's own result stands.
        match crate::census::changes(&root) {
            Ok(changes) if !changes.is_empty() => println!(
                "validate: {} census row(s) differ from the accepted census, {} for the worse; \
                 `cargo xtask census` lists them, and `--check` fails until they are accepted",
                changes.len(),
                changes.iter().filter(|change| change.is_worse()).count()
            ),
            Ok(_) => {}
            Err(error) => println!("validate: the census can't be taken: {error}"),
        }
    }
    if report.passed() {
        Ok(())
    } else {
        Err(format!(
            "{} metric(s) outside tolerance; see validation/reports/{written}.md",
            report.failures().len()
        ))
    }
}

/// Fails unless every scored metric passed, the run reproduces the committed report, and the
/// committed reports hold to the accepted census: `hpr_validate::committed::check`, which
/// `hpr validate` makes too.
fn check_committed(root: &Path, report: &Report) -> Result<(), String> {
    let checked = committed::check(root, report);
    if let Ok(census) = &checked.census {
        for line in census.lines() {
            println!("{line}");
        }
    }
    let problems = checked.problems();
    if checked.failed == 0 && checked.reproduced.is_ok() {
        println!("validate: the committed report reproduces");
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

/// Writes the Markdown and JSON reports, and returns the stem it wrote.
///
/// A whole run writes `latest`, which is committed and is the suite's record. A `--fast` run
/// writes `latest-fast`, which is not: a partial report that overwrote the committed one would
/// leave the repository claiming a green suite that never ran (Loft lesson L78).
fn write_reports(root: &Path, report: &Report) -> Result<&'static str, String> {
    let directory = root.join("validation/reports");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("creating {}: {error}", directory.display()))?;
    let stem = if report.fast { "latest-fast" } else { "latest" };
    let markdown = directory.join(format!("{stem}.md"));
    std::fs::write(&markdown, report.to_markdown())
        .map_err(|error| format!("writing {}: {error}", markdown.display()))?;
    let json = directory.join(format!("{stem}.json"));
    let text = serde_json::to_string_pretty(report)
        .map_err(|error| format!("serialising the report: {error}"))?;
    std::fs::write(&json, format!("{text}\n"))
        .map_err(|error| format!("writing {}: {error}", json.display()))?;
    Ok(stem)
}

/// Prints one line per case and a summary (`hpr_validate::summary`).
fn print_summary(report: &Report) {
    for case in summary::cases(report) {
        println!("{case}");
    }
    println!("{}", summary::totals(report));
}

#[cfg(test)]
mod tests {
    use hpr_validate::{Report, Source};

    use super::write_reports;

    /// A report with no comparisons, which is all the stem depends on.
    fn report(fast: bool) -> Report {
        Report {
            harness_version: "0.0.0".to_owned(),
            fast,
            cases: Vec::new(),
            skipped: Vec::new(),
            comparisons: Vec::new(),
            gaps: Vec::new(),
            sources: vec![Source {
                case: "x".to_owned(),
                oracle: "rocketpy 1.13.0".to_owned(),
                generator: "validation/oracles/rocketpy/recovery.py".to_owned(),
                command: "...".to_owned(),
                file: "validation/fixtures/recovery/rocketpy-descent.json".to_owned(),
                sha256: "0".repeat(64),
                model: "a point mass under a canopy".to_owned(),
                overrides: "none".to_owned(),
            }],
        }
    }

    #[test]
    fn a_fast_run_writes_its_own_report_and_leaves_the_committed_one_alone() {
        let root = std::env::temp_dir().join(format!("hpr-xtask-validate-{}", std::process::id()));
        let reports = root.join("validation/reports");
        std::fs::create_dir_all(&reports).unwrap();
        // Stand in for the committed whole-suite report.
        std::fs::write(
            reports.join("latest.md"),
            "the whole suite
",
        )
        .unwrap();
        std::fs::write(
            reports.join("latest.json"),
            "{}
",
        )
        .unwrap();

        assert_eq!(write_reports(&root, &report(true)).unwrap(), "latest-fast");
        assert!(reports.join("latest-fast.md").is_file());
        assert!(reports.join("latest-fast.json").is_file());
        assert_eq!(
            std::fs::read_to_string(reports.join("latest.md")).unwrap(),
            "the whole suite
",
            "a partial run overwrote the suite's record"
        );
        assert_eq!(
            std::fs::read_to_string(reports.join("latest.json")).unwrap(),
            "{}\n"
        );

        // A whole run owns `latest`.
        assert_eq!(write_reports(&root, &report(false)).unwrap(), "latest");
        assert!(
            std::fs::read_to_string(reports.join("latest.md"))
                .unwrap()
                .contains("Validation report")
        );
        std::fs::remove_dir_all(&root).ok();
    }
}
