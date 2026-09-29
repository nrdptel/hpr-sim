//! `hpr sim`: a design flown from a rail, its summary printed and its recording exported.
//!
//! The flight is the library's own: the design becomes an [`hpr::Rocket`] with
//! [`hpr::Rocket::from_design`], and [`hpr::Flight::builder`] flies it, so what this prints is
//! what a program calling the facade gets, number for number. The stack flies whole: no recovery
//! device and no separation is flown yet, which the output's notes say. What would make the
//! flight another rocket than the file's is refused (ADR-106).

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use hpr::hpr_design::{self, Configuration, Ignition, MountedMotor, checks};
use hpr::hpr_io::ork;
use hpr::hpr_motor::text::{ParseWarning, WarningKind as ReadWarning};
use hpr::hpr_motor::{eng, rse};
use hpr::hpr_sim::metrics::{self, FlightSummary};
use hpr::hpr_sim::{self, Channel, FlightSettings, Recorder, export};
use hpr::{Environment, Flight, Motor, Rocket};

use crate::motors::MotorFile;
use crate::output::{
    Apogee, DesignConfiguration, DesignFormat, EventKind, Export, ExportFormat, FileFormat,
    InputWarning, Landing, Launch, Margin, Peak, SimDesign, SimEvent, SimFlight, SimMotor,
    SimMotorSource, Stability, Summary, Termination, WarningKind,
};
use crate::{Failure, Out};

/// The rail's length when `--rail-length` isn't given, m.
pub const DEFAULT_RAIL_LENGTH_M: f64 = 1.5;

/// The recording's interval when `--interval` isn't given, s.
pub const DEFAULT_INTERVAL_S: f64 = 0.01;

/// The finest recording interval `--interval` takes, s: a finer one fills memory on a long fall,
/// at a million rows every 17 minutes of flight.
pub const MIN_INTERVAL_S: f64 = 0.001;

/// `hpr sim`'s arguments.
#[derive(Debug, clap::Args)]
pub struct SimArgs {
    /// The design: an OpenRocket .ork file, or a rocket's JSON (.json; not yet an .hpr document)
    pub design: String,
    /// The motor configuration to fly, by its id [default: the design's default, or its only one]
    #[arg(long, value_name = "ID")]
    pub config: Option<String>,
    /// Fly this motor in place of the configuration's, lit at launch: a catalog designation or
    /// common name, or a .eng or .rse file
    #[arg(long, value_name = "NAME_OR_FILE")]
    pub motor: Option<String>,
    /// The motor mount, by its component id, that --motor goes in [default: the configuration's,
    /// or the design's only one]
    #[arg(long, value_name = "ID", requires = "motor")]
    pub mount: Option<String>,
    /// The launch site's latitude, degrees north (south is negative)
    #[arg(
        long,
        value_name = "DEG",
        default_value_t = 0.0,
        allow_negative_numbers = true
    )]
    pub latitude: f64,
    /// The launch site's longitude, degrees east (west is negative)
    #[arg(
        long,
        value_name = "DEG",
        default_value_t = 0.0,
        allow_negative_numbers = true
    )]
    pub longitude: f64,
    /// The launch site's height above sea level, m
    #[arg(
        long,
        value_name = "M",
        default_value_t = 0.0,
        allow_negative_numbers = true
    )]
    pub elevation: f64,
    /// The rail's length, m, from the rocket's aft end to the rail's top
    #[arg(long, value_name = "M", default_value_t = DEFAULT_RAIL_LENGTH_M)]
    pub rail_length: f64,
    /// The rail's angle above the horizon, degrees [default: 90, vertical]
    #[arg(long, value_name = "DEG")]
    pub inclination: Option<f64>,
    /// The direction the rail leans toward, degrees clockwise from north [default: 0]
    #[arg(long, value_name = "DEG", allow_negative_numbers = true)]
    pub heading: Option<f64>,
    /// A wind of this speed at every height, m/s [default: calm]
    #[arg(long, value_name = "M_S")]
    pub wind: Option<f64>,
    /// The direction the wind blows from, degrees clockwise from north
    #[arg(
        long,
        value_name = "DEG",
        default_value_t = 0.0,
        requires = "wind",
        allow_negative_numbers = true
    )]
    pub wind_from: f64,
    /// Write the flight's recording to this file: .csv, .json, .parquet, .geojson or .kml
    /// (repeat for several)
    #[arg(long, value_name = "FILE")]
    pub export: Vec<String>,
    /// The recording's interval, s, at least 0.001; a row is also recorded at every event
    #[arg(long, value_name = "S", default_value_t = DEFAULT_INTERVAL_S)]
    pub interval: f64,
    /// Fly a design whose checks find errors, such as a motor wider than its mount
    #[arg(long)]
    pub accept_design_errors: bool,
}

