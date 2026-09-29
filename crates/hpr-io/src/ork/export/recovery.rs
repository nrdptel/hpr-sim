//! When recovery devices open and stages separate, written as [`super::super::recovery`] reads
//! them.
//!
//! A device's or stage's own setting is written as its three tags, each only when the design
//! states it. Each configuration that changes it is written as a `<deploymentconfiguration>` or
//! `<separationconfiguration>` stating every one of the three the design holds for it. The design
//! keeps a configuration's setting with what it left out already taken from the device's or
//! stage's own, so stating all three reads back the same: the reader takes each one the
//! configuration states. The configurations are written in the order of the design's motor
//! configurations, as OpenRocket writes them, then any others by id.

use std::collections::BTreeMap;

use hpr_design::tree::{Component, Stage};

use super::super::Design;
use super::super::document::Element;
use super::super::recovery::{DeployEvent, EventSetting, SeparationEvent};
use super::xml::{self, Build as _};

/// The tags a parachute or streamer carries for when it opens, and its drag coefficient.
pub(super) fn device(design: &Design, component: &Component) -> Vec<Element> {
    let Some(device) = design
        .recovery
        .devices
        .iter()
        .find(|device| device.id == component.id)
    else {
        return Vec::new();
    };
    let mut tags = Vec::new();
    if let Some(cd) = device.cd {
        tags.push(xml::tag("cd", xml::dimension(cd)));
    }
    tags.extend(trigger(
        design,
        "deploy",
        "deploymentconfiguration",
        &device.deployment,
        &device.configurations,
        DeployEvent::as_str,
    ));
    tags
}

/// The tags stage `index` carries for when it separates.
pub(super) fn stage(design: &Design, index: usize, stage: &Stage) -> Vec<Element> {
    let Some(separation) = design
        .recovery
        .separations
        .iter()
        .find(|separation| separation.stage == index && separation.id == stage.id)
    else {
        return Vec::new();
    };
    trigger(
        design,
        "separation",
        "separationconfiguration",
        &separation.separation,
        &separation.configurations,
        SeparationEvent::as_str,
    )
}

/// `<{prefix}event>`, `<{prefix}altitude>` and `<{prefix}delay>` for `own`, then a `<{per}>` for
/// each of `configurations`.
fn trigger<E>(
    design: &Design,
    prefix: &str,
    per: &str,
    own: &EventSetting<E>,
    configurations: &BTreeMap<String, EventSetting<E>>,
    word: fn(&E) -> &str,
) -> Vec<Element> {
    let mut tags = setting(prefix, own, word);
    for (id, over) in in_order(design, configurations) {
        let mut element = xml::element(per);
        element.with_attribute("configid", id);
        for tag in setting(prefix, over, word) {
            element.push(tag);
        }
        tags.push(element);
    }
    tags
}

/// The tags stating each of `setting`'s event, height and delay that it holds.
fn setting<E>(prefix: &str, setting: &EventSetting<E>, word: fn(&E) -> &str) -> Vec<Element> {
    let mut tags = Vec::new();
    if let Some(event) = &setting.event {
        tags.push(xml::tag(&format!("{prefix}event"), word(event)));
    }
    if let Some(altitude_m) = setting.altitude_m {
        tags.push(xml::tag(
            &format!("{prefix}altitude"),
            xml::number(altitude_m),
        ));
    }
    if let Some(delay_s) = setting.delay_s {
        tags.push(xml::tag(&format!("{prefix}delay"), xml::number(delay_s)));
    }
    tags
}

/// `configurations` in the order of the design's motor configurations, then any the design has no
/// motor configuration for, by id.
fn in_order<'a, T>(
    design: &Design,
    configurations: &'a BTreeMap<String, T>,
) -> Vec<(&'a str, &'a T)> {
    let mut ordered: Vec<(&str, &T)> = design
        .motors
        .configurations
        .iter()
        .filter_map(|configuration| configurations.get_key_value(&configuration.id))
        .map(|(id, value)| (id.as_str(), value))
        .collect();
    for (id, value) in configurations {
        if !ordered.iter().any(|(placed, _)| *placed == id) {
            ordered.push((id.as_str(), value));
        }
    }
    ordered
}

#[cfg(test)]
mod tests {
    //! Each device's and stage's tags put in place of the file's own, and read again. The design is
    //! the motors tests' invented one.

