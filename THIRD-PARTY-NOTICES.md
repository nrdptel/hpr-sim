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
  motor dimensions and aerodynamic-surface dimensions of seven example rockets, taken from RocketPy's
  notebooks and test code (never from its data files). They are recorded
  in `validation/fixtures/design/rocketpy-rocket-mass.json` with RocketPy's outputs (and, for six
  of them, the rail length, inclination and heading in
  `validation/fixtures/flight/rocketpy-whole-flight.json`, whose drag is declared by the generator
  rather than taken from RocketPy's exports), and turned into
  `validation/designs/rocketpy-*.json` by `cargo xtask designs`. RocketPy's motor thrust files carry
  their own terms and are **not** committed; the fixture pairs each example with a bundled
  public-domain curve instead (ADR-007). RocketPy's license, quoted under Ported, applies.
- **Whole flights on RocketPy's own drag** (ADR-023): `flight.py --own-drag` flies the Calisto,
  Valetudo and Juno III curves from the `refs/rocketpy` checkout and commits only the flights'
  results, each curve's path and sha256, to
  `validation/fixtures/flight/rocketpy-whole-flight-own-drag.json`; the curves are **not**
  committed. Prometheus 2022's drag function is ported under Ported, below.
- **Comparisons with RocketPy's drag curves** (ADR-009): `cargo xtask aero` reads the Calisto,
  Juno III, Cavour and Valetudo curves from the `refs/rocketpy` checkout and commits only derived
  numbers to `validation/fixtures/aero/rocketpy-drag-curves.json` (each curve's value at Mach 0.3,
  hpr's drag coefficients, the relative errors and each file's sha256; from Mach 0.1 to 2.0, hpr's
  values and the errors, which together give each curve's value back at the sampled Mach numbers,
  147 in all, ADR-029). The curve files themselves carry
  their own terms and are **not** committed. The designs' drag inputs cite RASAero II's Users
  Manual (2019, p. 53) for its default surface finish, and `hpr_aero`'s reader of RASAero II's
  export cites it for the export's units and datum (pp. 13, 72, 76, 114); Projeto Jupiter's
  rocket page (projetojupiter.com/foguetes) gives Juno III's fin profile. Neither is
  redistributed.
- **GeoJSON schema** (MIT, `geojson/schema` at commit 268ba0af, Copyright (c) 2018 Tim Schaub):
  `FeatureCollection.json` as published at https://geojson.org/schema/FeatureCollection.json,
  unchanged, in `crates/hpr-sim/tests/data/geojson-feature-collection.schema.json`. Tests only:
  exported GeoJSON is checked against it (ADR-079). Its license:

  > MIT License. Copyright (c) 2018 Tim Schaub. Permission is hereby granted, free of charge, to
  > any person obtaining a copy of this software and associated documentation files (the
  > "Software"), to deal in the Software without restriction, including without limitation the
  > rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the
  > Software, and to permit persons to whom the Software is furnished to do so, subject to the
  > following conditions: The above copyright notice and this permission notice shall be included
  > in all copies or substantial portions of the Software. THE SOFTWARE IS PROVIDED "AS IS",
  > WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
  > MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
  > AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN
  > ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
  > OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## Ported

- **RocketPy** (MIT), `rocketpy/motors/solid_motor.py` at v1.13.0: the BATES grain regression
  geometry and the grain-stack inertia, re-derived in closed form in `hpr_motor::grains`, and the
  constant-exhaust-velocity consumption in `hpr_motor::motor`. Its technical documentation
  `docs/technical/equations_of_motion.rst` and `equations_of_motion_v1.rst` at v1.13.0: the
  variable-mass rigid-body equations of motion in `hpr_sim::dynamics`. And
  `rocketpy/simulation/flight.py:2710-2790` at v1.13.0: the point-mass descent under a parachute's
  drag area, and its numeric deployment trigger (`rocketpy/rocket/parachute.py:354-364`), in
  `hpr_sim::recovery` and the descent branch of `hpr_sim::dynamics`. And
  `tests/fixtures/rockets/rocket_fixtures.py:354-380` at v1.13.0: Prometheus 2022's piecewise-linear
  drag, `prometheus_cd_at_ma`, as `PROMETHEUS_CD_POINTS` in `validation/oracles/rocketpy/flight.py`.
  And two upstream corrections to `rocketpy/simulation/flight.py` and `rocketpy/rocket/rocket.py`,
  applied to v1.13.0 by `validation/oracles/rocketpy/corrections.py` (ADR-026): the edited lines of
  pull request #1196 (open, head `927e771e`), quoted there as substitutions into RocketPy's own
  `u_dot_generalized`, and the nozzle gyration tensor of pull request #1188 (merged 2026-09-09).
  RocketPy's license applies
  to those portions:

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

