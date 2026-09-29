//! The hpr open design format: a rocket design as one JSON document, with its JSON Schema.
//!
//! **Guide:** [the format's page][guide-format] says what a document holds, how it is versioned,
//! and how it compares with `.ork`.
//!
//! A document is a [`DesignFile`]: a header naming the format ([`FORMAT`]), its version
//! ([`VERSION`]) and where the design came from ([`Provenance`]), then the design as
//! [`hpr_io::ork`] reads one — the rocket, every motor configuration, the recovery events, the
//! simulations stored with it, and what the source file holds that hpr does not model, kept under
//! a namespaced extension (`x-openrocket`). A `.ork` written from a document is the `.ork` written
//! from the design it was read from ([ADR-111][adr-111]).
//!
//! [`to_json`] writes the canonical text: two-space indents, keys in the order the types declare
//! them, and a final newline, so the same design always gives the same bytes and a change shows as
//! a small diff. [`from_json`] reads it back to the same value, and refuses a document of another
//! format or version with the reason. [`schema`] is the document's JSON Schema, committed as
//! [`schema/format/hpr-design-0.1.schema.json`][schema-file].
//!
//! The zip container that carries attachments, and migrations from older versions, come with
//! [M3.3b][m3-3b], the format's second step; generated TypeScript and Python types with
//! [M3.3c][m3-3c], its third.
//!
//! ```
//! use hpr_format::{DesignFile, Provenance, from_json, to_json};
//!
//! let bytes = include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-stable.ork");
//! let read = DesignFile::from_ork(bytes)?;
//! let text = to_json(&read.value)?;
//! assert!(text.starts_with("{\n  \"format\": \"hpr-design\",\n  \"version\": \"0.1\","));
//! assert_eq!(from_json(&text)?, read.value);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [guide-format]: https://nrdptel.github.io/hpr-sim/format/hpr.html
//! [adr-111]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29
//! [schema-file]: https://github.com/nrdptel/hpr-sim/blob/main/schema/format/hpr-design-0.1.schema.json
//! [m3-3b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m3-3b
//! [m3-3c]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m3-3c

use std::fmt;

use hpr_design::Rocket;
use hpr_io::ork::{
    self, Attachment, Curve, Design, Extensions, Imported, Motors, OrkError, Recovery,
    StoredSimulation,
};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The format's name, the value of every document's `format` key.
pub const FORMAT: &str = "hpr-design";

/// The version this crate writes and reads.
pub const VERSION: Version = Version { major: 0, minor: 1 };

/// The extension of a design written as plain JSON ([ADR-111][adr-111]): `.hpr`.
///
/// [adr-111]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29
pub const EXTENSION: &str = "hpr";

/// The extension reserved for the zip container of a design and its attachments, which comes with
/// the format's second step, [M3.3b][m3-3b]: `.hprz`.
///
/// [m3-3b]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m3-3b
pub const CONTAINER_EXTENSION: &str = "hprz";

/// A design as one document of the hpr design format.
///
/// The keys are written in this order. The design's own parts are those of [`hpr_io::ork::Design`],
/// whose documentation says what each holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(
    title = "hpr design",
    description = "A rocket design in the hpr design format."
)]
pub struct DesignFile {
    /// Always `hpr-design`.
    pub format: Format,
    /// The format's version, `major.minor`.
    pub version: Version,
    /// Which program wrote the document, and from what.
    pub provenance: Provenance,
    /// The rocket: its stages and their parts, with every configuration that flies.
    pub rocket: Rocket,
    /// Every motor configuration, flown or not, with why one is not.
    pub motors: Motors,
    /// When each parachute and streamer opens and each stage separates.
    pub recovery: Recovery,
    /// The simulations the source file stored, with their conditions and results.
    pub simulations: Vec<StoredSimulation>,
    /// What the source file holds that hpr does not model, by namespace, kept for writing it
    /// back.
    pub extensions: Extensions,
}

impl DesignFile {
    /// The document of `design`, with its provenance.
    pub fn new(design: Design, provenance: Provenance) -> Self {
        Self {
            format: Format::HprDesign,
            version: VERSION,
            provenance,
            rocket: design.rocket,
            motors: design.motors,
            recovery: design.recovery,
            simulations: design.simulations,
            extensions: design.extensions,
        }
    }

