//! The motors: each configuration, and each mount's motors, ignition and overhang, written as
//! [`super::super::motors`] reads them.
//!
//! **Configurations.** Each configuration `<rocket>` declared is written back as a
//! `<motorconfiguration configid="…">`, with `default="true"` on the default and its `<name>` when
//! it has one. A configuration only a mount names was not declared, so it is not declared here
//! either: its motor alone brings it back, as it did.
//!
//! **Stage flags.** OpenRocket lists each stage in a configuration as `<stage number="…"
//! active="…">`. The reader reads the number of a stage switched off, `active="false"`, so each of
//! those is written from the design, in its order. The number of a stage that flies it does not
//! read, so the file's own is kept in the design's [`super::super::Extensions`] as an attribute of
//! the flag, by its place; such a flag is written back at that place with `active="true"` alone,
//! and [`super::kept`] puts its number back on it. A design with nothing kept, such as one built
//! in code, gets no such flags: OpenRocket flies every stage a configuration does not switch off.
//!
//! **Mounts.** A mount's `<motor>`s are written in the order of the design's configurations, each
//! followed by its `<ignitionconfiguration>`, as OpenRocket 24.12 writes them. The design keeps
//! when each motor ignites in each configuration, not the mount's own default, so every motor is
//! given an `<ignitionconfiguration>` stating both its event and its delay, and the mount's own
//! `<ignitionevent>` and `<ignitiondelay>` say what all of its motors share, or OpenRocket's
//! default (`automatic`, no delay) when they differ. This is Loft lesson L67's second part: Loft
//! wrote the ignition at the mount alone, so every configuration took the same one.
//!
//! **Delays.** A plugged motor is written `none`, which is what the reader reads as plugged, and
//! never `0`, which is a charge at burnout: Loft lesson L67's first part.
//!
//! **Curves.** A motor names its curve by manufacturer, designation and digest; those are written
//! as read, so reading the file again with the same attachments and supplied curves finds the same
//! curve. Nothing is written about the curve itself.

use std::collections::BTreeSet;

use hpr_design::tree::Component;
use hpr_motor::Delay;

use super::super::Design;
use super::super::document::Element;
use super::super::motors::{Ignition, MotorConfiguration, OrkMotor};
use super::xml::{self, Build as _};

/// The tags the rocket itself carries for its motors: one `<motorconfiguration>` per
/// configuration it declared.
pub(super) fn rocket(design: &Design) -> Vec<Element> {
    design
        .motors
        .configurations
        .iter()
        .filter(|configuration| configuration.declared)
        .enumerate()
        .map(|(index, configuration)| {
            let flying = flying_flags(design, index);
            motor_configuration(configuration, &flying)
        })
        .collect()
}

/// A declared configuration's `<motorconfiguration>`, with a flag that flies at each place in
/// `flying`.
fn motor_configuration(configuration: &MotorConfiguration, flying: &BTreeSet<usize>) -> Element {
    let mut element = xml::element("motorconfiguration");
    element.with_attribute("configid", configuration.id.clone());
    if configuration.default {
        element.with_attribute("default", "true");
    }
    if !configuration.name.is_empty() {
        element.leaf("name", configuration.name.clone());
    }
    for flag in stage_flags(&configuration.inactive_stages, flying) {
        let mut stage = xml::element("stage");
        match flag {
            Flag::Off(number) => {
                if let Some(number) = number {
                    stage.with_attribute("number", number.to_string());
                }
                stage.with_attribute("active", xml::flag(false));
            }
            Flag::On => {
                stage.with_attribute("active", xml::flag(true));
            }
        }
        element.push(stage);
    }
    element
}

/// A `<stage>` flag of a configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    /// A stage switched off, with its number (`None` where the file gave none).
    Off(Option<u32>),
    /// A stage that flies; its number, if it had one, is kept in the design's extensions.
    On,
}

