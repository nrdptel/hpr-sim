# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e is held at M1.8e16 (on #108), M2.3c on Neer; next: M6.2b
  (discrete variables and constraints). **Run:** M0.1-4, M1.1-7, M1.8a-e19 bar e16, M1.9-13, M2.1-4 bar M2.3c, M3.1-3, M4.1-3, M5.1-5, M6.1, M6.2a.
- **Neer, 2026-09-20:** Debrief sunset; a log analyzer usable **on its own** is in scope (ADR-046, V21).
- **Last updated:** 2026-10-01; M6.2a: CMA-ES, held to four test functions and pycma (ADR-138).

## Handoff (overwrite each session)

- **Next (resume here):** M6.2b (ROADMAP): discrete variables (motor, catalogue part) and constraints. CMA-ES (ADR-138) is `hpr_analysis::optimize`: `Variable` (start, step, `within` bounds), `cmaes::{Cmaes, Run, Optimum, Stop, Parameters}` (`start` → `candidates` → `tell`, `minimize`; works in variables ÷ steps), private `eigen` (Jacobi), `benchmark`; `tests/optimize.rs` holds it to `tests/fixtures/cmaes/pycma.json` (`validation/oracles/cmaes/pycma_runs.py`, pycma 4.5.0); example `crates/hpr/examples/optimization.rs` (the 3,048 m J760 rocket), page `docs/optimization.md`. Speed (ADR-137): `crates/hpr/benches/ten_thousand.rs` (`cargo bench -p hpr --features parallel --bench ten_thousand`) times 10,000 flights; samples share the nominal's layout (`Rocket::lay_out` → `LaidOut`, `relay`, `Simulation::from_laid_out`) and supersonic table (`share_supersonic_table`), via `MonteCarlo`'s `shared`; `MonteCarlo::fly` shares for hand-made draws; the supersonic run is 9.4 s of 10, per-evaluation cuts are #285 (they move last bits: regenerate every report). Sensitivity (ADR-136) is `hpr_analysis::sensitivity`: `Factor` (uniform range), `morris::{Morris, MorrisDesign, ElementaryEffects}` (`design` → `points` → `analyse`, `screen`, exact `population` ≤2²⁴ points), `sobol::{Sobol, SobolDesign, SobolIndices}` (Saltelli (b), Jansen (f), outputs shifted by their mean, delta-method errors), `benchmark::{Ishigami, SobolG}`; `tests/sensitivity.rs` holds both to the closed forms and calibrates the errors over seeds; example `crates/hpr/examples/sensitivity.rs` (Morris on the MC rocket via `MonteCarlo::fly(&Draw)`), page `docs/sensitivity.md`; flights by hand go through `MonteCarlo::fly`. Ellipses (ADR-135) are `hpr_analysis::ellipse`: `Run::landing` → `Scatter` (failures counted), `ellipse` (`k² = −2 ln(1−p)`), `prediction_ellipse` (Hotelling), `share_inside`. Monte Carlo (ADR-134) is `hpr_analysis::montecarlo`: `FlightInputs` (`hpr::FlightBuilder::inputs`; `simulation()` goes through it), `Dispersion` (sds, zero draws nothing), `MonteCarlo::{draw, inputs, sample, run, run_parallel}` (feature `parallel`: rayon, the caller's pool; `inputs` applies any non-nominal `Draw` entry, lengths checked), streams `SeededRng::for_stream(seed, &[sample, input, copy])` (the `Input` numbers are part of every seeded result), `Simulation::with_drag_scale` (passes to a sustainer); `statistics::Distribution` (shifted moments, Hyndman–Fan 7, `Share` bounds); example `crates/hpr/examples/monte_carlo.rs`, page `docs/monte-carlo.md`; NFPA 1125 and four NAR sheets pinned in `refs.lock.toml`. Catalogue parts (ADR-133) are `hpr::rocket::catalog`: `from_catalog` on `Nose`, `Tube`, `Transition`, `MotorTube` and the new `Fitting` (`Rocket::add_fitting`); `tests/catalog_openrocket.rs` holds every part (or counts its gap) to `crates/hpr/tests/fixtures/orc/openrocket-built.json` (`validation/oracles/openrocket/orc_built.py`: rerun after a reader or builder change); a hollow shoulder takes its part's wall (OR's is zero); #280: a nominal 29 mm motor is refused in LOC's 28.956 mm bore. The reader (ADR-132) is `hpr_io::orc`: `read`, `bundled()`, `Catalog::find` / `search`; `tests/orc_openrocket.rs` holds it to `crates/hpr-io/tests/fixtures/orc/openrocket-presets.json` (`orc_presets.py`). `hpr motors search` (ADR-131) is `crates/hpr-cli/src/motor_search.rs`: the finder's list via `weather::client` (shared), `--from` a saved list, `--max-price` exact cents on `cheapest_in_stock`; `tests/motors_search.rs` filters the recording's JSON itself and edits a temp copy for the $150 example (the recording has none). ThrustCurve (ADR-130) is `hpr_net::thrustcurve`: `fetch_finder_records` (three whole searches), `join` (exact maker + designation; 282 of 282), `fetch_download` + `DataFile::read` (`.eng`/`.rse` via `hpr_motor`); `validation/reports/thrustcurve-join.md` is `Join::report`, rewritten by `HPR_WRITE_THRUSTCURVE_JOIN=1`. The finder (ADR-129) is `hpr_net::motor_finder`, eight answers of one build in `tests/fixtures/replay/motor-finder-*`. GeoTIFF (ADR-128) is `hpr_io::geotiff` over `tiff` (LZW, Deflate), geographic CRSs near WGS 84 only, the containing pixel as GDAL places it, GDAL's scale and offset; `validation/oracles/geotiff/dem.py cut|read` writes `crates/hpr-io/tests/fixtures/geotiff/` and rasterio's (1.5.2, in `pyproject.toml`) `rasterio.json`; `tests/geotiff_rasterio.rs` runs the whole `refs/dem/` tile where fetched; no `hpr` command or flight reads it. Geodesics (ADR-127) are `hpr_core::geodesic` over `geographiclib-rs`, `f` past 1/150 refused; `tests/geodtest.rs` checks Karney's every-500th and 21 mirror lines in CI and the whole `refs/sources/geodtest/GeodTest.dat` locally, its table against `validation/reports/geodesics.md` on a macOS debug build only (`HPR_WRITE_GEODESICS=1` rewrites it); nothing in a flight uses them. Elevation (ADR-126) is `hpr_net::elevation`, heights above EGM2008 as Open-Meteo gives them, `N` left to the caller; no `hpr` command yet. WMM2025 (ADR-125) is `hpr_core::magnetic`, its files in `crates/hpr-core/data/wmm2025/`; NCEI's 100-point file's `X` is held to its measured 7.2e-4 nT residue (pygeomag 1.1.0 agrees with hpr); nothing in a flight uses the field yet. GRIB2 5.40 (ADR-124) goes through `hayro-jpeg2000`, lossless to 21 bits, strict, its header held to NCEP's coding; `whole_file.py cut-rap` writes the RAP fixture's reading; a whole RAP file through `hpr weather rap --from` is untried. `cargo xtask grib2-values FILE` + `whole_file.py compare` check a whole file by hand; the GFS file is `refs/gfs/` (AWS `noaa-gfs-bdp-pds`), and `tests/weather.rs` flies it where present. `hpr weather` (`crates/hpr-cli/src/weather.rs`) runs the library's readers; `tests/weather.rs` fills a cache over `Replay` and runs `--offline`; `hpr sim` flying a profile is #265. Sources mirror `hpr_net::nomads`/`wyoming`/`open_meteo`: a request building the URL, a pure parser, `fetch` through `Client::fetch_checked`; recordings under `crates/hpr-net/tests/fixtures/replay/` (index keyed by the exact URL), never fetched live in tests; ecCodes (`validation/oracles/grib2/eccodes_dump.py`, in `validation/oracles/pyproject.toml`) reads GRIB2 as the oracle. `Http` drops a failed answer's body. HTTPS was checked once by hand, not in CI. Python (ADR-114 to
  116): `crates/hpr-py` wraps the builder; `models.rs` holds Python drag and wind (a call re-attaches the GIL; a raised
  exception is kept in `Raised` and `Flight` raises it). `gate.sh python` builds the wheel and runs pytest, `docs/python.md`'s
  blocks and `examples/calisto.py` (its table is in the page) included. Format (ADR-111 to 113): `hpr_format::DesignFile` over `hpr_io::ork::Design`; a type change
  needs `cargo xtask format` (schema, TS, Python; `cargo test` runs node and python3) and `cargo xtask ork`; one that stops old documents reading needs a new minor version, a step in `migrate.rs`, the old schema kept, and a fixture its program wrote (ADR-112 §2). `.ork` export (ADR-109, 110): writers mirror readers; after a writer change, rerun ork.md's three commands and commit the report. Logs (ADR-108): `synthetic-pnut.pf2` is rewritten by `HPR_WRITE_SYNTHETIC_LOG=1`; the public Pnut test
  runs only where `refs/` has Debrief. CLI (ADR-105 to 108): a command goes live by leaving `registry::PLANNED`, adding its output type to `output::schemas`,
  then `cargo xtask cli`; examples name repo files from the root, and write bare names to scratch. `hpr sim`'s recovery, staging: #240. Tube fins: OR's slope and centre per part in `openrocket-tube-fin-aero.json`
  (`tube_fin_aero.py`, ADR-102); a new OR with #3235 moves its centre past Mach 0.5; body
  interference open (#234). #185: `unlit_motors.py`. Tube-fin drag likely low (#228); public drag
  curves: `PUBLIC_DRAG_CURVES`'s doc. Library runs need `drag_curves.py` (ADR-097). Probes (ADR-093,
  ADR-094): `pod_probes.py`, `rod_probes.py`, then `flights.py` (its docstring's command) and
  `motor_database.py ... refs validation/fixtures/ork/{pod,rod}-flights --jar`. #216: a `.ork` part
  with no `<finish>` gets hpr's 20 µm, OR's 60 µm. **Census (ADR-084):** a regenerated report that
  moves a row needs `cargo xtask census --accept --reason "<why>"` in the same PR, or `validate
  --check` fails. #200: Linux's reproduction bound. M2.3c (ADR-083): fly a pair with
  `hpr_validate::real_flight`, commit only statistics; `xtask real-flights --check` needs
  `refs/rocketpy`. Flutter (ADR-078): moduli in `materials::SHEAR_MODULI`; metrics (ADR-077): margin
  `None` past `κ = √10`. `.ork` since M1.9c (ADR-076): ignitions, clusters, one powered split fly;
  open: #183, #184, #185. Leads, not causes: #177, private flights above sea level reading low,
  `C03`, `C09` margins (#172). After any physics change run `cargo xtask ork-flights --check`,
  `--library --check` and `real-flights --check`: CI can't fly them; corpus reruns jitter (≤5e-7).
- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest
  (rustdoc too), labels as links to their rows — a lesson a page names needs a row in
  `decisions-and-roadmap.md` — none in headings; new pages in `SUMMARY.md`, a new library a row in
  `docs/api.md`, a milestone's row saying `done` to match. `STATUS.md` holds 150 lines and
  `ROADMAP.md` 1000: reflow a long-met `*Done when:*` list into prose (M2.2a to b4, M3.1c3, c4 are
  reflowed), and shorten old done-log entries.
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024). The wind oracle flies RocketPy 1.13.0 with #1188 and #1196 by
  `corrections.py`. **Regeneration is not bit-identical**: use `cargo xtask validate` (debug), never
  `--release`; checks allow 1e-12 but **compare a fixture's strings as text**. `cargo xtask aero`
  rewrites `arcas-robin-gap.json`'s last digits whatever you changed.
- **M1.8a to e19** (ADR-027 to ADR-050). Measurements: the guide's aero page and *Known issues*.
  Working notes: `cargo xtask aero` writes the aero fixtures (scratch in `refs/scratch/m18*/`);
  `SupersonicBody` tabulates every 0.05 Mach from max(1.2, its start), joined over 0.3;
  `BEFORE_M1_8E6` keeps the old rules and `CONE_SLOPES` runs to 30°; a mesh-following answer is
  marked by the pressure **crossing** its tangent cone's, not `η < 0`; the near-flat flare's edges
  come from `flare_reduction_turns_rad` (#108).
- **Debrief, folded in** (ADR-046): `hpr-flightdata` is off `hpr-sim` and must stay off it
  (`forbids = ["hpr-sim"]`, walked by `cargo xtask wasm-check`); sim-versus-flight goes in
  `hpr-forensics`; notes in `debrief-{log-formats,flight-readings,porting-boundary}.md`. **Port
  from its `lib/`, never its `COMPETITION.md`** (GPL-3 Java); its 12 public fixtures may be used.
- **`.ork` readings (ADR-052 to ADR-054):** angles are **degrees**, but *which way they turn* is
  assumed (OR's `+x` aft, hpr's `+z` at the nose): on the guide's not-settled list for M2.2. A
  cached `auto` number is what OpenRocket last resolved, never an input — 24.12 ignores it on
  reading — and `cargo xtask ork` holds 67 of 71 to it; nothing caches an `outerradius` or
  `innerradius`. Finish heights: the author's forum post, in `refs/sources/openrocket-finish/`.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock;
  oracles run from the repo root with `refs/venv/bin/python` (Java 17 for the OpenRocket ones);
  `xtask designs`, `examples` and `ork` rewrite their outputs.
## Done log (newest first, keep about 15)
- 2026-10-01: M6.2a CMA-ES (ADR-138): 4 test functions to 1e-10 from 20 seeds (Rosenbrock 17, the rest in its local minimum); medians within 5% of pycma 4.5.0's; 12 generations equal a dense recomputation to 1e-12; a J760 rocket hits 3,048 m to 0.086 m in 78 flights, re-flown to the bit. M6.2 split a to e. M6.1d 10,000 flights (ADR-137): Valetudo 3.00 s (4.59 before), a K940 rocket past Mach 1.6 9.43 s (264 s); samples on the nominal's layout and supersonic table, bit for bit as alone. M6.1c sensitivity (ADR-136): Morris and Sobol' (Saltelli, Jansen) within 4 standard errors of Ishigami's and g's closed forms; the errors calibrated over 500 to 1,000 seeds (15% too small fails); Morris's grid moments exact to 1e-12; the guide's 70-flight screening ranks impulse and drag for apogee, wind for landing. M6.1b landing ellipses (ADR-135): axes to 1e-14, a normal density's mass inside to 1e-12, 100,000 points' shares within 5σ, next-flight ellipses at their level; the guide's 95% ellipse holds 94.5%. M6.1a Monte Carlo dispersion (ADR-134): 11 inputs as independent normals, a stream per sample, input and copy; sample k the same over run lengths and 1, 2, 5 threads; zero dispersion flies the nominal flight bit for bit; impulse keeps `I/m_p`; failures counted, shares bounded; the guide's 200 flights. M5.5b catalogue parts in the builder (ADR-133): 3,445 built, 4 refused; held to OpenRocket's built mass and centre (revolved 6.3e-4 and 9.7e-4, the rest 1e-14; ounces apart), hollow shoulders taken out; OR's hollow walls follow r − t√(1+r′²), hpr's exact (113 integrals), 4 blunt ellipsoids apart; a stated mass scales the density; LOC 2.56 in from the catalogue flown to 1,119 m. M5.5a `.orc` reader (ADR-132): OpenRocket 24.12's 16 files bundled; 3,449 parts, 17,911 values equal to OR's preset loader to the bit; 185 ounce masses, 252 makers' names, 207 densities from a stated mass, 3 undefined materials counted with causes; 37 probes; 55 warnings.
- 2026-09-30/10-01: M5.4c `hpr motors search` (ADR-131): none at $150 on the recording (cheapest L $260.99), the one edited to $149.99 on a copy; 7 searches equal the recording's JSON filtered apart; offline as from the file; both credits on every list. M5.4b ThrustCurve (ADR-130): 282 of 282 in stock mapped, misses reported; J450DM's recorded file reads, the bundled one to the byte. M5.4a motor finder (ADR-129): 8 answers read back, offline from the cache, the credit on each; M5.4 split a to c. M5.3c2 GeoTIFF (ADR-128): rasterio's reading of 7 fixtures and a USGS tile, 4,064 places' pixel and value. M5.3c1 geodesics (ADR-127): Karney's 500,000 within his 15 nm (11.18 nm distance); 21 mirror lines take either azimuth pair. M5.3b elevation (ADR-126): 2 Open-Meteo recordings, 4 heights exact, offline from the cache. M5.3a WMM2025 (ADR-125): the report's 12 test values to their printing, its Table 3b to 5e-11, the poles; NCEI's 100 to the last digit bar `X`'s 7.2e-4 nT residue. M5.2a Open-Meteo (ADR-119), M5.2b Wyoming (ADR-120), M5.2c GFS/RAP GRIB2 (ADR-121: 6,123 values to 2.2e-16 of ecCodes): every kept level to rounding. M5.2d1 `hpr weather` (ADR-122): 5 sources from a file, 4 from the cache offline, the library's profile to the bit. M5.2d2 complex packing (ADR-123): a whole GFS file's 746,770,303 values to 4.4e-16; its profile its cut's to 1.04e-7. M5.2d3 JPEG 2000 (ADR-124): 4 RAP fields, 182,443 points, ecCodes' sums and every 13th value.
- 2026-09-26/30: M5.1 `hpr-net` (ADR-117, 118): TTL cache, offline never calls the transport, `ureq`+rustls. M4.1 builder (ADR-103, 104); M2.2 closed (ADR-094 to 102); M1.11-13; M4.3 Python (ADR-114 to 116); M3.3 `.hpr`, `.hprz`, types (ADR-111 to 113); M3.2 `.ork` writer (ADR-109, 110: OR flies 151 of 151 within 0.5%); M4.2 CLI (ADR-105 to 108).
## Needs Neer (blocking or one-way decisions; the session keeps working on other things)
- **M2.3c needs a design with its flight's log** (ADR-083): no `loft-fixtures` design is the rocket
  of a `debrief-fixtures` log. Add one pair (design file as flown, plus log, date, site, motor) to
  those repos, and an ERA5 file of the day unless cached (Data Store account). Or drop M2.3c.
- **Scrub the first revisions of #186 and #210** (1 minute each): they quote a private design's
  sizes. On each issue click *edited* → the oldest revision (*created*) → *Delete revision from history*.
- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require `fmt`, `clippy`, `doc`, `deny`,
  `wasm-check`, `site`, `types`, the three `test (...)` and `validate (...)`; block force pushes; no approvals.
- **crates.io and PyPI names** (whenever): `hpr`, `hpr-sim`, `hpr-core`… unreserved; the Python wheel is `hpr-sim` (ADR-114), built in CI, kept nowhere (licence texts first). Reserve them? The design types (ADR-113) could go to npm/PyPI. Before publishing `hpr-io`: it embeds Apache-2.0-only data (`.orc` files, ADR-132), so its `license` may need `AND Apache-2.0`; your call.
- **OpenRocket example outputs in fixtures** (no action if fine): `openrocket-automatic-radius.json`,
  `-flights.json`, `-base-drag.json`, `-drag-curves.json`, `-tube-fin-aero.json` commit numbers OR computed for its GPL examples.
- **A glance at GPL source** (no action if fine): M3.1d2's research read about 15 lines of
  `orhelper`'s (GPL-2.0) signatures before its licence was checked; nothing derived (ADR-059 §5).
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers, and
  `rocketpy-drag-curves.json` enough to rebuild 147 values of five curves (ADR-029).
- **motor.fusionspace.co and ThrustCurve.org answers in fixtures** (no action if fine): eight answers of your site's API (2.6 MB, `crates/hpr-net/tests/fixtures/replay/motor-finder-*.json`, ADR-129 §7) are committed; its terms say "free to use" but name no data licence. To say so, add one line to its `docs/api.md`. Five ThrustCurve.org answers (0.55 MB, `thrustcurve-*.json`, ADR-130 §7: three makers' records, two public-domain curves) too; its API states no terms.
- **Wyoming soundings in fixtures** (no action if fine): `crates/hpr-net/tests/fixtures/replay/wyoming-*.csv` are three answers from the University of Wyoming's archive, which states no terms; they are U.S. National Weather Service observations (U.S. government works), and Unidata's BSD `siphon` commits the same kind (ADR-120). To drop them, delete the three files and their `index.json` entries, and move `tests/wyoming.rs` onto `refs/`.
## Decided without Neer (one line each; significant ones get an ADR)
- ADR-137 (M6.1d): a run's flights share the nominal's supersonic table where its segments and area are equal, and its layout where only stage overrides and motors differ; Valetudo is "an L2 design", a K940 rocket timed beside it; timed by a bench, not a test. ADR-135 (M6.1b): the normal ellipse by default, a next-flight one by Hotelling's `T²`, the landings inside counted, not a normality test. ADR-134 (M6.1a): dispersions independent normals about nominal, no motor presets (NFPA 1125's bounds and NAR's measured spreads in the guide); one stream per sample and input; impulse with propellant mass; drag scaled in the aero model; delays cut at zero, other impossible draws fail and count. ADR-133 (M5.5b): `from_catalog` per builder part and `Fitting`; the unsaid as OpenRocket builds it, but a hollow shoulder takes its part's wall; a stated mass scales the density; an undefined part material refused, undefined lines weightless.
- ADR-132 (M5.5a): the 16 `.orc` files bundled unchanged (Apache-2.0), held to OR's loader; exact units, not OR's rounded ounce; the file's makers, densities and stated mass kept; a bad part left out with a warning, not the file; `find` by the whole number.
- ADR-103 to 131 (M4.1 to M4.3, M3.2, M3.3, M5.1 to M5.4): `hpr motors search` cheapest first, `--max-price` on the cheapest in-stock offer in exact cents, both credits on every list, no curves in the command; ThrustCurve joined by exact name, misses reported not guessed, a day's TTL, licences passed through; the motor finder's answers committed (the owner's site), an hour's TTL, its derived rules tested not enforced; a GeoTIFF over `tiff`, near-WGS 84 geographic only, GDAL's pixel and scale, metres unless stated; geodesics by `geographiclib-rs`, not a port, held to Karney's set; WMM2025 in `hpr-core`, refused outside 2025 to 2030, NCEI's `X` held to its residue; GRIB2 in-house (`grib` is f32), its JPEG 2000 by `hayro-jpeg2000`, lossless to 21 bits (its f32 5/3 wavelet); Wyoming recordings committed (U.S. government works, no terms stated), FM 35 by default, rows from the longest chain fitting the hypsometric thickness, a same-pressure run's middle row, freshness by age; Open-Meteo's ground a level, heights geopotential; `ureq` not `reqwest`, the body limit on unpacked bytes, cache paths by hand (MPL-2.0 in `directories`); cache time passed in, stale beats none; Python drag `f(mach, thrusting)`, its exception raised as raised; Calisto's RocketPy metrics measured in the example, not the library; Python wraps the builder, unit-named, abi3-py310; TS and Python types by xtask, not a third-party generator; 0.2 renames `source_files`, records the airframe's reason; breaking changes migrate; `.hpr`/`.hprz`, unknown keys refused, `hpr-format` over `hpr-io`; OR flies the export, counts only, bar on designs OR opens; `.ork` written from the design, dropped values kept, UUID ids only; `.pf2` first, a running median not Debrief's Hampel, an invented log in CI; builder over crates' types; drag models `C_D0` only; CLI adds `weather`, one subcommand per source, `--from` held to a fetch's checks (ADR-122); `hpr sim` at 0°, 0°, 0 m; one check for xtask and `hpr validate`; `.rse` filled as RockSim's.
- ADR-096 to 102 (M2.2e7 to f): fillets a section prism; a nose's `auto` bore; tube fins ring wings, 8 at most; L19 left unmet.
- ADR-081 to ADR-095 (M2.3, M2.4, M1.11 to M1.13, M2.2e5, e6): netCDF classic by hand; real flights
  a barometer; M2.3c blocked; the census a 0.1% two-way ratchet; pieces fixed before flight; tumble
  areas integrated; a shift's cycloid; a released part at `v_O + ω×c`; pods one stack repeated,
  Barrowman's once per pod, on six OR probes; a rod as OR records it; the old flag as OR reads it.
- ADR-077 to 080 (M1.10): dense-output peaks, no margin past κ = √10; flutter by TN 4197 eq. 18; exports as core text, GeoJSON on the ellipsoid, Parquet by hand.
- ADR-071 to ADR-076: M2.2e's corpus is the library's 27 `.ork` (`.CDX1`, `.rkt` wait, #168); private
  flights by id, differences only; public copies out; a cause sized by OR flying without it; M1.9's
  body 0 flies on, a motor per tube.
- ADR-062 to ADR-069: M2.2b2-d split; exact fin inertia, fillets pinned as departures; packed parts
  as OR packs them; OR flies public designs in calm air; hpr flies OR's record unrecovered.
- ADR-059 to ADR-061: key geometry agrees unless apart from RocketSerializer and OR; no `orhelper`;
  M2.2 split a to e, mass first, thresholds set first; a `.ork`'s unsaid is OR's.
- ADR-055 to ADR-058: a motor's curve is its file's own first; only what lights at launch flies;
  recovery and stored simulations read as written, not flown; the unread kept in `x-openrocket`.
- ADR-051 to ADR-054: M3.1 split a to d; a `.ork` document kept whole; an automatic dimension
  keeps both halves; angles are degrees; a radius with nothing to take is OpenRocket's 25 mm.
- ADR-047 to 050: a flare attaches by NACA 1135's wedge limit under the cone tables' 30°; model 2
  closed on the base; a step in radius keeps its model (#87, #120, #121); a reduced element takes
  the generalized method with a tangent cone of its own, a cylinder's and a boattail's refuse (#123).
- ADR-046: Debrief folded in; `hpr-flightdata` off `hpr-sim`, `hpr-forensics` added, Phase 5 re-cut, `hpr analyze` in M4.2.
- ADR-038 to ADR-040: the march behind a blunt tip starts from the tangent cone, not TN D-4865's
  Newtonian state (which fails on the Arcas nose from Mach 3.96), handover capped at 24°; M1.8e's
  15% bullet is **not met** for the body alone.
- ADR-027 to ADR-037 (details in `DECISIONS.md`): M1.8 split a to e; fins' supersonic slope counts
  both faces; bulged ogives and Haack past `C = ⅓` refused. **Two gaps visible:** M1.8's drag
  bullet, M1.8a's miss.
- ADR-001 to ADR-026 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
  nose; Niskanen's drag as printed at 20 µm; own DOPRI5; recovery in `hpr-sim`; the site's link,
  label and number checks; 3% gates or a written reason. #11: `SolidMotor` refuses `c = I/m_p`
  outside 200–5,000 m/s. M2.1b1: same-drag cases declare `C_D0(M)`.
## Known issues and risks
- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C, WMO-No. 8). Dryden turbulence
  is an aircraft model, unvalidated for rockets, and no flight uses it (#39). Only 32 motor curves
  are bundled (none in class A); the rest wait for M5's cache, whose checks ran on unpinned
  `refs/samples/`. Wall and fin mass may differ from OpenRocket's (M2.2); `.CDX1` has no public
  spec, ERA5 `.nc` may be netCDF4. A new RustSec notice can turn CI red with no code change.
  Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke: never redistribute.
- Aero (M1.5a) is small-angle only; the Recruiter's six fins miss the printed slope by +3.42%
  (ADR-008). Body lift (Jorgensen, M1.8e6) reads 1–16% high where the crossflow is supersonic. The
  normal force misses the tunnel between Mach 0.8 and 1.2; fins off, the body reads 14–38% high from
  Mach 1.5 to 2.96 (M1.8e9's bullet). A blunt tip's cap (M1.8e7) is checked only on a sphere-cone
  (#101); a boattail past 16° is worth 0.67 to 1.35 calibres of doubt. A marched flare (M1.8e18)
  reads +51.5% and +50.4% at Mach 3.95 and 4.63; a near-flat flare leaves the crossing's pole —
  +0.129% on the tests' rocket, +4.3% on a short shoulder (#108); a step in radius takes the body
  off the method past 2.7e-11 m tube to tube or 1.3e-13 m at a boattail — −8.65% to −11.34% (#87).
- `.ork` (M3.1): hpr alone flies 4 of 170 configurations (93 with OR's database, ADR-067), one
  powered split at most (#183); recovery read, not flown (`hpr sim` flies neither, #240); freeform fins, parallel stages left out; tube fins fly, drag likely low (#228); screw
  heads read simpler, warned; supersonic pressure drag twice OR's on `C06` (#222); `polished` 2 µm may be 0.5 µm in a newer OR (ADR-061).
  Pods (ADR-092) fly without pod–body interference, a single pod's moments dropped (#213); two motor pod sets refused (#214).
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic (ADR-030); against
  MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and high through Mach 1 (#67, #68); against
  the Arcas Robin it reads high at every row (#70, #72, #73); a cylinder's base drag is unmeasured
  past Mach 0.3. In wind, a slow rocket's drift rests on body lift: Juno III's apogee drift is 245 m
  in hpr, 240 to 194 m over Galejs's `K` 1.0 to 1.5 (oracle corrections, #1196).
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α` (a fall
  with no recovery glides tail-first, #241; its ascent peaks unshown, #243); two apogees on a
  near-flat rail (#242). Recovery omits canopy overshoot, opening-load factor, added mass and airframe drag; attitude freezes at
  deployment, streamer pleats unmodelled (+58% on Kidwell's), tumble reads +19%. Reports pinned
  to six decimals or 1e-7 relative; no oracle runs in CI.