/// Runs `hpr sim`.
pub(crate) fn run(args: &SimArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    // Everything that can be refused is refused before the flight: the exports, the recording's
    // interval, the design, the motor, the site.
    let exports = exports(args)?;
    if args.interval.is_nan() || args.interval < MIN_INTERVAL_S {
        return Err(Failure::Input(format!(
            "--interval {}: the recording's interval is at least {MIN_INTERVAL_S} s",
            args.interval
        )));
    }
    let mut recorder = Recorder::new(Channel::ALL.to_vec(), Some(args.interval))
        .map_err(|error| Failure::Input(format!("--interval: {error}")))?;
    let mut read = read_design(&args.design)?;
    let configurations = read.configurations();
    let (configuration, motor_source) = match &args.motor {
        Some(name) => {
            let (motor, source, warnings) = motor(name)?;
            read.warnings.extend(warnings);
            let id = read.swap(args.config.as_deref(), args.mount.as_deref(), &motor)?;
            (id, source)
        }
        None => (read.flown(args.config.as_deref())?, SimMotorSource::Design),
    };
    let configuration_name = configurations
        .iter()
        .find(|c| c.id == configuration)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    // Only the configuration flown: the design's checks look at every configuration, and
    // another's error isn't this flight's.
    read.rocket.configurations.retain(|c| c.id == configuration);
    let flown_motors = read
        .rocket
        .configurations
        .first()
        .map(|flown| flown.motors.clone())
        .unwrap_or_default();
    // The stack flies whole, so a motor lit by its stage's separation would never light.
    if let Some(motor) = flown_motors
        .iter()
        .find(|motor| matches!(motor.ignition, Ignition::Separation { .. }))
    {
        return Err(Failure::Input(format!(
            "configuration {configuration} lights {} at its stage's separation, which hpr sim \
             doesn't fly yet; the library does, as its two-stage examples show",
            motor.designation
        )));
    }
    let (errors, findings): (Vec<_>, Vec<_>) = checks::check(&read.rocket)
        .map_err(|error| Failure::Input(format!("the design doesn't hold together: {error}")))?
        .into_iter()
        .partition(|finding| finding.severity() == checks::Severity::Error);
    if !errors.is_empty() {
        if !args.accept_design_errors {
            return Err(Failure::Input(format!(
                "the design's checks found {}, which hpr sim flies only with \
                 --accept-design-errors: {}",
                count(errors.len(), "error"),
                findings_text(&errors)
            )));
        }
        read.notes.push(format!(
            "flown with --accept-design-errors past {}, so its numbers are not a buildable \
             rocket's: {}",
            count(errors.len(), "error"),
            findings_text(&errors)
        ));
    }
    read.warnings
        .extend(findings.iter().map(|finding| InputWarning {
            at: "design checks".to_owned(),
            kind: WarningKind::Unusual,
            message: finding_text(finding),
        }));
    // How many of each motor fly: a cluster's tubes and a pod set's pods each carry one.
    let placed = read
        .rocket
        .assemble(&configuration)
        .map_err(|error| Failure::Input(format!("the design doesn't hold together: {error}")))?
        .motors;
    let names = component_names(&read.rocket);
    let design_name = read.rocket.name.clone();

    let rocket = Rocket::from_design(read.rocket, &configuration).map_err(input)?;
    let mut environment = Environment::new(args.latitude, args.longitude, args.elevation)
        .map_err(|error| Failure::Input(format!("the launch site: {error}")))?;
    if let Some(speed) = args.wind {
        environment = environment
            .with_constant_wind(speed, args.wind_from)
            .map_err(|error| Failure::Input(format!("the wind: {error}")))?;
    }
    // The builder's own rail unless an angle is given, so a vertical launch is the library's
    // default to the bit.
    let mut builder = Flight::builder(&rocket, &environment, args.rail_length);
    if let Some(inclination) = args.inclination {
        builder = builder.inclination_deg(inclination);
    }
    if let Some(heading) = args.heading {
        builder = builder.heading_deg(heading);
    }
    if args.accept_design_errors {
        builder = builder.settings(FlightSettings {
            accept_design_errors: true,
            ..FlightSettings::default()
        });
    }
    let flight = if exports.is_empty() {
        builder.fly()
    } else {
        builder.fly_with(&mut recorder)
    }
    .map_err(input)?;

    let mut written = Vec::new();
    let name = format!("{design_name}, {configuration}");
    for (path, format) in &exports {
        let contents = contents(*format, &recorder, &environment, flight.summary(), &name)
            .map_err(|error| Failure::Input(format!("{path}: {error}")))?;
        std::fs::write(path, contents)
            .map_err(|error| Failure::Input(format!("{path}: {error}")))?;
        written.push(Export {
            path: (*path).to_owned(),
            format: *format,
            rows: recorder.rows().len(),
        });
    }

    let document = SimFlight {
        design: SimDesign {
            file: file_name(&args.design),
            format: read.format,
            name: design_name,
            configuration,
            configuration_name,
            configurations,
        },
        motors: flown_motors
            .iter()
            .map(|flown| {
                let mine = placed.iter().filter(|p| {
                    p.mount == flown.mount && p.mounted.designation == flown.designation
                });
                SimMotor {
                    designation: flown.designation.clone(),
                    mount: flown.mount.clone(),
                    mount_name: names
                        .iter()
                        .find(|(id, _)| *id == flown.mount)
                        .map(|(_, name)| name.clone())
                        .unwrap_or_default(),
                    count: mine.clone().count(),
                    unlit: mine.filter(|p| p.fails).count(),
                    source: motor_source.clone(),
                    ignition: ignition(&flown.ignition),
                }
            })
            .collect(),
        launch: Launch {
            latitude_deg: args.latitude,
            longitude_deg: args.longitude,
            elevation_m: args.elevation,
            rail_length_m: args.rail_length,
            inclination_deg: args.inclination.unwrap_or(90.0),
            heading_deg: args.heading.unwrap_or(0.0),
            wind_speed_m_s: args.wind.unwrap_or(0.0),
            wind_from_deg: args.wind_from,
            atmosphere: "standard".to_owned(),
        },
        summary: summary(flight.summary()),
        events: flight.result().events.iter().map(event).collect(),
        exports: written,
        notes: read.notes,
        warnings: read.warnings,
    };
    to.emit(&document, |out| print(&document, out))
}

/// A library error, as the command reports it.
fn input(error: hpr::Error) -> Failure {
    Failure::Input(error.to_string())
}

/// Findings as their JSON, one after another.
fn findings_text(findings: &[checks::Finding]) -> String {
    findings
        .iter()
        .map(finding_text)
        .collect::<Vec<_>>()
        .join("; ")
}

