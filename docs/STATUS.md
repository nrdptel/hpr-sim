# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.3 Solid motors
- **Run:** the first autopilot run; M0.1, M0.2, M0.3, M1.1 and M1.2 have shipped
- **Last updated:** 2026-09-17 (M1.2 merged)

## Handoff (overwrite each session)

M1.2 built `hpr-atmos` (ADR-004). Start M1.3 from these notes:

- **M1.3 itself:**
  - The `.rse` spec and the ThrustCurve and motor-finder snapshots are already pinned.
  - Follow `validation/oracles/rocketpy/*.py` for the SolidMotor oracle script.
  - Loft lessons L36–L43 name the tests.
- **Heights for the atmosphere and wind:** every model takes geometric height above mean sea
  level. The flight engine (M1.6) must subtract the geoid undulation from ellipsoidal height
  first.
- **What M1.6 inherits:**
  - `Ussa76` (with `anchored` for field conditions), `SoundingProfile` and `WindModel`.
  - `GustField`, a precomputed Dryden realization. M1.6 picks its path coordinate (distance
    through the air, or altitude) and how gusts start on the rail.
  - Use the moist density and speed of sound for dynamic pressure and Mach.
- **For M2.1:**
  - RocketPy interpolates pressure linearly in height (up to 1.15% off between 700 and
    500 hPa), wind as u/v components (`WindInterpolation::Components`), and ignores humidity.
  - Its geopotential helper defaults to a radius ten times the Earth's (`rocketpy/tools.py:972`).
  - See `docs/physics/atmosphere.md`.
- **For M5.2:** convert forecast and sounding geopotential heights with
  `geometric_from_wmo_geopotential_m`, and clamp radiosonde humidity into `[0, 1]`.
- **For M6.1:** `hpr_core::random::SeededRng` (xoshiro256++, polar normals) is frozen. Changing
  its algorithm, seeding or draw order needs an ADR.
- **Process notes:**
  - `cargo test -p xtask` checks that STATUS names the first open ROADMAP milestone, and that each
    checked-off milestone's lesson tests exist. For M1.3 those are L36 to L43.
  - A notices row must contain its lock entry's title word for word.
  - In zsh, quote curl's `'=https'` (a bare `=word` expands to a command path).
  - The Bash guard hook rejects some tool names even in innocent phrases, so word commit messages
    and PR bodies plainly.
  - If a refs fetch reports snapshot drift, run `cargo xtask refs fetch --adopt-snapshots`.
  - Scanned PDFs (the 1976 standard) have unusable text layers; read the page images.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.2 Atmosphere and wind (PR #6): USSA76 matching its tables at 32 altitudes,
  offsets and field anchoring, moist air, sounding profiles, four wind models, exact Dryden
  turbulence with a PSD test, `hpr_core::random`, ADR-004, and five newly pinned sources.
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
- ADR-004: atmosphere and wind take height above sea level; ISA offsets apply at equal
  geopotential height (not the aviation pressure-altitude convention); soundings interpolate in
  geopotential height with hydrostatic pressure; wind defaults to speed-and-direction
  interpolation; Dryden uses MIL-F-8785C's lengths with exact discretization; an in-house frozen
  xoshiro256++ generator.
- M1.2: MIL-F-8785C is pinned from Abbott Aerospace's copy (everyspec blocks scripts and stamps
  each copy) and WMO-No. 8 from a national weather service's mirror; MIL-HDBK-1797 is cited for
  its differences only and not pinned (no stable public copy).

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors: MIL-F-8785C from Abbott Aerospace, and
  WMO-No. 8 from Mongolia's weather service. If either moves, the lock file names an archive.org
  fallback for MIL-F-8785C; WMO's own library needs a browser.
- Dryden turbulence is an aircraft model. How it applies to a climbing rocket (path coordinate,
  rail start, near apogee) is unvalidated until M1.6 and M2.3.
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
