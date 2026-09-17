//! Optional online data sources with an on-disk cache and an explicit offline mode: weather,
//! soundings, elevation, ThrustCurve and motor stock.
//!
//! Status: pre-alpha skeleton. This crate does network and file I/O, so it is never a dependency of
//! the pure core. The online layer arrives in M5.1.