/// A finding as its JSON: its kind and the parts and sizes it is about.
fn finding_text(finding: &checks::Finding) -> String {
    serde_json::to_string(finding).unwrap_or_else(|_| format!("{finding:?}"))
}

/// `n` things, in words: `1 error`, `2 errors`.
fn count(n: usize, thing: &str) -> String {
    if n == 1 {
        format!("1 {thing}")
    } else {
        format!("{n} {thing}s")
    }
}

/// The file name of a path, for the output: the path's folder depends on where it was run from.
fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}

/// The `--export` files and their formats, refused before the flight if a format is unknown, a
/// file's folder is missing, a file is given twice, or a file is one this run reads.
fn exports(args: &SimArgs) -> Result<Vec<(&str, ExportFormat)>, Failure> {
    let inputs: Vec<PathBuf> = std::iter::once(args.design.as_str())
        .chain(
            args.motor
                .as_deref()
                .filter(|motor| MotorFile::of(motor).is_some()),
        )
        .map(same_file)
        .collect();
    let mut seen = Vec::new();
    let mut exports = Vec::new();
    for path in &args.export {
        let format = export_format(path)?;
        let file = same_file(path);
        if inputs.contains(&file) {
            return Err(Failure::Input(format!(
                "{path}: this run reads that file, and --export would write over it"
            )));
        }
        if seen.contains(&file) {
            return Err(Failure::Input(format!(
                "{path}: --export names the file twice"
            )));
        }
        if let Some(folder) = Path::new(path).parent()
            && !folder.as_os_str().is_empty()
            && !folder.is_dir()
        {
            return Err(Failure::Input(format!(
                "{path}: there is no folder {}",
                folder.display()
            )));
        }
        seen.push(file);
        exports.push((path.as_str(), format));
    }
    Ok(exports)
}

/// A path as the file system knows it, to compare two: the file's canonical path if it exists,
/// or its folder's with its name.
pub(crate) fn same_file(path: &str) -> PathBuf {
    let path = Path::new(path);
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let folder = match path.parent() {
        Some(folder) if !folder.as_os_str().is_empty() => folder,
        _ => Path::new("."),
    };
    match (folder.canonicalize(), path.file_name()) {
        (Ok(folder), Some(name)) => folder.join(name),
        _ => path.to_path_buf(),
    }
}

/// A design read, with the configurations it could fly and what the reader said.
struct Read {
    format: DesignFormat,
    rocket: hpr_design::Rocket,
    /// The `.ork` file's own motor configurations, flyable or not; `None` for an hpr design.
    ork: Option<ork::Motors>,
    /// Why a `.ork` rocket wasn't read exactly as written, if it wasn't
    /// ([`ork::airframe_not_as_written`]).
    airframe: Option<String>,
    notes: Vec<String>,
    warnings: Vec<InputWarning>,
}

/// The note on the descent, after what the design says of its recovery.
fn descent(recovery: &str) -> String {
    format!(
        "{recovery}, so the rocket falls from apogee on its airframe alone, on aerodynamics that \
         hold only at small angles of attack: its landing time, speed and place, \
         and any peak it sets in the fall, are not a prediction"
    )
}

/// The note for a design of more than one stage.
const WHOLE_STACK: &str =
    "the stages fly as one stack: hpr sim flies no separation yet, so none comes apart";

/// Reads a `.ork` or a rocket's JSON, by its extension.
fn read_design(path: &str) -> Result<Read, Failure> {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let bytes = || std::fs::read(path).map_err(|error| Failure::Input(format!("{path}: {error}")));
    match extension.as_deref() {
        Some("ork") => {
            let file =
                ork::read(&bytes()?).map_err(|error| Failure::Input(format!("{path}: {error}")))?;
            let design = ork::design(&file.value);
            let mut warnings = file.warnings;
            warnings.extend(design.warnings);
            let design = design.value;
            let devices = design.recovery.devices.len();
            let mut notes = vec![if devices == 0 {
                descent("the file has no recovery device")
            } else {
                descent(&format!(
                    "the file's {} {} not flown yet",
                    count(devices, "recovery device"),
                    if devices == 1 { "is" } else { "are" },
                ))
            }];
            if design.rocket.stages.len() > 1 {
                notes.push(WHOLE_STACK.to_owned());
            }
            if design.is_reduced() {
                notes.push(
                    "the file has parts hpr keeps aside instead of flying, such as a parallel \
                     stage: the rocket flown is the rest of it"
                        .to_owned(),
                );
            }
            Ok(Read {
                format: DesignFormat::Ork,
                rocket: design.rocket,
                ork: Some(design.motors),
                airframe: ork::airframe_not_as_written(&file.value),
                notes,
                warnings: warnings.iter().map(ork_warning).collect(),
            })
        }
        Some("json") => {
            let rocket: hpr_design::Rocket = serde_json::from_slice(&bytes()?)
                .map_err(|error| Failure::Input(format!("{path}: not a rocket's JSON: {error}")))?;
            let mut notes = vec![descent("a rocket's JSON holds no recovery devices")];
            if rocket.stages.len() > 1 {
                notes.push(WHOLE_STACK.to_owned());
            }
            Ok(Read {
                format: DesignFormat::HprJson,
                rocket,
                ork: None,
                airframe: None,
                notes,
                warnings: Vec::new(),
            })
        }
        _ => Err(Failure::Input(format!(
            "{path}: hpr sim reads an OpenRocket .ork file or a rocket's JSON (.json), not yet an .hpr document"
        ))),
    }
}

