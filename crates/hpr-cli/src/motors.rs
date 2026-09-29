//! `hpr motors`: the bundled catalog, and motor files.
//!
//! `list` prints the catalog's motors with ThrustCurve.org's stated figures. `show` works a
//! motor's figures out from its thrust curve with `hpr_motor`, the curve the simulator flies, and
//! for a catalog motor sets ThrustCurve.org's stated figures beside them.

use std::io::{self, Write};
use std::path::Path;

use hpr::hpr_motor::catalog::{Catalog, CatalogMotor, CurveFormat, MotorType, bundled_curve_text};
use hpr::hpr_motor::curve::ThrustCurve;
use hpr::hpr_motor::delay::{self, DelayList};
use hpr::hpr_motor::text::{ParseWarning, WarningKind as ReadWarning};
use hpr::hpr_motor::{ImpulseClass, eng, rse};

use crate::output::{
    CatalogInfo, Delay, FileFormat, ListedMotor, MotorFigures, MotorKind, MotorList, MotorShow,
    MotorSource, StatedFigures, Warning, WarningKind,
};
use crate::{Failure, Out};

/// `hpr motors`'s subcommands.
#[derive(Debug, clap::Subcommand)]
pub enum MotorsCommand {
    /// List the bundled catalog's motors, with ThrustCurve.org's stated figures
    List(ListArgs),
    /// Work out a motor's figures from its thrust curve: a catalog name, or a .eng or .rse file
    Show(ShowArgs),
}

/// `hpr motors list`'s filters. Each one given must match.
#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only this impulse class, such as H or 1/2A
    #[arg(long)]
    pub class: Option<String>,
    /// Only this casing diameter, mm (within 0.5 mm)
    #[arg(long, value_name = "MM", allow_negative_numbers = true)]
    pub diameter: Option<f64>,
    /// Only this manufacturer: its name or abbreviation, any case
    #[arg(long)]
    pub manufacturer: Option<String>,
}

/// `hpr motors show`'s argument.
#[derive(Debug, clap::Args)]
pub struct ShowArgs {
    /// A catalog motor's name (J760, 1266J760-19A), or the path of a .eng or .rse file
    pub motor: String,
}

/// A motor file format `hpr motors show` reads, by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotorFile {
    /// RASP `.eng`.
    Eng,
    /// RockSim `.rse`.
    Rse,
}

impl MotorFile {
    /// Every format read, in the order the command table lists them.
    pub const ALL: [Self; 2] = [Self::Eng, Self::Rse];

    /// The extension, with its dot.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Eng => ".eng",
            Self::Rse => ".rse",
        }
    }

    /// The format a path's extension names, in any case.
    pub fn of(path: &str) -> Option<Self> {
        let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
        Self::ALL
            .into_iter()
            .find(|format| format.extension()[1..] == extension)
    }

    fn output(self) -> FileFormat {
        match self {
            Self::Eng => FileFormat::Eng,
            Self::Rse => FileFormat::Rse,
        }
    }
}

/// Runs `hpr motors`.
pub(crate) fn run(command: &MotorsCommand, to: &mut Out<'_>) -> Result<(), Failure> {
    match command {
        MotorsCommand::List(args) => list(args, to),
        MotorsCommand::Show(args) => show(&args.motor, to),
    }
}

/// The bundled catalog.
fn catalog() -> Result<Catalog, Failure> {
    Catalog::bundled().map_err(|error| Failure::Input(format!("the bundled catalog: {error}")))
}

