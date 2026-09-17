# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M0.2 Reference library
- **Run:** the first autopilot run; M0.1 has shipped
- **Last updated:** 2026-09-17 (M0.1 merged)

## Handoff (overwrite each session)

M0.1 shipped: the cargo workspace (17 skeleton crates under `crates/`, `xtask/` at the root),
toolchain pinned to 1.98.1, dual licenses, `deny.toml`, and CI (fmt, clippy, doc, wasm-check and
deny on Linux; tests on Linux, macOS and Windows). ADR-001 records the layout and license policy.

Start M0.2 by adding a `refs` command to `xtask` (`fetch|verify|doctor`) driven by a new
`validation/refs.lock.toml`. Notes for whoever picks it up:

- New dependencies need a line of justification in the PR and must pass `cargo deny check`
  (permissive licenses only; see `deny.toml`).
- A pure-core crate is marked `[package.metadata.hpr] wasm = true`; `xtask` itself is not pure and
  may do I/O. The test `the_workspace_pure_core_matches_the_architecture` pins the core list.
  `clippy.toml` bans `std::fs`, `std::net`, clocks and threads; `xtask`, `hpr-cli`, `hpr-net` and
  `hpr-validate` allow that at the crate root. The refs tooling belongs in `xtask`.
- Locally, `cargo-deny` 0.20.2 came from Homebrew; CI installs a prebuilt binary.
- The Bash guard hook rejects commit messages and PR bodies containing certain tool names, even
  in innocent phrases; word them plainly.
- `refs/` already holds `fusionspace-loft` and `loft-fixtures` (cloned by `scripts/preflight.sh`).

## Done log (newest first, keep about 15)

- 2026-09-17: M0.1 Workspace, CI, licenses (PR #2): workspace skeleton, xtask wasm-check,
  cargo-deny policy, CI on three OSes, ADR-001.
- 2026-09-16: Kickoff kit written. Scope, architecture, roadmap and validation inventory are
  set.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- (not blocking) Crate names on crates.io (`hpr`, `hpr-sim`, `hpr-core`...) are not reserved, and
  every crate has `publish = false`. Decide whether and when to reserve or publish them.
- (not blocking) `main` has no branch protection. Requiring the CI checks before merge would back
  up the guard hook; repo settings are off limits for the autopilot.

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-001: `MIT OR Apache-2.0` confirmed; permissive-only dependency licenses (weak copyleft
  rejected too); pure core marked by `[package.metadata.hpr] wasm = true`.
- `xtask` lives at the repo root (`xtask/`), not under `crates/` (ADR-001).
- MIT copyright line reads "Neer Patel and the hpr-sim contributors".
- Crates start at version 0.1.0 with `publish = false`.
- The CLI binary is named `hpr` and has rustdoc disabled to avoid colliding with the `hpr` facade.

## Known issues and risks

- ThrustCurve curve licenses are mixed (Loft found only 45 of 108 marked PD), so bundle only the
  clean ones.
- The RASAero `.CDX1` format has no public spec, so the importer relies on samples.
- ERA5 `.nc` files may be netCDF4 (HDF5), which affects the pure-Rust reader choice.
- A new RustSec vulnerability notice (anywhere in the graph) or unmaintained notice (direct
  dependencies) can turn CI red without a code change. Fix by upgrading or replacing the crate,
  or by an `ignore` entry with a reason.
