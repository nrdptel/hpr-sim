# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M2.1 Validation harness plus the RocketPy code-to-code suite
- **Run:** the first autopilot run; M0.1–M0.3, M1.1–M1.6 and all of M1.7 have shipped
- **Last updated:** 2026-09-17 (M1.7c merged)

## Handoff (overwrite each session)

M1.7 is done (ADR-012, ADR-013, ADR-014, `docs/physics/recovery.md`): parachutes within 3% of
RocketPy for five examples, streamers and tumble with cited drag models, and separation that flies
every body to its own landing. M2.1 is the first end-to-end milestone. Start from these notes:

- **What exists to build on:** `hpr_sim::Simulation` flies pad → rail → free flight → descent, with
  `with_recovery` and `with_separation`; `FlightResult` carries the events, the final sample and one
  `BodyFlight` per separated body. `validation/designs/` has eight RocketPy examples and two
  synthetic rockets, `validation/fixtures/` the oracle outputs, and `validation/oracles/rocketpy/`
  five generators (`recovery.py` shows the pattern: it reads the committed mass fixture, so nothing
  is transcribed twice).
- **Contracts for M2.1:**
  - It needs `hpr-validate` and `cargo xtask validate [--fast]`, TOML cases, reference JSON with
    provenance, and `validation/reports/latest.md`; `xtask` has `aero`, `designs` and `refs` to
    copy from.
  - Both modes are required: **same-drag** (the oracle's `C_D0(M)` through
    `Simulation::with_drag_table`) and **predicted** (hpr's own aero, which refuses `M ≥ 1` until
    M1.8, so supersonic cases are reported as gaps, not hidden).
  - RocketPy's motor files have unclear terms, so cases use the bundled curves (ADR-007); its
    weather files are Copernicus, so declare the environment as `recovery.py` does.
  - A start time must miss RocketPy's trigger grid (a deployment on a phase start gives NaNs) and
    its parachute noise must be zeroed (global `np.random`).
  - Differences to expect in the report: RocketPy's added mass under a canopy, its rail exit at
    the forward button, its `0.25·n²` nozzle gyration term (ADR-011).
