//! The accuracy census: the numbers the committed reports hold hpr to, counted once, classed by
//! what they were compared with and how fast the flight went, and held to the census last
//! accepted.
//!
//! The reports say how each case came out. The census says what they add up to, and it is the
//! gate against a regression that a regenerated report would otherwise carry in unnoticed. Four
//! reports feed it:
//!
//! - the harness's (`validation/reports/latest.json`): every metric of every case, and each known
//!   gap;
//! - the real flights' (`real-flights.json`): each flight's apogee and the RMS of its climb;
//! - OpenRocket's flights of its examples and of the private designs (`openrocket-flights.json`,
//!   `openrocket-library-flights.json`): apogee, largest speed, stability margin, mass at launch
//!   and at rod clearance, and centre of mass there, and each configuration hpr doesn't fly.
//!
//! Each becomes a [`Row`], keyed by its report's [`Group`], its case and its metric, and a key seen
//! twice is refused, so a case counts once ([Loft lesson L84][l84]).
//!
//! The census accepted last is committed (`validation/reports/census.json`). A run is held to it
//! ([`compare`]): a row that moves by more than its slack in either direction, that changes its
//! standing (a miss that starts passing too, [L85][l85]), or that comes or goes, a group whose
//! reference changes, or a changed slack, is a [`Change`], and any change fails the check until the
//! census is accepted again with a written reason. A predicted-mode row, whose 3% is only a target
//! ([ADR-023][adr-023], predicted mode), is held the same way, so hpr's own aerodynamics can't
//! drift unnoticed inside its target ([L88][l88]). Each group's headline names what it was
//! compared with, the kind of comparison, how many flights and their speeds ([L86][l86]).
//!
//! A row's slack is [`SLACK_SHARE`] of its scale: the tolerance it is held to, or for a row held
//! to none, the bar its kind is judged by. It is never less than the harness's own reproduction
//! bound, [`reproduction_bound`].
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
/// On a 3% tolerance that is 3e-5 of the reference. The committed reports are regenerated on one
/// machine and compared as files, so the noise that matters is a regeneration's: within
/// [`reproduction_bound`] for the harness, and at most 5e-7 per cent on an OpenRocket corpus rerun.
pub const SLACK_SHARE: f64 = 1e-3;

/// How far a harness value may move between runs and platforms and still reproduce: `2e-6` or
/// `1e-7` of the value, whichever is larger (`Report::reproduces`). A harness row's slack is never
/// less than this.
#[must_use]
pub fn reproduction_bound(value: f64) -> f64 {
    2e-6_f64.max(1e-7 * value.abs())
}

/// The scale of a harness row held to no tolerance: 3% of its reference, the bound
/// [M2.1][m2-1] set on each metric and that no committed gate or target exceeds
/// (`tests::no_committed_gate_is_looser_than_the_milestone_says`).
///
/// [m2-1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-1
pub const NOT_SCORED_SCALE: f64 = 0.03;

/// The bar an OpenRocket apogee or largest speed is judged by, per cent: the apogee difference
/// that needs a written cause ([ADR-070][adr-070], the corpus's rules), and the bar the staging
/// milestone set on both ([M1.9c][m1-9c]).
///
/// [m1-9c]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-9c
/// [adr-070]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-070-m22e-split-mass-and-centre-of-mass-first-then-the-corpus-2026-09-25
pub const OPENROCKET_BAR_PERCENT: f64 = 5.0;

/// The bar an OpenRocket mass difference is judged by, per cent: the 1% the mass comparison was
/// measured against when it began ([M2.2a][m2-2a]).
///
/// [m2-2a]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-2a
pub const MASS_BAR_PERCENT: f64 = 1.0;

/// The bar a stability margin or a centre of mass is judged by, calibres: the centre-of-pressure
/// target set before the transonic normal force was measured ([M1.8a][m1-8a]). A difference in
/// either moves the margin by as many calibres.
///
/// [m1-8a]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8a
pub const MARGIN_BAR_CAL: f64 = 0.5;

/// The bar a real flight's climb is judged by, per cent of its apogee: the bound the harness holds
/// a whole flight's height series to ([ADR-024][adr-024], the series RMS).
///
/// [adr-024]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-024-the-time-series-rms-aligned-at-ignition-held-to-3-of-its-traces-scale-2026-09-18
pub const TRACE_BAR_PERCENT: f64 = 3.0;