    /// Reads a `.ork` file's bytes (zip, gzip or raw XML) into a document whose provenance names
    /// this program and the file's SHA-256, with the reader's warnings.
    ///
    /// The design is [`hpr_io::ork::design`]'s: a motor configuration flies with the curve the
    /// file itself holds.
    ///
    /// # Errors
    ///
    /// [`OrkError`] when the bytes are not a `.ork` hpr can read.
    pub fn from_ork(bytes: &[u8]) -> Result<Imported<Self>, OrkError> {
        let file = ork::read(bytes)?;
        let design = ork::design(&file.value);
        let mut warnings = file.warnings;
        warnings.extend(design.warnings);
        let provenance = Provenance::hpr(Some(Source::of(SourceFormat::Ork, bytes)));
        Ok(Imported {
            value: Self::new(design.value, provenance),
            warnings,
        })
    }

    /// The design the document holds, as [`hpr_io::ork`] would have read it.
    pub fn design(&self) -> Design {
        Design::new(
            self.rocket.clone(),
            self.motors.clone(),
            self.recovery.clone(),
            self.simulations.clone(),
            self.extensions.clone(),
        )
    }

    /// The archive entries a `.ork` written from the document holds besides the design: each
    /// thrust curve the source file embedded, once, in the order the configurations name them.
    pub fn attachments(&self) -> Vec<Attachment> {
        let mut attachments: Vec<Attachment> = Vec::new();
        let curves = self
            .motors
            .configurations
            .iter()
            .flat_map(|configuration| &configuration.motors)
            .map(|motor| &motor.curve);
        for curve in curves {
            if let Curve::Embedded { entry, text, .. } = curve
                && !attachments
                    .iter()
                    .any(|attachment| attachment.name == *entry)
            {
                attachments.push(Attachment {
                    name: entry.clone(),
                    bytes: text.clone().into_bytes(),
                });
            }
        }
        attachments
    }

    /// The design written as a `.ork` ([`hpr_io::ork::export::write`]) with its
    /// [attachments](Self::attachments), so it reads back as the design the document holds.
    ///
    /// # Errors
    ///
    /// [`OrkError`] when the archive cannot be written.
    pub fn to_ork(&self) -> Result<Imported<Vec<u8>>, OrkError> {
        ork::export::write(&self.design(), &self.attachments())
    }
}

/// The format's name: a document holds only `hpr-design`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Format {
    /// A rocket design.
    #[serde(rename = "hpr-design")]
    HprDesign,
}

/// A version of the format, written `major.minor` ([ADR-111][adr-111]).
///
/// While the major version is 0, every minor version may change the document in ways an older
/// reader cannot follow, so a reader takes only its own version and those it can migrate from.
///
/// [adr-111]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    /// The major version.
    pub major: u32,
    /// The minor version.
    pub minor: u32,
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl From<Version> for String {
    fn from(version: Version) -> Self {
        version.to_string()
    }
}

impl TryFrom<String> for Version {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        let number = |part: &str| {
            // Digits only: no sign, no spaces, no leading zero but a lone "0".
            let plain = !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == "0" || !part.starts_with('0'));
            plain.then(|| part.parse::<u32>().ok()).flatten()
        };
        text.split_once('.')
            .and_then(|(major, minor)| Some((number(major)?, number(minor)?)))
            .map(|(major, minor)| Self { major, minor })
            .ok_or_else(|| format!("{text:?} is not a version: it is written major.minor, as 0.1"))
    }
}

impl JsonSchema for Version {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Version".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "description": "A version of the format, `major.minor`.",
            "type": "string",
            "pattern": "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$",
        })
    }
}

/// Which program wrote a document, and from what.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    /// The program, such as `hpr-sim`.
    pub tool: String,
    /// Its version.
    pub tool_version: String,
    /// The file the design was read from, if it was read from one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
}

impl Provenance {
    /// This program, at this version, reading `source`.
    pub fn hpr(source: Option<Source>) -> Self {
        Self {
            tool: "hpr-sim".to_owned(),
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            source,
        }
    }
}

/// The file a design was read from: its format and its SHA-256, which name it without its path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// The file's format.
    pub format: SourceFormat,
    /// The SHA-256 of the file's bytes, as 64 lowercase hexadecimal digits.
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
}

