# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.7c Separated bodies
- **Run:** the first autopilot run; M0.1–M0.3, M1.1–M1.6, M1.7a and M1.7b have shipped
- **Last updated:** 2026-09-17 (M1.7b merged)

## Handoff (overwrite each session)

M1.7a and M1.7b are done (ADR-012, ADR-013, `docs/physics/recovery.md`): parachutes within 3% of
RocketPy for five examples, and streamers and tumble with cited drag models. Start M1.7c
(separated bodies) from these notes:

- **What exists:** `hpr_sim::recovery` has `Device` (a `DeviceDrag` — a drag area, a canopy, a
  streamer or a tumbling airframe — with a `Trigger`, a lag, an `Inflation` and an optional
  `released_by`), Knacke's `CanopyType` table, `StreamerModel` and `terminal_speed_m_s`;
  `Simulation::with_recovery` validates them. The first deployment switches the flight to
  `Phase::Descent`, a point mass under the open devices' drag area. Deployments, filling ends and
  known triggers are stop times; the per-interval event list is `flight::Watch`.
- **Contracts for M1.7c:** separation needs an ADR on how a body's mass and drag are defined.
  `Assembly` has no split, and the flight loop carries one 13-element state, so two bodies mean
  two integrations. The cheapest honest way in (ADR-013): the descent phase already drops airframe
  aerodynamics, so a separated body needs mass properties and its own device, not an aerodynamic
  model; require every body to carry one. `Phase` is `#[non_exhaustive]`, so new phases are
  additive; `Termination` may need a variant if only some bodies land. Sources for anything more:
  `docs/research/streamer-and-tumble-drag.md`.
- **For M2.1:** the recovery oracle takes its inputs from the committed mass fixture; a start time
  must miss RocketPy's sampling grid (a deployment on a phase start gives NaNs) and its noise must
  be zeroed (global `np.random`). RocketPy ends the rail phase at the forward button and codes the
  nozzle gyration tensor's transverse term with `0.25·n²` (ADR-011).
