//! The validation harness: case files, oracle references, metrics and reports.
//!
//! **Guide:** [Accuracy][guide-accuracy] gives every validation result, and [Checking a
//! claim][guide-claim] traces a number to its source, its test and its validation.
//!
//! [guide-accuracy]: https://nrdptel.github.io/hpr-sim/accuracy.html
//! [guide-claim]: https://nrdptel.github.io/hpr-sim/checking-a-claim.html
//! [lessons]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
//! [validation]: https://github.com/nrdptel/hpr-sim/blob/main/docs/VALIDATION.md
//! [adr-015]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-015-the-validation-harness-cases-references-tolerances-and-reports-2026-09-17
//!
//! A **case** ([`Case`]) is a TOML file under `validation/cases/`: what to fly, how, and which
//! metrics to compare against which reference, each with its own tolerance. A **reference**
//! ([`Reference`]) is a JSON file under `validation/fixtures/`, written by a generator script
//! under `validation/oracles/`, carrying the oracle's name, the command that produced it and a
//! source for every value. Running a case gives a [`Comparison`] per metric, and the run gives a
//! [`Report`] in Markdown and JSON.
//!
//! Five [lessons from Loft][lessons], mistakes found in the project that came before hpr-sim, shape
//! the rules:
//!
//! - **[Loft lesson L75][lessons]:** an oracle's inputs come from the case file, never from hpr's
//!   own output.
//! - **[L76][lessons]:** a reference is never regenerated to make a comparison pass; it moves only
//!   when its generator runs, which is a separate, deliberate step.
//! - **[L77][lessons]:** every reference value carries provenance, so no number is a hand-written
//!   "stored result".
//! - **[L78][lessons]:** the cases that must run are locked, so a suite that silently skips one
//!   fails instead of reporting green.
//! - **[L79][lessons]:** every metric a case reports has a tolerance; nothing is "ungated".
//!
//! The method is in [the validation plan][validation], and the design in the decision record
//! [ADR-015][adr-015], the validation harness.

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

pub use case::{Case, CaseLock, DragMode, Metric, Tolerance, committed_cases};
pub use metrics::{Measured, Reference, ReferenceValue};
pub use report::{Comparison, Gap, NotReproduced, Report, Source, Verdict};
pub use run::{CaseRun, ValidateError, run_case, run_lock};

#[cfg(test)]
mod tests;
