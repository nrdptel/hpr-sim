//! The recovery counts `cargo xtask ork` prints (M3.1c2): the parachutes and streamers each design
//! holds, when they open and how much drag they state, and when its stages separate.

use std::collections::BTreeMap;

use hpr_io::ork::{Design, DeviceKind, Dimension, Imported};
use serde_json::{Value, json};

/// The counts, summed over the designs.
#[derive(Debug, Default)]
pub(crate) struct RecoveryTally {
    devices: BTreeMap<&'static str, usize>,
    cd: BTreeMap<&'static str, usize>,
    deploy_events: BTreeMap<String, usize>,
    devices_changed_per_configuration: usize,
    deployment_overrides: usize,
    unread: BTreeMap<String, usize>,
    stages: usize,
    stages_separating: usize,
    separation_events: BTreeMap<String, usize>,
    separation_overrides: usize,
    warnings: usize,
}

impl RecoveryTally {
    /// Counts one design and returns its per-file detail.
    pub(crate) fn add(&mut self, design: &Imported<Design>) -> Value {
        let recovery = &design.value.recovery;
        for device in &recovery.devices {
            *self.devices.entry(kind(device.kind)).or_default() += 1;
            let cd = match device.cd {
                None => "not written",
                Some(Dimension::Automatic { .. }) => "auto",
                Some(_) => "stated",
            };
            *self.cd.entry(cd).or_default() += 1;
            let event = device
                .deployment
                .event
                .as_ref()
                .map_or("not written", |event| event.as_str());
            *self.deploy_events.entry(event.to_owned()).or_default() += 1;
            if !device.configurations.is_empty() {
                self.devices_changed_per_configuration += 1;
            }
            self.deployment_overrides += device.configurations.len();
        }
        for device in &recovery.unread {
            *self.unread.entry(device.inside.clone()).or_default() += 1;
        }
        self.stages += design.value.rocket.stages.len();
        self.stages_separating += recovery.separations.len();
        for stage in &recovery.separations {
            let event = stage
                .separation
                .event
                .as_ref()
                .map_or("not written", |event| event.as_str());
            *self.separation_events.entry(event.to_owned()).or_default() += 1;
            self.separation_overrides += stage.configurations.len();
        }
        let warnings: Vec<Value> = design
            .warnings
            .iter()
            .filter(|w| w.at.contains("/deployment") || w.at.contains("/separation"))
            .map(|w| json!({ "at": w.at, "says": w.message }))
            .collect();
        self.warnings += warnings.len();
        json!({
            "devices": recovery.devices.len(),
            "unread": recovery.unread.len(),
            "stages_separating": recovery.separations.len(),
            "warnings": warnings,
        })
    }

    /// The counts, for the report's summary.
    pub(crate) fn summary(&self) -> Value {
        json!({
            "devices": self.devices,
            "drag_coefficients": self.cd,
            "deploy_events": self.deploy_events,
            "devices_changed_per_configuration": self.devices_changed_per_configuration,
            "deployment_overrides": self.deployment_overrides,
            "devices_in_parts_not_read": self.unread,
            "stages": self.stages,
            "stages_stating_a_separation": self.stages_separating,
            "separation_events": self.separation_events,
            "separation_overrides": self.separation_overrides,
            "warnings": self.warnings,
        })
    }

    /// Prints the counts under the rest of the survey.
    pub(crate) fn print(&self) {
        println!(
            "  recovery devices read: {}{}",
            self.devices.values().sum::<usize>(),
            listed(&self.devices, ", ")
        );
        println!(
            "  recovery devices left out, in parts hpr does not read: {}{}",
            self.unread.values().sum::<usize>(),
            listed(&self.unread, ", inside ")
        );
        println!("  drag coefficients:{}", listed(&self.cd, " "));
        println!("  deploy events:{}", listed(&self.deploy_events, " "));
        println!(
            "  devices a configuration changes: {}, with {} change(s) in all",
            self.devices_changed_per_configuration, self.deployment_overrides
        );
        println!(
            "  stage separations: {} of {} stage(s) state one{}; {} change(s) per configuration",
            self.stages_separating,
            self.stages,
            listed(&self.separation_events, ", "),
            self.separation_overrides
        );
        println!("  warnings reading recovery settings: {}", self.warnings);
    }
}

fn kind(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Parachute => "parachute",
        DeviceKind::Streamer => "streamer",
        _ => "other",
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