- **Open conventions for the jar (M2.2/M3.1):** OpenRocket's override order (L51), automatic radii,
  positions, ogive parameter, walls, fin mass, cant pivot; from M1.5b the drag-at-angle polynomial
  and the lug diameter; from M1.7b, that hpr's streamers are about twice OpenRocket's drag.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles, lesson
    tests once checked off, lock URLs and the generated designs.
  - `cargo xtask aero` and the oracles need `refs/rocketpy`; its data files are never committed.
    Scanned PDFs need `pdftoppm -f N -l N -r 90 -gray -png`; born-digital ones `pdftotext -layout`.
  - archive.org rate-limits (429), ScienceDirect refuses scripts (403). On snapshot drift, run
    `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.7b Streamers and tumble (ADR-013): Filippone's correlation by default, appendix
  C's on request, OpenRocket's tumble model from the airframe. Against Kidwell's drop tests the
  default is +12% on his flat streamer where appendix C is +110%.
- 2026-09-17: M1.7a Parachutes and descent (PR #23, ADR-012): Knacke's canopy tables and filling
  law, four triggers, drogue release, a point-mass descent phase. Terminal speed reproduces Loft's
  5.294 m/s; a descent follows the closed form to 2.1e-8 of `v_t`; five RocketPy examples within
  0.71% in descent time and 0.27% in drift.
- 2026-09-17: M1.6b Rigid-body flight (PR #22, ADR-011): RocketPy's variable-mass equations about
  the nose tip, component-wise aero with damping, rail to the last button; pitch period 8e-5 from
  linear theory, Valetudo to the ground in 1.10 ms. M1.6a Integrator and events (PR #19, ADR-010):
  DOPRI5 with dense output, RK4, stop times, Brent events; orders 5.09 and 4.01, events to 1.5e-8 s.
- 2026-09-17: M1.5b Drag and override tables (PR #17, ADR-009): Niskanen's buildup, drag at angle,
  roughness, CSV overrides. At Mach 0.3 against RASAero curves: Calisto +4.4%, Juno III −6.0%,
  Cavour −8.3%; gaps: Valetudo −47%, Cavour power-on −18%.
- 2026-09-17: M1.5a Normal force and CP (PR #16, ADR-008) within 1% of Barrowman's examples except
  the Recruiter's six fins; M1.4b Design tree (PR #14) to 8e-10; M1.4a mass; M1.3 Solid motors to
  8e-5; M1.2 Atmosphere; M1.1 Core math; M0.1–M0.3. 2026-09-16: kickoff.

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

- ADR-001: `MIT OR Apache-2.0`; permissive-only dependencies; pure core marked by
  `[package.metadata.hpr] wasm = true`; crates at 0.1.0, `publish = false`; the binary is `hpr`.
- ADR-002: refs pinned by commit, sha256 or dated capture, via git, curl and uv; `openjdk@21`.
- M0.3: lessons map to milestones through `Loft lessons:` lines, whose tests must exist once
  checked off; `cargo test` caps the docs; defects outside a milestone go to issues.
- ADR-003: body `+z` toward the nose, RocketPy's launch angles, ellipsoidal heights, exact WGS 84
  normal gravity and Coriolis by default.
- ADR-004: atmosphere by height above sea level; ISA offsets at equal geopotential height; wind by
  speed and direction; exact Dryden discretization; an in-house xoshiro256++.
- ADR-005: NFPA 1125 statistics as ThrustCurve computes them; constant exhaust velocity; BATES or
  column grains; only PD curves within 1% bundled (32).
- ADR-006: full inertia tensors; part frames at their forward end; Crowell's secant ogive by
  `ρ/ρ_t` (OpenRocket's `κ` left to M3.1); walls normal to the surface; materials by value, cited.
- ADR-007: body origin at the nose tip; one `Component` type with a `Part` enum; offsets positive
  aft; overrides rescale the tensor with mass; comparisons use bundled public-domain curves.
- M1.4, M1.5, M1.6 and M1.7 were split into increments, done-when bullets divided unchanged.
- ADR-008: body CP from the real volume with `sin α/α` and Galejs lift (`K` 1.1); Diederich fins
  with Prandtl–Glauert, CP at quarter MAC; over eight fins, tube fins (#15) and `M ≥ 1` refused.
- ADR-009: Niskanen's drag as printed (fully turbulent, jumps kept), friction on the axial
  projection; lug `d` outer; rail buttons as pins; 20 µm finish; only derived numbers committed.
- ADR-010: own DOPRI5 port (no ODE crate); the system carries weights, events and observer;
  events stop past the zero and fire together; discontinuities are stop times.
- ADR-011: nose-tip reference point; nozzle gyration tensor from the integral; mass rates by
  differences inside intervals; no Earth rate in rotation; `M ≥ 1` stops a flight; rail `μ` 0.
- ADR-012: recovery devices live in `hpr-sim`, not the design tree; `C_D0` on Knacke's nominal
  area, the middle of his range; his filling law, no overshoot or opening-load factor; a point-mass
  descent with no airframe drag or added mass; devices add, and one can release another.
- ADR-013: streamers take Filippone's three curves by default (+9% on Kidwell's flat drop), with
  appendix C's on request (+88%); tumble takes OpenRocket's §3.5, which reproduces its own drop
  tests to −10 to +19%, not the 3 to 14% claimed; pleats and tube fins are not modelled.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C from Abbott Aerospace, with an
  archive.org fallback; WMO-No. 8 from Mongolia's weather service).
- Dryden turbulence is an aircraft model; how it applies to a climbing rocket is unvalidated
  until M2.3.
- Only 32 curves are bundled (none in class A). The rest, and user curves, wait for M5's cache.
- The bundle's checks and the 1710-file sweep ran on unpinned `refs/samples/` caches; the committed
  curves are pinned by sha256.
- Wall and fin cross-section mass may differ from OpenRocket's undocumented conventions; M2.2
  measures it.
- `.CDX1` has no public spec (the importer relies on samples); ERA5 `.nc` may be netCDF4 (HDF5).
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason.
- API snapshots can't be reproduced byte for byte once an API moves: what CI checks comes from
  committed fixtures, never `refs/`.
- Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke's manual have no clear terms: cite
  them, never redistribute them.
- Aero (M1.5a) is small-angle only (no stall) and documented to Mach 0.8; 0.8–1 is extrapolated
  until M1.8. Body-lift `K` is uncertain (Galejs: 1.0 to 1.5), and the Recruiter's six fins miss
  TIR-33's print by +3.4% (ADR-008).
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
