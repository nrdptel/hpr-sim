//! Reading OpenRocket `.ork` design files.
//!
//! **Guide:** [OpenRocket `.ork` design files][guide] says what this reads and what it leaves out.
//!
//! [guide]: https://nrdptel.github.io/hpr-sim/format/ork.html
//!
//! A `.ork` is a design: a tree of components, the materials they are made of, the motors flown in
//! them, and the simulations OpenRocket last ran. This module is the first step of reading one —
//! getting the design document out of whichever container it arrived in, and into a tree that
//! keeps everything the file said. Turning that tree into [`hpr_design`] types is the next
//! milestone's work.
//!
//! Nothing here reads a file: `hpr-io` does no I/O and builds for `wasm32-unknown-unknown`, so the
//! caller supplies the bytes.
//!
//! ```
//! # fn main() -> Result<(), hpr_io::ork::OrkError> {
//! let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
//! <openrocket version="1.10" creator="OpenRocket 24.12">
//!   <rocket><name>Sounder</name></rocket>
//! </openrocket>"#;
//!
//! let read = hpr_io::ork::read(xml)?;
//! assert_eq!(read.value.container, hpr_io::ork::Container::Xml);
//! assert_eq!(read.value.document.version.to_string(), "1.10");
//! let rocket = read.value.document.root.child("rocket").expect("a rocket");
//! assert_eq!(rocket.child("name").expect("a name").text(), "Sounder");
//! assert!(read.warnings.is_empty());
//! # Ok(())
//! # }
//! ```

pub mod container;
pub mod document;
mod error;
mod warning;

pub use container::{Attachment, Container, Unpacked};
pub use document::{Document, Element, MAX_DEPTH, MAX_KNOWN_MINOR, Node, SchemaVersion};
pub use error::OrkError;
pub use warning::{Imported, Warning, WarningKind};

/// A `.ork` file, read: the container it came in, its design document, and everything else it
/// carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrkFile {
    /// Which of the three containers the bytes were packed in.
    pub container: Container,
    /// The archive entry the design came from, when it came from an archive.
    pub design_entry: Option<String>,
    /// The design document.
    pub document: Document,
    /// The archive's other entries, in archive order: thrust curves, preview images, lookup
    /// tables. Held as stored, for the milestones that read them and for an export to put back.
    pub attachments: Vec<Attachment>,
}

impl OrkFile {
    /// The attachment called `name`, if the archive had one.
    pub fn attachment(&self, name: &str) -> Option<&Attachment> {
        self.attachments
            .iter()
            .find(|attachment| attachment.name == name)
    }
}

/// Reads a `.ork` file from its bytes: sniffs the container, unpacks it, and parses the design.
///
/// # Errors
///
/// Returns [`OrkError`] when the bytes are not a `.ork` at all, when the container is damaged,
/// when no entry in it can be the design document, or when that document is not well-formed XML
/// rooted at `<openrocket>` with a readable schema version. Everything a reader can go on from is
/// a [`Warning`] instead.
pub fn read(bytes: &[u8]) -> Result<Imported<OrkFile>, OrkError> {
    let unpacked = container::unpack(bytes)?;
    let mut warnings = unpacked.warnings;
    let Unpacked {
        container,
        design_entry,
        design,
        attachments,
    } = unpacked.value;
    let document = Document::parse(&design)?;
    warnings.extend(document.warnings);
    let document = document.value;
    Ok(Imported {
        value: OrkFile {
            container,
            design_entry,
            document,
            attachments,
        },
        warnings,
    })
}

#[cfg(test)]
mod tests;