impl Read {
    /// Every configuration of the file, and whether it flies as the file has it.
    fn configurations(&self) -> Vec<DesignConfiguration> {
        match &self.ork {
            Some(motors) => motors
                .configurations
                .iter()
                .map(|c| DesignConfiguration {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    flies: c.left_out.is_none()
                        && c.staging.is_none()
                        && self.rocket.configuration(&c.id).is_some(),
                })
                .collect(),
            None => self
                .rocket
                .configurations
                .iter()
                .map(|c| DesignConfiguration {
                    id: c.id.clone(),
                    name: c.name.clone(),
                    flies: !c
                        .motors
                        .iter()
                        .any(|m| matches!(m.ignition, Ignition::Separation { .. })),
                })
                .collect(),
        }
    }

    /// The configuration flown as the design has it: `--config`, or the default or only one.
    fn flown(&self, wanted: Option<&str>) -> Result<String, Failure> {
        let id = match &self.ork {
            Some(motors) => {
                let chosen = ork_configuration(motors, wanted)?.ok_or_else(|| {
                    unchosen(
                        motors.configurations.iter().map(|c| &c.id),
                        "the file",
                        self.no_motor_flies(None),
                    )
                })?;
                if let Some(staging) = &chosen.staging {
                    return Err(powered_separation(&chosen.id, staging));
                }
                if let Some(left_out) = &chosen.left_out {
                    let mut hint = match self.no_motor_flies(Some(chosen)) {
                        None => "; give a motor with --motor".to_owned(),
                        Some(why) => format!("; nor can it with --motor: {why}"),
                    };
                    let flying: Vec<String> = self
                        .configurations()
                        .into_iter()
                        .filter(|c| c.flies)
                        .map(|c| c.id)
                        .collect();
                    if !flying.is_empty() {
                        hint.push_str(&format!(
                            "; the file's configurations that fly as written: {}",
                            list(&flying)
                        ));
                    }
                    return Err(Failure::Input(format!(
                        "configuration {} can't be flown as the file has it: {}{hint}",
                        chosen.id, left_out.message
                    )));
                }
                chosen.id.clone()
            }
            None => self
                .hpr_configuration(wanted)?
                .ok_or_else(|| {
                    unchosen(
                        self.rocket.configurations.iter().map(|c| &c.id),
                        "the design",
                        None,
                    )
                })?
                .id
                .clone(),
        };
        // A configuration the reader didn't leave out is among the rocket's; this is its check.
        if self.rocket.configuration(&id).is_none() {
            return Err(Failure::Input(format!(
                "configuration {id} can't be flown as the file has it"
            )));
        }
        Ok(id)
    }

    /// Why no motor of the user's own flies the `.ork` rocket in `chosen` (or, with none, in a
    /// configuration of its own), if none does: its airframe or stages aren't what hpr sim flies
    /// whole, or the configuration is left out for more than its motor. A configuration's
    /// `LeftOut` names only its first reason, so each is asked of the file directly.
    fn no_motor_flies(&self, chosen: Option<&ork::MotorConfiguration>) -> Option<String> {
        if let Some(why) = &self.airframe {
            return Some(format!(
                "the airframe was not read exactly as written: {why}"
            ));
        }
        if let Some(chosen) = chosen {
            if let Some(staging) = &chosen.staging {
                return Some(separation_text(staging));
            }
            if !chosen.inactive_stages.is_empty() {
                return Some(format!(
                    "configuration {} switches a stage off, and hpr flies every stage",
                    chosen.id
                ));
            }
            if !chosen.unread.is_empty() {
                return Some(format!(
                    "a motor of configuration {} is in a part hpr doesn't read",
                    chosen.id
                ));
            }
        }
        let stages = self.rocket.stages.len();
        if stages > 1 {
            return Some(format!(
                "the rocket has {stages} stages, and hpr sim can't yet tell when they would \
                 separate with a motor of your own"
            ));
        }
        match chosen.and_then(|chosen| chosen.left_out.as_ref()) {
            Some(left_out) if !motor_fixes(left_out.why) => Some(left_out.message.clone()),
            _ => None,
        }
    }

    /// Puts `motor`, lit at launch, in the configuration `--config` names, or the default or
    /// only one, or where there is none to choose, in a configuration named after the motor; and
    /// returns that configuration's id. A `.ork` rocket that no motor of the user's flies as the
    /// file's is refused ([`Read::no_motor_flies`]).
    fn swap(
        &mut self,
        wanted: Option<&str>,
        mount: Option<&str>,
        motor: &Motor,
    ) -> Result<String, Failure> {
        let chosen: Option<(String, Vec<String>)> = match &self.ork {
            Some(motors) => {
                let chosen = ork_configuration(motors, wanted)?;
                if let Some(why) = self.no_motor_flies(chosen) {
                    let what = chosen.map_or_else(
                        || "the rocket".to_owned(),
                        |c| format!("configuration {}", c.id),
                    );
                    return Err(Failure::Input(format!(
                        "{what} can't be flown with a motor of your own: {why}"
                    )));
                }
                chosen.map(|c| (c.id.clone(), owned(c.motors.iter().map(|m| &m.mount))))
            }
            None => self
                .hpr_configuration(wanted)?
                .map(|c| (c.id.clone(), owned(c.motors.iter().map(|m| &m.mount)))),
        };
        let (id, configured) =
            chosen.unwrap_or_else(|| (motor.designation().to_owned(), Vec::new()));
        let mount = self.mount(mount, &id, &configured)?;
        self.fit(&id, &mount, motor);
        Ok(id)
    }

