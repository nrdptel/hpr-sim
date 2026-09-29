//! The hpr open design format: a rocket design as one JSON document, with its JSON Schema.
//!
//! **Guide:** [the format's page][guide-format] says what a document holds, how it is versioned,
//! and how it was checked. Version 0.1 is a draft until hpr's first release: it can change in
//! place, so keep the source `.ork` too.
//!
//! A document is a [`DesignFile`]: a header naming the format ([`FORMAT`]), its version
//! ([`VERSION`]) and where the design came from ([`Provenance`]), then the design as
//! [`hpr_io::ork`] reads one — the rocket, every motor configuration, the recovery events, the
//! simulations stored with it, and what the source file holds that hpr does not model, kept under
//! a namespaced extension (`x-openrocket`) — and every other entry of the source file's archive,
//! such as an embedded thrust curve or a decal image. A `.ork` written from a document is the
//! `.ork` hpr writes from the file it was read from, byte for byte, as checked on the 73 designs
//! hpr's `.ork` checks read ([ADR-111][adr-111]).
//!
//! [`to_json`] writes the canonical text: two-space indents, keys in the order the types declare
//! them, and a final newline, so the same design always gives the same bytes and a change shows as
//! a small diff. [`from_json`] reads it back to the same value, takes a document of an older
//! version to the current one ([`migrate`]), and refuses one of another format or a newer version
//! with the reason. [`schema`] is the document's JSON Schema, committed as
//! [`schema/format/hpr-design-0.2.schema.json`][schema-file] beside the older versions' schemas.
//! A [`container`] (`.hprz`) carries a design with other files, such as its flight logs.
//!
//! Generated TypeScript and Python types come with the format's third step, [M3.3c][m3-3c].
//!
//! ```
//! use hpr_format::{DesignFile, Provenance, from_json, to_json};
//!
//! let bytes = include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-stable.ork");
//! let read = DesignFile::from_ork(bytes)?;
//! let text = to_json(&read.value)?;
//! assert!(text.starts_with("{\n  \"format\": \"hpr-design\",\n  \"version\": \"0.2\","));
//! assert_eq!(from_json(&text)?, read.value);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [guide-format]: https://nrdptel.github.io/hpr-sim/format/hpr.html
//! [adr-111]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29
//! [schema-file]: https://github.com/nrdptel/hpr-sim/blob/main/schema/format/hpr-design-0.2.schema.json
//! [m3-3c]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m3-3c

use std::collections::BTreeSet;
use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
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
pub const VERSION: Version = Version { major: 0, minor: 2 };

/// The extension of a design written as plain JSON ([ADR-111][adr-111]): `.hpr`.
///
/// [adr-111]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-111-m33-the-hpr-design-format-its-extensions-versions-and-crate-2026-09-29
pub const EXTENSION: &str = "hpr";

/// The extension of the zip container of a design and its attachments ([`container`]): `.hprz`.
pub const CONTAINER_EXTENSION: &str = "hprz";

/// A design as one document of the hpr design format.
///
/// The keys are written in this order. The design's own parts are those of [`hpr_io::ork::Design`],
/// whose documentation says what each holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
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
    /// The source file's other files, in the order it held them: a `.ork` archive's entries
    /// besides the design, such as embedded thrust curves and decal images. Version 0.1 called
    /// them `attachments`.
    pub source_files: Vec<SourceFile>,
}

impl DesignFile {
    /// The document of `design`, with its provenance and the source file's other files.
    ///
    /// `source_files` must hold each thrust curve the design's motors name as embedded
    /// ([`Curve::Embedded`]): the file's own
    /// [`OrkFile::attachments`](hpr_io::ork::OrkFile::attachments) do. Without one, [`to_json`]
    /// refuses the document.
    pub fn new(design: Design, provenance: Provenance, source_files: &[Attachment]) -> Self {
        Self {
            format: Format::HprDesign,
            version: VERSION,
            provenance,
            rocket: design.rocket,
            motors: design.motors,
            recovery: design.recovery,
            simulations: design.simulations,
            extensions: design.extensions,
            source_files: source_files.iter().map(SourceFile::of).collect(),
        }
    }

