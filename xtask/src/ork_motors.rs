//! The motor counts `cargo xtask ork` prints (M3.1c1): the configurations each design declares,
//! the motors its mounts hold, where each motor's thrust curve came from, and which
//! configurations the rocket flies. Kept apart from `ork.rs`, whose report is long enough.

use std::collections::BTreeMap;

use hpr_design::{Component, Rocket};
use hpr_io::ork::{Curve, Design, Imported, NoCurve, NotFlown, WarningKind};
use hpr_motor::Delay;
use serde_json::{Value, json};

/// The counts, summed over the designs.
#[derive(Debug, Default)]
pub(crate) struct MotorTally {
    /// Designs with at least one configuration.
    designs_configured: usize,
    /// Designs with at least one configuration the rocket flies.
    designs_flying: usize,
    mounts: usize,
    configurations: usize,
    undeclared: usize,
    motors: usize,
    kinds: BTreeMap<String, usize>,
    unread: BTreeMap<String, usize>,
    curves: BTreeMap<String, usize>,
    delays: BTreeMap<&'static str, usize>,
    ignitions: BTreeMap<String, usize>,
    flown: usize,
    left_out: BTreeMap<String, usize>,
    assembled: usize,
    not_assembled: BTreeMap<String, usize>,
    warnings: BTreeMap<&'static str, usize>,
}

impl MotorTally {
    /// Counts one design, whose reading raised `spine_warnings` warnings before its motors were
    /// read, and returns its per-file detail.
    pub(crate) fn add(&mut self, design: &Imported<Design>, spine_warnings: usize) -> Value {
        let motors = &design.value.motors;
        let rocket = &design.value.rocket;
        self.mounts += mounts(rocket);
        self.configurations += motors.configurations.len();
        if !motors.configurations.is_empty() {
            self.designs_configured += 1;
        }
        if !rocket.configurations.is_empty() {
            self.designs_flying += 1;
        }
        let mut left_out = Vec::new();
        let mut unresolved = Vec::new();
        for configuration in &motors.configurations {
            if !configuration.declared {
                self.undeclared += 1;
            }
            for motor in &configuration.motors {
                self.motors += 1;
                *self
                    .kinds
                    .entry(
                        motor
                            .kind
                            .clone()
                            .unwrap_or_else(|| "not written".to_owned()),
                    )
                    .or_default() += 1;
                let curve = match &motor.curve {
                    Curve::Embedded { .. } => "embedded".to_owned(),
                    Curve::Supplied { .. } => "OpenRocket's database, by digest".to_owned(),
                    Curve::Catalog { .. } => "bundled catalog".to_owned(),
                    Curve::Unresolved { why, reason } => {
                        unresolved.push(json!({
                            "configuration": configuration.id,
                            "motor": format!("{} {}", motor.manufacturer, motor.designation),
                            "reason": reason,
                        }));
                        format!("none: {}", no_curve(*why))
                    }
                    _ => "other".to_owned(),
                };
                *self.curves.entry(curve).or_default() += 1;
                let delay = match motor.delay {
                    None => "not written",
                    Some(Delay::Plugged) => "plugged (`none`)",
                    Some(Delay::Seconds(0.0)) => "0 s",
                    Some(Delay::Seconds(_)) => "seconds",
                    Some(_) => "other",
                };
                *self.delays.entry(delay).or_default() += 1;
                let ignition = if motor.ignition.delay_s == 0.0 {
                    motor.ignition.event.as_str().to_owned()
                } else {
                    format!("{} after a delay", motor.ignition.event.as_str())
                };
                *self.ignitions.entry(ignition).or_default() += 1;
            }
            for motor in &configuration.unread {
                *self.unread.entry(motor.inside.clone()).or_default() += 1;
            }
            match &configuration.left_out {
                None => self.flown += 1,
                Some(out) => {
                    *self
                        .left_out
                        .entry(not_flown(out.why).to_owned())
                        .or_default() += 1;
                    left_out.push(json!({
                        "configuration": configuration.id,
                        "why": not_flown(out.why),
                        "says": out.message,
                    }));
                }
            }
        }
        let mut assembly_errors = Vec::new();
        for configuration in &rocket.configurations {
            match rocket.assemble(&configuration.id) {
                Ok(_) => self.assembled += 1,
                Err(error) => {
                    let text = error.to_string();
                    *self.not_assembled.entry(text.clone()).or_default() += 1;
                    assembly_errors
                        .push(json!({ "configuration": configuration.id, "error": text }));
                }
            }
        }
        // The motors' warnings follow the spine's; a stored simulation's come after them, and are
        // counted with the simulations.
        let motor_warnings: Vec<_> = design.warnings[spine_warnings.min(design.warnings.len())..]
            .iter()
            .filter(|warning| !warning.at.starts_with("openrocket/simulations"))
            .collect();
        for warning in &motor_warnings {
            *self.warnings.entry(kind_name(warning.kind)).or_default() += 1;
        }
        json!({
            "configurations": motors.configurations.len(),
            "flown": rocket.configurations.iter().map(|c| c.id.clone()).collect::<Vec<_>>(),
            "left_out": left_out,
            "unresolved": unresolved,
            "not_assembled": assembly_errors,
            "warnings": motor_warnings.iter()
                .map(|warning| json!({ "at": warning.at, "says": warning.message }))
                .collect::<Vec<_>>(),
        })
    }