/// `hpr motors list`.
fn list(args: &ListArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    let class = match &args.class {
        Some(class) => Some(class_label(class)?),
        None => None,
    };
    if let Some(diameter) = args.diameter
        && !(diameter.is_finite() && diameter > 0.0)
    {
        return Err(Failure::Input(format!(
            "--diameter must be a positive number of millimetres, not {diameter}"
        )));
    }
    let catalog = catalog()?;
    let manufacturer = args.manufacturer.as_deref().map(str::to_lowercase);
    let motors = catalog
        .motors
        .iter()
        .filter(|motor| {
            class
                .as_ref()
                .is_none_or(|c| motor.impulse_class.label() == *c)
        })
        .filter(|motor| {
            args.diameter
                .is_none_or(|mm| (motor.diameter_mm - mm).abs() <= 0.5)
        })
        .filter(|motor| {
            manufacturer.as_ref().is_none_or(|name| {
                motor.manufacturer.to_lowercase() == *name
                    || motor.manufacturer_abbrev.to_lowercase() == *name
            })
        })
        .map(listed)
        .collect();
    let list = MotorList {
        catalog: CatalogInfo {
            source: catalog.source.clone(),
            captured: catalog.snapshot.captured.clone(),
            selection: catalog.selection.clone(),
        },
        motors,
    };
    to.emit(&list, |out| list_text(&list, out))
}

/// The class label `--class` names, as [`ImpulseClass::label`] writes it, or a refusal.
fn class_label(class: &str) -> Result<String, Failure> {
    let wanted = class.trim().to_ascii_uppercase();
    // Every class from 1/8A to Z, by each one's top impulse.
    let mut labels = (-2..=26).filter_map(|k| {
        ImpulseClass::from_total_impulse(1.25 * 2f64.powi(k))
            .ok()
            .map(ImpulseClass::label)
    });
    labels
        .find(|label| label.to_ascii_uppercase() == wanted)
        .ok_or_else(|| {
            Failure::Input(format!(
                "--class {class} is not an impulse class: use 1/8A, 1/4A, 1/2A or a letter A to Z"
            ))
        })
}

/// One catalog motor, as listed.
fn listed(motor: &CatalogMotor) -> ListedMotor {
    ListedMotor {
        designation: motor.designation.clone(),
        common_name: motor.common_name.clone(),
        manufacturer: motor.manufacturer.clone(),
        manufacturer_abbrev: motor.manufacturer_abbrev.clone(),
        impulse_class: motor.impulse_class.label(),
        motor_type: kind(motor.motor_type),
        diameter_mm: motor.diameter_mm,
        length_mm: motor.length_mm,
        total_impulse_ns: motor.total_impulse_ns,
        average_thrust_n: motor.average_thrust_n,
        burn_time_s: motor.burn_time_s,
        delays: motor.delays.clone(),
    }
}

fn kind(motor_type: MotorType) -> MotorKind {
    match motor_type {
        MotorType::SingleUse => MotorKind::SingleUse,
        MotorType::Reload => MotorKind::Reload,
        // `MotorType` is non-exhaustive; hybrids are the only other kind ThrustCurve lists.
        _ => MotorKind::Hybrid,
    }
}

/// `hpr motors list` as text: a table.
fn list_text(list: &MotorList, out: &mut dyn Write) -> io::Result<()> {
    writeln!(
        out,
        "{} motors from {}, captured {}; figures as ThrustCurve.org states them.",
        list.motors.len(),
        list.catalog.source,
        list.catalog.captured
    )?;
    if list.motors.is_empty() {
        return Ok(());
    }
    writeln!(out)?;
    let header = [
        "designation",
        "maker",
        "class",
        "dia mm",
        "len mm",
        "impulse N·s",
        "avg N",
        "burn s",
        "delays",
    ];
    let rows: Vec<[String; 9]> = list
        .motors
        .iter()
        .map(|m| {
            [
                m.designation.clone(),
                m.manufacturer_abbrev.clone(),
                m.impulse_class.clone(),
                format!("{}", m.diameter_mm),
                format!("{}", m.length_mm),
                format!("{:.1}", m.total_impulse_ns),
                format!("{:.1}", m.average_thrust_n),
                format!("{:.2}", m.burn_time_s),
                m.delays.clone().unwrap_or_else(|| "-".to_owned()),
            ]
        })
        .collect();
    let mut widths = header.map(|h| h.chars().count());
    for row in &rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let line = |out: &mut dyn Write, cells: &[&str]| -> io::Result<()> {
        let mut text = String::new();
        for (i, (cell, width)) in cells.iter().zip(widths).enumerate() {
            let pad = width - cell.chars().count();
            // Text columns align left, numbers right; the last column isn't padded.
            if i == cells.len() - 1 {
                text.push_str(cell);
            } else if (3..=7).contains(&i) {
                text.push_str(&" ".repeat(pad));
                text.push_str(cell);
                text.push_str("  ");
            } else {
                text.push_str(cell);
                text.push_str(&" ".repeat(pad + 2));
            }
        }
        writeln!(out, "{}", text.trim_end())
    };
    line(out, &header)?;
    for row in &rows {
        line(out, &row.each_ref().map(String::as_str))?;
    }
    Ok(())
}

