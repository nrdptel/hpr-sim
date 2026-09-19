//! Case files: what to fly, and which metrics to compare against which reference.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One validation case, read from a TOML file under `validation/cases/`.
///
/// Every metric it reports has to name a tolerance ([Loft lesson L79][l79]) or say in writing why
/// it is not scored, and the reference it compares against has to carry provenance ([L77][l77]).
/// The harness checks both before it flies anything.
///
/// [l77]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l77
/// [l79]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l79
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    /// The case's id, which is also its file name without the extension.
    pub id: String,
    /// What the case is for, in one line.
    pub title: String,
    /// What is flown and how ([`Flight`]).
    pub flight: Flight,
    /// The reference to compare against, relative to the repository root.
    pub reference: PathBuf,
    /// Which case of that reference file, when it holds several.
    #[serde(default)]
    pub reference_case: Option<String>,
    /// The metrics to compare, each with its tolerance.
    pub metrics: BTreeMap<String, Metric>,
    /// A limit of hpr's that this case is known to reach, in writing. The case still runs, and
    /// its metrics keep the tolerances they will be held to once the limit is lifted, but nothing
    /// is scored: the report shows the case as a gap, with this reason and hpr's own refusal.
    ///
    /// The one gap the harness accepts is hpr's refusal of a Mach number past its models' range,
    /// which ends at Mach 5 for the normal force and the drag buildup alike (until
    /// [M1.8b1][m1-8b1] the buildup stopped at Mach 1, and Prometheus on its own drag was such a
    /// gap). It is checked, not trusted: the reference must itself reach that Mach number, and hpr
    /// must refuse the flight with exactly that error. A gap that starts flying fails the run, so
    /// it cannot stay excused after it is fixed ([Loft lesson L85][l85]). No committed case
    /// declares one.
    ///
    /// [m1-8b1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-8b1
    /// [l85]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l85
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_gap: Option<String>,
}

/// What a case flies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[non_exhaustive]
pub enum Flight {
    /// A descent from a state the reference declares, under the devices the reference declares:
    /// the recovery comparison of [M1.7a][m1-7a] (parachutes, and the descent under them), run
    /// through the harness.
    ///
    /// Everything about the flight comes from the reference's own case (the site, the wind, the
    /// devices, the state at the first deployment), which is the point: the oracle's inputs are
    /// the case's, not hpr's output ([Loft lesson L75][l75]).
    ///
    /// [m1-7a]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-7a
    /// [l75]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l75
    RecoveryDescent {
        /// The design to fly, by file name under `validation/designs/`.
        design: String,
        /// Its configuration id.
        configuration: String,
    },
    /// A flight from the pad to the ground, in either mode of [M2.1][m2-1] ([`DragMode`]). In
    /// same-drag mode hpr flies the `C_D0(M)` table the reference declares, through
    /// [`hpr_sim::Simulation::with_drag_table`], so a difference is in the equations of motion,
    /// the environment or the motor, not in the drag. In predicted mode it flies the design's own
    /// aerodynamics against a reference that flew the example's own drag.
    ///
    /// The site, the wind, the rail, the drag table and its reference area, and the recovery
    /// devices all come from the reference's own record of what the oracle flew
    /// ([Loft lesson L75][l75]). The harness checks the rest rather than trusting it: the design
    /// must be the one the reference names, with the dry mass and reference area it recorded.
    ///
    /// [m2-1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-1
    /// [l75]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l75
    WholeFlight {
        /// The design to fly, by file name under `validation/designs/`.
        design: String,
        /// Its configuration id.
        configuration: String,
        /// Whose drag hpr flies: the reference's declared table (the default), or its own.
        #[serde(default, skip_serializing_if = "DragMode::is_same_drag")]
        mode: DragMode,
    },
}

/// Whose drag a whole flight flies, the two modes of [M2.1][m2-1].
///
/// [m2-1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-1
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DragMode {
    /// hpr flies the `C_D0(M)` table the reference declares, so a difference is in the equations
    /// of motion, the environment or the motor. Its metrics are gated.
    #[default]
    SameDrag,
    /// hpr flies its own drag against a reference in which RocketPy flies the example's own drag,
    /// so a difference is mostly the two drags ([M2.1c2][m2-1c2]). hpr's normal force is its own
    /// in both modes; only the zero-lift drag differs. Its metrics are
    /// held to a target and reported, never gated ([`crate::Verdict::WithinTarget`]).
    ///
    /// [m2-1c2]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m2-1c2
    Predicted,
}