    /// Reads a `.ork` file's bytes (zip, gzip or raw XML) into a document whose provenance names
    /// this program, the file's SHA-256 and why its airframe was not read exactly as written, if
    /// it wasn't, with the reader's warnings.
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
        let mut source = Source::of(SourceFormat::Ork, bytes);
        source.airframe_not_as_written = ork::airframe_not_as_written(&file.value);
        let provenance = Provenance::hpr(Some(source));
        Ok(Imported {
            value: Self::new(design.value, provenance, &file.value.attachments),
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

    /// The source file's other files, as bytes.
    ///
    /// # Errors
    ///
    /// [`FormatError::Invalid`] when a file's base64 does not decode; [`from_json`] refuses such a
    /// document, so only one built in code can hold one.
    pub fn source_files(&self) -> Result<Vec<Attachment>, FormatError> {
        self.source_files
            .iter()
            .map(|file| {
                Ok(Attachment {
                    name: file.name.clone(),
                    bytes: file.bytes()?,
                })
            })
            .collect()
    }

    /// The design written as a `.ork` ([`hpr_io::ork::export::write`]) with the source file's
    /// other files, so it is the `.ork` hpr writes from the source file itself.
    ///
    /// # Errors
    ///
    /// [`FormatError::Invalid`] for a source file that does not decode, and
    /// [`FormatError::Ork`] when the archive cannot be written.
    pub fn to_ork(&self) -> Result<Imported<Vec<u8>>, FormatError> {
        ork::export::write(&self.design(), &self.source_files()?)
            .map_err(|error| FormatError::Ork(error.to_string()))
    }

    /// What in a document read from JSON the types alone don't hold to: every source file decodes,
    /// has its own name, and is a file a `.ork` can hold besides its design, and every thrust curve
    /// embedded in the source file is among them.
    fn check(&self) -> Result<(), FormatError> {
        let mut names = BTreeSet::new();
        for file in &self.source_files {
            file.bytes()?;
            if !names.insert(file.name.as_str()) {
                return Err(FormatError::Invalid(format!(
                    "two source files are named {:?}",
                    shortened(&file.name)
                )));
            }
            // The design's own entry, and a directory's name, don't come back from a `.ork`.
            if file.name.is_empty() || file.name == "rocket.ork" || file.name.ends_with(['/', '\\'])
            {
                return Err(FormatError::Invalid(format!(
                    "a source file is named {:?}, which a .ork can't hold besides its design",
                    shortened(&file.name)
                )));
            }
        }
        let entries = self
            .motors
            .configurations
            .iter()
            .flat_map(|configuration| &configuration.motors)
            .filter_map(|motor| match &motor.curve {
                Curve::Embedded { entry, .. } => Some(entry),
                _ => None,
            });
        for entry in entries {
            if !names.contains(entry.as_str()) {
                return Err(FormatError::Invalid(format!(
                    "a motor's curve is embedded as {:?}, which the source files don't hold",
                    shortened(entry)
                )));
            }
        }
        Ok(())
    }
}

/// One of the source file's other files: its name, and its contents as text where they are UTF-8,
/// or else as base64 ([RFC 4648][rfc-4648], section 4).
///
/// Beyond the schema, the reader holds a document's source files to four rules: base64 decodes; no
/// two share a name; each thrust curve a motor names as embedded is among them; and a name is not
/// empty, not `rocket.ork` (the design's own entry) and not a directory's, ending in `/` or `\`.
///
/// [rfc-4648]: https://www.rfc-editor.org/rfc/rfc4648#section-4
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SourceFile {
    /// Its name in the source file, such as `thrustcurves/<digest>.rse`.
    pub name: String,
    /// Its contents.
    pub content: Content,
}

impl SourceFile {
    /// `attachment`, an entry of the source file's archive, as text if it is UTF-8.
    pub fn of(attachment: &Attachment) -> Self {
        let content = match std::str::from_utf8(&attachment.bytes) {
            Ok(text) => Content::Text(text.to_owned()),
            Err(_) => Content::Base64(BASE64.encode(&attachment.bytes)),
        };
        Self {
            name: attachment.name.clone(),
            content,
        }
    }

    /// Its bytes.
    ///
    /// # Errors
    ///
    /// [`FormatError::Invalid`] when its base64 does not decode.
    pub fn bytes(&self) -> Result<Vec<u8>, FormatError> {
        match &self.content {
            Content::Text(text) => Ok(text.clone().into_bytes()),
            Content::Base64(encoded) => BASE64.decode(encoded).map_err(|error| {
                FormatError::Invalid(format!("source file {:?}: {error}", shortened(&self.name)))
            }),
        }
    }
}

/// A file's contents: text, or base64 for bytes that are not UTF-8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[non_exhaustive]
pub enum Content {
    /// UTF-8 text, as the file holds it.
    Text(String),
    /// Any other bytes, in standard base64 with padding, which must decode.
    Base64(String),
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
#[non_exhaustive]
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

/// The file a design was read from: its format and its SHA-256, which name it without its path,
/// and whether its rocket was read exactly as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Source {
    /// The file's format.
    pub format: SourceFormat,
    /// The SHA-256 of the file's bytes, as 64 lowercase hexadecimal digits.
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
    /// Why the file's airframe or a motor mount was not read exactly as written, if it wasn't: a
    /// part left out, a value dropped or simplified, or something assumed
    /// ([`hpr_io::ork::airframe_not_as_written`]). No configuration of such a rocket flies, and
    /// `hpr sim` flies no other motor in it. Absent when the rocket was read as written.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub airframe_not_as_written: Option<String>,
}

impl Source {
    /// The source `bytes` of `format`, its rocket read as written; set
    /// [`Source::airframe_not_as_written`] when it wasn't.
    pub fn of(format: SourceFormat, bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let sha256 = digest
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                hex.push_str(&format!("{byte:02x}"));
                hex
            });
        Self {
            format,
            sha256,
            airframe_not_as_written: None,
        }
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
    /// The document is of a version this reader neither takes nor migrates from.
    #[error(
        "the document is version {found}, and this reader takes version {supported}{}",
        if found > supported {
            ": it was written by a newer program".to_owned()
        } else {
            format!(" and migrates from {} on", migrate::OLDEST)
        }
    )]
    Unsupported {
        /// The document's version.
        found: Version,
        /// The version this reader takes and writes.
        supported: Version,
    },
    /// The document names the format and version but does not follow the schema.
    #[error("not a valid hpr design {VERSION}: {0}")]
    Invalid(String),
    /// The design holds a value JSON cannot carry exactly, such as a number that is not finite,
    /// so the text would not read back as the same design.
    #[error("the design does not read back the same from its JSON: {0}")]
    NotRepresentable(String),
    /// The `.ork` could not be written.
    #[error("the .ork could not be written: {0}")]
    Ork(String),
    /// A `.hprz` container could not be read or written ([`container`]).
    #[error("not a valid .hprz: {0}")]
    Container(String),
}

