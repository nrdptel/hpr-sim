//! When a `.ork` design's parachutes and streamers open, how much drag each states, and when each
//! stage separates.
//!
//! **Where a `.ork` keeps them.** A `<parachute>` or `<streamer>` names the event that deploys it
//! (`<deployevent>`), a height for the event that needs one (`<deployaltitude>`) and a delay after
//! it (`<deploydelay>`), and may change any of the three for one configuration in a
//! `<deploymentconfiguration configid="…">`. A stage says when it separates the same way, with
//! `<separationevent>`, `<separationaltitude>`, `<separationdelay>` and
//! `<separationconfiguration configid="…">` ([the file specification][spec]). A per-configuration
//! setting replaces the three one at a time: whichever it leaves out, the device's or stage's own
//! stands.
//!
//! **What the words mean.** OpenRocket's documentation lists none of them. The words here are the
//! ones OpenRocket 24.12 writes, each measured by setting it through the program's public setters
//! and saving (`validation/oracles/openrocket/events.py`, whose results
//! `validation/fixtures/ork/openrocket-events.json` holds and a test reads). The same run shows a
//! deploy height is above the ground, not the sea, and that in its run a parachute set to open
//! above apogee did not open at all.
//!
//! A per-configuration setting replacing the three one at a time is hpr's reading: OpenRocket was
//! not probed on a file that leaves one out.
//!
//! **Drag.** `<cd>auto</cd>` leaves the drag coefficient to OpenRocket: 0.8 for a parachute, on the
//! canopy's area, which its technical documentation gives as the default (section 4.2.5) and the
//! same run reads back, and for a streamer a value from the strip's length and material, on the
//! strip's area (the documentation's appendix C). This reader keeps the word, and the number when
//! one is stated; choosing the model is the flight's business.
//!
//! Nothing here flies a device: that is `hpr-sim`'s, which `hpr-io` does not depend on.
//!
//! [spec]: https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html

use std::collections::{BTreeMap, BTreeSet};

use hpr_design::Rocket;
use serde::{Deserialize, Serialize};

use super::component::subcomponents;
use super::document::Element;
use super::motors::stage_of;
use super::value::{Dimension, Values};
use super::warning::{Warning, WarningKind};

/// The path segment every warning about when a device deploys carries, so that
/// [`super::design`] can tell it from one about the airframe: recovery is read, not flown.
pub(super) const DEPLOYMENT: &str = "/deployment";
/// The same, for a device's drag coefficient.
pub(super) const DRAG: &str = "/drag";
/// The same, for a stage's separation.
pub(super) const SEPARATION: &str = "/separation";

/// What deploys a recovery device, as `<deployevent>` names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeployEvent {
    /// `launch`: at launch, plus the delay.
    Launch,
    /// `ejection`: the first ejection charge of the device's own stage.
    Ejection,
    /// `apogee`.
    Apogee,
    /// `altitude`: the height `<deployaltitude>` gives, above the ground, on the way down.
    Altitude,
    /// `lowerstageseparation`: the separation of the stage below.
    LowerStageSeparation,
    /// `never`.
    Never,
    /// A word not listed here, kept as written.
    Other(String),
}

impl DeployEvent {
    /// Reads a `<deployevent>`'s text.
    pub fn parse(text: &str) -> Self {
        match text.trim() {
            "launch" => Self::Launch,
            "ejection" => Self::Ejection,
            "apogee" => Self::Apogee,
            "altitude" => Self::Altitude,
            "lowerstageseparation" => Self::LowerStageSeparation,
            "never" => Self::Never,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The word the file wrote.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Launch => "launch",
            Self::Ejection => "ejection",
            Self::Apogee => "apogee",
            Self::Altitude => "altitude",
            Self::LowerStageSeparation => "lowerstageseparation",
            Self::Never => "never",
            Self::Other(text) => text,
        }
    }
}