/// The largest Mach number of a subsonic flight, as the OpenRocket library report classes them.
pub const SUBSONIC_BELOW: f64 = 0.8;
/// The largest Mach number of a transonic flight; past it a flight is supersonic.
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
            Self::FlightLogs => "measured",
        }
    }

    /// What its numbers are held to in their own report. A gate fails the run, a target is
    /// reported and not enforced, and where there is neither, a bar marks a difference that needs
    /// a written cause. The census holds every number to where it was, whatever this says.
    #[must_use]
    pub const fn bar(self) -> &'static str {
        match self {
            Self::Descent | Self::SameDrag => "gate: each metric's tolerance, at most 3%",
            Self::Predicted => "target: 3% on each metric, reported, not enforced",
            Self::OpenRocketExamples | Self::OpenRocketLibrary => {
                "no target; an apogee more than 5% off needs a written cause"
            }
            Self::FlightLogs => "target: mean absolute apogee error 5%",
        }
    }

    /// The noun for its cases.
    const fn noun(self, count: usize) -> &'static str {
        match (self, count) {
            (Self::Descent, 1) => "descent",
            (Self::Descent, _) => "descents",
            (_, 1) => "flight",
            _ => "flights",
        }
    }

    const fn report(self) -> &'static str {
        match self {
            Self::Descent | Self::SameDrag | Self::Predicted => HARNESS,
            Self::FlightLogs => REAL,
            Self::OpenRocketExamples => EXAMPLES,
            Self::OpenRocketLibrary => LIBRARY,
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
    /// Not known: a flight hpr doesn't fly, or one its report gives no Mach number for.
    Unknown,
}

impl Regime {
    /// The class of a flight whose largest Mach number is `mach`; [`Regime::Unknown`] for a
    /// number that isn't finite.
    #[must_use]
    pub fn of_mach(mach: f64) -> Self {
        if !mach.is_finite() {
            Self::Unknown
        } else if mach < SUBSONIC_BELOW {
            Self::Subsonic
        } else if mach <= SUPERSONIC_ABOVE {
            Self::Transonic
        } else {
            Self::Supersonic
        }
    }

