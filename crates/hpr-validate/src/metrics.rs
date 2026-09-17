//! References and measured values: what an oracle said, and what hpr said.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One reference value and where it comes from.
///
/// Loft lesson L77: Loft shipped "stored results" that were hand-written, one set internally
/// inconsistent. A value without a source is not a reference, so `source` is required.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceValue {
    /// The value, in the metric's own unit.
    pub value: f64,
    /// Where it came from: the oracle's own field, a printed page, or a closed form.
    pub source: String,
}

/// An oracle's answers for one case, with the provenance of the run that produced them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    /// What produced it, as the generator records it (for example `rocketpy 1.13.0`).
    pub oracle: String,
    /// The generator script, relative to the repository root.
    pub generator: String,
    /// The command that regenerates it.
    pub command: String,
    /// What the oracle modelled, in the generator's words.
    pub model: String,
    /// What the generator overrode to make the comparison like-for-like.
    pub overrides: String,
    /// The reference's own case id.
    pub case: String,
    /// The file it was read from, relative to the repository root.
    pub file: String,
    /// That file's SHA-256, so an edited reference shows up in the report itself.
    pub sha256: String,
    /// The values, by metric name.
    pub values: BTreeMap<String, ReferenceValue>,
}

impl Reference {
    /// Whether the run that produced it is named at all: a file that does not say what wrote it,
    /// with which command, is not a reference (L77), however many numbers it holds.
    #[must_use]
    pub fn names_its_run(&self) -> bool {
        [&self.oracle, &self.generator, &self.command]
            .into_iter()
            .all(|field| !field.trim().is_empty())
    }

    /// The metrics that carry no source, which is what L77 refuses.
    #[must_use]
    pub fn without_provenance(&self) -> Vec<String> {
        self.values
            .iter()
            .filter(|(_, value)| value.source.trim().is_empty())
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// What hpr measured for one case, by metric name.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Measured {
    /// The values, by metric name, in the same units as the reference's.
    pub values: BTreeMap<String, f64>,
}

impl Measured {
    /// Records `value` for `name`.
    pub fn insert(&mut self, name: &str, value: f64) {
        self.values.insert(name.to_owned(), value);
    }
}
