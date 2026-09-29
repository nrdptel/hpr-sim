//! The `.hprz` container: a design and the files that go with it, such as flight logs, results
//! and photographs, in one zip archive ([ADR-112][adr-112]).
//!
//! The archive holds the design as its first entry, [`DESIGN_ENTRY`], written exactly as a `.hpr`
//! file ([`to_json`]), so unzipping a container gives a `.hpr` any reader of the
//! format takes. Every other entry is an attachment, kept byte for byte under its name, in order.
//! Every entry is deflated and dated 1980-01-01, zip's zero date, so with one build of this crate a
//! container's bytes depend only on what it holds. (Another deflate implementation, chosen by a
//! program's features, can compress the same files to other bytes, which read back the same.)
//!
//! An attachment's name is a relative path with `/` between folders, so an archive can't place a
//! file outside the folder it is unpacked into ([`check_name`]). A container holds at most
//! [`MAX_UNPACKED_BYTES`], unpacked. Reading is held to the same rules as writing, so a hostile
//! archive is refused with its reason rather than filling memory.
//!
//! ```
//! use hpr_format::DesignFile;
//! use hpr_format::container::{self, Entry, Hprz};
//!
//! let bytes = include_bytes!("../../../validation/fixtures/ork/loft-demo/demo-stable.ork");
//! let design = DesignFile::from_ork(bytes)?.value;
//! let log = Entry::new("logs/first-flight.csv", b"time_s,altitude_m\n0,0\n".to_vec());
//! let hprz = Hprz::new(design, vec![log]);
//! let written = container::write(&hprz)?;
//! let read = container::read(&written)?;
//! assert_eq!(read.value, hprz);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! [adr-112]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-112-m33b-the-hprz-container-and-migrations-2026-09-29

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};

use crate::{DesignFile, FormatError, Opened, read_json, shortened, to_json};

/// The name of the design's entry, the first in every container this crate writes.
pub const DESIGN_ENTRY: &str = "design.hpr";

/// The most a container holds, unpacked, over all its entries, the design's included: 256 MiB.
/// [`write`](fn@write) refuses more, and [`read`] decompresses no more.
///
/// A deflate stream can expand by about a thousand to one, so a small archive could otherwise ask
/// for more memory than a machine has. Use [`read_within`] to read with another limit.
pub const MAX_UNPACKED_BYTES: u64 = 256 * 1024 * 1024;

/// A design and its attachments.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Hprz {
    /// The design.
    pub design: DesignFile,
    /// The files that go with it, in the order the archive holds them.
    pub attachments: Vec<Entry>,
}

impl Hprz {
    /// A container of `design` and `attachments`.
    pub fn new(design: DesignFile, attachments: Vec<Entry>) -> Self {
        Self {
            design,
            attachments,
        }
    }
}

/// A file in a container: its name, a relative path such as `logs/flight-1.csv`, and its bytes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct Entry {
    /// Its name in the archive, with `/` between folders.
    pub name: String,
    /// Its contents.
    pub bytes: Vec<u8>,
}

impl Entry {
    /// The file `name` holding `bytes`.
    pub fn new(name: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            name: name.into(),
            bytes,
        }
    }
}

/// Whether `name` can name an attachment, and why not if it can't.
///
/// A name is a relative path: folders and a file name joined by `/`, none of them empty, `.` or
/// `..`. It holds no `\`, no `:` and no control character, since each can reach outside the
/// folder a container is unpacked into on some system. No part ends in `.` or a space, and none is
/// a Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to `COM9`, `LPT1` to `LPT9`, with or
/// without an extension), which Windows would drop or open as a device. And it is not
/// [`DESIGN_ENTRY`] in any case.
///
/// # Errors
///
/// [`FormatError::Container`], with the reason.
pub fn check_name(name: &str) -> Result<(), FormatError> {
    let quoted = shortened(name);
    let refused = |why: String| Err(FormatError::Container(why));
    if name.is_empty() {
        return refused("an attachment's name is empty".to_owned());
    }
    if name.eq_ignore_ascii_case(DESIGN_ENTRY) {
        return refused(format!(
            "an attachment is named {quoted:?}, which is the design's entry"
        ));
    }
    if let Some(bad) = name
        .chars()
        .find(|c| *c == '\\' || *c == ':' || c.is_control())
    {
        return refused(format!(
            "the attachment {quoted:?} has {bad:?} in its name, which a relative path with `/` \
             between folders doesn't"
        ));
    }
    if name
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return refused(format!(
            "the attachment {quoted:?} is not a relative path: it starts or ends with `/`, or has \
             an empty, `.` or `..` part"
        ));
    }
    if name.split('/').any(|part| part.ends_with(['.', ' '])) {
        return refused(format!(
            "the attachment {quoted:?} has a part ending in `.` or a space, which Windows drops"
        ));
    }
    if let Some(part) = name.split('/').find(|part| is_device(part)) {
        return refused(format!(
            "the attachment {quoted:?} has a part named {:?}, a Windows device's name",
            shortened(part)
        ));
    }
    Ok(())
}

