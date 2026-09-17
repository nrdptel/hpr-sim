//! The validation harness: case files, oracle references, metrics and reports.
//!
//! Status: pre-alpha skeleton. This crate reads case and reference files, so it is not part of the
//! pure core. The harness arrives in M2.1.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the validation harness reads case and reference files; it is not part of the pure core"
)]
