//! Reports: one line per metric, in Markdown for people and JSON for machines.

use serde::{Deserialize, Serialize};

use crate::case::Tolerance;

/// How one metric came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Verdict {
    /// Inside its tolerance.
    Pass,
    /// Outside it.
    Fail,
    /// Measured and printed, but held to no tolerance, for the written reason beside it. It is
    /// not a pass: the summary counts it apart, and it never makes a run green on its own.
    NotScored,
}

/// One metric of one case: what the oracle said, what hpr said, and whether that is close enough.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    /// The case's id.
    pub case: String,
    /// The metric's name.
    pub metric: String,
    /// What hpr measured.
    pub measured: f64,
    /// What the reference says.
    pub reference: f64,
    /// Where the reference value came from.
    pub source: String,
    /// The tolerance it was held to, if it was held to one.
    pub tolerance: Tolerance,
    /// `measured − reference`.
    pub difference: f64,
    /// That difference as a fraction of the reference, where the reference is not zero.
    pub relative: Option<f64>,
    /// Pass, fail, or not scored.
    pub verdict: Verdict,
    /// Why it is not scored, for a [`Verdict::NotScored`] row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Comparison {
    /// Compares `measured` against `reference` under `tolerance`.
    #[must_use]
    pub fn new(
        case: &str,
        metric: &str,
        measured: f64,
        reference: f64,
        source: &str,
        tolerance: Tolerance,
    ) -> Self {
        let verdict = if tolerance.accepts(measured, reference) {
            Verdict::Pass
        } else {
            Verdict::Fail
        };
        Self::build(
            case, metric, measured, reference, source, tolerance, verdict, None,
        )
    }

    /// Records `measured` against `reference` without scoring it, for the written `reason`.
    #[must_use]
    pub fn not_scored(
        case: &str,
        metric: &str,
        measured: f64,
        reference: f64,
        source: &str,
        reason: &str,
    ) -> Self {
        let reason = reason.trim();
        Self::build(
            case,
            metric,
            measured,
            reference,
            source,
            Tolerance::default(),
            if reason.is_empty() {
                // An excuse nobody wrote down is the thing this mechanism exists to prevent, so a
                // blank one fails rather than quietly not counting.
                Verdict::Fail
            } else {
                Verdict::NotScored
            },
            (!reason.is_empty()).then(|| reason.to_owned()),
        )
    }

    /// The common part of both.
    #[expect(
        clippy::too_many_arguments,
        reason = "a report row has this many columns"
    )]
    fn build(
        case: &str,
        metric: &str,
        measured: f64,
        reference: f64,
        source: &str,
        tolerance: Tolerance,
        verdict: Verdict,
        note: Option<String>,
    ) -> Self {
        let difference = measured - reference;
        Self {
            case: case.to_owned(),
            metric: metric.to_owned(),
            measured,
            reference,
            source: source.to_owned(),
            tolerance,
            difference,
            relative: (reference != 0.0).then(|| difference / reference),
            verdict,
            note,
        }
    }

    /// Whether this row counts towards the suite's verdict.
    #[must_use]
    pub fn scored(&self) -> bool {
        self.verdict != Verdict::NotScored
    }
}

/// Where a case's reference came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    /// The case's id.
    pub case: String,
    /// The oracle, as its generator records it.
    pub oracle: String,
    /// The generator script.
    pub generator: String,
    /// The command that regenerates the reference.
    pub command: String,
    /// The reference file, relative to the repository root.
    pub file: String,
    /// Its SHA-256, so an edited reference shows up in the report and not only in git history.
    pub sha256: String,
    /// What the oracle modelled, in its generator's words.
    pub model: String,
    /// What the generator had to override to make the comparison like-for-like.
    pub overrides: String,
}

/// A whole run: every comparison, and what it adds up to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    /// The harness's version.
    pub harness_version: String,
    /// Whether the run was `--fast`.
    pub fast: bool,
    /// The cases it covered, in report order.
    pub cases: Vec<String>,
    /// The locked cases it left out, which only a fast run has.
    #[serde(default)]
    pub skipped: Vec<String>,
    /// Every metric compared.
    pub comparisons: Vec<Comparison>,
    /// Where each case's reference came from, in case order.
    pub sources: Vec<Source>,
}