/// Whether `part` of a path names a Windows device, with or without an extension.
fn is_device(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part).to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || matches!(
            stem.as_bytes(),
            [b'C', b'O', b'M', b'1'..=b'9'] | [b'L', b'P', b'T', b'1'..=b'9']
        )
}

/// The names of a container's attachments checked, each by [`check_name`]; no two the same,
/// ignoring case, and none also a folder of another, since a file system that ignores case, or any
/// file system, would unpack such a pair as one.
fn check_names<'a>(names: impl Iterator<Item = &'a str> + Clone) -> Result<(), FormatError> {
    let mut seen = BTreeSet::new();
    let mut folders = BTreeSet::new();
    for name in names.clone() {
        check_name(name)?;
        let folded = name.to_lowercase();
        if !seen.insert(folded.clone()) {
            return Err(FormatError::Container(format!(
                "two attachments are named {:?}, ignoring case",
                shortened(name)
            )));
        }
        folders.extend(
            folded
                .match_indices('/')
                .map(|(at, _)| folded[..at].to_owned()),
        );
    }
    match names
        .into_iter()
        .find(|name| folders.contains(&name.to_lowercase()))
    {
        Some(name) => Err(FormatError::Container(format!(
            "the attachment {:?} is also the folder of another",
            shortened(name)
        ))),
        None => Ok(()),
    }
}

/// The container's bytes: the design as [`DESIGN_ENTRY`], then each attachment, in order.
///
/// # Errors
///
/// What [`to_json`] refuses in the design, and [`FormatError::Container`] for an attachment's name
/// [`check_name`] refuses, two of one name, or more than [`MAX_UNPACKED_BYTES`] in all.
pub fn write(hprz: &Hprz) -> Result<Vec<u8>, FormatError> {
    let text = to_json(&hprz.design)?;
    check_names(hprz.attachments.iter().map(|entry| entry.name.as_str()))?;
    let unpacked = hprz
        .attachments
        .iter()
        .fold(text.len() as u64, |sum, entry| {
            sum.saturating_add(entry.bytes.len() as u64)
        });
    within_the_limit(unpacked)?;
    let zip = |error: &dyn std::fmt::Display| FormatError::Container(error.to_string());
    // `SimpleFileOptions::DEFAULT` and a named date rather than `default()`, which reads the clock
    // when zip's `time` feature is on, and panics on `wasm32-unknown-unknown` (as `hpr-io` writes
    // a `.ork`).
    let options = zip::write::SimpleFileOptions::DEFAULT
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::DEFAULT);
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    archive
        .start_file(DESIGN_ENTRY, options)
        .map_err(|e| zip(&e))?;
    archive.write_all(text.as_bytes()).map_err(|e| zip(&e))?;
    for entry in &hprz.attachments {
        archive
            .start_file(entry.name.as_str(), options)
            .map_err(|e| zip(&e))?;
        archive.write_all(&entry.bytes).map_err(|e| zip(&e))?;
    }
    Ok(archive.finish().map_err(|e| zip(&e))?.into_inner())
}

/// Refuses `unpacked` bytes past [`MAX_UNPACKED_BYTES`].
fn within_the_limit(unpacked: u64) -> Result<(), FormatError> {
    if unpacked > MAX_UNPACKED_BYTES {
        return Err(FormatError::Container(format!(
            "it would hold {unpacked} bytes unpacked, and a container holds at most \
             {MAX_UNPACKED_BYTES} (256 MiB)"
        )));
    }
    Ok(())
}

/// How many records a zip archive's central directory holds from `start`: each begins with the
/// signature `PK\x01\x02` and is 46 bytes, then its name, extra field and comment, whose lengths
/// are the little-endian `u16`s at 28, 30 and 32 (APPNOTE.TXT 6.3.10, section 4.3.12).
fn central_records(bytes: &[u8], start: usize) -> usize {
    const SIGNATURE: &[u8] = b"PK\x01\x02";
    const FIXED: usize = 46;
    let u16_at = |at: usize| usize::from(u16::from_le_bytes([bytes[at], bytes[at + 1]]));
    let mut count = 0;
    let mut at = start;
    while let Some(record) = bytes.get(at..at.saturating_add(FIXED))
        && record.starts_with(SIGNATURE)
    {
        count += 1;
        // In range: the fixed part was just read.
        at += FIXED + u16_at(at + 28) + u16_at(at + 30) + u16_at(at + 32);
    }
    count
}

