use std::fs;

use serde_json::{Value, json};

use super::*;
use crate::case::Tolerance;
use crate::report::{Comparison, Gap, Source};
use crate::tests::root;

fn committed<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = root().join("validation/reports").join(name);
    let text = fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{path:?}: {error}"))
}

fn source(case: &str, oracle: &str) -> Source {
    Source {
        case: case.to_owned(),
        oracle: oracle.to_owned(),
        generator: "generator.py".to_owned(),
        command: "python generator.py".to_owned(),
        file: "fixture.json".to_owned(),
        sha256: "0".repeat(64),
        model: String::new(),
        overrides: String::new(),
    }
}

/// A harness report of one descent, one same-drag flight and one predicted flight, with `extra`
/// rows added.
fn harness(extra: Vec<Comparison>, gaps: Vec<Gap>) -> Report {
    let gate = Tolerance::relative(0.03);
    let mut comparisons = vec![
        Comparison::new("descent-d", "descent_time_s", 99.0, 100.0, "s", gate),
        Comparison::new("flight-a", "apogee_agl_m", 1010.0, 1000.0, "s", gate),
        Comparison::new("flight-a", "max_mach", 0.5, 0.5, "s", gate),
        Comparison::targeted("predicted-b", "apogee_agl_m", 2020.0, 2000.0, "s", gate),
        Comparison::targeted("predicted-b", "max_mach", 1.5, 1.5, "s", gate),
    ];
    comparisons.extend(extra);
    let mut cases = vec![
        "descent-d".to_owned(),
        "flight-a".to_owned(),
        "predicted-b".to_owned(),
    ];
    cases.extend(gaps.iter().map(|gap| gap.case.clone()));
    Report {
        harness_version: "test".to_owned(),
        fast: false,
        cases,
        skipped: Vec::new(),
        comparisons,
        gaps,
        sources: vec![
            source("descent-d", "rocketpy 1.13.0"),
            source("flight-a", "rocketpy 1.13.0"),
            source("predicted-b", "rocketpy 1.13.0"),
        ],
    }
}

fn take(report: &Report) -> Result<Census, CensusError> {
    let real: RealFlightReport = committed("real-flights.json");
    let examples: Value = committed("openrocket-flights.json");
    let library: Value = committed("openrocket-library-flights.json");
    Census::take(Reports {
        harness: report,
        real_flights: &real,
        openrocket_examples: &examples,
        openrocket_library: &library,
    })
}

/// A census of one row, `row`.
fn single(row: Row) -> Census {
    Census {
        slack_share: SLACK_SHARE,
        references: BTreeMap::from([(row.group, "rocketpy 1.13.0".to_owned())]),
        rows: vec![row],
    }
}

/// A predicted-mode apogee row `percent` off a 2000 m reference, held to a 3% target.
fn predicted_apogee(percent: f64) -> Row {
    let reference = 2000.0;
    let comparison = Comparison::targeted(
        "predicted-b",
        "apogee_agl_m",
        reference * (1.0 + percent / 100.0),
        reference,
        "s",
        Tolerance::relative(0.03),
    );
    let report = harness(Vec::new(), Vec::new());
    let mut rows = Vec::new();
    let mut references = BTreeMap::new();
    harness_rows(
        &Report {
            comparisons: vec![
                comparison,
                report.comparisons[4].clone(), // its max_mach row, to class its speed
            ],
            ..report
        },
        &mut rows,
        &mut references,
    )
    .unwrap();
    rows.into_iter()
        .find(|row| row.metric == "apogee_agl_m")
        .unwrap()
}