/// `hpr motors show`.
fn show(motor: &str, to: &mut Out<'_>) -> Result<(), Failure> {
    let show = match MotorFile::of(motor) {
        Some(format) => from_file(motor, format)?,
        None => from_catalog(motor)?,
    };
    to.emit(&show, |out| show_text(&show, out))
}

/// Every catalog motor `name` matches, with its bundled curve.
fn from_catalog(name: &str) -> Result<MotorShow, Failure> {
    let catalog = catalog()?;
    let matches: Vec<&CatalogMotor> = catalog.find(name).collect();
    if matches.is_empty() {
        return Err(Failure::Input(format!(
            "no motor in the bundled catalog is called {name}; `hpr motors list` lists them, \
             and a .eng or .rse file can be shown by its path"
        )));
    }
    let mut show = MotorShow {
        motors: Vec::new(),
        warnings: Vec::new(),
    };
    for motor in matches {
        let solid = motor
            .bundled_motor()
            .map_err(|error| Failure::Input(format!("{}: {error}", motor.designation)))?;
        // The curve `bundled_motor` flew: the first with a bundled file, which it just found.
        let Some(curve) = motor
            .curves
            .iter()
            .find(|curve| bundled_curve_text(&curve.file).is_some())
        else {
            continue;
        };
        let format = match curve.format {
            CurveFormat::Rasp => MotorFile::Eng,
            CurveFormat::RockSim => MotorFile::Rse,
            // `CurveFormat` is non-exhaustive, and the bundled catalog uses these two only.
            _ => {
                return Err(Failure::Input(format!(
                    "{}: its curve's format isn't .eng or .rse",
                    motor.designation
                )));
            }
        };
        let delays = motor.delays();
        show.warnings
            .extend(delay_warnings(&motor.designation, &delays));
        show.motors.push(figures(
            &motor.designation,
            &motor.manufacturer,
            MotorSource::Catalog {
                curve_url: curve.info_url.clone().unwrap_or_else(|| curve.url.clone()),
                format: format.output(),
            },
            Envelope {
                diameter_m: motor.diameter_mm / 1000.0,
                length_m: motor.length_mm / 1000.0,
                propellant_mass_kg: solid.propellant_initial_mass_kg(),
                loaded_mass_kg: solid.propellant_initial_mass_kg() + solid.dry().mass_kg,
            },
            solid.curve(),
            &delays,
            Some(StatedFigures {
                impulse_class: motor.impulse_class.label(),
                total_impulse_ns: motor.total_impulse_ns,
                average_thrust_n: motor.average_thrust_n,
                max_thrust_n: motor.max_thrust_n,
                burn_time_s: motor.burn_time_s,
            }),
        )?);
    }
    Ok(show)
}

