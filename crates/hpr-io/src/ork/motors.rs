//! The motors a `.ork` design flies: its motor configurations, the motor each mount holds in each
//! of them, when that motor ignites, its ejection delay, and the thrust curve it flies on.
//!
//! **Where a `.ork` keeps them.** A design declares its configurations once, under `<rocket>`, as
//! `<motorconfiguration configid="…">` elements with a name, a `default` flag and the stages that
//! fly. The motors themselves are not there: each body tube or inner tube that is a motor mount
//! carries a `<motormount>`, and that holds one `<motor configid="…">` per configuration it is
//! loaded in — a manufacturer, a designation, a `digest`, a case diameter and length and a delay —
//! plus when it ignites, as a default for the mount and an `<ignitionconfiguration>` per
//! configuration that changes it ([F], *Motor Mount*). So a configuration is read by collecting
//! every mount's motor with its id ([Loft lesson L65][l65]: Loft read only one of the two places).
//!
//! **The thrust curve.** A `<motor>` names a motor; it does not describe one. From schema 1.11 the
//! archive can carry the curve itself as `thrustcurves/<digest>.rse` ([F], *Embedded Thrust Curve
//! Data*), and that curve is used first ([Loft lesson L57][l57], which threw such curves away): it is the curve the design was saved
//! with, identified by the digest the file names. Otherwise the motor is looked up in the bundled
//! catalog by manufacturer and designation. A motor neither reaches is read with its reason, and
//! nothing is invented for it.
//!
//! **What hpr flies.** [`hpr_design::Configuration`] holds a set of motors that all ignite at
//! launch; staging and air starts come with [M1.9][m1-9]. So only a configuration whose every motor
//! has a curve and ignites at launch, in a mount hpr reads, becomes one of the rocket's
//! configurations. Every other one is kept here, whole, with the reason it is not flown.
//!
//! [F]: https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html
//! [m1-9]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-9
//! [l57]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l57
//! [l65]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l65

use std::collections::{BTreeMap, BTreeSet};

use hpr_design::{Configuration, MountedMotor, Rocket};
use hpr_motor::catalog::bundled_curve_text;
use hpr_motor::{Catalog, Delay, SolidMotor, rse};
use serde::{Deserialize, Serialize};

use super::component::subcomponents;
use super::container::Attachment;
use super::document::Element;
use super::value::Values;
use super::warning::{Warning, WarningKind};

/// Where a `<motor>` element's thrust curve came from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Curve {
    /// The archive's own `thrustcurves/<digest>.rse` entry.
    Embedded {
        /// The archive entry, such as `thrustcurves/<digest>.rse`.
        entry: String,
        /// The motor built from it.
        motor: Box<SolidMotor>,
    },
    /// The bundled ThrustCurve.org catalog ([`Catalog::bundled`]).
    Catalog {
        /// ThrustCurve.org's motor id.
        motor_id: String,
        /// ThrustCurve.org's id of the curve file flown.
        simfile_id: String,
        /// The motor built from it.
        motor: Box<SolidMotor>,
    },
    /// No curve, and why.
    Unresolved {
        /// Why no curve was found.
        why: NoCurve,
        /// The same, in words.
        reason: String,
    },
}

/// Why a motor has no thrust curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NoCurve {
    /// A hybrid: hpr flies commercial solid motors only.
    Hybrid,
    /// The `<motor>` names no designation to look up.
    NoDesignation,
    /// Neither an embedded curve nor the bundled catalog has it.
    NotFound,
    /// More than one motor in the bundled catalog has its manufacturer and designation.
    Ambiguous,
    /// A curve was found and could not be used: an embedded `.rse` that does not read, or a
    /// catalog curve that fails.
    Unusable,
}

impl Curve {
    /// The motor, when a curve was found.
    pub fn motor(&self) -> Option<&SolidMotor> {
        match self {
            Self::Embedded { motor, .. } | Self::Catalog { motor, .. } => Some(motor),
            Self::Unresolved { .. } => None,
        }
    }
}