/// What separates a stage from the one above it, as `<separationevent>` names it. "This stage" is
/// the stage that carries the setting, the lower one, which drops away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SeparationEvent {
    /// `launch`: at launch, plus the delay.
    Launch,
    /// `ignition`: this stage's motor igniting.
    Ignition,
    /// `burnout`: this stage's motor burning out.
    Burnout,
    /// `ejection`: this stage's ejection charge.
    Ejection,
    /// `upperignition`: the motor of the stage above igniting.
    UpperIgnition,
    /// `altitudeascending`: a height on the way up.
    AltitudeAscending,
    /// `apogee`.
    Apogee,
    /// `altitudedescending`: a height on the way down.
    AltitudeDescending,
    /// `never`.
    Never,
    /// A word not listed here, kept as written.
    Other(String),
}

impl SeparationEvent {
    /// Reads a `<separationevent>`'s text.
    pub fn parse(text: &str) -> Self {
        match text.trim() {
            "launch" => Self::Launch,
            "ignition" => Self::Ignition,
            "burnout" => Self::Burnout,
            "ejection" => Self::Ejection,
            "upperignition" => Self::UpperIgnition,
            "altitudeascending" => Self::AltitudeAscending,
            "apogee" => Self::Apogee,
            "altitudedescending" => Self::AltitudeDescending,
            "never" => Self::Never,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The word the file wrote.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Launch => "launch",
            Self::Ignition => "ignition",
            Self::Burnout => "burnout",
            Self::Ejection => "ejection",
            Self::UpperIgnition => "upperignition",
            Self::AltitudeAscending => "altitudeascending",
            Self::Apogee => "apogee",
            Self::AltitudeDescending => "altitudedescending",
            Self::Never => "never",
            Self::Other(text) => text,
        }
    }
}

/// An event, a height for the events that need one, and a delay after it: when a device deploys
/// or a stage separates. Each is `None` where the file does not say.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EventSetting<E> {
    /// The event.
    pub event: Option<E>,
    /// The height, m: above the ground for a deployment (measured; see the module docs).
    pub altitude_m: Option<f64>,
    /// Seconds after the event, s.
    pub delay_s: Option<f64>,
}

impl<E: Clone> EventSetting<E> {
    /// This trigger with each field `over` states put in place of this one's.
    fn overridden_by(&self, over: &Self) -> Self {
        Self {
            event: over.event.clone().or_else(|| self.event.clone()),
            altitude_m: over.altitude_m.or(self.altitude_m),
            delay_s: over.delay_s.or(self.delay_s),
        }
    }
}

/// When a recovery device deploys.
pub type Deployment = EventSetting<DeployEvent>;

/// When a stage separates.
pub type Separation = EventSetting<SeparationEvent>;

/// Which kind of recovery device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeviceKind {
    /// A `<parachute>`.
    Parachute,
    /// A `<streamer>`.
    Streamer,
}

/// A parachute's or streamer's recovery settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RecoveryDevice {
    /// The id of the device's component in the rocket.
    pub id: String,
    /// The index of its stage in [`Rocket::stages`].
    pub stage: usize,
    /// Parachute or streamer.
    pub kind: DeviceKind,
    /// `<cd>`: a stated drag coefficient, or `auto` for OpenRocket's own (0.8 for a parachute,
    /// from the strip's size for a streamer). `None` where the file says nothing.
    pub cd: Option<Dimension>,
    /// When it deploys, as the device states it.
    pub deployment: Deployment,
    /// When it deploys in each configuration that changes that, by `configid`, with anything the
    /// configuration leaves out taken from [`RecoveryDevice::deployment`].
    pub configurations: BTreeMap<String, Deployment>,
}

impl RecoveryDevice {
    /// When the device deploys in configuration `id`.
    pub fn deployment_in(&self, id: &str) -> &Deployment {
        self.configurations.get(id).unwrap_or(&self.deployment)
    }
}

/// A stage's separation settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StageSeparation {
    /// The stage's id in the rocket.
    pub id: String,
    /// The stage's index in [`Rocket::stages`].
    pub stage: usize,
    /// When it separates, as the stage states it.
    pub separation: Separation,
    /// When it separates in each configuration that changes that, by `configid`, with anything
    /// the configuration leaves out taken from [`StageSeparation::separation`].
    pub configurations: BTreeMap<String, Separation>,
}

