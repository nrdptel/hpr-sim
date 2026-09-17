# Third-party notices

hpr-sim is licensed under either of MIT or Apache-2.0, at your option. This file records every
outside source the project uses, its license, and how it is used. Update it in the same PR that
adds a source.

**Usage modes:**

- **bundled:** copied into this repository or into built artifacts. Needs a compatible license and
  its notice reproduced below.
- **dependency:** a Rust crate from crates.io. `cargo deny check` enforces the license allow list
  in `deny.toml`; copyleft licenses are rejected.
- **fetched:** downloaded into the gitignored `refs/` by tooling, and never committed.
- **run-only:** executed as an external oracle. Its source code is never read or ported.
- **ported:** code or methods ported from a permissively licensed source, with attribution here.

## Bundled

- **ThrustCurve.org thrust curves** (`crates/hpr-motor/data/thrustcurve/curves/`, compiled into
  `hpr-motor`): 32 files that ThrustCurve.org marks public domain (license `PD`), unchanged. The
  index `crates/hpr-motor/data/thrustcurve/catalog.json` records each file's source URL, simfile
  id, data source (certification, manufacturer or user), SHA-256 and download date, and copies
  each motor's published statistics from the ThrustCurve.org API (unstated terms; factual values,
  used with attribution). Data courtesy of ThrustCurve.org, https://www.thrustcurve.org/. Curves
  marked "free", "other" or with no license are never bundled (ADR-005).

- **RocketPy example rocket inputs** (MIT, RocketPy v1.13.0): the masses, inertias, positions,
  motor dimensions and aerodynamic-surface dimensions of six example rockets, taken from RocketPy's
  notebooks and test code (never from its data files). They are recorded
  in `validation/fixtures/design/rocketpy-rocket-mass.json` with RocketPy's outputs, and turned into
  `validation/designs/rocketpy-*.json` by `cargo xtask designs`. RocketPy's motor thrust files carry
  their own terms and are **not** committed; the fixture pairs each example with a bundled
  public-domain curve instead (ADR-007). RocketPy's license, quoted under Ported, applies.
