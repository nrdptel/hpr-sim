//! Foreign design formats: OpenRocket `.ork`, RockSim `.rkt`, RASAero `.CDX1`, RocketPy export and
//! the `.orc` parts database.
//!
//! **Guide:** [Start here][guide-start] says what works today and what is planned.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//!
//! Status: pre-alpha. Reading an OpenRocket `.ork` file's container and design document works
//! ([`ork`]); turning that document into a design is milestone [M3.1b][roadmap], and the other
//! formats are [M3.2][roadmap] to [M3.6][roadmap] and [M5.5][roadmap].

pub mod ork;
