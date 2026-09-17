# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.4b Design tree, configurations and checks
- **Run:** the first autopilot run; M0.1–M0.3, M1.1–M1.3 and M1.4a have shipped
- **Last updated:** 2026-09-17 (M1.4a merged)

## Handoff (overwrite each session)

M1.4 is split (ROADMAP). M1.4a built the parts and their mass properties (ADR-006). Start M1.4b
from these notes:

- **What exists:** `hpr_design` has `MassProperties` (full tensor, `translated`, `rolled`,
  `rotated`, `combine`, `validate`), `Profile`/`NoseShape`, `revolve` (filled or normal-thickness
  wall), `FinSet`/`TubeFinSet`, and `parts` (nose cone, tube, transition, inner tube, ring or
  bulkhead, lug, rail button, mass component, parachute, streamer, shock cord). Each part's frame
  has its origin on the axis at its forward end (a nose's tip), with the part at `z ≤ 0`.
  Body-attached parts take the body radius as an argument.
- **M1.4b builds on it:**
  - A tree of stages and components with axial placement (after the previous part, or relative
    to the parent's top, middle or bottom), and auto radii resolved from neighbours and parents.
  - Fix `z_ref` in `frames.md`; nose tip at `z = 0` is the obvious choice.
  - Mass/CG/inertia overrides per component and subtree.
  - Motor mounts that place `SolidMotor` elements with `MassProperties::from_motor_element`.
  - Reference diameter (L47) and typed checks (L50: a motor wider than its mount; fin roots off
    the body).
- **RocketPy comparison:** RocketPy's `Rocket` takes mass, inertia and CG without motor as inputs
  and adds the motor by the parallel-axis theorem. Match `total_mass(t)`, `center_of_mass(t)`,
  `I_11(t)` and `I_33(t)` for its example rockets (`validation/oracles/rocketpy/attitude.py` has
  Calisto's inputs); the M1670 file isn't bundled, so build its `SolidMotor` from its data file in
  the oracle script and commit the fixture.
- **Open conventions to settle in M2.2/M3.1 with the jar:** OpenRocket's ogive parameter, how it
  measures wall thickness and cuts steep ends (hpr rounds them: 2% of wall mass at 60°), whether
  its fin mass uses the cross-section, and its cant pivot.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles, lesson
    tests once a milestone is checked off, and that lock URLs use https.
  - Lesson rows name increments (`M1.4a`), and `- Loft lessons:` lines sit under the increments.
  - Data that must keep its bytes needs `-text` in `.gitattributes` (the repo forces LF).
  - archive.org rate-limits (429) and was offline on 2026-09-17; its `id_` captures are stable pins.
  - In zsh, quote curl's `'=https'`. The Bash guard hook rejects some tool names even in innocent
    phrases. Scanned PDFs need their page images read. On snapshot drift, run
    `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.4a Shapes, materials, component mass properties (PR #12): every nose and
  transition shape (clipped or not) checked against closed forms and 40-digit mpmath integrals,
  normal-thickness walls checked against an independent mpmath envelope, fin planforms with
  square/rounded/airfoil sections and tabs, all other
  parts, full inertia tensors matching hand calculations, 49 cited materials, adaptive G7K15
  quadrature in `hpr-core`, ADR-006, Wood Handbook pinned.
- 2026-09-17: M1.3 Solid motors (PR #9): thrust curves with NFPA 1125 statistics matching
  ThrustCurve's code, impulse classes, impulse-fraction consumption over a column or BATES grains,
  mass properties matching RocketPy's SolidMotor to 8e-5, `.eng`/`.rse` readers and writers that
  round-trip 1710 real files, 32 bundled public-domain curves, ADR-005, eight newly pinned sources.

- 2026-09-17: M1.2 Atmosphere and wind (PR #7): USSA76 matching its tables at 32 altitudes,
  offsets and field anchoring, moist air, sounding profiles, four wind models, exact Dryden
  turbulence with a PSD test, `hpr_core::random`, ADR-004, and five newly pinned sources.
- 2026-09-17: M1.1 Core math, frames, Earth (PR #5): `Table1D`, quaternion kinematics, ENU and
  launch-angle frames, WGS 84 geodesy (Karney's inverse), exact normal gravity and the `Earth`
  model with Coriolis, `docs/physics/` specs, ADR-003, and mpmath and RocketPy gravity fixtures.
- 2026-09-17: M0.3 Lessons from Loft (PR #4): `docs/research/loft-lessons.md` (97 lessons, 16
  process guards), `Loft lessons:` lines in ROADMAP, xtask doc checks, and the OpenRocket-source
  fetch guard.
- 2026-09-17: M0.2 Reference library (PR #3, ADR-002) and M0.1 Workspace, CI, licenses (PR #2,
  ADR-001). 2026-09-16: kickoff kit (scope, architecture, roadmap, validation inventory).

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
- M1.1: mpmath evaluates published formulas when no table exists; RocketPy conventions are pinned
  by running RocketPy; `criterion` benches hot paths, numbers in `docs/perf.md`.
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
- M1.4: split into M1.4a (parts and mass) and M1.4b (tree, configurations, checks, RocketPy), with
  the done-when bullets divided unchanged. Crowell (1996) is cited, not pinned (http-only mirror,
  archive.org offline); the Wood Handbook is pinned; data sheets are cited by URL in the code.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors: MIL-F-8785C from Abbott Aerospace, and
  WMO-No. 8 from Mongolia's weather service. If either moves, the lock file names an archive.org
  fallback for MIL-F-8785C; WMO's own library needs a browser.
- Dryden turbulence is an aircraft model. How it applies to a climbing rocket (path coordinate,
  rail start, near apogee) is unvalidated until M1.6 and M2.3.
- Only 32 curves are bundled (none in class A). The rest, and user curves, wait for M5's cache.
- The bundle's license and sample checks, and the 1710-file round-trip sweep, ran on unpinned
  caches under `refs/samples/` (POST captures can't be pinned yet); the committed curves are
  pinned by sha256.
- Wall mass (normal thickness) and fin cross-section mass may differ from OpenRocket's, whose
  conventions are undocumented; M2.2 measures and reports it. A 2 mm nose wall costs 2 ms.
- The RASAero `.CDX1` format has no public spec, so the importer relies on samples.
- ERA5 `.nc` files may be netCDF4 (HDF5), which affects the pure-Rust reader choice.
- A new RustSec vulnerability notice (anywhere in the graph) or unmaintained notice (direct
  dependencies) can turn CI red without a code change. Fix by upgrading or replacing the crate,
  or by an `ignore` entry with a reason.
- API snapshots can't be reproduced byte for byte on another machine once the API moves. Anything
  CI checks must come from committed fixtures, not from `refs/`.
- The Barrowman 1966 report, the Galejs article, the RockSim `.rse` spec and the Knacke manual
  (a contractor report with a restrictive title-page notice) have no clear terms: cite them, but
  never redistribute them.
- `refs doctor` "runnable" means the oracle's runtime starts (imports, JVM plus jar). No flight
  has been simulated through either oracle yet; M2.x writes those scripts. Node (for
  `analyze_stats.js`) is not checked by `refs doctor`.
