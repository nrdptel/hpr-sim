//! What a reader accepted but wants to say out loud.

use serde::{Deserialize, Serialize};

/// Something the reader accepted and flagged, rather than failing over.
///
/// The roadmap asks the `.ork` importer for "graceful warnings instead of failures": a real design
/// written by an older OpenRocket, or by another program, should still open. Every departure from
/// what the file format documents is reported here, with the place in the file it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// Where it happened: a zip entry name, or a slash-separated path of element names from the
    /// root, such as `openrocket/rocket/subcomponents/nosecone`.
    pub at: String,
    /// Whether anything was lost.
    pub kind: WarningKind,
    /// What was read, and how.
    pub message: String,
}

/// How serious a [`Warning`] is.
///
/// The same three levels `hpr_motor::WarningKind` uses for `.eng` and `.rse` files. The two enums
/// are kept apart because their readers report different places — a line number there, a path into
/// an archive or an element tree here — and because `hpr-io` does not otherwise need `hpr-motor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum WarningKind {
    /// A whole part of the file was left out of the result, such as an archive entry compressed
    /// by a method this reader does not implement.
    Skipped,
    /// A value was dropped or ignored.
    Dropped,
    /// Something unusual was read as it stands, such as a schema version newer than any documented.
    Unusual,
}

impl Warning {
    /// Records a warning at `at`.
    pub(crate) fn new(
        at: impl Into<String>,
        kind: WarningKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            at: at.into(),
            kind,
            message: message.into(),
        }
    }
}

/// A value read from a `.ork` file and the warnings its reader raised, in the order they arose.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Imported<T> {
    /// The value read.
    pub value: T,
    /// The warnings, in file order.
    pub warnings: Vec<Warning>,
}

impl<T> Imported<T> {
    /// Wraps a value with no warnings.
    pub(crate) fn clean(value: T) -> Self {
        Self {
            value,
            warnings: Vec::new(),
        }
    }

    /// Counts the warnings of one kind.
    pub fn count(&self, kind: WarningKind) -> usize {
        self.warnings.iter().filter(|w| w.kind == kind).count()
    }
}