/// When a motor ignites, as `<ignitionevent>` names it.
///
/// OpenRocket's file-format page shows only `automatic`. The five values here are the ones
/// OpenRocket 24.12 writes, each measured by setting it and saving; the meanings are its own
/// labels, and for `automatic` its FAQ ("How do I create a staged rocket?"). Anything else is kept
/// as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum IgnitionEvent {
    /// `automatic`, "Automatic (launch or ejection charge)": the lowest stage at launch, and each
    /// stage above it at the ejection charge of the stage below.
    Automatic,
    /// `launch`: at launch.
    Launch,
    /// `ejectioncharge`: at the first ejection charge of the stage below.
    EjectionCharge,
    /// `burnout`: at the first burnout of the stage below.
    Burnout,
    /// `never`.
    Never,
    /// A value not listed here, kept as written.
    Other(String),
}

impl IgnitionEvent {
    /// Reads an `<ignitionevent>`'s text.
    fn parse(text: &str) -> Self {
        match text {
            "automatic" => Self::Automatic,
            "launch" => Self::Launch,
            "ejectioncharge" => Self::EjectionCharge,
            "burnout" => Self::Burnout,
            "never" => Self::Never,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The word the file wrote.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Automatic => "automatic",
            Self::Launch => "launch",
            Self::EjectionCharge => "ejectioncharge",
            Self::Burnout => "burnout",
            Self::Never => "never",
            Self::Other(text) => text,
        }
    }
}

/// When a motor ignites: an event and a delay after it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ignition {
    /// The event.
    pub event: IgnitionEvent,
    /// Seconds after the event, s.
    pub delay_s: f64,
}

impl Default for Ignition {
    /// `automatic`, at no delay: what a mount says when it says nothing.
    fn default() -> Self {
        Self {
            event: IgnitionEvent::Automatic,
            delay_s: 0.0,
        }
    }
}

/// A motor in a mount, in one configuration: what the `<motor>` element says, when it ignites in
/// that configuration, and the curve it flies on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrkMotor {
    /// The id of the mount component in the rocket.
    pub mount: String,
    /// The index of the mount's stage in [`Rocket::stages`].
    pub stage: usize,
    /// `<type>` as written: `single`, `reload` or `hybrid`.
    pub kind: Option<String>,
    /// `<manufacturer>`.
    pub manufacturer: String,
    /// `<designation>`, such as `H148R`.
    pub designation: String,
    /// `<digest>`: OpenRocket's key for the thrust curve, which names an embedded curve.
    pub digest: Option<String>,
    /// `<diameter>`: the case diameter, m.
    pub diameter_m: Option<f64>,
    /// `<length>`: the case length, m.
    pub length_m: Option<f64>,
    /// `<delay>`: `none` is a plugged motor, with no ejection charge; a number is the seconds from
    /// burnout to the charge, and `0` fires it at burnout (OpenRocket's technical documentation,
    /// pages 8 and 10).
    pub delay: Option<Delay>,
    /// When it ignites in this configuration: the configuration's `<ignitionconfiguration>` where
    /// the mount has one, the mount's own default where it does not.
    pub ignition: Ignition,
    /// The thrust curve.
    pub curve: Curve,
}

/// A `<motor>` inside a part hpr does not read, such as a pod's mount.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnreadMotor {
    /// Where its mount is in the file.
    pub at: String,
    /// `<designation>`.
    pub designation: String,
    /// The tag of the part that was not read: the outermost `podset` or `parallelstage` around
    /// the mount, or else the mount's own tag.
    pub inside: String,
    /// Why its mount was not read, in words.
    pub reason: String,
}

/// A motor configuration: what the rocket declares, and every motor the mounts put in it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotorConfiguration {
    /// `configid`.
    pub id: String,
    /// `<name>`, which may be empty.
    pub name: String,
    /// Whether the file marks it `default="true"`.
    pub default: bool,
    /// Whether `<rocket>` declares it; one only a mount names is read all the same.
    pub declared: bool,
    /// The `<stage number>`s the configuration marks `active="false"`.
    pub inactive_stages: Vec<u32>,
    /// Its motors, in the order their mounts appear in the file.
    pub motors: Vec<OrkMotor>,
    /// Its motors in parts hpr does not read.
    pub unread: Vec<UnreadMotor>,
    /// Why it is not among the rocket's configurations, or `None` when it is.
    pub left_out: Option<LeftOut>,
}