    /// The mount `--motor` goes in: `--mount`, or the configuration's one motor's, or the
    /// design's only mount.
    fn mount(
        &self,
        given: Option<&str>,
        id: &str,
        configured: &[String],
    ) -> Result<String, Failure> {
        let mounts = mounts(&self.rocket);
        if configured.len() > 1 {
            return Err(Failure::Input(format!(
                "configuration {id} has {} motors, in {}; --motor flies one motor in place of \
                 them all, so it takes a configuration of one",
                configured.len(),
                list(configured)
            )));
        }
        let mount = match (given, configured, &mounts[..]) {
            (Some(given), _, _) => given.to_owned(),
            (None, [mount], _) | (None, [], [mount]) => mount.clone(),
            (None, [], []) => {
                return Err(Failure::Input(
                    "the design has no motor mount for --motor to go in".to_owned(),
                ));
            }
            (None, _, _) => {
                return Err(Failure::Input(format!(
                    "the design has {} motor mounts: say which with --mount ({})",
                    mounts.len(),
                    list(&mounts)
                )));
            }
        };
        if mounts.contains(&mount) {
            Ok(mount)
        } else {
            Err(Failure::Input(format!(
                "the design has no motor mount `{mount}` that hpr reads; its mounts: {}",
                list(&mounts)
            )))
        }
    }

    /// Puts `motor` in `mount`, lit at launch, as configuration `id`'s only motor.
    fn fit(&mut self, id: &str, mount: &str, motor: &Motor) {
        let motors = vec![MountedMotor {
            mount: mount.to_owned(),
            designation: motor.designation().to_owned(),
            diameter_m: motor.diameter_m(),
            length_m: motor.length_m(),
            motor: motor.solid_motor().clone(),
            delay: motor.delay(),
            ignition: Ignition::Launch,
            failed_tubes: Vec::new(),
        }];
        let configurations = &mut self.rocket.configurations;
        match configurations.iter_mut().find(|c| c.id == id) {
            Some(existing) => existing.motors = motors,
            None => configurations.push(Configuration {
                id: id.to_owned(),
                name: String::new(),
                motors,
            }),
        }
    }

    /// An hpr design's configuration: `--config`, or its only one; `None` if it has none, or
    /// several and none was named.
    fn hpr_configuration(&self, wanted: Option<&str>) -> Result<Option<&Configuration>, Failure> {
        let configurations = &self.rocket.configurations;
        match (wanted, &configurations[..]) {
            (Some(id), _) => self.rocket.configuration(id).map(Some).ok_or_else(|| {
                Failure::Input(format!(
                    "the design has no configuration `{id}`; its configurations: {}",
                    list(&owned(configurations.iter().map(|c| &c.id)))
                ))
            }),
            (None, [only]) => Ok(Some(only)),
            (None, _) => Ok(None),
        }
    }
}

/// Whether a motor of the user's own flies a configuration hpr left out for this reason: it
/// does when the reason is the file's motor, and not when it is the airframe, a stage or a
/// separation.
fn motor_fixes(why: ork::NotFlown) -> bool {
    matches!(
        why,
        ork::NotFlown::NoMotor
            | ork::NotFlown::NoCurve
            | ork::NotFlown::NoSize
            | ork::NotFlown::IgnitionNotFlown
    )
}

/// The refusal when no configuration was named and none can be taken as the one; `blocked`,
/// why no motor of the user's own flies the rocket either, if none does.
fn unchosen<'a>(
    ids: impl Iterator<Item = &'a String>,
    what: &str,
    blocked: Option<String>,
) -> Failure {
    let ids = owned(ids);
    if ids.is_empty() {
        Failure::Input(match blocked {
            None => format!("{what} has no motor configuration: give a motor with --motor"),
            Some(why) => {
                format!("{what} has no motor configuration, nor can it fly with --motor: {why}")
            }
        })
    } else {
        Failure::Input(format!(
            "{what} has {} configurations and names none the default: say which with --config \
             ({})",
            ids.len(),
            list(&ids)
        ))
    }
}

/// A `.ork` file's configuration: `--config` (its case ignored, as OpenRocket's ids are UUIDs),
/// or the file's default, or its only one; `None` if it has none, or several, none of them the
/// default, and none was named.
fn ork_configuration<'a>(
    motors: &'a ork::Motors,
    wanted: Option<&str>,
) -> Result<Option<&'a ork::MotorConfiguration>, Failure> {
    let configurations = &motors.configurations;
    match wanted {
        Some(id) => configurations
            .iter()
            .find(|c| c.id == id)
            .or_else(|| {
                configurations
                    .iter()
                    .find(|c| c.id.eq_ignore_ascii_case(id))
            })
            .map(Some)
            .ok_or_else(|| {
                Failure::Input(format!(
                    "the file has no motor configuration `{id}`; its configurations: {}",
                    list(&owned(configurations.iter().map(|c| &c.id)))
                ))
            }),
        None => Ok(
            match (motors.default_configuration(), &configurations[..]) {
                (Some(default), _) | (None, [default]) => Some(default),
                (None, _) => None,
            },
        ),
    }
}

/// What a powered separation is, in words.
fn separation_text(staging: &ork::Staging) -> String {
    format!(
        "stage {} drops away under power at {:.3} s, which hpr sim doesn't fly yet; the library \
         does, as its example ork_two_stage shows",
        staging.after_stage + 1,
        staging.time_s
    )
}

/// The refusal of a configuration whose stages come apart under power.
fn powered_separation(id: &str, staging: &ork::Staging) -> Failure {
    Failure::Input(format!(
        "configuration {id} separates under power: {}",
        separation_text(staging)
    ))
}

/// Every motor mount's component id, in the design's order.
fn mounts(rocket: &hpr_design::Rocket) -> Vec<String> {
    let mut mounts = Vec::new();
    walk(rocket, &mut |component| {
        if component.motor_mount.is_some() {
            mounts.push(component.id.clone());
        }
    });
    mounts
}