    /// Its name, as the census writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Descent => "under a parachute",
            Self::Subsonic => "subsonic",
            Self::Transonic => "transonic",
            Self::Supersonic => "supersonic",
            Self::Unknown => "of unknown speed",
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
    /// Inside the bar its kind is judged by, where there is no gate or target.
    WithinBar,
    /// Over that bar.
    OverBar,
    /// Not compared: the report withholds it, and says why.
    Withheld,
    /// A known gap: a harness flight hpr refuses, compared with nothing.
    Gap,
    /// A configuration an OpenRocket report lists and hpr doesn't fly, with the report's reason.
    NotFlown,
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
            Self::NotFlown => "not flown",
        }
    }

    /// Whether it stands for a case hpr doesn't fly, rather than a compared number.
    const fn unflown(self) -> bool {
        matches!(self, Self::Gap | Self::NotFlown)
    }

    /// How good it is, higher better, for telling a fall from a rise.
    const fn rank(self) -> u8 {
        match self {
            Self::Fail => 0,
            Self::Gap | Self::NotFlown => 1,
            Self::Withheld | Self::NotScored => 2,
            Self::OutsideTarget | Self::OverBar => 3,
            Self::WithinTarget | Self::WithinBar => 4,
            Self::Pass => 5,
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

/// One compared number, or one case that isn't flown.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// The report it comes from.
    pub group: Group,
    /// The case: a harness case id, an OpenRocket design and configuration, an anonymised id, or
    /// a real flight's id.
    pub case: String,
    /// The metric; empty for a case that isn't flown.
    pub metric: String,
    /// hpr less the reference, in [`Row::unit`]; for a climb, the RMS itself.
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

    /// Its difference as a share of its scale; not a number for a row with no scale.
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

    /// A row that isn't a compared number: a case that isn't flown.
    fn unflown(group: Group, case: String, standing: Standing, regime: Regime) -> Self {
        Self {
            group,
            case,
            metric: String::new(),
            difference: 0.0,
            percent: None,
            unit: Unit::Metric,
            scale: 0.0,
            slack: 0.0,
            standing,
            regime,
        }
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
                    report: row.group.report(),
                    case: row.case.clone(),
                    metric: row.metric.clone(),
                });
            }
        }
        let unflown: BTreeSet<(Group, &str)> = rows
            .iter()
            .filter(|row| row.standing.unflown())
            .map(|row| (row.group, row.case.as_str()))
            .collect();
        if let Some(row) = rows.iter().find(|row| {
            !row.standing.unflown() && unflown.contains(&(row.group, row.case.as_str()))
        }) {
            return Err(CensusError::Report {
                report: row.group.report(),
                what: format!("{} is both compared and not compared", row.case),
            });
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
        rows.push(Row {
            group: harness_group(&comparison.case)?,
            case: comparison.case.clone(),
            metric: comparison.metric.clone(),
            difference: comparison.difference,
            percent: comparison.relative.map(|relative| 100.0 * relative),
            unit: Unit::Metric,
            scale,
            slack: (SLACK_SHARE * scale).max(reproduction_bound(comparison.reference)),
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
        rows.push(Row::unflown(
            harness_group(&gap.case)?,
            gap.case.clone(),
            Standing::Gap,
            Regime::of_mach(gap.mach),
        ));
    }
    Ok(())
}

/// A row judged by a bar: within it or over it, by the difference's magnitude; withheld where the
/// report gives no difference.
fn barred(
    group: Group,
    case: &str,
    metric: &str,
    difference: Option<f64>,
    (unit, scale): (Unit, f64),
    regime: Regime,
) -> Row {
    let (difference, standing) = match difference.filter(|value| value.is_finite()) {
        Some(value) if value.abs() <= scale => (value, Standing::WithinBar),
        Some(value) => (value, Standing::OverBar),
        None => (0.0, Standing::Withheld),
    };
    Row {
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
    }
}

fn real_flight_rows(
    report: &RealFlightReport,
    rows: &mut Vec<Row>,
    references: &mut BTreeMap<Group, String>,
) {
    references.insert(Group::FlightLogs, "the teams' altimeter logs".to_owned());
    for flight in &report.flights {
        let regime = Regime::of_mach(flight.hpr_max_mach);
        // The 5% target is on the mean; one flight's apogee is judged by it as a bar.
        rows.push(barred(
            Group::FlightLogs,
            &flight.id,
            "apogee",
            Some(flight.apogee_error_percent),
            (Unit::Percent, APOGEE_TARGET_PERCENT),
            regime,
        ));
        rows.push(barred(
            Group::FlightLogs,
            &flight.id,
            "climb",
            Some(flight.trace_rms_percent),
            (Unit::Percent, TRACE_BAR_PERCENT),
            regime,
        ));
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

fn array<'a>(
    report: &'a Value,
    key: &str,
    name: &'static str,
) -> Result<&'a Vec<Value>, CensusError> {
    report[key].as_array().ok_or_else(|| CensusError::Report {
        report: name,
        what: format!("no `{key}` list"),
    })
}

/// A difference an OpenRocket report scored, or `None` where it withheld it.
fn scored(
    outcome: Option<&str>,
    value: Option<f64>,
    case: &str,
    metric: &str,
    name: &'static str,
) -> Result<Option<f64>, CensusError> {
    match (outcome, value) {
        (Some("scored"), Some(value)) if value.is_finite() => Ok(Some(value)),
        (Some("scored"), _) => Err(CensusError::Report {
            report: name,
            what: format!("{case} {metric} is scored with no finite difference"),
        }),
        (Some(_), _) => Ok(None),
        (None, _) => Err(CensusError::Report {
            report: name,
            what: format!("{case} {metric} has no outcome"),
        }),
    }
}

/// A mass or centre-of-mass difference: `None` only for an aborted flight, which the reports
/// withhold them from. An absent or null value on a flight that ran is a partial report.
fn measured(
    value: Option<f64>,
    flight: &Value,
    case: &str,
    metric: &str,
    name: &'static str,
) -> Result<Option<f64>, CensusError> {
    let aborted = flight["aborted"]
        .as_bool()
        .ok_or_else(|| CensusError::Report {
            report: name,
            what: format!("{case} doesn't say whether it was aborted"),
        })?;
    match value.filter(|value| value.is_finite()) {
        Some(value) => Ok(Some(value)),
        None if aborted => Ok(None),
        None => Err(CensusError::Report {
            report: name,
            what: format!("{case} {metric} is missing from a flight that ran"),
        }),
    }
}

/// Per cent of `openrocket`'s by which `hpr` differs, as `cargo xtask ork-flights` computes it.
fn percent_of(hpr: &Value, openrocket: &Value) -> Option<f64> {
    hpr.as_f64()
        .zip(openrocket.as_f64())
        .map(|(h, o)| 100.0 * (h - o) / o)
        .filter(|p| p.is_finite())
}

const APOGEE_BAR: (Unit, f64) = (Unit::Percent, OPENROCKET_BAR_PERCENT);
const MASS_BAR: (Unit, f64) = (Unit::Percent, MASS_BAR_PERCENT);
const CALIBRE_BAR: (Unit, f64) = (Unit::Calibre, MARGIN_BAR_CAL);

fn openrocket_example_rows(
    report: &Value,
    rows: &mut Vec<Row>,
    references: &mut BTreeMap<Group, String>,
) -> Result<(), CensusError> {
    let group = Group::OpenRocketExamples;
    references.insert(group, openrocket_reference(report, EXAMPLES)?);
    let case_of = |flight: &Value| -> Result<String, CensusError> {
        match (flight["design"].as_str(), flight["configuration"].as_str()) {
            (Some(design), Some(configuration)) => Ok(format!("{design} / {configuration}")),
            _ => Err(CensusError::Report {
                report: EXAMPLES,
                what: "a flight has no design or configuration".to_owned(),
            }),
        }
    };
    for flight in array(report, "flights", EXAMPLES)? {
        let case = case_of(flight)?;
        let regime = Regime::of_mach(flight["max_mach_openrocket"].as_f64().unwrap_or(f64::NAN));
        let metrics = &flight["metrics"];
        for key in metrics.as_object().into_iter().flat_map(|map| map.keys()) {
            if !matches!(
                key.as_str(),
                "apogee_m" | "max_speed_m_s" | "rod_clearance_margin_cal"
            ) {
                return Err(CensusError::Report {
                    report: EXAMPLES,
                    what: format!("{case}: the census has no bar for the metric {key}"),
                });
            }
        }
        for (key, metric, field, bar) in [
            ("apogee_m", "apogee", "relative_percent", APOGEE_BAR),
            ("max_speed_m_s", "max_speed", "relative_percent", APOGEE_BAR),
            (
                "rod_clearance_margin_cal",
                "margin",
                "difference",
                CALIBRE_BAR,
            ),
        ] {
            let entry = &metrics[key];
            let value = scored(
                entry["outcome"]["outcome"].as_str(),
                entry[field].as_f64(),
                &case,
                metric,
                EXAMPLES,
            )?;
            rows.push(barred(group, &case, metric, value, bar, regime));
        }
        let at = &flight["at_rod_clearance"];
        let (hpr, openrocket) = (&at["hpr"], &at["openrocket"]);
        let cg = hpr["cg_from_nose_m"]
            .as_f64()
            .zip(openrocket["cg_from_nose_m"].as_f64())
            .zip(openrocket["reference_length_m"].as_f64())
            .map(|((h, o), reference)| (h - o) / reference)
            .filter(|cal| cal.is_finite());
        let launch = &flight["launch_mass_kg"];
        for (metric, value, bar) in [
            (
                "launch_mass",
                percent_of(&launch["hpr"], &launch["openrocket"]),
                MASS_BAR,
            ),
            (
                "rod_clearance_mass",
                percent_of(&hpr["mass_kg"], &openrocket["mass_kg"]),
                MASS_BAR,
            ),
            ("rod_clearance_cg", cg, CALIBRE_BAR),
        ] {
            let value = measured(value, flight, &case, metric, EXAMPLES)?;
            rows.push(barred(group, &case, metric, value, bar, regime));
        }
    }
    for flight in array(report, "not_flown", EXAMPLES)? {
        rows.push(Row::unflown(
            group,
            case_of(flight)?,
            Standing::NotFlown,
            Regime::Unknown,
        ));
    }
    Ok(())
}

fn openrocket_library_rows(
    report: &Value,
    rows: &mut Vec<Row>,
    references: &mut BTreeMap<Group, String>,
) -> Result<(), CensusError> {
    let group = Group::OpenRocketLibrary;
    references.insert(group, openrocket_reference(report, LIBRARY)?);
    let case_of = |flight: &Value| -> Result<String, CensusError> {
        flight["flight"]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| CensusError::Report {
                report: LIBRARY,
                what: "a flight has no id".to_owned(),
            })
    };
    for flight in array(report, "flights", LIBRARY)? {
        let case = case_of(flight)?;
        let regime = match flight["mach"].as_str() {
            Some("subsonic") => Regime::Subsonic,
            Some("transonic") => Regime::Transonic,
            Some("supersonic") => Regime::Supersonic,
            Some("unknown") => Regime::Unknown,
            other => {
                return Err(CensusError::Report {
                    report: LIBRARY,
                    what: format!("{case}: no Mach class the census knows ({other:?})"),
                });
            }
        };
        for (metric, field, outcome, bar) in [
            ("apogee", "apogee_percent", "apogee_outcome", APOGEE_BAR),
            (
                "max_speed",
                "max_speed_percent",
                "max_speed_outcome",
                APOGEE_BAR,
            ),
            ("margin", "margin_cal", "margin_outcome", CALIBRE_BAR),
        ] {
            let value = scored(
                flight[outcome].as_str(),
                flight[field].as_f64(),
                &case,
                metric,
                LIBRARY,
            )?;
            rows.push(barred(group, &case, metric, value, bar, regime));
        }
        for (metric, field, bar) in [
            ("launch_mass", "launch_mass_percent", MASS_BAR),
            ("rod_clearance_mass", "rod_clearance_mass_percent", MASS_BAR),
            ("rod_clearance_cg", "rod_clearance_cg_cal", CALIBRE_BAR),
        ] {
            let value = measured(flight[field].as_f64(), flight, &case, metric, LIBRARY)?;
            rows.push(barred(group, &case, metric, value, bar, regime));
        }
    }
    for flight in array(report, "not_flown", LIBRARY)? {
        rows.push(Row::unflown(
            group,
            case_of(flight)?,
            Standing::NotFlown,
            Regime::Unknown,
        ));
    }
    Ok(())
}