/// Why a configuration is not among the rocket's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeftOut {
    /// The reason.
    pub why: NotFlown,
    /// The same, in words, naming the motor.
    pub message: String,
}

/// The reasons a configuration cannot be flown as written, in the order they are checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NotFlown {
    /// A motor is in a part hpr does not read, such as a pod.
    UnreadMotor,
    /// It holds no motor.
    NoMotor,
    /// It marks a stage inactive, and hpr flies every stage.
    InactiveStage,
    /// A motor has no thrust curve.
    NoCurve,
    /// A motor has no case diameter or length.
    NoSize,
    /// A motor sits in a cluster of tubes, which hpr reads as one tube.
    Cluster,
    /// A motor ignites after launch: staging and air starts come with [M1.9][m1-9], the
    /// milestone for staging, clusters and air starts.
    ///
    /// [m1-9]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-9
    IgnitesInFlight,
}

/// Every motor configuration a `.ork` design holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Motors {
    /// The configurations: those `<rocket>` declares in its order, then any only a mount names.
    pub configurations: Vec<MotorConfiguration>,
}

impl Motors {
    /// The configuration the file marks as its default, if any.
    pub fn default_configuration(&self) -> Option<&MotorConfiguration> {
        self.configurations.iter().find(|c| c.default)
    }
}

/// A `<motor>` element, read.
#[derive(Debug, Clone, PartialEq)]
struct MotorRead {
    kind: Option<String>,
    manufacturer: String,
    designation: String,
    digest: Option<String>,
    diameter_m: Option<f64>,
    length_m: Option<f64>,
    delay: Option<Delay>,
}

/// A `<motormount>`, read while its component is: the walk that builds the rocket hands these to
/// [`read`] with the id it gave the component.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct MountRead {
    /// Where the mount component is in the file.
    pub at: String,
    /// `<overhang>`, m.
    pub overhang_m: f64,
    /// The mount's own ignition, for a configuration with no override.
    default_ignition: Ignition,
    /// `<ignitionconfiguration>`s: an event and a delay, either of which may be missing.
    ignitions: BTreeMap<String, (Option<IgnitionEvent>, Option<f64>)>,
    /// `<motor>`s, by configuration id, in file order.
    motors: Vec<(String, MotorRead)>,
    /// The tube's `clusterconfiguration`, when it is a cluster rather than one tube.
    cluster: Option<String>,
}

/// Reads the `<motormount>` of `element`, a body tube or inner tube at `at`, if it has one.
pub(super) fn mount(element: &Element, at: &str, warnings: &mut Vec<Warning>) -> Option<MountRead> {
    let mount = element.child("motormount")?;
    let here = format!("{at}/motormount");
    let mut values = Values::new(mount, &here, warnings);
    let overhang_m = values.number(&["overhang"]).unwrap_or_default();
    let default_ignition = Ignition {
        event: values
            .word(&["ignitionevent"])
            .map_or(IgnitionEvent::Automatic, |text| IgnitionEvent::parse(&text)),
        delay_s: values.number(&["ignitiondelay"]).unwrap_or_default(),
    };
    let mut ignitions = BTreeMap::new();
    let mut motors = Vec::new();
    for child in mount.elements() {
        let Some(config) = child.attribute("configid").map(str::to_owned) else {
            continue;
        };
        let at = format!("{here}/{}", child.name);
        match child.name.as_str() {
            "ignitionconfiguration" => {
                let mut values = Values::new(child, &at, warnings);
                let event = values
                    .word(&["ignitionevent"])
                    .map(|text| IgnitionEvent::parse(&text));
                let delay_s = values.number(&["ignitiondelay"]);
                ignitions.insert(config, (event, delay_s));
            }
            "motor" => motors.push((config, motor(child, &at, warnings))),
            _ => {}
        }
    }
    let cluster = element
        .child("clusterconfiguration")
        .map(|c| c.text().trim().to_owned())
        .filter(|c| c != "single");
    Some(MountRead {
        at: at.to_owned(),
        overhang_m,
        default_ignition,
        ignitions,
        motors,
        cluster,
    })
}