/// The canonical JSON text of `document`: two-space indents, keys in declared order, a final
/// newline.
///
/// # Errors
///
/// [`FormatError::Unsupported`] for a document of another version, which this crate doesn't
/// write; [`FormatError::NotRepresentable`] when the text would not read back as `document`,
/// including when [`from_json`] would refuse it (a rule of [`SourceFile`]): every
/// document written is read again and compared, so a value JSON cannot carry (a number that is
/// not finite) is refused here rather than lost.
pub fn to_json(document: &DesignFile) -> Result<String, FormatError> {
    if document.version != VERSION {
        return Err(FormatError::Unsupported {
            found: document.version,
            supported: VERSION,
        });
    }
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

/// Reads a document from its JSON text, taking one of an older version to the current one.
///
/// [`read_json`] with the version the text was written in dropped.
///
/// # Errors
///
/// [`FormatError`]: not JSON, not an hpr design, a version this reader doesn't take, or not
/// valid.
pub fn from_json(text: &str) -> Result<DesignFile, FormatError> {
    read_json(text).map(|opened| opened.value)
}

/// A document read, and the version it was written in.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Opened<T> {
    /// What was read, in the current version.
    pub value: T,
    /// The version the text was written in: older than [`VERSION`] when it was migrated.
    pub written_as: Version,
}