- **fusionspace-loft** (MIT, the project owner's own), `fixtures/demo-*.ork` at `64f51ef1b3`: seven
  hand-made demonstration designs, copied unchanged into `validation/fixtures/ork/loft-demo/` as
  the public files `hpr_io::ork`'s snapshot test reads (M3.1d1, snapshots of public designs).
- **DOPRI5** (BSD-2-Clause), E. Hairer and G. Wanner's `dopri5.f`, version of 2004: the
  Dormand–Prince coefficients, the error norm, the PI step-size controller, the starting-step
  estimate and the dense-output formula, re-expressed in Rust in `hpr_sim::integrator`. Its license
  applies to those portions:

  > Copyright (c) 2004, UNIGE. Redistribution and use in source and binary forms, with or without
  > modification, are permitted provided that the following conditions are met: Redistributions of
  > source code must retain the above copyright notice, this list of conditions and the following
  > disclaimer. Redistributions in binary form must reproduce the above copyright notice, this list
  > of conditions and the following disclaimer in the documentation and/or other materials provided
  > with the distribution. THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
  > IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES
  > OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
  > REGENTS OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
  > CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
  > SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
  > THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR
  > OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
  > POSSIBILITY OF SUCH DAMAGE.

## Rust dependencies

`Cargo.lock` lists the full dependency graph. Direct third-party dependencies:

| crate | license | used by | why |
|---|---|---|---|
| `criterion` | Apache-2.0 OR MIT | `hpr-core` (benchmarks only) | statistics for `cargo bench` (`docs/perf.md`) |
| `flate2` | MIT OR Apache-2.0 | `hpr-io` | gzip and deflate, with the pure-Rust `miniz_oxide` backend so that `hpr-io` still builds for wasm32 and links no C: one of the three containers a `.ork` design arrives in, and the compression inside the other |
| `jsonschema` | MIT | `hpr-sim` (tests only) | checks exported GeoJSON against the published GeoJSON schema; no default features, so it fetches and reads nothing (ADR-079) |
| `glam` | MIT OR Apache-2.0 | `hpr-core` | `f64` vectors, quaternions and matrices (`ARCHITECTURE.md`) |
| `proptest` | MIT OR Apache-2.0 | `hpr-core`, `hpr-atmos`, `hpr-motor` (tests only) | property tests |
| `pulldown-cmark` | MIT | `xtask` | reads the documentation site's Markdown, with the parser mdBook itself uses, to check its links and labels (ADR-016) |
| `rand_core` | MIT OR Apache-2.0 | `hpr-core` (tests only) | the generator traits `rand_xoshiro` implements |
| `rand_xoshiro` | MIT OR Apache-2.0 | `hpr-core` (tests only) | an independent xoshiro256++ and SplitMix64 that `hpr_core::random` is checked against, bit for bit |
| `roxmltree` | MIT OR Apache-2.0 | `hpr-motor`, `hpr-io`; `hpr-sim` (tests only) | a strict, read-only XML 1.0 parser for `.rse` motor files and `.ork` designs |
| `serde_json` | MIT OR Apache-2.0 | `xtask`, `hpr-motor`, `hpr-sim` (JSON and GeoJSON exports); `hpr-core`, `hpr-atmos` (tests only) | reads `cargo metadata` output and the bundled motor catalog index; serde round-trip tests and JSON fixtures |
| `serde` | MIT OR Apache-2.0 | `xtask`, `hpr-core`, `hpr-atmos`, `hpr-motor` | derives the `validation/refs.lock.toml` types and the public data types |
| `thiserror` | MIT OR Apache-2.0 | `hpr-core`, `hpr-atmos`, `hpr-motor` | library error types |
| `sha2` | MIT OR Apache-2.0 | `xtask`; `hpr-motor` (tests only) | SHA-256 of fetched references and of the bundled motor curves |
| `toml` | MIT OR Apache-2.0 | `xtask` | reads `validation/refs.lock.toml` |
| `tempfile` | MIT OR Apache-2.0 | `xtask` (tests only) | temporary directories for the `refs` tests |
| `zip` | MIT | `hpr-io`, `xtask` | reads the zip archive a `.ork` design is packed in, and the example designs inside the OpenRocket jar. Read-only, with only the deflate method OpenRocket writes: the default features would pull bzip2, lzma, zstd and AES |

## Documentation site tools

The site is built from `docs/` by `cargo xtask site` into the gitignored `target/site` (ADR-016 in
`docs/DECISIONS.md`), with the workspace's rustdoc, the API reference, under `target/site/api`
(ADR-019). CI publishes it to GitHub Pages from `main`. Nothing below is a Rust dependency, and none
of it is committed.

| tool | license | mode | notes |
|---|---|---|---|
| mdBook 0.5.4 (`rust-lang/mdBook`) | MPL-2.0 | run-only | builds the site; installed by the user or by CI, never linked or ported |
| mdBook's theme: its HTML templates, CSS and JavaScript | MPL-2.0 | bundled in the built site | copied into every build unchanged, so publishing the site (M0.4d) distributes them under their own licence, with their source at `rust-lang/mdBook` |
| highlight.js 10.1.1, elasticlunr 0.9.5, mark.js 8.11.1, clipboard.js 2.0.4 | BSD-3-Clause; MIT; MIT; MIT | bundled in the built site | shipped by mdBook's theme, each with its licence header intact |
| Open Sans and Source Code Pro fonts | Apache-2.0; OFL-1.1 | bundled in the built site | shipped by mdBook's theme with their licence texts (`fonts/OPEN-SANS-LICENSE.txt`, `fonts/SOURCE-CODE-PRO-LICENSE.txt`) |
| rustdoc (part of the pinned Rust toolchain) | MIT OR Apache-2.0 | run-only | builds the API reference (ADR-019) |
| rustdoc's static files: its CSS and JavaScript, and normalize.css | MIT OR Apache-2.0; MIT | bundled in the built site | copied into `api/static.files/` by every `cargo doc`, with rustdoc's `COPYRIGHT` file naming each resource's terms and the licence texts beside it |
| Fira, Source Serif 4, Source Code Pro and Nanum Barun Gothic fonts | OFL-1.1 | bundled in the built site | shipped by rustdoc with their licence texts in `api/static.files/` |

## Reference library (`validation/refs.lock.toml`)

`cargo xtask refs fetch` downloads these into the gitignored `refs/`, pinned by commit or sha256;
none of them is committed. A test checks that every item in the lock file has a row here with the
same license and mode.

| name | source | license | mode | notes |
|---|---|---|---|---|
| `rocketpy` | RocketPy v1.13.0 (example rockets, flight data, RASAero Cd exports, acceptance tests) | MIT; data files carry their own terms | fetched | code is MIT and may be ported with attribution; flight data carries team permissions recorded in RocketPy's notebooks; the ERA5 weather files in `data/weather/` are Copernicus (C3S) data with attribution required; the NASADEM tile is NASA data |
| `openrocket-database` | `openrocket/openrocket-database` (`.orc` parts) | Apache-2.0 | fetched | may be bundled with notices in M5.5. `validation/fixtures/ork/openrocket-automatic-radius.json` records the four radii OpenRocket resolves in its `ork/parachutes.ork`, and nothing else from the file |
| `fusionspace-loft` | `nrdptel/fusionspace-loft` (the project owner's own) | MIT | fetched | ported with a note; its seven `fixtures/demo-*.ork` designs are committed as test data (below) |
| `fusionspace-debrief` | `nrdptel/fusionspace-debrief` (the project owner's own) | MIT | fetched | ported with a note; the flight-log format knowledge behind Phase 5 |
| `loft-fixtures` | `nrdptel/loft-fixtures` | private; third-party design files | fetched | never committed; only derived statistics are published |
| `debrief-fixtures` | `nrdptel/debrief-fixtures` | private; third-party flight logs | fetched | never committed; only derived statistics and anonymised case ids are published |
| `openrocket-jar` | OpenRocket 24.12 (needs Java 17; it refuses 21) | GPL-3.0 | run-only | second oracle; its source is never read. M3.1b4's probe (`validation/oracles/openrocket/automatic_radius.py`, through JPype) commits what the program *output* — the radii it resolved and wrote back for fifteen designs written by the probe, the body radii it resolved in the jar's 17 example designs (numbers only, no design content), and one refusal message — in `validation/fixtures/ork/openrocket-automatic-radius.json` (ADR-054). M3.1c2's probe (`validation/oracles/openrocket/events.py`) commits the words the program writes for each event value and the short labels it shows for them, two drag coefficients and three computed for streamers, and the heights at which one example's parachute opened in two runs, in `validation/fixtures/ork/openrocket-events.json` (ADR-056); M3.1c3's (`validation/oracles/openrocket/conditions.py`) commits the text it writes for a set of launch conditions and the values it holds for them, where one example lands from two rod directions and in two winds, the rod direction written when launching into the wind, and eight values of one stored row beside the same values held, in `validation/fixtures/ork/openrocket-conditions.json` (ADR-057) M2.2a's `validation/oracles/openrocket/mass.py` commits the structure mass, centre of mass and inertias, and the per-part breakdown, OpenRocket computes for Loft's six public designs it opens and for a probe tube (`validation/fixtures/ork/openrocket-mass-loft-demo.json`, ADR-060). M2.2b1's `validation/oracles/openrocket/conventions.py` commits the same numbers for 32 probe designs the script writes itself, and the names and densities of the four default materials the program gives a part that names none (`validation/fixtures/ork/openrocket-conventions.json`, ADR-061). M1.9b's `validation/oracles/openrocket/clusters.py` commits the places the program gives each tube of a cluster for 25 probe designs the script writes itself (its 14 patterns among them), with their mass, centre of mass and inertias (`validation/fixtures/ork/openrocket-clusters.json`, ADR-075). M2.2c1's `validation/oracles/openrocket/motors.py` commits what the program reads out of the 32 public-domain ThrustCurve.org curve files this repository already carries — each one's designation, common name, digest, envelope, masses, standard delays, point count and first and last time, and the total impulse, average and maximum thrust and burn times it computes — in `validation/fixtures/motor/openrocket-curve-stats.json` (ADR-066), and no motor data of the jar's own is committed. M2.2c2's `validation/oracles/openrocket/motor_database.py` does read the jar's own motor database, as the program loads it (curves from ThrustCurve.org, whose terms are unstated), and the program's reading of the curves the private designs embed; that record is written only to the gitignored `corpus-out/`, `cargo xtask ork` publishes counts from it, and nothing from it is committed (ADR-067). M2.2d1's `validation/oracles/openrocket/flights.py` commits what the program outputs when it flies every motor configuration of the jar's 17 example designs and Loft's public demos in calm air, and one example with its parachute disabled — its ten summary figures, a few values of each flight's time series (peaks, the rows either side of each event, and the stability margin with the centres of pressure and mass at rod clearance), the launch conditions used, and each configuration's name as the design gives it, usually its motor designations — in `validation/fixtures/ork/openrocket-flights.json` (ADR-068); no design content is committed. |
| `barrowman-1967-thesis` | J. S. Barrowman, The Practical Calculation of the Aerodynamic Characteristics of Slender Finned Vehicles, MS thesis, 1967 (NASA/TM-2001-209983) | US government work | fetched | cited; M1.8c reads the Basic Finner's dimensions (Fig. 5-6) and its plotted roll damping (Fig. 5-7) into `validation/fixtures/aero/basic-finner-roll-damping.json`, with figure and page |
| `barrowman-1966-cp-report` | J. S. and J. A. Barrowman, The Theoretical Prediction of the Center of Pressure, NARAM-8, 1966 | unknown terms | fetched | cited, not copied or redistributed |
| `barrowman-1966-naram8-nakka` | J. S. and J. A. Barrowman, The Theoretical Prediction of the Center of Pressure, NARAM-8, 1966 (complete scan with its Testbed II and Aerobee 350 examples), bound with J. S. Barrowman, Calculating the Center of Pressure of a Model Rocket, Centuri TIR-33, 1970 | unknown terms | fetched | cited; the worked examples' printed dimensions and results (facts) are committed in `validation/fixtures/aero/barrowman-worked-examples.json` with page numbers; no figures and no text beyond part names copied |
| `tir-33-centuri-cp` | J. S. Barrowman, Calculating the Center of Pressure of a Model Rocket, Centuri TIR-33, 1970 (standalone scan) | unknown terms | fetched | cited, not copied or redistributed |
| `niskanen-2009-thesis` | S. Niskanen, Development of an Open Source model rocket simulation software, MSc thesis, 2009 | CC BY-NC-ND 1.0 Finland | fetched | read for methods only; no text copied |
| `openrocket-techdoc-13.05` | S. Niskanen, OpenRocket technical documentation, v13.05, 2013 | CC BY-SA 3.0 | fetched | read for methods; M1.7b transcribes appendix C's streamer correlation, section 3.5's two tumbling coefficients and Table 3.4's eight fin efficiency factors as numbers, with their printed pages; no text copied |
| `us-std-atmosphere-1976` | U.S. Standard Atmosphere, 1976 (NOAA-S/T-76-1562, NASA-TM-X-74335) | US government work | fetched | cited, not copied |
| `nasa-tn-d-4013` | J. C. Ferris, Static stability investigation of a single-stage sounding rocket at Mach numbers from 0.60 to 1.20, NASA TN D-4013, 1967 | US government work | fetched | cited; M1.8a reads its model's dimensions and its plotted normal-force slopes and centres of pressure, and M1.8b1 its plotted axial force, into `validation/fixtures/aero/arcas-robin-wind-tunnel.json`, with figure and page |
| `nasa-tn-d-4014` | C. D. Babb and D. E. Fuller, Static stability investigation of a sounding-rocket vehicle at Mach numbers from 1.50 to 4.63, NASA TN D-4014, 1967 | US government work | fetched | cited; M1.8a reads its plotted normal-force slopes and centres of pressure, M1.8b1 its plotted axial and chamber axial force, and M1.8c its plotted roll effectiveness (Fig. 14), into `validation/fixtures/aero/arcas-robin-wind-tunnel.json`, with figure and page; M1.8e6 reads its fins-off normal force above +4° and pitching moment into `arcas-robin-high-alpha.json` and `arcas-robin-fins-off-moment.json` |
| `nasa-tr-r-100-stoney-1961` | W. E. Stoney, Collection of zero-lift drag data on bodies of revolution from free-flight investigations, NASA TR R-100, 1961 | US government work | fetched | cited; M1.8b1 reads Figure 12's nose pressure-drag curves (fineness 3) as points into `crates/hpr-aero/src/nose_drag.rs`, with panel and configuration; no text copied |
| `naca-tn-2114` | S. M. Harmon and I. Jeffreys, Theoretical lift and damping in roll of thin wings with arbitrary sweep and taper at supersonic speeds: supersonic leading and trailing edges, NACA TN 2114, 1950 | US government work | fetched | cited for linear theory's lift of tapered fins (M1.8a); no text copied |
| `rocketpy-calisto-rasaero-2018` | RocketPy data/calisto/CD Test.CSV at its first commit da91db9e (2018): a RASAero II export for Calisto | MIT; data files carry their own terms | fetched | never committed; `cargo xtask aero` commits hpr's values, its errors and the export's values at the compared Mach numbers only, as for the drag curves (ADR-009) |
| `galejs-wind-instability` | R. Galejs, Wind Instability: What Barrowman Left Out, Sentinel 39 (about 1999) | unknown terms | fetched | cited, not copied or redistributed |
| `mil-hdbk-762` | MIL-HDBK-762(MI), Design of Aerodynamically Stabilized Free Rockets, 1990 (Distribution A) | US government work | fetched | cited; M1.8b2 transcribes its sample drag calculation (Table 5-4, pp. 5-58 to 5-66) and the rocket's geometry (Fig. 5-155) into `validation/fixtures/aero/mil-hdbk-762-sample-drag.json`, with page; no text copied |
| `naca-tn-4197` | D. J. Martin, Summary of Flutter Experiences as a Guide to the Preliminary Design of Lifting Surfaces on Missiles, NACA TN 4197, 1958 | US government work | fetched | cited for the flutter criterion in `hpr_sim::flutter` (M1.10b), not copied |
| `naca-tn-2972-jack-1953` | J. R. Jack, Theoretical pressure distributions and wave drags for conical boattails, NACA TN 2972, 1953 | US government work | fetched | cited; M1.8b3 reads its Fig. 3 (second-order boattail wave drag) into `validation/fixtures/aero/measured-boattails.json`, with figure and page, to check MIL-HDBK-762's chart |
| `naca-tn-3527-syvertson-dennis-1956` | C. A. Syvertson and D. H. Dennis, A second-order shock-expansion method applicable to bodies of revolution near zero lift, NACA TN 3527, 1956 | US government work | fetched | cited; M1.8e1 implements its method in `hpr_aero::shock_expansion`, reads its Fig. 2 (cone normal-force slopes) into the code and its Tables I and II into `validation/fixtures/aero/tn3527-bodies.json`, with table and page |
| `nasa-sp-3007-sims-1964-cones-small-alpha` | J. L. Sims, Tables for supersonic flow around right circular cones at small angle of attack, NASA SP-3007, 1964 | US government work | fetched | cited; M1.8e5 transcribes Table 2 (cone normal-force slopes, p. 20) from Mach 1.5 to 3 and 2.5° to 12.5° into `xtask/src/aero_gap.rs`, which writes them to `validation/fixtures/aero/arcas-robin-gap.json`, to bound TN 3527's Fig. 2 below Mach 3 |
| `nasa-tr-r-474-jorgensen-1977` | L. H. Jorgensen, Prediction of static aerodynamic characteristics for slender bodies alone and with lifting surfaces to very high angles of attack, NASA TR R-474, 1977 | US government work | fetched | cited; M1.8e5 compares its viscous crossflow term (eq. 2.12, η from Fig. 4, `C_dn` from Figs. 1 and 2) with the Arcas Robin's measured curvature; M1.8e6 flies that term and reads its Figs. 1, 4 and 6 by hand into `hpr_aero::crossflow`, copied into `validation/oracles/rocketpy/wind_response.py` (a test checks they match) |
| `afatl-tr-77-8-butler-sears-pallas-1977-ogive-bluffness` | C. B. Butler, E. S. Sears and S. G. Pallas, Aerodynamic characteristics of 2-, 3-, and 4-caliber tangent-ogive cylinders with nose bluffness ratios of 0.00, 0.25, 0.50, and 0.75 at Mach numbers from 0.6 to 4.0, AFATL-TR-77-8 (DTIC AD-B031340), 1977 | US government work | fetched | cited; M1.8e5 quotes eight values of its Table 3 (a sharp and a blunted 4-caliber ogive, Mach 1.5 to 4) to size the Arcas Robin's blunt tip. Its AD-B number marks a limited release; the cover records the change to Distribution A, public release (USADTC letter, 4 Sep 1980) |
| `nswc-tr-81-156-mason-1981-aero-design-manual` | L. A. Mason, L. Devan, F. G. Moore and D. McMillan, Aerodynamic design manual for tactical weapons, NSWC TR 81-156 (DTIC AD-A109180), 1981 | US government work | fetched | cited; M1.8e5 quotes its statement on small nose bluntness (p. 112) |
| `washington-pettis-1968-rd-tm-68-5` | W. D. Washington and W. Pettis Jr., Boattail effects on static stability at small angles of attack, U.S. Army Missile Command report RD-TM-68-5 (DTIC AD-695658), 1968 | US government work | fetched | cited; M1.8e6 reads its Fig. 5 (the boattail normal-force correlation) and Fig. 6 (the boattail's centre of pressure) by hand into `hpr_aero::supersonic_boattail`. The cover is stamped approved for public release, distribution unlimited |
| `nasa-tn-d-4865-jackson-1968` | C. M. Jackson Jr., W. C. Sawyer and R. S. Smith, A method for determining surface pressures on blunt bodies of revolution at small angles of attack in supersonic flow, NASA TN D-4865, 1968 | US government work | fetched | cited; M1.8e7 implements its Newtonian cap and handover rule in `hpr_aero::blunt_tip` and reads its Fig. 8(a) (model 1's normal force and pitching moment) by pixel analysis into `validation/fixtures/aero/tn-d-4865-sphere-cone.json`, and M1.8e18 its Fig. 8(b) (model 2, a blunted cone with an 18.5° flare) the same way into `validation/fixtures/aero/tn-d-4865-flared-cone.json`; no text copied |
| `nasa-tn-d-1304-seiff-1962` | A. Seiff, Secondary flow fields embedded in hypersonic shock layers, NASA TN D-1304, 1962 | US government work | fetched | cited; M1.8e8 takes its embedded Newtonian flare slope (eq. 9, printed p. 12) as an upper bound on a lip's share in `xtask/src/aero_lip.rs`, and quotes its limits (pp. 4 and 13); no text copied |
| `naca-rm-e51f26-cortright-schroeder-1951` | E. M. Cortright Jr. and A. H. Schroeder, Investigation at Mach number 1.91 of side and base pressure distributions over conical boattails without and with jet flow issuing from base, NACA RM E51F26, 1951 | US government work | fetched | cited; M1.8b3 reads its measured boattail drag and base pressures (Figs. 24, 29, 30) into `validation/fixtures/aero/measured-boattails.json`, with figure and page |
| `naca-rm-l54c16-demoraes-nowitzky-1954` | C. A. de Moraes and A. M. Nowitzky, Experimental effects of propulsive jets and afterbody configurations on the zero-lift drag of bodies of revolution at a Mach number of 1.59, NACA RM L54C16, 1954 | US government work | fetched | cited; M1.8b3 reads its measured boattail drag and base pressures (Figs. 5 and 6) into `validation/fixtures/aero/measured-boattails.json`, with figure and page |
| `nasa-tn-d-6789-compton-1972` | W. B. Compton III, Jet effects on the drag of conical afterbodies at supersonic speeds, NASA TN D-6789, 1972 | US government work | fetched | cited; M1.8b3 reads its measured jet-off boattail drag (Fig. 12) into `validation/fixtures/aero/measured-boattails.json`, with figure and page |
| `naca-rm-e54b11-moskowitz-jack-1954` | B. Moskowitz and J. R. Jack, Aerodynamics of slender bodies at Mach number of 3.12 and Reynolds numbers from 2 × 10⁶ to 15 × 10⁶, V: Aerodynamic load distributions for a series of four boattailed bodies, NACA RM E54B11, 1954 | US government work | fetched | cited; M1.8b3 reads one measured boattail wave drag (Fig. 4) into `validation/fixtures/aero/measured-boattails.json`, with figure and page |
| `naca-rm-l57b21-cubbage-1957` | J. M. Cubbage Jr., Jet effects on the drag of conical afterbodies for Mach numbers of 0.6 to 1.28, NACA RM L57B21, 1957 | US government work | fetched | cited; M1.8b3 reads its measured jet-off boattail drag (Figs. 7 to 9) into `validation/fixtures/aero/measured-boattails.json`, with figure and page, and takes the separation angles from its text |
| `naca-tn-3819-love-1957` | E. S. Love, Base pressure at supersonic speeds on two-dimensional airfoils and on bodies of revolution with and without fins having turbulent boundary layers, NACA TN 3819, 1957 | US government work | fetched | cited; M1.8b3 reads Kurzweg's boattail base pressures from its Fig. 25(b) into `validation/fixtures/aero/measured-boattails.json`, with figure and page |
| `naca-report-1135` | Ames Research Staff, Equations, tables, and charts for compressible flow, NACA Report 1135, 1953 | US government work | fetched | cited for the Prandtl-Meyer function and isentropic pressure ratio in `hpr_aero::afterbody` (M1.8b3), and the pitot pressure and a wedge's largest deflection in `hpr_aero::blunt_tip` (M1.8e7); no text copied |
| `rasaero-ii-arcas-comparison-2022` | C. E. Rogers, RASAero II Comparisons with ARCAS Center of Pressure (CP) and Drag Coefficient (CD) Wind Tunnel Data, Rogers Aeroscience, 2022 (slides) | unknown terms | fetched | cited for its treatment of the Arcas Robin's base lip (M1.8b3); not redistributed |
| `fpl-gtr-190-wood-handbook` | Forest Products Laboratory, Wood Handbook: Wood as an Engineering Material, General Technical Report FPL-GTR-190, 2010 | US government work | fetched | cited for wood densities (M1.4a) and shear moduli (M1.10b) in `hpr_design::materials` |
| `knacke-1991-parachute-manual` | T. W. Knacke, Parachute Recovery Systems Design Manual, NWC TP 6575, 1991 (DTIC ADA247666) | unclear terms | fetched | contractor report with a DTIC public-release stamp but a restrictive title-page notice; cited, never redistributed |
| `carruthers-filippone-2005-streamer-drag` | J. Carruthers and A. Filippone, Aerodynamic Drag of Streamers and Flags, J. Aircraft 42(4), 2005 (author post-print) | unclear terms | fetched | the AIAA copy is paywalled; this is the authors' post-peer-review copy in the University of Manchester's repository, with no licence stated; its two drag correlations are cited and implemented, its text never copied |
| `kidwell-2001-streamer-duration` | C. Kidwell, Streamer Duration Optimization: Material and Length-to-Width Ratio, NAR R&D, NARAM-43, 2001 | unknown terms | fetched | NARHAMS library copy with no licence stated; its drop-test masses and descent rates are cited as measurements, its text never copied |
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
| `hairer-dopri5` | E. Hairer and G. Wanner, DOPRI5: explicit Runge-Kutta method of order (4)5 due to Dormand and Prince, with step size control and dense output (Fortran, version of 2004) | BSD-2-Clause | fetched | ported to `hpr_sim::integrator` (M1.6a); see Ported |
| `hairer-licence` | Licence of E. Hairer's ODE codes, Copyright (c) 2004, UNIGE | BSD-2-Clause | fetched | the terms of `hairer-dopri5` (M1.6a) |
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
| `JPype1` 1.7.1 | Apache-2.0 | run-only | starts the JVM for the OpenRocket oracle |
| `mpmath` 1.3.0 | BSD-3-Clause | run-only | arbitrary-precision reference values from published formulas (`validation/oracles/wgs84/`, `validation/oracles/ussa76/`, `validation/oracles/atmosphere/`) |
| `ambiance` 1.3.1 | Apache-2.0 | run-only | an independent 1976 standard atmosphere, cross-checking the transcribed tables (`validation/oracles/ussa76/`) |
| the dependencies `uv.lock` pins (numpy, scipy, matplotlib, netCDF4 and others) | as each package states | run-only | installed only as the oracles' runtime |
| a Java 17 runtime (for example `brew install openjdk@17`) | GPL-2.0 WITH Classpath-exception-2.0 | run-only | installed by the user, not fetched; `refs doctor` finds it |

## RocketSerializer's environment (`validation/oracles/rocketserializer/requirements.txt`)

`validation/oracles/rocketserializer/geometry.py` runs in an environment of its own, `refs/venv-rs`,
installed with `--no-deps` from that file (M3.1d2, ADR-059). Nothing in it is committed or bundled.

| package | license | mode | notes |
|---|---|---|---|
| `rocketserializer` at `66d8ca8` (after release 0.2.0) | MIT | run, source read | a second reader of `.ork` files: its extractors are called one by one on each design, and `cargo xtask ork` holds hpr's key geometry to theirs. Its declared dependency `orhelper` (GPL-2.0) is not installed |
| `JPype1` 1.7.1 | Apache-2.0 | run-only | starts the JVM for OpenRocket, as in the oracle environment |
| `numpy`, `beautifulsoup4`, `soupsieve`, `typing-extensions`, `lxml`, `packaging` | as each package states | run-only | what the extractors and JPype import |

## Planned sources (not fetched yet)

| source | license | mode | notes |
|---|---|---|---|
| `openrocket/motor-database` | GPL-3.0 | run-only reference | not bundled |
| ThrustCurve.org thrust-curve files | per file: public domain, free, other, or none | fetched and cached (M5) | 32 public-domain curves are bundled (M1.3, above); the rest are fetched and cached, never bundled |
| Open-Meteo | data CC BY 4.0 | fetched and cached (M5.2) | attribution required |
| WMM2025 | public domain | may be bundled (M5.3) | |
