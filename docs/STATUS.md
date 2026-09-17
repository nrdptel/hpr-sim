# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M0.1 Workspace, CI, licenses
- **Run:** the first autopilot run; the kickoff kit is in place and no code exists yet
- **Last updated:** 2026-09-16 (kickoff kit)

## Handoff (overwrite each session)

The repo holds only the kickoff kit: `CLAUDE.md`, `docs/*`, `.claude/*` and `scripts/*`. Start
M0.1. `scripts/preflight.sh` has already set the git identity, created the public GitHub repo and
cloned the private reference repos into `refs/`, if Neer ran it successfully; check with
`git remote -v`, `ls .claude refs/`.

## Done log (newest first, keep about 15)

- 2026-09-16: Kickoff kit written. Scope, architecture, roadmap and validation inventory are
  set.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- (none yet)

## Decided without Neer (one line each; significant ones get an ADR)

- (none yet)

## Known issues and risks

- ThrustCurve curve licenses are mixed (Loft found only 45 of 108 marked PD), so bundle only the
  clean ones.
- The RASAero `.CDX1` format has no public spec, so the importer relies on samples.
- ERA5 `.nc` files may be netCDF4 (HDF5), which affects the pure-Rust reader choice.
