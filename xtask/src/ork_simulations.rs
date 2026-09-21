//! The stored-simulation counts `cargo xtask ork` prints (M3.1c3): the runs each design stores,
//! the conditions they were flown in, and the results OpenRocket saved.

use std::collections::BTreeMap;

use hpr_io::ork::{Atmosphere, Design, Imported};
use serde_json::{Value, json};

/// The counts, summed over the documents.
#[derive(Debug, Default)]
pub(crate) struct SimulationTally {
    documents: usize,
    simulations: usize,
    statuses: BTreeMap<String, usize>,
    with_conditions: usize,
    wind_from_stated: usize,
    into_wind: usize,
    atmospheres: BTreeMap<&'static str, usize>,
    with_summary: usize,
    with_time_series: usize,
    branches: usize,
    rows: usize,
    events: BTreeMap<String, usize>,
    warnings: usize,
}

impl SimulationTally {
    /// Counts one document and returns its per-file detail.
    pub(crate) fn add(&mut self, design: &Imported<Design>) -> Value {
        let simulations = &design.value.simulations;
        if !simulations.is_empty() {
            self.documents += 1;
        }
        let mut rows_here = 0usize;
        for simulation in simulations {
            self.simulations += 1;
            *self
                .statuses
                .entry(
                    simulation
                        .status
                        .clone()
                        .unwrap_or_else(|| "not written".to_owned()),
                )
                .or_default() += 1;
            if let Some(conditions) = &simulation.conditions {
                self.with_conditions += 1;
                if conditions.wind_from_rad.is_some() {
                    self.wind_from_stated += 1;
                }
                if conditions.into_wind == Some(true) {
                    self.into_wind += 1;
                }
                let atmosphere = match conditions.atmosphere {
                    None => "not written",
                    Some(Atmosphere::Isa) => "isa",
                    Some(Atmosphere::Extended { .. }) => "extendedisa",
                    Some(_) => "other",
                };
                *self.atmospheres.entry(atmosphere).or_default() += 1;
            }
            if let Some(results) = &simulation.results {
                if results.max_altitude_m.is_some() {
                    self.with_summary += 1;
                }
                if !results.branches.is_empty() {
                    self.with_time_series += 1;
                }
                for branch in &results.branches {
                    self.branches += 1;
                    rows_here += branch.rows.len();
                    for event in &branch.events {
                        *self.events.entry(event.kind.clone()).or_default() += 1;
                    }
                }
            }
        }
        self.rows += rows_here;
        let warnings = design
            .warnings
            .iter()
            .filter(|w| w.at.starts_with("openrocket/simulations"))
            .count();
        self.warnings += warnings;
        json!({
            "simulations": simulations.len(),
            "rows": rows_here,
            "warnings": warnings,
        })
    }

    /// The counts, for the report's summary.
    pub(crate) fn summary(&self) -> Value {
        json!({
            "documents_with_simulations": self.documents,
            "simulations": self.simulations,
            "statuses": self.statuses,
            "with_conditions": self.with_conditions,
            "wind_direction_stated": self.wind_from_stated,
            "launched_into_the_wind": self.into_wind,
            "atmospheres": self.atmospheres,
            "with_a_summary": self.with_summary,
            "with_a_time_series": self.with_time_series,
            "branches": self.branches,
            "rows": self.rows,
            "events": self.events,
            "warnings": self.warnings,
        })
    }

    /// Prints the counts under the rest of the survey.
    pub(crate) fn print(&self) {
        println!(
            "  stored simulations: {} in {} document(s){}",
            self.simulations,
            self.documents,
            listed(&self.statuses, ", by status: ")
        );
        println!(
            "  their conditions: {} stated, {} with the wind's direction, {} launched into the \
             wind{}",
            self.with_conditions,
            self.wind_from_stated,
            self.into_wind,
            listed(&self.atmospheres, "; atmosphere: ")
        );
        println!(
            "  their results: {} with a summary, {} with a time series, over {} branch(es) and {} \
             row(s)",
            self.with_summary, self.with_time_series, self.branches, self.rows
        );
        println!("  events stored:{}", listed(&self.events, " "));
        println!("  warnings reading stored simulations: {}", self.warnings);
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
