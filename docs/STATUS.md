# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.5a Normal force and centre of pressure (M1.5 Aerodynamics I)
- **Run:** the first autopilot run; M0.1–M0.3 and M1.1–M1.4 have shipped
- **Last updated:** 2026-09-17 (M1.4b merged)

## Handoff (overwrite each session)

M1.4 is done (ADR-006 parts, ADR-007 tree). Start M1.5 from these notes:

- **What exists for aero:**
  - `Rocket::layout()` returns a `Layout`: every `PlacedComponent` with its resolved `Part`
    (automatic radii filled), `fore_station_m` (aft of the nose tip, `z = −s`), `length_m`, parent
    and `body_radius_m` for fins. It also has `reference_diameter_m` and `reference_area_m2()`.
  - `Profile` gives radius and slope; `revolve` gives wetted areas; fin planforms give area and
    centroid.
  - `validation/designs/` has seven RocketPy example cases with their geometry (nose, tubes,
    conical tails, trapezoidal fins, buttons) and two synthetic rockets. `cargo xtask designs`
    regenerates them, and a test keeps them in sync.
- **M1.5 references:**
  - RASAero exports are under `refs/rocketpy/data/rockets/`: `valetudo/Cd_Power{Off,On}_RASAero.csv`,
    `calisto/power{Off,On}DragCurve.csv`, `juno3/drag_curve.csv` (check each file's origin before
    calling it RASAero).
  - Barrowman's 1966 report and 1967 thesis are pinned in `refs/papers/`.
  - The example designs' fin thickness (3 mm), square sections and walls are placeholders. Set
    any value the drag check needs from the example's own data, and record it.
- **Contracts for later milestones:**
  - M1.6 must refuse designs whose `checks::check` has `Severity::Error` findings unless the caller
    accepts them, and takes `Assembly::mass_properties(t)` (about 0.1 µs) as the mass model.
  - Every motor in a configuration ignites at `t = 0` until M1.9.
  - RocketPy's data files (motor curves, `data/rockets/*.json`) must never be committed. Oracle
    scripts take inputs from notebooks and tests only, and substitute bundled public-domain
    curves, as `rocket_mass.py` does. Valkyrie was dropped for this reason.
  - Override semantics (rescaling the tensor with mass, moving the centre) are checked only by
    hand-worked tests. RocketPy sets mass, centre and inertia together; M2.2 measures OpenRocket's.
- **Open conventions to settle with the jar (M2.2/M3.1):**
  - OpenRocket's override order on parts with shoulders (L51), automatic-radius rules and positions.
  - Its ogive parameter, wall thickness, steep ends, fin cross-section mass and cant pivot.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles, lesson
    tests once a milestone is checked off, lock URLs, and the generated designs.
  - Data that must keep its bytes needs `-text` in `.gitattributes` (the repo forces LF).
  - archive.org rate-limits (429). In zsh, quote curl's `'=https'`. The Bash guard hook rejects some
    tool names even in innocent phrases. Scanned PDFs need their page images read. On snapshot
    drift, run `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.4b Design tree, configurations and checks (PR #14): stages, positions, automatic
  radii, overrides, the reference diameter (L47), motor mounts and configurations, typed checks
  (L50), ADR-007. Six RocketPy example rockets (`rocket_mass.py`, bundled public-domain curves)
  match to 8e-10 at RocketPy's LSODA knots and 2.6e-5 between them. `cargo xtask designs` writes
  `validation/designs/`.
- 2026-09-17: M1.4a Shapes, materials, component mass properties (PR #12): every nose and
  transition shape checked against closed forms and mpmath, normal-thickness walls, fin planforms
  and sections, all other parts, full inertia tensors by hand, 49 cited materials, G7K15 quadrature,
  ADR-006.
- 2026-09-17: M1.3 Solid motors (PR #9): NFPA 1125 statistics matching ThrustCurve's code, grain
  consumption, mass properties matching RocketPy's SolidMotor to 8e-5, `.eng`/`.rse` round trips of
  1710 files, 32 bundled public-domain curves, ADR-005.
- 2026-09-17: M1.2 Atmosphere and wind (PR #7, ADR-004) and M1.1 Core math, frames, Earth (PR #5,
  ADR-003): USSA76, soundings, four wind models, Dryden turbulence; WGS 84 geodesy and gravity.
- 2026-09-17: M0.3 Lessons from Loft (PR #4), M0.2 Reference library (PR #3, ADR-002), M0.1
  Workspace, CI, licenses (PR #2, ADR-001). 2026-09-16: kickoff kit.

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
  the done-when bullets divided unchanged. Crowell (1996) is cited, not pinned.
- ADR-007: body origin at the nose tip; one `Component` type with a `Part` enum; offsets positive
  aft; overrides rescale the tensor with mass; checks with error and warning severities; test
  designs as provisional JSON; the RocketPy comparison uses bundled public-domain curves because
  RocketPy's motor files have unclear terms. Fins on noses and transitions are refused for now.

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
