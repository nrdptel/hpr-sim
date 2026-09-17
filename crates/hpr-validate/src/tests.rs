//! Tests of the harness itself: the Loft lessons it exists to prevent, and the committed cases.

use std::path::{Path, PathBuf};

use crate::case::{Case, CaseLock, Metric, Tolerance, cases_dir, committed_cases};
use crate::metrics::{Reference, ReferenceValue};
use crate::report::{Report, Verdict};
use crate::run::{ValidateError, run_case, run_lock};

/// The repository root, from this crate's manifest.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate is two directories below the workspace root")
        .to_path_buf()
}

/// The committed lock.
fn lock() -> CaseLock {
    let text = std::fs::read_to_string(cases_dir(&root()).join("lock.toml"))
        .expect("the case lock is committed");
    toml::from_str(&text).expect("the case lock parses")
}

/// Every committed case, in lock order.
fn cases() -> Vec<Case> {
    lock()
        .cases
        .iter()
        .map(|id| {
            let path = cases_dir(&root()).join(format!("{id}.toml"));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            toml::from_str(&text).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        })
        .collect()
}

/// A directory that removes itself, so a failing assertion cannot leave one behind.
struct Scratch(PathBuf);

impl Scratch {
    /// A fresh directory.
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "hpr-validate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(&path).expect("a temporary directory");
        Self(path)
    }

    /// Its path.
    fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `text` to `relative`, creating what it needs.
    fn write(&self, relative: &str, text: &str) {
        let path = self.0.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("a temporary directory");
        }
        std::fs::write(&path, text).expect("writing a temporary file");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// A root holding one case, its design and its reference, so a test can change one of the three
/// and run the harness for real.
///
/// `edit` sees the reference document before it is written.
fn scratch_case(case_id: &str, edit: impl FnOnce(&mut serde_json::Value)) -> Scratch {
    let scratch = Scratch::new();
    let case = cases()
        .into_iter()
        .find(|case| case.id == case_id)
        .expect("a committed case");
    let source = root().join(&case.reference);
    let mut document: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&source).expect("the reference is committed"),
    )
    .expect("the reference is JSON");
    edit(&mut document);
    scratch.write(
        &case.reference.display().to_string(),
        &serde_json::to_string_pretty(&document).expect("the reference serialises"),
    );
    let crate::case::Flight::RecoveryDescent { design, .. } = &case.flight;
    let design_path = format!("validation/designs/{design}.json");
    scratch.write(
        &design_path,
        &std::fs::read_to_string(root().join(&design_path)).expect("the design is committed"),
    );
    scratch.write(
        &format!("validation/cases/{case_id}.toml"),
        &std::fs::read_to_string(cases_dir(&root()).join(format!("{case_id}.toml")))
            .expect("the case is committed"),
    );
    scratch.write(
        "validation/cases/lock.toml",
        &toml::to_string(&CaseLock {
            cases: vec![case_id.to_owned()],
            slow: Vec::new(),
        })
        .expect("the lock serialises"),
    );
    scratch
}

