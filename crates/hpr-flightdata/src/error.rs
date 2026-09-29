//! Why a flight log couldn't be read.

use thiserror::Error;

/// An error reading a flight log: the text isn't the format it should be, or its data can't be a
/// flight's record.
#[derive(Debug, Clone, PartialEq, Error)]
#[non_exhaustive]
pub enum LogError {
    /// A line of the file is malformed.
    #[error("{format} line {line}: {message}")]
    Syntax {
        /// The file format, such as `.pf2`.
        format: &'static str,
        /// The 1-based line number.
        line: usize,
        /// What is wrong.
        message: String,
    },
    /// The file isn't in the format asked for.
    #[error("not a {format} file: {message}")]
    NotThisFormat {
        /// The format the file was read as.
        format: &'static str,
        /// Why it isn't one.
        message: String,
    },
    /// The file has no data rows.
    #[error("{format}: the file has no data rows")]
    NoData {
        /// The file format.
        format: &'static str,
    },
    /// The file states its values in a unit the reader doesn't know, such as a PerfectFlite file
    /// whose stated apogee carries no foot mark.
    #[error("{format} line {line}: {message}")]
    Unit {
        /// The file format.
        format: &'static str,
        /// The 1-based line number.
        line: usize,
        /// What is wrong.
        message: String,
    },
}
