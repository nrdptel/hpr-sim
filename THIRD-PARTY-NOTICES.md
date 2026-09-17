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

None yet.

## Rust dependencies

`Cargo.lock` lists the full dependency graph. Direct third-party dependencies:

| crate | license | used by | why |
|---|---|---|---|
| `serde_json` | MIT OR Apache-2.0 | `xtask` | reads `cargo metadata` output |
| `serde` | MIT OR Apache-2.0 | `xtask` | derives the `validation/refs.lock.toml` types |
| `sha2` | MIT OR Apache-2.0 | `xtask` | SHA-256 of fetched references |
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
| `niskanen-2009-thesis` | S. Niskanen, Development of an Open Source model rocket simulation software, MSc thesis, 2009 | CC BY-NC-ND 1.0 Finland | fetched | read for methods only; no text copied |
| `openrocket-techdoc-13.05` | S. Niskanen, OpenRocket technical documentation, v13.05, 2013 | CC BY-SA 3.0 | fetched | read for methods only; no text copied |
| `us-std-atmosphere-1976` | U.S. Standard Atmosphere, 1976 (NOAA-S/T-76-1562, NASA-TM-X-74335) | US government work | fetched | cited, not copied |
| `nasa-tn-d-4013` | J. C. Ferris, Static stability investigation of a single-stage sounding rocket at Mach numbers from 0.60 to 1.20, NASA TN D-4013, 1967 | US government work | fetched | cited, not copied |
| `nasa-tn-d-4014` | C. D. Babb and D. E. Fuller, Static stability investigation of a sounding-rocket vehicle at Mach numbers from 1.50 to 4.63, NASA TN D-4014, 1967 | US government work | fetched | cited, not copied |
| `galejs-wind-instability` | R. Galejs, Wind Instability: What Barrowman Left Out, Sentinel 39 (about 1999) | unknown terms | fetched | cited, not copied or redistributed |
| `mil-hdbk-762` | MIL-HDBK-762(MI), Design of Aerodynamically Stabilized Free Rockets, 1990 (Distribution A) | US government work | fetched | cited, not copied |
| `naca-tn-4197` | D. J. Martin, Summary of Flutter Experiences as a Guide to the Preliminary Design of Lifting Surfaces on Missiles, NACA TN 4197, 1958 | US government work | fetched | cited, not copied |
| `knacke-1991-parachute-manual` | T. W. Knacke, Parachute Recovery Systems Design Manual, NWC TP 6575, 1991 (DTIC ADA247666) | unclear terms | fetched | contractor report with a DTIC public-release stamp but a restrictive title-page notice; cited, never redistributed |
| `rocksim-rse-spec` | RockSim Engine File Format (.rse) specification, as hosted by ThrustCurve.org | unknown terms | fetched | read to write the `.rse` reader (M1.3); not redistributed |
| `thrustcurve-metadata` | ThrustCurve.org API v1 `metadata.json` | unstated terms | fetched | attribution to ThrustCurve.org wherever the data is used; each curve file has its own data license |
| `thrustcurve-motors` | ThrustCurve.org API v1 `search.json` (all motors) | unstated terms | fetched | attribution to ThrustCurve.org wherever the data is used; each curve file has its own data license |
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
| the dependencies `uv.lock` pins (numpy, scipy, matplotlib, netCDF4 and others) | as each package states | run-only | installed only as the oracles' runtime |
| a Java 17+ runtime (for example OpenJDK from Homebrew) | GPL-2.0 WITH Classpath-exception-2.0 | run-only | installed by the user, not fetched; `refs doctor` finds it |

## Planned sources (not fetched yet)

| source | license | mode | notes |
|---|---|---|---|
| RocketSerializer | MIT | run-only | cross-checks the `.ork` importer (M3.1) |
| `openrocket/motor-database` | GPL-3.0 | run-only reference | not bundled |
| ThrustCurve.org thrust-curve files | per file: public domain, none, or unknown | fetched and cached | only curves with clear terms are bundled (M1.3) |
| Open-Meteo | data CC BY 4.0 | fetched and cached (M5.2) | attribution required |
| WMM2025 | public domain | may be bundled (M5.3) | |