/// Reads one `<motor>`.
fn motor(element: &Element, at: &str, warnings: &mut Vec<Warning>) -> MotorRead {
    let mut values = Values::new(element, at, warnings);
    let delay = match values.word(&["delay"]) {
        None => None,
        Some(text) => delay(&text, &mut values),
    };
    MotorRead {
        kind: values.word(&["type"]),
        manufacturer: values.word(&["manufacturer"]).unwrap_or_default(),
        designation: values.word(&["designation"]).unwrap_or_default(),
        digest: values.word(&["digest"]).filter(|digest| !digest.is_empty()),
        diameter_m: values.number(&["diameter"]),
        length_m: values.number(&["length"]),
        delay,
    }
}

/// A `<delay>`: `none` is a plugged motor, a number the seconds to the ejection charge.
fn delay(text: &str, values: &mut Values<'_>) -> Option<Delay> {
    if text.eq_ignore_ascii_case("none") {
        return Some(Delay::Plugged);
    }
    match text.parse::<f64>() {
        Ok(seconds) if seconds.is_finite() && seconds >= 0.0 => Some(Delay::Seconds(seconds)),
        _ => {
            values.warn_at(
                WarningKind::Dropped,
                format!(
                    "`delay` says `{text}`, which is neither `none` nor a delay; it was ignored"
                ),
            );
            None
        }
    }
}

/// Reads every motor configuration: the ones `<rocket>` declares, filled with the motors in
/// `mounts` (each with the id the rocket gave its component), plus the motors in parts the rocket
/// does not read, found in `rocket_element`. Curves come from `attachments` or the bundled catalog.
///
/// Every configuration whose motors can all be flown as written is added to `rocket`.
pub(super) fn read(
    rocket_element: &Element,
    rocket: &mut Rocket,
    mounts: &[(String, MountRead)],
    attachments: &[Attachment],
    warnings: &mut Vec<Warning>,
) -> Motors {
    let at = "openrocket/rocket";
    let mut configurations: Vec<MotorConfiguration> = Vec::new();
    for element in rocket_element.children_named("motorconfiguration") {
        let Some(id) = element.attribute("configid").map(str::to_owned) else {
            warnings.push(Warning::new(
                format!("{at}/motorconfiguration"),
                WarningKind::Dropped,
                "a motor configuration with no `configid` can hold no motor; it was ignored",
            ));
            continue;
        };
        if configurations.iter().any(|c| c.id == id) {
            warnings.push(Warning::new(
                format!("{at}/motorconfiguration"),
                WarningKind::Dropped,
                format!("motor configuration `{id}` is declared twice; the first was kept"),
            ));
            continue;
        }
        let inactive_stages = element
            .children_named("stage")
            .filter(|stage| stage.attribute("active") == Some("false"))
            .filter_map(|stage| stage.attribute("number")?.parse().ok())
            .collect();
        configurations.push(MotorConfiguration {
            id,
            name: element
                .child("name")
                .map(|name| name.text().trim().to_owned())
                .unwrap_or_default(),
            default: element.attribute("default") == Some("true"),
            declared: true,
            inactive_stages,
            motors: Vec::new(),
            unread: Vec::new(),
            left_out: None,
        });
    }

    let catalog = match Catalog::bundled() {
        Ok(catalog) => Some(catalog),
        Err(error) => {
            warnings.push(Warning::new(
                at,
                WarningKind::Unusual,
                format!("the bundled motor catalog could not be read ({error}); no motor was looked up in it"),
            ));
            None
        }
    };
    for (mount_id, mount) in mounts {
        let stage = stage_of(rocket, mount_id).unwrap_or_default();
        for (config, read) in &mount.motors {
            let index = configuration(&mut configurations, config, at, warnings);
            let ignition = match mount.ignitions.get(config) {
                Some((event, delay_s)) => Ignition {
                    event: event
                        .clone()
                        .unwrap_or_else(|| mount.default_ignition.event.clone()),
                    delay_s: delay_s.unwrap_or(mount.default_ignition.delay_s),
                },
                None => mount.default_ignition.clone(),
            };
            let curve = curve(read, attachments, catalog.as_ref(), &mount.at, warnings);
            configurations[index].motors.push(OrkMotor {
                mount: mount_id.clone(),
                stage,
                kind: read.kind.clone(),
                manufacturer: read.manufacturer.clone(),
                designation: read.designation.clone(),
                digest: read.digest.clone(),
                diameter_m: read.diameter_m,
                length_m: read.length_m,
                delay: read.delay,
                ignition,
                curve,
            });
        }
    }

    let read_at: BTreeSet<&str> = mounts.iter().map(|(_, mount)| mount.at.as_str()).collect();
    let mut unread = Vec::new();
    for (index, stage) in subcomponents(rocket_element).enumerate() {
        let path = format!("{at}/{}[{index}]", stage.name);
        unread_motors(stage, &path, None, &read_at, &mut unread);
    }
    for (config, motor) in unread {
        let index = configuration(&mut configurations, &config, at, warnings);
        configurations[index].unread.push(motor);
    }

    let last_stage = rocket.stages.len().saturating_sub(1);
    let clusters: BTreeMap<&str, &str> = mounts
        .iter()
        .filter_map(|(id, mount)| Some((id.as_str(), mount.cluster.as_deref()?)))
        .collect();
    for configuration in &mut configurations {
        configuration.left_out = left_out(configuration, last_stage, &clusters);
        if configuration.left_out.is_none() {
            rocket.configurations.push(flown(configuration));
        }
    }
    Motors { configurations }
}