/// Every motor in a `.eng` or `.rse` file.
fn from_file(path: &str, format: MotorFile) -> Result<MotorShow, Failure> {
    let bytes = std::fs::read(path).map_err(|error| Failure::Input(format!("{path}: {error}")))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| Failure::Input(format!("{path}: not a text file in UTF-8")))?;
    let source = || MotorSource::File {
        path: path.to_owned(),
        format: format.output(),
    };
    let refused = |error| Failure::Input(format!("{path}: {error}"));
    let mut show = MotorShow {
        motors: Vec::new(),
        warnings: Vec::new(),
    };
    match format {
        MotorFile::Eng => {
            let parsed = eng::parse(&text).map_err(refused)?;
            show.warnings.extend(read_warnings(&parsed.warnings));
            for entry in &parsed.value.entries {
                let curve = entry
                    .thrust_curve()
                    .map_err(|error| Failure::Input(format!("{path}: {}: {error}", entry.name)))?;
                let delays = entry.delays();
                show.warnings.extend(delay_warnings(&entry.name, &delays));
                show.motors.push(figures(
                    &entry.name,
                    &entry.manufacturer,
                    source(),
                    Envelope {
                        diameter_m: entry.diameter_mm / 1000.0,
                        length_m: entry.length_mm / 1000.0,
                        propellant_mass_kg: entry.propellant_mass_kg,
                        loaded_mass_kg: entry.total_mass_kg,
                    },
                    &curve,
                    &delays,
                    None,
                )?);
            }
        }
        MotorFile::Rse => {
            let parsed = rse::parse(&text).map_err(refused)?;
            show.warnings.extend(read_warnings(&parsed.warnings));
            for engine in &parsed.value.engines {
                let curve = engine
                    .thrust_curve()
                    .map_err(|error| Failure::Input(format!("{path}: {}: {error}", engine.code)))?;
                let delays = engine.delays();
                show.warnings.extend(delay_warnings(&engine.code, &delays));
                show.motors.push(figures(
                    &engine.code,
                    engine.manufacturer.trim(),
                    source(),
                    Envelope {
                        diameter_m: engine.diameter_mm / 1000.0,
                        length_m: engine.length_mm / 1000.0,
                        propellant_mass_kg: engine.propellant_mass_g / 1000.0,
                        loaded_mass_kg: engine.initial_mass_g / 1000.0,
                    },
                    &curve,
                    &delays,
                    None,
                )?);
            }
        }
    }
    if show.motors.is_empty() {
        return Err(Failure::Input(format!("{path}: no motor could be read")));
    }
    Ok(show)
}

/// A motor's casing and masses, SI.
struct Envelope {
    diameter_m: f64,
    length_m: f64,
    propellant_mass_kg: f64,
    loaded_mass_kg: f64,
}

/// A motor's figures from its curve.
fn figures(
    name: &str,
    manufacturer: &str,
    source: MotorSource,
    envelope: Envelope,
    curve: &ThrustCurve,
    delays: &DelayList,
    stated: Option<StatedFigures>,
) -> Result<MotorFigures, Failure> {
    let total_impulse_ns = curve.total_impulse_ns();
    let impulse_class = ImpulseClass::from_total_impulse(total_impulse_ns)
        .map_err(|error| Failure::Input(format!("{name}: {error}")))?
        .label();
    let (burn_start_s, burn_end_s) = curve.burn_window_s();
    Ok(MotorFigures {
        name: name.to_owned(),
        manufacturer: manufacturer.to_owned(),
        source,
        diameter_m: envelope.diameter_m,
        length_m: envelope.length_m,
        propellant_mass_kg: envelope.propellant_mass_kg,
        loaded_mass_kg: envelope.loaded_mass_kg,
        total_impulse_ns,
        impulse_class,
        average_thrust_n: curve.average_thrust_n(),
        peak_thrust_n: curve.peak_thrust_n(),
        burn_time_s: curve.burn_time_s(),
        burn_start_s,
        burn_end_s,
        curve_end_s: curve.end_time_s(),
        delays: delays.delays.iter().map(|&d| delay_out(d)).collect(),
        stated,
    })
}

fn delay_out(delay: delay::Delay) -> Delay {
    match delay {
        delay::Delay::Seconds(s) => Delay::Seconds(s),
        delay::Delay::Plugged => Delay::Plugged,
        // `Delay` is non-exhaustive; the ambiguous `0` is its only other setting.
        _ => Delay::ZeroOrPlugged,
    }
}

