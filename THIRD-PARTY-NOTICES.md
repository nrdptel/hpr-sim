# Third-party notices

hpr-sim is licensed under either of MIT or Apache-2.0, at your option. This file records every
outside source the project uses, its license, and how it is used. Update it in the same PR that
adds a source.

**Usage modes:**

- **bundled:** copied into this repository or into built artifacts. Needs a compatible license and
  its notice reproduced below.
- **dependency:** a Rust crate from crates.io. `cargo deny check` enforces the license allow list
  in `deny.toml`; copyleft licenses are rejected.
- **fetched:** downloaded into the gitignored `refs/` by tooling, and never committed.
- **run-only:** executed as an external oracle. Its source code is never read or ported.
- **ported:** code or methods ported from a permissively licensed source, with attribution here.

## Bundled

None yet.

## Rust dependencies

`Cargo.lock` lists the full dependency graph. Direct third-party dependencies:

| crate | license | used by | why |
|---|---|---|---|
| `serde_json` | MIT OR Apache-2.0 | `xtask` | reads `cargo metadata` output |

## Reference sources (planned; M0.2 pins versions and hashes in `validation/refs.lock.toml`)

| source | license | mode | notes |
|---|---|---|---|
| RocketPy | MIT | fetched; run-only oracle | example rockets, acceptance tests and RASAero Cd exports used as references |
| RocketPy flight data (`data/rockets/`) | MIT (repository); team permissions recorded in RocketPy's notebooks | fetched | small extracts may be committed later, with provenance |
| OpenRocket 24.12 jar | GPL-3.0 | run-only | never read or port its source |
| orhelper | GPL-2.0 | run-only | drives the OpenRocket jar through JPype |
| `openrocket/openrocket-database` (`.orc` parts) | Apache-2.0 | fetched; may be bundled with notices in M5.5 | |
| `openrocket/motor-database` | GPL-3.0 | run-only reference | not bundled |
| ThrustCurve.org motor data | per file: public domain, none, or unknown | fetched and cached | only curves with clear terms are bundled (M1.3); attribution given |
| motor.fusionspace.co API | free to use, attribution appreciated | fetched (M5.4) | |
| `nrdptel/fusionspace-loft` | MIT (the project owner's own) | fetched; ported with a note | |
| `nrdptel/loft-fixtures` | private; third-party design files | fetched into `refs/` only | never committed; only derived statistics are published |
| Papers and reports (Barrowman, USSA76, NASA and NACA reports) | mostly US government works | fetched to `refs/papers/` | cited, not copied |
| Niskanen 2009 thesis; OpenRocket technical documentation | CC BY-NC-ND; CC BY-SA | fetched | read for methods only; no text copied |
| Open-Meteo | data CC BY 4.0 | fetched and cached (M5.2) | attribution required |
| WMM2025 | public domain | may be bundled (M5.3) | |