- **Open conventions for the jar (M2.2/M3.1):** OpenRocket's override order (L51), automatic radii,
  positions, ogive parameter, walls, fin mass, cant pivot; from M1.5b the drag-at-angle polynomial
  and the lug diameter.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles, lesson
    tests once checked off, lock URLs and the generated designs.
  - `cargo xtask aero` and the oracles need `refs/rocketpy`; its data files are never committed.
    Scanned PDFs need `pdftoppm -f N -l N -r 90 -gray -png`; born-digital ones `pdftotext -layout`.
  - archive.org rate-limits (429), ScienceDirect refuses scripts (403). On snapshot drift, run
    `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.7c Separated bodies (ADR-014): a separation splits the stack at a stage boundary
  and flies each body as a point mass with its own mass and devices. Both bodies land, the masses
  add to the stack's to 1e-12 and the momenta to 1e-9.

- 2026-09-17: M1.7b Streamers and tumble (PR #24, ADR-013): Filippone's three curves by default
  (+9% on Kidwell's flat drop), appendix C's on request (+88%), OpenRocket's tumble model from the
  airframe (−10 to +19% on its own drop tests).
- 2026-09-17: M1.7a Parachutes and descent (PR #23, ADR-012): Knacke's canopy tables and filling
  law, four triggers, drogue release, a point-mass descent phase. A descent follows the closed
  form to 2.1e-8 of `v_t`; five RocketPy examples within 0.71% in descent time, 0.27% in drift.
- 2026-09-17: M1.6b Rigid-body flight (PR #22, ADR-011): RocketPy's variable-mass equations about
  the nose tip, component-wise aero with damping, rail to the last button; pitch period 8e-5 from
  linear theory, Valetudo to the ground in 1.10 ms. M1.6a Integrator and events (PR #19, ADR-010):
  DOPRI5 with dense output, RK4, stop times, Brent events; orders 5.09 and 4.01, events to 1.5e-8 s.
- 2026-09-17: M1.5b Drag and override tables (PR #17, ADR-009): Niskanen's buildup, drag at angle,
  roughness, CSV overrides. At Mach 0.3 against RASAero curves: Calisto +4.4%, Juno III −6.0%,
  Cavour −8.3%; gaps: Valetudo −47%, Cavour power-on −18%.
- 2026-09-17: M1.5a Normal force and CP (PR #16) within 1% of Barrowman's examples bar the
  Recruiter's six fins (+3.4%); M1.4b Design tree (PR #14) to 8e-10; M1.4a mass; M1.3 Solid motors
  to 8e-5; M1.2 Atmosphere; M1.1 Core math; M0.1–M0.3. 2026-09-16: kickoff.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- (not blocking, licensing) `hpr-motor` commits ThrustCurve.org's published statistics and names
  for its 32 bundled motors (`crates/hpr-motor/data/thrustcurve/catalog.json`); ThrustCurve states
  no terms for its metadata. They are facts, with attribution, and the M1.3 checks need them
  (ADR-005). Confirm, or ask John Coker; the fallback keeps only the checked numbers.
- (not blocking, safety) Loft's public flutter calculator overstates flutter speed by √2 (a 1.5
  margin is really about 1.06): `lib/sim/flutter.ts:287` uses 1.337·(λ+1)/2, where NACA TN 4197
  eq. 18 gives 2.674·(λ+1)/2. The autopilot may not post outside this repo; consider a notice or
  fix before Loft shuts down.
- (not blocking) Crate names on crates.io (`hpr`, `hpr-sim`, `hpr-core`...) are not reserved, and
  every crate has `publish = false`. Decide whether and when to reserve or publish them.
- (not blocking) `main` has no branch protection. Requiring the CI checks before merge would back
  up the guard hook; repo settings are off limits for the autopilot.
- (not blocking until M2.2) orhelper is GPL-2.0. CLAUDE.md allows running it as an oracle. But an
  OpenRocket oracle script in this MIT/Apache repo that does `import orhelper` could be read as a
  derivative work. The default, unless Neer says otherwise: M2.2 drives the jar through JPype
  (Apache-2.0) directly and keeps orhelper only as an installed, run-only tool.

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-001 to ADR-004 and M0.3, all in `DECISIONS.md`: the licence and workspace layout; refs
  pinned by hash; body `+z` toward the nose with WGS 84 normal gravity and Coriolis by default;
  the atmosphere and wind by height above sea level; and the doc guards `cargo test -p xtask` runs.
- ADR-005: NFPA 1125 statistics as ThrustCurve computes them; constant exhaust velocity; only PD
  curves within 1% bundled (32).
- ADR-006: full inertia tensors; part frames at their forward end; Crowell's secant ogive by
  `ρ/ρ_t`; walls normal to the surface; materials by value, cited.
- ADR-007: body origin at the nose tip; one `Component` type with a `Part` enum; offsets positive
  aft; overrides rescale the tensor with mass; comparisons use bundled public-domain curves.
- M1.4, M1.5, M1.6 and M1.7 were split into increments, done-when bullets divided unchanged.
- ADR-008: body CP from the real volume with `sin α/α` and Galejs lift (`K` 1.1); Diederich fins,
  CP at quarter MAC; over eight fins, tube fins (#15) and `M ≥ 1` refused.
- ADR-009: Niskanen's drag as printed (fully turbulent, jumps kept); lug `d` outer; rail buttons
  as pins; 20 µm finish; only derived numbers committed.
- ADR-010: own DOPRI5 port (no ODE crate); events stop past the zero and fire together;
  discontinuities are stop times.
- ADR-011: nose-tip reference point; nozzle gyration from the integral; mass rates by differences
  inside intervals; `M ≥ 1` stops a flight; rail `μ` 0.
- ADR-012: recovery devices live in `hpr-sim`, not the design tree; `C_D0` on Knacke's nominal
  area, the middle of his range; no overshoot or opening-load factor; a point-mass descent with no
  airframe drag or added mass; devices add, and one can release another.
- ADR-013: streamers take Filippone's three curves by default (+9% on Kidwell's flat drop), with
  appendix C's on request (+88%); tumble takes OpenRocket's §3.5 (−10 to +19% on its own drops, not
  the 3 to 14% claimed); pleats and tube fins are not modelled.
- ADR-014: a separation splits the stack at a stage boundary into two point-mass bodies with their
  own stages' mass and devices; no ejection impulse (linear momentum only), every body needs a
  device, only body 0's act before the split, and it must follow the last burnout (M1.9 stages).

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C from Abbott Aerospace, with an
  archive.org fallback; WMO-No. 8 from Mongolia's weather service).
- Dryden turbulence is an aircraft model; how it applies to a climbing rocket is unvalidated
  until M2.3.
- Only 32 curves are bundled (none in class A). The rest, and user curves, wait for M5's cache.
- The bundle's checks and the 1710-file sweep ran on unpinned `refs/samples/` caches.
- Wall and fin cross-section mass may differ from OpenRocket's undocumented conventions; M2.2
  measures it.
- `.CDX1` has no public spec (the importer relies on samples); ERA5 `.nc` may be netCDF4 (HDF5).
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason.
- API snapshots can't be reproduced byte for byte once an API moves: what CI checks comes from
  committed fixtures, never `refs/`.
- Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke's manual have no clear terms: cite
  them, never redistribute them.
- Aero (M1.5a) is small-angle only (no stall) and documented to Mach 0.8; body-lift `K` is
  uncertain (Galejs: 1.0 to 1.5) and the Recruiter's six fins miss TIR-33 by +3.4% (ADR-008).
- Drag (M1.5b): the RASAero comparison can't show 10% agreement without the exports' inputs (fins
  and finish move each case by 20% or more). hpr misses Valetudo's suspect table by 47% and
  Cavour's power-on by 18% (open; ADR-009); drag reads low from about Mach 0.6 until M1.8.
- Flight (M1.6b): no tip-off, roll forcing or damping (M1.8), turbulence or thrust misalignment;
  the small-angle aero is used at every `α`. Four `mass_properties` calls are most of an
  evaluation's 0.4 µs (`perf.md`).
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass, no airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s, above
  where hobby mains open (M1.7a). Streamer pleats are not modelled (Kidwell's pleated streamer
  descends 27% slower than his flat one, 2.2x the drag area), and the tumble model misses its own
  finless drop test by +19% and is fitted to 6.8 to 160 g models (M1.7b).
- `refs doctor` "runnable" means the oracle's runtime starts, not that a flight ran (M2.x).
