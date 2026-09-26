//! The accuracy census: every number the committed reports compare, counted once, classed by what
//! it was compared with and how fast the flight went, and held to the census last accepted.
//!
//! The reports say how each case came out. The census says what they add up to, and it is the
//! gate against a regression that a regenerated report would otherwise carry in unnoticed. Four
//! reports feed it: the harness's (`validation/reports/latest.json`), the real flights'
//! (`real-flights.json`), and OpenRocket's flights of its examples and of the private designs
//! (`openrocket-flights.json`, `openrocket-library-flights.json`). Each compared number becomes a
//! [`Row`], keyed by its report's [`Group`], its case and its metric, and a key seen twice is
//! refused, so a case counts once ([Loft lesson L84][l84]).
//!
//! The census accepted last is committed (`validation/reports/census.json`). A run is held to it
//! row by row ([`compare`]): a row that moves by more than its slack in either direction, that
//! changes its standing (a miss that starts passing too, [L85][l85]), or that comes or goes, is a
//! [`Change`], and any change fails the check until the census is accepted again with a written
//! reason. A predicted-mode row, whose 3% is only a target ([ADR-023][adr-023], predicted mode), is held the same way, so hpr's
//! own aerodynamics can't drift unnoticed inside its target ([L88][l88]). Each group's headline
//! names what it was compared with, how many flights, and their speeds ([L86][l86]).
//!
//! The slack is [`SLACK_SHARE`] of the row's scale: the tolerance it is held to, or for a row held
//! to none, the bar its kind is judged by ([`Group::bar`]). It is far above the reports' own
//! reproduction noise and far below any change worth knowing about.
//!
//! [l84]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l84
//! [l85]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l85
//! [l86]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l86
//! [l88]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l88
//! [adr-023]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-023-predicted-mode-each-codes-own-drag-reported-against-a-target-2026-09-18

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::real_flight::{APOGEE_TARGET_PERCENT, RealFlightReport};
use crate::report::{Report, Verdict};

/// The share of a row's scale it may move by before the census calls it a change: 0.1%.
///
/// On a 3% tolerance that is 0.003 percentage points. The harness's report reproduces to
/// `max(2e-6, 1e-7 |x|)` across platforms and a corpus rerun moves by at most 5e-7 per cent, so the
/// slack is at least 60 times the noise, and it is floored at the harness's reproduction bound.
pub const SLACK_SHARE: f64 = 1e-3;

/// The scale of a harness row that is held to no tolerance: 3% of its reference, the bound every
/// harness gate and target is capped at ([ADR-024][adr-024], the whole flight's bounds).
///
/// [adr-024]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-024-the-time-series-rms-aligned-at-ignition-held-to-3-of-its-traces-scale-2026-09-18
pub const NOT_SCORED_SCALE: f64 = 0.03;

/// The bar an OpenRocket apogee or largest speed is judged by, per cent: the OpenRocket
/// comparison's threshold for a difference that needs a written cause ([M2.2][m2-2],
/// [ADR-070][adr-070]), and the one the staging milestone set for its flights ([M1.9c][m1-9c]).
///
/// [m2-2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2
/// [m1-9c]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-9c
/// [adr-070]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-070-m22e-split-mass-and-centre-of-mass-first-then-the-corpus-2026-09-25
pub const OPENROCKET_BAR_PERCENT: f64 = 5.0;

/// The bar a stability margin is judged by, calibres: the centre-of-pressure target set before
/// the first aerodynamics milestone measured anything ([M1.5][m1-5]). A margin's difference is its
/// centre of pressure's.
///
/// [m1-5]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-5
pub const MARGIN_BAR_CAL: f64 = 0.5;

/// The bar a real flight's altitude trace is judged by, per cent of its apogee: the bound the
/// harness holds a whole flight's height series to ([ADR-024][adr-024], the whole flight's
/// bounds).
///
/// [adr-024]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-024-the-time-series-rms-aligned-at-ignition-held-to-3-of-its-traces-scale-2026-09-18
pub const TRACE_BAR_PERCENT: f64 = 3.0;

/// The largest Mach number of a subsonic flight, and of a transonic one, as the OpenRocket
/// library report classes them.
pub const SUBSONIC_BELOW: f64 = 0.8;
/// See [`SUBSONIC_BELOW`].
pub const SUPERSONIC_ABOVE: f64 = 1.2;

/// Which report a row comes from, and so what it was compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Group {
    /// The harness's descents under a parachute against RocketPy, on the same inputs: gated.
    Descent,
    /// The harness's whole flights against RocketPy on the same drag: gated.
    SameDrag,
    /// The harness's whole flights, each code on its own drag: a target, not a gate.
    Predicted,
    /// OpenRocket's calm flights of its own examples.
    OpenRocketExamples,
    /// OpenRocket's calm flights of the private designs, under anonymised ids.
    OpenRocketLibrary,
    /// Real flights: the teams' altimeter logs.
    FlightLogs,
}

impl Group {
    /// Every group, in the census's order.
    pub const ALL: [Self; 6] = [
        Self::Descent,
        Self::SameDrag,
        Self::Predicted,
        Self::OpenRocketExamples,
        Self::OpenRocketLibrary,
        Self::FlightLogs,
    ];

