# Decisions (ADR log)

Each entry is short: context, decision, consequences, date. Number them sequentially and never
renumber. Supersede an entry by adding a new one that points back to it.

| id | title | status |
|---|---|---|
| ADR-000 | Kickoff decisions | accepted |
| ADR-001 | License and workspace layout | accepted |

---

## ADR-000: Kickoff decisions (2026-09-16)

**Context.** Neer asked for a Rust hobby/HPR rocket simulator: library first, validation-heavy,
offline-first, open source. It replaces his Loft project (TypeScript/Next.js), which he is shutting
down.

**Decision.**

- **Language and layout:** Rust, in a workspace of focused crates (see `ARCHITECTURE.md`).
- **Order of work:** physics core and validation first; interop, bindings and analysis next; UI
  last.
- **License:** `MIT OR Apache-2.0`. MIT matches the other Fusion Space projects; Apache-2.0 adds
  a patent grant and is the Rust norm. ADR-001 confirms this in M0.1.
- **Clean room:** no GPL/AGPL source is read or ported. GPL tools may be run as external oracles.
- **No AI traces** in commits, PRs, code or docs. Commits are authored as Neer's GitHub noreply
  identity.
- **Private data:** `loft-fixtures` content is never committed; only derived statistics are
  published.
- **Name:** `hpr-sim` for now.
- **Scope:** COTS solid motors only.
- **Hosting:** a public GitHub repo (`nrdptel/hpr-sim`). Work ships through PRs; CI covers macOS,
  Windows and Linux; merging on green is pre-authorized.
- **How work runs:** unattended cycles, one milestone each, handing off through `STATUS.md`
  (see `docs/AUTOPILOT.md`).

**Consequences.**

- Work is slower at first, but every number is defensible.
- A few formats (`.CDX1`, parts of `.ork` 1.12) must be learned from samples rather than from
  source code.
- Publishing to crates.io or PyPI waits for Neer.

---

## ADR-001: License and workspace layout (2026-09-17)

**Context.** M0.1 turns ADR-000's provisional license and the crate map in `ARCHITECTURE.md` into
files and checks: license texts, a dependency-license policy that backs the clean-room rule, and a
workspace whose pure core can be verified.

**Decision.**

- **License:** `MIT OR Apache-2.0`, confirmed. MIT matches the other Fusion Space projects,
  Apache-2.0 adds an explicit patent grant, and the pair is the Rust norm. `LICENSE-MIT`
  (copyright "Neer Patel and the hpr-sim contributors") and `LICENSE-APACHE` (the canonical
  apache.org text, sha256 `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`).
  Every crate inherits `license` from `[workspace.package]`; the README carries the usual
  dual-license contribution clause.
- **Dependency licenses (`deny.toml`):** only permissive licenses are allowed: 0BSD, Apache-2.0
  (also WITH LLVM-exception), BSD-2-Clause, BSD-3-Clause, BSL-1.0, CC0-1.0, ISC, MIT, MIT-0,
  Unicode-3.0, Unlicense and Zlib. Everything else is rejected, weak copyleft (LGPL, MPL, EPL)
  included; widening the list needs a new ADR. Security, unsound and unmaintained advisories and
  yanked crates fail the check, and crates.io is the only allowed source. Checked on 2026-09-17: a
  throwaway workspace with GPL-3.0, AGPL-3.0, LGPL-2.1 and MPL-2.0 crates fails
  `cargo deny check licenses` (exit 4), and its MIT crate passes.
- **Layout:** a virtual workspace (resolver 3, edition 2024) with `rust-version` equal to the
  toolchain pinned in `rust-toolchain.toml` (1.98.1). The 17 crates of the crate map live under
  `crates/`, and their internal dependency edges are declared now, so the layering holds from the
  start. `xtask` sits at the root (`xtask/`), where the cargo-xtask convention puts it, because it
  is tooling rather than product. Crates start at 0.1.0 with `publish = false` until Neer decides
  on publishing.
- **Pure core:** a crate joins it with `[package.metadata.hpr] wasm = true`. Members: `hpr-core`,
  `hpr-atmos`, `hpr-motor`, `hpr-design`, `hpr-aero`, `hpr-sim`, `hpr-analysis`,
  `hpr-flightdata`, `hpr-format`, `hpr-io`, the `hpr` facade with default features (`net` is an
  optional feature) and `hpr-wasm`. Outside it: `hpr-net` (network and cache I/O),
  `hpr-validate` (reads case files), `hpr-cli`, `hpr-py`, `hpr-ffi` and `xtask`.
  `cargo xtask wasm-check` fails if a pure crate depends on a workspace crate outside the core
  (optional dependencies count when default features enable them; target-specific ones always
  count), then runs `cargo check --target wasm32-unknown-unknown` on the core. A unit test pins
  the membership list, so changing it is visible in review.
- **Lints:** `missing_docs`, `missing_debug_implementations` and `unsafe_code = "deny"` for rustc;
  `unwrap_used`, `expect_used`, `panic`, `print_stdout`, `print_stderr`, `dbg_macro`, `todo` and
  `unimplemented` for clippy. CI turns warnings into errors; `clippy.toml` relaxes the panic and
  print lints in tests, and binaries allow printing at the crate root.
- **Line endings:** `.gitattributes` checks text out with LF everywhere, so formatting, snapshot
  tests and reference-data hashes behave the same on Windows.
- **CI:** one workflow on every PR and on pushes to `main`. fmt, clippy, doc, wasm-check and deny
  run on Linux; tests run on Linux, macOS and Windows. Swatinem/rust-cache caches builds;
  cargo-deny is installed as a prebuilt binary.

**Consequences.**

- The CLI binary is named `hpr`, like the facade library, so its rustdoc is off (`doc = false`) to
  avoid an output collision in `target/doc`.
- A binding crate that needs `unsafe` must allow it item by item, with a reason.
- A dependency under MPL or LGPL (some geo and GUI crates) needs an ADR before it can be added.