/// Every component's id and name.
fn component_names(rocket: &hpr_design::Rocket) -> Vec<(String, String)> {
    let mut names = Vec::new();
    walk(rocket, &mut |component| {
        names.push((component.id.clone(), component.name.clone()));
    });
    names
}

/// Calls `visit` on every component, attached parts after their parent's.
fn walk(rocket: &hpr_design::Rocket, visit: &mut dyn FnMut(&hpr_design::Component)) {
    fn each(components: &[hpr_design::Component], visit: &mut dyn FnMut(&hpr_design::Component)) {
        for component in components {
            visit(component);
            each(&component.children, visit);
        }
    }
    for stage in &rocket.stages {
        each(&stage.components, visit);
    }
}

/// Owned copies of ids, for a message or a comparison.
fn owned<'a>(ids: impl Iterator<Item = &'a String>) -> Vec<String> {
    ids.cloned().collect()
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

/// A `--motor`: a `.eng` or `.rse` file by its extension, with the reader's warnings, or a
/// catalog name.
fn motor(name: &str) -> Result<(Motor, SimMotorSource, Vec<InputWarning>), Failure> {
    let refused = |error: hpr::Error| Failure::Input(format!("{name}: {error}"));
    let Some(format) = MotorFile::of(name) else {
        let motor = Motor::from_catalog(name).map_err(refused)?;
        return Ok((motor, SimMotorSource::Catalog, Vec::new()));
    };
    let bytes = std::fs::read(name).map_err(|error| Failure::Input(format!("{name}: {error}")))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| Failure::Input(format!("{name}: not a text file in UTF-8")))?;
    let (motor, format, warnings) = match format {
        MotorFile::Eng => (
            Motor::from_eng(&text).map_err(refused)?,
            FileFormat::Eng,
            eng::parse(&text).map(|parsed| parsed.warnings),
        ),
        MotorFile::Rse => (
            Motor::from_rse(&text).map_err(refused)?,
            FileFormat::Rse,
            rse::parse(&text).map(|parsed| parsed.warnings),
        ),
    };
    let file = file_name(name);
    let warnings = warnings
        .unwrap_or_default()
        .iter()
        .map(|warning| motor_warning(&file, warning))
        .collect();
    Ok((motor, SimMotorSource::File { file, format }, warnings))
}

fn motor_warning(file: &str, warning: &ParseWarning) -> InputWarning {
    InputWarning {
        at: format!("{file}, line {}", warning.line),
        kind: match warning.kind {
            ReadWarning::Skipped => WarningKind::Skipped,
            ReadWarning::Dropped => WarningKind::Dropped,
            _ => WarningKind::Unusual,
        },
        message: warning.message.clone(),
    }
}

fn ork_warning(warning: &ork::Warning) -> InputWarning {
    InputWarning {
        at: warning.at.clone(),
        kind: match warning.kind {
            ork::WarningKind::Skipped => WarningKind::Skipped,
            ork::WarningKind::Dropped => WarningKind::Dropped,
            _ => WarningKind::Unusual,
        },
        message: warning.message.clone(),
    }
}

/// An export's format, from its path's extension.
fn export_format(path: &str) -> Result<ExportFormat, Failure> {
    let extension = Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("csv") => Ok(ExportFormat::Csv),
        Some("json") => Ok(ExportFormat::Json),
        Some("parquet") => Ok(ExportFormat::Parquet),
        Some("geojson") => Ok(ExportFormat::Geojson),
        Some("kml") => Ok(ExportFormat::Kml),
        _ => Err(Failure::Input(format!(
            "{path}: --export writes .csv, .json, .parquet, .geojson or .kml"
        ))),
    }
}

/// A caveat for a peak the fall from apogee set: with no recovery device flown, not a prediction.
fn in_fall(peak: &Peak) -> &'static str {
    if peak.after_apogee {
        ", in the fall: not a prediction"
    } else {
        ""
    }
}

/// A recording file's contents. The maps draw no landing point: with no recovery device flown,
/// where the rocket came down is not a prediction, and a pin on a map reads as one. Once
/// `hpr sim` flies recovery (#240), a flight that deploys one should keep its landings.
fn contents(
    format: ExportFormat,
    recorder: &Recorder,
    environment: &Environment,
    summary: &FlightSummary,
    name: &str,
) -> Result<Vec<u8>, hpr_sim::SimError> {
    let unpinned = FlightSummary {
        landing: None,
        body_landings: Vec::new(),
        ..summary.clone()
    };
    let summary = &unpinned;
    Ok(match format {
        ExportFormat::Csv => export::csv(recorder)?.into_bytes(),
        ExportFormat::Json => export::json(recorder)?.into_bytes(),
        ExportFormat::Parquet => export::parquet(recorder)?,
        ExportFormat::Geojson => {
            export::geojson(&export::track(recorder, environment.sim())?, summary)?.into_bytes()
        }
        ExportFormat::Kml => {
            export::kml(&export::track(recorder, environment.sim())?, summary, name)?.into_bytes()
        }
    })
}

/// When a motor lights, in words.
fn ignition(ignition: &Ignition) -> String {
    match ignition {
        Ignition::Launch => "at launch".to_owned(),
        Ignition::Time { time_s } => format!("{time_s} s after launch"),
        Ignition::Burnout { mount, delay_s } => {
            format!("{delay_s} s after the motor in {mount} burns out")
        }
        Ignition::Separation { delay_s } => format!("{delay_s} s after its stage separates"),
        Ignition::Never => "never".to_owned(),
        other => format!("{other:?}"),
    }
}

