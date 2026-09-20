//! Why a `.ork` file could not be read at all.

use thiserror::Error;

/// A `.ork` file that could not be opened or whose design document could not be parsed.
///
/// A reader that *can* go on raises a [`Warning`](super::Warning) instead: only the faults listed
/// here stop the read, and each one leaves nothing useful behind.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum OrkError {
    /// The bytes start with neither a zip signature, a gzip signature, nor XML.
    #[error("not a .ork file: it begins with {head}, which is neither zip, gzip nor XML")]
    UnknownContainer {
        /// The first bytes, printed as hex, to say what was seen instead.
        head: String,
    },
    /// The zip container could not be read.
    #[error("the zip container could not be read: {reason}")]
    Zip {
        /// What the zip reader said.
        reason: String,
    },
    /// The gzip stream could not be decompressed.
    #[error("the gzip stream could not be decompressed: {reason}")]
    Gzip {
        /// What the decompressor said.
        reason: String,
    },
    /// The zip container holds no entry that could be the design document.
    #[error("the zip container holds no design document (no `rocket.ork`, `.ork` or `.xml` entry)")]
    NoDesign,
    /// The design document is not UTF-8. OpenRocket writes UTF-8 and declares it.
    #[error("the design document is not UTF-8")]
    NotUtf8,
    /// The design document is not well-formed XML.
    #[error("the design document is not XML: {reason}")]
    Xml {
        /// What the XML parser said.
        reason: String,
    },
    /// The document's root element is not `<openrocket>`.
    #[error("the design document's root element is `{root}`, not `openrocket`")]
    NotOpenRocket {
        /// The root element's name.
        root: String,
    },
    /// The document nests deeper than [`MAX_DEPTH`](super::MAX_DEPTH).
    #[error("elements nest {depth} deep, past the limit of {limit}")]
    TooDeep {
        /// The limit that was passed.
        limit: usize,
        /// How deep the document nests.
        depth: usize,
    },
    /// The root element carries no `version` attribute, or one that is not `major.minor`.
    #[error("the schema version `{text}` is not `major.minor`")]
    Version {
        /// The attribute as written, or the empty string when it is missing.
        text: String,
    },
}