impl Source {
    /// The source `bytes` of `format`.
    pub fn of(format: SourceFormat, bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let sha256 = digest
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                hex.push_str(&format!("{byte:02x}"));
                hex
            });
        Self { format, sha256 }
    }
}

/// The formats a design can be read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceFormat {
    /// OpenRocket's `.ork`.
    Ork,
    /// A document of this format.
    HprDesign,
}

/// Why a document could not be written or read.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum FormatError {
    /// The text is not JSON.
    #[error("not JSON: {0}")]
    Json(String),
    /// The JSON is not a document of this format: its `format` is missing or another.
    #[error("not an hpr design: the document's \"format\" is {found}, not \"hpr-design\"")]
    NotADesign {
        /// What the document holds, as JSON, or `missing`.
        found: String,
    },
    /// The document is of a version this reader does not take.
    #[error(
        "the document is version {found}, and this reader takes version {supported}{}",
        if found > supported { ": it was written by a newer program" } else { "" }
    )]
    Unsupported {
        /// The document's version.
        found: Version,
        /// The version this reader takes.
        supported: Version,
    },
    /// The document names the format and version but does not follow the schema.
    #[error("not a valid hpr design {VERSION}: {0}")]
    Invalid(String),
    /// The design holds a value JSON cannot carry exactly, such as a number that is not finite,
    /// so the text would not read back as the same design.
    #[error("the design does not read back the same from its JSON: {0}")]
    NotRepresentable(String),
}

/// The canonical JSON text of `document`: two-space indents, keys in declared order, a final
/// newline.
///
/// # Errors
///
/// [`FormatError::NotRepresentable`] when the text would not read back as `document`: every
/// document written is read again and compared, so a value JSON cannot carry (a number that is
/// not finite) is refused here rather than lost.
pub fn to_json(document: &DesignFile) -> Result<String, FormatError> {
    let mut text = serde_json::to_string_pretty(document)
        .map_err(|error| FormatError::NotRepresentable(error.to_string()))?;
    text.push('\n');
    match from_json(&text) {
        Ok(back) if back == *document => Ok(text),
        Ok(_) => Err(FormatError::NotRepresentable(
            "a value reads back different".to_owned(),
        )),
        Err(error) => Err(FormatError::NotRepresentable(error.to_string())),
    }
}

/// Reads a document from its JSON text.
///
/// The format and version are checked first, so a file of another kind or version is refused with
/// that reason rather than with the first field the reader does not know.
///
/// # Errors
///
/// [`FormatError`]: not JSON, not an hpr design, another version, or not valid.
pub fn from_json(text: &str) -> Result<DesignFile, FormatError> {
    /// The header, read on its own before the rest.
    #[derive(Deserialize)]
    struct Header {
        format: Option<serde_json::Value>,
        version: Option<serde_json::Value>,
    }
    let header: Header =
        serde_json::from_str(text).map_err(|error| FormatError::Json(error.to_string()))?;
    match &header.format {
        Some(serde_json::Value::String(format)) if format == FORMAT => {}
        Some(other) => {
            return Err(FormatError::NotADesign {
                found: other.to_string(),
            });
        }
        None => {
            return Err(FormatError::NotADesign {
                found: "missing".to_owned(),
            });
        }
    }
    let found = match header.version {
        Some(serde_json::Value::String(version)) => {
            Version::try_from(version).map_err(FormatError::Invalid)?
        }
        Some(other) => {
            return Err(FormatError::Invalid(format!(
                "its \"version\" is {other}, not a string such as \"0.1\""
            )));
        }
        None => return Err(FormatError::Invalid("it has no \"version\"".to_owned())),
    };
    if found != VERSION {
        return Err(FormatError::Unsupported {
            found,
            supported: VERSION,
        });
    }
    serde_json::from_str(text).map_err(|error| FormatError::Invalid(error.to_string()))
}

/// The document's JSON Schema (draft 2020-12), as committed under `schema/format/`.
pub fn schema() -> Schema {
    schemars::schema_for!(DesignFile)
}

/// [`schema`] as the canonical text committed: two-space indents and a final newline.
pub fn schema_json() -> String {
    let mut text = serde_json::to_string_pretty(&schema())
        // A schema is a JSON value built from strings and maps, which always serialises.
        .unwrap_or_default();
    text.push('\n');
    text
}

#[cfg(test)]
mod tests;