    use super::super::super::recovery::{DeployEvent, SeparationEvent};
    use super::super::super::value::Dimension;
    use super::super::super::{design_with, read};
    use super::super::motors::tests::{
        assert_same, component, round_trip, supplied, text, two_stage,
    };
    use super::*;

    /// A parachute with its drag left to OpenRocket, a streamer with its drag stated, a
    /// deployment changed per configuration, and a booster separating one way by default and
    /// two others per configuration all read back the same.
    #[test]
    fn deployments_and_separations_read_back_the_same() {
        let (original, again, _) = round_trip(&two_stage(), &supplied());
        let recovery = &original.recovery;
        let main = &recovery.devices[0];
        assert_eq!(main.cd, Some(Dimension::Automatic { cached: None }));
        assert_eq!(main.configurations.len(), 2, "the design is as meant");
        assert_eq!(
            recovery.devices[1].cd,
            Some(Dimension::Stated { value: 0.45 })
        );
        assert_eq!(recovery.separations[0].configurations.len(), 2);
        assert_same(&original.recovery, &again.recovery);
        assert_same(&original.motors, &again.motors);
        assert_same(&original.rocket, &again.rocket);
    }

    /// Each configuration states all three settings it has, the ones it took from the device
    /// included, in the order of the motor configurations, with OpenRocket's words.
    #[test]
    fn each_configuration_states_its_whole_deployment() {
        let file = read(two_stage().as_bytes()).expect("readable").value;
        let design = design_with(&file, &supplied()).value;
        let main = device(&design, component(&design, "main"));
        let written: String = main.iter().map(text).collect();
        let compact: String = written.split_whitespace().collect();
        assert!(
            compact.contains(
                "<cd>auto</cd><deployevent>apogee</deployevent><deploydelay>1</deploydelay>\
                 <deploymentconfigurationconfigid=\"a\"><deployevent>apogee</deployevent>\
                 <deploydelay>2</deploydelay></deploymentconfiguration>\
                 <deploymentconfigurationconfigid=\"b\"><deployevent>altitude</deployevent>\
                 <deployaltitude>152.4</deployaltitude><deploydelay>1</deploydelay>\
                 </deploymentconfiguration>"
            ),
            "{written}"
        );
        assert_eq!(
            design.recovery.devices[0].configurations["b"].event,
            Some(DeployEvent::Altitude)
        );

        let booster = &design.rocket.stages[1];
        let separation: String = stage(&design, 1, booster).iter().map(text).collect();
        let compact: String = separation.split_whitespace().collect();
        assert!(
            compact.contains(
                "<separationevent>burnout</separationevent><separationdelay>0.25</separationdelay>\
                 <separationconfigurationconfigid=\"a\"><separationevent>ejection</separationevent>\
                 <separationdelay>0.25</separationdelay></separationconfiguration>\
                 <separationconfigurationconfigid=\"c\">\
                 <separationevent>altitudeascending</separationevent>\
                 <separationaltitude>300.5</separationaltitude>\
                 <separationdelay>0.25</separationdelay></separationconfiguration>"
            ),
            "{separation}"
        );
        assert_eq!(
            design.recovery.separations[0].configurations["c"].event,
            Some(SeparationEvent::AltitudeAscending)
        );
        // The sustainer states no separation, and a stage asked for by the wrong index gets none.
        assert!(stage(&design, 0, &design.rocket.stages[0]).is_empty());
        assert!(stage(&design, 0, booster).is_empty());
        // A part that is not a recovery device carries none of these tags.
        assert!(device(&design, component(&design, "nose")).is_empty());
    }

    /// A drag coefficient `auto` with the number OpenRocket last worked out, and no `cd` at all,
    /// both read back as they were.
    #[test]
    fn a_cached_and_a_missing_drag_coefficient_read_back() {
        let xml = two_stage()
            .replace("<cd>auto</cd>", "<cd>auto 0.8125</cd>")
            .replace("<cd>0.45</cd>", "");
        let (original, again, _) = round_trip(&xml, &supplied());
        assert_eq!(
            original.recovery.devices[0].cd,
            Some(Dimension::Automatic {
                cached: Some(0.8125)
            })
        );
        assert_eq!(original.recovery.devices[1].cd, None);
        assert_same(&original.recovery, &again.recovery);
    }
}
