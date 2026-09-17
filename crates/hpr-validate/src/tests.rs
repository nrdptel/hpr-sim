//! Tests of the harness itself: the Loft lessons it exists to prevent, and the committed cases.

use std::path::{Path, PathBuf};

use crate::case::{Case, CaseLock, Metric, Tolerance, cases_dir};
use crate::metrics::{Reference, ReferenceValue};
use crate::report::{Comparison, Verdict};
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

#[test]
fn every_census_metric_has_a_per_case_tolerance() {
    // Loft lesson L79. Loft gated 2 of its 12 metrics, so a +204% deployment velocity passed as
    // "ungated". Here every metric a case reports has a tolerance that bounds something, and the
    // harness refuses a case that measures anything it does not gate (which the run below
    // exercises for real).
    for case in cases() {
        assert!(
            !case.metrics.is_empty(),
            "case {} gates nothing at all",
            case.id
        );
        for (metric, gate) in &case.metrics {
            assert!(
                gate.tolerance.is_set(),
                "case {}: {metric} has a tolerance that bounds nothing",
                case.id
            );
            if let Some(relative) = gate.tolerance.relative {
                assert!(
                    relative > 0.0 && relative <= 0.05,
                    "case {}: {metric}'s {relative} is not a gate a milestone would accept",
                    case.id
                );
            }
        }
    }
    // A tolerance that bounds nothing accepts nothing, so it fails rather than passing quietly.
    let unset = Tolerance::default();
    assert!(!unset.is_set());
    assert!(!unset.accepts(1.0, 1.0));
    assert_eq!(unset.describe(), "none");
    // And the harness refuses such a case before it flies anything.
    let mut case = cases().swap_remove(0);
    case.metrics.insert(
        "descent_time_s".to_owned(),
        Metric {
            tolerance: Tolerance::default(),
        },
    );
    let error = run_case(&root(), &case).expect_err("an unbounded tolerance");
    assert!(
        matches!(&error, ValidateError::Case(message) if message.contains("bounds nothing")),
        "{error}"
    );
}

#[test]
fn every_reference_value_has_provenance() {
    // Loft lesson L77. Loft shipped hand-written "stored results" in its demo designs, one set
    // internally inconsistent. Every value the harness reads carries a source naming the oracle,
    // the generator and the field it came from, and a reference with a blank source is refused.
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
    }
    // A blank source is not provenance.
    let blank = Reference {
        oracle: "rocketpy 1.13.0".to_owned(),
        generator: "validation/oracles/rocketpy/recovery.py".to_owned(),
        command: "...".to_owned(),
        case: "x".to_owned(),
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
    assert_eq!(blank.without_provenance(), vec!["descent_time_s"]);
}

#[test]
fn fewer_cases_run_than_the_lock_expects_fails() {
    // Loft lesson L78. Loft's suites skipped themselves when fixtures were missing and reported
    // green, and a filter quietly ignored 2 of 5 tool families. The lock names every case that
    // must run, so a missing one is an error, and `--fast` may only leave out cases the lock
    // itself marks slow.
    let lock = lock();
    assert!(lock.cases.len() >= 5, "{:?}", lock.cases);
    assert!(lock.unknown_slow().is_empty(), "{:?}", lock.slow);
    assert_eq!(lock.wanted(false), lock.cases);
    assert_eq!(lock.wanted(true).len(), lock.cases.len() - lock.slow.len());
    for id in &lock.cases {
        assert!(
            cases_dir(&root()).join(format!("{id}.toml")).is_file(),
            "the lock names {id}, which is not there"
        );
    }
    // Every committed case is locked: one that is not would never run.
    let mut committed: Vec<String> = std::fs::read_dir(cases_dir(&root()))
        .expect("the cases directory is committed")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            (path
                .extension()
                .is_some_and(|extension| extension == "toml")
                && path.file_stem().is_some_and(|stem| stem != "lock"))
            .then(|| {
                path.file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
            })
            .flatten()
        })
        .collect();
    committed.sort();
    let mut locked = lock.cases.clone();
    locked.sort();
    assert_eq!(committed, locked, "a case is committed but not locked");

    // A lock that names a case which is not there fails rather than running what it finds.
    let missing = CaseLock {
        cases: vec!["descent-nothing-like-this".to_owned()],
        slow: Vec::new(),
    };
    assert_eq!(missing.wanted(false).len(), 1);
    let directory = tempdir();
    std::fs::create_dir_all(directory.join("validation/cases")).expect("the temporary directory");
    std::fs::write(
        directory.join("validation/cases/lock.toml"),
        toml::to_string(&missing).expect("the lock serialises"),
    )
    .expect("writing the lock");
    let error = run_lock(&directory, false).expect_err("a lock naming a case that is not there");
    assert!(
        matches!(&error, ValidateError::Case(message) if message.contains("is not there")),
        "{error}"
    );
    std::fs::remove_dir_all(&directory).ok();
}