/// A spread of per-cent differences: the least, the largest and the mean magnitude.
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
    /// How many are within the scale they are judged by.
    pub within: usize,
}

impl Spread {
    fn of(rows: &[&Row]) -> Option<Self> {
        let values: Vec<(f64, bool)> = rows
            .iter()
            .filter_map(|row| {
                row.percent
                    .map(|percent| (percent, row.difference.abs() <= row.scale))
            })
            .collect();
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
            min: values.iter().map(|v| v.0).fold(f64::INFINITY, f64::min),
            max: values.iter().map(|v| v.0).fold(f64::NEG_INFINITY, f64::max),
            mean_absolute: values.iter().map(|v| v.0.abs()).sum::<f64>() / count,
            within: values.iter().filter(|v| v.1).count(),
        })
    }
}

/// How many of one metric's numbers are within the bar they are judged by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BarCount {
    /// The metric.
    pub metric: String,
    /// Its bar.
    pub scale: f64,
    /// The bar's unit.
    pub unit: Unit,
    /// How many are within it.
    pub within: usize,
    /// How many were compared.
    pub compared: usize,
}

/// The metrics judged by a bar, in the order the census reads them out, with their names and
/// what their bar is a share of, where that isn't the reference's own value.
const BARRED: [(&str, &str, &str); 7] = [
    ("apogee", "apogee", ""),
    ("max_speed", "largest speed", ""),
    ("margin", "margin", ""),
    ("launch_mass", "launch mass", ""),
    ("rod_clearance_mass", "mass at rod clearance", ""),
    ("rod_clearance_cg", "centre of mass at rod clearance", ""),
    ("climb", "climb's RMS height error", " of apogee"),
];

