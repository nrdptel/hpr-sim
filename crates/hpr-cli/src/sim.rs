//! `hpr sim`: a design flown from a rail, its summary printed and its recording exported.
//!
//! The flight is the library's own: the design becomes an [`hpr::Rocket`] with
//! [`hpr::Rocket::from_design`], and [`hpr::Flight::builder`] flies it, so what this prints is
//! what a program calling the facade gets, number for number. The stack flies whole: no recovery
//! device and no separation is flown yet, which the output's notes say.

use std::io::{self, Write};
use std::path::Path;

use hpr::hpr_design::{self, Configuration, Ignition, MountedMotor, checks};
use hpr::hpr_io::ork;
use hpr::hpr_sim::metrics::{self, FlightSummary};
use hpr::hpr_sim::{self, Channel, FlightSettings, Recorder, export};
use hpr::{Environment, Flight, Motor, Rocket};

use crate::motors::MotorFile;
use crate::output::{
    Apogee, DesignFormat, DesignWarning, EventKind, Export, ExportFormat, FileFormat, Landing,
    Launch, Margin, Peak, SimDesign, SimEvent, SimFlight, SimMotor, SimMotorSource, Stability,
    Summary, Termination, WarningKind,
};
use crate::{Failure, Out};

/// The rail's length when `--rail-length` isn't given, m.
pub const DEFAULT_RAIL_LENGTH_M: f64 = 1.5;

/// The recording's interval when `--interval` isn't given, s.
pub const DEFAULT_INTERVAL_S: f64 = 0.01;

/// `hpr sim`'s arguments.
#[derive(Debug, clap::Args)]
pub struct SimArgs {
    /// The design: an OpenRocket .ork file, or an hpr design file (.json)
    pub design: String,
    /// The motor configuration to fly, by its id [default: the design's default, or its only one]
    #[arg(long, value_name = "ID")]
    pub config: Option<String>,
    /// Fly this motor in place of the configuration's: a catalog designation or common name, or
    /// a .eng or .rse file
    #[arg(long, value_name = "NAME_OR_FILE")]
    pub motor: Option<String>,
    /// The motor mount, by its component id, that --motor goes in, where the configuration
    /// doesn't say
    #[arg(long, value_name = "ID", requires = "motor")]
    pub mount: Option<String>,
    /// The launch site's latitude, degrees north
    #[arg(
        long,
        value_name = "DEG",
        default_value_t = 0.0,
        allow_negative_numbers = true
    )]
    pub latitude: f64,
    /// The launch site's longitude, degrees east (negative west)
    #[arg(
        long,
        value_name = "DEG",
        default_value_t = 0.0,
        allow_negative_numbers = true
    )]
    pub longitude: f64,
    /// The launch site's elevation above sea level, m
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
    #[arg(long, value_name = "DEG", default_value_t = 0.0, requires = "wind")]
    pub wind_from: f64,
    /// Write the flight's recording to this file: .csv, .json, .parquet, .geojson or .kml
    /// (repeat for several)
    #[arg(long, value_name = "FILE")]
    pub export: Vec<String>,
    /// The recording's interval, s; a row is also recorded at every event
    #[arg(long, value_name = "S", default_value_t = DEFAULT_INTERVAL_S)]
    pub interval: f64,
    /// Fly a design whose checks find errors, such as a motor wider than its mount
    #[arg(long)]
    pub accept_design_errors: bool,
}

