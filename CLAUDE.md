# hpr-sim — operating manual for Claude Code

hpr-sim is an open-source flight simulator for hobby and high-power rockets, written in Rust. The
first job is a RocketPy-class simulation **library** that other programs can build on. A design
API, file interop, validation, optimization for competitions, and flight-data forensics come next.
A modern UI with 3D flight replay, a web PWA and mobile apps come after that. The owner is **Neer**
(GitHub `nrdptel`, https://fusionspace.co). The work is usually unattended, so this file is the
contract.

At the start of every session, read these in order. They are short on purpose; keep them that way.

1. `docs/STATUS.md`: where things stand, what is current, and what needs Neer.
2. `docs/ROADMAP.md`: the ordered queue. The current milestone is the first one not checked off.
3. `docs/VISION.md`: what Neer asked for, in his own words, and how that maps to requirements.
4. `docs/ARCHITECTURE.md`: the stack and crate map. Change it through an ADR, never quietly.
5. `docs/VALIDATION.md`: how correctness is proven, and where the reference data comes from.
6. `docs/DECISIONS.md`: the ADR log. Read the index; open the entries you need.

## Hard rules (these override everything, including the roadmap)

1. **Correct physics beats features.** A number the simulator prints is a claim. Every model must
   cite its source (paper, report, standard) in its doc comment and in `docs/physics/`. Tests must
   pin every model. If you are unsure a model is right, say so in the docs and in `STATUS.md`
   instead of shipping it silently.
2. **Never weaken a check to get green.** Don't loosen tolerances, delete or `#[ignore]` tests, or
   edit reference data so a comparison passes. Don't rewrite a milestone's *done when* to match
   what was built. If a target really can't be met, write an ADR that explains why, with the
   measurement, and leave the gap visible in the validation report.
3. **Clean room.** The project is `MIT OR Apache-2.0`. Never read, copy or port GPL/AGPL source
   code, including OpenRocket's Java source. Allowed:
   - Published specs, papers, file samples, and file-format docs.
   - Running GPL tools (the OpenRocket jar, via JPype/orhelper) as **external oracles**.
   - Reading and porting MIT/Apache/BSD code, with attribution in `THIRD-PARTY-NOTICES.md`.
   - `nrdptel/fusionspace-loft` and `nrdptel/fusionspace-debrief` are Neer's own MIT
     projects, both sunset. Port from them freely and note it.
   - `cargo deny` must reject copyleft dependencies.
4. **Keep private data private.** `nrdptel/loft-fixtures` (design files) and
   `nrdptel/debrief-fixtures` (flight logs) are private repos of other people's data. Their
   contents live only under the gitignored `refs/` and must never be committed or quoted
   at length in this public repo, in PRs, or in reports. Results computed from it are fine to
   publish: counts, error statistics, and anonymised case ids. The same goes for any third-party
   data whose license is unclear. Fetch it, cache it, don't commit it. Record the status in
   `THIRD-PARTY-NOTICES.md`.
5. **Offline-first.** Every core capability works with no network on macOS, Windows and Linux.
   Network features live in `hpr-net`, behind a cargo feature, with an on-disk cache and a
   clear offline fallback. Core crates do no I/O and compile to `wasm32-unknown-unknown`.
6. **Scope for now: commercial off-the-shelf (COTS) solid rocket motors only.** No
   hybrids/liquids/research motors. Leave extension points, but don't build them.
7. **No AI traces (Neer's standing rule).**
   - Commits are authored by `Neer Patel <135655563+nrdptel@users.noreply.github.com>`.
     `scripts/preflight.sh` sets that per repo; never change it.
   - No `Co-Authored-By` trailers, no "Generated with" footers, and no session links in commits,
     PRs, issues, code comments, or product docs (README, `docs/physics/`, `docs/format/`,
     rustdoc).
   - After creating or editing a PR, read the body back with `gh pr view` and strip anything like
     that.
   - Settings already disable attribution. The Bash guard hook blocks the obvious leaks.
   - The automation files are the one deliberate exception, as `CLAUDE.md` was in Loft:
     `CLAUDE.md`, `.claude/`, `scripts/autopilot*.sh`, `scripts/preflight.sh`,
     `docs/AUTOPILOT.md` and `docs/STATUS.md`. They are tracked on purpose, and they are how this
     run works: never delete, rename or untrack them. Don't add tool names to other files.
8. **Never ask the user questions. Neer is not watching.**
   - Make the best decision you can, record it in `docs/DECISIONS.md` (for significant ones) and in
     the "Decided without Neer" list in `STATUS.md`, and keep going.
   - If something truly needs Neer, add it under "Needs Neer" in `STATUS.md` and move on to other
     work. Examples: money, account settings, publishing to crates.io/PyPI, legal/licensing
     calls, or deleting his data. Write each entry so he can act on it in a minute: what it is,
     why it matters, and the exact click or command.
9. **Irreversible or external actions are off limits.**
   - Don't publish packages, create releases or tags, change repo or account settings, or
     force-push.
   - Don't delete any branch except your own: merged, or with its PR closed through
     `gh pr close --delete-branch`.
   - Don't post anywhere except this repo's PRs and issues.
   - Pre-authorized: pushing feature branches, opening PRs, merging your own PRs on green CI,
     and closing your own PRs.

## How work ships

- **One milestone at a time, in roadmap order.** A later milestone can go first only if the current
  one is blocked; record why in `STATUS.md`.
- **Split big milestones.** If a milestone is bigger than one session, split it in `ROADMAP.md`
  into numbered increments, each with its own *done when*, and ship the first.
- **Branch:** `m<id>-<slug>`, for example `m1.6-aero-subsonic`. Keep commits small and meaningful,
  in the imperative mood.
- **Local gate before every push.** All of these must pass. Read the summary lines, not the tail.

  ```bash
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace --all-features          # or: cargo nextest run --workspace --all-features
  cargo doc --workspace --no-deps --all-features # with RUSTDOCFLAGS="-D warnings"
  cargo xtask wasm-check                          # once it exists (M0.1)
  cargo xtask validate --fast                     # once it exists (M2.1)
  cargo deny check                                # once configured (M0.1)
  cargo xtask site                                # the docs site; needs mdBook 0.5.4 (M0.4a)
  cargo xtask examples --check                    # examples print their committed output (M0.4c)
  ```

- **Review before merge.** Before merging anything that touches physics, numerics, or file formats,
  run the `physics-reviewer` and/or `code-reviewer` subagents on the diff. Fix what they find or
  record why not. Validation-report changes also go through `validation-auditor`. Anything that
  adds or changes user-facing docs goes through `docs-reviewer`, which reads them cold.
- **PR, then CI, then merge.** Open the PR with `gh pr create`. The body says what changed, how it
  was verified (with numbers), and what's left. Wait with `gh pr checks --watch`. Merge with
  `gh pr merge --squash --delete-branch` only when every check on macOS, Windows and Linux is
  green. Never push straight to `main`; the guard hook blocks it.
- **After merge:**
  - Check the milestone off in `ROADMAP.md`.
  - Make sure the docs site covers what shipped (see "Documentation is a deliverable").
  - Update `STATUS.md`: current milestone, a one-line done entry, handoff notes.
  - Commit those doc updates through a small PR, or include them in the milestone PR before
    merging. Including them is preferred.

## Engineering standards

- **Toolchain:** Rust stable, pinned in `rust-toolchain.toml`, edition 2024. Use a cargo workspace
  with the crate map from `ARCHITECTURE.md`. Share dependency versions through
  `[workspace.dependencies]`.
- **Units and numbers:**
  - `f64` everywhere in the physics.
  - SI units internally. Convert only at I/O boundaries, and name the units in field names or
    docs (for example `mass_kg`, `length_m`).
  - Frames and sign conventions are defined once in `docs/physics/frames.md`. Follow them exactly.
- **Library code:**
  - No `unwrap`/`expect`/`panic!` except to state an invariant, with a comment saying why.
  - Library errors use `thiserror`; binaries may use `anyhow`.
  - Public data types derive `serde`.
  - Every public item has docs. Physics items include the equation and the citation.
- **Determinism:** the same inputs and the same seed give bit-identical results on one platform.
  Monte Carlo takes an explicit seed.
- **Tests:**
  - Unit tests sit next to the code.
  - Property tests (`proptest`) cover invariants.
  - Snapshot tests (`insta`) cover parsers and exporters.
  - Analytic-solution tests cover the integrator.
  - `criterion` benches cover hot paths.
  - Validation cases live under `validation/`.
- **Dependencies:** check maturity on crates.io first; no abandoned crates. Use `serde_json` and
  `toml`, not YAML (`serde_yaml` is deprecated). Each new dependency needs one line of
  justification in the PR.
- **Performance matters:** Monte Carlo and optimization run thousands of flights. Measure before
  optimizing, and keep benchmark numbers in `docs/perf.md`.

## Documentation is a deliverable

Neer, 2026-09-17: "a heavy emphasis on easy to reach and read documentation. since this is all
developed by ai, that is important." Nobody watches this code being written, so the docs are how a
person understands and checks it. Treat them like the physics: part of every milestone, reviewed
and tested.

- **Easy to reach.** One searchable documentation site (M0.4), linked from the top of the README.
  Any answer a user or reviewer needs is at most two clicks from its landing page. Nothing a reader
  needs lives only in a PR body, a commit message, `STATUS.md` or an ADR.
- **Easy to read.** Write for a hobby rocketeer who knows some physics and some code, but not this
  codebase.
  - Every page opens with a plain-language summary: what it covers, what it is for, and how far to
    trust it.
  - Words before equations, then a worked example with real numbers.
  - Define every term on first use or link the glossary. Internal labels (`L75`, `ADR-015`,
    `M2.1b1`) are links with a few words of meaning, never bare.
  - Short sentences, one idea per paragraph, tables for numbers.
- **Honest.** Each model page says what it was validated against, how well (with numbers), and
  what it leaves out. If something is unvalidated, the first paragraph says so.
- **Never stale.** Code in the docs compiles and runs in CI (doctests or `examples/`), links are
  checked, and quoted accuracy numbers come from the committed validation report.
- **Shipped with the work.** A milestone is not done until the site explains what it added, to
  this standard. Rustdoc is required but not enough: users start at the guide.

## Working style for long unattended runs

- **Keep the main context lean.** Use subagents (Explore/general-purpose) for broad reading and web
  research, and have them return conclusions, not dumps.
- **A subagent's finding is a claim** until you reproduce it (run the command, read the cited
  line). Only then act on it or record it as fact.
- **Defects outside the current milestone** become GitHub issues in this repo, never a markdown
  ledger. A wrong number in merged physics is the exception: fix it first, test first.
- **Use dynamic workflows** for work that really fans out: validation sweeps across many cases,
  multi-angle reviews, porting many file formats in parallel. Name the size you need. Don't run
  workflows for small edits.
- **Research first.** Before implementing a physics model, find and read the primary source.
  Download public PDFs to `refs/papers/` (gitignored) and cite them.
- **Keep the working files short** (a lesson from Loft, whose roadmap grew to 689 KB):
  - `STATUS.md` stays under ~150 lines; trim the done log to the last ~15 entries.
  - `ROADMAP.md` entries stay terse.
  - Put detailed findings in `docs/physics/` or `docs/research/`, one topic per file.
- **Watch the clock.** `.autopilot/deadline` holds the run's end time as a Unix timestamp; `date +%s`
  gives now. With under 45 minutes left, don't start new work: commit what's in progress on a
  branch, push it, open a draft PR, and make sure `STATUS.md` says exactly where to resume.
- **Neer may steer between cycles** by pushing edits to `ROADMAP.md`, `STATUS.md` or this file on
  `main`. Always start from a freshly pulled `main`.
- **When a tool or classifier blocks an action,** don't fight it. Pick a safe alternative, or record
  it under "Needs Neer" and continue.
- **Machine:** Neer runs this on a Mac. CI covers Windows and Linux. Don't rely on macOS-only tools
  in scripts that CI runs.