/// Reads a container, taking a design of an older version to the current one.
///
/// [`read_within`] with [`MAX_UNPACKED_BYTES`].
///
/// # Errors
///
/// As [`read_within`].
pub fn read(bytes: &[u8]) -> Result<Opened<Hprz>, FormatError> {
    read_within(bytes, MAX_UNPACKED_BYTES)
}

/// Reads a container, decompressing at most `max_unpacked_bytes` out of it, and says which version
/// its design was written in.
///
/// The design is read as a `.hpr` is ([`read_json`]). A folder's own entry, which some zip tools
/// write, holds nothing and is passed over; every other entry is an attachment, held to the rules
/// [`write`](fn@write) holds it to. Every name is checked before anything is decompressed.
///
/// # Errors
///
/// [`FormatError::Container`] when the bytes are not a zip archive, an entry can't be read, the
/// central directory holds more entries than the zip reader keeps (two of one name), an entry is a
/// symbolic link or its name isn't stored as UTF-8, an attachment's name is refused, the archive
/// would decompress to more than `max_unpacked_bytes`, or it holds no [`DESIGN_ENTRY`] or the
/// design isn't UTF-8; and what [`read_json`] refuses in the design.
pub fn read_within(bytes: &[u8], max_unpacked_bytes: u64) -> Result<Opened<Hprz>, FormatError> {
    let budget = max_unpacked_bytes;
    if !matches!(bytes, [b'P', b'K', 3, 4, ..] | [b'P', b'K', 5, 6, ..]) {
        return Err(FormatError::Container(
            "a .hprz is a zip archive, and these bytes don't start as one".to_owned(),
        ));
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| FormatError::Container(error.to_string()))?;
    // The zip reader keeps one entry of each name and drops the others without a word, so the
    // central directory's records are counted here: more records than entries means two share a
    // name, and one of them would be lost.
    let start = usize::try_from(archive.central_directory_start()).unwrap_or(usize::MAX);
    let records = central_records(bytes, start);
    if records != archive.len() {
        return Err(FormatError::Container(format!(
            "its central directory holds {records} entries, and the zip reader keeps {}: two \
             have the same name, and one would be lost, or the directory is damaged",
            archive.len()
        )));
    }
    // Every name, before anything is decompressed.
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).map_err(|error| {
            FormatError::Container(format!("entry {index} can't be opened: {error}"))
        })?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name();
        if entry.name_raw() != name.as_bytes() {
            return Err(FormatError::Container(format!(
                "the name of {:?} isn't stored as UTF-8",
                shortened(name)
            )));
        }
        if entry.is_symlink() {
            return Err(FormatError::Container(format!(
                "{:?} is a symbolic link, which a container doesn't hold",
                shortened(name)
            )));
        }
        names.push(name.to_owned());
    }
    // The first entry of the design's name is the design, as below; the central directory's count
    // has already refused a second.
    let Some(at) = names.iter().position(|name| name == DESIGN_ENTRY) else {
        return Err(FormatError::Container(format!(
            "it holds no {DESIGN_ENTRY:?}, the design"
        )));
    };
    names.remove(at);
    check_names(names.iter().map(String::as_str))?;
    let mut design = None;
    let mut attachments = Vec::new();
    let mut used = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| {
            FormatError::Container(format!("entry {index} can't be opened: {error}"))
        })?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        let left = budget.saturating_sub(used);
        let mut content = Vec::new();
        // One byte past what is left, so that an entry filling the budget exactly is still known
        // to have overrun it.
        entry
            .by_ref()
            .take(left.saturating_add(1))
            .read_to_end(&mut content)
            .map_err(|error| {
                FormatError::Container(format!(
                    "{:?} can't be decompressed: {error}",
                    shortened(&name)
                ))
            })?;
        if content.len() as u64 > left {
            return Err(FormatError::Container(format!(
                "it decompresses to more than {budget} bytes, the most one container may"
            )));
        }
        used += content.len() as u64;
        if name == DESIGN_ENTRY && design.is_none() {
            design = Some(content);
        } else {
            attachments.push(Entry::new(name, content));
        }
    }
    let design = design.ok_or_else(|| {
        FormatError::Container(format!("it holds no {DESIGN_ENTRY:?}, the design"))
    })?;
    let text = String::from_utf8(design)
        .map_err(|_| FormatError::Container(format!("its {DESIGN_ENTRY:?} is not UTF-8 text")))?;
    let opened = read_json(&text)?;
    Ok(Opened {
        value: Hprz::new(opened.value, attachments),
        written_as: opened.written_as,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The limit itself is held, and one byte more is not: the writer's test refuses one past it
    /// without compressing 256 MiB to show the limit is taken.
    #[test]
    fn a_container_may_hold_the_limit_and_no_more() {
        assert!(within_the_limit(MAX_UNPACKED_BYTES).is_ok());
        assert!(within_the_limit(MAX_UNPACKED_BYTES + 1).is_err());
    }
}