impl DragMode {
    /// Whether this is the default, same-drag mode.
    #[must_use]
    pub fn is_same_drag(&self) -> bool {
        *self == Self::SameDrag
    }
}

impl Flight {
    /// The design it flies, by file name under `validation/designs/`.
    #[must_use]
    pub fn design(&self) -> &str {
        match self {
            Self::RecoveryDescent { design, .. } | Self::WholeFlight { design, .. } => design,
        }
    }
}

/// One metric of a case: how far hpr may be from the reference, or why it is not scored.
///
/// In a case file that is a table of one or both bounds:
///
/// ```toml
/// [metrics.descent_time_s]
/// relative = 0.03
///
/// [metrics.drift_north_m]
/// relative = 0.03
/// absolute = 0.002    # this component passes through zero, so a fraction alone means nothing
/// ```
///
/// A metric whose table sets neither is refused, because that is the "ungated metric" of
/// [Loft lesson L79][l79] by another name.
///
/// The one alternative to a gate is to say, in the case file, that the metric is not scored and
/// why:
///
/// ```toml
/// [metrics.drift_north_m]
/// not_scored = "hpr and RocketPy differ by 28x on 0.5 mm and the cause is not established (#27)"
/// ```
///
/// Such a metric is still measured and still printed, with both numbers, the difference and the
/// reason: it is a gap on the face of the report, not a quiet omission and not a pass. Loft
/// excused its two largest misses as "no single target" ([Loft lesson L82][l82]), so the reason
/// has to be written down, and
/// `hpr_validate::tests::the_metrics_that_are_not_scored_are_these_and_no_others` pins the whole
/// set: a new excuse has to be argued in a test whose name says what it is.
///
/// [l79]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l79
/// [l82]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l82
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    /// How far hpr may be from the reference, as a fraction of it (`0.03` for 3%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<f64>,
    /// How far hpr may be from the reference, in the metric's own unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absolute: Option<f64>,
    /// Why the metric is measured and reported but not scored. Mutually exclusive with a bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_scored: Option<String>,
}

impl Metric {
    /// A relative bound alone.
    #[must_use]
    pub fn relative(relative: f64) -> Self {
        Self {
            relative: Some(relative),
            ..Self::default()
        }
    }

    /// The bounds it holds hpr to.
    #[must_use]
    pub fn tolerance(&self) -> Tolerance {
        Tolerance {
            relative: self.relative,
            absolute: self.absolute,
        }
    }

    /// The reason it is not scored, if it says one.
    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.not_scored
            .as_deref()
            .map(str::trim)
            .filter(|reason| !reason.is_empty())
    }

    /// Checks that the metric is either gated or declared, and that its bounds are real numbers
    /// that bound something.
    ///
    /// # Errors
    ///
    /// A sentence naming what is wrong, for [`crate::run::ValidateError::Case`].
    pub fn check(&self, case: &str, name: &str) -> Result<(), String> {
        let gated = self.tolerance().is_set();
        match (gated, self.reason()) {
            (false, None) => {
                // Loft lesson L79: a tolerance that bounds nothing gates nothing. A metric is
                // either held to a number or declared, in writing, not to be.
                Err(format!(
                    "case {case}: {name} has a tolerance that bounds nothing, and no written \
                     reason for not scoring it"
                ))
            }
            (true, Some(_)) => Err(format!(
                "case {case}: {name} is both gated and declared not scored; it is one or the other"
            )),
            (false, Some(_)) => Ok(()),
            (true, None) => {
                for (label, bound) in [("relative", self.relative), ("absolute", self.absolute)] {
                    if let Some(bound) = bound
                        && !(bound.is_finite() && bound > 0.0)
                    {
                        // An infinite or negative bound is a gate that cannot fail.
                        return Err(format!(
                            "case {case}: {name}'s {label} bound is {bound}, which bounds nothing"
                        ));
                    }
                }
                Ok(())
            }
        }
    }
}

/// How far a measured value may be from its reference: a fraction of it, an absolute difference,
/// or whichever of the two is larger.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tolerance {
    /// A fraction of the reference value, as `0.03` for 3%.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relative: Option<f64>,
    /// An absolute difference, in the metric's own unit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub absolute: Option<f64>,
}

