//! The hpr-sim library facade: re-exports of the workspace crates and a builder API for
//! environments, motors, rockets and flights.
//!
//! **Guide:** [Start here][guide-start] says what works and how far to trust it, and [Getting
//! started][guide-first] flies a first rocket.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [guide-first]: https://nrdptel.github.io/hpr-sim/getting-started.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//!
//! Status: pre-alpha skeleton. The builder API is planned for milestone [M4.1][roadmap] of the
//! roadmap. The `net` feature adds the online data sources from `hpr-net`, and the `parquet`
//! feature turns on `hpr-sim`'s Parquet export (`hpr_sim::export::parquet`, which `hpr` doesn't
//! re-export yet). One module holds code today: [`ork`], which flies the stage separation an
//! OpenRocket `.ork` file describes.

pub mod ork;