/// The index of configuration `id`, adding it, with a warning, when `<rocket>` does not declare it.
fn configuration(
    configurations: &mut Vec<MotorConfiguration>,
    id: &str,
    at: &str,
    warnings: &mut Vec<Warning>,
) -> usize {
    if let Some(index) = configurations.iter().position(|c| c.id == id) {
        return index;
    }
    warnings.push(Warning::new(
        at,
        WarningKind::Unusual,
        format!(
            "a mount puts a motor in configuration `{id}`, which the rocket does not declare; it \
             was read as a configuration of its own"
        ),
    ));
    configurations.push(MotorConfiguration {
        id: id.to_owned(),
        name: String::new(),
        default: false,
        declared: false,
        inactive_stages: Vec::new(),
        motors: Vec::new(),
        unread: Vec::new(),
        left_out: None,
    });
    configurations.len() - 1
}

/// Every `<motor>` under `element` whose mount is not among `read`, with its configuration id.
/// `inside` is the outermost part on the way down that hpr does not read, if any.
fn unread_motors(
    element: &Element,
    at: &str,
    inside: Option<&str>,
    read: &BTreeSet<&str>,
    found: &mut Vec<(String, UnreadMotor)>,
) {
    let inside = inside.or(match element.name.as_str() {
        tag @ ("podset" | "parallelstage") => Some(tag),
        _ => None,
    });
    if let Some(mount) = element.child("motormount")
        && !read.contains(at)
    {
        let reason = match inside {
            Some("podset") => {
                "its mount is inside a pod set, which hpr does not read yet".to_owned()
            }
            Some(_) => {
                "its mount is inside a parallel stage, which hpr does not read yet".to_owned()
            }
            None => format!("its mount, a `{}`, was not read", element.name),
        };
        let part = inside.unwrap_or(element.name.as_str());
        for motor in mount.children_named("motor") {
            found.push((
                motor.attribute("configid").unwrap_or_default().to_owned(),
                UnreadMotor {
                    at: at.to_owned(),
                    designation: motor
                        .child("designation")
                        .map(|d| d.text().trim().to_owned())
                        .unwrap_or_default(),
                    inside: part.to_owned(),
                    reason: reason.clone(),
                },
            ));
        }
    }
    for (index, child) in subcomponents(element).enumerate() {
        let path = format!("{at}/{}[{index}]", child.name);
        unread_motors(child, &path, inside, read, found);
    }
}

/// The index of the stage holding the component with id `id`.
fn stage_of(rocket: &Rocket, id: &str) -> Option<usize> {
    fn holds(components: &[hpr_design::Component], id: &str) -> bool {
        components
            .iter()
            .any(|c| c.id == id || holds(&c.children, id))
    }
    rocket
        .stages
        .iter()
        .position(|stage| holds(&stage.components, id))
}