#[test]
fn every_census_metric_has_a_per_case_tolerance() {
    // Loft lesson L79. Loft gated 2 of its 12 metrics, so a +204% deployment velocity passed as
    // "ungated". Here every metric a case reports is either held to a tolerance that bounds
    // something or declared, in writing, not to be scored.
    for case in cases() {
        assert!(
            !case.metrics.is_empty(),
            "case {} gates nothing at all",
            case.id
        );
        for (metric, gate) in &case.metrics {
            gate.check(&case.id, metric).expect("a committed case");
            if gate.reason().is_some() {
                continue;
            }
            let tolerance = gate.tolerance();
            assert!(tolerance.is_set());
            for bound in [tolerance.relative, tolerance.absolute]
                .into_iter()
                .flatten()
            {
                assert!(
                    bound.is_finite() && bound > 0.0,
                    "case {}: {metric}'s {bound} bounds nothing",
                    case.id
                );
            }
            if let Some(relative) = tolerance.relative {
                assert!(
                    relative > 0.0 && relative <= 0.05,
                    "case {}: {metric}'s {relative} is not a gate a milestone would accept",
                    case.id
                );
            }
        }
    }
    // A tolerance that bounds nothing accepts nothing, so it fails rather than passing quietly,
    // and neither an infinite nor a negative bound counts as one.
    for unset in [
        Tolerance::default(),
        Tolerance::relative(f64::INFINITY),
        Tolerance::relative(-1.0),
        Tolerance {
            relative: None,
            absolute: Some(f64::INFINITY),
        },
    ] {
        assert!(!unset.is_set(), "{unset:?}");
        assert!(!unset.accepts(1.0, 1.0), "{unset:?}");
        assert!(!unset.accepts(1e9, 1.0), "{unset:?}");
    }
    assert_eq!(Tolerance::default().describe(), "none");
    // And the harness refuses such a case before it flies anything.
    let mut case = cases().swap_remove(0);
    case.metrics
        .insert("descent_time_s".to_owned(), Metric::default());
    let error = run_case(&root(), &case).expect_err("an unbounded tolerance");
    assert!(
        matches!(&error, ValidateError::Case(message) if message.contains("bounds nothing")),
        "{error}"
    );
    // A metric cannot be both gated and excused: that is an argument waiting to be had in a diff.
    let mut case = cases().swap_remove(0);
    case.metrics.insert(
        "descent_time_s".to_owned(),
        Metric {
            relative: Some(0.03),
            absolute: None,
            not_scored: Some("because I say so".to_owned()),
        },
    );
    let error = run_case(&root(), &case).expect_err("gated and excused at once");
    assert!(
        matches!(&error, ValidateError::Case(message) if message.contains("one or the other")),
        "{error}"
    );
}

#[test]
fn no_committed_gate_is_looser_than_the_milestone_says() {
    // The floor an absolute bound provides is the quiet way to widen a gate: a metre of slack on a
    // metre of drift reads as "3% or 1.0" in the report and passes anything. Every scored
    // comparison's effective gate has to be within the 3% the milestone claims.
    let report = run_lock(&root(), false).expect("the committed cases run");
    for comparison in report.comparisons.iter().filter(|c| c.scored()) {
        let allowed = comparison.tolerance.allowed(comparison.reference);
        let claimed = 0.03 * comparison.reference.abs();
        assert!(
            allowed <= claimed * (1.0 + 1e-12),
            "{}'s {} may move by {allowed} where the milestone allows {claimed}",
            comparison.case,
            comparison.metric
        );
    }
}

#[test]
fn the_metrics_that_are_not_scored_are_these_and_no_others() {
    // Loft excused its two largest misses as "no single target" (L82). A metric that is measured
    // but not scored is the same move made honestly: it is printed with both numbers and a written
    // reason, it never counts as a pass, and the whole set is pinned here, so adding one means
    // editing a test whose name says what it is.
    let report = run_lock(&root(), false).expect("the committed cases run");
    let excused: Vec<(&str, &str)> = report
        .not_scored()
        .iter()
        .map(|comparison| (comparison.case.as_str(), comparison.metric.as_str()))
        .collect();
    assert_eq!(excused, vec![("descent-valetudo", "drift_north_m")]);
    for comparison in report.not_scored() {
        let reason = comparison.note.as_deref().unwrap_or_default();
        assert!(
            reason.contains("issue #"),
            "{}'s {} is excused without naming an issue: {reason}",
            comparison.case,
            comparison.metric
        );
        assert_eq!(comparison.verdict, Verdict::NotScored);
        // It is still measured and still printed, with both numbers.
        assert!(comparison.measured != 0.0 && comparison.reference != 0.0);
        assert!(report.to_markdown().contains(reason), "the report hides it");
    }
    // An unscored metric cannot make a run green on its own: the verdict counts failures only.
    assert!(report.passed());
    assert_eq!(report.failures().len(), 0);
}