fn warning_kind(kind: ReadWarning) -> WarningKind {
    match kind {
        ReadWarning::Skipped => WarningKind::Skipped,
        ReadWarning::Dropped => WarningKind::Dropped,
        // `WarningKind` is non-exhaustive; `Unusual` is its only other kind.
        _ => WarningKind::Unusual,
    }
}

fn read_warnings(warnings: &[ParseWarning]) -> impl Iterator<Item = Warning> + '_ {
    warnings.iter().map(|warning| Warning {
        motor: None,
        line: Some(warning.line),
        kind: warning_kind(warning.kind),
        message: warning.message.clone(),
    })
}

fn delay_warnings<'a>(motor: &'a str, delays: &'a DelayList) -> impl Iterator<Item = Warning> + 'a {
    delays.warnings.iter().map(move |warning| Warning {
        motor: Some(motor.to_owned()),
        line: None,
        kind: warning_kind(warning.kind),
        message: warning.message.clone(),
    })
}

/// `hpr motors show` as text.
fn show_text(show: &MotorShow, out: &mut dyn Write) -> io::Result<()> {
    for (i, motor) in show.motors.iter().enumerate() {
        if i > 0 {
            writeln!(out)?;
        }
        let from = match &motor.source {
            MotorSource::Catalog { .. } => "the bundled catalog".to_owned(),
            MotorSource::File { path, .. } => path.clone(),
        };
        writeln!(out, "{} ({}), from {from}", motor.name, motor.manufacturer)?;
        writeln!(out, "  impulse class    {}", motor.impulse_class)?;
        writeln!(out, "  total impulse    {:.1} N·s", motor.total_impulse_ns)?;
        writeln!(out, "  average thrust   {:.1} N", motor.average_thrust_n)?;
        writeln!(out, "  peak thrust      {:.1} N", motor.peak_thrust_n)?;
        writeln!(
            out,
            "  burn time        {:.2} s, from {:.3} s to {:.3} s (NFPA 1125, 5% of peak)",
            motor.burn_time_s, motor.burn_start_s, motor.burn_end_s
        )?;
        writeln!(
            out,
            "  propellant       {:.1} g of {:.1} g loaded",
            motor.propellant_mass_kg * 1e3,
            motor.loaded_mass_kg * 1e3
        )?;
        writeln!(
            out,
            "  casing           {} mm across, {} mm long",
            round_mm(motor.diameter_m),
            round_mm(motor.length_m)
        )?;
        writeln!(out, "  delays           {}", delays_text(&motor.delays))?;
        if let Some(stated) = &motor.stated {
            let peak = stated
                .max_thrust_n
                .map_or_else(String::new, |n| format!(", {n} N peak"));
            writeln!(
                out,
                "  ThrustCurve.org  {} class, {} N·s, {} N average{peak}, {} s burn",
                stated.impulse_class,
                stated.total_impulse_ns,
                stated.average_thrust_n,
                stated.burn_time_s
            )?;
        }
    }
    for warning in &show.warnings {
        let at = match (&warning.motor, warning.line) {
            (Some(motor), _) => format!("{motor}: "),
            (None, Some(line)) => format!("line {line}: "),
            (None, None) => String::new(),
        };
        writeln!(out, "warning: {at}{}", warning.message)?;
    }
    Ok(())
}

/// A length in millimetres, to a tenth and without a trailing `.0`.
fn round_mm(length_m: f64) -> String {
    let mm = (length_m * 1e4).round() / 10.0;
    format!("{mm}")
}

fn delays_text(delays: &[Delay]) -> String {
    if delays.is_empty() {
        return "none listed".to_owned();
    }
    delays
        .iter()
        .map(|delay| match delay {
            Delay::Seconds(s) => format!("{s} s"),
            Delay::Plugged => "plugged".to_owned(),
            Delay::ZeroOrPlugged => "0 (at burnout, or plugged)".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}
