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
    stored_eligible: usize,
    stored_excluded: usize,
    stored_exclusions: BTreeMap<String, usize>,
    reproducible: usize,
    not_reproducible: usize,
    reproduction_exclusions: BTreeMap<String, usize>,
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
        let mut stored_eligible_here = 0usize;
        let mut stored_exclusions_here: BTreeMap<String, usize> = BTreeMap::new();
        let mut reproducible_here = 0usize;
        let mut reproduction_exclusions_here: BTreeMap<String, usize> = BTreeMap::new();
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
            if let Some(exclusion) = simulation.reference_exclusion() {
                self.stored_excluded += 1;
                let reason = exclusion.reason().to_owned();
                *self.stored_exclusions.entry(reason.clone()).or_default() += 1;
                *stored_exclusions_here.entry(reason).or_default() += 1;
            } else {
                self.stored_eligible += 1;
                stored_eligible_here += 1;
                if let Some(exclusion) = design.value.reproduction_exclusion(simulation) {
                    self.not_reproducible += 1;
                    *self
                        .reproduction_exclusions
                        .entry(exclusion.reason().to_owned())
                        .or_default() += 1;
                    *reproduction_exclusions_here
                        .entry(exclusion.reason().to_owned())
                        .or_default() += 1;
                } else {
                    self.reproducible += 1;
                    reproducible_here += 1;
                }
            }
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
            "stored_eligible_references": stored_eligible_here,
            "stored_excluded_references": simulations.len() - stored_eligible_here,
            "stored_reference_exclusions": stored_exclusions_here,
            "reproducible_references": reproducible_here,
            "not_reproducible_references": stored_eligible_here - reproducible_here,
            "reproduction_exclusions": reproduction_exclusions_here,
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
            "stored_eligible_references": self.stored_eligible,
            "stored_excluded_references": self.stored_excluded,
            "stored_reference_exclusions": self.stored_exclusions,
            "reproducible_references": self.reproducible,
            "not_reproducible_references": self.not_reproducible,
            "reproduction_exclusions": self.reproduction_exclusions,
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
            "  stored reference screen: {} eligible, {} excluded{}",
            self.stored_eligible,
            self.stored_excluded,
            listed(&self.stored_exclusions, "; reasons: "),
        );
        println!(
            "  hpr reproduction screen: {} reproducible, {} not reproducible{}",
            self.reproducible,
            self.not_reproducible,
            listed(&self.reproduction_exclusions, "; reasons: "),
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

#[cfg(test)]
mod tests {
    use super::*;
    use hpr_io::ork;

    fn document(simulations: &str) -> Imported<Design> {
        let xml = format!(
            r#"<openrocket version="1.10"><rocket><name>R</name></rocket>{simulations}</openrocket>"#
        );
        let file = ork::read(xml.as_bytes()).expect("simulation fixture reads");
        ork::design(&file.value)
    }

    fn simulation(name: &str, status: &str, conditions: &str) -> String {
        format!(
            r#"<simulation status="{status}"><name>{name}</name><simulator>RK4Simulator</simulator><calculator>BarrowmanCalculator</calculator>{conditions}<flightdata maxaltitude="100" maxvelocity="80" maxacceleration="120" maxmach="0.23" timetoapogee="5" flighttime="20"/></simulation>"#
        )
    }

    #[test]
    fn tally_keeps_stored_and_reproduction_screens_separate() {
        let simulations = format!(
            "<simulations>{}<simulation status=\"uptodate\"><name>reduced</name><simulator>RK4Simulator</simulator><calculator>BarrowmanCalculator</calculator><flightdata maxaltitude=\"100\" maxvelocity=\"80\" maxacceleration=\"120\" maxmach=\"0.23\" timetoapogee=\"5\" flighttime=\"20\"/></simulation></simulations>",
            simulation(
                "eligible",
                "uptodate",
                "<conditions><configid>missing</configid></conditions>"
            )
        );
        let design = document(&simulations);
        // This fixture's two simulations both have complete stored data, but no motor
        // configuration can be reproduced from the conditions. The tally must still count both
        // as stored references before applying the separate reproduction screen.
        let mut tally = SimulationTally::default();
        let detail = tally.add(&design);
        let summary = tally.summary();
        assert_eq!(summary["simulations"], 2);
        assert_eq!(summary["stored_eligible_references"], 2);
        assert_eq!(summary["stored_excluded_references"], 0);
        assert_eq!(summary["reproducible_references"], 0);
        assert_eq!(summary["not_reproducible_references"], 2);
        assert_eq!(
            summary["reproduction_exclusions"]["configuration-unknown"],
            1
        );
        assert_eq!(
            summary["reproduction_exclusions"]["configuration-missing"],
            1
        );
        assert_eq!(detail["stored_eligible_references"], 2);
        assert_eq!(detail["not_reproducible_references"], 2);
    }

    #[test]
    fn tally_reports_stable_stored_exclusion_reasons() {
        let simulations = format!(
            "<simulations>{}{}</simulations>",
            simulation("stale", "outdated", ""),
            simulation("missing simulator", "uptodate", "")
                .replace("<simulator>RK4Simulator</simulator>", "",)
        );
        let design = document(&simulations);
        let mut tally = SimulationTally::default();
        let detail = tally.add(&design);
        let summary = tally.summary();
        assert_eq!(summary["stored_eligible_references"], 0);
        assert_eq!(summary["stored_excluded_references"], 2);
        assert_eq!(summary["stored_reference_exclusions"]["status-outdated"], 1);
        assert_eq!(
            summary["stored_reference_exclusions"]["simulator-missing"],
            1
        );
        assert_eq!(detail["stored_reference_exclusions"]["status-outdated"], 1);
        assert_eq!(
            detail["stored_reference_exclusions"]["simulator-missing"],
            1
        );
    }
}
