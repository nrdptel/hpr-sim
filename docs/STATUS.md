# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.1 Core math, frames, Earth
- **Run:** the first autopilot run; M0.1, M0.2 and M0.3 have shipped
- **Last updated:** 2026-09-17 (M0.3 merged)

## Handoff (overwrite each session)

M0.3 wrote `docs/research/loft-lessons.md`: 97 lessons (`L1` to `L97`) and 16 process mistakes
(`P1` to `P16`). Each lesson names its milestone and the tests to write, and each affected
`ROADMAP.md` entry now has a `Loft lessons:` line. Start M1.1 from these notes:

- **`cargo test -p xtask` checks the planning docs** (`xtask/src/docs.rs`):
  - STATUS must name the first open ROADMAP milestone.
  - STATUS stays within 150 lines and each `docs/research/` note within 200.
  - When a milestone is checked off, every lesson it owns must have its named test function in
    that crate. For M1.1 that is L1: `hpr_core::gravity::tests::somigliana_matches_published_values`.
    A rename that keeps the assertion is fine; moving a lesson later or dropping one needs an ADR.
- **Gravity source:** WGS84 Somigliana needs its primary source (NIMA TR8350.2) pinned in
  `validation/refs.lock.toml` before it is cited; it isn't in the lock yet.
- **Clean room:** Loft material that came from OpenRocket's Java source is listed in the lessons
  doc, with a blanket rule. Never port it. The guard hook now blocks fetching OpenRocket's source
  from Bash and WebFetch.
- **Loft's numbers are leads, not references.** Review confirmed against NACA TN 4197 eq. 18 that
  Loft's flutter speed is √2 too high (L32); M1.10 pins the correct denominator.
- **Reference library:**
  - `refs/rocketpy` is a shallow clone of `v1.13.0`; `rocketpy` has no `__version__`, so use
    `importlib.metadata.version`.
  - ThrustCurve `search.json` includes hybrids (152 of 1156); filter to solids.
  - If a fetch reports snapshot drift, run `cargo xtask refs fetch --adopt-snapshots` and commit
    the lock change.
- **Wording:** the Bash guard hook rejects commit messages and PR bodies containing certain tool
  names, even in innocent phrases; word them plainly.

## Done log (newest first, keep about 15)

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