#[test]
fn every_reference_value_has_provenance() {
    // Loft lesson L77. Loft shipped hand-written "stored results" in its demo designs, one set
    // internally inconsistent. Every value the harness reads carries a source naming the oracle,
    // the generator and the field it came from, the file's own hash is in the report, and a
    // reference that does not say what produced it is refused.
    let report = run_lock(&root(), false).expect("the committed cases run");
    assert!(!report.comparisons.is_empty());
    for comparison in &report.comparisons {
        assert!(
            comparison.source.contains("rocketpy") && comparison.source.contains("oracles/"),
            "{}: {}",
            comparison.metric,
            comparison.source
        );
    }
    for source in &report.sources {
        assert!(source.oracle.starts_with("rocketpy "), "{source:?}");
        assert!(
            source.generator.starts_with("validation/oracles/"),
            "{source:?}"
        );
        assert!(source.command.contains("recovery.py"), "{source:?}");
        assert_eq!(source.sha256.len(), 64, "{source:?}");
        assert!(source.model.contains("point mass"), "{source:?}");
        assert!(source.overrides.contains("noise"), "{source:?}");
    }
    // The hash is the file's, so an edited reference shows up in the report and not only in git.
    let committed =
        std::fs::read(root().join("validation/fixtures/recovery/rocketpy-descent.json"))
            .expect("the reference is committed");
    let expected: String = <sha2::Sha256 as sha2::Digest>::digest(&committed)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(report.sources[0].sha256, expected);

    // A reference that does not name the run that produced it is not a reference.
    let scratch = scratch_case("descent-juno-iii", |document| {
        document["generator"] = serde_json::Value::String(String::new());
    });
    let error = run_lock(scratch.path(), false).expect_err("a reference with no generator");
    assert!(
        matches!(&error, ValidateError::Case(message) if message.contains("names the run")),
        "{error}"
    );
    // A blank per-value source is not provenance either.
    let blank = Reference {
        oracle: "rocketpy 1.13.0".to_owned(),
        generator: "validation/oracles/rocketpy/recovery.py".to_owned(),
        command: "...".to_owned(),
        model: "a point mass".to_owned(),
        overrides: "none".to_owned(),
        case: "x".to_owned(),
        file: "validation/fixtures/recovery/rocketpy-descent.json".to_owned(),
        sha256: "0".repeat(64),
        values: [(
            "descent_time_s".to_owned(),
            ReferenceValue {
                value: 1.0,
                source: "   ".to_owned(),
            },
        )]
        .into_iter()
        .collect(),
    };
    assert!(blank.names_its_run());
    assert_eq!(blank.without_provenance(), vec!["descent_time_s"]);
}

#[test]
fn a_reference_metric_the_case_ignores_fails() {
    // The other direction of L79: a number the oracle published that no case mentions is a metric
    // nobody is watching, and the report would still call itself complete.
    let mut case = cases().swap_remove(0);
    case.metrics.remove("drift_north_m");
    let error = run_case(&root(), &case).expect_err("a reference metric the case ignores");
    assert!(
        matches!(&error, ValidateError::Case(message)
            if message.contains("drift_north_m") && message.contains("neither gates nor")),
        "{error}"
    );
}

#[test]
fn fewer_cases_run_than_the_lock_expects_fails() {
    // Loft lesson L78. Loft's suites skipped themselves when fixtures were missing and reported
    // green, and a filter quietly ignored 2 of 5 tool families. The lock names every case that
    // must run, so a missing one is an error, a committed case it does not name is an error too,
    // and `--fast` may only leave out cases the lock itself marks slow.
    let lock = lock();
    assert!(lock.cases.len() >= 5, "{:?}", lock.cases);
    assert!(lock.unknown_slow().is_empty(), "{:?}", lock.slow);
    assert_eq!(lock.wanted(false), lock.cases);
    assert!(lock.skipped(false).is_empty());
    for id in &lock.cases {
        assert!(
            cases_dir(&root()).join(format!("{id}.toml")).is_file(),
            "the lock names {id}, which is not there"
        );
    }
    let mut committed = committed_cases(&root()).expect("the cases directory is committed");
    let mut locked = lock.cases.clone();
    committed.sort();
    locked.sort();
    assert_eq!(committed, locked, "a case is committed but not locked");

    // A lock that names a case which is not there fails rather than running what it finds.
    let scratch = Scratch::new();
    scratch.write(
        "validation/cases/lock.toml",
        &toml::to_string(&CaseLock {
            cases: vec!["descent-nothing-like-this".to_owned()],
            slow: Vec::new(),
        })
        .expect("the lock serialises"),
    );
    let error =
        run_lock(scratch.path(), false).expect_err("a lock naming a case that is not there");
    assert!(
        matches!(&error, ValidateError::Case(message) if message.contains("is not there")),
        "{error}"
    );

    // And a case the lock does not name fails too, rather than sitting there never running.
    let scratch = scratch_case("descent-juno-iii", |_| {});
    scratch.write(
        "validation/cases/lock.toml",
        &toml::to_string(&CaseLock {
            cases: Vec::new(),
            slow: Vec::new(),
        })
        .expect("the lock serialises"),
    );
    let error = run_lock(scratch.path(), false).expect_err("a committed case nobody locked");
    assert!(
        matches!(&error, ValidateError::Case(message)
            if message.contains("descent-juno-iii") && message.contains("never run")),
        "{error}"
    );
}

