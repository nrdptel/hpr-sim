# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.7b Streamers, tumble and separated bodies
- **Run:** the first autopilot run; M0.1–M0.3, M1.1–M1.6 and M1.7a have shipped
- **Last updated:** 2026-09-17 (M1.7a merged)

## Handoff (overwrite each session)

M1.7a is done (ADR-012, `docs/physics/recovery.md`): parachutes, triggers, inflation, release,
drift and landing, within 3% of RocketPy for five examples. Start M1.7b from these notes:

- **What exists:** `hpr_sim::recovery` has `Device` (a `DeviceDrag`, a `Trigger`, a lag, an
  `Inflation`, an optional `released_by`), Knacke's `CanopyType` table and `terminal_speed_m_s`;
  `Simulation::with_recovery` validates them. The first deployment switches the flight to
  `Phase::Descent`, a point mass under the open devices' drag area (`dynamics::canopy_drag`).
  Deployments, filling ends and known triggers are stop times; the per-interval event list is
  `flight::Watch`.
- **Contracts for M1.7b:**
  - **Streamers need a source.** Knacke has no streamer data at all (checked: tables 5-1 to 5-5,
    section 5.8.4, the contents). Try the OpenRocket technical documentation (CC BY-SA,
    `refs/papers/`), Niskanen's thesis, or the jar in M2.2; never Loft's GPL-derived defaults (L29).
  - **Tumble** is where the airframe's own drag matters; the descent phase leaves airframe drag
    out (ADR-012), so add a cited broadside model, not the small-angle aero.
  - **Separation** needs an ADR on how a body's mass and drag are defined: `Assembly` has no
    split, and the loop carries one 13-element state. `Phase` is `#[non_exhaustive]`, so new
    phases are additive; `Termination` may need a variant if only some bodies land.
- **For M2.1:** the recovery oracle (`validation/oracles/rocketpy/recovery.py`) takes its inputs
  from the committed mass fixture; a start time must miss RocketPy's sampling grid (a deployment on
  a phase start collides and gives NaNs) and its parachute noise must be zeroed (global
  `np.random`). RocketPy ends the rail phase at the forward button and codes the nozzle gyration
  tensor's transverse term with `0.25·n²` (ADR-011).