/// The places among the `<stage>` flags of the `index`-th `<motorconfiguration>` where the file
/// had a flag whose number the design keeps: a stage that flies.
fn flying_flags(design: &Design, index: usize) -> BTreeSet<usize> {
    let prefix = format!("openrocket/rocket/@motorconfiguration[{index}]/@stage[");
    design
        .extensions
        .x_openrocket
        .attributes
        .iter()
        .filter(|attribute| attribute.name == "number")
        .filter_map(|attribute| {
            attribute
                .at
                .strip_prefix(&prefix)?
                .strip_suffix(']')?
                .parse()
                .ok()
        })
        .collect()
}

/// The `<stage>` flags of a configuration that switches off `inactive` and had a flag that flies
/// at each place in `flying`.
///
/// Each flag that flies goes back at its place, so the number kept for it goes back on it; the
/// switched-off stages fill the other places in their order, keeping repeats and missing numbers,
/// since the reader keeps all three. A place before the last flying one that neither fills is a
/// flag that left no trace in the design, one that flies with no number: it is written as one.
fn stage_flags(inactive: &[Option<u32>], flying: &BTreeSet<usize>) -> Vec<Flag> {
    let mut off = inactive.iter();
    let last = flying.last().map_or(0, |last| last + 1);
    let mut flags = Vec::with_capacity(inactive.len() + flying.len());
    for place in 0.. {
        if flying.contains(&place) {
            flags.push(Flag::On);
        } else if let Some(&number) = off.next() {
            flags.push(Flag::Off(number));
        } else if place < last {
            flags.push(Flag::On);
        } else {
            break;
        }
    }
    flags
}

/// The `<motormount>` of `component`, if it is a motor mount.
pub(super) fn mount(design: &Design, component: &Component) -> Option<Element> {
    let spec = component.motor_mount.as_ref()?;
    // Each configuration's motor in this mount, in the design's order of configurations; the
    // reader keeps one motor per configuration and mount.
    let motors: Vec<(&str, &OrkMotor)> = design
        .motors
        .configurations
        .iter()
        .filter_map(|configuration| {
            configuration
                .motors
                .iter()
                .find(|motor| motor.mount == component.id)
                .map(|motor| (configuration.id.as_str(), motor))
        })
        .collect();
    let default = shared_ignition(&motors);
    let mut element = xml::element("motormount");
    element
        .leaf("ignitionevent", default.event.as_str())
        .number("ignitiondelay", default.delay_s)
        .number("overhang", spec.overhang_m);
    for &(id, motor) in &motors {
        element.push(self::motor(id, motor));
        let mut ignition = xml::element("ignitionconfiguration");
        ignition
            .with_attribute("configid", id)
            .leaf("ignitionevent", motor.ignition.event.as_str())
            .number("ignitiondelay", motor.ignition.delay_s);
        element.push(ignition);
    }
    Some(element)
}

/// The ignition every one of `motors` has, or OpenRocket's default when they differ or there are
/// none: the mount's own, which each configuration's `<ignitionconfiguration>` then states again.
fn shared_ignition(motors: &[(&str, &OrkMotor)]) -> Ignition {
    match motors.split_first() {
        Some(((_, first), rest)) if rest.iter().all(|(_, m)| m.ignition == first.ignition) => {
            first.ignition.clone()
        }
        _ => Ignition::default(),
    }
}

/// The `<motor>` of configuration `id`, in OpenRocket's order of tags.
fn motor(id: &str, motor: &OrkMotor) -> Element {
    let mut element = xml::element("motor");
    element.with_attribute("configid", id);
    if let Some(kind) = &motor.kind {
        element.leaf("type", kind.clone());
    }
    element.leaf("manufacturer", motor.manufacturer.clone());
    if let Some(digest) = &motor.digest {
        element.leaf("digest", digest.clone());
    }
    element
        .leaf("designation", motor.designation.clone())
        .maybe_number("diameter", motor.diameter_m)
        .maybe_number("length", motor.length_m);
    if let Some(delay) = motor.delay {
        element.leaf("delay", self::delay(delay));
    }
    element
}