    /// What was flown, in a few words.
    #[must_use]
    pub const fn what(self) -> &'static str {
        match self {
            Self::Descent => "descents under a parachute",
            Self::SameDrag => "whole flights on the same drag",
            Self::Predicted => "whole flights, each code on its own drag",
            Self::OpenRocketExamples => "calm flights of OpenRocket's examples",
            Self::OpenRocketLibrary => "calm flights of the private designs",
            Self::FlightLogs => "real flights",
        }
    }

    /// The kind of evidence: another program's answer or a measurement.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Descent | Self::SameDrag => "code-to-code, same inputs",
            Self::Predicted => "code-to-code, each code's own drag",
            Self::OpenRocketExamples | Self::OpenRocketLibrary => {
                "code-to-code, each code's own model"
            }
            Self::FlightLogs => "measured: the teams' altimeter logs",
        }
    }

    /// What a row is held to: a gate fails the harness, a target is reported, and a bar only
    /// scales the census's slack and marks what needs a written cause.
    #[must_use]
    pub const fn bar(self) -> &'static str {
        match self {
            Self::Descent | Self::SameDrag => "gate: each metric's tolerance, at most 3%",
            Self::Predicted => "target: 3% on each metric",
            Self::OpenRocketExamples | Self::OpenRocketLibrary => {
                "no target; over 5% needs a written cause"
            }
            Self::FlightLogs => "target: mean absolute apogee error 5%",
        }
    }

    /// The noun for one of its cases.
    const fn noun(self, count: usize) -> &'static str {
        match (self, count) {
            (Self::Descent, 1) => "descent",
            (Self::Descent, _) => "descents",
            (_, 1) => "flight",
            _ => "flights",
        }
    }
}

/// How fast a case flew: the class of its largest Mach number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Regime {
    /// A descent under a parachute, slow by construction.
    Descent,
    /// Below Mach [`SUBSONIC_BELOW`].
    Subsonic,
    /// From Mach [`SUBSONIC_BELOW`] to [`SUPERSONIC_ABOVE`].
    Transonic,
    /// Above Mach [`SUPERSONIC_ABOVE`].
    Supersonic,
}

impl Regime {
    /// The class of a flight whose largest Mach number is `mach`.
    #[must_use]
    pub fn of_mach(mach: f64) -> Self {
        if mach < SUBSONIC_BELOW {
            Self::Subsonic
        } else if mach <= SUPERSONIC_ABOVE {
            Self::Transonic
        } else {
            Self::Supersonic
        }
    }

    /// Its name, as the reports write it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Descent => "under a parachute",
            Self::Subsonic => "subsonic",
            Self::Transonic => "transonic",
            Self::Supersonic => "supersonic",
        }
    }
}

/// Where a row stands against what it is held to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Standing {
    /// Inside its gate.
    Pass,
    /// Outside its gate.
    Fail,
    /// Inside its target.
    WithinTarget,
    /// Outside its target.
    OutsideTarget,
    /// Measured and held to nothing, for a written reason.
    NotScored,
    /// Inside the bar its kind is judged by, where there is no target.
    WithinBar,
    /// Over that bar.
    OverBar,
    /// Not compared: the report withholds it, and says why.
    Withheld,
    /// A known gap: a flight hpr refuses, compared with nothing.
    Gap,
}

impl Standing {
    /// Its name, as the census writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::WithinTarget => "within target",
            Self::OutsideTarget => "outside target",
            Self::NotScored => "not scored",
            Self::WithinBar => "within the bar",
            Self::OverBar => "over the bar",
            Self::Withheld => "withheld",
            Self::Gap => "known gap",
        }
    }
}

/// The unit a row's difference is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Unit {
    /// The metric's own unit, as its name says (`_m`, `_s`, `_m_s`, or none for a Mach number).
    Metric,
    /// Per cent of the reference.
    Percent,
    /// Calibres.
    Calibre,
}

impl Unit {
    const fn suffix(self) -> &'static str {
        match self {
            Self::Metric => "",
            Self::Percent => "%",
            Self::Calibre => " cal",
        }
    }
}

/// One compared number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// The report it comes from.
    pub group: Group,
    /// The case: a harness case id, an OpenRocket design and configuration, an anonymised id, or
    /// a real flight's id.
    pub case: String,
    /// The metric.
    pub metric: String,
    /// hpr less the reference, in [`Row::unit`]; for a trace, the RMS itself.
    pub difference: f64,
    /// The difference as a percentage of the reference, where it is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    /// The unit of the difference, the scale and the slack.
    pub unit: Unit,
    /// What the difference is measured against: the row's tolerance, or its kind's bar.
    pub scale: f64,
    /// How far the difference may move before the census calls it a change.
    pub slack: f64,
    /// Where it stands.
    pub standing: Standing,
    /// How fast its case flew.
    pub regime: Regime,
}

impl Row {
    fn key(&self) -> (Group, &str, &str) {
        (self.group, &self.case, &self.metric)
    }

    /// Its difference as a share of its scale.
    #[must_use]
    pub fn share(&self) -> f64 {
        if self.scale > 0.0 {
            self.difference.abs() / self.scale
        } else {
            f64::NAN
        }
    }

    fn describe(&self) -> String {
        format!(
            "{}: {} {}",
            self.group.what(),
            self.case,
            if self.metric.is_empty() {
                "(the case)"
            } else {
                &self.metric
            }
        )
    }
}

