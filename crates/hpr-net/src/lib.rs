//! Optional online data sources with an on-disk cache and an explicit offline mode: weather,
//! soundings, elevation, ThrustCurve and motor stock.
//!
//! **Guide:** [Start here][guide-start] says what works today and what is planned.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//!
//! Status: pre-alpha skeleton. This crate does network and file I/O, so it is never a dependency of
//! the pure core. The online layer is planned for milestone [M5.1][roadmap] of the roadmap.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the online layer does network and cache I/O; it is not part of the pure core"
)]
