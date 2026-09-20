//! Flight against simulation: residuals, parameter identification and fault diagnosis.
//!
//! **Guide:** [Start here][guide-start] says what works today and what is planned.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//! [adr-046]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-046-debrief-folded-in-and-flight-log-analysis-that-stands-without-the-simulator-2026-09-20
//!
//! This is the only crate where a reading taken from a flight log and a number the simulator
//! produced meet. Reading a log and working out what it says is [`hpr_flightdata`], which does not
//! depend on the simulator, so that [analysing a flight stands on its own][adr-046].
//!
//! Status: pre-alpha skeleton. Residuals and parameter identification are planned for milestone
//! [M7.3][roadmap] of the roadmap, and fault diagnosis for [M7.4][roadmap].