/// A `<delay>`'s text: `none` for a plugged motor, the seconds for a charge.
///
/// A `.ork` never reads as [`Delay::ZeroOrPlugged`], which only a RASP `.eng` file's `0` gives; a
/// design holding one is written `none`, plugged, which is what most such files mean
/// (`docs/format/eng.md`), rather than `0`, which a `.ork` reads as a charge at burnout. Any
/// setting that is not a number of seconds is written the same way: a `.ork` has no other word.
fn delay(delay: Delay) -> String {
    match delay {
        Delay::Seconds(seconds) => xml::number(seconds),
        _ => "none".to_owned(),
    }
}

#[cfg(test)]
pub(super) mod tests {
    //! Each hook's tags put in place of the ones the file was read from, and the file read again:
    //! the motors, the recovery and the rocket come back the same. The designs are invented.

    use hpr_design::tree::Component;

    use super::super::super::document::{Document, Element, Node};
    use super::super::super::motors::{CaseSize, Curve, IgnitionEvent, SuppliedCurves};
    use super::super::super::{Design, OrkFile, design_with, read};
    use super::super::recovery;
    use super::*;

    /// The tags a parachute's or streamer's deployment and drag are read from.
    const RECOVERY_TAGS: [&str; 5] = [
        "cd",
        "deployevent",
        "deployaltitude",
        "deploydelay",
        "deploymentconfiguration",
    ];

    /// The tags a stage's separation is read from.
    const SEPARATION_TAGS: [&str; 4] = [
        "separationevent",
        "separationaltitude",
        "separationdelay",
        "separationconfiguration",
    ];

