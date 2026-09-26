//! Foreign formats: OpenRocket `.ork`, RockSim `.rkt`, RASAero `.CDX1`, RocketPy export, the
//! `.orc` parts database, and ERA5 weather in netCDF classic files.
//!
//! **Guide:** [Start here][guide-start] says what works today and what is planned.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//!
//! Status: pre-alpha. An OpenRocket `.ork` file reads into a design, its motors and recovery
//! ([`ork`]); the other design formats are [M3.2][roadmap] to [M3.6][roadmap] and
//! [M5.5][roadmap]. An ERA5 pressure-level file gives the atmosphere over a launch site
//! ([`era5`], [ERA5 weather files][guide-era5]), read by the netCDF classic reader
//! ([`netcdf`]).
//!
//! [guide-era5]: https://nrdptel.github.io/hpr-sim/format/era5.html

pub mod era5;
pub mod netcdf;
pub mod ork;
