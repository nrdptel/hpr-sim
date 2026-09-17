//! `cargo xtask validate [--fast]`: runs the validation cases and writes the reports.
//!
//! The cases live in `validation/cases/`, their references in `validation/fixtures/`, and the
//! reports go to `validation/reports/latest.md` and `latest.json`. The command exits non-zero when
//! a metric is outside its tolerance, when the lock names a case that is not there, when a
//! reference value has no provenance, or when a metric has no tolerance
//! (`docs/VALIDATION.md`, ADR-015).
//!
//! `--fast` leaves out the cases the lock marks slow. It never leaves out a case silently: the
//! report says which ones it skipped.

use std::path::Path;

use hpr_validate::{Report, run_lock};

pub const USAGE: &str = "  validate [--fast]        Run the validation cases and write
                           validation/reports/latest.{md,json}. --fast leaves out the cases the
                           lock marks slow.";

/// Runs the suite and writes the reports.
pub fn run(args: &[String]) -> Result<(), String> {
    let mut fast = false;
    for arg in args {
        match arg.as_str() {
            "--fast" => fast = true,
            other => return Err(format!("unknown argument `{other}`\n\n{USAGE}")),
        }
    }
    let root = crate::designs::root()?;
    let report = run_lock(&root, fast).map_err(|error| error.to_string())?;
    write_reports(&root, &report)?;
    print_summary(&report);
    if report.passed() {
        Ok(())
    } else {
        Err(format!(
            "{} metric(s) outside tolerance; see validation/reports/latest.md",
            report.failures().len()
        ))
    }
}

/// Writes the Markdown and JSON reports.
fn write_reports(root: &Path, report: &Report) -> Result<(), String> {
    let directory = root.join("validation/reports");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("creating {}: {error}", directory.display()))?;
    let markdown = directory.join("latest.md");
    std::fs::write(&markdown, report.to_markdown())
        .map_err(|error| format!("writing {}: {error}", markdown.display()))?;
    let json = directory.join("latest.json");
    let text = serde_json::to_string_pretty(report)
        .map_err(|error| format!("serialising the report: {error}"))?;
    std::fs::write(&json, format!("{text}\n"))
        .map_err(|error| format!("writing {}: {error}", json.display()))?;
    Ok(())
}

/// Prints one line per case and a summary.
fn print_summary(report: &Report) {
    for case in &report.cases {
        let metrics: Vec<&hpr_validate::Comparison> = report
            .comparisons
            .iter()
            .filter(|comparison| comparison.case == *case)
            .collect();
        let worst = metrics
            .iter()
            .filter_map(|comparison| comparison.relative)
            .fold(0.0_f64, |worst, relative| worst.max(relative.abs()));
        let failed = metrics
            .iter()
            .filter(|comparison| comparison.verdict == hpr_validate::Verdict::Fail)
            .count();
        println!(
            "{case}: {} metric(s), worst {:+.2}%{}",
            metrics.len(),
            100.0 * worst,
            if failed == 0 {
                String::new()
            } else {
                format!(", {failed} OUT OF TOLERANCE")
            }
        );
    }
    println!(
        "validate: {} case(s), {} metric(s), {}",
        report.cases.len(),
        report.comparisons.len(),
        if report.passed() { "ok" } else { "FAILED" }
    );
}
