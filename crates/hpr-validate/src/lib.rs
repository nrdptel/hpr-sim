//! The validation harness: case files, oracle references, metrics and reports.
//!
//! A **case** ([`Case`]) is a TOML file under `validation/cases/`: what to fly, how, and which
//! metrics to compare against which reference, each with its own tolerance. A **reference**
//! ([`Reference`]) is a JSON file under `validation/fixtures/`, written by a generator script
//! under `validation/oracles/`, carrying the oracle's name, the command that produced it and a
//! source for every value. Running a case gives a [`Comparison`] per metric, and the run gives a
//! [`Report`] in Markdown and JSON.
//!
//! The Loft lessons this exists to prevent (`docs/research/loft-lessons.md`) shape the rules:
//!
//! - **L75:** an oracle's inputs come from the case file, never from hpr's own output.
//! - **L76:** a reference is never regenerated to make a comparison pass; it moves only when its
//!   generator runs, which is a separate, deliberate step.
//! - **L77:** every reference value carries provenance, so no number is a hand-written "stored
//!   result".
//! - **L78:** the cases that must run are locked, so a suite that silently skips one fails instead
//!   of reporting green.
//! - **L79:** every metric a case reports has a tolerance; nothing is "ungated".
//!
//! Method: `docs/VALIDATION.md`; decisions: ADR-015.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the validation harness reads case and reference files; it is not part of the pure core"
)]

pub mod case;
pub mod metrics;
pub mod report;
pub mod rocketpy;
pub mod run;

pub use case::{Case, CaseLock, Metric, Tolerance};
pub use metrics::{Measured, Reference, ReferenceValue};
pub use report::{Comparison, Report, Source, Verdict};
pub use run::{ValidateError, run_case, run_lock};

#[cfg(test)]
mod tests;