/// Lowercase letters and digits only, for comparing names written two ways.
fn key(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The thrust curve for `motor`: its embedded `.rse` first, then the bundled catalog.
fn curve(
    motor: &MotorRead,
    attachments: &[Attachment],
    catalog: Option<&Catalog>,
    at: &str,
    warnings: &mut Vec<Warning>,
) -> Curve {
    let unresolved = |why: NoCurve, reason: String| Curve::Unresolved { why, reason };
    if motor.kind.as_deref() == Some("hybrid") {
        return unresolved(
            NoCurve::Hybrid,
            "a hybrid motor; hpr flies commercial solid motors only".to_owned(),
        );
    }
    if motor.designation.is_empty() {
        return unresolved(
            NoCurve::NoDesignation,
            "the motor has no designation".to_owned(),
        );
    }
    let mut embedded_failed = None;
    if let Some(digest) = &motor.digest {
        let entry = format!("thrustcurves/{digest}.rse");
        if let Some(attachment) = attachments.iter().find(|a| a.name == entry) {
            match embedded(attachment, motor, at, warnings) {
                Ok(solid) => {
                    return Curve::Embedded {
                        entry,
                        motor: Box::new(solid),
                    };
                }
                Err(reason) => {
                    warnings.push(Warning::new(
                        entry.clone(),
                        WarningKind::Dropped,
                        format!(
                            "the embedded curve for {} was not used: {reason}",
                            motor.designation
                        ),
                    ));
                    embedded_failed = Some(format!("its embedded curve {entry} could not be used"));
                }
            }
        }
    }
    let Some(catalog) = catalog else {
        return match embedded_failed {
            Some(reason) => unresolved(NoCurve::Unusable, reason),
            None => unresolved(NoCurve::NotFound, "no embedded curve".to_owned()),
        };
    };
    let maker = key(&motor.manufacturer);
    let matches: Vec<_> = catalog
        .find(&motor.designation)
        .filter(|m| key(&m.manufacturer) == maker || key(&m.manufacturer_abbrev) == maker)
        .collect();
    // A curve that was there and failed says more about the file than a catalog that lacks it.
    let not_found = |kind: NoCurve, why: &str| match &embedded_failed {
        Some(lead) => unresolved(NoCurve::Unusable, format!("{lead}, and {why}")),
        None => unresolved(kind, format!("no embedded curve, and {why}")),
    };
    match matches.as_slice() {
        [] => not_found(
            NoCurve::NotFound,
            "no motor of that manufacturer and designation in the bundled catalog",
        ),
        [found] => {
            let Some((curve, text)) = found
                .curves
                .iter()
                .find_map(|curve| Some((curve, bundled_curve_text(&curve.file)?)))
            else {
                return not_found(
                    NoCurve::NotFound,
                    "the bundled catalog lists it without a curve",
                );
            };
            match found.motor(curve, text) {
                Ok(solid) => Curve::Catalog {
                    motor_id: found.motor_id.clone(),
                    simfile_id: curve.simfile_id.clone(),
                    motor: Box::new(solid),
                },
                Err(error) => not_found(
                    NoCurve::Unusable,
                    &format!("the bundled catalog's curve failed: {error}"),
                ),
            }
        }
        several => not_found(
            NoCurve::Ambiguous,
            &format!(
                "{} motors of that manufacturer and designation are in the bundled catalog",
                several.len()
            ),
        ),
    }
}

/// The motor an embedded `.rse` entry describes, built as the catalog builds one
/// ([`SolidMotor::from_envelope`] from the file's diameter, length and masses).
fn embedded(
    attachment: &Attachment,
    motor: &MotorRead,
    at: &str,
    warnings: &mut Vec<Warning>,
) -> Result<SolidMotor, String> {
    let text =
        std::str::from_utf8(&attachment.bytes).map_err(|_| "it is not UTF-8 text".to_owned())?;
    let parsed = rse::parse(text).map_err(|error| error.to_string())?;
    for warning in &parsed.warnings {
        warnings.push(Warning::new(
            attachment.name.clone(),
            WarningKind::Unusual,
            format!("line {}: {}", warning.line, warning.message),
        ));
    }
    let [engine] = parsed.value.engines.as_slice() else {
        return Err(format!(
            "it holds {} engines, not one",
            parsed.value.engines.len()
        ));
    };
    if key(&engine.code) != key(&motor.designation) {
        warnings.push(Warning::new(
            at,
            WarningKind::Unusual,
            format!(
                "the embedded curve its digest names is for `{}`, and the motor says `{}`; the \
                 curve was used",
                engine.code, motor.designation
            ),
        ));
    }
    let thrust = engine.thrust_curve().map_err(|error| error.to_string())?;
    SolidMotor::from_envelope(
        thrust,
        engine.diameter_mm * 1e-3,
        engine.length_mm * 1e-3,
        engine.propellant_mass_g * 1e-3,
        engine.initial_mass_g * 1e-3,
    )
    .map_err(|error| error.to_string())
}

/// Why `configuration` cannot be flown as written, or `None` when it can: every motor read, with
/// a curve, a size and a single tube to sit in, igniting at launch, and every stage flying.
fn left_out(
    configuration: &MotorConfiguration,
    last_stage: usize,
    clusters: &BTreeMap<&str, &str>,
) -> Option<LeftOut> {
    let out = |why: NotFlown, message: String| Some(LeftOut { why, message });
    if let Some(unread) = configuration.unread.first() {
        return out(
            NotFlown::UnreadMotor,
            format!(
                "{} of its motors is in a part hpr does not read ({}: {})",
                configuration.unread.len(),
                unread.designation,
                unread.reason
            ),
        );
    }
    if configuration.motors.is_empty() {
        return out(NotFlown::NoMotor, "it holds no motor".to_owned());
    }
    if !configuration.inactive_stages.is_empty() {
        return out(
            NotFlown::InactiveStage,
            format!(
                "it flies without stage {:?}, and hpr flies every stage",
                configuration.inactive_stages
            ),
        );
    }
    for motor in &configuration.motors {
        let name = &motor.designation;
        if let Curve::Unresolved { reason, .. } = &motor.curve {
            return out(
                NotFlown::NoCurve,
                format!("no thrust curve for {name}: {reason}"),
            );
        }
        if !motor.diameter_m.is_some_and(|d| d > 0.0) || !motor.length_m.is_some_and(|l| l > 0.0) {
            return out(
                NotFlown::NoSize,
                format!("{name} has no case diameter and length"),
            );
        }
        if let Some(cluster) = clusters.get(motor.mount.as_str()) {
            return out(
                NotFlown::Cluster,
                format!(
                    "{name} sits in a cluster of motor tubes (`{cluster}`), which hpr reads as one tube"
                ),
            );
        }
        let at_launch = motor.ignition.delay_s == 0.0
            && match motor.ignition.event {
                IgnitionEvent::Launch => true,
                IgnitionEvent::Automatic => motor.stage == last_stage,
                _ => false,
            };
        if !at_launch {
            return out(
                NotFlown::IgnitesInFlight,
                format!(
                    "{name} ignites at `{}` plus {} s, in stage {}; hpr ignites every motor at launch \
                 until staging and air starts (M1.9)",
                    motor.ignition.event.as_str(),
                    motor.ignition.delay_s,
                    motor.stage
                ),
            );
        }
    }
    None
}

/// `configuration` as a [`Configuration`] for [`Rocket::assemble`]; every motor has a curve and a
/// size, which [`left_out`] checked.
fn flown(configuration: &MotorConfiguration) -> Configuration {
    Configuration {
        id: configuration.id.clone(),
        name: configuration.name.clone(),
        motors: configuration
            .motors
            .iter()
            .filter_map(|motor| {
                Some(MountedMotor {
                    mount: motor.mount.clone(),
                    designation: motor.designation.clone(),
                    diameter_m: motor.diameter_m?,
                    length_m: motor.length_m?,
                    motor: motor.curve.motor()?.clone(),
                    delay: motor.delay,
                })
            })
            .collect(),
    }
}
