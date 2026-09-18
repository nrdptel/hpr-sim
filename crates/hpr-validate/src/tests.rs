//! Tests of the harness itself: the Loft lessons it exists to prevent, and the committed cases.

use std::path::{Path, PathBuf};

use crate::case::{Case, CaseLock, Metric, Tolerance, cases_dir, committed_cases};
use crate::metrics::{Reference, ReferenceValue};
use crate::report::{Comparison, Report, Verdict};
use crate::run::{ValidateError, peak_between, run_case, run_lock};

/// The repository root, from this crate's manifest.
pub(crate) fn root() -> PathBuf {
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
pub(crate) fn cases() -> Vec<Case> {
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
pub(crate) struct Scratch(PathBuf);

impl Scratch {
    /// A fresh directory. Tests run in parallel and the clock may tick coarsely, so a counter
    /// keeps two made in the same instant apart.
    fn new() -> Self {
        static MADE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "hpr-validate-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos()),
            MADE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).expect("a temporary directory");
        Self(path)
    }

    /// Its path.
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }

    /// Writes `text` to `relative`, creating what it needs.
    pub(crate) fn write(&self, relative: &str, text: &str) {
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
pub(crate) fn scratch_case(case_id: &str, edit: impl FnOnce(&mut serde_json::Value)) -> Scratch {
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
    let design_path = format!("validation/designs/{}.json", case.flight.design());
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
    // Predicted mode's targets too (ADR-023): a target widened past 3% would turn misses into
    // "within target" with nothing failing.
    let report = run_lock(&root(), false).expect("the committed cases run");
    // The time-series RMS has 0 for its reference, so its 3% is of the scale of its trace, the
    // same case's reference apogee or max speed (M2.1d1), as its case file argues.
    let reference_of = |case: &str, metric: &str| {
        report
            .comparisons
            .iter()
            .find(|c| c.case == case && c.metric == metric)
            .map(|c| c.reference)
            .expect("every whole flight reports its apogee and max speed")
    };
    let bounded = report
        .comparisons
        .iter()
        .filter(|c| c.scored() || c.targeted_row());
    assert_eq!(bounded.clone().count(), 94 + 10 + 75 + 10);
    for comparison in bounded {
        let allowed = comparison.tolerance.allowed(comparison.reference);
        let scale = match comparison.metric.as_str() {
            "series_height_rms_m" => reference_of(&comparison.case, "apogee_agl_m"),
            "series_speed_rms_m_s" => reference_of(&comparison.case, "max_speed_m_s"),
            _ => comparison.reference,
        };
        let claimed = 0.03 * scale.abs();
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
    // Valetudo's northward drift was excused while its 28x gap was unexplained; the gap was an
    // unlike gravity model, the suite now flies RocketPy's own, and the metric is gated like every
    // other. The ones below are whole flights (M2.1b2), each argued in its case file:
    //
    // - Calisto's maximum acceleration comes twice, 0.9% apart, and hpr's rail terms decide which
    //   is the higher, so its *time* jumps between the peaks; the value is gated.
    // - NDRT 2020's whole-flight maximum is the main opening, where RocketPy has added mass and hpr
    //   has none (ADR-012); the power-on maximum and the opening's time are gated.
    // - The horizontal path in wind, where hpr turns into the wind less than RocketPy
    //   (issue #50); in calm air the drifts agree to 1.3 to 3.7%, and Valetudo, in still air,
    //   keeps its apogee drift gated while its landing drift (-3.41%) is reported. These are open
    //   misses, not definitional differences: M2.1's landing offset is not met until #50 closes.
    //
    // Adding an excuse means editing this list.
    let drifts = |case| [(case, "apogee_drift_m"), (case, "landing_drift_m")];
    let mut expected = vec![];
    expected.extend(drifts("flight-calisto-tests-motor-at-minus-1.373"));
    expected.push((
        "flight-calisto-tests-motor-at-minus-1.373",
        "max_acceleration_time_s",
    ));
    expected.push(("flight-valetudo", "landing_drift_m"));
    expected.extend(drifts("flight-ndrt-2020-nose-to-tail"));
    expected.push(("flight-ndrt-2020-nose-to-tail", "max_acceleration_m_s2"));
    expected.extend(drifts("flight-juno-iii"));
    expected.extend(drifts("flight-bella-lui"));
    assert_eq!(excused, expected);
    // A known gap is the other way a case goes unscored, and the set of them is pinned the same
    // way: Prometheus 2022 reaches Mach 1.014 on the declared drag and Mach 1.049 on its own, both
    // of which hpr refuses until M1.8.
    let gaps: Vec<&str> = report.gaps.iter().map(|gap| gap.case.as_str()).collect();
    assert_eq!(
        gaps,
        vec![
            "flight-prometheus-2022-generic-motor",
            "predicted-prometheus-2022-generic-motor"
        ]
    );
    // Predicted mode's misses are pinned too, so a case file's account of them ("nothing else
    // misses") cannot go stale unnoticed (ADR-023). Adding a miss means editing this list and
    // explaining it in the case file.
    let outside: Vec<(&str, &str)> = report
        .comparisons
        .iter()
        .filter(|comparison| comparison.verdict == Verdict::OutsideTarget)
        .map(|comparison| (comparison.case.as_str(), comparison.metric.as_str()))
        .collect();
    let mut expected_outside = vec![];
    expected_outside.extend(drifts("predicted-calisto-tests-motor-at-minus-1.373"));
    for metric in [
        "apogee_agl_m",
        "apogee_drift_m",
        "apogee_time_s",
        "flight_time_s",
        "landing_drift_m",
        "series_height_rms_m",
    ] {
        expected_outside.push(("predicted-valetudo", metric));
    }
    for metric in [
        "apogee_agl_m",
        "apogee_drift_m",
        "apogee_time_s",
        "flight_time_s",
        "landing_drift_m",
        "max_acceleration_m_s2",
        "max_acceleration_time_s",
        "series_height_rms_m",
        "series_speed_rms_m_s",
    ] {
        expected_outside.push(("predicted-ndrt-2020-nose-to-tail", metric));
    }
    expected_outside.push(("predicted-juno-iii", "apogee_agl_m"));
    expected_outside.extend(drifts("predicted-juno-iii"));
    expected_outside.extend(drifts("predicted-bella-lui"));
    assert_eq!(outside, expected_outside);
    // Predicted mode is the third, and it is reported against a target rather than excused: every
    // metric of every predicted case, and no other, is a target row (ADR-023).
    for comparison in &report.comparisons {
        assert_eq!(
            comparison.targeted_row(),
            comparison.case.starts_with("predicted-"),
            "{}'s {}",
            comparison.case,
            comparison.metric
        );
    }
    assert!(report.passed());

    // The hatch itself still works, and still refuses to be a quiet pass: a declared metric is
    // printed with both numbers and its reason, and one with a blank reason fails outright.
    let declared = Comparison::not_scored(
        "a-case",
        "drift_north_m",
        5.5e-4,
        2.0e-5,
        "rocketpy 1.13.0, ..., metrics.drift_north_m",
        "  the cause is not established: issue #27  ",
    );
    assert_eq!(declared.verdict, Verdict::NotScored);
    assert!(!declared.scored());
    assert_eq!(
        declared.note.as_deref(),
        Some("the cause is not established: issue #27")
    );
    let blank = Comparison::not_scored("a-case", "drift_north_m", 5.5e-4, 2.0e-5, "...", "   ");
    assert_eq!(
        blank.verdict,
        Verdict::Fail,
        "an excuse nobody wrote down is not an excuse"
    );
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
        // The command regenerates the reference from the generator it names, and the model says
        // what that generator flew: a descent's point mass, or a whole flight's rigid body.
        assert!(source.command.contains(&source.generator), "{source:?}");
        assert_eq!(source.sha256.len(), 64, "{source:?}");
        let expected_model = if source.generator.ends_with("recovery.py") {
            "point mass"
        } else {
            assert!(source.generator.ends_with("flight.py"), "{source:?}");
            "6-DOF variable-mass rigid body"
        };
        assert!(source.model.contains(expected_model), "{source:?}");
        assert!(source.overrides.contains("noise"), "{source:?}");
    }
    assert!(
        report
            .sources
            .iter()
            .any(|source| source.generator.ends_with("flight.py")),
        "the whole flights are among the sources"
    );
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
    assert_eq!(report.cases.len(), 17, "{:?}", report.cases);
    // Five descents of six metrics, and five whole flights of seventeen in each mode; the sixth
    // whole flight is a known gap in both and compares nothing.
    assert_eq!(report.comparisons.len(), 200);
    assert_eq!(report.not_scored().len(), 11, "argued in the case files");
    assert_eq!(report.gaps.len(), 2);
    // Predicted mode's 85 rows are reported against a target and never count towards the verdict.
    let targeted = report
        .comparisons
        .iter()
        .filter(|comparison| comparison.targeted_row())
        .count();
    assert_eq!(targeted, 85);
    assert!(report.passed(), "{:?}", report.failures());
    // M2.1b2's own bar: at least five whole flights, every metric scored or argued, all passing.
    let whole_flights: Vec<&String> = report
        .cases
        .iter()
        .filter(|case| case.starts_with("flight-"))
        .filter(|case| !report.gaps.iter().any(|gap| &gap.case == *case))
        .collect();
    assert!(whole_flights.len() >= 5, "{whole_flights:?}");
    let (worst, relative) = report.worst_scored().expect("something was scored");
    assert!(
        relative < 0.03,
        "{}'s {} is at {relative}",
        worst.case,
        worst.metric
    );
    let markdown = report.to_markdown();
    assert!(
        markdown.contains("104 scored, all within tolerance"),
        "{markdown}"
    );
    assert!(
        markdown
            .contains("## Known gaps\n\n- **flight-prometheus-2022-generic-motor**: 17 metric(s)"),
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
    let flown: Vec<String> = report
        .cases
        .iter()
        .filter(|case| !report.gaps.iter().any(|gap| &gap.case == *case))
        .cloned()
        .collect();
    assert_eq!(compared, flown);
    // The committed reports are these reports, so a number that moves shows up in the diff. The
    // JSON is pinned too, to what the platforms share (`Report::reproduces`); CI's `validate` job
    // runs the same check through `cargo xtask validate --check`.
    let read = |name: &str| {
        std::fs::read_to_string(root().join("validation/reports").join(name))
            .expect("the report is committed")
    };
    if let Err(what) = report.reproduces(&read("latest.md"), &read("latest.json")) {
        panic!("{what}; run `cargo xtask validate`");
    }
    // It catches a report that moved, in either file.
    let moved = read("latest.md").replacen("| pass |", "| **fail** |", 1);
    let error = report
        .reproduces(&moved, &read("latest.json"))
        .expect_err("a verdict flipped in latest.md");
    assert!(
        error.0.starts_with("latest.md is not latest.json's"),
        "{error}"
    );
    let mut moved: Report = serde_json::from_str(&read("latest.json")).expect("it parses");
    moved.cases.pop();
    let moved = serde_json::to_string(&moved).expect("it serialises");
    let moved_markdown = serde_json::from_str::<Report>(&moved)
        .expect("it parses")
        .to_markdown();
    let error = report
        .reproduces(&moved_markdown, &moved)
        .expect_err("a case left out");
    assert!(error.0.starts_with("latest.json: the cases"), "{error}");
}

#[test]
fn reproducing_forgives_the_last_digits_and_nothing_else() {
    // A committed report of one row, and this run's, which differs in what each case says.
    let row = |measured: f64| {
        Comparison::new(
            "flight-x",
            "landing_drift_m",
            measured,
            340.986523,
            "rocketpy",
            Tolerance::relative(0.03),
        )
    };
    let report = |measured: f64| Report {
        harness_version: "0.0.0".to_owned(),
        fast: false,
        cases: vec!["flight-x".to_owned()],
        skipped: Vec::new(),
        comparisons: vec![row(measured)],
        gaps: Vec::new(),
        sources: Vec::new(),
    };
    let committed = report(345.240893);
    let json = serde_json::to_string(&committed).expect("it serialises");
    let markdown = committed.to_markdown();
    let check = |run: &Report, markdown: &str, json: &str| run.reproduces(markdown, json);
    assert_eq!(check(&committed, &markdown, &json), Ok(()));
    // Another platform's last digits, even where they move a printed percentage's last digit.
    assert_eq!(check(&report(345.240895), &markdown, &json), Ok(()));
    let at_a_boundary = report(340.986523 * 1.030_004_999_9);
    let json_there = serde_json::to_string(&at_a_boundary).expect("it serialises");
    assert_eq!(
        check(
            &report(340.986523 * 1.030_005_000_1),
            &at_a_boundary.to_markdown(),
            &json_there
        ),
        Ok(())
    );
    // A number that moved, which only the JSON shows at full precision.
    let error = check(&report(345.240993), &markdown, &json).expect_err("a number moved");
    assert!(
        error
            .0
            .starts_with("latest.json: flight-x's landing_drift_m's hpr value"),
        "{error}"
    );
    // A committed Markdown that is not its JSON's rendering.
    let edited = markdown.replace("| pass |", "| **fail** |");
    let error = check(&committed, &edited, &json).expect_err("the files disagree");
    assert!(
        error.0.starts_with("latest.md is not latest.json's"),
        "{error}"
    );
    let error =
        check(&committed, &format!("{markdown}more\n"), &json).expect_err("a line too many");
    assert!(error.0.contains("(the end)"), "{error}");
    // A committed difference that is not its own values' difference.
    let mut hand_edited = committed.clone();
    hand_edited.comparisons[0].relative = Some(0.02);
    let error = check(
        &committed,
        &hand_edited.to_markdown(),
        &serde_json::to_string(&hand_edited).expect("it serialises"),
    )
    .expect_err("a relative difference written by hand");
    assert!(
        error.0.contains("not its own values' difference"),
        "{error}"
    );
    // A note, a source or a case that changed.
    let mut noted = committed.clone();
    noted.comparisons[0].note = Some("a note".to_owned());
    assert!(check(&noted, &markdown, &json).is_err());
    let mut sourced = committed.clone();
    sourced.comparisons[0].source = "openrocket".to_owned();
    assert!(check(&sourced, &markdown, &json).is_err());
    let mut renamed = committed.clone();
    renamed.cases[0] = "flight-y".to_owned();
    assert!(check(&renamed, &markdown, &json).is_err());
}

#[test]
fn a_target_is_reported_and_never_gated_but_a_broken_one_fails() {
    // Predicted mode's rows (ADR-023): inside or outside the target, never scored, never failing
    // the run, never the worst scored difference; but a row with no bound or a number that is not
    // finite is a broken comparison, not a miss to explain.
    let row = |measured: f64, tolerance: Tolerance| {
        Comparison::targeted(
            "predicted-x",
            "apogee_agl_m",
            measured,
            100.0,
            "rocketpy",
            tolerance,
        )
    };
    let within = row(102.0, Tolerance::relative(0.03));
    let outside = row(110.0, Tolerance::relative(0.03));
    assert_eq!(within.verdict, Verdict::WithinTarget);
    assert_eq!(outside.verdict, Verdict::OutsideTarget);
    let gated = Comparison::new(
        "flight-x",
        "apogee_agl_m",
        101.0,
        100.0,
        "rocketpy",
        Tolerance::relative(0.03),
    );
    let report = Report {
        harness_version: "0.0.0".to_owned(),
        fast: false,
        cases: vec!["flight-x".to_owned(), "predicted-x".to_owned()],
        skipped: Vec::new(),
        comparisons: vec![gated, within.clone(), outside.clone()],
        gaps: Vec::new(),
        sources: Vec::new(),
    };
    assert!(!within.scored() && !outside.scored());
    assert!(within.targeted_row() && outside.targeted_row());
    assert!(report.passed());
    assert!(report.not_scored().is_empty());
    let (worst, _) = report.worst_scored().expect("the gated row");
    assert_eq!(worst.case, "flight-x");
    let markdown = report.to_markdown();
    assert!(markdown.contains("## Predicted mode"), "{markdown}");
    assert!(markdown.contains("| outside target |"), "{markdown}");
    assert!(
        markdown.contains("2 predicted-mode metric(s)"),
        "{markdown}"
    );
    for broken in [
        row(f64::NAN, Tolerance::relative(0.03)),
        row(102.0, Tolerance::default()),
    ] {
        assert_eq!(broken.verdict, Verdict::Fail);
    }
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

    // With every case marked slow, a fast run would cover nothing at all, and "ok" over nothing is
    // the report Loft's suites produced when their fixtures went missing. It fails instead.
    let scratch = scratch_case("descent-juno-iii", |_| {});
    scratch.write(
        "validation/cases/lock.toml",
        &toml::to_string(&CaseLock {
            cases: vec!["descent-juno-iii".to_owned()],
            slow: vec!["descent-juno-iii".to_owned()],
        })
        .expect("the lock serialises"),
    );
    let error = run_lock(scratch.path(), true).expect_err("a fast run that covers nothing");
    assert!(
        matches!(&error, ValidateError::Case(message) if message.contains("compared nothing")),
        "{error}"
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
fn a_peak_inside_a_step_is_found_smooth_or_kinked() {
    let found = |value: fn(f64) -> f64| {
        peak_between::<()>(0.0, 0.05, |t| Ok(value(t))).expect("the value never fails")
    };
    // A smooth peak, as speed's at burnout: its top is flat to rounding within 2.6e-9 s of it,
    // so that is how near it can be found, and the value there is the peak's to rounding.
    let speed = |t: f64| 186.68 - 4e3 * (t - 0.0317).powi(2);
    let smooth = found(speed);
    assert!((smooth - 0.0317).abs() < 5e-9, "{smooth}");
    assert!((speed(smooth) - 186.68).abs() < 1e-13, "{}", speed(smooth));
    // A kinked one, as where a wind table changes level: within 3.5e-11 of the 0.05 s bracket.
    let kinked = found(|t| 115.3 - 900.0 * (t - 0.0083).abs());
    assert!((kinked - 0.0083).abs() < 2e-12, "{kinked}");
    // At an end of the bracket it finds that end.
    let rising = found(|t| t);
    assert!((rising - 0.05).abs() < 2e-12, "{rising}");
    // And it stops at the first error.
    let mut calls = 0;
    let failed = peak_between(0.0, 1.0, |_| {
        calls += 1;
        if calls < 3 { Ok(0.0) } else { Err("no sample") }
    });
    assert_eq!((failed, calls), (Err("no sample"), 3));
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

/// `case_id`'s committed case file with `known_gap` set to `reason`, or removed for `None`.
fn with_gap(case_id: &str, reason: Option<&str>) -> String {
    let mut case = cases()
        .into_iter()
        .find(|case| case.id == case_id)
        .expect("a committed case");
    case.known_gap = reason.map(str::to_owned);
    toml::to_string(&case).expect("the case serialises")
}

#[test]
fn a_known_gap_is_checked_not_trusted() {
    // A known gap excuses a whole case, so it gets the scrutiny a not-scored metric does and more
    // (L82): the harness accepts one kind, hpr's refusal of M >= 1, and checks both sides of it.
    let file = |id: &str| format!("validation/cases/{id}.toml");

    // The committed one: the reference reaches Mach 1, hpr refuses the flight for exactly that,
    // and the report carries it with the case's reason and hpr's words, scoring nothing.
    let report = run_lock(&root(), false).expect("the committed cases run");
    let gap = report
        .gaps
        .iter()
        .find(|gap| gap.case == "flight-prometheus-2022-generic-motor")
        .expect("Prometheus is a gap");
    assert!(gap.reason.contains("Mach 1.014"), "{gap:?}");
    assert!(gap.refusal.contains("Mach 1.000"), "{gap:?}");
    assert_eq!(gap.metric_count, 17);
    assert!(
        !report
            .comparisons
            .iter()
            .any(|comparison| comparison.case == gap.case),
        "a gap scores nothing, not even a pass"
    );

    // Without the declaration, the same refusal fails the run: a case that stops short is not a
    // case that passes.
    let scratch = scratch_case("flight-prometheus-2022-generic-motor", |_| {});
    scratch.write(
        &file("flight-prometheus-2022-generic-motor"),
        &with_gap("flight-prometheus-2022-generic-motor", None),
    );
    let error = run_lock(scratch.path(), false).expect_err("an undeclared refusal fails");
    assert!(
        matches!(&error, ValidateError::Flight { what, .. } if what.contains("Mach 1.000")),
        "{error}"
    );

    // A gap the reference does not reach is refused before anything flies: Valetudo peaks at
    // Mach 0.33.
    let scratch = scratch_case("flight-valetudo", |_| {});
    scratch.write(
        &file("flight-valetudo"),
        &with_gap("flight-valetudo", Some("it goes supersonic")),
    );
    let error = run_lock(scratch.path(), false).expect_err("a gap the reference denies");
    assert!(
        error.to_string().contains("reference peaks at Mach"),
        "{error}"
    );

    // Loft lesson L85: a gap that has closed must not stay excused. With a reference that claims
    // Mach 1.2 but a flight hpr completes, the declared gap fails the run.
    let scratch = scratch_case("flight-valetudo", |document| {
        for case in document["cases"].as_array_mut().expect("cases") {
            case["metrics"]["max_mach"] = serde_json::json!(1.2);
        }
    });
    scratch.write(
        &file("flight-valetudo"),
        &with_gap("flight-valetudo", Some("it goes supersonic")),
    );
    let error = run_lock(scratch.path(), false).expect_err("a gap hpr flies through");
    assert!(error.to_string().contains("hpr flew it"), "{error}");

    // Each mode needs its own reference (ADR-023). A predicted case scored against the same-drag
    // reference would measure hpr's drag against the declared constant, and a same-drag case
    // against the own-drag reference would fly a table the reference never flew.
    let scratch = scratch_case("predicted-valetudo", |_| {});
    let predicted = std::fs::read_to_string(scratch.path().join(file("predicted-valetudo")))
        .expect("the scratch case");
    let same_drag_reference = "validation/fixtures/flight/rocketpy-whole-flight.json";
    scratch.write(
        same_drag_reference,
        &std::fs::read_to_string(root().join(same_drag_reference)).expect("committed"),
    );
    scratch.write(
        &file("predicted-valetudo"),
        &predicted.replace(
            "rocketpy-whole-flight-own-drag.json",
            "rocketpy-whole-flight.json",
        ),
    );
    let error = run_lock(scratch.path(), false).expect_err("predicted against the same drag");
    assert!(
        error
            .to_string()
            .contains("a predicted case needs a reference that flew the example's own drag"),
        "{error}"
    );
    scratch.write(
        &file("predicted-valetudo"),
        &predicted.replace("mode = \"predicted\"\n", ""),
    );
    let error = run_lock(scratch.path(), false).expect_err("same-drag against its own drag");
    assert!(
        error
            .to_string()
            .contains("a same-drag case needs a reference that flew its generator's"),
        "{error}"
    );

    // And a gap nobody explained is not one.
    let scratch = scratch_case("flight-prometheus-2022-generic-motor", |_| {});
    scratch.write(
        &file("flight-prometheus-2022-generic-motor"),
        &with_gap("flight-prometheus-2022-generic-motor", Some("   ")),
    );
    let error = run_lock(scratch.path(), false).expect_err("a blank gap");
    assert!(error.to_string().contains("gives no reason"), "{error}");
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