impl Tolerance {
    /// A relative bound alone.
    #[must_use]
    pub const fn relative(relative: f64) -> Self {
        Self {
            relative: Some(relative),
            absolute: None,
        }
    }

    /// Whether this bounds anything at all. A bound that is not a positive, finite number gates
    /// nothing, the "ungated metric" of [Loft lesson L79][l79], so it does not count as set.
    ///
    /// [l79]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l79
    #[must_use]
    pub fn is_set(self) -> bool {
        [self.relative, self.absolute]
            .into_iter()
            .flatten()
            .any(|bound| bound.is_finite() && bound > 0.0)
    }

    /// How much `reference` may move under this tolerance, in the metric's own unit.
    #[must_use]
    pub fn allowed(self, reference: f64) -> f64 {
        let relative = self
            .relative
            .filter(|bound| bound.is_finite() && *bound > 0.0)
            .map_or(0.0, |relative| relative * reference.abs());
        let absolute = self
            .absolute
            .filter(|bound| bound.is_finite() && *bound > 0.0)
            .unwrap_or(0.0);
        relative.max(absolute)
    }

    /// Whether `measured` is within this tolerance of `reference`: inside either bound that is
    /// given. A tolerance with neither accepts nothing, so an unset one fails rather than passing
    /// silently, and a value that is not a real number never passes.
    #[must_use]
    pub fn accepts(self, measured: f64, reference: f64) -> bool {
        self.is_set()
            && measured.is_finite()
            && reference.is_finite()
            && (measured - reference).abs() <= self.allowed(reference)
    }

    /// How the tolerance reads in a report.
    #[must_use]
    pub fn describe(self) -> String {
        match (self.relative, self.absolute) {
            (Some(relative), Some(absolute)) => format!("{:.3}% or {absolute}", 100.0 * relative),
            (Some(relative), None) => format!("{:.3}%", 100.0 * relative),
            (None, Some(absolute)) => format!("{absolute}"),
            (None, None) => "none".to_owned(),
        }
    }
}

/// The cases a run must cover, read from `validation/cases/lock.toml`.
///
/// [Loft lesson L78][l78]: a suite that quietly skips a case and reports green is worse than a
/// red one, so the lock names every case that has to run and the harness fails if one is missing.
///
/// [l78]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l78
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseLock {
    /// The case ids that must run, in report order.
    pub cases: Vec<String>,
    /// The ids `--fast` may leave out, which must all be in `cases`.
    #[serde(default)]
    pub slow: Vec<String>,
}

impl CaseLock {
    /// The ids a run covers: every locked case, or the ones that are not slow.
    #[must_use]
    pub fn wanted(&self, fast: bool) -> Vec<String> {
        self.cases
            .iter()
            .filter(|id| !(fast && self.slow.contains(id)))
            .cloned()
            .collect()
    }

    /// The ids a run leaves out, which is empty unless it is a fast one.
    #[must_use]
    pub fn skipped(&self, fast: bool) -> Vec<String> {
        self.cases
            .iter()
            .filter(|id| fast && self.slow.contains(id))
            .cloned()
            .collect()
    }

    /// The ids named as slow that are not cases at all.
    #[must_use]
    pub fn unknown_slow(&self) -> Vec<String> {
        self.slow
            .iter()
            .filter(|id| !self.cases.contains(id))
            .cloned()
            .collect()
    }
}

/// Where the cases and their lock live, relative to the repository root.
#[must_use]
pub fn cases_dir(root: &Path) -> PathBuf {
    root.join("validation/cases")
}

/// Every case id committed under `validation/cases/`, sorted.
///
/// A case that is committed but not locked would never run, which is the other half of
/// [Loft lesson L78][l78]: a suite must not quietly skip a case.
///
/// [l78]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l78
///
/// # Errors
///
/// The directory's own error, as a sentence.
pub fn committed_cases(root: &Path) -> Result<Vec<String>, String> {
    let directory = cases_dir(root);
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(&directory)
        .map_err(|error| format!("reading {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("reading {}: {error}", directory.display()))?
            .path();
        if path
            .extension()
            .is_some_and(|extension| extension == "toml")
            && let Some(stem) = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
            && stem != "lock"
        {
            ids.push(stem);
        }
    }
    ids.sort();
    Ok(ids)
}
