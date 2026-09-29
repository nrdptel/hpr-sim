//! A run's report in a line per case and one for the whole: what `cargo xtask validate` and
//! `hpr validate` print.
//!
//! The worst figure is over the **scored** metrics: a metric a case declares not scored is
//! counted and named separately, so a run cannot look green by leaving something out and cannot
//! look alarming because a declared difference is large in percentage terms.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::report::{Comparison, Report, Verdict};

/// How one case came out.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaseSummary {
    /// A known gap: hpr refuses the flight, so nothing is compared.
    Gap {
        /// The case's id.
        case: String,
        /// The metrics the case would compare.
        metrics: usize,
        /// Why hpr refuses it.
        refusal: String,
    },
    /// Predicted mode: every metric reported against a target, none gating.
    Predicted {
        /// The case's id.
        case: String,
        /// The metrics reported.
        metrics: usize,
        /// How many are within their target.
        within_target: usize,
        /// The metric furthest from its reference, and its relative difference.
        largest: Option<(String, f64)>,
    },
    /// Every metric held to its tolerance.
    Scored {
        /// The case's id.
        case: String,
        /// The metrics compared.
        metrics: usize,
        /// The largest relative difference among the scored metrics, as a magnitude.
        worst_scored: f64,
        /// The metrics the case declares not scored.
        not_scored: Vec<String>,
        /// How many metrics are outside their tolerance.
        failed: usize,
    },
}

impl CaseSummary {
    /// The case's id.
    pub fn case(&self) -> &str {
        match self {
            Self::Gap { case, .. } | Self::Predicted { case, .. } | Self::Scored { case, .. } => {
                case
            }
        }
    }
}

impl fmt::Display for CaseSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // A gap compares nothing, so "0 metrics, worst +0.00%" would read as a clean pass.
            Self::Gap {
                case,
                metrics,
                refusal,
            } => write!(
                f,
                "{case}: known gap, {metrics} metric(s) not scored: hpr {refusal}"
            ),
            // Predicted mode reports against a target and never gates, so it has no "worst
            // scored"; its largest difference is the headline.
            Self::Predicted {
                case,
                metrics,
                within_target,
                largest,
            } => {
                write!(
                    f,
                    "{case}: predicted, {metrics} metric(s) reported, {within_target} within target"
                )?;
                if let Some((metric, relative)) = largest {
                    write!(f, ", largest {metric} {:+.2}%", 100.0 * relative)?;
                }
                Ok(())
            }
            Self::Scored {
                case,
                metrics,
                worst_scored,
                not_scored,
                failed,
            } => {
                write!(
                    f,
                    "{case}: {metrics} metric(s), worst scored {:+.2}%",
                    100.0 * worst_scored
                )?;
                if !not_scored.is_empty() {
                    write!(f, ", not scored: {}", not_scored.join(", "))?;
                }
                if *failed != 0 {
                    write!(f, ", {failed} OUT OF TOLERANCE")?;
                }
                Ok(())
            }
        }
    }
}

/// The whole run, counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Totals {
    /// The cases run.
    pub cases: usize,
    /// The metrics compared or reported.
    pub metrics: usize,
    /// The metrics their cases declare not scored.
    pub not_scored: usize,
    /// The metrics reported against a target (predicted mode).
    pub predicted: usize,
    /// How many of those are outside their target.
    pub outside_target: usize,
    /// The scored metrics outside their tolerance.
    pub failed: usize,
}

impl Totals {
    /// Whether every scored metric is within its tolerance.
    pub fn passed(&self) -> bool {
        self.failed == 0
    }
}

impl fmt::Display for Totals {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let predicted = format!(
            "predicted, against a target, {} outside it",
            self.outside_target
        );
        let aside: Vec<String> = [
            (self.not_scored, "not scored"),
            (self.predicted, predicted.as_str()),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, what)| format!("{count} {what}"))
        .collect();
        write!(
            f,
            "validate: {} case(s), {} metric(s){}, {}",
            self.cases,
            self.metrics,
            if aside.is_empty() {
                String::new()
            } else {
                format!(" ({})", aside.join(", "))
            },
            if self.passed() { "ok" } else { "FAILED" }
        )
    }
}

/// Each case of `report`, in its order.
pub fn cases(report: &Report) -> Vec<CaseSummary> {
    report
        .cases
        .iter()
        .map(|case| {
            if let Some(gap) = report.gaps.iter().find(|gap| gap.case == *case) {
                return CaseSummary::Gap {
                    case: case.clone(),
                    metrics: gap.metric_count,
                    refusal: gap.refusal.clone(),
                };
            }
            let metrics: Vec<&Comparison> = report
                .comparisons
                .iter()
                .filter(|comparison| comparison.case == *case)
                .collect();
            if metrics.iter().any(|comparison| comparison.targeted_row()) {
                return CaseSummary::Predicted {
                    case: case.clone(),
                    metrics: metrics.len(),
                    within_target: metrics
                        .iter()
                        .filter(|comparison| comparison.verdict == Verdict::WithinTarget)
                        .count(),
                    largest: metrics
                        .iter()
                        .filter_map(|comparison| {
                            comparison.relative.map(|r| (comparison.metric.clone(), r))
                        })
                        .max_by(|(_, a), (_, b)| a.abs().total_cmp(&b.abs())),
                };
            }
            CaseSummary::Scored {
                case: case.clone(),
                metrics: metrics.len(),
                worst_scored: metrics
                    .iter()
                    .filter(|comparison| comparison.scored())
                    .filter_map(|comparison| comparison.relative)
                    .fold(0.0_f64, |worst, relative| worst.max(relative.abs())),
                not_scored: metrics
                    .iter()
                    .filter(|comparison| !comparison.scored())
                    .map(|comparison| comparison.metric.clone())
                    .collect(),
                failed: metrics
                    .iter()
                    .filter(|comparison| comparison.verdict == Verdict::Fail)
                    .count(),
            }
        })
        .collect()
}

/// The whole of `report`, counted.
pub fn totals(report: &Report) -> Totals {
    let count = |verdict| {
        report
            .comparisons
            .iter()
            .filter(|comparison| comparison.verdict == verdict)
            .count()
    };
    Totals {
        cases: report.cases.len(),
        metrics: report.comparisons.len(),
        not_scored: report.not_scored().len(),
        predicted: report
            .comparisons
            .iter()
            .filter(|comparison| comparison.targeted_row())
            .count(),
        outside_target: count(Verdict::OutsideTarget),
        failed: report.failures().len(),
    }
}