/// What the census counts: every row, and what each group was compared with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Census {
    /// [`SLACK_SHARE`] when it was taken.
    pub slack_share: f64,
    /// Each group's reference, as its report names it (a program and its version, or the logs).
    pub references: BTreeMap<Group, String>,
    /// Every row, sorted by group, case and metric.
    pub rows: Vec<Row>,
}

/// Why a census could not be taken.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CensusError {
    /// A report said something the census can't read.
    #[error("{report}: {what}")]
    Report {
        /// The report.
        report: &'static str,
        /// What is wrong.
        what: String,
    },
    /// A row's key came twice: a case would count twice.
    #[error("{report}: {case} {metric} is compared twice; a case counts once (L84)")]
    Duplicate {
        /// The report.
        report: &'static str,
        /// The case.
        case: String,
        /// The metric.
        metric: String,
    },
}

/// The reports the census reads, as parsed from their committed files.
#[derive(Debug, Clone, Copy)]
pub struct Reports<'a> {
    /// `validation/reports/latest.json`.
    pub harness: &'a Report,
    /// `validation/reports/real-flights.json`.
    pub real_flights: &'a RealFlightReport,
    /// `validation/reports/openrocket-flights.json`.
    pub openrocket_examples: &'a Value,
    /// `validation/reports/openrocket-library-flights.json`.
    pub openrocket_library: &'a Value,
}

const HARNESS: &str = "latest.json";
const REAL: &str = "real-flights.json";
const EXAMPLES: &str = "openrocket-flights.json";
const LIBRARY: &str = "openrocket-library-flights.json";

impl Census {
    /// Takes the census of `reports`.
    ///
    /// # Errors
    ///
    /// [`CensusError::Duplicate`] when a case's metric comes twice, and [`CensusError::Report`]
    /// when a report is partial or holds something the census has no class for.
    pub fn take(reports: Reports<'_>) -> Result<Self, CensusError> {
        let mut rows = Vec::new();
        let mut references = BTreeMap::new();
        harness_rows(reports.harness, &mut rows, &mut references)?;
        real_flight_rows(reports.real_flights, &mut rows, &mut references);
        openrocket_example_rows(reports.openrocket_examples, &mut rows, &mut references)?;
        openrocket_library_rows(reports.openrocket_library, &mut rows, &mut references)?;
        let mut seen = BTreeSet::new();
        for row in &rows {
            if !seen.insert(row.key()) {
                return Err(CensusError::Duplicate {
                    report: match row.group {
                        Group::Descent | Group::SameDrag | Group::Predicted => HARNESS,
                        Group::FlightLogs => REAL,
                        Group::OpenRocketExamples => EXAMPLES,
                        Group::OpenRocketLibrary => LIBRARY,
                    },
                    case: row.case.clone(),
                    metric: row.metric.clone(),
                });
            }
        }
        rows.sort_by(|a, b| a.key().cmp(&b.key()));
        Ok(Self {
            slack_share: SLACK_SHARE,
            references,
            rows,
        })
    }

    /// What each group adds up to, in [`Group::ALL`]'s order, leaving out a group with no rows.
    #[must_use]
    pub fn summaries(&self) -> Vec<Summary> {
        Group::ALL
            .into_iter()
            .filter_map(|group| Summary::of(self, group))
            .collect()
    }
}

fn harness_group(case: &str) -> Result<Group, CensusError> {
    if case.starts_with("descent-") {
        Ok(Group::Descent)
    } else if case.starts_with("flight-") {
        Ok(Group::SameDrag)
    } else if case.starts_with("predicted-") {
        Ok(Group::Predicted)
    } else {
        Err(CensusError::Report {
            report: HARNESS,
            what: format!(
                "case {case} is neither a descent, a same-drag flight nor a predicted one, by its id"
            ),
        })
    }
}

fn harness_rows(
    report: &Report,
    rows: &mut Vec<Row>,
    references: &mut BTreeMap<Group, String>,
) -> Result<(), CensusError> {
    if report.fast || !report.skipped.is_empty() {
        return Err(CensusError::Report {
            report: HARNESS,
            what: "a partial run: the census counts the whole suite".to_owned(),
        });
    }
    let mut oracles: BTreeMap<Group, BTreeSet<&str>> = BTreeMap::new();
    for source in &report.sources {
        oracles
            .entry(harness_group(&source.case)?)
            .or_default()
            .insert(&source.oracle);
    }
    for (group, names) in oracles {
        references.insert(group, names.into_iter().collect::<Vec<_>>().join("; "));
    }
    let regime = |case: &str| -> Result<Regime, CensusError> {
        if harness_group(case)? == Group::Descent {
            return Ok(Regime::Descent);
        }
        if let Some(gap) = report.gaps.iter().find(|gap| gap.case == case) {
            return Ok(Regime::of_mach(gap.mach));
        }
        report
            .comparisons
            .iter()
            .find(|row| row.case == case && row.metric == "max_mach")
            .map(|row| Regime::of_mach(row.reference))
            .ok_or_else(|| CensusError::Report {
                report: HARNESS,
                what: format!("case {case} has no max_mach row to class its speed by"),
            })
    };
    for comparison in &report.comparisons {
        let scale = if comparison.tolerance.is_set() {
            comparison.tolerance.allowed(comparison.reference)
        } else {
            NOT_SCORED_SCALE * comparison.reference.abs()
        };
        // The harness's own reproduction bound on a measured value (`Report::reproduces`).
        let noise = 2e-6_f64.max(1e-7 * comparison.reference.abs());
        rows.push(Row {
            group: harness_group(&comparison.case)?,
            case: comparison.case.clone(),
            metric: comparison.metric.clone(),
            difference: comparison.difference,
            percent: comparison.relative.map(|relative| 100.0 * relative),
            unit: Unit::Metric,
            scale,
            slack: (SLACK_SHARE * scale).max(noise),
            standing: match comparison.verdict {
                Verdict::Pass => Standing::Pass,
                Verdict::Fail => Standing::Fail,
                Verdict::NotScored => Standing::NotScored,
                Verdict::WithinTarget => Standing::WithinTarget,
                Verdict::OutsideTarget => Standing::OutsideTarget,
            },
            regime: regime(&comparison.case)?,
        });
    }
    for gap in &report.gaps {
        rows.push(Row {
            group: harness_group(&gap.case)?,
            case: gap.case.clone(),
            metric: String::new(),
            difference: 0.0,
            percent: None,
            unit: Unit::Metric,
            scale: 0.0,
            slack: 0.0,
            standing: Standing::Gap,
            regime: Regime::of_mach(gap.mach),
        });
    }
    Ok(())
}

