//! Monte Carlo dispersion, sensitivity analysis, optimization and challenge specifications.
//!
//! **Guide:** [Monte Carlo dispersion][guide-mc] shows a run and how far to trust it;
//! [Start here][guide-start] says what works today and what is planned.
//!
//! [guide-mc]: https://nrdptel.github.io/hpr-sim/monte-carlo.html
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//!
//! - [`montecarlo`]: one rocket flown many times with its uncertain inputs drawn afresh each
//!   time, seeded and reproducible sample by sample, in parallel with the `parallel` feature.
//! - [`statistics`]: the spread of what the samples gave, with the failed ones counted.
//! - [`ellipse`]: where the landings scatter, as an ellipse holding a chosen share of them.
//!
//! Status: dispersion, the apogee's spread and landing ellipses (milestones [M6.1a and
//! M6.1b][roadmap] of the roadmap). Sensitivity analysis, optimization and challenge
//! specifications are planned for the rest of [M6.1][roadmap] to [M6.3][roadmap].

pub mod ellipse;
mod error;
pub mod montecarlo;
pub mod statistics;

pub use error::AnalysisError;
