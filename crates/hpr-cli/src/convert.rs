//! `hpr convert`: a motor file written as a RASP `.eng` or a RockSim `.rse` file.
//!
//! The conversion is the library's own ([`hpr::hpr_motor::convert`]): `.eng` to `.rse` fills
//! what `.eng` doesn't give the way RockSim's files do, and `.rse` to `.eng` drops what `.eng`
//! can't hold, each named in a warning. The same format in and out rewrites the file in hpr's
//! layout. What the reader flagged in the input is passed on.

use std::io::{self, Write};
use std::path::Path;

use hpr::hpr_motor::catalog::{CatalogMotor, CurveFormat, bundled_curve_text};
use hpr::hpr_motor::convert::{self, ConvertWarning};
use hpr::hpr_motor::eng::{self, EngFile};
use hpr::hpr_motor::rse::{self, RseFile};

use crate::motors::{MotorFile, catalog, read_warnings, warning_kind};
use crate::output::{Convert, ConvertedFile, MotorSource, Warning};
use crate::sim::same_file;
use crate::{Failure, Out};

/// `hpr convert`'s arguments.
#[derive(Debug, clap::Args)]
pub struct ConvertArgs {
    /// The motors to read: a .eng or .rse file, or a catalog motor's name, such as H170M
    pub input: String,
    /// The file to write, .eng or .rse by its extension; it is replaced if it exists
    pub output: String,
    /// Delays for a .rse motor that gives none, when writing a .eng file, whose header must
    /// give them: such as 6-10-14, or P for plugged
    #[arg(long, value_name = "LIST")]
    pub delays: Option<String>,
}

/// The motors read, in their file's model.
enum Motors {
    Eng(EngFile),
    Rse(RseFile),
}

/// Runs `hpr convert`.
pub(crate) fn run(args: &ConvertArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    let output = &args.output;
    let target = MotorFile::of(output).ok_or_else(|| {
        Failure::Input(format!(
            "{output}: hpr convert writes a .eng or a .rse file, named by its extension"
        ))
    })?;
    let (source, format, text) = read_input(&args.input)?;
    if matches!(source, MotorSource::File { .. }) && same_file(&args.input) == same_file(output) {
        return Err(Failure::Input(format!(
            "{output}: that is the file hpr convert reads, and it would write over it"
        )));
    }
    if let Some(folder) = Path::new(output).parent()
        && !folder.as_os_str().is_empty()
        && !folder.is_dir()
    {
        return Err(Failure::Input(format!(
            "{output}: there is no folder {}",
            folder.display()
        )));
    }
    let input = &args.input;
    let refused = |error| Failure::Input(format!("{input}: {error}"));
    let mut warnings = Vec::new();
    let motors = match format {
        MotorFile::Eng => {
            let parsed = eng::parse(&text).map_err(refused)?;
            warnings.extend(read_warnings(&parsed.warnings)?);
            Motors::Eng(parsed.value)
        }
        MotorFile::Rse => {
            let parsed = rse::parse(&text).map_err(refused)?;
            warnings.extend(read_warnings(&parsed.warnings)?);
            Motors::Rse(parsed.value)
        }
    };
    if args.delays.is_some() && !(format == MotorFile::Rse && target == MotorFile::Eng) {
        return Err(Failure::Input(
            "--delays gives a .rse motor's missing delays for a .eng file; this conversion \
             writes none"
                .to_owned(),
        ));
    }
    let unwritable = |error| {
        Failure::Input(format!(
            "{input}: its motors can't be written as a {} file: {error}",
            target.extension()
        ))
    };
    let (written, names, converted) = match (motors, target) {
        (Motors::Eng(file), MotorFile::Eng) => (
            eng::write(&file).map_err(unwritable)?,
            eng_names(&file),
            Vec::new(),
        ),
        (Motors::Rse(file), MotorFile::Rse) => (
            rse::write(&file).map_err(unwritable)?,
            rse_names(&file),
            Vec::new(),
        ),
        (Motors::Eng(file), MotorFile::Rse) => {
            let converted = convert::eng_to_rse(&file).map_err(refused)?;
            (
                rse::write(&converted.value).map_err(unwritable)?,
                rse_names(&converted.value),
                converted.warnings,
            )
        }
        (Motors::Rse(mut file), MotorFile::Eng) => {
            if let Some(delays) = &args.delays {
                for engine in file.engines.iter_mut().filter(|e| e.delays.is_none()) {
                    engine.delays = Some(delays.clone());
                }
            }
            let converted = convert::rse_to_eng(&file).map_err(|error| {
                let hint = if file.engines.iter().any(|e| e.delays.is_none()) {
                    "; give them with --delays"
                } else {
                    ""
                };
                Failure::Input(format!("{input}: {error}{hint}"))
            })?;
            (
                eng::write(&converted.value).map_err(unwritable)?,
                eng_names(&converted.value),
                converted.warnings,
            )
        }
    };
    for warning in &converted {
        warnings.push(converted_warning(warning)?);
    }
    std::fs::write(output, written)
        .map_err(|error| Failure::Input(format!("{output}: {error}")))?;
    let document = Convert {
        input: source,
        output: ConvertedFile {
            path: output.clone(),
            format: target.output(),
        },
        motors: names,
        warnings,
    };
    to.emit(&document, |out| text_output(&document, input, out))
}