fn real_flight_rows(
    report: &RealFlightReport,
    rows: &mut Vec<Row>,
    references: &mut BTreeMap<Group, String>,
) {
    references.insert(
        Group::FlightLogs,
        "the teams' altimeter logs, from RocketPy 1.13.0's examples".to_owned(),
    );
    for flight in &report.flights {
        let regime = Regime::of_mach(flight.hpr_max_mach);
        let bar = |difference: f64, scale: f64, within: Standing, over: Standing| Row {
            group: Group::FlightLogs,
            case: flight.id.clone(),
            metric: String::new(),
            difference,
            percent: Some(difference),
            unit: Unit::Percent,
            scale,
            slack: SLACK_SHARE * scale,
            standing: if difference.abs() <= scale {
                within
            } else {
                over
            },
            regime,
        };
        rows.push(Row {
            metric: "apogee".to_owned(),
            ..bar(
                flight.apogee_error_percent,
                APOGEE_TARGET_PERCENT,
                Standing::WithinTarget,
                Standing::OutsideTarget,
            )
        });
        rows.push(Row {
            metric: "ascent trace RMS".to_owned(),
            ..bar(
                flight.trace_rms_percent,
                TRACE_BAR_PERCENT,
                Standing::WithinBar,
                Standing::OverBar,
            )
        });
    }
}

fn openrocket_reference(report: &Value, name: &'static str) -> Result<String, CensusError> {
    match (
        report["reference"]["tool"].as_str(),
        report["reference"]["version"].as_str(),
    ) {
        (Some(tool), Some(version)) => Ok(format!("{tool} {version}")),
        _ => Err(CensusError::Report {
            report: name,
            what: "no reference tool and version".to_owned(),
        }),
    }
}

/// A row of an OpenRocket report: `value` when the report scored it, withheld otherwise.
#[allow(
    clippy::too_many_arguments,
    reason = "one row's fields, named at each call"
)]
fn openrocket_row(
    group: Group,
    case: &str,
    metric: &str,
    scored: bool,
    value: Option<f64>,
    unit: Unit,
    scale: f64,
    regime: Regime,
) -> Result<Row, CensusError> {
    let (difference, standing) = match (scored, value) {
        (true, Some(value)) if value.is_finite() => (
            value,
            if value.abs() <= scale {
                Standing::WithinBar
            } else {
                Standing::OverBar
            },
        ),
        (false, _) => (0.0, Standing::Withheld),
        (true, _) => {
            return Err(CensusError::Report {
                report: if group == Group::OpenRocketLibrary {
                    LIBRARY
                } else {
                    EXAMPLES
                },
                what: format!("{case} {metric} is scored with no finite difference"),
            });
        }
    };
    Ok(Row {
        group,
        case: case.to_owned(),
        metric: metric.to_owned(),
        difference,
        percent: (unit == Unit::Percent && standing != Standing::Withheld).then_some(difference),
        unit,
        scale,
        slack: SLACK_SHARE * scale,
        standing,
        regime,
    })
}

fn flights<'a>(report: &'a Value, name: &'static str) -> Result<&'a Vec<Value>, CensusError> {
    report["flights"]
        .as_array()
        .ok_or_else(|| CensusError::Report {
            report: name,
            what: "no flights".to_owned(),
        })
}