/// The library's summary, field for field, each peak marked if it came after apogee.
fn summary(summary: &FlightSummary) -> Summary {
    let apogee_s = summary.apogee.map(|apogee| apogee.time_s);
    let mark = |p: metrics::Peak| peak(p, apogee_s);
    Summary {
        termination: match summary.termination {
            hpr_sim::Termination::GroundHit => Termination::GroundHit,
            hpr_sim::Termination::NoLiftoff => Termination::NoLiftoff,
            hpr_sim::Termination::StalledOnRail => Termination::StalledOnRail,
            hpr_sim::Termination::TimeCap => Termination::TimeCap,
            hpr_sim::Termination::StepLimit => Termination::StepLimit,
            hpr_sim::Termination::Separated => Termination::Separated,
            _ => Termination::Other,
        },
        launch_height_m: summary.launch_height_m,
        rail_exit_speed_m_s: summary.rail_exit_speed_m_s.map(mark),
        apogee: summary.apogee.map(|apogee| Apogee {
            time_s: apogee.time_s,
            height_above_ground_m: apogee.height_above_ground_m,
            gain_m: apogee.gain_m,
        }),
        max_speed_m_s: summary.max_speed_m_s.map(mark),
        max_mach: summary.max_mach.map(mark),
        max_dynamic_pressure_pa: summary.max_dynamic_pressure_pa.map(mark),
        max_acceleration_m_s2: summary.max_acceleration_m_s2.map(mark),
        max_descent_acceleration_m_s2: summary.max_descent_acceleration_m_s2.map(mark),
        min_static_margin_cal: summary.min_static_margin_cal.map(mark),
        min_flight_margin_cal: summary.min_flight_margin_cal.map(mark),
        rail_exit_stability: summary.rail_exit_stability.map(|stability| Stability {
            time_s: stability.time_s,
            height_above_ground_m: stability.height_above_ground_m,
            dynamic_pressure_pa: stability.dynamic_pressure_pa,
            cg_station_m: stability.cg_station_m,
            reference_diameter_m: stability.reference_diameter_m,
            static_margin: margin(&stability.static_margin),
            flight_margin: margin(&stability.flight_margin),
        }),
        landing: summary.landing.as_ref().map(landing),
        body_landings: summary.body_landings.iter().map(landing).collect(),
    }
}

fn peak(peak: metrics::Peak, apogee_s: Option<f64>) -> Peak {
    Peak {
        value: peak.value,
        time_s: peak.time_s,
        height_above_ground_m: peak.height_above_ground_m,
        after_apogee: apogee_s.is_some_and(|apogee| peak.time_s > apogee),
    }
}

fn margin(margin: &metrics::Margin) -> Margin {
    Margin {
        mach: margin.mach,
        angle_of_attack_rad: margin.angle_of_attack_rad,
        normal_force_slope_per_rad: margin.normal_force_slope_per_rad,
        slope_magnitude_sum_per_rad: margin.slope_magnitude_sum_per_rad,
        pitch_moment_slope_per_rad: margin.pitch_moment_slope_per_rad,
        cp_station_m: margin.cp_station_m,
        margin_cal: margin.margin_cal,
    }
}

fn landing(landing: &metrics::Landing) -> Landing {
    Landing {
        body: landing.body,
        time_s: landing.time_s,
        latitude_deg: landing.latitude_deg,
        longitude_deg: landing.longitude_deg,
        east_m: landing.east_m,
        north_m: landing.north_m,
        distance_m: landing.distance_m,
        ground_hit_speed_m_s: landing.ground_hit_speed_m_s,
        descent_rate_m_s: landing.descent_rate_m_s,
    }
}

fn event(event: &hpr_sim::FlightEvent) -> SimEvent {
    use hpr_sim::EventKind as Kind;
    let (kind, index) = match event.kind {
        Kind::Liftoff => (EventKind::Liftoff, None),
        Kind::RailExit => (EventKind::RailExit, None),
        Kind::Burnout => (EventKind::Burnout, None),
        Kind::Apogee => (EventKind::Apogee, None),
        Kind::GroundHit => (EventKind::GroundHit, None),
        Kind::Separation => (EventKind::Separation, None),
        Kind::Trigger(i) => (EventKind::Trigger, Some(i)),
        Kind::Deployment(i) => (EventKind::Deployment, Some(i)),
        Kind::Release(i) => (EventKind::Release, Some(i)),
        Kind::Ejection(i) => (EventKind::Ejection, Some(i)),
        Kind::Shift(i) => (EventKind::Shift, Some(i)),
        Kind::MassRelease(i) => (EventKind::MassRelease, Some(i)),
        Kind::User(i) => (EventKind::User, Some(i)),
        Kind::Ignition(i) => (EventKind::Ignition, Some(i)),
        _ => (EventKind::Other, None),
    };
    SimEvent {
        kind,
        index,
        time_s: event.sample.time_s,
        height_above_ground_m: event.sample.height_above_ground_m,
        speed_m_s: event.sample.cg_velocity_enu_m_s.length(),
    }
}

/// An angle north or south, east or west: `32.99° N`, `106.97° W`.
fn hemisphere(value: f64, positive: &str, negative: &str) -> String {
    if value < 0.0 {
        format!("{}° {negative}", -value)
    } else {
        format!("{value}° {positive}")
    }
}