#[test]
fn counts_are_generated_and_each_case_counts_once() {
    // Loft lesson L84: counts written by hand, and one disagreement counted many times.
    let report = harness(Vec::new(), Vec::new());
    let census = take(&report).unwrap();
    let summaries = census.summaries();

    // Every count is the census's own: cases are distinct case ids, rows are rows.
    for summary in &summaries {
        let rows: Vec<&Row> = census
            .rows
            .iter()
            .filter(|row| row.group == summary.group)
            .collect();
        let cases: BTreeSet<&str> = rows.iter().map(|row| row.case.as_str()).collect();
        assert_eq!(summary.rows, rows.len(), "{:?}", summary.group);
        assert_eq!(summary.cases, cases.len(), "{:?}", summary.group);
        assert_eq!(
            summary.regimes.values().sum::<usize>(),
            summary.cases,
            "{:?}: each case is in one speed class",
            summary.group
        );
        assert_eq!(
            summary.standings.values().sum::<usize>(),
            summary.rows,
            "{:?}: each row has one standing",
            summary.group
        );
    }
    let flights = |group| {
        summaries
            .iter()
            .find(|summary| summary.group == group)
            .unwrap()
            .cases
    };
    assert_eq!(flights(Group::SameDrag), 1);
    assert_eq!(flights(Group::Predicted), 1);
    // The other reports' flights, counted from their rows, agree with what each report says of
    // itself.
    let real: RealFlightReport = committed("real-flights.json");
    assert_eq!(flights(Group::FlightLogs), real.summary.flights);
    let examples: Value = committed("openrocket-flights.json");
    assert_eq!(
        Some(flights(Group::OpenRocketExamples) as u64),
        examples["summary"]["flown"].as_u64()
    );
    let library: Value = committed("openrocket-library-flights.json");
    assert_eq!(
        Some(flights(Group::OpenRocketLibrary) as u64),
        library["summary"]["flown"].as_u64()
    );
    // The real flights' mean is recomputed from the rows, and matches the report's own.
    let logs = summaries
        .iter()
        .find(|summary| summary.group == Group::FlightLogs)
        .unwrap();
    let mean = logs.apogee_percent.unwrap().mean_absolute;
    assert!((mean - real.summary.mean_absolute_apogee_error_percent).abs() < 1e-12);

    // The table prints the counts it was given, and no others.
    let table = table(&summaries, None);
    assert!(
        table.contains("| 1 flight (1 subsonic) | 2 of 2 gated metrics pass;"),
        "{table}"
    );
    assert!(
        table.contains("| 1 descent (1 under a parachute) | 1 of 1 gated metrics pass |"),
        "{table}"
    );

    // A case compared twice is refused, not counted twice.
    let twice = harness(
        vec![Comparison::new(
            "flight-a",
            "apogee_agl_m",
            1010.0,
            1000.0,
            "s",
            Tolerance::relative(0.03),
        )],
        Vec::new(),
    );
    assert_eq!(
        take(&twice),
        Err(CensusError::Duplicate {
            report: "latest.json",
            case: "flight-a".to_owned(),
            metric: "apogee_agl_m".to_owned(),
        })
    );
    // So is a partial run, and a case the census has no class for.
    let fast = Report {
        fast: true,
        ..harness(Vec::new(), Vec::new())
    };
    assert!(matches!(
        take(&fast),
        Err(CensusError::Report { report: "latest.json", what }) if what.contains("partial")
    ));
    let unknown = harness(
        vec![Comparison::new(
            "hover-c",
            "apogee_agl_m",
            1.0,
            1.0,
            "s",
            Tolerance::relative(0.03),
        )],
        Vec::new(),
    );
    assert!(matches!(
        take(&unknown),
        Err(CensusError::Report { report: "latest.json", what }) if what.contains("hover-c")
    ));
}

#[test]
fn known_gap_that_starts_passing_fails_the_gate() {
    // Loft lesson L85: the "now passes" nudge used half the tolerance, so it missed a known issue
    // that closed between half and the whole of it. Here any change of standing is a change.
    let gap = Gap {
        case: "flight-g".to_owned(),
        reason: "past Mach 5".to_owned(),
        refusal: "Mach 5.2 is past the models' range".to_owned(),
        mach: 5.2,
        metric_count: 2,
    };
    let accepted = take(&harness(Vec::new(), vec![gap])).unwrap();
    // hpr flies the gap's flight now, and within its gates.
    let gate = Tolerance::relative(0.03);
    let flown = take(&harness(
        vec![
            Comparison::new("flight-g", "apogee_agl_m", 5000.0, 5000.0, "s", gate),
            Comparison::new("flight-g", "max_mach", 5.2, 5.2, "s", gate),
        ],
        Vec::new(),
    ))
    .unwrap();
    let changes = compare(&accepted, &flown);
    let described: Vec<String> = changes.iter().map(Change::describe).collect();
    assert!(
        described
            .iter()
            .any(|line| line.contains("flight-g") && line.contains("hpr flies it now")),
        "{described:#?}"
    );
    assert_eq!(
        changes.len(),
        3,
        "the gap goes and two rows come: {described:#?}"
    );

    // A predicted miss that starts meeting its target, by any margin: 3.01% to 2.99% of a 3%
    // target, well inside the band half the tolerance would have missed.
    let outside = predicted_apogee(3.01);
    let within = predicted_apogee(2.99);
    assert_eq!(outside.standing, Standing::OutsideTarget);
    assert_eq!(within.standing, Standing::WithinTarget);
    let changes = compare(&single(outside.clone()), &single(within.clone()));
    assert!(
        matches!(&changes[..], [Change::Standing { .. }]),
        "{changes:?}"
    );
    assert!(
        !changes[0].is_worse(),
        "a miss that starts passing is better"
    );
    // And the other way, a pass that starts missing.
    let changes = compare(&single(within), &single(outside));
    assert!(
        matches!(&changes[..], [Change::Standing { .. }]) && changes[0].is_worse(),
        "{changes:?}"
    );
}