fn openrocket_example_rows(
    report: &Value,
    rows: &mut Vec<Row>,
    references: &mut BTreeMap<Group, String>,
) -> Result<(), CensusError> {
    references.insert(
        Group::OpenRocketExamples,
        openrocket_reference(report, EXAMPLES)?,
    );
    for flight in flights(report, EXAMPLES)? {
        let missing = |what: &str| CensusError::Report {
            report: EXAMPLES,
            what: format!("a flight has no {what}"),
        };
        let design = flight["design"].as_str().ok_or_else(|| missing("design"))?;
        let configuration = flight["configuration"]
            .as_str()
            .ok_or_else(|| missing("configuration"))?;
        let case = format!("{design} / {configuration}");
        let regime = Regime::of_mach(
            flight["max_mach_openrocket"]
                .as_f64()
                .ok_or_else(|| missing("largest Mach number"))?,
        );
        let metrics = flight["metrics"]
            .as_object()
            .ok_or_else(|| missing("metrics"))?;
        for (metric, entry) in metrics {
            let scored = entry["outcome"]["outcome"].as_str() == Some("scored");
            let (value, unit, scale) = match metric.as_str() {
                "apogee_m" | "max_speed_m_s" => (
                    entry["relative_percent"].as_f64(),
                    Unit::Percent,
                    OPENROCKET_BAR_PERCENT,
                ),
                "rod_clearance_margin_cal" => {
                    (entry["difference"].as_f64(), Unit::Calibre, MARGIN_BAR_CAL)
                }
                other => {
                    return Err(CensusError::Report {
                        report: EXAMPLES,
                        what: format!("{case}: the census has no bar for the metric {other}"),
                    });
                }
            };
            rows.push(openrocket_row(
                Group::OpenRocketExamples,
                &case,
                metric,
                scored,
                value,
                unit,
                scale,
                regime,
            )?);
        }
    }
    Ok(())
}

fn openrocket_library_rows(
    report: &Value,
    rows: &mut Vec<Row>,
    references: &mut BTreeMap<Group, String>,
) -> Result<(), CensusError> {
    references.insert(
        Group::OpenRocketLibrary,
        openrocket_reference(report, LIBRARY)?,
    );
    for flight in flights(report, LIBRARY)? {
        let missing = |what: &str| CensusError::Report {
            report: LIBRARY,
            what: format!("a flight has no {what}"),
        };
        let case = flight["flight"].as_str().ok_or_else(|| missing("id"))?;
        let regime = match flight["mach"].as_str() {
            Some("subsonic") => Regime::Subsonic,
            Some("transonic") => Regime::Transonic,
            Some("supersonic") => Regime::Supersonic,
            _ => return Err(missing("Mach class")),
        };
        for (metric, value, outcome, unit, scale) in [
            (
                "apogee",
                "apogee_percent",
                "apogee_outcome",
                Unit::Percent,
                OPENROCKET_BAR_PERCENT,
            ),
            (
                "max_speed",
                "max_speed_percent",
                "max_speed_outcome",
                Unit::Percent,
                OPENROCKET_BAR_PERCENT,
            ),
            (
                "margin",
                "margin_cal",
                "margin_outcome",
                Unit::Calibre,
                MARGIN_BAR_CAL,
            ),
        ] {
            let scored = match flight[outcome].as_str() {
                Some(outcome) => outcome == "scored",
                None => return Err(missing(outcome)),
            };
            rows.push(openrocket_row(
                Group::OpenRocketLibrary,
                case,
                metric,
                scored,
                flight[value].as_f64(),
                unit,
                scale,
                regime,
            )?);
        }
    }
    Ok(())
}

/// A spread of differences: the least, the largest and the mean magnitude.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Spread {
    /// How many.
    pub count: usize,
    /// The least.
    pub min: f64,
    /// The largest.
    pub max: f64,
    /// The mean of their magnitudes.
    pub mean_absolute: f64,
    /// How many are within the bar they are judged by.
    pub within: usize,
}

impl Spread {
    fn of(rows: &[&Row]) -> Option<Self> {
        let values: Vec<f64> = rows.iter().filter_map(|row| row.percent).collect();
        if values.is_empty() {
            return None;
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "a count of rows, far below 2^52"
        )]
        let count = values.len() as f64;
        Some(Self {
            count: values.len(),
            min: values.iter().copied().fold(f64::INFINITY, f64::min),
            max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            mean_absolute: values.iter().map(|value| value.abs()).sum::<f64>() / count,
            within: rows
                .iter()
                .filter(|row| row.percent.is_some() && row.difference.abs() <= row.scale)
                .count(),
        })
    }
}

/// What one group adds up to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// The group.
    pub group: Group,
    /// What it was compared with.
    pub reference: String,
    /// Its cases: flights, or descents.
    pub cases: usize,
    /// Its cases by speed.
    pub regimes: BTreeMap<Regime, usize>,
    /// Its rows.
    pub rows: usize,
    /// Its rows by standing.
    pub standings: BTreeMap<Standing, usize>,
    /// The apogee differences, per cent: the harness's relative to RocketPy's, OpenRocket's
    /// and the logs' as their reports give them. `None` for descents.
    pub apogee_percent: Option<Spread>,
}

impl Summary {
    fn of(census: &Census, group: Group) -> Option<Self> {
        let rows: Vec<&Row> = census
            .rows
            .iter()
            .filter(|row| row.group == group)
            .collect();
        if rows.is_empty() {
            return None;
        }
        let mut cases: BTreeMap<&str, Regime> = BTreeMap::new();
        let mut standings = BTreeMap::new();
        for row in &rows {
            cases.insert(&row.case, row.regime);
            *standings.entry(row.standing).or_insert(0) += 1;
        }
        let mut regimes = BTreeMap::new();
        for regime in cases.values() {
            *regimes.entry(*regime).or_insert(0) += 1;
        }
        let apogees: Vec<&Row> = rows
            .iter()
            .copied()
            .filter(|row| matches!(row.metric.as_str(), "apogee" | "apogee_m" | "apogee_agl_m"))
            .filter(|row| row.percent.is_some())
            .collect();
        Some(Self {
            group,
            reference: census.references.get(&group).cloned().unwrap_or_default(),
            cases: cases.len(),
            regimes,
            rows: rows.len(),
            standings,
            apogee_percent: Spread::of(&apogees),
        })
    }