    /// The component with id `id` anywhere in `design`'s rocket.
    pub(in super::super) fn component<'a>(design: &'a Design, id: &str) -> &'a Component {
        find(design, id).expect("the component is in the rocket")
    }

    /// The component with id `id` anywhere in `design`'s rocket, if there is one.
    fn find<'a>(design: &'a Design, id: &str) -> Option<&'a Component> {
        fn among<'a>(components: &'a [Component], id: &str) -> Option<&'a Component> {
            components.iter().find_map(|c| {
                if c.id == id {
                    Some(c)
                } else {
                    among(&c.children, id)
                }
            })
        }
        design
            .rocket
            .stages
            .iter()
            .find_map(|stage| among(&stage.components, id))
    }

    /// The component `element` was read as, by its `<id>`, or `None` for a part the rocket does
    /// not hold, such as one inside a pod, whose tags stay as they were.
    fn read_as<'a>(design: &'a Design, element: &Element) -> Option<&'a Component> {
        find(design, element.child("id")?.text().trim())
    }

    /// Takes `element`'s children called one of `names` out, and returns where the first of them
    /// was (the end when there was none).
    fn without(element: &mut Element, names: &[&str]) -> usize {
        let named =
            |child: &Node| matches!(child, Node::Element(e) if names.contains(&e.name.as_str()));
        let first = element.children.iter().position(named);
        element.children.retain(|child| !named(child));
        first.unwrap_or(element.children.len())
    }

    /// Puts `tags` into `element` at child `at`.
    fn insert(element: &mut Element, at: usize, tags: Vec<Element>) {
        element
            .children
            .splice(at..at, tags.into_iter().map(Node::Element));
    }

    /// Replaces, under `element`, every tag the motors and recovery readers read with what the
    /// hooks write for `design`. `stage` counts the stages met so far.
    fn splice(design: &Design, element: &mut Element, stage: &mut usize) {
        match element.name.as_str() {
            "rocket" => {
                let at = without(element, &["motorconfiguration"]);
                insert(element, at, rocket(design));
            }
            "stage" => {
                let index = *stage;
                *stage += 1;
                let written = design
                    .rocket
                    .stages
                    .get(index)
                    .map(|s| recovery::stage(design, index, s))
                    .unwrap_or_default();
                let at = without(element, &SEPARATION_TAGS);
                insert(element, at, written);
            }
            "parachute" | "streamer" => {
                if let Some(part) = read_as(design, element) {
                    let written = recovery::device(design, part);
                    let at = without(element, &RECOVERY_TAGS);
                    insert(element, at, written);
                }
            }
            _ => {}
        }
        if element.child("motormount").is_some()
            && let Some(part) = read_as(design, element)
        {
            let written = mount(design, part);
            let at = without(element, &["motormount"]);
            insert(element, at, written.into_iter().collect());
        }
        for child in &mut element.children {
            // A configuration's `<stage>` flags are not stages.
            if let Node::Element(child) = child
                && child.name != "motorconfiguration"
            {
                splice(design, child, stage);
            }
        }
    }

    /// Puts each stage flag's kept number back on the flag at its place, as the export's splicing
    /// of what was kept does. Everything else kept is still in the document, which the hooks' tags
    /// are spliced into rather than written from nothing.
    fn numbers_back(design: &Design, root: &mut Element) {
        for attribute in &design.extensions.x_openrocket.attributes {
            let Some((configuration, flag)) = attribute
                .at
                .strip_prefix("openrocket/rocket/@motorconfiguration[")
                .and_then(|rest| rest.split_once("]/@stage["))
            else {
                continue;
            };
            let (Ok(configuration), Some(Ok(flag))) = (
                configuration.parse::<usize>(),
                flag.strip_suffix(']').map(str::parse::<usize>),
            ) else {
                continue;
            };
            let on = nth(root, "rocket", 0)
                .and_then(|rocket| nth(rocket, "motorconfiguration", configuration))
                .and_then(|element| nth(element, "stage", flag));
            if let Some(on) = on
                && on
                    .attributes
                    .iter()
                    .all(|(name, _)| *name != attribute.name)
            {
                on.with_attribute(&attribute.name, attribute.value.clone());
            }
        }
    }

    /// The `n`-th child of `element` called `name`.
    fn nth<'a>(element: &'a mut Element, name: &str, n: usize) -> Option<&'a mut Element> {
        element
            .children
            .iter_mut()
            .filter_map(|child| match child {
                Node::Element(e) if e.name == name => Some(e),
                _ => None,
            })
            .nth(n)
    }

    /// Reads `xml` with `supplied`, puts every hook's tags in place of the file's own, and reads
    /// it again; returns both designs and the spliced file's text.
    pub(in super::super) fn round_trip(
        xml: &str,
        supplied: &SuppliedCurves,
    ) -> (Design, Design, String) {
        let mut file: OrkFile = read(xml.as_bytes()).expect("a readable design").value;
        let original = design_with(&file, supplied).value;
        splice(&original, &mut file.document.root, &mut 0);
        numbers_back(&original, &mut file.document.root);
        let text = file.document.to_xml();
        let again = design_with(&read(text.as_bytes()).expect("readable").value, supplied).value;
        (original, again, text)
    }

    /// `element` as XML text, to look for what was written.
    pub(in super::super) fn text(element: &Element) -> String {
        let mut root = xml::element("x");
        root.push(element.clone());
        let whole = Document {
            version: super::super::SCHEMA,
            creator: None,
            root,
        }
        .to_xml();
        let start = whole.find("<x version=\"1.10\">").expect("the wrapper") + 18;
        let end = whole.rfind("</x>").expect("the wrapper's end");
        whole[start..end].trim().to_owned()
    }

    /// Asserts `again` is `original`, naming the first line of their JSON that differs.
    pub(in super::super) fn assert_same<T: serde::Serialize + PartialEq>(original: &T, again: &T) {
        if original == again {
            return;
        }
        let json = |value: &T| serde_json::to_string_pretty(value).expect("serializable");
        let (was, is) = (json(original), json(again));
        let line = was
            .lines()
            .zip(is.lines())
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(n, (a, b))| format!("line {n}: `{a}` became `{b}`"))
            .unwrap_or_else(|| "a different length".to_owned());
        panic!("read back differently at {line}");
    }

    /// A synthetic motor no catalog lists, for a curve supplied by digest.
    fn synthetic() -> hpr_motor::SolidMotor {
        let thrust =
            hpr_motor::ThrustCurve::new(vec![0.0, 0.05, 1.0, 1.1], vec![0.0, 80.0, 80.0, 0.0])
                .expect("a valid curve");
        hpr_motor::SolidMotor::from_envelope(thrust, 0.029, 0.124, 0.05, 0.14)
            .expect("a valid motor")
    }

    /// An Estes F15 for configuration `config`, with `delay` as its `<delay>`.
    fn f15(config: &str, delay: &str) -> String {
        format!(
            "<motor configid='{config}'><type>single</type><manufacturer>Estes</manufacturer>\
             <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>\
             <delay>{delay}</delay></motor>"
        )
    }

    /// An invented two-stage design: a sustainer mount with a motor in five configurations, each
    /// lit its own way, one of them declared nowhere, one flying a supplied curve and one with no
    /// curve at all; a booster with a clustered mount and a mount in its body tube; a parachute
    /// with its drag left to OpenRocket and its deployment changed for two configurations; a
    /// streamer with a stated drag; and a booster that separates one way by default and two
    /// others per configuration.
    pub(in super::super) fn two_stage() -> String {
        let sustainer_mount = format!(
            "<ignitionevent>ejectioncharge</ignitionevent><ignitiondelay>0.25</ignitiondelay>\
             <overhang>0.0125</overhang>{}\
             <ignitionconfiguration configid='a'><ignitionevent>burnout</ignitionevent>\
             </ignitionconfiguration>{}\
             <ignitionconfiguration configid='b'><ignitionevent>launch</ignitionevent>\
             <ignitiondelay>1.5</ignitiondelay></ignitionconfiguration>{}\
             <motor configid='d'><type>reload</type><manufacturer>Nobody</manufacturer>\
             <digest>0a1b2c3d</digest><designation>G80X</designation><diameter>0.029</diameter>\
             <length>0.124</length><delay>7</delay></motor>\
             <motor configid='e'><manufacturer>Nobody</manufacturer><digest>feedface</digest>\
             <designation>Z9</designation><delay>none</delay></motor>",
            f15("a", "none"),
            f15("b", "4"),
            f15("c", "0.0"),
        );
        let booster_cluster = format!(
            "<ignitionevent>launch</ignitionevent><overhang>0.003</overhang>{}{}",
            f15("a", "3"),
            f15("b", "none")
        );
        let booster_body = format!(
            "<overhang>0.0</overhang><ignitionevent>never</ignitionevent>{}",
            f15("c", "none")
        );
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Invented</name>
    <motorconfiguration configid="a" default="true"><name>Pair</name>
      <stage number="0" active="true"/><stage number="1" active="true"/></motorconfiguration>
    <motorconfiguration configid="b"/>
    <motorconfiguration configid="c"><stage number="1" active="false"/></motorconfiguration>
    <motorconfiguration configid="e"><name>Unknown motor</name></motorconfiguration>
    <subcomponents>
      <stage><name>Sustainer</name><id>upper</id><subcomponents>
        <nosecone><name>Nose</name><id>nose</id>
          <material type="bulk" density="1000.0">Plastic</material>
          <length>0.15</length><thickness>0.002</thickness><shape>ogive</shape>
          <aftradius>0.02</aftradius></nosecone>
        <bodytube><name>Body</name><id>upper-body</id>
          <material type="bulk" density="680.0">Cardboard</material>
          <length>0.5</length><thickness>0.001</thickness><radius>0.02</radius>
          <subcomponents>
            <innertube><name>Mount</name><id>upper-mmt</id>
              <material type='bulk' density='680.0'>Cardboard</material>
              <axialoffset method='bottom'>0.0</axialoffset><length>0.2</length>
              <outerradius>0.0152</outerradius><thickness>0.0005</thickness>
              <motormount>{sustainer_mount}</motormount></innertube>
            <parachute><name>Main</name><id>main</id>
              <axialoffset method='top'>0.05</axialoffset><packedlength>0.05</packedlength>
              <packedradius>0.01</packedradius><cd>auto</cd>
              <material type='surface' density='0.067'>Ripstop nylon</material>
              <deployevent>apogee</deployevent><deploydelay>1.0</deploydelay>
              <deploymentconfiguration configid='b'><deployevent>altitude</deployevent>
                <deployaltitude>152.4</deployaltitude></deploymentconfiguration>
              <deploymentconfiguration configid='a'><deploydelay>2</deploydelay>
                </deploymentconfiguration>
              <diameter>0.6</diameter><linecount>6</linecount><linelength>0.5</linelength>
              <linematerial type='line' density='0.0018'>Elastic cord</linematerial></parachute>
            <streamer><name>Streamer</name><id>streamer</id>
              <axialoffset method='top'>0.1</axialoffset><packedlength>0.04</packedlength>
              <packedradius>0.01</packedradius><cd>0.45</cd>
              <material type='surface' density='0.067'>Ripstop nylon</material>
              <deployevent>ejection</deployevent><deployaltitude>200.0</deployaltitude>
              <deploydelay>0.0</deploydelay>
              <striplength>1.2</striplength><stripwidth>0.08</stripwidth></streamer>
          </subcomponents></bodytube>
      </subcomponents></stage>
      <stage><name>Booster</name><id>lower</id>
        <separationevent>burnout</separationevent><separationdelay>0.25</separationdelay>
        <separationconfiguration configid="a"><separationevent>ejection</separationevent>
          </separationconfiguration>
        <separationconfiguration configid="c"><separationevent>altitudeascending</separationevent>
          <separationaltitude>300.5</separationaltitude></separationconfiguration>
        <subcomponents>
        <bodytube><name>Booster body</name><id>lower-body</id>
          <material type="bulk" density="680.0">Cardboard</material>
          <length>0.3</length><thickness>0.001</thickness><radius>0.02</radius>
          <motormount>{booster_body}</motormount>
          <subcomponents>
            <innertube><name>Cluster</name><id>cluster-mmt</id>
              <material type='bulk' density='680.0'>Cardboard</material>
              <axialoffset method='bottom'>0.0</axialoffset><length>0.2</length>
              <outerradius>0.0152</outerradius><thickness>0.0005</thickness>
              <clusterconfiguration>3-ring</clusterconfiguration>
              <motormount>{booster_cluster}</motormount></innertube>
          </subcomponents></bodytube>
      </subcomponents></stage>
    </subcomponents></rocket>
</openrocket>"#
        )
    }

    /// The curves supplied for the invented design: one for the `G80X`'s digest.
    pub(in super::super) fn supplied() -> SuppliedCurves {
        let mut supplied = SuppliedCurves::new("the test's curves");
        supplied
            .insert(
                "0a1b2c3d",
                CaseSize {
                    diameter_m: 0.029,
                    length_m: 0.124,
                },
                synthetic(),
            )
            .expect("a valid case");
        supplied
    }

    /// Every configuration, each mount's motors, their ignitions per configuration, a plugged
    /// delay, a charge at burnout, a configuration only a mount names, a switched-off stage, a
    /// cluster, a motor with no curve and one whose curve its digest supplies all read back the
    /// same from what the hooks write.
    #[test]
    fn the_motors_read_back_the_same() {
        let (original, again, _) = round_trip(&two_stage(), &supplied());
        let ids: Vec<&str> = original
            .motors
            .configurations
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(ids, ["a", "b", "c", "e", "d"], "the design is as meant");
        assert!(
            original.motors.configurations[4]
                .motors
                .iter()
                .any(|m| matches!(m.curve, Curve::Supplied { .. })),
            "the supplied curve is used"
        );
        let flown: Vec<&str> = original
            .rocket
            .configurations
            .iter()
            .map(|c| c.id.as_str())
            .collect();
        // Two configurations fly, so the rocket's own configurations are compared too.
        assert_eq!(flown, ["a", "b"]);
        assert_same(&original.motors, &again.motors);
        assert_same(&original.rocket, &again.rocket);
        assert_same(&original.recovery, &again.recovery);
        // The numbers of the stages that fly were kept, and go back on the flags written for them.
        assert_same(&original.extensions, &again.extensions);
        assert_same(&original, &again);
    }

    /// Loft lesson L67: a plugged motor is written `none`, not `0`; each configuration's ignition
    /// is written in that configuration; and the mount's own default is what its motors share, or
    /// OpenRocket's when they differ.
    #[test]
    fn plugged_delays_and_ignitions_are_written_per_configuration() {
        let file = read(two_stage().as_bytes()).expect("readable").value;
        let design = design_with(&file, &supplied()).value;
        let upper = text(&mount(&design, component(&design, "upper-mmt")).expect("a mount"));
        let compact: String = upper.split_whitespace().collect();
        assert!(compact.contains("<delay>none</delay>"), "{upper}");
        assert!(compact.contains("<delay>0</delay>"), "{upper}");
        for (config, event, delay) in [
            ("a", "burnout", "0.25"),
            ("b", "launch", "1.5"),
            ("c", "ejectioncharge", "0.25"),
            ("d", "ejectioncharge", "0.25"),
        ] {
            let written = format!(
                "<ignitionconfigurationconfigid=\"{config}\">\
                 <ignitionevent>{event}</ignitionevent><ignitiondelay>{delay}</ignitiondelay>\
                 </ignitionconfiguration>"
            );
            assert!(compact.contains(&written), "{config}: {upper}");
        }
        assert!(
            compact.contains("<ignitionevent>automatic</ignitionevent>"),
            "{upper}"
        );

        // Every motor in the booster's body lights the same way, so that is the mount's own.
        let body = text(&mount(&design, component(&design, "lower-body")).expect("a mount"));
        assert!(
            body.contains("<ignitionevent>never</ignitionevent>"),
            "{body}"
        );
        // A tube that holds no motor writes no mount.
        assert!(mount(&design, component(&design, "upper-body")).is_none());
    }

    /// The stage flags put each flying one back at its place and fill the others with what was
    /// switched off, in its order and with its gaps; a place before the last flying one that
    /// neither fills is a flag that flies.
    #[test]
    fn stage_flags_keep_every_place() {
        use Flag::{Off, On};
        let places = |places: &[usize]| places.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(
            stage_flags(&[Some(1)], &places(&[0, 2])),
            [On, Off(Some(1)), On]
        );
        assert_eq!(
            stage_flags(&[Some(2), None, Some(0), Some(2)], &places(&[1])),
            [Off(Some(2)), On, Off(None), Off(Some(0)), Off(Some(2))]
        );
        assert_eq!(
            stage_flags(&[Some(0)], &places(&[3])),
            [Off(Some(0)), On, On, On]
        );
        assert_eq!(stage_flags(&[], &places(&[])), []);
    }

    /// A switched-off stage with no number, repeated numbers and an ignition event hpr does not
    /// know all read back as they were read.
    #[test]
    fn unusual_flags_and_events_are_written_as_read() {
        let xml = two_stage()
            .replace(
                r#"<stage number="1" active="false"/>"#,
                r#"<stage number="1" active="false"/><stage active="false"/>
                   <stage number="1" active="false"/><stage number="x" active="false"/>"#,
            )
            .replace(
                "<ignitionevent>burnout</ignitionevent>",
                "<ignitionevent>whenever</ignitionevent>",
            );
        let (original, again, _) = round_trip(&xml, &supplied());
        assert_eq!(
            original.motors.configurations[0].motors[0].ignition.event,
            IgnitionEvent::Other("whenever".to_owned())
        );
        assert_eq!(
            original.motors.configurations[2].inactive_stages,
            [Some(1), None, Some(1), None]
        );
        assert_same(&original.motors, &again.motors);
        assert_same(&original.rocket, &again.rocket);
    }
}