impl StageSeparation {
    /// When the stage separates in configuration `id`.
    pub fn separation_in(&self, id: &str) -> &Separation {
        self.configurations.get(id).unwrap_or(&self.separation)
    }
}

/// A parachute or streamer inside a part hpr does not read, such as a pod.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnreadDevice {
    /// Where it is in the file.
    pub at: String,
    /// `parachute` or `streamer`.
    pub tag: String,
    /// The tag of the outermost part that was not read: a `podset` or `parallelstage`, or else
    /// the device's own tag.
    pub inside: String,
}

/// Every recovery setting in a `.ork` design.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Recovery {
    /// The parachutes and streamers read, in file order.
    pub devices: Vec<RecoveryDevice>,
    /// Every stage that states when it separates, in file order.
    pub separations: Vec<StageSeparation>,
    /// The parachutes and streamers in parts hpr does not read.
    pub unread: Vec<UnreadDevice>,
    /// The parallel stages that state a separation, which hpr does not read yet.
    pub unread_separations: Vec<UnreadDevice>,
}

/// A device's settings, read while the walk reads its component.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DeviceRead {
    /// Where the device is in the file.
    pub at: String,
    kind: DeviceKind,
    cd: Option<Dimension>,
    deployment: Deployment,
    configurations: BTreeMap<String, Deployment>,
}

/// A stage's separation, read while the walk reads the stage.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SeparationRead {
    separation: Separation,
    configurations: BTreeMap<String, Separation>,
}

/// Reads the recovery settings of `element`, a `<parachute>` or `<streamer>` at `at`.
pub(super) fn device(element: &Element, at: &str, warnings: &mut Vec<Warning>) -> DeviceRead {
    let kind = if element.name == "streamer" {
        DeviceKind::Streamer
    } else {
        DeviceKind::Parachute
    };
    // Warnings here are about when the device opens and how much drag it states, not its shape,
    // and say so in their path.
    let cd = Values::new(element, &format!("{at}{DRAG}"), warnings).dimension(&["cd"]);
    let (deployment, configurations) = trigger(
        element,
        &format!("{at}{DEPLOYMENT}"),
        "deploy",
        "deploymentconfiguration",
        DeployEvent::parse,
        |event| !matches!(event, DeployEvent::Other(_)),
        warnings,
    );
    DeviceRead {
        at: at.to_owned(),
        kind,
        cd,
        deployment,
        configurations,
    }
}

/// Reads a stage's separation settings, or `None` when it states none.
pub(super) fn separation(
    element: &Element,
    at: &str,
    warnings: &mut Vec<Warning>,
) -> Option<SeparationRead> {
    let (separation, configurations) = trigger(
        element,
        &format!("{at}{SEPARATION}"),
        "separation",
        "separationconfiguration",
        SeparationEvent::parse,
        |event| !matches!(event, SeparationEvent::Other(_)),
        warnings,
    );
    let empty = separation.event.is_none()
        && separation.altitude_m.is_none()
        && separation.delay_s.is_none()
        && configurations.is_empty();
    (!empty).then_some(SeparationRead {
        separation,
        configurations,
    })
}