    fn count(&self, standing: Standing) -> usize {
        self.standings.get(&standing).copied().unwrap_or(0)
    }

    /// Its cases by speed, in words: "7 subsonic, 1 transonic".
    #[must_use]
    pub fn regimes_in_words(&self) -> String {
        self.regimes
            .iter()
            .map(|(regime, count)| format!("{count} {}", regime.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Its result in words: the counts against its gate, target or bar, and its apogees.
    #[must_use]
    pub fn result(&self) -> String {
        let mut parts = Vec::new();
        let gated = self.count(Standing::Pass) + self.count(Standing::Fail);
        let targeted = self.count(Standing::WithinTarget) + self.count(Standing::OutsideTarget);
        let barred = self.count(Standing::WithinBar) + self.count(Standing::OverBar);
        if gated > 0 {
            parts.push(format!(
                "{} of {gated} gated metrics pass",
                self.count(Standing::Pass)
            ));
        }
        if targeted > 0 && self.group != Group::FlightLogs {
            parts.push(format!(
                "{} of {targeted} metrics within target",
                self.count(Standing::WithinTarget)
            ));
        }
        if barred > 0 && self.group != Group::FlightLogs {
            parts.push(format!(
                "{} of {barred} differences within the bar",
                self.count(Standing::WithinBar)
            ));
        }
        for (standing, words) in [
            (Standing::NotScored, "not scored, each for a written reason"),
            (Standing::Withheld, "withheld by the report"),
            (Standing::Gap, "known gaps"),
        ] {
            if self.count(standing) > 0 {
                parts.push(format!("{} {words}", self.count(standing)));
            }
        }
        if let Some(apogee) = self.apogee_percent {
            if self.group == Group::FlightLogs {
                parts.push(format!(
                    "mean absolute apogee error {:.2}% against the 5% target, {}; apogee {:+.2}% \
                     to {:+.2}%, {} of {} within 5%",
                    apogee.mean_absolute,
                    if apogee.mean_absolute <= APOGEE_TARGET_PERCENT {
                        "within it"
                    } else {
                        "outside it"
                    },
                    apogee.min,
                    apogee.max,
                    apogee.within,
                    apogee.count
                ));
            } else {
                parts.push(format!("apogee {:+.2}% to {:+.2}%", apogee.min, apogee.max));
            }
        }
        parts.join("; ")
    }

    /// Its headline: what was flown, against what and of which kind, how many and how fast, and
    /// the result ([Loft lesson L86][l86]).
    ///
    /// [l86]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l86
    #[must_use]
    pub fn headline(&self) -> String {
        format!(
            "{} against {} ({}; {}): {} {} ({}); {}.",
            capitalised(self.group.what()),
            self.reference,
            self.group.kind(),
            self.group.bar(),
            self.cases,
            self.group.noun(self.cases),
            self.regimes_in_words(),
            self.result()
        )
    }
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// How a row differs from the census accepted last.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Change {
    /// A row the accepted census doesn't have.
    Added(Row),
    /// A row the accepted census has and this one doesn't.
    Removed(Row),
    /// A row whose standing changed, better or worse.
    Standing {
        /// As accepted.
        accepted: Row,
        /// Now.
        now: Row,
    },
    /// A row that moved by more than its slack, further from its reference.
    Regressed {
        /// As accepted.
        accepted: Row,
        /// Now.
        now: Row,
    },
    /// A row that moved by more than its slack, closer to its reference or across it.
    Improved {
        /// As accepted.
        accepted: Row,
        /// Now.
        now: Row,
    },
    /// A row whose unit, scale or speed class changed: it is not the same comparison.
    Redefined {
        /// As accepted.
        accepted: Row,
        /// Now.
        now: Row,
    },
}

impl Change {
    /// One line saying what changed.
    #[must_use]
    pub fn describe(&self) -> String {
        let moved = |accepted: &Row, now: &Row| {
            format!(
                "{:+.6}{unit} ({:.2}% of its scale) to {:+.6}{unit} ({:.2}%), slack {:.2e}",
                accepted.difference,
                100.0 * accepted.share(),
                now.difference,
                100.0 * now.share(),
                now.slack,
                unit = now.unit.suffix()
            )
        };
        match self {
            Self::Added(row) => format!("added: {} ({})", row.describe(), row.standing.name()),
            Self::Removed(row) => {
                if row.standing == Standing::Gap {
                    format!(
                        "removed: {}, a known gap: hpr flies it now (L85)",
                        row.describe()
                    )
                } else {
                    format!("removed: {} ({})", row.describe(), row.standing.name())
                }
            }
            Self::Standing { accepted, now } => format!(
                "{}: {} to {} (L85 when it starts passing); {}",
                now.describe(),
                accepted.standing.name(),
                now.standing.name(),
                moved(accepted, now)
            ),
            Self::Regressed { accepted, now } => {
                format!("regressed: {}: {}", now.describe(), moved(accepted, now))
            }
            Self::Improved { accepted, now } => {
                format!("improved: {}: {}", now.describe(), moved(accepted, now))
            }
            Self::Redefined { accepted, now } => format!(
                "redefined: {}: unit, scale or speed class changed ({:?} {} {} to {:?} {} {})",
                now.describe(),
                accepted.unit,
                accepted.scale,
                accepted.regime.name(),
                now.unit,
                now.scale,
                now.regime.name()
            ),
        }
    }

    /// Whether it makes a result worse: a regression, a row lost, or a standing that fell.
    #[must_use]
    pub fn is_worse(&self) -> bool {
        match self {
            Self::Regressed { .. } | Self::Removed(_) => true,
            Self::Standing { accepted, now } => rank(now.standing) < rank(accepted.standing),
            Self::Added(_) | Self::Improved { .. } | Self::Redefined { .. } => false,
        }
    }
}

/// How good a standing is, higher better, for telling a fall from a rise.
const fn rank(standing: Standing) -> u8 {
    match standing {
        Standing::Fail => 0,
        Standing::Gap => 1,
        Standing::Withheld | Standing::NotScored => 2,
        Standing::OutsideTarget | Standing::OverBar => 3,
        Standing::WithinTarget | Standing::WithinBar => 4,
        Standing::Pass => 5,
    }
}

/// Every way `now` differs from `accepted`, in row order. Empty when the census holds.
///
/// A row is a change when its standing differs, when its unit, scale or speed class differs, or
/// when its difference moved by more than its slack (the larger of the two rows' slacks) either
/// way: a ratchet that only watched for regressions would let an improvement slip back unseen.
#[must_use]
pub fn compare(accepted: &Census, now: &Census) -> Vec<Change> {
    let before: BTreeMap<_, &Row> = accepted.rows.iter().map(|row| (row.key(), row)).collect();
    let after: BTreeMap<_, &Row> = now.rows.iter().map(|row| (row.key(), row)).collect();
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
    let mut changes = Vec::new();
    for key in keys {
        match (before.get(&key), after.get(&key)) {
            (Some(was), None) => changes.push(Change::Removed((*was).clone())),
            (None, Some(is)) => changes.push(Change::Added((*is).clone())),
            (Some(was), Some(is)) => {
                let (was, is) = ((*was).clone(), (*is).clone());
                let moved = (is.difference - was.difference).abs() > was.slack.max(is.slack);
                if was.standing != is.standing {
                    changes.push(Change::Standing {
                        accepted: was,
                        now: is,
                    });
                } else if was.unit != is.unit
                    || was.regime != is.regime
                    || (was.scale - is.scale).abs() > 1e-9 * was.scale.abs().max(is.scale.abs())
                {
                    changes.push(Change::Redefined {
                        accepted: was,
                        now: is,
                    });
                } else if moved {
                    if is.difference.abs() > was.difference.abs() {
                        changes.push(Change::Regressed {
                            accepted: was,
                            now: is,
                        });
                    } else {
                        changes.push(Change::Improved {
                            accepted: was,
                            now: is,
                        });
                    }
                }
            }
            (None, None) => {}
        }
    }
    changes
}

/// The census as accepted: the rows, what they add up to, and why it was last accepted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Accepted {
    /// The command that writes it.
    pub generated_by: String,
    /// Why the census was last accepted, in the words of whoever accepted it.
    pub reason: String,
    /// What changed then, one line each.
    pub changes: Vec<String>,
    /// What each group added up to.
    pub summaries: Vec<Summary>,
    /// The census.
    pub census: Census,
}

impl Accepted {
    /// The census `census`, accepted over `previous` for `reason`.
    #[must_use]
    pub fn new(census: Census, previous: Option<&Census>, reason: &str) -> Self {
        let changes = previous.map_or_else(
            || vec![format!("the first census: {} rows", census.rows.len())],
            |previous| {
                compare(previous, &census)
                    .iter()
                    .map(Change::describe)
                    .collect()
            },
        );
        Self {
            generated_by: "cargo xtask census --accept".to_owned(),
            reason: reason.trim().to_owned(),
            changes,
            summaries: census.summaries(),
            census,
        }
    }

    /// The census page, `validation/reports/census.md`.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# Accuracy census\n");
        let _ = writeln!(
            out,
            "Generated by `{}` from the committed reports; do not edit. `cargo xtask validate \
             --check` holds every row of those reports to this census: a row that moves by more \
             than {}% of its scale, changes its standing, or comes or goes fails it until the \
             census is accepted again with a reason.\n",
            self.generated_by,
            100.0 * self.census.slack_share
        );
        let _ = writeln!(out, "## Headlines\n");
        for summary in &self.summaries {
            let _ = writeln!(out, "- {}", summary.headline());
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "{}", table(&self.summaries, None));
        let _ = writeln!(out, "{}", reading());
        let _ = writeln!(out, "## Rows by standing\n");
        let _ = writeln!(out, "| group | rows | standing | count |");
        let _ = writeln!(out, "|---|---:|---|---:|");
        for summary in &self.summaries {
            for (standing, count) in &summary.standings {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {count} |",
                    summary.group.what(),
                    summary.rows,
                    standing.name()
                );
            }
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "## Last accepted\n");
        let _ = writeln!(out, "Reason: {}\n", self.reason);
        const SHOWN: usize = 50;
        for change in self.changes.iter().take(SHOWN) {
            let _ = writeln!(out, "- {change}");
        }
        if self.changes.len() > SHOWN {
            let _ = writeln!(
                out,
                "- and {} more, in `census.json`",
                self.changes.len() - SHOWN
            );
        }
        out
    }
}

