//! Foreign formats: OpenRocket `.ork`, RockSim `.rkt`, RASAero `.CDX1`, RocketPy export, the
//! `.orc` parts database, ERA5 weather in netCDF classic files, GRIB2 weather fields, and GeoTIFF
//! elevation files.
//!
//! **Guide:** [Start here][guide-start] says what works today and what is planned.
//!
//! [guide-start]: https://nrdptel.github.io/hpr-sim/start-here.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//!
//! Status: pre-alpha. An OpenRocket `.ork` file reads into a design, its motors and recovery
//! ([`ork`]); the other design formats are [M3.2][roadmap] to [M3.6][roadmap]. OpenRocket's
//! `.orc` parts catalogues read into parts by maker and part number, and the 16 files OpenRocket
//! 24.12 ships are built in ([`orc`], [M5.5a][roadmap], [parts catalogues][guide-orc]). An ERA5 pressure-level file gives the atmosphere over a launch site
//! ([`era5`], [ERA5 weather files][guide-era5]), read by the netCDF classic reader
//! ([`netcdf`]). [`grib2`] decodes the GRIB2 fields NOAA's NOMADS cuts from its GFS and RAP
//! forecasts ([M5.2c][roadmap]) and whole GFS files' complex packing ([M5.2d2][roadmap]) and
//! JPEG 2000 ([M5.2d3][roadmap]). [`geotiff`] reads a launch site's height from a GeoTIFF
//! elevation file ([M5.3c2][roadmap], [A launch site's elevation][guide-elevation]).
//!
//! [guide-era5]: https://nrdptel.github.io/hpr-sim/format/era5.html
//! [guide-orc]: https://nrdptel.github.io/hpr-sim/format/orc.html
//! [guide-elevation]: https://nrdptel.github.io/hpr-sim/elevation.html#from-an-elevation-file-of-your-own

pub mod era5;
pub mod geotiff;
pub mod grib2;
pub mod netcdf;
pub mod orc;
pub mod ork;