/// The input's source, format and text: a motor file by its extension, or else a catalog motor.
fn read_input(input: &str) -> Result<(MotorSource, MotorFile, String), Failure> {
    if let Some(format) = MotorFile::of(input) {
        let bytes =
            std::fs::read(input).map_err(|error| Failure::Input(format!("{input}: {error}")))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| Failure::Input(format!("{input}: not a text file in UTF-8")))?;
        let source = MotorSource::File {
            path: input.to_owned(),
            format: format.output(),
        };
        return Ok((source, format, text));
    }
    let catalog = catalog()?;
    let matches: Vec<&CatalogMotor> = catalog.find(input).collect();
    let motor = match matches[..] {
        [motor] => motor,
        [] => {
            return Err(Failure::Input(format!(
                "{input} is neither a .eng or .rse file nor a motor in the bundled catalog; \
                 `hpr motors list` lists them"
            )));
        }
        _ => {
            let names: Vec<&str> = matches.iter().map(|m| m.designation.as_str()).collect();
            return Err(Failure::Input(format!(
                "{input} names {} catalog motors: {}; give one's designation",
                matches.len(),
                names.join(", ")
            )));
        }
    };
    let (curve, text) = motor
        .curves
        .iter()
        .find_map(|curve| bundled_curve_text(&curve.file).map(|text| (curve, text)))
        .ok_or_else(|| {
            Failure::Input(format!("{}: no curve of it is bundled", motor.designation))
        })?;
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
    let source = MotorSource::Catalog {
        curve_url: curve.info_url.clone().unwrap_or_else(|| curve.url.clone()),
        format: format.output(),
    };
    Ok((source, format, text.to_owned()))
}

fn eng_names(file: &EngFile) -> Vec<String> {
    file.entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect()
}

fn rse_names(file: &RseFile) -> Vec<String> {
    file.engines
        .iter()
        .map(|engine| engine.code.clone())
        .collect()
}

fn converted_warning(warning: &ConvertWarning) -> Result<Warning, Failure> {
    Ok(Warning {
        motor: Some(warning.motor.clone()),
        line: None,
        kind: warning_kind(warning.kind)?,
        message: warning.message.clone(),
    })
}

/// `hpr convert` as text: what was read, what was written, and the warnings.
fn text_output(document: &Convert, input: &str, out: &mut dyn Write) -> io::Result<()> {
    let from = match &document.input {
        MotorSource::Catalog { curve_url, .. } => format!("the bundled catalog ({curve_url})"),
        MotorSource::File { .. } => file_name(input),
    };
    writeln!(out, "read   {from}")?;
    writeln!(
        out,
        "wrote  {}: {}",
        file_name(&document.output.path),
        document.motors.join(", ")
    )?;
    for warning in &document.warnings {
        let at = match (&warning.motor, warning.line) {
            (Some(motor), _) => format!("{motor}: "),
            (None, Some(line)) => format!("line {line}: "),
            (None, None) => String::new(),
        };
        writeln!(out, "warning: {at}{}", warning.message)?;
    }
    Ok(())
}

/// The file name of a path, for the text: its folder depends on where it was run from.
fn file_name(path: &str) -> String {
    Path::new(path).file_name().map_or_else(
        || path.to_owned(),
        |name| name.to_string_lossy().into_owned(),
    )
}