/// An element's `<{prefix}event>`, `<{prefix}altitude>` and `<{prefix}delay>`, and the same in
/// each `<{per}>` child, merged over them.
fn trigger<E: Clone>(
    element: &Element,
    at: &str,
    prefix: &str,
    per: &str,
    parse: fn(&str) -> E,
    known: fn(&E) -> bool,
    warnings: &mut Vec<Warning>,
) -> (EventSetting<E>, BTreeMap<String, EventSetting<E>>) {
    let tag = format!("{prefix}event");
    let own = |element: &Element, at: &str, warnings: &mut Vec<Warning>| {
        let mut values = Values::new(element, at, warnings);
        let event = match values.word(&[&tag]) {
            None => None,
            Some(text) if text.is_empty() => {
                values.warn_at(
                    WarningKind::Dropped,
                    format!("`{tag}` is empty; it was read as not stated"),
                );
                None
            }
            Some(text) => {
                let event = parse(&text);
                if !known(&event) {
                    values.warn_at(
                        WarningKind::Unusual,
                        format!(
                            "`{tag}` says `{text}`, a word OpenRocket 24.12 does not write; it \
                             was kept as written"
                        ),
                    );
                }
                Some(event)
            }
        };
        EventSetting {
            event,
            altitude_m: values.number(&[&format!("{prefix}altitude")]),
            delay_s: values.number(&[&format!("{prefix}delay")]),
        }
    };
    let default = own(element, at, warnings);
    let mut configurations = BTreeMap::new();
    for child in element.children_named(per) {
        let here = format!("{at}/{per}");
        let Some(id) = child
            .attribute("configid")
            .map(str::trim)
            .filter(|id| !id.is_empty())
        else {
            warnings.push(Warning::new(
                here,
                WarningKind::Dropped,
                format!("a `{per}` with no `configid` belongs to no configuration; it was ignored"),
            ));
            continue;
        };
        let over = own(child, &here, warnings);
        if configurations
            .insert(id.to_owned(), default.overridden_by(&over))
            .is_some()
        {
            warnings.push(Warning::new(
                here,
                WarningKind::Dropped,
                format!("configuration `{id}` is set twice here; the last was kept"),
            ));
        }
    }
    (default, configurations)
}

/// Every recovery setting: the devices and stages the walk read (each with the id the rocket gave
/// it), and the devices in parts it did not, found in `rocket_element`.
pub(super) fn read(
    rocket_element: &Element,
    rocket: &Rocket,
    devices: Vec<(String, DeviceRead)>,
    separations: Vec<(String, SeparationRead)>,
) -> Recovery {
    let mut placed = BTreeSet::new();
    let mut read_devices = Vec::new();
    for (id, device) in devices {
        let Some(stage) = stage_of(rocket, &id) else {
            continue;
        };
        placed.insert(device.at.clone());
        read_devices.push(RecoveryDevice {
            id,
            stage,
            kind: device.kind,
            cd: device.cd,
            deployment: device.deployment,
            configurations: device.configurations,
        });
    }
    let read_separations = separations
        .into_iter()
        .filter_map(|(id, read)| {
            Some(StageSeparation {
                stage: rocket.stages.iter().position(|stage| stage.id == id)?,
                id,
                separation: read.separation,
                configurations: read.configurations,
            })
        })
        .collect();
    let mut unread = Vec::new();
    let mut unread_separations = Vec::new();
    for (index, child) in subcomponents(rocket_element).enumerate() {
        let path = format!("openrocket/rocket/{}[{index}]", child.name);
        unread_devices(
            child,
            &path,
            None,
            &placed,
            &mut unread,
            &mut unread_separations,
        );
    }
    Recovery {
        devices: read_devices,
        separations: read_separations,
        unread,
        unread_separations,
    }
}

/// Every `<parachute>` or `<streamer>` under `element` that the walk did not read. `inside` is the
/// outermost part on the way down that hpr does not read, if any.
fn unread_devices(
    element: &Element,
    at: &str,
    inside: Option<&str>,
    read: &BTreeSet<String>,
    found: &mut Vec<UnreadDevice>,
    separations: &mut Vec<UnreadDevice>,
) {
    let inside = inside.or(match element.name.as_str() {
        tag @ ("podset" | "parallelstage") => Some(tag),
        _ => None,
    });
    let states_a_separation = ["separationevent", "separationconfiguration"]
        .iter()
        .any(|tag| element.child(tag).is_some());
    if element.name == "parallelstage" && states_a_separation {
        separations.push(UnreadDevice {
            at: at.to_owned(),
            tag: element.name.clone(),
            inside: inside.unwrap_or(element.name.as_str()).to_owned(),
        });
    }
    if matches!(element.name.as_str(), "parachute" | "streamer") && !read.contains(at) {
        found.push(UnreadDevice {
            at: at.to_owned(),
            tag: element.name.clone(),
            inside: inside.unwrap_or(element.name.as_str()).to_owned(),
        });
    }
    for (index, child) in subcomponents(element).enumerate() {
        let path = format!("{at}/{}[{index}]", child.name);
        unread_devices(child, &path, inside, read, found, separations);
    }
}