#[test]
fn references_unchanged_when_hpr_drag_is_perturbed() {
    // Loft lesson L76. Loft's own instructions said "if the drift guard fails, regenerate the
    // reference", and the reference moved with Loft's drag. Here a reference is a file a generator
    // writes: running the harness reads it and never writes it, so hpr cannot move its own goal
    // posts. This flies a case twice with different drag and checks that every fixture in the
    // repository is byte-for-byte what it was, while the comparison moves.
    let fixtures = root().join("validation/fixtures");
    let before = tree(&fixtures);
    assert!(before.len() >= 5, "the fixtures are committed");
    let report = run_lock(&root(), false).expect("the committed cases run");
    assert_eq!(tree(&fixtures), before, "a run rewrote a reference");

    // Fly the same case with half the drag area: hpr lands somewhere else entirely.
    let scratch = scratch_case("descent-juno-iii", |document| {
        for case in document["cases"]
            .as_array_mut()
            .expect("the fixture has cases")
        {
            for device in case["devices"].as_array_mut().expect("a case has devices") {
                let halved = device["cd_s_m2"].as_f64().expect("a drag area") / 2.0;
                device["cd_s_m2"] = serde_json::json!(halved);
            }
        }
    });
    let perturbed = run_lock(scratch.path(), false).expect("the perturbed case runs");
    let hpr_of = |report: &Report, metric: &str| {
        report
            .comparisons
            .iter()
            .find(|comparison| comparison.case == "descent-juno-iii" && comparison.metric == metric)
            .map(|comparison| comparison.measured)
            .expect("the metric was compared")
    };
    assert!(
        hpr_of(&perturbed, "impact_speed_m_s") > 1.3 * hpr_of(&report, "impact_speed_m_s"),
        "halving the drag area has to move hpr's answer"
    );
    // The references did not move with it: not the committed ones, and not even the scratch copy
    // the perturbed run read, because a run has no way to write one.
    assert_eq!(
        tree(&fixtures),
        before,
        "a perturbed run rewrote a reference"
    );
    assert!(
        perturbed
            .comparisons
            .iter()
            .any(|c| c.verdict == Verdict::Fail),
        "a perturbed hpr has to fail against an unchanged reference"
    );
    let reference_of = |report: &Report, metric: &str| {
        report
            .comparisons
            .iter()
            .find(|comparison| comparison.case == "descent-juno-iii" && comparison.metric == metric)
            .map(|comparison| comparison.reference)
            .expect("the metric was compared")
    };
    assert_eq!(
        reference_of(&perturbed, "impact_speed_m_s"),
        reference_of(&report, "impact_speed_m_s"),
        "the reference followed hpr"
    );
}

