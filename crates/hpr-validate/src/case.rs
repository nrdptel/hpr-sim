//! Case files: what to fly, and which metrics to compare against which reference.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One validation case, read from a TOML file under `validation/cases/`.
///
/// Every metric it reports has to name a tolerance (Loft lesson L79), and the reference it
/// compares against has to carry provenance (L77). The harness checks both before it flies
/// anything.
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
}

/// What a case flies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[non_exhaustive]
pub enum Flight {
    /// A descent from a state the reference declares, under the devices the reference declares:
    /// M1.7a's recovery comparison, run through the harness.
    ///
    /// Everything about the flight comes from the reference's own case (the site, the wind, the
    /// devices, the state at the first deployment), which is the point: the oracle's inputs are
    /// the case's, not hpr's output (L75).
    RecoveryDescent {
        /// The design to fly, by file name under `validation/designs/`.
        design: String,
        /// Its configuration id.
        configuration: String,
    },
}

/// One metric of a case: how far hpr may be from the reference.
///
/// In a case file that is a table of one or both bounds:
///
/// ```toml
/// [metrics.descent_time_s]
/// relative = 0.03
///
/// [metrics.drift_north_m]
/// relative = 0.03
/// absolute = 2.0     # this component passes through zero, so a fraction alone means nothing
/// ```
///
/// A metric whose table sets neither is refused, because that is the "ungated metric" of Loft
/// lesson L79 by another name.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    /// How far hpr may be from the reference.
    #[serde(flatten)]
    pub tolerance: Tolerance,
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

    /// Whether this bounds anything at all. A tolerance that bounds nothing gates nothing (L79).
    #[must_use]
    pub const fn is_set(self) -> bool {
        self.relative.is_some() || self.absolute.is_some()
    }

    /// Whether `measured` is within this tolerance of `reference`: inside either bound that is
    /// given. A tolerance with neither accepts nothing, so an unset one fails rather than passing
    /// silently.
    #[must_use]
    pub fn accepts(self, measured: f64, reference: f64) -> bool {
        let allowed = self
            .relative
            .map_or(0.0, |relative| relative * reference.abs())
            .max(self.absolute.unwrap_or(0.0));
        self.is_set() && (measured - reference).abs() <= allowed
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
/// Loft lesson L78: a suite that quietly skips a case and reports green is worse than a red one,
/// so the lock names every case that has to run and the harness fails if one is missing.
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
