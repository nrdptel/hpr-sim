# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.4 Design model and mass properties
- **Run:** the first autopilot run; M0.1–M0.3 and M1.1–M1.3 have shipped
- **Last updated:** 2026-09-17 (M1.3 merged)

## Handoff (overwrite each session)

M1.3 built `hpr-motor` (ADR-005). Start M1.4 from these notes:

- **M1.4 itself:** Loft lessons L44–L50 and L91 name the tests. Build `validation/designs/` from
  RocketPy's examples (inputs in `validation/oracles/rocketpy/attitude.py`); RocketPy's Calisto
  uses its own M1670 data file, which hpr doesn't bundle.
- **Motor frame:** `SolidMotor::state(t)` gives thrust, `ṁ`, and the propellant and whole-motor
  `MassElement`s (mass, CG in metres forward of the nozzle exit, axial and transverse inertia
  about their own centre). A motor mount places the nozzle exit in the body frame; retainers join
  through `with_added_dry_mass` or as design parts. `CatalogMotor` gives diameter and length for
  fit checks.
- **Catalog motors are crude:** `from_envelope` centres the dry mass and propellant at `L/2`, so
  their CG doesn't move. Designs with real motor data should use `SolidMotor::new` with grains.
- **What M1.6 inherits:** burnout (`burnout_time_s`) is a thrust discontinuity and an event. The
  ambient-pressure term (`thrust_at_pressure_n`) needs a nozzle, which COTS data never gives.
  Delays come from `CatalogMotor::delays`.
- **For M2.1:** RocketPy prepends `(0, 0)` to `.eng` curves (an explicit one makes its impulse
  NaN), and `GenericMotor.load_from_eng` uses the diameter as the radius. `SolidMotor` takes its
  propellant mass from the grains, not the file header.
- **For M5:** fetch other ThrustCurve curves with `download.json` (POST batches of 250 work; the
  `license` key is missing when blank). Never bundle non-PD curves. `bundle.py` rebuilds the
  bundle; `survey.py` and `analyze_stats.js` are the checks.
- **Process notes:**
  - `cargo test -p xtask` checks STATUS against ROADMAP, notices rows against lock titles, lesson
    tests once a milestone is checked off, and that lock URLs use https.
  - Data that must keep its bytes needs `-text` in `.gitattributes` (the repo forces LF).
  - archive.org rate-limits (429) when several agents fetch at once; its `id_` captures are
    stable pins.
  - In zsh, quote curl's `'=https'`. The Bash guard hook rejects some tool names even in innocent
    phrases. Scanned PDFs (the 1976 standard, SP-8039) need their page images read. On snapshot
    drift, run `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: M1.3 Solid motors (PR #8): thrust curves with NFPA 1125 statistics matching
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

- ADR-001: `MIT OR Apache-2.0` confirmed; permissive-only dependency licenses (weak copyleft
  rejected too); pure core marked by `[package.metadata.hpr] wasm = true`.
- `xtask` lives at the repo root (`xtask/`), not under `crates/` (ADR-001).
- MIT copyright line reads "Neer Patel and the hpr-sim contributors".
- Crates start at version 0.1.0 with `publish = false`.
- The CLI binary is named `hpr` and has rustdoc disabled to avoid colliding with the `hpr` facade.
- ADR-002: the reference library pins git by commit, files by sha256, and live APIs by dated
  capture; it shells out to git, curl and uv instead of adding HTTP crates.
- orhelper comes from `openrocket/orhelper` at a pinned commit (PyPI's predates OR 24.12); Knacke's
  manual from archive.org's DTIC mirror; `openjdk@21` from Homebrew on the dev Mac.
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
- ADR-005: NFPA 1125 statistics as ThrustCurve's code computes them; constant exhaust velocity;
  column or BATES grains (bores required); file models keep file units for bit-exact round
  trips; bundle only PD curves within 1% of ThrustCurve's values (32), with their metadata.
- M1.3: `roxmltree` for `.rse`; NAR's motor-code page pinned from its 2014 archive capture;
  ThrustCurve's ISC `analyze.js` is run unchanged as an oracle under Node (fixture committed).

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