#[test]
fn a_case_that_flies_another_rocket_than_the_reference_did_is_refused() {
    // Loft lesson L75: the comparison has to be like-for-like, and the reference records which
    // rocket the oracle flew. A case that names a different one would report the difference
    // between two vehicles as a difference in the physics.
    let scratch = scratch_case("descent-juno-iii", |document| {
        for case in document["cases"]
            .as_array_mut()
            .expect("the fixture has cases")
        {
            case["design"] = serde_json::json!("validation/designs/rocketpy-valetudo.json");
        }
    });
    let error = run_lock(scratch.path(), false).expect_err("a case flying another rocket");
    assert!(
        matches!(&error, ValidateError::Flight { what, .. }
            if what.contains("was flown with") && what.contains("valetudo")),
        "{error}"
    );

    // The same goes for the mass: two builds of one rocket, not two measurements of it.
    let scratch = scratch_case("descent-juno-iii", |document| {
        for case in document["cases"]
            .as_array_mut()
            .expect("the fixture has cases")
        {
            let heavier = case["dry_mass_kg"].as_f64().expect("a dry mass") * 1.01;
            case["dry_mass_kg"] = serde_json::json!(heavier);
        }
    });
    let error = run_lock(scratch.path(), false).expect_err("a case flying another mass");
    assert!(
        matches!(&error, ValidateError::Flight { what, .. } if what.contains("where the reference recorded")),
        "{error}"
    );
}

