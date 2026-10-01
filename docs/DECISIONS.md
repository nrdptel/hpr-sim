# Decisions (ADR log)

Each entry is short: context, decision, consequences, date. Number them sequentially and never
renumber. Supersede an entry by adding a new one that points back to it.

| id | title | status |
|---|---|---|
| ADR-000 | Kickoff decisions | accepted |
| ADR-001 | License and workspace layout | accepted |
| ADR-002 | The reference library: lock file, fetch, verify and doctor | accepted; the orhelper dependency and its doctor check superseded by ADR-035, and the Java floor replaced by a range (`max_major`) because OpenRocket 24.12 refuses 21 |
| ADR-003 | Frames, attitude, geodesy and the gravity model | accepted |
| ADR-004 | Atmosphere, wind, turbulence and the seeded generator | accepted |
| ADR-005 | Solid motors: statistics, consumption, grains, file models and the bundled catalog | accepted |
| ADR-006 | Component geometry and mass properties: frames, shapes, walls, fins and materials | accepted |
| ADR-007 | Design tree: stations, placement, automatic radii, overrides, motors and checks | accepted |
| ADR-008 | Subsonic normal force and centre of pressure | accepted |
| ADR-009 | Subsonic drag buildup, surface finishes and drag override tables | accepted; the held nose drag and the `M ≥ 1` refusal superseded by ADR-028 |
| ADR-010 | Time integration: Dormand–Prince with dense output, RK4, stop times and events | accepted |
| ADR-011 | Rigid-body flight: equations of motion, aerodynamic coupling, rail, phases and termination | accepted |
| ADR-012 | Recovery: drag areas, triggers, inflation and the descent phase | accepted |
| ADR-013 | Streamer and tumble drag | accepted |
| ADR-014 | Separation: bodies, their masses and their descents | accepted |
| ADR-015 | The validation harness: cases, references, tolerances and reports | accepted |
| ADR-016 | The documentation site: mdBook over `docs/`, and checks for links, labels and equations | accepted |
| ADR-017 | Model pages open with *In short*; Accuracy traces its numbers; the records stay files | accepted |
| ADR-018 | Examples run in CI against committed output; pages quote files, checked line for line | accepted |
| ADR-019 | Publishing the site and the API reference to GitHub Pages | accepted |
| ADR-020 | The reader test, and labels that lead to plain words | accepted |
| ADR-021 | Whole flights against RocketPy: what is compared, and the gaps it may declare | accepted; the `M ≥ 1` gap superseded by ADR-028 |
| ADR-022 | Validation in CI, and regenerating references only by hand | accepted |
| ADR-023 | Predicted mode: each code's own drag, reported against a target | accepted |
| ADR-024 | The time-series RMS: aligned at ignition, held to 3% of its trace's scale | accepted |
| ADR-025 | The calm-air cases, and Juno III's drifts left to the rail release | accepted; Juno III's drifts superseded by ADR-026 |
| ADR-026 | The path in wind: RocketPy's corrected equations, and hpr's body lift | accepted; Prometheus's drifts superseded by ADR-027 |
| ADR-027 | The normal force through Mach 1: supersonic linear theory, a transonic join, and the measured references | accepted |
| ADR-028 | Drag through Mach 1: Niskanen's appendix B, Stoney's curves, and the Arcas Robin's axial force | accepted |
| ADR-029 | Drag against RASAero II through Mach 2: the gap by band, MIL-HDBK-762's sample calculation, and the boattail's wave drag | accepted |
| ADR-030 | The afterbody faster than sound: a boattail's wave drag, the base behind it, and a lip in its wake | accepted |
| ADR-031 | Roll from canted fins and roll damping by Barrowman's strip theory | accepted |
| ADR-032 | Normal-force overrides from RASAero II: the static force replaced, hpr's damping kept | accepted |
| ADR-033 | The body faster than sound: Syvertson and Dennis's second-order shock-expansion method | accepted |
| ADR-034 | The body's supersonic normal force in flight: tabulated shock-expansion shares, joined linearly from Mach 1.2 | accepted |
| ADR-035 | Drop the orhelper dependency; how M2.2 drives OpenRocket is decided when M2.2 starts | accepted |
| ADR-036 | The Arcas Robin's supersonic body gap: judged as the tunnel measures; M1.8e6 takes crossflow's size and the boattail | accepted |
| ADR-037 | Body lift by Jorgensen's crossflow at every speed, and a boattail's measured share faster than sound | accepted |
| ADR-038 | Blunt and vertical nose tips faster than sound by a Newtonian cap, the method started from the tangent cone | accepted |
| ADR-039 | A lip in a boattail's wake carries nothing faster than sound | accepted |
| ADR-040 | A steep boattail reads its measured correlation no steeper than 16°, and M1.8e's 15% target judged | accepted |
| ADR-041 | A lip's shelter is weighed as the drag buildup weighs it, not switched at a threshold | accepted |
| ADR-042 | Cone slopes from 24° to 30° come from Sims's tables, where TN 3527's chart stops | accepted |
| ADR-043 | The blunt tip's handover cap: what it is worth, and what stops it moving | accepted |
| ADR-044 | What the answer follows when it follows the mesh is a crossing of the tangent cone, not a reduced element | accepted |
| ADR-045 | Where a flare's march stops is the corner's isentropic turn, not the shock detaching | accepted |
| ADR-046 | Debrief folded in, and flight-log analysis that stands without the simulator | accepted |
| ADR-047 | A flare flies the method where its corner's shock is attached, and is read drawn out where it is not | accepted |
| ADR-048 | What a marched flare is worth, measured against TN D-4865's model 2 | accepted |
| ADR-049 | What a step in radius costs, and why the obvious fix is not taken yet | accepted |
| ADR-050 | A reduced element is read by the generalized method wherever it has a tangent cone of its own | accepted |
| ADR-051 | M3.1 split, and the `.ork` document kept whole rather than interpreted | accepted |
| ADR-052 | What a `.ork` value means: automatic dimensions, two names for one tag, and overrides | accepted; item 5's warning superseded by ADR-095 |
| ADR-053 | The parts on and inside a `.ork` body: degrees, what is left out, and a sourced finish | accepted |
| ADR-054 | An automatic radius with nothing to take is OpenRocket's default, and a rocket with no stage or component holds no design | accepted |
| ADR-055 | M3.1c split, and the motors a `.ork` flies: its own curve first, and only what lights at launch | accepted |
| ADR-056 | A `.ork` design's recovery and separation, read as written, with OpenRocket's words measured | accepted |
| ADR-057 | A `.ork` design's stored simulations, read back as written, with their units measured | accepted |
| ADR-058 | What a `.ork` holds that hpr does not model, kept whole in `x-openrocket` | accepted |
| ADR-059 | The RocketSerializer cross-check: three readers, with OpenRocket settling a difference | accepted |
| ADR-060 | M2.2 split, and the structure's mass held to OpenRocket's | accepted |
| ADR-061 | What a `.ork` leaves unsaid, read as OpenRocket reads it; overrides measured, two departures kept | accepted |
| ADR-062 | Fins and rail buttons against OpenRocket; roll inertia explained | accepted |
| ADR-063 | Packed parts read and weighed as OpenRocket packs them | accepted |
| ADR-064 | Clusters, fillets and unread parts remain visible departures | accepted; fillets superseded by ADR-096 |
| ADR-065 | Stored results are references only when current and structurally plausible | accepted |
| ADR-066 | Every curve hpr flies is integrated as OpenRocket integrates it | accepted |
| ADR-067 | Curves come from OpenRocket's own database by digest, each held to its impulse | accepted |
| ADR-068 | OpenRocket's flights of the public designs, and what its metric words mean | accepted |
| ADR-069 | hpr's flights of the public designs against OpenRocket's | accepted |
| ADR-070 | M2.2e split: mass and centre of mass first, then the corpus | accepted |
| ADR-071 | The corpus OpenRocket flies is its `.ork` files | accepted |
| ADR-072 | hpr's flights of the private library, under anonymised ids | accepted |
| ADR-073 | Each named cause sized by OpenRocket's own flight without it | accepted; a cause in the drag sized by ADR-097 |
| ADR-074 | Ignition times and powered staging: the sustainer flies on as a rigid body | accepted |
| ADR-075 | A cluster is one tube repeated, and a motor in it one motor per tube | accepted |
| ADR-076 | A `.ork` file's ignitions and one powered separation flown against OpenRocket | accepted; its refusal of a motor that never lights, and its reading of one private record, superseded by ADR-100 |
| ADR-077 | Flight metrics: peaks on the dense output, margins only where they mean something, and `None` for what didn't happen | accepted |
| ADR-078 | Fin flutter by NACA TN 4197: the lower reading wherever the source leaves room | accepted |
| ADR-079 | Exports as text built in the core, heights on each format's own datum | accepted |
| ADR-080 | Parquet written in-house, read back by Apache's library | accepted |
| ADR-081 | ERA5 weather read from netCDF classic in `hpr-io`; M2.3 split a to c | accepted |
| ADR-082 | Real flights read from refs, compared over the ascent, with checked explanations | accepted |
| ADR-083 | M2.3c blocked: no private design is the rocket of a logged flight | accepted |
| ADR-084 | The accuracy census: the reports' numbers held to the ones accepted | accepted |
| ADR-085 | Ejected pieces: an airframe that parts at any joint | accepted |
| ADR-086 | Ejection impulse and tumbling pieces | accepted |
| ADR-087 | Mass that moves along the airframe | accepted |
| ADR-088 | Mass released in flight | accepted |
| ADR-089 | A pod is a stack of body components repeated around the axis | accepted |
| ADR-090 | `.ork` pods placed as OpenRocket places them; pods of no length left out | accepted; pods of no length and the empty pod set left out superseded by ADR-091 |
| ADR-091 | A pod of no length weighs nothing and holds its parts on its axis | accepted |
| ADR-092 | A pod's parts are Barrowman's, once per pod, on the axis | accepted |
| ADR-093 | Pod probes flown as the public designs are, and listed apart | accepted |
| ADR-094 | A tilted launch rod flown as OpenRocket records it | accepted |
| ADR-095 | The single pre-1.9 override flag read as OpenRocket reads it | accepted |
| ADR-096 | Fin fillets and an automatic radius inside a nose cone read as OpenRocket reads them | accepted |
| ADR-097 | A cause in the drag sized by hpr flying OpenRocket's drag | accepted |
| ADR-098 | A tube fin set's automatic radius read as OpenRocket reads it | accepted |
| ADR-099 | Tube fins flown as ring wings | accepted |
| ADR-100 | A motor whose ignition never comes flown unlit, as OpenRocket flies it | accepted |
| ADR-101 | OpenRocket's mass conventions rolled up; M2.2 left open for two lessons | accepted |
| ADR-102 | Tube fins' centre of pressure measured against OpenRocket; L19's bar not met, the gap pinned | accepted |
| ADR-103 | The builder API wraps the crates' own types, with no default materials | accepted |
| ADR-104 | A drag model replaces the zero-lift drag only, as a drag table does | accepted |
| ADR-105 | The command line's surface: every command registered, JSON by schema, a generated table | accepted |
| ADR-106 | `hpr sim` flies the library's flight, the stack whole, from a stated launch | accepted |
| ADR-107 | `hpr validate` shares the project's own validation check; `hpr convert` translates `.eng` and `.rse` by the format notes | accepted |
| ADR-108 | A flight log read alone: PerfectFlite's `.pf2` first, heights after a running median, an invented log in CI | accepted |
| ADR-109 | M3.2 split, and a `.ork` written from the design | accepted |
| ADR-110 | M3.2b: OpenRocket flies the export; only counts are published | accepted |
| ADR-111 | M3.3: the hpr design format, its extensions, versions and crate | accepted |
| ADR-112 | M3.3b: the `.hprz` container, the first migration (0.2), and the design format on the command line | accepted |
| ADR-113 | M3.3c: TypeScript and Python types, and a reader for each, generated from the schema by xtask | accepted |
| ADR-114 | M4.3 split a to c; the Python package wraps the builder, SI names, flies on construction, one abi3 wheel per OS | accepted |
| ADR-115 | M4.3b: a drag table on the builder, gravity and a parachute release in Python; Calisto measured in the example, not in the library | accepted |
| ADR-116 | M4.3c: Python drag `f(mach, thrusting)` and wind `f(height_m)`; a function's exception kept and raised in place of the library's error | accepted |
| ADR-117 | M5.1 split a and b; M5.1a: the cache, TTL freshness and an offline mode that never calls the transport | accepted |
| ADR-118 | M5.1b: `ureq` 3 over rustls behind feature `http`, the body limit on unpacked bytes, the platform cache folder by hand | accepted |
| ADR-119 | M5.2 split a to d; M5.2a: Open-Meteo's pressure levels as a sounding, the ground as its lowest level, levels below it dropped, heights geopotential | accepted |
| ADR-120 | M5.2b: University of Wyoming soundings from its CSV, FM 35 by default or BUFR, the first row as the ground, the middle of each same-pressure run, rows off the longest chain fitting the hypsometric thickness from the row before, or not above the last kept, dropped; freshness by age; U.S. stations' soundings committed as fixtures | accepted |
| ADR-121 | M5.2c: GFS and RAP from NOMADS' grib filter as a sounding, bilinear at the site, RAP's winds turned from its grid; an in-house GRIB2 decoder (3.0, 3.30 tangent, 4.0, 5.0) in `hpr-io` in place of the `grib` crate, checked value for value against ecCodes | accepted |
| ADR-122 | M5.2d split d1 to d3; M5.2d1: `hpr weather`, one subcommand per source, fetched through `hpr_net`'s cache, `--offline` from the cache alone, `--from` a saved answer held to a fetch's checks; the profile written as `SoundingProfile`'s JSON | accepted |
| ADR-123 | M5.2d2: complex packing (5.2, 5.3) and product template 4.8 in `hpr_io::grib2`; a whole GFS file's 746,770,303 values against ecCodes by hand, eight of its messages committed with ecCodes' sums and samples, the cut repacked by ecCodes for CI; NOMADS parsing keys layers by both surfaces, skips statistics and wraps a global grid | accepted |
| ADR-124 | M5.2d3: JPEG 2000 (5.40) in `hpr_io::grib2` through `hayro-jpeg2000` in strict mode, lossless, to 21 bits, NCEP's coding only, the rest refused by name; four public RAP messages against ecCodes in CI | accepted |
| ADR-125 | M5.3 split a to c; WMM2025 in `hpr_core::magnetic`, finite at the poles, refused outside 2025 to 2030 and −1 to 850 km; NOAA's 112 test values, NCEI's file's `X` to its measured 7.18e-4 nT residue, shown to lie in the file's `X′` | accepted |
| ADR-126 | M5.3b: Open-Meteo's elevation in `hpr_net::elevation`, up to 100 places a request, coordinates to 5 decimals in the URL, a year's TTL, heights refused outside −1,000 to 9,000 m, a surface model above the EGM2008 geoid with `N` left to the caller; no command yet | accepted |
| ADR-127 | M5.3c split c1, c2; geodesics in `hpr_core::geodesic` through `geographiclib-rs`, flattening past 1/150 refused; Karney's 500,000-line test set within his 15 nm on five measures, either azimuth pair on its 21 mirror lines | accepted |
| ADR-128 | M5.3c2: a user's GeoTIFF in `hpr_io::geotiff` over the `tiff` crate; geographic CRSs within a few metres of WGS 84 only, projections and far datums refused; the containing pixel placed as GDAL places it; GDAL's scale and offset, units the file states, else metres; held to rasterio 1.5.2 on seven fixtures and a whole USGS tile | accepted |
| ADR-129 | M5.4 split a to c; M5.4a: motor.fusionspace.co's five files in `hpr_net::motor_finder` through the cache, an hour's TTL, its structural rules refused and its derived ones pinned on the recording; eight answers of one build committed as fixtures; the credit with the site's caution on every answer | accepted |
| ADR-130 | M5.4b: ThrustCurve.org's search and download in `hpr_net::thrustcurve` through the cache, a day's TTL; the join by exact maker and designation, misses reported not guessed; three makers' searches and two public-domain files committed as fixtures; the join's report in `validation/reports/thrustcurve-join.md` | accepted |
| ADR-131 | M5.4c: `hpr motors search`, the finder's list from the network, the cache or a saved file; five filters, `--max-price` in exact cents on the cheapest in-stock offer; cheapest first; both credits on every list; the example tested on an edited copy, as the recording lists nothing at $150 | accepted |
| ADR-132 | M5.5 split a and b; M5.5a: `hpr_io::orc` reads OpenRocket's `.orc` parts catalogues; the 16 files OpenRocket 24.12 ships bundled unchanged (Apache-2.0); held part by part to OpenRocket's preset loader, run as an oracle; exact unit definitions, the file's makers' names and densities kept, a stated mass kept beside them; unreadable parts left out with a warning, not the file | accepted |
| ADR-133 | M5.5b: catalogue parts in the builder (`from_catalog` on `Nose`, `Tube`, `Transition`, `MotorTube`, and the new `Fitting`); what the file leaves unsaid as OpenRocket 24.12 builds it, but a hollow part's shoulder takes its wall; a stated mass scales the part's density; a part with an undefined material refused; every part held to OpenRocket's built mass and centre, the two codes' hollow walls each checked | accepted |
| ADR-134 | M6.1a: Monte Carlo dispersion in `hpr_analysis::montecarlo`; each input an independent normal about its nominal value; one random stream per sample, input and copy, keyed by `SeededRng::for_stream`; impulse dispersed with the propellant mass; a drag scale in the aero model; failed samples kept; M6.1 split a to d | accepted |
| ADR-135 | M6.1b: landing ellipses in `hpr_analysis::ellipse`; the normal ellipse of the sample's mean and covariance, scaled by `k² = −2 ln(1 − p)`; a prediction ellipse for the next flight by Hotelling's `T²`; the landings each ellipse really holds counted, failures as bounds; tested against normal spreads with known answers | accepted |
| ADR-136 | M6.1c: sensitivity analysis in `hpr_analysis::sensitivity`; uniform independent factors; Morris's paths with `μ`, `μ*`, `σ` and `μ*`'s standard error, and the exact grid moments by `Morris::population`; Saltelli 2010's first-order and Jansen's total estimates on pseudo-random rows, with delta-method standard errors; held to Ishigami and Sobol's g in closed form and calibrated over seeds | accepted |
| ADR-137 | M6.1d: a Monte Carlo run's flights share the nominal design's supersonic table (`AeroModel::share_supersonic_table`, when the covered segments and reference area are equal) and its layout (`Rocket::lay_out`, `LaidOut::relay`, `Simulation::from_laid_out`); evaluations borrowed, not copied; 10,000 flights timed on Valetudo (the flight benchmarks' Level 2 rocket) and on a supersonic K940 rocket, by a bench target; per-evaluation speed-ups left to #285 | accepted |
| ADR-138 | M6.2 split a to e; M6.2a: CMA-ES (Hansen's tutorial, Table 1, positive weights only) over continuous variables, worked in each variable divided by its step, with optional bounds by resampling, ask-and-tell (`Run::tell`), Jacobi eigen-decomposition each generation; held to four test functions' minima, to pycma 4.5.0's evaluation counts (medians within 25%) and to a dense recomputation of every generation; the 3,048 m problem re-flown from a fresh build and at 100× tighter tolerances | accepted |
| ADR-139 | M6.2b split b1, b2; M6.2b1: constraints for CMA-ES by Deb's (2000) feasibility rules (`Evaluation`, `Run::tell_constrained`), no penalty weight; infeasible candidates ranked, not redrawn; a target and TolFun count only feasible points; held to the sphere with `x₀ ≥ 1`, the tangent problem and CEC 2006 g06 from 20 seeds | accepted |

---

## ADR-000: Kickoff decisions (2026-09-16)

**Context.** Neer asked for a Rust hobby/HPR rocket simulator: library first, validation-heavy,
offline-first, open source. It replaces his Loft project (TypeScript/Next.js), which he is shutting
down.

**Decision.**

- **Language and layout:** Rust, in a workspace of focused crates (see `ARCHITECTURE.md`).
- **Order of work:** physics core and validation first; interop, bindings and analysis next; UI
  last.
- **License:** `MIT OR Apache-2.0`. MIT matches the other Fusion Space projects; Apache-2.0 adds
  a patent grant and is the Rust norm. ADR-001 confirms this in M0.1.
- **Clean room:** no GPL/AGPL source is read or ported. GPL tools may be run as external oracles.
- **No AI traces** in commits, PRs, code or docs. Commits are authored as Neer's GitHub noreply
  identity.
- **Private data:** `loft-fixtures` content is never committed; only derived statistics are
  published.
- **Name:** `hpr-sim` for now.
- **Scope:** COTS solid motors only.
- **Hosting:** a public GitHub repo (`nrdptel/hpr-sim`). Work ships through PRs; CI covers macOS,
  Windows and Linux; merging on green is pre-authorized.
- **How work runs:** unattended cycles, one milestone each, handing off through `STATUS.md`
  (see `docs/AUTOPILOT.md`).

**Consequences.**

- Work is slower at first, but every number is defensible.
- A few formats (`.CDX1`, parts of `.ork` 1.12) must be learned from samples rather than from
  source code.
- Publishing to crates.io or PyPI waits for Neer.

---

## ADR-001: License and workspace layout (2026-09-17)

**Context.** M0.1 turns ADR-000's provisional license and the crate map in `ARCHITECTURE.md` into
files and checks: license texts, a dependency-license policy that backs the clean-room rule, and a
workspace whose pure core can be verified.

**Decision.**

- **License:** `MIT OR Apache-2.0`, confirmed. MIT matches the other Fusion Space projects,
  Apache-2.0 adds an explicit patent grant, and the pair is the Rust norm. `LICENSE-MIT`
  (copyright "Neer Patel and the hpr-sim contributors") and `LICENSE-APACHE` (the canonical
  apache.org text, sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`).
  Every crate inherits `license` from `[workspace.package]`; the README carries the usual
  dual-license contribution clause.
- **Dependency licenses (`deny.toml`):** only permissive licenses are allowed: 0BSD, Apache-2.0
  (also WITH LLVM-exception), BSD-2-Clause, BSD-3-Clause, BSL-1.0, CC0-1.0,
  CDLA-Permissive-2.0 (the data license of `webpki-roots`, which rustls needs), ISC, MIT, MIT-0,
  Unicode-3.0, Unlicense and Zlib. Everything else is rejected, weak copyleft (LGPL, MPL, EPL)
  included; widening the list, or a per-crate exception, needs a new ADR. Security and unsound
  advisories and yanked versions fail anywhere in the graph. Unmaintained notices fail only for
  direct dependencies: picking maintained crates is our call, and a notice deep in the graph must
  not block unrelated PRs in unattended runs. crates.io is the only allowed source. CI pins
  cargo-deny to the version `deny.toml` was tested with (0.20.2). Checked on 2026-09-17: a
  throwaway workspace with GPL-3.0, AGPL-3.0, LGPL-2.1 and MPL-2.0 crates fails
  `cargo deny check licenses` (exit 4), and its MIT crate passes.
- **Layout:** a virtual workspace (resolver 3, edition 2024) with `rust-version` equal to the
  toolchain pinned in `rust-toolchain.toml` (1.98.1); bump the two together. The 17 crates of the crate map live under
  `crates/`, and their internal dependency edges are declared now, so the layering holds from the
  start. `xtask` sits at the root (`xtask/`), where the cargo-xtask convention puts it, because it
  is tooling rather than product. Crates start at 0.1.0 with `publish = false` until Neer decides
  on publishing.
- **Pure core:** a crate joins it with `[package.metadata.hpr] wasm = true`. Members: `hpr-core`,
  `hpr-atmos`, `hpr-motor`, `hpr-design`, `hpr-aero`, `hpr-sim`, `hpr-analysis`,
  `hpr-flightdata`, `hpr-format`, `hpr-io`, the `hpr` facade with default features (`net` is an
  optional feature) and `hpr-wasm`. Outside it: `hpr-net` (network and cache I/O),
  `hpr-validate` (reads case files), `hpr-cli`, `hpr-py`, `hpr-ffi` and `xtask`.
  `cargo xtask wasm-check` enforces it in two steps. First, no workspace crate outside the core
  may appear in the core's normal dependency graph for any target, as resolved by `cargo tree`
  (so features one pure crate enables in another count). Second,
  `cargo clippy --target wasm32-unknown-unknown -- -D warnings` on the core. Compiling for wasm32
  does not rule out I/O (`std::fs` and `Instant::now` compile and then fail at run time), so
  `clippy.toml` also disallows the filesystem, network, clock, thread, process and environment
  APIs everywhere. Crates outside the core allow those two lints at the crate root. A unit test
  pins the membership list, so changing it is visible in review.
- **Lints:** `missing_docs`, `missing_debug_implementations` and `unsafe_code = "deny"` for rustc;
  `unwrap_used`, `expect_used`, `panic`, `print_stdout`, `print_stderr`,
  `allow_attributes_without_reason`, `dbg_macro`, `todo` and `unimplemented` for clippy.
  `clippy.toml` relaxes the panic and print lints in tests, and binaries allow printing at the
  crate root. CI fails on warnings in clippy (Linux host and wasm32), rustdoc, and the macOS and
  Windows test builds.
- **Line endings:** `.gitattributes` checks text out with LF everywhere, so formatting, snapshot
  tests and reference-data hashes behave the same on Windows.
- **CI:** one workflow on every PR and on pushes to `main`. fmt, clippy, doc, wasm-check and deny
  run on Linux; tests run on Linux, macOS and Windows. Swatinem/rust-cache caches builds;
  cargo-deny is installed as a prebuilt binary.

**Consequences.**

- The CLI binary is named `hpr`, like the facade library, so its rustdoc is off (`doc = false`) to
  avoid an output collision in `target/doc`.
- A binding crate that needs `unsafe` must allow it item by item, with a reason.
- A dependency under MPL or LGPL needs an ADR before it can be added. Known case: `directories`
  pulls in `option-ext` (MPL-2.0), so M5.1 needs either an ADR for a scoped exception or another
  platform-paths crate.
- Tests inside the pure core that read fixture files must use `include_str!`/`include_bytes!` or
  allow the lint on that test with a reason.

## ADR-002: The reference library: lock file, fetch, verify and doctor (2026-09-17)

**Context.** Validation needs other simulators, papers, motor data and private design files on
disk, reproducibly and never committed. M0.2 asks for `cargo xtask refs fetch|verify|doctor`
driven by `validation/refs.lock.toml`.

**Decision.**

- **One lock file, four kinds of item.**
  - `[[git]]`: pinned by full commit id, optionally shallow. Existing checkouts are reused and
    moved to the pin; checkouts with local changes are never touched.
  - `[[file]]`: an immutable download pinned by sha256 (the OpenRocket jar, 11 papers and the
    RockSim `.rse` spec).
  - `[[snapshot]]`: a live API (ThrustCurve, motor.fusionspace.co) pinned by the sha256 and date
    of one capture. APIs move (the motor finder hourly), so a missing snapshot that no longer
    matches fails. The new capture is kept beside it, with a record of its URL, its hash, its
    date and the pin it drifted from. `fetch --adopt-snapshots` moves that kept capture into place
    only if the entry still has the same URL and pin and the bytes are unchanged; otherwise it
    downloads a fresh capture and says why. It then rewrites only that entry's `sha256` and
    `captured` lines, using the capture's real date.
  - `[python]`: a `uv` project in `validation/oracles/` (`pyproject.toml` and `uv.lock`,
    committed), installed into `refs/venv` with `uv sync --locked`. That refuses a lock that is
    stale against `pyproject.toml` and checks every artifact against the hash in `uv.lock`;
    orhelper is pinned by git commit.

  Every destination must be a plain relative path inside the gitignored `refs/`, and
  destinations may not overlap.
- **Private repositories** (`private = true`, today `loft-fixtures`) are fetched with the
  user's git credentials, never prompting. A private repository that isn't checked out and
  doesn't answer `git ls-remote` (no access, as in CI) is skipped with a note. Any other failure,
  such as a bad pin or a checkout that can't move, fails. No partial checkout is left behind. A `sha256sum` manifest in a checkout can be pinned too;
  `verify` then hashes every listed file.
- **Idempotence:** anything already in its pinned state is left alone, so a second `fetch`
  downloads nothing. Downloads go to `<dest>.part` and are renamed into place only after the hash
  matches.
- **Verify re-hashes content**, not timestamps.
  - Files are hashed with SHA-256.
  - Git checkouts are compared, byte for byte (`core.autocrlf=false`), against a fresh temporary
    index read from the pinned commit. That makes git hash every tracked file, whereas a plain
    `git status` trusts cached stat data and can miss a same-size edit; a test shows both.
    Untracked files that the repository doesn't ignore also fail.
  - The environment must pass `uv sync --locked --check`, which compares package versions only.
    Then every installed file is re-hashed against the sha256 its package's `RECORD` lists
    (4,675 files today). Any file or symlinked directory that no `RECORD` lists fails, such as a
    `sitecustomize.py` or a `.pth` file, which would run at startup. The exception is the venv's
    own scaffolding (activation scripts, interpreter links, `pyvenv.cfg`, `_virtualenv.py`),
    which is allowed by name. Its content isn't pinned, because uv generates it, except
    `_virtualenv.pth`, whose one line is checked.
    The check runs Python with `-I`, so `PYTHONPATH` and the current directory can't leak in.
    `fetch` repairs hash failures with `uv sync --reinstall --no-cache`. Files no package owns
    can't be repaired that way, so `fetch` names them and asks for them to be deleted. uv installs by copying
    (`UV_LINK_MODE=copy`), so an edited venv file can't corrupt uv's cache through a hard link.
  - Git commands drop `GIT_DIR`-style variables, so a git hook can't redirect them at this
    repository.
- **External tools instead of crates:** `git`, `curl` and `uv` are run as commands. They exist on
  all three CI operating systems, and an HTTP and TLS stack in `xtask` would add many dependencies
  for no gain. curl is restricted to `https` (and `file` for tests), redirects included, and gives
  up on a transfer that stalls below 1 kB/s for a minute. New
  `xtask` dependencies: `serde`, `toml`, `sha2`, and `tempfile` for tests.
- **Doctor** lists the tools and their versions, a quick reference check (hashes of files,
  commits of checkouts), and whether each oracle is runnable. RocketPy: `rocketpy` imports at the
  locked version. OpenRocket: Java 17+ is found (`JAVA_HOME`, macOS `java_home`, Homebrew's
  keg-only `openjdk` formulae, then `PATH`), the jar verifies, `orhelper` and `jpype` import, and
  a smoke test starts the JVM through JPype and loads the jar's `Main-Class`. The smoke test sets
  `JAVA_HOME` from the runtime's own `java.home` property, which is reliable behind shims. The smoke test uses
  JPype (Apache-2.0) directly, so no project code calls into GPL orhelper. "Runnable" therefore
  means the runtime starts, not that a flight has been simulated; the M2.x oracle scripts do that.
- **Consistency tests:** every lock item needs a row in `THIRD-PARTY-NOTICES.md` with the same
  license and mode (and title, for papers), and every real URL uses `https`.
- **Source choices:** orhelper comes from `openrocket/orhelper` at a pinned commit, because PyPI's
  0.1.3 predates OpenRocket 24.12. The Knacke manual comes from archive.org's mirror of the DTIC
  copy, because DTIC refused automated downloads. It is a contractor report with a restrictive
  title-page notice, so it is labelled "unclear terms" and is never redistributed. RocketPy is a shallow clone of `v1.13.0`
  (about 390 MB, mostly ERA5 weather files that M2.3 needs).

**Consequences.**

- A fresh machine cannot reproduce a snapshot byte for byte once the API has moved; it has to
  adopt a new capture, and results that depend on it must name the capture date. Validation
  results that CI checks must come from committed fixtures, not from `refs/`.
- The `refs` tests need `git` and `curl` on the test machine. They use local `file://` sources
  and never touch the network.
- Java is not fetched: the user installs a JDK, and `doctor` says how.
- M2.2's oracle scripts will need a decision on importing GPL-2.0 orhelper from this repository,
  as opposed to driving the jar through JPype directly. This is recorded under "Needs Neer" in
  `STATUS.md`.

---

## ADR-003: Frames, attitude, geodesy and the gravity model (2026-09-17)

**Context.** M1.1 fixes the conventions that every later crate uses: the frames, the attitude
representation, the Earth model, and interpolation tables. Loft simulated a flat Earth with
constant 9.80665 m/s² gravity and never stated a frame. That left it 0.3% off RocketPy's gravity
at the equator (lesson L1), and its apogee datum and ground-hit frame were never defined (L35).
The comparisons in M2.1 need hpr's conventions to map exactly onto RocketPy's.

**Decision.**

- **Math types:** glam's `f64` types (`DVec3`, `DQuat`, `DMat3`) with the `f64`, `serde` and
  `std` features only. `hpr-core` re-exports them.
- **Frames** (`docs/physics/frames.md`):
  - **ECEF:** WGS 84.
  - **Launch frame `L`:** East-North-Up, origin at the pad, `z_L` along the ellipsoid normal.
    State propagates in `L`, which is Earth-fixed.
  - **Body frame `B`:** `+z_B` along the axis toward the nose, `x_B` along the design's zero
    radial direction.
  - **Attitude:** a unit Hamilton quaternion mapping body to launch components, with
    `q̇ = ½ q ⊗ (0, ω_B)` and renormalization after every step.
  - **Launch angles:** `q = R_z(−A) R_x(E − π/2) R_z(φ)`. This is RocketPy's 3-1-3 initial
    attitude with heading, inclination and rail-button angle, pinned by a RocketPy oracle
    fixture (`validation/oracles/rocketpy/attitude.py`).
  - **Rationale:** RocketPy is the main oracle, so matching its axes removes a class of sign
    errors from the comparisons. At the identity attitude the rocket stands vertical on the pad.
- **Heights:** every height in the core is ellipsoidal (`Geodetic::height_m`). Heights above sea
  level are converted at the I/O boundary. The flight engine reports altitude from the geodetic
  height of the position, not from `z_L`.
- **Geodetic conversion:** the forward conversion is NGA.STND.0036 eq. 4-14. The inverse is
  Karney's (2011, appendix B) version of Vermeille's closed form. It needs no iteration, is exact
  to rounding, and fails only within 43 km of the Earth's centre, where it returns an error.
  Bowring's iterative method was rejected: it needs an iteration count and a convergence
  tolerance, and its primary source is paywalled. Karney's paper is open.
- **Gravity:** WGS 84 normal gravity from its four defining parameters (NGA.STND.0036 ch. 4,
  app. B), with the exact ellipsoidal-harmonic closed form as the default.
  - The flight engine chooses from `constant`, `vertical_taylor` (RocketPy's), `vertical` and
    `ellipsoidal` (the default). Earth rotation contributes Coriolis only, by default. The
    centrifugal term is inside normal gravity.
  - `q` and `q′` use their `atan` series below `ε = 0.5` to avoid cancellation. That makes the
    derived constants round to Table 3.6.
  - **Rationale:** exact at every height a hobby rocket reaches, cheap enough for the inner loop
    (35.5 ns a call on the dev machine, `docs/perf.md`), and able to reproduce RocketPy's
    formula exactly when a comparison needs it.
- **Reference values:** no publication tabulates normal gravity by latitude and height.
  `validation/oracles/wgs84/normal_gravity.py` evaluates the published formulas at 40 digits with
  mpmath, which was added to the oracle environment. It checks the derived constants against the
  tables and the vector against the gradient of the normal potential. Its JSON output is committed
  under `validation/fixtures/earth/`, and the Rust tests read it with `include_str!`, so CI needs
  no Python.
- **Tables:** `Table1D` offers linear or natural-cubic interpolation, with a `clamp`, `linear` or
  `error` extrapolation policy. Every lookup reports whether it extrapolated. A monotone cubic
  waits until a model needs one (`docs/physics/interpolation.md`).
- **JSON floats:** `serde_json` has `float_roundtrip` enabled workspace-wide, so a parsed number
  is the exact `f64` that was written.

**Consequences.**

- M1.6 must:
  - use the geodetic height for ground contact and apogee;
  - decide whether its rotational dynamics include the frame rate `Ω` (at most 7.3e-5 rad/s);
  - call `Earth::gravity_enu_mps2` and `Earth::rotation_acceleration_enu_mps2` instead of a
    constant.
- M2.1 cases that compare with RocketPy select `vertical_taylor`. They must also account for
  RocketPy feeding height above sea level to its formula, and holding gravity constant above
  `max_expected_height` (80 km by default).
- M1.4 fixes the body reference point and the station mapping `z_B = z_ref − s`.

## ADR-004: Atmosphere, wind, turbulence and the seeded generator (2026-09-17)

**Context.** M1.2 builds `hpr-atmos`. Loft's atmosphere was wrong in five ways (lessons L2–L6):
geometric height fed to geopotential formulas, four layers, the wrong Sutherland constants, the
standard lapse used where a sounding existed, and a wind that stepped at the lowest forecast level.
RocketPy, the M2.1 oracle, interpolates everything linearly and ignores humidity. Several choices
had no single right answer and are recorded here.

**Decision.**

- **Height datum:** atmospheres and winds are queried with geometric height above mean sea
  level. The flight engine converts from ellipsoidal height with the site's geoid undulation.
  Every sample says whether the model extrapolated.
- **1976 standard:** implemented from its equations and constants: `R* = 8314.32`,
  `S = 110.4 K`, and `M/M₀` from 80 to 86 km, which the printed tables omit.
  - The committed fixture is transcribed from the scan and cross-checked by mpmath and
    `ambiance`.
  - Outside −5 to 86 km the lowest layer continues downward, and the air is isothermal above 86
    km. Both are flagged.
- **Temperature offsets:** `ΔT` is added at equal geopotential height, and pressure is
  integrated hydrostatically from a sea-level pressure. `anchored` fits both to a measured
  temperature and pressure.
  - Rejected: the aviation convention (offset at equal pressure altitude, ESDU 77022). It is
    equally hydrostatic, but it detaches the lapse rate from height. A launch-site measurement
    and a sounding are both keyed on height. The two differ by 0.38% in density at 3 km for
    +20 K.
- **Moist air:** an ideal mixture of dry air and water vapour.
  - Saturation vapour pressure is WMO-No. 8 eq. 4.B.1 over water at all temperatures, with no
    enhancement factor.
  - Water vapour `C_p = 4R*`.
  - Dry-air Sutherland viscosity.
  - Measured errors: density 0.047% against CIPM-2007, speed of sound 0.009% from the `C_p`
    choice, viscosity 2.1% at saturation and 30 °C. CIPM-2007 itself was rejected: it is valid
    only from 15 to 27 °C and from 600 to 1100 hPa.
- **Soundings:**
  - A profile takes its site's latitude and works in WMO geopotential height (eqs. 12.15–12.16),
    so its hydrostatics use the local normal gravity (±0.27% from `g₀`). The latitude-free
    geopotential was rejected: it is 0.1% off in pressure over 3 km at the equator and poles.
  - Temperature and humidity are linear in geopotential height, and pressure uses the
    temperature-shaped log form, which is exact for dry hydrostatic layers.
  - Given pressures must fall with height.
  - Omitted pressures are filled hydrostatically with virtual temperature.
  - Beyond the levels, the standard continues, anchored at the end level. Above the top, the
    vapour mole fraction is held and capped at saturation.
  - Geopotential heights convert with WMO-No. 8 eqs. 12.15–12.16 (latitude-dependent).
  - Rejected: linear-in-height pressure as RocketPy does it, which is up to 1.15% off between
    700 and 500 hPa.
- **Wind:**
  - Meteorological "from" directions, and ENU velocities.
  - `LayeredWind` defaults to speed-and-direction interpolation along the shorter arc, which
    keeps a veering wind's speed. A calm level takes the other level's direction.
  - `Components` interpolation reproduces RocketPy.
  - Beyond the end levels the end wind is held and flagged.
- **Turbulence:** Dryden spectra with MIL-F-8785C's scale-length convention (not
  MIL-HDBK-1797's halved transverse lengths).
  - Generated by the exact discretization of the shaping filters, not MIL-HDBK-1797's Euler
    filters, whose variance is biased by `1/(1 − aT/2)`.
  - `GustField` precomputes a realization so the integrator sees a pure function.
  - Medium/high-altitude intensities are caller-supplied: the specification gives them only as
    a graph.
- **Seeded generator:** `hpr_core::random::SeededRng` is xoshiro256++, seeded through SplitMix64,
  with Marsaglia–Bray polar normals. It is written in-house (about 60 lines) and checked bit for
  bit against `rand_xoshiro`, which is only a test dependency.
  - **Rationale:** seeded results must not change when a dependency updates, and a sampling
    crate can change its algorithm in a new release. It also keeps the pure core free of runtime
    dependencies.
  - **Frozen:** changing the algorithm, seeding or draw order needs an ADR.
  - **Portability:** the integer stream is the same on every platform. Normal deviates,
    turbulence and the atmosphere use the platform's `ln`, `exp` and `powf`, so those are
    bit-identical on one platform (the CLAUDE.md requirement) but may differ in the last bit
    across platforms.
  - **Serialized form:** checkpoints write the state as four `0x` hex strings, because JSON
    readers lose integers above 2⁵³.

**Consequences.**

- M1.6 must:
  - convert ellipsoidal height to height above sea level before calling the atmosphere or wind;
  - pick the gust field's path coordinate (distance through the air, or altitude) and how gusts
    start on the rail;
  - use the moist speed of sound and density for Mach and dynamic pressure.
- M2.1 cases against RocketPy need levels dense enough, or a linear-pressure compatibility
  option, and `Components` wind interpolation. RocketPy's Wyoming import converts heights with a
  radius only.
- M5.2 importers convert geopotential heights with the WMO formula, and clamp radiosonde
  humidity above 100% into `[0, 1]`.
- M6.1 reuses `SeededRng` for dispersions.

## ADR-005: Solid motors: statistics, consumption, grains, file models and the bundled catalog (2026-09-17)

**Context.** M1.3 builds `hpr-motor`. Loft's motor code went wrong in eight ways (lessons L36–L43):
one `.eng` block per file, lost delay markers, class letters off at band tops, the last sample as
burn time, a fixed midpoint CG with no inertia, unlicensed curves, loose impulse checks, and
header envelopes trusted over catalog data. RocketPy, the M2.1 oracle, has its own conventions.
Several choices had no single right answer (`docs/physics/motor.md`).

**Decision.**

- **Statistics:** total impulse integrates the straight-line curve exactly, from an implicit
  `(0, 0)`. Burn time follows NFPA 1125: between the 5%-of-peak crossings. Average thrust is total
  impulse over that burn time. This is ThrustCurve's glossary and its site code, which a committed
  fixture runs unchanged (hpr agrees to 1.8e-15 on the bundle); its statistics page words average
  thrust differently, and the difference is about 0.1% of impulse.
- **Impulse classes:** upper limits are inclusive (NAR: "5.01 to 10.0 N-sec" for `C`), from
  `1/8A` to `O`, and on to `Z` by doubling.
- **Consumption:** constant effective exhaust velocity, `m_p(t) = m_p0 (1 − I(t)/I)`. This is
  RocketPy's `SolidMotor` and ThrustCurve's `.rse` mass columns. It is an approximation (NASA
  SP-8039 shows `I_sp` drifting within a burn), recorded as such.
- **Propellant layout:** a fixed-shape column (the default, RocketPy's `GenericMotor` model) or
  BATES grains. The grain regression is solved exactly in burned mass instead of RocketPy's ODE in
  time. Motor mass and inertias agree with RocketPy within 7.9e-5 relative, and the centre of
  mass within 5.8e-6 of the motor length.
- **Envelope default:** from a catalog entry alone, the propellant is a solid column and the dry
  mass a thin tube, both over the full length and centred. Crude, and documented as such; motors
  with data use `SolidMotor::new`. Catalog metadata overrides the curve file's header.
- **Steps:** equal consecutive times in a curve are a step, not an error; decreasing times,
  negative thrust, and curves with no impulse or no NFPA burn time are errors. ThrustCurve's code
  instead averages points under 50 µs apart: identical on every bundled curve, and up to 1.1% in
  average thrust on 17 of 1710 survey files, all with repeated times.
- **Grains** need a bore: a solid end burner isn't a BATES grain, and its regression differs.
- **Ambient pressure:** the full-flow term `(p_ref − p_a) A_e` applies strictly inside the burn,
  as in RocketPy's flight, and only where the curve's thrust is positive. `p_ref`, the test site's
  pressure, is stored with the nozzle, since files don't record it. The term steps in after
  ignition and to zero at burnout, and overstates tail-off thrust (about 1% of
  impulse in vacuum for a 38 mm reload with a known exit); COTS data gives no exit diameter, so
  it is off by default.
- **File models:** `.eng` and `.rse` keep the file's units and points exactly, so read-write-read
  cycles are bit-exact. Readers are lenient and return warnings; writers refuse anything the
  reader would read differently. Readers take `&str`; decoding bytes is `hpr-io`'s job.
- **Delays:** the raw string is kept; `P`, `100` and `1000` read as plugged, and `0` reads as its
  own ambiguous "zero or plugged" setting, never as an ejection at burnout.
- **Curve end:** the thrust is zero from the last sample's instant on, so thrust, mass flow and
  propellant agree at burnout. The motor's dry mass must be positive.
- **Bundled catalog:** only curves ThrustCurve marks public domain, whose total impulse, burn time
  and average thrust each match ThrustCurve's stored values within 1%. That is 32 curves from B to
  O. Of 554 public-domain curves, 196 pass, so the rule leaves out 358. Most miss on burn time or
  average thrust, or their impulse is 1–5% off; the breakdown is in
  `docs/research/thrustcurve-data.md`. "Free" curves (which can be GPL) are never bundled.
- **Catalog metadata:** each bundled motor's published statistics and identifiers (names, class,
  type, dimensions, masses, impulse, thrust, burn time, delays, case) are copied from
  ThrustCurve's API into the committed index, with attribution. ThrustCurve states no terms for
  its metadata. These are facts about commercial products, mostly from certification bodies, for
  32 motors, and the milestone's 1% check and offline catalog need them. Neer confirmed on
  2026-09-18 that they are treated as facts, used with attribution; the catalog keeps them all.
- **XML:** `roxmltree`, a strict read-only parser that refuses DTDs; the `.rse` writer is
  hand-written.

**Consequences.**

- M1.4 places the motor's nozzle exit in the body frame and adds retainers with
  `with_added_dry_mass`.
- M1.6 takes mass, centre, inertias and `ṁ` from `SolidMotor::state`, and decides whether to
  apply the ambient-pressure thrust term (COTS files give no exit diameter).
- M2.1 feeds RocketPy only curves without an explicit `(0, 0)`: RocketPy prepends one, and the
  duplicate makes its impulse NaN. RocketPy's `GenericMotor.load_from_eng` uses the diameter as
  the chamber radius.
- M5 fetches the other curves into a cache and never bundles them.

## ADR-006: Component geometry and mass properties: frames, shapes, walls, fins and materials (2026-09-17)

**Context.** M1.4a builds the geometry and mass half of `hpr-design`. Loft's mass model went wrong
in the ways lessons L44–L46 and L48–L49 name: pitch inertia only, no radial terms, fin span and
tabs ignored, solid centroids for hollow parts, only a tangent ogive, and a kinked transition
profile. The published sources leave several conventions open. OpenRocket's documentation doesn't
say how wall thickness is measured, its ogive parameter is contradictory, and it doesn't say how
a boattail is oriented or where cant pivots (`docs/physics/shapes.md`, `docs/physics/mass.md`).

**Decision.**

- **Full tensor.** `MassProperties` carries mass, a 3D centre and the full inertia tensor about it
  in body axes, with positive products of inertia. Every part computes its tensor from geometry,
  so one- and two-fin sets, off-axis masses and lugs keep their products of inertia. It is not the
  "two nearly equal transverse moments" of OpenRocket's documentation (§4.2.3).
- **Component frames.** Each part has body axes with its origin on the axis at its forward end (a
  nose cone's tip), so it lies at `z ≤ 0`. The tree (M1.4b) translates and rolls it, and fixes
  the body origin `z_ref` that `frames.md` leaves to M1.4.
- **Shapes.** The ogive is Crowell's secant ogive, parameterized by `ρ/ρ_t` (1 is tangent; below
  1 bulges). OpenRocket's `κ` is not adopted, because its documentation defines it two ways; M3.1
  maps it by running the jar. The Haack parameter may reach 2/3, the monotone limit. A transition's
  shape puts its tip at the smaller end. Clipped transitions follow [TD] §A.7. Parameters out of
  range are errors, never defaults.
- **Walls are measured normal to the surface.** The wall is the part of the solid within `t` of the
  lateral surface: its inner surface is the envelope of circles of radius `t` on the profile, ends
  included and not extended, and the wall fills in near a tip. This is how a molded or laid-up
  shell is made.
  - At a cut end where the surface meets the end plane at an obtuse angle inside the wall, the
    inner corner is rounded rather than square: `t² (tan φ − φ)/2` less section per unit rim
    length, 3.1e-4 `t²` at 7°. On steep ends it is not small: 1.26% of wall mass at 56° and 2.24%
    at 60° for a 2 mm wall. M2.2 checks this against OpenRocket.
  - Extending the surface along its end tangent would cut it square. But that closes a steep end
    with a disc, and it made wall mass jump by 8.3% as an end slope rounded from finite to
    infinite, so it was dropped. Radial thickness, as in Crowell, overstates the wall by
  `√(1 + y′²)`. M2.2 measures what OpenRocket does, and any gap goes in the report, not into this
  model.
- **Fin cross-sections change the mass.** The section's thickness distribution is integrated
  exactly: square, rounded with semicircular edges, or airfoil as the NACA four-digit thickness
  distribution. OpenRocket's documentation uses the section for drag only. A slab would overstate
  an airfoiled fin by 46%; the NACA section is a definition, and it is documented as one.
- **Fin cant** pivots about the fin's span axis through the root mid-chord. Tabs are square slabs.
  The flat root sits at the body radius, and fillets wait for a milestone that needs them.
- **Recovery parts and mass components** are solid cylinders of their packed size. A parachute's
  cloth is `πD²/4`.
- **Numerics.** `hpr_core::quadrature` gains an adaptive vector G7K15 (QUADPACK's `QAG` without
  extrapolation), with substitutions and end-relative evaluation at blunt tips. Filled solids match closed forms
  to 1e-10 and 40-digit mpmath integrals to 1e-12; walls match an independent 25-digit mpmath
  envelope to 1e-10 (worst measured 5.9e-12). Principal moments
  use cyclic Jacobi, not the closed-form eigenvalue method that loses `√ε` for repeated moments.
- **Materials** are values stored in the design (name and density with its kind), not library
  keys, so designs stay complete offline. Built-in values cite primary public sources: USDA's Wood
  Handbook, manufacturers' data sheets and military specifications. Hobby tubes (cardboard, kraft
  phenolic, Blue Tube, Quantum) have no published density, so theirs is derived from the makers'
  published weights and dimensions, with the derivation recorded.
- **Crowell (1996)** is cited but not pinned: its only copy is on a plain-http mirror, and the
  Internet Archive was offline. The pinned OpenRocket documentation gives the same curves, and
  closed forms and mpmath integrals check them.

**Consequences.**

- M1.4b builds the tree on these parts: placement, auto radii, overrides, configurations with
  `SolidMotor`, reference diameter and checks. The part types hold resolved geometry. "Auto or
  fixed" radii and overrides belong to the tree's own types, so these serialized forms don't
  change when the tree arrives.
- Fin cant sign: a positive cant turns fin 0's leading edge toward `−y_B`, and a test pins it.
  M1.8 derives the roll-forcing sign from this geometry.
- M1.5 takes wetted areas, planform areas and centroids from `revolve` and the fin planforms.
- M2.2 compares OpenRocket's component masses, and reports the wall-thickness and cross-section
  conventions if they explain differences.
- M3.1 maps `.ork` shape parameters, the clipped flag, shoulders, tabs, instances and packed sizes
  onto these types, and settles the ogive parameter with the jar.

## ADR-007: Design tree: stations, placement, automatic radii, overrides, motors and checks (2026-09-17)

**Context.** M1.4b assembles M1.4a's parts into rockets. Loft's tree placed parts from OpenRocket
conventions it had read from OpenRocket's source, which this project can't do. It chose the
reference diameter from any component, internal ones included (L47). It flew a motor wider than
its mount and fins off the airframe without complaint (L50). RocketPy takes a rocket's mass,
centre and inertia as inputs and adds the motor; it computes nothing from geometry.

**Decision.**

- **Body origin at the nose tip.** `z_ref = 0`, so station `s` (aft of the tip) is `z = −s`.
- **One `Component` type holding a `Part` enum.** Body components (nose cone, body tube, transition)
  are a stage's list and stack from `s = 0` through every stage. External parts (fins, tube fins,
  lugs, rail buttons) hang off body tubes. Internal parts hang off body components or inner tubes.
  Roles are checked when the tree resolves, so importers build one type. Fins on noses and
  transitions are refused until a milestone models a sloped root.
- **Positions** are `top`, `middle`, `bottom`, `after` (the previous sibling) and `absolute`, with
  offsets positive aft. An attached part's extent is its root chord for fins, a whole row for lugs
  and buttons, and its packed length for packed parts.
- **Automatic radii** are a list of named dimensions on the component; stored values for them are
  ignored. Body radii take neighbours across stage boundaries. The previous component wins for
  tubes, and the next is a fallback. Shoulders take the adjoining tube's bore, rings their parent's
  bore and the widest overlapping on-axis sibling tube, and packed parts the bore.
- **Overrides** apply mass (rescaling the tensor with the mass), then the centre (along the axis
  from the forward end of what is overridden, and optionally off it, keeping the tensor about the
  centre), then the full tensor. A component's overrides cover it alone, or its subtree with `overrides_include_children`.
  A stage's cover the stage. Deeper overrides apply first; motors are never covered. The inertia
  override has no OpenRocket counterpart; RocketPy's examples need it.
- **Motors.** A `MotorMount` on a body tube or inner tube, with an overhang. Configurations put an
  embedded `SolidMotor`, with its case diameter and length, in each mount. The nozzle exit sits at
  the mount's aft end plus the overhang, on the mount's axis. All motors ignite at `t = 0` until
  M1.9. Catalog references wait for the facade (M4.1) and the online catalog (M5.4).
- **Reference diameter** defaults to the widest body component in any stage. `nose_base` and
  `custom` are the alternatives.
- **Checks** return typed findings with a severity.
  - Errors: a motor wider than its mount or wholly outside it, an external part that doesn't
    overlap its body tube, an internal part off the rocket or reaching past its parent's bore
    (measured about the parent's own axis), and a stage centre moved off the rocket by an override.
  - Warnings: a motor past its mount's top, attachments and internal parts past their parent's ends,
    a ring crossing an inner tube, radius steps, no nose cone.
  - Lengths compare with 1 nm of slack.
  - The flight engine (M1.6) must refuse designs with errors unless the caller accepts them.
  - `Layout::place_motors` takes any configuration, so a candidate motor is checked before it is
    stored.
- **Errors and limits.** An error inside a stage or component carries its id
  (`DesignError::InComponent`). Components nest at most 32 levels. `MassProperties::validate` now
  also refuses inertia on a body with no mass.
- **Public test designs** live in `validation/designs/` as JSON of `hpr_design::Rocket`. The format
  is provisional: M3.3 defines the open format and migrates them.
- **RocketPy comparison with substituted curves.** RocketPy's data files carry their own terms
  (`THIRD-PARTY-NOTICES.md`).
  - The committed fixture takes inputs only from RocketPy's notebooks and test code. It keeps each
    example rocket's inputs exactly, and pairs its motor with the bundled public-domain curve
    nearest in impulse.
  - Valkyrie is left out, because its inputs exist only in a data file.
  - Mass composition doesn't depend on which curve schedules the burn. Values at ignition are
    identical, and at burnout they agree to 1e-9. A local run with the examples' own curves stays
    under `refs/`.
  - The fixture also samples RocketPy at its LSODA knots. There hpr agrees to the solver's
    accuracy (1e-9), and the test holds 1e-8.

**Consequences.**

- M1.5 takes stations, radii and the reference area from `Layout`.
- M1.6 refuses designs with error findings, and takes `Assembly::mass_properties(t)` as the
  time-varying mass model.
- M1.9 adds ignition times, separation and per-stage assemblies to configurations.
- M2.2 measures OpenRocket's override order (L51) and automatic-radius rules with the jar. M3.1
  maps `.ork` positions, auto flags and overrides onto these types, and reports any rule that
  doesn't map.

## ADR-008: Subsonic normal force and centre of pressure (2026-09-17)

**Context.** M1.5a adds Barrowman's normal-force slope and centre of pressure. The sources
disagree in places. Barrowman's 1966 report fits ogive noses with 0.466 L, and his 1967 thesis
adds slender-body interference terms that the report leaves out. Niskanen (2009) keeps
`sin α/α` and adds body lift. The OpenRocket technical documentation (13.05) replaces the thesis's
roll-dependent three- and four-fin terms with plain `N/2`. Barrowman's own TIR-33 (1970) treats
six fins with its own interference factor. Loft reused the conical CP for every transition (L9),
had no fin-count correction (L8), and swapped elliptical fins for an equal-area trapezoid (L10).

**Decision.**

- **Bodies.** `(C_Nα)_B = (2/A_ref)ΔA` and `X_B = [l A(l) − V]/ΔA`, with `V` integrated from the
  real profile (no 0.466 L fit). Moments are summed as `(2/A_ref)[l A(l) − V]`, which stays well
  conditioned when `ΔA → 0`. The potential term carries `sin α/α` (Niskanen eq. 3.19). No Mach
  term.
- **Body lift.** Galejs's `K (A_plan/A_ref) sin² α` with `K = 1.1` at the planform centroid
  (Niskanen eq. 3.26–3.27), on every body component.
- **Fins.** Diederich's slope with `β = √(1 − M²)` (Barrowman 1967 eq. 3-6), the mean aerodynamic
  chord integrals with the CP at its quarter chord for all subsonic Mach (Barrowman 1967 p. 6).
  Niskanen's aft CP shift above Mach 0.5 (eq. 3.35–3.36) belongs with the supersonic fit it
  interpolates to and moves to M1.8. Freeform fins follow Niskanen pp. 27–29: the filled chord
  for the CP, the true area for the slope, the span-averaged mid-chord angle.
- **Fin count and roll.** `Σ sin² Λ_k` (exactly `N/2` for three or more fins) times 1, 0.948,
  0.913, 0.854 or 0.810 for up to 4, 5, 6, 7 or 8 fins (technical documentation eq. 3.54, from
  MIL-HDBK-762(MI) p. 5-24). More than eight fins are refused: the documentation's 0.750 has no
  source. One- and two-fin sets also report a side force across the flow's plane,
  `Σ sin Λ_k cos Λ_k`, derived from Niskanen's per-fin angle `α sin Λ_k` (eq. 3.50). Niskanen drops
  it, arguing that it cancels for two or more fins, but for two fins it adds. Fin sets at the same
  station are not combined.
- **Interference.** `K_T(B) = 1 + r_t/(s + r_t)` (Barrowman 1966 eq. 77, a fit for
  `r_t/(s + r_t) < 0.4`), not the thesis's full slender-body terms; the fin-induced body lift
  `K_B(T)` is neglected, as in the report.
- **Scope.** `0 ≤ M < 1`; `M ≥ 1` is an error until M1.8. The sources document the subsonic
  models only to Mach 0.8, and Niskanen's fin CP shift would already be 0.05 MAC aft there, so
  0.8–1 is an unvalidated extrapolation, accepted so M1.6 can fly through it until M1.8. Tube fins
  and unknown part kinds are refused. Lugs and rail buttons add no normal force. Cant is ignored
  until roll (M1.8). The model is a small-angle model; `α` is accepted over `[0, π]` so the flight
  engine can decide what to do near apogee. A slope that cancels to below 1e-12 of its terms has no
  CP; `moment_m` is always defined.
- **Six fins: MIL-HDBK-762 over TIR-33.** TIR-33 handles six fins with `K = 1 + 0.5 R/(S + R)` and
  no fin-count factor, without a derivation and for six fins only. MIL-HDBK-762 gives six and eight
  fins from slender-body theory, the technical documentation interpolates five and seven, and
  OpenRocket (the M2.2 oracle) uses the same factors, so a comparison there isolates other
  differences. With `x = r/(s + r)`, hpr's six-fin slope over TIR-33's is
  `0.913 (1 + x)/(1 + 0.5 x)`: −8.7% as `x → 0`, −4.3% at 0.1, +3.2% at 0.3 (the Recruiter) and
  +6.5% at 0.4, the edge of the interference fit's range.
- **Radius steps (an extrapolation).** Where one body component's aft radius differs from the next
  one's fore radius, the step adds `(2/A_ref)ΔA` at the joint (a zero-length transition), so the
  body's total slope is Barrowman 1966 eq. 10 over the whole body. Barrowman 1967 p. 18 assumes no
  discontinuities, so this goes beyond the source; dropping the step would lose its slope silently.
  It is reported as part of the aft component. A body that starts blunt (no nose cone) gets no term
  for its front face, as eq. 10 gives.
- **Validation, and the gap it leaves.** Barrowman's five published worked examples (NARAM-8's
  Testbed II and Aerobee 350; TIR-33's Javelin, Recruiter and Arcon-Hi), every printed component
  and total. With hpr's own model, four examples agree within 1% (worst −0.77%), and the
  Recruiter's six-fin slopes do not: +3.42% (fins) and +2.87% (total). That gap is this ADR's
  six-fin choice, measured: the rules differ by +3.22% and +2.83%, and with TIR-33's rule
  substituted in the slope and the CP weighting the Recruiter agrees within 1% (worst +0.19%). The
  test pins that exactly those two values fall outside 1% with hpr's model, and prints both.

**Consequences.**

- M1.6 builds an `AeroModel` once per design and calls `normal_force` (about 10 ns) per
  derivative evaluation, using `moment_m` for the moment (defined even with no net force). It
  decides how to treat large angles of attack.
- Unknown part kinds are refused, so a new `Part` variant needs an aerodynamic decision.
- M1.8 adds the transonic and supersonic slopes, the fin CP shift, roll forcing and damping.
- M2.2 compares CNα and CP against OpenRocket, which uses the same fin-count factors.

## ADR-009: Subsonic drag buildup, surface finishes and drag override tables (2026-09-17)

**Context.** M1.5b adds drag. Niskanen's thesis (2009, §3.4 and appendix B) is the primary
source; the OpenRocket technical documentation 13.05 reprints the same drag equations unchanged,
and Barrowman's 1967 thesis (ch. 4) is the source of its friction, roughness and leading-edge
formulas. The sources leave gaps: no coefficients for drag at angle of attack, undefined areas in
the boattail rule, an unstated diameter in the lug rule, no rail buttons, and friction that jumps
where eq. 3.81 switches branches. Loft merged fin sets (L11), used uncited constants (L12), had no
power-on base relief (L13), uncited lug drag (L14), no drag at bare steps (L15) and a silent cap of
10 (L16). The done-when compares with RocketPy's RASAero curves, whose inputs aren't recorded.

**Decision.**

- **Buildup.** `C_D0` = friction + pressure + base + parasitic on the reference area (eq. 3.75,
  3.97); `C_A = C_D0 f(α)`, positive toward the tail. Each component keeps its own terms (fin sets
  are never merged), and `AeroModel::buildup_components` reports them.
- **Friction.** Fully turbulent (Niskanen p. 43, who measured laminar runs changing apogee by under
  5%), `R` on the rocket's length (nose tip to
  the aft end of the last body component), eq. 3.78–3.84 as printed, the `R < 1e4` branch first,
  keeping the jumps at `R_crit` and at Mach 1. Roughness is per component (`hpr_design::Finish`),
  always over the rocket's length. The body form factor `1 + 1/(2 f_B)` uses body length over
  maximum body diameter; the fin factor `1 + 2t/c̄` is per fin set.
- **No laminar or transitional friction (M1.5's scope bullet names it).** Barrowman 1967's
  transitional form (eq. 4-2, 4-6: laminar below `R = 5e5`, `− 1700/R` above, turbulent throughout
  on any rough surface) and RASAero II's default laminar trip at `5e5` would lower smooth-surface
  friction by `1700/R`, about 4% of it and 2% of `C_D0` on Calisto (`R` 1.8e7). Following Niskanen,
  hpr stays fully turbulent; an option is issue #18.
- **Friction area (a departure).** A body's friction area is its axial projection
  `2π ∫ r dx = π A_plan`, not the slant surface of eq. 3.85: only the axial share of the wall shear
  makes drag. It changes slender noses by about 1% of their own friction (2.4% for a tangent ogive
  of fineness 2). With the slant surface, a shoulder's drag would stay about `C_fc ΔA/A_ref`
  (0.0015 in the L15 test) above a bare step's as its length goes to zero. M2.2 will see the
  difference against OpenRocket.
- **Pressure drag.** Noses and shoulders `0.8 sin² φ`, `φ = atan(dr/dx)` at the aft joint, on the
  increase in area (eq. 3.86), held through subsonic flow. Boattails follow eq. 3.88 on their
  decrease in area. Niskanen writes `A_base/A_boattail` without defining them and says a
  zero-length boattail drags like "the total base drag" (p. 48). Read with `A_base` as the aft
  base, a zero-length boattail would count that base twice and leave the uncovered annulus out, so
  hpr reads both as the decrease in area (Calisto's boattail: 0.052, against 0.046 the other way).
  Steps in radius are zero-length shoulders (`0.8 ΔA`) or boattails (base drag on `ΔA`); a body
  without a nose cone gets `0.8 A` on its face.
- **High subsonic.** Eq. 3.87 interpolates nose and shoulder pressure drag from eq. 3.86 at
  Mach 0 to appendix B's value and slope at Mach 1 (closed forms for cones and ogives, Stoney's
  data for other shapes); all of it arrives with M1.8. Until then the drag reads low from about
  Mach 0.6: a 3:1 tangent ogive misses 0.006 at Mach 0.7 and 0.021 (4–5% of `C_D0`) at 0.8, a 2:1
  cone 0.037 at 0.8. The buildup accepts Mach numbers up to 1 so M1.6 can fly, and sets
  `Drag::beyond_subsonic_methods` above Mach 0.8, the top of Niskanen's subsonic region (Table 3.1);
  the flag marks that edge, not the start of the error.
- **Base drag.** Eq. 3.94 on the last body component's aft area less the thrusting motors'
  cross-section (`DragConditions::thrusting` with the burning motors' area; Niskanen p. 50), down
  to zero.
- **Fins.** Eq. 3.89–3.93 by cross-section (square, rounded, airfoil) on `N t s`; `Γ_L` is
  `atan(x_t/s)` for trapezoids, the span average for freeform outlines, and a closed-form average for
  ellipses. Averaging the angle follows eq. 3.91; the drag goes as `cos² Γ`, whose span average is
  6% lower for an ellipse of `k = 1` (for M2.2).
- **Parasitic.** Launch lugs follow eq. 3.95–3.96 with `d` the outer diameter (the rail-pin passage
  on p. 52 only reads that way). Rail buttons follow the rail-pin rule, the stagnation coefficient
  on their side profile. Neither adds friction.
- **Angle of attack (derived).** Niskanen gives only the shape: 1 at 0°, 1.3 at 17°, 0 at 90°, zero
  slope at each. hpr uses the unique cubic per part that meets those conditions. Past 90° the flow
  meets the tail and hpr mirrors with the sign reversed, `f(α) = −f(180° − α)` (an assumption), so
  drag still opposes the motion. M2.2 compares with OpenRocket.
- **Refusals.** `M ≥ 1` for the buildup (the term functions are defined and finite to Mach 5),
  geometry the terms can't use, negative roughness (already at `Rocket::layout`), a coasting
  condition with a motor area, non-finite conditions and non-finite results are errors. Nothing is
  clamped.
- **Finishes.** `hpr_design::Finish` names the fifteen rows of Barrowman 1967 Table 4-1 (after
  Hoerner p. 5-3; Niskanen Table 3.2 reprints ten) plus a custom height, on `Component::finish`.
  The default is "paint in aircraft mass production", 20 µm: no source gives a hobby-rocket
  default, RASAero II defaults to smooth, and OpenRocket's "regular paint" is 60 µm (Niskanen
  p. 83).
- **Override tables.** `DragTable` holds `C_D0(M)` power-off and optionally power-on, read from CSV
  text: two columns (RocketPy's curves), or a named column of a headed file with rows at non-zero
  `Alpha` skipped (RASAero II's export). An identical repeated row is skipped; a Mach number
  repeated with another value, or out of order, is refused with its line. Linear interpolation,
  end values held, extrapolation reported. `DragConditions::thrusting` selects power-on, and an
  optional reference diameter rescales a table made on another reference area. The angle-of-attack
  factor still applies, and a model with a table accepts any Mach number for drag.
- **The comparison with RocketPy's curves.** Every RocketPy example whose drag curve is labelled
  RASAero: Calisto, Juno III, Cavour and Valetudo. At Mach 0.3 and USSA76 sea level (RASAero II
  computes its exports' Reynolds numbers at sea level, Users Manual p. 84). The curves stay in
  `refs/`; `cargo xtask aero` writes only derived numbers (each curve's value at Mach 0.3, hpr's
  `C_D0`, the error, the file's sha256) to `validation/fixtures/aero/rocketpy-drag-curves.json`,
  `hpr_aero`'s test recomputes hpr's values and the errors, and an xtask test reruns the comparison
  when `refs/rocketpy` is present. Provenance, traced through RocketPy's history:
  - Calisto's curve is a RASAero II export: the alpha-0 `CD Power-Off` column of the `CD Test.CSV`
    in RocketPy's first commit (2018). Its power-on column equals power-off (RASAero's v1 manual:
    power-on equals power-off without a nozzle exit diameter), so only power-off is compared. The
    design with the 2018 notebook's fins is the case: the export's `C_Nα` (6.09 per rad) and CP
    (66.2 in) are within 1.5% of hpr's for those fins (6.18, 66.2 in), not for the getting-started
    fins (7.32, 70.3 in), which are reported as a variant, not as more evidence.
  - Juno III's, Cavour's and Valetudo's are labelled RASAero but are 3-decimal tables with no input
    file. Juno III's serves power-off and power-on alike; Cavour's and Valetudo's have separate
    power-on tables, which are compared too (Calisto's power-on file is its power-off file; hpr's
    power-on result would be −5.0%). Cavour's power-off table has 22 extra rows repeating 13 Mach
    numbers up to 0.107, 7 with values 0.001 apart; the comparison keeps the first of each, and
    `parse_mach_csv` itself refuses that curve.
- **Inputs for the comparison: one declared rule, placeholders where nothing is known.** The
  exports record no inputs. Finish: RASAero II's documented default, smooth (p. 53,
  `Finish::Mirror`). Fin cross-section: a NACA 00xx airfoil file in the example gives an airfoil
  section that thick at the mean aerodynamic chord (Calisto's getting-started fins, NACA 0012); a
  published section is used (Juno III's team was cited for "análise de aletas com perfil de
  aerofólio truncado", an analysis of truncated-airfoil fins: a rounded leading edge and a blunt
  trailing edge, taken as rounded, with no thickness given); otherwise the M1.4b placeholder,
  square 3 mm (Calisto's 2018 fins, Cavour, Valetudo). A lift-curve airfoil says
  nothing about the edges and doesn't count. Rail buttons stay as the RocketPy examples define
  them; Calisto's come from RocketPy's test fixture, not the 2018 notebook.
- **Result, and the gaps it leaves.**

  | case | error | range over the inputs below |
  |---|---|---|
  | Calisto, 2018 fins | +4.4% (+1.8% without the buttons) | −14.0% to +12.8% |
  | Calisto, getting-started fins (variant) | −7.3% | −12.9% to +19.3% |
  | Juno III | −6.0% | −10.5% to +24.1% |
  | Cavour, power-off | −8.3% | −22.3% to −0.4% |
  | **Cavour, power-on** | **−18.3%** | −32.2% to −10.3% |
  | **Valetudo, power-off** | **−47.0%** | −59.4% to −42.5% |
  | **Valetudo, power-on** | **−50.4%** | −62.8% to −45.9% |

  The range is over square, rounded and airfoil fins (3 mm, or 12% of the chord for the airfoil),
  0 or 20 µm, with and without rail buttons. Before the published-section rule, square 3 mm fins
  gave Juno III +14.6% and the getting-started Calisto +10.7%. The test pins exactly the three
  cases outside 10%:
  - **Cavour under power, cause open.** At Mach 0.3, subtracting the motor's area removes 42% of
    Cavour's base drag (0.055) and 29% of Valetudo's (0.038). Cavour's power-on table is within its
    0.001 rounding of power-off from Mach 0.16 up (0.0001 at 0.3; 0.001 to 0.013 lower below),
    Valetudo's 0.004 lower, about a ninth of hpr's relief. The RocketPy designs have no motor case,
    so their motor diameter is the larger of the grain and nozzle exit diameters, and Cavour's
    result runs from −8.3% with no relief to −20.8% with its 75 mm motor (−18.3% at the 67 mm
    nozzle exit). The miss may be Niskanen's relief, a RASAero run with little or no nozzle exit
    diameter, or tables sampled along a flight (their uneven Mach spacing suggests it;
    unconfirmed). Flights with known motors (M2.1, M2.3) will test the relief.
  - **Valetudo's table** gives 1.05 at Mach 0.3, 1.44 times the OpenRocket export for the same
    rocket in RocketPy's RocketPaper repository (0.728). With that `.ork`'s inputs (regular paint,
    60 µm; two 14 mm × 30 mm lugs instead of rail buttons; its 3 mm square fins) hpr gives 0.714,
    1.9% under the OpenRocket export and 32% under the table. RocketPy's own notebook rescales the
    table by 0.9081/1.05.
  - **What the passing cases show.** Unrecorded inputs move each by 20% or more, and each passing
    rocket falls outside 10% under some plausible inputs. The check places hpr near RASAero's
    subsonic drag with a declared rule; it can't show agreement to 10% without the inputs. M2.2
    (OpenRocket, with known inputs) is the sharper test.

**Consequences.**

- M1.6 supplies `DragConditions` (Reynolds number per metre from the airspeed and the atmosphere's
  kinematic viscosity; whether a motor is thrusting and the burning motors' case area), uses
  `axial_coefficient` along `−z_B`, and decides what to do with `beyond_subsonic_methods`.
- M1.8 adds eq. 3.87's nose and shoulder interpolation, the transonic and supersonic branches of
  every term, and extends override tables to `C_Nα` and CP.
- M2.1's same-drag mode uses `DragTable`. It must decide whether RocketPy's drag scales with angle
  of attack as hpr's does, and how to read curves with repeated Mach numbers such as Cavour's, which
  `parse_mach_csv` refuses; its flights test the power-on base relief.
- M2.2 checks the angle-of-attack polynomial, the lug diameter, the boattail reading, the friction
  area and the leading-edge sweep average against OpenRocket.
- `AeroModel::drag` takes about 50 to 110 ns (`docs/perf.md`).

## ADR-010: Time integration: Dormand–Prince with dense output, RK4, stop times and events (2026-09-17)

**Context.** M1.6 asks for adaptive Dormand–Prince 5(4) with dense output and event
root-finding, plus a fixed-step RK4 option. M1.6 is split into M1.6a (the integrator and events)
and M1.6b (the flight). Loft's RK4 had no error control and no apogee convergence check (L21). It
didn't root-find events (L22), and it let burnout fall inside steps (L23). No crate in the
workspace had an ODE integrator or a root finder. Hairer and Wanner's `DOPRI5` is the reference
implementation of the method in their book and is BSD-2-Clause.

**Decision.**

- **One integrator in `hpr_sim::integrator`, no dependency.** It works on `[f64; N]` states
  through an `OdeSystem<N>` trait whose derivative can fail with the system's own error.
  Established Rust ODE crates either lack event location on dense output or pull in a
  linear-algebra stack. The method is a few hundred lines, and owning it lets the tests pin every
  coefficient.
- **Port `DOPRI5` as the reference.** Take its coefficients, the RMS error norm, the PI controller
  (`β = 0.04`, growth limits 1/5 to 10, safety 0.9), the starting step and the dense output
  (`CONTD5`). The stiffness detection is left out. The port is attributed in
  `THIRD-PARTY-NOTICES.md`, and the source is pinned (`hairer-dopri5`).
- **Per-component tolerance weights come from the system.** `Adaptive` holds one relative and one
  absolute tolerance, so it serializes, and `OdeSystem::absolute_tolerance_weights` scales `atol`
  per component. The default is `rtol = atol = 1e-8`. M1.6b sets the flight's weights and may
  change the defaults, from its benchmark.
- **`advance(system, t_stop)` is the only driver, and the system carries everything.**
  - `OdeSystem<N>` has the derivative and provided methods for tolerance weights, events and
    `accept_step`. One flight-phase type can then compute forces, define its events, record
    each step and stop the run, without the double borrows that separate event and observer
    arguments would force.
  - `advance` stops exactly at `t_stop`, at events (`Advance::Events`), or when `accept_step`
    breaks (`Advance::Stopped`).
  - Discontinuities are stop times, and the caller sets the phase between calls, because the step
    that ends at a stop time evaluates its last stage there.
  - Each call evaluates `f` afresh, and the step-size estimate carries over.
- **Events are sign changes between step ends, located by Brent's method on the dense output** to
  1e-12 s. The integrator stops at the end of the final bracket on the far side of the zero, with
  the dense-output state there, and reports every event past its zero at that state
  (`fired_events()`).
  - Stopping past the zero guarantees that the next call doesn't report those events again. A
    fresh full-order step can land a hair short of the zero and trigger again. Reporting all of them
    keeps coincident events, such as two deployments at apogee, from being lost.
  - The cost: the state at an event is fourth order (third for RK4) rather than fifth. The tests
    show event times within about 1.5e-8 s at the default tolerances.
  - Double crossings inside one step go unseen, and `max_step_s` bounds that. A `reset` that moves
    an event function back across zero re-fires it, so systems normalize inside their functions.
- **Departures from `DOPRI5`, all for robustness.**
  - A derivative that fails in a trial stage, or a non-finite error estimate, is a rejection. It
    becomes an error only when the step can't shrink.
  - A final interval within rounding counts as reached.
  - An overflowing step is an error.
  - The controller, the starting step and the error norm are otherwise `DOPRI5`'s, pinned by
    Hairer's Arenstorf problem, whose evaluation and step counts match an independent
    transcription.
- **RK4 is fixed-step,** shortened only to land on a stop time or an event. Its dense output is the
  cubic Hermite interpolant, whose end derivative is the next step's first stage.
- **Failures are errors, never quiet stops.**
  - The errors are `Derivative`, `StepTooSmall`, `NotFinite`, `StepLimit` (default 10⁶ attempted
    steps, adjustable with `set_step_limit`), `EventNotFinite`, `Backward` and `Settings`.
  - The integrator stays at its last accepted step and can resume.
  - M1.6b maps the errors to termination reasons (L25).
- **Settings files.**
  - `Method` is tagged `dopri5` or `rk4` and refuses unknown fields.
  - `Adaptive` keeps public fields with `Default`, for `..Adaptive::default()`. The crates are
    unpublished, so adding a field later is acceptable.

**Consequences.**

- M1.6b builds the flight as `OdeSystem<13>` phases (rail, powered, coast) separated by stop times
  (motor ignition and burnout, thrust-curve knots where they matter) and events (liftoff, rail
  exit, apogee, ground contact from the ellipsoidal height). It sets the tolerance weights for
  position, velocity, quaternion and body rate. It normalizes the quaternion where it is used, and
  resets only away from events.
- The recorder samples `Step::state_at` at its output times rather than forcing steps onto them.
- Stiff phases (a canopy opening in M1.7) will show up as small steps or `StepTooSmall`. M1.7
  decides whether they need a bounded step or a different method.

## ADR-011: Rigid-body flight: equations of motion, aerodynamic coupling, rail, phases and termination (2026-09-17)

**Context.** M1.6b builds the flight on the M1.6a integrator. It needs:

- a variable-mass rigid body with jet damping;
- aerodynamic forces from `hpr-aero`, which has no damping coefficients and refuses `M ≥ 1`;
- a rail with friction and button geometry, and events;
- a flight that names why it stopped;
- a Level 2 flight in 5 ms.

M2.1 will compare with RocketPy, whose documented equations are MIT. Loft's lessons L20, L24, L25
and L26 set tests.

**Decision.**

- **Equations: RocketPy's documented variable-mass formulation about a body-fixed point, with
  that point at the nose tip.**
  - RocketPy's technical documentation, Equations of Motion v0/v1 (Kane plus Reynolds transport,
    quasi-steady internal flow), covers centre-of-mass
    migration, `İ`, the jet's Coriolis force and jet damping in one exact form.
  - With the reference point at the body origin, the state needs no conversion to the design's
    stations.
  - The equations are written out in `flight.md` and checked against their classical limits:
    torque-free Euler motion about the centre of mass, and the classical jet damping
    `I_c ω̇ = [ṁ(r_e²/4 + l²) − İ_c] ω` (to 1e-6).
  - Measured thrust curves already contain the internal momentum terms `T04` subtracts
    (`−m r″ − 2ṁ r′ + m̈(n − r)`). They are kept, as RocketPy keeps them, for M2.1's comparison.
    The cost is at most 0.05 m/s at burnout and 21 N at Valetudo's liftoff (`flight.md`). Revisit
    after M2.3's flight data.
- **The nozzle gyration tensor comes from the integral, not from RocketPy's code.**
  - `S = (r_e²/4) diag(1, 1, 2) + |n|² 1 − n nᵀ`.
  - RocketPy 1.13.0 codes the transverse distance term as `0.25·n²`. M2.1 has to account for the
    difference.
- **Mass-property rates by central differences inside the integration interval** (half-width
  1e-4 s).
  - `hpr-motor` gives no derivatives of the grain inertia.
  - Stop times at every thrust-curve knot and burnout keep each difference on one side of every
    kink, and the mass rate matches the motor's `−F/c` to 1e-6.
  - The four `mass_properties` calls per evaluation are most of the 0.4 µs an evaluation costs
    (`perf.md`). Analytic motor derivatives are a later optimization, not a correctness need.
- **Earth's rotation: Coriolis force at the centre of mass, no Earth rate in the rotational
  equations.** It is at most 7.3e-5 rad/s against body rates of 0.01–10 rad/s. RocketPy does the
  same. Gravity is normal gravity at the centre of mass.
- **Aerodynamics component by component at the local flow.**
  - Each body and fin set sees `v_O − wind + ω × p_i` at its small-angle CP. That gives the pitch
    and yaw damping `hpr-aero` doesn't have, as RocketPy does.
  - The axial force comes from the whole rocket at the centre of mass's airspeed, power-on while a
    motor burns.
  - The linear pitch model matches the flight's period to 8e-5.
  - `hpr-aero` gains `component_count`, `component_normal_force` (allocation-free) and
    `component_station_m`.
- **Fins follow the crossflow at any angle: `C_Nα sin α` instead of `C_Nα α` in flight.**
  - This is Niskanen's substitution for bodies (eq. 3.16–3.17), applied to fins. No large-angle
    fin model is in hand, and stall is not modelled.
  - It changes nothing at small angles and makes the force vanish for tail-first axial flow.
  - With the linear force, a calm vertical flight falling tail first after apogee flipped the fin
    force with rounding noise and ran to the step limit (found in review, pinned by a test).
  - `hpr-aero`'s own `normal_force` stays linear. M1.8 should move a large-angle fin model there.
- **Motors are evaluated inside their burn at interval ends.** A stage evaluated on ignition or
  burnout takes the one-sided limit inside the burn, because the pressure correction switches
  there. The step ending at burnout otherwise saw the burnt-out thrust, and RK4 converged at first
  order.
- **Out-of-range aerodynamics stop the flight.**
  - `M ≥ 1` anywhere is a `SimError::Aero` until M1.8. There is no silent clamp.
  - Large angles of attack use the small-angle models extended as above, recorded in
    `Sample::angle_of_attack_rad`.
- **Rail.**
  - The aft end starts at the rail's foot.
  - The rocket is guided with one degree of freedom until the aft edge of its last rail button or
    lug passes the top (L26), or its aft end without guides. RocketPy ends at the forward button.
  - Tip-off rotation is not modelled.
  - Coulomb friction `μ|ΣN|` uses the net rail reaction across the axis (weight, aerodynamic and
    mass terms), not the sum over the buttons. The default `μ = 0`, for want of a cited value.
  - The pad phase holds the rocket until the force along the rail beats friction. A stall on the
    rail returns it to the pad, held where it stopped rather than sliding back.
- **Events and termination.**
  - The events are liftoff, rail exit, burnout (a stop time), apogee (the centre of mass's
    ellipsoidal-height rate), ground hit (the centre of mass at the site's ellipsoidal height) and
    user events on a `Sample`.
  - Flights end as `GroundHit`, `NoLiftoff`, `StalledOnRail`, `TimeCap` or `StepLimit` (L25).
    Everything else is an error.
  - The atmosphere takes `h − N` with the geoid undulation given in `Environment`.
- **Defaults: `rtol = atol = 1e-8`, unit weights.** A Level 2 flight takes 1.1 ms and its apogee
  is within 1.1e-6 m of the converged value. At 1e-6 it would take 0.6 ms and 7e-5 m.
- **Observation.**
  - An `Observer` trait sees every accepted step (`FlightStep`: the dense state, and a `Sample` from
    one evaluation) and every event.
  - `Recorder` keeps chosen `Channel`s at a fixed interval or at every step, plus event rows.
    It is built, not deserialized, and cleared between flights.
  - Runs don't mutate the `Simulation` (L24).
  - `Environment` shares its atmosphere and wind through `Arc`, so it clones cheaply, and
    `Simulation` is `Send + Sync` for parallel Monte Carlo.

**Consequences.**

- M1.7 adds recovery devices as phases and events on this engine. A parachute's opening is stiff;
  bound the step there.
- M1.8 replaces the Mach refusal and adds roll forcing and damping. Until then the roll rate
  changes only through inertia coupling.
- M1.9 adds ignition times and staging (new stop times and a changing assembly).
- M2.1's RocketPy comparison must align the rail-exit convention, the nozzle gyration tensor and
  RocketPy's per-surface stations.


## ADR-012: Recovery: drag areas, triggers, inflation and the descent phase (2026-09-17)

**Context.** M1.7a adds parachutes to the M1.6 flight engine. What is in hand:

- Knacke's *Parachute Recovery Systems Design Manual* (NWC TP 6575, 1991), which prints canopy
  drag coefficients on the nominal area, canopy fill constants, drag-area growth laws, opening-load
  methods and the equilibrium descent speed. Its title page limits distribution, so it is cited,
  never redistributed, and no text is copied.
- RocketPy 1.13.0 (MIT), whose parachute phase is a point mass and which M1.7a is compared against
  within 3% (the milestone's *done when*).
- Loft's lessons L27 (instant inflation, no opening load, an unsourced body term, no drogue
  release), L28 (a step floor and a time cap that left descents unlanded), L29 (a parachute `C_D`
  copied from OpenRocket's GPL source) and L92 (the terminal-descent case).
- `hpr-design` already has parachute and streamer **parts**, which carry mass and packing but no
  drag or deployment data.

**Decision.**

- **The milestone is split.** M1.7a is parachutes, triggers, inflation, release, drift and landing,
  with both of M1.7's *done when* bullets. M1.7b is streamers, tumble and separated bodies, with
  its own bullets. Streamers and tumble need drag sources Knacke does not have (he has no streamer
  data at all), and separation needs a decision about how a body's mass and drag are defined; that
  is a milestone's worth of work on its own.
- **Recovery devices live in `hpr-sim`, not in the design tree.** A `Device` is a drag area, a
  trigger, a lag and an inflation law, given to a `Simulation` through `with_recovery`. The design's
  `Parachute` part stays what it is: mass and packing. A design file that carries deployment data is
  M3.1's problem (`.ork` has it) and can map onto these types.
- **Drag areas are `C_D S`, either given directly (RocketPy's `cd_s`) or from a canopy's nominal
  diameter with `C_D0` on the nominal area `S₀ = π D₀²/4`** (Knacke's convention, printed page 5-2).
- **Canopy data comes from Knacke's tables, with the printed page at each accessor**, for thirteen
  types: the `C_D0` range (Tables 5-1 and 5-2), the fill constant (Table 5-6, unreefed), Pflanz's
  drag-area growth exponent (Figure 5-51) and the infinite-mass opening-force coefficient `C_x`.
  - **The default `C_D0` is the middle of the printed range** (flat circular: 0.775 of 0.75 to
    0.80). Knacke prints no single value, and hpr will not copy OpenRocket's 0.8 (L29). Where his
    tables say "insufficient data" the accessor returns `None` and the caller must supply a number.
  - RocketPy's default `C_D` of 1.4 is a hemispherical canopy's coefficient on the **projected**
    area, used only to turn `cd_s` into a radius for its added mass. It is not a `C_D0`.
- **Inflation: `(C_D S)(t) = (C_D S)₀ min(1, (t − t_d)/t_f)^j`**, with `t_f` either zero (instant,
  RocketPy's model), fixed, or Knacke's `t_f = n D₀/v` from the airspeed at line stretch.
  - Knacke's measured **overshoot** (10 to 80% above the steady drag area, Figure 5-40) and his
    `C_x`/`X1` opening-load methods are **not** modelled. hpr's peak load is therefore a lower
    bound on the real opening shock, and instant inflation is hpr's own upper bound. Both are
    stated in `docs/physics/recovery.md`; Ludtke's and Pflanz's laws are later work.
    - *Corrected 2026-09-18 (M0.4b's physics review):* neither bound holds in general. Opening at
      once is Knacke's infinite-mass case, and his force-reduction factor `X1` (printed page 5-50)
      falls as low as 0.02 for a canopy large for its load, so the real peak can be far below
      hpr's. Near the canopy's terminal speed, as at apogee, a filling time gives hpr a higher
      peak than an instant opening. `docs/physics/recovery.md` states the corrected limits.
  - A deployment at zero airspeed has no filling time in `n D₀/v`, so the canopy opens at once.
- **Triggers: apogee, a height above the site while descending, a time after ignition, and a
  motor's ejection delay after its burnout.** Both the apogee and the height trigger are numeric as
  well as event-driven — "descending", and "descending at or below the height" — which is
  RocketPy's own form (`y[5] < 0`) and an altimeter's behaviour, and which means a flight that
  starts past its apogee still deploys (found in review: an event-only apogee trigger fell
  ballistically to the ground with no deployment and no error). A motor with no delay in seconds
  (plugged, or unset) is refused rather than assumed.
  There is no sampling rate and no barometric noise: hpr locates the crossing with its event
  finder. Monte Carlo can perturb the setting instead (M1.10).
- **A device can be released by another's opening** (`released_by`), which cuts a drogue away under
  a main (L27). The release waits until the releasing device is **fully open**: releasing at its
  line stretch collapsed the drag area to almost nothing while the main filled, and the descent
  sped up (found in review, now a test). A device released before its own charge fires never
  deploys. Open devices otherwise **add** their drag areas, where RocketPy keeps one `cd_s` and
  replaces it, which is why the comparison gives its drogue `released_by` the main.
- **The descent is a point mass in a new `Phase::Descent`**, entered at the first deployment:
  `m a_cg = −½ ρ (C_D S) |v_cg − w| (v_cg − w) + m (g + a_Coriolis) + T`.
  - The attitude freezes and the body rates are set to zero. A tethered rocket's attitude under a
    canopy is not modelled by anything hpr can cite.
  - **The airframe's own drag is left out**, as RocketPy leaves it out, rather than carrying Loft's
    unsourced `0.5 A_ref` term (L27). For a drogue whose drag area is near the body's broadside
    area this is a real omission, and it is where M1.7b's tumble drag belongs.
  - The thrust is kept along the frozen axis, so an off-nominal deployment under thrust is not
    silently thrust-free.
  - **Added mass is not modelled.** Knacke gives no closed form (printed page 5-40 is qualitative),
    and RocketPy's `m_a = k_a ρ (2/3) π R² H` carries no citation in its code and no weight in its
    equations, so it changes no equilibrium rate, only the transient. The comparison shows the
    cost: NDRT 2020, whose main's added mass is 15.9 kg against a 20.8 kg rocket, is hpr's worst
    case at +0.71% in descent time and +2.86% in the smaller drift component.
- **Every deployment, every end of filling (which is also a release) and every known trigger time
  is a stop time**, so no step straddles a change in the drag area, and events are located as in
  M1.6a. The numeric triggers are checked once per interval, on one evaluation shared by every
  pending device, which `Stats` does not count. The event list is
  now built per interval as a `Watch` list instead of numbered by hand, because the height triggers
  come and go.
- **The comparison with RocketPy starts where both models agree.** Both simulators start from the
  same declared post-burnout state near apogee with the first device's lag overridden to zero, so
  almost no segment under either model's aerodynamics separates them (RocketPy's trigger sampling
  leaves 2.5 to 13 ms of its own 6-DOF flight, `recovery.md`), with RocketPy's noise zeroed (it
  draws on the global `np.random`) and a declared wind (the examples' own winds need the network or
  Copernicus files). Five example rockets. The oracle runs at `rtol = atol = 1e-8` and records its
  own solver's contribution per metric: at most 3.5e-6 on every compared metric (its one larger
  entry, 2.1e-3, is on a 20 µm drift component this comparison did not compare; M2.1a measures it,
  and reading 28x high there is what found the gravity-model difference of ADR-015, issue #27). The
  differences that remain
  are listed in `recovery.md`, with the trigger-sampling one measured rather than assumed away. Whole-flight comparisons, ascent included, are M2.1's.

**Consequences.**

- M1.7b adds streamers, tumble and separation. It needs a streamer drag source (Knacke has none),
  a tumble model, and a decision about how a separated body's mass and drag are defined.
- A cited apparent-mass model would close the gap against RocketPy's transient; it is the only
  known model difference left in the descent.
- `Sample` gained `recovery_drag_area_m2` and `Channel` gained `RecoveryDragArea`, so reports can
  show the opening.
- M1.10's Monte Carlo can vary deployment heights, lags and drag areas; nothing here samples.
- M3.1 maps `.ork` recovery devices onto these types.

## ADR-013: Streamer and tumble drag (2026-09-17)

**Context.** M1.7b needs a cited drag model for a streamer and for a body tumbling with nothing
deployed. Knacke's manual, which M1.7a leaned on, has **no streamer data at all** (checked: the
type tables 5-1 to 5-5, the measured-drag section 5.2.3, the miscellaneous-decelerator section
5.8.4 and the contents) and no bluff-body crossflow table. What is in hand
(`docs/research/streamer-and-tumble-drag.md`):

- The **OpenRocket technical documentation v13.05** (CC BY-SA, a published document, pinned):
  Appendix C fits a streamer drag coefficient to its own wind-tunnel tests, and §3.5 fits a
  tumbling model to 22 m drop tests. Its Java source is GPL and is never read.
- **Carruthers and Filippone (2005)**, peer-reviewed wind-tunnel measurements of streamers and
  flags, whose AIAA copy is paywalled and whose author post-print states no licence.
- **Kidwell's NARAM-43 drop tests (2001)**, OpenRocket's own reference for its appendix, with per
  streamer masses and measured descent rates. No licence stated.
- Loft's L29 lesson: its recovery defaults came from OpenRocket's **source**, which must not be
  ported.

**Decision.**

- **A streamer is a drag area from a correlation on its planform area `S = l w` and aspect ratio
  `AR = l/w`, and hpr carries both correlations** (`StreamerModel`), because they disagree by a
  factor that runs from 1.9 (at 80 g/m²) to 5.8 (at 10 g/m²) and only one of them survives a
  comparison with a free drop.
  - **The default is Carruthers and Filippone's**, all three of its printed curves:
    `0.561 AR^−0.480` at `S = 0.025 m²` (eq. 2), `0.6514 AR^−0.6075` at 0.05 m² (the trend line on
    Figure 3, which the text does not repeat) and `0.405 AR^−0.494` at 0.075 m² (eq. 1). hpr
    interpolates between neighbours linearly in `ln S` and holds the end curve outside; that is
    hpr's choice, documented as such. Blending only the two extremes, as the first draft did,
    reads 18% below the paper's own middle curve at `AR = 3.3` (found in review).
  - **`StreamerModel::OpenRocket`** is appendix C's `C_Dm = 0.034 ((ρ_m + 25)/105)((l + 1)/l)`,
    kept for comparing with OpenRocket in M2.2. It is the only one of the two that uses the
    material.
  - **The measurement that decides it.** Recomputed here from Kidwell's Table 1 and his results,
    with his normalisation to a notional 5 g weight and his distance-over-time rates compared
    against the same average from the closed-form fall (both corrected in review): his crêpe
    streamer, the one he left unpleated, descends at 2.80 m/s, a `C_D` of 0.155 on its planform.
    Filippone's correlation gives 3.05 m/s (+9%); appendix C gives 5.28 m/s (+88%). Kidwell's
    pleated streamers descend slower still (Micafilm at 2.04 m/s, `C_D` 0.338), which neither
    model reaches.
  - **Above the largest fitted area hpr holds the end curve rather than extrapolating.** The
    paper's trend would give a lower `C_D` (about `S^−0.3`), but the only free-drop measurement in
    hand is higher than either, so the clamp is the closer of the two. `recovery.md` states both.
  - **Pleats are not modelled**, so hpr predicts a faster descent for a folded streamer. That is
    the safe direction for a landing, and it is stated in `docs/physics/recovery.md`.
  - hpr's streamer descent rates will therefore differ from OpenRocket's by 1.4 to 2.4 times,
    depending on the fabric (the drag-area ratio runs 1.9 to 5.8). M2.2 will see that; it is the
    intended difference, not a defect.
- **Tumble is the technical documentation's §3.5 model**, `C_D S = 1.42 A_f + 0.56 A_bt`, computed
  from the airframe by `DeviceDrag::tumbling`: `A_bt` by integrating the outer diameter along the
  axis (each body component's mean diameter times its length), `A_f` as one fin's planform area
  times Table 3.4's efficiency factor for the fin count. More than eight fins is refused, because
  the table stops there.
  - Its constants come from Hoerner's *Fluid-Dynamic Drag*, which is copyrighted with no legal
    free copy. hpr cites the documentation, and the research note records NASA TN D-540 and
    TR R-474, which are free and carry the same numbers, for when a pinned source is needed.
  - **hpr states what it can demonstrate, not the documentation's accuracy claim.** Replaying its
    own Table 3.3 through hpr's reading of the model gives −5.8%, −5.4%, −7.2%, +19.0% and −10.0%
    on the five drop-test models, not the 3 to 14% it claims: the finless tube wants a body
    coefficient near 0.79 where the model prints 0.56, and the text pins neither area convention.
    The spread is a test (`the_tumble_model_against_its_own_drop_tests`) and is what the docs
    quote (found in review: the claim had been repeated without checking it).
  - **The fit is for small models** (44 to 103 mm, 6.8 to 160 g, 5.0 to 6.6 m/s). A high-power
    booster is outside it, and above `Re ≈ 2e5` a cylinder's crossflow drag falls by about half,
    so hpr will read slow there. The limit is documented rather than extrapolated.
- **A tumbling body is a device with a trigger, like a canopy.** hpr does not decide by itself
  when a rocket tumbles: no source in hand says when a stage becomes unstable enough, and the same
  documentation declines to model the analogous twirling streamer regime.
- **Both new sources are pinned and cite-only** (`validation/refs.lock.toml`,
  `THIRD-PARTY-NOTICES.md`): the Filippone post-print and Kidwell's report state no licence, so
  their numbers are used and their text is never copied or redistributed.
- **M1.7 is split again.** M1.7b is streamers and tumble, with the first of M1.7's remaining
  *done when* bullets; M1.7c is separated bodies, with the second. Separation needs its own
  decision about how a body's mass properties and drag are defined, and `Assembly` has no split.

**Consequences.**

- M1.7c flies separated bodies. Since the descent phase already drops airframe aerodynamics, a
  separated body needs mass properties and its own device, not an aerodynamic model — which is
  the cheapest honest way in.
- M2.2's OpenRocket comparison should compare streamers under both models, and report the gap
  rather than tune either.
- If a streamer model is ever fitted to more drop data, `StreamerModel` is the place for it; the
  research note lists what would be needed.

## ADR-014: Separation: bodies, their masses and their descents (2026-09-17)

**Context.** M1.7c has to fly every body of a separated rocket to its own landing, with its own
mass properties and drag. What is in hand:

- The descent phase (ADR-012) is a point mass: it needs a mass and a drag area, and deliberately
  has no airframe aerodynamics.
- `hpr-design` carries `MassProperties` per stage (`Layout::stages`) and a stage index on every
  placed motor, so a body's mass properties are a sum over its stages.
- `hpr-aero` needs a nose-first layout, so a partial airframe has **no** aerodynamic model. A body
  that had to fly aerodynamically would need M1.8's work and a way to build a model for a
  headless stack.
- M1.9 will add staging, where a sustainer lights after separation and keeps flying under thrust.

**Decision.**

- **A separation is a trigger plus a stage boundary** (`Separation { trigger, after_stage }`), with
  the same triggers a device has. Stages `0..=after_stage` keep the nose (body 0), the rest form
  body 1. Two bodies for M1.7c; more splits are additive and nothing in the types forbids them
  later.
- **The ascent ends at the separation**: `Termination::Separated`, a `Separation` event, and one
  `BodyFlight` per body in `FlightResult::bodies`. Nothing continues in six degrees of freedom,
  because no body has an aerodynamic model.
- **A body is its own stages and their motors.** Its mass properties are the sum, so the bodies'
  masses add to the rocket's at that instant (a test). A body's mass is then held **constant**
  through its descent, which is why a separation must follow the last burnout. That is **enforced**
  when the trigger fires, not merely documented: whether it fires before the burnout depends on the
  flight, so it is a flight-time `SimError::Domain` rather than a setup check. A release across the
  separation is refused when the devices are given, because a line cuts a device on its own body.
- **The separation adds no impulse.** Each body starts at its own centre of mass with the velocity
  that point already had (`v_O + ω × r_cg`), so the bodies' **linear** momenta add to the stack's
  (a test). Angular momentum is not conserved: the bodies drop their rotation, which discards each
  body's spin about its own centre (31% of the total at the test's 0.6 rad/s). The identity is
  exact only after burnout, because `v_cg` also carries the centre of mass's motion inside the
  body — which is the other reason the burnout rule is enforced. An ejection charge's impulse, the
  tip-off it gives each body and the tumbling that follows are not modelled; hpr says so rather
  than inventing a spring constant.
- **Every body must carry a recovery device, and that device must open.** A flight whose bodies
  are not all covered is refused when it is set up; a body that reaches the ground with no
  deployment at all is refused in flight. With no airframe drag in the descent, either case would
  be a fall in a vacuum, which is a wrong number rather than a missing feature. The corollary is
  recorded in `recovery.md`: a body coasts drag-free between the separation and its first
  deployment, which reads high in arrival speed. A spent booster's device is
  normally `DeviceDrag::tumbling_stages` over its own stages, which is §3.5's model applied to
  that body rather than to the whole stack (the whole-stack form is still there for a stack that
  tumbles without separating).
- **The bodies fly as 6-state point masses** through the same integrator, with the descent's
  equations less the thrust, and their own stop times and events: their devices' trigger times,
  the deployments, ends of filling and releases their devices already have, their own apogee, and
  their own deployment heights. Review found the first draft missing the apogee and the carried
  times, which made a body separated while climbing fall ballistically and a deployment scheduled
  before the separation vanish. They share the flight's devices and their progress, so a canopy
  that opened before the separation stays open on the body that carries it.
- **Only body 0's devices act before the separation.** A device for another body has a drag area
  computed for that body, which is not a model of the whole stack.
- **The separation's own trigger is a stop time, and its height is a located event**, as a
  device's is; review found it polled at whatever boundary happened to come next, which fired a
  timed separation 186 s late and an altitude one never.
- **A body's descent is not observed.** `Observer` sees a 13-element rigid-body step; a body's
  state is six. Its events and samples are in its `BodyFlight` (`BodySample`, `BodyEvent`), which
  is what a report needs. Wiring the observer to bodies can come with M2.1's reports.

**Consequences.**

- M1.9's staging extends this: a body that keeps flying needs an aerodynamic model for a headless
  stack, which is the real work, and the ignition times and mass variation that go with it.
- M2.1's landing metrics take the bodies, not just the final sample.
- A separation with three or more bodies needs `Separation::BODIES` generalised to a list of
  boundaries; nothing else changes.
- Because a body is a point mass, its attitude is not tracked at all after separation, so nothing
  can report how a booster is oriented as it tumbles.

## ADR-015: The validation harness: cases, references, tolerances and reports (2026-09-17)

**Context.** M2.1 is the first end-to-end milestone: hpr's numbers against other tools' for whole
flights. M2.1a builds the harness the suite runs on. What shapes it is less the plumbing than
Loft's five validation lessons (`docs/research/loft-lessons.md`): a comparison that wasn't
like-for-like (L75), a reference regenerated whenever it disagreed (L76), hand-written "stored
results" (L77), suites that skipped themselves and reported green (L78), and 10 of 12 metrics with
no gate at all (L79).

Already in hand: five oracle fixtures under `validation/fixtures/` with their generator scripts,
and M1.7a's recovery comparison, whose references are real RocketPy output.

**Decision.**

- **A case is a TOML file, a reference is JSON a generator wrote, and the harness only ever reads
  the reference.** `validation/cases/<id>.toml` says what to fly and which metrics to compare
  against which reference file and case; `validation/fixtures/**` holds what the oracle said. A run
  writes nothing but the report, so hpr cannot move its own goal posts (L76) — there is no
  `--update-references` flag, and there will not be one: a reference moves when its generator runs.
- **Every reference value carries a source** naming the oracle, the generator and the field it came
  from, and a value with a blank source is refused (L77). The harness builds those strings from the
  generator's own provenance block rather than trusting a hand-written label.
- **Every metric a case reports has a tolerance that bounds something, or a written reason why it
  is not scored** (L79). A tolerance is a fraction, an absolute difference, or both; one that
  bounds nothing — including an infinite or negative bound — accepts nothing rather than passing
  quietly, and a case that measures or publishes anything it does not account for is refused, in
  both directions: hpr may not measure a metric the case does not gate, and the reference may not
  publish one the case ignores.
- **A metric that cannot honestly be scored is declared, not smoothed over.** The descent cases
  carry no absolute floors: an absolute bound wide enough to carry Valetudo's near-zero northward
  drift would also have been 8.7x looser than 3% of that case's whole drift, which is a weakened
  check wearing a tolerance's clothes. The alternative is `not_scored = "<reason>"`: the harness
  measures and prints the metric with both numbers and the reason, and counts it apart from the
  verdict. Loft excused its two largest misses as "no single target" (L82), so a blank reason fails
  outright and the whole excused set is pinned by a test named after what it is. **No metric uses
  it today.** It was written for Valetudo's northward drift, where hpr read 28x RocketPy; the cause
  turned out to be the gravity model below, and the metric is now gated at 3% like every other. The
  mechanism stays for M2.1b, whose predicted-mode supersonic cases are gaps by construction.
- **A RocketPy comparison flies RocketPy's gravity model.** hpr's default is the full
  normal-gravity vector, which above the ellipsoid leans a few parts in 10⁶ toward the equator;
  RocketPy
  applies gravity to the vertical axis alone. The difference is invisible in every metric that
  matters and decisive in the one that does not: 5.2e-4 m of northward drift over an 800 m
  descent, against a 2.0e-5 m Coriolis signal. hpr already ships `GravityModel::VerticalTaylor`
  as RocketPy's formula "for like-for-like comparisons", so the harness uses it, and Valetudo's
  northward drift comes to −1.8% instead of +2704%. The general rule this is an instance of: when
  the oracle's model is a documented simplification of hpr's and hpr can be asked for the same
  simplification, the comparison uses it and says so, rather than reporting the modelling gap as
  a physics gap (L75).
- **The cases that must run are locked** in `validation/cases/lock.toml`, and a locked case that is
  not there is an error, not a skip (L78). A committed case that is not locked is an error too, so
  a case cannot be added and forgotten — and both checks live in the command, not only in a test.
  `--fast` may only leave out cases the lock marks slow, names them in the report, and writes
  `latest-fast.{md,json}` rather than the committed record.
- **The oracle's inputs come from the reference's own record of what it flew** (L75): the descent
  cases take the site, the wind, the devices and the state at the first deployment from the
  fixture, so a case cannot compare hpr against hpr. That extends to the vehicle: the reference
  records which design it flew and what it weighed, and the harness refuses a case that names
  another design or whose mass differs by more than 1e-9, so a copied case file cannot report the
  difference between two rockets as a difference in the physics. The one place that knows a
  generator's JSON shape is `hpr_validate::rocketpy`, and a file that does not name the oracle,
  the generator and the command that produced it is not a reference at all.
- **The report is committed**, in Markdown for people and JSON for machines, and carries no
  timestamp, so a run that changes nothing changes no bytes and a number that moves shows up in
  the diff. It carries each reference's SHA-256 and the generator's own description of what the
  oracle modelled and what it had to override, so a hand-edited reference or an unlike comparison
  shows up in the report rather than only in git history. A test asserts the committed Markdown is
  byte-for-byte what the harness produces. The JSON is pinned in two parts, because it carries
  full-precision floats and hpr's determinism promise is bit-identical results **on one platform**,
  not across three: everything that cannot differ by platform (cases, metric names, sources,
  tolerances, verdicts, reasons, hashes) is compared exactly, and the numbers through the Markdown
  the committed JSON renders, to the six decimals that report prints. That is the resolution at
  which the descents reproduce on macOS, Windows and Linux, measured, not assumed — CI failed the
  first time this was asserted at full precision. If a platform ever diverges at six decimals, the
  answer is to find out why, not to loosen the comparison.
- **M2.1a's first cases are M1.7a's descents.** They are the only references in hand that cover a
  whole hpr flight path end to end, and reusing them means the harness ships with five real cases
  rather than a demonstration. M2.1b adds the ascent cases in both modes, the CI job and the
  regeneration workflow, which is where `Flight` grows a variant.

**Consequences.**

- M2.1b adds a `Flight::WholeFlight` variant, the same-drag and predicted modes, `hpr-validate`'s
  own binary if one is wanted, and the CI job. The metric names M2.1 lists (apogee, time to
  apogee, maximum velocity and Mach, rail-exit velocity, burnout state, time-series RMS) arrive
  with it.
- M2.4's census reads `validation/reports/latest.json`.
- `hpr-validate` reads files, so it is not part of the pure core and `cargo xtask wasm-check`
  leaves it out, as `ARCHITECTURE.md` already says.

## ADR-016: The documentation site: mdBook over `docs/`, and checks for links, labels and equations (2026-09-18)

**Context.** M0.4 asks for one searchable site that a hobby rocketeer can read (VISION V15, and
CLAUDE.md's "Documentation is a deliverable"). Its first increment, M0.4a, asks for three things:
an ADR that picks the tool and the layout; one source per page, with equations that render both
on the site and on GitHub; and a CI build that fails on a broken link or a bare internal label.
What was in hand on 2026-09-18:

- **The pages.** There are 18 pages under `docs/physics/` and `docs/format/`. They write
  equations in Unicode inside ```` ```text ```` blocks and inline code (`γ_e = GM/(ab)`), with no
  LaTeX anywhere, and they had no Markdown links at all. Code comments, docs and oracles cite
  their paths 135 times. They carried 146 bare labels: `L12`, `ADR-008`, `M1.5b` and the like.
- **The tools.** Versions and licences were read from crates.io on 2026-09-18:
  - mdBook 0.5.4 (MPL-2.0).
  - mdbook-linkcheck 0.7.7 (MIT), which is built against mdBook 0.4 and pulldown-cmark 0.8 and
    does not load in 0.5.
  - lychee 0.24.2 (MIT OR Apache-2.0).
  - pulldown-cmark 0.13.4 (MIT), the Markdown parser mdBook 0.5.4 itself uses.

**Decision.**

- **mdBook 0.5.4 builds the site.** It is a single binary with built-in search. It takes its
  sidebar from a plain `SUMMARY.md`, and its pages are ordinary Markdown with no front matter, so
  GitHub renders the same file. The alternatives each break one of those:
  - Zola needs TOML front matter on every page, which GitHub would show as text.
  - MkDocs needs a Python toolchain and a YAML nav.
  - Docusaurus needs a Node toolchain.

  mdBook is MPL-2.0, and we only run it: it is never linked or ported, and `cargo deny` never
  sees it. Its theme files (MPL-2.0) and the libraries they bundle are copied into the built site
  unchanged, with their licence headers and texts. `THIRD-PARTY-NOTICES.md` lists them. CI pins
  0.5.4, and `cargo xtask site` refuses any other release series. A local build with another 0.5
  release (Homebrew installs the latest) may differ slightly from CI's; `cargo install mdbook
  --version 0.5.4 --locked` matches it exactly.
- **The site's source is `docs/` itself.**
  - `book.toml` sits at the root with `src = "docs"`, and the site builds into the gitignored
    `target/site`.
  - `docs/SUMMARY.md` lists the pages. *Start here* is `docs/start-here.md`, not `README.md`,
    because mdBook turns a link to `README.md` into `README.html`, a file it never writes.
  - `docs/physics/` and `docs/format/` are therefore in the site's source without moving. Each
    page has one source, and the 135 citations of their paths keep working.
  - The working files are not pages: `STATUS.md`, `ROADMAP.md`, `DECISIONS.md` and the research
    notes. The site links to them on GitHub. Whether the decisions and the roadmap become pages
    is M0.4b's call.
- **Equations stay in Unicode**, in ```` ```text ```` blocks and inline code.
  - They render the same on GitHub, on the site and in rustdoc, which renders no LaTeX either.
    Every existing page already writes them this way.
  - mdBook's MathJax doesn't accept the `$` delimiters GitHub uses. `mdbook-katex` would add a
    preprocessor to build and pin, for pages that don't need one.
  - `$x$`, `` $`x`$ ``, `$$` and ```` ```math ```` fail the check, because only GitHub would
    render them. Two prices in a sentence ("$5 and $10") are not math.
    Revisit this if a page needs typeset math.
- **One link rule serves both renderers.** A relative link goes only to another page of the site,
  or to a file under `docs/`, which mdBook copies. Anything else in the repository is linked by
  its `https://github.com/nrdptel/hpr-sim/blob/main/...` URL. A relative link to anything else
  works on GitHub but breaks on the site.
- **The checks are our own**, in `xtask/src/site.rs`, and read pages with `pulldown-cmark`, so
  they see the same links the site renders.
  - **On the sources**, which `cargo test` checks too:
    - every summary entry exists, and every page under `docs/physics/` and `docs/format/` is
      listed;
    - relative links and their `#fragments` resolve, with anchors derived as GitHub derives them;
    - a GitHub URL to `main` names a file in the working tree, and for Markdown, a heading in it;
    - an undefined `[text][ref]` fails, as do bare labels, math only GitHub renders, and a link
      inside a heading.
  - **On the built HTML**, which `cargo xtask site` checks after `mdbook build`: every relative
    `href` and `src` must reach a file, and every fragment an `id`. This catches what mdBook
    rewrites, and any id it derives differently from GitHub. A warning from mdBook fails the build
    too.
  - **Why not lychee.** The link rule needs a check of our own anyway: lychee accepts a relative
    link to `../ROADMAP.md`, because the file exists on disk. Labels need a Markdown parser
    anyway. A Rust check runs in `cargo test` with unit tests that show each failure, offline,
    with nothing to install but mdBook. lychee remains the candidate for fetching external links.
- **External links are counted, not fetched.** A PR's checks must not depend on the network
  (CLAUDE.md rule 5, and flaky CI). Today the site has eight: RocketPy's and OpenRocket's sites,
  the three format specs, and three links to this repository's issues. Links to files in this
  repository on GitHub (`blob/`, `tree/` or `raw/main/`) are checked offline against the working
  tree. A scheduled check of external links is tracked in
  [issue #38](https://github.com/nrdptel/hpr-sim/issues/38).
- **Bare labels fail.** `L` or `ADR-` followed by digits, and milestone ids (`M1.5b`, `M2.1b1`),
  fail as whole words outside a link's text: in prose, tables, headings, inline code, raw HTML
  and image alt text. Fenced code blocks are exempt, since they quote files verbatim. A label that
  emphasis splits in two (`**ADR**-008`) is not caught.
  - The fix is a link with a few words of meaning: `[Loft lesson L10][lessons]`,
    `[ADR-009][adr-009] (drag)`, or `[M1.8][roadmap], the supersonic aerodynamics milestone`.
    Reference definitions at the end of the page point into `DECISIONS.md` (by heading anchor),
    `ROADMAP.md` and `research/loft-lessons.md` on GitHub.
  - Some ordinary words look like labels, and are written so they don't: a motor designation in
    full (`L1150R`, not `L1150`, which the lesson pattern would catch); a certification level as
    "Level 2", not "L2"; an appendix equation as "eq. A1.3" is fine, since only `M` starts a
    milestone id.
  - Labels stay out of headings: mdBook wraps each heading in a link, so a link inside one is
    invalid HTML, and the check fails it.
- **CI.** A `site` job on Linux installs mdBook 0.5.4 through `taiki-e/install-action` and runs
  `cargo xtask site` on every PR.

**Consequences.**

- A PR that moves or renames a file a page links to fails until the page follows.
- Milestone and lesson links open the whole roadmap or lessons page, because their entries have no
  heading anchors. The link text names the label, so the browser's search finds it.
- Anchors are GitHub's slugs. mdBook's ids agree with them for every heading on the site today, and
  the built check confirms that on every build. A heading whose two anchors differ fails that
  check, and is reworded.
- The local gate gains `cargo xtask site`, which needs mdBook installed (`brew install mdbook`, or
  `cargo install mdbook --version 0.5.4 --locked`).
- The site is not published until M0.4d, which waits for Neer to turn on GitHub Pages.

---

## ADR-017: Model pages open with *In short*; Accuracy traces its numbers; the records stay files (2026-09-18)

**Context.** M0.4b asks that every model page open with *In short* (what it models, its source,
how well it is validated, what it leaves out), that a page without it fail CI, that an *Accuracy*
page give every validation result from the committed report, and that the decisions and the
roadmap be reachable from the site. ADR-016 left open whether the last two become pages. On
2026-09-18 there were 16 model pages under `docs/physics/`, one validation report with 5 cases and
30 metrics, 17 decision records and a roadmap of 806 lines.

**Decision.**

- **The form of *In short* is fixed, so a check can hold every page to it.** Right under the
  page's title, a `## In short` section holds only a bulleted list of four items, which open with
  the bold labels `What it models:`, `Sources:`, `How well it is validated:` and
  `What it leaves out:`, in that order, each followed by its answer. The labels repeat the
  milestone's own words. What pages used to open with (code paths, decisions, the source list)
  moves under a heading of its own, usually `## Code and sources`. A model page is any page under
  `docs/physics/`; the format pages under `docs/format/` describe files, not models, and are
  exempt. The check (`in_short` in `xtask/src/site.rs`) reports the first departure and its line.
- **Its answers follow the page, and say how far the evidence goes.** *How well it is validated*
  names which of the four kinds of evidence in `VALIDATION.md` the model has (analytic, published
  source, another code, real flights), with the page's own numbers, and says plainly when a model
  has not been compared with another code or a real flight. The check can't judge that; the
  physics review does.
- ***Accuracy* traces every number it quotes.** Each number in a paragraph, list item or table row
  (outside code) must appear in a repository file that the same paragraph, item or row links to:
  a model page (less its own *In short*), the report, a case file. The pages at the top of the
  site don't count, so a page can't vouch for itself. A number is anything with a decimal point,
  an exponent (`1e-6`, `10⁻¹²`), a percent sign or two digits; "Level 2", "6-DOF" and "3 fins"
  are words. It matches only a number written the same way: the same digits as a whole number,
  and the same sign and percent sign where the page writes them. So a reader checks it in one
  click, and a number that moves at its source fails the site check until the page follows. The
  check can't tell whether a number is quoted in the right context; reviews do that.
- **The report's results are checked cell by cell.** A table on *Accuracy* whose header opens
  with `case`, and names a report metric in code in each other column, must give each case's
  difference exactly as the report writes it, and all the report's results must be in such a
  table. So the page gives every validation result, and each one right. Numbers are checked, not
  generated: a generated page would read worse than one written for people, and the check gives
  the same guarantee. The page must also link every model page.
- ***In short* traces its numbers too.** Each number in a model page's *In short* must appear in
  the rest of that page, or in a file its item links to, so the summary can't claim what the page
  doesn't show.
- **The decisions and the roadmap stay files, with a page that indexes them.** Rendered as pages,
  `DECISIONS.md` and `ROADMAP.md` would carry hundreds of labels at the places that define them,
  which the label check would have to learn to accept, and they are working files that change
  with every milestone. Instead, *Decisions and the roadmap* explains the three kinds of label and
  links every decision record and every phase of the roadmap on GitHub, each with a line of
  meaning. The check fails if a record or a phase is missing from it.

**Consequences.**

- A new model page fails CI until it opens with *In short*, and until *Accuracy* links it.
- A regenerated report that moves a quoted number, or adds a case or a metric, fails CI until
  *Accuracy* follows. So does a model page whose quoted number changes.
- A new decision record fails CI until the records page links it.
- The check can't tell a true *In short* from a false one. Reviews do that, against the page's
  body, `VALIDATION.md` and the report.

## ADR-018: Examples run in CI against committed output; pages quote files, checked line for line (2026-09-18)

**Context.** M0.4c asks for a *Getting started* page whose example program runs in CI. CLAUDE.md
asks that code in the docs compile and run in CI, and that the numbers a page quotes can't go
stale. mdBook's `{{#include}}` would put a file into a page, but only on the site: GitHub shows the
directive as text, and every page must read the same on both (ADR-016). And a page that shows what
a program prints quotes numbers that move whenever a model does.

**Decision.**

- **The first flight is Valetudo, recovered.** `crates/hpr-sim/examples/first_flight.rs` flies
  RocketPy's Valetudo design (K400C curve) from a 3 m rail in a 5 m/s wind, with a drogue at
  apogee and a main at 150 m. The design is committed, the flight tests and benchmarks already fly
  it, and it stays subsonic (Mach 0.36). The synthetic 54 mm design on its I175 reaches Mach 1,
  which hpr refuses until M1.8. The design is built in with `include_str!`, so the program runs
  from any directory.
- **What an example prints is committed beside it,** as `<name>.output.txt`. `cargo xtask
  examples` runs every example target that `cargo metadata` lists and writes those files;
  `--check` writes nothing, and fails if an example fails, has no file, or prints anything else.
  CI's test job runs `--check` on macOS, Windows and Linux, which shows the output is the same on
  each. Examples print rounded values (0.1 m, 0.01 s), where the platforms agree; they agree to six
  decimals on descents (M2.1a).
- **A page quotes a file under a marker.** A `<!-- quote: <path> -->` comment right above a fenced
  code block makes the block a quote of that repository file, and `cargo xtask site` fails if the
  two differ by a line (a `\r\n` reads as `\n`). The comment shows on neither GitHub nor the site.
  So the page, the file and what the program prints on each OS are one text.
- **`Environment::with_wind`** sets a wind without `Arc` and struct-update syntax, which a first
  program shouldn't need.

**Consequences.**

- A model change that moves a printed digit fails CI until `cargo xtask examples` rewrites the
  output and the page's quote is copied again: two steps, on purpose, so the page can't go stale.
- An example that prints more digits than the platforms agree on fails CI; it must print fewer.
- The test job builds and runs every example, which adds seconds to it.
- A block without a marker is not checked, and code in prose (such as the page's suggested edits)
  is not compiled. Reviews cover those.

## ADR-019: Publishing the site and the API reference to GitHub Pages (2026-09-18)

**Context.** M0.4d asks for the site to be published from `main`, with the workspace's rustdoc
beside it and each linking the other. GitHub Pages hosts a public repository's site for free, but
turning it on is a repository setting, which only the owner may change (the project's rule 9);
until then, a deploy fails. Rustdoc merges its search index and list of crates with whatever an
earlier run left in its output. With `--no-deps`, cargo documents a workspace's crates in no set
order, and rustdoc links another crate's item only if that crate's pages already exist; otherwise
it leaves the link as text, without a warning. And the crates' documentation is also read outside
the site, in `cargo doc --open` and later perhaps on docs.rs, so it can only link the guide by its
address.

**Decision.**

- **The API reference is part of the site, under `api/`.** `cargo xtask site` builds the rustdoc
  of every library crate (not `xtask`, not the command-line binary), with all features and
  `-D warnings`, and copies it to `target/site/api`. The site is one artifact, and a local build
  has both halves.
- **Built on its own, in order.** The rustdoc build has its own target directory,
  `target/site-rustdoc`, whose documentation is cleared before each build (not its compiled
  dependencies, so a rebuild takes seconds). Crates are documented one at a time, each after the
  workspace crates it depends on, so every link between crates resolves. `api/index.html` sends a
  reader to the guide's page, *The API reference*.
- **They link each other, and the check holds both ends.** *The API reference* links each crate's
  front page by a relative link. Each crate's `//!` documentation links the guide's pages for its
  models by address, `https://nrdptel.github.io/hpr-sim/...`, which the built-site check reads as
  the local build. So a crate the site doesn't link fails, as does a crate that links no page of
  the guide, or a link to a page the guide doesn't have.
- **Every link in the reference is checked**, as the guide's are, and a link to another crate that
  rustdoc left as text fails. Three kinds of link are exempt, each named in the code: the
  implementor scripts rustdoc loads only when they exist, line ranges on its source pages, and
  links inside documentation it copies from a dependency (glam's, for the vector types), which it
  passes on as written when it can't place them.
- **The site knows its path.** Pages serves a project's site under `/hpr-sim/`. `book.toml` sets
  `site-url` to it, so mdBook's 404 page works at any address, and the check resolves
  root-absolute links, links by the site's address and a `<base href>` against it, as a browser
  would.
- **CI deploys from `main`, after every other check.** The `site` job uploads `target/site` as
  the Pages artifact on every run (`actions/upload-pages-artifact@v5`). A `deploy` job publishes
  it (`actions/deploy-pages@v5`) on `main` only, once fmt, clippy, the three test jobs, doc,
  wasm-check, deny and site have passed on that commit. It alone may write to Pages.
- **Until Pages is on, the deploy is skipped, and says so.** On `main`, the `site` job asks
  GitHub's API whether Pages is on and deploys from GitHub Actions. If not (a 404, or another
  source), it prints a warning and the deploy job is skipped, so `main` stays green. Any other
  answer from the API fails the job, so a broken check can't pass for "off". Pull requests don't
  ask, since they never deploy.
- **The README's first lines link the address,** and say the site goes live once Pages is on.

**Alternatives.**

- A workflow of its own for Pages: it would publish a commit whose other checks failed.
- Letting the deploy fail until Pages is on: `main` would be red for a reason outside the code,
  which hides real failures.
- `actions/configure-pages` with `enablement: true`, which turns Pages on from CI: a change to a
  repository setting, which is the owner's call.
- docs.rs for the rustdoc: it needs the crates published on crates.io, also the owner's call.
- Linking the guide relatively from the crates (`../../physics/aero.html`): it works inside the
  site only, not in `cargo doc --open` or on docs.rs.
- One `cargo doc` for all the crates: faster by a few seconds, but its cross-crate links depend on
  which crate cargo happens to document first.
- Not inlining glam's types (`#[doc(no_inline)]`): no copied links to exempt, but `DVec3` in every
  signature would lead nowhere, since glam's own pages aren't built.

**Consequences.**

- The deploy bullet of M0.4d waits for the owner to turn Pages on (`STATUS.md`, Needs Neer).
  Then the next push to `main` deploys, or `gh workflow run CI --ref main` without one, and the
  README's "goes live once" sentence goes.
- `cargo xtask site` also runs `cargo doc` once per library crate, about 5 s with a warm build and
  10 s from cold.
- A library crate added to the workspace fails the site until *The API reference* links it and its
  documentation links the guide.

## ADR-020: The reader test, and labels that lead to plain words (2026-09-18)

**Context.** M0.4e asks for a reviewer with no project context to answer ten new-user questions
from the site alone, and for every term it flags as unclear to be fixed. The first cold pass
answered five of the ten fully and five in part. It had four blocking findings: no way to describe
your own rocket, no path from a motor file to a flight, pages that disagreed about clusters, and an
ambiguous height for wind tables. It flagged about forty terms. One finding ran across every page:
a milestone or Loft lesson label linked to the top of a long file (`ROADMAP.md`, the lessons
table), which says nothing about the label, and about ninety labels in the API reference were bare
([issue #44](https://github.com/nrdptel/hpr-sim/issues/44)). The roadmap has no heading per
milestone to link to, and a Markdown anchor comes only from a heading, which may not hold a label
(ADR-016).

**Decision.**

- **The test reads the built site, cold.** A reviewer with no project context gets a copy of
  `target/site`, the API reference included, and nothing else from the repository, except a file that a page links on
  GitHub, as a reader would click through. The ten questions are listed in the PR. Each answer
  cites its pages; every flagged term is fixed; then a fresh reviewer, on the fixed site, answers
  the same ten again, to show the fixes read as meant.
- **Every label leads to a line of plain words.** *Decisions and the roadmap* has a row for every
  milestone and increment of the roadmap, anchored by its id (`M1.10` as `#m1-10`), saying in a
  sentence what it covers and whether it is done. Each Loft lesson a page names has a row too
  (`#l15`), which links its section of the lessons file. The guide's labels link those rows; the
  crates' documentation links them by the site's address.
- **The table can't go stale.** The site check fails if a milestone of `ROADMAP.md` has no row, a
  row names no milestone, or a row's status isn't the roadmap's: `done` when checked off, `blocked`
  when marked so, `not yet done` otherwise.
- **An `<a id="...">` is an anchor.** A page's raw HTML may name an anchor for a place that isn't a
  heading, such as a table row. mdBook keeps the `id`, and GitHub scrolls to it, so the link
  check accepts it like a heading's.
- **The API reference is checked for bare labels,** as the guide is: the visible text of every
  rustdoc page, except the source pages, which quote the code as written.
- **Questions become examples.** What a new user most asked for, and couldn't do from the pages,
  is shown by a program that CI runs and the page quotes: flying your own rocket with its centre of
  pressure and stability margin, reading a motor file and the bundled motors, each kind of wind,
  and recording a trajectory.

**Alternatives.**

- Headings for milestones and lessons on the records page: a heading can't hold a label, so its
  anchor would be words (`#staging-clusters-and-air-starts`) that a writer can't derive from the
  label.
- Linking each label to its phase of the roadmap or its section of the lessons file: closer than
  the top of the file, but still a long list to search, and the reader lands on GitHub rather than
  a line of plain words.
- Anchors in `ROADMAP.md` itself: it is a working file, edited with every milestone, and not a
  page of the site, so a reader still leaves the site to decode a label.

**Consequences.**

- Checking a milestone off in `ROADMAP.md` fails the site check until its row on *Decisions and the
  roadmap* says `done`; the error gives the row to write. A new milestone needs a row.
- A page that names a lesson without a row fails the link check until the row is added.


## ADR-021: Whole flights against RocketPy: what is compared, and the gaps it may declare (2026-09-18)

**Context.** M2.1b2 scores hpr's whole flights against the same-drag reference M2.1b1 committed,
`validation/fixtures/flight/rocketpy-whole-flight.json`. Its done-when asks for at least five cases
that pass. RocketPy's five examples in that reference include Prometheus 2022, which peaks at Mach
1.014, and hpr refuses `M ≥ 1` until M1.8 (ADR-008, ADR-011). The first run of the other four put
hpr 2.7 to 3.9% high on peak acceleration and 6 to 9% high on apogee, everywhere but NDRT 2020.
At Valetudo's peak, 0.034 s after ignition, there is no drag yet, so the difference had to be
thrust or mass. The reviews then found three definitional differences and a real one: the
landing points, recorded but not compared, were far apart in wind.

**Decision.**

- **The designs fly the thrust curve as RocketPy does.** RocketPy's
  `Motor(reference_pressure=None)` default, which every example keeps, makes its `pressure_thrust`
  zero (`motor.py:1188-1189`). `cargo xtask designs` had written hpr's 101,325 Pa stand-in
  instead, which adds `16 kPa × A_e` of thrust at a 1,400 m site; NDRT, at 206 m, was the one case
  it barely touched. `Nozzle::reference_pressure_pa` is now an `Option`, and the key is required
  (`null` for none), so a design says which it means; the transcription writes `null`. The site's
  example flights move with it: the first flight's apogee goes from 874.0 to 779.0 m.
- **A sixth rocket, not a weaker bar.** Bella Lui, on M2.1's own list of example rockets, is added
  to `flight.py` alone, with its example's site and rail and a declared wind (its example's weather
  is an ERA5 file).
- **The reference records everything the flight needs, and the harness checks hpr against it**
  (L75): the parachutes, RocketPy's `effective_1rl`, and the motor (total impulse, burn-out time,
  propellant mass, reference pressure). A design whose dry mass, reference area or motor differs
  by more than 1e-9, or a case whose drag is not the generator's declaration, is refused.
- **Metrics mean what RocketPy means by them** (L80). Speeds and accelerations are those of the
  centre of dry mass, the point RocketPy's state follows (`v_O + ω × p`,
  `a_O + ω̇ × p + ω × (ω × p)`). Heights are measured from that point's height at launch, because
  RocketPy's starts at the ground (`z_init = elevation`): the main opens and the flight lands at
  RocketPy's heights too. The rail exit is where the rocket has travelled `effective_1rl`, the
  forward button at the top; hpr's own exit is the last guide's (L26). The maxima are over both
  ends of every solver step, so a peak at an event, such as a canopy opening, is read at its
  instant. The drifts of the apogee and the landing point, and the landing speed, are added to
  `flight.py`'s metrics, as M2.1 lists them.
- **The committed report is pinned where the platforms agree.** A whole flight reproduces across
  macOS, Windows and Linux to about 1e-8 of each value, not always to the sixth decimal the report
  prints (NDRT's landing drift is 354.240893 m on macOS and 354.240895 m on Linux; one number of
  75 differed). ADR-015 asks to find out why before loosening. hpr is deterministic on each
  platform, and the likely source is the platforms' maths libraries, whose `sin`, `cos` and `exp`
  differ in their last bit between macOS, glibc and MSVC; an 84 s flight in a sheared wind carries
  that to 6e-9 of the drift. This amends ADR-015's six-decimal rule for whole flights: the test
  that holds the committed report to this run's allows a number two units of its sixth decimal
  or 1e-7 of itself, and nothing else; every word, tolerance and verdict must match. It is still
  a million times tighter than any gate.
- **A known gap is declared, checked and pinned.** A case may say `known_gap = "..."`. The harness
  accepts one kind, hpr's refusal of a real Mach number at or past 1; it checks that the reference
  reaches Mach 1 and fails the run once hpr flies the case
  ([Loft lesson L85](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md)).
  The report lists gaps in a section of their own; they count neither as a pass nor as a fail, and
  a test pins the set.
- **What does not agree is reported, not scored, and not called a pass.** In wind, hpr turns into
  the wind less than RocketPy: its apogee moves 67 to 85% as far upwind (Juno III 769 m against
  1,147 m, ending 228 m from the pad against 582 m). Flown once in calm air, the two agree on the
  apogee to 0.18% and on the drifts to 1.3 to 3.7%. The drifts of the four windy cases, and
  Valetudo's still-air landing drift (−3.41%), are open misses whose cause is unknown: they are
  printed with both numbers and pinned, issue #50 tracks them, and **M2.1's landing offset is not
  met** until it closes. A reviewer argued they should count as failures; they don't, because a
  suite that fails on a known, tracked gap can't gate anything else, but the roadmap, the status
  and *Accuracy* say plainly that the metric is not met. So are Calisto's time of peak acceleration (two peaks
  0.9% apart, which hpr's rail terms reorder) and NDRT's whole-flight peak, the main opening, where
  RocketPy has added mass and hpr has none (ADR-012). Every other metric is gated at 3% with no
  floor, as ADR-015 requires.

**Alternatives.**

- Scoring four cases and calling Prometheus the fifth: it compares nothing, so it would not be a
  pass.
- Keeping the stand-in and gating at 10%: that would hide an input difference as a model
  difference.
- Leaving the drifts out of the metrics, as M2.1b1's fixture did: M2.1 names the landing offset,
  and the difference is the largest one found.
- Gating the drifts with a tolerance wide enough to pass: that is the "no single target" excuse of
  L82 with a number on it. They are printed, pinned and tied to an issue instead.
- Flying Prometheus's subsonic part only, or giving it a drag that keeps it subsonic: a different
  flight from the reference's, or a reference tuned to suit hpr.

**Consequences.**

- The heights, speeds, times and accelerations of five flights agree within 3%, the largest
  +1.783% (Bella Lui's 7 ms ignition spike on the rail, where hpr keeps variable-mass terms that
  RocketPy's `udot_rail1` leaves out: +1.2 to 1.3 m/s² with thrust and mass the same to five
  digits) and +1.710% (Juno III's apogee, in the strongest wind). The path in wind is open
  (issue #50): each code's own normal force and damping, hpr's drag growth with the angle of
  attack and the rail release are the candidates.
- No motor gets the sea-level correction by default: catalog and `.eng` motors carry no nozzle,
  and a design's nozzle must say which reference pressure it means.
- When M1.8 lifts the Mach limit, the Prometheus case fails until its gap is removed. Its
  tolerances are already argued in its case file.

## ADR-022: Validation in CI, and regenerating references only by hand (2026-09-18)

**Context.** M2.1c asks for a CI job that runs `cargo xtask validate` against the stored references
and is green on three OSes, and for a separate workflow, triggered only by a person, that
regenerates the references and whose output is a diff to review, never a commit. `cargo test`
already reran every case and compared the result with the committed report, but inside a test
that nobody would read as "the validation suite ran", and `cargo xtask validate` itself rewrote
the report rather than checking it. hpr is bit-identical on one platform, not across three
(ADR-015), so the committed report cannot be compared byte for byte on Windows or Linux. The
references come from RocketPy, which needs the oracle environment in the gitignored `refs/` (uv,
RocketPy 1.13.0 and its dependencies); no CI job had one. Loft lesson L76 is why a reference must
never move on its own: Loft's advice to regenerate a reference whenever a check failed let the
reference follow Loft's own drag.

**Decision.**

- **`cargo xtask validate --check`** runs every locked case, writes nothing, and fails unless every
  scored metric passes and the run reproduces the committed `latest.md` and `latest.json`. The
  comparison is `Report::reproduces`, moved from the report test into `hpr-validate` so the test
  and the command share one definition: everything that cannot differ by platform (cases, sources,
  tolerances, verdicts, notes, gaps but for the Mach number the integrator narrowed onto) exactly,
  and hpr's and the reference's value at full precision from the JSON, to 2e-6 or 1e-7 of
  itself, whichever is larger. `latest.md` must be `latest.json`'s rendering exactly, since both
  come from one platform. The old test compared the rendered Markdown instead, where a printed
  percentage's last digit can round the other way on another platform.
  `--check` with `--fast` is refused: a partial run cannot check the whole suite's record.
- **A `validate` job** in `ci.yml` runs it on `ubuntu-latest`, `macos-latest` and
  `windows-latest`, with no oracle and no network; the Pages deploy waits for it.
- **`scripts/regenerate-references.sh`** runs the chain behind the references the harness reads, in
  order: `rocket_mass.py`, `cargo xtask designs`, `recovery.py`, `flight.py`, then
  `cargo xtask validate --check`. Each generator writes to a temporary file, so a failure leaves
  the committed fixture alone. The report is rewritten only when the check fails: the committed
  one holds macOS digits, and a Linux run would otherwise rewrite it on every run for last-digit
  rounding alone. A fixture that moved at all changes the hash the report records, so the report
  is then rewritten. It is written even when a metric fails, so the diff shows what moved, and the
  script then exits non-zero.
- **The *Regenerate references* workflow** (`regenerate-references.yml`) runs the script on
  `macos-latest`, an arm64 Mac like the one the committed references came from, so that an empty
  diff means something, and only on `workflow_dispatch`. Its token has `contents: read` and the checkout keeps no
  credentials, so it cannot push. It uploads `references.diff` and a summary as an artifact, and
  collects them even when the script fails.
- The script covers the harness's references and the design fixture they are built from. The
  other oracles' fixtures (atmosphere, geodesy, shapes, walls, motors, ThrustCurve) feed unit
  tests, not the harness; they keep their own commands.

**Alternatives.**

- Diffing the report `cargo xtask validate` writes with `git diff --exit-code`: exact bytes fail
  on the platforms that round the last digits differently.
- Only the existing test: it runs the same check, but a reader of CI cannot see the suite ran.
- A workflow that opens a PR or commits: that is L76 automated. A person commits the diff, in a PR
  that says why the reference moved.
- Running the oracles in CI on every PR: RocketPy's environment takes minutes to build, and a
  stored reference is the point of ADR-015.

**Consequences.**

- A change that moves a validation number cannot merge without the report that says so, on any
  of the three OSes.
- The workflow could not run before it was on `main` (GitHub only dispatches workflows the default
  branch has). The script ran locally first, on macOS: in 41 s it reproduced every committed fixture
  and the report byte for byte.
- M2.1c2's predicted-mode reference joins the chain when it lands.

## ADR-023: Predicted mode: each code's own drag, reported against a target (2026-09-18)

**Context.** M2.1 asks for every whole-flight case to run in two modes: same-drag, which M2.1b2
scores, and predicted, in which hpr flies its own aerodynamics. Its done-when asks for
predicted-mode results in the report, "with explained gaps", and for `M ≥ 1` cases to be reported
as gaps until M1.8. The committed whole-flight reference flies a declared constant `C_D0` of 0.5,
so hpr's own drag scored against it would measure hpr's drag against an arbitrary number. The
like-for-like reference is RocketPy flying each example's own drag, whose curves carry their own
terms and stay in the gitignored `refs/` (ADR-009). And neither code's drag is the truth: each
example's came from RASAero, OpenRocket or its team, and M1.5b already measured hpr's drag 47%
below Valetudo's table and 6.0% below Juno III's at Mach 0.3.

**Decision.**

- **A second reference,** `validation/fixtures/flight/rocketpy-whole-flight-own-drag.json`, from
  `flight.py --own-drag`: the same six cases, each with the drag its example flies in RocketPy
  1.13.0. That is the Calisto, Valetudo and Juno III curves read from `refs/rocketpy`, NDRT 2020's
  constant 0.44, Bella Lui's 0.43, and Prometheus 2022's `prometheus_cd_at_ma` (ported from
  RocketPy's MIT test fixtures, with 1.02 times it power-on). "As RocketPy flies it" was checked in
  RocketPy's source: `Rocket.__init__` fixes the drag the flight reads (`power_off_drag_7d`), so
  the Juno III notebook's rescaling and Bella Lui's replacement curve, both applied to the rocket
  afterwards, never reach the flight, and the reference does not apply them either. The fixture
  records each curve's path and SHA-256 and each constant, never a curve's values. It reproduces
  byte for byte, and the same-drag fixture is unchanged by the new flag.
- **A case says its mode:** `mode = "predicted"` under `[flight.whole_flight]` (`DragMode`;
  same-drag is the default). A predicted case flies the design with no drag table. Each mode
  refuses the other's reference: a predicted case against a declared table would score hpr's drag
  against a constant, and a same-drag case against the own-drag reference would fly a table the
  reference never flew. The L75 checks (design, dry mass, reference area, motor) apply to both.
- **Targets, not gates.** Each predicted metric keeps M2.1's 3% as a *target* and gets a verdict of
  `within target` or `outside target` (`Verdict::WithinTarget`, `Verdict::OutsideTarget`, from
  `Comparison::targeted`). Neither counts as scored, neither fails the run, and the rows sit in the
  report's own *Predicted mode* section, apart from the gated table and from *not scored*, the
  harness's escape hatch. Every miss is explained in its case file with its measurement. The
  report is pinned like the rest (ADR-022), so a predicted number that moves still has to be
  committed. A test pins that every predicted row, and no other, is a target row.
- **Prometheus 2022 is a known gap in predicted mode too:** RocketPy on its own drag peaks at
  Mach 1.049, and the harness checks the gap as it does the same-drag one (L85).
- **Predicted mode flies at rtol = atol = 1e-11,** same-drag mode at the default 1e-8. Its
  drag calls `ln` and `powf`, whose last bits differ between platforms' maths libraries, which
  moves the adaptive step sequence, so the answer differs by the solver's global error: at 1e-8,
  CI measured NDRT 2020's predicted apogee 1.7e-7 apart on macOS and Linux (1404.058522 against
  1404.058761 m), past ADR-022's 1e-7 reproduction bound. On macOS that apogee is 1404.058522,
  .057883, .058122 and .058145 m at 1e-8 to 1e-11, so 1e-11 converges it to about 1e-5 m, for
  0.5 s more over the suite. The bound is not loosened.
- **Peaks are found between the solver's steps, not only at them.** At 1e-11, CI still found
  NDRT 2020's predicted max Mach 6.6e-6 apart on macOS and Linux, and its max speed 1.2e-7 apart
  on macOS and Windows. The harness read each peak at the steps' ends, so a peak was off by
  wherever the step control put them, and that differs between platforms. Moving predicted mode's
  tolerance by 1e-7 of itself stands in for that: it moved NDRT's max speed by 2.4e-6 and Bella
  Lui's by 2.7e-6, but the event-located apogee by only 1.3e-9. (Changing the tolerance by one
  part in 10⁷ imitates another platform's step sequence on one machine.) Now, wherever speed, Mach or
  acceleration rises out of a step's start and falls into its end, a golden-section search on the
  step's dense output finds the peak between them. The same perturbation then moves no metric by
  more than 7.9e-9. The step-end reading had been low by up to 6.3e-5 (same-drag NDRT's max
  speed). Every verdict stands, and five of *Accuracy*'s cells moved in the third decimal. The
  bound is not loosened. This departs on purpose from measuring as RocketPy does (Loft lesson L80,
  that a metric must mean what the reference means): RocketPy reads its maxima at its solution's
  points only. hpr's peak now reads at or above its own step-end reading, by at most the 6.3e-5
  above. How much RocketPy's sampling misses depends on its steps and was not measured; its
  maxima move by at most 7.8e-6 between its tight and loose runs. Either way it is far inside 3%.
  The alternative, a platform-dependent number in a report that must reproduce on three
  platforms, is worse. The search applies in both modes, and to the power-on maximum.
  Its limits, from sampling every step at 400 points: a step whose quantity turns more than once,
  or jumps (the skin friction at the critical Reynolds number), is not searched, and none of those
  is a flight's maximum today. The computed acceleration carries about 1e-7 m/s² of numerical
  noise, so a smooth acceleration peak is found to about 1e-8 of itself and its time to about
  1e-4 s. Both are [issue #53](https://github.com/nrdptel/hpr-sim/issues/53).
- The set of predicted rows outside their target is pinned by a test, as the not-scored set is,
  so a case file's "nothing else misses" cannot go stale unnoticed.
- `scripts/regenerate-references.sh` regenerates the new reference with the others, and now
  prints how far each fixture moved, number by number. ADR-022's first dispatched run, on GitHub's
  macOS runner, showed why: RocketPy's fixtures moved in their last digits (the descents by at most
  3.6e-11 relative; in the whole flights, a landing height of 2e-8 m by 3e-9 m), which changed
  their hashes and so the report, with no printed metric moving.

**Alternatives.**

- Scoring predicted mode against the same-drag reference: it measures hpr's drag against 0.5.
- Gating it at 3%: two of five apogees miss by 10%, from drag tables that are not the truth
  either, and hpr's drag runs on placeholder fin edges and finishes where the examples record
  none. A gate would fail the suite on a disagreement nobody can yet settle, or be loosened to
  pass, which rule 2 forbids. M2.1's own done-when asks for predicted-mode results "reported, with
  explained gaps", not passed. `VALIDATION.md`'s initial targets called the 3% "targets, not
  gates, until the first report exists"; since then it is a gate in same-drag mode and, by this
  decision, a target in predicted mode.
- Declaring every predicted metric *not scored*: it would bury the same-drag suite's eleven open
  misses among 75 routine notes, and a target that is met would read like one that is not.
- Committing the examples' curves, or reading them in CI: their terms forbid the first (ADR-009),
  and CI has no RocketPy checkout.

**Consequences.**

- The heights: Calisto −0.527%, Bella Lui +1.118%, Juno III +3.181%, Valetudo +10.007% and NDRT
  2020 +10.232%, each where hpr's drag sits against the example's. Flown on the same drag, all five
  agree within 1.710%.
- A reader can see how hpr's own aerodynamics compare with the drag RocketPy's examples ship, and
  why; nothing says which drag is right until real flights (M2.3).
- When M1.8 lifts the Mach limit, both Prometheus cases fail until their gaps are removed.
- A predicted case's known gap is justified by the reference reaching Mach 1, as a same-drag
  one's is. hpr's own drag could take a rocket past Mach 1 where RocketPy's example stays below
  it; no case does that today (both Prometheus references exceed Mach 1), and such a case would
  have to leave the lock or wait for M1.8.
- A predicted flight past Mach 0.8 would fly hpr's drag beyond the range its build-up is
  documented for, and the harness does not flag it; none does today (Calisto peaks at Mach 0.746).

## ADR-024: The time-series RMS: aligned at ignition, held to 3% of its trace's scale (2026-09-18)

**Context.** M2.1 lists a "time-series RMS after alignment" among its comparisons, and M2.1a to
M2.1c built none. Each whole-flight fixture already carries a `series`: 120 rows of time since
ignition, the height of the centre of dry mass above the ground, and its speed, on a uniform grid
from ignition to RocketPy's impact. Nothing said what "alignment" means or what bound the RMS
answers to. hpr keeps no trajectory after a run, and the report's other rows are point metrics
held to 3% of their own reference.

**Decision.**

- **Two metrics per whole-flight case,** `series_height_rms_m` and `series_speed_rms_m_s`: the
  root mean square of hpr's value less RocketPy's at the reference's own series times, over every
  time at or before hpr's landing. Their reference value is 0, exact agreement. The harness
  requires every whole-flight case to name both.
- **Alignment is the shared clock.** Both codes start their clocks at ignition with the rocket on
  the rail, so the times align as they are. No time shift is fitted: a fitted shift would absorb a
  real difference in the burn or on the rail, which is what the comparison is for. hpr's values
  come from the dense output of the solver step that holds each time, not from interpolating
  between samples, so the comparison adds no error of its own beyond the step's interpolant.
- **The window is while both fly.** RocketPy's series ends at its impact; hpr's comparison stops
  at hpr's landing, the same event its `flight_time_s` measures. A landing-time difference is
  already gated by that metric, so it is not counted twice as a tail of ground-level zeros.
- **Each RMS is held to 3% of its trace's scale:** the height RMS to 3% of the reference's apogee,
  the speed RMS to 3% of its max speed, rounded down to 0.1. That is M2.1's 3% for a point metric,
  applied to a whole trace, so a trace passes only if it is on average no further off than its
  peak may be. The bound was set before the first measurement. The gate test holds each RMS gate
  no looser than that scale, so no absolute floor can widen it. In predicted mode the same bound is
  a target (ADR-023).

**Consequences.**

- Measured, same-drag: height RMS 1.4 to 39.2 m (0.12% to 1.5% of apogee, Juno III the largest)
  and speed RMS 0.13 to 2.06 m/s; all ten RMS rows pass. Both Prometheus 2022 cases name both
  metrics with their bounds but fly nothing: they stay the checked `M ≥ 1` gap until M1.8, so ten
  of the twelve whole-flight cases report an RMS. Predicted: Valetudo's height RMS and NDRT 2020's
  height and speed RMS are outside target, for the drag that puts their apogees 10% high; each is
  explained in its case file and pinned with the other misses.
- The RMS weights every grid time equally, so a long descent weighs more than a short burn. A
  difference in the burn shows in the point metrics (burnout, max speed and acceleration), which
  remain gated. The speed RMS is loose on the descent, where speeds are 5 to 25 m/s against a bound
  set by the top speed (Calisto falling under its main at twice RocketPy's rate would give about
  5 m/s, inside 7.3); the descent rate is gated by `impact_speed_m_s` instead.
- The window ends where the first code lands, so the tail where the heights differ most is not
  counted; the same-drag `flight_time_s` gate covers it, and in predicted mode it is a target.
- The horizontal path is not in the series, so the drifts stay with issue #50 (M2.1d2).

## ADR-025: The calm-air cases, and Juno III's drifts left to the rail release (2026-09-18)

**Context.** In wind, hpr's whole flights turn into the wind less than RocketPy's (issue #50), so
the windy cases report their drifts without scoring them. Issue #50 also flew three of those
rockets once with no wind, to separate the response to wind from everything else. M2.1d2 commits
those calm-air runs as cases, with the drifts scored at M2.1's 3%.

**Decision.**

- **Three calm-air cases:** Juno III, Calisto and Bella Lui, each flown by RocketPy on its windy
  case's rocket, rail, site and declared drag with both wind components zero (`flight.py`'s
  `CALM_AIR_BASES`). Only the same-drag fixture has them: they measure the response to wind, which
  predicted mode would mix with the drag. Regenerating the fixture left every earlier case
  bit-identical.
- **Every metric scored at 3%, drifts included,** with each RMS held to 3% of the calm reference's
  apogee and max speed, as ADR-024 sets. Calisto and Bella Lui pass every scored metric: drifts
  −1.258% to −2.583%, every other within 1.8%. Calisto's `max_acceleration_time_s` is not scored,
  for the reason its windy case gives (two peaks 0.9% apart).
- **Juno III's two drifts are reported, not scored,** because they miss by 3.7%, and a measured
  difference between the codes' rail models accounts for about 1.6 of those points. hpr keeps the
  rocket guided until its last rail button leaves the rail; RocketPy frees it when its first
  button does (`effective_1rl`). Neither models tip-off, the nose dipping as the rocket pivots on
  its last button (hpr's `rail.rs` says so), and tip-off would add drift, so hpr's full guidance
  is the further of the two from a real launch. Juno III's buttons are 1.41 m apart, twice Calisto's, so hpr
  guides it along its 85° rail for 1.41 m more, and its path stays steeper: apogee drift −3.670% and
  landing drift −3.695%, with the apogee within 0.060%. `rail_release.py` flies each calm case in
  RocketPy on a rail longer by its button spacing, so both codes free the rocket at the same
  point:

  | case | spacing | RocketPy drift, apogee / landing | with the longer rail | hpr against it |
  |---|---|---|---|---|
  | Juno III | 1.410 m | 570.1 / 651.3 m | 561.0 / 641.0 m | −2.1% / −2.2% |
  | Calisto | 0.700 m | 453.5 / 515.8 m | 451.9 / 514.0 m | −0.9% / −1.1% |
  | Bella Lui | 0.600 m | 21.5 / 27.7 m | 21.3 / 27.3 m | −0.5% / −1.3% |

  With the release matched, every calm drift is within 2.2%, and Juno III's would pass. The
  release closes 43% of Juno III's gap (9.1 of 20.9 m at apogee), 25% to 28% of Calisto's and 50%
  to 66% of Bella Lui's. What remains is negative in all six drifts, −0.5% to −2.2%: hpr's path is
  steeper for a second reason, not yet named. The case file gives these numbers.

**Alternatives rejected.**

- *Loosening Juno III's drifts to 4%.* That would widen a gate to fit a result, which the hard
  rules forbid, and would hide the cause.
- *Flying the calm references on the longer rail.* hpr reads its rail from the reference, so it
  would fly the longer rail too and still free the rocket later. Matching the release needs a
  change to one code's rail model, which is M2.1d3's first candidate for issue #50.
- *Leaving Juno III failing.* The suite has to be green to merge, and the miss is measured and
  partly explained. It stays visible in the report as a not-scored row with its value.
- *Scoring Juno III's drifts against a release-matched RocketPy drift, at 3%.* That keeps a live
  bound (it would pass at −2.1% and −2.2%), but it needs the release-matched run committed as a
  fixture the harness reads. M2.1d3 takes it up with the rail release.

**Consequences.**

- In calm air, the rail release is a quarter to two thirds of each drift gap, and the rest has
  one sign in every case. In wind, hpr's upwind shift is 67% to 85% of RocketPy's (issue #50):
  Juno III's in-wind gap is 378 m, against about 9 m from the release. So M2.1d3 still looks at
  the response to wind, and at the steeper path that remains in calm air.
- `rail_release.py` is a measurement, not a fixture or a check. If M2.1d3 matches the release,
  Juno III's drifts should be scored again.

## ADR-026: The path in wind: RocketPy's corrected equations, and hpr's body lift (2026-09-18)

**Context.** In wind, hpr's whole flights turned into the wind less than RocketPy 1.13.0's
(issue #50). Juno III's apogee drift was −60.8% of RocketPy's (its landing drift +151%), NDRT
2020's −18.6%, Bella Lui's −15.6% and Calisto's −5.6%. In still air, Valetudo's landing drift was −3.4%, and in calm air
Juno III's drifts were −3.7% (ADR-025). Same-drag mode shares only `C_D0`, so M2.1d3 looked at
everything else that turns a rocket in wind: the rail release, the drag's growth with angle of
attack, the normal force and the damping.

**How it was found.**

- *One candidate at a time.* In a local build with switches (not committed), hpr was flown with
  each candidate matched to RocketPy. Juno III's apogee drift went from −60.8% to −44.1% with body
  lift off, −58.3% with the rail release at the first button, −60.5% with a linear normal force and
  −60.9% with no angle-of-attack drag factor. With all four matched, and Juno III's fin slope
  raised 7.6% to RocketPy's, it was still −31.9%. Something else was at work.
- *The same state in both codes.* RocketPy's states along Juno III's windy flight were fed to
  hpr's equations of motion (a local probe of `Simulation`'s derivative). With hpr's normal force
  made linear like RocketPy's, the two agreed on the lateral acceleration to 1%. But at the rail
  exit, where the rotation rate is zero and no damping of any kind acts, RocketPy's angular
  acceleration was 1.75 times hpr's (0.3608 against 0.2066 rad/s²). After burnout the two agreed.
  With no rotation, both codes' rotational equations reduce to the moment about the centre of mass
  over the inertia there. So they disagreed about where the centre of mass was.

**The cause, in RocketPy.** `Flight.u_dot_generalized`, RocketPy's default 6-DOF equations, is
written about the centre of dry mass (CDM) and needs the vectors *from* it to the centre of mass
(`r_CM`) and to the nozzle exit (`r_NOZ`). It reads `Rocket.com_to_cdm_function` and
`Rocket.nozzle_to_cdm`, which point *to* the CDM: the note in `rocket.py:996-1025` says
`com_to_cdm_function + center_of_mass == center_of_dry_mass_position`. So while the motor burns,
RocketPy takes its moments about a point as far forward of the CDM as the true centre of mass is
behind it. For Juno III at the rail exit that point is 1.315 m from the nose tip, where
RocketPy's own `center_of_mass` puts the centre of mass at 1.639 m. The margin RocketPy flies there
is 1.75 times the one its own `static_margin` reports. After burnout `r_CM` is zero and the
error ends. A rocket that is too stable during the burn turns into the wind too much.

Upstream, an outside contributor reported the sign in issue #1186 (2026-08-25) and proposed the
fix in PR #1196 (open, not yet reviewed, head `927e771e`): three edits that negate `r_CM` with its
derivatives, negate `r_NOZ`, and flip the `r_CM ^ w_dot` term in `v_dot`, which was written for
the reversed vector. PR #1196 builds on PR #1188, merged into `develop` 2026-09-09 and not yet
released, which corrects the nozzle gyration tensor's parallel-axis term from
`0.25 * nozzle_to_cdm**2` to `nozzle_to_cdm**2`, so that the jet damping uses the whole lever.
RocketPy's own `develop` branch agrees on the convention: the tip-off phase merged there in PR #920
(2026-09-14) says, at `flight.py:2086-2087`, "The generalized EOM store r_CM / r_NOZ as (point ->
CDM) vectors, i.e. the negative of the true-frame position; hence the sign flips below", and
negates `com_to_cdm_function` for its own use.

Two checks:

- *RocketPy against itself* (`validation/oracles/rocketpy/wind_response.py` prints it for every
  case). At the rail exit of each windy case, RocketPy's angular acceleration as released equals
  the moment about the mirrored point over the inertia, to four digits. For Juno III it is 0.3608
  rad/s²; about RocketPy's own centre of mass the same moment gives 0.2067.
- *Corrected RocketPy against hpr* (the local probe). With both corrections, at RocketPy's states
  through Juno III's burn with the rocket rotating, its angular acceleration agrees with hpr's
  (normal force linear) to 0.1 to 5%, the larger where the value is small after burnout, and its
  CDM acceleration to 0.01 m/s². So hpr's variable-mass terms and jet damping match RocketPy's
  corrected ones.

**What remains is the two codes' models.** With the corrections, RocketPy's drifts in wind move
by up to 78% (Juno III's landing; 87% on its own drag). hpr against them: Juno III −42.5% (apogee drift) and +40.9%
(landing drift), Bella Lui −11.3% and −23.8%, NDRT 2020 −4.65% and +1.95%, Calisto −0.99% and
+1.43%, Valetudo −0.93% and −1.95%. In calm air every drift agrees within 2.2%. `wind_response.py`
then adds hpr's choices to the corrected RocketPy one at a time. The drifts, in metres:

| case, drift | RocketPy as released | corrected | + release at the last button | + body lift | + thin fins | hpr |
|---|---|---|---|---|---|---|
| Juno III, apogee | 582.4 | 396.6 | 360.7 | 270.6 | 231.1 | 228.0 |
| Juno III, landing | 268.3 | 478.5 | 519.6 | 624.0 | 670.2 | 674.4 |
| Bella Lui, apogee | 123.9 | 117.9 | 110.9 | 104.7 | | 104.6 |
| Bella Lui, landing | 74.6 | 67.2 | 58.9 | 51.7 | | 51.2 |
| NDRT 2020, apogee | 69.6 | 59.4 | 58.1 | 57.1 | | 56.6 |
| NDRT 2020, landing | 341.0 | 347.5 | 348.3 | 354.4 | | 354.2 |
| Calisto, apogee | 447.1 | 426.4 | 424.3 | 422.2 | | 422.2 |
| Calisto, landing | 1195.5 | 1261.5 | 1265.7 | 1278.3 | | 1279.6 |
| Valetudo, apogee | 167.7 | 165.2 | 163.6 | 163.6 | | 163.7 |
| Valetudo, landing | 206.4 | 203.3 | 201.4 | 199.5 | | 199.4 |

With hpr's three choices, RocketPy lands within 1.4% of hpr in every windy case. The local build
agrees from the other side: hpr with its normal force linear, no body lift, the first-button
release, no drag factor and Juno III's fin slope matched is within 0.7% of the corrected RocketPy
in every windy case (Juno III 396.4 against 396.6 m). The three choices:

- **Body lift** (ADR-008): `C_N = K (A_plan/A_ref) sin² α`, `K = 1.1` (Galejs), acting at each
  body component's planform centroid. It is zero at small angles, but a slow rocket leaves the rail
  at a large one: Juno III at 18 m/s in an 8.5 m/s wind, its rail leaning 5° downwind, 26° off the
  airflow, where body lift is about half its normal force. Much of it acts ahead of the loaded
  rocket's centre of mass, the nose's above all (its planform centroid is 0.35 m from the tip, the
  centre of mass 1.64 m), so at a steep angle it moves the centre of pressure forward by about
  0.3 m and weakens the moment that turns the rocket into the wind. It turns the rocket more than
  it pushes it: in RocketPy with hpr's release and fins, Juno III's apogee drift is 328.0 m with no
  body lift and 231.1 m with it, 232.6 m with the nose's alone, and 311.2 m with all of it placed
  at the centre of mass, where it can only push (Bella Lui: 110.9, 104.7, 108.2 and 110.0 m).
  RocketPy's normal force is linear in `α` and has no body term.
- **The rail release** (ADR-025): hpr guides the rocket until its last button leaves the rail;
  RocketPy frees it at the first. Neither models tip-off, the pivot about the last button between
  the two, so the real release lies between them: each is a modelling choice.
- **Juno III's fins:** the example gives them an airfoil lift curve, which RocketPy uses in place
  of the thin-plate `2π`, and which hpr does not model; RocketPy's fin slope is 7.6% steeper.

The drag's growth with the angle of attack (hpr's, up to 1.3 times at 17°) moves no drift by more
than 0.1%, and hpr's `sin α` in place of `α` moves none in wind by more than 0.9% (local
build).

**Decision.**

- **The oracle flies RocketPy 1.13.0 with PRs #1188 and #1196 applied** (`corrections.py`). The
  corrected `u_dot_generalized` is RocketPy's own source recompiled with #1196's three edits, each
  required to match exactly once, so a RocketPy that has moved stops the script instead of flying
  something else. The fixtures record both corrections (`corrections`), and their `model` names
  them. Both whole-flight references were regenerated; the mass and descent fixtures are
  bit-identical. RocketPy's apogees move by at most 1.6% (Prometheus 2022; Juno III +1.0%).
- **A drift is gated at 3% unless a measurement shows why it misses.** Six drifts reported before
  are now gated, and pass: Calisto's two in wind, Valetudo's and NDRT 2020's landing drifts, and
  Juno III's two in calm air (superseding ADR-025's decision on them). Five stay reported but not
  scored, each as a measured model difference: Juno III's and Bella Lui's two, and NDRT 2020's
  apogee drift. Prometheus 2022's drifts, excused before, are held to 3% for when hpr flies it.
- **hpr keeps its body lift and its rail release.** Body lift is a real force at these angles, and
  OpenRocket carries it too. The release at the last button is hpr's modelling choice (ADR-025);
  with no tip-off in either code, neither end is the real one.
- **Drop the corrections when RocketPy releases them.** Then re-pin the oracle and delete
  `corrections.py`, or keep only what the release still lacks.

**Alternatives rejected.**

- *Keeping RocketPy as released and the drifts unscored.* The reference would carry an error in
  its equations of motion that its own `static_margin` contradicts, that RocketPy's `develop`
  branch describes in a comment (PR #920), and that PR #1188 (merged) and PR #1196 (open)
  correct. hpr's correct answer would read as a miss:
  Juno III's apogee, +1.71% as released, is +0.70% corrected.
- *Only #1196.* It is written on top of #1188, which is merged upstream.
- *RocketPy's `develop` branch with #1196.* Unreleased, and it would move every reference for
  reasons unrelated to this one.
- *Gating the five drifts now against RocketPy flying hpr's body lift, release and fin slope.* It
  would check hpr's equations at steep angles as same-drag mode checks them with the drag shared,
  but not hpr's normal force against another's, and it needs a reference whose rail differs from
  the one hpr flies, which the harness does not read yet. Until then the five are pinned as the
  committed report pins every number (`validate --check` fails if one moves) and the regeneration
  script prints `wind_response.py` beside them. Filed as issue #61.
- *Dropping or shrinking hpr's body lift to agree.* That fits hpr to a code that leaves a real
  force out. `K` is uncertain (Galejs gives 1.0 to 1.5) but it is not zero.
- *Loosening the drifts' gate.* That would widen a gate to fit a result, which the hard rules
  forbid.

**Consequences.**

- Issue #50 closes. M2.1's landing offset is met in calm air, in still air (Valetudo), for
  Calisto in wind and for NDRT 2020's landing. It is not met for Juno III and Bella Lui in wind,
  where the two codes' models differ: body lift and the rail release by design, Juno III's airfoil
  fins a feature hpr lacks. The report shows both numbers.
- Whether `K` should be lower at these rockets' crossflow Reynolds numbers is open: crossflow
  methods other than Galejs's (Allen and Perkins; Jorgensen, NASA TR R-474) are not yet read.
- In wind, a slow rocket's drift in hpr depends on body lift's uncertain `K`. Flown in RocketPy
  with hpr's body lift, rail release and fin slope (`wind_response.py`), Juno III's apogee drift is
  240.2 m at `K = 1.0`, 231.1 m at 1.1 and 194.1 m at 1.5, and 328.0 m with no body lift; its
  landing drift is 659.5, 670.2 and 714.2 m. hpr itself gives 237, 228, 191 and 326 m (local
  build). Calisto, off the rail at 28 m/s and 11°, changes its drifts by under 0.5% across
  `K = 1.0` to 1.5. Which code is nearer a real flight is for M2.3.
- The corrections do not only move RocketPy toward hpr: Valetudo's apogee difference grew from
  +0.003% to +0.116%, and predicted NDRT 2020's apogee drift from −4.44% to +11.906% (outside its
  target before and after).
- The time-series RMS: Juno III's height RMS is 15.9 m, from 39.2, and Bella Lui's 1.9 m, from
  2.3; four others grow by under a metre (Valetudo 1.4 to 2.4 m, NDRT 2020 2.0 to 2.8 m, and
  Juno III's and Calisto's calm cases 1.3 and 1.1 to 2.1 and 2.0 m). Burnout speed now reads
  +0.018% to +0.063% in every case, where it read −0.066% to +0.040%: a small rest of one sign,
  far inside the gate.
- Predicted mode: 66 of 85 metrics within target, from 63; Calisto's drifts and Juno III's apogee
  are now within, and nothing new misses.
- `rail_release.py` now flies the corrected equations. The numbers ADR-025 quotes from it were
  measured before this correction; `wind_response.py` gives the current ones.

## ADR-027: The normal force through Mach 1: supersonic linear theory, a transonic join, and the measured references (2026-09-18)

**Context.** M1.8 is too big for one session and is split into M1.8a to M1.8d (`ROADMAP.md`).
M1.8a carries the normal force and centre of pressure through Mach 1, so that a flight on a drag
table can pass it; the drag buildup's transonic terms are M1.8b. Until now `hpr-aero` refused
`M ≥ 1` (ADR-008) and a flight stopped there (ADR-011), which left Prometheus 2022, the suite's
one supersonic rocket, a known gap in both modes. Loft lesson L7: Loft's fin slope had no
compressibility factor, so its slope and CP never moved with Mach.

Three findings shaped the method:

- *Niskanen's supersonic fin slope counts one surface.* [N09] eq. 3.48–3.49 write the fin's
  normal force as its area times one strip's pressure coefficient, `K₁α + K₂α² + K₃α³` with
  `K₁ = 2/β`. That is the pressure on one face. A flat plate carries the difference between its
  faces, `2(K₁α + K₃α³)` (the `K₂` terms cancel), whose first term is Ackeret's `4α/β`.
  Barrowman's own method sums every surface region ([B67] appendix A, pp. 82–86). Niskanen's
  thesis finds its simulated `C_Nα` for the Arcas Robin "notably lower than the experimental
  values", with the reason unknown (p. 91, over a comparison that runs to Mach 4). hpr uses both
  faces.
- *Barrowman's half-load in the tip's Mach cone is exact linear theory for a rectangle*, in lift
  and in CP: the exact cone load `(2/π) asin √t` averages one half, and [N09] eq. 3.35 (from
  Fleeman, *Tactical Missile Design*, p. 33) is the rectangle's exact CP. For tapered fins the
  strip method runs above exact linear theory: against [TN2114] eq. A7 (printed p. 18), a fin
  with a taper ratio of 0.5, an unswept trailing edge and `βA = 3` gets 4.5% more slope (worked
  by hand here; the research found +2.1% and +0.8% at `βA = 4` and 6).
- *No source gives the transonic region in closed form.* MIL-HDBK-762 reads it from McDevitt's
  transonic-similarity charts for rectangular fins (pp. 5-104–5-105), and [B67] §1.00 makes "no
  attempt" at it. RocketPy 1.13.0 flies Diederich's subsonic slope at every Mach number, with
  `β` held at 0.6 from Mach 0.8 to 1.1 (`aero_surface.py:51-56`); supersonic, that tends to
  `2π cos Γ_c/β`, about π/2 times linear theory's `4/β`. Its fin CP doesn't move with Mach.

**Decision.**

- **Supersonic fins: linear theory on the fin's outline** (`FinOutline::supersonic`). The load is
  `4α/β` per unit area, halved inside the Mach cone from the tip's leading edge, with the root a
  reflection plane (a cone that crosses the root counts its part over the mirror image). So
  `(C_Nα)₁ = (4/β)(A_fin − A_cone/2)/A_ref`, and the CP is the load's centroid. The outline is the
  planform's polygon (an ellipse as a 256-gon, 2.5e-5 short in area), clipped against the Mach
  line by Sutherland–Hodgman and summed by the shoelace formula without allocating. Only the
  first-order term: `K₃α²` adds 1% at 0.1 rad at Mach 2, but 35% at Mach 1.2, where the expansion
  itself fails.
- **Where linear theory starts:** `M_s = max(1.2, 1/cos Γ_L, 1/cos Γ_T, √(1 + 1/A²),
  √(1 + (c_t/2s)²))`: the bottom of [N09]'s supersonic region (Table 3.1), supersonic leading and
  trailing edges ([TN2114]'s case), `βA ≥ 1` with `A = 2s²/A_fin`, and the mirror fin's tip cone
  off this fin's tip, `β ≥ c_t/(2s)`. The tip-chord term came from review: `βA ≥ 1` alone let an
  inverse-tapered fin's two tip cones overlap its tip, and its slope rose past `M_s`. The
  trailing-edge term is TN 2114's stated case, supersonic trailing edges. Neither binds for any
  validated fin, so no reported number moved. Leading edges swept forward stay outside the
  half-load's domain: the tip sits ahead of the root, and the slope rises past `M_s` by up to
  +7.7% at 40° to 50° forward (issue #64).
- **The transonic join:** from Mach 0.8, the top of [N09]'s subsonic region and of the drag
  buildup's documented range, to `M_s`, slope and CP each linear in `M` between the subsonic
  method's values at 0.8 and linear theory's at `M_s`. Continuous, with the slope's peak at `M_s`.
  Below 0.8 nothing changes: Diederich's slope with Prandtl–Glauert at the quarter chord.
- **Bodies don't change with Mach** (slender-body theory, [B67] p. 18), and `K_T(B)` stays
  Barrowman's. `K_B(T)`, the fins' lift carried onto the body, stays out, as below Mach 1.
- **Ranges.** `AeroError::Mach` gains the refusing model's limit. The normal force covers
  `0 ≤ M < 5` (`NORMAL_FORCE_MACH_LIMIT`; [N09]'s hypersonic region starts at about 5); the drag
  buildup `0 ≤ M < 1` (`BUILDUP_MACH_LIMIT`) until M1.8b; a drag table any Mach number.
- **In flight.** A fin set's CP moves with Mach, so `AeroModel::component_station_m` takes the
  Mach number, and the flight takes each component's local airflow at its CP for the centre of
  mass's Mach number, each step.

**The references.**

- *RASAero II's Calisto export* (`refs/rocketpy-history/calisto-cd-test-2018.csv`, RocketPy's first
  commit, now pinned in the lock). Its `CNalpha (0 to 4 deg)` is the secant slope to 4° and, from
  Mach 0.95, includes a viscous cross-flow term that is zero below. So the comparison takes its
  `CN Potential` at 2° over the angle and its CP at 0°, against hpr's small-angle slope and CP. A
  code, not a measurement. The choice decides much of the result: against its 4° columns, 1 of
  the 11 rows from Mach 0.8 is within the targets, not 6, and Mach 2 is −30.6%. Below Mach 0.8
  the agreement is partly by construction, since ADR-009 gave the Calisto design the 2018 fins
  because they reproduce this export at low speed. The fixture commits the export's values at
  the 15 Mach numbers compared (30 values), more than ADR-009's one per curve; STATUS asks Neer
  whether that is fine.
- *NASA's half-scale Arcas wind-tunnel models* (TN D-4013, Mach 0.6–1.2; TN D-4014, Mach
  1.5–4.63): the Arcas Robin (short, 18.2 calibers) and the long bioscience version (23.8). The
  reports print no tables, so their plots were read on 600-dpi renders, each plot's grid
  calibrated locally, into `validation/fixtures/aero/arcas-robin-wind-tunnel.json` with every
  point's figure and page. The CP read back from C_m and C_N reproduces TN D-4014 Fig. 7 within
  1.2% of body length (0.0% at Mach 2.96). The reports plot `C_N` against `α`, so hpr is fitted
  the same way: its `C_N` at the plotted angles, a least-squares slope; its CP from its moment and
  normal force over −2° to 2°. `cargo xtask designs` builds both models from the recorded
  geometry. The nose is a coordinate table, not a tangent ogive; the design's power-series nose
  (`n = 0.6369`) has its volume, which sets the slender-body CP, and its planform within 2.4%.

**Result** (`validation/fixtures/aero/normal-force-vs-mach.json`, pinned by
`hpr_aero::tests::normal_force_against_mach`; 37 rows, 16 outside the targets). The targets were
written into `ROADMAP.md` before the comparison first ran, and landed in the same commit as it;
they have not moved since.

| reference, band | `C_Nα`, hpr against the reference | CP, calibers | within both targets |
|---|---|---|---|
| Arcas Robin, short, Mach 1.5–2.96 | −13.4% to +3.3% | −0.02 to +0.42 | 4 of 4 |
| Arcas, long, Mach 1.8–2.96 | −8.2% to +0.5% | −0.12 to −0.04 | 3 of 3 |
| Arcas Robin, short, Mach 3.96–4.63 | −19.6%, −25.0% | −0.17, −0.16 | 0 of 2 |
| Arcas, long, Mach 3.96–4.63 | −17.2%, −22.8% | −0.16, −0.19 | 0 of 2 |
| Arcas, both, Mach 0.6 | −2.4%, +3.6% | +0.06, −0.44 | 2 of 2 |
| Arcas, both, Mach 0.8–1.2 | −6.9% to +29.3% | −0.46 to +2.29 | 2 of 9 |
| Calisto RASAero II, Mach 0.1–0.7 | +0.1% to +10.1% | −0.08 to +0.43 | 4 of 4 |
| Calisto RASAero II, Mach 0.8–2.0 | −16.8% to +21.9% | −0.56 to +0.95 | 6 of 11 |

Every miss, measured:

- *Past Mach 3: the body.* Fins on less fins off, the fins' share agrees with hpr's fins within
  −1.4% to +7.0% at Mach 3.96 and 4.63. The body alone lifts 3.9 to 4.6 per radian there, fitted
  the same way, against hpr's 2.3 to 2.8 with its body lift. Slender-body theory's nose (2 per
  radian) and boattail (−1.15 here) don't change with Mach, and the real body's lift grows. The
  CP stays within 0.19 calibers, so the stability margin holds; the slope, and so the weathercock
  rate, is low. M1.8e takes this on. The fins' agreement is uncertain by about its own size: the
  design leaves out the strip where each fin's root follows the boattail below the cylinder, about
  0.32 in² of 5.8 in² (5.5%).
- *Mach 0.6 passes by cancelling errors:* hpr's body is 40% (short) and 34% (long) above the
  fins-off readings and its fins' share 9.3% and 3.9% below the measured one.
- *Transonic, Mach 0.8 to 1.2.* Fins on less fins off, the measured fin lift falls from Mach 0.6
  to 0.9 (9.5, 8.5, 7.8 per radian on the short model) and jumps at 1.0 (14.2); hpr's rises by
  Prandtl–Glauert and then along the join to linear theory's peak at `M_s = 1.2` (16.6 against
  13.0 measured there). The long model's CP jumps forward at Mach 1.0 (71.5% of its length
  against 75.6% at 0.8), 2.29 calibers from hpr's. These are the flows no closed-form method
  covers. The transonic body readings are poorly determined (±0.02 per point on a coarse grid).
- *RASAero II.* Its subsonic slope and CP stay constant to Mach 0.9; hpr's rise by Prandtl–Glauert,
  14.4% by Mach 0.8, and its fins' CP starts moving aft at 0.8, so the CP misses from Mach 0.8 to
  0.95 by 0.52 to 0.95 calibers. The wind tunnel sides with neither: at Mach 0.8 the short model's
  `C_Nα` is +11.9% in hpr against the measurement, and the long model's +12.0%. At Mach 1.3 hpr is
  just past its join's peak (+12.7%, CP +0.65); at Mach 2, −16.8% with the CP 0.56 calibers forward.
  Calisto has no fins-off data, so that miss can't be split into fins and body; the wind
  tunnel's gap at the same speeds is the body's.

Prometheus 2022, the suite's supersonic rocket, flies through Mach 1.010 on its declared drag:
14 metrics scored and passing, the largest its apogee at +1.525%. Its drifts are −9.273% (apogee)
and +6.283% (landing). `wind_response.py`, which now flies it, puts RocketPy with hpr's rail
release and body lift at 1484.3 m and 1468.5 m, within 0.1% of hpr's 1483.1 m and 1469.4 m. So
they are ADR-026's body-lift difference and are reported, not scored, as ADR-026's are. Near
Mach 1 it flies almost straight into the airflow (its angle of attack, sampled at hpr's steps by a
local probe of the harness, stays below 0.11° from Mach 0.8 to 1.2), so the transonic normal force
has little to act on. Its apogee, +1.525%, is body lift too: RocketPy with hpr's body lift and rail
release reaches 3735.4 m, against hpr's 3735.3. On its own drag it stays a known gap until M1.8b.
No other case passes Mach 0.8, so no other number in the report moved.

**Rejected.**

- *Tuning the join or `M_s` to the wind tunnel.* The misses would shrink by fitting the model to
  its own check. `M_s` and the join's start come from the sources' regions, set before measuring.
- *Niskanen's one-face slope and his quintic CP fit from Mach 0.5.* The first is half of linear
  theory; the second would move every subsonic flight's CP from Mach 0.5 with no measurement
  asking for it (the Arcas Robin's CP moves forward, not aft, from Mach 0.6 to 0.8).
- *RocketPy's Diederich slope past Mach 1.* About π/2 times linear theory.
- *The third-order Busemann terms.* See Decision.

**Consequences.**

- Flights on a drag table fly to Mach 5. On hpr's own drag they still stop at Mach 1 until M1.8b.
- M1.8e: the body's supersonic normal force, against the Arcas Robin's fins-off measurements.
- M1.8c's roll forcing and damping will use the same outline and its Mach cone; TN D-4014 Fig. 14
  (roll effectiveness per degree of cant, recorded in the digitization but not yet committed)
  and TN 2114's roll-damping formulas are its references.
- `AeroModel::component_station_m` and `FinSetAero` changed shape (`fin: FinAero`, `fore_station_m`,
  `cp_station_m(mach)`); nothing outside the workspace uses them yet.

## ADR-028: Drag through Mach 1: Niskanen's appendix B, Stoney's curves, and the Arcas Robin's axial force (2026-09-18)

**Context.** M1.8b is too big for one session and is split into M1.8b1 (the buildup through
Mach 1, against the Arcas Robin wind tunnel, predicted Prometheus flying) and M1.8b2 (drag against
RocketPy's RASAero curves through Mach 2) (`ROADMAP.md`). Until now the buildup held nose and
shoulder pressure drag at eq. 3.86's value at rest, which ADR-009 expected to read low from about
Mach 0.6, and refused `M ≥ 1` (ADR-009); predicted Prometheus 2022 was the harness's one known
gap (ADR-021, ADR-023).
Every other term already had its supersonic branch: friction (eq. 3.83–3.84), base drag
(`0.25/M`, eq. 3.94), the fins' edges (eq. 3.89–3.93) and the stagnation pressure (eq. B.1).
Loft lesson L17: Loft froze the fin leading-edge drag at its Mach 1 value and gave nose drag no
Mach term.

What the sources give ([N09] §3.4.3, pp. 47–48, and appendix B, pp. 106–110; the OpenRocket
technical documentation 13.05 reprints them word for word):

- Eq. 3.87 carries a nose's pressure drag from eq. 3.86 at rest to "the lower bound of the
  transonic method" as `a Mᵇ + C₀`, "non-decreasing" with zero slope at rest; `a` and `b` fit the
  value and slope there. The thesis gives no closed form for them and doesn't say what to do
  where they can't fit.
- Cones have closed forms (eq. B.3–B.6, after Hoerner), from Mach 1: `sin ε` and slope
  `4/(γ + 1)(1 − sin ε/2)` at Mach 1, eq. B.4 from "M ≳ 1.3", and "polynomial interpolation"
  between. Ogives are the cone times `0.72(κ − ½)² + 0.82` (eq. B.8, after NAVWEPS 1488 p. 239).
- Elliptical, power, parabolic and Haack noses take Stoney's measured curves at fineness 3 (NASA
  TR R-100, 1961, Figure 12), "written into the software as data curve points", scaled to other
  fineness ratios by eq. B.9 through a flat face at fineness 0. The thesis prints no values; Stoney
  prints only the plots.
- Shoulders are treated "similar to nose cones", "somewhat dubious at supersonic velocities"
  (p. 48), without saying what their fineness is.

The measured reference is NASA's half-scale Arcas Robin (TN D-4013, Mach 0.6–1.2; TN D-4014,
Mach 1.5–4.63), already the normal force's (ADR-027). Both reports take the base apart, because
the models sat on a sting: TN D-4013 plots `C_A,corr`, "corrected for base axial force" to the
free stream's pressure over the full base (its Fig. 3 base pressure over Fig. 11's `C_A,b` gives
0.43 of the reference area, the 1.470-in base's 0.427); TN D-4014 plots `C_A` with the balance
chamber's force in it and `C_A,c` apart, uncorrected (printed p. 4), and states no chamber area.
[N09] Fig. 6.6 compares OpenRocket with the same data and finds its drag about 80% high by Mach
3.96, blaming the boattail, the airfoil fins and "less reliable results" at higher speeds.

**Decision.**

- **Every nose, shoulder and step: eq. 3.86 at rest, eq. 3.87 to `M_L`, appendix B from `M_L`**
  (`hpr_aero::nose_drag::PressureDragCurve`, precomputed per component when the model is built).
  `b = C_T′(M_L) M_L/Δ` and `a = Δ/M_Lᵇ` with `Δ = C_T(M_L) − C₀`, evaluated as `Δ (M/M_L)ᵇ`,
  which stays within `[0, Δ]`: in review, `a` overflowed where `Δ` is tiny and `b` huge (an x^0.868
  nose at 3:1 gave NaN below Mach 0.8), and a regression test holds it. The power is used only
  where it meets Niskanen's conditions, a rise with `b > 1` (flat at rest); see the fallback below.
- **Cones and ogives** as printed from fineness 1, with a cubic Hermite between Mach 1 and 1.3 and
  `M_L` = 1 (the thesis's "interpolated using equation (3.86)" between 0 and 1 in B.2 can only mean
  eq. 3.87). The ogive's `κ` is the reciprocal of hpr's radius ratio. **A bulged secant ogive**
  (radius ratio below 1, `κ > 1`) is outside eq. B.8 and **refused by the buildup** when it is
  evaluated: the model still builds (after review), so the normal force and a drag table work.
- **Below fineness 1, cones and ogives scale by eq. B.9's form** between a flat face at fineness
  0 and their own whole curve at fineness 1, `C₀ (C₁/C₀)^(ln(f + 1)/ln 2)`, at every Mach number.
  Eq.
  B.4 runs past a flat face's drag as a cone flattens (2.39 at Mach 2 as `f → 0`, against 1.41),
  and with it a shrinking shoulder no longer tended to a step (Loft lesson L15's test failed:
  0.801 against 0.842 at Mach 0.3). Continuous at fineness 1 and at 0 (the step), so L15 holds
  exactly.
- **Stoney's curves, digitized and committed as data** (`StoneyNose`, in the source: core crates
  do no I/O). Read from a 600-dpi render of Figure 12 (printed p. 16), each panel's grid fitted
  line by line (skew up to 6 px in panel (b)), at the line's centre, checked on overlays: about
  ±0.0015, up to ±0.005 on the steepest rises. Panel (a), the flight models, for the seven shapes
  it has (x^½, x^¾, the ½, ¾ and full parabolas, L-V Haack, von Kármán), from Mach 0.8 to each
  line's end at 1.94–1.99; panel (b), the wind tunnel of Stoney's ref. 30, for the x^¼ and the
  ellipsoid, which begin at Mach 1.2, to 3.59. Past the last point the value is held; panel (b)
  puts the hold within 8% for von Kármán (to Mach 3.59) and x^¾ (to 3.2), and panel (a)'s x^½ is
  still rising at its end. After review, the x^¼ and the ellipsoid are joined by a straight line
  to 0 at Mach 0.8, where every smooth 3:1 nose of panel (a) reads 0: before, their `M_L` of 1.2
  stretched eq. 3.87 to 1.2, which made a power series' drag jump at `n = ½` (0.0511 against 0 at
  Mach 0.9) and gave elliptical noses subsonic drag the data doesn't show. Stoney's
  configuration key (Fig. 9) numbers the parabolas 59 full, 62 three-quarter, 57 half. Where (a)
  and (b) overlap, (b)'s von Kármán reads 0.004–0.011 higher from Mach 1.2; (a) is used because it
  is Stoney's own data and covers the shapes together. A NASA report is a U.S. Government work.
- **Between measured shapes, linear in the parameter at fineness 3, then eq. B.9**: a power series
  through the flat face (`n = 0`), x^¼, x^½, x^¾ and the 3:1 cone (`n = 1`); a parabolic series
  through the 3:1 cone (`K′ = 0`) and the three parabolas; a Haack series between von Kármán and
  L-V Haack. **A Haack series past `C = ⅓` is refused** by the buildup in the same way, as [N09]
  limits it (p. 103). `M_L` is 0.8, [N09]'s start of the semi-empirical method (p. 47), for every
  measured shape. The ends of the power and parabolic series are B.9's 3:1 cone, not the cone of
  the nose's own fineness, so a power series of exponent 1 and a cone of the same fineness agree
  only at fineness 3 (review, advisory; the structure is [N09]'s).
- **Where eq. 3.87 can't fit** (`Δ ≤ 0`, or `b ≤ 1`), `C₀ + Δ (M/M_L)²`: continuous, flat at
  rest, a kink at `M_L`. Falling, where a joint that isn't smooth meets a measured curve still at 0
  at Mach 0.8, it follows Stoney's measurement rather than [N09]'s "non-decreasing" (review,
  advisory; coefficients below 0.01). Rising with `b ≤ 1`, it serves near-flat noses, which eq.
  3.86 gives almost nothing at rest: x^0.05 at 3:1 rises to 0.80 at Mach 0.8, flat at rest where
  the power rose to 0.29 by Mach 0.1 (review).
- **A shoulder's fineness is `l/(d_aft − d_fore)`**, a nose's `l/d` when `d_fore = 0`: the cone of
  the same surface angle. A clipped transition takes its shape as if unclipped. A widening too
  small to change the diameter as computed is no shoulder (review). **A step** (and a body's bare
  front face) is a flat face, the blunt cylinder's `0.85 q_stag/q` at every Mach number (eq. B.2),
  0.85 at rest: eq. 3.86 "does not take into account the effect of extremely blunt nose cones
  (length less than half of the diameter)" (p. 47), and a step has no length (review). Before, it
  was 0.8 at every speed.
- **The buildup covers `0 ≤ M < 5`** (`BUILDUP_MACH_LIMIT`), like the normal force.
  `Drag::beyond_subsonic_methods` is removed: nothing read it, and the flag no longer marked an
  error of hpr's own.
- **The known gap** (ADR-021) is now a refusal of a Mach number at or past the normal force's or
  the drag buildup's top, both Mach 5, which the reference must reach too; no case declares one.
  The logic moved into `run::settle`, which checks the reference against the refusing model's own
  limit, so its paths stay tested without a flight that reaches Mach 5.
- **The Arcas Robin comparison: forebody drag, hpr's `C_D0` less its base drag, against `C_A,corr`
  (TN D-4013) and `C_A − 1.383 C_A,c` (TN D-4014)**, fins at 0° and off, at every Mach number the
  reports give, at their 3.0 × 10⁶ per foot. 1.383 = (1.470/1.250)², the base over the chamber's
  1.250-in cavity in TN D-4014 Fig. 1(a), so the chamber's pressure acts over the whole base as TN
  D-4013's correction assumes; `C_A − C_A,c` is committed beside it, 0.002 to 0.015 higher, and
  changes no row's verdict; a test checks both against `C_A` and `C_A,c`.
  Digitized as the normal force was (ADR-027), each plot's grid mapped for skew and shear; TN
  D-4014's `C_A` at zero angle is a quadratic through the symbols within 2.6°. Checking TN
  D-4013's earlier readings found Fig. 11's scan sheared (up to −0.0045 at Mach 1.2) and its
  base-force labels swapped at Mach 0.6; both are corrected in the committed readings. For drag,
  the committed designs take hpr's airfoil section for the double-wedge fins (Niskanen's choice,
  p. 90; square edges give about 3.7 times the fin drag at Mach 0.6 and 1.45 times at Mach 1.5)
  and a polished finish, 0.5 µm, for machined steel (the reports state none; at 20 µm the friction
  would be about 26% higher). Both choices moved hpr toward the tunnel, and they were made in the
  same commit as the measurement, so the audit recomputed the count for each: 3 of 44 within 10%
  with square edges and 20 µm, 5 with the airfoil section alone, 6 with the finish alone, 8 with
  both (`drag_against_mach_depends_on_the_fins_and_finish`). The airfoil section follows the
  drawings; the finish is a guess. Target, set before measuring: M1.8's 10%. `cargo xtask aero` writes `validation/fixtures/aero/drag-vs-mach.json`
  with the drag by part; `hpr_aero::tests::drag_against_mach` recomputes it and pins the rows
  within target.

**Result.**

- L17's test and the model's own tests pass: a 3:1 cone by hand (0.0216 at rest, 0.1644 at
  Mach 1, 0.1042 at Mach 2), the joins smooth, eq. B.9 through its anchors, a shoulder tending to
  the step, Stoney's curves reproduced, and a property test over every shape to Mach 5.
- **Against the Arcas Robin, 8 of 44 rows within 10%** (fixture), 3 of them within their reading
  uncertainty and the reports' ±0.004 of the 10% edge, so 5 to 8. Fins off: +28% to +49% from Mach
  0.6 to 0.9, −9.2% to +18.4% from 0.95 to 1.8, +20.5% to +71.1% from 2.3. Fins on: +31% to +47%
  subsonic, −10.9% to +11.3% from 0.95 to 1.2, +30% to +191% from 1.5. The drag by part explains
  it:
  - *The fins past Mach 1.2*: the rounded leading-edge formula (eq. 3.89) holds hpr's fin drag near
    0.30 from Mach 1.5, where the measured fins-on less fins-off falls from 0.153 to 0.046 (+78% to
    +551%). A thin, sharp fin's wave drag is far smaller and falls with Mach; nothing in hpr models
    it. This is Niskanen's own Fig. 6.6 miss.
  - *The model's 1.3-mm reflexed lip*, taken as a shoulder in the free stream (fineness 0.33),
    worth 0.065 to 0.086. It sits in the boattail's wake. Without it, fins off, the short model's
    forebody is within −28% to +14% at every Mach number (−1.2% at 2.96) and the long model's
    within −15% to +23%. The short model also keeps raised fin-root fairings with its fins off,
    which hpr leaves out and TN D-4013 (pp. 4–5) blames for its higher drag from Mach 0.975 to 1.2.
  - *The boattail rule* (eq. 3.88) gives the 15° boattail 0.063 at Mach 0.6, where the measured
    fins-off forebody, 0.22, leaves little above hpr's friction of 0.19: the rule over-predicts
    this boattail, as [N09] found against the same tunnel (p. 90). It is most of the subsonic
    excess, with the lip. (A first draft called it bookkeeping; the audit and the physics review
    showed the tunnel's forebody holds exactly the pressure the rule models.)
- Niskanen's 3:1 cone against Stoney's measured one, high through the whole rise: +87% at Mach
  0.8, +105% at 0.85, +49% at 1.0, +48% at 1.1 (0.234 against 0.158), +15% at 1.5 and +4% at 1.94.
  Ogives inherit it. So stubby cones and ogives gain the most at high subsonic speeds. The whole
  rocket's `C_D0` at sea level against the buildup before M1.8b1 (measured in review; "rest" held
  the nose at eq. 3.86):

  | rocket (nose) | Mach 0.6 | 0.8 | 0.9 | 0.95 |
  |---|---|---|---|---|
  | Bella Lui (1.55:1 tangent ogive) | +6.5% | +22.7% | +37.6% | +47% |
  | NDRT 2020 | +0.4% | +5.6% | +16.0% | +26% |
  | Valetudo | +0.1% | +2.3% | +7.7% | +13% |
  | Calisto, Prometheus (von Kármán) | 0 | 0 | 0 | +0.3% to +0.6% |

  An earlier draft said the held value "read low" by what eq. 3.87 now adds; that measured the old
  model against the new one, and Stoney's data say the opposite for cones (review).
- **Predicted Prometheus 2022 flies**, to Mach 1.059 against RocketPy's 1.048: 17 metrics, 9
  within target; apogee −6.985%. hpr's coasting `C_D0` rises to about 0.49 at Mach 0.8 and 0.54 at
  Mach 1, mostly base drag, where the example's falls to 0.30; under power hpr relieves the base by
  the motor's area. The misses are explained in the case file and pinned.
- The predicted flights barely move, as none spends long above Mach 0.6 on a stubby nose: Bella Lui's
  `C_D0` at Mach 0.3 from 0.423 to 0.424 and its predicted apogee from +1.018% to +1.004%; NDRT
  2020's from +10.322% to +10.306%; of the Mach 0.3 comparison with RocketPy's curves (ADR-009),
  only Valetudo's `C_D0`, by 4e-7.

**Consequences.**

- Supersonic drag with fins reads high, the more so the thinner and sharper the fins, until a fin
  wave-drag model replaces the blunt leading edge for sharp sections. M1.8b2 measures it against
  RASAero II through Mach 2 and decides. It should also weigh Stoney's measured 3:1 cone (digitized
  with the other curves) against Niskanen's closed form for cones and ogives through Mach 1.2.
- The buildup has no transonic or supersonic check of its base drag (the tunnel's base is the
  sting's), and shoulders past Mach 1 rest on [N09]'s own doubt.
- ADR-009's held nose drag and `M ≥ 1` refusal, and ADR-021's `M ≥ 1` gap, are superseded here.

## ADR-029: Drag against RASAero II through Mach 2: the gap by band, MIL-HDBK-762's sample calculation, and the boattail's wave drag (2026-09-18)

**Context.** M1.8's drag bullet asks for `C_D` within 10% of RocketPy's RASAero curves from Mach
0.1 to 2.0 for the available rockets, with the errors by band in the report; M1.8b2 carries it
(ADR-028). ADR-009 compared the same curves at Mach 0.3 only, with one declared rule for the
inputs the curves don't record (fin section, thickness, finish). What the curves are:

- **Calisto**: a real RASAero II export (RocketPy's first commit, 2018, so version 1.0.1.0 or
  earlier), to Mach 2. The only curve traceable to a RASAero II run.
- **Juno III**: 3 decimals, hand-edited: from Mach 0.93 to 1.0 it climbs a constant 0.072 per
  0.01, then drops to 0.001.
- **Cavour**: stops at Mach 0.895 (power-off) and 0.923 (power-on).
- **Valetudo**: to Mach 1.53, 1.44 times its own rocket's OpenRocket export at Mach 0.3 (ADR-009).

RASAero II's Users Manual (1.0.2.0, 2019) cites no drag method. Its drag breakdown lists terms
Niskanen's buildup doesn't have: fin–body interference, fin base drag, and "other body wave" drag
(pp. 90, 92). Its exports take the Reynolds number at sea level (p. 84), a smooth finish by default
(p. 53), and laminar flow up to a transition at `5 × 10⁵` unless "All Turbulent" is set (p. 55).
It names eight fin sections (p. 14) and no default among them. No RASAero input file (`.CDX1`) for
any of the four rockets is public: RocketPy's history on every branch, its companion repositories,
the two teams' repositories and a GitHub search found none. So the inputs stay unknown, and the
comparison is between two codes, one of whose inputs are guessed.

**Decision.**

- **The comparison.** `cargo xtask aero` compares hpr's `C_D0` with each curve every 0.05 from
  Mach 0.1 to 2.0, wherever the curve reaches without extrapolation. Each point uses USSA76 sea
  level's Reynolds number for its Mach number, as RASAero II computes its exports. ADR-009's rule
  for the inputs is unchanged, and Juno III's curve is used to Mach 0.92. Bands are Niskanen's
  Table 3.1: subsonic to 0.8, transonic below 1.2, supersonic from 1.2.
  `validation/fixtures/aero/rocketpy-drag-curves.json` records hpr's value and the error at each
  Mach number, and each band's count within 10%, least and greatest error, and RMS. It doesn't
  record the curves, which carry their own terms (ADR-009), but hpr's value and the error at full
  precision give the curve back exactly at each sampled Mach number: 147 values of five curves.
  That is more than ADR-009's one value per curve, so the Needs Neer entry on RASAero values now
  covers it. The report for this comparison, as for M1.8a's and M1.8b1's, is the fixtures and the
  *Aerodynamics* and *Accuracy* pages; `validation/reports/latest.md` holds whole flights only.
- **Loft lesson L18's test measures and pins** instead of asserting agreement. It is renamed
  `hpr_aero::drag::tests::supersonic_cd_against_rasaero_tables`. It recomputes every row from the
  committed designs, checks every verdict and band summary, and pins each band's count within
  10%. The lesson named it `supersonic_cd_within_tolerance_of_rasaero_tables`; that assertion
  doesn't hold, and a changed assertion needs a decision record (`docs/research/loft-lessons.md`).
- **A second reference, with every input known: MIL-HDBK-762's sample drag calculation.** This is
  Table 5-4 (printed pp. 5-58 to 5-66) for the rocket of Fig. 5-155 (pp. 5-223 to 5-224): a
  3-calibre tangent ogive on a 21-calibre cylinder 0.16 m across, and four fins flush with the
  base. The table gives each term from Mach 0.5 to 3.2 at a flight's Reynolds numbers: friction,
  the nose's wave drag, the fins' wave and trailing-edge base drag, and the body's jet-off base
  drag. It is a calculation by the handbook's methods, not a measurement; its base drag comes
  from measured bases. The handbook is a U.S. Government work. Its values are transcribed with
  their pages into `validation/fixtures/aero/mil-hdbk-762-sample-drag.json`, checked against the
  rendered pages, with each row summing to its printed total. `cargo xtask designs` builds the
  rocket with a smooth finish (the handbook's flat-plate friction).
  - **The fins are left out.** Fig. 5-155 draws each fin as a single wedge, sharp at the leading
    edge and blunt at the trailing edge, and the calculation gives them a double wedge's wave drag
    and base drag on the trailing edge. hpr has no such section (#70). The design takes square
    edges, and the fins' pressure drag is recorded on both sides but compared on neither: a first
    draft compared the totals and read hpr high faster than sound, which was the square leading
    edge's 0.06 to 0.10 (review).
  - `cargo xtask aero` compares it term by term in `validation/fixtures/aero/drag-vs-mach.json`,
    and `hpr_aero::tests::drag_against_mil_hdbk_762_sample` pins it. The target is M1.8's 10%,
    not set blind: hpr's numbers were first computed in a scratch run while choosing references.
- **M1.8's drag bullet is not met, and the gap stays in the report.** Closing it by tuning would
  move hpr away from the references whose inputs are known. The candidates for its cause become a
  new increment, M1.8b3, for the afterbody (a boattail's supersonic wave drag and the base), with
  targets on the Arcas Robin's measured forebody and on Calisto's supersonic band, and issues for
  the rest.

**Result.**

- **By band, rows within 10%** (fixture):

  | case | subsonic (to 0.8) | transonic | supersonic (from 1.2) |
  |---|---|---|---|
  | Calisto, 2018 fins | 15 of 15, +3.9% to +8.9% | 2 of 7, −31.4% to +3.8% | 0 of 17, −29.8% to −24.4% |
  | Calisto, getting-started fins (variant) | 12 of 15, −8.2% to +30.7% | 4 of 7, −7.8% to +48.5% | 6 of 17, −1.8% to +21.6% |
  | Juno III (to 0.9) | 15 of 15, −6.2% to +9.1% | 0 of 2, +14.1% to +21.9% | — |
  | Cavour, power-off (to 0.85) | 6 of 15, −12.6% to −2.2% | 0 of 1, −12.6% | — |
  | Cavour, power-on (to 0.9) | 1 of 15, −26.2% to −9.2% | 0 of 2, −27.0% to −26.7% | — |
  | Valetudo, power-off (to 1.5) | 0 of 15, −51.4% to −43.5% | 0 of 7, −53.3% to −50.9% | 0 of 7, −54.6% to −52.9% |
  | Valetudo, power-on (to 1.5) | 0 of 15, −55.6% to −46.6% | 0 of 7, −57.6% to −54.7% | 0 of 7, −57.8% to −56.2% |

- **No plausible input closes Calisto's gap**
  (`hpr_aero::tests::calistos_supersonic_gap_survives_every_plausible_fin_and_finish`). Over
  square, rounded and airfoil fins, 2 to 6.35 mm thick, smooth or painted (20 µm), no combination
  has rows within 10% in both the subsonic and the supersonic band, and none passes 3 of 7
  transonic. Supersonic rows come within 10% only with square fins 4.76 mm or thicker (all 17 at
  6.35 mm painted), and those put every subsonic row 14.6% to 44.9% high. Every other combination
  stays 10.1% to 37.6% low supersonic. So the supersonic gap is a difference between the two
  codes' models, not the inputs.
- **Against MIL-HDBK-762's sample calculation, fins left out, hpr's body reads high through
  Mach 1 and low faster than sound.** 6 of 12 within 10%: −10.3% at Mach 0.5 and −1.1% at 0.7;
  +20.1% to +31.9% from 0.9 to 1.1 and +12.3% at 1.2; −6.0% to −9.6% from 1.6 to 3.2. By term
  (handbook against hpr):
  - Nose, Niskanen's ogive: 0.052 against 0.164 at Mach 1.0, and 0.109 against 0.234 at 1.1.
    The handbook's transonic ogive curve (Fig. 5-113) and Stoney's measured 3:1 cone (ADR-028)
    both sit well under it (#67). From Mach 2 the two agree within 10%.
  - Base, Fleeman's formula (eq. 3.94): 0.183 against 0.250 at Mach 1.0, but 0.147 against 0.125
    at Mach 2. Above Mach 1.2 the handbook's base pressure follows Love's correlation of measured
    bases (NACA TN 3819, Fig. 5-139) (#68).
  - Friction, 10.4% to 15.9% lower in hpr: its body form factor is 1.02, the handbook's 1.15,
    which accounts for about 11 points; the rest, unexplained, may be the two methods'
    compressibility corrections.
  So faster than sound hpr's body reads 6% to 10% low against a method with every input known:
  the same sign as Calisto's gap, a third of its size. NASA's Arcas Robin wind tunnel, with the
  fins off and the base left out, reads hpr high, most of it a lip at the model's base (ADR-028).
- **A candidate for the rest: the boattail's supersonic wave drag.** Calisto's boattail is short
  and steep, 0.472 calibres long, down to `(d_b/d)² = 0.469` (18.4°). hpr's boattail rule (eq.
  3.88) only scales base drag, and gives it 0.083 at Mach 1.2, 0.066 at 1.5 and 0.050 at 2.0.
  MIL-HDBK-762's chart for conical boattails at supersonic speeds (Fig. 5-122, p. 5-187, read for
  this decision) gives 0.338 ± 0.008 at Mach 1.2 and 0.215 ± 0.007 at 1.5; at Mach 2 its
  parameter `√(M² − 1)/(2 l/d)` is 1.83, past the chart's end at 1.4 (Mach 1.66 here), and
  extrapolating gives about 0.13. Against hpr's whole gap of 0.20, 0.156 and 0.128, the chart's
  extra 0.255, 0.149 and about 0.08 would overshoot at Mach 1.2, match at 1.5 and fall short at 2.
  RASAero II lists such drag as its own term, "other body wave" drag (p. 92). Against it:
  - The Mach trends differ: the gap falls by 1.6 times from Mach 1.2 to 2, the chart's extra
    over hpr's rule by 3.2 times.
  - The handbook advises boattails under 8° to avoid flow separation (p. 5-12); a separated
    18.4° boattail sees about the base's pressure, which is roughly what hpr's rule gives.
  - A boattail raises its base pressure (Fig. 5-141, only at Mach 2.5 to 3.5), which would offset
    part of the term; that is not subtracted here.
  - The one measured boattail argues against the whole term. The Arcas Robin's 15° boattail is in
    the tunnel's forebody. With the fins off the short model reads +8.1% at Mach 1.5, where hpr's
    boattail is 0.069; with the chart's 0.198 in its place it would read about +46%, or +21% with
    the whole lip removed. At Mach 2.96, with the lip removed, hpr's rule already agrees (−1.2%),
    where the chart's 0.072 would put it about +17% high.
  So the chart's term is a candidate, not a finding: the measured boattail wants more pressure
  drag than hpr's rule at Mach 1.5, less than the chart's, and about the rule's at Mach 2.96.
  M1.8b3 measures it before hpr changes.

**Consequences.**

- **M1.8b3, the supersonic afterbody**, is next:
  - The conical boattail's wave drag (Fig. 5-122 or the linear theory behind it), weighed against
    the Arcas Robin's measured boattail.
  - The base pressure behind a boattail, and the base drag faster than sound (#68).
  - The Arcas Robin's lip, which hpr takes as a shoulder in the free stream though it sits in the
    boattail's wake (ADR-028).
  - Its targets, set before measuring, are on the Arcas Robin's fins-off forebody and Calisto's
    supersonic band (`ROADMAP.md`).
- **Issues, outside M1.8b3:**
  - #67: Niskanen's transonic cone and ogive drag, against Stoney's measured cone and the
    handbook's ogive.
  - #68: Fleeman's base drag against Love's correlation.
  - #69: fin–body interference drag, which RASAero II has and Niskanen's buildup neglects (p. 41).
  - #70: a sharp double-wedge fin section, for the Arcas Robin's fins supersonic (ADR-028).
- **The Needs Neer entry on RASAero values is widened** to the drag sweep's 147 recoverable values.
  If Neer objects, the sweep keeps its band summaries and drops its rows' errors.

## ADR-030: The afterbody faster than sound: a boattail's wave drag, the base behind it, and a lip in its wake (2026-09-18)

**Context.** ADR-029 left Calisto's drag −29.8% to −24.4% under RASAero II from Mach 1.2 and
named a boattail's supersonic wave drag as a candidate, with the Arcas Robin's measured 15°
boattail arguing against taking MIL-HDBK-762's chart whole. M1.8b3's targets, set before
measuring (`ROADMAP.md`): the Arcas Robin's 11 fins-off rows from Mach 1.5 within 10%, the 2
already within staying; Calisto's 17 supersonic rows within 10%; each row's change reported.
hpr's boattail was Niskanen's rule (eq. 3.88) at every speed: a share of the base drag on the area
removed, 0 for boattails longer than three times their drop in diameter.

What the sources give (all NACA and NASA reports are U.S. Government works; pinned in
`validation/refs.lock.toml`):

- **Fig. 5-122's source.** MIL-HDBK-762 cites none. Jack, NACA TN 2972 (1953), computed conical
  boattails by Van Dyke's second-order theory at Mach 1.5 to 4.5, 3° to 11°, area ratio 0.2 to
  0.8. The chart's axes are the small-disturbance similarity variables, and it agrees with Jack's
  points within −10.4% to +8.0% for area ratios up to 0.6 (83 points inside it); first-order
  theory reads far higher. So the chart is second-order theory, not measurement.
- **Measured boattails, jet off, turbulent**: Cortright and Schroeder (RM E51F26, Mach 1.91,
  5.6° to 9.3°, boattail drag and base pressure; "the method of characteristics overestimated the
  side pressure drag by about 18 to 20 percent", p. 17), de Moraes and Nowitzky (RM L54C16, Mach
  1.59, 5° and 10°), Compton (TN D-6789, 3°, 5° and 10° at Mach 0.3 to 2.20, the points to 1.3
  from his NASA TM X-1960 in another tunnel; those near Mach 1 his report calls "questionable",
  p. 9), Moskowitz and Jack (RM E54B11, Mach 3.12, 7.1°), Love (TN 3819, Kurzweg's base pressures
  at Mach 3.24, 2.5° to 15°), and Cubbage (RM L57B21, Mach 0.6 to 1.28, 5.6° to 45°, a boundary
  layer 0.20 d thick). Cubbage's boattails stay attached at 16° and "between boattail angles of
  16° and 30°, the external flow separates completely" (p. 8); a separated boattail's pressure is
  "approximately equal to the pressure measured at the base of a cylindrical model" (p. 6).
  Measured transonic boattail drag is half-way up its rise by about Mach 0.89 (Compton's 10°) to
  0.92–0.96 (Cubbage's), peaks at Mach 1.0 to 1.1, and at 1.2 is 0.83 to 0.90 of that peak
  (Cubbage).
- **The base behind a boattail**: MIL-HDBK-762 Fig. 5-141 (p. 5-210, after Rubin, Brazzel and
  Henderson 1970, Mach 2.5 to 3.5), `p_cyl/p_bt = 0.442 + 0.558 a_b`, with Love's cylinder
  correlation (Fig. 5-139, p. 5-208). Used as a pressure ratio at Mach 1.59 and 1.91 it
  over-predicts the measured relief.
- **Through Mach 1** the handbook finds no method and advises holding the supersonic value to a
  peak between Mach 1.0 and 1.2, "with a sharp reduction to a lower value at subsonic speeds"
  (p. 5-47).
- **The lip**: TN D-4014 (p. 6) finds the chamber force low at Mach 1.50 and 1.80 "particularly for
  the fin-off condition. This is believed to be because of the reflex lip at the model base; when
  separation occurs over the afterbody boattail at the higher Mach numbers or the boundary layer
  is thickened by the addition of the fins, the effect of the reflex lip is masked." RASAero II's
  own comparison with the tunnel (Rogers 2022, slide 2) left the lip out, "assumed that this small
  lip was buried in the boattail boundary layer". hpr took it as a shoulder in the free stream,
  0.085.

**Decision** (`hpr_aero::afterbody`, and `hpr_aero::drag::couple_afterbody` for the coupling):

- **A boattail's supersonic wave drag is Fig. 5-122**, digitized (eight curves at 20 abscissae,
  ±(0.005 + 2%)), log-log in `x` and linear in the area ratio, to 0 at `a = 1` as `(1 − √a)²`,
  **held to the 2D Prandtl–Meyer limit** `−C_p,PM(M, θ)(1 − a)` (NACA Report 1135, eq. 44 and
  171c), which a round boattail can't exceed and the chart's small-angle law can near Mach 1.
  Past the chart's end (`x = 1.4`) its share of that limit closes on 1 as `1/x`, the form of the
  quasi-cylinder solution's recovery; against Jack's 28 points there, −2.7% to +8.0%. Curved
  boattails are taken as the cone through their ends.
- **Separation**: from 16° to 30° a straight-line blend in the half-angle from the attached value
  to the base drag coefficient on the annulus (Cubbage).
- **Through Mach 1**: the rule to Mach 0.8, where the buildup's other transonic terms start
  (Niskanen p. 47); a straight line to Mach 1; from Mach 1 the attached drag, held at its Mach 1.2
  value to Mach 1.2 (the handbook's advice), blended with the separated value at the Mach number
  itself, so a boattail steep enough to separate completely drags like the step it tends to
  (physics review). History: a first draft joined the rule at Mach 0.8 to the chart's near-sonic
  value at Mach 1, which read the Arcas Robin +85% to +117% there; the next draft held the Mach
  1.2 value and moved the start to Mach 0.9, a choice made right after seeing that target. The
  validation audit caught it, and Compton's transonic data, transcribed then, put the measured
  rise's half-way at about 0.89, earlier than the 0.95 a start at 0.9 gives. The start went back
  to the buildup's own 0.8, half-way at 0.9. It costs the Arcas Robin 2 of its 4 rows within 10%.
- **A boattail in parts** (physics review, three rounds): a narrowing part right after another
  drags as a blend, by a merge weight, of its own drag as a boattail and its share of the boattail
  it continues: the cone from that boattail's start through its aft end less the cone through its
  fore end (held at 0 until the fourth round, below). The weight is 1 for a turn of up to 3° between the parts, 0 from 10°
  (a corner), linear between, and faded by what the parts between have left of that boattail
  (below). So parts of one straight cone add up to one cone, a corner keeps each part its own
  boattail, and a pair drags between the two. It applies at every speed, so below Mach 0.8 a
  curved boattail in parts drags as the cones through its ends rather than part by part as eq.
  3.88 would (physics review: a 6°, 9°, 12° boattail in three 20 mm parts, 0.00716 part by part,
  0 merged, 2.8% of `C_D0` at Mach 0.5). (Superseded in the third round, below:) A part shallower than 1° merges only in proportion to
  its angle. The first draft left each part its own boattail (the same cone drawn as two transitions
  read +10% at Mach 1.5 on the Arcas Robin); the second merged any adjacent parts into one cone
  (a 15° boattail closed by a near-vertical transition read +13% at Mach 0.6 against a step
  down); the third blended the cones' geometry, which gave a gentler second part negative drag.
  The 3° and 10° are a judgement.
- **The base behind a boattail** (the last body component a boattail, or a lip in its wake):
  from Mach 2.5, Fig. 5-141 with Love's cylinder, taken as the ratio of the two pressure
  coefficients and applied to hpr's own base drag (Fleeman's, unchanged, #68); below Mach 2.5
  that ratio at Mach 2.5, chosen because it matches the measured bases at Mach 1.59 and 1.91,
  which therefore check it in sample; back to 1 from Mach 1 to 0.8; toward 1 with the separation
  weight.
- **A lip in a boattail's wake**: a lip behind a boattail, drawn as a shoulder, a step up or both,
  in one part or several, loses its pressure drag while its top rises up to a quarter of the
  boattail's drop in diameter above the boattail's end, keeps all of it from half, and a
  straight-line share between; a lip in parts takes, at each part, the smallest share any top so
  far leaves, a step up by its fore radius and a shoulder by its aft radius too (the physics
  review found one fraction for both moved the drag 25% when a nanometre of tube split them), and
  the base behind it takes the same share of the relief, less the lip's own length's fade. The quarter and half are a judgement made knowing the Arcas Robin's lip, the one
  measured, rises 0.17, and all 44 Arcas Robin rows depend on it (with the lip as a shoulder in
  undisturbed air each would read 0.065 to 0.086 higher). The first draft gave any shoulder up to
  the boattail's fore diameter no drag, and the second required an exact match of radii; the
  physics review showed both switching abruptly (a flare back to full diameter got none; a
  micrometre of step or tube moved the Arcas Robin 25%).
- **Gaps and steps are continuous** (physics and code reviews, three rounds): the flow behind the
  boattails is shared among their tails, the surfaces it may still follow, and each tail holds its
  share faded over one fall (its drop in diameter) by what follows it: a tube's, a lip's or a
  part's length, a step's or a narrowing part's drop in diameter, and a lip's rise. A narrowing
  part moves to a continuation of each tail the share it merges with (the turn's weight, times,
  from the third round, the smaller half-angle over the larger, the larger at most 1°, so a part
  narrowing by nothing is a tube), fades what
  it leaves as a step and a tube, and takes what no tail then holds as its own boattail; a step
  down does the same as a boattail of no length, fully separated, so a closure drawn ever shorter
  is a step. The base and each lip add the tails' holds, at most 1, and tails in the same state
  are one, so there are at most as many as pairs of parts. A change of `ε` in any radius or length
  changes the drag in proportion to `ε`
  (`drag::tests::a_part_narrowing_by_nothing_is_a_tube_and_one_of_no_length_a_step`, behind
  boattails of 2° to 14°; `a_partial_merge_shares_the_flow`; `a_zigzag_boattail_keeps_its_tails_few`;
  `a_sharp_corner_keeps_its_boattails_apart`, `a_lip_in_a_boattails_wake_fades_with_its_rise`,
  `a_lip_drawn_as_a_step_up_is_a_lip`, `a_hairline_step_before_a_lip_changes_nothing`,
  `soft_merges_stay_between_their_limits`). Between Mach 0.8 and 1.2 a part of no length still
  drags its own boattail's straight-line rise where a step drags the base drag's curve, as before.
  The rules the reviews rejected, in turn: one tail that each narrowing part replaced (a part
  narrowing by one ulp after the Arcas Robin's boattail read +15%, a nanometre's widening over
  100 mm −5.7%, a 1 µm closure +4%); several tails and the strongest taken (a partial merge halved
  a lip's wake, the tails grew exponentially on a zigzag boattail, 90 ms a drag call at 26 parts,
  and a part narrowing by `ε` behind a shallow boattail merged into it, −7.5%). With the lip's
  1.3 mm now a length, the Arcas Robin's base keeps 0.944 of its relief; its forebody rows, which
  leave the base out, don't move. A straight cone drawn in parts behind a boattail it partly
  merges with can drag a little differently from the one cone, the old tail's unmerged share
  getting a second chance at a later part: an 8° cone behind a 14° part reads the same in 2 or 4
  parts, up to −0.45% of `C_D0` in 8 and −2.24% in 512, worst at Mach 1.0 (physics review's Mach
  scan). Fourth round: a part's share is no longer held at 0 where the chart makes the longer cone
  drag less than the shorter, so extending a boattail can lower its drag and a part's pressure
  drag can be below 0; merged wholly, the shares add up to the whole cone's drag exactly, which
  is not below 0, and a pair of parts stays between its two limits. Held at 0, they had read a
  7° boattail from 98 mm to 44 mm in 4 parts 1.35% high at Mach 1.0, and a 5° one closing to an
  eighth of its diameter 5% high in 2 at Mach 1.3 (`a_straight_cone_in_parts_is_one_cone`). Third
  round (physics and code reviews): the 1° factor
  now takes the smaller angle over the larger (the larger at most 1°), so a straight cone under
  1° merges wholly (the product of the two angles over 1° had put a 0.8° cone in 8 parts 3.3%
  high); a step down's corner now shelters a lip behind a plain step with no boattail ahead, as a
  closure drawn ever shorter already did: a 98 mm airframe stepping down to a 54 mm motor tube
  showing for 12 mm, then a 62 mm retainer, reads 12% to 23% lower in `C_D0` from Mach 0.3 to 2.5
  (15% at 0.3, 12% at 0.95, 23% at 2.5), unmeasured; the retainer's step keeps 12/44 of its drag
  (`a_retainer_behind_a_step_down_is_in_its_wake`), and none of the shelter is left once the
  exposed motor tube is as long as the step's drop in diameter; and fades now add where steps multiplied
  (behind a 10° boattail, a 1 mm step and a 2 mm tube leave the base 0.433 of its relief, was
  0.513).
- **Fig. 5-141 is power-off**, as is Fleeman's base drag it scales; under power hpr applies both to
  what the motors leave of the base. Love's value held past Mach 5.5 would ask for less than a
  vacuum, and is clamped there.
- **Checks**: `validation/fixtures/aero/measured-boattails.json` transcribes 193 boattail drags
  (Compton's whole Fig. 12, 152 of them), 12 base pressures and Jack's 151 points, with figure,
  page, reading uncertainty and the points a report calls questionable; the readings were checked
  a second time against the scans, which corrected seven base pressures and Compton's first
  supersonic readings (taken from the theory symbols). `cargo xtask aero` compares them in
  `drag-vs-mach.json`, and `hpr_aero::tests::boattails_against_measurements` pins every group.
- **Tests that pinned the old model**, re-pinned with the change recorded in each:
  `drag_against_mach` (2 rows of 44 within 10%, was 8), `supersonic_cd_against_rasaero_tables`,
  `drag_against_mach_depends_on_the_fins_and_finish` (0, 0, 0, 2, was 3, 5, 6, 8), and
  `calistos_supersonic_gap_survives_every_plausible_fin_and_finish`, whose assertion no longer
  holds, renamed `calistos_rows_by_fin_and_finish` to pin every combination's rows by band and
  the error ranges of the committed and the best inputs (physics review).

**Result: M1.8b3's targets are not met.**

- **Measured boattails** (fixture; "in sample" marks rows that helped build the model):
  - Attached, 3° to 10°, from Mach 1.2 to 3.12: −21.9% to +28.3%, within 0.0123 in drag
    coefficient (58 rows of 20 boattails); the largest percentages are 3° and 5° boattails whose
    drag is 0.01 to 0.02. Inviscid theory reads such boattails up to about 20% high.
  - From Mach 1.0 to 1.1, Cubbage's 5.6° and 8°: −18.2% to −5.4% (4, under the peak; partly in
    sample, as his peak was weighed in holding the Mach 1.2 value). Compton's
    questionable points from Mach 0.95 to 1.1: −46.2% to +60.0% (27). Through the rise, Mach 0.85
    to 0.95: −77.5% to +7.6% (28). Under the rule, to Mach 0.8: −100% to −83.5% (58, #73).
  - Cubbage's 16° in his thick boundary layer: +26.4% to +54.2% from Mach 1.0 (9), −30.2% to
    +60.4% below (6). His separated 30° and 45°: −2.8% to +6.6% (3, in sample: they set the
    separation angles).
  - The base drag behind 5° to 10° boattails at Mach 1.59 and 1.91 (8, in sample): within 0.0102
    of the measured on the cylinder's area, though behind small bases that is up to about 40% of
    the base's own drag, and the correlation ignores the angle (Cortright and Schroeder's relief
    grows from 5.6° to 9.3° at one area ratio). Behind Kurzweg's at Mach 3.24 (4): within 0.003.
- **The Arcas Robin, fins off, from Mach 1.5: 0 of 11 within 10%**, +13.5% to +24.1% (was +8.1%
  to +71.1%, 2 within). RMS 19.1% (was 41.9%). The 2 within before were within because the lip's
  0.085 made up for the missing wave drag. If the rest of hpr's forebody were right, the tunnel's
  boattail would drag about 0.12 at Mach 1.5 against hpr's 0.196: the same over-prediction as
  Cubbage's 16°, larger, for a 15° boattail whose flow NASA reports separating at the higher Mach
  numbers. All 44 rows now read high, 2 within 10% (was 8). Fins off, row for row: at Mach 0.6
  and 0.8, +12.0% to +22.8% (was +40.7% to +49.1%); at 0.9 and 0.95, +38.8% to +54.1% (was +18.3%
  to +43.2%); from 1.0 to 1.2, +20.3% to +50.8% (was −9.2% to +17.2%).
- **Calisto against RASAero II: 8 of 17 supersonic rows within 10%**, −14.9% to −5.1% (was 0,
  −29.8% to −24.4%); transonic 3 of 7, −10.1% to +16.4% (was 2); subsonic unchanged, 15 of 15.
  The inputs the export doesn't record span most of the supersonic rest: rounded fins 4.76 mm
  thick, smooth, have 15, 4 and 14 rows within 10% by band, airfoil fins 6.35 mm thick 11, 4 and
  17, and no combination every row. The committed design keeps ADR-009's rule; picking the inputs
  that fit would be tuning. Calisto's 18.4° boattail is steeper than any attached boattail
  measured, so this agreement is no support for the model at that angle. The getting-started
  variant moves from 6 of 17 supersonic to 0, 23% to 32% high.
- **Why not tuned.** The one correction the evidence asks for, less wave drag for steep boattails
  in a thick boundary layer, has no cited method among these sources, and fitting it to the
  Arcas Robin would make the target its own calibration. It stays an open gap (#72).
- **Whole flights don't move**: no boattailed rocket in the suite passes Mach 0.8, and
  `cargo xtask validate --check` reproduces the committed report.

**Consequences.**

- The drag of a supersonic rocket with a boattail rises: Calisto's `C_D0` at Mach 1.5 from 0.443
  to 0.542. A steep boattail in a thick boundary layer reads high, and a gentle one reads low
  through Mach 1; the guide says so.
- Issues: #72, steep boattails in a thick boundary layer; #73, the subsonic rule against Cubbage
  and Compton. #68 (Fleeman's base drag against Love's) is unchanged: the relief multiplies
  Fleeman's value.
- M1.8b's done-when is carried: M1.8's drag bullet recorded not met here and in ADR-029, the
  Arcas Robin compared, predicted Prometheus flying.

## ADR-031: Roll from canted fins and roll damping by Barrowman's strip theory (2026-09-19)

**Context.** M1.8c's done-when (`ROADMAP.md`): M1.8's roll bullet, "roll-rate steady state
matches the analytic cant/damping balance", and hpr's roll forcing compared with the Arcas Robin's
measured roll effectiveness (TN D-4014 Fig. 14). No target was set for the comparison. Until now
the design carried a cant (`FinSet::cant_rad`, used by mass properties only) and the flight
integrated the roll rate with no aerodynamic moment about the axis. Sources, all U.S. Government
works except Niskanen's thesis: Barrowman 1967 (NASA/TM-2001-209983) §3.13–3.14, §3.33, §3.42,
appendix A, Figs. 5-6 and 5-7; Niskanen 2009 §3.3; TN D-4014 Fig. 14.

**Decision.**

- **Strip theory, Barrowman's.** One fin's forcing is its normal force at its mean aerodynamic
  chord, `C_lδ = (C_Nα)₁ (r_t + y_MAC)/d` (eq. 3-35); its damping sums strips at the incidence
  `−pξ/V` (eq. 3-40–3-49), `C_lp = −2a ∫ξ² dA/(A_ref d²)`. Faster than sound both take M1.8a's
  load, `4α/β` halved in the tip's Mach cone (appendix A, first order), and between Mach 0.8 and
  `M_s` each is a straight line in `M`, as the fin's slope is (ADR-027). The integrals come from the
  fin's polygon, so any outline works.
- **The fin's own slope in the damping,** `a = (C_Nα)₁ A_ref/A_fin`. Barrowman's text (eq. 3-40)
  and Niskanen's (eq. 3.69) write the airfoil's `C_Nα0 = 2π/β`, but Barrowman's computed curve for
  the Basic Finner, −34.21 at Mach 0.07 read from Fig. 5-7, is the fin's slope spread over the
  strips (hpr: −33.53, −2.0%); the airfoil's gives about −81, and would climb without bound toward
  Mach 1 where his curve rises about 20%. That point chose the method, so it is not a validation. Stubby fins lift far less than an airfoil section.
- **The body's interference,** Barrowman's `k_T(B)` (eq. 3-95, 3-105) on the forcing and `k_R(B)`
  (eq. 3-122, 3-123) on the damping, from slender-body theory; Niskanen leaves both out. `k_R(B)`
  is for a chord falling linearly from root to tip; another outline takes it at its tip-to-root
  chord ratio (an elliptical fin as a triangle: 5.5% too much damping at `τ` of 2 and 2.9, by
  integrating eq. 3-121 over the ellipse). Both are held constant below `τ = 1.001`, where their
  terms cancel to rounding, and past `τ = 10⁶`, where they overflow. `k_T(B)` is checked only
  against its limits and its own transcription; no independent tabulation was at hand.
- **Fin–fin interference is not applied to roll** (Niskanen eq. 3.66 uses `N`).
- **Physics review's caveats, recorded, not changed:** `k_R(B)` is a force ratio applied to a
  moment (weighted by the moment it is 3.4% to 4.2% smaller here); `k_T(B)` is reference 23's
  factor for fins turned together, not derived for cant's antisymmetric load; appendix A takes the
  supersonic strip moment about the root, where hpr takes it about the axis as eq. 3-35 does
  (about half the forcing otherwise); uniform strips likely overstate a short fin's subsonic
  damping. hpr refuses a cant beyond 15° (stall) and cant on a single fin (its side force isn't
  carried).
- **Signs.** `C_l` is about `+z_B`; a positive cant turns fin 0's leading edge toward `−y_B`
  (`hpr_design`'s convention, `docs/physics/mass.md`), so `C_l0 = −N C_lδ k_T(B) δ`.
- **In flight** the moment is `q A d (C_l0 cos α + C_lp p d/2V)` at the centre of mass's Mach
  number, the damping written `ρ V A d² C_lp p/4`. The `cos α` is a judgement (code review): the
  cant meets the air as it runs along the axis, so a rocket falling tail first spins the other
  way and one broadside isn't driven, as ADR-011 has the fins' normal force follow `sin α`. The
  terms that don't change with Mach are built once per fin set (`FinRollTerms`). Pitch and yaw damping stay the local-flow damping
  (ADR-011); `ROADMAP.md` and `STATUS.md` had cited ADR-026 for it.
- **The references committed:** TN D-4014 Fig. 14's readings (made in M1.8a at `α` = −4°, 0°,
  +4°) into `arcas-robin-wind-tunnel.json`, and the Basic Finner's geometry and Fig. 5-7 read on a
  300-dpi render into `basic-finner-roll-damping.json` (five wind-tunnel points, ±0.3; the
  computed curve at Mach 0.07). `cargo xtask aero` writes `roll-vs-mach.json`, and
  `hpr_aero::tests::roll_against_mach` recomputes every row.

**Result.**

- *The balance* (`hpr_sim::tests::canted_fins_spin_to_the_analytic_balance`): Valetudo with 1° of
  cant at 100 m/s, with no drag and no gravity, settles on `p = −δ V A_fin (r_t + y_MAC) k_T(B) /
  (k_R(B) ∫ξ² dA)` = −16.948 rad/s within 1e-6, the test's bound (1e-11 measured), and is within
  1e-5 of the exponential approach one time constant (0.48 s) in (2e-10 measured). The closed
  form uses the trapezoid's
  integrals (Niskanen eq. 3.70), independent of the polygon code. M1.8's roll bullet is met.
- *The forcing against TN D-4014* at `α = 0`: from Mach 2.3, all 8 readings within 5.3%
  (+3.2%, +2.4%, −0.1%, +1.0%, −1.4%, −1.0%, +2.5%, −5.3%); at Mach 1.5, +47.8%; at 1.8, +14.3%
  and +17.8%. *The short model's readings were corrected* (validation audit): M1.8a had placed
  each of its six panels' zeros 14.5 to 18.5 px (0.005 to 0.007) above the grid line the zero
  lies on (the grid's bottom edge at Mach 1.50, where the "0" label sits, then every 0.2); each
  reading is now M1.8a's curve position measured from the grid line, and agrees within 0.0016
  with the audit's own re-read. Before, the comparison read 10.3% worst from Mach 2.3 and +53.6%
  at 1.5; the long model's zeros were on their lines. The correction was found by the audit, not
  sought to improve agreement, and it moved every short-model reading the same way, by 0.005 to
  0.007. Linear theory's load climbs as `1/β` toward Mach 1 and the fins' doesn't; Barrowman
  found the same on the Tomahawk ("the theoretical value at M = 1.5 is no good", p. 66).
- *The damping against the Basic Finner:* −5.9%, −7.8%, −14.3%, −15.8%, −16.2% from Mach 1.51 to
  3.00. Barrowman's curve, with Busemann's third-order terms, averages 5.68% from the same
  points (p. 66); the fins are wedges 8% thick, which first order doesn't see, part of the gap.

**Alternatives considered.**

- *The airfoil's slope in the damping,* as the texts write it: about 2.4 times the damping for the
  Basic Finner, and unbounded toward Mach 1.
- *No body interference,* as Niskanen: 7% more forcing and about 17% less damping on the Arcas
  Robin's fins; Barrowman has the factors and the forcing already reads high.
- *Busemann's higher-order terms faster than sound:* M1.8a left them out of the normal force
  (ADR-027); adding them for roll alone would make the fin's roll and its normal force disagree.
- *A target set now:* the roadmap set none, and one chosen after measuring would be no target.

**Consequences.**

- Canted fins now spin a flight; the roll rate follows the forcing and damping. Designs with zero
  cant fly as before, but for the damping of a roll rate from inertia coupling or the jet.
- Open: roll near Mach 1.5 reads high; nothing measured checks roll below Mach 1.5 (TN D-4013's
  rolling-moment plots at Mach 0.6 to 1.2, with the fins canted 2°, are the next reference); the
  roll forcing doesn't change with the angle of attack, where TN D-4014 measures up to 13%.

## ADR-032: Normal-force overrides from RASAero II: the static force replaced, hpr's damping kept (2026-09-19)

**Context.** M1.8d's done-when (`ROADMAP.md`): "a RASAero II export's `C_Nα` and CP columns
replace hpr's in a flight, and the reading is tested on the Calisto export"; M1.8's bullet asks for
the M1.5 override tables extended to `C_Nα` and CP against Mach and angle of attack, importable
from RASAero CSV. A flight takes its pitch and yaw damping from each component in its own local
flow (ADR-011), so a whole-rocket normal force can't simply stand in for the components. RASAero
II's aerodynamic export (its Aero Plots screen, File, Export, To CSV File: RASAero II Users Manual,
2019, p. 76) gives, per Mach number and angle of attack, `CN`, its `CN Potential` and `CN Viscous`
parts, `CP` and two columns that repeat the 4° values, and no damping. The manual takes every
dimension in inches (p. 13), measures the CP from the nose ("distance measured from the nose",
p. 114), puts the coefficients on the largest cross-section of the body (p. 72), and takes the
viscous part from Jorgensen's crossflow method (p. 55).

**Decision.**

- **The table** (`hpr_aero::NormalForceTable`): one column per angle of attack in `[0°, 90°)`, each
  `C_N/α` and CP against Mach number. A lookup reads each column at the Mach number (linear,
  holding its ends and saying so) and interpolates linearly in `α` between columns, so `C_N` comes
  back exactly at the columns' angles and is a part linear in `α` plus one in `α²` between them.
  In the Calisto export the viscous part starts at Mach 0.91, and `CN Viscous(4°)/CN Viscous(2°)`
  is (sin 4°/sin 2°)² = 3.995 from there through Mach 1.3 (exactly `sin² α`), then 3.90 at 1.5,
  3.16 at 2, 1.73 at 3 and 1.05 at 4: the quadratic is RASAero II's shape through Mach 1.3 and an
  assumption faster. Below the first column the table holds that column.
- **Past the last column** `α_n`, the force at `α_n` splits: the 0° column's slope times `α_n`
  is the linear share, at the 0° CP, growing as `sin α / sin α_n` (hpr's fins, ADR-011); the rest
  of the force and of the moment grows as `(sin α / sin α_n)²`, the crossflow form hpr's body
  lift takes (Galejs; Niskanen eq. 3.26) and RASAero II's viscous part takes from Jorgensen.
  Written as `C_N(α_n) s + R (s² − s)` with `s = sin α / sin α_n` and `R` the rest, the extra
  term vanishes at `α_n` whatever the rest's station, so the station is held within the rocket
  (`lookup_within`, which `AeroModel` calls with nose tip to aft end) and the linear share within
  the force (a falling `C_N/α` leaves no rest), with no jump anywhere: force and CP are continuous
  at `α_n` and in the table's values, the force is never negative, and tail first it is zero. A
  first draft scaled the whole force by `sin α` with the CP held; physics review showed that
  loses the viscous part's faster growth and its forward CP (on the guide's invented example, 18%
  less force at 10° with the CP 5.3 cm aft). The second review found the unguarded split turned
  the force round for a falling `C_N/α`; the third, that a guard switching the split on and off
  jumped (36% in `C_N`) as the Mach number moved the rest through zero, and left the rest's CP
  unbounded aft. Proptests now hold the sign, the CP within the stations while `s ≥ 1`, and the
  continuity in Mach.
  From Mach 3 RASAero II's viscous part hardly grows between 2° and 4°, so there the `sin² α`
  share probably overstates the force. All of this is an assumption past the data; the lookup
  reports it (`beyond_alpha`), and `NormalForce::table` carries the lookup out of the model.
- **Reading RASAero II** (`from_rasaero_csv`): the columns `Mach`, `Alpha`, `CN`, `CN Potential`,
  `CP`, the only ones that must be numbers. At `α > 0` the slope is `CN/α`. At 0° `CN` is zero,
  so the slope is `CN Potential(α₁)/α₁` at the smallest positive angle and the same Mach number:
  the potential part is linear in `α` (the Calisto export's spread between 2° and 4° is 2.3e-15),
  and through Mach 1.3 the viscous part is `sin² α`, which has no slope at zero; faster, how it
  starts from 0° isn't in the export, and leaving it out is an assumption. Not
  `CNalpha (0 to 4 deg)`, the 4° secant with the viscous part in it (ADR-027). `CP` is converted at 0.0254 m to
  the inch from the nose tip, the datum of hpr's stations. The table's reference is
  `TableReference::LargestBody`, which `AeroModel` resolves from the largest body radius, so a
  design whose reference is its nose base still gets RASAero II's area. Angles need not share
  their Mach numbers (the Calisto export's 4° rows end at Mach 24.99, the others at 25); rows out
  of order within an angle, or an angle with one row, are refused with their line.
- **A units guard**: `AeroModel::with_normal_force_table` and `Simulation::with_normal_force_table`
  refuse a table with a CP value outside the rocket, nose tip to aft end, at its Mach numbers a
  flight can reach (to Mach 5 and the first knot past it), as `SolidMotor` refuses an impossible
  exhaust speed (#11). It checks the table's values, not what a user-built table's own
  interpolation might give between or past them. Both return `Result` where `with_drag_table` doesn't: a check at
  attachment names the problem before a flight starts. `with_reference_diameter_m` checks its
  diameter when given, for the same reason.
- **In flight** (`Simulation::with_normal_force_table`): the table's normal force at the centre
  of mass's airflow acts at its CP; each component adds its force in its own local flow less its
  force in the centre of mass's. In linear theory that difference is `q̄A C_Nα,i θ̇ ℓᵢ/V`, so the
  damping moment stays hpr's `q̄A Σ C_Nα,i ℓᵢ² θ̇/V` and the path term `q̄A Σ C_Nα,i ℓᵢ`. With no
  rotation the two component terms are the same number and cancel exactly. The table has no side
  force (an axisymmetric code). Mach 5 and faster is still refused, since the damping needs hpr's
  components; `AeroModel::normal_force` alone takes any Mach number with a table, as a drag table
  does. A flight on a table evaluates each component twice; flights without one are unchanged,
  bit for bit (the validation report doesn't move).
- **Tested on the Calisto export without committing more of it.** `cargo xtask aero` reads the
  pinned export with the library's reader and writes `normal-force-override.json`: each column's
  count and Mach range; every row at a positive angle read again with a plain split and compared
  with the table; a hash of the table; the viscous part's growth; the 0° column at the 15 Mach
  numbers M1.8a compared (the 30 values ADR-027 already commits); and Calisto flown four ways.
  CI has no `refs/`: there a test checks that the fixture's 30 values agree with those M1.8a's
  own parser committed (the same 0° rule, so it checks the reading, not the rule) and flies the
  15-point table again. Reading the export itself needs it: `cargo xtask aero --check`, which
  `aero::tests` runs where `refs/` is present.

**Result.**

- *The reading:* 0°, 2° and 4° columns of 2,500, 2,500 and 2,499 Mach numbers, Mach 0.01 to 25;
  the 4,999 rows at a positive angle come back with `CN` within 2.2e-16 relative and `CP` exact;
  the 30 values equal M1.8a's to 1e-12.
- *Linear theory* (`hpr_sim::tests::pitch_oscillation_follows_a_normal_force_table`): Valetudo at
  100 m/s with a table of 1.5 times hpr's slope and its CP 5 cm aft oscillates in pitch and in yaw
  with a period of 1.1040774 s against the theory's 1.1040734 s (1.44965 s on hpr's own; bound
  3e-5), and decays within 0.03% of the rate hpr's damping gives (bound 1%). The table's `K₁` in
  the path would predict 1.1044079 s and lumped damping a decay of −0.138 against −0.244: the
  test tells them apart.
- *Tables of hpr's own normal force* fly Valetudo in a crosswind to within 7.8 mm of hpr's own
  apogee (every 0.5° and Mach 0.01; bound 5 cm) and 5.5 cm (0°, 2° and 4°, past which the
  continuation flies; bound 10 cm); at 0°, 1°, 2°, 4°, 8°, 16°, 30°, 60° and 89°, every Mach
  0.05, 1.18 m, the interpolation's. A RASAero-shaped table continues its `sin² α` part to 1e-12.
  Calisto's export has no viscous part below Mach 0.91, so its flights (to Mach 0.75) grow no rest;
  the tables of hpr's own normal force, whose body lift is a rest, are the flights that do.
  A table on RASAero II's reference is rescaled by 4 on a rocket whose reference is half its
  largest body.
- *Calisto* from a 5.2 m rail at 85° in 5 m/s of crosswind, peaking at Mach 0.746: apogee
  2,793.11 m on the export's normal force against 2,794.39 m on hpr's own, 14.2 m further upwind.
  hpr's own as a table at the export's angles and Mach numbers moves it 0.03 m (the method
  alone). Each flight spends about 2.2 s past 4°: 0.3 s after the rail (up to 7.9°) and the last
  1.9 s before apogee. The 15-point table's apogee is within 0.006 m of the whole export's, and
  its position within 0.07 m.

**Alternatives considered.**

- *The table's force alone, at its CP:* the damping becomes `C_Nα (X_cp − x_cg)²`, smaller than
  the components' `Σ C_Nα,i ℓᵢ²` by their spread about the CP (the parallel-axis theorem), so the
  rocket would oscillate longer.
- *hpr's components scaled to the table:* matching both the slope and the CP takes two scale
  factors, and no split between body and fins is unique.
- *The 4° columns `CNalpha (0 to 4 deg)` and `CP (0 to 4 deg)` as a table in Mach only:*
  simpler, but every angle would carry the 4° viscous part.
- *The viscous share growing as `sin α` past the last angle from Mach 3,* where the export's own
  growth slows: a Mach-dependent rule fitted to one code's trend; recorded as a limit instead.
- *A warning when the table's CP at low Mach differs from hpr's by more than a caliber* (physics
  review): hpr has no warning channel on a flight yet; the guard refuses only a CP outside the
  rocket.

**Consequences.**

- A flight can fly RASAero II's normal force; with a drag table too, both of its main forces. The
  2018 Calisto export flies subsonic only (plain Barrowman there: no viscous part), so no real
  export has been flown through Mach 1; the transonic and supersonic columns are tested by the
  reader's unit tests and the re-read of every row.
- Open: the continuation past the last column is unmeasured; a design whose nose tip isn't
  RASAero II's reads a shifted CP with no warning unless it leaves the rocket. `STATUS.md`'s
  question to Neer about RASAero values in fixtures is unchanged: no new values are committed
  (the fixture adds counts, differences, ratios, a hash and flights).

## ADR-033: The body faster than sound: Syvertson and Dennis's second-order shock-expansion method (2026-09-19)

**Context.** M1.8a measured the gap (ADR-027): past Mach 3 the Arcas Robin's body alone lifts 3.9
to 4.6 per radian in NASA's wind tunnel (TN D-4014), where hpr's slender-body terms, with body
lift at the plotted angles, give 2.3 to 2.8. M1.8e asks for a cited supersonic method for noses,
boattails and crossflow, and holds the Arcas Robin to 15%. The research for it found that the
measured slopes, fitted over about ±4°, carry crossflow lift the linear term doesn't, and that no
source covers a 15° boattail. Doing all of it, and flying it, is more than one session.

**Decision.**

- **Split M1.8e** into M1.8e1, the method as a tested library model, and M1.8e2, flying it (the
  body's terms taking Mach, a join from subsonic, the boattail, crossflow), which carries M1.8e's
  bullet unchanged. M1.8e1's targets were set before measuring: within 0.05 per radian and 0.1
  calibers of TN 3527's own second-order values, and within its stated ±0.2 per radian and ±0.2
  calibers of its measurements (Summary, p. 1).
- **The method: Syvertson and Dennis's second-order shock-expansion method** (NACA TN 3527, 1956),
  the multi-step form, over MIL-HDBK-762's charts (Figs. 5-4 to 5-7, from the RAeS data sheets):
  the charts are carpet plots read by hand with an ambiguous corner, and they stop at an
  afterbody of 7 in their scaled length `l_a/(d√(M² − 1))`, which the Arcas Robin's cylinder
  passes below Mach 2.2 (11.8 at Mach 1.5); the method
  takes any pointed profile, and its report tabulates its own values and its measurements, at
  `α → 0`, for 144 bodies (Tables I and II), public data a test can hold it to. Van Dyke's hybrid
  theory, which Jorgensen (NASA TR R-474, p. 26) recommends faster than sound, is a
  characteristics solution, far more code.
- **Its inputs.** The tip cone by the Taylor–Maccoll equation (NACA Report 1135 eq. 177),
  integrated by classical Runge–Kutta at up to 0.001 rad, the step shrinking so the equation's
  denominator (zero where the flow normal to the rays is sonic) changes by at most 2% per step,
  and a shock angle found by regula falsi to rounding. A fixed step, in the first draft, gave
  slender cones (under about 2°, a tangent ogive's last elements) pressures that jumped and NaN
  (code and physics review); the step is a continuous function of the state rather than an
  error estimate's accept-or-reject, so platforms differ in last bits only. Below 5e-4 rad
  (0.029°) the start is too near the singular line even so (the second physics review measured
  errors of 45% at 0.006° and a wrong `Ok` from a capped run), so there the flow is slender-cone
  linear theory's, blended linearly into Taylor–Maccoll up to twice that angle, and a run whose
  surface misses the cone by over 1e-3 of its angle is an error. Checked against NACA Report
  1135's cone charts, slender-cone linear theory, monotone from 1e-6 rad, smooth in Mach, and on
  the weak shock up to detachment (a third physics review caught the strong shock just under it); Prandtl–Meyer from `afterbody` (ADR-030). The tangent cones' slopes are TN 3527's
  Fig. 2, read by hand at 0° to 24° for Mach 3 to 10, linear between readings, the Mach 3 curve
  held below Mach 3 and the Mach 10 curve above (an assumption M1.8e2 must measure).
- **The report's tangent body**: ten elements per curved piece, tangent at `x/l = 0, 0.1, …, 1.0`
  (footnote 9, p. 15), one per cone or cylinder. With 40 elements per curve in place of 10, no
  ogive-cylinder tried moves by 0.01.
- **Its limit** (p. 13): the exponential relaxation holds only where the gradient behind a corner
  has the sign of `p_c − p₂` (`η ≥ 0`). The report states that as a condition and doesn't say
  how it continued where it fails. hpr's reading: the element becomes the generalized method's
  (which the report says the equations reduce to at `η = 0`), its pressure constant and no
  gradient carried on. A first version carried the gradient on;
  on the fineness-3 ogive from Mach 5.05 it then ran away and the surface flow went subsonic at
  every element count. A second held the pressure but kept the gradient, and grew toward the
  generalized method's value (5.4 per radian, against the report's 2.8) as elements were added.
  This reading converges: 10 and 320 elements agree within 0.008 per radian. An element aft of
  the nose that would need it is refused, since it would carry its loading over any length
  (physics review found a guard on the last element only bypassed by a small boattail, and one on
  cylinders and boattails bypassed by a long shallow flare at Mach 16). The report's range of Mach number over nose fineness, 0.4 to 2, isn't enforced: its
  own Mach 6.28 rows are at 2.09; M1.8e2 decides for flights.
- **Boattails** by footnote 8 (`p_c = p₀`, slope 2), unvalidated here; the Arcas Robin's 57° lip
  is past Fig. 2 and left out.
- **The Arcas Robin's nose** is the secant ogive through its tip and base nearest the report's
  coordinate table (golden-section search on the arc radius): radius ratio 1.744, rms miss
  0.003 in, tip half-angle 10.76°. hpr's committed design keeps its power-series nose (ADR-027),
  whose tangent is vertical at the tip, which the method refuses.

**Result.** Not met, recorded: 75 of 528 comparisons outside the targets. Against the report's own
values: slopes 102 of 144 within 0.05 (−0.134 to +0.146), CPs 125 of 144 within 0.1 calibers
(−0.670 to +0.257). 49 of those misses are where the march stays inside the method's limit.
There a second implementation of the same equations agrees with hpr within 0.0001 per radian on
all 72 cone-cylinders (by the report's Appendix C closed form) and within 0.0006 per radian and
0.0003 calibers on the 60 ogive-cylinders inside the limit. It was written from the paper during
this work: a Python script with SciPy's cone solver, patched to hpr's hand-read Fig. 2, kept in
`refs/scratch/m18e/` and not committed. The same author wrote it, so it can't catch a misreading
both share. So the printed values depart from the equations as read here: the fineness-7 cone
on long cylinders reads high, and the ogives read low, growing with the cylinder. Why is unknown.
The report read its cone pressures from charts; sampling the loading only at tangent points moves
it by 0.004 (physics review). The other 12 are the fineness-3 ogive at Mach 5.05 and 6.28,
where the march reaches the limit near the tip. At Mach 5.05 hpr's CP is 0.19 to 0.67 calibers
ahead of the report, whose measurements agree with it, so the gap is hpr's (validation audit).
No reading tried reproduces both Mach numbers (issue #81). Against its measurements: slopes 117
of 120 within ±0.2 (−0.278 to +0.251), CPs 109 of 120 (−0.540 to +0.328). Six are on the
fineness-7 cone on long cylinders (the report already 0.07 to 0.15 high), three where the report
is itself 0.20 to 0.22 off, four on the fineness-3 ogive at Mach 5.05 (#81), and one at −0.206.
The Arcas Robin's nose and cylinder has no target. Short model: −18.7% to +16.4% (within 5% from
Mach 1.8 to 2.96; −15.0% and −18.7% at 3.96 and 4.63). Long model: −13.7% to −26.4%. The boattail
by footnote 8 takes 0.03 to 0.18 off. The method's slope grows with Mach (2.55 to 3.37) but not
with the longer cylinder, whose measured extra 0.57 at Mach 3.96 goes with its side area. That is
crossflow, M1.8e2's to settle.

**Consequences.** `hpr_aero::shock_expansion` (`ShockExpansionBody`, `cone_flow`,
`cone_normal_force_slope`) is public and flies nothing yet. `cargo xtask aero` writes
`validation/fixtures/aero/shock-expansion.json`, and `shock_expansion::tests` recomputes it and
pins its misses. TN 3527 is pinned in `refs.lock.toml`; its tables are in
`validation/fixtures/aero/tn3527-bodies.json` (a U.S. government work). M1.8e2 has to decide
what M1.8e1 leaves: noses with a blunt or vertical tip (power series, elliptical, Haack), the
boattail, Mach numbers below 3, crossflow at the angles flown, and the join to the subsonic terms.

## ADR-034: The body's supersonic normal force in flight: tabulated shock-expansion shares, joined linearly from Mach 1.2 (2026-09-19)

**Context.** M1.8e1 built the second-order shock-expansion method (ADR-033) as a library model,
but a flight still took slender-body theory's body terms at every Mach number, and
`hpr-sim`'s dynamics cached the bodies' damping stations at Mach 0. M1.8e2 flies the method for
a pointed nose and the cylinder behind it; M1.8e3 takes the boattail, crossflow and blunt tips.
One run of the method takes about 2.5 ms in a release build and 4 ms in debug, so it can't run
at each step of a flight. A flight's damping needs each component's own force at its own station
(ADR-011), so the method's force has to land on components, not only on the whole body.

**Decision.**

- **What it covers.** The nose, if it is the first body and pointed, and the body tubes straight
  behind it at the same radius. The run stops at the first transition, step in radius (over a
  millionth of the area) or gap. Each covered component takes its own segment's share
  (`ShockExpansionBody::segment_slopes`). Body lift (Galejs's `sin² α` term) is unchanged.
- **Only a body it can finish.** The method flies only if no body after the covered run has a
  potential-flow slope of its own (a boattail, a flare, a step). Otherwise the whole body keeps
  slender-body theory until M1.8e3. The first draft flew the method's nose and cylinder beside
  slender-body theory's boattail; the physics review showed that this moves the body's centre of
  pressure further from the wind tunnel's than slender-body theory alone. On the Arcas Robin at
  Mach 2.3, with the centre of mass 12 calibers aft, the body's moment slope about it was
  −31.6 calibers mixed, −25.8 by slender-body theory, and −25.7 by the method with its own
  boattail.
- **A table, built when first needed.** The shares are tabulated every 0.05 in Mach from Mach 5
  (the normal force's limit) down to the lowest Mach at which the method holds, every share is
  positive and every share's centre of pressure lies on its own segment, and interpolated
  linearly. The table is built the first time a flow faster than Mach 1.2 asks for it
  (`OnceLock`, shared by a model's clones), which took 0.3 s in a debug build on the
  development Mac, once per model. Built eagerly, it took the `hpr-aero` unit tests from 4.7 s
  to 238 s. Between rows the interpolation stays within 1e-4 of the method on the Arcas Robin.
- **The join.** From `M_j` = the larger of Mach 1.2 and the table's first row, over 0.3 in Mach:
  each covered component's slope, moment and damping station are slender-body theory's plus
  `w (shock-expansion − slender-body)`, `w = (M − M_j)/0.3` clamped to [0, 1]. Everything is
  linear in Mach, so it is continuous by construction; tests probe the join's ends, table rows,
  points between rows and Mach 4.999 at ±1e-9, on a body joined at 1.2 and on a 20° cone joined
  higher. Mach 1.2 to 1.5 is a judgement: below Mach 1.2 the flow over the nose is transonic,
  which the method doesn't cover, and Mach 1.5 is the lowest Mach at which TN D-4014 measured the
  Arcas Robin and M1.8e1 checked the method.
- **No refusal in flight.** A body the method can't take keeps slender-body theory at every
  Mach. The table is only read inside its rows, so a flight never meets the method's refusals.
- **Stations.** `dynamics.rs` no longer caches body stations; each evaluation asks for them at
  its Mach number, as it already did for fins. Below the join they are the same numbers. A
  covered cylinder's one station now serves its body lift too (it was the planform centroid), a
  compromise the nose already made.

**Result.** The Arcas Robin's nose and cylinder alone (TN D-4014), with the fitted secant-ogive
nose (ADR-033), through the flight's path: the method's slope and CP on table rows (Mach 1.5, 1.8,
2.3) and within 1e-4 between them; against the measured body, short model +16.4% to −18.7%, long
model −13.7% to −26.4%, as in M1.8e1. The committed design, whose power-series nose the method
refuses and which has a boattail, stays at M1.8a's slender-body values, 61% to 82% low. No
validation case flies past Mach 1.06, so the validation report is unchanged.

**Consequences.** `AeroModel::supersonic_body`, `SupersonicBody`, `SUPERSONIC_JOIN_START_MACH`
and `SUPERSONIC_JOIN_WIDTH_MACH` in `hpr-aero`. A rocket with a boattail or a pointed nose the
method refuses gets nothing from M1.8e2. Small changes in geometry can switch a body between the
two models (a step in radius past a millionth of the area, a nose just past Fig. 2, a boattail
appearing), and the join's start snaps to the 0.05 grid; both matter for dispersion and
optimisation studies ([issue #87](https://github.com/nrdptel/hpr-sim/issues/87)). `cargo xtask aero` adds the flight's values to
`validation/fixtures/aero/shock-expansion.json`.

**Update (M1.8e3, 2026-09-19).** The join's start no longer snaps to the grid: where the method
stops holding above Mach 1.2, bisection between the two rows finds that Mach to the last bit of
an `f64` (about 48 halvings, each one run of the method) and the table gains a row there. It has
to be that exact: the shares climb from zero like `√(M − M_start)` (the tip cone's surface flow
turning sonic), so the row's shares are `√δ`-sized for a start off by `δ`. The physics review
measured 24 halvings (`δ` up to 3e-9) on a 20° cone stepped by 2e-9°: the cylinder's row share
jumped 2.2e-5 to 2.5e-4 and the blended slope just above the start by 1.5e-6 per radian, a
sawtooth in shape. At full resolution the row's share is 1e-7 and the slope moves by under 4e-10
per radian. Between that row and the first even one the table is linear where the method rises
like a root: at Mach 1.345955 the cylinder's share reads 30% low (0.203 against 0.291), under
1e-3 per radian after the join's weight (not fixed; recorded). A 20° cone
now joins from Mach 1.341910, not 1.35. The model switches in #87 remain (M1.8e5). M1.8e3 was
split: the boattail became M1.8e4, crossflow and blunt tips M1.8e5, which carries M1.8e's bullet.


## ADR-035: Drop the orhelper dependency; how M2.2 drives OpenRocket is decided when M2.2 starts (2026-09-19)

**Context.** `orhelper` is a thin Python wrapper that starts a JVM with OpenRocket's jar on the
classpath and gives Python access to it. ADR-002 added it to `validation/oracles/pyproject.toml`
and had `refs doctor` check that it imports. It was never used: nothing in the repository imports
it, and no oracle script exists yet.

Checking the installed wheel rather than trusting the metadata: `orhelper` 0.1.5 ships the stock
**GPLv2** text, and its PyPI metadata carries no license field at all. Its `.py` files have no
licence headers, so whether the grant is "version 2 only" or "version 2 or later" is unstated.
Its own code is 555 lines, 91 of them mirroring OpenRocket's enums. It is pinned to a fork commit
because the PyPI release predates OpenRocket 24.12's `info.openrocket` packages.

Copyleft obligations attach on distribution. This repository is public, so
`validation/oracles/*.py` is distributed; a script that imported orhelper would raise the question
of a combined work. The Rust crates never touch it, run in a different process, and are unaffected
either way. `CLAUDE.md` rule 3 permits "Running GPL tools (the OpenRocket jar, via JPype/orhelper)
as external oracles", so importing it was allowed; the question was whether to.

**Decision.** Drop the dependency now. Nothing imports it, so removing it costs nothing and stops
GPL code being installed into `refs/venv` for no purpose. `refs doctor` checks only `jpype` for
the OpenRocket oracle, which is what its smoke test already used.

How M2.2 actually drives the jar is deliberately left open, to be decided with the evidence in
hand: JPype directly, or the jar as a subprocess. Note that JPype loads the JVM **into the Python
process**, so driving OpenRocket without orhelper still puts GPL-3.0 classes in that process.
Dropping orhelper removes the GPL-2.0 Python layer, not all contact with copyleft code. Only the
subprocess route isolates properly, and whether the jar has a usable command-line interface is
unverified: launching it opened the GUI.

**Consequences.**

- `uv.lock` loses one package, and the environment no longer installs any GPL-licensed Python.
- Whoever writes M2.2's oracle reimplements what orhelper wrapped, or takes the subprocess route.
  555 lines is the upper bound on the first, most of it thin glue and enum mirroring.
- `CLAUDE.md` still names orhelper as permitted. That permission is unchanged; this only records
  that the project is not taking it up for now.
- The licence of the fork was never in doubt, but its scope was under-specified. If M2.2 revisits
  orhelper, settle "v2 only" against "v2 or later" with the fork's maintainers first.

## ADR-036: The Arcas Robin's supersonic body gap: judged as the tunnel measures; M1.8e6 takes crossflow's size and the boattail (2026-09-19)

**Context.** M1.8e5 sized each cause of the gap between NASA's Arcas Robin body alone (TN D-4014,
fins off) and hpr's supersonic body (`docs/research/body-supersonic-gap.md`, fixture
`validation/fixtures/aero/arcas-robin-gap.json`). M1.8e4 had reported hpr 8.3% high to 27.0% low,
comparing hpr's slope at `α → 0` with a straight line fitted through points from about −5° to
+4°. That line carries crossflow lift. Fitted the same way, with the body lift a flight adds, hpr
reads 15% to 73% high. The fit's slope at `α → 0` and its curvature correlate at −0.95 to −0.96,
so the readings can't say how much of that is body lift and how much the slope at `α → 0`. With
body lift at Jorgensen's `η C_dn` (NASA TR R-474), hpr still reads 8% to 61% high. The one
sized cause that size is the boattail's share: TN 3527's footnote 8 gives −0.18 to −0.03 per
radian, slender-body theory −1.32. M1.8e's 15% bullet names no comparison basis, and M1.8e6's
title ("crossflow and blunt tips", "e5's top ranks") predates the ranking.

**Decision.**

- **The basis, fixed before measuring.** M1.8e6 and M1.8e's 15% bullet (carried by M1.8e7) judge
  the Arcas Robin as M1.8a fits it: hpr's bodies' `C_N` through a flight's path at the tunnel's
  plotted angles, fitted the same way (`hpr.fitted_c_n_alpha`). Slopes at `α → 0` are reported
  beside it, not judged.
- **The tunnel's slope at `α → 0` is never one number.** Report it under both fitted forms
  (`α |α|` and `α³`) and with the curvature held at a cited `K`.
- **M1.8e6's scope follows the ranking:** crossflow's size (Jorgensen's `η C_dn` or a cited
  alternative; the report's points to 16° to 21° show how it grows with `M sin α`) and the
  boattail's share, together.
  Blunt tips stay in it for coverage: the committed design's power-series nose is refused by the
  method. Its done-when is unchanged.

**Consequences.**

- M1.8e6 may change body lift, which drives a slow rocket's drift in wind (ADR-026); it decides
  whether a change applies below Mach 1 and regenerates the report.
- A result judged at `α → 0` alone no longer counts for the Arcas Robin.
- The ranking rests on hand readings of plots (±0.01 in `C_N`; the report states ±0.03 and ±0.1°
  in `α`), and the blunt tip on an assumed scaling; both are stated in the note.

## ADR-037: Body lift by Jorgensen's crossflow at every speed, and a boattail's measured share faster than sound (2026-09-19)

**Context.** M1.8e5 (ADR-036) ranked the causes of the gap between NASA's Arcas Robin body alone
(TN D-4014, fins off) and hpr's supersonic body: crossflow's size first, the boattail's share
second. hpr's body lift was Galejs's `K (A_plan/A_ref) sin² α` with `K` = 1.1 at every Mach
number; the tunnel's fins-off curvature implies 0.66 to 1.05 from Mach 2.3 and Jorgensen (NASA TR
R-474) about 0.9. A boattail took TN 3527 footnote 8's share, −0.18 to −0.03 per radian where
slender-body theory gives −1.32; nothing measured it. Measured, the fitted-nose body read +14.9% to
+73.2% like for like.

**Decision.**

- **Body lift is Jorgensen's `η C_dn (A_p/A_r) sin² α`** (TR R-474 eq. 2.12), at every Mach
  number: one model, so no join and no jump; below Mach 1 it brings the committed Arcas Robin
  bodies' fitted slopes closer to the tunnel (short at Mach 0.6: +41% to +25%). `C_dn` is Fig. 1's
  (subcritical; from `M_n` 0.6 to 1.2 its Ames points), `η` Fig. 4's against fineness and Fig. 6's
  against the crossflow Mach number `M_n = M sin α`. Fig. 6 holds for bodies of fineness 10 to 12,
  so for fineness `f` hpr takes `η = η₆ [η₄(f) + (1 − η₄(f)) r] / [0.69 + 0.31 r]`, with `r` the
  most that Fig. 6's rise `(η₆ − 0.69)/0.31` has reached up to that `M_n`: a judgement that keeps
  Fig. 4 at low `M_n`, gives Fig. 6 back near `f` = 10.6, gives every fineness Fig. 5's `η C_dn`
  past `M_n` 0.8 (the physics review found that letting the share fall back with Fig. 6's dip at
  `M_n` = 1, an artifact of Jorgensen's division by Fig. 1's peak, brought the length's effect back
  there), and stays below 1. hpr pairs `η C_dn` with its own attached-flow term, not the
  `sin 2α cos(α/2)` Jorgensen subtracted to find it. Both curves are sampled
  at Fig. 6's eleven points, so their product is Jorgensen's own Fig. 5 there (within 3%) rather
  than the product of two steep curves read separately. The fineness is the body's length over
  its largest diameter. The drop in `C_dn` past the critical crossflow Reynolds number is left out:
  Jorgensen computes it only for illustration, with no data.
- **A supersonic boattail takes Washington and Pettis's measured increment** (MICOM RD-TM-68-5,
  1968, Fig. 5: `ΔC_Nα / [1 − (D_B/D)²]` against `√(M² − 1)/(L_B/D)`) on the share the method gives
  a cylinder of its length and fore radius in its place, at their Fig. 6 centre of pressure. Their
  boattails were conical, 4° to 9.5°, 0.82 to 1.18 diameters long, to 0.72 to 0.86 of the diameter;
  anything else is an extrapolation, stated, as is a transition that isn't conical (it takes the
  same correlation from its length and radii); past the curve's end (a short or steep boattail at
  a high Mach number) the last value is held. A
  tube behind the boattail keeps the method's share. The run the method covers is unchanged.
- **The old rules stay selectable** (`BodyModel`, `BodyLift::Galejs`, `SupersonicBoattail::Footnote8`),
  so M1.8e5's fixture reproduces unchanged and sweeps of `K` stay possible; the default is the new
  model.
- **The split.** Blunt tips, which ADR-036 kept in M1.8e6 for coverage, move to a new M1.8e7 with
  the lip behind a boattail: together they keep the committed Arcas Robin designs off the method.
  The old M1.8e7 (issues #87 and #90, M1.8e's 15% bullet) becomes M1.8e8. No done-when changes.
- **M1.8a's comparison gains a miss.** The committed designs fly slender-body theory past Mach 1 and
  read 37% low fins off at Mach 2.96 (short); Galejs's larger body lift had covered enough of that
  for the whole rocket to pass at −13.4%. With Jorgensen's it reads −16.3%, outside M1.8a's 15%.
  The miss is recorded and pinned in `normal_force_against_mach`, not hidden; M1.8e7 is the fix.
- **"The Arcas Robin through a flight's path in the report"** means, as for M1.8e2 and M1.8e4,
  the committed fixture `arcas-robin-crossflow.json` and the guide's tables pinned to it; the
  validation report holds whole flights only.
- **The wind oracle flies the new body lift.** `wind_response.py` carries the same tables (a test
  checks they are the library's) and its constant-`K` sweep adds 1.1.

**Consequences.**

- Like for like, the fitted-nose body reads +3.4% to +41.0% (`arcas-robin-crossflow.json`): each
  change takes about half of the old excess off. From Mach 3.96 both models are within 15% (+3.4% to
  +7.3%); Mach 1.5 to 2.96 still read 16% to 41% high, which the readings can't split between
  body lift and the slope at `α → 0`; at Mach 1.5 and 1.8 on the short model, where the tunnel's
  points from −5° to +4° barely curve (its 6° points need a factor of 0.47 and 0.58 on body lift,
  hpr's is about 0.9), it is mostly body lift.
- At the tunnel's 62 plotted angles from 5.5° to 21.7° (`arcas-robin-high-alpha.json`, read for this
  milestone), 48 are within 15% (34 before).
- **The moment, checked** (a partial model swap can match a slope with its lift in the wrong place):
  the body's centre of pressure at the tunnel's low angles, against its fins-off pitching moment
  (`arcas-robin-fins-off-moment.json`, read for this milestone), was 0.90 to 3.85 calibres aft of
  the tunnel's on the short model and 0.59 to 1.94 on the long; now −0.19 to +1.59 and −0.88 to
  −0.18 (±0.5 from the readings). The long model's low-angle points are M1.8a's, which issue #97
  suspects read 3% to 5% high in slope; its numbers here rest on how that is settled. Where the crossflow is supersonic (`M_n` ≥ 0.95) hpr
  now reads 1% to 16% high, where Galejs's constant read 7% to 22% low.
- Whole flights: every gated metric still passes. The drifts reported as model differences moved
  (Juno III's apogee drift from −42.5% to −38.2%); RocketPy flown with hpr's rail release, body lift
  and fin slope lands within 1.3% of hpr's in every windy case.


## ADR-038: Blunt and vertical nose tips faster than sound by a Newtonian cap, the method started from the tangent cone (2026-09-19)

**Context.** The second-order shock-expansion method (NACA TN 3527, ADR-033) starts at a pointed
tip with the flow on a cone. Power-series noses with `n` below 1, Haack series and elliptical noses
leave the tip at 90°, so hpr refused them and their rockets kept slender-body theory past Mach 1:
the committed Arcas Robin designs (whose power-series nose stands in for the model's tabulated
nose with a 0.062-in spherical tip) and the von Kármán noses of Calisto, Cavour, Juno III and
Prometheus 2022 among them. ADR-037 split blunt tips and the lip into M1.8e7 with a lead: NASA
TN D-4865 (C. M. Jackson Jr., W. C. Sawyer and R. S. Smith, 1968) puts a Newtonian cap ahead of
TN 3527's method and compares it with its own tunnel data from Mach 1.50 to 4.63.

**Decision.**

- **The split.** Milestone ids allow one increment level, so the siblings are renumbered: M1.8e7
  is blunt tips, M1.8e8 the lip (with the committed designs' done-when), and the old e8 (issues #87
  and #90, M1.8e's 15% bullet) M1.8e9. e7 keeps "flown with no jump at ±1e-9 in Mach", and takes
  "the Arcas Robin's committed nose (lip left off) through a flight's path in the report" and a
  check against TN D-4865's own sphere-cone; e8 keeps "the committed Arcas Robin designs through a
  flight's path in the report". Like the Arcas Robin's since ADR-036, the sphere-cone check carries
  no target: it shows where hpr stands, set out before measuring.
- **The cap** is TN D-4865's modified Newtonian `C_p = C_p,max sin²δ` (eq. 1, p. 5), `C_p,max`
  from the Rayleigh pitot formula (NACA Report 1135 eq. 100, p. 619). At `α → 0`, in TN 3527's
  loading form (`C_Nα = (2π/A_ref) ∫ Λ r dx`), the wind's slope `δ + α cos φ` gives
  `Λ = C_p,max sin δ cos δ`; a hemisphere then carries its Newtonian drag turned, `C_p,max/2`.
- **The handover** is TN D-4865's (p. 5): where the slope falls to the largest angle a
  two-dimensional wedge turns with an attached shock (NACA Report 1135 eqs. 138 and 168, p. 621 and
  p. 624), which the report chose for its agreement at low supersonic speeds; capped at 24°, the
  steepest cone of TN 3527's Fig. 2, whose slope the method needs. From about Mach 2.1 the cap
  therefore reaches further aft than the report's.
- **The march starts behind it as at a pointed vertex**: the flow on the cone tangent to the body
  at the handover, that cone's loading and no gradient (TN 3527 sketch (a), p. 6), not the
  report's Newtonian pressure and Mach number (eq. 2). Read at `α → 0` as the report's equivalent
  bodies imply (eqs. 4a and 4b: the handover holds still in the wind, so the flow behind it turns
  by `α cos φ` less and its loading is the Prandtl–Meyer flow's `λ/(γM²)`, hpr's reading), the
  report's start, measured in `blunt-tips.json` (`starts`, `sphere_cone`):
  - fails on the Arcas Robin's committed nose from Mach 3.96, where the march reduces the element
    at the nose's end, which hpr refuses aft of a nose (its pressure at the handover lies below the
    tangent cone's at every speed here); since a flight's table is built from Mach 5 down
    (ADR-034), that would leave such a rocket no method;
  - reduces elements from Mach 2.96 (issue #81), so its answer moves with their number (2.544 to
    2.583 per radian at Mach 2.96, 3.361 to 3.418 at 3.5, from 10 to 40 elements);
  - on the report's own sphere-cone, against the measured slope at `α → 0` (fitted both ways,
    ADR-036), reads closer than hpr's at Mach 1.9, 3.95 and 4.63, about the same at 2.96, further
    at 2.3, and 95% high at Mach 1.5, where the handover (12.1°) sits 0.6° above the 11.5° cone
    and the linear range is that small. That is hpr's `α → 0` reading of the report's start; the
    report's method itself, at its own angles, reads 1.844 per radian there.

  The tangent cone's start holds to Mach 5 on the Arcas Robin's nose and moves by under 0.01 per
  radian from 10 to 40 elements. Both stay in the library: `HandoverStart::Newtonian` selects the
  report's, for comparison; a flight takes the tangent cone's.
- **Scope.** A blunt tip is a first segment whose slope is infinite at the tip (power series with
  `n` below 1, Haack, elliptical, and `BodySegment::SphericalCap` in the library for sphere-cones).
  Pointed noses are unchanged. A nose steeper than the handover's slope all the way to its base is
  refused and keeps slender-body theory. The table, the join and its bisected start (ADR-034) are
  unchanged, and so is drag.
- **"In the report"** means, as for M1.8e2, e4 and e6, the committed fixture `blunt-tips.json` and
  the guide's tables pinned to it cell by cell: no validation flight passes Mach 1.2 (Prometheus
  2022 peaks at 1.06), so the whole-flight report is unchanged.
- **TN D-4865's Fig. 8(a)** (model 1, printed p. 101) is read from the report's 300-ppi scan by
  pixel analysis (axes fitted to the labelled grid lines, each circle's centre fitted to its ring)
  into `tn-d-4865-sphere-cone.json`, to about ±0.003; the α = 0 circles read −0.0027 to
  +0.0163, so the plotting itself is good to about ±0.01. It is compared as ADR-036 compares the
  Arcas Robin: hpr's `C_N` at the plotted 0° to 12° (the method's slope with Jorgensen's body lift,
  for a fineness of 1.75, below his Fig. 4's range), fitted with a straight line as the measured
  `C_N` and `C_m` are, the report's own method the same way; the slopes at `α → 0` beside it under
  both curved fits, not judged.

**Consequences.**

- TN D-4865's sphere-cone, like for like: hpr −1.2% to +32.1%, close to Mach 2.3 and high from
  Mach 2.96 (+12.5%, +29.7%, +32.1%), where the report's own method reads +5.2% to +13.6% (−3.3%
  to +13.6% overall); its centre of pressure within 0.06 diameters. At Mach 3.95 and 4.63 hpr's
  slope at `α → 0` is 13% to 21% above the measured, and body lift raises its fitted slope 25% to
  27% above that where the measured curve rises 9% to 16%. At `α → 0` hpr reads −13.5% to +20.5%
  (`α |α|` fit) and −12.3% to +13.6% (`α³`).
- The Arcas Robin's committed nose with the lip left off, like for like: short +37.2% to −4.8%,
  long +19.3% to −4.1%, at or below the fitted secant ogive (+3.4% to +41.0%) at every Mach
  number, by up to 3.8 points to Mach 2.96 and 5.1 to 8.2 past Mach 3, where it reads within 5% of
  the tunnel. Lower is not always closer: past Mach 4 its error changes sign, so at Mach 4.63 it
  reads −4.8% where the ogive reads +3.4%, 1.5 points further out. It is nearer at nine of the
  eleven rows. The committed designs themselves keep slender-body theory for their lip until
  M1.8e8, so M1.8a's short model at Mach 2.96 still reads −16.3%.
- M1.8a (`normal-force-vs-mach.json`): Calisto's von Kármán nose flies the method past Mach 1.2.
  Against RASAero II's potential-flow slope, Mach 1.5 −3.1% to +13.2%, 1.75 −11.7% to +9.7%, 2.0
  −16.8% to +8.8% (now within both targets), 1.3 +12.7% to +16.1% (still outside); 11 of 15 rows
  within both targets (10 before). Against its secant slope to 4°, which carries its crossflow
  lift, 3 of the 11 rows from Mach 0.8 (1 before), Mach 2 −9.2% (−30.6%).
- Unvalidated, and said so: no measurement checks a tip that isn't spherical; Newtonian theory on
  the cap and the tangent cone's start are approximations; #81 still applies behind the handover.
- **A cap that shrinks to nothing doesn't reach the cone it sits on** (issue #101, raised by the
  physics review and pinned by `a_vanishing_cap_does_not_reach_the_cone_it_sits_on`). The march
  keeps its start cone's total pressure the whole way, as TN 3527 does from any vertex, and nothing
  makes that fade with the cap. A power-series nose of `n` = 0.99, a 7.1° cone but for a tip 1e-55
  calibres across, carries 1.21 per radian on its cylinder at Mach 4 where the cone carries 1.37
  (12% less, the body 7%), because the march runs on the 24° cone's total pressure, 107 free
  streams, rather than the 7.1° cone's 151. Fixing it needs a model of the tip's entropy layer
  fading downstream, with a source; the shapes a rocket uses have caps that are small but not
  vanishing, and their share of this bias is unknown.
- **At `α → 0` the handover is held where it sits on the body**, as TN 3527 holds every other
  point; the report's equivalent bodies slide it along the surface instead, and that term is left
  out, its size unmeasured here. The two starts in `blunt-tips.json` differ by more than it alone,
  their pressure and total pressure differing too: 2.53 against 2.87 per radian on the Arcas nose
  at Mach 1.5, 1.702 against 1.700 on the sphere-cone at Mach 2.96.
- **The cap covers much of a slender nose near the join's start:** 59% of the Arcas Robin's nose
  and 48% of a five-calibre von Kármán's length at Mach 1.25, about 5% by Mach 1.5 and under 1.5%
  past Mach 2 (a two-calibre ellipse keeps 13%), against 8% for the report's own sphere-cone at
  Mach 1.5. The join's weight, 0 at its start and 1 a third of a Mach number later, damps it; the
  guide says to read those rows as the blend they are.
- Two switches in shape stay open, of issue #87's family: a vertical tip steeper than 24° to its
  base gets no method, and a pointed tip steeper than Fig. 2's 24° is refused where a vertical one
  flies. Which tangency points merge behind the cap also changes with the handover, so the answer
  steps by about a millionth of a per-radian slope as they do.
- Cost: a vertical-tip rocket builds the supersonic table (Calisto's 363 ms, once per model, only
  once a flow passes Mach 1.2); a subsonic flight never builds it (`docs/perf.md`).

## ADR-039: A lip in a boattail's wake carries nothing faster than sound (2026-09-19)

**Context.** M1.8e7 gave the committed Arcas Robin designs' vertical tip a Newtonian cap, but their
*lip* — a reflexed flare 0.053 in long at the very base, rising from 1.308 in to 1.47 in behind a
15° boattail, about 57° to the axis — still kept the shock-expansion method off the whole body:
the method covers a run only while nothing behind it carries a potential-flow slope of its own
(ADR-034), and slender-body theory gives that lip `2 ΔA/A_ref` = 0.178 per radian at the base.
So both designs flew slender-body theory past Mach 1, 15% to 50% below the tunnel's body alone, and
M1.8a's short model read −16.3% at Mach 2.96 and −28.0% at 4.63.

**Decision.**

- **A lip wholly in a boattail's wake carries no potential-flow slope faster than sound**, so the
  method covers the body to its base. Below the join it keeps slender-body theory's share, and the
  join blends them, so nothing jumps. The shelter is the drag buildup's own measure
  (`crate::drag::WakeTerm`, ADR-030): wholly in the wake to a rise of a quarter of the boattail's
  drop in diameter, not at all from half of it, fading over any tube between. Anything else — a
  flare further out, or further back — still keeps the whole body on slender-body theory.
- **Why nothing, from three readings.**
  - TN D-4014 (p. 6) traces an odd chamber axial force at Mach 1.50 and 1.80, fins off, to the
    reflex lip, and says separation over the boattail at higher Mach numbers, or the thicker
    boundary layer with fins on or on the longer model, "masks" it.
  - Seiff's embedded Newtonian flare method (NASA TN D-1304), which TN D-4865 uses for a flare
    whose shock has detached, holds for "thin shock layers when the flow is not extensively
    separated" (p. 13), and he notes "a 90° ramp will invariably separate the flow" (p. 4). This
    lip's face stands about 57° to the axis behind a 15° expansion.
  - Taken anyway as an upper bound (eq. 9, p. 12; for a conical flare at one dynamic pressure
    `2 (q₁/q∞) cos²θ ΔA/A_ref`, with `q₁` the flow expanded through the boattail's turn), it gives
    0.044 per radian at Mach 1.5 falling to 0.014 at 4.63 — a quarter to a twelfth of slender-body
    theory's 0.178.
- **The moment, checked, and found unable to settle it** (`arcas-robin-lip.json`). For each
  fins-off row, the share at the lip's station that would put hpr's centre of pressure on the
  measured one runs from −0.256 ± 0.068 per radian (short, Mach 1.5) to +0.229 ± 0.084 (long,
  Mach 3.96), changing sign with Mach number and with the model's length. Fitting one share gives
  +0.021 ± 0.019 over all eleven rows (χ² per degree of freedom 4.5), −0.016 ± 0.022 over the short
  model's six (6.4, so its rows disagree among themselves) and +0.108 ± 0.034 over the long
  model's five (0.9, so those five agree — on a share three standard errors above zero and two
  below slender-body theory's). The number blames the lip for every miss in the centre of pressure,
  and the long model's body alone reads 15% to 19% high at those speeds, which moves its centre of
  pressure by more than any lip could; the short model's fit comes out negative, which no flare can
  give. So the moment rejects slender-body theory's 0.178 (8.7 standard errors on the short model,
  2.1 on the long) and cannot separate zero from Seiff's bound. The physics above, not this,
  decides the rule (the physics review asked for the split).
- **"The committed Arcas Robin designs through a flight's path in the report"** means, as for
  M1.8e2, e4, e6 and e7, the committed fixture `arcas-robin-lip.json` and the guide's table pinned
  to it; no validation flight passes Mach 1.2, so the whole-flight report is unchanged.

**Consequences.**

- M1.8a (`normal-force-vs-mach.json`): every row from Mach 1.5 is now within the 15% slope target,
  +8.8% to −3.3% on the short model and +9.4% to −2.3% on the long, where the short read −16.3% at
  Mach 2.96 and −28.0% at 4.63. The short model's body alone, fins off, went from 2.0–2.1 per
  radian to 3.0–4.0 and the long model's from 2.5–2.6 to 3.8–4.4, against the tunnel's 2.2–4.6.
- Five rows left the miss list (the short model at Mach 2.96, 3.96 and 4.63, the long at 3.96 and
  4.63) and two joined it: the long model's centre of pressure at Mach 1.8 and 2.3 now sits 0.53
  and 0.52 calibres forward of the measured, just outside the half-calibre target, because its body
  alone reads 15% to 19% high there — the excess M1.8e6 sized and left (ADR-037), which M1.8e9
  carries.
- With the lip left off entirely the same bodies read within 0.05 percentage points at ten of the
  eleven rows, and 0.5 at Mach 1.5 on the short model, so the rule changes which parts the method
  may cover far more than it changes the lip's own lift.
- Unvalidated, and said so: nothing here measures a lip's lift directly.
- The shelter's threshold is a switch in shape, of issue #87's family, and a large one, since it
  decides whether the whole body flies the method: on the test rocket at Mach 3 and 4°, a lip
  rising 0.2499 of the boattail's drop gives `C_N` 0.2976 and one rising 0.2501 gives 0.1995, a
  third less, with the centre of pressure 1.8 calibres further **forward** — the direction was
  printed backwards here and corrected in ADR-041, which also replaced the rise's threshold with
  a weight, so the two sides now agree to a ten-thousandth. Pinned by
  `a_lip_in_a_boattails_wake_carries_nothing` and noted on the issue.
- A narrowing part behind the run is a boattail the method hasn't covered, not a lip, whatever the
  wake does to its drag: it keeps slender-body theory's share (the code review found this; a second
  boattail drawn with a small step up would otherwise have been zeroed).

## ADR-040: A steep boattail reads its measured correlation no steeper than 16°, and M1.8e's 15% target judged (2026-09-19)

**Context.** Issue #90 left two things open from M1.8e4 and M1.8e6. hpr gave *any* narrowing
transition the supersonic boattail treatment, though Washington and Pettis measured boattails of
4° to 9.5° and TN 3527's footnote 8 claims only "reasonable results for bodies having moderate
amounts of boattail": a 30° boattail took about a twentieth of the lift slender-body theory removes,
which flatters a rocket's stability. And footnote 8's size was pinned only by its sign and its
continuity, never by a hand calculation. Separately, M1.8e's 15% bullet — the Arcas Robin's
body alone within 15% from Mach 1.5, and both configurations' whole rocket within 15% at Mach 3.96
and 4.63 — had never been judged, since until M1.8e7 and M1.8e8 the committed designs didn't fly
the method at all.

**Decision.**

- **The correlation is read no steeper than 16°.** The drag buildup already treats a boattail's
  flow as separating from 16° (Cubbage's steepest attached boattail, NACA RM L57B21). Past that
  angle hpr stops reading Washington and Pettis's correlation any steeper: a boattail takes the
  increment of one of the **same radii** drawn out to 16°, its centre of pressure staying on the
  real boattail. This is issue #90's cap. It is continuous in shape (at 16° the two lengths are
  equal), so nothing jumps as a boattail is drawn steeper.
  - **Why hold it rather than fade it to nothing.** A fade was written first and rejected on
    review. The increment is negative — it takes lift off the tail — so letting it fade moves the
    centre of pressure **aft** and makes a steep boattail look *more* stable, which is the very
    complaint issue #90 filed: with a fade a 30° boattail removed 26 times less lift than
    slender-body theory, against footnote 8's 21 times that the issue measured. Holding the
    correlation keeps the centre of pressure at the forward end of the honest range. On the tests'
    rocket the two rules differ by 1.35 calibres of body centre of pressure at 30° at Mach 1.5,
    0.91 at Mach 2, 0.75 at Mach 3 and 0.67 at Mach 4.63 — largest at the low speeds a hobby
    rocket flies — and both ends are pinned by a test.
  - **One further bound, on the invented length only.** Reading a longer boattail walks the
    correlation's argument toward zero, where Fig. 5's curve comes from the report's lowest
    supersonic runs and rises past Munk's slender-body line — which RD-TM-68-5 plots there for
    comparison at subsonic speeds (p. 3), not as a supersonic bound. hpr does not invent a length
    and then read that branch, so the *extra* the holding removes stops at potential flow's
    `2 (A_aft − A_fore)/A_fore`. The two reads are equal at 16°, so this adds a kink, never a
    jump (`holding_the_correlation_stops_at_potential_flow`).
    - It bites when the aft radius is under about `1 − √(M² − 1)/1.11` of the fore radius (the
      constant is the curve's crossing, 0.635, over `2 tan 16°`, from a trace read to ±0.0003 per
      degree, so it is good to about a percent): two fifths at Mach 1.2, a quarter at 1.3, a
      twentieth at 1.45, nothing much above Mach 1.49.
    - What that is worth is measured, not argued. `what_the_potential_flow_bound_reaches` sweeps
      boattails of 16° to 53.6° narrowing to between a thousandth and three tenths of the fore
      radius, reads the shares back out of the table, and pins three things: at the table's rows
      a boattail never takes off more than potential flow, except in the sliver where its own
      read already passes it; there, the table carries the correlation as published; and at most
      the bound moves a **printed** coefficient (after the join's weight) by about 0.060 per
      radian, at 53.5° narrowing to a thousandth of the radius, the steepest the sweep found the
      method willing to table; the holdback on the boattail's own cross-section is up to about six
      times that, near the join where the weight is small. Deleting the bound fails that test, and so does clipping the floor.
    - No committed design reaches it: the steepest, Calisto's 18.4°, has an aft radius 0.685 of
      its fore radius against the 0.40 the bound would need even at Mach 1.2.
    - It does **not** bound the correlation itself. A boattail's read at its own length is used as
      published, wherever it sits — a genuinely long one reads the same near-Mach-1 branch
      unbounded (a 4° boattail to 0.6 of the radius reads 1.29 times Munk at Mach 1.5). The
      extrapolation list in the guide therefore names longer boattails as well as steeper,
      shorter and narrower ones.
    - Three things are left open. Where a boattail's **own** read already passes potential flow
      the bound does not clip it — that is the correlation as published — so the 16° hold
      contributes nothing there and the boattail flies an extrapolation nothing measured checks;
      the sweep finds that sliver at 16°, 16.5°, 17° and 17.25°, and pins those; it closes as the
      angle or the speed rises rather than at a fixed angle. The bound
      applies at the table's rows, where the shares are computed, so a printed value between a
      bounded row and a floor row can sit a little past potential flow. And when the bound binds,
      hpr reports slender-body theory's size at Washington and Pettis's station, which for a cone
      differs from slender-body theory's own by up to about a tenth of the boattail's length.
  - **What is not known.** Nothing measures a separated boattail's supersonic normal force, so
    neither limit is validated. The 16° itself is Cubbage's, from transonic *drag* data (Mach 0.6
    to 1.28), used here from Mach 1.2 up, where a shoulder's Prandtl–Meyer turn makes separation
    less likely — so if anything it is early. The guide says all of this where the rule is
    described.
  - The Arcas Robin's 15° boattail is unchanged. Calisto's 18.4° now reads its correlation as a
    16° boattail, which moves its four supersonic rows against RASAero II by under half a point,
    each one closer in slope than before (Mach 2: +8.8% to +8.3%).
- **Footnote 8's size, and the tube behind, are integrated by hand.**
  `footnote_eights_boattail_share_by_hand` integrates eq. 19 —
  `Λ = (1 − e^(−η)) Λ_c + e^(−η) Λ₂`, `C_Nα = (2π/A_ref) ∫ Λ r dx` — over both a conical boattail
  element and the tube behind it, as issue #90 asks, from the flow the method reports at each
  corner, and matches `segment_slopes` to 1e-6. It pins the integration and footnote 8's two
  tangent-cone terms (`p_c = p₀`, `Λ_c = 2 tan δ`), checked against hard-coded values. It does
  **not** pin the decay rate `η`, which comes from eq. 9 and is read from the method; over the tube,
  where `Λ_c = 0`, `η` alone sets the answer, and the tube carries the larger part of the pair. The
  new `ShockExpansionBody::element_flows` makes that flow public, so any such check can be written
  from outside the crate.
- **M1.8e's 15% bullet: half met, half not, and recorded** (`arcas-robin-body-gap.json`).
  - **Met:** both configurations' whole rocket at Mach 3.96 and 4.63, +2.8% to −3.3%.
  - **Not met:** the body alone is outside 15% on six of eleven rows — the short model at Mach 1.5
    (+37.7%), 1.8 (+25.9%), 2.3 and 2.96 (+16.9%), and the long at 1.8 (+19.4%) and 2.3 (+15.5%).
    At Mach 3.96 and 4.63 both are within 5%.
  - **How well the miss can be explained: less than it first looked.** Each fitted slope splits
    into its value at `α → 0` and the curvature the plotted angles add, but the measurement's own
    split is soft: over seven points spanning about ±4.5°, the `α |α|` fit's two terms correlate at
    −0.96, so a low slope forces a high curvature. The measured `α → 0` slopes carry ±0.30 to
    ±0.42 per radian and the curvatures at least as much.
  - **What the split still shows.** At `α → 0` hpr is within 1.5 standard errors of the
    measurement on every row outside the target, so the readings cannot convict the
    shock-expansion method, the Newtonian cap or the boattail's share. On five of those six rows
    most of the gap sits in the curvature, which for hpr is body lift — Jorgensen's crossflow
    term, chosen in ADR-037 on the tunnel's high-angle points, reading too large at a few degrees.
    The sixth row is a counterexample and is recorded as one: on the short model at Mach 2.96, 77%
    of the gap is hpr's own `α → 0` slope, 1.2 times the measured. The worst row, the short model
    at Mach 1.5, also sits at `M/f_n` 0.36, below TN 3527's stated 0.4.
  - **Not tuned.** Closing it needs a cited rule for how the crossflow term grows over the first
    few degrees, or measurements at finer angles than the reports plot. Neither is in hand, so the
    gap stays visible in the guide and in the fixture, as M1.8e's bullet allows.

**Consequences.**

- A boattail steeper than 16° takes the same increment as a 16° one of its radii, so it removes
  more lift than reading the correlation raw would give, and its rocket's centre of pressure sits
  forward of that — the conservative end of a range about three quarters of a calibre wide.
  Calisto's four supersonic rows against RASAero II each move under half a point, all closer in
  slope; its Mach 1.3 row stays outside both targets, as it was, and moves slightly in
  (+16.06% to +15.94%, 0.720 to 0.716 calibres). Their centres of pressure move the other way at
  Mach 1.75 and 2.0, from −0.252 to −0.281 and from −0.410 to −0.443 calibres, the last 89% of the
  half-calibre target: taking more lift off the tail moves the whole rocket's centre of pressure
  forward.
- `ShockExpansionBody::element_flows` is new public API: the state behind each element's corner,
  its tangent cone, how fast the pressure relaxes toward it, and the radius eq. 19 needs.
- M1.8e's bullet is now judged in one place, with a test pinning which rows are outside; M1.8e10
  carries issue #87's model switches, the last of M1.8e's open work.

## ADR-041: A lip's shelter is weighed as the drag buildup weighs it, not switched at a threshold (2026-09-20)

**Context.** Issue #87 lists five places where the body's supersonic normal force is continuous in
Mach but jumps with a tiny change of shape. All five flip the same gate — whether the
shock-expansion method covers the body at all — so each is worth the *whole body*, not the part
that changed. The largest was the lip's: ADR-039 let a lip ride along when the drag buildup's wake
covered it wholly, and the aero rule read that as a threshold, `fraction >= 1`. On the tests'
rocket at Mach 3 and 4°, drawing the lip a ten-thousandth of the boattail's drop taller took a
third off the normal force and moved the centre of pressure 1.77 calibres forward.

**Decision.**

- **The weight is the wake's own share.** `crate::drag::WakeTerm` already grades a lip's shelter
  continuously as it rises out of the wake — all of it up to a quarter of the boattail's drop in
  diameter, none from a half, a straight line between (ADR-030, from TN D-4014 p. 6 and Cubbage).
  The normal force now reads that same number as a weight on the method's share
  (`SupersonicBody::shape_weight`, which multiplies the Mach join's weight) instead of a threshold
  on it. Slender-body theory takes the rest, exactly as it does below the Mach join.
  - The run's *shape* does not change across the band: the lip stays inside the run, carrying
    nothing, and the weight carries the whole body to slender-body theory by the far edge. So the
    set of parts being blended never jumps.
  - **What is borrowed, and what is new.** The band and its grading are ADR-030's, already
    decided and already cited. Using that fraction as a weight on the *normal force* is new:
    ADR-030 fitted it to how much of a lip's own step pressure **drag** the wake removes, not to
    how much of the body's lift the method should carry. The narrower alternative — weighing only
    the lip's own share — is not available, because the lip's method share is already zero and a
    part-method, part-slender-body body is exactly the mixture ADR-034 refuses. So the whole body
    is weighed, which keeps the total an honest convex combination of two admissible whole-body
    models at the cost below.
  - **It removes the jump, not the disagreement.** At one fixed shape — a lip rising a quarter of
    the boattail's drop — the two models still differ by 33.0% of the normal force and 1.77
    calibres of centre of pressure, and nothing measured says which is right. What the band buys
    is that a rocket crosses that difference over its whole width instead of in a ten-thousandth
    of its geometry. As a design sensitivity that is still steep: over the rise band the printed
    force moves 29.1% and the centre of pressure 0.93 calibres (1.25 mm of lip radius), and over a
    gap of one boattail drop in diameter, 33.8% and 1.97 calibres. A dispersion run over a lip
    tolerance will see it.
  - **It is the whole wake fraction**, not the rise alone: the shelter fades with the rise, with
    any tube between the lip and the boattail, and with anything else in the way. Both are pinned.
  - **Two rough edges, recorded.** The weight clamps at 1, so the blend is continuous but has a
    corner there (about −0.95 calibres per millimetre on one side, flat on the other). And with
    two sheltered lips the run takes the smallest fraction, so raising one lip gives the other
    normal force the drag buildup says it does not have; one lip is the only case any committed
    design has, and a rocket with two wants its own decision.
  - **One threshold is left, at the far edge.** A share of shelter under a millionth counts as
    none (`SUPERSONIC_SHELTER_FLOOR`), because weighing the method in at less than that changes
    no number a reader could see and building its table costs half a second — which matters in
    Monte Carlo, where near-edge shapes actually appear. The step it leaves is that millionth of
    the difference between the two models.
- **Nothing measured moves.** Both committed Arcas Robin designs have lips rising 0.17 of their
  boattail's drop, inside the wake's full-shelter quarter, so their weight is exactly 1 and every
  aero fixture is unchanged — checked by `cargo xtask aero --check`.
- **What each remaining switch is worth, measured** (`issue_87s_switches_are_this_big` for the
  four of #87's list, `a_lip_in_a_boattails_wake_carries_nothing` for the lip's two rows). On the
  tests' rocket at Mach 3 and 4°:

  | drawing this | normal force | centre of pressure |
  |---|---|---|
  | a step in radius, past a billionth of the local radius | −8.7% | 1.03 calibres |
  | a flare behind the run, however small | −27.5% | 0.29 calibres |
  | a pointed tip past TN 3527 Fig. 2's 24° | −10.4% | 1.14 calibres |
  | a vertical tip steeper than the cap's handover to its base | −7.0% | 0.64 calibres |
  | a lip leaving its boattail's wake by rising or sitting back (a ramp since this ADR) | −29 to −34% | 0.93 to 1.97 calibres |
  | a lip longer than its boattail's drop in diameter, however little it rises | −33.0% | 1.77 calibres, forward |

  Across the first four the centre of pressure moves **aft**, so a rocket that trips one reads
  more stable. The lip rows go the other way — on that body the boattail takes enough lift off
  that slender-body theory's centre of pressure sits forward of the method's — so the direction is
  the body's, not the switch's; the size is what carries over. The last row is a switch this ADR
  does **not** smooth: shelter requires the lip to be no longer than the boattail's drop in
  diameter, the wake's own scale, and that length is a threshold. It is also the one the tests use
  to take a rocket off the method without changing a radius or an angle.

  M1.8e11 takes the two tips, where NASA SP-3007's cone tables reach 30° and retire Fig. 2's edge.
  The step and the flare are M1.8e12: nothing measures what either carries faster than sound, so
  there is nothing to blend toward and no band anyone can cite. The step's threshold is not the
  coverage gate's millionth of the area but the method's own element layout, which refuses two
  elements parallel but apart by more than a billionth of the radius — about 3e-11 m on the tests'
  rocket.

**Consequences.**

- A lip drawn taller, or set further back, moves a rocket between the two models smoothly, in the
  wake's own proportion; dispersion and optimisation see a slope instead of a cliff, of the size
  above.
- Where the weight is below 1 at every speed, a component's station and its own centre of pressure
  part company — the station blends stations, the force blends slopes and moments — by about two
  calibres at half weight. The moment is not affected: the station is only where a flight samples
  the local flow (`ω × p`), so the cost is an error in the incidence sampled there, of order
  `Δx ω / V` — about 1e-4 rad at Mach 3 and 1 rad/s — which is a fraction `Δx/(x − x_cg)` of that
  component's own pitch and yaw damping: appreciable for a part near the centre of mass, small in
  the total, which the fins dominate. The static margin is untouched.
  The mismatch is ADR-034's and has always existed inside the Mach join; M1.8e10 makes it
  reachable at every supersonic Mach number, and
  [issue #106](https://github.com/nrdptel/hpr-sim/issues/106) records it with its size.
- `SupersonicBody::shape_weight` is new public API, and `SupersonicBody::weight` now includes it.
- Five switches keep their sizes on record, in the guide and in this ADR, until M1.8e11 and
  M1.8e12 close or bound them: the four of issue #87's list, and the lip's own length, which that
  list did not name.

## ADR-042: Cone slopes from 24° to 30° come from Sims's tables, where TN 3527's chart stops (2026-09-20)

**Context.** The second-order shock-expansion method needs each tangent cone's normal-force slope
at `α → 0`. hpr read those from TN 3527's Fig. 2, a chart that stops at a half-angle of 24°, so
any body with a steeper tangent cone was refused outright and kept slender-body theory — one of
the model switches issue #87 tracks, worth −10.4% of the normal force and 1.14 calibres of centre
of pressure on the tests' rocket. A fineness-1 cone is 26.57°, so hpr could not fly one.

**Decision.** Past 24° the slopes come from J. L. Sims, *Tables for Supersonic Flow Around Right
Circular Cones at Small Angle of Attack*, NASA SP-3007 (1964), Table 2 (printed p. 20), at his own
grid of 25°, 27.5° and 30°.

- **It is the same theory, tabulated rather than drawn.** Sims solves Stone's problem with Ferri's
  correction and says his expressions are "identical to those found by Kopal" (p. 7); Fig. 2 plots
  Kopal's tables. Same reference area (the cone's base), same `γ` = 1.4, and his Mach grid
  contains all six of hpr's rows — 3, 4, 5, 6, 8 and 10 — exactly, so nothing is interpolated
  between the two sources.
- **The overlap is the check.** At 22.5°, the steepest angle both cover, this reading of Fig. 2
  and Sims's value differ by 0.0005 to 0.0021 per radian, the largest at Mach 6. The chart is read
  to about ±0.001, so where they disagree most it is the hand reading that is loose, not the
  theory; `sims_and_fig_2_agree_where_they_overlap` pins it.
- **Nothing below 24° moves.** The chart's columns stay as they are, so every number already
  validated against TN 3527's own tables is untouched and no committed fixture changes. Replacing
  them with Sims's throughout is a separate question, and a tempting one now that the overlap is
  measured; it belongs with M1.8e12, which revisits the tables.
- **The edge moves rather than disappearing.** A tangent cone past 30° is still refused, and that
  is still a switch: −7.7% and 0.81 calibres on the tests' rocket, against −10.4% and 1.14 at the
  old 24°. Sims's tables stop there.

**Consequences.**

- A fineness-1 cone flies the method, as does any nose whose tangent cones stay under 30°.
- One of issue #87's switches moves from 24° to 30° and shrinks; the guide's table records both
  the new size and the old.
- A blunt tip's handover is still capped at 24° ([`blunt_tip::MAX_HANDOVER_RAD`]) even though the
  slopes now reach 30°. Raising it changes what every committed blunt nose flies, so it is
  M1.8e12's decision, not a side effect of this one.

## ADR-043: The blunt tip's handover cap: what it is worth, and what stops it moving (2026-09-20)

**Context.** A blunt or vertical nose tip flies a Newtonian cap (TN D-4865) that hands over to the
second-order shock-expansion method where the surface slope falls to the largest angle a wedge can
turn the flow through with its shock attached, `δ_max` (ADR-038). hpr hands over at the lesser of
`δ_max` and a cap, `blunt_tip::MAX_HANDOVER_RAD`, 24°: the march reads the normal-force slope of
the tangent cone at the handover, and TN 3527's Fig. 2 stopped at 24°. Since ADR-042 those slopes
reach 30°, so the cap became a choice. `δ_max` passes 24° at Mach 2.06 and 30° at Mach 2.52, so a
30° cap keeps the report's own rule over that band instead of cutting it short, and above Mach
2.52 it still starts the march nearer the report's handover.

**Decision.** The cap stays at 24°, and becomes a parameter of the method
(`ShockExpansionBody::with_handover_cap_rad`, bounded by `blunt_tip::CONE_TABLE_CAP_RAD`) so that
the sweep is measured rather than argued. `cargo xtask aero` writes it to `handover_caps` in
`validation/fixtures/aero/blunt-tips.json`, and the guide's
[What the cap is worth](https://nrdptel.github.io/hpr-sim/physics/aero.html#what-the-cap-is-worth)
carries all three of its tables.

- **A steeper cap reads nearer the report's own case.** On TN D-4865's sphere-cone, the body whose
  handover rule this is, fitted as the tunnel measured it, the whole step from 24° to 30° reads
  nearer wherever a cap binds at all: +12.5% to +11.5% at Mach 2.96, +29.7% to +28.0% at Mach
  3.95, +7.5% to +7.1% at Mach 2.3. It is not monotone in the cap: 28° to 30° at Mach 4.63 reads
  further out, +30.9% against +31.3%. Below Mach 2.06 no cap binds and all four readings are the
  same. The Mach 2.3 and 2.96 rows also read a cone slope held at the tables' Mach 3 row, since
  that is where they start, so the band that most motivates the move is the softest evidence for
  it.
- **And it breaks the march on a nose that flattens fast.** On the Arcas Robin's committed
  power-series nose, a 30° cap puts 109 of 160 elements at Mach 4.63 and 145 at Mach 5 into
  TN 3527's `η < 0`, where the exponential law would run away from the tangent cone's pressure and
  hpr reduces the element to the generalized method (issue #81). The answer then follows the
  element count, and not even in order: 3.047 per radian at 10 elements, 2.928 at 40 and 3.260 at
  160, a spread of 0.33 or 11%, where under the flown cap the same nose moves 3.030 to 3.034, a
  part in a thousand. Through Mach 3.96 sixteen
  times the elements move the answer by under 0.01 per radian under the flown cap and under 0.013
  under the 30° one, so this is the top of the range only. At the flown 10 elements the 30° cap
  reduces 5 of them at Mach 4.63 and 9 at Mach 5.
- **The break sits at the flown cap's own edge, and it is not orderly.** It is not a 30° effect:
  at Mach 5 the element count is worth 0.046 per radian at 26° (40 of 160 reduced), 0.69 at 28°
  (140), and 0.055 at 30° (145), against 0.004 at 24° (2). 28° is the worst of the four, so no
  cap between the ends is a middle ground, and none above the flown one holds its answer to Mach
  5. The flown cap has little room to spare either: at the flown 10 elements the same nose first
  reduces an element at Mach 5.6024 (bisected), just past the range hpr's aerodynamics claim.
- **It is the method, not the arithmetic.** Which elements reduce is a decision on the sign of
  `η`, and none sits close enough to zero to turn on rounding: nudging the Mach number by eight of
  its last bits leaves the same elements reduced and the answer within a part in a billion
  (`a_steeper_handover_moves_the_march_out_of_its_range`). What drifts is the loading, not the
  pressure: under the 30° cap at Mach 4.63, `element_flows` gives the nose's last element a
  loading `Λ` (lift per unit length, per radian of angle of attack) of 0.1485 at 40 elements
  against 0.1590 at 80, where its pressure agrees to 0.0005. A reduced element transports `Λ` by
  the `λ₂/λ₁` ratio at each corner instead of relaxing it toward the tangent cone's, and how much
  of each the march does depends on how the nose is cut up.
- **Which way the pressure sits, measured.** Under the 30° cap at Mach 4.63 and 160 elements, 108
  of the nose's elements carry a pressure *above* their own tangent cone's, from 11.9% of the nose
  back, and 108 of those are 108 of the 109 reduced: the steeper start's pressure does not fall as
  fast as the tangent cone's does while the nose flattens, and the gradient behind each corner
  keeps driving it away from that cone. Under the flown cap not one nose element sits above its
  cone. The trouble is not an over-expansion at the handover, which is the first thing the
  geometry suggests.
- **`|η|` as a relaxation rate was tried and rejected.** Reading `η < 0` as a decay toward the
  tangent cone at the rate's magnitude — `ElementFlow::decay_rate` returning `self.eta_rate()
  .abs()` in place of `.max(0.0)`, a one-line change to measure by hand — leaves the blunt case
  just as loose (2.826 per radian at 10 elements against 3.095 at 160 at Mach 4.63) and makes
  TN 3527's own fineness-3 tangent ogive on a 7-calibre cylinder settle more slowly, 2.7865 to
  2.7131 per radian from 10 to 160 elements at Mach 6.28, where the flown reading moves 2.6992 to
  2.6984. That body is not one of the committed rows of `shock-expansion.json`, which step the
  afterbody by twos; the pair is in issue #108 so that M1.8e13 starts from a checkable baseline.
  hpr's reading is the report's own `η = 0` equations, and nothing measured here beats it.
- **Not chosen: raise the cap and disclose.** It would trade a converged answer at Mach 4 to 5 for
  a nearer one at Mach 2.3 to 4, and the guide would have to say the top of the range depends on
  the element count. A model whose answer moves with its mesh is not a model, and nothing forces
  the trade: 24° is a measured position, not a target that cannot be met.

**Consequences.**

- Nothing a rocket flies changes: the default cap is what it was, and no committed fixture moves.
- The cap's justification changes. It was Fig. 2's edge; it is now the march's range, measured
  over the whole sweep and recorded in the fixture, the guide and issue #108.
- Issue #108 holds what would let the cap move: a reading of `η < 0` whose answer stops depending
  on how finely the nose is cut. The vertical-tip switch of issue #87 — a nose steeper than the
  handover all the way to its base gets no method — waits on the same thing, since its edge is the
  cap.
- **The sweep's numbers are stored to six decimals, which loosens their check.** CI's Linux and
  Windows runs read the 30° cap's Mach 4.63 slope at 160 elements as 3.2597630456558506 where this
  machine reads 3.259763045663582 — 7.7e-12 apart, 2.4e-12 relative — against a fixture check that
  allows 1e-12 relative (`designs::same`). That is the reduced march amplifying the last bits of
  `exp` and `powf`, which each platform's library rounds its own way, and it is the number the
  march is least able to promise. So `handover_caps`, and only it, is written through
  `sweep_number`, which rounds to six decimals and refuses a value sitting on the rounding
  boundary. What this gives up is real and worth stating: within that section a later change that
  moves a number by up to about 5e-7 absolute now passes where 1e-12 used to fail. The guide's
  tables quote three decimals, so nothing a reader sees is affected; everything outside the
  section keeps the old check, and a sweep of the `starts` section found its worst 1-ulp-of-Mach
  sensitivity at 3.2e-14 relative, so it needs nothing.
- **The milestone numbers move with the split.** What ADR-040 to ADR-042 call M1.8e12 — moving the
  handover past 24° — is M1.8e13 from here, with its *done when* carried over word for word; the
  step and the flare become M1.8e14. This increment, the measurement, takes the M1.8e12 number.
- The sweep runs four caps over eight Mach numbers at three element counts each time
  `cargo xtask aero` runs, about four seconds of it.

## ADR-044: What the answer follows when it follows the mesh is a crossing of the tangent cone, not a reduced element (2026-09-20)

**Context.** ADR-043 left the blunt tip's handover cap at 24° because a steeper cap puts the
second-order shock-expansion march into readings whose answer follows the element count instead of
settling, and recorded the blocker as issue #108: *a reading of `η < 0` whose answer settles as the
nose is cut finer*. That framing came from the observation that a steep cap reduces most of the
nose to the generalized method (`η < 0`, TN 3527 p. 13, issue #81). M1.8e13 was to move the cap
once such a reading existed. It cannot start there, because the framing is wrong.

**Decision.** Record the measured cause, and re-aim the work at it. The count that says an answer
cannot be trusted is `ShockExpansionBody::tangent_cone_crossings`: how many times the marched
surface pressure passes through its own tangent cone's. `cargo xtask aero` stores it beside every
reading of the cap sweep in `validation/fixtures/aero/blunt-tips.json`, and the guide's
[What a crossing is, and what it costs](https://nrdptel.github.io/hpr-sim/physics/aero.html#what-a-crossing-is-and-what-it-costs)
explains it. Issue #108 is re-scoped to the crossing, and what is left of the old M1.8e13 — the
reading, then the vertical-tip switch of issue #87 — becomes M1.8e16, after the flare and the step,
because it is the only one of the three that is blocked.

- **A crossing is counted within one segment only.** Where a nose meets a cylinder, a boattail or
  a flare, `p_c` itself steps — to the free stream's pressure, to footnote 8's, or to a steeper
  cone's — so the gap can change sign without ever passing through zero and there is no pole.
  Counting those would report a crossing on an ordinary boattailed or flared design, which was the
  first rule's mistake. Within a segment the profile is continuous, so `p_c` is too.
- **A crossing is a pole in the rate, and it separates the sweep's three meshes exactly.** Along an element the
  method relaxes as `e^(−η)` with `η = k (x − x₂)`, the rate per unit length being
  `k = (∂p/∂s)₂ / ((p_c − p₂) cos δ₂)`. A crossing closes `p_c − p₂` while the gradient carries on,
  so `k` runs to infinity. The pressure is unharmed: `k (p_c − p) cos δ₂` is the gradient, which
  stays finite. The loading is not, because eq. 19 relaxes it at the pressure's `k` while its own
  gap `Λ_c − Λ` does not close with it, so at a crossing the loading is driven onto the tangent
  cone's arbitrarily fast. A march applies `k` from one corner across a whole element, so it takes
  one step of whatever size the mesh gives it: at Mach 4.63 under a 30° cap the element at the
  second crossing sheds 98% of the loading's gap in its own length on the 40-element march and 12%
  on the 160-element one. Over the sweep's thirty-two readings — four caps at eight Mach numbers on
  one nose — twenty-seven have no crossing and hold to 0.012 per radian across 10, 40 and 160
  elements; the five that cross move by at least 0.035, up to 0.69. No overlap, and nearly three
  times (2.9×) between the two groups
  (`over_the_sweeps_meshes_a_crossing_separates_the_readings_that_move`).
- **A crossing is a flag, not a verdict, and the ADR claims no more.** It does not prove an answer
  never settles: 28° at Mach 5 crosses at every mesh, and its 0.69 spread is all in the coarse end
  — from 60 elements to 640 it holds to 0.005 per radian, tighter than the worst reading that never
  crosses, where 30° at Mach 4.63 moves by more than 0.2 over the same range
  (`a_crossing_says_the_answer_moved_not_that_it_never_settles`). Nor does a
  zero prove the opposite: 26° at Mach 5 and 28° at Mach 4.63 read zero at the flown 10 elements
  per curve and two at 40 and 160, and both move; the count is not even monotone in the mesh. What
  is claimed is only what the sweep measured: across its three meshes the crossings, and only the
  crossings, mark the readings that move. The count is not consulted during a flight, and the
  sweep is one nose, so another blunt nose above Mach 4 could cross under the flown cap without
  saying so. The guide says all of this.
- **Reduced elements do not move an answer.** TN 3527's fineness-3 tangent ogive reduces 27 of 160
  elements at Mach 5.05 and 50 of 160 at Mach 6.28, and settles to 0.002 per radian from 10
  elements to 160. There `η < 0` comes from the *gradient* changing sign, with the surface pressure
  below its tangent cone's the whole way; the gap never closes, so the rate stays bounded
  (`a_reduced_element_settles_where_tn3527s_own_bodies_never_cross`). That ogive does not cross at
  either Mach number hpr can check it against the report at, which is why the report could state
  its condition (gradient and `p_c − p₂` of one sign, p. 13) and stop: it never had to say what a
  crossing does.
- **Issue #108 was asking half a question.** Its trouble does not begin on the `η < 0` side. On the
  Mach 4.63 march under a 30° cap the step is taken at the *second* crossing by an element that is
  inside the method, not reduced, and how much of the loading's gap it sheds in one step is the
  mesh's answer: 98% on 40 elements against 12% on 160. But the gap it sheds — the loading standing
  about a quarter above its tangent cone's — was opened by the reduced stretch behind it, which is
  hpr's `η = 0` reading and not
  the report's rule. So the two are tangled: a different reading of `η < 0` changes the size of the
  step, and neither can be judged without the other. What is settled is that a rule for `η < 0`
  alone is not obviously enough, which is why `|η|` failed in ADR-043 while leaving the crossing
  where it was.
- **Not chosen: clamp the rate, or the exponent.** Bounding `η`, or the exponent `η Δx` an element
  may apply, would make every answer settle, and would be arbitrary at exactly the point where the
  physics is unknown — the bound, not the method, would then set the loading through the crossing.
  What is missing is a statement about the loading where the pressure gap closes, and a clamp
  hides the question instead of answering it.
- **Not chosen: renumber the milestones.** Making the remainder M1.8e14 would push the flare to
  e15 and the step to e16, and would leave ADR-043's record of the last split describing entries
  that no longer exist. Milestone ids carry one increment level, so the remainder takes the next
  free number, e16, and the roadmap's order does the rest.

**Consequences.**

- Nothing a rocket flies changes. The cap is still 24°, and no committed number moved except the
  new count beside each reading of the sweep.
- `ShockExpansionBody::tangent_cone_crossings` is public, and its documentation says plainly that
  a non-zero count means the answer is the mesh's rather than the model's. `reduced_elements`
  stays, and is no longer the thing to read for that.
- The guide's *What the cap is worth* now carries the crossing count in all three of its tables
  and a section explaining it, so a reader meets the real cause where they meet the numbers.
- **Issue #108 is re-scoped, not closed, and the gap stays visible.** What would close it is a
  reading of the loading through a crossing that settles as the nose is cut finer. Nothing
  measured here is one. The blunt tip's handover past 24°, and issue #87's vertical-tip switch
  with it, wait on that as M1.8e16; the flare (M1.8e14) and the step (M1.8e15) do not, so they go
  first.
- **The counts are integers from a bare sign test, and that is safe here by a wide margin.** The
  sweep's floats are rounded to six decimals because a reduced march is only reproducible to about
  2.4e-12 relative across platforms (ADR-043). A crossing count has no such rounding: it is the
  sign of `p_c − p₂`. A test holds the gap either side of every crossing above 1e-8 of the
  pressure, four orders clear of that drift, so the committed integers do not turn on a last bit
  (`a_crossing_is_a_pole_in_the_rate_the_march_relaxes_at`).
- The sweep costs one more march per cell to count crossings, about a second of `cargo xtask aero`.

## ADR-045: Where a flare's march stops is the corner's isentropic turn, not the shock detaching (2026-09-20)

**Status:** accepted. **Milestone:** M1.8e14, which is the first of the three the old M1.8e14 splits into.

**Context.** M1.8e14 is to fly a flare through the second-order shock-expansion method, which
already marches one: a 2.75° cone, a tube and a conical flare return a normal-force slope at Mach 3
(6.55 per radian on the body's cross-section for an 18.5° flare). What stops a flared rocket today
is the model around the method, which ends its run at the first widening body. The milestone's
*done when* asks for a flared body that flies the method **where the flare's shock is attached**,
with no jump across that boundary, so the first question is where that boundary is and what the
march itself does on either side of it. NASA TN D-4865 tested exactly this shape and says its
Mach 1.50 run is past attachment even in theory.

**Decision.** Measure the edge before moving it. M1.8e14 bisects, to f64 resolution, the steepest
flare the method marches over Mach, on a pointed 2.75° cone with five calibres of tube and a
conical flare. Both angles are TN D-4865 model 2's; the layout is not — that model is blunt-nosed
with no tube — and the edge depends on the body ahead of the flare, which is what sets the flow
reaching it. Two tests in `crates/hpr-aero/src/shock_expansion.rs` pin the edge and what makes it:
`a_flare_marches_to_the_isentropic_turn_not_to_detachment` and
`past_mach_2_13_the_flare_stops_where_the_cone_tables_do`.

**What the edge is made of.** Two different things, one below Mach 2.13 and one above.

- **The corner's isentropic turn.** Second-order shock-expansion fixes the pressure just behind a
  corner from the Prandtl and Meyer turn there (TN 3527 pp. 7-8, the first of the three conditions
  on eq. 3; `ν` itself is NACA 1135 eq. 171c), so the march stops where the turn would take the
  flow reaching the flare to Mach 1: not where an oblique shock would detach. The two are different
  angles, and neither bounds the other.

  | free stream | the method marches to | a wedge's shock detaches at (NACA 1135) | the method is |
  |---|---|---|---|
  | Mach 1.5 | 11.9312175° | 12.1126689° | 0.1814514° short |
  | Mach 2 | 26.4714031° | 22.9735318° | 3.4978713° past |

  They cross at **Mach 1.547787962528**, both at 13.346819°, bisected to f64 resolution. Below it the method refuses
  flares whose shock is attached; above it the method answers for flares whose shock is not.
- **The cone tables' 30°.** From **Mach 2.129702032593** up, the limit is not the flow at all: the
  tangent cone at each element is looked up in NASA SP-3007 Table 2, which stops at 30°
  (ADR-042), so every Mach number above that
  gives the same edge, 30° plus the millionth of a degree `cone_normal_force_slope` admits for the
  degree conversion's rounding. At Mach 2.5 the flow could turn 29.8° before its shock detached and
  far more before it went sonic; the reference data is what runs out.

**What happens where the shock is detached.** The march keeps marching, and returns a number. It
is a number from an isentropic compression through a corner that a real flow crosses through a
detached bow shock standing ahead of the juncture, with a subsonic pocket behind it, so it is
wrong in a way the march cannot report. Taking a wedge's limit as the stand-in, the band on the
body above is narrow but real for an 18.5° flare, the report's angle: the method answers from
**Mach 1.721760**, and a wedge's shock reaches 18.5° only at **Mach 1.767575**. TN D-4865's Mach
1.50 run is below both, where the method refuses outright.

**Consequences.**

- **M1.8e17 cannot use the march's refusal as its attachment test.** "Where the flare's shock is
  attached" has to be decided by a detachment criterion of its own, evaluated on the flow reaching
  the flare, and the method's own edge sits on both sides of it. That is now what M1.8e17 starts
  from.
- **The criterion itself is still open, and M1.8e17 owns it.** The wedge's largest deflection is a
  conservative stand-in here, not the answer: a cone's shock stays attached to angles a wedge's
  cannot hold (NACA 1135), and a conical flare on a cylinder sits between the two. Every number
  above is stated against the wedge's limit and says so; none of them is claimed as the flare's own
  attachment boundary.
- **Nothing a rocket flies changes.** The run still stops at the first widening body, so no
  committed number moves. The tests are new, and they are the baseline M1.8e17 has to keep.
- **The 30° cap is a limit of the reference data, and the guide says so where it appears.** It is
  the same cap that bounds a pointed tip (ADR-042),
  met from a second direction.

- **The two halves left over take the next free numbers, e17 and e18, and go straight after
  e14.** A milestone id carries one increment level, so `M1.8e14a` is not an id the roadmap's own
  check accepts (ADR-043 settled the same point for the e12/e13 split). The old e14's remaining
  clauses are carried over word for word into M1.8e17 and M1.8e18, and both sit immediately after
  M1.8e14 in the list, which is the execution order: the flare's three pieces stay contiguous and
  nothing that was ahead of them moves behind them.

**Not chosen: let the flare into the run now and measure afterwards.** The boundary is the thing
the *done when* is about; opening the run first would have meant choosing an attachment test with
nothing measured to choose it against, and the two edges above show how easily that choice goes
wrong in both directions.


## ADR-046: Debrief folded in, and flight-log analysis that stands without the simulator (2026-09-20)

**Context.** Debrief (`nrdptel/fusionspace-debrief`) is the owner's own MIT-licensed browser
flight-log analyzer, now being sunset for the same reason Loft was: the web application became the
product and the library under it did not. What it knows, though, is real and expensive to
rediscover — ten logger families parsed byte by byte, a catalogue of readings with the method and
caveats of each written up against published sources, and a corpus of real logs
(`nrdptel/debrief-fixtures`, private) that those parsers were proved on.

hpr-sim already planned to read flight logs: Phase 5 (M7.1 to M7.4) covers importers,
reconstruction, parameter identification and fault diagnosis, and the vision lists flight
forensics. So the use case is not new. What is new is the owner's requirement that it also be
usable **on its own**: a user who flew a rocket, has a log, has no design file and does not want to
simulate anything should be able to use this project as a flight analyzer and nothing else.

The crate map already had `hpr-flightdata`, and it depended on `hpr-sim`. The crate is still an
empty skeleton, so the dependency had cost nothing yet — but it is exactly the edge that would have
made a standalone analyzer impossible, and it would have been expensive to remove once M7.1 had
filled the crate.

**Decision.**

- **Debrief's use case is folded into hpr-sim**, as prior art for Phase 5 rather than as a new
  direction. The repository is mirrored into the gitignored `refs/fusionspace-debrief` through the
  reference lock, the private corpus into `refs/debrief-fixtures`, and both have rows in
  `THIRD-PARTY-NOTICES.md`. What comes across is the format knowledge, the reading methods and
  their citations, and the corpus. What does not is the web application: the Next.js app, its
  components and its deployment stay behind. A user interface waits for Phase 7, as it already did.
- **`hpr-flightdata` must not depend on `hpr-sim`.** It depends on `hpr-core` and `hpr-atmos`
  instead — an atmosphere is needed because Mach number, dynamic pressure and pressure altitude are
  meaningless without one. The rule is declared in the crate's own manifest, where the dependency
  would be added, and enforced by `cargo xtask wasm-check`:

  ```toml
  [package.metadata.hpr]
  forbids = ["hpr-sim"]
  ```

  The check walks the workspace graph, not just direct dependencies, and fails with the path it
  found (`hpr-flightdata -> hpr-analysis -> hpr-sim`), because the way this rule breaks is a
  forbidden crate arriving through a helper that looked harmless. It follows normal, build and
  target-specific dependencies, and an optional one whose feature is off, since a crate that stands
  on its own must do so in every configuration. **Dev-dependencies are excluded**, because they
  never reach anyone who depends on the crate: a test in `hpr-flightdata` may fly a simulated
  flight and compare it with a parsed one. The mechanism is general, and it reads outwards — a
  crate names what **it** must not reach, not who may not reach it.
- **`hpr-forensics` is added to the crate map**, depending on `hpr-flightdata`, `hpr-sim` and
  `hpr-analysis`. It is the only place a reading taken from a log and a number the simulator
  produced meet.
- **Phase 5 is re-cut along that boundary.** M7.1 (importers) and M7.2 (readings, reconstruction
  and ghost data) are analyzer-only and must work with no design file present; sim-versus-real
  residuals move out of M7.2 and into M7.3, which with M7.4 lives in `hpr-forensics`. No milestone
  ids change.
- **Provenance travels with a reading.** Every reading says whether it was measured by an
  instrument, derived from what was measured, or clipped because the sensor saturated; a reading
  the log cannot support is withheld with a reason rather than printed as a number. Two recordings
  of one flight are read side by side as independent measurements and never averaged into one.
  M7.2's *done when* holds these, and M7.3's says a reading keeps its provenance where it meets a
  simulated number.
- **`hpr analyze <log>` is added to M4.2's command list**, taking no design and running no
  simulation, so the standalone use has a way in that is not "write a program".
- **What was mirrored is written up now, not when M7.1 starts**, in
  `docs/research/debrief-log-formats.md` and `docs/research/debrief-flight-readings.md`. The
  knowledge is perishable once nobody touches those repositories again.
- **The clean room is drawn inside the mirror, not around it.** Debrief's `lib/`, its tests and
  its public fixtures may be ported. Its `COMPETITION.md` may not: two of its rows carry
  OpenRocket's simulation-status vocabulary and behaviour read out of named GPL-3 Java files, with
  links, which CLAUDE.md rule 3 does not allow as an input to this project. That exclusion is
  recorded in the formats note and applies to M3.1 as much as to Phase 5.

**Consequences.**

- The layering is enforced from today, while the crate is empty, rather than argued about later.
  The check was confirmed to fail on a direct `hpr-sim` dependency and on one reached through
  `hpr-analysis`, and to pass once both were removed.
- Phase 5's documentation and *done when* clauses roughly double: each analyzer milestone now has
  to show it works with nothing but a log. That is the cost of the promise.
- The accuracy story forks. Sim-versus-reference error and reading-extraction error are different
  claims measured against different references, and the validation report will have to keep them
  apart rather than average them into one number.
- Debrief drew a hard line — a measurement instrument, never a predictor — and merging it into a
  simulator is what could erase it. The layering rule, the provenance fields and the separate
  forensics crate are that line rewritten as structure rather than intent.
- **The `.ork` parser's provenance was checked, not assumed, because the owner did not answer the
  question before the work started.** Its header states it was written from OpenRocket's published
  file-format page, which names the ten `flightdata` attributes, and that the `status` vocabulary
  was deliberately left alone because the page defers it to the GPL-3 reference implementation; the
  code bears that out, carrying `status` verbatim and never branching on it. Two things the header
  gets wrong about itself, found by reading the code beside it: the `<databranch>` series **is**
  read, by exact column name, which a later change added — from the file's own `types=` header, so
  the clean room still holds — and only one of the ten attributes' units is proved rather than
  inferred. Porting it is allowed; repeating its header's claim verbatim is not.
- OpenRocket's own example design is GPL-3, which is why Debrief ships no public `.ork` fixture.
  hpr-sim may not vendor one either, and M3.1 will need its own sample designs.
- The corpus is other people's flight logs. It is fetched, never committed, and only counts, error
  statistics and anonymised case ids are published from it, exactly as the Loft fixtures are. The
  twelve fixtures Debrief ships publicly in its own repository are the ones that may appear in
  examples and doctests.

**Not chosen: start Phase 5 now.** The knowledge is perishable, so it is mirrored and written up
now, but M1.8's aerodynamics is mid-flight and the roadmap order holds. Whether M7.1 should move
ahead of some of Phase 3 and 4 — real flights are what validation ultimately wants to compare
against — is left open rather than decided here.

**Not chosen: port the TypeScript.** Around 3 MB of it exists, and it is the owner's own MIT code,
so it may be ported freely with a note. But a browser analyzer's code is not a Rust library's
code; what is worth carrying is the format knowledge and the methods, which the research notes
record, not the implementation.


## ADR-047: A flare flies the method where its corner's shock is attached, and is read drawn out where it is not (2026-09-20)

**Status:** accepted. **Milestone:** M1.8e17, the second of the three the old M1.8e14 split into.

**Context.** M1.8e17 asks for a flared body that flies the second-order shock-expansion method
**where the flare's shock is attached**, with no jump across that boundary. The march has always
been able to take a flare; what refused one was the model around it, which ended its run at the
first widening body and then dropped the *whole* body to slender-body theory at every Mach number
— worth −27.5% of the normal force and 0.29 calibres on the tests' rocket at Mach 3.

M1.8e14 measured the march's own edge first (ADR-045) and found it is the corner's isentropic turn
running out, not the shock detaching, and that it lands on **both** sides of a wedge's detachment
angle depending on the tube ahead of the flare. So the attachment test had to be chosen here, not
read off the march's refusal.

**Decision.**

- **The test is the corner's, read at the flow reaching it.** A flare's shock springs from a
  circular corner, not from a point apex, so where it forms the flow is two-dimensional: the
  body's radius is the scale over which the axisymmetric relief acts, and at the corner itself
  there is none of it yet. hpr therefore takes NACA Report 1135's largest deflection behind an
  attached plane oblique shock (eq. 168 into eq. 138,
  `blunt_tip::wedge_detachment_angle_rad`) — **the same test TN D-4865 p. 5 uses to hand a blunt
  tip's Newtonian cap over to this method**, so the project now applies one attachment rule at
  both corners it has.

  It is read at the surface Mach number the march delivers to the corner, not at the free stream:
  a new `ShockExpansionBody::aft_flow` returns it, and the march is downstream-only, so the flare
  cannot change it. On the tests' flared rocket that flow is Mach 1.9998 in a Mach 2.0 free
  stream, and the limit 22.9698° rather than 22.9735°.

  A cone's shock holds to steeper angles than a wedge's and a conical flare on a cylinder sits
  between the two (ADR-045), so **if** that is where a flare's own boundary lies, this errs one
  way only: it stops reading some flares whose shock is still attached, and reads none whose shock
  is not. "Conservative" is the wrong word for what that buys, though. Erring low does not mean no
  reading — it means the reading of a different body, and nothing here measures how different.

- **Past the limit, the flare is read drawn out, not dropped.** A flare steeper than the limit is
  read as one of **the same radii** drawn out to the limiting turn — a longer, shallower flare —
  with its centre of pressure kept on the real flare, at the same fraction along it. This is
  Washington and Pettis's boattail rule turned around (ADR-040: a boattail past 16° reads the
  correlation of one of the same radii drawn out to 16°), and it is what makes the reading
  continuous: **at the limit the drawn-out flare is the real flare**, so the two branches meet by
  construction. The radii, which set how much air the flare turns, are never changed.

- **The 30° of the cone tables caps the angle, not the turn.** Past `CONE_TABLE_CAP_RAD` an
  element has no tangent cone to look up (NASA SP-3007 stops at 30°, ADR-042) — but that is a
  bound on the flare's *surface angle*, while the shock bounds the *turn* at its corner. They are
  bounds on different things, so each caps its own quantity: `flare_corner_limit_rad` returns the
  turn, and the model takes `min(turn + the angle ahead, 30°)` as the angle it draws. On a flare
  behind a cylinder, where the angle ahead is zero, the two coincide and the tables bind from Mach
  2.5192034260 up.

- **The run ends at the flare.** Nothing behind a flare is marched, and it takes slender-body
  theory's share, which for a cylinder is nothing. Keeping the run open would have meant marching
  parts whose stations move when the flare is drawn out.

- **Only a conical flare joins the run, and only in the free stream.** Any other widening shape
  ends the run as before: the corner's turn is then the profile's own slope at its fore end, which
  this test is not written for. A widening part behind a boattail stays a lip in its wake under
  ADR-039, whose flow the march does not compute.

- **The old rule stays selectable.** `BodyModel::with_supersonic_flare(SupersonicFlare::SlenderBody)`
  reproduces every number from before this milestone, and `BodyModel::BEFORE_M1_8E6` takes it.

**What it is worth.** On the tests' flared rocket — an ogive nose, a tube, a 0.3 m conical flare,
a tail tube and four fins — the method reads *less* normal force than slender-body theory and puts
the centre of pressure forward of it, so a flared rocket now reads **less** stable, not more:

The rocket is an ogive nose 0.25 m long on a 27 mm radius, a 0.7 m tube, a 10° conical flare
0.3 m long opening to 79.9 mm, a 0.2 m tail tube and four fins; the reference is its largest
diameter, 0.1598 m.

| | the method | slender-body theory | the method's centre of pressure |
|---|---|---|---|
| Mach 2.0 | 3.5371 per rad at 7.365 calibres | 3.7153 per rad | 0.089 calibres forward |
| Mach 3.0 | 2.8899 per rad at 7.015 calibres | 3.0815 per rad | 0.169 calibres forward |
| Mach 4.95 | 2.4880 per rad at 6.681 calibres | 2.6431 per rad | 0.238 calibres forward |

Nothing here compares either column with a measured flare. That is M1.8e18, which commits TN
D-4865 model 2's readings, and until it lands the guide says the size is unvalidated.

**That it does not jump.** At Mach 2.0 — a row of the table, so the reading is that row's and not
an interpolation — the boundary is a flare of 22.969761173077°. Probing either side of it:

| probe | the normal-force slope moves by |
|---|---|
| ±1e-9° | 4.527e-11 |
| ±1e-7° | 4.527e-9 |
| ±1e-5° | 4.527e-7 |

Each figure is a fraction of the slope itself. The gap falls a hundredfold for each hundredfold in
the probe, so what the probe finds is a slope, not a step: the reading is continuous across the
boundary.

**Its first derivative is not, and this decision does not claim it is.** A cap makes a kink: below
the boundary the flare's angle moves the body the march sees, above it only the radii do. Measured
at Mach 2 on the tests' flared rocket, `dC_Nα/dδ` changes by **−31.4%** across the boundary on the
whole rocket and by **−141.6%** on the flare's own share, where it changes sign. The marched branch
is not smooth in the flare's angle either — the same probe at 20°, away from any boundary, finds
+3.5% and +41.1% — so the kink is a change of degree, not of kind.
`the_cap_makes_a_kink_in_the_slope_even_though_the_reading_holds` pins all six figures. ADR-039
claimed only "continuous in shape" for the same construction, and that is the claim here too.

The same probe in the Mach number, at 18.5° — TN D-4865 model 2's angle, whose shock holds on this
body from Mach 1.767666917849 — gives 3.622e-10, 3.622e-8 and 3.622e-6. That half is taken on the
**rows** the table is built from rather than on a reading between them: the crossing falls inside
the Mach 1.75 to 1.80 interval, where `SupersonicBody::share` runs a straight line, so a reading
there would look continuous whatever the two branches did.
`nothing_jumps_where_the_flares_shock_detaches` pins both halves.

**What still switches, and it is not the boundary above.** Where the march itself refuses a flare,
the model keeps slender-body theory rather than reading a flare it has not marched — and that
refusal is a switch of shape. Three cases, which behave differently. *(The second and third of
them are superseded by ADR-050, which reads that element rather than refusing it; the two tests
named below were replaced with it. The first is unchanged.)*

- **Below Mach 1.5552** on this body the corner's isentropic turn runs out before its shock
  detaches (ADR-045's crossing), so the table simply starts there. The join's weight rises from
  zero over 0.3 Mach, so the reading is continuous in Mach through it — measured at 8.5e-10 for a
  1e-9 probe.
- **A flare of about 0.03816° to 0.05882°** — a rise of a third of a millimetre over 0.3 m — has
  its one element *reduced* aft of the nose (issue #81: the pressure behind the corner moves away
  from its tangent cone's), and the march then refuses a run of Mach numbers from the top down:
  none at 0.0589°, from Mach 5.0 at 0.045°, from Mach 4.70 at 0.03817°. The table is built
  downward from Mach 5 and needs the join's whole width inside it, so once that run reaches Mach
  4.70 there is no table at all and the whole body falls back to slender-body theory at every Mach
  number. (Shallower than the band the refusal only shortens the table and lifts the join, which
  stays continuous.) Crossing the steep edge is worth −8.3% of the normal force and 1.16 calibres
  at Mach 3 and 4°, the size of the switches issue #87 already tracks. It is **not** in #87,
  because M1.8e15's *done when* is to close or narrow that issue to the step alone: it has its own
  issue, #117, and its own milestone, **M1.8e19**.
  `the_march_refuses_two_bands_of_flare_and_the_model_keeps_slender_body_theory` pins the
  mechanism either side and the switch's size.

  The band's two edges are quoted to five digits because that is about what they carry. Each is
  the root of the march's own refusal at a fixed Mach number, which turns on the sign of
  `p_c − p₂`, a difference of two pressures within a thousandth of each other there; the
  cancellation leaves the root about nine significant digits, and the three platforms CI runs
  spread the shallow edge over 2.6e-10° (6.8e-9 relative). Bisecting it to f64 is a statement
  about one machine, not about the model.
- **Shallower still, from about 0.0009°, the join's start steps rather than slides.** The refused
  run of Mach numbers is what the table stops at, so as the flare flattens the table's start walks
  up in 0.05 Mach steps — and not monotonically: 0.00024° and 0.0003° sit at the floor, Mach 1.2,
  while 0.00025° between them does not. The shallowest switch that stays switched is at
  **0.00090182°**, a rise of 4.7 µm over 0.3 m, where the start goes from Mach 1.2 to Mach 2.2:
  −4.6% of the rocket's normal force and 0.75 calibres at Mach 2.
  `a_near_flat_flare_lifts_the_joins_start_in_steps` pins it. This is the same defect as the band
  above, one step down in angle, and #117 and M1.8e19 carry the whole near-flat region, not just
  the band.

**Not chosen: fade the method out approaching detachment.** A weight going to zero at the
boundary would also be continuous, but it throws away the method exactly where it is still valid,
and it needs a fade width nothing measures. ADR-039 rejected the same construction for a boattail,
for the same reason.

**Not chosen: cap the flare's angle by shrinking its rise.** Reading a 30° flare as a 23° one of
the same *length* throws away most of the radius change, which is what sets how much air the flare
turns: the reading would fall far below even slender-body theory's, which depends on the radii
alone. Drawing the flare out keeps the radii and changes only the angle, which is the quantity the
attachment test is about.

Note what that means above the limit: with the radii fixed, the drawn body no longer depends on
the flare's angle at all, so **every flare steeper than the limit reads the same force**. Held at
the limit, in other words, which is the honest description of the rule. There is no bound on how
far that goes: at Mach 2 a 75° flare — an annular face a detached bow shock stands in front of —
reads 1.638 per radian against slender-body theory's 2.010, 18.5% below, where the truth is above
both. It reads a steep flare as less stabilizing than it is, which is the safe direction for a
stability margin and the wrong one for a load. The guide says so.

**Not chosen: read a detached flare by a detached-shock model.** There isn't one here. A bow shock
standing ahead of the juncture with a subsonic pocket behind it is not something a tangent-body
march describes, and inventing one would have been a model with nothing to check it against.

## ADR-048: What a marched flare is worth, measured against TN D-4865's model 2 (2026-09-20)

**Status:** accepted. **Milestone:** M1.8e18, the last of the three the old M1.8e14 split into.

**Context.** ADR-047 gave a flared body the second-order shock-expansion method and compared it
with nothing measured: the guide had to say, in its own opening paragraph, that no measured flare
force had been compared with any of it. M1.8e18 asks for TN D-4865 model 2's readings, committed
with their provenance, and for the guide to say what a marched flare is worth and what it leaves
out.

Model 2 is the only flared body in this project's sources whose **normal force and pitching
moment** are printed rather than only its pressures: a blunt 2.75° cone with an 18.5° flare, in the
Langley Unitary Plan tunnel at Mach 1.50, 1.90, 2.30, 2.96, 3.95 and 4.63, with fig. 8(b) plotting
`C_N`, `C_m` and `C_A` against `α` from 0° to 12°. It is the same report, the same figure and the
same tunnel as model 1, whose readings M1.8e7 already committed for the blunt tip's cap — so the
two can be read, fitted and compared the same way, and the difference between them is as close as
this source gets to "with a flare and without".

**Decision.**

- **The readings are committed as `tn-d-4865-flared-cone.json`**, read from fig. 8(b) by the pixel
  pipeline M1.8e7 wrote for fig. 8(a), with the grid fits' residuals, the per-circle uncertainty
  and the report's own statements about the flow stored beside them. `C_N` and `C_m` are read;
  `C_A` is not, because hpr's supersonic drag is a separate model this milestone does not touch.
  The `α` = 0 circles read −0.0039 to +0.0043 where they should read 0, which is the plotting's own
  accuracy.

- **The drawing is closed on the base, not on the printed lengths.** Fig. 3(b)'s printed
  dimensions are mutually inconsistent at the 1% level: the nose derived from its three radii is
  0.3429960 diameters long, 0.3429960 + 0.743 + 0.523 = 1.6090 is the printed length exactly, but
  the two printed half-angles then carry the base to 1.0084 diameters rather than 1.000. hpr keeps
  the nose, both half-angles, the length and the base — every quantity the measured coefficients
  are divided by, and every angle the flow turns through — and solves for the split of the rest.
  The alternative, keeping the printed lengths and scaling the body to a 1.000 base, is computed
  too and published beside it, and so is a third that keeps the printed lengths **and** the base
  and gives up the flare's stated half-angle instead (18.5° → 18.0864°) — the one closure that
  moves the quantity the drawing actually contradicts, since the cone closes on its own printed
  numbers and the flare does not. Over all three the spread is 0.64 points in the slope and 0.0043
  calibres in the centre of pressure, and Mach 1.50 is refused in every one, so the comparison
  does not rest on the choice. The angles are kept because they are the report's *text* (printed
  p. 8), stated to three decimals, while the lengths are only its drawing; the drawing's fourth
  longitudinal dimension, 0.722 from the blend arc's centre to the juncture, is recorded as an
  independent check on the derived nose and agrees to 0.0004 diameters.

- **A blunt nose may take more than one segment, and the cap may hand over on a later one — but
  never past the nose.** Model 2's nose is a 0.257 sphere blended into its cone by a 0.429 arc
  whose centre sits 0.135 below the axis, and the sphere is still at 38.3° where the arc takes
  over, steeper than the handover's 24° cap at any Mach number. `ShockExpansionBody::handover_m`
  therefore searches the nose's segments instead of the first segment alone, and `lay_out` skips
  the segments a cap covers wholly. The same run says which elements the rule on a reduced element
  treats as the nose's.

  What counts as the nose is deliberately narrow: normally the first segment and nothing else,
  and behind a **spherical cap** — the one shape that is a piece of a nose rather than a whole one
  — the curved segments that follow it, stopping at the first that is straight *or narrows*. So a
  pointed nose followed by a curved widening transition reads exactly as it did before, and a cap
  that reached a cylinder or a boattail is still refused: it would hand the flow over at no angle
  at all, with none of the total pressure the tip took out of it. No committed number moved, and
  `a_blunt_nose_hands_over_on_a_later_segment_but_never_past_the_nose` pins all four cases.

- **What it is worth is published as three tables, with no target.** M1.8e18 set none, and none is
  invented here. hpr reads model 2's normal-force slope −1.9% at Mach 1.90, +7.0% at 2.30, +13.4%
  at 2.96, +51.5% at 3.95 and +50.4% at 4.63, with the centre of pressure within 0.05 calibres
  through Mach 2.96 and 0.088 at 3.95. Model 1, unflared, reads −1.2%, +0.0%, +7.5%, +12.5%,
  +29.7% and +32.1% over the same six rows, so **the flare adds between −1.9 and +0.9 points
  through Mach 2.96 and 21.7 and 18.3 points at 3.95 and 4.63**. That is where the report's own
  shadowgraphs show the laminar boundary layer separating ahead of the juncture (printed p. 10),
  and where the measured `C_Nα` itself falls from 1.594 to 1.270 while both attached-flow methods
  on the figure stay between 1.57 and 1.96. Read as **consistent with** separation, not measured
  by it: the report blames the separated flow for its own method's disagreement with the measured
  *pressures* and for its over-prediction of *axial* force (printed p. 10 and p. 12), and says
  nothing about the normal force; and its own method is uniformly high on model 2 (+24.8% and
  +31.6% at the attached Mach 1.90 and 2.30 against +28.8% and +20.3% at the separated rows), so
  it carries no separation signature to corroborate with.

- **Below about Mach 1.5289 hpr has no reading for model 2, and that is left standing.** At Mach
  1.50 the 18.5° flare is steeper than the 15.4885° its corner's shock holds, so ADR-047 draws it
  out to 15.4885° — and the march then refuses, because the corner's *isentropic* turn runs out at
  15.3647°. ADR-045 and ADR-047 bound different quantities and cross near Mach 1.55; below the
  crossing the march's bound is the tighter one, so drawing a flare out to the shock's bound can
  land past what the march can do. Bisected to `f64` resolution the first reading is at Mach
  1.5288696 (the fixture keeps the whole f64; seven figures is what the three platforms agree
  on), where the two bounds agree to five parts in 1e14, both 16.2844275°:
  the reading begins exactly where they meet. Swept every 0.005 Mach from 1.05 to 4.63, the
  reading turns on exactly once, so "the first" is a measured claim and not the artefact of a
  bisection over a predicate that can refuse in bands. A flared body below that takes slender-body
  theory, carried up by the join.

- **A refusal is committed with its numbers rounded.** Where the method declines a row, the
  fixture keeps `hpr-aero`'s own words for why — but that text prints `f64`s in full, and the same
  refusal reads `Mach 1.5250956752494207` on macOS and `Mach 1.525095675249415` on Linux. The
  fixture's *numbers* are compared to 1e-12 relative; its *strings* are compared as text, so the
  message is rounded to six decimals before it is stored. Six decimals is far more than the
  reading is worth and far less than the platforms disagree at. For the same reason the guide
  quotes seven figures of the bisected Mach number, not the sixteen the fixture keeps.

**Consequences.**

- The guide's flare section no longer says that nothing has been compared with a measurement; it
  says what the comparison found, on one body and one flare angle, three of whose six rows are a
  separated flare.
- A separated flare is now a named, sized gap rather than an unexamined one: +51.5% and +50.4% on
  this body, with nothing in the sources to say how that scales with a flare's angle, its length or
  the boundary layer's thickness.
- The drawn-out reading past the limit is still measured against nothing. The one row where it
  would have applied is the row the march then refused, so ADR-047's rule above the limit remains a
  construction chosen for continuity, not a checked one.
- `xtask/src/aero_flare.rs` repeats, on a hand-built body, the six lines `SupersonicRun::shares`
  runs for a flare, because `hpr-design` has no spherical-cap nose and model 2 cannot be flown
  through a `Rocket`. `the_flare_is_read_as_the_model_reads_it` pins the two share by share to a
  part in 1e12, on a flared body the design route can express, above the corner's limit and below
  it, so the fixture cannot quietly drift into being a second model.

**Not chosen: read model 2 through a `Rocket` by adding a spherical-cap nose to `hpr-design`.** A
design-level blunt nose is a real gap, but it is a design feature with file formats, mass
properties and a UI behind it, not a step in this milestone. The pinning test buys the same
guarantee for the price of one test.

**Not chosen: draw a refused flare out to the march's own edge instead of the shock's.** It would
have given a number at Mach 1.50 rather than a refusal, and the number would have been a third rule
invented to avoid an empty cell — with the report itself saying the flare's shock is detached
there, and its own method +28.6% high. A refusal that falls back to slender-body theory is the
honest reading, and where it starts is now measured to `f64` resolution.

**Not chosen: call the 3.95 and 4.63 rows an error in hpr.** They are a flow hpr does not model,
named by the report that measured them. Counting them as a model error would hide that the same
rows read +29.7% and +32.1% on the unflared model 1, and that the report's own attached-flow method
goes the same way.

## ADR-049: What a step in radius costs, and why the obvious fix is not taken yet (2026-09-20)

**Status:** accepted. **Milestone:** M1.8e15.

**Context.** [Issue #87](https://github.com/nrdptel/hpr-sim/issues/87) collected the places where
the body's supersonic normal force jumps with a small change of shape. The largest of them, and the
last without an owner, was a **step in radius**: a joint where one component's fore radius does not
match the previous one's aft radius. The march cannot cross one — its tangent body needs a profile
without a jump in it — and the model around it then takes the **whole body** off the second-order
shock-expansion method, at every Mach number.

M1.8e15 asks for #87 closed or narrowed to the step alone, with the step's measured size in an ADR
and the guide. It says a step "needs a model of its own rather than a decision about one that
exists". Searching the pinned sources for one came up empty: MIL-HDBK-762 treats a rearward-facing
step only as **base drag** and cites Zukoski on separation ahead of a forward-facing one; NACA
TN 3527, the method's own source, requires a continuous profile; NASA TR-1386 is nose **drag**; and
Washington and Pettis's boattails are continuous. Nothing in `refs/papers/` gives a step's normal
force faster than sound.

**Decision.** Measure it, publish it, and leave the model alone for now.

- **What a step costs is measured and pinned**, by
  `a_step_takes_the_whole_body_off_the_method`, so that a fix can be weighed against a number
  rather than argued about.
- **The step's own force stays slender-body theory's** `(2/A_ref)ΔA` at the joint, the limit of a
  transition whose length goes to zero. That is already an extrapolation — Barrowman 1967 p. 18
  assumes no discontinuities, and the guide says so.
- **Issue #87 is narrowed to the step alone.** Its other switches are owned elsewhere, and the two
  that were not now have issues of their own: [#120](https://github.com/nrdptel/hpr-sim/issues/120)
  (a lip longer than its boattail's drop in diameter) and
  [#121](https://github.com/nrdptel/hpr-sim/issues/121) (a pointed tip steeper than the cone
  tables' 30°). Its body text is rewritten: it said the threshold was the coverage gate's millionth
  of the area, and it is not.

**Measured**, on the tests' straight rocket (a tangent-ogive nose and three tubes, 1.3 m, the
reference pinned at 54 mm) at Mach 3 and 4°. With no step it reads `C_N` = 0.899592 with its centre
of pressure 16.9492 calibres aft of the tip. The reference is pinned on purpose: left on
`ReferenceDiameter::Maximum`, a step **up** widens the reference with it (56 mm at 1 mm, 58 mm at
2 mm), and a ratio of two coefficients taken on two different areas is not a quantity. A first pass
published that ratio; it read the step-up cost as −13% to −18% and the centre of pressure as moving
*forward*, both of which are artifacts of the moving reference. The test now pins the reference.

- **Where the switch sits is a pair, not a number.** At a joint where the radius changes but the
  slope does not, the tangent body merges two elements whose radii agree to a billionth of the
  radius (`shock_expansion::lay_out`) and refuses them past that: **2.7e−11 m** here, bisected, the
  same either way. At a joint where the slope changes too, a step **up** is refused earlier, by the
  requirement that the elements' corners stay in order along the body: 1e−12 × the body's length ×
  the change of slope, which on the tests' finned rocket (1.3 m, a 0.027 → 0.022 m boattail) is
  **1.3e−13 m**, bisected — 208× finer, and a function of the body rather than of the step. Neither
  is the run's coverage gate at a millionth of the fore area (13.5 nm of radius), which is looser
  than both and is what refuses every step anyone could draw; all three give the same reading, and
  the test pins which owns which range.
- **What it costs at the threshold:** −8.6519% of the normal force and 1.0285 calibres of centre of
  pressure, **wherever on the body the step is and whichever way it goes** — at that size the shape
  is flush to a part in 1e9, so the whole difference is the method itself.
- **What it costs as the step grows** — and the two directions part, because a step down takes area
  off the body while a step up adds area that carries slender-body normal force of its own.
  Stepping **down**: at 1 mm, −10.62% and 1.194 calibres at the nose's joint, −10.29% and 0.995 at
  the last; at 2 mm, −12.55% and 1.359 against −11.89% and 0.960. Stepping **up**: at 1 mm, −6.72%
  and 0.867 against −7.05% and 1.064; at 2 mm, −4.75% and 0.706 against −5.41% and 1.099. The
  centre of pressure moves aft in every row.
- **On a boattailed body the switch is larger:** −11.3409% and 1.0951 calibres on the tests' finned
  rocket, at 1.3e−13 m of step up or 2.7e−11 m of step down. That is the commonest high-power shape
  there is, and it is the case a fix most needs to handle.

**Why the obvious fix is not taken.** The obvious fix is to stop the march *at* the step and let
the body ahead of it keep the method, as the run already ends at a flare (ADR-047). It was built in
this milestone's first commit (`9efe721`, on PR #122, which is squashed on merge — that commit is
where the three readings below come from and the only place the prototype survives), measured, and
reverted:

- **It does not stay inside ADR-034's objection.** That decision rejected mixing the method's
  shares with slender-body theory's on a *measured* case, a boattail. A step's remainder is
  supposed to be a tube, whose slender-body share is zero — but "the run stopped at a step" does
  not imply "what follows is a tube". On the tests' finned rocket with its boattail's fore radius
  stepped 2.8e−11 m, the mixture reads `C_N` 0.8283554 with its centre of pressure at **16.7209
  calibres — forward of both pure models**, the method's 16.7286 and slender-body theory's 17.8237.
  A reading outside the envelope of the two models it is made of is the pathology ADR-034 measured.
  (Stated as a moment, the mixture crosses the pure method at 17.08 calibres and pure slender-body
  theory at 6.06, and between those two stations it is *less* restoring than either — an earlier
  draft of this ADR had that direction backwards.)
- **It does not close the band it was meant to close.** Matching the coverage gate to `lay_out`'s
  tolerance only lines the two up where the slope does not change. At a joint where it changes the
  corner-ordering bound binds instead, at 1.3e−13 m, so a boattailed rocket comes off the method
  anyway — worth −11.34% and 1.0951 calibres, the same switch `main` has today. The commonest shape
  the fix was meant to help is the one it does not.
- **It cannot tell a step from a shape the method has no reading for** — *as built*. The prototype
  keyed off the joint, not the shape, so any reason the run closed (a *non-conical* flare, a lip out
  of its wake, a flare under `SupersonicFlare::SlenderBody`) kept the forebody marched as soon as
  its fore radius was a picometre off: a **new** switch of +7.2% and 0.69 calibres on an ogive
  flare, where today's behaviour is continuous, and `BodyModel::BEFORE_M1_8E6` would stop
  reproducing earlier results on such a body. This one is a property of the two-line shortcut rather
  than of the idea — keying off which shape closed the run avoids it — so it is recorded as what a
  fix has to get right, not as evidence the idea cannot work. The rejection rests on the two above.

So the fix needs to be built on which *shape* stopped the run, not on whether the joint was flush,
and on what the march itself accepts rather than on a tolerance guessed to match it. That is a
model, not a condition, and it is what the narrowed issue #87 now asks for.

**Consequences.**

- Nothing in `hpr-aero` changed. No committed number moved, and no design in `validation/designs/`
  has a step.
- The guide keeps the step in its table of switches and gains a section saying what it costs, where
  the threshold is and what a fix would have to handle.
- Issue #87 stays open, narrowed to the step, with the three measurements above in it.

**Not chosen: ship the truncating fix anyway and record its faults.** Two of them are *new*
discontinuities larger than ones the same milestone left in the table, and one is a body that gets
no reading at all where today it gets one. A milestone whose subject is a jump in the model should
not add two.

**Not chosen: read a step as a flare of zero length, drawn out to its corner's limit (ADR-047).**
A step is infinitely past that limit whatever its rise, so every step would read the same held
value and a step of a micron would read like a step of a centimetre. ADR-047's drawn-out rule is
itself measured against nothing, and this would lean the whole of a step's force on it.

**Not chosen: model the step's supersonic normal force.** There is no source in hand. What one
would need is a measurement of normal force on a body with a forward- or rearward-facing step at
small angles of attack, which none of the pinned reports provides.

## ADR-050: A reduced element is read by the generalized method wherever it has a tangent cone of its own (2026-09-20)

**Status:** accepted. **Milestone:** M1.8e19.

**Context.** Along each element the second-order shock-expansion method relaxes the pressure toward
its tangent cone's as `p = p_c − (p_c − p₂) e^(−η)`, `η = (∂p/∂s)₂ (x − x₂) / ((p_c − p₂) cos δ₂)`
(TN 3527 eqs. 8 and 9). That form is monotone, so it holds only where the gradient behind the
corner points at `p_c`; the report keeps it for `η ≥ 0` (p. 13) and says that at `η = 0` "all
equations reduce to those given by the generalized shock-expansion method". Where `η < 0` hpr takes
`η = 0`, which is issue #81's reading. Until now it took it **only on the nose**: behind the nose
the march refused outright, on the ground that a cylinder, a boattail or a long shallow flare would
otherwise carry its corner's loading over any length.

A conical flare is one element behind the nose, and on a *near-flat* one it is reduced — so the
march refused a run of Mach rows, the table (built downward from Mach 5, needing the join's whole
0.3 Mach inside it) vanished, and a flare of about 0.03816° to 0.05882° on the tests' rocket, a
rise of a third of a millimetre over 0.3 m, took the whole body to slender-body theory at every
Mach number: −8.3% of the normal force and 1.16 calibres at Mach 3 and 4°. Shallower still the
refusal lifted the table's start in steps — −4.6% and 0.75 calibres at 0.00090182° — and not even
monotonically. ADR-047 recorded all of it; issue #117 and M1.8e19 carry it. M1.8e19 asks for the
region's edges **derived** rather than bisected, and then for a rule that carries the reading
across or a demonstration that refusing is right.

**Decision.**

- **The edges are two turns, each the zero of one of the quantities whose signs must agree.**
  `η`'s sign is the sign of `(∂p/∂s)₂` over the sign of `p_c − p₂`. Each of those is a smooth
  function of the turn through the corner, and on every corner state measured here each has a
  single zero, so an element is reduced on exactly the open interval between them:

  - the **crossing**, where `p₂(θ) = p_c(δ₁ + θ)` — the compression lands the pressure on its
    tangent cone's, and `η` has a pole;
  - the **balance**, where `(∂p/∂s)₂ = 0` — eq. 4 rearranged to
    `sin(δ₁ + θ) = (Ω₁/Ω₂(θ)) (sin δ₁ + r (∂p/∂s)₁ / B₁)` — the corner's own compression cancels
    the gradient the body ahead delivers, and `η` is zero.

  *A single zero* is not a promise, and a corner that breaks it exists: a 25° cone with 20 mm of
  tube behind it at Mach 7 meets its tangent cone's three times (about 0.91°, 7.3° and 24°), so a
  flare there is reduced on **two** bands. `flare_reduction_turns_rad` therefore sweeps the
  widening turns at a hundred stations before it brackets, and reports a corner with any number of
  crossings but one as an error rather than returning whichever root it reached
  (`a_corner_whose_gap_has_three_zeros_is_refused_rather_than_guessed_at`). A pair of roots inside
  one station would still be missed. Which of the two is the shallower is not fixed either — on the
  tests' rocket the crossing is below the balance from about Mach 1.5 up and above it below that,
  where both turns are shallower than the element-merging floor anyway, so no corner is drawn.

  Both are functions of the corner's own state alone: the Mach number, pressure, gradient, radius
  and angle the body hands to it, plus the free stream it was read in.
  `ShockExpansionBody::aft_flow` now reports all six, and `flare_reduction_turns_rad` takes just
  that and solves the two equations — a contraction on `Ω₁/Ω₂` for the balance, false position
  over the turns a widening corner can make for the crossing, started from
  `θ ≈ (1/p₁ − 1)√(M₁² − 1)/(γM₁²)`. Neither root is promised: each is returned with the residual
  it left, and a body whose cone flow is harder leaves more (4e-9 of the free stream's pressure on
  a fatter body at Mach 2.6). Nothing bisects the model's refusal, which is the search the
  milestone asked to be rid of; bracketing an explicit residual is not that.

  On the tests' flared rocket the derivation reproduces all three numbers issue #117 bisected: the
  band's lower edge is the **crossing at Mach 4.70**, 0.038161270°; its upper edge is the
  **balance at Mach 5**, 0.058820517°; and the shallowest angle that lifted the join's start is
  the **crossing at Mach 2.20**, 0.000901825°. The non-monotone part is explained too: the refused
  rows at a given angle are the Mach numbers between the two turns' inverses, an interval that
  narrows as the flare flattens until it holds no 0.05-Mach row at all.

  **What limits the digits is now stated, not observed.** Where the tangent cone is slender-cone
  theory's closed form (under `SLENDER_CONE_RAD`, 0.029°) the crossing closes to the last bits of
  an `f64` — the residual is under 2e-14 of the free stream's pressure at Mach 2.0, 2.2 and 3.0.
  Above it the cone flow is a Taylor–Maccoll integration, blended with the closed form up to
  0.0573°, and the residual is that integration's, about 1e-10 (Mach 4.3 to 5). Divided by the
  gap's slope in the turn that is about 2e-10°, which is the 2.6e-10° the three CI platforms were
  seen to spread the band's lower edge over. The spread was the cone's, not the search's, and the
  two edges past Mach 4 are pinned to 2e-9° rather than tighter for that reason.

- **The reading is carried across, by TN 3527's own reduction, wherever the element has a tangent
  cone of its own.** The refusal keeps only its first clause: an element whose angle is under
  `CONE_ANGLE_FLOOR_RAD` still refuses. That is issue #123.

  **What that line is and is not.** It is *not* that a widening element relaxes toward something
  and a cylinder does not: a reduced element does not relax at all. `decay_rate` returns zero, so
  in `ElementFlow::at` the decay is 1, `p_c` and `Λ_c` are multiplied by nothing, and the element
  holds `p₂` and `Λ₂` over its whole length — on a flare exactly as on a cylinder. What differs is
  what the element is being read *against*. On a widening element `p_c` and `Λ_c` are a real
  cone's at the flow's own Mach number, so the two branches either side of the region are two
  readings of one picture and they meet where `η = 0`. A cylinder's `Λ_c = tan δ (dC_N/dα)_tc` is
  identically zero and its `p_c` is the free stream's, so there is no cone there to meet; a
  boattail's is footnote 8's `p_c = p₀`, `(dC_N/dα)_tc = 2`, a stand-in rather than a solution of
  that element's flow.

  **And the length the old refusal worried about is real, admitted and measured.** Because a
  reduced element's reading does not depend on its length, what it costs grows with it. On the
  body alone, either side of its own crossing at Mach 5, the tests' rocket's 0.3 m flare moves
  `C_Nα` by +0.40% and its centre of pressure by 0.064 calibres, and the same body with a **2 m**
  flare by +2.61% and 0.761 calibres. That is the failure the blanket refusal existed to prevent,
  now taken on the flare with its size on the page rather than avoided by throwing the body off
  the method. Nothing measures it on a cylinder or a boattail, which is why those keep the
  refusal, and #123 is what would close that.

  **The trade is not one-way, and that is the decision.** On a body whose reduced rows sit at the
  bottom of the table, the old refusal truncated the table there, so the join carried the reading
  up from slender-body theory continuously and the pole was outside it; reading those rows puts
  the pole inside the table, where it is a step between two neighbouring Mach rows. What is given
  up on such a body is a continuous join; what is bought is that an arbitrarily small change of
  shape no longer moves the whole body between two models. The refusal is taken to be **not**
  right because the reason published for it does not hold — a reduced element behaves the same
  whatever it sits on — and because refusing one whole side of a pole is not the same as having no
  reading there: TN 3527 names `η = 0` as what its equations become, and both sides of the pole
  are then readings of the method. What the method does not say is what the loading does through
  the pole itself, and that is issue #108, unchanged by this either way.

  The measurement says the rest plainly. With the reading
  carried, the table starts at Mach 1.2 at fifteen flare angles from zero to a degree, and swept
  in sixteen steps from a cylinder to 0.08° — well past the region's steep end — the whole
  rocket's normal force falls all the way at both Mach 3 and Mach 4, with no neighbouring pair
  moving it by a fifth of a percent, bar the single pair that straddles that Mach number's
  crossing. The old switch moved it by 8.3% in one step. Both switches are gone
  (`a_near_flat_flare_reads_through_and_leaves_only_the_corners_crossing`), and their sizes are
  still measured, because they are the size of the fallback the model dropped to:
  `SupersonicFlare::SlenderBody` on the same rocket still reads −8.30% and 1.1574 calibres at
  0.058820517° and Mach 3, and −4.62% and 0.7522 calibres at 0.00090182° and Mach 2
  (`a_near_flat_flare_marches_every_row_and_the_fallback_is_still_measured`).

- **What is left is the crossing itself, and it is reported, not hidden.** At the crossing `η` has
  a pole. The pressure rides through — the gap it multiplies is zero there — but the loading does
  not: on the side the method still owns, `η → +∞` sheds the corner's loading onto the tangent
  cone's within the element's own length; on the reduced side it holds the corner's. (Which side
  is which follows the two turns' order, which is not fixed.) Measured on the whole
  rocket at 4° with a ±1e-9° probe either side of that Mach number's own crossing:

  | Mach | normal force | centre of pressure |
  |---|---|---|
  | 2.00 | +0.00032% | −0.0000016 calibres |
  | 3.00 | +0.011% | +0.00018 calibres |
  | 4.00 | +0.055% | +0.0017 calibres |
  | 4.95 | +0.129% | +0.0051 calibres |

  Widening the probe a hundredfold, to ±1e-7°, leaves the figure where it is at Mach 3, 4 and
  4.95, which is what says it is a step and not the reading's ordinary movement. **At Mach 2 it
  does not**: +0.00032% is already about what the reading moves over a ±1e-7° probe there, so that
  row is an upper bound on the step rather than a measurement of one, and the test only pins the
  ±1e-9° figure for it.

  **That is one rocket, and it is not a bound.** The step is the loading's gap at the pole, so it
  grows with the length of the element holding it, and where the region sits depends on the body
  ahead. On the **body alone** at Mach 5, either side of its own crossing: the tests' rocket's
  body +0.40% and 0.064 calibres; the same with a 2 m flare +2.61% and 0.761; the same with the
  tube cut to 0.1 m +4.34% and 0.186, its crossing at **1.388°**; a 10° cone with 0.3 m of tube
  +3.82% and 0.251, crossing at 0.696°. With a short shoulder the region is not near-flat at all —
  0.7° to 4.6°, where real flares live.

  **There is a bound, though, and it is exact rather than sampled.** The two sides of the pole
  take the two constants eq. 19 relaxes between — `Λ_c = tan δ₂ (dC_N/dα)_tc` on the side the
  method owns, `Λ₂ = (λ₂/λ₁) Λ₁` on the reduced side — and along a conical flare both are
  constant, so eq. 19's `C_Nα = (2π/A_ref) ∫ Λ r dx` integrates a constant:

  `ΔC_Nα = (2π/A_ref) (Λ₂ − Λ_c) · ½(r_fore + r_aft) · L`

  `flare_reduction_turns_rad` returns `Λ₂ − Λ_c` beside the turns as
  `crossing_loading_gap_per_rad` (6.116195e-4 per radian on the tests' rocket's body at Mach 5),
  and the formula reproduces the measured step to a part in 1e5 at flare lengths of 0.3, 1, 2 and
  5 m. So the numbers above are an illustration and the formula is the claim, which a reader
  evaluates for their own flare in one line.

  **And the pole is crossed in Mach as well as in shape.** On that short-shouldered body with a 1°
  flare the table's rows step **−2.77% and 0.14 calibres** from Mach 2.90 to 2.95, against a fifth
  of that either side. Under the old rule those rows were refused, so the table started above them
  and the join covered the pole; it is now inside the table.
  `what_the_crossing_costs_is_the_loading_gap_times_the_element_that_holds_it` pins all of it.

  The whole-rocket worst above is a 64th of the switch it replaces in the force and a 227th of it
  in the centre of pressure — the step at Mach 4.95 against the switch measured at Mach 3, so a
  comparison of sizes rather than of one flight condition; the body-alone numbers are not that
  small. It is not a new question, though, but ADR-044's: the loading through a tangent-cone
  crossing, open as issue #108. (That issue had been closed by mistake
  while ADR-044 was being written, which said in terms that it was re-scoped and *not* closed; it
  is reopened.) The region's other edge, the balance, leaves nothing at all —
  `η` is zero there, so the two readings coincide by construction, and a ±1e-9° probe moves the
  rocket's force and station by under 1e-7.

**Not chosen: read `η < 0` as `η = +∞` instead, putting the element on its tangent cone.** That is
continuous at the crossing — which is exactly where the pressure already sits on the cone — but it
tears the other edge open: at the balance the accepted branch is the `η = 0` reading, so the whole
gap `p_c − p₂` and the loading with it would step there, and that gap is far larger than the
crossing's (1.7e-3 of the free stream's pressure at Mach 5 against nothing). The two edges demand
opposite branches, and `η = 0` is the one the report names.

**Not chosen: blend the two branches across the band.** A weight running from the tangent cone's
loading at the crossing to the corner's at the balance would be continuous at both. It is also a
model with no source: nothing sets the weight, and the band is exactly where the method has no
statement to make. ADR-047 rejected a fade at a flare's detachment boundary for the same reason.

**Not chosen: let the table skip a refused row.** Building the table from the rows that hold and
interpolating over the ones that do not would have removed both switches without touching the
march — and would have published readings at Mach numbers where the method returns nothing.

**Consequences.**

- A flared rocket's reading is continuous and monotone in the flare's angle across the whole
  near-flat region, at every Mach number the table covers. No committed flight number moves: no
  validation case has a flare.
- `AftFlow` gains `pressure_ratio`, `gradient_p0_per_m`, `radius_m`, `loading_per_rad` and
  `free_stream_mach`, which with the two it had make it the corner's whole state — so `flare_reduction_turns_rad` takes it
  and nothing else, and the two halves of one state cannot be passed in disagreeing with each
  other. It and `ReductionTurns` are public, and the
  guide's *A near-flat flare* explains them with the table above.
- `ShockExpansionBody::reduced_elements` now counts reduced elements anywhere on the body, not
  only on the nose, and `slope`'s error list says the refusal is a cylinder's or a boattail's.
- `validation/fixtures/aero/blunt-tips.json` changes in six strings only: the refusal TN D-4865's
  own Newtonian start hits on the Arcas Robin from Mach 3.96 is a **cylinder's** element, so it
  still refuses and ADR-038's evidence stands; the message names the cause more exactly.
- Issue #117 is closed. Issue #123 is opened for the cylinder and the boattail. Issue #81 is
  unchanged: what the report would have done where `η < 0` is still unknown, and this ADR widens
  where hpr's own reading of it is used rather than narrowing it.
- ADR-047's two tests are replaced. `the_march_refuses_two_bands_of_flare_and_the_model_keeps_slender_body_theory`
  and `a_near_flat_flare_lifts_the_joins_start_in_steps` became
  `a_near_flat_flare_marches_every_row_and_the_fallback_is_still_measured` and
  `a_near_flat_flare_reads_through_and_leaves_only_the_corners_crossing`; three more were added —
  `the_turns_a_reduced_element_lies_between_come_from_the_corners_own_state` for the derivation,
  `what_the_crossing_costs_is_the_loading_gap_times_the_element_that_holds_it` for the sizes, and
  `a_corner_whose_gap_has_three_zeros_is_refused_rather_than_guessed_at` for the counter-example.
  A fourth, `a_corner_behind_a_relaxed_body_still_has_both_turns`, guards a body that hands the
  free stream's own pressure to the corner, where the crossing is a turn of nothing and rounding
  it through the isentropic relations would have decided the answer on one bit. The low-Mach band
  ADR-047 also described is untouched and still pinned by
  `where_the_corners_turn_runs_out_the_join_carries_the_reading`.

## ADR-051: M3.1 split, and the `.ork` document kept whole rather than interpreted (2026-09-20)

**Context.** M3.1 asks for a whole OpenRocket importer: three containers, schema 1.0 to 1.11,
components, materials, finishes, motor configurations, recovery, stages, stored simulation
results, unknown content preserved for a lossless round trip, warnings instead of failures, and a
cross-check against RocketSerializer. Loft owns eleven lessons in it (L56 to L66) and Loft's
importer is 2,793 lines of TypeScript without its tests. That is several sessions' work, and the
roadmap's rule is to split in place.

There is no XSD for `.ork` and no published grammar — only prose documentation and sample files —
and OpenRocket's Java source is GPL, so it cannot be read. Every OpenRocket release has added tags.
The reference library holds 78 `.ork` files spanning schema 1.4 to 1.11.

**Decision.**

1. **Split M3.1 into M3.1a to M3.1d**, one increment level as usual: the container and the
   document (a), the component tree (b), motors, recovery, stages and stored results (c), then the
   corpus and the cross-check (d). The parent's four *done when* bullets are unchanged and are
   M3.1d's; each increment carries its own.
2. **The document is read whole and interpreted by nobody.** `hpr_io::ork` reads the XML into a
   plain tree of elements and text — every attribute in order, every leaf's text as written — and
   later increments walk that tree. With no schema to check a file against, keeping the whole
   document is the only way to be sure nothing was quietly dropped, and it is what makes
   `extensions.x-openrocket` (M3.1c) and export (M3.2) possible at all.
3. **Reading, writing and reading again returns the same document.** The writer is canonical
   rather than byte-faithful: it lays the file out afresh, and escapes the characters an XML
   parser would otherwise change (a carriage return in text; a tab, newline or return in an
   attribute). Blank text between child elements is dropped on the way in, so laying it out again
   is free. Text an element holds beside child elements is kept, because OpenRocket writes it —
   a simulation's `<warning>` prints its message that way in 48 elements of 19 corpus files. The
   property is tested over generated trees and over all 76 corpus files that open.
4. **Nesting is counted before the text is parsed**, and a document deeper than 64 elements is
   refused. On a debug test build with a 2 MiB stack `roxmltree` read 120 levels of nesting and
   died on 130 — a crash where L56 asks for an error, and a figure that moves with the stack a
   platform gives a thread, which is the argument for not relying on it. Of the 76 corpus files
   that open, 53 nest 11 deep and the deepest reaches 17 (OpenRocket's parallel-booster example,
   where a nested stage or inner tube costs two levels and an appearance three). The count comes
   from a scan that skips comments, CDATA and processing instructions and tracks quotes, so it can
   only overstate the depth a parser will find, which a property test holds it to.
5. **Two dependencies**, both with justification in `THIRD-PARTY-NOTICES.md`: `zip` 8.6 (MIT,
   read-only, only the deflate method OpenRocket writes — the default features would pull bzip2,
   lzma, zstd and AES) and `flate2` 1.1 (MIT OR Apache-2.0, pure-Rust backend). Both build for
   `wasm32-unknown-unknown`, which `hpr-io` must, and `cargo deny` accepts both.
6. **`cargo xtask ork` is the corpus survey.** It reads every `.ork` under `refs/` and the example
   designs inside the pinned OpenRocket jar (a zip; reading the designs it ships is not reading its
   source), round-trips each one, writes the per-file detail to a gitignored `corpus-out/`, and
   prints counts only. Two files that are not well-formed XML are named in a committed list with
   the reason, and the survey fails if either ever reads or fails differently — an exclusion that
   polices itself rather than one that hides a regression.

7. **Two limits, both because the alternative is an abort rather than an error.** A read
   decompresses at most 256 MiB out of one archive (deflate expands about a thousandfold, so a
   1 MB `.ork` can ask for 2 GB of memory; the largest corpus design unpacks to 2,052,024 bytes), and
   an entry that would pass it is left out with a warning. And a run of text split by a comment, a
   processing instruction or a CDATA section is joined back as it is read: the writer drops the
   comment that made the split, so without joining, reading what was written would not give the
   same document. XML namespaces are the one thing the tree does not keep, and a document that
   declares any says so in a warning.

**Consequences.** M3.1a ships a reader that cannot yet produce a rocket, and the guide says so in
its second paragraph. The 78-file corpus gives every later increment a regression net it can run in
one command. `Document`'s `version` and `creator` fields are a reading of attributes that also stay
in the tree, so a document built by hand can contradict itself; the round-trip guarantee is stated
for documents that came from `parse`. The canonical writer means a `.ork` re-written by hpr will
not be byte-identical to the one it was read from, which M3.2 will have to live with — matching
OpenRocket's own layout was never achievable without reading its source.


## ADR-065: Stored results are references only when current and structurally plausible (2026-09-24)

**Context.** Loft lesson [L87][l87] says stored results were counted as references merely because a
`.ork` carried them. That made an outdated or not-simulated run, a missing summary, or an impossible
flight able to affect a validation gate and its denominator. ADR-057 correctly requires preserving
the file's stored results, but it did not yet distinguish faithful read-back from reference
eligibility. A stored result has no geometry fingerprint, so numerical disagreement cannot prove that
the design changed; a later comparison layer must establish design identity independently.

**Decision.**

1. **Keep first, classify second.** `StoredSimulation::reference_exclusion` is a pure classifier.
   It never changes or hides `Design::simulations`. The survey continues to count every stored run,
   its status, conditions, summary, branches, rows, events and parser warnings. A stored reference
   must name both `RK4Simulator` and `BarrowmanCalculator`, the simulator and aerodynamic-calculator
   identifiers shown in the public file specification's Simulation Data example. Missing and
   unsupported identifiers have stable exclusion reasons. They are provenance markers, not a full
   identity fingerprint for an OpenRocket version, settings or design.
2. **Only current, plausible OpenRocket runs can be stored references.** An explicitly `uptodate`
   run is eligible only when it has finite, non-negative stored values, positive ascent altitude,
   speed and time-to-apogee, and a flight time no shorter than time to apogee when present. Missing
   results or any required summary value exclude it. `outdated`, `notsimulated`, `external`,
   `loaded`, `cantrun`, `aborted`, missing-status and unknown statuses each have a stable exclusion
   reason. An external result remains readable, but is not silently promoted to an OpenRocket
   reference.
3. **Reject contradictions that the stored data itself proves.** Time columns may not run
   backwards, time or altitude may not be negative, and a stored time-series altitude may not differ
   from the stored apogee by more than `max(0.1% of apogee, 1 mm)`. This is a data-integrity policy
   for stored-value rounding, not a claim about flight-model accuracy. When several branches record
   an apogee event, the first such branch in file order is treated as the primary flight branch;
   the reader does not choose the branch whose maximum happens to match best. A single
   altitude-bearing branch is checked even when its apogee event is absent; multiple branches
   without an apogee event are uninspectable because their summary branch cannot be identified.
   Stored time-series rows and events must be finite, non-negative, and ordered. When summaries are
   present, rows may not exceed the stored flight time, event times may not exceed it, and the first
   apogee event must agree with `timetoapogee` within 1 ms plus floating-point rounding. This
   allowance covers the time columns' three-decimal stored precision; it is a data-integrity rule,
   not a model-accuracy tolerance.
   Fatal events, including normalized forms such as OpenRocket 24.12's `SIM_ABORT`, are excluded.
   No arbitrary minimum apogee is invented.
4. **Keep hpr reproduction as a separate screen.** `Design::reproduction_exclusion` reports reduced
   designs, missing or unknown configurations, and configurations hpr cannot assemble. These are
   limitations on hpr's ability to reproduce a stored reference, not evidence that the stored
   OpenRocket result is internally invalid. A later flight-comparison gate must report both screens.
5. **Count the policies separately.** `cargo xtask ork` reports stored-reference eligibility and
   hpr-reproduction eligibility alongside the unconditional stored-data census. On 2026-09-25, the
   survey found 91 stored-reference-eligible and 83 excluded runs among 174 stored simulations:
   47 inconsistent, 17 external, 11 outdated, 7 not-simulated and 1 missing simulator. Of the 91,
   1 is reproducible by hpr and 90 are not: 79 configurations are unflyable and 11 designs are reduced. The
   per-file detail remains in the private survey report; public docs quote only these aggregate
   counts.

**Consequences.** Stale statuses, missing provenance and contradictions cannot enter stored-reference
screens, distributions or stored-result denominators, while every result remains readable and
round-trippable. Hpr's current motor catalog and reduced-design boundary no longer masquerade as
stored-data defects. A future geometry fingerprint or comparison proof can add a new exclusion
reason without changing read-back.

[l87]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#l87


## ADR-064: Clusters, fillets and unread parts remain visible departures (2026-09-22)

**Context.** M2.2b4 asks whether motor clusters, fin fillets and parts hpr does not read are hpr's
rule or a written departure, then asks for the M2.2a corpus comparison again. The reader currently
reads a clustered inner tube as one `InnerTube`, omits the mass of a fin's fillet, and keeps parts it
cannot model in `x-openrocket` while marking the design reduced. The first two are deliberate
simplifications; the third is the lossless boundary set in [ADR-058][adr-058]. Full clustered-motor
mount and flight behavior belongs to M1.9, not this mass-convention increment.

**Decision.**

1. **Clusters stay one tube for now.** A non-`single` `clusterconfiguration` remains one hpr inner
tube and raises the existing warning. A new fixed probe, `a tube and a clustered inner tube`,
adds a 3-ring inner tube to the conventions oracle. OpenRocket 24.12, after saving the probe, gives
0.3813893481458014 kg, a centre station of 0.24036243822075784 m, roll inertia
0.0007674901427941916 kg m² and pitch inertia 0.007191233036548777 kg m². hpr's one-tube reading
is pinned as a departure: relative mass −12.85%, centre +5.95 mm, relative roll inertia −2.43%
and relative pitch inertia −3.68%. The values are measured by the external oracle; hpr does not
read OpenRocket's source. N-tube geometry, motor summation and motor-out moments remain M1.9's
work.
2. **Fillets remain omitted, explicitly.** A positive `filletradius` raises the existing warning;
`filletmaterial` is not used to invent a shape or mass. The 5 mm probe pins hpr's omission at
−0.808% mass, +0.737 mm centre and −0.439% pitch inertia; the 10 mm probe pins −2.79% mass,
+2.54 mm centre and −1.52% pitch inertia. Roll is compared with OpenRocket's fin shortcut, so the
pinned roll departure is zero for these probes. A future fillet model must replace this departure
with a measured shape, not silently change it.
3. **Unread parts remain losslessly visible.** Pods, parallel stages and unsupported fin shapes, plus five skipped parts across three kinds, remain
in `x-openrocket`, with their paths and a reduced-design flag;
hpr does not guess their geometry or add their mass. The existing warning taxonomy and survey
matching remain the evidence that the gap is visible. This confirms [ADR-058][adr-058] as the b4
rule rather than treating unread content as an unexplained comparison failure.
4. **The corpus rerun is the check.** On 2026-09-22, `cargo xtask ork` found 78 files, read 76,
laid out 75 and compared 74 designs (54 distinct contents). Structure mass was within 1% on 61
of 74, centre of mass within 1% of rocket length on 62, pitch inertia within 1% on 53 and roll
inertia within 1% on 57 when OpenRocket's fin rule was used. The eight distinct contents outside
one or more thresholds have nine cause assignments: cluster read as one tube (2), fillets left
out (2), unread/reduced parts (5), and switched-off stage (0). The one overlap is why assignments
sum to nine. The probe suite and `cargo test -p xtask ork_mass` both pass.

**Consequences.** M2.2b4 is complete without pretending that one tube is a cluster or that an
unread part has a mass model. The import warnings, reduced flag and `x-openrocket` preservation
make each departure inspectable. M1.9 may replace the cluster departure when its mount and flight
work is ready; a fillet model may replace its two measured departures only with new probes. M2.2b5
is next.

The numbers above are the 2026-09-22 as-of snapshot. The reproducible rerun on 2026-09-23 excludes
generated `refs/scratch/` files from the default scan: it found 75 files, read 73, laid out 72 and
compared 71 designs (51 distinct contents). Mass was within 1% on 58/71, centre of mass on 59/71,
pitch inertia on 50/71 and roll inertia on 56/71 with OpenRocket's fin rule. The current survey's
cause summary is 2 cluster cases, 2 fin-fillet cases and 6 reduced designs among the 11 distinct
contents whose roll remains outside 1% after that rule; the mass-or-CG comparison has 2 cluster,
2 fillet and 5 reduced cases. `cargo xtask ork` prints these counts and keeps its detailed record
private in `corpus-out/ork-survey.json`.

[adr-058]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-058-what-a-ork-holds-that-hpr-does-not-model-kept-whole-in-x-openrocket-2026-09-21

## ADR-063: Packed parts read and weighed as OpenRocket packs them (2026-09-21)

**Context.** M2.2b3 (ADR-062 §1) held four things: clusters, fillets, packed parts OpenRocket
sizes itself, and the parts kept unread. ADR-062 §7 had measured two packed-part gaps on its
probes: a parachute that writes no packed size (hpr's centre of mass 0.611 mm off, roll −0.167%,
pitch +0.744% of the probe's), and a mass override on a parachute that weighs nothing (roll
−0.805%, pitch −0.128%). Clusters need M1.9's mounts and fillets a shape of their own; neither
fits in the same session.

**Decision.**

1. **M2.2b3 splits.** b3 takes the packed parts. A new b4 takes clusters, fillets and the parts
   kept unread, and the stored results (Loft lesson L87) become b5. Every item of the old *done
   when* is in b3's or b4's.
2. **Measured on nine more probes** in `validation/oracles/openrocket/conventions.py`, each a
   tube and one part, fixed ids: a streamer, a shock cord and a mass component written with no
   packed size; a parachute written with only a packed length, and one with only a packed radius;
   a parachute with no packed size in a tube twice as wide, and in one whose bore is 8 mm in
   radius; a mass component of no mass and a shock cord of no length, each under a 30 g override.
   The earlier probes' records are unchanged by the rerun, bit for bit.
3. **An unwritten packed size is OpenRocket's.** A packed part that writes no `packedlength` is
   25 mm long; one that writes no `packedradius` is 12.5 mm in radius, a fixed number: it is the
   same in bores 48 and 98 mm in radius, and OpenRocket does not shrink it to fit a bore 8 mm in
   radius. hpr does not either (only an automatic radius is fitted to the bore). Each holds on
   its own: the half-written parachutes take the written half and the default for the other. It
   holds for all four kinds. The numbers are inferred from OpenRocket's output, not taken from
   its source. `hpr_io::ork` reads them with no warning, as ADR-061 reads what else a file leaves
   unsaid. Before, hpr read a length of zero silently and a radius of zero with a warning.
4. **A mass override on a weightless packed part is spread over its packing.** hpr gives the
   override `m` the packing's solid cylinder: `m r²/2` in roll, `m (3r² + l²)/12` in pitch, its
   centre halfway along. Any other part that weighs nothing still becomes a point mass under an
   override, which ADR-061 measured as both programs' rule. `hpr_design::tree` does this in both
   places an override applies, the part's own and one covering the parts inside. A streamer
   takes the rule by the same packing; no probe weighs a weightless one under an override.
5. **One cause retired.** `cargo xtask ork` no longer counts "packed parts hpr weighs as point
   masses" as a cause for a roll gap: hpr no longer makes such a point mass.

**Consequences.** `hpr_validate::openrocket::tests` holds all 42 probes of one tube and one part:
the two that were pinned gaps and the nine new ones are OpenRocket's to 1e-12 in mass, centre,
roll and pitch. On 2026-09-21 (`cargo xtask ork`, 74 files OpenRocket opens, before and after):

| | before | after |
|---|---|---|
| mass within 1% | 61 | 61 |
| centre of mass within 0.1% of length | 54 | 55 |
| pitch inertia within 0.1%, median | 34, 0.110% | 37, 0.086% |
| roll inertia with OpenRocket's fin rule, within 0.1% and 1% | 49, 55 | 53, 57 |
| files outside 1% in that roll, by content | 19 (15) | 17 (13) |
| roll inertia, hpr's own fins, within 1%, median | 29, 2.112% | 28, 2.354% |

The last row moves the wrong way on purpose: in one file the point mass's missing roll inertia had
offset part of hpr's fin departure (ADR-062 §3). The 17 still outside each have a cause: a
covering override (7 by content) or a reduced design (6). The Loft demo whose streamer writes no
packed radius no longer warns of it. A flight changes only where a `.ork` packed part writes no
size, or a weightless one carries an override. Two readings are hpr's and unprobed: a packed size
written but unreadable is read as zero, with a warning, as every unreadable number is; and every
probe places its part from the top, so the 25 mm length's effect on a part placed from the middle,
bottom or after another is derived, not measured.

## ADR-062: Fins and rail buttons against OpenRocket; roll inertia explained (2026-09-21)

**Context.** M2.2b2 (ADR-061) carried seven things: clusters, fillets, airfoil fins, elliptical
fins, the parts kept unread, a rail button's place (#151), and a roll inertia a median 2.1% from
OpenRocket's that nothing explained, even on Loft's public designs, whose mass, centre of mass and
pitch inertia agree within 0.1%. That is more than one session.

**Decision.**

1. **M2.2b2 splits.** b2 takes the fins, the rail button and the roll inertia. A new b3 takes
   clusters, fillets, packed parts that OpenRocket sizes itself, and the parts kept unread. The old
   b3, stored results (Loft lesson L87), becomes b4. Every item of the old *done when* is in b2's
   or b3's.
2. **Measured on probes.** `validation/oracles/openrocket/conventions.py` gains 33 probes, each a
   tube and one part: fin sets of every outline, one fin, sections, a tab, fillets and a cant;
   rectangles of two chords, two spans and on two tubes; every attached and packed part; rail
   buttons, one or a row of two, from each end. The tube alone is OpenRocket's, so a probe's roll
   inertia less the tube's is its part's. `hpr_validate::openrocket::tests` holds hpr to each or
   pins the gap.
3. **Roll inertia is the fins, and it is explained.** A bulkhead, centering ring, inner tube, mass
   component, parachute, shock cord and streamer are OpenRocket's in roll inertia to 1e-15; a rail
   button is apart by up to 1.4e-5 of its probe's. A fin set is not. OpenRocket gives two or more
   fins their mass times `R² + R hₑ + hₑ²/3`, with `hₑ² = A h / c_r`: the roll inertia of a thin
   rod from the body at radius `R` out to `R + hₑ`, where `A` is one fin's area, `h` its span and
   `c_r` its root chord; one fin, the same rod about its middle, `m hₑ²/12`. This is **inferred
   from OpenRocket's output**, each probe less its tube, not taken from its source. It holds to
   1e-12 on every fin probe but two: an ellipse (OpenRocket's is a 30-sided polygon) and a cant
   (−2.66e-5). A tab's mass takes the planform's value, and neither the section nor the thickness
   enters. Every non-rectangular probe has a span half its root chord; the Loft demos (span 0.39 to
   0.50 of the root) hold to 5e-6 as well. hpr keeps its own, the exact integral of `r²` over the
   fin: **a departure**, pinned per set. The two agree on a rectangle without a tab, bar the
   thickness term OpenRocket leaves out (+0.013%). They part on a tapered fin (hpr +2.41% on the
   probe's trapezoid, −2.14% on its triangle) and more on a tab, whose mass the rod puts out with
   the fin's though it lies 40–50 mm from the axis (−5.12%).
   `hpr_validate::openrocket::openrocket_fin_set_roll_kg_m2` states the rule.
4. **Fin sections: hpr's own, pinned.** OpenRocket weighs a fin set as outline × thickness ×
   a factor: 0.99 for a rounded section and 0.85 for an airfoil, at 3 and 6 mm alike. hpr
   integrates the section: a semicircular edge (0.9914 of the slab on the probe) and NACA's
   four-digit section (0.6851, Abbott and von Doenhoff). A file saying `airfoil` does not say which
   airfoil. hpr's is a cited shape, and a builder who weighed the fins overrides the mass anyway.
   So hpr keeps its sections, a departure: its airfoil fins are 19.4% lighter. The elliptical fin
   stays the exact ellipse too (OpenRocket's polygon is 0.18% lighter).
5. **A rail button's place is OpenRocket's (#151).** OpenRocket gives a button no length. It
   puts the button's centre where a part of no length would sit, from whichever end the file
   measures, and a row's first button there, the rest following aft. `hpr_io::ork` now moves the
   offset so hpr's button, placed by its forward edge, has its centre there. The probes check the
   top, middle and bottom, one button and two. After and absolute are read the same way, unprobed;
   a part placed after a button still follows hpr's button, which has a length.
6. **How the survey says it.** `cargo xtask ork` prints the roll inertia with OpenRocket's rule in
   hpr's place, on OpenRocket's own mass for each fin set: paired by id, or in an older file with
   no ids by the set of the same name nearest in mass (within 30%). A file outside 1% needs a
   cause, or the survey fails: a mass override covering the parts inside (ADR-061's departure), a
   reduced design, a fin set with nothing to pair, or packed parts hpr weighs as point masses,
   counted only for a gap of their sign and no larger than they could add in a packing as wide as
   their tube.
7. **Findings left for b3, each pinned.** A parachute that writes no packed size is packed 25 mm
   long and 12.5 mm in radius by OpenRocket; hpr reads no radius, with a warning. A mass override
   on a packed part that weighs nothing is spread over the packing by OpenRocket (`m r²/2`) and is
   a point mass in hpr (−0.805% on the probe). Fillets' mass (0.81% of the probe's structure at
   5 mm) is still left out. Untraced and small: a cant's mass (−4.19e-5 on the probe's structure),
   fins' pitch inertia (up to 0.41% of a probe's, on one fin), a lug's pitch (3.1e-4), and a rail
   button's inertias (up to 5.0e-4). A hypothesis for the last two, to test in b3: OpenRocket
   leaves out the off-axis term in pitch.

**Consequences.** On 2026-09-21 (`cargo xtask ork`, OpenRocket's record unchanged):
- **Roll inertia with OpenRocket's fin rule in hpr's place:** median 0.001% (hpr's own: 2.112%);
  within 0.1% on 49 of 74 (was 11) and within 1% on 55 (was 29). Five of Loft's six public demos
  come within 5e-6. The sixth, `demo-boattail.ork`, is 0.093% apart; with OpenRocket's polygon
  ellipse it is 1.5e-6 (a test).
- **The 19 files still outside 1%** (15 by content) each have a cause. By content: a mass override
  covering the parts inside 7, a reduced design 6, packed point masses 3; some have two.
- **Mass and centre of mass** unchanged: 61 and 62 of 74 within 1%. The rail button moved no file
  across a threshold.
- **hpr's own numbers change** only where a `.ork` rail button sits, and so the rail guide a flight
  leaves the rail by. How far depends on the end the file measures from (r is the button's radius,
  s the spacing, n the count):
  - from the top, after a part, or absolute: forward r;
  - from the middle: aft (n − 1)s/2, so one button does not move;
  - from the bottom: aft r + (n − 1)s.

  A row placed from the middle or the bottom now leaves the rail later.

---
## ADR-061: What a `.ork` leaves unsaid, read as OpenRocket reads it; overrides measured, two departures kept (2026-09-21)

**Context.** M2.2a (ADR-060) traced the designs outside a threshold to five causes. Two were
readings, not physics. hpr read a shoulder written with no wall as solid, where OpenRocket gives it
no mass. hpr gave a part written with no material no mass, where OpenRocket gives it a default. And
[Loft lesson L51](https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md) asks which
centre-of-gravity override wins: Loft's rule came from OpenRocket's source, which this project does
not read. M2.2b's *done when* asks for each convention to be hpr's rule or a written departure.
M2.2b is more than one session, so it splits into b1 (these readings and the overrides), b2
(clusters, fillets, airfoil fins, the parts kept unread, and roll inertia) and b3 (Loft lesson L87,
stored results).

**Decision.**

1. **Measured, not assumed.** `validation/oracles/openrocket/conventions.py` writes 32 small probe
   designs, each asking one question, and records what OpenRocket 24.12 makes of each: its
   structure and its own per-part breakdown, and the material it gives each part through the
   public `getMaterial` and `getLineMaterial`. The record is
   `validation/fixtures/ork/openrocket-conventions.json`. `hpr_validate::openrocket::tests` reads
   the same documents and holds hpr to it, and fails if a probe is added that no test reads.
2. **Where the file leaves something unsaid, hpr reads it as OpenRocket does.** OpenRocket's
   reading is what the file means to the people who wrote it.
   - A wall of no thickness on a nose cone, transition or body tube is a surface with no wall: the
     part keeps its shape and weighs nothing. So does an inner tube, coupler or lug of no wall,
     which hpr already read so but warned of. `hpr_design::solids::Wall::Shell` now takes a
     thickness of zero. A solid part is written `filled`; OpenRocket writes it so.
   - A shoulder of no wall, or none written, weighs nothing, capped or not, and whether or not
     its nose is filled. A filled nose keeps its shoulder's own wall (the probe: 0.84446 kg, the
     solid cone and the walled shoulder).
   - A nose cone, transition or body tube that writes no thickness has a 2 mm wall, whatever its
     radius: a nose and a tube at 50 mm and at 30 mm, and a transition, each weigh what a 2 mm
     wall gives.
   - A part that names no material is made of OpenRocket's default for its kind: cardboard,
     680 kg/m³, for a part weighed by its volume (probed on a nose, transition, body tube, inner
     tube, coupler, engine block, three kinds of fin set, ring, bulkhead and lug); ripstop nylon, 0.067 kg/m², for a canopy or streamer; an elastic
     cord, 0.0018 kg/m, for shroud lines and a shock cord; and Delrin, 1,420 kg/m³, for a rail
     button.

   - A mass override on a part that weighs nothing is a point mass at the middle of its length,
     in both programs.

   None of these is warned of any more: nothing is assumed. So a configuration held back only for
   them now flies. One reading is not taken: an inner tube, coupler or lug that writes no
   thickness gets a wall of OpenRocket's own (0.5 mm on the probe's 20 mm inner tube, 1 mm on its
   5 mm lug, none on the coupler). One size each does not say whether it follows the radius and
   no file in the library has one, so hpr keeps reading it as no wall, with a warning, and the
   test pins the difference. OpenRocket's default materials were read with none saved in its
   preferences; the script records that and refuses to run otherwise.
3. **Which override wins is OpenRocket's.** On every override probe hpr's mass is OpenRocket's.
   - A parent's override that covers its children wins over a child's own, and a stage's wins over
     everything in it.
   - A centre-of-gravity override is measured from the part's front, not its shoulder's, and moves
     the shoulder with the part.
   - A centre-of-gravity override alone, written to cover the parts inside, sets the whole
     assembly's centre. hpr had taken the scope from the absent mass flag; it now takes the flag
     of the one quantity overridden. (How the parts inside are placed differs, which shows only in
     the inertia: §4.)
4. **Two departures are kept, and one limitation, each pinned by a test.**
   - **The centre under a covering mass override.** When a mass override covers the parts inside
     and states no centre, OpenRocket puts the centre at the overriding part's own and leaves the
     parts inside out of it. hpr keeps the centre its parts lay out. A builder who weighs a tube
     with its fins and mount inside has not moved their centre, and hpr's is the better estimate.
     On the probe the two are 3.7 mm apart, or 19.7 mm when the part inside has an override of its
     own.
   - **Inertia under an override.** hpr scales the inertia of everything a mass override covers by
     the override's ratio, so the extra mass sits where the parts' mass does and the radius of
     gyration is kept. OpenRocket scales only the overriding part's own inertia, keeps the parts
     inside at theirs, and a stage, having none of its own, scales nothing. On a lone part the two
     agree. On the probes hpr's roll inertia is 6.9% to 37% below OpenRocket's under a tube's
     covering override, and 2.5 to 5.0 times OpenRocket's under a stage's. Neither rule is right
     for every rocket; hpr's is consistent with its own mass. Likewise under a centre override
     that covers the parts inside: hpr moves the assembly whole, OpenRocket the part alone, and
     OpenRocket adds the parts inside where they were (pitch inertia 2.65% apart on the probe).
   - **Flags that disagree** (a limitation). A part overriding both quantities with flags that
     disagree cannot be said in `hpr-design`, which scopes a part's overrides once: the mass flag
     decides, with a warning, as before. On the probe that is 4.7 mm.

**Consequences.** On 2026-09-21, rerunning M2.2a's survey (`cargo xtask ork`, OpenRocket's record
unchanged):
- **Mass:** 61 of 74 within 1% (was 57), median 0.002% (was 0.020%).
- **Centre of mass:** 62 of 74 within 1% of length (was 58).
- **Pitch inertia:** 53 of 74 within 1% (was 46).
- **Outside a threshold:** 13 files (8 by content), down from 17 (12). Each is a cluster, fillets
  left out, or parts kept unread, b2's and M1.9's.
- **Flying:** a second configuration flies (2 of 174).
- **Warnings:** those reading designs fall from 96 to 57, and parts that weigh nothing from 21 to 14.
- **Two gaps the probes found**, both for b2 and both pinned: OpenRocket puts a rail button's
  centre at its position, where hpr places it by its forward edge, one radius (5 mm) further aft
  (#151); and OpenRocket's elliptical fin weighs 0.18% less than hpr's exact ellipse, which
  matches a 30-sided polygon inscribed at equal angles to 13 digits (inferred from its output).
- **L51 is live:** `hpr_validate::openrocket::tests::override_precedence_matches_oracle`.

---
## ADR-060: M2.2 split, and the structure's mass held to OpenRocket's (2026-09-21)

**Context.** M2.2 asks for at least 20 designs in a report against OpenRocket, with apogee,
velocity, stability margin, mass and centre of mass. It also carries M1.4's deferred mass and CG
checks. But `cargo xtask ork` flies only 1 of 174 motor configurations: the rest mostly lack a
thrust curve. Staging waits for M1.9, and recovery is read but not flown. A mass or CG error would
also spoil every flight compared later. So M2.2 is more than one session, and mass comes first.

**Decision.**

1. **M2.2 splits into five increments**, each with its own *done when* in the roadmap:
   - a: each design's structure against OpenRocket's;
   - b: OpenRocket's mass conventions, with Loft lessons L51 and L87;
   - c: the motors OpenRocket flies, for the configurations held back for want of a curve;
   - d: flights to apogee on the public designs, with L80 and L81;
   - e: the corpus, carrying the parent's *done when* unchanged, with L19 and L82.

   L19 (tube fins' centre of pressure) needs a cited tube-fin model, so it goes with e, where the
   report meets the designs that have tube fins.
2. **The structure, not the launch mass.** `validation/oracles/openrocket/mass.py` asks
   OpenRocket 24.12 for `MassCalculator.calculateStructure` on each design's selected
   configuration, after saving it once to settle its automatic dimensions (ADR-054). hpr's side is
   `Layout::structure`. Both are every stage with no motor, so no thrust curve is needed.
3. **Which inertia is which is measured.** The script first reads a probe tube, 1 m by 50 mm with
   a 2 mm wall at 1,000 kg/m³, and records its mass, centre of mass and inertias worked by hand
   beside OpenRocket's.
   - OpenRocket's `ixx` and rotational inertia are the roll inertia; `iyy`, `izz` and the
     longitudinal inertia are the pitch inertia; all are about the centre of mass. They match the
     hand values to 15 digits.
   - hpr's layout of the same probe matches too, which pins hpr's side: the centre of mass at
     `z = −0.5`, and roll about `z`.
   - Pitch is compared as the mean of the two transverse inertias, so neither program's turn about
     the axis matters.
4. **Thresholds set before measuring:** 1% of OpenRocket's mass, and 1% of the rocket's length for
   the centre of mass. A design outside either needs a written cause. Inertias are reported, not
   held to a threshold.
5. **A difference is traced to parts.** The script also records OpenRocket's own per-part
   breakdown (`getCMAnalysis`). `cargo xtask ork` pairs each part with hpr's by id, an assembly
   under one override as a whole. In an older file that writes no ids, parts are grouped by name,
   since OpenRocket then makes up random ids. For every design outside a threshold, the survey
   prints its causes and the parts that differ most. Only a file from a public source is named;
   any other appears as the start of its file's hash.

**Consequences.** On 2026-09-21, over 74 design files (54 distinct by content):
- **Within 1%:** 57 in mass and 58 in centre of mass. The medians are 0.020% and 0.013% of length.
- **Outside a threshold:** 17 files (12 by content, 11 designs). The parts behind each fall into
  five causes, and hpr warns of every one when it reads the file; four have two. `cargo xtask ork`
  works the causes out from those warnings, counts them by content, and fails on a file outside
  with none. By content:
  - a shoulder written with no wall, read as solid, which OpenRocket gives no mass (6);
  - a motor cluster read as one tube (2);
  - fin fillets left out (2);
  - a part with no material, which OpenRocket gives 680 kg/m³ (1);
  - parts hpr keeps unread, such as pods, parallel stages and tube fins (5).
- **The arithmetic for the shoulder cause.** In the jar's *Two stage high power rocket*, the
  difference, 0.3666 kg, is exactly the solid cylinder of polypropylene the shoulder would be. In
  *Airstart timing* it is 5.853 kg of fibreglass. `hpr_io::ork` has left this choice "for the M2.2
  oracle to settle" since M3.1b2: OpenRocket gives such a shoulder no mass. M2.2b decides whether
  hpr follows.
- **Two more conventions, outside no threshold.** hpr scales a part's inertia with an overridden
  mass and OpenRocket does not (Loft's `stage-weighed.ork`: pitch +100.9%, the override 2.009 times
  its parts); and hpr's airfoil fin is lighter (the jar's *Simulation scripting* CONTROL fins, 19.4%).
- **Roll inertia is not explained:** median 2.1% apart, 1.2% to 3.8% even on Loft's public
  designs, whose mass, CG and pitch inertia agree within 0.1% and every part within 0.3 g. It goes
  to M2.2b, with the six
  conventions.
- **CI.** The committed record for Loft's seven public designs is held by `cargo test -p xtask
  ork_mass`.
- **Stale records fail.** The survey fails if the record is out of date (a changed file, or a
  laid-out design missing from it), or if the probe's mapping stops holding.

---
## ADR-059: The RocketSerializer cross-check: three readers, with OpenRocket settling a difference (2026-09-21)

**Context.** M3.1d2's *done when* has two parts. Every `.ork` in `refs/loft-fixtures` and the
OpenRocket example set must import with zero errors, and "the RocketSerializer cross-check agrees
on the key geometry". RocketSerializer (MIT, RocketPy-Team) turns a `.ork` design into RocketPy's
inputs. Research for this increment found three things:
- Release 0.2.0 fails on OpenRocket 24.12.
- Its `ork2json` command needs a stored simulation, which Loft's demo designs lack, and writes
  files beside its input.
- It depends on `orhelper` (GPL-2.0), which pins JPype below 1.5.

The first run of its extractors also showed that its stations are not always OpenRocket's. Its
walk of the tree adds the lengths of the parts before each one in its parent. And it matches parts
by name, so a two-way comparison could never agree everywhere without hpr copying its mistakes.

**Decision.**

1. **Three readers, not two.** `validation/oracles/rocketserializer/geometry.py` calls
   RocketSerializer's extractors one by one, as its `ork_extractor` does:
   `process_elements_position`, `get_rocket_radius`, `search_nosecone`,
   `search_trapezoidal_fins`, `search_elliptical_fins` and `search_transitions`. For each
   component it reports, the script also asks OpenRocket 24.12, loaded on the same file and
   settled by a throwaway save (ADR-054), for the same numbers. `cargo xtask ork` holds each of
   hpr's numbers to RocketSerializer's, to 1 part in 10⁹. Where they differ, OpenRocket settles it:
   - When OpenRocket's number is hpr's, RocketSerializer is apart.
   - Otherwise hpr is apart, and the survey fails.

   And whatever RocketSerializer says, the survey fails when one of hpr's numbers is not
   OpenRocket's, or when OpenRocket draws a nose otherwise than hpr. Without that, a mistake hpr
   and RocketSerializer share (a cant OpenRocket clamps at 15°, #148) would pass as agreement.

   **This is what "agrees on the key geometry" is taken to mean.** Every number RocketSerializer
   reads as OpenRocket does, hpr reads the same; no number of hpr's is apart from both; and every
   number of hpr's is OpenRocket's.
2. **The key geometry** is what RocketSerializer reports about the airframe's shape:
   - the nose cone's shape, length and base radius, and a Haack series's parameter;
   - each transition's length and end radii;
   - each trapezoidal and elliptical fin set's count, chords, span, sweep, cant and cross-section;
   - the station of each of those parts;
   - the body radius (the largest radius written as a number).
3. **A cause is named only where the record proves it.** Two causes can be checked:
   - A station further aft by exactly the lengths of the parts before it in its parent. The
     script records that sum from OpenRocket for every part.
   - A transition given the radii OpenRocket gives the first transition of its name.

   Every other difference is counted as "no cause shown", with all three numbers kept.
4. **Two of OpenRocket's own conventions are allowed for, each measured.**
   - OpenRocket turns a canted fin about the middle of its root chord, which moves the root's front
     aft by (c/2)(1 − cos δ). For all 10 canted fin sets in the library and the jar's examples, its
     turned station is aft of its unturned one by that amount, to 1 part in 10¹¹. The script
     therefore reads a fin set's station with the cant set to zero, restores the cant, and keeps
     the turned station too. `cargo xtask ork` checks the amount for every canted fin set and fails
     if one is off.
   - An ogive of parameter 0 is a cone. OpenRocket still calls it an ogive, so a shape is settled
     by OpenRocket's profile radius a quarter, a half and three quarters of the way along.
5. **Pinned as a tool, not as a reference checkout.** RocketSerializer is pinned at `66d8ca8`, after
   release 0.2.0, with the environment it runs in:
   `validation/oracles/rocketserializer/requirements.txt`, installed with `--no-deps`. `orhelper` is
   never installed, and JPype is the oracle environment's 1.7.1. (A first run, with `orhelper`
   and JPype 1.4.1 installed, gave the same records when compared locally; that run is not kept.) During the research, about 15 lines of
   `orhelper`'s source (its function signatures) were read before its licence was checked. Nothing
   here comes from them: the script imports none of it, and it starts the JVM the way
   `automatic_radius.py` (ADR-054) already did.
6. **Public record committed, library record not.** The record for Loft's seven public demo designs
   is `validation/fixtures/ork/rocketserializer-loft-demo.json`. An `xtask` test holds hpr to it in
   CI. The library's record goes to the gitignored `corpus-out/` and is summarised as counts.
   RocketSerializer's own 23 example designs are not added to the reference library here. Adding
   them moves every count the `.ork` page quotes, and brings findings of their own, so that is
   #147.
7. **Import errors are counted per source, and held.** An import error is a file that does not
   read, or a design that reads but does not lay out. Warnings do not count, since they never stop
   an import. `cargo xtask ork` prints the count for each source, and fails if `loft-fixtures` or
   the jar's examples has any, or if a design OpenRocket opens does not lay out in hpr. It also
   fails when a RocketSerializer extractor raises, since its parts would go uncompared.

**Consequences.** On 2026-09-21:
- **Imports.** `loft-fixtures` imports 27 of 27 files and the jar's examples 17 of 17, each with 0
  errors. The only errors under `refs/` are the two Loft test fixtures that are not XML (ADR-051).
- **The cross-check.** The record covers all 78 files the survey reads, and the survey fails if a
  design it lays out is missing from it. It compares 1,212 numbers over the 74 designs OpenRocket
  opens (54 distinct files; 896 numbers counting each once):
  - 1,102 agree (813 of the 896);
  - in 110 (83), RocketSerializer is apart and hpr's number is OpenRocket's: 75 stations from its
    walk, 15 transition radii from a same-named transition, 20 with no cause shown;
  - in 0, hpr is apart from both.
  - hpr's number is OpenRocket's in all 1,212, the 1,102 agreements included, so no agreement is a
    mistake hpr and RocketSerializer share. All 72 noses match OpenRocket's profile at three points.
  - RocketSerializer's stations come from OpenRocket's tree, and it agrees with hpr on 8 of 96 fin
    stations, so stations rest on OpenRocket; what it reads from the file agrees every time.
- **Not compared:** 6 parts inside pods or parallel stages, 3 values RocketSerializer gives none
  for, and 4 files OpenRocket does not open.
- **The public record.** Six of the seven Loft designs open in OpenRocket. Of their 80 numbers, 74
  agree, and 6 fin stations are RocketSerializer's walk.

M3.1's *done when* is met.

---
## ADR-058: What a `.ork` holds that hpr does not model, kept whole in `x-openrocket` (2026-09-21)

**Context.** M3.1c4 carries the last of M3.1c's *done when*: Loft lesson L66 (Loft dropped pods,
parallel stages and booster sets, and its export lost the note that the rocket was reduced), and "a
document with unknown content round-trips through `extensions.x-openrocket`". hpr's design has no
pods until M1.13, and writing a `.ork` is M3.2; ADR-051 already keeps the whole document in
`OrkFile`.

**Decision.**

1. **`hpr_io::ork::Design` gains `extensions`, with one namespace, `x-openrocket`,** serialized
   under that name. It holds what hpr does not model, each element whole with its path.
2. **Parts:** every child of a `<subcomponents>` under `<rocket>` that the reader (the code that
   walks the tree into `hpr_design` types) did not read — a pod set, a parallel stage, a part left
   out for want of an honest shape, a tag it has never seen. The walk records the path of every
   stage and component it reads, and a child it did not is kept whole. A design with any is
   *reduced* (`Design::is_reduced`), which is derived from the extension rather than stored beside
   it, so an export cannot keep one and lose the other.
3. **Sections:** every child of `<openrocket>` besides the first `<rocket>` and the first
   `<simulations>`, and every child of a stored `<simulation>` that no reader asks for.
4. **Tags:** every child tag no reader asks for in an element hpr does read — the rocket, a stage,
   a part, a stored simulation, and, below them, any tag a reader did ask for — such as a part's
   `<appearance>` or a wind's `<standarddeviation>`. The readers record each tag and attribute they
   ask for, by name and element, while `design()` reads, so one is kept exactly when nothing asked
   for it. A tag asked for only on some paths, such as a shoulder's radius when the shoulder has no
   length, is kept whenever it was not asked; a tag read and then dropped with a warning (a fin's
   fillet) is named in that warning and not kept, since it was asked for.
5. **Attributes:** every attribute no reader asks for on an element hpr does read, with the path of
   that element: a material's `group`, an event's `id`. The survey shows two hpr should read: the
   reference an angle offset or a radius offset is measured from (#145).
6. **A path leads back.** `openrocket/rocket/stage[0]/bodytube[1]/podset[0]` counts each step among
   its parent's `<subcomponents>`, as a warning's path does; a section's step counts among its
   parent's elements, and so does a tag's, marked `@`, at any depth. `hpr_io::ork::element_at`
   follows one, and `cargo xtask ork` fails if a kept element or attribute is not found again.
7. **The round trip is to JSON and back, and to the file.** A test writes the extension out as JSON
   and reads it back unchanged, and finds every kept element at its path in the document it came
   from. That is what an export needs; the export itself is M3.2. An unknown namespace beside
   `x-openrocket` is not read back yet; the design format (M3.3) decides how namespaces travel.
8. **Not kept:** the text of a second copy of a tag a reader takes once by name, since the reading
   is recorded by name and the reader uses the first copy; the second's attributes and unread
   children are kept. 13 fin tabs in the library carry a second `<tabposition>`. The text stays in
   the document `OrkFile` keeps whole (ADR-051), as everything does; M3.2 starts from both.

**Consequences.** On 2026-09-21 the library keeps 17 parts in 10 reduced designs (9 pod sets, 3
parallel stages, and the 5 parts left out), 87 sections, 1,947 tags and 3,132 attributes, and all
5,183 are found again at their paths. With M3.1c1 to M3.1c4, M3.1c's *done when* is met: L57, L64,
L65 and L66's tests are live, a design's stored results are read back, and unknown parts,
sections, tags and attributes round-trip through `x-openrocket`, short of the text of a second
copy of a tag a reader takes once (item 8).

## ADR-057: A `.ork` design's stored simulations, read back as written, with their units measured (2026-09-21)

**Context.** M3.1c3 carries "a design's stored results are read back" from M3.1c's *done when*,
with Loft lesson L64: Loft read the wind's direction from `launchroddirection` and dropped it from
the stored conditions. OpenRocket's file-format page shows `<conditions>` and `<flightdata>` with
no units, and its own example writes the rod's direction as `90.0` and the wind's as
`1.5707963267948966`.

**Decision.**

1. **Read, not re-flown.** `hpr_io::ork::StoredSimulation` holds each run's name, status,
   simulator, conditions and results as the file states them; `Design::simulations` lists them,
   even for a document with no design in it. They are OpenRocket's answers, for M2.2 to compare
   against, and they do not affect which configurations hpr flies.
2. **The units are measured, and committed.** `validation/oracles/openrocket/conditions.py` sets
   the conditions through OpenRocket 24.12's public setters, saves, loads and flies them;
   `validation/fixtures/ork/openrocket-conditions.json` holds what it wrote and held, and L64's test
   holds the reader to it. The rod's angle and direction are degrees, and this reader gives them in
   radians; the rod's direction is a compass bearing (a rod tilted toward 90 lands the rocket
   east, while the example's own wind stays put); the wind's direction is radians, the bearing it
   blows from (in a wind from the east, a rocket off a vertical rod lands west); with
   `launchintowind`, OpenRocket writes the wind's bearing, in degrees, as the rod's. All of it is
   OpenRocket 24.12's; a much older file may have meant the rod's direction otherwise.
3. **The results are kept as written.** A time series keeps its column names, its rows and its
   events, and the results keep OpenRocket's stored `<warning>`s. In the eight columns the probe
   compared, rows are SI with angles in radians and latitude in degrees, rounded to three decimal
   places or, for a large value, four significant figures; the summary and the other columns are
   taken to follow. `NaN` becomes `None`, so the design survives JSON and back. A row with the
   wrong number of values, an event with no time or type, and an atmosphere with no model or one
   OpenRocket 24.12 does not write are each warned about.
4. **The wind's speed and direction come from the legacy tags first**, `<windaverage>` and
   `<winddirection>`, and from the average `<wind>` element where those are missing: the file
   specification says OpenRocket still writes the legacy tags for older versions. Where both are
   written and disagree, a warning says so. `<windmodeltype>` says which wind the run flew, since
   OpenRocket writes both.

**Consequences.** On 2026-09-21 the library's 178 stored simulations are read, in 64 documents;
164 carry a summary and 144 a time series (101,955 rows). M3.1c's "a design's stored results are
read back" is met here.

## ADR-056: A `.ork` design's recovery and separation, read as written, with OpenRocket's words measured (2026-09-21)

**Context.** M3.1c2 reads when each parachute and streamer opens, the drag coefficient it states,
and when each stage separates (ADR-055). OpenRocket's file-format page shows the tags but lists none
of the words they take, and says nothing of whether a deploy height is above the ground or the sea.
ADR-055 read the ignition words with one of them, `ejectioncharge`, resting on a probe that was not
committed, and promised one that was.

**Decision.**

1. **Read, not flown.** `hpr_io::ork::Recovery` holds each device's settings and each stage's
   separation as the file states them. Turning them into `hpr_sim::recovery::Device`s is left to the
   flight that uses them: `hpr-io` does not depend on `hpr-sim` (ARCHITECTURE.md), and two of the
   choices below are that step's to make.
2. **The words are OpenRocket 24.12's own, measured and committed.**
   `validation/oracles/openrocket/events.py` sets every value of the ignition, deployment and
   separation events through OpenRocket's public setters on its own examples, saves, and records
   the word and the label; `validation/fixtures/ork/openrocket-events.json` holds them, and a test
   holds every reader to every word. This settles ADR-055's `ejectioncharge`. A word not listed is
   kept as written.
3. **A setting is an event, a height and a delay, each `None` where the file is silent**, and a
   per-configuration setting replaces them one at a time, as ignition does (ADR-055), so a file
   that leaves one out still reads. That is hpr's reading: OpenRocket was not probed on such a
   file. An empty or unknown event word is kept with a warning.
4. **A deploy height is above the ground** (the launch site; OpenRocket has no terrain). The probe
   flies in calm air with a fixed seed, sets a parachute to open at 30 m on a pad 1,000 m above sea
   level, and it opens at 29.9 m above the ground; a height above the sea would never be reached.
5. **A height the rocket never reaches is recorded, not resolved.** The probe sets the same
   parachute to 100 m on a flight whose apogee is 51.7 m, and in that run OpenRocket never opens
   it, though the flight reaches the ground. One run of one design, not a stated rule.
   `hpr_sim::recovery::Trigger::Altitude` opens at apogee in that case. Which the flight of a
   `.ork` follows is for the step that flies it to decide, in the open.
6. **`<cd>` is kept as the file wrote it**, `auto` or a number (`Dimension`). OpenRocket reports an
   automatic parachute's as 0.8, on the canopy's area, the default its technical documentation
   gives (section 4.2.5) and the probe reads back; an automatic streamer's comes from the strip's
   length and material, on the strip's area (appendix C, equations C.4 and C.5), an estimate the
   documentation puts at about 20% and one user (issue #2031) finds far too small for a 2.5 by
   44 in streamer. The probe records three. Choosing a model is the flight's business, as with the
   height.
7. **A device inside a part hpr does not read is kept apart** (`UnreadDevice`), as a motor is, and
   so is a parallel stage's separation.
8. **The warnings about these settings say so in their path** (`…/deployment`, `…/drag`,
   `…/separation`, shared constants), so they do not count against the airframe when ADR-055's rule
   decides which configurations fly; a test pins both sides.

**Consequences.** On 2026-09-21 the library's 137 parachutes and streamers are read, 2 more left
out inside pod sets; 77 leave their drag to OpenRocket and 60 state it; 18 of its 93 stages state a
separation, and 2 parallel stages' separations are left out. The two open choices, a height above apogee and an automatic drag coefficient, are on
the `.ork` page.

## ADR-055: M3.1c split, and the motors a `.ork` flies: its own curve first, and only what lights at launch (2026-09-21)

**Context.** M3.1c asks for everything a `.ork` holds beyond the airframe: motor configurations and
embedded curves, recovery settings, pods and parallel stages, stored conditions and results, and
`extensions.x-openrocket` for the rest, with Loft lessons L57, L64, L65 and L66. In the reference
library that is 174 motor configurations, 212 `<motor>` elements, 137 parachutes and streamers,
9 `podset` and 3 `parallelstage`, and a stored simulation in most designs (`cargo xtask ork`):
more than one session. The reading of each also
rests on what OpenRocket means by its words, which its file-format page ([F]) mostly does not say.

**Decision.**

1. **M3.1c is split into four increments**, each carrying part of M3.1c's *done when* unchanged,
   and M3.1c's own bullet stays unticked until the last is done:
   - M3.1c1, motors and their configurations (L57, L65);
   - M3.1c2, recovery and separation settings;
   - M3.1c3, stored launch conditions and results (L64), carrying "a design's stored results are
     read back";
   - M3.1c4, pods, parallel stages and the rest (L66), carrying "a document with unknown content
     round-trips through `extensions.x-openrocket`".
2. **A configuration is what `<rocket>` declares plus what the mounts put in it.** Each mount's
   `<motor configid>` joins its configuration; a configuration only a mount names is read as one of
   its own, with a warning (L65). A per-configuration `<ignitionconfiguration>` overrides the
   mount's event and delay each on its own, falling back to the mount's for the one it leaves out.
3. **A motor's curve is the archive's `thrustcurves/<digest>.rse` first, then the bundled
   catalog.** The file-format page says OpenRocket itself looks in its motor database first and
   falls back to the embedded curve. hpr takes the embedded one first because the digest "uniquely
   identifies the functional characteristics" of a curve (OpenRocket's GitHub wiki, file format
   1.2): it is exactly the curve the design was saved with, while hpr's bundled catalog is 32
   motors chosen for validation, not OpenRocket's database. The cost is that on a file carrying
   curves, hpr and OpenRocket can fly different curves for the same motor: the file specification
   says OpenRocket prefers its database's, which "may have more accurate or updated data". The
   motor is built from the curve's header, as a catalog motor is, and the mass and centre of
   gravity listed beside each thrust point are not used; a case size that differs from the
   design's by more than a millimetre is warned about. The catalog match is on manufacturer
   (its full name or abbreviation) and designation, compared without case, spaces or hyphens,
   because both are needed: the library's Estes `B4` is not the catalog's Quest `B4`. Two matches
   are refused as ambiguous. A hybrid is never given a curve, whether the design or its embedded
   curve says so: the project flies commercial solid motors only. A motor with no curve is kept
   with its reason, and nothing is invented for it.
4. **`<delay>none</delay>` is a plugged motor, and `0` a charge at burnout.** OpenRocket's
   technical documentation defines the delay as the time "between the motor burnout and the
   ignition of the ejection charge", which "can also be replaced by 'P', which stands for
   plugged" (p. 8), and describes "zero-delay motors, that ignite the ejection charge immediately
   at burnout" (p. 10); issue #2002 calls typing `none` into the delay box the way to say plugged.
   OpenRocket flies a `0` that way: its "Parallel booster staging" example stores the E12-0's
   burnout and ejection charge at the same 2.44 s. Older designs may mean plugged by it — until
   23.09 added *plugged* to the delay list (#2090), OpenRocket's own examples used `0` so (#2111) —
   and hpr reads it as the file says, as OpenRocket does. `hpr_motor::Delay::Seconds` now documents
   zero as a value a file may state plainly.
5. **The ignition words are OpenRocket 24.12's own**: `automatic`, `launch`, `ejectioncharge`,
   `burnout` and `never`. The library's files use all but `ejectioncharge`, whose spelling was
   measured by setting it through OpenRocket's public setters and saving, in a probe not committed;
   M3.1c2 commits one for every event word. `automatic` lights "the lowest stage … at launch" and each stage above at "the ejection
   charge of the lower stage" (OpenRocket's FAQ, "How do I create a staged rocket?"). An unknown
   word is kept as written.
6. **The rocket flies a configuration only when it can be flown as written.** Every motor must be
   read, have a curve and a case size, sit in a mount read as the single tube it is, and light at
   launch — `launch`, or `automatic` in the bottom stage (the last in the file), at no delay — and no stage may be
   switched off (OpenRocket removes an inactive stage from the flight: release notes 22.02.beta.05,
   PR #1478). A mount holding two motors for one configuration keeps it out too, since which
   OpenRocket would fly is not known. Two more rules are about the rocket rather than its motors:
   the rocket and its motor mounts must have been read without a single warning — nothing left
   out (a pod, a parallel stage, a part hpr cannot shape), dropped or simplified (a cluster of tubes
   read as one, a flipped nose cone read forward, a material it could not read) or assumed (a
   shoulder of no wall read as solid) — and the rocket must have one stage (OpenRocket drops a booster when it separates, and hpr would
   carry it to the ground until M3.1c2 reads separation and M1.9 flies it). Only those
   configurations become `hpr_design::Rocket::configurations`. The reason is that
   `hpr_design::Configuration` lights every motor at `t = 0` until [M1.9] brings staging: flying a
   two-stage configuration there would light the sustainer on the pad. Every other configuration
   stays in `hpr_io::ork::Motors` with its first reason (`NotFlown`).
7. **`hpr-io` depends on `hpr-motor`**, for the curve reader and the catalog; both are pure crates
   that build for `wasm32-unknown-unknown`, so the pure core is unchanged. `ARCHITECTURE.md`'s crate
   map says so.
8. **`cargo xtask ork` counts the motors and fails if a configuration the importer passed as
   flyable does not assemble.**

**Consequences.**

- On 2026-09-21, the library reads 206 motors into 174 configurations and leaves 6 out, 4 inside
  pod sets and 2 inside parallel stages. 4 motors have an embedded curve, all in the library's one
  schema-1.11 file, a two-stage design, so none of them flies yet; 2 have a bundled one; 3 are
  hybrids and 197 have no curve hpr holds. So 1 of the 174 configurations flies, and it assembles;
  1 more is held back only by its airframe's warnings, shoulders of no wall read as solid, which
  M2.2's oracle is to settle. The catalog, not the reader, is the main limit, until [M5.1]'s
  online layer and cache bring ThrustCurve.org's curves.
- A cluster mount, read as one tube since M3.1b3, keeps its configurations out rather than flying
  one motor where the file has several.
- Recovery, separation, stored results, pods and `extensions.x-openrocket` are M3.1c2 to M3.1c4.

[F]: https://openrocket.readthedocs.io/en/latest/dev_guide/file_specification.html
[M1.9]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m1-9
[M5.1]: https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#m5-1

## ADR-054: An automatic radius with nothing to take is OpenRocket's default, and a rocket with no stage or component holds no design (2026-09-20)

**Context.** After M3.1b3, 3 of the 76 readable `.ork` files in the reference library still gave
no rocket that lays out. One, Debrief's `sample-design.ork`, has a `<rocket>` holding a name, a
comment and nothing else; its stored simulation is the whole point of the file. The other two have
a chain of automatic radii with no fixed radius anywhere along it: Loft's `demo-quirks.ork` makes a
nose cone's base, a tube and a transition's forward end automatic, with only the transition's aft
end fixed (the nose caches 0.033 m), and the `openrocket-database` parachute catalogue stacks four
tubes whose radii are all a bare `auto`. `hpr_design`'s neighbour rule ([ADR-007][adr-007]) has
nothing to take in either, so `layout()` refused both. M3.1b4's *done when* asks that each lay out
or be shown to hold no design, and that any rule for such a chain rest on something written down
rather than on a cached number.

**What is written down.** OpenRocket's user guide says only that "the Automatic checkbox will
adjust the dimensions of the component automatically", and its file-format page shows `auto`
without explaining it. The issue tracker says more, in prose:

- A maintainer: when the radius is automatic "and there is no previous or next component to the
  body tube … OR returns the default radius" ([openrocket#1988][or-1988], 2023).
- An open issue: a tube left with nothing to take "reverts to default diameter" ([#1992][or-1992]).
- A user, guessing: the nose cone "may be retaining the default 1.969 in. base diameter"
  ([#871][or-871], 2020), which is 50.0 mm across.
- The change that closed #871 ([PR #998][or-998], 2021) disables the checkbox "when the component
  that you want to get the diameter from already has its auto checkbox checked", or when there is
  none. #1988 shows a later version still leaving one ticked, so the dialogs make such chains rare
  rather than impossible; most come from older or hand-written files. PR #998 also calls the
  number saved beside `auto` the "manual value".

No document gives the default as a number, so it was **measured**.
`validation/oracles/openrocket/automatic_radius.py` runs the OpenRocket 24.12 jar as an external
oracle on fifteen small designs of its own, and with `--library` on the 17 examples inside the jar
and the two library files. It reads each body radius OpenRocket holds when the file is opened,
saves the design (which makes OpenRocket work every radius out again), and reads each radius again.
`validation/fixtures/ork/openrocket-automatic-radius.json` holds the result, with the jar's hash
and the Java and JPype versions:

- Every automatic radius with nothing fixed along its chain settles at **0.025 m**, except a nose
  cone's base or a transition's end that looks at another automatic radius (next bullet but one): a
  lone tube, two tubes, a lone nose cone, a lone transition, and the tube in each longer chain.
- A cached number is ignored: `auto 0.04` and `auto 0.03` both come back 0.025. OpenRocket writes
  back what it resolved, not the number it read (`auto 0.04` is saved as `auto 0.025`).
- Where a nose cone's base or a transition's end looks at another automatic radius, OpenRocket
  settles on **−1 m**, and writes `auto -1.0`. Four shapes show it, 6 radii in all.
- A chain that reaches a fixed radius takes it in OpenRocket as in hpr, in every case probed (up to
  two automatic radii between) and across a stage boundary. The control, a nose before a fixed tube, takes the
  tube's 0.03 m.
- **OpenRocket's first reading is not always its answer.** Where the tube after an automatic tube
  holds a coupler whose own radius is automatic, OpenRocket first reads the automatic tube at its
  default, and settles on the neighbour's radius once it works the design out again. That is what
  happens in its own "Dual parachute deployment" example, whose file caches 0.025 m for that tube;
  M3.1b3 left that cache as a question for this oracle, and the settled answer is hpr's 0.028321 m.

Of the library files, the parachute catalogue settles with its four tubes at 0.025 m, and
`demo-quirks.ork` does not open at all: OpenRocket refuses a `<parallelstage>` directly under
`<rocket>` ("Booster Set not currently compatible with component: Rocket"). Its chain is measured
on the probe's own copy.

**Decision.**

1. **An automatic body radius with no fixed radius anywhere along its chain takes OpenRocket's
   default radius, 25 mm, as a fixed radius, with a warning at its tag naming the radius.** The
   number rests on the maintainers' "default radius" and on the measurement, which the user's
   1.969 in agrees with. It never rests on a cached number, which OpenRocket itself ignores. The rule
   lives in the `.ork` importer (`hpr_io::ork::OPENROCKET_DEFAULT_RADIUS_M`), not in `hpr_design`:
   it is one program's convention, and a design that says nothing about a radius should still be
   refused by `layout()`. `Rocket::unresolvable_body_radii` lists exactly the radii `layout()`
   would refuse, located by stage and index, and `Rocket::fill_unresolvable_body_radii` fills those
   and no others; a property test holds both, and that every radius the neighbour rule reached
   comes out as before.
2. **hpr departs from OpenRocket where OpenRocket answers −1 m.** A negative radius has no
   geometry, so the nose base or transition end that gets it takes the default too, and the chain
   is one radius end to end. `hpr_io`'s test against the fixture holds every other radius to
   OpenRocket's settled answer, bit for bit, and counts the departures: 6 of the 39 radii.
3. **hpr is held to OpenRocket's settled answer, not its first reading.** The first reading
   depends on an unrelated part, and OpenRocket corrects it itself; the test counts the one case
   where the two differ, so another would show.
4. **A tube given the default has its wall judged against it**, by the rule a stated radius gets:
   `filled`, or a wall at least as thick as 25 mm, is solid to 25 mm. Before the radius was known
   the reader had read a `filled` tube as weightless, or as solid to a cached number; those readings
   and their warnings are withdrawn. No corpus file has such a tube; the physics review found the
   gap and a test holds the fix.
5. **A `<rocket>` with no stage or component of any kind holds no design,** and so does a document
   with no `<rocket>`. The reader says so in a warning ending "so the document holds no design", and
   `cargo xtask ork` counts such a document apart from the designs, not as a design that fails to
   lay out. The document is still read whole, and its stored results are M3.1c's to read.
6. **The probe runs OpenRocket through JPype, inside the Python process** (CLAUDE.md rule 3 permits
   it). It uses only class and method names that `javap` prints for the jar, and binds empty motor
   and preset databases itself, because the graphical providers need a display. This is evidence
   for M2.2's choice of how to drive the jar ([ADR-035][adr-035]), not that choice. It shows the
   in-process route works headless with Java 17.

**Consequences.** All 75 designs in the reference library lay out, and the 76th document is
counted as holding none. `cargo xtask ork` prints both, and the 7 radii given the default in 2
designs. On the parachute catalogue's four, OpenRocket agrees; on the quirks file's three it can't
be asked, and on the probe's copy of that chain it agrees on the tube and answers −1 m on the other
two. `cargo xtask ork` now also holds every body radius hpr resolves in the 18 designs OpenRocket
opens to OpenRocket's settled answer: **67 of 67 agree**. The in-file oracle's premise, that
`auto 0.0125` is what OpenRocket last worked out, holds for what 24.12 writes, but a cached number
can be a first reading that a save wrote out, and PR #998 says an older release wrote the manual
value; 67 of the 71 cached numbers on components with an `<id>` agree with hpr. The `Rocket` no
longer records that a defaulted radius was automatic; the document the importer keeps still says
`auto`, so a writer working from it loses nothing.

[adr-007]: #adr-007-design-tree-stations-placement-automatic-radii-overrides-motors-and-checks-2026-09-17
[adr-035]: #adr-035-drop-the-orhelper-dependency-how-m22-drives-openrocket-is-decided-when-m22-starts-2026-09-19
[or-871]: https://github.com/openrocket/openrocket/issues/871
[or-998]: https://github.com/openrocket/openrocket/pull/998
[or-1988]: https://github.com/openrocket/openrocket/issues/1988#issuecomment-1397654629
[or-1992]: https://github.com/openrocket/openrocket/issues/1992

## ADR-053: The parts on and inside a `.ork` body: degrees, what is left out, and a sourced finish (2026-09-20)

**Context.** M3.1b2 read a design's spine. M3.1b3 reads what hangs off it — the tubes, couplers,
rings and bulkheads inside a body component, the fins, tube fins, lugs and rail buttons on it, and
the mass objects and recovery gear packed in it — which is 765 parts across the reference corpus
against the spine's 285. Five things had to be settled first, and [ADR-052][adr-052] left one of
them open on purpose: what the older of two names for a radial offset is measured from.

**Decision.**

1. **Angles in a `.ork` are degrees.** Nothing in the file says so and every length beside them is
   in metres, so it is a mistake waiting to happen: read as radians,
   `<angleoffset>180</angleoffset>` is more than twenty-eight turns instead of half of one. The
   corpus settles it — of the 993 angles written in it, 188 are not zero and **178 of those are
   larger than 2π**, more than a whole turn — and the values themselves are 180, 90, 45, 30 and
   120, carrying the float dust (`119.99999999999999`) of a conversion that went through radians
   and back. `cargo xtask ork` prints the three counts and
   `hpr_io::ork::tests::angles_are_degrees_not_radians` holds the reading.
2. **`radialposition` and `radiusoffset` are read, each on the parts that carry it, and never one
   as the other.** ADR-052 left both unread for want of a source saying what the older name's
   frame is. That question does not have to be answered to read them, because the corpus shows the
   two never meet: `radialposition` (542 elements) is written on the parts *inside* a body — inner
   tubes, couplers, rings, mass objects, recovery gear — and `radiusoffset` (106) on the parts
   *on* it (fins, tube fins and rail buttons: 95) and on the 11 pods and parallel stages, and no
   element carries both. They are two tags on
   different components, not two names for one. The same goes for the angle: `angleoffset` and the
   older `rotation` or `radialdirection` are read as one number because on all 121 elements that
   carry both they agree on it, and the frames they differ on — `relative` to the parent against
   `fixed` in the rocket — are the same angle for every parent this reader builds, all of which
   sit on the rocket's own axis. A pod set does not, and a pod set is M3.1c's work.
3. **A part this reader cannot give an honest shape is left out with its reason, never guessed.**
   Four rules, five elements in the corpus: an external part on anything but a body tube (2 fin
   sets on a nose cone, whose root is not a straight line); a freeform outline that does not end on
   the root (the same 2, which would have to be closed along a root they never touch); a tube fin
   set whose radius OpenRocket sizes from the body, which hpr has no rule for (2, [#133][issue]);
   and a part whose automatic radius needs a bore its parent has not got (1 coupler in a nose
   cone). The design still opens and still lays out; the warning names the part and the reason, and
   `cargo xtask ork` counts them.
4. **A tube of no wall thickness carries no mass**, which is the opposite of the rule the spine
   gives a body component and a shoulder, and deliberately so. A body component has a `filled`
   spelling in the file, so a zero there is ambiguous and is read as solid; an inner tube has no
   such spelling, and OpenRocket's own geometry makes a tube's bore its outer radius less its wall,
   so a wall of nothing is a part of nothing. Reading those 12 elements as solid would invent the
   mass instead — a solid coupler filling a 50 mm airframe for 180 mm is a few hundred grams two of
   OpenRocket's own example designs never had. `hpr_design::parts::hollow_cylinder` therefore
   accepts a zero wall and answers zero mass, which its formula already did.
5. **An inner tube's automatic outer radius is its parent's bore**, which is the rule a centering
   ring already had (`AutoDimension::OuterRadius`, extended to `InnerTube`): 37 couplers and engine
   blocks in the corpus need it. Automatic **outer** radii resolve in a pass of their own, before
   any ring's automatic **bore**, because a ring's bore reads its siblings' outer radii — resolving
   both in one pass would give a ring whose bore depended on whether the tube inside it was written
   first. That is [Loft lesson L60][l60], and it is why `hpr_design`'s resolution is two passes.
6. **The five surface-finish words take the roughness heights OpenRocket's author published**: 500,
   150, 60, 20 and 2 µm for `rough`, `unfinished`, `normal`, `smooth` and `polished`. The file
   format documentation gives none of them. The default is confirmed twice over — the [OpenRocket
   technical documentation][techdoc] section 6 says regular paint "corresponds to an average
   surface roughness of 60 µm", and the user guide's body-tube dialog reads "Regular paint
   (2.36 mil)", which is 59.9 µm — and the other four come from the program's author on The
   Rocketry Forum. Each is a `Finish::Custom` height rather than one of hpr's named finishes, whose
   names mean other surfaces. **`polished` is not settled:** 2 µm rests on that post alone, and the
   technical documentation's own Table 3.2 puts 2 µm at *aircraft sheet metal* while "finished and
   polished surface" is 0.5 µm — so OpenRocket's label does not match the row its name points at,
   and a later version could have moved it (forum posts from 2023 list nine finishes, not five,
   though no release note mentions them). It would be worth 1.32× in skin friction on the 14
   components that say it. No file in the corpus writes any word but the five. An unknown word
   takes hpr's default and says so.
7. **Which way an angle turns is assumed, and said to be assumed.** hpr measures a roll angle
   right-handed about an axis pointing at the nose; the OpenRocket technical documentation §3.1.4
   points its own `x` along the centreline *aft* and leaves the other two axes unstated. If that is
   what it means, every angle read here is mirrored — a mass object at 90° on the other side, a
   canted fin set rolling the other way. No file can settle it, because a mirrored design is still
   a valid design, so it goes on the guide's list of readings that are not settled, for one
   asymmetric design through the M2.2 oracle to decide.
8. **Whether an override covers the parts inside a component is taken from the mass flag.** A
   `.ork` says it once per quantity and `hpr-design` says it once for the component, so the two
   cannot always agree; mass is the quantity the flag is written for (95 of the 104 in the corpus),
   and a centre-of-gravity flag that disagrees raises a warning. Until this milestone the spine
   read it as never covering the children, which was harmless while no component had any.
9. **M3.1b splits once more.** M3.1b3 is the parts; M3.1b4 is the three designs of the 76 that
   still do not lay out, none of which this milestone could reach: one document holds no rocket at
   all, and two have a chain of automatic radii with no fixed radius anywhere to resolve against.

**Consequences.** 765 parts read across 73 designs that lay out, and every claim above is a count
`cargo xtask ork` prints. *Some* of the resolution rules now have an oracle that needs no
OpenRocket: `auto 0.0125` is the answer OpenRocket itself last worked out, so the layout can be
held to it — 67 of the 71 cached dimensions on a component the file gives an id agree to a part in
10⁹. It reaches less far than that sounds: OpenRocket never caches a number for `outerradius`
(0 of 131) or `innerradius` (0 of 80), so **the two rules point 5 adds have no oracle coverage at
all**, and rest on their tests and on the argument for them. `cargo xtask ork` prints the per-tag
denominators and names the tags nothing reaches. The four that
do not are one body tube and the parachute packed inside it, in one design the corpus holds twice,
and **that file's caches contradict each other**: the nose cone ahead of that tube caches
0.028321 m, and a nose cone's automatic base radius *is* the radius of the component behind it,
while the tube itself caches 0.025 m. hpr resolves it to 0.028321 m, with the design's three other
cached radii and its one stated radius against the single odd one. That is an argument from the
file, not a proof — it shows the cache is inconsistent, not which half went stale, and which one
OpenRocket would compute today is for M2.2 to settle. It is ADR-052's point about a cached value,
made checkable.

Point 4 is a reading OpenRocket could settle, and M2.2 will: it is listed with the spine's two
open readings on the guide's `.ork` page. Point 6's `polished` is the one number here that a newer
OpenRocket may have moved; it is written down as unsettled rather than left to be discovered. Fin
fillets, the screw head on a rail button, motor clusters and instanced rings are read as the
simpler part hpr models, each with a warning, so what is missing from a mass is never silent.

[adr-052]: #adr-052-what-a-ork-value-means-automatic-dimensions-two-names-for-one-tag-and-overrides-2026-09-20
[l60]: https://github.com/nrdptel/hpr-sim/blob/main/docs/research/loft-lessons.md
[issue]: https://github.com/nrdptel/hpr-sim/issues/133
[techdoc]: https://openrocket.sourceforge.net/techdoc.pdf

## ADR-052: What a `.ork` value means: automatic dimensions, two names for one tag, and overrides (2026-09-20)

**Context.** M3.1b turns the document tree into a design. Before any component can be read, three
things about `.ork` values have to be settled, and Loft got all three wrong: a dimension may say
`auto 0.0125` rather than a number (L58), a tag may be written under two names at once (L62), and a
stated `0` is a value rather than a missing one (L63). There is no schema to settle them from, so
each rests on what the reference corpus shows, counted by `cargo xtask ork`.

**Decision.**

1. **M3.1b splits into M3.1b1 (the values) and M3.1b2 (the components).** The values are what every
   component reader asks for, and they can be settled and tested on their own.
2. **An automatic dimension keeps both halves.** `Dimension::Automatic { cached }` holds what
   OpenRocket last worked out — `auto 0.0125` gives `Some(0.0125)`, a bare `auto` gives `None` —
   and is never confused with `Dimension::Stated`. Saving a design must write `auto` back, which is
   what Loft's dropping of the flag broke. 413 dimensions in the corpus are automatic, across
   `outerradius`, `innerradius`, `radius`, `aftradius`, `foreradius`, `packedradius` and `cd`.
3. **Either name of a renamed tag may be read, newest first — for the two renames that are only
   renames.** Where OpenRocket writes both `axialoffset` and `position` (642 elements) or both
   `instancecount` and `fincount` (109), it agrees with itself on the text *and* on the
   `type`/`method` attribute that says what the number is measured from, every time. So either
   name may be read, the newer wins, and a disagreement — in the number or in the frame — raises
   a warning.

   `angleoffset`/`radialdirection` and `radiusoffset`/`radialposition` are **not** read this way,
   though they look the same. The newer name of each carries a `method` the older never carries:
   of the 26 elements with both `angleoffset` and `radialdirection` the texts agree every time and
   the frames differ every time, and `radiusoffset` (106 elements) and `radialposition` (542) are
   never written together at all. Reading one as the other would move a component in silence, so
   what the older name's frame is belongs to M3.1b2, with a source. The constants for them are not
   in the public API.
4. **A stated zero is a value.** `<overridecd>0.0</overridecd>` (2 in the corpus) is an override to
   no drag at all. The six override tags — three values, three flags — are read independently.
5. **The single pre-1.9 `overridesubcomponents` flag sets all three.** It is what the three
   per-quantity flags replaced; 20 elements of the corpus carry it and **none** of them carries a
   per-quantity flag beside it, so reading it as all three cannot contradict a file. It raises a
   warning, so the inference is never silent. *Amended by ADR-095:* OpenRocket 24.12's reading
   was measured, and hpr now follows it without a warning; the survey now counts 10 such elements.

**Consequences.** The value layer is settled before any component reads it, and its claims are
counts anyone with the corpus can reproduce by running `cargo xtask ork`, which prints each one.
Points 3 and 5 are readings of what a rename meant, not statements from a specification: if a file
ever turns up where the two names disagree, or where the old flag sits beside a new one, the
warning says so rather than the reading being wrong in silence. Two renames are left unread
rather than guessed, which means M3.1b2 cannot place an instanced component until it settles
them. `Dimension` carries no unit, because the tags it reads are metres, radians and
plain numbers alike; the component reader names the unit.
---

## ADR-066: Every curve hpr flies is integrated as OpenRocket integrates it (2026-09-25)

**Context.** [M2.2c][roadmap] asks two things: that every configuration held back only for want of a
thrust curve flies or is named with its reason, and that each curve's total impulse is within 0.1%
of OpenRocket's. The two halves need different evidence. The impulse check is a property of a curve
file, testable in CI on the 32 public-domain ThrustCurve.org curves this repository already carries
([the motor page](physics/motor.md#the-bundled-motors)). Making the held-back configurations fly
needs curves this repository does not have and may not redistribute: the reference library's
designs embed their own, and OpenRocket's bundled set lives in its GPL jar. So M2.2c is split, and
the measurable half goes first.

**Decision.**

1. **The oracle hands OpenRocket the same bytes hpr reads.**
   `validation/oracles/openrocket/motors.py` loads each curve file with
   `MotorLoaderHelper.load(File)` and records what OpenRocket 24.12 makes of it: designation,
   common name, digest, envelope, masses, standard delays, point count and first and last time,
   and its own total impulse, average and maximum thrust and burn time. The comparison is therefore
   of two integrations of **one file**, not of two catalogues' data for one motor, which would
   measure the catalogues instead of the code. OpenRocket is run, never read.
2. **The committed record covers the bundled curves only.** They are public-domain files already in
   the repository, so the record adds no data whose licence is unclear. The curves the reference
   library embeds are private (rule 4 of `CLAUDE.md`): M2.2c2 counts them and publishes the counts,
   not the curves.
3. **What is held, and what is recorded.** A test in `hpr-motor` holds four quantities on all 32
   curves, tying the record to each file by SHA-256: total impulse to the milestone's 0.1%, and peak
   thrust, the 5%-of-peak burn-time window (`getBurnTimeEstimate`) and the curve's whole duration
   (`getBurnTime`, which is the last listed time, not a window) to rounding. Measured: **all four
   are bit for bit equal** at `f64`, and the test also asserts that equality, so the claim on the
   guide cannot go stale in silence. Both codes integrate the listed points trapezoidally, in the
   same order, and both prepend an origin (see point 4), so nothing rounds differently.
4. **One difference is real and written down rather than held; two apparent ones are not.**
   - **Average thrust.** OpenRocket divides the impulse **inside** the window by the window; hpr,
     with ThrustCurve.org's own code, divides the **whole curve's** impulse by the same window
     ([TC-A]'s line 231, cited on the motor page). The tails below 5% are what differ, so hpr's
     average is the higher on every one of the 32, by **+0.0107% to +0.3147%**, median
     **+0.0965%**. The test prints those three and asserts the sign, so the sizes come from the
     suite and not from this page. hpr keeps the published rule it cites; the burn-time *window*
     itself needs no departure, being OpenRocket's to the bit.
   - **Designation spelling.** OpenRocket reads the designation from the file's own header,
     ThrustCurve.org's catalog from its database: `131-G84-GR-10A` against `131G84-10A`. The test
     compares the common name and identifies the motor by the file's SHA-256.
   - **The prepended origin.** hpr adds `(0, 0)` when a file's first point is after ignition, and
     so does OpenRocket: 29 of the 32 files start between 1 and 40 ms, and OpenRocket reports a
     first time of zero and one point more than the file lists. That is not a departure but the
     reason the integrals agree, so the test holds the point counts and both first times equal.

[TC-A]: https://github.com/JohnCoker/thrustcurve3/blob/577afa62302f70c6b2ba04e97a39240638cd704b/simulate/analyze/analyze.js

**Consequences.** The 0.1% half of M2.2c is met, and met with room to spare: hpr's reading and
integration of a curve file is OpenRocket's, bit for bit, on every curve it can publish, and that
now covers the burn-time window too. What remains for M2.2c2 is supply, not arithmetic — the 193
motor references in the reference library that resolve to no curve, and the configurations they hold
back. This says nothing about the rest of the motor model: how the propellant burns back, what its
mass and inertias are during the burn, and what a delay does are not compared with OpenRocket here.
An average thrust is the one number a later milestone must not carry across the two codes without
naming its numerator; the sizes above are that warning. The record is regenerated by running the script, never edited; it carries the jar's SHA-256,
the Java and JPype versions and the command, as the other OpenRocket records do.

[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md

---
## ADR-067: Curves come from OpenRocket's own database by digest, each held to its impulse (2026-09-25)

**Context.** [M2.2c2][roadmap] asks that the curves the reference library embeds are held to the
0.1% of total impulse that [ADR-066](#adr-066-every-curve-hpr-flies-is-integrated-as-openrocket-integrates-it-2026-09-25)
set for the bundled ones, and that every configuration held back only for want of a curve flies or
is named with its reason, both counted by `cargo xtask ork`. The first half is small: the library's
75 files embed 3 distinct `.rse` curves, which 4 motor references use. The second half is supply.
Of 202 motor references, 196 had no curve: 3 hybrids and 193 neither embedded nor in hpr's
32-curve bundled catalog. That left 162 of 170 configurations held back for want of a curve, and 2
flying. A `.ork` names its curve by OpenRocket's digest, a hash of the curve's data that
"uniquely identifies the functional characteristics" of the curve (OpenRocket's GitHub wiki, file
format 1.2), and OpenRocket finds most curves in the motor database it ships inside its jar. There
were two places to get curves: ThrustCurve.org through `hpr-net` ([M5.1][roadmap], not built, and
matched by name), or OpenRocket's own database, which is where the reference program gets the
curve it flies.

**Decision.**

1. **An oracle records OpenRocket's database, and what OpenRocket flies.**
   `validation/oracles/openrocket/motor_database.py` does three things:
   - It loads the database with `MotorDatabaseLoader`, the public class OpenRocket 24.12 loads it
     with at startup. It stops if OpenRocket's user motor directories hold any file, so the record
     is the jar's alone.
   - For each of the 1,452 motors it records the digest, the identity, the envelope, the masses
     and the centre of mass. It records the samples, and OpenRocket's total impulse, average and
     peak thrust and burn times.
   - It hands every embedded `thrustcurves/*.rse` to OpenRocket's loader, keyed by the entry's
     SHA-256, as `motors.py` does for bundled files.
   - It opens every design with the database bound, as the program does, and records the digests
     of the motors OpenRocket places in each configuration, by configuration id.

   It walks the designs `cargo xtask ork` surveys: those under `refs/` but `refs/scratch/`, and with
   `--jar` the jar's example designs. It finds and opens them as `mass.py` does, retrying a design
   OpenRocket refuses for a leading comment without it. The record goes to the
   gitignored `corpus-out/openrocket-motors.json` and is never committed. The database comes from
   ThrustCurve.org through OpenRocket, with unstated terms, and the embedded curves belong to the
   designs' owners. Only counts are published. OpenRocket is run, never read.
2. **`hpr-io` takes supplied curves, by digest only.**
   - `SuppliedCurves` maps a digest to a solid motor and its `CaseSize`. It refuses a case that is
     not finite and positive.
   - `ork::design_with(file, &supplied)` looks in this order: the embedded curve, then a supplied
     curve for the motor's digest (`Curve::Supplied`), then the bundled catalog.
   - A supplied curve is never matched by name: the database holds 286 manufacturer-and-designation
     pairs under more than one digest.
   - A `<motor>` typed `hybrid` is refused before any lookup. The caller supplies solid motors
     only, since a `SolidMotor` carries no type.
   - `ork::design` passes nothing to `design_with`, so its output and its tests are unchanged.
   - A reason says where hpr looked: the supplied curves lack the digest, or no digest is recorded.
   - A supplied case more than a millimetre from the design's is warned about, as a catalog one is.
   - A supplied curve comes ahead of the bundled catalog. Where both hold a motor, OpenRocket's
     masses are the ones flown, and they need not match the ThrustCurve.org metadata the catalog
     carries. One library motor moved from the catalog to the database this way.
     Matching OpenRocket is the point of the survey.
3. **The survey screens the database before supplying it.**
   - Hybrids are never supplied: 164 motors under 161 digests.
   - 6 digests are held by two motors that differ in samples, case or masses, so none of them is
     supplied.
   - One motor is refused, Cesaroni 836J210-16A. Its propellant mass gives an exhaust velocity of
     10.1 km/s, outside the 200 to 5,000 m/s that `SolidMotor::from_envelope` accepts.
   - 54 repeats of a digest with the same samples and envelope (to 1e-9) collapse to one.
   - That leaves 1,221 digests supplied, from the 1,288 solid motors less the 54 repeats, the 12
     motors sharing the 6 digests and the refused one.
   - Other masses are taken as OpenRocket holds them, including some that give physically
     unlikely exhaust velocities. OpenRocket flies the same masses.
4. **Every curve is held to OpenRocket's impulse, or the survey fails.** hpr's integration is
   compared with OpenRocket's `getTotalImpulseEstimate`. All are within 0.1%, and **identical to the
   last bit**. The two sets are different kinds of check:
   - The 3 distinct embedded curves are real checks, as the 32 bundled ones were. hpr parses the
     entry's bytes, and OpenRocket parses the same file with its own loader.
   - The 1,288 solid database curves are checked against the samples OpenRocket has already
     parsed, so their agreement proves the hand-off, not two readings.
   - A curve outside 0.1% is not supplied.
   - The survey fails on any of these:
     - a curve outside 0.1%;
     - an embedded curve missing from the record, or one OpenRocket refused;
     - a record missing a list, or with no solid motor to check;
     - a record written by another version of any of the four oracle scripts (with `motors.py`,
       `automatic_radius.py` and `geometry.py`), by another OpenRocket release, or with another
       jar than the pinned one;
     - a flown configuration whose supplied curves OpenRocket does not place (item 5).
5. **OpenRocket places the curves hpr is supplied.** For every configuration hpr flies with a
   supplied curve, the survey finds the same design (by SHA-256) and configuration id in the
   record, and requires OpenRocket's placed digests to include every supplied one. They do in all
   67 such configurations (67 motors), and OpenRocket opens every design they are in. A mismatch,
   a configuration id OpenRocket lacks, a design missing from the record or one the oracle failed
   on fails the survey; a configuration in a design OpenRocket does not open is counted apart and
   printed. Configuration ids are compared in lowercase, as OpenRocket keys them.
6. **Each configuration the bundled catalog left without a curve is followed.** The survey reads
   every design twice, with and without the supply, and counts what became of the 162:
   - **66 fly.** With the 2 that already did, 68 of 170 configurations fly, in 16 designs, and
     all 68 assemble.
   - **72 are held back for another reason,** by the first reason the reader finds:
     - 29 for an airframe not read exactly as written
     - 20 for a motor in a cluster
     - 12 for more than one stage
     - 11 for a motor igniting in flight
   - **24 still have no curve,** each for a named reason:
     - 20 record no digest, and the bundled catalog has no such motor
     - 3 are hybrids
     - 1 has a digest the database lacks

   The stored-run reproduction screen ([ADR-065](#adr-065-stored-results-are-references-only-when-current-and-structurally-plausible-2026-09-24))
   moves from 1 to 40 reproducible of the 91 eligible runs. One supplied case differs from its
   design's by 7 mm in length, and the survey warns about it.

**Consequences.** M2.2c is met: every configuration held back only for want of a curve flies or is
named, and every curve hpr flies from the library is within 0.1% of OpenRocket's impulse. Two
things are left:
- **Where a motor's weight sits is not OpenRocket's.** hpr builds every motor from its envelope,
  whether bundled, embedded or supplied: centre of mass at mid-case, the dry case a thin tube.
  OpenRocket gives each database motor a fixed centre of mass of its own, and treats the motor as
  a solid cylinder for inertia. For 163 of the 1,221 supplied motors, OpenRocket's centre of mass
  is more than 1 mm from mid-case. Among the 31 distinct supplied motors in configurations that
  fly, 3 are, by up to 5.0 mm. The survey prints both counts. [M2.2d][roadmap] flies these designs against OpenRocket
  and will meet this in the stability margin. It is the milestone to decide whether hpr takes
  OpenRocket's centre of mass, or records the difference as a departure.
- **Motors that record no digest** (older files) stay unflown. A name rule could fly them, but it
  would be hpr's choice among curves.

`Curve` is `#[non_exhaustive]`, so the new variant breaks no caller.

[roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md

---
## ADR-068: OpenRocket's flights of the public designs, and what its metric words mean (2026-09-25)

**Context.** M2.2d asks for the public designs that fly in a report against OpenRocket (apogee,
largest speed, stability margin), with Loft lessons L80 and L81 live. Nothing yet asked OpenRocket
to fly a design, and hpr had no written meaning for OpenRocket's summary words. L80 is Loft's
finding that one word can be a different quantity in another tool or version; Loft took OpenRocket
24.12's meanings from its source, which this project may not read. L81 is Loft scoring a metric as
0 when its event never happened. hpr has no stability-margin output yet (M1.10), so hpr's side of
the report needs its own session.

**Decision.**

1. **M2.2d splits in two.** d1 is OpenRocket's side: the flights, and a written definition for
   each summary word, with L80 and L81. d2 is hpr's side, carrying the parent's *done when*
   unchanged.
2. **OpenRocket flies every public configuration in calm air.**
   `validation/oracles/openrocket/flights.py` flies every motor configuration of the jar's 17
   examples and the seven Loft demos. Each flight takes the launch conditions of the design's first
   stored simulation, with no wind and no turbulence (the average wind model on all), so it can be
   repeated and flown by hpr. Extensions are not carried over. The conditions are recorded with
   each flight. Of the demos, only `demo-payload-separation.ork` has a motor OpenRocket finds, and
   `demo-quirks.ork` does not open (a booster set it refuses): 57 runs. One, *Pods--powered with
   recovery deployment* `[C6-7; 2× A3-4, B6-0]`, is aborted at 1.81 s ("Stage began to tumble under
   thrust"); its figures are where it stopped, so it is recorded as aborted and is no reference,
   which leaves 56 complete flights. One more flight, the simple example with its parachute set
   never to open, gives a complete flight with no deployment. The record is committed, as the
   automatic-radius record is, because every design is public; a second run is byte for byte the
   same, and a test holds the scripts' hashes to the committed files.
3. **Each summary word is measured, not read.** The record keeps each word beside the quantities
   of the time series it could mean, and the steps either side of every event with their time,
   speed and altitude. The test holds OpenRocket 24.12's definitions on all 56 complete flights, to
   1e-9:
   - the apogee, largest speed and largest Mach number are the peaks of their columns;
   - the time to apogee is the time of the highest step, not the apogee event (13 differ); the
     flight time is ground hit's, the last step;
   - the rod-clearance speed is at the first step past the rod's length (the step before is short
     of it on all 56), which is 0.06% to 7.50% above the speed at the rod's end;
   - the deployment and ground-hit speeds are the total velocity interpolated linearly at the
     event; 53 deployments fall between steps, and the deployment speed is the **last**
     deployment's (17 flights have a different first; on one the last is the slower);
   - the stability margin, which has no summary word, is the column at rod clearance, exactly
     (CP − CG) ÷ reference length (Niskanen 2009, p. 12), with OpenRocket's default reference.
4. **One word has no definition.** The largest acceleration is the peak of total acceleration
   before the first deployment (56 of 56; over the whole flight it differs on 15). But the optimum
   delay is not the apogee event less the last burnout on 15 flights, nor, on the same 15, the
   apogee event of the same configuration flown again with nothing deployed, which the record also
   holds. Those 15 are exactly the flights on which a recovery device deploys before apogee (an
   early ejection charge alone, on 5 others, changes nothing), and the re-flights' own optimum delay
   is their apogee event less their last burnout on all 56. So an early deployment changes the
   figure, but how is not measured; it is withheld: M2.2d1's
   *done when* asks each word to be defined **or** withheld with the flights that rule out its
   candidates, so the gap stays visible rather than guessed.
5. **Definitions are per tool and version.** `hpr_validate::flight_metrics::definition` gives
   OpenRocket 24.12's and RocketPy 1.13.0's (at the centre of dry mass, with the rail exit found
   between steps, ADR-021), and `None` for any other version, read from a `.ork`'s `creator`. No
   older OpenRocket jar is pinned, so its words are withheld rather than read as 24.12's.
   OpenRocket does not document which point its speeds belong to, so its point is *unstated*.
6. **A metric is never scored when an event is missing** (`flight_metrics::compare`). OpenRocket
   writes `NaN` exactly when the event is missing. When neither flight had the event, the metric is
   withheld; when only one did, the comparison fails, since the flights disagree. A value from hpr
   that is not a number fails too, and every metric of an aborted reference is withheld.

**Consequences.** L80 and L81 are live, owned by M2.2d1. The record is the reference for M2.2d2,
which flies hpr in the recorded conditions and compares the apogee, largest speed and margin at rod
clearance by these definitions: hpr's margin must be taken at the recorded rod-clearance step (its
time, Mach number and mass), and its speed there at the step past the rod, not at the rod's end.
The motor's centre of mass (ADR-067) meets the margin there. Stored results in files written by
other OpenRocket versions stay readable, but their summary words are withheld until another
version's jar is pinned and measured.

## ADR-069: hpr's flights of the public designs against OpenRocket's (2026-09-25)

**Context.** M2.2d2 carries M2.2d's *done when*: the public designs that fly are in a report
against OpenRocket (apogee, largest speed, stability margin), by the definitions ADR-068 measured.
hpr flies 21 of the record's 57 powered configurations, in five of OpenRocket's examples; the
motors of most come from OpenRocket's own database by digest (ADR-067), which lives under the
gitignored `corpus-out/`, and the examples from the pinned jar. hpr flies no recovery device read
from a `.ork` (ADR-057 reads them), and has no margin output (M1.10).

**Decision.**

1. **`cargo xtask ork-flights` flies them and writes the report.** Each configuration flies in its
   recorded conditions: a vertical rod of the recorded length, the site's latitude, longitude and
   height, hpr's standard atmosphere and calm air. `validation/reports/openrocket-flights.{md,json}`
   are committed. `--check` flies them again and compares, where the jar and the database record
   are fetched. CI does not fly them: its tests hold every OpenRocket figure in the report to the
   record, every outcome to `compare`, the summary and the page to the flights, and every powered
   configuration of the record to a flight or a named reason. After a change to the physics,
   `--check` must be run by hand.
2. **Each metric is taken by ADR-068's definition.** Apogee: the largest height of hpr's centre of
   mass, at its apogee event, counted from its height at the start, since OpenRocket's altitude is
   0 at launch. Largest speed: the centre of mass's, sampled at each step's end and three points
   inside it from the dense output, up to apogee. hpr's unbraked fall is left out, since
   OpenRocket's rockets come down under parachutes. Margin: at the recorded rod-clearance step's
   time and Mach number, hpr's centre of pressure with the air along the axis, less its centre of
   mass at that time, over its reference diameter. hpr's mass and centre of mass at that time are
   its own, reported beside OpenRocket's.
3. **Design checks are recorded, not enforced.** OpenRocket flies a design whatever hpr's checks
   find, so hpr flies it too and the report lists the errors its checks found: on two examples, an
   inner part 0.46 mm wider than the room for it, and a 29 mm motor in a 28.956 mm mount.
4. **No recovery is flown.** A reference whose parachute opened before its apogee (6 of the 21, plus
   2 with a part set to no drag) is marked with how long before the apogee of the same flight with
   nothing deployed, which the record holds, and summarised apart. Flying a `.ork`'s recovery is
   later work.
5. **A difference is summarised under its named cause, and each apogee off by more than 5% needs
   one** (M2.2's parent bar, applied here). Two causes are named: a reference parachute open before
   apogee, which puts OpenRocket's apogee lower; and a part whose drag OpenRocket is told is zero,
   which hpr reads (L63) but cannot apply (#165). For the second, the report flies the same
   configuration again with those parts removed. That is a probe, not the override, since it takes
   their mass, lift and shape too. A part set to any other drag coefficient stops the command.

**Consequences.** Measured on 2026-09-25: the margin at rod clearance is within 0.016 calibres on
all 21 (−0.0151 to +0.0037). With no named cause, hpr's apogee is low on all 12, by 0.06% to
4.34% (median −1.46%), and the largest speed −0.69% to +0.85% (18). The *Dual parachute
deployment* example reads 0.63% to 4.34% low on all six motors, untraced; its J570W flight, the
only one past Mach 1 (1.147), is the largest. Five apogees are more than 5% off, each with a named
cause:

- two C6-3 flights whose parachute opened 2.59 s and 2.99 s early (+12.19%, +13.80%). The same
  rockets' C6-7 flights, the same climb with no early parachute, read −0.16% and −1.07%;
- *Base drag hack (short-wide)*, whose aft transition is set to no drag: −15.47% to −19.13%. With
  the transition removed, its C11-5 flight reads −0.29% in apogee and +0.04% in speed. Its D12-3
  and E12-4 flights also have early parachutes, and the probe overshoots their speeds (+1.99%,
  +5.23%), so how the two causes split there is not measured.

M2.2d is met; M2.2e carries the corpus.

## ADR-070: M2.2e split: mass and centre of mass first, then the corpus (2026-09-25)

**Context.** M2.2e carries M2.2's *done when*: at least 20 designs in a report with an error
spread over apogee, largest speed, stability margin, mass and centre of mass (CG); a written
hypothesis for every apogee more than 5% off; private designs only as anonymised ids. After
M2.2d2 (ADR-069) the flight report covers 21 configurations of five public designs, and holds
each flight's mass and CG beside OpenRocket's, but gives no spread of them. OpenRocket has not yet
flown the private corpus (`refs/loft-fixtures`), and hpr has flown none of it. That is more than
one session.

**Decision.** Split M2.2e into four increments, each with its own *done when*; the parent's is
kept unchanged and is met only when the last is.

1. **M2.2e1, mass and CG in the flight report.** The report summarises hpr's mass at launch and
   at the rod-clearance step, in per cent of OpenRocket's, and its CG at that step, hpr's less
   OpenRocket's distance from the nose in OpenRocket's calibres, over every flight not aborted. A
   CG difference in calibres moves the margin by as much the other way, so the two read together.
2. **M2.2e2, OpenRocket's flights of the corpus.** `flights.py` flies every configuration of
   `refs/loft-fixtures` OpenRocket can, into the gitignored `corpus-out/`; only counts are
   committed.
3. **M2.2e3, hpr's flights of the corpus.** A report of anonymised ids beside the public one,
   together holding at least 20 designs with the five spreads.
4. **M2.2e4, the causes.** A written hypothesis for every apogee more than 5% off, with the
   evidence of its size where there is any.

**Consequences.** M2.2e1 adds no physics: its numbers are ones M2.2d2 already recorded, and a test
pins the arithmetic. On the 21 public flights, launch mass is within 0.21% of OpenRocket's and the
CG within 0.016 calibres. On the 12 flights with no named cause, mass is within 0.025% at launch
and 0.03% at rod clearance, and on the largest miss (−4.34%) hpr is the lighter: mass does not
explain those misses. The CG gap above 0.003 calibres is all on *Dual parachute deployment*,
whose launch mass matches by an override covering its parts.

## ADR-071: The corpus OpenRocket flies is its `.ork` files (2026-09-25)

**Context.** ADR-070's M2.2e2 has `flights.py` fly every configuration of `refs/loft-fixtures`
OpenRocket can. That library holds 27 `.ork` files, 4 RASAero `.CDX1` files and 4 RockSim `.rkt`
files. `flights.py` finds `.ork` files only. Named directly, the other eight give 3 driver errors
(`'NoneType' object is not iterable`, all `.CDX1`) and 5 files that open with no configuration
(`cargo xtask ork-flights --corpus corpus-out/openrocket-flights-other-formats.json`). OpenRocket
itself opens them (#168). hpr reads neither format until M3.4 (RockSim) and M3.5 (RASAero).

**Decision.** M2.2e's corpus is the library's 27 `.ork` files. The eight others are counted and
named here, not flown: M2.2e3 could not compare them, since hpr can't read them, and fixing the
driver belongs with the milestones that bring their readers (#168). The `.ork` flights are
counted by `cargo xtask ork-flights --corpus`, which prints no file name.

**Consequences.** OpenRocket flies 88 of the 89 configurations of the 27 `.ork` files to the end,
none refused or aborted. The 89th names a motor, with its digest, that OpenRocket loads no motor
for, so it has none to fly. The record keeps neither OpenRocket's loader warnings nor the digest
of the curve each flight used, so M2.2e3 must check that its curve is the file's before comparing.

## ADR-072: hpr's flights of the private library, under anonymised ids (2026-09-25)

**Context.** ADR-070's M2.2e3 has hpr fly the private library OpenRocket flew in M2.2e2, in a
report of anonymised ids that, with the public report, holds at least 20 designs with the five
spreads: apogee, largest speed, stability margin, mass and centre of mass. Measured before
deciding:

- Of the library's 27 `.ork` files, 13 are byte for byte OpenRocket examples in the public record
  (the public report compares 4 of them and lists the rest with a reason), and two more are edited
  copies of examples. One keeps the example's rocket name, stored conditions and configuration
  ids, with a changed comment, one moved freeform-fin point and new stored flight data; the other
  keeps its configuration and component ids, with the rocket renamed and all four motor
  digests changed. That leaves 12 private designs.
- hpr flies 17 configurations of 4 of them. The public report flies 5 designs. That is 9, not 20.
- The 8 private designs hpr flies no configuration of wait on: a motor igniting in flight, which
  staging brings (M1.9), 2 designs; an airframe read simpler than written, 5 (pods 1, the single
  `overridesubcomponents` flag OpenRocket later replaced 2, fin fillets 1, an inner tube taking an
  automatic radius from a nose cone 1); a tilted launch rod, 1.
- The public record has 19 designs hpr does not fly: staging 4, clusters 2, pods, parallel stages
  and tube fins 4, hybrid motors 2 (out of scope), a motor with no curve 1; and 6 of Loft's demos,
  5 declaring a motor with no digest that neither program finds a curve for, 1 OpenRocket refuses.
  M1.9 (staging, clusters, air starts) is worth up to 6 public and 2 private designs, so 17 in all.
  The last three can come from the tilted rod (#173, 1 design), the airframe readings (#174, 5
  designs, the old override flag alone 2), or the 4 public designs held by pods and parallel
  stages (M1.13) or tube fins (#133).

**Decision.**

1. **Split M2.2e3.** Milestone ids take one increment level, so the siblings are renumbered.
   M2.2e3 is now the report and its screens, and prints the count against the bar. A new M2.2e5
   takes M2.2e3's bar word for word, at least 20 designs with the five spreads, is blocked on
   M1.9 and then on three designs from #173, #174, M1.13 or #133, and also meets M2.2e4's bar on the flights
   it adds. M2.2e4, the causes, is unchanged and next, on the flights so far; then M1.9.
2. **Anonymised ids.** A private design is `C01` to `C12` in the order of its file's SHA-256, and a
   flight `C09/2`, the design's second configuration. The report's `ids_sha256` changes when the
   ids would move, so `--check` fails and prose citing an id is looked at again.
3. **Copies.** A file that is a public design, by its bytes or its rocket, is left to the public
   report, and a private design found twice counts once: two readings of one rocket are not two
   designs. A rocket is its name with its configurations' ids, or those ids alone when OpenRocket
   made them all (random UUIDs, which a renamed rocket keeps). The name alone is never enough,
   since OpenRocket gives every new rocket one default name, which 4 of the 12 keep.
4. **Differences, never values.** The committed report holds each flight's differences (apogee,
   largest speed and mass in per cent of OpenRocket's; margin, centre of mass and centre of
   pressure in calibres), the class of OpenRocket's largest Mach number and of the launch site,
   how early OpenRocket's parachute opened, how many parts carry a drag override, what hpr's
   design checks found by kind, and each unflown configuration's reason. A value and its
   difference in per cent give back the other value, so no value is published. Unrounded, a
   difference of one floating-point step gave back a launch mass's digits, so differences are
   rounded to 6 decimals, masses to 3 (across a design's configurations they give the ratios of
   its launch masses), and the parachute time to 0.01 s. A row with a key off a fixed list, or a
   number not so rounded, stops the report; tests hold every string to a closed list per key and
   the summary to the rows it is computed from.
5. **The curve OpenRocket flew.** A configuration is compared only when every motor's curve is
   the design's own or one supplied for its digest (ADR-067), and the motor record finds
   OpenRocket placing exactly those digests there. A curve from hpr's bundled catalog is found by
   name and could be another curve, so it is named, not flown (1 configuration). A design the
   motor record does not read back is an error, not a reason. The public report keeps all 21 of
   its flights under the same test.
6. **Only a vertical rod in calm standard air.** A tilted rod is listed with its reason (3
   configurations of one design) until OpenRocket's rod direction is pinned (#173). A drag
   override that is not zero, or one on the rocket or a stage, is named as a cause with no removal
   probe, which only stands in for a zero on a part. A record lacking what a comparison needs is
   an error; a flight hpr fails is listed with that reason, the detail printed locally only.

**Consequences.** The library's 17 flights: apogee −4.84% to +1.17%, none more than 5% off;
largest speed −0.65% to +2.28%; margin −0.0008 to +0.0730 calibres; launch mass within 0.004%;
centre of mass −0.0273 to +0.0042 calibres. Two designs carry the margin gap on every flight, hpr
reading the rocket more stable: `C03` by +0.056 to +0.073 calibres, +0.061 of it the centre of
pressure, and `C09` by about +0.04, half centre of pressure and half centre of mass (#172; M2.2e4
covers apogees only). The 14 flights launched above sea level all read low in apogee and the 3 at
sea level high, but they are two designs each, and the public report's flights, all at sea level,
read low: a pattern for M2.2e4 to test, not yet a lead. With 9 designs of 20 the parent's bar is
visibly not met, in the report, the guide and `STATUS.md`. `cargo xtask ork-flights --library`
needs the private library, so CI checks only the committed report's arithmetic and privacy.

## ADR-073: Each named cause sized by OpenRocket's own flight without it (2026-09-25)

**Context.** M2.2e4's bar is a written hypothesis for every apogee more than 5% from
OpenRocket's. The public report has five; the private report none. ADR-069 named two causes, an
early parachute (both *C6-3* flights, +12.19% and +13.80%) and a part stated to have no drag that
hpr does not apply (#165; the three *Base drag hack* flights, −15.47% to −19.13%). Loft excused its
largest misses with known issues (L82), so a name alone is not a hypothesis. The evidence then was
indirect: another configuration's flight for the parachute (C6-7), and hpr flown with the part
removed, which also takes its lift and shape, for the override. On the two flights with both
causes that probe overshot, and the split was not measured.

**Decision.**

1. **Size a cause by OpenRocket flying without it.** `flights.py` flies every configuration again
   with nothing deployed (as it did for the optimum delay) and now records that flight's apogee.
   Where a part states a drag coefficient, it flies a third time with nothing deployed and every
   stated coefficient cleared, which is OpenRocket flying what hpr flies (hpr reads the setting and
   applies none); and where every such part states zero and is neither the rocket nor a stage, a
   fourth time with those parts removed, the rocket hpr's removal probe flies. Each takes the
   summary's apogee, as the flight's own is taken, and records the ids of the parts it changed.
   `cargo xtask ork-flights` compares hpr's apogee with each and prints what the causes leave. The
   record must name the same parts hpr reads, or the report stops; a flight OpenRocket refused or
   aborted, or the oracle failed on, is shown with its reason and sizes nothing.
2. **What counts.** An apogee more than 5% off is brought within the bar by its named causes when
   every flight of OpenRocket's with all of them taken out, the cleared setting and the removed
   part alike, is within the same 5% of hpr's. The 5% is M2.2's own threshold, not a new one. The
   strictest comparison decides, because two differences can cancel in one of them. It is a
   code-to-code statement about OpenRocket's flight, not a claim that hpr would match with the
   cause modelled: hpr still flies no `.ork` parachute and applies no drag override.
3. **Both ways.** A test holds that over the record's 56 finished flights, the undeployed apogee
   equals the flown one to the bit on the 41 whose parachute opens at or after apogee, is higher
   on 14 of the 15 where it opens before, and on the 15th (0.10 s early) differs by 1 mm, within
   the 3 mm the deployment event alone can move the peak of rows 0.05 s apart. It also holds each
   flight's summary apogee equal to its altitude column's peak. Another test recomputes every
   sized number and count from the record.
4. **The library.** Its record is written by the same script, so it was flown again; its
   rows are unchanged, none of its 17 flights is more than 5% off, and M2.2e5 adds these columns to
   it when it has one.

**Consequences.**

- The bar is met: each of the five has a written hypothesis with its size. Four are brought within
  5%. With nothing deployed the *C6-3* flights read −0.16% and −1.07%. On the *Base drag hack*,
  holding the parachute alone gives −16.77%, −20.80% and −20.50%, since the early parachute and
  the ignored setting pull opposite ways. With the setting cleared too they read +1.34%, +3.51%
  and +4.79%, and with the cone removed from both programs +1.15%, +4.76% and +7.80%.
- The E12-4 is not brought within 5%. Its +4.79% rests on a cancellation: the cone costs hpr 84.0 m
  of apogee and OpenRocket 71.1 m. Like for like it is +7.80%.
- What is left on the *Base drag hack* is not explained, and it has the other sign from the 12
  flights with no named cause (−0.06% to −4.34%). It is not the cone's own drag, since it stays
  with the cone removed from both. A same-state comparison of the two programs' drag by part, a
  scratch probe that is not committed, suggests two candidates, neither sized in apogee. One is the
  nose: OpenRocket charges this blunt ellipsoid nose (length 0.58 diameters) subsonic pressure
  drag, 0.0117 at Mach 0.1 to 0.0482 at Mach 0.25 on the 4.87e-3 m² cross-section. hpr charges it
  none below Mach 0.8, where hpr joins Stoney's ellipsoid curve (measured from Mach 1.2) to 0, and
  eq. B.9 keeps that 0 at any fineness. The other is the base while the motor burns: hpr takes the
  motor's area off it (0.1100 against 0.1213 at Mach 0.1), OpenRocket 24.12 does not. Which
  program is right is not measured; #177 holds it.
- Flying the corpus again moved no row of the private report. From one run to the next of the same
  jar and Java, OpenRocket's figures for six library designs moved in the 13th to 16th digit, and
  one apogee by 5e-7 of itself; the public record came back bit for bit.

## ADR-074: Ignition times and powered staging: the sustainer flies on as a rigid body (2026-09-25)

**Context.** M1.9 asks for stage separation on a burnout plus a delay, a height or a time; sustainer
ignition; the booster flown through its recovery; clusters in one mount with their thrust offset;
and a two-stage and a cluster design each within the per-case tolerance of OpenRocket. What was in
hand: every motor lit at `t = 0`; a separation (ADR-014) allowed only after the last burnout, with
both bodies descending as point masses; and ADR-014's warning that a body flying on would need "an
aerodynamic model for a headless stack". Loft staged before the flight, so an apogee or height
separation fell back to the burnout and no booster was flown (L30); its tests held the sustainer's
ignition time, the mass step and a trigger that never lights (L93). That is more than one session.

**Decision.**

1. **Split into a to c.** M1.9a: ignition times and powered staging (L30, L93), and the parent's
   event-ordering bullet. M1.9b: several motors in one mount and a motor out (L31). M1.9c: the
   `.ork` ignition and separation settings and clusters flown against OpenRocket, the parent's
   first bullet. Each has its own *done when* in `ROADMAP.md`; the parent's is unchanged.
2. **Ignition belongs to the configuration.** `MountedMotor::ignition` is `Launch` (the default,
   and omitted from a design file, so older files read the same), a `Time` after launch, a `delay`
   after another mount's motor's `Burnout`, or a `delay` after the `Separation` that frees the
   motor's stage. Times are on the flight's clock (`t = 0` at launch). Each motor burns on its own
   clock from its ignition, and one not yet lit, or never lit, is carried loaded. Every time but a
   separation's is known before the flight and is a stop time, with the curve's knots shifted by
   it. Assembly refuses a burnout of a mount holding no motor, a chain of burnouts that returns to
   itself, and a time or delay that is negative or not finite.
3. **A powered separation keeps the nose body flying.** When the separation fires and body 0 still
   has a motor to burn (burning, due at a known time, or lit by this separation), body 0 is the
   sustainer and flies on in six degrees of freedom. The flight's state is the nose tip's
   (ADR-011), which the sustainer keeps, so the state carries across unchanged. The integrator
   restarts there, because the mass steps down by the booster's. Its aerodynamics are hpr's own, on
   the design cut after the boundary, built only when a separation is powered, so an unpowered one
   never needs it: the sustainer keeps the nose, so it needs no headless model. Only a body
   without the nose would, and that body is the booster, which descends as a point mass under its
   own device as in ADR-014. The booster's motors must be spent: a separation while one burns or
   is still to light is refused, before the flight when the time is known and in flight
   otherwise. **A device on the booster must be open at the split.** In the first draft the
   example's booster, its tumble set to its own apogee, left at 341 m/s and coasted drag-free: it
   landed at 350.18 s, against 47.21 s with the tumble open at the split, having climbed far above
   the sustainer. A body-1 device with `Time { time_s: 0.0 }` opens there, since a body's
   devices act only once it flies. hpr tumbles the booster side-on (ADR-012) from the instant it
   separates, while a real finned booster flies nose-first for a while, so from near Mach 1 it
   stops almost at once (the example's peaks 102 m above the split). That is likely low, and #179
   holds a model of the booster's own drag.
4. **An unpowered separation is ADR-014's, unchanged.** If body 0 has no thrust to come, both
   bodies descend as point masses. So a sustainer that separates after its own burnout, in free
   flight, still falls without airframe drag until its device opens. That is ADR-014's limit, not
   a new one, and it is kept rather than changed here, because it is what every existing
   separation test flies.
5. **No impulse.** The sustainer keeps the nose tip's velocity and rotation; the booster leaves
   with its own centre of mass's velocity, `v_O + ω × r`. With the sustainer unlit and the booster
   spent, the two momenta add to the stack's (a test, to 1e-9). With the sustainer burning,
   `m v_cg` is not additive, because the motion of the centre of mass inside the body carries mass
   flow terms, so no such claim is made.
6. **Triggers and events.** `Trigger::Burnout { motor, delay_s }` fires a delay after a motor's
   burnout, for separations and devices alike. A trigger on a motor that never lights never fires.
   The height trigger stays ADR-012's (descending); an ascending one waits for M1.9c, if the `.ork`
   settings need it. `EventKind::Ignition(i)` records a motor lit after launch. `Burnout` is
   recorded when no motor is burning or due to light at a known time, and again if one lights
   afterwards, as a sustainer lit by its separation does. So the same staging planned two ways
   (lit at the booster's burnout plus 1 s, or at a separation 0.5 s later plus 0.5 s) records one
   `Burnout` or two, and `FlightResult::event` returns the first. A per-motor burnout event is
   left for when a report needs it.
7. **Refused rather than guessed.** A drag or normal-force table is the whole stack's, so a
   powered separation under one is an error. So is a powered separation after a device opened on
   body 0, which the point-mass descent can't fly under thrust. A separation timed from a motor
   with no ignition known before the flight (the sustainer it lights, say) could never fire, and is
   refused when given; so is a separation ignition in the last stage, which nothing can free.
8. **Found on the way.** A body whose device opened on the stack before the separation was refused
   as reaching the ground with nothing open: the check read only the body's own events. An apogee
   separation with an apogee parachute on the sustainer hit it. The check now reads the devices'
   state. The test that pins the refusal (`a_body_whose_device_never_opens_is_refused`) passed
   only because of that bug. Its booster's trigger, a height above the body, fires at once on a
   body already descending below it (the altimeter rule). It now uses a time after the landing,
   and asserts that the booster is the body refused. Review also found the stack's apogee event
   firing a booster's apogee charge while the booster was still attached; it now fires only the
   stack's own devices, and each body finds its own apogee.

**Consequences.**

- M1.9a's bar is met by tests in `hpr_sim::staging` and `hpr_design::config`. They cover the
  sustainer's ignition at the booster's burnout plus its delay (to 1e-12 s), and the mass stepping
  down by exactly the booster's and holding until the sustainer lights. They cover a separation
  that never fires, leaving the sustainer unlit and loaded to the ground. They cover the event
  order (liftoff, rail exit, separation, ignition, burnout, apogee, landing), an apogee and a
  height separation firing in flight with both bodies landing, an air start at 3 s, linear
  momentum on a rail 10° off vertical, and each refusal.
- The staged flight is not validated. M1.9c compares it with OpenRocket; until then the staging
  page says so.
- Not modelled: the booster's own airframe drag (#179), an ejection or separation impulse, the
  flow between separating bodies, a motor's ignition transient beyond its curve, and the booster's
  attitude (ADR-014).

## ADR-075: A cluster is one tube repeated, and a motor in it one motor per tube (2026-09-25)

**Context.** M1.9b asks that several motors in one mount sum their thrust and mass, that a motor
out give the pitch moment a hand calculation predicts (Loft lesson L31: Loft's clusters were on
the axis only), and that `.ork` clusters be read. What was in hand: a mount held at most one motor,
so a cluster needed a mount per motor; the thrust already acted at each nozzle with its moment
(M1.3), so an off-axis motor already turned the rocket; and the `.ork` reader read a clustered
inner tube as the one tube it is written as, with a warning, leaving its configurations out
(ADR-064). No document gives OpenRocket's cluster patterns: `clusterconfiguration` names one
(`3-ring`, `4-ring` and so on), `clusterscale` spreads it and `clusterrotation` turns it.

**Decision.**

1. **A cluster is a list of places on the inner tube.** `InnerTube::cluster_m` holds each tube's
   axis, `[x, y]` in body axes from the axis the tube's radial offset and angle give; empty is one
   tube, as before, bit for bit. The part weighs every tube, each with its own parallel-axis term.
   Whatever the tube holds (an engine block, a mass) is repeated in every tube, so each placed
   component carries its copies (`PlacedComponent::copies_m`) and its mass counts them all. A mass
   override on a cluster sets the whole cluster's mass, as OpenRocket's does; an override on a part
   inside it is each copy's (OpenRocket 24.12 weighs an engine block overridden to 0.01 kg in a
   3-ring as 0.03 kg), so a part is weighed and overridden in one tube and then repeated. A
   general list rather
   than named patterns keeps the library free of OpenRocket's names; the `.ork` reader turns a
   pattern into the list.
2. **A motor in a cluster is one motor per tube.** A configuration still names one motor per
   mount. Placing it gives one `PlacedMotor` per tube, one after another in the order of the tubes,
   each with its nozzle on its tube's axis. Thrust, mass and the moments then sum over the motors
   as they always did, with no new term in the equations of motion. A cluster mount holds one
   kind of motor, as an OpenRocket clustered tube does; a mixed cluster is a mount per kind, each
   motor its own, so none is sent anywhere as copies of another (Loft's lesson L31). A motor
   index (a device's `Trigger::MotorDelay`, say) counts every placed motor. A burnout of a
   clustered mount is the burnout of its first motor that lights, since they light together.
3. **A motor out is a failed tube.** `MountedMotor::failed_tubes` names tubes whose motor never
   lights: carried loaded, giving no thrust. It serves what-if studies, as a motor that fails to
   light is the cluster's common failure. A sustainer's cut after a powered separation rebuilds
   one entry per mount, keeping the failed tubes.
4. **OpenRocket's patterns, measured.** `validation/oracles/openrocket/clusters.py` asks
   OpenRocket 24.12, run as an external oracle through its public API, for each of its fourteen
   patterns' points and, on 25 probes, for where it puts every tube and what it weighs. The points
   are the figures they look like, in units of the separation between neighbouring axes: rows one
   apart, a triangle and a square of side one, a pentagon of side one, rings of radius one round a
   centre tube (the stars), and a grid and an eight-ring of spacing 1.4. A tube's place is
   `2 R s · Rot(θ − ρ) · pₖ` from the cluster's axis, for the tube's outer radius `R`, the scale
   `s`, its roll angle `θ` and the rotation `ρ` (degrees in the file), with OpenRocket's `(y, z)`
   read as hpr's `(x, y)` as every roll angle is. hpr's reading puts every tube of every probe
   within 1e-15 m of OpenRocket's. A name OpenRocket has no pattern for (`4-square`), it reads as
   one tube; so does hpr, with a warning.
5. **OpenRocket weighs a cluster's tubes stacked on its axis.** On its probes the mass and centre
   agree with hpr's to 1e-12, but a 3-ring at scale 1 and at scale 1.5 have the same inertias:
   OpenRocket counts each tube's own inertia and no parallel-axis term for its spread, while an
   engine block inside each tube is weighed where it is, spread and all. hpr places each tube where
   it is, as the physics asks, and does not copy that. On every cluster on the body's axis, hpr's
   roll inertia is OpenRocket's plus `Σ m |c|²` over the tubes and its pitch inertia (the mean
   across the axis) plus half of it, to 1e-12; on the 3-ring probe that is +5.11% in roll and
   +0.273% in pitch. Off the axis the roll rule holds as well, but three departures are left and
   pinned: a lone tube 10 mm off the axis, whose offset OpenRocket leaves out (+0.303% roll), and
   the pitch of two clusters off the axis (+0.015%, +0.021%), which hpr has not traced.
6. **The survey's cause is sized.** `cargo xtask ork` held the four designs with a cluster outside
   1% in mass and centre, traced to the one-tube reading. They are now within, and their roll
   inertia is outside by +1.01% and +2.08%. The survey names the cause, OpenRocket's stacking, only
   when hpr's roll less the clusters' own spread is within 1% of OpenRocket's; it is +0.04% and
   +0.00%.
7. **Flights of `.ork` clusters wait for M1.9c.** A configuration with a motor in a clustered tube
   is still left out of hpr's flights of a `.ork` (`NotFlown::Cluster`), as one lit in flight is,
   until M1.9c holds such a flight to OpenRocket's. The design itself reads whole, and hpr flies
   it when asked.

**Consequences.**

- M1.9b's bar is met by tests. `hpr_design` holds a cluster's mass, its contents repeated and its
  motors placed, to hand calculations. `hpr_sim::staging` holds the thrust and mass of three
  motors in one mount, and a motor out's angular acceleration at rest to the hand calculation
  `I_c⁻¹ Σ (pᵢ − c) × T ẑ` within 3.7e-7 (L31's test,
  `cluster_motor_out_produces_pitch_moment`). `hpr_io` holds the fourteen patterns, and
  `hpr_validate` every probe's tubes and mass.
- The mass survey's counts move: 62 of 71 within 1% in mass (was 58), 63 in centre (was 59); roll
  with OpenRocket's fin rule 52 within 1% (was 56), the four clusters' spread now the named cause.
- Not modelled: a ring's automatic bore round a cluster is the tube's own radius, as OpenRocket
  gives it, so the tubes run through the ring, and the design checks warn of it. A part inside a
  tube off the body's axis is read at its own radial offset from the body's axis; the `.ork`
  reader adds no parent's offset to it, as before, where OpenRocket places it from the tube's axis
  (measured with an engine block in a tube 10 mm off the axis). The reader warns of it and no
  survey file has one; a cluster on the axis, the common case, is not affected
  ([#181](https://github.com/nrdptel/hpr-sim/issues/181)). Real clusters' thrust misalignment and ignition spread, short of a motor that fails
  outright, are not modelled.

## ADR-076: A `.ork` file's ignitions and one powered separation flown against OpenRocket (2026-09-25)

**Context.** M1.9c carries M1.9's first bullet: a `.ork` two-stage design and a cluster design each
within the per-case tolerance of OpenRocket's flight. hpr-io read every motor's ignition and every
stage's separation (M3.1c2, ADR-056) but flew only what lights at launch (ADR-055), left clustered
configurations out (ADR-075 §7) and flew no rocket of more than one stage. No tolerance for a
flight of a `.ork` against OpenRocket's had been written down: ADR-069 holds each apogee more than
5% off to a named cause, and holds nothing to a bound.

**Decision.**

1. **The tolerance, set before any staged or clustered flight was measured.** A design is within
   when every configuration of it that hpr flies has its apogee within 5% of OpenRocket's and its
   largest speed within 5% of OpenRocket's. Where OpenRocket's parachute opened before its apogee
   (ADR-069 §4), the apogee is held to OpenRocket's flight of the same configuration with nothing
   deployed, which the record holds (ADR-073). 5% is M2.2's bar for an apogee with no named cause;
   the largest speed is held to the same.

2. **Ignition, as the file says it.** `launch` plus `d`, and `automatic` plus `d` in the bottom
   stage, light at `t = d` (OpenRocket's *Airstart timing* example writes its air starts as
   `automatic` plus 1, 2, 4 and 6 s, and its record lights them then). `automatic` in a stage above
   is `ejectioncharge`: the burnout of the stage below's motor plus its ejection delay and `d`.
   `burnout` is that burnout plus `d`. Each becomes an `hpr_design::Ignition` (ADR-074 §2). The
   stage below is the next stage aft, and its motors must sit in one mount, whose first burnout is
   then the stage's. A configuration is left out (`IgnitionNotFlown`) when a motor is set `never`,
   names a word hpr does not know, waits on a plugged motor's charge, or on a stage below with no
   motor or with motors in more than one mount. hpr has no motor that is never lit on purpose, and
   choosing among mounts would be a guess. One private configuration sets its bottom stage's motor to `burnout`,
   with no stage below; OpenRocket's record lights it at launch. One flight is not a rule, so hpr
   leaves it out rather than read the word as `launch`.
3. **One powered separation.** hpr's flight takes one separation, so a configuration flies at most
   one. Its time comes from the file: `launch` plus `d` is a time; `ignition` and `upperignition`
   are the ignition of the stage's own motor or of the stage above's, plus `d`; `burnout` and
   `ejection` are the stage's own motor's burnout plus `d`, and plus its ejection delay for
   `ejection`. `never` flies the stack whole. A missing delay is 0, as a missing ignition delay is;
   a negative or non-finite one on a separation hpr flies is refused. The separation is powered when, at that time, a motor
   ahead of it is still burning or yet to light: the test hpr-sim makes when it fires (ADR-074 §3),
   made here from the times rather than from where the motors sit, so a file whose sustainer has
   already burnt out by the split is not called powered. A motor behind the split still burning
   there is refused too, as hpr-sim would. The reader gives the split as
   `hpr_io::ork::Staging` (the boundary, a `Time` or `Burnout` trigger, and the resolved time), and
   `hpr::ork::separation` turns it into hpr-sim's `Separation`: the rocket the configuration builds
   does not carry it, so a program that flies a staged `.ork` configuration must pass it on, as
   the example `ork_two_stage` does. Left out (`SeparationNotFlown`): two stages or more that
   separate (#183), a stage that states no event, `altitudeascending` (hpr has no such trigger),
   and anything else.
4. **An unpowered separation is the descent's.** `apogee` and `altitudedescending` can't come
   before apogee: the climb is the whole stack's in both programs, and the separation is part of
   the descent, which hpr's `.ork` flights do not fly (ADR-069 §4), so the configuration flies
   whole. That assumes every motor is spent by apogee; `cargo xtask ork-flights` refuses to report
   a flight where one is not. Any other separation with nothing ahead of it left to burn could
   come before apogee,
   where hpr would fly both parts as point masses without their airframes' drag (ADR-014), so it
   is left out (#184).
5. **Clusters fly.** `NotFlown::Cluster` is dropped: a configuration with a motor in a clustered
   tube flies with a motor in every tube (ADR-075 §2). OpenRocket still weighs the tubes stacked on
   the cluster's axis (ADR-075 §5), which the report does not hide.
6. **The report's flight.** `cargo xtask ork-flights` gives a staged configuration its separation
   and the two devices hpr's descent needs on separated bodies (ADR-074 §3): the booster tumbling
   from the split, the sustainer from its apogee. Neither acts on the climb, which is what is
   compared. OpenRocket's record holds its branch 0, the one that keeps the nose, so both
   programs' apogee and largest speed are the sustainer's. A staged flight records its separation
   time beside OpenRocket's, and a clustered flight its count of clustered motors. The mass and
   centre of mass at rod clearance now count a motor as loaded until it lights.

**Consequences.** Measured on 2026-09-25, after §1 was committed:

- *Two stage high power rocket*, both configurations: `[H148R-0; H148R-0]` apogee −1.79%, largest
  speed −0.54%; `[I59WN-P; I357T-14]` −0.13% and −0.04%. Both separate when OpenRocket's do, at
  1.535 s and 1.515 s.
- *Clustered motors* (four motors in a `4-ring`), all five: apogee −0.35% for the A8-3; −0.79%,
  −0.72% and −0.72% for the B4-4, C6-3 and C6-5 against OpenRocket's flights with nothing deployed,
  whose parachutes opened 0.37 s, 2.59 s and 0.59 s early (+9.43% for the C6-3 against its
  record); −0.72% for the C6-7. Largest speed +0.20% to +0.92%.
- *Airstart timing* (a `3-ring` of I211W beside a K550W in its own mount, the ring lit at launch or 1, 2, 4 or 6 s
  after): apogee +0.48% to +1.03%, largest speed +0.46% to +0.81%.
- So M1.9's first bullet is met, and a test (`a_two_stage_and_a_cluster_design_are_within_5_percent_of_openrocket`)
  holds the committed report to §1.
- OpenRocket's mass on `[H148R-0; H148R-0]` falls 1.9992 times as fast as hpr's before the
  sustainer lights: 0.0823 kg against 0.0412 kg from launch to rod clearance. The launch masses
  agree to 3e-6 kg, and on the other configuration, with two different motors, the rod-clearance
  masses agree to 1.05e-6 of the mass. OpenRocket's recorded mass falls as if both H148Rs lost
  propellant from launch. Its effect on the apogee
  is not sized (#185).
- The public report flies 33 configurations (was 21), 6 apogees more than 5% off, each with a named
  cause (was 5), 5 of them within 5% once it is removed. The private library's flies 18 (was 17),
  in 5 private designs (was 4), and the two reports make 13 designs toward M2.2's 20 (was 9).
- Not flown: the three-stage example (#183) and two payload designs whose separation has no motor
  ahead of it (#184).
- The private flight newly flown, C08/1 (two stages), reads +0.1108 calibres in margin because its
  centre of mass at rod clearance is 0.1102 calibres forward of OpenRocket's, its mass within
  about 0.001%. That predates this record: hpr's structure alone is forward by less than the mass
  survey's 1% of length, and a stage mass override several times its parts' weight magnifies it.
  Two parts differ: an airfoil fin set (ADR-062 §4's kept departure) and packed parachutes
  whose automatic radius OpenRocket may resolve by stretching the packed length (#186).
- The 5% on the largest speed is loose: every flight above is within 0.93% of it, so a speed error
  of several per cent would still pass. It is kept because it was set before measuring (§1); the
  report's numbers are what to read. CI does not fly OpenRocket, so it checks the committed report
  against §1, and `cargo xtask ork-flights --check` re-flies hpr's side on a machine with the jar.
- At the split, hpr's reference point, the centre of mass, jumps forward from the stack's to the
  sustainer's, which on a vertical flight raises its height by that distance. Whether
  OpenRocket's branch 0 altitude does the same is not measured (#187). The jump is less than the
  stack's centre of mass from the nose at the split, which is forward of where it is at rod
  clearance, since only the booster burns between: 1.320 m then on *Two stage high power rocket*,
  under 0.2% of the H148R apogee (666 m) and under 0.1% of the other (1382 m), about the size of
  that flight's whole gap (−0.13%).

## ADR-077: Flight metrics: peaks on the dense output, margins only where they mean something, and `None` for what didn't happen (2026-09-26)

**Context.** M1.10 asks for the stability margin over the flight, the optimum ejection delay, max
q, flutter, the landing point in latitude and longitude, and five file exports. That is more than
one session, so it is split in three: a, the flight metrics; b, fin flutter; c, the exports. Four
Loft lessons bear on a: L33 (margins of ±12 to 15 calibres published as the net normal-force slope
went to zero), L34 (the opening shock taken as the peak acceleration, and a finite difference that
read a thrust spike low), L35 (zeros for "never happened", and heights with no datum) and L94 (an
optimum delay that moved with the delay flown). hpr-sim is a pure core crate, so the metrics can
compute but not write files.

**Decision.**

1. **The split.** M1.10a: a `metrics` module in hpr-sim with an observer, `FlightMetrics`, and a
   `FlightSummary`; L33, L34, L35 and L94 go live. M1.10b: flutter from its primary source, with
   L32. M1.10c: the exports, written as text or bytes by the core, with the parent's schema and
   parsing checks. The parent's three *done when* bullets are unchanged; a carries the first, b
   the third, c the second.
2. **Peaks on the dense output.** The observer evaluates the equations of motion at each accepted
   step's start, middle and end. When the parabola through the three bends down with its top inside
   the step, it runs a golden-section search (Kiefer 1953) on the step's dense output, down to 1e-9
   of the flight's clock (at least 1 s), and keeps the largest of that and the three samples. (A
   first version searched only when the middle beat both ends, and missed peaks in a step's outer
   quarter by up to 1e-4: Juno III's max q, 27205 Pa for 27208.)
   Thrust-curve knots and events already end steps (M1.6a), so a spike's peak is a step's end.
   Peaks start at liftoff; a rocket that never lifts off has none. Measured on Valetudo in a
   vacuum: the peak acceleration matches the hand value from the motor and the masses to 1.6e-7
   (the test holds 1e-6), where a 100 Hz finite difference reads it 1.3% low.
3. **Acceleration.** The nose tip's (the body origin's) acceleration relative to the launch frame,
   as `Sample::acceleration_enu_m_s2` gives it, including gravity. The boost's peak is kept in the
   rail and free phases, the opening shock's in the descent phase, apart (L34).
4. **Two margins.** The static margin is the centre of pressure at Mach 0 with the air along the
   axis, against the centre of mass of the instant: RocketPy's `static_margin`. The roadmap's
   "dynamic" margin is read as the flight margin: the margin at the flight's own Mach number with
   the air along the axis, defined as RocketPy's `stability_margin` is (RocketPy's
   `min_stability_margin` takes its least over the whole flight, rail and descent included, so it
   can differ). A pitch damping ratio, the other reading, is not built here. Both margins are kept
   at each step's end from the rail exit to apogee or the first deployment, since on the rail the
   rail holds the rocket; a powered separation adds the sustainer's own entry at the split. The
   least of each is searched for inside steps, as a peak is, and a later least replaces an earlier
   one only when lower by more than 1e-12 of the earlier (or 1e-12 calibres below one calibre), so
   a flat least keeps its first time. The angle of attack is left out, after three versions that
   followed it failed review: its least came at the apogee in calm air (1.28 calibres on Valetudo); a floor on the dynamic
   pressure at the rail exit's let the apogee through off a tilted rail, which still crosses the
   air there as fast as it left the rail; and a 15° cap on the angle (the limit on a fin's cant)
   put the least on the cap, where it moved with the step size (0.84 to 1.23 calibres on one
   trajectory in review), and hpr models no stall to justify that cap. `metrics::margin` gives the
   margin at any angle for whoever wants it.
5. **No margin where it would be noise (L33).** With `κ = Σ |C_Nα,i| / Σ C_Nα,i` over the
   components (the table's own slope with a normal-force table), each acting at a station on the
   rocket, the centre of pressure lies within `κ L` of every station, so a fractional error `ε` in
   one slope moves it by up to about `ε κ² L` (to first order). A component that is a pure couple
   on its own has no station, and `κ` doesn't count it. The margin and
   the centre of pressure are `None` when the net slope is not positive or `κ > √10`, where a 1%
   error can move the centre of pressure a tenth of the rocket. (A first draft took the bound as
   `ε κ L` and the limit as 10; review showed the bound fails when the centre of pressure lies off
   the rocket, which is when `κ` is large.) The limit is a chosen bound on that sensitivity, not a
   measurement. All 13 designs in `validation/designs/` stay below `κ = 1.35` from Mach 0 to 2
   and to 20°, so it
   leaves ordinary designs their margin. The pitch-moment slope about the centre of mass,
   `C_mα = −(Σ C_Nα,i x_i − x_cg Σ C_Nα,i)/d`, is always given, from a new
   `NormalForce::moment_slope_m` that keeps a component's pure couple.
6. **The optimum delay (L94).** A crate-private copy of the simulation with every stack charge
   held flies to its apogee; each motor that burns out before it gets `apogee − burnout`. A
   separation with nothing ahead of it left to burn is recovery and is held too; a powered one
   still happens, and the motors of the body it drops get no optimum, since their charges fire in
   that body. No apogee gives `None`.
7. **Datum and absence (L35).** Heights are the centre of mass's ellipsoidal height above the
   site's; the summary adds the starting height, and the apogee's gain from it (OpenRocket's
   altitude). Every metric that may not happen is an `Option`, which is `null` in JSON.
8. **Landings.** The flight's own (the stack's, or the sustainer's after a powered separation) and
   each separated body's that landed, in WGS 84 latitude and longitude through `LaunchFrame`, with
   east and north metres and the ground-hit speed.
9. **`FlightStep::stability`.** Each step gives the margins at a time from the model flying (the
   sustainer's after a powered split), so a new required trait method; hpr-validate's test stub
   refuses it. A watcher keeps one flight, and `summary` refuses a flight unless it saw each of
   its accepted steps once.

**Consequences.** The site gains *Flight metrics*, with an example whose output CI checks. The
margin limit of √10 is a choice that a later model of component uncertainties could replace with a
measured one. The metrics watch only the main flight: a separated body gets a landing but no
peaks.

## ADR-078: Fin flutter by NACA TN 4197: the lower reading wherever the source leaves room (2026-09-26)

**Context.** M1.10b asks for the flutter speed and margin from a cited primary source, matching its
worked example, with each shear modulus cited (L32: Loft's constant was half the source's, and 7
of its moduli had no source). The source is D. J. Martin, NACA TN 4197 (1958), eq. 18, which the
hobby community's flutter formulas descend from. Martin's appendix derives it from
Theodorsen and Garrick's flutter speed; his worked examples (pp. 6–7) read his figure 4.

**Decision.**

1. **Martin's eq. 18 as printed, constant from eq. 16.** `(V_f/a)² = G_E / D` with
   `D = (24εγ/π) p · A³/((t/c)³(A + 2)) · (λ + 1)/2`, `ε = 0.25`, `γ = 1.4`. The constant is
   computed (`24 · 0.25 · 1.4/π · 14.696 psi = 39.29 psi`), not his rounded 39.3.
   `hpr_sim::flutter::FlutterPanel` holds `A`, `λ` and `t/c`; `λ` must lie in `[0, 1]`, where
   his taper factors are defined.
2. **A flutter dynamic pressure.** Eq. 16 depends on the air only through `ρa²`, so it fixes
   `q_f = π G_E / (24 ε X (λ + 1))` at every height, and `V_f/V = √(q_f/q)`. The margin of a
   flight is that ratio at its max q, which M1.10a's watcher already finds on the dense output; no
   new observer is needed. A booster's fins get the whole flight's max q, which can only
   understate their margin.
3. **The worked example is matched at Martin's printed resolution.** His examples are chart
   readings: `X` "about 1.25 × 10⁶" psi (eq. 19: 1.228, which rounds to 1.25 at his 0.05 steps),
   and titanium thicknesses 2.5, 4.5 and "about 6.5" percent (eq. 19: 2.54, 4.61, 6.43, each
   those at his half-percent steps). The test asserts that rounding, not a percentage tolerance.
   His verdicts on the first wing (magnesium in the flutter region, aluminium marginal, steel
   probably safe) and titanium's margin are the example's margin half: with the moduli he marks
   on figure 3's axis, each box measured on the scan, magnesium's figure 3 ratio lies wholly above
   his band, aluminium's overlaps it, steel's and titanium's lie below.
4. **Martin's line is a measured band, and `V_f/V = 1` is not it.** Figure 3's shaded band,
   measured on a 250 dpi scan (both log axes calibrated on their ticks, 69 columns traced from
   `G_E` = 0.05 to 10 × 10⁶ psi; the axis runs to 20), runs at `D/G_E` = 0.25 to 0.31 all along
   that range: `V_f` of 1.8 to 2.0 times the speed of sound,
   for wings that flew to at least Mach 1.3. `FIGURE_3_BAND` holds it and
   `FlutterPanel::figure_3_ratio` gives `D/G_E`, the reading that decides. The hobby convention of
   `V_f/V = 1` as the limit isn't calibrated by the source, and no fixed `V_f/V` is: at max q,
   `D/G_E = 1/(M · V_f/V)²`, so the band is at `1.8/M` to `2.0/M`, and a fin is below it only
   above `2.0/M`. (A first draft treated 1 as the line, and a second 1.5; review caught both.)
5. **The lower flutter speed wherever the source leaves room.** The thickness ratio is taken at
   the root, the smallest on a constant-thickness fin. A solid fin's `G_E` is its material's `G`,
   as Martin's text says (p. 6), though his eq. 12 with a flat plate's `J = ct³/3` would give
   twice that (a `√2` higher speed). His `(λ + 1)/2` replaces `1/(f₁² f₂²)`, which it exceeds by
   up to 47% at `λ = 0.31` (17.5% lower speed); it is kept, since his figure 3 was drawn with it.
   One reading goes the other way and is stated: an airfoiled fin's `G_E` by eq. 12 is `0.946 G`,
   so its `V_f` is up to 2.7% high.
6. **Shear moduli with their own sources, beside the densities.** `Material` is unchanged (a
   design stores a density only); `hpr_design::materials::SHEAR_MODULI` gives 14 built-in
   materials an in-plane shear modulus with source, page, URL and basis: metals from MIL-HDBK-5J,
   carbon from NCAMP's AS4/8552 `G₁₂`, plywood from Riga Wood's panel shear, woods from the Wood
   Handbook's `G_LT/E_L` times 1.10 × the bending modulus (its footnote), and nylon and acetal as
   `E/(2(1 + ν))` from their data sheets. Where a source gives a range, the lower is kept. No
   source found gives G10/FR-4's, or PLA's, ABS's, PETG's, polycarbonate's or acrylic's; they have
   none, and the caller passes one.
7. **Refused, not guessed.** Elliptical, freeform and reverse-tapered fins return
   `SimError::Unsupported` (a new variant): Martin's taper factors are for trapezoids tapering
   outward. A panel's numbers are checked when it is made and when it is read from JSON.

**Consequences.** hpr's flutter numbers are a screening check, not a flutter analysis, and are
not shown to be conservative as a whole: the fin's mounting, sweep and Mach effects are left out,
and nothing is checked against a hobby rocket. On the synthetic 54 mm rocket on an I175, 3.2 mm
birch plywood fins reach 1.75 times `V_f` at max q; carbon fibre has `V_f/V` of 1.46 but a
figure 3 ratio of 0.45, above the band; aluminium passes both (3.40 and 0.083). A later milestone could add a plate-theory or measured-stiffness
option; this one doesn't.

## ADR-079: Exports as text built in the core, heights on each format's own datum (2026-09-26)

**Context.** M1.10c asks for CSV, JSON, KML and GeoJSON files, Parquet behind a feature, with
GeoJSON checked by schema and KML by parsing. The core crates do no I/O and build for wasm32
(CLAUDE.md rule 5), and a number the simulator prints is a claim, so an export must not round it.
Parquet needs the `arrow`/`parquet` stack, a large dependency whose choice and feature wiring is a
piece of work of its own.

**Decision.**

1. **Split.** M1.10c1 ships the four text formats; M1.10c2 adds Parquet behind a cargo feature.
2. **Text built in `hpr-sim`, no files.** `hpr_sim::export::{csv, json, track, geojson, kml}`
   return `String`s; a caller (the example, later the CLI) writes them. `hpr-sim` gains
   `serde_json` as a normal dependency, which already builds for wasm32.
3. **Exact numbers, or none.** Numbers are written in Rust's shortest round-trip form (`{:?}` in
   CSV and KML, `ryu` in JSON), so each reads back to the recorded `f64`; tests assert equality,
   not a tolerance. A value that isn't finite is refused with `SimError::Domain`, since CSV's
   `NaN` and JSON's `null` are read differently by different tools.
4. **Each format's own height datum.** GeoJSON positions are `[longitude, latitude, h]` with `h`
   above the WGS 84 ellipsoid (RFC 7946, section 4). KML's `absolute` altitude is above sea level
   (OGC 07-147r2), so KML writes `H = h − N` with the site's geoid undulation `N`, taken as
   constant over the flight. A `TrackPoint` carries both.
5. **Checked by the published schema and a strict parser.** The GeoJSON `FeatureCollection`
   schema from `geojson/schema` (MIT, commit 268ba0a) is bundled under
   `crates/hpr-sim/tests/data/`, and the `jsonschema` crate (MIT, tests only, no default
   features, so it fetches nothing) validates against it; a test shows it rejects a broken
   position. KML is parsed by `roxmltree` (already a dependency) and checked for the KML 2.2
   namespace, `altitudeMode` and every coordinate.
6. **Two observers on one flight.** `(A, B)` implements `Observer`, so the metrics and a recorder
   watch the same flight rather than flying it twice.

**Consequences.** A path crossing the antimeridian is not cut as RFC 7946 asks; it is documented.
Landings are ground-clamped points without a height. The recorder must keep the time and the CG
position for a map; without them `track` refuses. Nothing here changes a simulated number.

## ADR-080: Parquet written in-house, read back by Apache's library (2026-09-26)

**Context.** M1.10c2 asks for a recording as a Parquet file, behind a cargo feature, which an
independent reader reads back the same. `ARCHITECTURE.md` suggested the `arrow`/`parquet` crates.
The writer lives in `hpr-sim`, a pure crate that builds for wasm32 (CLAUDE.md rule 5). The
`parquet` crate without default features still adds some ten runtime crates to it (`ahash`,
`chrono`, `half`, `num-bigint`, `twox-hash` and others), and a file read back by the library that
wrote it doesn't show that someone else's reader agrees.

**Decision.**

1. **Written by hand, from the specification.** `hpr_sim::export::parquet` builds the bytes from
   the Apache Parquet format (`parquet-format` at commit `bf09939`: `README.md` and
   `parquet.thrift`) and the Thrift compact protocol for the footer. It writes the simplest file a
   reader must accept: one `REQUIRED` `DOUBLE` column per recorded column, one row group (none for
   an empty recording), version 1 data pages of at most 1024 values (the 8 KiB page the
   specification recommends), `PLAIN` encoding, no compression, statistics, dictionary or index.
2. **Behind a `parquet` feature that adds no dependency.** `hpr-sim` has the feature and the `hpr`
   facade forwards it. It keeps a binary writer out of builds that don't ask for one, and gives
   compression codecs a place to go later without touching the default build.
3. **Apache's library is the independent reader.** The `parquet` crate (Apache arrow-rs, 60.0.0,
   no default features) is a test-only dependency, renamed `parquet-reader` so that it doesn't
   clash with the feature's name. Tests read every file back through it and compare every value
   bit for bit: a flight's recording, one of every channel (30 columns, over 2048 rows, so at least
   three pages per column, with the chunks' offsets and sizes checked to tile the file), an empty
   recording and extreme values. The fields that reader ignores (uncompressed sizes, the row
   group's offset) are pinned by a two-row file compared byte for byte with bytes worked out by
   hand from the specification, and the compact-protocol encoder by the protocol's own examples.
   Once, when this was written, pyarrow 25.0.1 and DuckDB 1.5.5 read the example's file and a
   30-column, 3005-row file equal to their CSV. `cargo xtask wasm-check` builds the feature for
   wasm32.
4. **The text exports' rules.** A value that isn't finite is refused with `SimError::Domain`, and a
   recorder with no columns with `SimError::Unsupported`. `Recorder::new` now refuses a channel
   listed twice (`SimError::Unsupported`): two columns of one name make pyarrow and Polars refuse
   the file, and made the CSV header ambiguous.

**Consequences.** A file is 8 bytes per value, plus about 20 bytes per page and a footer; without
statistics a query tool can't skip pages by value. The `export_flight` example needs the feature
(`required-features`), so its command gains `--features parquet`. The `parquet` crate stays out of
every build but the tests. `ARCHITECTURE.md` now names this writer instead of the
`arrow`/`parquet` crates.

## ADR-081: ERA5 weather read from netCDF classic in `hpr-io`; M2.3 split a to c (2026-09-26)

**Context.** M2.3 compares hpr with real flights, flown in the weather of their day. RocketPy's
logged flights come with ERA5 reanalysis files: netCDF, some in the classic formats (`CDF\x01`,
`CDF\x02`), some netCDF-4, which is HDF5 underneath. The roadmap asks for a netCDF reader or a
documented conversion. `ARCHITECTURE.md` lists `hdf5` among crates to avoid (abandoned; C
library) and has no weather-file crate. The milestone is larger than a session: a reader, then
the flights, then the corpus flights with logs.

**Decision.**

1. **Split M2.3 into a to c.** M2.3a reads ERA5 files; M2.3b flies RocketPy's logged flights in
   their weather and meets the parent's bullets; M2.3c adds the private corpus designs that have
   logs, published as statistics.
2. **netCDF classic, by hand, from Unidata's specification**, in `hpr_io::netcdf`: both classic
   formats, every type, record variables (with the lone-record padding case), and the attribute
   conventions for packed and missing data. It depends on nothing new and builds for wasm32.
   netCDF-4 and CDF-5 are refused with a conversion: xarray's `to_netcdf(...,
   format="NETCDF3_64BIT")` after dropping `number` and `expver`, the Data Store's 64-bit integer
   and string variables that the classic formats can't hold (`nccopy` can't drop them for you).
   Reading HDF5 would mean a large format or a C library, for files a three-line conversion turns
   into ones this reader reads. A header whose variables claim more bytes than the file holds is
   refused before anything is allocated. Each dimension's name is allocated once and shared by
   every axis that names it, and duplicate names are found through ordered sets, so a hostile
   file costs memory a small multiple of its size and time `n log n` in its names (fuzzed by
   proptest).
3. **The Users Guide's conventions over netCDF4-python's.** With no valid bounds, the fill value
   bounds the valid range on its own side (one step away for integers, two units in the last
   place for floats), and a byte with no explicit fill has every value valid (netCDF Users Guide,
   "Attribute Conventions"). netCDF4-python 1.7.4 masks only values equal to a fill, the byte
   default included. The tests compare against that library everywhere else and pin each cell
   where the two differ, in both classic formats. It matters: RocketPy's multi-year netCDF-4 ERA5
   files for EuroC (2001–2021) and Spaceport America (2002–2021) hold −32768 under a −32767 fill
   (102 and 298 geopotentials, 4 and 2 temperatures), which hpr reads as missing and
   netCDF4-python as, for example, 198.66 K among neighbours near 301 K.
4. **ERA5 in `hpr_io::era5`, as a `SoundingProfile`.** `hpr-io` gains `hpr-atmos` (both pure).
   Values are bilinear in latitude and longitude, as RocketPy 1.13 takes them (its MIT
   `bilinear_interpolation`), and linear in time between the two hours around the launch, where
   RocketPy takes the nearest hour. Geopotential height `Z = z/g₀` becomes geometric height by
   WMO-No. 8 at the site's latitude, the relation `SoundingProfile` inverts, so each level's
   geopotential round-trips. ECMWF's Knowledge Base suggests `R·Z/(R − Z)` instead, "neglecting
   horizontal variations" of gravity, and RocketPy does that. Both are approximations. Taking
   the model's surface geopotential as `g₀ h_s` (an assumption; ECMWF doesn't say), WMO's reading
   is off by about `h_s(g₀/γ_s − 1) + h_s²/R` at every height and ECMWF's by
   `h_s²/R − (h − h_s)(g₀/γ_s − 1)`, growing with height above the model's ground. Neither is
   always the smaller: for a ground 1400 m up at 33° N, WMO's is 1.88 m and ECMWF's is smaller up
   to 1.95 km above the ground, −3.07 m at 3 km (with RocketPy's Earth radius). hpr keeps WMO's
   because it is the rule `SoundingProfile` uses for every sounding, so a level's geopotential
   round-trips; a test pins these numbers. The readings differ by `g₀/γ_s(φ) − 1` of the height:
   −0.0158% at 47.21° N and +0.0343% at 41.78° N, pinned by a test. Humidity is not read yet (dry
   air). Beyond the levels the profile is `SoundingProfile`'s: hydrostatic between levels and the
   offset standard atmosphere above them, where RocketPy holds the end level's values.
5. **Fixtures.** `validation/oracles/netcdf/write_cases.py` writes the reader's test files with
   the Unidata C library (through netCDF4-python) and records its reading. `era5.py` cuts small
   extracts of RocketPy's Bella Lui and NDRT 2020 files (the second one from the current Data
   Store, converted by the guide's recipe) and records RocketPy's reading of the full files. The
   extracts are committed under ERA5's CC BY 4.0 licence with Copernicus's attribution and
   disclaimer (`THIRD-PARTY-NOTICES.md`), and every fixture records its generator, tool versions,
   date, command and input hashes. netCDF4 and xarray join the oracle environment.

**Consequences.** `Era5Profile::read` agrees with RocketPy's levels to 1e-12 on the hour at both
sites, and two downloads of the same analysis four years apart agree to 0.23 m²/s², 0.35 mK and
0.11 mm/s, consistent with each file's rounding (`hpr_io::era5` tests). A user with a current
Data Store file runs three lines of Python first; a native
netCDF-4 reader stays open for later. Humidity and single-level (surface) files are not read yet.

## ADR-082: Real flights read from refs, compared over the ascent, with checked explanations (2026-09-26)

**Context.** M2.3b compares hpr with real flights: at least six in the report, each with its
apogee error and an altitude-trace RMS, the mean absolute apogee error against the 5% target of
`docs/VALIDATION.md`, and an explanation for each outlier. RocketPy 1.13.0's documentation flies
ten rockets against their teams' logs, most in an ERA5 file of the day. Its logs were shared with
RocketPy by the teams (each notebook records the permission), its motor files come from
ThrustCurve.org with each file's own terms, and only some of those are public domain. The public
designs of M1.4 fly a substitute bundled curve (ADR-007), which says nothing about a real flight.

**Decision.**

1. **The inputs stay in `refs/`.** `cargo xtask real-flights` reads each log, the example's own
   thrust file and the ERA5 file from the pinned RocketPy checkout, and commits only
   `validation/reports/real-flights.{json,md}`: each flight's apogees, errors, RMS and counts,
   and every file's SHA-256. Committing the logs would redistribute data shared with RocketPy,
   not with us (rule 4). So the flights run where the checkout is, as the corpus flights do
   (ADR-071); `--check` flies them again. CI, without `refs/`, holds the committed report to
   itself: its summary to its rows, its page to its data, and each explanation to its numbers
   (`hpr_validate::tests::real_flight_cases_report_apogee_and_trace_rms`).
2. **Seven flights:** the five with a public design already (Bella Lui, NDRT 2020, Prometheus,
   Juno III, Cavour), and Genesis and Lince, added to `rocket_mass.py` for this: COTS motors, in
   EuRoC 2023's classic file (their motors have no dry mass, like Cavour's). Left for later:
   Astra and Andromeda (COTS, but EuRoC 2022's file is netCDF-4 and needs ADR-081's conversion);
   Camões, Erebus 11, Halcyon and Hedy (their teams' own motors, outside rule 6's COTS scope);
   Valetudo and Defiance (an apogee, no trace); Valkyrie (inputs only in a data file, ADR-007).
   Juno III also flies its team's motor, but its design was already public: hpr flies the thrust
   file like any other, and nothing of a research motor is modelled.
3. **As a user would fly it:** the design (the example's masses, inertias and geometry), hpr's own
   aerodynamics, the example's own thrust file read as RocketPy reads it (a `(0, 0)` point first
   for `.eng`, clipped at the example's burn time, reshaped where the example reshapes; the
   total impulse matches RocketPy's reading of all seven to 1e-15, but Juno III's),
   the example's rail, site and launch hour (local hours converted to UTC), hpr's WGS84 Earth,
   and the ERA5 profile of ADR-081. Juno III's thrust file ends in five negative points, which
   hpr refuses; they are read as zero, adding 2.62 N s to its 8800 (reported), as RocketPy's
   flight holds its thrust at zero too. Prometheus flies the weather of 24 June 2023 for a 2022
   flight, as RocketPy's example does; RocketPy has no file of the day, and the row says so.
4. **hpr is read as the log's altimeter reads.** A barometric altimeter converts the pressure it
   measures to the standard atmosphere's altitude and subtracts the pad's: on a day warmer than
   the standard the air column is thicker and it reads less than the height climbed, by several
   per cent at Spaceport America in June. Prometheus's TeleMetrum and Juno III's RRC3 log their
   pressure, and their height columns are that reading less the first row's, to 0.195 m and
   1.321 m over the rows compared (the report computes the gap). So hpr's
   height goes through the same conversion, of the ERA5 pressure at its centre of mass
   (`Ussa76::pressure_altitude_m`, `hpr_validate::real_flight::Barometer`). NDRT's Featherweight
   Raven is a barometric altimeter by make. Cavour's CATS Vega, and Genesis's and Lince's filtered
   estimates, fuse a barometer with an accelerometer, and Bella Lui's avionics are unnamed: those
   four are marked *assumed* barometric, with the evidence in each row. The report gives hpr's
   apogee as a height too, and the mean both ways. Two logs also carry satellite (GNSS) heights,
   which the report reads: Juno III's rises 3369.3 m above its pad and Prometheus's 4133.0 m, so
   their barometric apogees are 0.935 and 0.943 of those, where hpr's conversion makes its own
   0.921 and 0.932 of its height. The conversion has the right direction and nearly the right
   size; it reads 1 to 2 points lower on both, on the side of both flights' misses. Juno III's
   log is cut while still climbing (item 5). Lining the two logs up by vertical speed (17.5 m/s,
   the log's at its cut), the satellite height there is about 3350 m, which would make its ratio
   0.941.
5. **Apogee** is the log's highest reading, read up to the recovery. Three logs have pressure
   transients near apogee: Juno III's reading, as it levels off, dips 94 m, then rises 62 m above
   the level within 0.3 s (the flight card gives that spike, 3213 m); Prometheus's drops 600 m and
   returns 8 m above its highest reading before; Lince's filtered height swings by hundreds of
   metres up to 3668.5 m, where its highest before, 3587.7 m, is its flight card's. No one
   threshold separates these from the ascent's own dips (Lince's drops over 50 m at 8.5 s), so
   each log is read to a time set by hand, before its transient, with the reason in its note
   (24.60 s, 29.58 s, 26.80 s; Juno III's cut also drops its two corrupt rows). At Juno III's cut
   its own velocity column still reads 17.5 m/s up and its satellite height climbs another 19 m,
   so its apogee may be 10 m to 20 m low: its miss would be about half a point larger, and on the
   recorded thrust about half a point smaller.
6. **The trace RMS is over the ascent**: each clock is aligned where its trace first reaches 30 m,
   since a log's zero is its own (armed, launch detected, power on), not ignition; the RMS runs
   over every log row from there to the first of the two apogees, hpr's heights interpolated on
   a 0.01 s grid of its dense output. The descent is left out: its events are the team's. The
   early climb of a barometric log is not a measure of the boost: Prometheus's barometer reaches
   150 m 0.87 s after its own speed column, integrated, puts it there, as the pressure in its bay
   lags or under-reads at speed, so no explanation rests on it.
7. **Diagnostic flights.** Each flight is flown again on the example's own drag (a constant, the
   notebook's knots, or its CSV files, scaled as the notebook specifies, on the example's radius),
   and, where the example reshapes its thrust file to a burn time and an impulse, on the file as
   recorded. Neither is a second prediction: a team's drag is an estimate, from a table, RASAero
   II, CFD or a constant the notebook doesn't source, and may have been tuned to its flight.
   RocketPy 1.13.0 itself doesn't fly two of the drags as written: a `power_off_drag` assigned
   after the rocket is built, or scaled in place, never reaches the function its flight evaluates,
   so it flies Bella Lui on 0.43 and Juno III on the unscaled curve. hpr flies what the notebooks
   specify.
8. **An explanation is a checked claim.** An outlier (outside the 5% target) must carry one, and a
   flight inside it must not. `drag` claims the flight on the team's drag is within the target,
   and the one on the recorded thrust file, where there is one, is not: the miss is consistent
   with hpr's drag. `thrust` claims the flight on the recorded thrust file
   is within the target and the one on the team's drag is not: the miss is consistent with the
   impulse the notebook sets, and not with drag. CI checks
   each claim against the row, each row's percentages against its metres, and the words, the
   digests of the committed files read and of each flight's inputs against the code's, so a
   change that makes a claim false or the report stale fails.

**Consequences.** Over seven flights, the mean absolute apogee error is 6.04%, outside the 5% target
(−8.90% to +10.40%; mean −0.25%), and the trace RMS is at most 7.26% of an apogee. The target is
missed however the assumed altimeters are read: 6.63% with only the three known barometric logs read
so (Lince, at +7.99%, would then miss with no checked explanation), and 4.47% only were every log a
height, which three of them are known not to be. Reading hpr as a barometer moves its apogee by
−7.9% (Juno III, in June's heat) to +1.7% (NDRT 2020, in February). Five flights are outliers. Four
are consistent with hpr's drag: on their teams' drag NDRT 2020 (+10.40%) lands at +0.17%, Prometheus
(−8.90%) at +0.45%, Cavour (+5.63%) at −2.29% and Genesis (−5.85%) at −0.08%; hpr's drag is low for
two and high for two. Prometheus's reading also rests on weather a year off its day; its share can't
be told apart, since Juno III, on its own day, shows as large a gap against its satellite height.
Juno III (−7.24%) is consistent with its motor's impulse: the notebook reshapes its team's curve to
8800 N s, 4.9% below the file (9249.0 N s; 9251.7 N s with its negative end read as zero), and on
the file as recorded hpr lands at +1.13%, where on the team's drag it lands at −9.05%. Lince, inside
the target, is −12.20% on its team's drag, not investigated. These are consistent explanations, not
proofs: the teams' drags and the reshape are estimates too. A log is a single flight, with its own
sensor and filter; the numbers are those seven flights', not a bound. The report is not reproduced
in CI, only held to itself there.

## ADR-083: M2.3c blocked: no private design is the rocket of a logged flight (2026-09-26)

**Context.** M2.3c asks for the private designs that have a flight log, flown in their day's
weather and published as anonymised statistics beside M2.3b's report. The private data is in two
collections, both read only from `refs/` (rule 4): the design library (`loft-fixtures`, the corpus
of ADR-071 and ADR-072) and the flight logs (`debrief-fixtures`). Neither manifest links one to the
other: the logs have a flight group and no design column, and the library's groups only tie together
copies of one design saved in different tools. So on 2026-09-26 each log was matched against each design by
hand, on the rocket's name, its motor, the flight's date and the source each file came from, in
both manifests and in the files themselves.

**Findings.**

1. **No private design is the rocket of a logged flight.** No log in `debrief-fixtures` matches a
   design in the library. The only designs there whose rockets have logs at all are copies of
   RocketPy's examples, which are public; their logs are RocketPy's (`refs/rocketpy`), and they
   were already flown in M2.3b (ADR-082 item 2) or left out there for the reason given (an apogee
   and no climb).
2. **One design shares a source with some logs, and nothing more.** Its team published both, but
   the design was saved before any of those flights, names none of the logged rockets, and holds
   several motor configurations, and none of those logs names its motor. Choosing a pairing
   and a configuration would be a guess, and an apogee error against a guessed pair measures
   nothing. It is not flown.
3. **One design has a flown apogee stated by its source, but no log,** and no date or site, and
   hpr can't fly it yet ([#174](https://github.com/nrdptel/hpr-sim/issues/174)).
4. **No design carries a recorded flight.** Every data point in the library's files belongs to a
   saved simulation. One file among the logs is a simulation too, of a public example, not a flight.
5. **Weather.** No ERA5 file under `refs/` covers the date and site of the flight in item 2. The
   Copernicus Climate Data Store, ERA5's source, needs an account to download one.

**Decision.**

1. **M2.3c is `[blocked]` on data only Neer can add** ("Needs Neer" in `STATUS.md`): a pair, that
   is a design file of a logged rocket as flown, or a log of a library design's flight, with its
   date, site and motor, in the private collections. A day not covered by a cached file also needs
   an ERA5 file, and that needs a Data Store account. Its *done when* is unchanged.
2. **Nothing is flown against a guessed pair.** The survey's result is recorded here, on the
   roadmap and on the accuracy page, without naming a private design.
3. **M2.3's own bullets are met by M2.3b's seven flights.** There are at least six; the mean
   absolute apogee error, 6.04%, is reported against the 5% target and is outside it; each outlier
   has an explanation the report checks. The milestone stays open only for M2.3c.
4. **Work moves on to M2.4,** the accuracy census gate, which does not depend on M2.3c.

**Consequences.** The real-flight evidence stays at M2.3b's seven public flights; no private design
adds to it. When a pair is added, M2.3c reuses M2.3b's reading of a log and of a barometer
(`hpr_validate::real_flight`) and M2.2e3's anonymised ids (`cargo xtask ork-flights --library`).
It also needs a way to fly a `.ork` design from the library, where M2.3b flies committed designs.

## ADR-084: The accuracy census: the reports' numbers held to the ones accepted (2026-09-26)

**Context.** M2.4 asks for a census of the validation results, in the README with a badge, and for
CI to fail on any per-case regression beyond tolerance; *done when* a perturbed drag coefficient on a
throwaway draft PR turns CI red. CI already failed when the committed report was stale
(`validate --check`, ADR-022), and a same-drag miss or a flipped predicted-mode verdict failed the
run or a pinned test. What nothing caught was a report regenerated with a worse number that kept
its verdict: a predicted-mode apogee going from 1% to 2.9% of RocketPy's, inside its 3% target
(ADR-023), or an OpenRocket or real-flight difference, which have no gate at all. Loft's lessons
L84 (hand-written counts), L85 (a "now passes" check at half the tolerance), L86 (a headline with
no oracle, population or regime) and L88 (its own aerodynamics never gated) apply.

**Decision.**

1. **One row per number a report holds hpr to** (`hpr_validate::census`), from the four committed
   reports: every comparison and known gap of the harness's (`latest.json`); each real flight's
   apogee error and climb RMS; and each OpenRocket flight's apogee, largest speed, margin, mass at
   launch and at rod clearance and centre of mass there, with each configuration the report lists
   as not flown (L86's population: 24 of the examples' and 19 of the private designs'). Left out:
   figures a report gives to explain a difference, not to measure one (a real flight on its team's
   drag, its satellite heights). A row is keyed by its report's group, its case and its metric; a
   key seen twice is refused, and every count is the census's own (L84). Six groups, each naming its
   reference and version, its kind (code-to-code on the same inputs, code-to-code on each code's own
   model, or measured), and what its report holds it to (a gate, a target, or neither).
2. **Speed classes** by the largest Mach number: subsonic below 0.8, transonic to 1.2, supersonic
   above, the OpenRocket library report's classes, and "unknown" where a report has none. The
   harness's flights by RocketPy's number (its `max_mach` reference), OpenRocket's by
   OpenRocket's, and the logged flights by hpr's, which the real-flight report now records
   (`hpr_max_mach`), since a log has none. Each headline names reference, kind, population, the
   not-flown count and the classes (L86).
3. **A ratchet, both ways.** The census accepted last is committed (`census.json`). It is a change
   when a row's difference moves by more than its accepted slack, when its standing changes (a miss
   that starts passing too, at any margin: L85), when its unit, scale, slack or class changes, when
   it comes or goes (a gap or not-flown case that is flown now is named so), when a group's
   reference changes, or when the slack rule does. An improvement is a change too, or it could slip
   back unseen. The slack is 0.1% of the row's scale, and for a harness row never less than the
   harness's reproduction bound (2e-6 or 1e-7 of the value). The scale is the row's tolerance, or
   where it has none its kind's bar: 3% of the reference for a harness metric not scored (M2.1's
   bound on each metric, which `no_committed_gate_is_looser_than_the_milestone_says` holds every
   gate and target to); 5% for OpenRocket's apogee (ADR-070's line for a written cause) and largest
   speed (M1.9c's bar on both); 1% for its masses (M2.2a's); 0.5 calibres for a margin or centre of
   mass (M1.8a's centre-of-pressure target; either moves the margin as much); 5% for a logged apogee
   (the real-flight target, which is on the mean, used per flight as a bar); 3% for a logged climb
   (ADR-024's bound on a series RMS). The committed reports are regenerated on one machine and
   compared as files, so the noise the slack must clear is a regeneration's, not a platform's.
4. **Checked in `validate --check`,** whether or not the run reproduces the report, so CI's
   `validate` job holds it on three platforms and the gate keeps its nine steps. A change passes
   only once accepted: `cargo xtask census --accept --reason "<why>"` refuses a changed census
   without a reason, and an unchanged one with one, and writes the reason and the list of changes
   into `census.json` and its page. A regression can merge, in writing, in the diff that brings it;
   not by regenerating a report.
5. **Its outputs are written from the accepted rows, its summaries computed again each time** and
   checked as text: `census.md`, the table between markers in `README.md` and `docs/accuracy.md`
   (the same table, linking the census page at its GitHub address), and two static badges in
   `docs/images/` (no network): the gated code-to-code metrics that pass ("vs RocketPy, same
   inputs") and, beside it, the real flights' mean apogee error against its target, so the one
   measurement is as visible as the agreement with another program.

**Consequences.** The first census holds 648 rows. Predicted mode's rows, which failed a run before
only when a verdict flipped, and the OpenRocket and real-flight differences, which no run could
fail, now fail CI when they move. On the
throwaway PR, 2% more subsonic skin friction with the report regenerated failed the check on 74
rows, 17 for the worse. Two limits stay. The census holds each number to where it was, not to the
truth, and adds no evidence of its own. The OpenRocket and real-flight reports need files CI lacks,
so CI holds their committed numbers, not a fresh run: a code change that would move them is caught
only when someone runs them again and commits the result. Every physics change that moves any
number now needs `census --accept` with a reason, one more command than before, which is the point.

## ADR-085: Ejected pieces: an airframe that parts at any joint (2026-09-26)

**Context.** M1.11 asks for a separation at any joint, not only at a stage boundary: an ejected
nose cone, a body section or a payload carried inside, each flown to its own landing under its
own recovery device or tumbling, with triggers as a device's and an optional ejection impulse.
*Done when:* a design that ejects its nose cone and a payload, each under its own parachute, lands
every piece and reports each landing point; the masses sum to the rocket's and momentum is
conserved at each split to M1.7c's tolerances (1e-12 and 1e-9); each descent rate matches the
analytic terminal velocity. What is in hand is ADR-014's separation: one split at a stage boundary
into two bodies, each a point mass under its own devices, with body 0 the nose's, and ADR-074's
powered case, where body 0 flies on as a sustainer on a design cut at the boundary. `hpr-design`
has no notion of a joint below a stage, but its layout places every component with its own mass
(`own`) and its subtree's (`with_children`), and a parent before its children.

**Decision.**

1. **Split the milestone.** M1.11a is the pieces: partings at any joint and around a payload,
   several of them, every body landed; its *done when* is the milestone's. M1.11b is the ejection
   impulse and a tumbling model for a piece that is not a whole stage.
2. **An `Ejection` beside the `Separation`, not a new separation.** `Ejection { trigger, parting }`
   with `Parting::AftOf { component }` (the joint just aft of a body component) or
   `Parting::Payload { component }` (an internal component and its subtree), given by id. The
   separation keeps its type, its API and ADR-074's powered path unchanged. A powered separation
   in a flight with ejections is refused in flight: the sustainer flies on a cut design whose
   components are not the pieces the ejections name.
3. **The pieces are fixed before the flight.** Every joint that can part (the separation's
   boundary and each `AftOf`) cuts the body components, nose to tail, into sections; each payload
   is a piece of its own. Piece 0 is the nose's, the separation's is 1, and ejection `k`'s is the
   next number after those, in the order given. At any moment a body is the pieces still joined,
   numbered by its lead piece: the section nearest the nose among them, or a payload alone. So
   numbering does not depend on the order the triggers fire in, body 0 is always the nose's, and a
   separation-only flight numbers its bodies as before.
4. **A piece's mass is its components'.** A stage wholly in one piece counts by its placed mass,
   with its overrides, which keeps a separation-only flight bit-identical (every earlier test and
   example output is unchanged). A stage that parts counts each subtree that stays in one piece by
   `with_children`, and a component whose subtree parts by `own` plus its children's. An override
   that doesn't say how its mass divides is refused rather than spread: a stage's, or a
   component's that covers what it holds (`overrides_include_children`). So is a payload that is
   one copy of several in a cluster of tubes, an external part, or one inside another payload (the
   numbering handles it, but nothing tests it yet). A motor belongs to its mount's piece.
5. **Each parting adds no impulse.** At the first parting, from the rigid stack, every body starts
   at its own centre of mass with that point's velocity, `v_O + ω × r_cg`, as in ADR-014. A later
   parting happens to a body already flying as a point mass, which has no attitude to place its
   pieces by. Both pieces start at its point and velocity, and its mass steps down; the error in
   position is at most the rocket's length. In both cases the momenta add up exactly, less
   rounding; on the way down that is by construction. Partings that fire in one pass are taken
   one at a time, asking after each which are still the body's: a parting can move another's
   piece to the body that leaves, which then parts it at its start. Review found the first draft
   asking once, which counted a piece twice.
6. **Every motor must have burned out** when an ejection fires, as ADR-014 requires of an aft
   body's: a trigger known before the flight to come earlier is refused when it is given, and one
   that fires early is an error in flight. An ejection before a separation that would light a
   motor is refused too, since the pieces would never light it. So a separation that fires on the
   way down never lights one.
7. **Devices act when their body flies.** As in ADR-014, only body 0's devices act before the
   airframe first parts. After it, a body's devices act on it, and a piece still joined to its
   body waits: its devices' drag area was computed for that piece, not for the body.
8. **Checking devices against bodies.** A builder checks that every body known so far has a
   device. That no device names a body that nothing makes is checked when the flight starts,
   because a separation or ejection given later can still make it, and an ejection's number
   counts the separation. `separations_outside_their_domain_are_refused` now asserts that refusal
   at `run`, with its text and value; the refusal itself is unchanged.
9. **Shock cords are not modelled as such.** Pieces tied together fly as one, which is what not
   declaring an ejection gives.

**Consequences.**

- The milestone's design (`pieces::tests`, and the example `ejected_pieces`) flies the 54 mm test
  rocket with a 250 g payload: nose cone at apogee, payload at 300 m. All three bodies land, each
  within 0.1% of its own `v_e` in uniform air. Masses add to 1e-12 at both partings, and momenta
  to 1e-9, with wind and a 0.6 rad/s body rate at the first.
- `BodyFlight` gains `pieces`; its `mass_kg` is the mass at landing, and a body's events include
  the partings on its way down (`EventKind::Ejection`, or `Separation` if the separation comes
  after an ejection). `SimError::Parting` names the component a refused parting runs through.
- Left for M1.11b: the ejection impulse (a charge or spring's push, equal and opposite on the
  two pieces), and a tumbling model over a piece's components (`tumbling_stages` covers whole
  stages only). Left for later: a sustainer's pieces; the attitude of a body after it parts; the
  `.ork` importer's reading of OpenRocket's own component-level recovery.

## ADR-086: Ejection impulse and tumbling pieces (2026-09-26)

**Context.** M1.11b, the rest of M1.11 after ADR-085: an optional impulse, equal and opposite on
the two pieces, and a tumble model over a piece's own components. *Done when:* an impulse gives
each piece the hand-computed change of velocity and conserves momentum to 1e-9, and a tumbling
nose cone lands at its tumble model's terminal speed. Two things were open. A body that parts on
the way down is a point mass with no attitude, so an impulse there has no axis to act along. And
the tumble model (ADR-013, the OpenRocket technical documentation's §3.5) took each body
component's side area from its end diameters, 25% low on Valetudo's tangent ogive nose: for a
lone nose cone that is the whole area.

**Decision.**

1. **An impulse `J` per ejection, N·s,** `Ejection::with_impulse`, default zero and written only
   when set. It is refused unless finite and zero or more. A separation keeps none: its type and
   ADR-014's behaviour are unchanged.
2. **Direction.** The side forward of the joint takes `+J` toward the nose and the side aft `−J`;
   a payload leaves forward, out of its host, as one does when the nose cone comes off first.
   Each body's velocity changes by its push over its own mass, so the momentum is unchanged by
   construction. A payload that leaves aft is not given a way to say so yet, so a push on a
   payload in the nose's own piece, which is closed at the nose, is refused when the flight
   starts (a separation given later can put it in a piece of its own; review found the first
   draft refusing it by builder order).
3. **The axis while the airframe flies whole with nothing open** is its attitude at that instant,
   and the push is added to each body's `v_O + ω × r_cg`. A device that opens in the same pass
   as the parting freezes that same attitude, so it counts. When several splits part the stack
   in one pass, each pushes the two bodies on its own two sides.
4. **Otherwise the body has no attitude to go by**: a point mass after a parting, or a whole
   stack whose attitude froze when a device opened at an earlier time (review found the first
   draft pushing such a stack along its apogee attitude, nearly sideways, after minutes under a
   drogue). hpr then goes by the velocity through the air `v − w`:
   - a body hanging from a device, any but a tumble, open just before that instant (deployed
     before it and not released before it) points its forward end against `v − w`, toward the
     device, assumed to have left through that end, as a main does once the nose cone is off
     (review found the first draft pushing a payload down, away from the canopy it leaves toward).
     What happens at the parting's own instant, a deployment or a release, doesn't count yet, so
     the answer doesn't depend on the inflation law (review found the push flipping with it, at
     a deployment and then at a release);
   - a body with nothing open, or only a tumble, points its nose along `v − w`, as a statically
     stable airframe does;
   - below 1 mm/s through the air (a body's own apogee in still air), up. That speed is drift or
     round-off, not a flight path.

   Nothing in hand says how a body hangs under a drogue, so these are stated assumptions. The
   push itself moves little where every device opens as its piece leaves: in the example, 1 N·s
   at apogee and at 300 m moves the airframe's and the payload's landings by 0.0 m and 1.3 m.
5. **Partings at one instant part a body together,** on the way down as at the first parting:
   which fire and the direction are decided on the body as the pass starts, every firing split
   opens, and each final body takes the pushes of the joints on its sides. Review found the first
   draft taking them one at a time, so that the list order moved the nose cone's push from 15.85
   to 17.66 m/s, and a push could delay a split that fired with it. The events are the parting
   body's, each with the body before and after the whole instant. A pushed payload whose
   section's forward joint hasn't parted by then is an error in flight.
6. **The body after a parting is recorded.** `BodyEvent::after` holds the body just after a piece
   leaves it on the way down (mass without the piece, velocity after the push), so the remaining
   body's change of velocity can be checked. It is `None` for every other event, and not written.
7. **A piece tumbles over its own components.** `Simulation::tumbling_piece(k)` applies the tumble
   model to the body components and fin sets of piece `k`, which leads body `k`. A payload is
   refused, since it has no body tube or fin of its own. So is a piece the airframe doesn't part
   into. Cut into sections, the pieces' drag areas add to the whole airframe's.
8. **The side profile is integrated.** `A_bt` is now `∫ d dx` over each nose cone's and
   transition's own profile, by `hpr_design::revolve`'s planform area. Tubes and cones are
   unchanged. It is a fix to merged physics, not a new model: the documentation defines `A_bt`
   as the side profile area, and the end-diameter reading was a stated approximation. Valetudo
   tumbling moves from 36.77 to 36.38 m/s, and the `.ork` two-stage example's tumbling
   sustainer from 10.6 to 10.4 m/s. The drop-test replay and the two-stage booster are
   unchanged, since they have no curved part.
9. **A lone nose cone is outside the model's fit.** The constants were fitted to whole model
   rockets. The milestone asks that the nose cone land at *its model's* terminal speed, which is
   what the test shows. That the model is right for a nose cone on its own is not claimed.

**Consequences.**

- `pieces::tests` pins the impulse at both kinds of parting. With 1 N·s, in wind, from a stack
  tilted 60° up toward 30° east of north, moving sideways and turning at 0.6 rad/s, each body's
  change of velocity is `J/m` to 1e-9: along the hand-computed rail axis at the first parting,
  15.9 m/s on the 0.063 kg nose cone, and 4 m/s on the 250 g payload at the second, against the
  airframe's velocity through the air. The momenta add up to 1e-9 at both partings. Each
  direction rule has its own test, and so do a separation with a pushed ejection and two pushed
  partings in one pass.
- The tumbling 0.25 m tangent-ogive nose cone of the 54 mm test design has 0.56 times its
  closed-form side area, to 1e-12. It lands at 13.849 m/s, its model's `v_e` to 1e-6.
- Left for later: a payload that leaves aft; the attitude of a body after it parts; an impulse
  on a separation.

## ADR-087: Mass that moves along the airframe (2026-09-26)

**Context.** M1.12 (VISION V18) asks for payload mass that moves along the airframe or leaves it,
on an event or a schedule, with the mass, centre of mass and inertia updated through the flight,
and for the equations of motion to carry the moving mass's relative-motion terms or an ADR to show
them negligible. That is two pieces of work: a mass that moves inside a rocket that stays whole,
and a mass that leaves while the rest flies on in six degrees of freedom. Today a parting turns
every body into a point mass (ADR-085), so the second needs the vehicle swapped as a powered
separation does. The equations (ADR-011) already carry a moving centre of mass (`r′`, `r″`) and a
changing inertia (`I_O′`), fed by central differences of the motors' mass properties while a motor
burns.

**Decision.**

1. **Split M1.12 into M1.12a (a mass that moves) and M1.12b (a mass released).** M1.12a's *done
   when* is the milestone's first bullet for a move, its second bullet, and a test of the
   relative-motion terms. M1.12b keeps the release half of the first bullet.
2. **A `MassShift` moves one internal part** (by component id, with everything inside it) a set
   travel along the axis, positive aft, over a set time, on the triggers a recovery device has.
   One with a trigger known before the flight starts then. The flight watches for the apogee and
   for a height on the way down, as it does for a device. Once a shift's start is known, its
   start, its end and 16 equal intervals between are stop times, so even a fixed step takes 16
   steps across it: review measured RK4 at 10 ms taking a 10 ms move in one step off by 0.071 m/s
   in the centre's velocity, and 1.2e-4 m/s with the stops.
3. **The motion is a cycloid**, `s(τ) = τ − sin(2πτ)/2π` of the travel over `τ = (t − t₀)/T`, the
   cam designer's cycloidal motion (R. L. Norton, *Design of Machinery*, the chapter on cam
   design): at rest at both ends with zero acceleration there, so neither the part's speed nor its
   acceleration jumps. A move's profile is not given by any source for rockets; this is a choice
   that keeps the equations smooth, and it is stated in the docs.
4. **The mass properties are exact.** The part's point contribution about the new centre is taken
   out where it was and put back where it is (`MassProperties::with_part_moved`, the
   parallel-axis theorem); its own inertia moves with it unchanged.
5. **The shift's rates are in closed form**, not differenced: `r′`, `r″` and `I_O′` gain the part's
   terms, including their coupling to a burning motor's `M′` and `M″`, which are still
   differenced. A first draft differenced the whole, 0.1 ms apart as for a motor; in free flight
   that held the centre's velocity only to 1e-8 m/s, the stencil's `(2πh/T)²` error, so the
   closed form replaced it.
6. **The relative-motion terms are carried, not shown negligible.** Summing over the particles
   about the nose tip gives the equations of ADR-011 exactly, plus `ω × h + h′` in the rotational
   equation, where `h = Σ m ρ × ρ′` is the moving parts' angular momentum relative to the
   airframe. It is zero for a part on the axis, and not for one off it. `T21` subtracts it.
7. **What a shift refuses.** A body component or an external one; one copy of a cluster's; a part
   that holds a motor (the motor would stay put); a part inside another that moves; a part in a
   stage, or inside a component, whose overridden mass covers it; a travel that can take the part
   out of the component that holds it, forward moves added together and aft moves added
   together, or past where the design already puts it (review found a weight in a nose cone's
   shoulder refused at any travel); a move shorter than 10 ms, nearer an impact than a motion
   (63 km/s² per metre of travel at that bound); a shift in a flight with a separation or
   ejections, whose pieces are fixed with every part where the design puts it (M1.12b's to lift);
   and, in flight, a shift that starts on the pad or the rail. The rail has no stop at its foot,
   and review found an aft shift at launch lifting the rocket at 4 µs, where it could stall and
   hang above the pad.
8. **`Simulation::mass_properties(flight, t)`** gives the stack's mass properties as a flight flew
   them: a shift with a trigger known before the flight started then, and one the flight watched
   for started where the flight's `EventKind::Shift` event says.

**Consequences.**

- `shifts::tests` holds the milestone's bullets. The 54 mm test design with 200 g of ballast moved
  0.3 m aft over 1 s from 5 s: its mass, centre and inertia before, halfway and after match the
  two-body hand calculation (reduced mass times the separation's `|L|² E − L Lᵀ`) to 1e-15. Its
  static margin at every coast sample is the hand value, `−m Δ s(τ)/(M d)`, to 1e-12 calibres:
  4.30 before, 3.00 after.
- In free flight, with no air and no gravity, the ballast 1 cm off the axis moving while the
  rocket turns about all three axes keeps the angular momentum about the centre to 6.9e-12 of
  itself and the centre's velocity to 2.6e-12 m/s. The part's relative angular momentum peaks at
  1.8% of the whole, so without `ω × h + h′` the error would be of that order.
- The closed-form rates match differences of the mass properties during the burn and after it:
  `r′` to 2e-11 m/s, `r″` to 3e-8 of itself (the difference's own error), `I_O′` to 1e-9.
- In a vacuum, a drogue that opens halfway through a move keeps the centre's velocity to
  1e-12 m/s, and the centre then falls freely to 1e-9 m/s while the ballast finishes. Every
  refusal has a test that checks which rule fired.
- A flight with no shift runs the same arithmetic as before: the validation report, the corpus
  flights and the real flights reproduce.
- Left for later: M1.12b's release; a shift with a separation or ejections; a mass that moves
  across the axis or turns.

## ADR-088: Mass released in flight (2026-09-26)

**Context.** M1.12b (ADR-087's split of VISION V18) asks for ballast or a payload that leaves the
rocket while the rest flies on in six degrees of freedom, with the mass properties after a release
matching hand-computed values and the release conserving mass and momentum. A separation or an
ejection already parts the airframe, but once it does every body is a point mass (ADR-085); only
a powered separation flies a sustainer on in six degrees of freedom, by swapping the vehicle
under a state that carries across, the nose tip's.

**Decision.**

1. **A `MassRelease` lets one internal part go** (by component id, with everything inside it) on
   the triggers a recovery device has, as a shift is started (ADR-087): one with a trigger known
   before the flight comes then, and the flight watches for the apogee and for a height on the
   way down. The parts a release refuses are a shift's: a body component or an external one, one
   copy of a cluster's, a part that holds a motor, a part in a stage, or inside a component, whose
   overridden mass covers it; and a part released twice or inside another that is released, a
   part with no mass, and releases that would leave the airframe none, its motors aside. It is
   refused on the pad or the rail, where the part has nowhere to go.
2. **The rest flies on by a vehicle swap**, as at a powered separation: the stack's structure
   loses the part (`MassProperties::without_part`, the parallel-axis theorem run backwards), the
   state carries straight across, and the integrator restarts at the same instant. The
   aerodynamics are unchanged: the part was inside the airframe.
3. **The part leaves at the velocity its centre had in the airframe**, `v_O + ω × c`, from where
   it was. Every material point keeps its velocity, so after burnout the rest and the part carry
   the rocket's momentum and angular momentum exactly; nothing pushes them apart. During a burn
   the reported centre-of-mass velocity includes the centre's drift along the airframe, which
   steps at the release, so the reported momenta differ by `Ṁ (cg' − cg)`: stated in the docs,
   not a loss. A spring or a charge that pushes is left for later, as the ejection impulse was
   added to ejections in ADR-086.
4. **The part then falls as a point mass** under a drag area the user gives (`drag_area_m2`,
   positive): a separated body's equations (ADR-085) with a fixed `C_D S`, to the ground or the
   time cap, in `FlightResult::released`. A point mass drops the part's own spin, `I_p ω`; its
   share of the angular momentum is stated with the test. hpr's tumble model needs body tubes and
   fins, so the area is the user's; the docs point to that model's body term, `0.56` of the side
   profile, as a start. A zero area (a fall as if in a vacuum) is refused.
5. **The flight has one apogee.** The rest's centre sits apart from the rocket's, so on a flight
   that turns it can rise for a moment after the rocket's apogee (review found a second `Apogee`
   8 ms later off an 85° rail), or already be falling when a part leaves just before it. Once a
   part has left and an apogee is recorded, the flight stops watching for one; a release that
   leaves the rest falling before any apogee makes the apogee, and fires the devices waiting for
   it, there. A release on the apogee fires on the recorded one, as a device does, and the
   releases are scanned again until no part leaves, so a part waiting for the apogee leaves at
   it however the others move the rest's centre (re-review found a second one riding to the
   ground). Whether the rest is at or past its apogee is judged once per scan, on the stack as
   the scan began, so the parts listed before one waiting for the apogee don't decide it (a third
   review found the list order changing the apogee's mass by 0.23 kg); a height trigger is judged
   there too. Past the recorded apogee the stack counts as descending for a height trigger, a
   device's included: the third review found a main set above the apogee never opening when a
   part let go there set the rest's centre rising, landing at 98 m/s instead of about 7.
6. **A release can land the rest.** A part let go just above the ground, forward of the centre
   of a rocket falling nose up, can step the rest's centre to or below the ground, which a
   crossing from above never sees (re-review found the flight running underground to the time
   cap); the rocket has then landed at the release (a `Recorder`'s last row is the whole rocket
   there, before the part left).
   A rest stepped below the ground while climbing has not landed; that flight is refused. A part
   let go at or below the ground has landed too.
7. **The optimum-delay flight holds a release, or a shift, fired by a motor's delay**, with the
   charge that fires it (review found the optimum delay moving from 13.53 s to 12.62 s with the
   delay flown). Releases and shifts on other triggers still happen in it.
8. **No release with a separation, ejections or mass shifts**, in either order: their pieces and
   parts are fixed before the flight with every part where the design puts it.
9. **`Simulation::mass_properties(flight, t)`** leaves out a part released at or before `t`,
   taking parts out in the order the flight did.

**Consequences.**

- `releases::tests` holds the milestone's bullets. The 54 mm test design dropping its 200 g of
  ballast at 5 s: its mass, centre and inertia after the release match the two-body hand
  calculation (the rest's inertia is the whole's less the part's own and its reduced mass times
  `|L|² E − L Lᵀ`) and the design built without the ballast to 1e-15, and every step after it
  flies the rest's mass and centre.
- In free flight, with no air and no gravity, the rocket turning about all three axes and the
  ballast 1 cm off the axis, the rest and the part keep the rocket's momentum to 1.5e-13 of
  itself and its angular momentum about their common centre of mass to 7.3e-12, over 213 steps.
  The part leaves 0.146 m/s from the rocket centre's velocity: leaving at the nose tip's velocity
  would miss the momentum by 5.0e-3, and at the centre's by 3.4e-3. Its own spin is 1.5e-3 of
  the angular momentum.
- Under a drogue in uniform air, a release on the way down leaves the rest landing at the
  lighter rocket's terminal speed and the part at its own, both to 1e-6 m/s.
- A flight with no release runs the same arithmetic as before: the separated bodies' equations
  moved into `Simulation::point_mass_derivative` unchanged, and the validation report, the corpus
  flights and the real flights reproduce.
- Left for later: a push at the release; a release in a flight with a separation, ejections or
  shifts; recovery devices on a released part.

## ADR-089: A pod is a stack of body components repeated around the axis (2026-09-27)

**Context.** M1.13 (VISION V19) asks for pods: bodies beside the airframe, as side pods or
outboard motor pods, with their mass properties off the axis, a cited normal force, drag and
interference for each, the `.ork` reader taking them, and a pod design matching OpenRocket. That
is more than one session, and the mass is what the rest stands on.

**Decision.**

1. **Split a to c.** M1.13a is the pod's mass, done when a pod's mass properties match the
   hand-computed parallel-axis values (the milestone's first bullet) and what a pod holds is
   repeated in every pod. M1.13b reads `.ork` pod sets. M1.13c is the aerodynamics and the
   OpenRocket comparison (the second bullet), which needs a design in both codes, so it comes
   after the reader.
2. **A `PodSet` part, external, on a body tube**, with `count`, `radial_offset_m` and `angle_rad`,
   placed along its tube by the usual positions. Its children are the pod's body components (nose
   cone, body tubes, transitions): they take no position and stack aft from the pod set's
   position, and their automatic radii resolve among themselves by the stage rule. Anything else
   as a direct child is refused. This is OpenRocket's own layout for a pod set (a component
   assembly of body components), so the reader in M1.13b maps element to element.
3. **One pod, repeated as a rotational pattern.** The pod is laid out on the body's axis; pod `k`
   is it turned by `φ_k = angle + 2π k / count` about the body's axis and moved to
   `r (cos φ_k, sin φ_k)`. The copies a cluster already uses (ADR-075) carry a roll angle for
   this (`Placement`; a cluster's is 0, so its numbers are unchanged bit for bit), and nested
   copies compose. Review found that moving without turning put a one-fin pod's fin inward on the
   far pod and a symmetric pair's centre 20 mm off the axis; turning keeps what a pod holds in the
   same place relative to the airframe on every pod, as a fin set's fins are. Whether OpenRocket
   turns a pod's contents is to be checked with its jar in M1.13b or c. Each copy is weighed where
   it sits, with its parallel-axis term `I_p = I_cg + m (|d|² E − d dᵀ)` (Meriam and Kraige,
   appendix B), and a motor mount in a pod gives a motor per pod. The pod set weighs nothing.
   Overrides follow ADR-075: one on the pod set must cover its pods (`overrides_include_children`)
   and sets their total; one on a part inside a pod is each copy's, a centre `cg_xy_m` measured in
   the pod as written.
4. **Checks.** A pod's body components are not internal parts: they are neither "outside the
   rocket" nor "past their parent's end". A pod set past its tube's end raises no warning, since
   pods hang from pylons and outboard boosters often run past the tube; one that doesn't touch its
   tube at all is still an error.
5. **Refused until M1.13c.** The aerodynamic model refuses a design with pods (`Unsupported`,
   naming the pod set), before a pod's tube could be read as part of the airframe; so does the
   tumble model. A mass shift and a release refuse a pod's body component as a body component,
   and an ejection refuses it as a joint or a payload with its own message, even for a single pod
   whose one copy would otherwise pass. A part inside several pods is several copies and is
   refused as a cluster's is. The nose-base reference diameter takes the airframe's nose, never a
   pod's.
6. **Refused trees.** A pod set in a pod (copies would multiply), an empty pod set, one of more
   than 64 pods, and an override on a pod set that doesn't cover its pods. Geometry checks for
   pods (overlap with the airframe or each other, radius steps, the airframe's extent) are left
   to #206.

**Consequences.** `hpr-design` gains `PodSet`, `Part::PodSet`, `Component::length_m` and
`Placement`; `PlacedComponent::copies_m` becomes `copies` and `contents_copies_m`
`contents_copies`, each a list of placements. The design page's *Pods* section works two pods and
one off the axis by hand; the test pins them to 1e-15 kg, m and kg·m², and parses the page's JSON.
A pod can't fly until M1.13c; a `.ork` file's pods are still kept unread until M1.13b.

## ADR-090: `.ork` pods placed as OpenRocket places them; pods of no length left out (2026-09-27)

**Context.** M1.13b reads a `.ork` file's `podset` into `hpr_design::PodSet` (ADR-089). The file
writes a count, a `radiusoffset` and an `angleoffset`, each with a `method`, and no document says
what the offset is measured from. The corpus holds 9 pod sets. Three pods are a tube of no length,
drawn to hang fins off the axis, which `hpr-design` cannot lay out. One pod set holds no body
component at all. One pod ends in a flipped nose cone whose automatic radius cannot resolve while
it is read pointing forward.

**Decision.**

1. **Split b1, b2.** M1.13b1 reads every pod set whose pods are body components with a length, and
   counts all of them. M1.13b2 is the rest: pods of no length, and the empty pod set. It needs fins
   that stand off a tube, or a pod of no length in `hpr-design`, and carries M1.13b's bullet
   unchanged.
2. **The distance from the axis is OpenRocket's.** OpenRocket 24.12 was run as an oracle on eight
   probes (`validation/oracles/openrocket/pods.py`). It puts each pod's axis at `R + ρ + v` for
   `relative`, `R + ρ` for `surface` (the number ignored) and `v` for `free`. Here `R` is the
   tube's outer radius, `v` the number and `ρ` the pod's widest radius: a probe whose widest part
   is neither its first part, its first tube nor its last settles that. An automatic radius in a
   pod can only take one stated in the pod, so `ρ` is the widest stated radius. An unknown method
   is read as `relative`, with a warning. `PodSet` keeps a fixed distance, so an automatic tube
   radius is taken from its cached number, with a warning, as a filled tube's thickness already
   is; with none cached, the pod set is left out. A later change can move the rule into the layout.
3. **The angle is `angleoffset`, read as every angle** (the direction is still assumed, as the
   `.ork` page says). Its `method` changes nothing on the probes, for a pod set on a tube on the axis.
4. **A flipped nose cone is a tail cone**, read as a transition from its base radius, the fore
   radius, to a point. Its shoulder goes on the base. With one end a point, a clipped and an
   unclipped transition are the same shape (`hpr_design::shapes`), so the solid is the nose's,
   turned end for end. This replaces "read pointing forward, with a warning" everywhere; the only
   one in the corpus closes a pod. `cargo xtask ork` maps its cached `aftradius` to `foreradius`.
5. **Left out, with a `Skipped` warning:** a pod set on anything but a body tube, one inside a pod,
   one with no body component, one whose pod has a part of no length, one whose pod radii are all
   automatic, one on a tube whose automatic radius cached nothing, and one whose distance is
   negative. A child of a pod set that is not a body component is left out with its own warning.
   An override on a pod set is its pods' total, as a stage's is, with a warning when the file
   does not say so.

**Consequences.** `cargo xtask ork` prints the pod sets written, read, pods read and the reasons any
were left out: 9, 5, 8 and 4 on 2026-09-27. Cached numbers agree on 71 of 75, the pods' 4 among
them. `hpr_validate`'s tests hold every part in the 9 probes' pods to OpenRocket's place to 1e-15 m.
Mass is within 1.2e-7, the centre within 1.1e-7 and roll inertia within 2e-9. OpenRocket's one
pitch inertia is hpr's about `x_B` within 6.1e-7; with one or two pods hpr's about `y_B` differs
by 0.3% to 1.0%, and hpr's is the true one. The public *Pods--powered with recovery deployment*
now weighs within 1% of OpenRocket, from 12.5% light. Its two configurations that had no reason
beyond "a motor in a part not read" now give the real one in `openrocket-flights.md`.

## ADR-091: A pod of no length weighs nothing and holds its parts on its axis (2026-09-27)

**Context.** M1.13b1 left four of the corpus's nine pod sets out (ADR-090). Three are a pod whose
only part is a body tube of no length, no radius and no wall, holding fins (OpenRocket's own
example *Pods--airframes and winglets*, twice in the corpus) or a launch lug (a private design).
OpenRocket names that tube "(phantom body)". The fourth pod set holds nothing at all. `hpr-design`
refused all four shapes: a hollow cylinder of no length, and a pod set with no body component.

**Decision.**

1. **A body tube of no length weighs nothing**, whatever its radius, since it has no wall. Its
   radius and thickness must still be finite and not negative, and the wall no thicker than the
   radius. Only a pod can hold one: a stage still refuses a body component of no length.
2. **What hangs from it sits on a tube of its radius**, as on any tube. With the phantom body's
   radius of 0, that is the pod's own axis: fin roots on it, and a lug's axis its own radius out
   from it. Everything turns with its pod. The pod's widest radius is the tube's.
3. **An empty pod set is read and weighs nothing**, in `hpr-design` as in the reader.
4. **The design checks** treat a pod set of no length as touching its tube when it sits between
   the tube's ends. A part on a pod's tube of no length is never off that tube, nor past its end.
5. **What a tube of no length cannot hold** is left out with a warning, rather than refusing the
   whole design:
   - a part inside it, since it has no room: the pod set is left out;
   - a fin tab deeper than its radius, since it has no wall: the tab is dropped.
   A pod with a nose cone or transition of no length, or a part of negative length, is still left
   out. The corpus has none of these.
6. **An override on an empty pod set is dropped**, with a warning. OpenRocket 24.12 puts that mass
   at the rocket's tip, on its axis, which no design means.
7. **Evidence.** OpenRocket 24.12 was run as an oracle on nine more probes
   (`validation/oracles/openrocket/pods.py`):
   - fins on a tube of no length, four ways: two at 90°; three on each of two pods; two on a tube
     10 mm in radius; and two pods at 30° on a 10 mm tube, three fins each at 20°;
   - a lug turned to 180°, and one at 0°;
   - the empty pod set, with and without an override;
   - a pod set of M1.13b1's kind placed from its tube's `bottom`.
   The probes' numbers are invented. The nine earlier probes came back unchanged.

**Consequences.**

- `cargo xtask ork` reads all 9 corpus pod sets, holding 12 pods, and leaves none out. Cached
  numbers still agree on 71 of 75.
- `hpr_validate`'s tests hold these to OpenRocket on the eighteen probes:
  - every fin root and lug axis in a pod is where OpenRocket puts it, to 1e-15 m;
  - every tube, fin set and lug in a pod has OpenRocket's mass and centre to 2e-15, and a pod's
    nose cone within 1e-7;
  - the roll inertia is within 2e-9, with OpenRocket's fin shortcut (ADR-062);
  - the bare airframe is pinned at 1.17e-7 in mass and −1.125e-7 in centre from OpenRocket's,
    all of it the nose cone, and no probe with pods is further than that.
  - Where pods hold fins, the pitch gap is pinned to three figures, as on the airframe:
    OpenRocket's pitch rule for fins is not known.
- On the 71 designs, those within 0.1% of OpenRocket's mass go from 56 to 57, and within 0.1% of
  its centre from 59 to 62. Roll inertia within 0.1%, with the fin shortcut, goes from 53 to 56.
  Reduced designs go from 8 to 7.
- The private design with the lug pod still does not fly. `openrocket-library-flights.md` lists it
  as "hpr's flight of it failed", not as a reduced airframe. The flight's error names the cause:
  the aerodynamics refuse pods until M1.13c. An empty pod set flies: the aerodynamics and the
  tumble model let it through, since it adds no force and no drag area.
- The RocketSerializer cross-check still leaves out parts in pods (issue #211).
- A lug that writes no angle may be at 180° in OpenRocket, not 0° as hpr reads it. One uncommitted
  probe showed this; it is issue #210.

## ADR-092: A pod's parts are Barrowman's, once per pod, on the axis (2026-09-27)

**Context.** M1.13c asks for each pod's normal force and drag, and its interference with the body,
from a cited source, and a pod design matching OpenRocket. No published source models a pod whole.
OpenRocket documents nothing on its pod aerodynamics; its developers' posts say a pod's forces are
counted per pod and interference is not. The aerodynamic model was built for an airframe on one
axis: forces as a normal-force slope and a moment along it, drag as one axial force, and the
flight engine applying each component's force on the axis. The corpus's pods are OpenRocket's two
examples (one with a single pod) and one private design with a lug on each of its pods.

**Decision.**

1. **Split c1, c2.** M1.13c1 flies pods on the cited model below, checked by hand. M1.13c2 is the
   milestone's second bullet: a pod design with bodies and fins against OpenRocket, with both
   codes' limits stated, which needs probe designs flown in OpenRocket: neither of its own pod
   examples flies in hpr for reasons outside the pods (a freeform fin hpr leaves out; rail buttons
   read without their screw heads).
2. **A pod's body components are Barrowman's** (Barrowman 1967 eq. 3-65, Niskanen 2009 eq. 3.19):
   `C_Nα = 2 ΔA/A_ref` at the part's own centre of pressure, the step from the pod's previous part
   included and its first part stepping up from nothing, as the airframe's first does. Slender-body
   theory's slope is kept at every Mach number: the shock-expansion method covers the airframe
   alone, and a pod's slope kept out of the airframe's list leaves the airframe's march unchanged.
   Body lift is Jorgensen's crossflow at the pod's own fineness. A pod adds all of it once per pod.
3. **A pod's fins are the pod's, turned with it.** Copy `k`'s fin `j` stands at `θ_j + φ_k` in the
   flow; the fin–fin factor counts one pod's fins; the body interference is the pod tube's (1 on a
   tube of no radius). Canted fins on a pod are refused: their roll forcing about the rocket's
   axis, from a root off it, is not modelled.
4. **Drag once per pod.** Each pod gets its own buildup: friction at its own fineness's form
   factor, its nose, steps, boattails and base, coupled among its own parts only; its fins, lugs
   and buttons once per pod. The Reynolds number and the roughness scale stay the rocket's length,
   as for every part. A burning motor's area comes off the base of the body it sits in: the
   airframe's motors off the airframe's base, a pod's off its own pod's (each pod its share of the
   pods' total). Motor mounts in two different pod sets are refused. A pod's tube of no length with
   a radius (a flat disc) is refused; one with no radius, the phantom body, adds nothing.
5. **Forces act on the axis; roll damping is added.** For two pods or more, spaced evenly, the
   pods' offsets add to zero, so their forces summed on the axis have the moments they would have
   at the pods, to first order. What an offset adds at first order is roll damping: a body part
   `ρ` from the axis gives `C_lp = −2 C_Nα ρ²/d²`, and a pod's fin damps by Barrowman's strips taken
   about the rocket's axis, each strip `ρ_0 + y` out, `ρ_0` the root's distance along the span.
   Summed over the fins, that is quadratic in `ρ_0`, so two strip evaluations at the offsets' mean
   plus and minus their standard deviation give the sum exactly. The whole sum takes the pod
   tube's roll-damping factor `k_R(B)`: exact on a tube of no radius; on the offset part, whose
   factor is nearer `K_T(B)`, some 10% small at `τ = 2` (1.33 against 1.5), not sized by a test.
   The pods' drag also damps pitch, `C_mq ≈ −2 C_D,pod Σρ²/d²`, about −0.15 on the worked example,
   against the fins' order of −10³: left
   out.
6. **Left out, sized in the docs.** The body's and pods' effect on each other's flow: in potential
   flow past the body the pods meet the air at `α (1 − (a²/r²) cos 2θ)` (NACA Report 1307 eq. 15
   for `θ = 90°`), with a side component `−α (a²/r²) sin 2θ`; both average out for three pods or
   more and not for one or two (up to ±46% on the worked example's pair). A pod sunk into the
   airframe is not refused (#206). Interference drag, as Barrowman and Niskanen leave it out. A single
   pod's off-axis drag and normal-force moments (issue #213; about 0.15° of trim on the worked
   example with a one-calibre margin).
7. **The tumble model** counts each pod's tubes and fins as the airframe's, once per pod: the
   documentation's fit has no term for pods, which the recovery page says.

**Consequences.**

- `hpr_aero::AeroModel` gains `pod_sets()` (`PodSetAero`), `fin_set_start()` and
  `FinSetAero::pods` (`PodFins`); components run airframe bodies, pod bodies, fin sets.
  `ComponentDragTerms` gains `copies`, `in_pod` and `pod_holds_motors`, and `DragConditions` gains
  `thrusting_pod_motor_area_m2` (`with_pod_motors`). `hpr_design::Layout` gains `pod_set_of`. The
  flight engine's first fin index is `fin_set_start()`, and it splits the burning motors' area
  between the airframe and the pods.
- Tests hold the rules to hand-worked values: slopes and centres to 1e-11, body lift and drag
  shares to 1e-12, roll damping to 1e-12 and to Barrowman's strips summed by hand over pods with
  fins pointing each way to 1e-7. Valetudo with three pods and canted fins spins to the balance
  the pods' damping predicts, to 1e-6.
- The private design whose pods hold a lug now flies: over its 5 configurations, apogee −0.94% to
  +2.15% from OpenRocket's, margin +0.0041 to +0.0048 calibres (census accepted); 3 of the 5 are
  compared with an OpenRocket flight whose parachute opened early. Its pods add only the lug's
  drag, so it checks the pods' placement and weight, not these rules: M1.13c2 does.

## ADR-093: Pod probes flown as the public designs are, and listed apart (2026-09-27)

**Context.** M1.13's second bullet asks for a pod design within the per-case tolerance of
OpenRocket, with the limits of both codes' pod models stated. ADR-092 left it to M1.13c2 because
no design that flies has pods with bodies or fins: OpenRocket's two pod examples fail in hpr for
other reasons, and the one private design with pods holds only lugs. OpenRocket documents no pod
model, and its source is not to be read.

**Decision.**

1. **Probe designs, written by a script.** `validation/oracles/openrocket/pod_probes.py` writes
   six plain-XML `.ork` files to `validation/fixtures/ork/pod-flights/`: one airframe on an
   AeroTech H128W (named by OpenRocket's database digest, so both codes fly one curve, ADR-067)
   carrying three body pods, two finned pods, four finned pods with tail cones, two pods of no
   length with winglets turned to lift in the plane the margin is taken in, two finned pods with a
   motor each and none in the airframe, or no pods (the control). Every outside
   part states `<finish>normal</finish>`, since the two codes read a missing finish differently
   (issue #216).
2. **Flown by the existing pipeline.** `flights.py` flies them with the public designs, and
   `cargo xtask ork-flights` flies hpr on them, so they take the same conditions, curve checks and
   metric definitions as every other comparison.
3. **Listed apart.** A probe is not a design: the report holds them in `probes`, out of the
   designs' statistics and out of M2.2's count toward twenty designs, and a probe hpr cannot fly
   is an error rather than a row in `not_flown`. The census (ADR-084) leaves them out too: its
   OpenRocket group is the examples' headline in the README and the accuracy page, and probes
   would pad it. In CI they are held instead by the bounds in item 4 and by the test that ties
   every report row to the record and to `compare`; `ork-flights --check` holds them locally.
4. **The bar.** Each probe within 5% of OpenRocket's apogee and largest speed (ADR-076's per-case
   tolerance). Because a calm vertical flight tests drag and mass far more than normal force, each
   margin is held within 0.005 calibres, and what the pods change against the control, in apogee
   and in margin at rod clearance, within 1 point and 0.005 calibres: bounds set after measuring
   0.32 points and 0.0004 calibres, tight enough that a pod fin's interference factor taken on the
   airframe (some 0.04 calibres) fails.

**Consequences.**

- Met: apogee +0.41% to +0.81%, largest speed +0.59% to +1.24%, margin +0.0011 to +0.0014
  calibres; the pods change the apogee from −31.3% to +13.0% in OpenRocket and within 0.32 points
  of that in hpr, and the margin within 0.0004 calibres.
- Agreement this close, with interference left out by both, means OpenRocket's pod model is, in
  effect, hpr's on these probes' drag and centre of pressure: the comparison shows the rules are
  applied alike, and cannot size what both leave out. A single pod, a pod past Mach 0.81, flight at
  an angle of attack (and so a pod's body lift), airframe and pod motors burning together, and roll
  are not tested.
- With no `<finish>` stated, the probes flew 6.0% to 7.3% high in hpr: its reader gives such a part
  hpr's 20 µm default where OpenRocket reads 60 µm (issue #216).

## ADR-094: A tilted launch rod flown as OpenRocket records it (2026-09-27)

**Context.** `cargo xtask ork-flights` flew only a vertical rod, so one private design (`C12`,
three configurations) launched from a tilted rod was listed, not compared (issue #173). M2.2's
bar is 20 designs with the five spreads (ADR-072); with the old M2.2e5 blocked on three issues,
the tilted rod is split off as M2.2e5 and the bar moves unchanged to M2.2e6. `conditions.py`
had already measured OpenRocket 24.12's reading: the rod's angle is from the vertical, and its
direction is a compass bearing (a rod tilted 10 degrees toward 0 lands the example north of the
pad, toward 90 east). hpr's `Rail` takes a bearing clockwise from true north and an angle above
the horizon.

**Decision.**

1. **The rail.** `rod_rail` builds hpr's rail from the recorded rod: the azimuth is OpenRocket's
   direction, the elevation `π/2` less its angle, frictionless and unrolled as the vertical rod
   was. A rod tilted below 0 or from 90 degrees is named (`ROD_NOT_TAKEN`) rather than flown.
   A vertical rod keeps its recorded direction too (90 degrees by default), so the rule has no
   jump as the tilt goes to zero; on a vertical rail it only turns the rocket about its axis on
   the pad, which moved the public flights' apogees by under 0.001%.
2. **Rod probes.** `rod_probes.py` writes `pods-none`'s airframe (ADR-093) with a stored
   simulation, as OpenRocket 24.12 writes one, whose rod tilts 5 degrees toward north, 10 toward
   east and toward south-west, and 20 toward east. `flights.py` flies them from each file's
   stored conditions with the other public designs; they are listed apart as probes, as ADR-093's
   are, and `pods-none` is their vertical control.
3. **Where the rocket goes.** `flights.py` now records the position east and north of the pad
   at the row of the largest altitude, and the angle of attack at the rod-clearance row. The
   report gives hpr's position at apogee from where it started beside OpenRocket's, and hpr's
   margin at OpenRocket's angle of attack beside its margin at none; the private report adds
   that margin's difference (`margin_at_openrocket_alpha_cal`), a difference like its others.
4. **The bar.** Each probe within 5% of OpenRocket's apogee and largest speed (ADR-076); its
   position at apogee within 0.1 degrees of OpenRocket's bearing and 1% of its distance; the
   apogee the rod takes off, against `pods-none`, within 0.5 points of OpenRocket's; and hpr's
   margin at OpenRocket's angle of attack within 0.006 calibres of OpenRocket's. The bounds were
   set after measuring 0.03 degrees, 0.64%, 0.27 points and 0.0048 calibres; a bearing read
   anticlockwise fails them by far, and an angle read from the horizon makes a vertical rod a
   horizontal rail, which hpr refuses. A test also points the rail at the bearings
   `conditions.py` measured OpenRocket's rocket landing on. The bearing's residue is motion
   across the tilt: hpr's fits Earth's rotation on eastward motion in sign and size, and
   OpenRocket's goes the other way with no measured cause (hpr's 10-degree east rod
   peaks 0.17 m above its south-west one, OpenRocket's 0.02 m below; at 20 degrees east hpr's
   rocket is 0.09 m south at apogee, OpenRocket's 0.09 m north), and
   part of the distance's is that OpenRocket's position is its highest 0.05 s row, hpr's its
   apogee event: a longer probe would take more of both bounds.

**Consequences.**

- Met on the probes: the tilt takes 0.65%, 2.60% and 10.09% off OpenRocket's apogee at 5, 10 and
  20 degrees, and 0.63%, 2.52% and 9.82% off hpr's; the bearing at apogee agrees within 0.03
  degrees, and the distance within 0.64%. The south-west rod loses what the east one does.
- OpenRocket's rocket reaches its rod-clearance row at an angle of attack that grows with the
  tilt (0.116 degrees at 10, 0.228 at 20, none from a vertical rod), where hpr's margin is taken
  at none. OpenRocket's margin falls 0.016 calibres at 20 degrees, so the margins part by 0.017;
  hpr's margin at OpenRocket's angle falls 0.012 and leaves 0.005. The residue grows with the
  tilt because hpr's CP moves about a fifth less than OpenRocket's for the same angle. The
  report's margin stays at no angle of attack, as ADR-069 defined it.
- `C12`'s three configurations fly: apogee −0.53% to +0.14%, largest speed within 0.33%, margin
  +0.0125 to +0.0366 calibres, nearly all from the CP; at OpenRocket's angle of attack
  +0.0033 to +0.0074, so most of it is the angle. The two reports now hold 15 designs with the
  five spreads, against M2.2e6's 20.
- Taking the recorded rod on a vertical flight turned several public designs' sideways drift
  with the rocket: hpr's vertical flights of three designs drift 1.7 to 10.5 m by apogee where
  OpenRocket's go straight up, which the new positions show (issue #219).
- Not tested: wind with a tilted rod, a rod longer than 1 m on the probes, and roll on the rod.

## ADR-095: The single pre-1.9 override flag read as OpenRocket reads it (2026-09-27)

**Context.** Before schema 1.9 a `.ork` said once, with `overridesubcomponents`, whether a
part's overrides cover the parts inside it; later files say it per quantity, with
`overridesubcomponentsmass`, `...cg` and `...cd`. hpr read the old flag as setting all three,
unless a per-quantity flag was also written, which then won; and it warned (ADR-052, item 5).
The warning made any design carrying the flag "an airframe not read exactly as written", which is
not flown (ADR-055). Issue #174 names two private designs held that way: `C05` (six elements, one
`true`, on a stage's mass override; schema 1.4) and `C10` (three, all `false`; schema 1.8). M2.2's
bar is 20 designs with the five spreads (ADR-072); after M2.2e5 the two reports held 15. The old
M2.2e6 ("Twenty designs", blocked on #174 and #133) is split by reading: M2.2e6 the old flag,
M2.2e7 fin fillets and an inner tube's automatic radius (the rest of #174), M2.2e8 tube fins whose
radius OpenRocket works out (#133), and M2.2e9 the bar, unchanged.

**Decision.**

1. **Measured, not assumed.** `conventions.py` gained eleven probes: the old flag alone on a
   stage's mass and centre overrides and on a tube's mass override, in schema 1.4, 1.8 and 1.10
   files; `false` as well as `true`; and beside a per-quantity mass, centre or drag flag, in both
   orders. It records the structure and, through OpenRocket's public getters, the three flags it
   read each part with. OpenRocket 24.12 reads the old flag as setting all three, in all three
   schema versions, and where both forms are written **the later one wins**, quantity by
   quantity. The 75 earlier probes' answers are unchanged.
2. **hpr reads it so.** `Values::overrides` takes, for each quantity, whichever of its own flag
   and the old one comes later among the element's children, and raises no warning. This amends
   ADR-052's item 5. The old rule differed only where the old flag comes second, which no file in
   the corpus does. A flag tag written twice on one element is read at its first copy and warned
   of (`Dropped`), because which copy OpenRocket takes is not measured; no file in the corpus does
   that either.
3. **Held to the probes.** `hpr_validate::openrocket` holds hpr to OpenRocket on all eleven: the
   three flags part by part, the mass, no warning, and the centre of mass. The centre is
   OpenRocket's except where a mass override covering the parts inside states no centre. That is
   ADR-061's departure: 3.686 mm, the same as on the per-quantity flag's probe. Making the
   per-quantity flag win again fails the test.

**Consequences.**

- `C05` flies all five configurations: apogee −0.03% to +0.26%, largest speed within 0.61%,
  margin within 0.0113 calibres, mass at launch +0.000%. The two reports hold 16 designs.
  `C05/2`'s margin gap is nearly all its centre of mass (−0.0111 calibres, against at most
  0.0004 on the other four): not traced.
- **M2.2e6's bar is not met for `C10`.** Its three flags are `false`, which the old and new rules
  read alike, so the flag never decided its numbers. It had a second, independent reason not to
  fly: its one configuration separates a stage with no motor ahead of it, which can come before
  apogee, and `ork-flights` does not fly that (#184). With the flag read, its reason for not
  flying changes from "an airframe not read exactly as written" to "stages hpr can't separate as
  written"; the report lists it so. It stays in M2.2e9's pool behind #184.
- The corpus survey's warnings fall from 31 to 21: the 10 were this flag, 9 in `C05` and `C10`
  and 1 in a Loft demo design outside both reports.
- Not measured: an old flag that is neither `true` nor `false` (hpr drops it, out loud, and the
  file is not flown), and which copy of a tag written twice OpenRocket takes.

## ADR-096: Fin fillets and an automatic radius inside a nose cone read as OpenRocket reads them (2026-09-28)

**Context.** Issue #174 holds back two private designs from M2.2's count of 20. `C01` has fin
fillets, the rounded glue joint along a fin root. hpr left them out with a warning, a measured
departure since ADR-064 (−0.808% and −2.79% of the probe's mass at 5 and 10 mm). A warning makes a
design "an airframe not read exactly as written", which is not flown (ADR-055). `C06` has fillets
too, and a coupler inside a nose cone whose outer radius is `auto`. hpr's neighbour rule
(ADR-007, ADR-054) looks only at body tubes, so inside a nose it found nothing and took
OpenRocket's default, which is wrong there.

**Decision.**

1. **Measured, not assumed.** `conventions.py` gained 21 probes. The 86 earlier probes' answers are
   unchanged.
   - Seven fillet probes: a 30 mm radius, their own material, no material named, a single fin,
     four rounded fins, a freeform fin set and a wider tube.
   - Fourteen bore probes: a coupler with an automatic radius at a nose's bottom, past its base,
     with a shoulder, long from its middle, at the tip, and with a wall thicker than its bore; an
     ogive nose; a transition; an engine block; a centering ring and a bulkhead; a mass component
     inside; an inner tube written `auto`, in a nose and in a tube.
2. **A fillet is a prism of its section along the root.** The section is the region between the
   tube, the fin's mid-plane and the fillet's circle: OpenRocket's reading, which leaves the fin's
   thickness out. With body radius `R` and fillet radius `r`,
   the circle's centre is `c = √(R² + 2Rr)` along the fin and `r` off it, and the region is the
   triangle `(0, 0)`, `(c, 0)`, `(c, r)` less the body's sector up to `θ = atan2(r, c)` and the
   fillet's quarter circle up to the tangent points. There is one fillet each side of every fin,
   of the root chord's length, in `filletmaterial` (cardboard's density when none is named, as
   OpenRocket does). hpr computes the section's area, first moment and second moments in closed
   form, the sectors' second moments in a stable form, and tests it by quadrature to 1e-11. The
   final subtraction still loses about `log10(R/r)` digits, at most 2 for a real fillet
   (`r/R ≥ 0.01`). Past a thousand times the body radius it cancels away, so such a fillet is
   refused; under a millionth of it, or on a body of no radius, a fillet weighs nothing.
   OpenRocket's mass and centre of mass agree to 1e-15 on every probe (the test holds them to
   1e-12). The pitch inertia is apart by −0.0077% to −0.638%, largest with the 30 mm fillets, and
   +0.425% on a single fin: hpr's is the exact prism, and OpenRocket's pitch rule for fins is not
   measured (ADR-062).
3. **An automatic outer radius inside a nose or transition is its bore at the narrow end.** It is
   the parent's outer radius at whichever end of the part is narrower, less the parent's wall,
   the shoulder left out; zero if the wall is thicker. A filled nose has no bore and is refused.
   A packed radius inside a nose is still not taken. A coupler at the tip, where the wall meets
   the axis, has no radius: OpenRocket weighs it as nothing, and hpr refuses the design. An inner
   tube whose own radius is thinner than its wall is solid, in a body tube too.
4. **An `innertube` written `auto` keeps 9.5 mm**, as OpenRocket does (a motor mount's radius is
   not something OpenRocket works out). It raises no warning: it is OpenRocket's reading, and a
   warning would keep the design from flying (ADR-055). A number written after `auto` is not an
   input (ADR-052), so it is not kept either.
5. **Held to the probes.** `hpr_validate::openrocket` holds the fillets' mass, centre, roll and
   pitch inertia (pinned rows), and every bore probe's parts to OpenRocket's mass (to 1e-14) and
   station (to 1e-15). The noses carry the gaps their walls already had: up to 3.9e-5 of the mass and 2.5e-6 m.
   One mass component is pinned apart: OpenRocket shortens its packed length to keep its volume,
   and hpr keeps the written length (#186).

**Consequences.**

- `C01` and `C06` fly. The two reports hold 18 designs.
- In the mass survey the fillet cause is gone. One private design stays outside 1% in mass: its
  airfoil fins, which OpenRocket weighs as the outline times the thickness times 0.85 (ADR-062),
  where hpr integrates the NACA section. That cause is now sized: `cargo xtask ork` names it only
  when the design comes within both thresholds with its fin sections weighed OpenRocket's way.
- The library report's largest speed is now taken over every sample up to the apogee. The old
  rule stopped at the first sample whose vertical speed stopped being positive. A rocket held on
  the pad can read a few µm/s either way (#223), so on `C06/1` the old rule stopped before
  liftoff.
- `C06/1`'s apogee is 13.60% above OpenRocket's; ADR-097 sizes it.

## ADR-097: A cause in the drag sized by hpr flying OpenRocket's drag (2026-09-28)

**Context.** `C06/1` is the first supersonic flight in either OpenRocket report. Its apogee is
+13.60% and its largest speed +7.98% from OpenRocket's, with neither named cause of ADR-073.
M2.2e4's bar asks for a written, sized cause. The mass is within 0.77%. Taken per component at
zero angle of attack through OpenRocket's public API (a look not kept as a record), OpenRocket's
drag differs from hpr's in two ways:

- **Base drag while a motor burns.** hpr takes the burning motor's cross-section off the aft base
  (Niskanen 2009, pp. 50–51, who cites Fleeman's *Tactical Missile Design* for it; Loft lesson
  L13). OpenRocket 24.12 does not.
- **Supersonic pressure drag.** hpr's is about twice OpenRocket's. OpenRocket gives the tangent
  ogive nose almost none well above Mach 1, and the airfoiled fins about a quarter of hpr's
  (#222).

The two pull in opposite directions. So neither can be named a cause by being present
(ADR-073's point); each needs a size.

**Decision.**

1. **OpenRocket's base rule is measured on public designs.** A new oracle, `base_drag.py`, flies
   every motor configuration of the jar's example designs with nothing deployed. It records
   OpenRocket's base-drag column over Niskanen's whole-base coefficient (`0.12 + 0.13 M²` below
   Mach 1, `0.25/M` above), with thrust and without, and the burning motors' area over the
   reference area. It is committed as `validation/fixtures/ork/openrocket-base-drag.json`. Of its
   56 flights, the 42 of one data branch (nothing separating) are compared; on each, the
   ratio while a motor burns equals the ratio after, to 1e-12. The motors there cover 9% to 94%
   of the reference area, and one flight burns motors in pods. Taking the area off would drop
   the ratio by about that much. On every flight the drag coefficient is the sum of the
   friction, pressure and base columns while a motor burns, so the base column is what
   OpenRocket flies. A test in `hpr_validate` holds all of this. The 14 flights left out
   separate, which changes the base itself.
2. **An opt-in flies OpenRocket's base rule.** `AeroModel::with_full_base_drag_under_power` and
   `Simulation::with_full_base_drag_under_power` keep the whole base while a motor burns, pods
   included; a sustainer lit after a separation keeps the rule. It is for comparisons with
   OpenRocket, not a better model. hpr keeps its documented source's rule by default. Neither rule
   has been checked against a measured flight; #222 holds both questions.
3. **A cause in the drag is sized by hpr flying OpenRocket's drag.** A second oracle,
   `drag_curves.py`, flies each configuration as `flights.py` does, with nothing deployed. It
   records OpenRocket's drag coefficient along the first branch, launch to apogee, as two curves in
   Mach number:
   - power on: the burning rows, each faster than every earlier one;
   - power off: the rows after the last burning one, each slower than every earlier one.

   `cargo xtask ork-flights --library` flies hpr on them as a drag table. It does this for an
   apogee more than 5% off with no other named cause and no separation. It also flies hpr with
   only OpenRocket's base rule. The curves are private values, so the record stays in
   `corpus-out/`. A probe hpr fails to fly is marked failed, and the flight stays compared.
4. **What that shows, and what it doesn't.** Within 5% of OpenRocket's on OpenRocket's drag,
   the metric's cause is "hpr's own drag coefficient". The apogee and the largest speed are each
   judged by their own number. The label is weaker than ADR-073's causes. Those are things hpr
   knowingly does not do; this is the whole drag model. With mass within 1% and OpenRocket's own
   thrust curves, it bounds the net effect of every other cause on that flight by what is left,
   though parts of it can cancel, and the table in Mach number leaves out OpenRocket's
   dependence on the Reynolds number. It cannot tell a deliberate modelling choice from a defect
   in hpr's drag, nor which code is right.
5. **So each flight it names needs its own written breakdown.** The breakdown says which drag rule
   moves the flight, by how much, and what is left open. A list in the library report's tests
   (`DRAG_CAUSES_WRITTEN`) names each such flight with the record that breaks it down. A new flight
   labelled this way fails the bar's test until one is written.

**Consequences.**

- **`C06/1`'s breakdown** (this record):
  - On OpenRocket's drag: apogee +1.11%, largest speed +1.04%, within the bar.
  - With only OpenRocket's base rule: −10.71% and −3.43%. That switch alone lowers the apogee by
    24.3 percentage points; it is measured by its own flight.
  - Switching the rest of the drag to OpenRocket's then raises it 11.8 percentage points. That number is by
    subtraction, and the split depends on which change is made first.
  - A look at OpenRocket's per-component columns on this flight, not kept as a record, points
    the rest to the supersonic pressure drag, with the friction and base close. It is a lead for
    #222, not a measurement the repository reproduces.
- Which supersonic pressure drag is right, and which base rule, is open (#222). Neither code has
  been checked against a measurement on this shape, and the public report has no supersonic
  flight.
- The library run now needs `corpus-out/openrocket-drag-curves.json`, written by the current
  scripts with the pinned jar. The public report is unchanged: none of its flights is more than
  5% off without a named cause.

## ADR-098: A tube fin set's automatic radius read as OpenRocket reads it (2026-09-28)

**Context.** A tube fin set written `<radius>auto</radius>` asks OpenRocket to size its tubes from
the body they ring (#133). hpr had no rule for it and left the set out with a warning, which made
OpenRocket's *Tube fin rocket* example "an airframe not read exactly as written" (ADR-055). Its
two copies are the only tube fins in the reference library. M2.2e8 asks for the radius as
OpenRocket 24.12 resolves it, and for the example to fly. hpr-aero refuses tube fins until a cited
method exists, so the flight cannot follow from the radius alone.

**Decision.**

1. **Split M2.2e8.** M2.2e8 is now the radius; a new M2.2e9 is a cited aerodynamic method for tube
   fins, so that the example flies with e4's bar on it; the bar of 20 designs moves, unchanged,
   from M2.2e9 to M2.2e10. Between them, the old M2.2e8's *done when* is kept whole.
2. **Measured, not assumed.** `conventions.py` gained 19 probes. On a 50 mm tube: `auto` sets of 1,
   2, 3, 4, 5, 6, 8, 9, 12, 20 and 100 tubes; 6 tubes of a stated 20 mm radius, with and without a
   10 mm radial offset; 12 of a stated radius; 6 `auto` tubes with a wall thicker than their
   radius; and 4 `auto` tubes on a tube whose own radius is `auto`. On a 20 mm tube: 1, 2 and 5
   `auto` tubes. Each probe records the set's resolved radius, wall, count, body radius, centre
   and unit inertias through OpenRocket's public getters. The 107 earlier probes' answers are
   unchanged.
3. **The radius closes the ring.** For `N ≥ 3` tubes on a body of radius `R`, the tubes' axes sit on
   a circle of radius `R + r`, and neighbours touch when the chord between them, `2(R + r)
   sin(π/N)`, is `2r`: `r = R sin(π/N) / (1 − sin(π/N))`. OpenRocket's radius equals it bit for
   bit on every probe in Python, and within 1e-15 in hpr's test (six tubes on 50 mm: 50 mm; four:
   120.7 mm; eight: 31.0 mm). For one or two tubes no finite ring closes, and OpenRocket gives the
   body's radius, on both bodies; hpr does the same.
   `TubeFinSet::closing_radius_m` is the rule, and `AutoDimension::OuterRadius` now applies to a
   tube fin set.
4. **A wall thicker than that radius is cut to it**, so the tubes are solid, as OpenRocket weighs
   them, and as an inner tube's is (ADR-096).
5. **More than 8 tubes are read as 8.** OpenRocket 24.12 reads 9, 12, 20 and 100 as 8, stated
   radius or `auto`. hpr does the same, with a `Dropped` warning, before its own limit of 64 parts
   in a row.
6. **A radial offset moves nothing.** OpenRocket's numbers for the 10 mm offset are those without
   it. hpr already read such a set sitting on the body, with a warning; that stands.
7. **Roll inertia is a departure.** For two tubes or more, OpenRocket's rotational unit inertia is
   larger than `(R + 2r)²`, the most any mass inside the ring can have: 0.3047 m² for six 20 mm
   tubes on 50 mm, against a bound of 0.0081 m². hpr keeps its hollow tubes at `R + r`. For one
   tube, OpenRocket's centre is at `R + r`, where hpr puts it, and its unit inertia is the tube's
   own, `(r² + rᵢ²)/2`, as hpr's is.
8. **Pitch inertia is a second departure.** On all 19 probes OpenRocket's longitudinal unit inertia
   is `N` times one tube's own across its axis, `N((r² + rᵢ²)/4 + L²/12)`, with no term for the
   tubes' distance from the body's axis. hpr's, for three tubes or more, is one tube's own plus
   `(R + r)²/2`, the ring's spread. OpenRocket's is 1.3 to 2.6 times it on the probes (1.77 on six
   tubes of 20 mm radius). On the probes, whose tubes are 0.1 m long, it stays under the bound
   `(R + 2r)² + (L/2)²` for any mass inside the ring; on the *Tube fin rocket*'s longer tubes it is
   18% over it (3.352e-3 m² against 2.834e-3 m²), since `N L²/12` passes `L²/4` once `N > 3`. hpr
   keeps its own. For one tube the two agree; for two, on the probes, OpenRocket's lies between
   hpr's two pitch values, across the pair and along it.
9. **A design with tube fins is weighed, not flown.** `hpr-aero` refuses tube fins, so `hpr-io`'s
   screen leaves every configuration of such a design out as `NotFlown::NoAerodynamicModel`, and its
   stored runs out of the reproduction screen as unflyable. M2.2e9 removes the screen.

**Evidence.** `hpr_validate::openrocket::tests::a_tube_fin_sets_automatic_radius_reads_as_openrocket_does`
holds all 19 probes: each part's mass within 1e-14 of OpenRocket's and its station within 1e-15 m
(the one nose cone, on the probe whose tube takes its radius from it, within 5e-5 and 5e-6 m, the
bore probes' bound), the radius and wall within 1e-15, the count, hpr's roll and pitch against
their closed forms (pitch for one tube and for three or more), and both departures both ways: OpenRocket's pitch as `N` times one tube's own
to 1e-14, its roll past the ring's bound. `hpr_design`'s
`a_closing_ring_of_tubes_touches_the_body_and_its_neighbours` checks the rule by construction.
`cargo xtask ork` now reads both copies of the *Tube fin rocket*: its mass is within 5.0e-6 of
OpenRocket's and its centre within 2.4e-6. Designs within 1% of OpenRocket's mass go from 66 to 68
of 71, and within 1% of its centre from 66 to 68; reduced designs go from 6 to 4. Its pitch
inertia is 1.9800% below OpenRocket's. `cargo xtask ork` swaps OpenRocket's pitch rule into hpr's
structure for each tube fin set (`tube_fin_pitch_shift_kg_m2`, tested on three probes to 1e-12) and
prints the gap left: +0.0023%.

**Consequences.**

- The *Tube fin rocket*'s `[D12-7]` stays unflown in `cargo xtask ork-flights`, now for a reason of
  its own: "tube fins, which hpr has no aerodynamic model for yet". M2.2e9 removes that reason.
  The configurations the reader flies stay 106 of 170, in 27 designs.
- Its roll inertia stays about 99% below OpenRocket's in the mass survey. That is the departure
  above, filed under the survey's existing cause for tube fins.
- The count of 8 is OpenRocket's reading of the file, not a limit of the physics. hpr's own design
  files still take any count.

## ADR-099: Tube fins flown as ring wings (2026-09-28)

**Context.** Since ADR-098 hpr reads and weighs tube fins, but `hpr-aero` refused them for want
of a cited method, and `hpr-io` screened every configuration of a design holding them out of
flight (`NotFlown::NoAerodynamicModel`). M2.2e9 asks for a cited normal-force and drag method,
pinned by tests, and for OpenRocket's *Tube fin rocket* to fly with M2.2e4's bar on it: an apogee
more than 5% off needs a written, sized cause. OpenRocket's own source is GPL and was not read;
its method is known only from its pull requests' prose.

**Decision.**

1. **Each tube is a thin ring wing.** Its normal-force slope on the area `d L` (`d` the mean
   diameter, `λ = L/d`) is Weissinger's `π²/(1 + πλ/2 + λ arctan 1.2λ)` (1955, as quoted by Wagner
   2021, arXiv:2102.02647, eq. 15). It runs from Ribner's lifting-line `π²` for a short ring to
   slender-body theory's `π/λ` for a long one, which is Hoerner's `L = q d² π α` (*Fluid-Dynamic
   Drag*, 1965, p. 7-13): a ring turns the air inside it too. On Fletcher's five measured rings
   (NACA TN 4117, 1957, Fig. 11, Mach 0.13, `A = d/c` from 1/3 to 3), it is within 3% (2.3% at
   worst, read from the chart). Faster than Mach 0 the slope takes Göthert's rule,
   `C_Lα(λ/β)/β`, the idea of Barrowman's fin slope. It leaves the long-ring limit unchanged.
2. **The set is `N` times one tube, with no interference factor.** None is measured or cited. The
   body's crossflow at a tube, `R²/s² e^{−2iφ}`, sums to zero over three or more tubes evenly
   spaced, so the model refuses fewer than three. That is a derivation, stated as one.
3. **The centre is Fletcher's measured aerodynamic centre, from `A = 2/3`.** Fletcher's Fig. 8
   (p. 16) reads 0.143, 0.203, 0.253 and 0.355 of the chord at `A = 2/3`, 1, 1.5 and 3. The
   table ends there: a shorter ring is refused (point 5). Below `A = 2/3` the centre runs in a straight line to the leading edge at
   `A = 0`, where hpr's own slender-body derivation puts a thin ring's lift. His `A = 1/3`
   ring, whose centre sits 0.11 of its chord ahead of its leading edge, is left out. Fletcher
   puts that point down to its low aspect ratio: that ring behaves more like a slender body of
   revolution than the others (p. 4). That it would not carry over to a paper tube is hpr's
   inference, untested: his rings had a Clark Y section 11.7% of the chord thick, all outside a
   straight bore, so at a chord of three bores the wall is 0.35 of the bore thick and about two
   thirds of the frontal disc, against a few per cent for a paper tube. His thinner rings may
   carry some of the same forward shift. The choice is a judgement that moves the *Tube fin
   rocket*'s margin a long way: 0.29 calibres with his −0.11 held below `A = 1/3` (a one-off
   run, not in the report), 0.79 with the line, against OpenRocket's 1.87. The rule is taken at the
   stretched ring's `β A`.
4. **The drag is friction inside and out, and a square edge on the wall.** Friction is on
   `2π L (r_o + r_i)` per tube at the rocket's skin-friction coefficient. The wall's annulus
   takes a square fin edge's pressure drag: stagnation at the front and base drag behind
   (Niskanen 2009, eqs. 3.90 and 3.92).
5. **Mach 0.8 and above are refused**, by the normal force, the component stations, the roll and
   the drag buildup. No source covers tube fins near the speed of sound, where a tube's flow may
   choke. A flight that reaches Mach 0.8 stops with the tube-fin model's error. Also refused:
   solid tubes, tube fins on a pod, a ring shorter than a third of its diameter (`A > 3`, past
   Fletcher's rings), and tubes that overlap each other. Roll damping follows the pods' rule,
   `−2 C_Nα ρ²/d²` per tube at its axis's distance `ρ`, a derivation. Pitch damping comes from the
   local flow at the set's centre, like any other component's.
6. **The importer's screen is gone.** `NotFlown::NoAerodynamicModel` had no other use and is
   removed. A tube fin set the model still refuses fails the flight, as any refused part does.
7. **The public report sizes a cause in the drag as the library's does** (ADR-097).
   `drag_curves.py`, unchanged, flew the jar's 17 examples, unpacked outside `refs/`. Its record
   is committed as `validation/fixtures/ork/openrocket-drag-curves.json`, since the examples are
   public. `cargo xtask ork-flights` reads it for the public designs. A test holds it to the
   flight record's examples, by digest, and to the current scripts. The public report's tests
   hold their own `DRAG_CAUSES_WRITTEN`, which names this record for the *Tube fin rocket*.

**Evidence.**

- `hpr_aero::tube_fins` tests:
  - the slope against Fletcher's five rings within 3%, and both of its limits;
  - the centre against the table, the line to the leading edge, and the hold past `A = 3`.
- `hpr_aero::model` tests:
  - a set's slope, moment, station, listed component and roll damping against the formulas by
    hand, at Mach 0, 0.3 and 0.7, with no side force at any roll;
  - its friction and wall drag by hand;
  - the refusal at Mach 0.8 in all seven calls, in one shape, pinned to the tube-fin model's
    error, and in the tubes' own drag terms called alone;
  - a short ring's slope by hand at Mach 0 and 0.6, and its centre from Fletcher's table;
  - the guide's worked example, slope, centre and areas;
  - the refusal of two tubes, solid tubes, overlapping tubes and a ring past `A = 3`.
- `hpr_sim` flies a rocket with six tube fins below Mach 0.8 and stops its flight past it with
  the tube-fin model's error.
- `cargo xtask ork-flights` flies the *Tube fin rocket* `[D12-7]`:
  - its apogee is 302.5 m against OpenRocket's 282.8 m, +6.95%, and its largest speed +6.40%;
  - on OpenRocket's drag curve they are +0.025% and +0.033%, so the cause is named "hpr's own
    drag coefficient", and M2.2e4's bar holds;
  - with only OpenRocket's whole base under power, the apogee is +4.61%.
- A look at OpenRocket's per-component output at Mach 0.2, not kept as a record, as ADR-097's
  was. Its numbers are leads for #228, not measurements the repository reproduces:
  - the nose, body and lug agree with hpr's within 0.005;
  - the tube fins take 1.18 of OpenRocket's 1.777 and 1.05 of hpr's 1.654;
  - OpenRocket's tube fins take a slope of 37.8 per radian, 1.62 times the 23.4 that six isolated
    thin rings reach at their long-ring limit; hpr's is 22.9;
  - OpenRocket's centre sits a quarter of the tube's length aft of the leading edge.
- OpenRocket's pull requests #1333 and #2066 refined its tube-fin drag against measured flights
  of two tube-fin rockets. The table, in a comment on #2066, has seven flights in five motor
  cases; OpenRocket's apogee is within −2% to +5.2% of the measured one and high in four of the
  five.

**Consequences.**

- **`Tube fin rocket [D12-7]`'s breakdown** (this record):
  - On OpenRocket's drag: apogee +0.025%, largest speed +0.033%, within the bar. The net gap is
    the drag's; parts of it could cancel.
  - With only OpenRocket's base rule, the whole base under power: +4.61% and +3.01%. That switch
    alone lowers the apogee by 2.34 percentage points; it is measured by its own flight.
  - Switching the rest of the drag to OpenRocket's then lowers it 4.59 percentage points. That
    number is by subtraction, and the split depends on which change is made first.
  - The look above points the rest to the tube fins, 1.18 of OpenRocket's drag coefficient
    against 1.05 of hpr's. It is a lead for #228, not a measurement the repository reproduces.
- hpr's tube-fin drag probably reads low. OpenRocket's was fitted to measured flights, and hpr's
  leaves out the gaps between the tubes and the body and how the flow develops inside a tube. No
  measurement checks either code here. #228 holds the search for a measured source, for the drag
  and for a thin tube's centre.
- The example's margin at rod clearance is 0.79 calibres, 1.08 below OpenRocket's 1.87. The quarter
  length alone would add about 0.51, by the look above. The report shows the gap. It is past the
  census's 0.5-calibre margin bar (ADR-084), accepted in writing with this record; M2.2e4's bar
  covers only the apogee. Nothing measured separates the two codes.
- The configurations the reader flies go from 106 to 108 of 170, in 29 designs. The public report
  flies 34 configurations and names a cause for each of its seven apogees over 5%. With the
  library, 19 designs carry all five spreads, one short of M2.2e10's 20.
- A tumbling airframe with tube fins is still refused: the tumble model has no factor for them.
- A flight on a drag or normal-force override table (ADR-009, ADR-032) still stops at Mach 0.8
  when it holds tube fins: it takes the components' stations and the roll from the model, which
  refuse past it.

## ADR-100: A motor whose ignition never comes flown unlit, as OpenRocket flies it (2026-09-28)

**Context.** M2.2e10 asks for 20 designs across the two OpenRocket reports with all five spreads;
M2.2e9 left 19. Of the candidates, the private design `C04` was held back by one reading alone: a
motor set to light at an event that never comes. ADR-076 refused such a motor ("hpr has no motor
that is never lit on purpose"), and with it the configuration. It also read `C04`'s record as
lighting that motor at launch. That was wrong: the ignition at launch in the record is another
motor's, set to `launch`, and the record shows the motor in question never lit and rode with the
stack. The rest wait on larger work: a payload's own drag (#184), two separations (#183), parallel
stages and the pod examples' parts.

**Decision.**

1. **`hpr_design::Ignition::Never`.** A motor set so is placed as a motor out is in every tube
   (`PlacedMotor::fails`): carried loaded, with no thrust, never a burnout that lights another
   motor. That reuses M1.9b's failed tube, whose mass and timing are already tested. The check for
   a cycle of burnouts takes it as lit, as it takes every motor. A sustainer cut for a powered
   separation keeps it unlit.
2. **The `.ork` words that never fire read as `Never`.** `never`; `burnout` or `ejectioncharge` in
   the bottom stage, which has no stage below (`automatic` there still means launch); and
   `ejectioncharge` or `automatic` over a plugged motor (`<delay>none</delay>`). A motor lit by
   one that never lights is written `Never` too, so every reader of the ignitions agrees. A
   separation at the burnout or ejection charge of a stage's own motors that never light never
   comes, and is dropped.
3. **Still refused.** A word hpr does not know; a stage below with no motor (the same event that
   never comes, but not probed) or with motors in more than one mount; a motor below that states no
   delay; a separation at the ignition of a motor that never lights, or at the charge of the
   stage's own plugged motor (not probed); a configuration in which no motor lights, which would
   not leave the pad; and, newly reachable, a separation at launch, since hpr's flight fires a
   separation only once the rocket is off the rod.
4. **Measured, not assumed.** `validation/oracles/openrocket/unlit_motors.py` runs OpenRocket 24.12
   on its *Two stage high power rocket* example's second configuration (an I59WN-P over an
   I357T-14). In three flights the booster never lights (`never`; `burnout` with a `burnout`
   separation; `ejectioncharge` with an `ejection` separation) and the sustainer lights at launch.
   In two the booster is plugged and lit at launch, and the sustainer waits on its charge
   (`ejectioncharge`, `automatic`). In all five, OpenRocket logs one ignition, at launch, no
   separation, one branch, and by apogee the stack has lost exactly the lit motor's propellant
   (0.272 kg or 0.1792 kg, to 1e-9 kg). Two controls: as written, both motors light and the
   booster drops; with the booster not plugged, its charge lights the sustainer 14 s after its
   burnout, which also measures ADR-076's reading of `ejectioncharge`. The record is
   `validation/fixtures/ork/openrocket-unlit-motors.json`, pinned to its scripts' and the jar's
   digests by the test `openrocket_flies_a_motor_whose_ignition_never_comes_unlit`; the reader's
   tests set the same words on synthetic designs and pin `Never`.
5. **Two different motors, on purpose.** The same probe on the first configuration, an H148R-0 in
   each stage, shows OpenRocket's mass column losing nothing while the sustainer burns (#185). It is
   recorded as `h148r-pair-booster-never` and held by the same test, so a change is seen.

**Consequences.**

- `C04/1` flies: apogee +0.88%, largest speed +1.37%, margin +0.0223 calibres, launch mass
  −0.001%, within M2.2e4's bar with no cause needed. The library report has 11 of its 12 designs
  in all five spreads, and with the public report's 9 the reports hold 20: M2.2's bar of 20
  designs is met. `cargo xtask census --accept` records the row.
- `C04/1` is a staged flight, the kind #185 concerns. Its mass at rod
  clearance agrees with OpenRocket's to 0.000% (the report's row). A check run locally on the
  private file, not committed, found OpenRocket's mass falling by each burning motor's propellant
  along the flight, so #185's symptom does not show there; no committed check holds that, and the
  reports screen no flight for #185.
- A separation timed after launch but before the rocket leaves the rod still reads, and the flight
  fires it only as the rocket leaves (#231); only one at launch itself is refused.
- A motor waiting on one that never lights is read `Never` by inference: none of the probes chains
  two.
- `cargo xtask ork`: the reader flies 109 configurations of 170, in 30 designs (108 in 29 before);
  one is left out as *a motor hpr can't light as written* (2 before), a public pod example whose
  stage below has three mounts.
- The public report does not move: none of its configurations had a motor whose ignition never
  comes.
- A design whose only motor is set to light at an event that never comes is still left out, now
  as one in which no motor lights; hpr gives no warning for a motor it flies unlit.
- M2.2e is met (its *done when* is M2.2's, unchanged). M2.2 stays open until M2.2b, whose five
  parts are met, is rolled up.

## ADR-101: OpenRocket's mass conventions rolled up; M2.2 left open for two lessons (2026-09-28)

**Context.** M2.2b had three conditions. The tests of Loft lessons L51 (which mass or centre
override wins) and L87 (stored results used as references) must be live. Every convention
ADR-060 lists, and roll inertia, must be hpr's rule or a written departure. And M2.2a's survey
must be run again. Its five parts (ADR-061 to ADR-065) are met. Later decisions changed some
answers: pods (ADR-089 to ADR-091), clusters (ADR-075), fillets (ADR-096) and tube fins
(ADR-098). This roll-up says where each convention stands now, and why M2.2 cannot close with it.

**Decision.**

1. **Each convention ADR-060 lists, where it stands.** Every departure below is held by a test.
   Other departures the probes found along the way are in the guide: the elliptical fin's outline
   on the mass page, a mass component's packed length in the `.ork` format guide.

   | convention | now | settled by |
   |---|---|---|
   | a shoulder written with no wall | OpenRocket's rule: it weighs nothing | ADR-061 |
   | a part weighed by volume, with no material | OpenRocket's rule: 680 kg/m³ | ADR-061 |
   | which override wins (L51) | OpenRocket's mass on every probe; the centre under a part's covering mass override that states no centre is a departure (3.7 mm; 19.7 mm when the part inside has its own override); flags that disagree are a limitation (4.7 mm) | ADR-061 |
   | inertia under a mass override | departure: hpr scales all it covers | ADR-061 |
   | rounded and airfoil fin sections | departure: hpr's cited sections | ADR-062 |
   | roll inertia | explained by OpenRocket's fin shortcut; hpr keeps its exact integral | ADR-062 |
   | packed parts with no size, overrides on weightless parts | OpenRocket's rule | ADR-063 |
   | clusters | every tube weighed where it is; OpenRocket stacks them on the cluster's axis, so roll and pitch depart | ADR-075 |
   | fin fillets | OpenRocket's mass and centre; pitch apart by −0.64% to +0.43%, pinned: hpr's is the exact prism, OpenRocket's rule for fins unmeasured | ADR-096 |
   | pods | OpenRocket's mass and centre, and roll with its fin shortcut; pitch with one or two pods that have a length departs (0.3% to 1.1%); an override on an empty pod set is dropped | ADR-090, ADR-091 |
   | tube fins | OpenRocket's radius and mass; roll and pitch depart | ADR-098 |
   | parts hpr still keeps unread (parallel stages, some fin sets) | departure: kept whole, design marked reduced | ADR-058, ADR-064 |
   | stored results (L87) | references only when current and plausible | ADR-065 |

2. **L51 and L87 are owned by M2.2b.** Their lesson rows named only M2.2, so the check that a
   lesson's tests are live would wait for M2.2. Each row now names M2.2b first. Checking M2.2b
   off makes `cargo test -p xtask` hold both tests live. A row pointed at a test that does not
   exist fails it.
3. **M2.2 stays open for M2.2f, which takes L19 and L82 from M2.2e.** ADR-060 §1 gave both
   lessons to increment e, and M2.2e was checked off on the parent's bar without them. This
   supersedes that assignment. Their tests do not exist: `tube_fin_cp_within_0_25_cal_of_oracle`
   and `excused_cases_stay_in_the_census_statistics_against_both_references`. L19's bar is not
   met today. On the *Tube fin rocket* at rod clearance, hpr's margin is 0.79 calibres against
   OpenRocket's 1.87 (ADR-099). The two centres of mass there are 0.002 calibres apart
   (`validation/reports/openrocket-flights.json`), so the gap is the centre of pressure's:
   hpr's is 1.07 calibres forward of OpenRocket's, further than Loft's 0.9. M2.2f is done when the
   test asserts L19's 0.25 calibres and passes. If that cannot hold, an ADR gives the measurement,
   and the lesson's row names a test that pins the gap and says so, as L18's row does.

**Evidence.** `cargo xtask ork`, run on 2026-09-28 with OpenRocket's record unchanged, passes on
71 files, 51 distinct by content:

| quantity | within 0.1% | within 1% | median |
|---|---|---|---|
| mass | 60 of 71 | 68 of 71 | 0.001% |
| centre of mass (share of length) | 65 of 71 | 68 of 71 | 0.000% |
| pitch inertia | 41 of 71 | 56 of 71 | 0.065% |
| roll inertia | 10 of 71 | 29 of 71 | 1.686% |
| roll inertia, with hpr using OpenRocket's fin shortcut | 57 of 71 | 57 of 71 | 0.001% |

- Three files, two by content, are outside the mass or centre-of-mass threshold, each with a
  cause: a parallel stage kept unread, and fins weighed by hpr's section. ADR-060 had 17 files,
  12 by content.
- Roll inertia is outside 1% on 14 files even with OpenRocket's shortcut. By content they are 9,
  each with a cause: a mass override (5), a cluster's tubes (2), a part kept unread (1) and tube
  fins (1).
- Pitch inertia is held to no threshold (ADR-060 §4). The mass page says which of the 15 files
  outside 1% have a cause and which do not.
- The mass page's table is this run's, number for number.

**Consequences.** M2.2b is checked off. The next increment is M2.2f, then M4.1. Replacing any
departure above takes new probes, not a quiet change.


## ADR-102: Tube fins' centre of pressure measured against OpenRocket; L19's bar not met, the gap pinned (2026-09-28)

**Context.** Loft lesson L19 asks that hpr put a tube fin set's centre of pressure within a quarter
calibre of OpenRocket's; Loft's was about 0.9 calibres forward of it. M2.2f is done when a test
asserts that bar and passes, or, if it cannot hold, when an ADR measures the gap and L19's row
names a test that pins it, as L18's row does (ADR-101 §3). On OpenRocket's *Tube fin rocket* hpr's
centre is 1.07 calibres forward of OpenRocket's (ADR-099). ADR-099's account of where that comes
from, OpenRocket's slope and centre for the tubes, was a look at its per-component output that
was never kept. A search for a measured tube-fin centre of pressure found none: not in NACA or NASA
reports, Apogee's newsletters (issues 27, 119 and 335) or OpenRocket's own pull requests, whose
author wrote that "we won't really know until we can find a wind tunnel" (openrocket#1413).

**Decision.**

1. **OpenRocket's answers are kept as a record.** `validation/oracles/openrocket/tube_fin_aero.py`
   runs OpenRocket 24.12 as an external oracle. It writes 14 probe designs: one nose and body, with
   a tube fin set at the foot that varies one thing at a time. The variations are the tubes'
   length (25 to 300 mm), their count at a stated radius (3 to 8), their radius and their wall.
   The script also reads the *Tube fin rocket* from the jar. It records the slope `C_Nα` and the
   centre of pressure its Barrowman calculator gives each component at Mach 0.05, 0.3, 0.5, 0.6
   and 0.75. Each answer comes from a new load and a new calculator, because a calculator asked
   again after a part changes gives its cached answer. The record is
   `validation/fixtures/ork/openrocket-tube-fin-aero.json`.
2. **What OpenRocket does, from its output alone** (its source is GPL and was not read):
   - The tubes' slope is the same at every Mach number and grows with the count, the length and
     the bore.
   - On every probe it is 1.26 to 1.86 times hpr's, and 1.20 to 1.86 times `N π d²/A_ref`. That is
     the long-ring limit of `N` isolated thin rings of mean diameter `d` (Hoerner 1965, p. 7-13),
     which Weissinger's
     formula approaches from below as a ring lengthens.
   - Its slope per tube is the same, to 1e-15, for 3, 4, 5, 6 and 8 tubes of 6 mm. So it
     models no interference that changes with the count. Slender-body theory with the body
     included does predict one, and it falls as the count rises. A 2D apparent-mass solve run
     during review, not kept here and not checked (#234), puts three such tubes at 1.96 times as
     many isolated rings, four at 1.86, five at 1.72, six at 1.57, eight at 1.30, and six touching
     each other at 1.13, each at a gap of 0.005 radii and still rising as the gap closes.
     OpenRocket's slope is 1.73 times the long-ring limit on the 6 mm probes, and 1.62 on the
     touching one. So it is below that estimate for three to five tubes, and above it for six,
     eight and the touching tubes. At five the two are close: a finer solve, at 0.0005 radii, gives
     1.79, about 3.5% above OpenRocket. Nothing measured supports either code.
   - Its centre is a quarter of a tube's length aft of the leading edge up to Mach 0.5, and at the
     leading edge from Mach 0.6. So the *Tube fin rocket*'s own centre of pressure moves 0.73
     calibres forward between those two speeds in OpenRocket. OpenRocket's pull request #3235,
     merged on 2026-08-11 after 24.12, calls that jump a bug and fixes it.
   - Its pull request #3262 describes the quarter chord as the subsonic rule that conventional fins
     and tube fins share: a flat fin's rule, not a ring's.
3. **hpr keeps ADR-099's ring wing.** OpenRocket's centre runs against the evidence there is, and
   nothing measured supports its slope:
   - Fletcher's measured centre moves forward, as a share of the length, as a ring gets longer:
     0.355 of the chord at `A = 3`, 0.143 at `A = 2/3`.
   - Hoerner and Borst (*Fluid-Dynamic Lift*, 1985, p. 19-16) take the lift of the air turned
     inside an open tube as `2α` on its frontal area, "assuming that the turning takes place at or
     near the rim of the inlet". That assumption is the leading edge hpr's line runs to, which
     ADR-099 derived. They write that they "do not have suitable experimental results at hand"
     on the influence of an axial duct on a slender body's lift and moment. On the same page they
     treat Fletcher's thick `A = 1/3` ring as a ducted body, and find its measured slope
     "practically the same" as that treatment gives. Fletcher measured that ring's centre ahead of its leading
     edge.
   - hpr leaves out the body's interference that slender-body theory predicts (#234). With it,
     hpr's slope would rise. On the three-tube probe the gap would all but close, but only by
     cancellation: a larger slope placed at hpr's centre, 0.03 of the length, not OpenRocket's
     0.25. On the touching tubes it would fall from 1.04 to about 0.89 calibres. That is not
     measured either, and it is a separate change.
   - OpenRocket's pull request #1413 says its tube-fin method comes from "a paper cited in the
     code". hpr cannot see which without reading GPL source, and no measurement supports its
     numbers. Taking them to pass a code-to-code bar is what the first hard rule forbids.
4. **L19's row names the test that pins the gap:**
   `hpr_validate::openrocket::tests::tube_fin_cp_against_the_oracle_measured_and_pinned`. The test
   builds every probe with `hpr_io::ork` and `hpr_aero`, and holds three things to the record:
   - the geometry, and the nose and body, which agree;
   - OpenRocket's rule as listed above;
   - each gap, to 5e-4 calibres.

   Each of the 70 gaps, 14 probes at five Mach numbers, is pinned in a table.

   It also shows that the whole gap is the tubes'. Given OpenRocket's slope and centre for the
   tubes, hpr's rocket has OpenRocket's centre of pressure within 0.01 calibres, whatever the probe
   or Mach number. A change in either code fails it. Meeting L19 would too, and the row would then
   have to change.
5. **L82's test is live:**
   `hpr_validate::tests::excused_cases_stay_in_the_census_statistics_against_both_references`.
   - Every number the reports compare is a census row, with the report's own difference: the
     harness's metrics, the six OpenRocket metrics of every example and library flight, and each
     log's apogee and climb. A scored one is never withheld, the flights with written causes
     (ADR-073) among them, and a real flight with a written explanation stays a miss.
   - Every group's apogee spread and bar counts are over all its compared rows.
   - A harness rocket that RocketPy flew, and whose team logged it, counts against both. On a
     flight whose miss is explained, the two references are more than 5% apart.
   - Mutations of the census each fail the test: withholding the misses of any one scored
     metric (apogee, max speed, margin, climb), withholding any one mass or centre-of-mass metric,
     dropping misses from the spread, and dropping them from the bar counts.
   - The results stored inside `.ork` files, which M2.2 also names as a reference, never enter the
     census (ADR-065), so this test does not cover them.
6. **M2.2 closes.** Its *done when* was met in M2.2e10 (ADR-100), and its last two lessons are now
   live.

**Evidence.** `cargo test -p hpr-validate --lib tube_fin_cp_against_the_oracle` on the record.
hpr's rocket centre less OpenRocket's, in calibres:

| probe | Mach 0.05 | 0.3 | 0.5 | 0.6 | 0.75 |
|---|---|---|---|---|---|
| six tubes touching the body and each other, 75 mm long (the *Tube fin rocket*'s build) | −1.043 | −1.050 | −1.065 | −0.363 | −0.387 |

- On that probe at Mach 0.05, OpenRocket's centre alone moves hpr's 0.495 calibres aft, and its
  slope alone 0.532.
- Up to Mach 0.5, no probe is within the bar: the gaps run from 0.420 to 2.998 calibres. From Mach
  0.6, 5 of the 28 are within it: the 25 mm and 300 mm tubes at both speeds, and the 3 mm wall at
  Mach 0.6. Those five probably meet the bar only because of the jump that #3235 fixes; no
  OpenRocket with that change was run.
- On the *Tube fin rocket* itself at rod clearance (Mach 0.056), hpr's centre is 1.075 calibres
  forward of OpenRocket's (`openrocket-flights.json`). The record's own answer for the example is
  1.4e-5 m from the flight record's.

**Consequences.**

- The flight report's 1.08-calibre margin gap on the *Tube fin rocket*, accepted in writing with
  ADR-099, stands; the guide's tube-fin section now quotes the record.
- [#234](https://github.com/nrdptel/hpr-sim/issues/234) holds the body's interference, and the
  first-order wording of ADR-099's "sums to zero", which the guide and the module doc now qualify.
- [#228](https://github.com/nrdptel/hpr-sim/issues/228) stays open for a measured source.
  Weissinger's lifting-surface coefficients for thin rings (Bagley, Kirby and Marcer, ARC R&M 3146,
  1961) are a lead for a thin ring's centre down to `A = 1/2`; they are not checked here.
- The next milestone is M4.1.

## ADR-103: The builder API wraps the crates' own types, with no default materials (2026-09-28)

**Context.** M4.1 asks the `hpr` crate for a builder API in the manner of RocketPy's
(`Environment`, `Motor`, `Rocket`, `Flight`), trait-based custom models, at least 4 examples and a
rustdoc guide. Building a rocket in code took about 140 lines of struct literals in
`own_rocket.rs`, since `hpr_design` has only public fields and no constructors. The milestone is
more than one session's safe share, and its two halves are separable: the builder, then the
models a user can put in hpr's place.

**Decision.**

1. **M4.1 is split.** M4.1a is the builder, with Loft lesson L95; M4.1b is a drag-model trait the
   flight calls in place of hpr's drag, the custom aero example, two more examples and the rustdoc
   guide. M4.1's own bullets close with M4.1b.
2. **The builder wraps, never replaces.** Each type hands over what it makes: a `Rocket`'s
   `design()` is the `hpr_design::Rocket` a design file holds, a `FlightBuilder`'s `simulation()`
   the `hpr_sim::Simulation` it runs, an `Environment`'s `sim()` the `hpr_sim::Environment`. What
   the builder doesn't offer (staging, clusters, pods, shifts, user events) stays reachable, and
   no physics is written twice. The crates are re-exported by name (`hpr::hpr_sim`), not renamed.
3. **No default materials or walls.** Every part names its material (`rocket::material(id)` finds a
   built-in one) and a hollow part its wall: each default would be a guess at the rocket's mass. The
   defaults the builder does take change the drag or the placing, not the mass: fins square-edged,
   every surface the design's default finish (mass-production paint), a part flush with its tube's
   aft end, a motor tube with no overhang, a mass a point until packed. A position places a
   packing's end, so packing a mass moves its centre by half its length unless it is placed by its
   middle: 22 mm of the example rocket's centre of gravity for its 15 cm recovery bay.
4. **Order is the tree's.** Body parts stack from the nose in the order added; fins, the motor tube
   and masses go on the last body tube. A nose after any body part, fins before any tube, a second
   motor tube and a motor with no tube are refused with `Error::Order`, whose `Order` says which,
   so a binding can match it. The nose's base radius is the rocket's diameter, a tube's the aft
   radius of the part before it unless given; the nose's shoulder's radius is left automatic, to
   fit the tube behind it. Fins take a named `FinPlanform`, not a list of lengths that could be
   given in the wrong order. Ids are the part's kind, numbered from the second (`tube`, `tube-2`).
5. **One motor, lit at launch.** `set_motor` replaces the design's one configuration, named after
   the designation. `Motor::from_catalog` takes only motors with a bundled curve, and refuses a
   name that matches several (`I175` matches two) rather than taking the first. The delay is set,
   never read from the designation.
6. **Weighing runs the flight's checks.** `Rocket::assemble`, and so the mass properties and the
   margin, refuse with `Error::DesignChecks` what `Simulation::new` refuses: a program never shows
   a margin for a rocket the flight won't fly. The same findings reach a flight as
   `Error::Sim(SimError::DesignChecks)`, from the simulation beneath. A flight's settings can set
   `accept_design_errors` to fly such a design anyway; the builder's weighing has no such switch.
7. **Parachutes go on the rocket**, as RocketPy's `add_parachute` has it: a `Device`, drag only;
   its mass is a `Mass` the user adds.
8. **An elevation is both heights.** `Environment::new` takes the site's elevation as its height
   above mean sea level and above the ellipsoid, as `hpr_sim::Environment` does with no undulation;
   `from_sim` takes an environment built with one.
9. **L95, degenerate designs.** The builder checks each part's numbers as it is added (finite,
   positive or not negative) and names each in `Error::Domain`; fins go through
   `FinSet::validate`, a nose's and a transition's shape through `Profile`. The rail's numbers are
   checked when the flight is set up, in the degrees they were given. Put straight
   into a design, a zero-radius tube, a NaN length or span, zero fins, a negative mass and a
   negative mass override are refused by `Simulation::new`'s design checks, each by its part; a
   rocket with no fin set flies, tumbling, to finite numbers.

**Evidence.** `cargo test -p hpr`: the builder's rocket, assembled and flown in `own_rocket.rs`'s
conditions, has that example's mass properties at six times, the same dry mass properties, and a
`FlightResult` equal to the hand-built tree's, bit for bit
(`the_builder_makes_the_rocket_own_rocket_builds_by_hand`); `degenerate_designs_error_or_stay_finite`
pins each refusal by its part and quantity; `packing_a_mass_moves_its_centre_unless_placed_by_its_middle`
pins the packing's shift to 1e-12 m against `m L / 2 M`; `weighing_refuses_what_flying_refuses`
refuses a 54 mm motor in the 29 mm tube both ways.

**Consequences.**

- The facade's first two examples, `build_and_fly` and `motor_choice`, run in CI; the guide's page
  *The builder* explains them.
- `FlightBuilder` borrows its rocket and environment. The Python and WebAssembly bindings
  (M4.3, M4.4) can't hold a borrow, so they will want an owned launch description; that is theirs
  to add.
- A design fault reaches the caller as `Error::Design` from weighing and as
  `Error::Sim(SimError::Design)` from a flight, the crate each came through.
- Custom drag waits for M4.1b. Until then a drag table (`Simulation::with_drag_table`) through
  `FlightBuilder::simulation` is the way to put another source's drag in hpr's place.

## ADR-104: A drag model replaces the zero-lift drag only, as a drag table does (2026-09-29)

**Context.** M4.1b asks for trait-based custom models: a drag model the flight calls in place of
hpr's own, an example that overrides a built-in model through the trait, and a rustdoc guide.
The wind and the atmosphere were already traits (`hpr_atmos::Wind`, `Atmosphere`), taken by
`Environment::with_wind` and `with_atmosphere`; the drag was a table or hpr's buildup. A drag
table already had a path through the model and the flight, pinned by tests: the template.

**Decision.**

1. **`hpr_aero::DragModel` gives the zero-lift drag coefficient only**, on the rocket's reference
   area, from a `DragQuery`: the flow, the drag conditions (Reynolds number per metre, whether a
   motor is thrusting, as the buildup reads them) and hpr's own buildup at that flow
   (`DragQuery::buildup`, which calls the new `AeroModel::buildup_drag`). As with a table, the
   flight scales it by `f(α)` for the angle of attack; the normal force, centre of pressure, roll
   and damping stay hpr's, so a rocket's margin doesn't change. A whole-force model (normal force
   and moments) would have to replace the local-flow damping too (ADR-032's reasoning), and is
   left until someone needs one.
2. **A model and a table replace each other**: the last set is the one flown, on
   `AeroModel`, `Simulation` and `FlightBuilder`. Neither is added to the other.
3. **A model is held as `Arc<dyn DragModel>`**, `Debug + Send + Sync` as a `Wind` is.
   `with_drag_model` takes a model and shares it; `with_shared_drag_model` takes an
   `Arc<dyn DragModel>` as it is, which the facade's clonable `FlightBuilder` uses, so every
   flight of one builder shares one model. There is no blanket `impl DragModel for Arc<T>`: it
   would let an `Arc` be wrapped in a second one, silently. Two `AeroModel`s are equal when they
   hold the same model (`Arc::ptr_eq`). A serialized `AeroModel` names its model by its `Debug`
   text, and leaves the field out when there is none, so no committed output changes.
4. **A query is narrow.** `DragQuery` gives the flow, the conditions as the buildup reads them,
   the reference area, the length and hpr's buildup (whole and by component), but not the
   `AeroModel` itself, whose `drag` would ask the model again without end. Its constructor is the
   crate's own, so fields can be added without breaking a model; a model is tried by giving it
   to an `AeroModel`.
5. **Checks.** A coefficient that is negative or not finite is refused with
   `AeroError::Domain { what: "zero-lift drag coefficient from a drag model" }`, and a Mach
   number that is negative or not finite before the model is asked. The model's own errors,
   including the buildup's refusals it passes on, come back wrapped in the new
   `AeroError::DragModel`, so a caller can tell them from hpr's. Like a table, a model accepts
   any finite Mach number; the flight's normal force still ends at Mach 5.
6. **A model is the whole stack's**, as a table is: a flight with a powered separation refuses it
   at the separation, since the sustainer's own model is built from the design.
7. **Examples.** `custom_drag` (hpr's drag scaled, and a curve against Mach number with power-on
   and power-off columns, invented), `custom_wind` (a veering wind through the `Wind` trait) and
   `fin_sizing` (a design study as a loop) join `build_and_fly` and `motor_choice`: five builder
   examples. The three open the parachute at apogee, so the motor's charge doesn't cut the climb
   short and change the apogee they compare. The guide in the reference is `hpr::guide`, five chapters of docs with doctests;
   the site's page is *Models of your own*.

**Evidence.** `cargo test -p hpr-aero custom`: a model handing back the buildup gives the
buildup's `C_D0` and `C_A` bit for bit over Mach 0 to 4.9 and 0° to 120°; a constant model equals
a constant table to Mach 7; the model sees the conditions as read under
`with_full_base_drag_under_power`; each refusal pinned by its `what` or value, a model's own
error by its wrapping; two models holding one shared model are equal; a serialized model names
its drag model. `cargo test -p hpr`
(`a_drag_model_is_flown_in_place_of_hprs_drag`): the builder's rocket flown with that model has a
`Flight` equal to the one without, and a constant model's `FlightResult` equals the constant
table's. `staging_refuses_what_it_cannot_fly` refuses a model at a powered separation, and fails
when `with_drag_model` doesn't mark the override (mutation checked). A wind that isn't finite
stops the flight, named by the Reynolds number's check (`a_wind_that_is_not_finite_stops_the_flight`).

**Consequences.**

- M4.1 closes: the examples run in CI, rustdoc builds with no warnings, and `custom_drag`
  overrides hpr's drag through the trait.
- An atmosphere of your own has no test of its own through the facade yet, and no example.
- `AeroModel` is no longer `UnwindSafe` or `RefUnwindSafe`, as `hpr_sim::Environment`, which
  holds an `Arc<dyn Wind>`, already wasn't. A caller that catches a panic around one wraps it in
  `AssertUnwindSafe`.
- hpr doesn't check a wind's value where it reads it; a wind that isn't finite is caught later,
  by the drag's Reynolds number while the rocket climbs. The site says so, and #237 tracks a check
  of its own.
- The Python and WebAssembly bindings (M4.3, M4.4) will need a way to call back into their
  languages for a drag model; that is theirs to design.

## ADR-105: The command line's surface: every command registered, JSON by schema, a generated table (2026-09-29)

**Context.** M4.2 asks for `hpr sim|validate|convert|motors|mc|optimize|compare|analyze|diagnose`,
stubs being fine for a command whose milestone hasn't come, `--json` everywhere, shell
completions, and the README's command table generated from the registered commands (Loft lesson
P10: Loft's README claimed importers that didn't exist). `hpr-cli` was a stub. Most commands have
little under them yet: flying a `.ork` from the command line has to join `hpr-io`'s reader to the
facade (its parachutes are not flown yet), `hpr-flightdata` is empty (its readers are M7.1), and
`convert` has only the motor formats to convert. That is more than one session.

**Decision.**

1. **Split into a to d.** M4.2a: the command surface, `hpr motors` and `hpr completions`. M4.2b:
   `hpr sim`. M4.2c: `hpr validate` and `hpr convert`. M4.2d: `hpr analyze`, with
   `hpr-flightdata`'s first log reader. M4.2's own *done when* is unchanged and closes with d.
2. **Every command is registered from the start.** A command whose milestone hasn't come accepts
   any arguments and refuses with exit status 3 and that milestone, so `hpr --help`, the
   completions and the README list the whole tool without claiming it works. `weather` (M5.2),
   which `ARCHITECTURE.md` lists, joins the roadmap's nine; `completions` is the tenth. `mc` is
   M6.1, `optimize` M6.2, `compare` (a log against its simulation) M7.3, `diagnose` M7.4.
3. **Exit codes:** 0 done; 1 an input missing, unreadable or refused; 2 a wrong command line
   (clap's own); 3 not available yet. `--help` and `--version` are answers, status 0.
4. **`--json` prints exactly one document on standard output**, success or failure, and nothing
   on standard error; a failure is an `ErrorDocument` whose `kind` (`input`, `usage`,
   `not_available`) matches the status. `--json` is looked for before parsing, up to a `--`, so a
   usage error is JSON too. clap's built-in `help` command takes no `--json`. The registry, not
   the dispatch, decides which commands refuse.
5. **The output types are the CLI's own** (`hpr_cli::output`), not the libraries', so a published
   schema changes only when the command's output does. Each derives `schemars::JsonSchema`
   (draft 2020-12); `cargo xtask cli` writes them to `schema/cli/`. Units are SI and named in the
   field, except `motors list`, which keeps the catalog's millimetres and says so in its names.
6. **One registry, generated tables.** Names and summaries come from clap's command tree;
   `registry::availability` adds what a command reads and writes, or its milestone. A format
   listed is taken from the enum the command dispatches on (`MotorFile::ALL`,
   `clap_complete::Shell`), so the table can't list one the code doesn't read. `cargo xtask cli`
   writes the table into `README.md` and `docs/cli.md`, and re-runs the page's examples in-process;
   an example must exit as its marker says (`exits 3`, else 0), takes no quotes or paths, and a
   block missing its end is refused rather than spliced up to the next block's.
7. **`hpr motors show` works a motor's figures out from its curve** with `hpr_motor` (the curve
   the simulator flies), and for a catalog motor sets ThrustCurve.org's stated figures beside
   them; `list` repeats the catalog's stated figures. A name several motors share shows each. A
   `--manufacturer` the catalog doesn't know is refused, listing its makers: `CTI`, the files'
   abbreviation, would otherwise list nothing, as if Cesaroni had no motors.
8. **A closed pipe** (`hpr motors list | head`) ends with status 0 and says nothing: the reader
   chose to stop, and whether the write fails depends on the pipe's buffer.

**Evidence.** `cargo test -p hpr-cli`: 15 tests run the built binary with `assert_cmd` and check
every `--json` document against the committed schema. Every planned command refuses with 3 and
its milestone, as text and as JSON, whatever its arguments; `motors list` is the catalog, and its
three filters narrow it; a filter that can't mean anything, `--manufacturer CTI` among them, is
exit 1; `--json` after `--` is an argument; `motors show` gives the
library's own figures for a name, a shared name, each file format `MotorFile::ALL` lists, and a
two-motor `.eng` with a `0` delay and its warning; missing, broken and non-UTF-8 files are exit 1;
every shell's completions; six wrong command lines are exit 2. Over the whole catalog, `motors
show`'s total impulse, average thrust and burn time are within 1% of the stated ones, and its
peak thrust from 16.7% below to 2.1% above, the range the guide quotes. Unit tests hold the registry to
clap both ways and pin the exit codes and the closed pipe. `cargo test -p xtask cli` fails when
a schema, a table or an example is stale.

**Consequences.** New dependencies: `clap` (derive), `clap_complete`, `schemars`; `assert_cmd`
for tests. A command's milestone makes it available by moving its name out of
`registry::PLANNED`, adding its output type to `output::schemas`, and running `cargo xtask cli`.
`ARCHITECTURE.md`'s `hpr-cli` row lists `analyze` and `completions`.

## ADR-106: `hpr sim` flies the library's flight, the stack whole, from a stated launch (2026-09-29)

**Context.** M4.2b asks for a `.ork` or hpr design flown with a catalog motor or a motor file,
its summary printed and its recording exported, and is done when a public `.ork` flown by
`hpr sim` gives the library's flight bit for bit, with its JSON valid. No public `.ork` in the
repository flies with hpr's own motors: the Loft demos and the pod and rod probes name AeroTech
motors the bundled catalog doesn't hold. The library flies a `.ork`'s powered separation only
through `hpr_sim` (`crates/hpr/examples/ork_two_stage.rs`), and flies none of its recovery
devices (ADR-056). A configuration left out of a `.ork` (`LeftOut`) names only the first reason
on `NotFlown`'s list, and a missing curve comes before an incomplete airframe.

**Decision.**

1. **The flight is the facade's.** The design, cut to the configuration flown, becomes
   `hpr::Rocket::from_design`, and `hpr::Flight::builder` flies it. The rail's angles are set only
   when given, so a default launch is the builder's own rail, bit for bit. A recording is a
   `Recorder` of every channel every `--interval` s (0.01 by default) and at each event; it
   samples the steps' dense output, so recording doesn't change the flight.
2. **The launch is stated, and printed.** Without options the site is at 0° N, 0° E and 0 m, the
   rail vertical, frictionless and 1.5 m long (the builder examples' rail), the air calm and the
   1976 standard atmosphere. No site is neutral; every output states the one used, and the guide
   says to set `--elevation` (1,400 m lifts the example's apogee about 8%, a test pins it). A
   `.ork`'s stored launch conditions are not read. `--wind` is one speed at every height; real
   weather is `hpr weather` (M5.2).
3. **The stack flies whole, with no recovery device**, and the notes say so. The fall from apogee
   then runs on small-angle aerodynamics far outside their range, and settles in a tail-first
   glide on the synthetic 54 mm design (145° angle of attack, #241), so the notes, the text's
   landing line and the schema's `landing` all say the landing is not a prediction. The summary
   still carries it, field for field as the library gives it. A peak the fall sets is no
   prediction either (the dual-deploy demo, flown with `--motor H54` and no parachute, is fastest
   as it reaches the ground): each peak carries `after_apogee`, and the text marks such a top
   speed or Mach number "in the fall: not a prediction". The GeoJSON and KML maps get the summary
   without its landings, so no pin reads as a landing place. Recovery and staging: #240.
4. **Refused rather than flown as another rocket:** a `.ork` configuration with a powered
   separation (`MotorConfiguration::staging`); a motor lit at its stage's separation; a `.ork`
   rocket not read exactly as written, asked of the new `hpr_io::ork::airframe_not_as_written`
   (that rule stood inside `ork::design`, where a missing curve hid it); design-check errors
   unless `--accept-design-errors`, which lists them in the notes; a hybrid motor in a `.rse`,
   whose `Type` says so (a `.eng` doesn't say, so a hybrid's `.eng` flies as a solid, and the
   guide says so).
5. **`--motor` puts one motor in one mount, lit at launch**: a catalog name through
   `Motor::from_catalog`, a `.eng` through `Motor::from_eng`, or a `.rse` through the new
   `Motor::from_rse`, which refuses a hybrid by its `Type`. It goes into the chosen
   configuration's one mount, or `--mount`, or the design's only mount; a cluster's tubes and a pod
   set's pods each carry one, and the output counts them. A `LeftOut` names only its first reason,
   so on a `.ork` each reason no motor of the user's own can fix is asked of the file itself:
   the airframe, a powered separation, a stage switched off, a motor in an unread part, more than
   one stage (hpr can't yet tell when they would separate with another motor), or a
   configuration left out for anything but its motor. The same is asked when the file has no
   configuration at all. A configuration with motors in more than one mount is refused.
6. **The output mirrors the library's summary field for field.** `sim.schema.json` holds every
   field of `FlightSummary`, each event's time, height and speed, the file's configurations with
   their names, the motors with their mount's name and count, the exports, the notes, and the
   warnings of the readers and the design's checks. Paths are shown by file name, so an output
   doesn't depend on where it ran. `--export` refuses a file the run reads, a file named twice
   and a missing folder, before the flight. `--interval` is at least 0.001 s: a finer recording
   of a long fall fills memory.
7. **An example may name a file of the repository** (amending ADR-105 §6), given from the root.
   `cargo xtask cli` reads it from the root wherever it runs. It must be of plain path parts
   (no `..`, no root, no drive), a file, and tracked by git, so nothing gitignored under `refs/`
   reaches the page; an output that shows the root's own path is refused.

**Evidence.** `cargo test -p hpr-cli`, 35 tests. `sim_flies_a_public_ork_as_the_library_does`
flies `validation/fixtures/ork/pod-flights/pods-none.ork` (the repository's own probe) with
`--motor H54`. It flies the same rocket through the facade as a program would, and finds every
summary field and every event's time, height and speed equal to the bit (`to_bits`). The
`--export`ed CSV equals the library's `export::csv` byte for byte, and the document validates
against `sim.schema.json`. The same holds for:

- the probe with the H54 written into the file as its own motor;
- the probe stripped of its configuration, flown with `--motor H54`;
- the F15's `.rse` and the H54's `.eng`, with all five export formats written and the Parquet
  equal to the library's;
- the synthetic 54 mm design from Spaceport America's site on an 85° rail in a 5 m/s wind.

Leaning the rail east puts that design's apogee east of where leaning it west does.
`sim_refuses_what_it_cant_fly`, `sim_refuses_a_motor_it_cant_place` and three more tests pin each
refusal above by its message, most on the pod probe edited as text, and the pod probe's two H54s
are counted. The one refusal no test reaches is a motor in a part hpr doesn't read: no public
file has one. `sim_marks_what_the_fall_sets` pins the fall's marks on the dual-deploy demo, and
the motor-file test finds only the path on both maps. `hpr-io`'s
`an_incomplete_airframe_is_found_behind_a_missing_curve` pins the airframe check behind a
`NoCurve`; `hpr`'s motor test pins `from_rse`'s masses and its hybrid refusal.

**Consequences.** `hpr-cli` enables `hpr`'s `parquet` feature, which adds no dependency. The
README's and guide's tables list `hpr sim` as available; its reference is the guide's page,
`docs/cli.md#hpr-sim`. Issues #240 (recovery and staging in `hpr sim`) and #241 (the
no-recovery descent) hold what is left.

## ADR-107: `hpr validate` shares the project's own validation check; `hpr convert` translates `.eng` and `.rse` by the format notes (2026-09-29)

**Context.** M4.2c is done when `hpr validate` fails where `cargo xtask validate --check` does,
and `hpr convert` round-trips `.eng` and `.rse` motor files. The check (the report reproduced,
every scored metric in tolerance, the census held, ADR-084) lived in `xtask`, the repository's
own tool, which a user's `hpr` can't call. `hpr_motor` wrote each format back to itself, but
converting between them was only sketched in `docs/format/rse.md` ("not implemented yet"). The
formats weigh in different units: `.eng` in kg, `.rse` in g, and multiplying by 1000 rounds in
binary for about a quarter of four-decimal masses (`0.0041 × 1000 = 4.1000000000000005`).

**Decision.**

1. **One check, in `hpr-validate`.** `committed::check` holds a run to the committed reports and
   the census, and `Checked::lines` and `summary` give what is printed. `cargo xtask validate
   --check` and `hpr validate` both call them, so they pass and fail together by construction.
   On every path that reaches the reports, xtask prints byte for byte what it printed before the
   move; the reviewer checked nine spoiled copies. Two error paths changed: an unreadable
   `latest.md` or `.json` is reported with the census's result rather than alone, and a report
   that isn't JSON says so rather than "reading". `hpr-cli` depends on `hpr-validate` (the crate
   map's row changes); `census --accept` stays in xtask.
2. **`hpr validate` needs a copy of the repository** (`--root`, the current folder by default),
   runs every case in the lock (no `--fast`: a partial run can't reproduce the whole report) and
   writes nothing. The lock holds the 20 RocketPy cases; the OpenRocket and real-flight reports
   are checked only against the census, as their oracles need OpenRocket and private files. A
   failing check prints its own document, `passed: false` with the reasons, and exits 1, even
   when standard output is closed; it is a result, not an unreadable input, so it isn't an error
   document. `hpr` must be built from the copy's commit. A release build reproduces the report
   too (the check compares the digits the platforms share).
3. **`hpr convert` converts motor files**, by `hpr_motor::convert`, in the pure core. To `.rse`
   it fills what ThrustCurve.org's `.rse` files give, as the format notes counted them (the
   origin, `Itot`, `peakThrust`, `burn-time`, `avgThrust`, `m` by the impulse fraction, `cg` at
   half the length, the flags, `massFrac`, `Isp`), and `Type="unspecified"`, which RockSim's
   guide requires. To `.eng` it drops those, named in a warning; hpr uses none of them for a
   solid motor. Delays trade `-` for `,` and `P` for `1000`. A name or maker of several words is
   joined by `_`, also when rewriting `.eng`: OpenRocket 24.12 refuses a `.eng` header of eight
   fields (`motor_files.py` checks it), and no real file has one. A hybrid and an engine without delays it can
   read are refused, the latter until `--delays` gives them, which fills only those. A catalog
   motor converts from its bundled curve with the catalog's size and masses, the ones hpr flies,
   warned where the curve's header differs (10 of the 32 differ by over 0.1%); a header that is
   already the mass hpr flies (`g × 1e-3`) keeps its digits, and in a `.rse` file `massFrac`,
   `Isp`, `m` and `cg` are rescaled to the replaced figures. The delays stay the curve's. The output is replaced if it
   exists, never when it is the input. Design files are not converted (M3.2).
4. **Masses move by the decimal point.** A mass is printed in its shortest digits, its exponent
   shifted by three and read back, so a mass of up to 15 significant digits in a double's normal
   range comes back bit for bit (such decimals never share a double; a proptest pins it). One of
   16 or 17 digits may fall between the other unit's doubles, and a warning says so; one with no
   finite, nonzero value in the other unit is refused.
5. **What "round-trips" means, measured.** The thrust curve, the size and the masses come back
   bit for bit both ways. Other text may come back respelled (delays, names and makers of
   several words, the origin point, comments), each read the same or warned. After one
   conversion a file is a fixed point, byte for byte, but for `.eng` delays of `-`, which name
   none: `.rse` leaves them out, and converting back needs `--delays`. Over the 32 bundled curves: all 29 `.eng`
   files come back whole except two whose delays are respelled (`p`, `1000` to `P`) and two with
   a 17-digit mass, each warned; the 3 `.rse` files keep every shared value but one maker,
   joined by `_`.
6. **Checked by another reader.** `validation/oracles/openrocket/motor_files.py` converts the 32
   bundled curves with `hpr convert` and loads both files in OpenRocket 24.12: it opens all 32
   and reads 29 as it reads the originals, comparing name, maker, type, size, masses, curve and
   delays. Two `.rse` files' type (reloadable, single-use) reads as unknown from `.eng`, which
   can't say it; one `.eng` delay `1000` reads as no delay there and as plugged from the `.rse`,
   as hpr reads both. RockSim is not checked.
7. **The guide's examples may write files.** An argument with no `/` ending in an extension of
   letters (`F15.eng`) goes to a scratch folder of the call's own, so `cargo xtask cli` never
   writes into the repository, and the page shows that folder as `<the scratch folder>/`.

**Consequences.** M4.2c's two commands leave `registry::PLANNED`; the README's and the guide's
tables list them as available; `schema/cli/` gains `convert.schema.json` and
`validate.schema.json`. The facade's `hpr::Motor::from_rse` and the catalog's curve reader still
turn grams into kilograms in binary, so a motor read from a converted `.rse` may differ from its
`.eng` in a mass's last bit; nothing claims them equal. `hpr validate` output lines change
whenever the report does, so the guide describes them in outline instead of quoting a run.

## ADR-108: A flight log read alone: PerfectFlite's `.pf2` first, heights after a running median, an invented log in CI (2026-09-29)

**Context.** M4.2d is done when `hpr analyze` is tested on a log with no design file present, its
JSON validating. `hpr-flightdata` was an empty skeleton; the importers are M7.1 and the readings
M7.2 (ADR-046). M4.2d needs one reader and enough readings for the command to be worth running.
Debrief's notes (`docs/research/debrief-*.md`) give the formats, the thresholds and the
provenance design. Its twelve public fixtures come from publicly shared flights, but their
upstream terms aren't recorded; ADR-046 said they "may appear in examples and doctests".

**Decision.**

1. **The first reader is PerfectFlite's `.pf2`** (`hpr_flightdata::perfectflite`): plain text, a
   barometer only, and a public Pnut log that states its own apogee, which makes it the cheapest
   format with a real cross-check. Units from Debrief's reading of exported files (feet, ft/s,
   °F); the columns from the `Data:` line where Debrief assumed them; a stated height not marked
   in feet refused rather than guessed; SI at the boundary. The record, `log::FlightLog`, is the
   one every later reader fills.
2. **A first set of readings** (`hpr_flightdata::readings`): liftoff, apogee and the time to it,
   the top speed in the climb, landing and the mean descent rate. The top acceleration is
   withheld for a log with no accelerometer. Each reading is `Reading::Read` with a `Source`, or
   `Reading::Withheld` with a `Reason` code and a sentence carrying the log's numbers, the
   non-optional field Debrief's notes recommend. Debrief's corpus-set thresholds are taken as
   they are (3 m, 2 m and 5 m for a second, 4,000 m/s, 20%) and said to be corpus-set, with no
   citation. The landing adds one bound of hpr's, a fall from rest in vacuum, which only drag can
   slow: the height lost from apogee to the landing sample may not come down faster, allowing one
   step of the altitude's rounding in the height and one sample for the apogee's time. A test
   sweeps a vacuum hop's apogee through a foot at 10 to 100 Hz and fails without either
   allowance. The pad is the median of the raw altitude before it first rises
   `PAD_RISE_M` (1 m, hpr's choice), so one jittery first sample doesn't set it. A pad more than
   3 m from the logger's zero withholds liftoff (`starts_off_the_pad`), and with it the top speed
   and the landing (`needs`), which are read against the pad. A record whose channels differ in
   length from its clock, or whose times don't increase, is withheld whole (`bad_record`): the
   reader never builds one, but a program can. A log sampled so fast that the 0.3 s window would
   hold more than 1,000 samples either side (`MAX_MEDIAN_HALF_WINDOW`, over about 6.7 kHz) is
   withheld whole too (`sampled_too_fast`): the median, kept sorted as it slides, costs the
   record's length times the window's, which a far finer clock would make grow with the square of
   the file's length.
3. **Heights after a running median, not a Hampel filter.** Debrief despikes with a Hampel filter
   (0.3 s, threshold 4). On the public Pnut log the trace dips just before the ejection pulse and
   stays lower after it, and those samples widen the pulse's own window's spread until the filter
   keeps it: the Hampel-filtered peak is the pulse's 1,028 ft against the 1,009 ft the logger
   states. The running median (the Hampel filter at threshold 0, Pearson et al. 2015, §2) removes
   any pulse up to half its window wide and reads a noise-free coasting peak low by at most
   `g ((⌈K/2⌉ + ½) Δt)² / 2`, 0.077 m at 20 Hz with `K = 3`, held by a property test. It reads
   1,010 ft.
   `filter::hampel` is kept to show the difference in tests. The apogee's time is the middle of the
   run of samples at the filtered peak, as the one-foot resolution leaves it flat: on the invented
   flight the first sample of the run reads over 0.2 s early, the middle 0.02 s late.
4. **An invented log in CI; the real one only where fetched.** CLAUDE.md's rule on third-party
   data of unclear terms outranks ADR-046's line, so no Debrief fixture is committed.
   `validation/fixtures/logs/synthetic-pnut.pf2` is a flight made up for the tests (boost at
   50 m/s², a drag-free coast, drogue and main at fixed rates, an ejection pulse of the real one's
   shape), written by `hpr_flightdata`'s own test code and held to it. Every reading is checked
   against its closed form within a bound worked out from the rounding and the filter. The public
   Pnut log is read by a test that runs where `refs/` has it, which holds the numbers the docs
   quote: stated 1,009 ft, read 1,010 ft, pulse 1,028 ft, liftoff 0.15 s, top speed 257 ft/s. In
   CI it prints that it skipped, but the harness still lists it as `ok`, so the docs say plainly
   that this check isn't in CI.
5. **`hpr analyze <log>`** reads a `.pf2` by its extension or a first line naming PerfectFlite,
   and refuses anything else, naming M7.1. Its JSON is `output::Analyze`, each reading tagged
   `status: read | withheld` (`schema/cli/analyze.schema.json`). The done-when's "no design file
   present" is tested literally: the log copied alone into an empty folder, `hpr` run there.

**Consequences.** M4.2 closes. The readings are checked on one invented flight and one real
one; the private corpus waits for M7.1, and each leg's descent rate, Mach number and burnout for
M7.2. A barometric altitude is printed as the logger converted it, uncorrected for the day's air.

**Not chosen: commit Debrief's public fixtures**, as ADR-046 allowed. Their upstream terms are
unrecorded, and the invented log checks more (a closed-form truth) in CI.

**Not chosen: the Hampel filter with a physical bound on climb rate.** It would need a speed to
bound by, which is what the pulse corrupts; the median needs nothing.

## ADR-109: M3.2 split, and a `.ork` written from the design (2026-09-29)

**Context.** M3.2 is done when a corpus design read by hpr and written back out loads in
OpenRocket 24.12, and OpenRocket flies the written file within 0.5% of the original's apogee. The
reader keeps the document whole (ADR-051) and, beside the design, whatever hpr does not model in
`x-openrocket` (ADR-058). `ARCHITECTURE.md` says every exporter maps out of the `hpr-design` model.
Loft's exporter lost plugged delays, per-configuration ignition, conditions, freeform fin points,
cluster scale and rotation, and every digit past the sixth (L67, L68).

**Decision.**

1. **Split in two.** M3.2a writes the file and proves, in Rust, that it reads back as the same
   design; M3.2b runs the OpenRocket oracle on the written files, which is the milestone's own
   two bullets.
2. **Written from the design, not the kept document.** `hpr_io::ork::export` builds the document
   from the `Design` alone: each writer writes exactly the tags its reader asks for, and the
   splice (`export/kept.rs`) puts back every kept part, section, tag and attribute. A design built
   in code, or edited after reading, is written the same way. `cargo xtask ork` holds it: every
   corpus design is written, read back and compared, and written again from what was read.
3. **Tags and sections count among their own name.** A kept tag's path now says it was the k-th
   `<appearance>` of its element, not the k-th tag of any name (a change to M3.1's paths); one
   that names a configuration counts among those naming the same one,
   `@deploymentconfiguration(b)[0]`, since the writer lists configurations in the design's order,
   not the file's. OpenRocket reads no meaning into tag order, with one exception below, and
   counting by name lets the splice put a tag back without knowing where it stood; a part still
   counts among all parts, since their order is where they stack.
4. **A value the reader drops is kept.** A reader that asks for a tag and then drops what it says
   (a rail button's screw height, a drag override, a ring of three, a finish word hpr doesn't
   know) marks it unasked (`reads::forget`), so `x-openrocket` keeps it, and the kept copy is
   written in place of whatever the design would say. Reading the written file raises the same
   warning, and OpenRocket reads the original value. The price: after an edit to the design, a
   kept tag's text still wins on export.
5. **Numbers exactly.** Each number is the shortest decimal that parses back to the same `f64`
   (L68). An angle is the shortest number of degrees whose conversion gives back the design's
   radians; the ogive parameter and rail-button offsets are searched for the same way.
6. **What OpenRocket 24.12 insists on,** found by loading written files in it: an id that is not
   a UUID makes it refuse the whole file, so such an id is left out (the reader invents the same id
   for a part with none, `bodytube-3`, and the tag of a part hpr reads as one kind, such as an
   engine block, is taken from that id or from what is kept), and an id that is not one the
   reader invents is warned of; an inner tube's and a packed part's roll angle is read only as
   `radialdirection`, so it is written so, with `angleoffset` beside it only where the file's
   disagreeing older tag is kept; a lug's and a rail button's `radiusoffset` is not read, so it is
   not written; a kept `<preset>` goes first, after the part's name and id, because OpenRocket
   applies a catalogue preset where it reads it, over the sizes read before. Schema 1.10, a zip
   with `rocket.ork` first and the original's attachments after, dated zip's zero date so the
   bytes never depend on the clock.

**Measured** (`cargo xtask ork`, 2026-09-29): 73 designs written; 73 read back the same; 73
written again byte for byte; no export warnings. Loaded in OpenRocket 24.12 once, by hand, every
written file opened whose original opens (71 of 73; the other two fail as originals do); M3.2b
commits that check.

**Still lost** (none in the corpus changes a design read back): a tag the file leaves out, where
the reader states its assumption (a radius with nothing to take, a missing wall or density) and
the written file states it, so its warning is gone on reading again; a second motor in one mount
for one configuration; a configuration with no id or declared twice; simulation rows and events
the reader left out; a fin tab dropped on a flat pod tube; the text of a second copy of a tag.

**Not chosen: writing the kept document back with the design's values patched in.** It would
round-trip read files trivially but could not write a design with no document behind it, and
every value would need a map back to the tag it came from.

**Not chosen: comparing designs with the kept tags' paths ignored.** It would hide a tag put back
in the wrong element.


## ADR-110: M3.2b: OpenRocket flies the export; only counts are published (2026-09-29)

**Context.** M3.2's bullets: every corpus design, read by hpr and written back out, loads in
OpenRocket 24.12, and OpenRocket's flight of the written file is within 0.5% of its flight of the
original. M3.2a (ADR-109) wrote files that read back in hpr as the same design, but OpenRocket
reads more than hpr does. Flown before a kept `<preset>` was moved first (ADR-109, point 6), a jar
example flew 0.6% to 0.9% low and a private design 10% to 11%: the check must fly files, not only
read them. The designs are other people's, so neither the files nor per-design results can be
committed.

**Decision.**

1. **Two `flights.py` records, compared.** Both are gitignored: OpenRocket's calm-air flights
   (seed 1, conditions from each file's first stored simulation, or OpenRocket's defaults) of the originals under `refs/` and
   in the jar (`corpus-out/openrocket-flights.json`), and of the files
   `cargo xtask ork --export corpus-out/ork-export` writes (`corpus-out/openrocket-export-flights.json`).
   `cargo xtask ork-export-flights` matches them configuration by configuration and commits counts
   by source (`validation/reports/openrocket-export-flights.{json,md}`), never a file name.
2. **The records must be current and comparable.** It refuses two records made with different
   jars, scripts or seeds, or with other scripts or another jar than the committed ones; an
   original whose SHA-256 is not the one flown; an export record whose files are not the ones hpr
   writes now; and a design found in one record only, unless hpr cannot read it.
3. **"Every corpus design" means every design OpenRocket 24.12 opens as written.** An export can't
   be expected to open where its original doesn't. Of the others, a design hpr can't read has no
   export, and one hpr reads must be refused as its original is, with the same error. A
   configuration left unflown (no motor, or aborted) must be left unflown alike both ways. The
   bar is met when every export of an opened original opens, no design is refused for a new
   reason or opened only as exported, no configuration is unmatched, and every configuration
   flown both ways is within 0.5%.
4. **CI holds the committed report to itself**: its page is its JSON, its total the sum of its
   sources, it meets the bar, and it names the committed scripts. CI has neither OpenRocket nor the corpus, so `--check`, which
   remakes the report and compares, runs only where both are.

**Measured** (2026-09-29): 75 designs; OpenRocket opens 71 as written and all 71 exports. The other
four: hpr reads two of them, and OpenRocket refuses those two exports with the originals' errors.
151 configurations of 48 designs fly both ways, all within 0.5%, 142 to the same apogee bit for
bit; the largest difference is 0.000162%, cause untraced. 23 opened designs have nothing to fly
(no configuration, or no motor); eighteen configurations fly neither way (17 with no motor, one
aborted both times for the same cause).

**Not chosen: a tolerance tighter than 0.5%.** The flights agree far more closely, but the
milestone's bar is 0.5%, and the report publishes the largest difference beside it.

## ADR-111: M3.3: the hpr design format, its extensions, versions and crate (2026-09-29)

**Context.** M3.3 asks for hpr's own design format: a JSON Schema generated by `schemars`, a
migrations framework, a zip container, TypeScript and Python types, a spec compared with `.ork`,
`.rkt`, `.CDX1` and `.rpy`, and an ADR naming the file extensions and the versioning policy. Its
first bullet: every corpus design goes `.ork` → hpr format → `.ork`, and hpr's apogee for the
result is the original import's within 1e-9 relative. That is more than one session. The design a
`.ork` reads into is already typed, serialised and tested on 73 designs: `hpr_io::ork::Design`,
whose motors, recovery events, stored simulations and `x-openrocket` extension live in `hpr-io`,
while the crate map had `hpr-io` depend on `hpr-format` (a stub nothing used).

**Decision.**

1. **Split into three.** M3.3a: the JSON document, its schema and the corpus round trip, carrying
   the round-trip bullet, schema validation in tests, and this ADR. M3.3b: the zip container,
   migrations, the comparison table, and `hpr convert` and `hpr sim` taking `.hpr`. M3.3c: the
   generated TypeScript and Python types.
2. **Extensions.** A design written as JSON is a `.hpr` file; the zip container of a design and
   its attachments (M3.3b) will be a `.hprz`. A bare `.json` can't say it is a design, a doubled
   `.hpr.json` is badly handled by file associations, and the pair follows glTF's `.gltf`/`.glb`.
3. **Versions.** A document opens with `"format": "hpr-design"` and `"version": "major.minor"`,
   checked before anything else, so another kind of file or version is refused with that reason.
   While the major version is 0 any minor version may change the document; a reader takes its own
   version, migrates older ones (M3.3b), and refuses newer ones as "written by a newer program".
   From 1.0, a minor version only adds, and a major version is the breaking change a migration
   carries. 0.1 is a draft until hpr's first release and changes in place, its schema regenerated;
   after a release, a change bumps the version and the old schema stays committed beside the new.
4. **Namespaced extensions, unknown keys refused.** What a source file holds that hpr doesn't model
   is kept under `extensions`, by namespace: 0.1 has one, `x-openrocket` (ADR-058). Every object in
   the document refuses a key its version doesn't define, including an unknown namespace, rather
   than dropping it: a document either reads whole or says why not. Keeping another program's
   namespace verbatim is left for a later version.
5. **`hpr-format` builds on `hpr-io`'s design.** The document is `hpr_io::ork::Design`'s parts
   under a header, so `hpr-format` depends on `hpr-io`, and `hpr-io` no longer on `hpr-format`
   (`ARCHITECTURE.md`'s crate map). The schema publishes those types as they are, some of them
   named for `.ork` (`OrkMotor`); a later version can re-home them through a migration.
6. **Canonical text.** Two-space indents, keys in the order the types declare them, a final
   newline. The writer reads its own output back and compares, so a value JSON can't carry (a
   number that is not finite, which `serde_json` would write as `null`) is refused, not lost.
7. **Provenance** names the program, its version, and the source file's format and SHA-256, never
   its path or name, which can name a person or a private design.
8. **The source file's other files travel with the design.** A top-level `attachments` list holds
   each archive entry besides the design, in order: UTF-8 text as text, other bytes (decal images)
   as base64 (RFC 4648). A `.ork` written from the document puts them all back. The first corpus
   run, without them, lost one design's two configurations, which flew on a curve its archive
   embedded. The reader refuses base64 that doesn't decode, two entries of one name, and an
   embedded curve whose entry is missing.
9. **Checked two ways.** `cargo xtask ork` takes every corpus design through the format: the
   document must follow the committed schema, read back the same, write the `.ork` M3.2a writes
   from the file itself (its other entries included) byte for byte, and read back from it as
   first read; each configuration that flies is flown three ways (as read, from the document, from
   its `.ork`), calm and standard at sea level off a 1.5 m rail, its stages separating if they do,
   and must agree within 1e-9. In CI, `hpr-format`'s tests do the same on the 17 public designs,
   with bundled motors standing in for OpenRocket's database; hold the committed schema to the
   generated one; and take each key out of two documents in turn, the schema and the reader having
   to agree on whether it is still valid.

**Measured** (2026-09-29): the 73 designs of M3.2a's corpus are all valid against the schema, read
back the same, write M3.2a's `.ork` byte for byte, and read back from it as first read; they carry
55 other archive entries, 3 as text and 52 as base64. 109 configurations, in 30 designs, fly all
three ways to the same apogee bit for bit (largest relative difference 0), most on OpenRocket's
motor database; the other 61 of the 170 are left out of their rockets as the `.ork` reader leaves
them (ADR-055), the same all three ways.
In CI the 17 public designs fly 18 configurations, the same from the document bit for bit and
within 1e-9 through the written `.ork`.

**Not chosen: moving the `.ork` design's types into `hpr-format`.** About 30 types would move and
every reader, writer and check would change, with no change in behaviour; it can come with a
version that renames them.

**Not chosen: the schema derives behind a feature.** `schemars` is now a dependency of `hpr-motor`,
`hpr-design` and `hpr-io`, compiled by every program that uses them; a `cfg_attr` on some 90
derives would spare them. The facade re-exports `hpr-format`, which needs them all, so it is left
until a program asks.

**Not chosen: keeping unknown keys.** A reader that drops what it doesn't know loses data without
a word; one that keeps it needs a place for it in every type. Refusing is honest until then.

## ADR-112: M3.3b: the `.hprz` container and migrations (2026-09-29)

**Context.** M3.3b asks for a zip container of a design and its attachments that reads back the
same, a migration test from an older version to the current one, the comparison with `.ork`,
`.rkt`, `.CDX1` and `.rpy`, and `hpr convert` writing and `hpr sim` reading `.hpr`. Version 0.1
(ADR-111) was the only version, so a migration had nothing to start from, and its document already
used `attachments` for the source `.ork`'s other entries, which the brief's container calls
"attachments" too.

**Decision.**

1. **Version 0.2 renames `attachments` to `source_files`** (type `SourceFile`), leaving
   "attachment" to mean a file carried beside the design in a `.hprz`. It is a real change made to
   documents that already exist, so it takes a new version with a migration, and 0.1's schema stays
   committed beside 0.2's.
2. **The version policy, amended.** ADR-111 §3 let 0.1 change in place until a release. From 0.2,
   while the major version is 0, the current version may still change in place only by a change
   that leaves every document already written readable with the same meaning; anything else (a
   rename, a removal, a key made required, a meaning changed) takes a new minor version, a
   migration from the one before, the old schema kept, and a document the old version's program
   wrote committed under `crates/hpr-format/fixtures/`, which a test migrates.
3. **Migrations work on the JSON, oldest first.** Each step rewrites the top-level object of one
   version into the next, since the old types are gone; the result is then read as a current
   document, so it is held to every current rule. A step refuses what its version doesn't
   define. `read_json` returns the version the text was written in; `from_json` migrates silently.
4. **The source's airframe, recorded.** `hpr sim --motor` refuses a `.ork` rocket whose airframe
   or a motor mount was not read exactly as written (ADR-106): a part left out, a value dropped,
   something assumed. A document keeps no reader's warnings, and the `.ork` written from it can
   state outright what the source left to be assumed: on the corpus, 2 of 73 designs gave another
   reason, or none, when their written `.ork` was read again. So 0.2's `provenance.source` records
   `airframe_not_as_written`, set by `DesignFile::from_ork`. 0.1 kept it only in a configuration
   left out for it, which the `.ork` reader asks after the motors and their ignition; the migration
   works it out: a configuration left out for the airframe gives the reason; one that flies, or is
   left out only for its separation (the one check after the airframe), shows the airframe was
   read as written; otherwise, including a document with no configuration, it is marked unknown
   (`migrate::UNKNOWN`), and `hpr sim` refuses another motor, saying so. On the corpus it recovers
   5 of the 8 reasons and marks 3 unknown, marks 28 of the 65 designs read as written unknown, and
   is never wrong; `cargo xtask ork` fails on a wrong one.
5. **The container.** A `.hprz` is a zip archive whose first entry, `design.hpr`, is the document
   exactly as a `.hpr` file, so unzipping one gives a `.hpr`; every other entry is an attachment,
   kept byte for byte in order. Entries are deflated and dated 1980-01-01, so with one build the
   bytes depend only on the contents. An attachment's name is a relative path with `/` between
   folders: none of its parts empty, `.` or `..`, longer than 255 bytes, ending in `.` or a space,
   or naming a Windows device with or without an extension (`CON`, `CONIN$`, `COM0` to `COM9`,
   `LPT¹` and the like), no `\`, `:` or control character, not `design.hpr` in any case; no two
   names the same ignoring case (compared upper-cased then lower-cased, so `ς`, `σ` and `Σ` meet,
   and `ß` meets `ss`, a conservative refusal), none also another's folder, and none in a
   top-level folder named `design.hpr`. A container holds at most 256 MiB unpacked, which
   the writer and the reader both hold to. The reader checks every name before it decompresses,
   passes over a folder's own entry if it is empty, refuses a symbolic link and a name beyond ASCII
   not marked as UTF-8, and
   refuses an archive whose central directory has more records than the zip reader keeps, which it
   does when two share a name (it keeps one and drops the other without a word). Unicode
   normalization (two spellings of one accented letter) is not checked.
6. **The command line.** `hpr convert` takes a design from any of `.ork`, `.hpr` and `.hprz` to
   any, by extension, and `--attach` adds files to a `.hprz` under their file names; a `.hprz`'s
   attachments that the output has no place for are warnings. Its JSON is the motor document
   unchanged or a design document with `rocket` (untagged, so no existing output changes). A
   conversion keeps the provenance it read: it names where the design came from, which rewriting it
   doesn't change. `hpr sim` reads `.hpr` and `.hprz` as it reads the `.ork`, with the airframe's
   reason from the provenance; a migrated document and a container's unread attachments are notes.

**Measured** (2026-09-29, `cargo xtask ork`): the 73 corpus documents are valid against the 0.2
schema, read back the same, write M3.2a's `.ork` byte for byte, and read back from it as first read;
each, rewritten as 0.1, migrates back to the same document but for the airframe's reading,
which 0.1 didn't record.
109 configurations fly three ways to the same apogee (largest relative difference 0).

**Not chosen: taking the airframe's reason from the `.ork` written from the document.** It is what
the first draft did; the corpus check above showed it wrong for 2 designs, one of them flyable
with another motor where its `.ork` is not.

**Not chosen: reading a 0.1 document with no sign of its airframe as read as written.** The first
draft of the migration did; review showed a rocket with a part left out flying another motor from
its 0.1 document. Marking it unknown means `hpr sim` refuses another motor in 28 corpus designs
that could fly one, until their `.ork` is converted again; that is the cheaper mistake.

**Not chosen: the source's files as container entries.** A `.hprz` could store `source_files` as
entries rather than inside the document. Keeping the document whole means the container's design
is a `.hpr` any reader takes, and a `.hpr` and a `.hprz` hold one design the same way.

**Not chosen: changing 0.1 in place.** The rename breaks every 0.1 document already written, and
a migration needs an older version to prove itself on.

## ADR-113: M3.3c: TypeScript and Python types generated from the schema (2026-09-29)

**Context.** M3.3c asks for TypeScript and Python types generated from the design format's schema
(ADR-111), a check that fails when they are stale, and each reading a public document. Types alone
check nothing at run time: `JSON.parse(text) as DesignFile` believes any JSON. Ready-made
generators exist under permissive licences (quicktype, json-schema-to-typescript,
datamodel-code-generator), but each covers one language or needs a Node.js or Python toolchain
inside `cargo test`, their output moves with their versions, and none writes a reader that checks
a document the way hpr's does.

**Decision.**

1. **xtask generates both**, from the schema `hpr_format::schema()` builds, which a test holds equal
   to the committed one (`xtask/src/format_types.rs`, about 1,700 lines with its two reader
   templates, tests aside). `cargo xtask format` writes the schema and both files, `--check` fails when any is
   stale, and so does a test. The generator refuses a schema keyword or `format` it doesn't
   check, so a new kind of constraint can't pass the readers silently.
2. **What they hold.** A type per schema definition, under the schema's names, with its
   descriptions as comments. TypeScript: interfaces, unions and string literals. Python:
   `TypedDict`s (the functional form where a key isn't a Python name, `from` and `x-openrocket`),
   `Literal`s and `Union`s; an object written inline in the schema takes a name from where it sits
   (`PartNoseCone`, `PositionTop`). A document stays plain JSON data; written back by
   `JSON.stringify` or `json.dumps`, it reads in hpr as the same design, tested on the 18 public
   documents, though numbers may be spelled differently.
   Only the current version gets types; an older document goes through `hpr convert`.
3. **A reader in each**, `readDesign` and `read_design`, checks the format and version first, then
   the document against a copy of the schema embedded without its prose. It checks what the schema
   says, no more: the rules only hpr's reader checks (ADR-112's source-file names, base64) stay
   unchecked (one gap runs the other way: #253). Before the schema, both refuse what hpr's JSON
   reader refuses: a number too large for a 64-bit float, a lone UTF-16 surrogate, nesting 128
   levels deep, `NaN`. Python also refuses a repeated key and `2.0` for a whole number, as hpr
   does; JavaScript's `JSON.parse` can't see either, so TypeScript takes them, and refuses a
   `uint` of 2^53 or more, which it would round. The generator also refuses a schema the readers
   would read differently: a boolean schema, a constraint beside a `$ref`, union or `const`, and a
   `pattern` with a class such as `\d` (Python's `$` is translated to `\Z`).
4. **Where.** `schema/format/typescript/hpr-design.ts` and `schema/format/python/hpr_design.py`,
   unversioned, beside an example that reads documents (`read-design.ts`, `read_design.py`). They
   are not published to npm or PyPI: that is Neer's call (CLAUDE.md, rule 9).
5. **How they are checked.** xtask's tests run both examples, under Node.js (22.18 or later runs
   TypeScript as it is) and Python 3.11 or later, on the 17 public designs' documents and the
   migrated 0.1 fixture, and on 4,892 mutations of two of them (the schema refuses 4,114), where
   each reader takes a document exactly when the `jsonschema` crate does; edge cases are held to
   `hpr_format::read_json`. A missing interpreter fails the tests rather than
   skipping them; CI's test job installs Node.js 24 and Python 3.12 on all three systems.
   `cargo xtask format --typecheck` writes the 18 documents as literals of type `DesignFile` and
   checks them, and both examples, with TypeScript 7.0.2's `tsc --strict --target es2020` and mypy
   2.3.1's `--strict`, fetched by `npx`
   and `uvx`, and that each refuses a misspelt tag; CI runs it in its own `types` job, on Linux,
   and the gate as an extra step, `types`, since it needs the network the first time.

**Consequences.** `cargo test` now needs Node.js and Python on the path. A schema change regenerates
both files with `cargo xtask format`; one that adds a keyword needs the generator and both reader
templates taught it first.

## ADR-114: M4.3: the Python package, and its split (2026-09-29)

**Context.** M4.3 asks for Python bindings (PyO3 abi3 wheels built by maturin) with NumPy outputs,
a RocketPy-like API, Python callbacks for custom models, a pytest suite and wheels built in CI on
three operating systems; it is done when pytest passes in CI and a notebook-style example
reproduces a RocketPy example flight within M2.1's 3%. That is more than one session. The one
RocketPy example the validation suite flies with the same drag (Calisto) needs a drag table the
builder takes only as a model (`FlightBuilder::drag_model`), and callbacks need a Python function
behind the `DragModel` and `Wind` traits.

**Decision.**

1. **Split.** M4.3a: the package over the builder, its pytest suite, and wheels built and tested
   in CI on Linux, macOS and Windows. M4.3b: a drag table on a flight, and RocketPy's Calisto
   reproduced from Python by a notebook-style example within M2.1's 3%. M4.3c: drag and wind models
   written as Python functions. Each has its own *done when* in `ROADMAP.md`; the parent's two
   bullets are b's and a's.
2. **What it wraps.** The `hpr` builder (ADR-103), not the crates below it: `Environment`,
   `Motor`, `Rocket` and `Flight`, as the Rust builder has them, with `Rocket.from_file` reading a
   design as `hpr sim` does (`.hpr`, `.hprz`, `.ork`, a rocket's JSON): the same configuration
   chosen, the same refusals of one that stages under power or doesn't fly as written, and its
   notes kept in `Rocket.notes`. No physics is added, so the
   bindings' tests hold a Python flight to the Rust example's printed output rather than to a
   reference of their own.
3. **"RocketPy-like."** RocketPy's shape, not its names: the same four objects, a `Flight` that
   flies as soon as it is made, time series as arrays, parachutes by `cd_s`. Names keep the Rust
   API's units (`apogee_m`, `wind_from_deg`), as CLAUDE.md asks of every field, since a Python
   user reads the same guide. Choices among several things are strings, read through the Rust
   types' own `serde` names where they have them, so the two can't drift.
4. **Records as dictionaries.** A flight's summary, landing, events, mass properties and margin
   cross as the objects `json.loads` makes of the Rust types' JSON: one code path, every field.
   The recording crosses as read-only NumPy arrays, one per column, copied from the recorder.
5. **Build.** PyO3 0.29 with `abi3-py310`: one wheel per operating system for CPython 3.10 and
   later (3.9 is past its end of life). NumPy through the `numpy` crate (BSD-2-Clause). No
   `extension-module` feature: maturin links the module itself, and the feature would break
   linking under `cargo test --all-features`. The crate's Rust test binary is off (`test =
   false`); its tests are pytest's, run on the built wheel by `scripts/python-tests.sh` in CI's
   `python` job on 3.10 and 3.13, and in the gate's optional `python` step. With no test binary,
   `cargo test` doesn't compile the crate; clippy and rustdoc check it on Linux, and the `python`
   job compiles it on each OS with warnings as errors.
6. **Not published.** The distribution name is `hpr-sim`; PyPI is Neer's call (CLAUDE.md, rule 9).
   CI builds and tests each OS's wheel but keeps none: a wheel handed out, even as a CI artifact,
   is a binary distribution, and needs the licence texts of what it links first.

**Consequences.** A builder change shows up in Python only when a binding exposes it. The
guide's [Python page](python.md) runs in the tests, block by block, against its printed output.
A design read from a file flies without its stored parachutes and separations, as the builder's
`Rocket::from_design` does; `Rocket.notes` says what isn't flown. Type stubs (`.pyi`) are not
written yet, so editors see the classes' docstrings but not their signatures' types. Before a
wheel is handed out anywhere, it needs the licence texts of what it links (rust-numpy's
BSD-2-Clause asks for its notice in binary distributions).

## ADR-115: M4.3b: a drag table, and RocketPy's Calisto from Python (2026-09-29)

**Context.** M4.3b asks for a flight that takes a drag table (`C_D0` by Mach, power on and off),
and a notebook-style example that flies RocketPy's Calisto from Python within M2.1's 3% on every
scored metric, run in CI. The validation suite's `fly_whole_flight` flies that case through
`hpr_sim` directly. Its setup differs from what the builder and the package offered in four ways:
the drag table (`Simulation::with_drag_table`), RocketPy's gravity formula
(`GravityModel::VerticalTaylor`), one parachute at a time (`Device::with_release_by`), and metrics
measured as RocketPy defines them. RocketPy's metrics follow the dry centre of mass from its start
at the ground. Its rail exit is the forward button's, after `effective_1rl`. Its landing is when
that point is back at its starting height.

**Decision.**

1. **The builder takes a table.** `FlightBuilder::drag_table(DragTable)` sets it as `drag_model`
   sets a model. The last one set is flown, as `AeroModel` has it. `Environment::with_gravity`
   swaps the gravity model and keeps the site and the Earth's rotation.
2. **The package exposes them.** `hpr.DragTable(power_off, power_on=None, *,
   reference_diameter_m=None)` takes `(mach, cd)` rows, linear between rows and held at the ends,
   as `parse_mach_csv`'s tables are. `DragTable.from_csv` reads RocketPy's two-column files. The
   rest are keyword arguments: `Flight(..., drag_table=)`, `Environment(..., gravity=)` and
   `add_parachute(..., released_by=)`. `gravity` names one of the three models that take no number,
   read through `GravityModel`'s `serde` tag. `released_by` is the other parachute's index, as in
   Rust; a bad index is refused when the rocket flies, by the simulation's own check.
3. **Drag that pushes is refused.** A drag table's negative coefficient, in a row or past or
   between its rows (a curve that extrapolates linearly, a cubic), was flown as it was, as drag
   pushing the rocket along (#257); a drag model's was already refused. The aerodynamics now
   refuse a table's as they do a model's, where a flight meets it, and the package refuses a
   negative row when a table is made. The corpus checks (`cargo xtask ork-flights --check`,
   `--library --check`, `real-flights --check`) and the validation report reproduce unchanged.
4. **RocketPy's definitions stay in the example.** The library gains no RocketPy-shaped metrics.
   `crates/hpr-py/examples/calisto.py` measures them on the flight's recording and events: the dry
   centre of mass is found by turning its body-frame position by each row's attitude. `h0` comes
   from a first flight of the rocket before its parachutes are added. The rail exit is found in
   the step that crosses `effective_1rl`, filled in with a cubic for the speed that matches the
   speed and the acceleration at both ends. The landing is interpolated linearly between the two
   steps where `height_above_ground_m`, the height the suite uses, crosses `h0`. The main opens
   `h0` above RocketPy's 800 m, as the suite has it. Two settings differ from the suite's and
   don't matter here: the builder's 3,600 s time limit (the suite's is 6,000 s; the flight takes
   261 s), and a table that holds its end values where the suite's refuses a Mach number past
   them (the flight stays below Mach 0.74, the table runs to 3).
5. **Checked twice.** `test_calisto.py` runs the example on the built wheel. It takes the scored
   metrics from the committed report's rows for the case that carry a relative tolerance: 14 of
   the 16 it scores, the other two being the trajectory's RMS rows, held to absolute bounds by the
   suite alone. It holds each to 3% of RocketPy's value, and to 1e-5 of the suite's own measurement of
   the same flight. The guide's printed table must equal what the example prints.

**Consequences.** Every metric scored in percent is within 3%; the largest difference is the
landing drift's, +1.257%, as in the suite's report. The example agrees with the suite's hpr
numbers to 3.2e-6 or better; the largest gap is the top speed's, which the example takes at the
integrator's steps and the suite also finds inside them. A wrong gravity model or a main opening
at 800 m rather than 800 m + h0 moves the landing drift by 1.8e-4 or 5.9e-4, so the test's 1e-5 catches
it. Only the windy case is flown from Python; the calm one and predicted mode stay the suite's. The example reads
the reference fixture's JSON, so it runs only from a checkout, not from an installed wheel alone.

## ADR-116: M4.3c: drag and wind as Python functions (2026-09-29)

**Context.** M4.3c's *done when*: a drag and a wind written as Python functions fly, their
exceptions reach Python, and a flight with Python's constant drag equals a table's. The Rust
library already takes both as traits (`hpr_aero::DragModel`, `hpr_atmos::Wind`); a flight runs
without the GIL (ADR-114), and a trait's error is the library's own type, which can't carry a
Python exception.

**Decision.**

1. `Flight(..., drag=f)` calls `f(mach, thrusting)` and takes a float, `C_D0` on the rocket's
   reference area. RocketPy's drag functions take the Mach number only; `thrusting` is the one
   other input a drag table reads, so a function can say what a power-on and a power-off table
   say. The Reynolds number and hpr's own buildup (`DragQuery`) stay Rust's: a query object
   would hold references into the flight, and nothing in M4.3 needs them.
2. `Environment(..., wind=f)` calls `f(height_m)` with the height above sea level, as the Rust
   `Wind` is asked, and takes `(east_m_s, north_m_s)`. Mean wind is horizontal (`wind.rs`), so
   there is no up component. A value that isn't finite is refused.
3. Each call takes the GIL with `Python::attach` and gives it back. An exception is kept in one
   slot per flight, shared by its drag and its wind (the environment keeps the function and binds
   it to the flight's slot as the flight starts), and the library gets an error of its own kind.
   The integrator retries a failed evaluation with a shorter step (`integrator.rs`), which would
   drop an exception at a trial state, and Ctrl-C with it; so the slot is sticky: once it holds
   one, both functions answer with an error and no call, and `Flight` raises the kept exception
   whatever the library's flight returned, so `except LookupError` works as the caller wrote it.
4. A function beside its constant counterpart (`drag` with `drag_table`, `wind` with
   `wind_speed_m_s`) is refused rather than one silently winning.

**Consequences.** `test_models.py` flies both and holds a constant function's flight of 0.5 to a
table's bit for bit, every recorded column. At 0.3 or 0.45 the apogees differ by up to 4e-11 of
themselves: a table's linear interpolation `(1 - t) y0 + t y1` (`interp.rs`) rounds a constant
row, so the table flies a neighbouring float; the tests hold those to 1e-10. Exceptions at any
call, `KeyboardInterrupt`, and eight threads sharing one windy environment are tested. An
atmosphere in Python is not in M4.3.


## ADR-117: M5.1 split; the cache and offline mode (2026-09-29)

**Context.** M5.1's *done when*: tests run against recorded fixtures, and offline mode never
touches the network, asserted by a test. The cache and the offline rule don't depend on HTTP, and
the HTTP transport brings the first network dependency with its `cargo deny` review. One session
held about an hour.

**Decision.**

1. Split M5.1 into M5.1a (the cache, TTLs, the offline mode and attribution over a `Transport`
   trait) and M5.1b (a `ureq` transport with rustls behind a feature, and the platform cache
   directory).
2. `Client::fetch(source, url, now_s)` takes the time as an argument, so tests are deterministic
   and the crate reads no clock. A copy is fresh while `now_s - fetched_at_s < ttl_s`; one exactly
   a TTL old is refetched online.
3. Offline returns any cached copy, marked `Stale` past its TTL, and never calls the transport;
   with no copy it is `NotCached`. Online, a failed fetch with an old copy returns the copy as
   `Stale` with the transport's reason in `stale_reason`, since stale weather with a label beats
   no flight; with no copy the error comes through. A copy dated after `now_s` is never fresh.
   Online, an unreadable entry is a miss and the fetch overwrites it; offline it is the error.
4. The cache stores `<key>.bin` and `<key>.json` per URL, the key being 64-bit FNV-1a of the URL.
   The metadata keeps the URL, so a hash collision reads as a miss. Each file is written to a
   temporary name unique to the process and the call, then renamed, the metadata last.
5. A `Replay` transport reads recorded responses from a folder's `index.json` and counts its calls.
   Its committed fixture is a hand-written body: the cache doesn't read content, and real recorded
   responses come with each source from M5.2, with their licences in `THIRD-PARTY-NOTICES.md`.

**Consequences.** `tests/offline.rs` asserts offline mode with a transport that panics if called,
before and after the cache is filled, fresh and 30 days stale. No pruning and no locking yet:
two writers of one URL leave whole files, but possibly one's body beside the other's fetch time.

## ADR-118: M5.1b, HTTP over the cache (2026-09-30)

**Context.** M5.1b's *done when*: a loopback server replaying a recorded fixture fills the cache,
and `cargo deny` passes. The architecture names `ureq` or `reqwest` with rustls only, and
`directories` for platform paths.

**Decision.**

1. `Http`, a `Transport` over `ureq` 3.4 with its `rustls` and `gzip` features and no others,
   behind `hpr-net`'s `http` feature (off by default; the facade's `net` turns it on). `ureq` is
   blocking, which `hpr-net` allows, and small beside `reqwest`'s tokio stack. rustls uses the
   ring provider and Mozilla's roots compiled in (`webpki-roots`, CDLA-Permissive-2.0, already
   allowed by ADR-001's list), so no OpenSSL and no platform certificate store; ring compiles its
   own C and assembly with the platform's C compiler, which CI's three runners have. `ureq`'s defaults
   stay for redirects (ten) and the proxy (the first of `ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY`,
   for every URL, bar `NO_PROXY`). Three are tightened, each pinned by a test: any status but 2xx
   fails (`ureq` fails only 4xx and 5xx, so a 304 read as an empty body that the cache would keep
   for a TTL); a SOCKS proxy from the environment is refused, where `ureq` without its
   `socks-proxy` feature warns and connects directly (hosts `NO_PROXY` exempts are fetched with no
   redirect followed, since the next host might not be exempt, and the error names only the
   proxy's protocol: its address may carry a password, and a malformed one puts the user name
   where the host should be); and the timeout is capped at 30 days, since
   `Duration::MAX`, the usual "no limit", overflowed `Instant` and panicked on the first request.
   A 60 s timeout covers the whole request. The `User-Agent` names hpr-sim, its version and the
   repository, so providers can tell who calls. Settings live in a `#[non_exhaustive]`
   `HttpConfig` (timeout, body limit, whether to read the proxy from the environment), so later
   knobs add fields, not constructors; the tests turn the environment's proxy off.
2. The body limit (64 MiB by default) counts **unpacked** bytes and is inclusive. `ureq`'s own
   limit sits inside its gzip decoder, so it counts bytes on the wire, and a kilobyte of gzip can
   unpack to gigabytes; it also refuses a body of exactly the limit. `Http` reads the unpacked
   stream through `take(limit + 1)` and refuses more than `limit`. `tests/http.rs` pins both: a
   body one byte over is refused and one at the limit passes, and a megabyte of gzipped zeros,
   under 64 KiB on the wire, is refused at 64 KiB.
3. `Cache::platform_dir()` is written by hand, not with `directories`: its `dirs-sys` 0.5
   depends on `option-ext` (MPL-2.0) on every target, which `deny.toml` rejects. The rules are
   those crates' for caches, read from the environment: `$HOME/Library/Caches/hpr-sim` on macOS,
   `%LOCALAPPDATA%\hpr-sim\cache` on Windows, `$XDG_CACHE_HOME/hpr-sim` (absolute only, per the
   XDG Base Directory Specification) or `$HOME/.cache/hpr-sim` elsewhere. `HPR_CACHE_DIR`
   overrides all three. `None` when nothing is set; the caller then names a folder.
4. The loopback test serves `tests/fixtures/replay`'s recording at its path and query, from a
   thread on `127.0.0.1` in the test itself: no fixture server dependency, no network. A dropped
   connection is a route that hangs up, not a port freed and reused, which another test could take.
5. `Transport::get` keeps its `String` error. Telling a 404 (don't retry) from a 503 or a timeout
   (retry later) matters once a source retries, and nothing retries yet; the trait can change then,
   before any release.

**Consequences.** The first network dependency: `ureq` brings 26 crates on macOS, all permissive;
`cargo deny` passes, with a second `base64` (0.23 beside our 0.22) as a duplicate warning only.
TLS is compiled and linked but no test performs a handshake, since CI has no network and a local
TLS server would need certificates made for the test. A one-off manual run on macOS on 2026-09-30,
from a scratch program outside the repository, fetched an Open-Meteo forecast over HTTPS (328
bytes, then `Cached` on the second call) and refused `expired.badssl.com`'s expired certificate;
repeatable checks come with M5.2's recorded responses. Windows reads `LOCALAPPDATA` from the environment rather
than asking the shell for the known folder; the two agree unless a user unsets the variable.


## ADR-119: M5.2 split; Open-Meteo's pressure levels as a sounding (2026-09-30)

**Context.** M5.2 names four sources: Open-Meteo's forecast and historical-forecast APIs with
pressure-level winds, NOAA's GFS and RAP as GRIB2 read in pure Rust, University of Wyoming
soundings, and ERA5 or GFS files the user provides. Its *done when*: recorded-fixture tests pass,
and a profile built from a recorded Open-Meteo response reproduces the pressure, temperature and
wind at the pressure levels. A GRIB2 decoder alone is a session's work.

**Decision.**

1. **Split a to d**, each with its own *done when*: a, Open-Meteo; b, Wyoming soundings; c, GRIB2;
   d, the user's files and the `hpr weather` command that the CLI already lists as planned for
   M5.2 (ADR-105). The parent's two bullets are M5.2a's.
2. **`hpr_net::open_meteo`**: an `OpenMeteoRequest` (place, time, API, optional model, optional
   endpoint for a self-hosted server) builds the URL, `OpenMeteoProfile::parse` reads the JSON, and
   `fetch` goes through a `Client`, so the cache and the offline mode apply. The request asks for
   the two whole hours around the launch (`start_hour`, `end_hour`, UTC, `timeformat=unixtime`),
   so one launch time is one cached answer of about 7 KB; the 19 levels Open-Meteo serves, each
   with temperature, relative humidity, wind speed and direction, and geopotential height; and the
   surface pressure, 2 m temperature and humidity, and 10 m wind. Units are named in the request
   (`wind_speed_unit=ms`) and checked in `hourly_units`: any other unit is refused, not converted.
   A forecast stays fresh for an hour, an archived forecast for 30 days.
3. **The ground is a level**, at the answer's `elevation`, with the surface pressure, the 2 m
   temperature and humidity, and the 10 m wind: the wind on the rail comes from the 10 m wind, not
   a step to the lowest level above (Loft lesson L6). Placing 2 m and 10 m values at 0 m is a
   stated approximation.
4. **Levels below the ground are dropped**: a level is kept only if its pressure is below the
   surface pressure and its height above the elevation. The models extrapolate beneath high
   ground; at Spaceport America's 1,400 m, 1000 to 900 hPa. A level with a `null` at either hour is
   dropped too, and one whose relative humidity is outside 0 to 100% (which `SoundingProfile`
   refuses), rather than refusing a whole forecast over one level; a `null` or an out-of-range
   humidity at the surface refuses the answer. Each is listed in `dropped` with its reason.
5. **Heights are geopotential metres**, converted with WMO-No. 8 eq. 12.16 at the answer's
   latitude, as `docs/physics/atmosphere.md` already said. Open-Meteo's documentation calls the
   variable an altitude above sea level, so the recordings are checked: from 500 to 30 hPa, the
   recorded layers match the hypsometric thickness from the levels' virtual temperatures to within
   0.03% to 0.13% on average; read as geometric heights they would be 0.58% to 0.68% too thin
   (`recorded_heights_are_geopotential` holds each mean to those ranges).
6. **Between the hours**, values are linear in time, as `a + w (b − a)`, which is exact when the
   hours agree (so 100% humidity stays 100%; `(1 − w) a + w b` rounded it above 1 at some
   seconds), and the wind is interpolated by components, as `hpr_io::era5` does. On the hour, the
   recorded speed and direction pass through unchanged. Directions are folded into `[0, 2π)`.
   Hours outside 1970 to 9999 are refused, which keeps the time arithmetic from overflowing.
7. **Relative humidity is read as over liquid water**, which `SoundingLevel` means. Whether each
   model reports it over ice at cold levels is not known; the density effect, `0.378 (e_w − e_i)/p`
   with `e_w − e_i` at most 27 Pa near −12 °C, is under 0.03% at 400 hPa; higher up the air is
   colder and the gap smaller (about 6 Pa at −40 °C, 0.08% even at 30 hPa).
8. **Fixtures** are two answers recorded unchanged on 2026-09-30 (historical forecast for 21 June
   2025, forecast for 2 October 2026, both at 32.99° N, 106.97° W), CC BY 4.0 with attribution in
   `THIRD-PARTY-NOTICES.md`. `Replay` serves them keyed by the exact URL `OpenMeteoRequest` builds,
   which pins the URL; the loopback server serves them over HTTP.
9. **Only an answer that parses is cached.** `Client::fetch_checked` takes the source's check (here
   `OpenMeteoProfile::parse` at the launch time, then `sounding`): a refused body is not stored, and online a stale
   good copy comes back in its place with the reason, else `NetError::Refused`; a cached copy the
   check refuses is a miss online and the error offline. Without it, a 200 with empty hours (the
   historical API before its data lands) would be kept for 30 days and could overwrite a good
   copy. `Client::fetch` is the check that accepts everything.
10. The request and the answer's types are `#[non_exhaustive]`, so an API key (Open-Meteo's
    commercial servers) or another field can come without a breaking change.

**Consequences.** M5.2a meets the parent's two bullets for Open-Meteo. The forecast's own accuracy
is unmeasured: no flight in Open-Meteo weather is compared with a log, and no forecast with a
balloon. A refused request (HTTP 400) reports its status but not Open-Meteo's reason, since `Http`
drops a failed answer's body. The example `open_meteo_weather` flies Calisto in the recorded
weather.

## ADR-120: University of Wyoming soundings (2026-09-30)

**Context.** M5.2b's *done when*: a recorded sounding's profile reproduces its pressure,
temperature and wind at every level, from recorded-fixture tests. The archive's current interface
(`/wsgi/sounding`) serves each sounding as an HTML page (`TEXT:LIST`) or as comma-separated text
(`TEXT:CSV`), and in two versions (`src=FM35`, the coded TEMP message; `src=BUFR`, a row a
second); with no `src` it picks one. The site states no terms of use (its pages were read on
2026-09-30), and `CLAUDE.md` keeps third-party data of unclear licence out of the repository.

**Decision.**

1. **`hpr_net::wyoming`**, mirroring `open_meteo`: a `WyomingRequest` (station, nominal hour,
   `version`, optional endpoint) builds the URL, always naming `src` so the cache key is exact;
   `WyomingSounding::parse` reads the answer; `fetch` goes through `Client::fetch_checked` with
   parse-then-sounding as the check. The station must be 1 to 16 ASCII letters and digits (no
   escaping needed), the time a whole hour from 1970 to 9999, and an endpoint non-empty with no
   `?` or `#`. `latest_before` picks the last 00 or 12 UTC, saturating at the ends of `i64`.
2. **Freshness follows the sounding's age.** The archive's copy fills in for some hours after the
   flight, as later messages arrive. Until a day after its hour an answer stays fresh an hour;
   after, the TTL is `min(30 days, now − (hour + 1 day))`, so only a copy fetched after the day
   is fresh, and a copy fetched young is fetched again.
3. **Read the CSV, not the HTML.** Its header names each column's unit (`pressure_hPa`,
   `geopotential height_m`, `wind speed_m/s`), which the parser checks, refusing any other unit,
   as `open_meteo` does with `hourly_units`. Columns are found by name, once each; a header of
   more than 64 names and a row of more than the header's fields are refused before they are
   split further, so a hostile answer costs no more memory than a good one.
4. **FM 35 by default, BUFR as an option.** The coded message is about 200 rows and 21 KB, the
   BUFR file about 6,000 rows and 540 KB. They differ most near the ground: in the recorded
   Santa Teresa pair the BUFR wind is 11.1 m/s 8 m up, where the coded message ramps from 5.7 to
   10.7 m/s over 186 m. Calisto is 402 m from the pad at apogee in one and 563 m in the other,
   upwind (the example `wyoming_sounding`, which also flies the BUFR file without those rows:
   419 m west). Which is nearer a rocket's wind is not known; the guide says so.
5. **The first row is the ground**; a first row missing a value or with one out of range refuses the
   answer. A later row with a value missing is dropped (`NoData`), and so is one out of range
   (`OutOfRange`: pressure outside 0.1 to 1,200 hPa, temperature outside −150 to 80 °C, geopotential
   height outside −1 to 60 km, wind speed outside 0 to 300 m/s, humidity below zero, direction
   outside 0° to 360°), rather than refusing the whole sounding; the bounds keep every level `parse`
   returns within what `SoundingProfile` accepts. A row with a pressure, height and temperature fits
   the row before it when its geopotential height above (or below) it is the layer's hypsometric
   thickness, `(R_d T̄_v/g₀) ln(p_bottom/p_top)` (WMO-No. 8 (2023) Vol. I eqs. 12.17 and 12.18;
   `T_v` from the humidity, dry without one, the vapour's share of the pressure capped at 1 so
   hostile input can't make it infinite), within 5% of it plus what rounding the two pressures can
   move it (half of 1 hPa for a pressure of 100 hPa or more in an answer whose pressures there are
   all whole, a coded message's; else half of 0.1 hPa) plus 30 m. Rows are kept from the longest
   chain from the ground in which each row fits the one before it, skipping at most 10 rows with a
   temperature at a time; of chains equally long, the one whose misses, as shares of their
   allowances, sum least. It is found by dynamic programming over the 11 rows before each, so it is
   linear in the rows. A row missing only its wind or humidity can be in the chain, so a gap in the
   wind keeps the checked layers thin. A row not in the chain is dropped (`Thickness`). Nothing
   before the ground checks it, so a chain starting at one of the 11 rows after it with a pressure,
   height and temperature within the bounds that beats every chain from it (longer, or as long with
   a smaller sum) refuses the answer (`GroundMisfit`). Bad rows right after a good ground that miss
   it but fit the rows above can win the same way, in narrow bands of error just past the ground's
   allowance, and refuse the answer: two BUFR rows 31 m high (not 20 m, which fit the ground, nor
   35 m, which fit nothing but each other), or three rows of the winter coded message 45 m high.
   So do 11 bad rows that fit each other, however far off. The data can't tell these from a
   bad ground, and a refusal is the safe failure.
   Rows grossly off fit nothing above and are passed by, up to 10. A ground with one row after it
   goes unchecked. Of each run of complete rows in the chain with the same pressure the middle one
   is a candidate (`SamePressure` for the rest; none at the ground's pressure): BUFR's pressures, to
   0.1 hPa, repeat on 1,931 of 5,851 rows, and the rounded value is the pressure at about the middle
   of its run. Keeping the first row of each run put 10 hPa 26 m low (the physics review) and misses
   a row by up to 0.093 hPa; with the middle, every BUFR row lies within 0.071 hPa of the profile. A
   candidate higher than, and at a lower pressure than, the last row kept is kept (`NotAbove`
   otherwise). In the three recordings every row fits the one before it within 1 m beyond rounding
   (0 in the coded messages, 0.97 m in BUFR; the test derives the thickness from the file's mixing
   ratio), so the 30 m and 5% are margin: 30 m for heights rounded to 10 m (the coded message's,
   from 500 hPa up), and because a height 30 m off misplaces a level by as much as a 0.33% to 0.55%
   pressure error; 5%, a judgment rather than a measurement, for layers whose inner rows lack a
   temperature. A bad row (57 hPa for 557, which would have to be 18.6 km above the row before it,
   or a height 850 m off) is dropped instead of kept to hide the good rows after it, and rows that
   fall or stay at one height fit and are dropped as `NotAbove`, however many. More than 10 rows
   after the chain's end refuse the answer (`Misfit`): the chain's end or all of them are wrong, or
   a long run of rows with no temperature leaves a layer too thick for its ends' mean to give. Ten
   rows are about 50 m of a BUFR climb and can be kilometres of a coded message. A ground at 1,200
   hPa, −1,000 m or −150 °C, which the bounds allow, now refuses. Counting rows hidden below a kept
   row, tried first, failed each way the code review probed it: a tail's first and last rows let a
   burst then a fall through and refused a float ending higher; a climbing chain stopped at a second
   bad row; rows above the row kept before failed on two bad rows in a row. The first thickness
   check, against the last row kept with 10 m of slack and 0.5 hPa for every whole pressure, refused
   a BUFR answer for a 20 m glitch and, across a long gap in the wind, for its ends' mean
   temperature; the chain without the look ahead let a row, or a ground, that only just fit drop the
   good rows after it; refusing when one row missed the ground and the next fitted that row refused
   a good ground before two bad rows; and local rules on top (the next row deciding an odd one out,
   a backtrack to an earlier chain row, three rows refusing the ground) kept a ground just outside
   its allowance and could drop a BUFR run of ten good rows for two bad ones (the physics and code
   reviews). The longest chain replaces them all. Raising one row, or a block of two or three, by
   20, 35, 50 or 100 m, or lowering it by 50 or 100 m, refuses nothing and loses at most 2 other
   levels of the coded messages; in the BUFR file, sampled every 150th row (for the test's run time)
   plus the worst rows a sweep of every row found, at most 8 other rows for one row and 10 for a
   block. Not caught: a wrong wind, humidity or temperature (the check doesn't use the wind,
   humidity moves it a few percent, and on layers under about 100 m any temperature within the
   bounds fits), a height error within the allowance (50 m on the coded message's 557 to 549 hPa
   layer), a pressure and height both wrong yet fitting, a ground within its first layer's
   allowance, and bad rows within the allowance of their neighbours, which can be kept while good
   rows are passed by, no more than the bad and fewer unless the bad fit more closely (683 and 673
   hPa raised 50 m leave out 664 hPa). `hpr-net` now depends on `hpr-core` for standard gravity: it
   is pure, `hpr-atmos` already uses it, and no third-party crate is added. More than 100,000 rows
   are refused as they are read, which bounds the memory a hostile answer costs. A relative humidity
   above 100% is kept as recorded and clamped to 100% in `sounding()`, as ADR-004 asked of
   radiosonde imports.
6. **Heights are geopotential**, converted with WMO-No. 8 (2023) eqs. 12.15 and 12.16 at the first
   row's latitude, the latitude `SoundingProfile` then uses; the balloon's drift would move a height
   about 0.8 m per degree at 10 km, 2.5 m at 30 km. Checked as in ADR-119: across the 13 layers
   between the standard levels from 850 to 10 hPa the recorded thicknesses match the hypsometric
   ones (the file's mixing ratios for the virtual temperature) to 0.01% to 0.03% on average, single
   layers 0.05% to 0.13% off on average either way; read as geometric heights they would be 0.51% to
   0.60% too thin on average. The test holds the mean under 0.1% and beyond −0.4%.
7. **Relative humidity** is the file's `relative humidity_%`, over liquid water; the file gives
   `humidity wrt ice_%` separately.
8. **Fixtures**: three answers recorded unchanged on 2026-09-30, Santa Teresa (72364) on 21 June
   2025 at 12 UTC in both versions and Salt Lake City (72572) on 15 January 2025 at 12 UTC in the
   coded version, served by `Replay` keyed by the exact URL. Committing them is decided without
   Neer, as a "no action if fine" item in `STATUS.md`: both are U.S. National Weather Service
   stations, whose observations are U.S. government works (17 U.S.C. § 105) exchanged freely
   under WMO Resolution 40; the archive's table of them adds only derived values; the archive
   states no terms; and Unidata's `siphon` (BSD-3-Clause) commits recorded answers from the same
   archive as test fixtures (`tests/fixtures/wyoming_sounding` and others). Only U.S. stations'
   soundings are committed; any other stays under `refs/`.
9. The dates in URLs and in the release time use Hinnant's algorithms, now in the private
   `civil` module shared with `open_meteo`.

**Consequences.** M5.2b is met by `tests/wyoming.rs`. How far a station's sounding is from the air
over a launch site, in distance and time, is unmeasured. The HTML form, the archive's other
formats and a station list are not read. The examples `wyoming_sounding` and `open_meteo_weather`
now print where Calisto is at apogee, east and north, since a drift distance alone read as
downwind when the rocket had weathercocked upwind.

## ADR-121: GFS and RAP from NOMADS' grib filter, read by an in-house GRIB2 decoder (2026-09-30)

**Context.** M5.2c's *done when*: a recorded GRIB2 cut decodes to the values an outside decoder
(ecCodes, run only) prints, and its levels become a profile. `ARCHITECTURE.md` named the `grib`
crate (0.18.6, MIT OR Apache-2.0, maintained) for GRIB2. It decodes to `f32`, so it cannot give
back ecCodes' doubles; its default features bind C libraries (`openjpeg-sys`, `libaec-sys`,
`proj`). NOMADS' grib filter (`filter_gfs_0p25.pl`, `filter_rap.pl`) cuts variables, levels and
a latitude/longitude box from a run's file. Both recorded cuts are simple packing (template 5.0)
on the model's own grid: GFS's 0.25° latitude/longitude (3.0), RAP's 13 km Lambert conformal grid
130 (3.30, tangent at 25° N, `LoV` 265°, winds along the grid).

**Decision.**

1. **An in-house decoder, `hpr_io::grib2`**, for what the filter serves: grids 3.0 and 3.30 (a
   northern tangent cone on a sphere; a secant cone or `LaD` off the tangent latitude is refused,
   since it would need the scale factor there and no NCEP grid in use has one), product 4.0,
   packing 5.0, and a bitmap given, absent or reused. Anything else is refused by template number.
   Values are `Y = (R + X · 2^E) / 10^D` in `f64` with exact powers (WMO-No. 306 Vol. I.2,
   Regulation 92.9.4); signed integers are sign and magnitude (92.1.5). `hpr-io` holds it because
   it is pure and M5.2d reads users' files offline; `hpr-net` now depends on `hpr-io`. It replaces
   the `grib` crate in `ARCHITECTURE.md`; no third-party crate is added.
2. **Bounded against hostile files.** `parse` keeps each field's bitmap and packed values
   borrowed, so it costs memory per field, not per grid point; a bitmap carries its running count
   of set bits every 64 bytes, shared by the fields that reuse it, so neither checking a field nor
   reading a value recounts it (the code review found a file of reused bitmaps took 31 s); a grid is refused above 2²⁴ points
   (a 0-bit field has no data to bound it); every section's length, the bitmap's length and marked
   count, and the packed bits are checked against the grid and the packing before a value is read;
   scale factors that make an infinite value are refused; so are latitudes off the Earth and a
   given radius outside 6,000 to 7,000 km. `hpr_net::nomads` keys fields in a hash map, so neither
   the duplicate check nor the lookups are quadratic, refuses a cut of more than 1,000 fields (a real
   one has 147 or 192); `fetch` refuses a cut whose grid steps and projection aren't the model's (GFS's 0.25°
   steps; RAP's 13,545 m cells, tangent at 25° N about 265° E; where the grid starts is not checked), which a self-consistent but wrong grid
   would otherwise pass: the bilinear weights and the points' positions come from the same grid.
3. **Lambert grids** use Snyder's spherical Lambert conformal conic (USGS PP 1395, 1987,
   eqs. 14-1, 14-2, 14-4, 15-1 and 15-2; inverse 14-9 to 14-11 and 15-5) with `n = sin φ₁`, eq.
   15-3's one-parallel case. **Grid-relative winds** are
   turned at the site by `θ = n (λ − λ₀)`, the bearing of the grid's `+y` axis: `u_E = u cos θ +
   v sin θ`, `v_N = −u sin θ + v cos θ`. The sign was first written backwards in the draft; the test
   measures the grid's `+x` bearing from ecCodes' own positions of two grid points and matches
   `90° + θ` to 2.2e-6°. Components are interpolated first and turned once, at the site (`θ`
   changes about 0.06° across a 13 km cell).
4. **`hpr_net::nomads`**, mirroring `open_meteo`: a `NomadsRequest` (model, cycle, forecast hour,
   site, optional endpoint) builds the filter URL for `HGT`, `PRES`, `RH`, `TMP`, `UGRD`, `VGRD` at
   the ground, 2 m, 10 m and each pressure level (GFS 1000 to 10 hPa, 28 levels; RAP all 37, 1000 to
   100 hPa) in a box 0.3° each way, written to 0.01°; `NomadsProfile::parse` decodes it, refuses a
   cut whose fields differ in grid, run or forecast time or give a variable twice, and interpolates
   bilinearly in grid indices between the four points around the site; `fetch` goes through
   `Client::fetch_checked`, whose check also refuses a cut of another run or hour, so it is never
   cached, as is one whose grid is not the model's (point 2).
   GFS runs every 6 hours, hourly to hour 120 and every third hour to 384; RAP every hour, to hour 21,
   and to 51 from its 03, 09, 15 and 21 UTC runs; other cycles and hours are refused. A run's file doesn't change once written, so a copy stays fresh 30 days.
5. **The ground and the levels** follow ADR-119: the ground at the model's terrain height with the
   surface pressure, 2 m temperature and humidity and 10 m wind; a level is dropped when its pressure
   is not below the ground's or its height not above it, or when a variable is absent or has no value
   at a grid point with weight. Heights are GRIB2's geopotential metres (code table 4.2), converted
   with WMO-No. 8 eq. 12.16 at the site's latitude; the terrain height, also in gpm, is converted the
   same way (1.9 m at 1,400 m and 33° N; if a model's terrain is a geometric height, the ground
   sits that far high, ADR-081's caveat). Checked as in ADR-119: from 500 hPa up, the recorded layers match
   the hypsometric thickness to −0.002% (GFS) and −0.08% (RAP) on average; read as geometric heights
   they would be 0.63% and 0.51% too thin. Humidity over 100% is kept and clamped in `sounding()`
   (ADR-004).
   Whether NCEP reports humidity over ice at cold levels is not settled; the density effect is under
   0.1% (ADR-119 §7).
6. **Checked** by `tests/nomads.rs` against ecCodes 2.49.0 (Apache-2.0, run only, not ported), whose
   reading `validation/oracles/grib2/eccodes_dump.py` writes to `tests/fixtures/nomads-eccodes.json`:
   every field's identity, all 6,123 values within 2.2e-16 relative (one rounding: ecCodes multiplies
   by an inexact `10^−D`) and every grid point within 5.7e-14° on macOS (the test's bound is
   1e-12°: RAP's points go through `tan`, `powf` and `atan`, whose last digits vary between maths
   libraries). The profile gives back ecCodes' values
   interpolated to the site at every level kept: 22 GFS levels and 31 RAP levels, 6 of each
   underground at the 1,400 m site. The ground test is made at the site only, so a kept level can
   take weight from a grid point where it is underground (RAP's 850 hPa here, 13%, about 0.02 K).
   ecCodes is pinned in the oracle environment (`eccodes` 2.48.0 with `eccodeslib` 2.49.0.30).
7. **Fixtures**: the two cuts recorded unchanged on 2026-09-30 at Spaceport America, GFS's 00 UTC run
   at hour 18 and RAP's 12 UTC run at hour 6, both for 18 UTC. NCEP's forecasts are U.S. government
   works (17 U.S.C. § 105).

**Consequences.** M5.2c is met. NCEP's whole files use complex packing (5.2, 5.3) or JPEG 2000
(5.40), which the decoder refuses; M5.2d, which reads users' GFS files, adds them or reconsiders the
`grib` crate's pure-Rust features. There is no interpolation in time: the user picks the run and the
hour. How good either forecast is at a launch is unmeasured.

## ADR-122: `hpr weather`, and M5.2d split (2026-09-30)

**Context.** M5.2d's *done when*: `hpr weather` writes a site's profile from each source, offline
from a fixture. Its title adds the user's ERA5 `.nc` files and whole GFS GRIB2 files, whose
complex packing (templates 5.2, 5.3) and JPEG 2000 (5.40) `hpr_io::grib2` refuses by number
(ADR-121). Each packing is a decoder of its own, to be checked value for value against ecCodes;
with the command, that is more than a session.

**Decision.**

1. **Split d1 to d3.** d1: the command, for every source hpr reads today (Open-Meteo, Wyoming,
   the NOMADS cuts, ERA5 `.nc`); its *done when* is the parent's for those. d2: complex packing,
   5.2 and 5.3, from a whole GFS file. d3: JPEG 2000, 5.40. The parent's clause holds for GFS's
   whole files only when d2 and d3 are met.
2. **One subcommand per source**: `open-meteo`, `wyoming`, `gfs`, `rap`, `era5`. Each takes what
   its request type takes, by the same names as `hpr sim`'s site (`--latitude`, `--longitude`).
   Times are UTC written `YYYY-MM-DDTHH[:MM[:SS]]Z`; one without its `Z` is refused, so a local
   time is never read as UTC. Open-Meteo reads at `--time` (`--historical` for the archive, `--model`
   passed through); Wyoming fetches the station's latest sounding before the launch `--time`
   (`WyomingRequest::latest_before`), `--bufr` for the BUFR version; GFS and RAP take `--cycle` and
   `--hour` as `NomadsRequest` does. No source is picked for the user, and nothing depends on the
   clock but the cache's freshness.
3. **Where the answer comes from.** Online, through `hpr_net::Client` over `Http` (rustls) and the
   platform cache folder (`Cache::platform_dir`, `HPR_CACHE_DIR`), so the library's freshness,
   fallback to a stale copy, and checked caching (ADR-119 §9) all apply. `--offline` answers from
   the cache alone and fails with nothing there. `--from FILE` reads a saved answer and touches
   neither the network nor the cache, since a file is not known to be the answer to any URL. It is held
   to the checks `fetch` makes: the parser, the sounding, and for GFS and RAP the model's grid
   and, when `--cycle` and `--hour` are given, the run and hour. The options that choose what to
   fetch and that a saved answer can't be checked against (Open-Meteo's site, `--historical` and
   `--model`; Wyoming's station, time and `--bufr`) are refused beside `--from` rather than
   ignored, so a wrong file can't pass as the place asked for. A time outside the answer's hours
   is named in UTC, as it was asked. `hpr-cli` gets the facade's `net`
   feature; no crate is added to the workspace.
4. **What is written.** `--output` writes `hpr_atmos::SoundingProfile`'s JSON, which the library
   reads back with `SoundingProfile::new`'s checks. Winds between levels are interpolated by speed and
   direction (`WindInterpolation::SpeedDirection`, the type's default) for every source; the
   field is in the file. The `--json` document is the CLI's own type (`weather.schema.json`,
   ADR-105 §5): the source and its credit, where the answer came from, the profile's position and
   time, its levels (wind direction in degrees, as `hpr sim`'s `--wind-from`), and the levels left
   out with the reason. The credit is always printed; ERA5's is the one its licence asks for
   (`docs/format/era5.md`).
5. **`hpr sim` doesn't read the profile yet** (#265). The command stops at the file.
6. **Checked** by `crates/hpr-cli/tests/weather.rs`: the six recordings in
   `crates/hpr-net/tests/fixtures/replay/` (two Open-Meteo, two Wyoming, GFS, RAP) through
   `--from`, and through `--offline` from a cache filled by the library's `fetch` over `Replay`,
   and two ERA5 files, each write the profile the library builds from the same bytes, equal to the
   bit, with the document's levels, position, run and levels left out equal to the library's and
   checked against the schema. Offline with no copy,
   all six are refused; a RAP cut read as GFS, and a cut of another hour, are refused. The live
   fetch was run by hand on 2026-09-30 for all four online sources over HTTPS, then again from the
   cache; CI never goes online. A saved answer is read whole with no size limit, since M5.2d2 reads
   whole GFS files, about 500 MB, through `--from`.

**Consequences.** M5.2d1 is met. A user can fetch, cache and save a launch day's weather from the
command line, but must fly it from a program until #265. NOAA's whole files stay refused until
d2 and d3.

## ADR-123: Complex packing, and a whole GFS file (2026-09-30)

**Context.** M5.2d2's *done when*: a whole GFS file's fields decode to ecCodes' values, and
`hpr weather gfs --from` writes its profile. The file is GFS's 0.25° file of the 00 UTC run of
2026-09-30 at hour 18, `gfs.t00z.pgrb2.0p25.f018` (550,166,372 bytes, SHA-256 `648fcc36…c1c62a`),
from NOAA's open data bucket on AWS (`noaa-gfs-bdp-pds`): the run the recorded NOMADS cut
(ADR-121 §7) was taken from. ecCodes' survey of it: 743 messages, 742 in template 5.3 (all
second-order differencing, 1 to 3 bytes per descriptor, 21 with missing values marked in the data,
34 with a bitmap) and one 5.0; 701 in product template 4.0 and 42 in 4.8 (statistics over an
interval, such as accumulated rain). No 5.2 or first-order 5.3.

**Decision.**

1. **Complex packing in `hpr_io::grib2`** (`grib2/complex.rs`), from WMO-No. 306 Vol. I.2's
   templates 5.2, 5.3, 7.2 and 7.3: group references, widths and lengths, each list padded to a
   byte; the last group's length as given; primary and secondary missing values (code table 5.5)
   by all ones and all ones less one, in the value or, for a group of width 0, in its reference;
   first- and second-order differences rebuilt over the values present only, the packed integers
   in the first one or two places unused; then `Y = (R + h · 2^E) / 10^D` as for simple packing.
   `Field::packing` becomes an enum, `Packing::Simple` or `Packing::Complex`. The values of a
   complex field can only be read in order, so `Field::value` reads the field up to the point,
   `Field::values_at` reads several points in one pass, and `Field::values` returns a `Result`.
2. **Bounded, and refused by name where decoders differ.** `parse` reads each group's width and
   length once, stopping as soon as the lengths pass the packed count; with lists of 0 bits
   every group is alike and is counted at once, so the work is bounded by the file's size (the
   code review built a 2 MB file of 0-bit lists claiming 2^24 groups a field, and one whose sums
   overflowed 64 bits). The lengths must add up to the packed count and the values fit the data;
   a group may be at most 32 bits wide, the lists' entries at most 32 bits, 1 to 8 bytes per
   descriptor, order 1 or 2, and at most as many groups as values. Refused by name: a group
   splitting method other than 1 (with 0, row by row, lengths have no meaning); no groups for a
   nonzero count (ecCodes reads it as the reference everywhere, issue ECC-2095; the regulation is
   silent); secondary missing values with 0-bit references (no bits for all ones less one); and a
   first value with its top bit set, which ecCodes reads unsigned and NCEP's g2clib as sign and
   magnitude. Primary missing values with 0-bit references are read as ecCodes reads them (every
   group of width 0 missing; its encoder writes a field with no values that way). The rebuilt
   integers are not bounded by the headers, so the scale factors must keep `i64`'s ends finite,
   and rebuilding refuses a difference that overflows 64 bits (only a broken file does) when read.
   Each field keeps a private copy of the packing it was checked against and decodes from it.
3. **Product template 4.8** is read, with one time range (more are refused): the statistic (code
   table 4.10), the interval's length and unit, and its end, in `Product::statistics`;
   `Product::template` says which. `hpr_net::nomads` leaves statistics out.
4. **NOMADS parsing of a whole file.** A field is keyed by its parameter, its first surface and,
   for a layer, its second: the whole file's humidity over sigma 0.44 to 1 and 0.44 to 0.72 share a
   first surface and were refused as a duplicate. A layer is not a level (a test found a layer on
   500 hPa counted as a second 500 hPa level). A latitude/longitude grid whose `ni` steps make
   360° joins its last column to its first (`Grid::circles_the_earth`), so a site between 359.75°
   and 0° reads.
5. **Checked, the whole file, by a script run outside CI.** `validation/oracles/grib2/whole_file.py
   compare` streams every value `cargo xtask grib2-values` decodes and compares it with ecCodes
   2.49.0's, and writes its record, with ecCodes' survey of the file, to
   `validation/oracles/grib2/gfs-whole-file.json`: all 746,770,303 values agree within 4.4e-16
   relative (ecCodes multiplies by an inexact `10^−D`, one or two roundings), and the 24,642,017
   points without a value are the same points. The file is not committed (550 MB); it decoded in
   5 s in a release build on a Mac.
6. **Checked in CI.** Eight whole messages cut unchanged from the file (`crates/hpr-io/tests/
   fixtures/gfs-messages.grib2`, 404,731 bytes): for 1-, 2- and 3-byte descriptors and template
   4.8, the smallest over 2,000 bytes, for a message of some size; the smallest with a bitmap, with
   missing values in the data, and of template 4.8; and the one 5.0 field. The
   committed reading (`whole_file.py cut`) is, per message, its identity, template 4.8's interval,
   its missing points, `math.fsum` of its values and of each value times its index plus one, and
   every 997th value: 8.3 million values can't be committed. The sums hold the decoder
   to 1.35e-15 of the terms' sizes; in every committed message one packing step (`2^E / 10^D`) is
   at least 2.3e4 times that, so a single value off by a step fails the plain sum; the weighted one
   also depends on where each value sits. Two mutations were tried by hand, on the decoder before
   the review's rework of `layout`: a second-order start off by one failed the plain sum, and a
   missing-value code off by one failed the count of points without a value. Besides, the recorded GFS cut repacked by ecCodes
   (`repack.py`: 5.2, first- and second-order 5.3 in turn, 24 bits, 1 to 4 bytes per descriptor)
   joins the NOMADS tests (every value against ecCodes, the profile at every level) and gives
   `hpr weather gfs --from` a complex-packed file; its profile is the cut's within 6.9e-8 (bound
   7e-8).
   Hand-built messages pin each rule: groups, both kinds of missing value, both orders, a bitmap,
   and each refusal.
7. **`hpr weather gfs --from` the whole file** writes a profile in 0.33 s (release, on a Mac) whose 22
   levels are the cut's within 1.04e-7 relative, with 13 more above 10 hPa, where the cut stops;
   the same six levels are left out below the ground. The cut is repacked by NOMADS' filter in
   fewer bits (9 for a 650 hPa wind where the file has 13): by ecCodes alone, the two files'
   values differ by up to 2.4e-6 relative at the cut's grid points (`gfs-whole-file.json`). A test
   runs this where `refs/gfs/` holds the file (bound 1.1e-7) and passes without it, so CI skips
   it. Only 0.25° files read: `--from` holds a file to the model's grid (ADR-122 §3).
8. **Fixtures.** NCEP's output is a U.S. government work (17 U.S.C. § 105). ecCodes is run as an
   outside decoder and encoder, never ported.

**Consequences.** M5.2d2 is met. GFS's whole 0.25° files read offline. Files in JPEG 2000 (5.40)
stay refused, by name, until M5.2d3. RAP's whole files were not surveyed. The whole-file check is
by hand; CI holds eight of its messages and the repacked cut.

## ADR-124: JPEG 2000 packing, through `hayro-jpeg2000` (2026-09-30)

**Context.** M5.2d3's *done when*: a GRIB2 file in JPEG 2000 packing (data representation template
5.40) decodes to ecCodes' values. Template 5.40 codes a field's integers as a greyscale image in a
JPEG 2000 codestream (ISO/IEC 15444-1), then unpacks them as simple packing does,
`Y = (R + X · 2^E) / 10^D`. Messages sampled by byte range from NOMADS's 00 UTC run of
2026-09-30 were in 5.40 in each of RAP's pressure-level files (grids 32, 130, 200, 236, 242, 243,
252; `wrfmsl` is 5.3) and in 5.3 in NAM's. A JPEG 2000 decoder (tier-1 arithmetic decoding, tier-2 packets, the wavelets) is
far more code than the packings before it.

**Decision.**

1. **`hayro-jpeg2000` 0.4.0 decodes the codestream.** It is pure Rust, `MIT OR Apache-2.0`, forbids
   `unsafe` in its own code, and builds for wasm32; with no default features (only `std`) it pulls
   in no other crate. Writing a decoder in house was judged a milestone of its own for no gain in
   what is checked, since the check is against ecCodes either way. The C OpenJPEG (through
   `jpeg2k`) was ruled out: it links C and does not build for wasm32.
2. **Lossless, to 21 bits, the rest refused by name.** The crate runs the reversible 5/3 wavelet
   in `f32`. Its inverse adds two neighbouring high-pass coefficients before each floor, and for
   `B`-bit samples those sums reach nearly `16 · 2^(B−1)` (the cascaded analysis filters bound a
   coefficient by about `8.2 · 2^(B−1)`), so every step is exact only while that stays below
   `2^24`: `B ≤ 21`. The first draft took 24; review probes (fields ecCodes encoded on the
   fixture's grid) came back off by one in 121 and 136 of 10,152 values at 24 bits, and in one
   value in one of six probes at 23, every one whole and in range; ten probes at 21 and 22 were
   exact. More bits (`bits per value in JPEG 2000`) and
   lossy coding (code table 5.40's 1) are refused. Every decoded sample must also be a whole
   number from 0 to `2^bits − 1`.
3. **NCEP's coding only, checked before decoding; strict mode.** The main header and each
   tile-part header are read by hand (ISO/IEC 15444-1, Annex A) when the file is parsed: one
   unsigned component, no subsampling, no image or tile offset, one tile, a side of at most
   60,000 (the crate's limit), the 5/3 transform in COD and COC, no quantization in QCD and QCC,
   default precincts, and no marker but these, COM, TLM, PLM and PLT. SIZ, COD, COC and SOT must
   be exactly as long as their fields (the crate reads them field by field, so a longer length
   would hide a segment from the check that the crate still reads). Anything else is refused by
   name, and the image must hold one sample per packed value at section 5's bits. Review found
   that without these a 272-byte message with subsampled 1×1 tiles made the crate multiply past
   `u32` (a panic in debug builds, an eager allocation of about 1.3e9 tiles in release). The crate
   decodes in strict mode: its lenient mode filled a cut-short codestream with its DC offset,
   whole numbers in range, which review reproduced; a test cuts the fixture's codestream at every
   length. A field of 0 bits has no codestream and is `R / 10^D` everywhere, by the regulation;
   ecCodes gave `R` for one with `D = 2` (review's probe), so that case is not checked against it.
4. **Decoded whole, on each read.** A JPEG 2000 image is decoded whole, so `Field::value` decodes it
   for one point; `values` and `values_at` read it once. Nothing is cached in `Field`, which
   borrows the file and stays `Clone` and cheap.
5. **Checked against ecCodes in CI, on public data.** Four whole messages cut by byte range from
   RAP's 00 UTC run of 2026-09-30 (a U.S. government work): 500 hPa temperature on grids 200
   (6 bits) and 130 (9 bits, 151,987 points), and cloud base and top heights on grid 200 with
   bitmaps (15 and 16 bits; the top marks 434 of 10,152 points). `whole_file.py cut-rap` writes
   ecCodes 2.49.0's reading: counts of points and missing points, two correctly rounded sums over
   every value, and every 13th value. `tests/grib2_gfs.rs` holds `hpr_io::grib2` to it as it holds
   the GFS messages. A damage test edits the codestream at random and asks for a refusal, never a
   panic (5,000 cases run once by hand; CI runs proptest's default 256).

**Consequences.** M5.2d3 is met, and with it M5.2d and M5.2. No whole RAP file was read: `hpr
weather rap --from` on one is untried. NCEP codes a field with a bitmap as one row of its packed
values, so such a field of more than 60,000 values is refused by name (`JPEG 2000 image side`); a
whole grid-130 file may hold some. A new dependency, `hayro-jpeg2000`, sits in `hpr-io`. Lossy
JPEG 2000 and fields over 21 bits stay refused; the four fixture fields have 6 to 16 bits.

## ADR-125: Site data split, and WMM2025 in `hpr-core` (2026-09-30)

**Context.** M5.3 asks for elevation (Open-Meteo, cached, a user's GeoTIFF optional), geodetic
helpers and magnetic declination from WMM2025, done when the WMM matches NOAA's test values and
elevation lookups are cached and work offline. That is three pieces of different kinds: a pure
model, a network source, and a file reader. NCEI publishes WMM2025 (released 2024-12-17) as a
coefficient file, `WMM2025.COF`, with two sets of test values: the report's Table 6
(`WMM2025_TEST_VALUES.txt`, 12 points, 0.1 nT and 0.01°, with grid variation) and 100 points to
1e-6 nT (`WMM2025_TestValues.txt`, in `WMM2025COF.zip`). The report also prints one point's
intermediate values to ten decimals (Table 3b) and the field over each pole (section 1.4). All
are works of the United States government.

**Decision.**

1. **M5.3 splits into a to c.** a: WMM2025; b: elevation from Open-Meteo, cached and offline after
   the first fetch; c: geodetic helpers (distance and bearing between two places) and a user's
   elevation file. a goes first: it needs no network and no new dependency.
2. **WMM2025 in `hpr_core::magnetic`.** The Earth's field is an Earth model beside gravity and
   geodesy, used by sims (a rail heading read off a compass) and by flight logs (magnetometers),
   so it sits in the pure core and builds for wasm32. The 90 coefficient rows are a `const` table;
   the coefficient file and both test-value files are committed under
   `crates/hpr-core/data/wmm2025/`, and a test holds the table to the file row by row.
3. **The report's equations, finite at the poles.** Equations 7 to 20, with each Schmidt function
   written as `P̆ₙᵐ = cₙₘ cosᵐφ′ qₙᵐ(sin φ′)`, `qₙᵐ = dᵐPₙ/dμᵐ` from DLMF 14.10.3's recurrence. The
   derivative and `P̆ₙᵐ / cos φ′` then need no division by `cos φ′`, so a pole needs no special
   case and the field there is the limit along the given meridian (report, section 1.4). Equation
   15 prints `ġ cos mλ − ḣ sin mλ` for `Ż′`; the potential gives `+`, which the test values
   confirm.
4. **Refused outside the model.** Times outside 2025.0 to 2030.0 and heights outside −1 km to
   850 km above the ellipsoid return `CoreError::Domain`. A flight log from before 2025 needs
   WMM2020, which is not bundled. `decimal_year` turns a date into the start of its day.
5. **What is held, and to what.** Table 6: every column of every row to half its last printed
   digit, grid variation included (`None` exactly where the table prints `NaN`). Table 3b: every
   printed value to half its last digit, plus eight units in the last place. The poles: to
   0.05 nT. NCEI's 100 points (rows counted from 0): `Y`, `D`, `I` and the rates of `Y`, `Z`, `D`
   and `I` to half the last digit, after an allowance of 64 units in the last place of the row's
   total field for both sides' rounding (7e-10 nT at 50,000 nT). `X` differs at 97 points, by up to
   7.18e-4 nT (row 35: 2026.5, 12 km, 33° N, 145° W), at most 2.11e-8 of the total field; `H`
   and `F` follow it, and `Z` moves by up to 2.2e-6 nT with it. The difference lies in the file's
   geocentric `X′`: one residue there, taken from `X`, brings the file's `Z` to 4.9e-7 nT of this
   code's, and this code's `X′` is the potential's derivative taken by differences to 1.3e-7 nT at
   all 100 points (a test), and the report's ten-decimal `X′` (Table 3b) to 5e-11 nT. pygeomag
   1.1.0 (MIT, a port of NOAA's `geomag` program), run once by hand on rows 3 and 35, gives this
   code's `X` to 7e-7 nT. The rate of `X` differs at 24 points, by up to 9.5e-7 nT a year, a
   second residue whose cause is not known; this code's `Ẋ′` passes the same derivative test, and
   the file's rates of `H` and `F` follow from its own `X` and rates to 1e-6. Those columns are
   held to their measured worst (7.2e-4 nT; 2.2e-6 nT; 1.5e-6 nT a year), 140 times below the
   0.1 nT the report allows single precision. The cause in NCEI's program is not known (an
   uncommitted check in review suggested the residue grows with `|X′|` and stays within about
   one single-precision step of `X′`).
6. **Compass zones by the report's thresholds.** `MagneticField::compass_zone` reports the
   blackout (horizontal intensity under 2,000 nT) and caution (under 6,000 nT) zones of section
   1.8, so a program can warn where a declination cannot be trusted.

**Consequences.** M5.3a is met, with NCEI's `X` column held to its measured residue rather than
its printing. Nothing in a flight uses the field yet: a heading given as magnetic is converted by
the caller with `MagneticField::true_from_magnetic_rad`. The model runs through 2030.0; WMM2030
will be a new table and a new `MagneticModel` constant. The height is above the
ellipsoid, as `Geodetic` holds it; a site's height above sea level differs by the geoid, which the
report puts at about 1 nT of field.

## ADR-126: M5.3b, Open-Meteo's elevation through the cache (2026-09-30)

**Context.** M5.3b is done when a recorded elevation lookup's height is the answer's and a second
lookup works offline from the cache (ADR-125 §1). Open-Meteo's elevation API
(`api.open-meteo.com/v1/elevation`) takes comma-separated latitudes and longitudes, up to 100
places, and answers `{"elevation":[...]}`, one height per place in order, or HTTP 400 with
`{"error":true,"reason":...}`. Its data is the Copernicus DEM GLO-90 (2021 release, 3″ in
latitude), a digital *surface* model (buildings and vegetation included; the dataset's readme),
whose heights are above the EGM2008 geoid (product handbook issue 5.0, §1.2.1, p. 13), stated
to under 4 m absolute, 90% linear error, a global mean outside Antarctica and Greenland (Table 1,
p. 10); 184 of the 16,363 geotiles there (each about a degree across) are over 10 m (Table 12,
p. 31). Its licence asks for
the credit "© DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under
COPERNICUS by the European Union and ESA; all rights reserved"; Open-Meteo's API data is CC BY 4.0
and it asks for a credit to Copernicus and to itself. The recorded heights are whole metres, and
Spaceport America's 1,400 m equals the `elevation` of the two weather recordings there.

**Decision.**

1. **A module beside the weather.** `hpr_net::elevation` mirrors `open_meteo`: an
   `ElevationRequest` builds the URL (1 to 100 places, latitude and longitude ranges, an optional
   self-hosted endpoint with no `?` or `#`), `parse` is pure and returns an `Elevation` (the place
   and `height_msl_m`) per place, and `fetch` goes through `Client::fetch_checked`, so an answer
   `parse` refuses is never cached. The URL writes each coordinate to 5 decimals (about 1 m),
   halves away from zero, trailing zeros dropped and zero unsigned: the cache key is the URL, and
   a place kept in radians comes back in degrees with other last digits (−106.91 as
   −106.91000000000001), which would miss its cached answer offline. Rounding the binary value
   straight to 5 decimals still flips about 1 in 17 six-decimal values on a half step (32.990415
   comes back as 32.990415000000006), so the value is first written exactly to 9 decimals and
   that decimal is rounded: a value given to 8 decimals or fewer keeps its key. The other sources
   still write `f64`'s shortest digits (#272).
2. **A year's TTL.** The DEM changes with a new release, years apart; a year keeps an old cache
   from going stale on the field while still refreshing.
3. **What `parse` refuses.** Another count of heights than places, a height that is not a number,
   and a height outside −1,000 to 9,000 m: the lowest land (by the Dead Sea, about −440 m) and the
   highest (8,849 m) with margin, so a 16-bit no-data value (−32,768) can't pass as a height. The
   ocean, which has no tiles, reads 0 m and passes.
4. **Heights above sea level, `N` left to the caller.** hpr has no geoid model, so the height is
   returned as the API gives it. The guide says to give a site `H + N` and the environment `N`
   when `N` is known, and otherwise `H` with `N = 0`, which keeps the atmosphere's height right and
   moves the site's place in space by `N` (gravity by about 3×10⁻⁴ m/s² per 100 m).
5. **Two recordings, no command.** `open-meteo-elevation.json` (Spaceport America) and
   `open-meteo-elevation-three.json` (with the Dead Sea's surface as the radar saw it, −427 m, and
   the open Atlantic, 0 m), recorded unchanged; the example `site_elevation` reads the second. A command-line lookup
   is left for later: the done-when is the library's.

**Consequences.** M5.3b is met. Nothing measures the DEM's accuracy against surveyed heights, and
nothing in a flight looks the site up by itself: a program passes the height to its `Geodetic`
site. Over trees or buildings the height is the surface's, not the pad's. The cache key is the
whole request, so a place looked up in a list is found offline only by the same list.

## ADR-127: M5.3c1, geodesics through GeographicLib, held to Karney's test set (2026-09-30)

**Context.** M5.3c asks for distance and bearing between two places on WGS 84, matched against a
published geodesic test set, and a site's height from a user's GeoTIFF, matched against another
reader's. The two share nothing: one is a pure model, the other a file reader. The method of
record is C. F. F. Karney, *Algorithms for geodesics*, J. Geodesy 87 (2013) 43–55
(arXiv:1109.4448v2): an auxiliary sphere with series in the flattening to `O(f⁶)` (§2) and
Newton's method for the inverse (§4, §5), whose round-off "in the direct and inverse methods are
less than 15 nanometers" (§7, page 10), with the truncation below round-off for `f` up to 1/150
(page 9). Karney publishes the set it was tested on, `GeodTest.dat` (doi:10.5281/zenodo.32156,
CC0): 500,000 WGS 84 geodesics in nine kinds (random, nearly antipodal, short, near a pole, and
so on), worked from exact `φ₁`, `α₁`, `s₁₂` with series to `O(f³⁰)` in high precision, each end
to 1e-18°. GeographicLib (MIT) is Karney's own code; georust's `geographiclib-rs` 0.2.7 (MIT,
2026-02 release, `libm` its one dependency without default features) ports it to Rust. Its
series grow inaccurate for large `f` (GeographicLib's table: 10 µm at 0.05, 0.3 m at 0.2; a
review measured the quarter meridian 0.32 m short at 0.2), and the inverse-then-direct round trip
can't see that, both halves sharing the series.

**Decision.**

1. **M5.3c splits.** c1: geodesics, done when distances and bearings match Karney's test set.
   c2: a site's height from a user's GeoTIFF, done when it matches another reader's. c1 first:
   no file format, and the oracle is a published file.
2. **`geographiclib-rs`, not a port.** It is Karney's algorithm line for line, maintained, pure
   Rust and wasm32-clean; a port would be the same code with our bugs. `hpr_core::geodesic` puts
   it behind `Ellipsoid::geodesic_inverse` and `geodesic_direct`, in radians and metres, heights
   ignored. Both return `Result`: an ellipsoid flatter than 1/150 (`GEODESIC_MAX_FLATTENING`,
   Karney's own limit) is refused, and so is a point built past `Geodetic`'s checks; the direct
   also refuses a non-finite azimuth or distance. A longitude outside `[−π, π]` is reduced modulo
   2π before its degrees could overflow. The direct returns latitude and longitude, not a
   `Geodetic`, so a height of zero can't pass as a site by accident; `GeodesicDirect::end` takes
   the height. Both result types are `#[non_exhaustive]`.
3. **What is measured, per kind, over the whole set, every line held to Karney's 15 nm.** The
   inverse's `|s₁₂|` error; where the direct problem from point 1 with the inverse's `α₁` and
   `s₁₂` lands, from point 2; the inverse's azimuth errors times `|m₁₂|` (the reduced length, so
   the sideways miss an azimuth error stands for at the other end), not measured on the
   "between vertices" kind, whose `|m₁₂|` is at most 1e-13 m; the direct's end-point miss in ECEF;
   and the direct's heading at the end, compared as a direction in ECEF (near a pole an azimuth
   turns through large angles as its point moves by nanometres; the heading does not) times `a`.
   Every per-line value is asserted finite.
4. **Mirror lines take either pair.** When `φ₂ = −φ₁` exactly and `α₁ ≠ α₂`, two geodesics of
   the same length join the points, the second with `α₁` and `α₂` swapped (GeographicLib's
   `GeodSolve` manual, *Multiple solutions*); where `α₁ = α₂` (the 50,000 between-vertices
   lines) the geodesic is unique. 21 lines of the set are mirror lines once read as `f64`, all
   nearly antipodal with `m₁₂` under a centimetre, so their azimuths are nearly undetermined; on
   4 the answer is nearer the swapped pair (2 of them were first misread as an ill-conditioned
   excess of 75 nm). The test takes the smaller error of the two pairs on those lines only; with
   it every line is within 15 nm.
5. **Where it runs.** Every 500th line (1,000) and the 21 mirror lines are committed and checked
   in CI; the whole file is pinned in `validation/refs.lock.toml` and checked where fetched. The committed
   table, `validation/reports/geodesics.md` (rewritten by `HPR_WRITE_GEODESICS=1`), is compared
   only on the build that wrote it, a debug build on macOS aarch64: its cells are a few ulps of
   ECEF coordinates and a release build moves three by up to 1.8 nm. Elsewhere only the bound is
   held.

**Consequences.** M5.3c1 is met: the largest errors over the 500,000 lines are 11.18 nm in
distance, 11.26 nm landing, 8.49 nm in the inverse's azimuths times `m₁₂`, 14.02 nm in the
direct's end point and 13.99 nm in its heading times `a`. Multiplying `a` by `1 + 1e-15` fails
CI's sample at 26.08 nm. Nothing in a flight uses geodesics yet, and `hpr` has no command for
them; the landing distance a flight prints is still a flat offset from the pad. The set is
WGS 84 only, so other ellipsoids up to 1/150 rest on Karney's method, not on a measurement here.

## ADR-128: M5.3c2, a site's height from a user's GeoTIFF, held to rasterio's reading (2026-09-30)

**Context.** M5.3c2 asks for a site's height from an elevation file the user gives, matched
against another reader's. Elevation files (DEMs) mostly come as GeoTIFF: a TIFF image of heights
whose tags tie pixels to places (OGC GeoTIFF Standard 1.1, OGC 19-008r4, 2019). The USGS's 3D
Elevation Program (public domain), Copernicus and SRTM publish GeoTIFFs on a latitude and
longitude grid; the USGS's 1 m lidar comes projected (UTM). GDAL is the reader nearly every other
program uses, and rasterio 1.5.2 (BSD-3-Clause) wraps it, its wheel bundling GDAL 3.12.2. GDAL
reads some files differently from the standard (`frmts/gtiff/gtiffdataset_read.cpp`, MIT, read
at v3.12.2): it takes a negative `ScaleY` as north-up, prefers the pixel scale when a matrix is
also present, and turns `S_z` and the tiepoint's heights into a band scale and offset when the
file has a vertical CRS, as it does the `scale` and `offset` items of its `GDAL_METADATA` tag.
The TIFF layer (codecs, predictors, tiles, strips, BigTIFF, byte orders) is large; image-rs's
`tiff` 0.11.3 (MIT, 125 million downloads, 2026-02 release) decodes it in pure Rust, its
floating-point predictor included.

**Decision.**

1. **`hpr_io::geotiff`, over the `tiff` crate.** Its LZW and Deflate codecs only (PackBits and
   no compression are built in); JPEG, fax, WebP and zstd stay off, so no C code and nothing that
   stops wasm32. A file using another codec is refused at `parse`, naming it. The GeoKeys, the
   georeferencing and GDAL's tags are read here.
2. **Geographic CRSs near WGS 84 only.** `GTModelTypeGeoKey` 2, or a geodetic CRS key with no
   model type (GeoTIFF 1.0 writers), in degrees from Greenwich, and an EPSG code in
   `NEAR_WGS84`: WGS 84, NAD83 and its realisations, ETRS89, GDA94, GDA2020, NZGD2000, JGD2011,
   SIRGAS 2000, CGCS2000, all within a few metres of WGS 84 (plate motion since each was fixed),
   except near the rupture of a large earthquake since (several metres: Chile 2010 for SIRGAS
   2000, Wenchuan 2008 for CGCS2000, for example). JGD2000 is out: Japan's 2011 earthquake moved its
   north-east more than 5 m, and JGD2011 replaced it. A point's WGS 84 coordinates are read in the file's datum unchanged. Anything else (a
   projection such as UTM, NAD27 at about 55 m in New Mexico, Tokyo at hundreds) is refused naming
   its code, with the `gdalwarp -t_srs EPSG:4326` that converts it; datum shifts and projections
   would be models of their own, for later if users ask.
3. **The containing pixel, as GDAL places it.** A point reads the pixel whose area holds it, with
   no interpolation: GDAL's and rasterio's point sampling. The corner is GDAL's arithmetic
   (`X − I·S_x`, `Y − J·(−S_y)`, half a pixel back for pixel is point), so corners and pixel sizes
   come out bit for bit as GDAL's; the column is `⌊(λ − λ₀)/Δλ⌋`, the floor of a correctly rounded
   quotient. rasterio's `index` inverts the transform instead, so a point within about 1e-13 of a
   pixel of an edge can land on the other side of it; the tests sample random points, none on an
   edge. A longitude is tried as given and 360° to either side. One tiepoint with a pixel scale,
   or an unrotated matrix (its terms `a, d, f, h`). Refused, where GDAL and the standard differ or
   GDAL reads more: a negative `ScaleY`, a pixel scale beside a matrix (the standard forbids it),
   rotation, ground control points, and an internal mask image (GDAL's nodata mask).
4. **Heights: GDAL's scale and offset, then the file's unit.** A raw value `v` is
   `(v·scale + offset)·unit` metres. The scale and offset are GDAL's: `S_z` and `Z₀ − z₀·S_z` when
   any of the three is non-zero and GDAL reads a vertical CRS, else `GDAL_METADATA`'s first-band
   items, else 1 and 0; both sources disagreeing, or a scale of 0, are refused. Whether GDAL reads
   a vertical CRS turns on the directory's revision and on how GDAL and PROJ resolve the keys, so
   `S_z` applies only where that is certain: a GeoTIFF 1.1 directory of model type 2 naming a
   vertical CRS from the short list of EPSG codes below, with no `VerticalDatumGeoKey` and a
   geographic CRS other than WGS 84 3D (GDAL's `gt_wkt_srs.cpp`, with model type 2, drops the
   vertical CRS for a private key value and for datum 6030 beside WGS 84, and the whole CRS
   beside WGS 84 3D; with no model type it builds a local CRS, with a vertical part only when
   there is a unit key). It is ignored, as GDAL ignores it, in a 1.0 directory (rasterio shows
   GDAL dropping the vertical CRS there) and where no vertical key is present. Otherwise a file with
   heights in its tags is refused, unless they give GDAL's own scale 1 and offset 0 and
   `GDAL_METADATA` gives no other scale. GDAL matches `GDAL_METADATA` with quirks
   (`gtiffdataset_read.cpp`: attribute names in any case, compared with their prefix; C's `atoi`
   for the sample; text only as an item's one child, CDATA a child of its own; only ASCII blanks
   skipped). An item with a scale, offset or `unittype` role is therefore refused if it has a
   namespace, a capital in an attribute's name, a sample that isn't plain digits, a value that
   isn't a single text node, or the `IMAGE_STRUCTURE` domain; element names in either case and
   other domains read as GDAL reads them, and the skips GDAL makes (no name, no sample, another
   band, another root) were measured through rasterio and are made.
   The unit is `VerticalUnitsGeoKey` (metres, feet, US survey feet; refused if it disagrees with
   the vertical CRS's, which GDAL takes instead with model type 2), else the unit of a vertical CRS from a short
   list of EPSG codes (`VERTICAL_CRS_UNITS`: EGM2008, EGM96, EGM84, ODN, MSL, NAVD88 in metres, feet
   and US survey feet), else `GDAL_METADATA`'s `unittype` (refused if it disagrees with the keys,
   or names another unit; read after trimming ASCII blanks, of which GDAL drops only leading ones
   typed as they are); a file
   naming none is read as metres, flagged by `vertical_unit_stated` (GDAL reports no unit, except
   beside a vertical datum key alone, where it assumes metres too). A vertical CRS off the list is
   refused, with or without a unit key: with model type 2 GDAL takes its unit from EPSG's
   registry, which hpr doesn't hold. A user-defined one (32767, reported as no code) is read with a unit key and
   refused without. Vertical keys GDAL drops, unit and all, or reads by rules of its own, are
   refused (with model type 2, hpr would read a unit GDAL doesn't report; with none, GDAL reads
   the unit key alone and the refusal is conservative): a private value (above 32767) in any of
   them (dropped with a model type, read without one), any beside WGS 84 3D, datum 6030 beside
   WGS 84 with model type 2 (GDAL makes it WGS 84 3D), and any with no model type and no unit
   key. A blank `GDAL_METADATA` value written as a character reference, which GDAL reads as 0 or
   a blank unit, is refused.
   The vertical datum is reported, not applied. Nodata is `GDAL_NODATA` rounded to the
   sample type (a value an integer can't hold matches nothing); NaN is no data too.
5. **Bounded on a hostile file.** `height_at` decodes only the tile or strip holding the point;
   `parse` refuses a tile or strip over 256 MiB decoded (`MAX_CHUNK_BYTES`: the `tiff` crate's own
   chunk limit, which its padding of a floating-point tile bypasses) and a chunk count past a u32.
   The tile size is multiplied in u128, as a u64 product can overflow. `values` grows its output
   fallibly a row of tiles at a time, as they decode, up to 2²⁸ pixels. `GDAL_METADATA` nested
   past 16 deep is refused before its XML is parsed.
   `values_at` decodes each needed chunk once and fails only the points in a tile that won't
   decode. The `tiff` crate prints one `dbg!` line to standard error when a tag's value passes its
   1 MiB limit; that is its code, left as it is.
6. **The oracle.** `validation/oracles/geotiff/dem.py` cuts 70 by 50 pixels around Spaceport
   America from USGS 3DEP's tile n33w107 (pinned in `refs.lock.toml`) into seven encodings: float32
   LZW with the floating-point predictor on 16-pixel tiles; int16 Deflate with the horizontal
   predictor in 7-row big-endian strips; float64 uncompressed BigTIFF, pixel is point; uint16
   PackBits in NAVD88 US survey feet with nodata holes; uint32 centimetres with a scale and offset
   in `S_z` and EGM2008; uint8 quarter feet with a scale, offset and unit in `GDAL_METADATA`; int32 LZW on 32-pixel
   tiles with longitudes past 180°. It records rasterio's reading: the transform, the CRS, the
   vertical CRS and unit, the scale, offset and band unit, the nodata value, `math.fsum` of every value and
   of each value times its index, and 400 seeded points per file (the site, the rest random, some
   off the raster) with rasterio's pixel and value, each value checked against rasterio's own
   `sample`. The whole tile gets the same with 2,000 points. `tests/geotiff_rasterio.rs` holds hpr
   to all of it exactly, and every 97th point's height to rasterio's scale and offset applied to
   its value; the whole tile runs where `refs/` has it.

**Consequences.** M5.3c2 is met: all seven fixtures and the whole 3,612 by 3,612 tile read to
rasterio's corners, pixel sizes, scales and offsets bit for bit, to its two sums exactly (13
million values in the tile), and at 4,800 points (4,064 on a raster, 9 of them nodata) to its
pixel and value. Removing the pixel-is-point shift fails the float64 fixture's corner. At
Spaceport America (32.99° N, 106.97° W) the tile reads 1,400.691 m, which the USGS states is
above NAVD88; Open-Meteo's answer there (ADR-126) is 1,400 m. M5.3 is complete. `hpr` has no
command for a site's height yet, and no flight reads one from a file; a projected file, or one on
an older datum, needs `gdalwarp` first. Review found the first draft's gaps: a tile header that
could ask for 32 GiB, GDAL's scale and offset unread (a decimetre file read 10× high), any datum
accepted. A second round found a tile size that overflowed a u64, `S_z` applied where GDAL
would not, JGD2000 in the near list and unbounded XML nesting; a third, GDAL's metadata matched
more narrowly than GDAL matches it, and a unit key read where GDAL takes the vertical CRS's; a
fourth and fifth, gaps of the same kinds (metadata quirks, an unlisted vertical CRS's unit,
GDAL's own `S_z` 1 beside another metadata scale, vertical keys GDAL drops), answered by failing
closed. All are fixed and tested
above.

## ADR-129: M5.4 split, and M5.4a, the motor finder's API through the cache (2026-10-01)

**Context.** M5.4 asks for a motor.fusionspace.co client (`meta`, `motors`, `in-stock`,
`vendors` and per-motor endpoints) joined with ThrustCurve's curves, an offline snapshot, and
`hpr motors search --in-stock --class L --max-price 150`; done when recorded-fixture tests pass,
the designation to ThrustCurve id mapping covers 95% of in-stock motors with a report of the
misses, and attribution is displayed as the API asks. That is three pieces of work: a client, a
second client and a join, and a command. The site's API (documented in the public MIT repository
`nrdptel/Hobby-Rocket-Motor-Finder`, `docs/api.md`) is static JSON on a CDN, rebuilt about
hourly, with no key, rate limit or query parameters; `cache-control: public, max-age=600`. Every
file carries `schema_version` 1 and `generated_at`; a breaking change ships under `/api/v2/`.
One motor's file is `motors/{aerotech|cesaroni|loki}/{designation}.json`, a `/` in the
designation written `~`; an unknown one is a 404 HTML page. Its terms: "Free to use; attribution
to motor.fusionspace.co is appreciated. The data is aggregated from public vendor listings and
ThrustCurve; it's provided as-is, with no warranty — verify stock and price on the vendor's own
page before relying on it." Motors carry the finder's own `id`, not ThrustCurve's, but its
designations are "verbatim as ThrustCurve spells them": on 2026-10-01 all 598 listed motors
match exactly one ThrustCurve record on (manufacturer, designation), as all 598 did on
2026-09-17's snapshot.

**Decision.**

1. **Split a to c.** M5.4a, the finder's five files through the cache; M5.4b, ThrustCurve's
   search and a motor's curve through the cache, and the join with its report of misses; M5.4c,
   `hpr motors search`, from the network, a recorded snapshot or the cache, showing the credit.
   The parent's done-when is unchanged and met by the three.
2. **`hpr_net::motor_finder`,** shaped as the other sources: `Endpoint` builds each URL; pure
   parsers `parse_meta`, `parse_motors`, `parse_in_stock`, `parse_vendors` and `parse_motor`; and
   `fetch_*` through `Client::fetch_checked`, so an answer that doesn't parse is never cached. The
   types mirror the API's JSON field for field (prices `u64` cents, impulse and thrust `f64`, the
   listing status, motor type and hazmat as enums), so a value read is the answer's and writes
   back to it, bar a listing status added later (below). Unknown fields are ignored (the API may
   add some under v1). The cheapest offer's prices are optional, as the API's OpenAPI schema has
   them, though no recorded offer lacks one; a listing status the API adds later reads as
   `Unknown` (its word is lost), so one new word doesn't refuse the list; a stock count is read as
   the vendor shows it, signed. An unknown motor type or hazmat label is refused (ThrustCurve's
   metadata lists the three types). `fetch_motor` returns the page, with its build time.
3. **Refused: the structural rules; pinned: the derived ones.** The parser refuses another schema
   version, a build time that isn't UTC ISO 8601, a list whose `count` disagrees, an impulse
   class that isn't one capital letter, a diameter not above zero, a negative impulse, thrust or
   burn time, a `listing_count` other than the listings', a cheapest offer on a motor out of stock
   or none on one in stock, a pack of zero, a unit price over its sticker price, a motor out of
   stock in `in-stock.json`, and a page holding another motor than asked. The rules the site
   derives (the unit price is the sticker over the pack, rounded half up; the cheapest offer is
   the lowest-priced in-stock listing; in stock exactly when a listing is; distinct-vendor counts;
   each motor's `path`) are tested on the recording, not enforced: a change in how the site
   computes them would otherwise refuse the whole catalogue. A value that breaks a checked rule
   refuses its whole file: the last good copy is kept, served stale online with the reason.
4. **A manufacturer and designation.** `Endpoint::motor` takes the API's three manufacturers by
   name or slug, any case, and a designation of ASCII letters, digits, `-`, `_`, `.` and `/` (the
   characters ThrustCurve's designations use), not empty and not all dots, so no request leaves
   the API's `motors/` folder; anything else is refused before the client is asked. The name is a
   `MotorName` with private fields, so no caller builds or changes one past the checks.
5. **An hour's TTL,** the site's rebuild interval; offline, or with the site down, the cached copy
   is served stale, as every source does (ADR-117).
6. **The credit.** `ATTRIBUTION` names motor.fusionspace.co, its two sources and its caution to
   check the vendor's page; it is on every `Fetched`, and the guide page and the example print
   it first. The API asks for credit but says nothing on where; M5.4c will print it with every
   listing, in text and JSON.
7. **Fixtures.** Eight answers of one build (2026-10-01 07:07:29 UTC) are committed under
   `crates/hpr-net/tests/fixtures/replay/`: the four lists (`motors.json` 1.6 MB, `in-stock.json`
   0.96 MB) and four motors' pages, one per manufacturer and one with a `/` (`F27R~L.json`,
   stored as `F27R_L` for file systems). The site is Neer's own, its terms say free to use, and
   its listings are public vendor pages; the motor figures in them are ThrustCurve's published
   values, which the bundled catalogue already copies with attribution (ADR-005). Recorded under
   "Needs Neer" as no action if fine.

**Consequences.** M5.4a is met: `tests/motor_finder.rs` reads each of the eight answers, writes it
back and finds exactly the recording's keys and values; reads it again from the cache without a
fetch, and offline, through a transport that fails if called, fresh for the hour and stale after;
and finds `ATTRIBUTION` on every answer. A typed test pins the enums' meanings (H128W a reload
shipped as hazardous material; 827 listings in stock, 2,363 out, 495 special order), since a
round trip through the same names can't. Review found the first draft refusing the whole list
for an offer with no price, which the API allows; an enum whose names could be swapped unseen;
several refusals untested; and a page fetch that dropped its build time. All are fixed. The files agree with each other (`in-stock.json` is
`motors.json`'s 282 motors in stock, value for value; `meta.json` counts 598, 282 and 12), each
refusal above is tested by a one-field change to a recording, and the derived rules hold on all
598 motors and 3,685 listings. On that build, 20 L motors were in stock and none sold for $150
or less a motor (the cheapest, $260.99), so M5.4c's example command lists nothing on the
recording; its test will need a price that does. The join (M5.4b) starts from the exact match
above, with ThrustCurve's whole `search.json` (1,156 motors, 0.93 MB, its terms unstated) kept
under `refs/` unless a smaller recorded answer can carry the test.

## ADR-130: M5.4b, ThrustCurve searches and curves through the cache, and the in-stock join (2026-10-01)

**Context.** M5.4b (ADR-129 §1) asks for ThrustCurve.org's search and a motor's curve through the
cache; done when the designation to ThrustCurve id mapping covers at least 95% of the motors in
stock, with a report of the misses, and a mapped motor's recorded curve reads with `hpr_motor`.
ThrustCurve's API (`/api/v1`, an OpenAPI 2 spec under the ISC licence; its page says the JSON
endpoints take a query string by GET or a JSON body by POST, and no key or header) has a search,
whose records carry ThrustCurve's 24-hex-digit `motorId`, and a download, whose files come
base64-encoded with their format (`RASP` or `RockSim`), source (`cert`, `mfr`, `user`) and licence
(`PD`, `free`, `other`, or absent). Its spec says "only fields with values will be returned". The
API states no terms for its data. Errors come back as HTTP 200 with an `error` field, on the
answer or on a search criterion; an unknown motor id downloads as an empty list. On 2026-10-01 a
search by maker with `maxResults=5000` returned AeroTech's 307 records, Cesaroni Technology's 296
and Loki Research's 60, each whole (`matches` equal to the records returned), and the same records
whether the maker was named in full or by its abbreviation.

**Decision.**

1. **`hpr_net::thrustcurve`,** shaped as the other sources: `Search` and `Download` build the URLs
   (GET, query values percent-encoded, fields in a fixed order so a URL is one cache key);
   `parse_search` and `parse_download` are pure; `fetch_search` and `fetch_download` go through
   `Client::fetch_checked`, so an answer that doesn't parse is never cached. The types mirror the
   JSON field for field and write back to it; every record field but `motorId`, `manufacturer` and
   `designation` is optional, as the spec allows. The motor type, availability, source and licence
   stay the API's words (strings: the response schema types them as open strings); the format is
   an enum of the two the request takes, and the answer must hold the one asked. A request's motor
   id is taken in either case and kept in lower case, as the API writes ids, so one motor has one
   cache key and its answer matches the request.
2. **Refused:** an answer or criterion carrying `error`; a search returning more records than
   `matches`; a `motorId` not of 24 hex digits; a size, thrust, impulse, burn time or weight below
   zero; a file's data that isn't base64; a download holding another motor's file or format. A
   request's motor id is checked before the client is asked. A file that decodes but isn't UTF-8
   (`hpr motors` reads motor files as UTF-8 too), or that `hpr_motor` refuses, is an error of
   `DataFile::text` or `DataFile::read` for that file alone, not of the parser: the answer is
   still ThrustCurve's, and the motor's other files stay readable.
3. **The join by name only.** `join` maps a finder motor to the one record whose `manufacturer`
   (full name) and `designation` equal the motor's, byte for byte; none or several is a `Miss` with
   its reason, and `Join::report` lists counts by maker and every miss. A record given twice (the
   same `motorId`, as from overlapping searches) counts once. A `Mapped` carries the finder
   motor's index and the whole record, for M5.4c to show price beside curve. No fallback on case,
   punctuation, impulse or diameter: the finder copies ThrustCurve's spellings (ADR-129), and a
   guessed match could hand a flier the wrong curve, where a miss only costs a lookup. As the
   finder copies ThrustCurve's figures too, a full match was expected on the recording (ADR-129
   found 598 of 598 exact); the join is there to catch drift.
4. **Whole searches for the join.** `fetch_finder_records` runs one search per maker the finder
   reads (`motor_finder::MANUFACTURERS`, by full name), each asking `maxResults=5000`, and refuses
   (and doesn't cache) one that matches more records than it returns, or holds another maker's
   record, so a join never runs on part of a maker.
5. **A day's TTL.** Records and curves change seldom; offline or with the site down, the cached
   copy is served stale, as every source does (ADR-117).
6. **The credit.** The API asks for none. `ATTRIBUTION`, "Motor data and thrust curves courtesy of
   ThrustCurve.org", matches the bundled curves' notice (ADR-005) and is on every `Fetched`. A
   file's licence is passed through, not filtered: the caller decides what to keep or pass on.
7. **Fixtures.** Five answers recorded 2026-10-01 08:22 UTC are committed under
   `crates/hpr-net/tests/fixtures/replay/`: the three makers' searches (250, 246 and 50 kB) and
   two downloads, AeroTech J450DM's RASP file (certification data, `PD`) and AeroTech F27R/L's
   RockSim file (`user`, `PD`), requested by format so no file without a public-domain licence is
   recorded. The records are published motor figures, as the bundled catalogue already copies
   (ADR-005); ThrustCurve's whole `search.json` (1,156 records) stays under `refs/`, the three
   makers' searches being enough for the join.
8. **The report.** `validation/reports/thrustcurve-join.md` holds `Join::report` on the recordings,
   compared as text on every OS by `tests/thrustcurve.rs` (counts only, no float);
   `HPR_WRITE_THRUSTCURVE_JOIN=1` rewrites it.

**Consequences.** M5.4b is met: on the recorded in-stock list (282 motors, the finder's build of
07:07:29 UTC) and the three searches (663 records), 282 of 282 motors map to exactly one record,
each listing a data file, against the 95% asked. The test builds its own name-to-id table from the
recordings' JSON, not through `join` (the same rule, so it shows the code keeps the rule), and,
independently of the rule, checks that every match's diameter, total impulse, average thrust and
burn time equal the finder motor's: they do for all 282. J450DM, mapped and in stock, has its recorded RASP file read
by `hpr_motor::eng` to the 36 points in its lines, and the file is byte for byte the public-domain
one `hpr_motor` bundles (`5f4294d20002e9000000086b.eng`, downloaded 2026-09-17); F27R/L's RockSim
file reads by `hpr_motor::rse` to the points in its XML. A renamed motor, a doubled record and a
designation in another case are misses with their reasons; a record given twice counts once, and
another maker's record of the same designation leaves the match alone. Each refusal is tested by a
one-field change to a recording. Review found an upper-case id accepted and then its answer
refused, a record given twice making every motor a miss, a search of another maker accepted, one
non-UTF-8 file refusing a motor's whole answer, and gaps in the refusal tests; all are fixed. The example `motor_stock` prints the join and J450DM's curve: 1,061.6 N·s,
2.28 s and 541.4 N from the file, beside its record's published figures of 1,055 N·s, 2.27 s
and 558 N. M5.4c (`hpr motors search`) can now show a motor in stock with its curve.

## ADR-131: M5.4c, `hpr motors search`, stock and prices at the command line (2026-10-01)

**Context.** M5.4c (ADR-129 §1) asks for `hpr motors search --in-stock --class L --max-price 150`,
listing from a recorded snapshot and offline from the cache, with the credit shown as the API
asks. The library reads motor.fusionspace.co's lists (`hpr_net::motor_finder`, ADR-129) and joins
them to ThrustCurve.org's records (`hpr_net::thrustcurve`, ADR-130). On the recorded build
(2026-10-01 07:07:29 UTC), 20 L motors were in stock and the cheapest sold for $260.99 a motor, so
the example lists nothing on the recording.

**Decision.**

1. **A third subcommand of `hpr motors`,** in `crates/hpr-cli/src/motor_search.rs`. The list comes
   from the network through the platform's cache (`Http`, an hour's TTL, as ADR-129 §5), from the
   cache alone with `--offline`, or from a saved `motors.json` or `in-stock.json` with `--from`,
   which touches neither; `hpr weather`'s client and its read-from line are shared, not copied.
   `--in-stock` or `--max-price` (which keeps only motors in stock) fetches `in-stock.json`
   (0.96 MB), and otherwise `motors.json` (1.6 MB). Offline, a search of motors in stock with no
   copy of `in-stock.json` reads `motors.json`'s copy, which answers it too; with neither, the
   refusal names `in-stock.json`. A saved file is read with `parse_motors`, whose checks a fetched
   answer passes too; `--in-stock` then filters it, so either file serves any search.
2. **Five filters, each refused before anything is read when it can't match anything real:**
   `--in-stock`; `--class` as `hpr motors list` reads it; `--diameter` within 0.5 mm, as `list`;
   `--manufacturer` by the API's three names or slugs, in any case (`Cesaroni` is a slug); and
   `--max-price` in U.S. dollars, read as exact cents (`149.99` is 14,999 cents, never a float just
   under it; at most two decimals; no sign, `$` or exponent). The price compared is the site's
   `cheapest_in_stock.unit_price_cents`, one motor's price at the cheapest vendor with it in
   stock: a motor out of stock has none, so `--max-price` keeps only motors in stock, and an offer
   in another currency, or with no price, never passes. The site derives `cheapest_in_stock`
   itself (ADR-129 §3, tested on the recording); hpr doesn't recompute it from the listings, so a
   cheapest offer in another currency would hide a dearer one in dollars. Every recorded offer is
   in dollars.
3. **Cheapest first,** by that price, then by maker and designation; motors with no price in
   dollars last, by maker and designation. A search for the cheapest motor is the command's use.
4. **Both credits on every list,** empty or not: the finder's `ATTRIBUTION`, with its caution,
   then ThrustCurve.org's, whose published figures the finder repeats. Text prints them on lines 3
   and 4, under the count and where the list was read from; JSON carries them in `attribution`.
   ADR-129 §6 promised the finder's credit with every listing, in text and JSON.
5. **An empty `--max-price` search says why:** a last line names the cheapest motor in stock the
   other filters keep, and its price, so the example's empty answer still answers "what does an L
   cost?"; counted in stock, it reads the same from either file. Text only; the JSON's motors are
   the answer. Text cells from the site (designations, makers, vendors, currencies) print with
   control characters as `?`, so a scraped escape sequence can't reach the terminal.
6. **No ThrustCurve.org lookup in the command.** The figures shown are the finder's (ThrustCurve's
   published values); matching to ThrustCurve.org records and downloading curves stays in the
   library (ADR-130), which a program calls, until a command to fetch a curve is asked for.
7. **The example's test.** On the recording, the example lists nothing, and the hint names AeroTech
   L1520T at $260.99. The test then edits a copy of the recording in a temporary folder, L1520T's
   cheapest offer and its matching listing at $149.99, and the same command lists exactly that
   motor; at `--max-price 149.98` it lists none. The committed recording is not edited.

**Consequences.** M5.4c is met, and with it M5.4: `tests/motors_search.rs` runs the command as a
user does. From the recorded in-stock list, `--in-stock --class L --max-price 300` lists 4 motors
and the example none, each output checked against `schema/cli/motors-search.schema.json`. Every
listed motor's values equal the recording's, and the motors and their order equal what the test
finds by filtering the recording's JSON itself, on seven searches over both lists (282 motors in
stock, 598 in all, 55 of class L, 20 of them in stock; 81 of 75 mm, and 94 at 75.5 mm, the
tolerance's edge). An edited copy with one L offer in Canadian dollars and one with no price shows
both left out of `--max-price` and listed last. Offline, from a cache filled through the
recorded answers, three searches list what the saved file does, read from the cache; with an empty
cache both lists are refused, naming their URLs; with one list cached, the searches it can answer
read it. Each refusal is tested, before and after a file is read. Review found the
non-dollar and unpriced paths and the diameter's tolerance untested, an offline search refused
with the other list cached, the hint counting motors out of stock from the whole list,
`--max-price` taking the next option as its value, and site text printed raw; all are fixed. The guide's [CLI page](cli.md#motors-you-can-buy) runs both examples through
`cargo xtask cli`. Fetching online was not run; it is the same `Client` and `Http` as `hpr weather`,
which were run by hand on 2026-09-30.

## ADR-132: M5.5a, OpenRocket's `.orc` parts catalogues, held to OpenRocket's reading (2026-10-01)

**Context.** M5.5 asks for OpenRocket's `.orc` component database, looked up by vendor and part
number, with parts usable from the design API; done when "all `.orc` files parse, and a design
built from catalog parts simulates". The database is `openrocket/openrocket-database`
(Apache-2.0), pinned in `refs.lock.toml` at `1512874a` (2025-07-27). OpenRocket 24.12's jar ships
its 16 files byte for byte. The format has no schema; the project's `docs/TechnicalInfo.md` lists
fields and units, and says a shape parameter "cannot be specified" and a material must be defined
in the same file. A survey of the files: 3,449 parts of ten kinds, 402 materials, lengths in `in`,
`mm`, `cm` and `ft`, masses in `oz`, `g` and `kg`; 3 materials named but not defined; 21 part
numbers naming two parts; no shape parameters.

**Decision.**

1. **Split a and b.** M5.5a reads the catalogue (this ADR); M5.5b builds parts from it in the
   builder and flies a rocket of them, each part's built mass held to OpenRocket's for the same
   preset.
2. **The 16 files are bundled unchanged** in `crates/hpr-io/data/openrocket-database/` with the
   project's `LICENSE`, compiled in by `include_str!` (2.1 MB of text; `hpr_io::orc::bundled`
   reads them once), and listed under Bundled in `THIRD-PARTY-NOTICES.md`. Apache-2.0 allows it
   with the licence and the notices kept; the files carry no `NOTICE`. `.gitattributes` keeps
   their bytes. The `refs/` copy stays the pinned source; the oracle checks the jar's copies equal
   the bundled ones.
3. **OpenRocket's own reading is the oracle.** `validation/oracles/openrocket/orc_presets.py`
   runs OpenRocket 24.12's public `OpenRocketComponentLoader.load` on each file (run, never read)
   and records every value of every preset (`ComponentPreset.ORDERED_KEY_LIST`) to
   `crates/hpr-io/tests/fixtures/orc/openrocket-presets.json` (1.5 MB, one part a line), with each
   part's densities as OpenRocket reads the file with every `<Mass>` removed.
   `tests/orc_openrocket.rs` holds hpr's reading to it, part by part in order, every value to
   the bit. Where the two differ the test counts each departure and checks its cause. The script
   also has OpenRocket read 37 probe catalogues, each asking one question (every documented
   unit, values with no units, a part or file it can't read, a material or a list stated twice),
   and records each probe's text with
   OpenRocket's parts or refusal; the same test reads each probe and names what hpr does.
4. **Units are their exact definitions** (NIST Handbook 44, Appendix C), so hpr departs from
   OpenRocket's rounded factors: its ounce is 0.0283495231 kg against the exact 0.028349523125 (185
   masses differ by 8.8e-10 of their value); the fixture's probes show `lb/ft³`, `oz/in²`, `oz/ft²`,
   `lb/ft²` and `oz/ft` rounded the same way, none used by the bundled files. Values with no `Unit`
   are SI, and a density with no `UnitsOfMeasure` is SI, as OpenRocket reads them (probes). `g/m2`
   is read as written, as OpenRocket does, though six ripstop nylons labelled so (five in
   `generic_materials.orc`, unused; Giant Leap's, whose six canopies state their mass) are plainly
   kg/m² (0.067 g/m² is no fabric); a surface density under 1 g/m² warns.
5. **The file's values are kept.** The maker is the file's (OpenRocket shows "LOC Precision" as
   "LOC/Precision" and "Public Missiles" as "Public Missiles, Ltd.": 252 parts). A stated `Mass` is
   kept in `Part::mass_kg` and the material keeps the file's density; OpenRocket instead gives a
   bulk part its stated mass by replacing the density (207 parts). The cause is shown on all 207:
   with every `<Mass>` removed, OpenRocket's density equals hpr's on every part, and it differs
   exactly on the bulk parts stating a mass; on the 54 simple solids among them (7 body tubes, 4
   bulkheads, 34 filled conical noses and 9 transitions, shoulders as solid cylinders) the replaced
   density times the closed-form volume is the stated mass to 1e-15 (largest 5.9e-16). M5.5b makes a
   built part's mass the stated one. A material the file doesn't define has no density (`None`)
   where OpenRocket gives it zero (3 parts). A field stated twice keeps the last, as OpenRocket does
   (3 descriptions).
6. **Strict per part, lenient per file.** Only text that isn't XML, whose root isn't
   `<OpenRocketComponent>`, or nested past `MAX_DEPTH` (16, refused before it is parsed:
   `OrcError::TooDeep`) is refused. A part with a missing, unreadable or negative dimension, a unit
   or shape the format lacks, a material of the wrong kind, a value holding an element, or an
   unknown element is left out with a warning. OpenRocket 24.12 throws on the whole file for a
   missing dimension, an unknown unit or shape (probes); it reads an unreadable number (`ten`) as
   zero, a material of the wrong kind with a density of zero, and `BT<b>-</b>20` as `20` (the text
   after the last element, a quirk not copied). An unknown field is ignored with a warning
   (OpenRocket ignores it silently): the 37 nose cones that state an `InsideDiameter` read that
   way. `in/64` is refused: OpenRocket reads it as inches. As OpenRocket does (probes), a list
   stated twice keeps the last and a material defined twice keeps the first, each with a warning.
   Implausible values read as written with a warning: an inside diameter not under the outside, a
   solid under 1 kg/m³, a fabric under 1 g/m². A density below zero or past `f64` is left out.
   Warnings carry a `WarningKind` and stop at `MAX_WARNINGS` (1,000) with a count of the rest;
   `read` returns a `CatalogFile`. Materials are found through an index by kind and name, so a
   file's reading stays linear in its size.
7. **Lookup.** `Catalog::find(maker, number)` matches the whole part number exactly and the maker
   in any case, both trimmed, and returns every match (21 numbers name two parts, 3 pairs
   identical). Numbers are
   often several in one (`BT-20, 30316`); pieces are ambiguous (`White` names five parts), so
   `Catalog::search` matches text in numbers and descriptions instead.

**Consequences.** M5.5a is met: the 16 bundled files read; every part OpenRocket reads is read, in
its order, 3,449 parts; 17,911 values equal OpenRocket's to the bit, and the departures are exactly
185 ounce masses, 252 makers' names, 207 derived densities and 3 undefined materials, of 18,306
sizes, masses and densities compared (the parachutes' counts of sides and lines are compared too).
The reader's warnings on the bundled files are 55: 37 nose cones' inside diameters, 3 doubled
descriptions, 3 undefined materials, 3 tube-like parts no narrower inside than out, 3 solids lighter
than air (18 paper rings and blocks would weigh a millionth of real ones) and 6 fabrics under
1 g/m². Moving the inch one float step fails the oracle test. Of the 37 probes, hpr reads 23 as
OpenRocket does to the bit, 6 within 2e-9 by its exact units, and leaves out what OpenRocket can't
read (4 it refuses whole, `oz/in` among them, which hpr reads with the cord's density undefined) or
reads oddly (4: `in/64`, `ten`, a material of the wrong kind, an element inside a part number).
Nothing flies a catalogue part yet (M5.5b), and no shape parameter or shoulder wall is chosen for
one: the file gives neither.

## ADR-133: M5.5b, catalogue parts in the builder, weighed as OpenRocket builds them (2026-10-01)

**Context.** M5.5b's *done when* is "a rocket built from catalog parts flies through the builder;
each part's mass as built is held to OpenRocket's for its preset". ADR-132 reads OpenRocket's 16
bundled `.orc` files into `hpr_io::orc::Part`s. A part gives its sizes and material, and
sometimes its mass, but never a shape parameter, a shoulder's wall or a parachute's drag
coefficient. The builder (`hpr::rocket`) made noses, tubes, transitions, fins, one motor tube and
point masses; it had no couplers, rings, bulkheads, lugs or transition shoulders, and a nose's base
always took the rocket's diameter.

**Decision.**

1. **A `from_catalog` per builder part.** `Nose`, `Tube`, `Transition` and `MotorTube` (a body
   tube as the motor tube) each get `from_catalog(&orc::Part)`, and a new builder type, `Fitting`,
   takes the rest: a tube coupler or engine block (an `InnerTube`, as the `.ork` reader makes
   them), a centering ring or bulkhead (`CenteringRing`), a launch lug, and a parachute or
   streamer (their `hpr_design` parts, packed into a point). `Fitting` also has constructors to the
   caller's sizes (`coupler`, `centering_ring`, `bulkhead`, `launch_lug`), and
   `Rocket::add_fitting` puts one on the last body tube, flush with its aft end unless `at` places
   it, weighing it there so a part the design can't take is refused where it is added. Each part
   is named by its maker and part number. A catalogue nose or transition states its own diameters;
   the builder's nose no longer always takes the rocket's. `Tube::with_length_m` and
   `MotorTube::with_length_m` cut a tube sold long, and its mass follows. A part of
   the wrong kind, a material the file names but doesn't define, a nose or transition neither
   filled nor given a wall, or a shape the builder doesn't know is `Error::Catalog` with a
   `CatalogProblem`.
2. **What the file leaves unsaid is OpenRocket's choice,** measured by applying every bundled part
   and 6 probe parts to a new OpenRocket 24.12 component (`RocketComponent.loadPreset`, its public
   API, run never read) in `validation/oracles/openrocket/orc_built.py`: an ogive is tangent, a
   parabola's `K′` is 1, a Haack series is von Kármán's (`C` 0), a power series' exponent is ½;
   elliptical, Haack and power-series transitions are clipped, the rest not; no shoulder is
   capped; a filled part's shoulders are solid. The test checks each of these on every part.
3. **But a hollow part's shoulder takes the part's wall** (solid where the wall is thicker than its
   radius). OpenRocket gives it a wall of zero, so it weighs nothing; a molded plastic nose cone's
   shoulder is a tube of the same plastic, and a shoulder of no mass would be a number known to be
   wrong (CLAUDE.md rule 1). The catalogue's stated masses side with the wall: on the 74 hollow,
   shouldered parts that state one, the file's density weighs nearer it with the shoulder's wall
   than without on 46, and the median stated mass is 0.97 of the mass with the wall and 1.30 of
   the mass without.
4. **A stated mass scales the part's density**, so that the part as the catalogue sizes it weighs
   that mass, and the material's name says so. This is what OpenRocket does for a rigid part; for
   a parachute it sets a mass override instead, the same for a part left as it is. A density,
   unlike an override, follows a later change: a tube cut shorter weighs less. OpenRocket leaves a
   streamer's stated mass unused; the one streamer stating one weighs it here.
5. **A part naming an undefined material is refused** (1 part, a nose cone), where OpenRocket
   weighs it as zero. A parachute whose file names no line material, or one it doesn't define, has
   weightless lines, as in OpenRocket, which finds no density for them: 8 parachutes, 6 of them
   stating their mass.
6. **Thresholds, set before measuring:** a nose cone's or transition's mass within 1e-3 of
   OpenRocket's and its centre of mass within 1e-3 of its length; every other part within 1e-12.
   A parachute's or streamer's centre is where it is packed and is not compared. Nor are the
   moments of inertia: the fixture records none, and `hpr_design`'s are checked against hand
   sums on their own (`docs/physics/mass.md`).
7. **`CatalogProblem::Kind` names the kinds as strings** (`found`, `builder`), not as an enum of
   `hpr_io` kinds, so the error type doesn't tie `hpr`'s public API to the reader's enum, which
   may grow.

**Consequences.** M5.5b is met. `crates/hpr/tests/catalog_openrocket.rs` builds all 3,449 parts
through the builder and holds each to `crates/hpr/tests/fixtures/orc/openrocket-built.json`, every
part in one count: 2,230 tubes, couplers, blocks, rings, bulkheads, lugs, parachutes and streamers
within 1e-14 of OpenRocket's mass and their centres within 1e-13 of their length; 1,029 filled
nose cones and transitions within 2.0e-4 in mass and 7.0e-5 of their length in centre; 181 hollow
ones, their hollow shoulders taken out in closed form, within 6.3e-4 and 9.7e-4; 4 hollow ones
whose walls differ (below); in each group, masses stated in ounces apart by OpenRocket's rounded
ounce instead (185); 1 streamer's stated mass; and 4 refused, which OpenRocket weighs as
zero (1 undefined material, 3 tube-like parts with no bore). hpr's 85 filled conical noses equal
the closed form to 1e-12, so their differences are OpenRocket's volumes. The hollow parts' gap is
the two codes' walls, each checked on its own: OpenRocket's 185 hollow parts follow a wall whose
inner radius at each station is `r − t √(1 + r′²)`, integrated in the test, to 1.1e-4 of their
length in centre, and the 111 stating no mass to 2.5e-4 in mass; hpr's wall, every point within
`t` of the surface, equals integrals worked out in the test, in volume and centre, on 113 hollow
nose cones (16 cones, 87 tangent ogives, 10 ellipsoids) to 1e-9. Dropping the slope term from the
station-wise wall fails the test, and so does moving a cone's centre. The 4 whose walls differ by
more than the threshold are short, blunt elliptical nose cones, each with hpr's wall checked:
three up to 0.48% heavier here (the test asserts hpr's is the heavier), one stating its mass
1.0e-3 of its length apart in centre; the largest centre gap is 1.7e-3. Two mutations,
ellipsoid transitions not clipped and a parabola's `K′` of 0.75, each fail the test. `crates/hpr/examples/catalog_rocket.rs` builds LOC Precision's
2.56 in airframe from the catalogue, its fins by hand, and flies it on an AeroTech H170 to
1,119 m. A nominal 29 mm motor doesn't fit LOC's 29 mm motor tube (bore 28.956 mm) under the
design checks, so the example uses the 38 mm tube; that is issue #280. What is checked is each
part's mass as its file describes it: a catalogue's sizes and densities are the makers' or the
database's, and none was weighed here.

## ADR-134: Monte Carlo dispersion: independent normals, one stream per sample and input (2026-10-01)

**Context.** M6.1 asks for seeded, parallel dispersion over mass and CG, drag, motor impulse and
timing, wind, launch angle and deployment delays; landing ellipses; Morris and Sobol sensitivity;
and 10,000 flights of an L2 design in 10 s. Its *done when* is three bullets, and it carries five
Loft lessons: L52 (thrust scaled without propellant mass), L53 (one random stream for a whole run),
L54 (failed samples dropped), L55 (bearings drawn uniformly, the forecast thrown away) and L96 (the
same seed and zero dispersion). That is more than one session.

**Decision.**

1. **M6.1 splits in four**, each with its own *done when* in `ROADMAP.md`: a, the dispersion and
   its reproducibility, with all five lessons' tests; b, landing ellipses against analytic
   Gaussians; c, Morris and Sobol against functions with known indices; d, the 10,000-flight
   timing.
2. **Each input is an independent normal about its nominal value**, its standard deviation given
   by the user and zero by default. This is RocketPy's default reading of a `(nominal, standard
   deviation)` pair (`rocketpy/stochastic/stochastic_model.py:190-199`, v1.13.0). Fractions for quantities that
   scale (mass, drag, impulse, burn time, wind speed), absolute values for the rest. There are no
   presets: NFPA 1125 (the 2019 edition's §8.1.7 and §8.2.7, as quoted in its next revision's
   public first-draft documents, whose first revisions keep them) bounds a motor type's impulse
   spread at 6.7% and a delay's error at 1.5 s or 20%, capped at 3 s, but a bound is not a spread;
   four NAR certification sheets measured 1.3% to 3.1% in impulse and 1.5% to 18% in burn time.
   The guide gives these and leaves the choice to the user.
3. **One stream per sample, input and copy.** `SeededRng::for_stream(seed, &[sample, input,
   copy])` folds the key into one word with SplitMix64's output function and seeds xoshiro256++
   from it, the counter-based idea of Salmon et al. (SC11). A sample is then the same whatever the
   run's length, its thread count, or the other inputs dispersed. Jumping one generator ahead
   (xoshiro's `jump`) would also give disjoint streams, but costs a jump per sample and ties
   sample `k` to the order streams are handed out. Two keys collide only by a 64-bit hash
   collision. An input with zero dispersion draws nothing and changes nothing, so zero dispersion
   flies the nominal flight bit for bit (L96).
4. **Impulse scales the thrust and the propellant mass together** (`dispersed_motor`), keeping
   `I/m_p` (L52): a column's mass, BATES grains' density so the grain geometry is kept. Burn time
   stretches the curve and divides the thrust by the same factor, keeping the impulse. A cluster is
   one draw, because the design holds it as one motor.
5. **Drag is scaled in the aero model** (`AeroModel::with_drag_scale`,
   `Simulation::with_drag_scale`), multiplying whatever gives the zero-lift coefficient: the
   buildup, a table or a drag model. Wrapping the drag in a drag model instead would have made it
   the whole stack's, refused at a powered separation; the scale passes to the sustainer instead.
6. **Wind is turned about its own direction** (`DispersedWind`): the base model's velocity at every
   height, scaled and turned clockwise, so a profile keeps its shape and the forecast's heading is
   the mean (L55). The rail's heading is likewise an offset. A rail drawn past vertical leans the
   other way (`E′ = π − E`, heading and roll turned half a turn), the same attitude.
7. **Delays and the wind's speed are cut at zero**, since a charge can't fire before its event
   and a wind can't blow at less than calm (failing those samples would bias the run towards
   strong winds); every other impossible draw (a negative mass, a rail below the horizon) fails its
   sample, which is kept with its reason and where it failed (`FailedAt`), and counted (L54). A
   probability is reported as two bounds, failures counted as failing and as passing (`Share`).
8. **Statistics on sorted values**: mean and standard deviation shifted by the smallest value
   (Chan, Golub and LeVeque, 1983), quantiles by Hyndman and Fan's definition 7. Sums run over the
   sorted values, so a summary is the same however the samples were flown.
9. **Parallelism is rayon behind `hpr-analysis`'s `parallel` feature**, off by default and never
   built for wasm32 (`ARCHITECTURE.md`'s plan, `clippy.toml`'s rule). Its `collect` keeps the
   samples in order. `run_parallel` uses the caller's pool, so an optimizer calling it per
   candidate starts no threads, and the thread count is the caller's `ThreadPool::install`.
10. **The inputs are `FlightInputs`**, the arguments of `Simulation::new` plus recovery, a drag
    override and the drag scale, because `Simulation` is neither `Clone` nor open. Events,
    separations and staging a `Simulation` can be given aren't in it yet; `hpr::FlightBuilder::inputs`
    builds it, and `FlightBuilder::simulation` now goes through it.
11. **A `Draw` says what a sample flies.** `MonteCarlo::inputs` applies every entry that isn't at
    its nominal value (a factor of 1, an offset of 0), whatever the dispersion, and checks the
    lists' lengths against the rocket, so a draw read back or written by hand flies as it says. An
    entry at its nominal value changes nothing, which keeps zero dispersion bit for bit. The
    public structs (`Dispersion`, `Draw`, `FlightInputs`) are not `#[non_exhaustive]`, so callers
    can write them with `..Default::default()`; adding a dispersion is then a breaking change,
    acceptable before 1.0. `DragOverride` is `#[non_exhaustive]`.

**Consequences.** M6.1a is met (`crates/hpr-analysis/src/montecarlo.rs`'s tests, the guide's
`docs/monte-carlo.md` and its example). Each sample rebuilds its `Simulation`, so a supersonic
design rebuilds its supersonic tables every flight (300 to 700 ms each, `docs/perf.md`); M6.1d has
to share them. Not dispersed: the atmosphere, ignition times, recovery devices' drag, correlations
between inputs. No measured set of repeated flights has checked the spread a run gives.

## ADR-135: Landing ellipses: the normal ellipse, the next flight's, and a count of what each holds (2026-10-01)

**Context.** M6.1b asks for landing ellipses at confidence levels, tested against analytic
Gaussians. A run gives each sample's landing east and north of the pad (`FlightSummary::landing`),
or none for a failed flight or one that never landed. Three choices are open: which ellipse "95%"
means, what to do with the samples that gave no landing, and what to say when the landings aren't
normal.

**Decision.**

1. **The default ellipse is the normal one** (`Scatter::ellipse`): centred on the sample's mean,
   its axes along the sample covariance's eigenvectors, `k √λᵢ` long with `k² = −2 ln(1 − p)`, the
   chi-square quantile with two degrees of freedom in closed form (Abramowitz and Stegun, eq.
   26.3.21 and 26.4.5; Wang, Shi and Miao, *PLoS ONE* 10(3), e0118537, 2015). It is what a
   reader expects from a "95% ellipse", and what other tools draw.
2. **A prediction ellipse sits beside it** (`Scatter::prediction_ellipse`): the region the next
   flight lands in with probability `p` exactly, for normal landings whose mean and covariance were
   estimated from `n`, by Hotelling's `T²`: `k² = ((n² − 1)/n)((1 − p)^(−2/(n − 2)) − 1)`
   (NIST/SEMATECH e-Handbook §6.5.4.3.4, after Ryan, 2000; the `F(2, m)` function from A&S eq.
   26.6.4). At 200 flights its axes are 1.3% longer; at 10, 36%. Tolerance regions (an ellipse
   holding `p` of all flights with a stated confidence) are left out: they have no closed form.
3. **Samples without a landing are counted, not dropped**, as in a `Distribution` (L54): a
   `Scatter` keeps the samples tried, and the share an ellipse holds is a `Share`, the missing
   samples counted as outside and as inside. The ellipse itself is drawn from the landings there
   are.
4. **Whether the landings are normal is measured, not assumed**: `Scatter::share_inside` counts the
   landings inside each ellipse, and the guide says what a share far from the level means (an arc
   from an uncertain wind heading). No test of normality, no other shape (kernel density, convex
   hull): the count is the check a reader can act on.
5. **The heading is reported clockwise from north in `[0, π)`**, as the rail's and the wind's are;
   a circle's is east's, `π/2`. Sums run over the points sorted by east then north, shifted by the
   first, as `Distribution`'s are, so an ellipse is the same however the samples were flown.
6. **A scatter's axes are measured from its points** (`Scatter::principal_axes`): the heading from
   the covariance, each variance as the mean square along or across it (a Rayleigh quotient,
   wrong only by `δ²(λ₁ − λ₂)` for a heading error `δ`). A covariance's entries are each rounded
   to about `ε λ₁`, so no formula on them keeps a spread narrower than that; a given
   `Covariance`'s minor variance is `(ac − b²)/λ₁`, which keeps it along the axes. The covariance
   of points on a line is cut to the Cauchy–Schwarz bound, so it passes its own checks, and
   `Covariance::new` allows four units of rounding past it. A point on a flat ellipse's axis is
   kept inside by a floor of `10⁻¹²` of its size and distance on each semi-axis. A point more
   than 10⁹ m from the pad is refused, so no square or sum can overflow. An `Ellipse`'s fields
   stay public, as `Summary`'s do, but it is `#[non_exhaustive]` and checked when read back.
7. **The tests use normal spreads whose answers are known**: the scale against NIST's chi-square
   table and the closed forms; the axes of rotated covariances to 1e-14; the normal density
   integrated over its ellipse (along rays, in closed form per ray) to its level within 1e-12;
   100,000 seeded points giving back their covariance and their share inside within five standard
   errors; the prediction ellipse of 3, 5 and 20 points holding a new point at its level over
   20,000 trials, and the normal ellipse visibly less; a spread 10⁸ times longer than wide; any
   2 to 20 points inside their own 99.99% ellipse (proptest, from the leverage bound
   `(n − 1)²/n`).

**Consequences.** M6.1b is met. The guide's example draws the 50% and 95% ellipses of its 200
landings (48.0% and 94.5% inside). Hotelling (1931) and Chew (1966) were not read; the prediction
formula rests on NIST's handbook and on the Monte Carlo test. NIST's two pages are cited by URL,
not pinned in `refs.lock.toml`: each fetch differs (a per-request script). Landings over uneven ground, and
a landing ellipse in latitude and longitude, are not handled: the ellipse is in the pad's local
east-north plane.

## ADR-136: Sensitivity analysis: Morris paths and Saltelli's Sobol' estimates, with their standard errors (2026-10-01)

**Context.** M6.1c asks for Morris screening and Sobol' indices that match the known indices of
test functions with closed forms (Ishigami, Sobol's g) within sampling error. Open: what a factor
is, how the two methods meet a flight, which estimators, what "within sampling error" is measured
against, and what Morris's "known indices" are, since Morris's measures have no published closed
forms for these functions.

**Decision.**

1. **A factor is a uniform range** (`Factor`: name, low, high), independent of the others, as in
   Morris (1991) and Saltelli and others (2010). Normal inputs are given as a range; other
   distributions and correlations are left out.
2. **Each method is a design, then an analysis** (`Morris::design` → `MorrisDesign::points` →
   `analyse(outputs)`; `Sobol::design` → `SobolDesign::points` → `analyse`), with a closure shortcut
   (`screen`, `indices`). The model is the caller's: a flight is flown by setting the factors into a
   `Draw` and `MonteCarlo::inputs`, as the guide's example does, and the runs can go to any threads
   or machines. No flight-specific factor type: which inputs to vary, and over what, is the caller's
   choice, and `Draw` already names every input `MonteCarlo` can vary. A model that can fail uses
   the two steps, as the closures return a bare `f64`. Factor names must differ (`DuplicateFactor`).
   A design is held to `MAX_DESIGN_POINTS` (2²⁰) points and `MAX_DESIGN_VALUES` (2²⁴) coordinates,
   and a grid to `MAX_LEVELS` (2¹⁶), so a design that passes `new` takes under about 200 MiB, on
   wasm32 too. An index or standard error that overflows from an extreme output is refused, not
   returned. Designs aren't serialized: a configuration and its seed rebuild one bit for bit. Result
   types are `#[non_exhaustive]`, so second-order indices can be added later.
3. **Morris: his paths** (Technometrics 33(2), 1991, eq. (1) p. 163; `B*` p. 164), `p` even,
   `Δ = p/(2(p − 1))`, start levels in the lower half, a sign and an order per path. Each path
   draws from `SeededRng::for_stream(seed, &[j])`. Reported: `μ`, `μ*` (Campolongo, Cariboni and
   Saltelli, 2007), `σ` with `r − 1`, and `μ*`'s standard error, `sd(|d|)/√r`. Effects are in the
   output's units per the factor's whole range. Campolongo's choice of spread-out paths is left
   out. Campolongo's finding that `μ*` ranks as `S_T` does is shown by experiment only, and
   Ishigami's function is a counter-example. A step of about half the range misses `sin² x₂`,
   whose period is half the range: at four levels `μ*` puts `x₂` first and `S_T` puts `x₁`
   first; at six or more `μ*` puts `x₂` last, though `S₂` is the largest first-order index. A
   test pins both, and the guide warns of it.
4. **Morris's known answers are the moments of `Fᵢ`**, the finite distribution each path's effect
   is drawn uniformly from (Morris, p. 164). `Morris::population` computes them exactly by running
   the model at all `p^k` grid points (at most 2²⁴). For Ishigami and the g function at `p = 4`
   they are also derived in closed form in the test file, and `population` matches both to 1e-12.
5. **Sobol': Saltelli and others' design and estimates** (CPC 181, 2010, Table 2, p. 262): rows of
   `A`, `B` and `A_B⁽ⁱ⁾`, `Vᵢ` by (b), which they recommend for its design's points (p. 263), and
   `V_Tᵢ` by Jansen's (f), "the best practice so far" (p. 262). `V` comes from `A` and `B` together
   (the Primer, p. 166). The outputs are shifted by their mean first. Sobol' (2001, p. 277, remark
   3) advises that against a loss of accuracy. Our own further reason: the first-order estimate's
   variance has a term in the output's mean squared, which the shift removes, as an apogee's mean of
   a kilometre would otherwise swamp it. Rows are pseudo-random from `for_stream(seed, &[j])`, not
   quasi-random: rows are independent, so the error is the central limit theorem's.
6. **Standard errors by the delta method**: each index is a smooth function of row means, so its
   error is `√(Σψⱼ²/(N(N − 1)))` with `ψⱼ` row `j`'s linearised influence, including the shift's.
   This is deterministic and needs no bootstrap stream. A unit test computes it a second way,
   the gradient over the raw row means times their sample covariance, to 1e-9. Dropping the
   shift's `D` term fails that test, though no calibration over seeds could see it (it is
   `O(1/√N)`).
7. **"Within sampling error" is two checks.** First, each estimate lies within four of its own
   standard errors of the closed form (Ishigami at `N = 2¹⁵`, g at `2¹⁴`; Morris at 1,000 paths).
   Second, over 1,000 seeds (500 for g's Sobol' indices), `(estimate − known)/error` has a mean
   within `4/√n` of 0 and a standard deviation within `4/√(2n)` of 1. The g function's
   calibrations use a four-factor g (`aᵢ` = 0, 1, 4.5, 9); Morris's `μ*` is calibrated the same
   way on it over 1,000 seeds. Each calibration also
   asserts its own power: the same `z`s divided by 0.85, as from standard errors 15% too small,
   must fail it. The g function's `aᵢ` = 0, 1, 4.5, 9, 99 × 4 span Marrel and others' (2008) four
   classes as the SFU library quotes them. Sobol' and Levitan's printed `b = 0.05` indices are
   held to their three or four printed digits.

**Consequences.** M6.1c is met. In the guide's example, Ishigami's and g's indices at `N` = 8,192
lie within 1.4 standard errors of the closed forms. The rocket's Morris screening, 70 flights,
ranks the impulse and the drag first for the apogee and the wind's speed for the landing. A Sobol'
analysis of a flight costs `N(k + 2)` flights, about 47 ms each in a debug build, so the example
leaves it out. M6.1d's speed work makes it practical. The papers were read from the authors' or
institutions' copies and are cited by DOI, not pinned in `refs.lock.toml`, because no test reads
them. Second-order indices, quasi-random rows, other distributions and a command-line front end are
left out.

## ADR-137: Ten thousand flights: a run's flights share the nominal's layout and supersonic table (2026-10-01)

**Context.** M6.1d asks for 10,000 flights of a Level 2 design in 10 s or less on the development
machine, recorded in `docs/perf.md`. Each Monte Carlo sample builds its own `Simulation` from its
draw. Before this change, timed on an Apple M5 with 10 threads, release build, flights to the
ground under a drogue and a main:

- Valetudo on a K400C, peaking at Mach 0.3 to 0.4, took 4.59 s for 10,000 flights.
- A 66 mm rocket with a 54 mm mount on a K940, peak Mach 1.6 to 2.0, took 264 s. Each of its
  flights built its own supersonic table on its first pass of Mach 1.2: most of a 193 ms flight.
- Building a simulation laid the design out twice, 0.4 ms each time.

Open: which design is "an L2 design", and how to stop paying for that work every flight without
changing a flight.

**Decision.**

1. **The flights of a run share the nominal design's supersonic table.**
   - `MonteCarlo::new` builds the nominal design's `AeroModel`. Each sample calls
     `Simulation::share_supersonic_table` with it.
   - The table is a pure function of the shock-expansion run's segments and the reference area
     (`SupersonicBody::new(run, reference_area_m2)`). So `AeroModel::share_supersonic_table`
     shares only when both are equal (`PartialEq`, `==` on `f64`), and leaves a different shape or
     reference area alone.
   - No dispersion changes the shape. Masses and centres are stage overrides, a dispersed motor
     keeps its geometry, and drag is a scale on the model's output. So every sample of a design
     shares, yet nothing is assumed: a sample that did differ would build its own table.
   - The table sits behind `Arc<OnceLock>`. The first sample that needs it builds it, and threads
     that need it meanwhile wait.
   - A sustainer built at a powered separation still builds its own table.
2. **The flights of a run share the nominal design's layout.**
   - `Rocket::lay_out` gives a `LaidOut`: an owned copy of the design, its layout, and each
     stage's masses before its overrides. `LaidOut::check` and `LaidOut::assemble` share the one
     layout, and `Simulation::from_laid_out` builds on it; `Simulation::new` is `from_laid_out`
     of `lay_out`.
   - `LaidOut::relay(rocket)` keeps the placed parts when `rocket` differs only in its stages'
     overrides and its configurations. Every other field is compared, by an exhaustive
     destructure, so a new field can't be missed. It redoes only the stages, by the same code as
     `Rocket::layout`; any other difference lays the design out from the start.
   - Because `LaidOut`'s fields are private, a layout can't be paired with another design. That
     answers the review of a first version, which had public `check_with_layout` and
     `assemble_with_layout` taking any layout.
   - Samples, and the new `MonteCarlo::fly` for draws made by hand (the sensitivity guide's),
     use both. `FlightInputs::fly` builds everything itself.
3. **The integrator borrows each evaluation** from the phase's cache rather than copying a few
   hundred bytes out of it at every stage and event check.
4. **"An L2 design" is Valetudo**, RocketPy's 9.7 kg rocket on a K400C. It has been the "typical
   Level 2" flight of `hpr-sim`'s flight benchmarks since M1.6b. The K940 rocket is timed beside
   it, because supersonic flights are where a run was slow. Both fly to the ground under a drogue
   and a main.
5. **The measurement is a bench target, not a test:** `crates/hpr/benches/ten_thousand.rs`, run by
   `cargo bench -p hpr --features parallel --bench ten_thousand`.
   - A wall-clock limit in CI would fail on shared runners for reasons that have nothing to do with
     the code.
   - Each timed run starts from `MonteCarlo::new`, so the table's one build is counted. The program
     prints the fastest of three runs, and where one flight's time goes.
   - It does nothing unless `cargo bench` runs it.

**Checks.** Every flight is unchanged bit for bit:

- `samples_share_the_nominal_table_and_fly_as_alone` flies supersonic samples against the same
  inputs flown alone, through `sample`, `fly`, and fresh runs on two and five threads. It checks
  that the samples built the nominal's table, which fails with sharing off.
- `models_built_apart_share_the_table_only_on_the_same_body` fails if either half of the sharing
  rule is dropped.
- `a_relaid_design_is_laid_out_as_alone` holds relaid layouts, checks and assemblies to fresh ones,
  and fails if `relay` keeps the parts of a changed design.

**Consequences.** M6.1d is met.

- Valetudo's 10,000 flights take 3.00 s and the K940 rocket's 9.43 s (2.91 to 3.00 s and 9.33 to
  9.47 s over three runs of the program during the last changes), with the same apogee and Mach statistics as before.
- The supersonic run is about 6% inside the budget. A profile, taken before item 3, puts 11% of its
  busy time in `atan2`
  (flow angles, the geodetic conversion, gravity) and 8% in `pow` (drag terms, the atmosphere).
  Each fix changes the flown numbers in their last bits, so every committed report, including the
  corpus reports that CI can't fly, would have to be regenerated. That is left to #285.
- A sensitivity analysis of a supersonic flight through `MonteCarlo::fly` now costs a few
  milliseconds a flight, not a fifth of a second.

## ADR-138: Optimization: CMA-ES first, held to test functions and to pycma (2026-10-01)

**Context.** M6.2 asks for an optimization engine: continuous and discrete variables (motor
choice, catalogue parts), constraints, single- and multi-objective goals, CMA-ES, NSGA-II and
Bayesian optimization (EGO), and a robust mode with Monte Carlo in the loop. Done when benchmark
functions converge to known optima within tolerance and a "hit 3,048 m" problem is solved, the
result validated by re-simulation. That is more than one session.

**Decision.**

1. **Split in five.** M6.2a CMA-ES over continuous variables, with both of M6.2's done-when tests
   for it; M6.2b discrete variables (a motor, a catalogue part) and constraints (a stability
   margin, a rail-exit speed); M6.2c several objectives (NSGA-II, a Pareto front); M6.2d Bayesian
   optimization (EGO); M6.2e robust designs (a Monte Carlo run's statistic as the objective).
   M6.2 closes when all five have.
2. **CMA-ES as N. Hansen's tutorial gives it** (arXiv:1604.00772v2, 2023): Figure 6's update,
   eqs. (38) to (47), and Table 1's defaults, eqs. (48) to (58), with the negative weights zero,
   as in the tutorial's own code (p. 36). The active variant is left out: one fewer thing to get
   wrong, and the tests' pycma runs are non-active too. `C` is decomposed every generation, which
   the tutorial's lazy schedule (B.2) also gives for up to about 85 variables at the default
   population. The eigen-decomposition is our own cyclic Jacobi (Golub and Van Loan, §8.5), with
   Demmel and Veselić's (1992) stopping test, so the smallest eigenvalue of an ill-conditioned
   covariance keeps its relative accuracy. It is tested against a closed-form spectrum and a
   graded matrix's closed-form determinant. No linear-algebra dependency is added.
3. **Variables carry a start, a step and optional bounds** (`Variable`). The strategy works in
   each variable divided by its step, starting as Figure 6 does (`σ = 1`, `C = I`), the
   tutorial's "a scaling of the variables should be applied" (its footnote, p. 29). A first draft
   started at `C = diag(stepᵢ²)` instead; review found that measures ConditionCov on the unscaled
   `C`, so steps of 10⁻⁴ and 10⁴ stopped a solvable run after one generation. Bounds are handled
   by drawing a candidate again from its own stream until it falls inside, the second of the
   tutorial's two methods for an optimum inside the feasible region (B.5), up to 1,000 draws
   (`Stop::Bounds`, or `OutOfBounds` for the first generation); a candidate that overflows ends
   the run as ConditionCov. No repair, which the tutorial advises against; an optimum on a bound is left to
   M6.2b's constraint handling.
4. **Ask and tell.** `Cmaes::start` draws a generation; `Run::tell(values)` takes the model's
   values in order. So the caller evaluates candidates however it likes, as the sensitivity
   designs allow (ADR-136). `minimize` is the closure shortcut. A value may be `+∞` (a flight that
   fails ranks last); NaN and `−∞` are refused with the evaluation's index.
5. **Streams.** Candidate `k` of generation `g` draws from `SeededRng::for_stream(seed, &[g, k])`,
   resampling included, so a run is bit for bit the same on one platform.
6. **Stops** are the tutorial's TolX (in the scaled variables), TolFun and ConditionCov (B.3), with
   its 10⁻¹² tolerances, plus a finite target value and an evaluation cap (10,000 by default). An
   overflowed `σ` counts as ConditionCov. NoEffectAxis, NoEffectCoord, Stagnation and TolXUp are
   left out. `Cmaes` deserializes through the same checks as its builder; `Parameters` is read
   from a run, not built.
7. **Held to an outside implementation.** pycma 4.5.0 (BSD-3-Clause, the author's), pinned in
   `validation/oracles/pyproject.toml`, runs the four test functions of Hansen, Müller and
   Koumoutsakos (2003, Table 1) at ten variables, from the same starts and steps, non-active, every
   stop but the target (10⁻¹⁰, the paper's) and the cap off, seeds 1 to 20
   (`validation/oracles/cmaes/pycma_runs.py` → `crates/hpr-analysis/tests/fixtures/cmaes/pycma.json`).
   The random numbers differ, so runs are compared by median evaluations, required within 25%
   (set before the first comparison). pycma's `c_σ` and `d_σ` (`n + μ_eff + 3`), its exact
   `E‖N‖`, its `h_σ` test and its step-size clip depart from Table 1 by its author's choice; the
   weights, `μ_eff`, `c₁`, `c_μ` and `c_c` it shares with Table 1 agree to 1e-15.

**Consequences.** M6.2a is met. Medians: sphere 1,635 (pycma 1,640), ellipsoid 5,920 (5,910),
rotated ellipsoid 6,010 (5,930), Rosenbrock 6,445 (6,195); every sphere and ellipsoid run reaches
10⁻¹⁰, Rosenbrock from 17 of 20 seeds, the other 3 in its local minimum near `(−1, 1, …, 1)`
(pycma 19 of 20; Kern, Hansen and Koumoutsakos, 2006, report 17 to 19 of 20 at 4 to 16 variables).
The test requires at least 17, which the measurement meets exactly; once, over seeds 1 to 300,
290 reached the global minimum (97%), so 17 of 20 is the low end of chance, not a weak method. A
second test recomputes twelve generations by a separate dense implementation of Figure 6 and
Table 1 (`C^(−1/2)` by the Denman–Beavers iteration), steps 0.5, 2 and 0.01, a linear model that
takes `h_σ` through both values: mean, `σ` and `C` agree to 1e-12. One-off mutations: dropping the
rank-μ update takes the ellipsoid's median to 7,875 and fails the pycma comparison; `h_σ` held at
1, its exponent `2g` for `2(g + 1)`, and the mean's step unscaled each pass the pycma comparison
(found in review) and fail the dense recomputation. The guide's example finds the ballast and
body length of a J760 rocket for a 3,048 m apogee and a 2.20-calibre margin together (one goal
alone leaves a curve of answers, and the run stopped at whichever it met first): 0.3373 kg and
1.1084 m in 300 flights, apogee 3,047.9997 m, margin 2.199996; flown again from a fresh build it
gives the optimizer's value to the bit, and at tolerances 100 times tighter 3,048.0011 m. The
example prints only what doesn't depend on the platform's last bits: CI first failed on Linux
and Windows, where `ln` and `exp` round differently and Windows' sphere run took 1,580
evaluations to macOS's 1,660. So it prints whether each test function reached its minimum, and
the design to 0.01: the ballast 2.3 g and the body 3.4 mm from a rounding edge, against a
solution good to about 10⁻⁴. The papers are cited, not pinned in `refs.lock.toml`, as
no test reads them. Left out: active CMA, restarts
(IPOP/BIPOP), repair or penalty bound handling, and the command line and Python.

## ADR-139: Optimization constraints by Deb's feasibility rules (2026-10-01)

**Context.** M6.2b asks for discrete choices (a motor, a catalogue part) and constraints (a
stability margin, a rail-exit speed), held by "hit 3,048 m" with the motor free and each
constraint re-checked on the winner. That is more than one session, and the constraints are
needed by the rocket problem whichever way the discrete choices go.

**Decision.**

1. **Split in two.** M6.2b1 constraints, held to test problems with known constrained minima;
   M6.2b2 discrete choices and the rocket problem with M6.2b's done-when.
2. **Deb's feasibility rules** (K. Deb, *CMAME* 186 (2000) 311–338, §3): feasible beats
   infeasible; two feasible by value; two infeasible by total violation `Σ max(0, gⱼ)`, ties by
   value. CMA-ES uses only ranks, so the rules plug into its ranking with no penalty weight to
   tune, and values and violations are never compared. `Evaluation { value, violation }` carries
   both; `Run::tell_constrained` and `Cmaes::minimize_constrained` take them; `Run::tell` is the
   case of zero violations, bit for bit (a unit test compares whole runs).
3. **Infeasible candidates are ranked, not redrawn** (bounds still redraw), so the distribution can
   straddle a constraint's edge and close in on a minimum on it. The best point is the best by the
   same rules; a target counts only for a feasible best; TolFun needs the window's every best
   feasible, and looks at this generation's feasible values only.
4. **Held to three problems** whose constrained minima are known in closed form: the sphere with
   `x₀ ≥ 1` (1 at `(1, 0, …)`), the tangent problem `Σ xᵢ ≥ n` (n at `x = 1`, by Lagrange), both
   in 10 variables, and CEC 2006's g06 (J. J. Liang et al., 2006), whose minimum is the crossing
   of its two circles, `x₀ = 14.095` exactly; the test also checks the published −6961.81387558015.
   Every one of 20 seeds reaches the minimum to 10⁻¹⁰ relative and within 10⁻⁴ of the point,
   also from an infeasible start. Measured on seeds 0 to 19: errors about 10⁻¹², median
   evaluations 1,890 (g06) to 11,550 (tangent).

**Consequences.** A user scales each constraint (Deb normalizes them) so they count alike; the
guide says so. Not done: equality constraints (write `|h| − ε ≤ 0`), the adaptive penalty or
augmented-Lagrangian methods of the CMA-ES literature, and any rocket constraint, which comes with
M6.2b2's problem.