    /// The counts, for the report's summary.
    pub(crate) fn summary(&self) -> Value {
        json!({
            "designs_with_a_configuration": self.designs_configured,
            "designs_flying_a_configuration": self.designs_flying,
            "motor_mounts": self.mounts,
            "configurations": self.configurations,
            "configurations_only_a_mount_names": self.undeclared,
            "motors": self.motors,
            "motor_types": self.kinds,
            "motors_in_parts_not_read": self.unread,
            "curves": self.curves,
            "delays": self.delays,
            "ignitions": self.ignitions,
            "configurations_flown": self.flown,
            "configurations_left_out": self.left_out,
            "flown_configurations_assembled": self.assembled,
            "flown_configurations_not_assembled": self.not_assembled,
            "motor_warnings": self.warnings,
        })
    }

    /// Prints the counts under the rest of the survey.
    pub(crate) fn print(&self) {
        println!(
            "  motor configurations: {} in {} design(s) ({} named only by a mount), over {} \
             motor mount(s)",
            self.configurations, self.designs_configured, self.undeclared, self.mounts
        );
        println!(
            "  motors read into their configurations: {}{}",
            self.motors,
            listed(&self.kinds, ", by type: ")
        );
        println!(
            "  motors left out, in parts hpr does not read: {}{}",
            self.unread.values().sum::<usize>(),
            listed(&self.unread, ", inside ")
        );
        println!("  thrust curves:{}", listed(&self.curves, " "));
        println!("  ejection delays:{}", listed(&self.delays, " "));
        println!("  ignition:{}", listed(&self.ignitions, " "));
        println!(
            "  configurations the rocket flies: {} of {}, in {} design(s); {} of those assemble{}",
            self.flown,
            self.configurations,
            self.designs_flying,
            self.assembled,
            listed(&self.not_assembled, "; not assembling: ")
        );
        println!(
            "  configurations left out, by first reason:{}",
            listed(&self.left_out, " ")
        );
        println!("  warnings reading motors:{}", listed(&self.warnings, " "));
    }

    /// Why the survey should fail: a configuration the rocket flies that does not assemble.
    pub(crate) fn failure(&self) -> Option<String> {
        let failed = self.not_assembled.values().sum::<usize>();
        (failed > 0).then(|| {
            format!(
                "{failed} configuration(s) the importer passed as flyable do not assemble: {}",
                self.not_assembled
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })
    }
}

/// How many components in `rocket` are motor mounts.
fn mounts(rocket: &Rocket) -> usize {
    fn count(components: &[Component]) -> usize {
        components
            .iter()
            .map(|c| usize::from(c.motor_mount.is_some()) + count(&c.children))
            .sum()
    }
    rocket.stages.iter().map(|s| count(&s.components)).sum()
}

/// `counts` as `key n, key n`, after `lead`; nothing at all when empty.
fn listed<K: std::fmt::Display>(counts: &BTreeMap<K, usize>, lead: &str) -> String {
    if counts.is_empty() {
        return if lead.trim().is_empty() {
            " none".to_owned()
        } else {
            String::new()
        };
    }
    let line = counts
        .iter()
        .map(|(key, count)| format!("{key} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{lead}{line}")
}

pub(crate) fn no_curve(why: NoCurve) -> &'static str {
    match why {
        NoCurve::Hybrid => "a hybrid",
        NoCurve::NoDesignation => "no designation",
        NoCurve::NotFound => "not embedded, supplied or in the bundled catalog",
        NoCurve::Ambiguous => "ambiguous in the catalog",
        NoCurve::Unusable => "a curve that could not be used",
        _ => "other",
    }
}

pub(crate) fn not_flown(why: NotFlown) -> &'static str {
    match why {
        NotFlown::UnreadMotor => "a motor in a part not read",
        NotFlown::NoMotor => "no motor",
        NotFlown::InactiveStage => "a stage switched off",
        NotFlown::NoCurve => "a motor with no curve",
        NotFlown::NoSize => "a motor with no size",
        NotFlown::IgnitionNotFlown => "a motor hpr can't light as written",
        NotFlown::AirframeNotAsWritten => "an airframe not read exactly as written",
        NotFlown::SeparationNotFlown => "stages hpr can't separate as written",
        _ => "other",
    }
}

fn kind_name(kind: WarningKind) -> &'static str {
    match kind {
        WarningKind::Skipped => "skipped",
        WarningKind::Dropped => "dropped",
        WarningKind::Unusual => "unusual",
        _ => "other",
    }
}