/// Reads a document from its JSON text, and says which version it was written in.
///
/// The format and version are checked first, so a file of another kind or version is refused with
/// that reason rather than with the first field the reader does not know. A document of an older
/// version this reader migrates from ([`migrate`]) is rewritten into the current version's shape
/// and then read as one, so it is held to every rule a current document is.
///
/// # Errors
///
/// [`FormatError`]: not JSON, not an hpr design, a version this reader doesn't take, or not
/// valid.
pub fn read_json(text: &str) -> Result<Opened<DesignFile>, FormatError> {
    // A byte-order mark, which some Windows editors write at the start of UTF-8, is not JSON.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| FormatError::Json(error.to_string()))?;
    let serde_json::Value::Object(mut object) = value else {
        return Err(FormatError::NotADesign {
            found: format!("absent: the JSON is {}, not an object", kind(&value)),
        });
    };
    match object.get("format") {
        Some(serde_json::Value::String(format)) if format == FORMAT => {}
        Some(other) => {
            return Err(FormatError::NotADesign {
                found: shortened(&other.to_string()),
            });
        }
        None => {
            return Err(FormatError::NotADesign {
                found: "missing".to_owned(),
            });
        }
    }
    let found = match object.get("version") {
        Some(serde_json::Value::String(version)) => {
            Version::try_from(shortened(version)).map_err(FormatError::Invalid)?
        }
        Some(other) => {
            return Err(FormatError::Invalid(format!(
                "its \"version\" is {}, not a string such as \"{VERSION}\"",
                shortened(&other.to_string())
            )));
        }
        None => return Err(FormatError::Invalid("it has no \"version\"".to_owned())),
    };
    let document: DesignFile = if found == VERSION {
        // Read from the text, not the value, so an error names its line and column.
        serde_json::from_str(text).map_err(|error| FormatError::Invalid(error.to_string()))?
    } else if migrate::migrates_from(found) {
        migrate::migrate(&mut object, found).map_err(FormatError::Invalid)?;
        serde_json::from_value(serde_json::Value::Object(object))
            .map_err(|error| FormatError::Invalid(format!("migrated from {found}: {error}")))?
    } else {
        return Err(FormatError::Unsupported {
            found,
            supported: VERSION,
        });
    };
    document.check()?;
    Ok(Opened {
        value: document,
        written_as: found,
    })
}

/// What kind of JSON value `value` is, in words.
fn kind(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "a boolean",
        serde_json::Value::Number(_) => "a number",
        serde_json::Value::String(_) => "a string",
        serde_json::Value::Array(_) => "an array",
        serde_json::Value::Object(_) => "an object",
    }
}

/// `text` cut to its first 40 characters, so a message quoting a file's value stays short.
fn shortened(text: &str) -> String {
    const MAX_CHARS: usize = 40;
    match text.char_indices().nth(MAX_CHARS) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

/// The document's JSON Schema (draft 2020-12), as committed under `schema/format/`.
pub fn schema() -> serde_json::Value {
    schemars::schema_for!(DesignFile).to_value()
}

/// [`schema`] as the canonical text committed: two-space indents and a final newline.
pub fn schema_json() -> String {
    let mut text = serde_json::to_string_pretty(&schema())
        // A schema is a JSON value built from strings and maps, which always serialises.
        .unwrap_or_default();
    text.push('\n');
    text
}

pub mod container;
pub mod migrate;

#[cfg(test)]
mod tests;