- **Comparisons with RocketPy's drag curves** (ADR-009): `cargo xtask aero` reads the Calisto,
  Juno III and Valetudo curves from the `refs/rocketpy` checkout and commits only derived numbers
  to `validation/fixtures/aero/rocketpy-drag-curves.json` (hpr's drag coefficients, the relative
  errors and each file's sha256). The curves themselves carry their own terms and are **not**
  committed. The designs' drag inputs cite RASAero II's Users Manual (2019, p. 53) for its default
  surface finish; the manual itself is not redistributed.

## Ported

- **RocketPy** (MIT), `rocketpy/motors/solid_motor.py` at v1.13.0: the BATES grain regression
  geometry and the grain-stack inertia, re-derived in closed form in `hpr_motor::grains`, and the
  constant-exhaust-velocity consumption in `hpr_motor::motor`. RocketPy's license applies to those
  portions:

  > MIT License. Copyright (c) 2018 Giovani Hidalgo Ceotto. Permission is hereby granted, free of
  > charge, to any person obtaining a copy of this software and associated documentation files
  > (the "Software"), to deal in the Software without restriction, including without limitation
  > the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
  > the Software, and to permit persons to whom the Software is furnished to do so, subject to the
  > following conditions: The above copyright notice and this permission notice shall be included
  > in all copies or substantial portions of the Software. THE SOFTWARE IS PROVIDED "AS IS",
  > WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES
  > OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  > AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
  > ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
  > OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## Rust dependencies

`Cargo.lock` lists the full dependency graph. Direct third-party dependencies:

| crate | license | used by | why |
|---|---|---|---|
| `criterion` | Apache-2.0 OR MIT | `hpr-core` (benchmarks only) | statistics for `cargo bench` (`docs/perf.md`) |
| `glam` | MIT OR Apache-2.0 | `hpr-core` | `f64` vectors, quaternions and matrices (`ARCHITECTURE.md`) |
| `proptest` | MIT OR Apache-2.0 | `hpr-core`, `hpr-atmos`, `hpr-motor` (tests only) | property tests |
| `rand_core` | MIT OR Apache-2.0 | `hpr-core` (tests only) | the generator traits `rand_xoshiro` implements |
| `rand_xoshiro` | MIT OR Apache-2.0 | `hpr-core` (tests only) | an independent xoshiro256++ and SplitMix64 that `hpr_core::random` is checked against, bit for bit |
| `roxmltree` | MIT OR Apache-2.0 | `hpr-motor` | a strict, read-only XML 1.0 parser for `.rse` motor files |
| `serde_json` | MIT OR Apache-2.0 | `xtask`, `hpr-motor`; `hpr-core`, `hpr-atmos` (tests only) | reads `cargo metadata` output and the bundled motor catalog index; serde round-trip tests and JSON fixtures |
| `serde` | MIT OR Apache-2.0 | `xtask`, `hpr-core`, `hpr-atmos`, `hpr-motor` | derives the `validation/refs.lock.toml` types and the public data types |
| `thiserror` | MIT OR Apache-2.0 | `hpr-core`, `hpr-atmos`, `hpr-motor` | library error types |
| `sha2` | MIT OR Apache-2.0 | `xtask`; `hpr-motor` (tests only) | SHA-256 of fetched references and of the bundled motor curves |
| `toml` | MIT OR Apache-2.0 | `xtask` | reads `validation/refs.lock.toml` |
| `tempfile` | MIT OR Apache-2.0 | `xtask` (tests only) | temporary directories for the `refs` tests |

## Reference library (`validation/refs.lock.toml`)

`cargo xtask refs fetch` downloads these into the gitignored `refs/`, pinned by commit or sha256;
none of them is committed. A test checks that every item in the lock file has a row here with the
same license and mode.

| name | source | license | mode | notes |
|---|---|---|---|---|
| `rocketpy` | RocketPy v1.13.0 (example rockets, flight data, RASAero Cd exports, acceptance tests) | MIT; data files carry their own terms | fetched | code is MIT and may be ported with attribution; flight data carries team permissions recorded in RocketPy's notebooks; the ERA5 weather files in `data/weather/` are Copernicus (C3S) data with attribution required; the NASADEM tile is NASA data |
| `openrocket-database` | `openrocket/openrocket-database` (`.orc` parts) | Apache-2.0 | fetched | may be bundled with notices in M5.5 |
| `fusionspace-loft` | `nrdptel/fusionspace-loft` (the project owner's own) | MIT | fetched | ported with a note |
| `loft-fixtures` | `nrdptel/loft-fixtures` | private; third-party design files | fetched | never committed; only derived statistics are published |
| `openrocket-jar` | OpenRocket 24.12 (needs Java 17+) | GPL-3.0 | run-only | second oracle; its source is never read |
| `barrowman-1967-thesis` | J. S. Barrowman, The Practical Calculation of the Aerodynamic Characteristics of Slender Finned Vehicles, MS thesis, 1967 (NASA/TM-2001-209983) | US government work | fetched | cited, not copied |
| `barrowman-1966-cp-report` | J. S. and J. A. Barrowman, The Theoretical Prediction of the Center of Pressure, NARAM-8, 1966 | unknown terms | fetched | cited, not copied or redistributed |
| `barrowman-1966-naram8-nakka` | J. S. and J. A. Barrowman, The Theoretical Prediction of the Center of Pressure, NARAM-8, 1966 (complete scan with its Testbed II and Aerobee 350 examples), bound with J. S. Barrowman, Calculating the Center of Pressure of a Model Rocket, Centuri TIR-33, 1970 | unknown terms | fetched | cited; the worked examples' printed dimensions and results (facts) are committed in `validation/fixtures/aero/barrowman-worked-examples.json` with page numbers; no figures and no text beyond part names copied |
| `tir-33-centuri-cp` | J. S. Barrowman, Calculating the Center of Pressure of a Model Rocket, Centuri TIR-33, 1970 (standalone scan) | unknown terms | fetched | cited, not copied or redistributed |
| `niskanen-2009-thesis` | S. Niskanen, Development of an Open Source model rocket simulation software, MSc thesis, 2009 | CC BY-NC-ND 1.0 Finland | fetched | read for methods only; no text copied |
| `openrocket-techdoc-13.05` | S. Niskanen, OpenRocket technical documentation, v13.05, 2013 | CC BY-SA 3.0 | fetched | read for methods only; no text copied |
| `us-std-atmosphere-1976` | U.S. Standard Atmosphere, 1976 (NOAA-S/T-76-1562, NASA-TM-X-74335) | US government work | fetched | cited, not copied |
| `nasa-tn-d-4013` | J. C. Ferris, Static stability investigation of a single-stage sounding rocket at Mach numbers from 0.60 to 1.20, NASA TN D-4013, 1967 | US government work | fetched | cited, not copied |
| `nasa-tn-d-4014` | C. D. Babb and D. E. Fuller, Static stability investigation of a sounding-rocket vehicle at Mach numbers from 1.50 to 4.63, NASA TN D-4014, 1967 | US government work | fetched | cited, not copied |
| `galejs-wind-instability` | R. Galejs, Wind Instability: What Barrowman Left Out, Sentinel 39 (about 1999) | unknown terms | fetched | cited, not copied or redistributed |
| `mil-hdbk-762` | MIL-HDBK-762(MI), Design of Aerodynamically Stabilized Free Rockets, 1990 (Distribution A) | US government work | fetched | cited, not copied |
| `naca-tn-4197` | D. J. Martin, Summary of Flutter Experiences as a Guide to the Preliminary Design of Lifting Surfaces on Missiles, NACA TN 4197, 1958 | US government work | fetched | cited, not copied |
| `fpl-gtr-190-wood-handbook` | Forest Products Laboratory, Wood Handbook: Wood as an Engineering Material, General Technical Report FPL-GTR-190, 2010 | US government work | fetched | cited for wood densities in `hpr_design::materials` (M1.4a) |
| `knacke-1991-parachute-manual` | T. W. Knacke, Parachute Recovery Systems Design Manual, NWC TP 6575, 1991 (DTIC ADA247666) | unclear terms | fetched | contractor report with a DTIC public-release stamp but a restrictive title-page notice; cited, never redistributed |
| `wgs84-nga-stnd-0036` | NGA.STND.0036_1.0.0_WGS84, Department of Defense World Geodetic System 1984, Its Definition and Relationships with Local Geodetic Systems, 2014 | US government work | fetched | cited for the ellipsoid, geodetic conversion and normal gravity (M1.1); no text copied |
| `karney-2011-geodesics` | C. F. F. Karney, Geodesics on an ellipsoid of revolution, arXiv:1102.1215v1, 2011 | arXiv non-exclusive license | fetched | appendix B cited for the ECEF-to-geodetic conversion (M1.1); no text copied |
| `sola-2017-quaternion-kinematics` | J. Solà, Quaternion kinematics for the error-state Kalman filter, arXiv:1711.02508v1, 2017 | CC BY-NC-SA 4.0 | fetched | cited for the attitude kinematics (M1.1); no text copied |
| `mil-f-8785c` | MIL-F-8785C, Military Specification: Flying Qualities of Piloted Airplanes, 5 November 1980 | US government work | fetched | cited for the Dryden spectra, turbulence parameters and log wind law (M1.2); no text copied |
| `nasa-tm-2008-215633` | D. L. Johnson (ed.), Terrestrial Environment (Climatic) Criteria Guidelines for Use in Aerospace Vehicle Development, 2008 Revision, NASA/TM-2008-215633 | US government work | fetched | cited for the power-law wind profile and roughness lengths (M1.2); no text copied |
| `wmo-no8-vol1-2023` | WMO-No. 8, Guide to Instruments and Methods of Observation, Volume I: Measurement of Meteorological Variables, 2023 edition | WMO copyright; short extracts with full citation | fetched | cited for saturation vapour pressure, virtual temperature and geopotential height (M1.2); formulas cited, not redistributed |
| `picard-2008-cipm-2007` | A. Picard, R. S. Davis, M. Gläser and K. Fujii, Revised formula for the density of moist air (CIPM-2007), Metrologia 45 (2008) 149-155 | BIPM and IOP Publishing copyright | fetched | its equation is evaluated by `validation/oracles/atmosphere/moist_air.py` to check humid-air density (M1.2); not redistributed |
| `iapws-r12-08` | IAPWS R12-08, Release on the IAPWS Formulation 2008 for the Viscosity of Ordinary Water Substance | IAPWS: publication allowed with attribution | fetched | its dilute-gas viscosity (eq. 11) is evaluated by `validation/oracles/atmosphere/moist_air.py` to size humidity's effect on viscosity (M1.2) |
| `rocksim-rse-spec` | RockSim Engine File Format (.rse) specification, as hosted by ThrustCurve.org | unknown terms | fetched | read to write the `.rse` reader (M1.3); not redistributed |
| `nasa-sp-8039` | NASA SP-8039, Solid Rocket Motor Performance Analysis and Prediction, 1971 | US government work | fetched | cited for the thrust equation and effective exhaust velocity (M1.3); no text copied |
| `nar-standard-motor-codes` | National Association of Rocketry, Standard Motor Codes (nar.org/NARmotors.html, archived 2014-02-05) | unclear terms | fetched | cited for the impulse-class limits (M1.3); not redistributed |
| `thrustcurve3-analyze` | ThrustCurve.org site source, simulate/analyze/analyze.js at commit 577afa6 | ISC | fetched | read to confirm ThrustCurve's burn-time, impulse and average-thrust definitions, and run unchanged by `validation/oracles/thrustcurve/analyze_stats.js` for a committed fixture of its results (M1.3); no code ported |
| `thrustcurve-metadata` | ThrustCurve.org API v1 `metadata.json` | unstated terms | fetched | attribution to ThrustCurve.org wherever the data is used; each curve file has its own data license |
| `thrustcurve-motors` | ThrustCurve.org API v1 `search.json` (all motors) | unstated terms | fetched | attribution to ThrustCurve.org wherever the data is used; each curve file has its own data license |
| `thrustcurve-rasp-format` | ThrustCurve.org "RASP File Format" page | unstated terms | fetched | cited for the `.eng` format (M1.3, `docs/format/eng.md`) |
| `thrustcurve-glossary` | ThrustCurve.org glossary page | unstated terms | fetched | cited for burn time, average thrust, loaded weight and delays (M1.3) |
| `thrustcurve-motorstats` | ThrustCurve.org "Motor Statistics" page | unstated terms | fetched | cited for the NFPA 1125 burn-time normalization (M1.3) |
| `thrustcurve-contribute` | ThrustCurve.org "Contribute" page | unstated terms | fetched | cited for the meaning of the curve data licenses (M1.3) |
| `thrustcurve-simulators` | ThrustCurve.org "Flight Simulators" page | unstated terms | fetched | cited for RockSim's motor type and CG columns (M1.3, `docs/format/rse.md`) |
| `motor-finder-meta` | motor.fusionspace.co API v1 `meta.json` | free to use, attribution appreciated | fetched | attribution to motor.fusionspace.co |
| `motor-finder-motors` | motor.fusionspace.co API v1 `motors.json` | free to use, attribution appreciated | fetched | attribution to motor.fusionspace.co |
| `motor-finder-in-stock` | motor.fusionspace.co API v1 `in-stock.json` | free to use, attribution appreciated | fetched | attribution to motor.fusionspace.co |
| `motor-finder-vendors` | motor.fusionspace.co API v1 `vendors.json` | free to use, attribution appreciated | fetched | attribution to motor.fusionspace.co |

## Oracle environment (`validation/oracles/uv.lock`)

`cargo xtask refs fetch` installs these into the gitignored `refs/venv` with `uv sync --frozen`.
Nothing here is bundled.

| package | license | mode | notes |
|---|---|---|---|
| `rocketpy` 1.13.0 | MIT | run-only | the primary code-to-code oracle |
| `orhelper` 0.1.5 (`openrocket/orhelper` at commit `fb132c49e661`) | GPL-2.0 | run-only | drives the OpenRocket jar; its source is never read |
| `JPype1` 1.7.1 | Apache-2.0 | run-only | starts the JVM for the OpenRocket oracle |
| `mpmath` 1.3.0 | BSD-3-Clause | run-only | arbitrary-precision reference values from published formulas (`validation/oracles/wgs84/`, `validation/oracles/ussa76/`, `validation/oracles/atmosphere/`) |
| `ambiance` 1.3.1 | Apache-2.0 | run-only | an independent 1976 standard atmosphere, cross-checking the transcribed tables (`validation/oracles/ussa76/`) |
| the dependencies `uv.lock` pins (numpy, scipy, matplotlib, netCDF4 and others) | as each package states | run-only | installed only as the oracles' runtime |
| a Java 17+ runtime (for example OpenJDK from Homebrew) | GPL-2.0 WITH Classpath-exception-2.0 | run-only | installed by the user, not fetched; `refs doctor` finds it |

## Planned sources (not fetched yet)

| source | license | mode | notes |
|---|---|---|---|
| RocketSerializer | MIT | run-only | cross-checks the `.ork` importer (M3.1) |
| `openrocket/motor-database` | GPL-3.0 | run-only reference | not bundled |
| ThrustCurve.org thrust-curve files | per file: public domain, free, other, or none | fetched and cached (M5) | 32 public-domain curves are bundled (M1.3, above); the rest are fetched and cached, never bundled |
| Open-Meteo | data CC BY 4.0 | fetched and cached (M5.2) | attribution required |
| WMM2025 | public domain | may be bundled (M5.3) | |
