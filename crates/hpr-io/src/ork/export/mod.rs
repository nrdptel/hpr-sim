//! Writing a design back out as an OpenRocket `.ork` file.
//!
//! **Guide:** [OpenRocket `.ork` design files][guide] says what is written and what is not.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/format/ork.html
//!
//! The file is written from the [`Design`] alone: its rocket, its motors, when its recovery
//! opens and its stages separate, and the simulations OpenRocket last ran, each written as the
//! tags [`super::design`] reads them from. Everything the design keeps but hpr does not model —
//! its [`super::Extensions`], such as a pod set, a part's colour or a simulation's extension — is
//! put back where it was. So a `.ork` read and written again reads back as the same design, which
//! is how `cargo xtask ork` checks this on every file in the reference library
//! ([ADR-109][adr-109]).
//!
//! What is written is schema 1.10, the version OpenRocket 24.12 writes, in a zip archive with
//! the design as `rocket.ork`, as OpenRocket packs one.
//!
//! [adr-109]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-109-m32-split-and-a-ork-written-from-the-design-2026-09-29

mod kept;
mod motors;
mod recovery;
mod rocket;
mod simulations;
mod xml;

use std::io::{Cursor, Write as _};

use super::container::Attachment;
use super::document::{Document, SchemaVersion};
use super::error::OrkError;
use super::warning::{Imported, Warning};
use super::{Design, MAX_KNOWN_MINOR};
use xml::Build as _;

/// The schema version written: OpenRocket 24.12's.
pub const SCHEMA: SchemaVersion = SchemaVersion {
    major: 1,
    minor: 10,
};

/// The `creator` written on the document.
pub const CREATOR: &str = concat!("hpr-sim ", env!("CARGO_PKG_VERSION"));

/// The archive entry the design is written to, where OpenRocket looks for it first.
pub const DESIGN_ENTRY: &str = "rocket.ork";

// Schema 1.10 is one this crate reads as known; a newer written version would need its reader.
const _: () = assert!(SCHEMA.major == 1 && SCHEMA.minor <= MAX_KNOWN_MINOR);

/// The design document for `design`, with a warning for anything kept from the file it came from
/// that has no place in it any more.
///
/// ```
/// # fn main() -> Result<(), hpr_io::ork::OrkError> {
/// let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
/// <openrocket version="1.10" creator="OpenRocket 24.12">
///   <rocket><name>Sounder</name><designer>A. Flyer</designer></rocket>
/// </openrocket>"#;
/// let read = hpr_io::ork::read(xml)?;
/// let design = hpr_io::ork::design(&read.value).value;
///
/// let written = hpr_io::ork::export::document(&design);
/// assert!(written.warnings.is_empty());
/// let text = written.value.to_xml();
/// // The name is the rocket's; the designer, which hpr does not read, was kept and put back.
/// assert!(text.contains("<name>Sounder</name>"), "{text}");
/// assert!(text.contains("<designer>A. Flyer</designer>"), "{text}");
/// # Ok(())
/// # }
/// ```
pub fn document(design: &Design) -> Imported<Document> {
    let mut warnings: Vec<Warning> = Vec::new();
    let mut root = xml::element("openrocket");
    root.with_attribute("version", SCHEMA.to_string())
        .with_attribute("creator", CREATOR);
    root.push(rocket::rocket(design, &mut warnings));
    if let Some(simulations) = simulations::simulations(design, &mut warnings) {
        root.push(simulations);
    }
    let mut document = Document {
        version: SCHEMA,
        creator: Some(CREATOR.to_owned()),
        root,
    };
    warnings.extend(kept::splice(&mut document, &design.extensions.x_openrocket));
    Imported {
        value: document,
        warnings,
    }
}

/// A `.ork` file for `design`: a zip archive holding its document as `rocket.ork`, and then
/// `attachments`, such as the thrust curves and preview image of the file it was read from
/// ([`super::OrkFile::attachments`]). An attachment called `rocket.ork` is left out, with a
/// warning: the design is that entry.
///
/// # Errors
///
/// [`OrkError::Zip`] if the archive cannot be written, which in memory means an attachment's name
/// is one no zip entry can have.
pub fn write(design: &Design, attachments: &[Attachment]) -> Result<Imported<Vec<u8>>, OrkError> {
    let Imported {
        value: document,
        mut warnings,
    } = document(design);
    let zip = |error: &dyn std::fmt::Display| OrkError::Zip {
        reason: error.to_string(),
    };
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file(DESIGN_ENTRY, options)
        .map_err(|e| zip(&e))?;
    archive
        .write_all(document.to_xml().as_bytes())
        .map_err(|e| zip(&e))?;
    for attachment in attachments {
        if attachment.name == DESIGN_ENTRY {
            warnings.push(Warning::new(
                DESIGN_ENTRY,
                super::WarningKind::Skipped,
                "an attachment has the design's own name, `rocket.ork`; it was left out",
            ));
            continue;
        }
        archive
            .start_file(attachment.name.as_str(), options)
            .map_err(|e| zip(&e))?;
        archive.write_all(&attachment.bytes).map_err(|e| zip(&e))?;
    }
    let bytes = archive.finish().map_err(|e| zip(&e))?.into_inner();
    Ok(Imported {
        value: bytes,
        warnings,
    })
}

#[cfg(test)]
mod tests;