/// The text form: what flew, from where, what it leaves out, its events and its figures.
fn print(flight: &SimFlight, out: &mut dyn Write) -> io::Result<()> {
    let design = &flight.design;
    let named = |name: &str, id: &str| {
        if name.is_empty() {
            id.to_owned()
        } else {
            format!("\"{name}\" ({id})")
        }
    };
    writeln!(
        out,
        "{} ({}), configuration {}",
        design.name,
        design.file,
        named(&design.configuration_name, &design.configuration)
    )?;
    let others: Vec<String> = design
        .configurations
        .iter()
        .filter(|c| c.id != design.configuration)
        .map(|c| named(&c.name, &c.id))
        .collect();
    if !others.is_empty() {
        writeln!(out, "  its other configurations: {}", others.join(", "))?;
    }
    for motor in &flight.motors {
        let source = match &motor.source {
            SimMotorSource::Design => "the design's".to_owned(),
            SimMotorSource::Catalog => "from the bundled catalog".to_owned(),
            SimMotorSource::File { file, .. } => format!("from {file}"),
        };
        let unlit = if motor.unlit > 0 {
            format!(", {} never lit", motor.unlit)
        } else {
            String::new()
        };
        writeln!(
            out,
            "  {} × {} ({source}) in {}, lit {}{unlit}",
            motor.count,
            motor.designation,
            named(&motor.mount_name, &motor.mount),
            motor.ignition
        )?;
    }
    let launch = &flight.launch;
    let rail = if launch.inclination_deg == 90.0 {
        format!("a {} m vertical rail", launch.rail_length_m)
    } else {
        format!(
            "a {} m rail {}° above the horizon, leaning toward {}°",
            launch.rail_length_m, launch.inclination_deg, launch.heading_deg
        )
    };
    let wind = if launch.wind_speed_m_s == 0.0 {
        "calm air".to_owned()
    } else {
        format!(
            "a {} m/s wind from {}°",
            launch.wind_speed_m_s, launch.wind_from_deg
        )
    };
    writeln!(
        out,
        "  launched at {}, {}, {} m above sea level, from {rail}, in {wind}",
        hemisphere(launch.latitude_deg, "N", "S"),
        hemisphere(launch.longitude_deg, "E", "W"),
        launch.elevation_m,
    )?;
    writeln!(
        out,
        "See the Accuracy page before trusting these numbers: \
         https://nrdptel.github.io/hpr-sim/accuracy.html"
    )?;
    for note in &flight.notes {
        writeln!(out, "note: {note}")?;
    }
    for warning in &flight.warnings {
        writeln!(out, "warning: {}: {}", warning.at, warning.message)?;
    }
    writeln!(out)?;
    writeln!(
        out,
        "{:<18} {:>9} {:>10} {:>11}",
        "event", "time", "height", "speed"
    )?;
    for event in &flight.events {
        let name = match event.kind {
            EventKind::Liftoff => "liftoff",
            EventKind::RailExit => "rail exit",
            EventKind::Burnout => "burnout",
            EventKind::Apogee => "apogee",
            EventKind::GroundHit => "ground hit",
            EventKind::Trigger => "charge",
            EventKind::Deployment => "deployment",
            EventKind::Release => "release",
            EventKind::Separation => "separation",
            EventKind::Ejection => "ejection",
            EventKind::Shift => "mass shift",
            EventKind::MassRelease => "mass release",
            EventKind::User => "user event",
            EventKind::Ignition => "ignition",
            EventKind::Other => "other",
        };
        // `+ 0.0` prints a -0 as 0: the flight ends a hair below the ground.
        writeln!(
            out,
            "{name:<18} {:>7.2} s {:>8.1} m {:>7.1} m/s",
            event.time_s,
            event.height_above_ground_m.max(0.0) + 0.0,
            event.speed_m_s
        )?;
    }
    writeln!(
        out,
        "(heights are the centre of gravity's above the site; speeds are over the ground)"
    )?;
    writeln!(out)?;
    let summary = &flight.summary;
    if let Some(apogee) = &summary.apogee {
        writeln!(
            out,
            "apogee                {:.1} m above the site at {:.2} s",
            apogee.height_above_ground_m, apogee.time_s
        )?;
    }
    if let Some(speed) = &summary.max_speed_m_s {
        writeln!(
            out,
            "top speed             {:.1} m/s at {:.2} s{}",
            speed.value,
            speed.time_s,
            in_fall(speed)
        )?;
    }
    if let Some(mach) = &summary.max_mach {
        writeln!(
            out,
            "top Mach number       {:.3}{}",
            mach.value,
            in_fall(mach)
        )?;
    }
    if let Some(speed) = &summary.rail_exit_speed_m_s {
        writeln!(out, "rail exit speed       {:.1} m/s", speed.value)?;
    }
    if let Some(margin) = summary
        .rail_exit_stability
        .and_then(|stability| stability.static_margin.margin_cal)
    {
        writeln!(out, "static margin, rail   {margin:.2} calibres")?;
    }
    if let Some(margin) = &summary.min_static_margin_cal {
        writeln!(
            out,
            "least static margin   {:.2} calibres at {:.2} s, before apogee",
            margin.value, margin.time_s
        )?;
    }
    if let Some(landing) = &summary.landing {
        writeln!(
            out,
            "landing               {:.1} m from the pad at {:.2} s, at {:.1} m/s: with no \
             recovery device flown, not a prediction",
            landing.distance_m, landing.time_s, landing.ground_hit_speed_m_s
        )?;
    }
    let ended = match summary.termination {
        Termination::GroundHit => None,
        Termination::NoLiftoff => Some("the rocket never left the rail's foot"),
        Termination::StalledOnRail => Some("the rocket stalled on the rail"),
        Termination::TimeCap => Some("the flight reached its time limit before landing"),
        Termination::StepLimit => Some("the integrator reached its step limit before landing"),
        Termination::Separated => Some("the stack separated"),
        Termination::Other => Some("the flight ended in a way this build doesn't name"),
    };
    if let Some(ended) = ended {
        writeln!(out, "the flight ended: {ended}")?;
    }
    for export in &flight.exports {
        writeln!(out, "wrote {} ({} rows)", export.path, export.rows)?;
    }
    Ok(())
}
