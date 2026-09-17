# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M0.3 Lessons from Loft
- **Run:** the first autopilot run; M0.1 and M0.2 have shipped
- **Last updated:** 2026-09-17 (M0.2 merged)

## Handoff (overwrite each session)

M0.2 shipped the reference library. `cargo xtask refs fetch|verify|doctor` is driven by
`validation/refs.lock.toml` (ADR-002). On this Mac, `refs/` is fully fetched and verifies (24 ok),
and `doctor` reports both oracles runnable: RocketPy 1.13.0 in `refs/venv`, and OpenRocket 24.12
on Homebrew's `openjdk@21`.

Start M0.3 by reading `refs/fusionspace-loft` (pinned at `64f51ef1b3`; MIT, the owner's own, so
reading and porting are fine). Write `docs/research/loft-lessons.md` (at most 200 lines). Notes:

- Every quirk or weakness listed must map to a roadmap milestone or a named test to write. Add
  the test names to the relevant milestone's bullets in `ROADMAP.md` if they aren't obvious.
- Loft's fixture pin and manifest match ours (`loft-fixtures` at `37251476e5`, 38 files). Never
  quote those files; counts and anonymised ids only.
- Loft's RocketPy use was unpinned (`pip install rocketpy`, 1.12.1 in the browser path), so don't
  reuse its RocketPy numbers as references without re-running them against 1.13.0.
- `refs/rocketpy` is a shallow clone of `v1.13.0`. `rocketpy` has no `__version__`; use
  `importlib.metadata.version`.
- ThrustCurve `search.json` includes hybrids (152 of 1156). Filter to solids (COTS scope).
- Snapshots of live APIs are pinned per capture. If a fetch elsewhere reports drift, run
  `cargo xtask refs fetch --adopt-snapshots` and commit the lock change.
- The Bash guard hook rejects commit messages and PR bodies containing certain tool names, even
  in innocent phrases; word them plainly.

## Done log (newest first, keep about 15)

- 2026-09-17: M0.2 Reference library (PR #3): `xtask refs fetch|verify|doctor`, 23 pinned
  references plus the uv oracle environment, notices rows per source, ADR-002.
- 2026-09-17: M0.1 Workspace, CI, licenses (PR #2): workspace skeleton, xtask wasm-check,
  cargo-deny policy, CI on three OSes, ADR-001.
- 2026-09-16: Kickoff kit written. Scope, architecture, roadmap and validation inventory are
  set.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

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
