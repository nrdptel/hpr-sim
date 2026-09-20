//! Flight-log importers, the canonical flight record, smoothing and time alignment, and the
//! readings taken from a flight with the provenance of each.
//!
//! **Guide:** [Start here][guide-start] says what works today and what is planned.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//! [adr-046]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-046-debrief-folded-in-and-flight-log-analysis-that-stands-without-the-simulator-2026-09-20
//!
//! This crate does not depend on the simulator, and [must not][adr-046]: reading a flight log and
//! working out what it says is a use of this project in its own right, for someone who has a log
//! and neither a design file nor any wish to simulate anything. Comparing a flight with a
//! simulation of it is `hpr-forensics`, which depends on both.
//!
//! Every reading carries where it came from — measured by an instrument, derived from what was
//! measured, or clipped because the sensor saturated — and a reading the log cannot support is
//! withheld with a reason rather than printed.
//!
//! Status: pre-alpha skeleton. The importers are planned for milestone [M7.1][roadmap] of the
//! roadmap, and the readings and reconstruction for [M7.2][roadmap].
