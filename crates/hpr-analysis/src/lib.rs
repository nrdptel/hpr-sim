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
//!
//! Status: dispersion and the apogee's spread (milestone [M6.1a][roadmap] of the roadmap).
//! Landing ellipses, sensitivity analysis, optimization and challenge specifications are planned
//! for the rest of [M6.1][roadmap] to [M6.3][roadmap].

mod error;
pub mod montecarlo;
pub mod statistics;

pub use error::AnalysisError;