/// What one group adds up to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// The group.
    pub group: Group,
    /// What it was compared with.
    pub reference: String,
    /// Its cases compared, or refused as known gaps: flights, or descents.
    pub cases: usize,
    /// Its cases the report lists as not flown.
    pub not_flown: usize,
    /// Its cases by speed, not-flown ones left out.
    pub regimes: BTreeMap<Regime, usize>,
    /// Its rows.
    pub rows: usize,
    /// Its rows by standing.
    pub standings: BTreeMap<Standing, usize>,
    /// Each metric judged by a bar, in the census's order.
    pub bars: Vec<BarCount>,
    /// The apogee differences, per cent: the harness's relative to RocketPy's, OpenRocket's and the
    /// logs' as their reports give them. `None` for descents.
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
        let mut not_flown = BTreeSet::new();
        let mut standings = BTreeMap::new();
        for row in &rows {
            if row.standing == Standing::NotFlown {
                not_flown.insert(row.case.as_str());
            } else {
                cases.insert(&row.case, row.regime);
            }
            *standings.entry(row.standing).or_insert(0) += 1;
        }
        let mut regimes = BTreeMap::new();
        for regime in cases.values() {
            *regimes.entry(*regime).or_insert(0) += 1;
        }
        let bars = BARRED
            .iter()
            .filter_map(|(metric, ..)| {
                let judged: Vec<&Row> = rows
                    .iter()
                    .copied()
                    .filter(|row| {
                        row.metric == *metric
                            && matches!(row.standing, Standing::WithinBar | Standing::OverBar)
                    })
                    .collect();
                let first = judged.first()?;
                Some(BarCount {
                    metric: (*metric).to_owned(),
                    scale: first.scale,
                    unit: first.unit,
                    within: judged
                        .iter()
                        .filter(|row| row.standing == Standing::WithinBar)
                        .count(),
                    compared: judged.len(),
                })
            })
            .collect();
        let apogees: Vec<&Row> = rows
            .iter()
            .copied()
            .filter(|row| matches!(row.metric.as_str(), "apogee" | "apogee_agl_m"))
            .collect();
        Some(Self {
            group,
            reference: census.references.get(&group).cloned().unwrap_or_default(),
            cases: cases.len(),
            not_flown: not_flown.len(),
            regimes,
            rows: rows.len(),
            standings,
            bars,
            apogee_percent: Spread::of(&apogees),
        })
    }

    fn count(&self, standing: Standing) -> usize {
        self.standings.get(&standing).copied().unwrap_or(0)
    }

    /// Its cases by speed, in words: "7 subsonic, 1 transonic". Descents are left out: their noun
    /// already says it.
    #[must_use]
    pub fn regimes_in_words(&self) -> String {
        self.regimes
            .iter()
            .filter(|(regime, _)| **regime != Regime::Descent)
            .map(|(regime, count)| format!("{count} {}", regime.name()))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Its population in words: "33 flights (32 subsonic, 1 transonic); 24 more not flown".
    #[must_use]
    pub fn population(&self) -> String {
        let mut out = format!("{} {}", self.cases, self.group.noun(self.cases));
        let regimes = self.regimes_in_words();
        if !regimes.is_empty() {
            let _ = write!(out, " ({regimes})");
        }
        if self.not_flown > 0 {
            let _ = write!(out, "; {} more not flown", self.not_flown);
        }
        out
    }

    /// Its result in words: the counts against its gate, target or bars, and its apogees.
    #[must_use]
    pub fn result(&self) -> String {
        let mut parts = Vec::new();
        let gated = self.count(Standing::Pass) + self.count(Standing::Fail);
        let targeted = self.count(Standing::WithinTarget) + self.count(Standing::OutsideTarget);
        if gated > 0 {
            parts.push(format!(
                "{} of {gated} gated metrics pass",
                self.count(Standing::Pass)
            ));
        }
        if targeted > 0 {
            parts.push(format!(
                "{} of {targeted} metrics within target",
                self.count(Standing::WithinTarget)
            ));
        }
        if let (Group::FlightLogs, Some(apogee)) = (self.group, self.apogee_percent) {
            parts.push(format!(
                "mean absolute apogee error {:.2}% (target {APOGEE_TARGET_PERCENT}%, {}); apogee \
                 {:+.2}% to {:+.2}%",
                apogee.mean_absolute,
                if apogee.mean_absolute <= APOGEE_TARGET_PERCENT {
                    "met"
                } else {
                    "missed"
                },
                apogee.min,
                apogee.max,
            ));
        } else if let Some(apogee) = self.apogee_percent {
            parts.push(format!("apogee {:+.2}% to {:+.2}%", apogee.min, apogee.max));
        }
        let bars: Vec<String> = self
            .bars
            .iter()
            .map(|bar| {
                let (name, of) = BARRED
                    .iter()
                    .find(|(metric, ..)| *metric == bar.metric)
                    .map_or((bar.metric.as_str(), ""), |(_, name, of)| (name, of));
                format!(
                    "{name} within {}{}{of} on {} of {}",
                    bar.scale,
                    match bar.unit {
                        Unit::Calibre => " calibres",
                        Unit::Percent => "%",
                        Unit::Metric => "",
                    },
                    bar.within,
                    bar.compared
                )
            })
            .collect();
        if !bars.is_empty() {
            parts.push(bars.join(", "));
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
        parts.join("; ")
    }

    /// Its headline: what was flown, against what and of which kind, how many and how fast, and
    /// the result ([Loft lesson L86][l86]).
    ///
    /// [l86]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l86
    #[must_use]
    pub fn headline(&self) -> String {
        format!(
            "{} against {} ({}; {}): {}; {}.",
            capitalised(self.group.what()),
            self.reference,
            self.group.kind(),
            self.group.bar(),
            self.population(),
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

/// A reference as the table shows it: RocketPy's name and version, marked when patched, and
/// every other reference as its report names it. The census page gives each in full.
#[must_use]
pub fn short_reference(reference: &str) -> String {
    reference
        .split("; ")
        .map(|one| match one.strip_prefix("rocketpy ") {
            Some(rest) => {
                let (version, more) = rest.split_once(' ').unwrap_or((rest, ""));
                if more.is_empty() {
                    format!("RocketPy {version}")
                } else {
                    format!("RocketPy {version}, patched")
                }
            }
            None => one.to_owned(),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// How the committed reports differ from the census accepted last.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Change {
    /// A row the accepted census doesn't have.
    Added(Row),
    /// A row the accepted census has and this one doesn't.
    Removed(Row),
    /// A known gap, or a configuration not flown, that hpr flies now.
    Flown {
        /// The case's row as accepted.
        accepted: Row,
        /// Its rows now.
        now: Vec<Row>,
    },
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
    /// A row whose unit, scale, slack or speed class changed, or whose percentage no longer
    /// follows its difference (its reference moved): it is not the same comparison.
    Redefined {
        /// As accepted.
        accepted: Row,
        /// Now.
        now: Row,
    },
    /// A group compared with another reference, or another version of it.
    Reference {
        /// The group.
        group: Group,
        /// As accepted; empty if it had none.
        accepted: String,
        /// Now; empty if it has none.
        now: String,
    },
    /// The census's slack rule changed.
    Rule {
        /// [`Census::slack_share`] as accepted.
        accepted: f64,
        /// Now.
        now: f64,
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
                accepted.slack,
                unit = now.unit.suffix()
            )
        };
        match self {
            Self::Added(row) => format!("added: {} ({})", row.describe(), row.standing.name()),
            Self::Removed(row) => format!("removed: {} ({})", row.describe(), row.standing.name()),
            Self::Flown { accepted, now } => format!(
                "flown: {}, {} before: hpr flies it now (L85); {}",
                accepted.describe(),
                accepted.standing.name(),
                now.iter()
                    .map(|row| format!(
                        "{} {:+.6}{} ({})",
                        row.metric,
                        row.difference,
                        row.unit.suffix(),
                        row.standing.name()
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Standing { accepted, now } => format!(
                "{}: {} to {}; {}",
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
                "redefined: {}: unit, scale, slack, speed class or percentage changed ({:?}, {}, \
                 {:.2e}, {}, {:?} to {:?}, {}, {:.2e}, {}, {:?})",
                now.describe(),
                accepted.unit,
                accepted.scale,
                accepted.slack,
                accepted.regime.name(),
                accepted.percent,
                now.unit,
                now.scale,
                now.slack,
                now.regime.name(),
                now.percent
            ),
            Self::Reference {
                group,
                accepted,
                now,
            } => format!(
                "reference: {} compared with `{accepted}` before and `{now}` now",
                group.what()
            ),
            Self::Rule { accepted, now } => {
                format!("rule: the slack's share of a row's scale was {accepted} and is {now}")
            }
        }
    }

    /// Whether it makes the record worse: a regression, a row lost, a standing that fell, a
    /// wider scale or slack, a changed unit, or a case flown now that fails or misses a bar.
    #[must_use]
    pub fn is_worse(&self) -> bool {
        match self {
            Self::Regressed { .. } | Self::Removed(_) => true,
            Self::Standing { accepted, now } => now.standing.rank() < accepted.standing.rank(),
            Self::Rule { accepted, now } => now > accepted,
            Self::Redefined { accepted, now } => {
                now.unit != accepted.unit
                    || now.scale > accepted.scale
                    || now.slack > accepted.slack
            }
            Self::Flown { now, .. } => now.iter().any(|row| {
                matches!(
                    row.standing,
                    Standing::Fail | Standing::OverBar | Standing::OutsideTarget
                )
            }),
            Self::Added(_) | Self::Improved { .. } | Self::Reference { .. } => false,
        }
    }
}

/// Every way `now` differs from `accepted`. Empty when the census holds.
///
/// A row is a change when its unit, scale, slack, speed class or percentage's reference differs
/// ([`Change::Redefined`], listed as well as what follows), when its standing differs, or when its
/// difference moved by more than the accepted slack either way: a ratchet that only watched for
/// regressions would let an improvement slip back unseen. A case that was a known gap or not
/// flown, and is compared now, is one [`Change::Flown`] carrying its new rows.
#[must_use]
pub fn compare(accepted: &Census, now: &Census) -> Vec<Change> {
    let mut changes = Vec::new();
    if accepted.slack_share.to_bits() != now.slack_share.to_bits() {
        changes.push(Change::Rule {
            accepted: accepted.slack_share,
            now: now.slack_share,
        });
    }
    let groups: BTreeSet<&Group> = accepted
        .references
        .keys()
        .chain(now.references.keys())
        .collect();
    for group in groups {
        let (was, is) = (
            accepted.references.get(group).cloned().unwrap_or_default(),
            now.references.get(group).cloned().unwrap_or_default(),
        );
        if was != is {
            changes.push(Change::Reference {
                group: *group,
                accepted: was,
                now: is,
            });
        }
    }
    let before: BTreeMap<_, &Row> = accepted.rows.iter().map(|row| (row.key(), row)).collect();
    let after: BTreeMap<_, &Row> = now.rows.iter().map(|row| (row.key(), row)).collect();
    // The cases compared now, and those among them that were known gaps or not flown before:
    // their new rows come with them.
    let compared_now: BTreeSet<(Group, &str)> = now
        .rows
        .iter()
        .filter(|row| !row.standing.unflown())
        .map(|row| (row.group, row.case.as_str()))
        .collect();
    let newly_flown: BTreeSet<(Group, &str)> = accepted
        .rows
        .iter()
        .filter(|row| row.standing.unflown() && !after.contains_key(&row.key()))
        .map(|row| (row.group, row.case.as_str()))
        .filter(|case| compared_now.contains(case))
        .collect();
    let keys: BTreeSet<_> = before.keys().chain(after.keys()).copied().collect();
    let differs = |a: f64, b: f64| (a - b).abs() > 1e-9 * a.abs().max(b.abs());
    for key in keys {
        let flown = newly_flown.contains(&(key.0, key.1));
        match (before.get(&key), after.get(&key)) {
            (Some(was), None) if flown => changes.push(Change::Flown {
                accepted: (*was).clone(),
                now: now
                    .rows
                    .iter()
                    .filter(|row| row.group == key.0 && row.case == key.1)
                    .cloned()
                    .collect(),
            }),
            (Some(was), None) => changes.push(Change::Removed((*was).clone())),
            (None, Some(_)) if flown => {}
            (None, Some(is)) => changes.push(Change::Added((*is).clone())),
            (Some(was), Some(is)) => {
                let (was, is) = ((*was).clone(), (*is).clone());
                if was.unit != is.unit
                    || was.regime != is.regime
                    || differs(was.scale, is.scale)
                    || differs(was.slack, is.slack)
                    || !percent_follows(&was, &is)
                {
                    changes.push(Change::Redefined {
                        accepted: was.clone(),
                        now: is.clone(),
                    });
                }
                if was.standing != is.standing {
                    changes.push(Change::Standing {
                        accepted: was,
                        now: is,
                    });
                } else if (is.difference - was.difference).abs() > was.slack {
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

/// Whether `now`'s percentage is still its difference over the reference `accepted`'s was: a
/// percentage that moved on its own means the reference moved, which the difference can't show.
/// Held to the accepted slack, carried into per cent.
fn percent_follows(accepted: &Row, now: &Row) -> bool {
    match (accepted.percent, now.percent) {
        (None, None) => true,
        // A zero difference has a zero percentage, and says nothing of its reference.
        (Some(was), Some(_)) if accepted.difference == 0.0 => was == 0.0,
        (Some(was), Some(is)) => {
            // Per cent per unit of difference: 100 over the reference, as accepted.
            let per_unit = was / accepted.difference;
            let expected = now.difference * per_unit;
            (is - expected).abs()
                <= accepted.slack * per_unit.abs() + 1e-9 * is.abs().max(expected.abs())
        }
        // A difference withheld, or no longer withheld, is a change of standing, not of definition.
        _ => accepted.standing == Standing::Withheld || now.standing == Standing::Withheld,
    }
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
    /// What each group added up to: [`Census::summaries`], written out for a reader, and
    /// never read back: [`Accepted::refreshed`] computes them again.
    #[serde(default, skip_deserializing)]
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

    /// The same, with its summaries computed again from its rows.
    #[must_use]
    pub fn refreshed(&self) -> Self {
        Self {
            summaries: self.census.summaries(),
            ..self.clone()
        }
    }

    /// The census page, `validation/reports/census.md`.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let summaries = self.census.summaries();
        let mut out = String::new();
        let _ = writeln!(out, "# Accuracy census\n");
        let _ = writeln!(
            out,
            "Generated by `{}` from the committed reports; do not edit. `cargo xtask validate \
             --check` holds those reports to this census, row by row: a row that moves by more \
             than its slack, changes its standing, or comes or goes, a reference that changes, or \
             a changed slack rule fails it until the census is accepted again with a reason. It \
             holds {} rows.\n",
            self.generated_by,
            self.census.rows.len()
        );
        let _ = writeln!(out, "## Headlines\n");
        for summary in &summaries {
            let _ = writeln!(out, "- {}", summary.headline());
        }
        let _ = writeln!(out);
        let _ = writeln!(out, "{}", table(&summaries, None));
        let _ = writeln!(out, "{}", reading(self.census.slack_share));
        let _ = writeln!(out, "## Rows by standing\n");
        let _ = writeln!(out, "| group | metric | standing | rows |");
        let _ = writeln!(out, "|---|---|---|---:|");
        let mut counts: BTreeMap<(Group, &str, Standing), usize> = BTreeMap::new();
        for row in &self.census.rows {
            let metric = if row.metric.is_empty() {
                "(the case)"
            } else {
                row.metric.as_str()
            };
            *counts.entry((row.group, metric, row.standing)).or_insert(0) += 1;
        }
        for ((group, metric, standing), count) in counts {
            let _ = writeln!(
                out,
                "| {} | {metric} | {} | {count} |",
                group.what(),
                standing.name()
            );
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
fn reading(slack_share: f64) -> String {
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
        "A row's slack is {}% of its scale, and for a harness row never less than the harness's \
         reproduction bound, 2e-6 or 1e-7 of the value, whichever is larger. The scale is the \
         row's own tolerance where it has one, and otherwise the bar of its kind:\n",
        100.0 * slack_share
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
            "OpenRocket mass, at launch or at rod clearance",
            format!("{MASS_BAR_PERCENT}%"),
        ),
        (
            "OpenRocket stability margin, or centre of mass",
            format!("{MARGIN_BAR_CAL} calibres"),
        ),
        ("logged apogee", format!("{APOGEE_TARGET_PERCENT}%")),
        (
            "logged climb (the RMS of the climb's heights, as a share of the apogee)",
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
        slack_share * allowed_m
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
        let reference = short_reference(&summary.reference);
        let _ = writeln!(
            out,
            "| {}: {} | {} | {} | {} | {} |",
            link.map_or_else(
                || reference.clone(),
                |link| format!("[{reference}]({link})")
            ),
            summary.group.what(),
            summary.group.kind(),
            summary.group.bar(),
            summary.population(),
            summary.result()
        );
    }
    out
}

/// One badge: a flat SVG, grey label on the left, coloured message on the right.
fn badge(label: &str, message: &str, colour: &str) -> String {
    // Verdana at 11 px averages about 6.5 px a character; 10 px of padding each side.
    let width = |text: &str| 20 + (13 * text.chars().count()).div_ceil(2);
    let (left, right) = (width(label), width(message));
    let total = left + right;
    format!(
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
    )
}

/// The badges, by file name: the gated code-to-code metrics that pass, of all of them, and the
/// real flights' mean apogee error against its target. Two, so the one measurement stands beside
/// the agreement with another program.
#[must_use]
pub fn badges(summaries: &[Summary]) -> Vec<(&'static str, String)> {
    let (pass, gated) = summaries
        .iter()
        .filter(|summary| matches!(summary.group, Group::Descent | Group::SameDrag))
        .fold((0, 0), |(pass, gated), summary| {
            (
                pass + summary.count(Standing::Pass),
                gated + summary.count(Standing::Pass) + summary.count(Standing::Fail),
            )
        });
    let mut out = vec![(
        "census-badge.svg",
        badge(
            "vs RocketPy, same inputs",
            &format!("{pass} of {gated} gated metrics pass"),
            if pass == gated && gated > 0 {
                "#2e7d32"
            } else {
                "#c62828"
            },
        ),
    )];
    let flights = summaries
        .iter()
        .find(|summary| summary.group == Group::FlightLogs)
        .and_then(|summary| summary.apogee_percent);
    // Always written, so a badge from an earlier census can't linger unchecked.
    out.push((
        "real-flights-badge.svg",
        flights.map_or_else(
            || badge("real flights", "none compared", "#6e6e6e"),
            |apogee| {
                let met = apogee.mean_absolute <= APOGEE_TARGET_PERCENT;
                badge(
                    &format!("{} real flights", apogee.count),
                    &format!(
                        "mean absolute apogee error {:.2}% (target {APOGEE_TARGET_PERCENT}%, {})",
                        apogee.mean_absolute,
                        if met { "met" } else { "missed" }
                    ),
                    if met { "#2e7d32" } else { "#b35c00" },
                )
            },
        ),
    ));
    out
}

#[cfg(test)]
mod tests;
