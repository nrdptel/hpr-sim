# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.6 6-DOF flight engine
- **Run:** the first autopilot run; M0.1–M0.3, M1.1–M1.4 and M1.5 have shipped
- **Last updated:** 2026-09-17 (M1.5b merged)

## Handoff (overwrite each session)

M1.5 is done (ADR-008, ADR-009, `docs/physics/aero.md`). Start M1.6 from these notes:

- **What exists:**
  - `hpr_aero::AeroModel::new(&Layout)` builds normal-force and drag terms once (about 11 µs).
    Per evaluation: `normal_force(&Flow)` (about 25 ns) and `drag(&Flow, &DragConditions)` (50 to
    110 ns), which returns `zero_lift_coefficient` and `axial_coefficient` (`C_A`, positive along
    `−z_B`, negative past 90°), and flags `beyond_subsonic_methods` above Mach 0.8.
  - `DragConditions::coasting(V/ν)` or `::thrusting(V/ν, motor_area_m2)`: `ν` from
    `hpr_atmos::AirState::kinematic_viscosity_m2_s`, the area from the burning motors'
    `MountedMotor::diameter_m` (for the RocketPy designs, the grain or nozzle exit diameter).
    `thrusting` also selects an override table's power-on curve.
  - `AeroModel::with_drag_table(DragTable)` flies another tool's `C_D0(M)`; a table accepts any
    Mach. `parse_mach_csv` reads RocketPy curves and RASAero II exports.
  - `hpr_design::Finish` on `Component::finish` (default 20 µm).
- **Contracts for M1.6:**
  - Refuse designs whose `checks::check` has `Severity::Error` findings unless the caller accepts
    them. Use `Assembly::mass_properties(t)` (about 0.1 µs) and one `AeroModel` per design.
  - The aero models are small-angle models (`α` accepted over `[0, π]`; drag mirrors past 90°).
    Decide how to treat large `α` near rail exit and apogee, and record it.
  - Normal force and the drag buildup refuse `M ≥ 1` until M1.8; drag reads low from Mach 0.6.
    A flight that goes faster needs an override table until then; decide what the engine does
    otherwise.
  - Every motor in a configuration ignites at `t = 0` until M1.9.
  - RocketPy's data files must never be committed. Oracle scripts take inputs from notebooks and
    tests only; `cargo xtask aero` shows how to commit derived numbers only.