impl Report {
    /// Whether every scored comparison passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.failures().is_empty()
    }

    /// The comparisons that failed.
    #[must_use]
    pub fn failures(&self) -> Vec<&Comparison> {
        self.comparisons
            .iter()
            .filter(|comparison| comparison.verdict == Verdict::Fail)
            .collect()
    }

    /// The comparisons that were measured but held to no tolerance.
    #[must_use]
    pub fn not_scored(&self) -> Vec<&Comparison> {
        self.comparisons
            .iter()
            .filter(|comparison| comparison.verdict == Verdict::NotScored)
            .collect()
    }

    /// The largest relative difference among the scored comparisons, as a fraction.
    #[must_use]
    pub fn worst_scored(&self) -> Option<(&Comparison, f64)> {
        self.comparisons
            .iter()
            .filter(|comparison| comparison.scored())
            .filter_map(|comparison| comparison.relative.map(|value| (comparison, value.abs())))
            .max_by(|(_, a), (_, b)| a.total_cmp(b))
    }

    /// The report as Markdown: a summary line, then one table row per metric.
    ///
    /// It carries no date, so a run that changes nothing changes no bytes and the committed report
    /// only moves when a number does.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let scored = self.comparisons.iter().filter(|c| c.scored()).count();
        let mut out = String::new();
        out.push_str("# Validation report\n\n");
        out.push_str(&format!(
            "hpr-validate {}, {} case(s), {} metric(s): {}.\n\n",
            self.harness_version,
            self.cases.len(),
            self.comparisons.len(),
            if self.passed() {
                format!("{scored} scored, all within tolerance")
            } else {
                format!(
                    "{scored} scored, {} OUT OF TOLERANCE",
                    self.failures().len()
                )
            }
        ));
        if let Some((worst, relative)) = self.worst_scored() {
            out.push_str(&format!(
                "The largest scored difference is {}'s {} at {:+.3}%, against a {} gate.\n\n",
                worst.case,
                worst.metric,
                100.0 * worst.relative.unwrap_or(relative),
                worst.tolerance.describe()
            ));
        }
        if !self.not_scored().is_empty() {
            out.push_str(&format!(
                "{} metric(s) are measured and printed but held to no tolerance; each says why in \
                 its row, and none of them counts towards the verdict.\n\n",
                self.not_scored().len()
            ));
        }
        if self.fast {
            out.push_str(&if self.skipped.is_empty() {
                "Run with `--fast`. The lock marks no case slow, so nothing was left out.\n\n"
                    .to_owned()
            } else {
                format!(
                    "Run with `--fast`, which left out {} of the locked cases: {}.\n\n",
                    self.skipped.len(),
                    self.skipped.join(", ")
                )
            });
        }
        out.push_str(
            "Generated by `cargo xtask validate`. Every reference value names its own \
                      source; the generators are under `validation/oracles/`.\n\n",
        );
        out.push_str(
            "| case | metric | hpr | reference | difference | tolerance | verdict | note |\n",
        );
        out.push_str("|---|---|---|---|---|---|---|---|\n");
        for comparison in &self.comparisons {
            let relative = comparison.relative.map_or_else(
                || format!("{:+.6}", comparison.difference),
                |relative| format!("{:+.3}%", 100.0 * relative),
            );
            out.push_str(&format!(
                "| {} | {} | {:.6} | {:.6} | {} | {} | {} | {} |\n",
                comparison.case,
                comparison.metric,
                comparison.measured,
                comparison.reference,
                relative,
                comparison.tolerance.describe(),
                match comparison.verdict {
                    Verdict::Pass => "pass",
                    Verdict::Fail => "**fail**",
                    Verdict::NotScored => "not scored",
                },
                comparison.note.as_deref().unwrap_or("")
            ));
        }
        out.push_str("\n## References\n\n");
        for source in &self.sources {
            out.push_str(&format!(
                "- **{}**: {}, `{}` (sha256 `{}`).\n  - Oracle: {}\n  - Overrides: {}\n",
                source.case,
                source.oracle,
                source.file,
                source.sha256.get(..16).unwrap_or(&source.sha256),
                source.model,
                source.overrides
            ));
        }
        if let Some(source) = self.sources.first() {
            out.push_str(&format!(
                "\nRegenerate with `{}`, after `cargo xtask refs fetch` has put the oracle in the \
                 gitignored `refs/`. A reference moves only when its generator runs, which is a \
                 deliberate step: it is never regenerated to make a comparison pass (Loft lesson \
                 L76).\n",
                source.command
            ));
        }
        out
    }
}
