//! `hpr convert` of a design: a `.ork`, `.hpr` or `.hprz` file written as any of the three.
//!
//! The conversion is the library's own ([`hpr::hpr_format`]). A `.ork` is read into a document of
//! the hpr design format ([`DesignFile::from_ork`]), which names the file's SHA-256 as its source;
//! a `.hpr` or `.hprz` of an older version is migrated to the current one. The document is written
//! as its canonical text, in a container with its attachments, or as the `.ork` hpr writes from
//! it. A conversion keeps the document's provenance as read: it names where the design came
//! from, which rewriting it doesn't change (ADR-112).

use std::io::{self, Write};
use std::path::Path;

use hpr::hpr_format::container::{self, Entry, Hprz};
use hpr::hpr_format::{self, DesignFile};

use crate::convert::{ConvertArgs, file_name, output_folder};
use crate::output::{
    Convert, ConvertDesign, DesignFilePath, DesignFormat, InputWarning, WarningKind,
};
use crate::sim::{ork_warning, same_file};
use crate::{Failure, Out};

/// The design formats `hpr convert` reads and writes, by a path's extension.
fn format_of(path: &str) -> Option<DesignFormat> {
    let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    match extension.as_str() {
        "ork" => Some(DesignFormat::Ork),
        "hpr" => Some(DesignFormat::Hpr),
        "hprz" => Some(DesignFormat::Hprz),
        _ => None,
    }
}

/// Whether `path` names a design file `hpr convert` reads or writes.
pub(crate) fn is_design(path: &str) -> bool {
    format_of(path).is_some()
}

/// A design read: the document, the version it was written in, a container's attachments, and
/// what the reader flagged.
struct ReadDesign {
    document: DesignFile,
    written_as: hpr_format::Version,
    attachments: Vec<Entry>,
    warnings: Vec<InputWarning>,
}

/// Runs `hpr convert` for a design.
pub(crate) fn run(args: &ConvertArgs, to: &mut Out<'_>) -> Result<(), Failure> {
    let (input, output) = (&args.input, &args.output);
    let (Some(from), Some(target)) = (format_of(input), format_of(output)) else {
        return Err(Failure::Input(format!(
            "{input} to {output}: hpr convert writes a design (.ork, .hpr or .hprz) from a \
             design, and motors (.eng or .rse) from motors"
        )));
    };
    if args.delays.is_some() {
        return Err(Failure::Input(
            "--delays gives a .rse motor's delays for a .eng file, and hpr convert is writing a \
             design"
                .to_owned(),
        ));
    }
    if !args.attach.is_empty() && target != DesignFormat::Hprz {
        return Err(Failure::Input(format!(
            "{output}: --attach adds files to a .hprz, and this is not one"
        )));
    }
    if same_file(input) == same_file(output) {
        return Err(Failure::Input(format!(
            "{output}: that is the file hpr convert reads, and it would write over it"
        )));
    }
    output_folder(output)?;
    let read = read(input, from)?;
    let mut warnings = read.warnings;
    let mut attachments = read.attachments;
    for path in &args.attach {
        let name = file_name(path);
        container::check_name(&name).map_err(|why| Failure::Input(format!("{path}: {why}")))?;
        let bytes =
            std::fs::read(path).map_err(|error| Failure::Input(format!("{path}: {error}")))?;
        attachments.push(Entry::new(name, bytes));
    }
    let unwritable = |error: &dyn std::fmt::Display| {
        Failure::Input(format!("{output}: the design can't be written: {error}"))
    };
    let (bytes, written) = match target {
        DesignFormat::Hprz => {
            let names = attachments.iter().map(|entry| entry.name.clone()).collect();
            let hprz = Hprz::new(read.document.clone(), attachments);
            (container::write(&hprz).map_err(|e| unwritable(&e))?, names)
        }
        _ => {
            // A `.hpr` or `.ork` has no place for a container's own files.
            for entry in &attachments {
                warnings.push(InputWarning {
                    at: entry.name.clone(),
                    kind: WarningKind::Dropped,
                    message: format!(
                        "the {} file has no place for a .hprz's attachments, so this one was left \
                         out",
                        extension(target)
                    ),
                });
            }
            if target == DesignFormat::Hpr {
                let text = hpr_format::to_json(&read.document).map_err(|e| unwritable(&e))?;
                (text.into_bytes(), Vec::new())
            } else {
                let ork = read.document.to_ork().map_err(|e| unwritable(&e))?;
                warnings.extend(ork.warnings.iter().map(ork_warning));
                (ork.value, Vec::new())
            }
        }
    };
    std::fs::write(output, bytes).map_err(|error| Failure::Input(format!("{output}: {error}")))?;
    let document = ConvertDesign {
        input: DesignFilePath {
            path: input.clone(),
            format: from,
        },
        output: DesignFilePath {
            path: output.clone(),
            format: target,
        },
        rocket: read.document.rocket.name.clone(),
        configurations: read.document.motors.configurations.len(),
        migrated_from: (read.written_as != hpr_format::VERSION)
            .then(|| read.written_as.to_string()),
        attachments: written,
        warnings,
    };
    to.emit(&Convert::Design(document.clone()), |out| {
        text_output(&document, out)
    })
}

