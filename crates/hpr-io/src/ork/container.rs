//! The three packages a `.ork` file arrives in, told apart by their first bytes.
//!
//! OpenRocket writes a zip archive holding an entry called `rocket.ork`, which is the design as
//! XML, beside whatever else the design carries (motor files, a preview image). Older versions
//! wrote the XML gzipped, and a plain XML file is accepted too — the format documentation calls
//! all three `.ork`. Nothing here touches the filesystem: the caller hands over the bytes.

use std::io::{Cursor, Read};

use super::error::OrkError;
use super::warning::{Imported, Warning, WarningKind};

/// The three ways a `.ork` file packages its design document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Container {
    /// A zip archive; the design is the `rocket.ork` entry. What OpenRocket writes today.
    Zip,
    /// A single gzip stream whose contents are the design document.
    Gzip,
    /// The design document itself, uncompressed.
    Xml,
}

impl Container {
    /// The name the format documentation gives this container.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::Gzip => "gzip",
            Self::Xml => "xml",
        }
    }

    /// Names the container `bytes` are packed in, from its first bytes alone.
    ///
    /// This is [Loft lesson L56: containers are told apart by their magic bytes, and malformed
    /// input must give an error rather than crash][lessons].
    ///
    /// [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
    ///
    /// Zip archives begin `PK` followed by `\x03\x04` (a local file header), `\x05\x06` (an empty
    /// archive) or `\x07\x08` (a spanned one); gzip members begin `\x1f\x8b`. Anything else is
    /// taken to be XML if, once a byte-order mark and leading whitespace are passed, it begins
    /// with `<`. Returns `None` when it is none of the three.
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [b'P', b'K', 3 | 5 | 7, 4 | 6 | 8, ..] => Some(Self::Zip),
            [0x1f, 0x8b, ..] => Some(Self::Gzip),
            _ if xml_body(bytes).starts_with(b"<") => Some(Self::Xml),
            _ => None,
        }
    }
}

/// An entry of a `.ork` archive that is not the design document, kept as it was stored.
///
/// A `.ork` may carry the thrust curves its motors were flown with (`thrustcurves/*.rse`, schema
/// 1.11), a `preview.png`, or lookup tables. They are held here byte for byte so that later
/// milestones can read them and an export can put them back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    /// The entry's name inside the archive, as stored, with `/` between directories.
    pub name: String,
    /// The entry's contents, decompressed.
    pub bytes: Vec<u8>,
}

/// A `.ork` file unpacked: the design document as text, and everything else that came with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpacked {
    /// Which of the three containers it was in.
    pub container: Container,
    /// The name of the archive entry the design came from, when it came from an archive.
    pub design_entry: Option<String>,
    /// The design document, as XML text.
    pub design: String,
    /// Every other entry of the archive, in archive order. Empty for gzip and raw XML.
    pub attachments: Vec<Attachment>,
}

/// Unpacks `bytes`, whichever of the three containers they are in.
///
/// Fails only when the bytes are not a `.ork` file at all, when the container itself is damaged,
/// or when no entry inside it can be the design. An archive entry that cannot be decompressed is
/// skipped with a warning, unless it is the design.
pub fn unpack(bytes: &[u8]) -> Result<Imported<Unpacked>, OrkError> {
    match Container::sniff(bytes) {
        Some(Container::Zip) => unpack_zip(bytes),
        Some(Container::Gzip) => {
            let mut text = Vec::new();
            flate2::read::GzDecoder::new(bytes)
                .read_to_end(&mut text)
                .map_err(|error| OrkError::Gzip {
                    reason: error.to_string(),
                })?;
            Ok(Imported::clean(Unpacked {
                container: Container::Gzip,
                design_entry: None,
                design: utf8(&text)?,
                attachments: Vec::new(),
            }))
        }
        Some(Container::Xml) => Ok(Imported::clean(Unpacked {
            container: Container::Xml,
            design_entry: None,
            design: utf8(bytes)?,
            attachments: Vec::new(),
        })),
        None => Err(OrkError::UnknownContainer { head: head(bytes) }),
    }
}

/// The name OpenRocket gives the design entry inside the archive.
const DESIGN_ENTRY: &str = "rocket.ork";

fn unpack_zip(bytes: &[u8]) -> Result<Imported<Unpacked>, OrkError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|error| OrkError::Zip {
        reason: error.to_string(),
    })?;
    let mut warnings = Vec::new();
    let mut entries: Vec<Attachment> = Vec::new();
    for index in 0..archive.len() {
        let mut entry = match archive.by_index(index) {
            Ok(entry) => entry,
            Err(error) => {
                warnings.push(Warning::new(
                    format!("entry {index}"),
                    WarningKind::Skipped,
                    format!("this entry could not be opened and was left out: {error}"),
                ));
                continue;
            }
        };
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        let mut content = Vec::new();
        if let Err(error) = entry.read_to_end(&mut content) {
            warnings.push(Warning::new(
                name,
                WarningKind::Skipped,
                format!("this entry could not be decompressed and was left out: {error}"),
            ));
            continue;
        }
        entries.push(Attachment {
            name,
            bytes: content,
        });
    }

    let chosen = choose_design(&entries, &mut warnings).ok_or(OrkError::NoDesign)?;
    let design_bytes = entries.remove(chosen);
    let design = utf8(&design_bytes.bytes)?;
    Ok(Imported {
        value: Unpacked {
            container: Container::Zip,
            design_entry: Some(design_bytes.name),
            design,
            attachments: entries,
        },
        warnings,
    })
}

/// Picks the entry that holds the design, preferring the documented name.
fn choose_design(entries: &[Attachment], warnings: &mut Vec<Warning>) -> Option<usize> {
    if let Some(index) = entries.iter().position(|e| e.name == DESIGN_ENTRY) {
        return Some(index);
    }
    let candidates: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            let lower = e.name.to_ascii_lowercase();
            lower.ends_with(".ork") || lower.ends_with(".xml")
        })
        .map(|(index, _)| index)
        .collect();
    let first = *candidates.first()?;
    warnings.push(Warning::new(
        entries[first].name.clone(),
        WarningKind::Unusual,
        if candidates.len() > 1 {
            format!(
                "the archive has no `{DESIGN_ENTRY}`; this is the first of {} entries that could \
                 be the design, and the others are kept as attachments",
                candidates.len()
            )
        } else {
            format!("the archive has no `{DESIGN_ENTRY}`; this entry was read as the design")
        },
    ));
    Some(first)
}

/// Passes a UTF-8 byte-order mark and any leading whitespace, so that sniffing sees the first tag.
fn xml_body(bytes: &[u8]) -> &[u8] {
    let rest = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let start = rest
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(rest.len());
    &rest[start..]
}

fn utf8(bytes: &[u8]) -> Result<String, OrkError> {
    let text = std::str::from_utf8(bytes).map_err(|_| OrkError::NotUtf8)?;
    Ok(text.strip_prefix('\u{feff}').unwrap_or(text).to_owned())
}

/// The first four bytes as hex, for the error that says what was seen instead of a `.ork`.
fn head(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "nothing (the file is empty)".to_owned();
    }
    bytes
        .iter()
        .take(4)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