#[test]
fn references_unchanged_when_hpr_drag_is_perturbed() {
    // Loft lesson L76. Loft's own instructions said "if the drift guard fails, regenerate the
    // reference", and the reference moved with Loft's drag. Here a reference is a file a generator
    // writes: running the harness reads it and never writes it, so hpr cannot move its own goal
    // posts. This checks that directly — the bytes are the same after a run, and a perturbed hpr
    // measurement changes the comparison rather than the reference.
    let reference_path = root().join("validation/fixtures/recovery/rocketpy-descent.json");
    let before = std::fs::read(&reference_path).expect("the reference is committed");
    let report = run_lock(&root(), false).expect("the committed cases run");
    let after = std::fs::read(&reference_path).expect("the reference is still there");
    assert_eq!(before, after, "a run rewrote its own reference");

    // Perturbing what hpr measures moves the verdict, not the reference.
    let first = report
        .comparisons
        .first()
        .expect("the run compared something");
    let perturbed = Comparison::new(
        &first.case,
        &first.metric,
        first.reference * 1.5,
        first.reference,
        &first.source,
        Tolerance::relative(0.03),
    );
    assert_eq!(first.verdict, Verdict::Pass);
    assert_eq!(perturbed.verdict, Verdict::Fail);
    assert_eq!(perturbed.reference, first.reference);
    assert_eq!(
        std::fs::read(&reference_path).expect("the reference is still there"),
        before
    );
}

#[test]
fn the_committed_cases_all_pass_and_the_report_says_so() {
    // The milestone's own check: every locked case runs against its stored reference, and the
    // report that `cargo xtask validate` writes is the one this produces.
    let report = run_lock(&root(), false).expect("the committed cases run");
    assert_eq!(report.cases.len(), 5, "{:?}", report.cases);
    assert_eq!(report.comparisons.len(), 25);
    assert!(report.passed(), "{:?}", report.failures());
    let markdown = report.to_markdown();
    assert!(markdown.contains("all within tolerance"), "{markdown}");
    assert!(
        markdown.contains("| descent-valetudo | descent_time_s |"),
        "{markdown}"
    );
    assert!(markdown.contains("never regenerated to make a comparison pass"));
    // The committed report is this report, so a number that moves shows up in the diff.
    let committed = std::fs::read_to_string(root().join("validation/reports/latest.md"))
        .expect("the report is committed");
    assert_eq!(committed, markdown, "run `cargo xtask validate`");
}

#[test]
fn a_fast_run_says_what_it_left_out() {
    // `--fast` may only leave out cases the lock marks slow, and the report says it was fast so a
    // short run cannot be mistaken for a whole one (Loft lesson L78's other half).
    let report = run_lock(&root(), true).expect("a fast run");
    assert!(report.fast);
    assert_eq!(report.cases, lock().wanted(true));
    assert!(report.to_markdown().contains("--fast") || lock().slow.is_empty());
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
    assert_eq!(either.describe(), "3.000% or 0.05");
    assert_eq!(Tolerance::relative(0.03).describe(), "3.000%");
}

/// A fresh directory for a test that needs one.
fn tempdir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "hpr-validate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&path).expect("a temporary directory");
    path
}