/// Runs `hpr sim`.
pub(crate) fn run(args: &SimArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    // Everything that can be refused is refused before the flight: an export's format, the
    // design, the motor, the site.
    let exports = args
        .export
        .iter()
        .map(|path| export_format(path).map(|format| (path.as_str(), format)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut read = read_design(&args.design)?;
    let (configuration, motor_source) = match &args.motor {
        Some(name) => {
            let (motor, source) = motor(name)?;
            let id = read.swap(args.config.as_deref(), args.mount.as_deref(), &motor)?;
            (id, source)
        }
        None => (read.flown(args.config.as_deref())?, SimMotorSource::Design),
    };
    let design_name = read.rocket.name.clone();
    let flown_motors = read
        .rocket
        .configuration(&configuration)
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
    let rocket = Rocket::from_design(read.rocket.clone(), &configuration).map_err(input)?;
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
        let findings = checks::check(rocket.design()).map_err(|error| {
            Failure::Input(format!("the design doesn't hold together: {error}"))
        })?;
        let errors: Vec<_> = findings
            .into_iter()
            .filter(|finding| finding.severity() == checks::Severity::Error)
            .collect();
        if !errors.is_empty() {
            read.notes.push(format!(
                "flown with --accept-design-errors past {}, so its numbers are not a buildable \
                 rocket's: {}",
                count(errors.len(), "error"),
                findings_text(&errors)
            ));
        }
    }
    let (flight, recorder) = if exports.is_empty() {
        (builder.fly().map_err(input)?, None)
    } else {
        let mut recorder = Recorder::new(Channel::ALL.to_vec(), Some(args.interval))
            .map_err(|error| Failure::Input(format!("--interval: {error}")))?;
        let flight = builder.fly_with(&mut recorder).map_err(input)?;
        (flight, Some(recorder))
    };

    let mut written = Vec::new();
    if let Some(recorder) = &recorder {
        let name = format!("{design_name}, {configuration}");
        for (path, format) in &exports {
            let contents = contents(*format, recorder, &environment, flight.summary(), &name)
                .map_err(|error| Failure::Input(format!("{path}: {error}")))?;
            std::fs::write(path, contents)
                .map_err(|error| Failure::Input(format!("{path}: {error}")))?;
            written.push(Export {
                path: (*path).to_owned(),
                format: *format,
                rows: recorder.rows().len(),
            });
        }
    }

    let document = SimFlight {
        design: SimDesign {
            file: file_name(&args.design),
            format: read.format,
            name: design_name,
            configuration,
        },
        motors: flown_motors
            .iter()
            .map(|flown| SimMotor {
                designation: flown.designation.clone(),
                mount: flown.mount.clone(),
                source: motor_source.clone(),
                ignition: ignition(&flown.ignition),
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

/// A library error, as the command reports it: a design's failed checks in full, with the way
/// past them.
fn input(error: hpr::Error) -> Failure {
    match error {
        hpr::Error::Sim(hpr_sim::SimError::DesignChecks(findings)) => Failure::Input(format!(
            "the design's checks found {}, which hpr sim flies only with \
             --accept-design-errors: {}",
            count(findings.len(), "error"),
            findings_text(&findings)
        )),
        error => Failure::Input(error.to_string()),
    }
}

/// Findings as their JSON, one after another.
fn findings_text(findings: &[checks::Finding]) -> String {
    findings
        .iter()
        .map(|finding| serde_json::to_string(finding).unwrap_or_else(|_| format!("{finding:?}")))
        .collect::<Vec<_>>()
        .join("; ")
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
    warnings: Vec<DesignWarning>,
}

/// Reads a `.ork` or an hpr design file, by its extension.
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
            let mut notes = Vec::new();
            let devices = design.recovery.devices.len();
            if devices > 0 {
                notes.push(format!(
                    "the file's {} {} not flown yet: the rocket comes down on its airframe \
                     alone, so its landing speed, landing time and drift are not a recovered \
                     flight's",
                    count(devices, "recovery device"),
                    if devices == 1 { "is" } else { "are" },
                ));
            } else {
                notes.push(NO_RECOVERY.to_owned());
            }
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
                warnings: warnings.iter().map(design_warning).collect(),
            })
        }
        Some("json") => {
            let rocket: hpr_design::Rocket =
                serde_json::from_slice(&bytes()?).map_err(|error| {
                    Failure::Input(format!("{path}: not an hpr design file: {error}"))
                })?;
            let mut notes = vec![
                "an hpr design file holds no recovery devices, so none is flown: the rocket \
                 comes down on its airframe alone, and its landing speed, landing time and drift \
                 are not a recovered flight's"
                    .to_owned(),
            ];
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
            "{path}: hpr sim reads an OpenRocket .ork file or an hpr design file (.json)"
        ))),
    }
}