/// How to read the census: its speed classes, its slack and each kind of row's scale, from the
/// constants that set them.
fn reading() -> String {
    // A worked example: a relative tolerance on a round apogee.
    let (tolerance, apogee_m) = (NOT_SCORED_SCALE, 1000.0);
    let allowed_m = tolerance * apogee_m;
    let mut out = String::new();
    let _ = writeln!(out, "## How to read it\n");
    let _ = writeln!(
        out,
        "Each flight is classed by its largest Mach number: subsonic below {SUBSONIC_BELOW}, \
         transonic from {SUBSONIC_BELOW} to {SUPERSONIC_ABOVE}, supersonic above \
         {SUPERSONIC_ABOVE}. The harness's flights are classed by RocketPy's largest Mach \
         number, OpenRocket's by OpenRocket's, and the logged flights by hpr's, since a log has \
         none.\n"
    );
    let _ = writeln!(
        out,
        "A row's slack is {}% of its scale. The scale is the row's own tolerance where it has \
         one, and otherwise the bar of its kind:\n",
        100.0 * SLACK_SHARE
    );
    let _ = writeln!(out, "| row | scale |");
    let _ = writeln!(out, "|---|---|");
    for (row, scale) in [
        (
            "harness metric, held to a tolerance",
            "that tolerance".to_owned(),
        ),
        (
            "harness metric, not scored",
            format!("{}% of its reference", 100.0 * NOT_SCORED_SCALE),
        ),
        (
            "OpenRocket apogee or largest speed",
            format!("{OPENROCKET_BAR_PERCENT}%"),
        ),
        (
            "OpenRocket stability margin",
            format!("{MARGIN_BAR_CAL} calibres"),
        ),
        ("logged apogee", format!("{APOGEE_TARGET_PERCENT}%")),
        (
            "logged climb (the trace's RMS, as a share of the apogee)",
            format!("{TRACE_BAR_PERCENT}%"),
        ),
    ] {
        let _ = writeln!(out, "| {row} | {scale} |");
    }
    let _ = writeln!(
        out,
        "\nFor example, a {}% tolerance on a {apogee_m} m apogee allows {allowed_m} m, and the \
         census lets the difference move by {} m before it fails.",
        100.0 * tolerance,
        SLACK_SHARE * allowed_m
    );
    out
}

