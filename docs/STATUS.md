# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.2 Atmosphere and wind
- **Run:** the first autopilot run; M0.1, M0.2, M0.3 and M1.1 have shipped
- **Last updated:** 2026-09-17 (M1.1 merged)

## Handoff (overwrite each session)

M1.1 put the conventions every later crate uses into `hpr-core` (ADR-003). Start M1.2 from these
notes:

- **Frames are fixed** in `docs/physics/frames.md`:
  - ENU launch frame at the pad; body `+z` points to the nose.
  - Hamilton quaternion from body to launch frame.
  - Launch angles use RocketPy's 3-1-3 convention.
  - All heights are **ellipsoidal**. Atmosphere and wind tables keyed on height above sea level
    must say so and convert at the boundary. Geoid undulation is up to ±100 m.
- **Use `hpr_core::interp::Table1D`** for soundings and profiles.
  - It interpolates linearly or with a natural cubic, and every lookup flags extrapolation.
  - Linear tables suit data with kinks, such as USSA76 layer boundaries. Natural cubics overshoot.
- **Gravity for geopotential altitude.** USSA76 uses its own constant `g₀ = 9.80665` and radius
  `r₀`. Use those, not `NormalGravity`, when converting geometric to geopotential height
  (`STANDARD_GRAVITY_MPS2` exists for this).
- **Reference values:** where no table is published, follow
  `validation/oracles/wgs84/normal_gravity.py`: evaluate the published formulas in mpmath, check
  against the printed digits, commit the JSON under `validation/fixtures/`, and read it with
  `include_str!`.
- **For M1.6 and M2.1** (recorded in ADR-003 and `docs/physics/gravity.md`):
  - The flight engine must use geodetic height, not `z_L`, for ground contact and apogee.
  - RocketPy evaluates gravity at height above sea level and holds it constant above 80 km.
- **Process notes:**
  - `cargo test -p xtask` checks that STATUS names the first open ROADMAP milestone, and that each
    checked-off milestone's lesson tests exist. For M1.2 those are L2 to L6.
  - The Bash guard hook rejects some tool names even in innocent phrases, so word commit messages
    and PR bodies plainly.
  - If a refs fetch reports snapshot drift, run `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.1 Core math, frames, Earth (PR #5): `Table1D`, quaternion kinematics, ENU and
  launch-angle frames, WGS 84 geodesy (Karney's inverse), exact normal gravity and the `Earth`
  model with Coriolis, `docs/physics/` specs, ADR-003, and mpmath and RocketPy gravity fixtures.
- 2026-09-17: M0.3 Lessons from Loft (PR #4): `docs/research/loft-lessons.md` (97 lessons, 16
  process guards), `Loft lessons:` lines in ROADMAP, xtask doc checks, and the OpenRocket-source
  fetch guard.
- 2026-09-17: M0.2 Reference library (PR #3): `xtask refs fetch|verify|doctor`, 23 pinned
  references plus the uv oracle environment, notices rows per source, ADR-002.
- 2026-09-17: M0.1 Workspace, CI, licenses (PR #2): workspace skeleton, xtask wasm-check,
  cargo-deny policy, CI on three OSes, ADR-001.
- 2026-09-16: Kickoff kit written. Scope, architecture, roadmap and validation inventory are
  set.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

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

- ADR-001: `MIT OR Apache-2.0` confirmed; permissive-only dependency licenses (weak copyleft
  rejected too); pure core marked by `[package.metadata.hpr] wasm = true`.
- `xtask` lives at the repo root (`xtask/`), not under `crates/` (ADR-001).
- MIT copyright line reads "Neer Patel and the hpr-sim contributors".
- Crates start at version 0.1.0 with `publish = false`.
- The CLI binary is named `hpr` and has rustdoc disabled to avoid colliding with the `hpr` facade.
- ADR-002: the reference library pins git by commit, files by sha256, and live APIs by dated
  capture; it shells out to git, curl and uv instead of adding HTTP crates.
- orhelper comes from `openrocket/orhelper` at a pinned commit, not PyPI (0.1.3 predates OR 24.12).
- Knacke's manual is fetched from archive.org's mirror of the DTIC copy (DTIC blocks automated
  downloads).
- Installed `openjdk@21` with Homebrew on the dev Mac (the preflight script names this step), so
  the OpenRocket oracle runs.
- M0.3: Loft lessons map to milestones through `Loft lessons:` lines, and an xtask test requires a
  checked-off milestone's lesson tests to exist as live `#[test]`s. This tightens later *done
  when* criteria; nothing was loosened. Moving a lesson to a later milestone needs an ADR.
- M0.3: `cargo test` caps research notes at 200 lines, ROADMAP at 1,000 (40 per entry) and STATUS
  at 150, and checks that STATUS's current milestone matches ROADMAP.
- M0.3: new process rules: subagent findings are claims until reproduced; defects outside the
  milestone go to GitHub issues; milestones are never removed or moved later without an ADR.

- ADR-003: body `+z` toward the nose and RocketPy's launch-angle convention; ellipsoidal heights
  everywhere; exact WGS 84 normal gravity as the default, with a RocketPy-compatible
  `vertical_taylor` option; Coriolis on by default.
- M1.1: reference values come from published formulas evaluated with mpmath (added to the oracle
  environment) when no table is published; `serde_json` parses floats exactly (`float_roundtrip`).
- M1.1: RocketPy conventions are pinned by running RocketPy itself (a real `Flight`'s initial
  quaternion), not by re-typing its formulas; `criterion` (no default features) benches hot paths,
  with numbers in `docs/perf.md`.

## Known issues and risks

- ThrustCurve curve licenses are mixed (Loft found only 45 of 108 marked PD), so bundle only the
  clean ones.
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
  has been simulated through either oracle yet; M1.3 and M2.x write those scripts.