- **Open conventions to settle with the jar (M2.2/M3.1):** OpenRocket's override order (L51),
  automatic radii, positions, ogive parameter, walls, fin cross-section mass and cant pivot; from
  M1.5b, the drag-at-angle polynomial, the lug diameter, the boattail areas and the friction area.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles (verbatim),
    lesson tests once a milestone is checked off, lock URLs, and the generated designs.
  - `cargo xtask aero` needs `refs/rocketpy`; CI checks its fixture through `hpr_aero`'s tests.
  - Scanned PDFs have no text layer: `pdftoppm -f N -l N -r 90 -gray -png` and read the image.
  - archive.org rate-limits (429). In zsh, quote curl's `'=https'`. The Bash guard hook rejects some
    tool names. On snapshot drift, run `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.5b Drag and override tables (PR #17, ADR-009): Niskanen's buildup, drag at angle
  of attack, Barrowman's roughness table, CSV override tables. At Mach 0.3 against RASAero-labelled
  curves: Calisto +4.4%, Juno III −6.0%, Cavour −8.3%; documented gap: Valetudo −47%.
- 2026-09-17: M1.5a Normal force and centre of pressure (PR #16): Barrowman slopes and CPs for
  bodies (real volumes, L9) and fins (Prandtl–Glauert, MAC, fin-count factors L8, elliptical L10,
  freeform), Galejs body lift, ADR-008. Barrowman's five worked examples (NARAM-8, TIR-33) agree
  within 1% except the Recruiter's six-fin slopes (+3.4%, +2.9%), which follow TIR-33's own rule.
- 2026-09-17: M1.4b Design tree, configurations, checks (PR #14, ADR-007): six RocketPy examples
  match to 8e-10; M1.4a Shapes, materials, component mass properties (PR #12, ADR-006).
- 2026-09-17: M1.3 Solid motors (PR #9, ADR-005): RocketPy's SolidMotor to 8e-5, `.eng`/`.rse`
  round trips of 1710 files, 32 bundled public-domain curves.
- 2026-09-17: M1.2 Atmosphere and wind (PR #7, ADR-004); M1.1 Core math, frames, Earth (PR #5).
- 2026-09-17: M0.1–M0.3 Workspace, reference library, Loft lessons (PRs #2–#4). 2026-09-16: kickoff.

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
  `[package.metadata.hpr] wasm = true`; `xtask/` at the root; crates at 0.1.0 with `publish = false`;
  the CLI binary is `hpr` with rustdoc off; copyright "Neer Patel and the hpr-sim contributors".
- ADR-002: refs pinned by commit, sha256 or dated capture, via git, curl and uv; orhelper from
  `openrocket/orhelper` at a pinned commit; Knacke from archive.org's DTIC mirror; `openjdk@21`.
- M0.3: lessons map to milestones through `Loft lessons:` lines, and checked-off milestones must
  have their lesson tests; `cargo test` caps research notes (200), ROADMAP (1,000; 40 per entry)
  and STATUS (150); subagent findings are claims until reproduced; out-of-milestone defects go to
  GitHub issues; milestones are never removed or moved later without an ADR.
- ADR-003: body `+z` toward the nose and RocketPy's launch-angle convention; ellipsoidal heights;
  exact WGS 84 normal gravity by default; Coriolis on by default.
- ADR-004: atmosphere by height above sea level; ISA offsets at equal geopotential height;
  soundings in geopotential height; wind by speed and direction; exact Dryden discretization;
  an in-house xoshiro256++. MIL-F-8785C and WMO-No. 8 pinned from mirrors.
- ADR-005: NFPA 1125 statistics as ThrustCurve's code computes them; constant exhaust velocity;
  column or BATES grains; file models keep file units; bundle only PD curves within 1% (32).
  `roxmltree` for `.rse`; ThrustCurve's `analyze.js` runs unchanged as an oracle.
- ADR-006: full inertia tensors; part frames at their forward end; Crowell's secant ogive by
  `ρ/ρ_t` (OpenRocket's contradictory `κ` left to M3.1); walls measured normal to the surface;
  fin cross-sections change the mass (NACA 00xx for "airfoil"); materials stored by value, each
  built-in value cited.
- ADR-007: body origin at the nose tip; one `Component` type with a `Part` enum; offsets positive
  aft; overrides rescale the tensor with mass; checks with error and warning severities; test
  designs as provisional JSON; the RocketPy comparison uses bundled public-domain curves because
  RocketPy's motor files have unclear terms. Fins on noses and transitions are refused for now.
- M1.4 and M1.5 were split into a and b increments, done-when bullets divided unchanged.
- ADR-008: body CP from the real volume with `sin α/α` and Galejs lift (`K` 1.1); Diederich fins
  with Prandtl–Glauert and the CP at quarter MAC to Mach 1; the technical documentation's fin-count
  factors; more than eight fins, tube fins (#15) and `M ≥ 1` refused.
- ADR-009: Niskanen's drag as printed (fully turbulent, jumps kept) with friction on the axial
  projection; boattail areas as the decrease; lug `d` the outer diameter; rail buttons as rail pins;
  derived cubic drag-at-angle; 20 µm default finish; RASAero comparison designs smooth, fins from
  the examples' airfoils; commit derived numbers only.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors: MIL-F-8785C from Abbott Aerospace, and
  WMO-No. 8 from Mongolia's weather service. If either moves, the lock file names an archive.org
  fallback for MIL-F-8785C; WMO's own library needs a browser.
- Dryden turbulence is an aircraft model. How it applies to a climbing rocket (path coordinate,
  rail start, near apogee) is unvalidated until M1.6 and M2.3.
- Only 32 curves are bundled (none in class A). The rest, and user curves, wait for M5's cache.
- The bundle's license checks and the 1710-file sweep ran on unpinned `refs/samples/` caches (POST
  captures can't be pinned yet); the committed curves are pinned by sha256.
- Wall mass (normal thickness) and fin cross-section mass may differ from OpenRocket's, whose
  conventions are undocumented; M2.2 measures and reports it. A 2 mm nose wall costs 2 ms.
- `.CDX1` has no public spec (the importer relies on samples); ERA5 `.nc` may be netCDF4 (HDF5).
- A new RustSec notice can turn CI red without a code change. Upgrade or replace the crate, or add
  an `ignore` entry with a reason.
- API snapshots can't be reproduced byte for byte on another machine once the API moves. Anything
  CI checks must come from committed fixtures, not from `refs/`.
- The Barrowman 1966 report, TIR-33, the Galejs article, the RockSim `.rse` spec and the Knacke
  manual (restrictive title-page notice) have no clear terms: cite them, never redistribute them.
- Aero (M1.5a) is small-angle only (fin slopes linear in `α`, no stall) and documented only to
  Mach 0.8; 0.8–1 is extrapolated until M1.8. Body-lift `K` is uncertain (Galejs: 1.0 to 1.5).
  The Recruiter's six fins miss TIR-33's print by +3.4% because the six-fin rules differ (ADR-008).
- Drag (M1.5b): the RASAero comparison can't show 10% agreement without the exports' inputs
  (fins and finish move each case by 20% or more). hpr misses Valetudo's suspect table by 47%, and
  its power-on base relief is about 9× that table's (ADR-009). Drag reads low from about Mach 0.6
  until M1.8 (flagged above 0.8).
- `AeroModel::normal_force` measured about 2× slower after M1.5b with nothing changed on its path
  (probably code layout; `docs/perf.md`). M1.6's flight benchmark will show whether it matters.
- `refs doctor` "runnable" means the oracle's runtime starts (imports, JVM plus jar). No flight
  has been simulated through either oracle yet; M2.x writes those scripts. Node (for
  `analyze_stats.js`) is not checked by `refs doctor`.