/// The census table, as the README and the accuracy page show it, each row's reference linked to
/// `link` (the census page) where one is given.
#[must_use]
pub fn table(summaries: &[Summary], link: Option<&str>) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "| compared with | kind | held to | flights (speed) | result |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for summary in summaries {
        let _ = writeln!(
            out,
            "| {}: {} | {} | {} | {} {} ({}) | {} |",
            link.map_or_else(
                || summary.reference.clone(),
                |link| format!("[{}]({link})", summary.reference)
            ),
            summary.group.what(),
            summary.group.kind(),
            summary.group.bar(),
            summary.cases,
            summary.group.noun(summary.cases),
            summary.regimes_in_words(),
            summary.result()
        );
    }
    out
}

/// The badge: the gated code-to-code metrics that pass, of all of them.
#[must_use]
pub fn badge_svg(summaries: &[Summary]) -> String {
    let (pass, gated) = summaries
        .iter()
        .filter(|summary| matches!(summary.group, Group::Descent | Group::SameDrag))
        .fold((0, 0), |(pass, gated), summary| {
            (
                pass + summary.count(Standing::Pass),
                gated + summary.count(Standing::Pass) + summary.count(Standing::Fail),
            )
        });
    let label = "vs RocketPy, same inputs";
    let message = format!("{pass} of {gated} gated metrics pass");
    let colour = if pass == gated && gated > 0 {
        "#2e7d32"
    } else {
        "#c62828"
    };
    // Verdana at 11 px averages about 6.5 px a character; 10 px of padding each side.
    let width = |text: &str| 20 + (13 * text.chars().count()).div_ceil(2);
    let (left, right) = (width(label), width(&message));
    let total = left + right;
    let mut out = String::new();
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total}\" height=\"20\" role=\"img\" \
         aria-label=\"{label}: {message}\">\n\
         <title>{label}: {message}</title>\n\
         <rect width=\"{left}\" height=\"20\" fill=\"#555\"/>\n\
         <rect x=\"{left}\" width=\"{right}\" height=\"20\" fill=\"{colour}\"/>\n\
         <g fill=\"#fff\" font-family=\"Verdana,DejaVu Sans,sans-serif\" font-size=\"11\" \
         text-anchor=\"middle\">\n\
         <text x=\"{}\" y=\"14\">{label}</text>\n\
         <text x=\"{}\" y=\"14\">{message}</text>\n\
         </g>\n\
         </svg>\n",
        left / 2,
        left + right / 2
    );
    out
}

#[cfg(test)]
mod tests;