/// The note for a `.ork` with no recovery device.
const NO_RECOVERY: &str = "the file has no recovery device, so none is flown: the rocket comes down on its airframe alone";

/// The note for a design of more than one stage.
const WHOLE_STACK: &str =
    "the stages fly as one stack: hpr sim flies no separation yet, so none comes apart";

impl Read {
    /// The configuration flown as the design has it: `--config`, or the default or only one.
    fn flown(&self, wanted: Option<&str>) -> Result<String, Failure> {
        let id = match &self.ork {
            Some(motors) => {
                let chosen = ork_configuration(motors, wanted)?.ok_or_else(|| {
                    unchosen(motors.configurations.iter().map(|c| &c.id), "the file")
                })?;
                if let Some(staging) = &chosen.staging {
                    return Err(powered_separation(&chosen.id, staging));
                }
                if let Some(left_out) = &chosen.left_out {
                    return Err(Failure::Input(format!(
                        "configuration {} can't be flown as the file has it: {}{}",
                        chosen.id,
                        left_out.message,
                        if motor_fixes(left_out.why) {
                            "; give a motor with --motor"
                        } else {
                            ""
                        }
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
                    )
                })?
                .id
                .clone(),
        };
        // A configuration the reader didn't leave out is among the rocket's; this is its check.
        if self.rocket.configuration(&id).is_none() {
            return Err(Failure::Input(format!(
                "configuration {id} can't be flown as the file has it; give a motor with --motor"
            )));
        }
        Ok(id)
    }

    /// Puts `motor`, lit at launch, in the configuration `--config` names, or the default or
    /// only one, or where there is none to choose, in a configuration named after the motor; and
    /// returns that configuration's id. A `.ork` configuration hpr leaves out for its airframe,
    /// its stages or its separation is refused: a motor of its own doesn't make that rocket
    /// flyable.
    fn swap(
        &mut self,
        wanted: Option<&str>,
        mount: Option<&str>,
        motor: &Motor,
    ) -> Result<String, Failure> {
        if let Some(why) = &self.airframe {
            return Err(Failure::Input(format!(
                "the rocket can't be flown with any motor: the airframe was not read exactly as \
                 written: {why}"
            )));
        }
        let chosen: Option<(String, Vec<String>)> = match &self.ork {
            Some(motors) => match ork_configuration(motors, wanted)? {
                Some(chosen) => {
                    if let Some(staging) = &chosen.staging {
                        return Err(powered_separation(&chosen.id, staging));
                    }
                    if let Some(left_out) = &chosen.left_out
                        && !motor_fixes(left_out.why)
                    {
                        return Err(Failure::Input(format!(
                            "configuration {} can't be flown with any motor: {}",
                            chosen.id, left_out.message
                        )));
                    }
                    Some((
                        chosen.id.clone(),
                        mounts_of(chosen.motors.iter().map(|m| &m.mount)),
                    ))
                }
                None => None,
            },
            None => self.hpr_configuration(wanted)?.map(|chosen| {
                (
                    chosen.id.clone(),
                    mounts_of(chosen.motors.iter().map(|m| &m.mount)),
                )
            }),
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
                "configuration {id} has {} motors, in {}; --motor flies one motor on its own",
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
                    list(&mounts_of(configurations.iter().map(|c| &c.id)))
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

/// The refusal when no configuration was named and none can be taken as the one.
fn unchosen<'a>(ids: impl Iterator<Item = &'a String>, what: &str) -> Failure {
    let ids = mounts_of(ids);
    if ids.is_empty() {
        Failure::Input(format!(
            "{what} has no motor configuration: give a motor with --motor"
        ))
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
                    list(&mounts_of(configurations.iter().map(|c| &c.id)))
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

/// The refusal of a configuration whose stages come apart under power.
fn powered_separation(id: &str, staging: &ork::Staging) -> Failure {
    Failure::Input(format!(
        "configuration {id} separates under power (stage {} drops away at {:.3} s), which hpr \
         sim doesn't fly yet; the library does, as its example ork_two_stage shows",
        staging.after_stage + 1,
        staging.time_s
    ))
}

/// Every motor mount's component id, in the design's order.
fn mounts(rocket: &hpr_design::Rocket) -> Vec<String> {
    fn walk(components: &[hpr_design::Component], mounts: &mut Vec<String>) {
        for component in components {
            if component.motor_mount.is_some() {
                mounts.push(component.id.clone());
            }
            walk(&component.children, mounts);
        }
    }
    let mut mounts = Vec::new();
    for stage in &rocket.stages {
        walk(&stage.components, &mut mounts);
    }
    mounts
}

/// Owned copies of ids or mounts, for a message or a comparison.
fn mounts_of<'a>(ids: impl Iterator<Item = &'a String>) -> Vec<String> {
    ids.cloned().collect()
}

fn list(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_owned()
    } else {
        items.join(", ")
    }
}

/// A `--motor`: a `.eng` or `.rse` file by its extension, or a catalog name.
fn motor(name: &str) -> Result<(Motor, SimMotorSource), Failure> {
    let refused = |error: hpr::Error| Failure::Input(format!("{name}: {error}"));
    match MotorFile::of(name) {
        Some(format) => {
            let bytes =
                std::fs::read(name).map_err(|error| Failure::Input(format!("{name}: {error}")))?;
            let text = String::from_utf8(bytes)
                .map_err(|_| Failure::Input(format!("{name}: not a text file in UTF-8")))?;
            let (motor, format) = match format {
                MotorFile::Eng => (Motor::from_eng(&text).map_err(refused)?, FileFormat::Eng),
                MotorFile::Rse => (Motor::from_rse(&text).map_err(refused)?, FileFormat::Rse),
            };
            let source = SimMotorSource::File {
                file: file_name(name),
                format,
            };
            Ok((motor, source))
        }
        None => Ok((
            Motor::from_catalog(name).map_err(refused)?,
            SimMotorSource::Catalog,
        )),
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

/// A recording file's contents.
fn contents(
    format: ExportFormat,
    recorder: &Recorder,
    environment: &Environment,
    summary: &FlightSummary,
    name: &str,
) -> Result<Vec<u8>, hpr_sim::SimError> {
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
        Ignition::Launch => "launch".to_owned(),
        Ignition::Time { time_s } => format!("{time_s} s after launch"),
        Ignition::Burnout { mount, delay_s } => {
            format!("{delay_s} s after the motor in {mount} burns out")
        }
        Ignition::Separation { delay_s } => format!("{delay_s} s after its stage separates"),
        Ignition::Never => "never".to_owned(),
        other => format!("{other:?}"),
    }
}

fn design_warning(warning: &ork::Warning) -> DesignWarning {
    DesignWarning {
        at: warning.at.clone(),
        kind: match warning.kind {
            ork::WarningKind::Skipped => WarningKind::Skipped,
            ork::WarningKind::Dropped => WarningKind::Dropped,
            _ => WarningKind::Unusual,
        },
        message: warning.message.clone(),
    }
}

/// The library's summary, field for field.
fn summary(summary: &FlightSummary) -> Summary {
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
        rail_exit_speed_m_s: summary.rail_exit_speed_m_s.map(peak),
        apogee: summary.apogee.map(|apogee| Apogee {
            time_s: apogee.time_s,
            height_above_ground_m: apogee.height_above_ground_m,
            gain_m: apogee.gain_m,
        }),
        max_speed_m_s: summary.max_speed_m_s.map(peak),
        max_mach: summary.max_mach.map(peak),
        max_dynamic_pressure_pa: summary.max_dynamic_pressure_pa.map(peak),
        max_acceleration_m_s2: summary.max_acceleration_m_s2.map(peak),
        max_descent_acceleration_m_s2: summary.max_descent_acceleration_m_s2.map(peak),
        min_static_margin_cal: summary.min_static_margin_cal.map(peak),
        min_flight_margin_cal: summary.min_flight_margin_cal.map(peak),
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

fn peak(peak: metrics::Peak) -> Peak {
    Peak {
        value: peak.value,
        time_s: peak.time_s,
        height_above_ground_m: peak.height_above_ground_m,
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

/// The text form: what flew, from where, its events and its metrics.
fn print(flight: &SimFlight, out: &mut dyn Write) -> io::Result<()> {
    let design = &flight.design;
    writeln!(
        out,
        "{} ({}), configuration {}",
        design.name, design.file, design.configuration
    )?;
    for motor in &flight.motors {
        let source = match &motor.source {
            SimMotorSource::Design => "the design's".to_owned(),
            SimMotorSource::Catalog => "from the bundled catalog".to_owned(),
            SimMotorSource::File { file, .. } => format!("from {file}"),
        };
        writeln!(
            out,
            "  {} ({source}) in {}, lit at {}",
            motor.designation, motor.mount, motor.ignition
        )?;
    }
    let launch = &flight.launch;
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
        "  launched at {}° N, {}° E, {} m up, from a {} m rail at {}° heading {}°, in {wind} and \
         the standard atmosphere",
        launch.latitude_deg,
        launch.longitude_deg,
        launch.elevation_m,
        launch.rail_length_m,
        launch.inclination_deg,
        launch.heading_deg
    )?;
    writeln!(
        out,
        "See the Accuracy page before trusting these numbers: \
         https://nrdptel.github.io/hpr-sim/accuracy.html"
    )?;
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
    writeln!(out)?;
    let summary = &flight.summary;
    if let Some(apogee) = &summary.apogee {
        writeln!(
            out,
            "apogee            {:.1} m above the site at {:.2} s",
            apogee.height_above_ground_m, apogee.time_s
        )?;
    }
    if let Some(speed) = &summary.max_speed_m_s {
        writeln!(
            out,
            "top speed         {:.1} m/s at {:.2} s",
            speed.value, speed.time_s
        )?;
    }
    if let Some(mach) = &summary.max_mach {
        writeln!(out, "top Mach          {:.3}", mach.value)?;
    }
    if let Some(speed) = &summary.rail_exit_speed_m_s {
        writeln!(out, "rail exit speed   {:.1} m/s", speed.value)?;
    }
    if let Some(margin) = summary
        .rail_exit_stability
        .and_then(|stability| stability.static_margin.margin_cal)
    {
        writeln!(out, "margin off rail   {margin:.2} calibres")?;
    }
    if let Some(margin) = &summary.min_static_margin_cal {
        writeln!(
            out,
            "least margin      {:.2} calibres at {:.2} s",
            margin.value, margin.time_s
        )?;
    }
    if let Some(landing) = &summary.landing {
        writeln!(
            out,
            "landing           {:.1} m from the pad at {:.2} s, at {:.1} m/s",
            landing.distance_m, landing.time_s, landing.ground_hit_speed_m_s
        )?;
    }
    if summary.termination != Termination::GroundHit {
        writeln!(out, "the flight ended: {:?}", summary.termination)?;
    }
    for export in &flight.exports {
        writeln!(out, "wrote {} ({} rows)", export.path, export.rows)?;
    }
    for note in &flight.notes {
        writeln!(out, "note: {note}")?;
    }
    for warning in &flight.warnings {
        writeln!(out, "warning: {}: {}", warning.at, warning.message)?;
    }
    Ok(())
}
