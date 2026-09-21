//! The `x-openrocket` counts `cargo xtask ork` prints (M3.1c4): the parts and sections of each
//! document hpr does not model, kept whole, and a check that each is found again at its path.

use std::collections::BTreeMap;

use hpr_io::ork::{Design, Document, Imported, element_at};
use serde_json::{Value, json};

/// The counts, summed over the documents.
#[derive(Debug, Default)]
pub(crate) struct ExtensionTally {
    reduced: usize,
    parts: BTreeMap<String, usize>,
    sections: BTreeMap<String, usize>,
    tags: BTreeMap<String, usize>,
    attributes: BTreeMap<String, usize>,
    lost: Vec<String>,
}

impl ExtensionTally {
    /// Counts one document and returns its per-file detail.
    pub(crate) fn add(&mut self, design: &Imported<Design>, document: &Document) -> Value {
        let kept = &design.value.extensions.x_openrocket;
        if design.value.is_reduced() {
            self.reduced += 1;
        }
        for part in &kept.parts {
            *self.parts.entry(part.element.name.clone()).or_default() += 1;
        }
        for section in &kept.sections {
            *self
                .sections
                .entry(section.element.name.clone())
                .or_default() += 1;
        }
        for tag in &kept.tags {
            *self.tags.entry(tag.element.name.clone()).or_default() += 1;
        }
        for attribute in &kept.attributes {
            // Counted by the element it is on and its name, `material@group`.
            let step = attribute.at.rsplit('/').next().unwrap_or_default();
            let on = step
                .trim_start_matches('@')
                .split('[')
                .next()
                .unwrap_or_default();
            *self
                .attributes
                .entry(format!("{on}@{}", attribute.name))
                .or_default() += 1;
            let found = element_at(document, &attribute.at)
                .and_then(|element| element.attribute(&attribute.name))
                == Some(attribute.value.as_str());
            if !found {
                self.lost
                    .push(format!("{}/@@{}", attribute.at, attribute.name));
            }
        }
        for kept in kept.parts.iter().chain(&kept.sections).chain(&kept.tags) {
            if element_at(document, &kept.at) != Some(&kept.element) {
                self.lost.push(kept.at.clone());
            }
        }
        json!({
            "reduced": design.value.is_reduced(),
            "parts": kept.parts.iter().map(|k| k.at.clone()).collect::<Vec<_>>(),
            "sections": kept.sections.iter().map(|k| k.at.clone()).collect::<Vec<_>>(),
        })
    }

    /// The counts, for the report's summary.
    pub(crate) fn summary(&self) -> Value {
        json!({
            "designs_reduced": self.reduced,
            "parts_kept": self.parts,
            "sections_kept": self.sections,
            "tags_kept": self.tags,
            "attributes_kept": self.attributes,
            "not_found_again": self.lost,
        })
    }

    /// Prints the counts under the rest of the survey.
    pub(crate) fn print(&self) {
        println!(
            "  kept in x-openrocket: {} part(s){} in {} reduced design(s); {} section(s){}",
            self.parts.values().sum::<usize>(),
            listed(&self.parts, ": "),
            self.reduced,
            self.sections.values().sum::<usize>(),
            listed(&self.sections, ": ")
        );
        println!(
            "  tags kept, in elements hpr reads: {}{}",
            self.tags.values().sum::<usize>(),
            listed(&self.tags, ": ")
        );
        println!(
            "  attributes kept, on elements hpr reads: {}{}",
            self.attributes.values().sum::<usize>(),
            listed(&self.attributes, ": ")
        );
        let all = self.parts.values().sum::<usize>()
            + self.sections.values().sum::<usize>()
            + self.tags.values().sum::<usize>()
            + self.attributes.values().sum::<usize>();
        println!(
            "  kept elements found again at their path: {} of {all}",
            all - self.lost.len()
        );
    }

    /// Why the survey should fail: a kept element its path does not lead back to.
    pub(crate) fn failure(&self) -> Option<String> {
        (!self.lost.is_empty()).then(|| {
            format!(
                "{} kept element(s) are not found again at their path: {}",
                self.lost.len(),
                self.lost.join(", ")
            )
        })
    }
}

/// `counts` as `key n, key n`, after `lead`; nothing when empty.
fn listed<K: std::fmt::Display>(counts: &BTreeMap<K, usize>, lead: &str) -> String {
    if counts.is_empty() {
        return String::new();
    }
    let line = counts
        .iter()
        .map(|(key, count)| format!("{key} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{lead}{line}")
}