#[test]
fn headline_names_oracle_kind_population_and_regime() {
    // Loft lesson L86: a headline accuracy figure published without its oracle, its kind of
    // comparison, its population or its speed.
    let report = harness(Vec::new(), Vec::new());
    let census = take(&report).unwrap();
    let summaries = census.summaries();
    assert_eq!(summaries.len(), Group::ALL.len(), "every group has rows");
    for summary in &summaries {
        let headline = summary.headline();
        assert!(!summary.reference.is_empty(), "{headline}");
        assert!(headline.contains(&summary.reference), "{headline}");
        assert!(headline.contains(summary.group.kind()), "{headline}");
        assert!(headline.contains(summary.group.bar()), "{headline}");
        let population = format!("{} {} (", summary.cases, summary.group.noun(summary.cases));
        assert!(headline.contains(&population), "{headline}");
        for (regime, count) in &summary.regimes {
            assert!(
                headline.contains(&format!("{count} {}", regime.name())),
                "{headline}"
            );
        }
    }
    let predicted = summaries
        .iter()
        .find(|summary| summary.group == Group::Predicted)
        .unwrap();
    assert_eq!(
        predicted.headline(),
        "Whole flights, each code on its own drag against rocketpy 1.13.0 (code-to-code, each \
         code's own drag; target: 3% on each metric): 1 flight (1 supersonic); 2 of 2 metrics \
         within target; apogee +1.00% to +1.00%."
    );
    // The logs' version names the altimeters, and the census's own mean.
    let logs = summaries
        .iter()
        .find(|summary| summary.group == Group::FlightLogs)
        .unwrap();
    assert!(
        logs.headline()
            .contains("measured: the teams' altimeter logs")
    );
    assert!(logs.headline().contains("mean absolute apogee error"));
}

#[test]
fn predicted_mode_regressions_fail_the_gate() {
    // Loft lesson L88: hpr's own aerodynamics were never gated against an oracle. Predicted mode's
    // 3% is a target (ADR-023), so a regenerated report could carry a worse number in unseen; the
    // census holds every predicted row to the one accepted.
    let accepted = predicted_apogee(1.0);
    assert_eq!(accepted.standing, Standing::WithinTarget);
    // 1% to 1.5%, still within the target, is a regression.
    let worse = predicted_apogee(1.5);
    assert_eq!(worse.standing, Standing::WithinTarget);
    let changes = compare(&single(accepted.clone()), &single(worse));
    assert!(
        matches!(&changes[..], [Change::Regressed { .. }]) && changes[0].is_worse(),
        "{changes:?}"
    );
    assert!(changes[0].describe().starts_with("regressed: "));
    // Its slack is 0.1% of its 60 m allowance: 6 cm.
    assert!((accepted.slack - 0.06).abs() < 1e-12, "{}", accepted.slack);
    // Just inside the slack holds; just past it doesn't.
    let inside = Row {
        difference: accepted.difference + 0.059,
        ..accepted.clone()
    };
    assert!(compare(&single(accepted.clone()), &single(inside)).is_empty());
    let past = Row {
        difference: accepted.difference + 0.061,
        ..accepted.clone()
    };
    assert!(matches!(
        &compare(&single(accepted.clone()), &single(past))[..],
        [Change::Regressed { .. }]
    ));
    // An improvement is a change too, to be accepted, so it can't slip back unseen.
    let better = predicted_apogee(0.5);
    let changes = compare(&single(accepted.clone()), &single(better));
    assert!(
        matches!(&changes[..], [Change::Improved { .. }]) && !changes[0].is_worse(),
        "{changes:?}"
    );
    // So is crossing to the other side at the same size.
    let across = predicted_apogee(-1.0);
    assert!(matches!(
        &compare(&single(accepted), &single(across))[..],
        [Change::Improved { .. }]
    ));
}

#[test]
fn a_census_reads_back_as_it_was_written() {
    let census = take(&harness(Vec::new(), Vec::new())).unwrap();
    let accepted = Accepted::new(census.clone(), None, "  the first census ");
    assert_eq!(accepted.reason, "the first census");
    let text = serde_json::to_string_pretty(&accepted).unwrap();
    let back: Accepted = serde_json::from_str(&text).unwrap();
    assert_eq!(back, accepted);
    assert!(compare(&back.census, &census).is_empty());
    let page = accepted.to_markdown();
    for summary in &accepted.summaries {
        assert!(page.contains(&summary.headline()), "{page}");
    }
}

#[test]
fn the_badge_counts_the_gated_rows_and_turns_red_on_a_failure() {
    let census = take(&harness(Vec::new(), Vec::new())).unwrap();
    let badge = badge_svg(&census.summaries());
    assert!(badge.contains("3 of 3 gated metrics pass"), "{badge}");
    assert!(badge.contains("#2e7d32"), "{badge}");
    let failing = take(&harness(
        vec![Comparison::new(
            "flight-a",
            "max_speed_m_s",
            150.0,
            100.0,
            "s",
            Tolerance::relative(0.03),
        )],
        Vec::new(),
    ))
    .unwrap();
    let badge = badge_svg(&failing.summaries());
    assert!(badge.contains("3 of 4 gated metrics pass"), "{badge}");
    assert!(badge.contains("#c62828"), "{badge}");
    // An OpenRocket row with no bar is refused, not dropped.
    let mut examples: Value = committed("openrocket-flights.json");
    examples["flights"][0]["metrics"]["new_metric"] = json!({"outcome": {"outcome": "scored"}});
    let mut rows = Vec::new();
    let mut references = BTreeMap::new();
    assert!(matches!(
        openrocket_example_rows(&examples, &mut rows, &mut references),
        Err(CensusError::Report { what, .. }) if what.contains("new_metric")
    ));
}
