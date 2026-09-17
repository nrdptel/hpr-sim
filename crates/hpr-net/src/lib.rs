//! Optional online data sources with an on-disk cache and an explicit offline mode: weather,
//! soundings, elevation, ThrustCurve and motor stock.
//!
//! Status: pre-alpha skeleton. This crate does network and file I/O, so it is never a dependency of
//! the pure core. The online layer arrives in M5.1.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the online layer does network and cache I/O; it is not part of the pure core"
)]