#[test]
fn the_committed_cases_all_pass_and_the_report_says_so() {
    // The milestone's own check: every locked case runs against its stored reference, and the
    // report that `cargo xtask validate` writes is the one this produces.
    let report = run_lock(&root(), false).expect("the committed cases run");
    assert_eq!(report.cases.len(), 5, "{:?}", report.cases);
    assert_eq!(report.comparisons.len(), 30);
    assert_eq!(report.not_scored().len(), 1);
    assert!(report.passed(), "{:?}", report.failures());
    let (worst, relative) = report.worst_scored().expect("something was scored");
    assert!(
        relative < 0.03,
        "{}'s {} is at {relative}",
        worst.case,
        worst.metric
    );
    let markdown = report.to_markdown();
    assert!(
        markdown.contains("29 scored, all within tolerance"),
        "{markdown}"
    );
    assert!(
        markdown.contains("| descent-valetudo | descent_time_s |"),
        "{markdown}"
    );
    assert!(markdown.contains("never regenerated to make a comparison pass"));
    // The report records what ran, not what a lock hoped would run.
    let mut compared: Vec<String> = report
        .comparisons
        .iter()
        .map(|comparison| comparison.case.clone())
        .collect();
    compared.dedup();
    assert_eq!(compared, report.cases);
    // The committed reports are these reports, so a number that moves shows up in the diff.
    let committed = std::fs::read_to_string(root().join("validation/reports/latest.md"))
        .expect("the report is committed");
    assert_eq!(committed, markdown, "run `cargo xtask validate`");
    // The JSON is pinned too, but not by its bytes: it carries full-precision floats, and hpr
    // promises bit-identical results on one platform, not across three (ADR-015). So everything
    // that cannot differ by platform is compared exactly...
    let committed: Report = serde_json::from_str(
        &std::fs::read_to_string(root().join("validation/reports/latest.json"))
            .expect("the JSON report is committed"),
    )
    .expect("the JSON report parses");
    assert_eq!(committed.harness_version, report.harness_version);
    assert_eq!(committed.fast, report.fast);
    assert_eq!(committed.cases, report.cases, "run `cargo xtask validate`");
    assert_eq!(committed.skipped, report.skipped);
    assert_eq!(
        committed.sources, report.sources,
        "run `cargo xtask validate`"
    );
    let shape = |report: &Report| {
        report
            .comparisons
            .iter()
            .map(|comparison| {
                (
                    comparison.case.clone(),
                    comparison.metric.clone(),
                    comparison.source.clone(),
                    comparison.tolerance,
                    comparison.verdict,
                    comparison.note.clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        shape(&committed),
        shape(&report),
        "run `cargo xtask validate`"
    );
    // ...and its numbers through the Markdown it renders, to the six decimals that report prints,
    // which is where the descent reproduces on macOS, Windows and Linux. A tighter check here
    // would assert a cross-platform bit-identity hpr does not claim.
    assert_eq!(
        committed.to_markdown(),
        markdown,
        "run `cargo xtask validate`"
    );
}

#[test]
fn a_fast_run_says_what_it_left_out() {
    // `--fast` may only leave out cases the lock marks slow, and the report names them, so a short
    // run cannot be mistaken for a whole one (Loft lesson L78's other half).
    let report = run_lock(&root(), true).expect("a fast run");
    assert!(report.fast);
    assert_eq!(report.cases, lock().wanted(true));
    assert!(report.skipped.is_empty(), "the lock marks no case slow");
    assert!(report.to_markdown().contains("nothing was left out"));

    // With a case marked slow, a fast run leaves out that case and only that case, and says so.
    let scratch = scratch_case("descent-juno-iii", |_| {});
    scratch.write(
        "validation/cases/lock.toml",
        &toml::to_string(&CaseLock {
            cases: vec!["descent-juno-iii".to_owned()],
            slow: vec!["descent-juno-iii".to_owned()],
        })
        .expect("the lock serialises"),
    );
    let fast = run_lock(scratch.path(), true).expect("a fast run");
    assert!(fast.cases.is_empty());
    assert_eq!(fast.skipped, vec!["descent-juno-iii".to_owned()]);
    assert!(
        fast.to_markdown()
            .contains("left out 1 of the locked cases: descent-juno-iii"),
        "{}",
        fast.to_markdown()
    );
    // The same lock run whole covers it.
    let whole = run_lock(scratch.path(), false).expect("a whole run");
    assert_eq!(whole.cases, vec!["descent-juno-iii".to_owned()]);
    assert!(whole.skipped.is_empty());
}

#[test]
fn a_metric_the_flight_cannot_measure_is_refused_before_it_flies() {
    // A typo in a case file should not cost a flight to find, and it must never pass: the metric
    // names a flight can report are the ones the harness measures, and nothing else.
    let mut case = cases().swap_remove(0);
    case.metrics
        .insert("apogee_m".to_owned(), Metric::relative(0.01));
    let started = std::time::Instant::now();
    let error = run_case(&root(), &case).expect_err("a metric the descent does not measure");
    assert!(
        matches!(&error, ValidateError::Case(message)
            if message.contains("apogee_m") && message.contains("not metrics this flight measures")),
        "{error}"
    );
    // The flights in this suite take seconds; this has to fail without making one.
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "it flew"
    );
}

#[test]
fn a_tolerance_accepts_what_it_says() {
    // The values stay off the boundary itself, where the comparison is at the mercy of rounding
    // (`1.03 - 1.0` is 0.030000000000000027, which is not `<= 0.03`).
    let relative = Tolerance::relative(0.03);
    assert!(relative.accepts(1.02, 1.0));
    assert!(!relative.accepts(1.04, 1.0));
    assert!(relative.accepts(-1.02, -1.0), "it works on either sign");
    assert!(
        !relative.accepts(1.0, 0.0),
        "a zero reference has no fraction"
    );
    assert!(
        !relative.accepts(f64::NAN, 1.0) && !relative.accepts(f64::INFINITY, 1.0),
        "a value that is not a number never passes"
    );
    let either = Tolerance {
        relative: Some(0.03),
        absolute: Some(0.05),
    };
    assert!(
        either.accepts(0.05, 0.0),
        "the floor carries a zero reference"
    );
    assert!(!either.accepts(0.051, 0.0));
    assert!(
        either.accepts(103.0, 100.0),
        "the fraction carries a large one"
    );
    assert_eq!(either.allowed(100.0), 3.0);
    assert_eq!(either.allowed(0.0), 0.05);
    assert_eq!(either.describe(), "3.000% or 0.05");
    assert_eq!(Tolerance::relative(0.03).describe(), "3.000%");
}

/// Every file under `path`, by relative name, with its bytes.
fn tree(path: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut stack = vec![path.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in std::fs::read_dir(&directory)
            .expect("a directory")
            .flatten()
        {
            let entry = entry.path();
            if entry.is_dir() {
                stack.push(entry);
            } else if let Ok(bytes) = std::fs::read(&entry) {
                files.insert(entry.display().to_string(), bytes);
            }
        }
    }
    files
}
