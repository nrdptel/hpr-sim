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
//! and Linux, so a change that moves a number cannot merge without the report that says so.

use std::path::Path;

use hpr_validate::{Report, run_lock};

pub const USAGE: &str = "  validate [--fast|--check]
                           Run the validation cases and write
                           validation/reports/latest.{md,json}. --fast leaves out the cases the
                           lock marks slow and writes latest-fast.{md,json} instead. --check
                           writes nothing and fails unless the committed report reproduces.";

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
    if report.passed() {
        Ok(())
    } else {
        Err(format!(
            "{} metric(s) outside tolerance; see validation/reports/{written}.md",
            report.failures().len()
        ))
    }
}

/// Fails unless every scored metric passed and the run reproduces the committed report.
fn check_committed(root: &Path, report: &Report) -> Result<(), String> {
    let read = |name: &str| {
        let path = root.join("validation/reports").join(name);
        std::fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()))
    };
    let reproduced = report.reproduces(&read("latest.md")?, &read("latest.json")?);
    match (report.passed(), reproduced) {
        (true, Ok(())) => {
            println!("validate: the committed report reproduces");
            Ok(())
        }
        (passed, reproduced) => {
            let mut problems = Vec::new();
            if !passed {
                problems.push(format!(
                    "{} metric(s) outside tolerance",
                    report.failures().len()
                ));
            }
            if let Err(what) = reproduced {
                problems.push(format!(
                    "the committed report is not this run's ({what}); run `cargo xtask validate` \
                     and commit validation/reports/latest.{{md,json}}"
                ));
            }
            Err(problems.join("; "))
        }
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

/// Prints one line per case and a summary.
///
/// The worst figure is over the **scored** metrics: a metric a case declares not scored is
/// counted and named separately, so a run cannot look green by leaving something out and cannot
/// look alarming because a declared difference is large in percentage terms.
fn print_summary(report: &Report) {
    for case in &report.cases {
        if let Some(gap) = report.gaps.iter().find(|gap| gap.case == *case) {
            // A gap compares nothing, so "0 metrics, worst +0.00%" would read as a clean pass.
            println!(
                "{case}: known gap, {} metric(s) not scored: hpr {}",
                gap.metric_count, gap.refusal
            );
            continue;
        }
        let metrics: Vec<&hpr_validate::Comparison> = report
            .comparisons
            .iter()
            .filter(|comparison| comparison.case == *case)
            .collect();
        if metrics.iter().any(|comparison| comparison.targeted_row()) {
            // Predicted mode reports against a target and never gates, so it has no "worst
            // scored"; its largest difference is the headline.
            let within = metrics
                .iter()
                .filter(|comparison| comparison.verdict == hpr_validate::Verdict::WithinTarget)
                .count();
            let largest = metrics
                .iter()
                .filter_map(|comparison| comparison.relative.map(|r| (r, &comparison.metric)))
                .max_by(|(a, _), (b, _)| a.abs().total_cmp(&b.abs()));
            println!(
                "{case}: predicted, {} metric(s) reported, {within} within target{}",
                metrics.len(),
                largest.map_or_else(String::new, |(relative, metric)| format!(
                    ", largest {metric} {:+.2}%",
                    100.0 * relative
                ))
            );
            continue;
        }
        let worst = metrics
            .iter()
            .filter(|comparison| comparison.scored())
            .filter_map(|comparison| comparison.relative)
            .fold(0.0_f64, |worst, relative| worst.max(relative.abs()));
        let failed = metrics
            .iter()
            .filter(|comparison| comparison.verdict == hpr_validate::Verdict::Fail)
            .count();
        let unscored: Vec<&str> = metrics
            .iter()
            .filter(|comparison| !comparison.scored())
            .map(|comparison| comparison.metric.as_str())
            .collect();
        println!(
            "{case}: {} metric(s), worst scored {:+.2}%{}{}",
            metrics.len(),
            100.0 * worst,
            if unscored.is_empty() {
                String::new()
            } else {
                format!(", not scored: {}", unscored.join(", "))
            },
            if failed == 0 {
                String::new()
            } else {
                format!(", {failed} OUT OF TOLERANCE")
            }
        );
    }
    let not_scored = report.not_scored().len();
    let targeted = report
        .comparisons
        .iter()
        .filter(|comparison| comparison.targeted_row())
        .count();
    let aside: Vec<String> = [
        (not_scored, "not scored"),
        (targeted, "predicted, against a target"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, what)| format!("{count} {what}"))
    .collect();
    println!(
        "validate: {} case(s), {} metric(s){}, {}",
        report.cases.len(),
        report.comparisons.len(),
        if aside.is_empty() {
            String::new()
        } else {
            format!(" ({})", aside.join(", "))
        },
        if report.passed() { "ok" } else { "FAILED" }
    );
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
