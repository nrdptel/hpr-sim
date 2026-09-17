# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M2.1b The RocketPy code-to-code suite (M2.1a shipped)
- **Run:** the first autopilot run; M0.1–M0.3, M1.1–M1.7 and M2.1a have shipped
- **Last updated:** 2026-09-17 (M2.1a merged)

## Handoff (overwrite each session)

M2.1a shipped the validation harness (ADR-015, `docs/VALIDATION.md`): `cargo xtask validate
[--fast]` runs the cases in `validation/cases/lock.toml` against stored references and writes
`validation/reports/latest.{md,json}`. Five descent cases, 30 metrics: 29 scored and all inside
the milestone's 3% (worst +2.87%), 1 declared not scored (issue #27). M2.1b is the suite itself.
Start from these notes:

- **The harness:** a case is `validation/cases/<id>.toml` (`Flight` says what to fly, `metrics`
  names each metric's tolerance, `reference` points at a fixture); `hpr-validate` reads it, flies
  hpr and reports, and `rocketpy.rs` is the only place that knows a generator's JSON shape. It
  never writes a reference (L76), refuses a value with no source (L77) or a metric with no gate
  (L79), and fails on a locked case it cannot find or a committed case the lock does not name
  (L78). A metric that cannot honestly be gated is declared `not_scored = "<reason naming an
  issue>"`: printed, never a pass, the set pinned by a test. No absolute floors; use that hatch.
- **What M2.1b adds:** a `Flight::WholeFlight` variant beside `RecoveryDescent`, M2.1's whole-flight
  metrics (apogee and time to it, max velocity/Mach/acceleration, rail exit, burnout, time-series
  RMS), both modes, the CI job and the regeneration workflow.
- **Both modes are required:** **same-drag** (the oracle's `C_D0(M)` through
  `Simulation::with_drag_table`) and **predicted** (hpr's own aero, which refuses `M ≥ 1` until
  M1.8, so supersonic cases are gaps in the report, not hidden). Predicted mode needs its own
  looser, stated tolerance.
- **Oracle notes:** RocketPy's motor files have unclear terms, so cases use the bundled curves
  (ADR-007); its weather files are Copernicus, so declare the environment as `recovery.py` does.
  Zero the parachute noise (global `np.random`); a deployment on a phase start gives NaNs. Expect
  differences from its added mass, its rail exit at the forward button and `0.25·n²` (ADR-011).
- **Open conventions for the jar (M2.2/M3.1):** override order (L51), automatic radii, positions,
  ogive, walls, fin mass, cant pivot, the drag-at-angle polynomial, lug diameter.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles, lesson
    tests once checked off, lock URLs and the generated designs.
  - `cargo xtask aero` and the oracles need `refs/rocketpy`; its data files are never committed.
    Scanned PDFs need `pdftoppm -f N -l N -r 90 -gray -png`, born-digital ones `pdftotext -layout`;
    archive.org rate-limits (429) and ScienceDirect refuses scripts (403). On snapshot drift, run
    `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M2.1a Validation harness (ADR-015): TOML cases, references with per-value provenance
  and a hash, per-metric 3% gates, a case lock and committed reports. Five descent cases, 30
  metrics: 29 scored and inside 3% (worst +2.87%), 1 not scored (#27). L76–L79 have live tests.
- 2026-09-17: M1.7c Separated bodies (ADR-014): a separation splits the stack at a stage boundary
  and flies each body as a point mass with its own mass and devices. Both bodies land, the masses
  add to the stack's to 1e-12 and the momenta to 1e-9.
- 2026-09-17: M1.7b Streamers and tumble (PR #24, ADR-013): Filippone's three curves by default
  (+9% on Kidwell's flat drop), appendix C's on request (+88%), OpenRocket's tumble model from the
  airframe (−10 to +19% on its own drop tests).
- 2026-09-17: M1.7a Parachutes and descent (PR #23, ADR-012): Knacke's canopy tables and filling
  law, four triggers, drogue release, a point-mass descent phase. A descent follows the closed
  form to 2.1e-8 of `v_t`; five RocketPy examples within 0.71% in descent time, 0.27% in drift.
- 2026-09-17: M1.6b Rigid-body flight (ADR-011): variable-mass equations about the nose tip,
  component-wise aero, rail to the last button; pitch period 8e-5 from linear theory. M1.6a
  Integrator (ADR-010): DOPRI5 with dense output, RK4, Brent events; orders 5.09 and 4.01.
- 2026-09-17: M1.5b Drag and override tables (ADR-009): Niskanen's buildup, drag at angle,
  roughness, CSV overrides. At Mach 0.3 against RASAero: Calisto +4.4%, Juno III −6.0%, Cavour
  −8.3%; gaps: Valetudo −47%, Cavour power-on −18%.
- 2026-09-17: M1.5a CP within 1% of Barrowman bar the Recruiter's six fins (+3.4%); M1.4b Design
  tree to 8e-10; M1.4a mass; M1.3 Solid motors to 8e-5; M1.2 Atmosphere; M1.1; M0.1–M0.3.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- (not blocking, licensing) `hpr-motor` commits ThrustCurve.org's published statistics and names for
  its 32 bundled motors (`crates/hpr-motor/data/thrustcurve/catalog.json`), which states no terms
  for its metadata. They are facts, with attribution, and the M1.3 checks need them (ADR-005).
  Confirm, or ask John Coker; the fallback keeps only the checked numbers.
- (not blocking, safety) Loft's public flutter calculator overstates flutter speed by √2 (a 1.5
  margin is really about 1.06): `lib/sim/flutter.ts:287` uses 1.337·(λ+1)/2, where NACA TN 4197
  eq. 18 gives 2.674·(λ+1)/2. Consider a notice or fix before Loft shuts down.
- (not blocking) Crate names on crates.io (`hpr`, `hpr-sim`, `hpr-core`...) are not reserved, and
  every crate has `publish = false`. Decide whether and when to reserve or publish them.
- (not blocking) `main` has no branch protection. Requiring the CI checks before merge would back
  up the guard hook; repo settings are off limits for the autopilot.
- (not blocking until M2.2) orhelper is GPL-2.0, and CLAUDE.md allows running it as an oracle, but a
  script in this MIT/Apache repo that does `import orhelper` could be read as a derivative work.
  The default: M2.2 drives the jar through JPype (Apache-2.0) and keeps orhelper run-only.

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-001 to ADR-004 and M0.3, all in `DECISIONS.md`: the licence and workspace layout; refs
  pinned by hash; body `+z` toward the nose with WGS 84 normal gravity and Coriolis by default;
  the atmosphere and wind by height above sea level; and the doc guards `cargo test -p xtask` runs.
- ADR-005: NFPA 1125 statistics as ThrustCurve computes them; constant exhaust velocity; 32 curves.
- ADR-006: full inertia tensors; part frames at their forward end; Crowell's secant ogive; walls
  normal to the surface; materials by value, cited.
- ADR-007: body origin at the nose tip; one `Component` type with a `Part` enum; offsets positive
  aft; overrides rescale the tensor with mass; comparisons use bundled public-domain curves.
- M1.4, M1.5, M1.6 and M1.7 were split into increments, done-when bullets divided unchanged.
- ADR-008: body CP from the real volume with `sin α/α` and Galejs lift (`K` 1.1); Diederich fins at
  quarter MAC; over eight fins, tube fins (#15) and `M ≥ 1` refused.
- ADR-009: Niskanen's drag as printed (fully turbulent, jumps kept); lug `d` outer; rail buttons
  as pins; 20 µm finish; only derived numbers committed.
- ADR-010: own DOPRI5 (no ODE crate); events stop past the zero; discontinuities are stop times.
- ADR-011: nose-tip reference point; nozzle gyration from the integral; mass rates by differences
  inside intervals; `M ≥ 1` stops a flight; rail `μ` 0.
- ADR-012: recovery devices live in `hpr-sim`, not the design tree; `C_D0` on Knacke's nominal area,
  the middle of his range; a point-mass descent, no added mass; devices add and can release one
  another.
- ADR-013: streamers take Filippone's three curves by default (+9% on Kidwell's flat drop), appendix
  C's on request (+88%); tumble takes OpenRocket's §3.5 (−10 to +19% on its own drops, not the 3 to
  14% claimed).
- ADR-014: a separation splits the stack at a stage boundary into two point-mass bodies with their
  own stages' mass and devices; no ejection impulse (linear momentum only), every body needs a
  device, only body 0's act before the split, and it must follow the last burnout (M1.9 stages).
- ADR-015: a run reads references and never writes them (no update flag); every value carries its
  generator's source and the file its hash; every reported metric is gated at the milestone's 3%
  with no absolute floor, or declared not scored in writing against an issue; the locked cases must
  all run; a case's inputs, design and mass come from the reference's own record of what the oracle
  flew; the report is committed and carries no date.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C from Abbott Aerospace; WMO-No. 8
  from Mongolia's weather service). Dryden turbulence is an aircraft model, unvalidated until M2.3.
- Only 32 curves are bundled (none in class A); the rest wait for M5's cache. The bundle's checks
  and the 1710-file sweep ran on unpinned `refs/samples/` caches.
- Wall and fin mass may differ from OpenRocket's undocumented conventions; M2.2 measures it.
- `.CDX1` has no public spec (the importer relies on samples); ERA5 `.nc` may be netCDF4 (HDF5).
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason. API snapshots can't be reproduced once an API moves: CI checks committed fixtures only.
- Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke have no clear terms: never redistribute.
- Aero (M1.5a) is small-angle only and documented to Mach 0.8; body-lift `K` is uncertain (Galejs:
  1.0 to 1.5) and the Recruiter's six fins miss TIR-33 by +3.4% (ADR-008).
- Drag (M1.5b): the RASAero comparison can't show 10% agreement without the exports' inputs (fins
  and finish move each case by 20% or more). hpr misses Valetudo's suspect table by 47% and
  Cavour's power-on by 18% (open; ADR-009); drag reads low from about Mach 0.6 until M1.8.
- Flight (M1.6b): no tip-off, roll forcing or damping (M1.8), turbulence or thrust misalignment;
  the small-angle aero is used at every `α`. Four `mass_properties` calls are most of an
  evaluation's 0.4 µs (`perf.md`).
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s, above
  where hobby mains open (M1.7a). Streamer pleats are not modelled (Kidwell's pleated streamer
  descends 27% slower, 2.2x the drag area); tumble misses its own finless drop by +19% (M1.7b).
- `refs doctor` "runnable" means the oracle's runtime starts, not that a flight ran; no oracle runs
  in CI, which compares against stored output (M2.1b).
- Valetudo's northward drift is 0.55 mm in hpr against RocketPy's 0.020 mm, a factor of 28 on a
  quantity both codes compute the same way; a quasi-steady Coriolis balance gives RocketPy's value,
  so hpr's is suspect. Unexplained, unscored in the suite, open as issue #27 (M2.1a).