- **Open conventions for the jar (M2.2/M3.1):** OpenRocket's override order (L51), automatic radii,
  positions, ogive parameter, walls, fin mass and cant pivot; and from M1.5b the drag-at-angle
  polynomial, the lug diameter and the boattail areas.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles, lesson
    tests once a milestone is checked off, lock URLs, and the generated designs.
  - `cargo xtask aero` and the oracles need `refs/rocketpy`; its data files are never committed.
    Scanned PDFs have no text layer: `pdftoppm -f N -l N -r 90 -gray -png` and read the image.
  - archive.org rate-limits (429), ScienceDirect refuses scripts (403). On snapshot drift, run
    `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.7a Parachutes and descent (ADR-012): Knacke's canopy tables and filling law, four
  triggers, drogue release, a point-mass descent phase. Terminal speed reproduces Loft's 5.294 m/s;
  a descent follows the closed form to 2.1e-8 of `v_t`; drift is the wind times the flight time to
  1e-8; five RocketPy examples within 0.71% in descent time and 0.27% in drift.
- 2026-09-17: M1.6b Rigid-body flight (PR #22, ADR-011): RocketPy's variable-mass equations about
  the nose tip, component-wise aero with rotational damping, rail to the last button. Pitch period
  8e-5 from linear theory; jet damping 1e-6; Valetudo to the ground in 1.10 ms.
- 2026-09-17: M1.6a Integrator and events (PR #19, ADR-010): DOPRI5 port with dense output, RK4,
  stop times, Brent events. Step halving: orders 5.09 and 4.01; events within 1.5e-8 s.
- 2026-09-17: M1.5b Drag and override tables (PR #17, ADR-009): Niskanen's buildup, drag at angle
  of attack, roughness, CSV overrides. At Mach 0.3 against RASAero-labelled curves: Calisto +4.4%,
  Juno III −6.0%, Cavour −8.3%; gaps: Valetudo −47%, Cavour power-on −18%.
- 2026-09-17: M1.5a Normal force and CP (PR #16, ADR-008): Barrowman's examples within 1% except
  the Recruiter's six fins (+3.4%). M1.4b Design tree (PR #14, ADR-007) to 8e-10; M1.4a mass
  (PR #12); M1.3 Solid motors (PR #9, ADR-005) to 8e-5; M1.2 Atmosphere (PR #7, ADR-004); M1.1
  Core math (PR #5); M0.1–M0.3 (PRs #2–#4). 2026-09-16: kickoff.

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
- ADR-002: refs pinned by commit, sha256 or dated capture, via git, curl and uv; Knacke from
  archive.org's DTIC mirror; `openjdk@21`.
- M0.3: lessons map to milestones through `Loft lessons:` lines, whose tests must exist once the
  milestone is checked off; `cargo test` caps the docs; defects outside a milestone go to issues.
- ADR-003: body `+z` toward the nose and RocketPy's launch-angle convention; ellipsoidal heights;
  exact WGS 84 normal gravity by default; Coriolis on by default.
- ADR-004: atmosphere by height above sea level; ISA offsets at equal geopotential height; wind by
  speed and direction; exact Dryden discretization; an in-house xoshiro256++.
- ADR-005: NFPA 1125 statistics as ThrustCurve computes them; constant exhaust velocity; column or
  BATES grains; file units kept; only PD curves within 1% bundled (32); `roxmltree` for `.rse`.
- ADR-006: full inertia tensors; part frames at their forward end; Crowell's secant ogive by
  `ρ/ρ_t` (OpenRocket's contradictory `κ` left to M3.1); walls normal to the surface; fin
  cross-sections change the mass; materials stored by value, each built-in value cited.
- ADR-007: body origin at the nose tip; one `Component` type with a `Part` enum; offsets positive
  aft; overrides rescale the tensor with mass; typed check severities; test designs as provisional
  JSON; comparisons use bundled public-domain curves (RocketPy's motor files have unclear terms).
- M1.4, M1.5, M1.6 and M1.7 were split into increments, done-when bullets divided unchanged.
- ADR-008: body CP from the real volume with `sin α/α` and Galejs lift (`K` 1.1); Diederich fins
  with Prandtl–Glauert, CP at quarter MAC; over eight fins, tube fins (#15) and `M ≥ 1` refused.
- ADR-009: Niskanen's drag as printed (fully turbulent, jumps kept), friction on the axial
  projection; boattail areas as the decrease; lug `d` outer; rail buttons as pins; derived cubic
  drag-at-angle; 20 µm default finish; only derived numbers committed.
- ADR-010: own DOPRI5 port (no ODE crate); the system carries weights, events and observer; events
  stop past the zero on the dense output and fire together; discontinuities are stop times.
- ADR-011: nose-tip reference point; nozzle gyration tensor from the integral; mass rates by
  differences inside intervals; no Earth rate in rotation; `M ≥ 1` stops a flight; rail `μ` 0.
- ADR-012: recovery devices live in `hpr-sim`, not the design tree; `C_D0` on Knacke's nominal
  area, defaulting to the middle of his printed range; his filling time and growth exponents, with
  no overshoot and no opening-load factor; a point-mass descent phase with no airframe drag and no
  added mass; devices add their drag areas, and one can release another.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C from Abbott Aerospace, with an
  archive.org fallback in the lock; WMO-No. 8 from Mongolia's weather service, whose own library
  needs a browser).
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
- Recovery (M1.7a): no canopy overshoot or opening-load factor (the peak load is a lower bound),
  no added mass, no airframe drag under a canopy, and the attitude freezes at deployment.
- `refs doctor` "runnable" means the oracle's runtime starts, not that a flight ran (M2.x).
