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

/// A case that ran into a limit of hpr's it declares in writing ([`crate::Case::known_gap`]): it
/// was flown, hpr refused it as the case said it would, and none of its metrics was scored.
///
/// It is not a pass and not a quiet omission. The report prints it in a section of its own, with
/// the case's reason and hpr's refusal, and the summary counts it apart.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Gap {
    /// The case's id.
    pub case: String,
    /// The case's written reason.
    pub reason: String,
    /// What hpr said when it refused the flight.
    pub refusal: String,
    /// The Mach number hpr refused.
    pub mach: f64,
    /// How many metrics the case would have scored, with the tolerances they will be held to.
    pub metric_count: usize,
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
    /// The cases that ran into a limit of hpr's they declare, in case order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<Gap>,
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

    /// Whether this run reproduces a committed report, given the text of its `latest.md` and
    /// `latest.json`.
    ///
    /// hpr is bit-identical on one platform, not across three ([ADR-015][adr-015], the validation
    /// harness's decisions), so the JSON is not compared by its bytes: it carries full-precision
    /// floats. Everything that cannot differ by
    /// platform is compared exactly: the harness version, the cases, what was left out, the
    /// sources, each comparison's case, metric, source, tolerance, verdict and note, and each gap
    /// but for the Mach number the integrator narrowed onto 1. The numbers are compared through the
    /// Markdown, the committed file's and the committed JSON's rendering alike, which must match
    /// this run's letter for letter except that a number may differ by two units in its sixth
    /// decimal or by 1e-7 of itself, whichever is larger. A tighter check would assert a
    /// cross-platform bit-identity hpr does not claim.
    ///
    /// # Errors
    ///
    /// The first difference found, or why the committed JSON does not parse.
    ///
    /// [adr-015]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-015-the-validation-harness-cases-references-tolerances-and-reports-2026-09-17
    pub fn reproduces(&self, committed_markdown: &str, committed_json: &str) -> Result<(), String> {
        let markdown = self.to_markdown();
        same_but_for_platform_rounding(committed_markdown, &markdown)
            .map_err(|what| format!("latest.md: {what}"))?;
        let committed: Self = serde_json::from_str(committed_json)
            .map_err(|error| format!("latest.json does not parse: {error}"))?;
        let differs = |what: &str| Err(format!("latest.json: {what} differ"));
        if committed.harness_version != self.harness_version {
            return differs("the harness versions");
        }
        if committed.fast != self.fast || committed.skipped != self.skipped {
            return differs("the cases left out");
        }
        if committed.cases != self.cases {
            return differs("the cases");
        }
        // A gap's Mach number is where the integrator narrowed onto 1, to the last bits of which
        // the platforms need not agree; everything else about it must.
        let gap_shape = |report: &Self| {
            report
                .gaps
                .iter()
                .map(|gap| {
                    (
                        gap.case.clone(),
                        gap.reason.clone(),
                        gap.refusal.clone(),
                        gap.metric_count,
                        (gap.mach - 1.0).abs() < 1e-9,
                    )
                })
                .collect::<Vec<_>>()
        };
        if gap_shape(&committed) != gap_shape(self) {
            return differs("the known gaps");
        }
        if committed.sources != self.sources {
            return differs("the sources");
        }
        let shape = |report: &Self| {
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
        if shape(&committed) != shape(self) {
            return differs("the comparisons' metrics, sources, tolerances, verdicts or notes");
        }
        same_but_for_platform_rounding(&committed.to_markdown(), &markdown)
            .map_err(|what| format!("latest.json: {what}"))
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
        if !self.gaps.is_empty() {
            out.push_str(&format!(
                "{} case(s) are known gaps: flown, refused by hpr for the reason the case gives, \
                 and not scored. They are listed under Known gaps, below the table.\n\n",
                self.gaps.len()
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
        if !self.gaps.is_empty() {
            out.push_str("\n## Known gaps\n\n");
            for gap in &self.gaps {
                out.push_str(&format!(
                    "- **{}**: {} metric(s), none scored. {} hpr: {}\n",
                    gap.case, gap.metric_count, gap.reason, gap.refusal
                ));
            }
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
        // Each reference's own command, once, in the order the cases first name it.
        let mut commands: Vec<&str> = Vec::new();
        for source in &self.sources {
            if !commands.contains(&source.command.as_str()) {
                commands.push(&source.command);
            }
        }
        if !commands.is_empty() {
            out.push_str(&format!(
                "\nRegenerate with {}, after `cargo xtask refs fetch` has put the oracle in the \
                 gitignored `refs/`. A reference moves only when its generator runs, which is a \
                 deliberate step: it is never regenerated to make a comparison pass (Loft lesson \
                 L76).\n",
                commands
                    .iter()
                    .map(|command| format!("`{command}`"))
                    .collect::<Vec<_>>()
                    .join(" and ")
            ));
        }
        out
    }
}

/// Whether two renderings of the report are the same, letter for letter, except that a number
/// may differ by two units in its sixth decimal or by 1e-7 of itself, whichever is larger.
///
/// hpr is bit-identical on one platform, not across three (ADR-015). The descents reproduce to
/// the six decimals the report prints; a whole flight does to about 1e-8 of each value, not
/// always to its sixth decimal: NDRT 2020's landing drift is 354.240893 m on macOS and
/// 354.240895 m on Linux, after 84 s of six-degree-of-freedom flight in a sheared wind. Every
/// word, case, metric, tolerance and verdict must still match exactly.
pub(crate) fn same_but_for_platform_rounding(
    committed: &str,
    computed: &str,
) -> Result<(), String> {
    let (committed, computed): (Vec<&str>, Vec<&str>) =
        (committed.lines().collect(), computed.lines().collect());
    if committed.len() != computed.len() {
        return Err(format!(
            "the committed report has {} lines and this run's {}",
            committed.len(),
            computed.len()
        ));
    }
    let number = |cell: &str| {
        cell.trim()
            .trim_end_matches('%')
            .trim_start_matches('+')
            .parse::<f64>()
            .ok()
    };
    for (line, (old, new)) in committed.iter().zip(&computed).enumerate() {
        if old == new {
            continue;
        }
        let (old_cells, new_cells): (Vec<&str>, Vec<&str>) =
            (old.split('|').collect(), new.split('|').collect());
        let close = old_cells.len() == new_cells.len()
            && old_cells.iter().zip(&new_cells).all(|(a, b)| {
                a == b
                    || matches!((number(a), number(b)), (Some(a), Some(b))
                        if (a - b).abs() <= (2e-6_f64).max(1e-7 * b.abs()))
            });
        if !close {
            return Err(format!(
                "line {} differs beyond platform rounding:\n  committed: {old}\n  this run:  {new}",
                line + 1
            ));
        }
    }
    Ok(())
}