/// Reads the design at `path`, of `format`.
fn read(path: &str, format: DesignFormat) -> Result<ReadDesign, Failure> {
    let bytes = std::fs::read(path).map_err(|error| Failure::Input(format!("{path}: {error}")))?;
    let refused = |error: &dyn std::fmt::Display| Failure::Input(format!("{path}: {error}"));
    match format {
        DesignFormat::Ork => {
            let read = DesignFile::from_ork(&bytes).map_err(|e| refused(&e))?;
            Ok(ReadDesign {
                document: read.value,
                written_as: hpr_format::VERSION,
                attachments: Vec::new(),
                warnings: read.warnings.iter().map(ork_warning).collect(),
            })
        }
        DesignFormat::Hprz => {
            let opened = container::read(&bytes).map_err(|e| refused(&e))?;
            Ok(ReadDesign {
                document: opened.value.design,
                written_as: opened.written_as,
                attachments: opened.value.attachments,
                warnings: Vec::new(),
            })
        }
        _ => {
            let text = String::from_utf8(bytes)
                .map_err(|_| Failure::Input(format!("{path}: an .hpr file is UTF-8 text")))?;
            let opened = hpr_format::read_json(&text).map_err(|e| refused(&e))?;
            Ok(ReadDesign {
                document: opened.value,
                written_as: opened.written_as,
                attachments: Vec::new(),
                warnings: Vec::new(),
            })
        }
    }
}

/// A design format's extension, with its dot.
fn extension(format: DesignFormat) -> &'static str {
    match format {
        DesignFormat::Ork => ".ork",
        DesignFormat::Hpr => ".hpr",
        DesignFormat::Hprz => ".hprz",
        DesignFormat::HprJson => ".json",
    }
}

/// `hpr convert` of a design as text: what was read, what was written, and the warnings.
fn text_output(document: &ConvertDesign, out: &mut dyn Write) -> io::Result<()> {
    let migrated = document
        .migrated_from
        .as_ref()
        .map(|version| format!(", migrated from version {version}"))
        .unwrap_or_default();
    writeln!(out, "read   {}{migrated}", file_name(&document.input.path))?;
    let configurations = match document.configurations {
        1 => "1 motor configuration".to_owned(),
        n => format!("{n} motor configurations"),
    };
    let version = match document.output.format {
        DesignFormat::Hpr | DesignFormat::Hprz => {
            format!(", hpr design format {}", hpr_format::VERSION)
        }
        _ => String::new(),
    };
    writeln!(
        out,
        "wrote  {}: {:?}, {configurations}{version}",
        file_name(&document.output.path),
        document.rocket,
    )?;
    if !document.attachments.is_empty() {
        writeln!(out, "with   {}", document.attachments.join(", "))?;
    }
    for warning in &document.warnings {
        writeln!(out, "warning: {}: {}", warning.at, warning.message)?;
    }
    Ok(())
}
