# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M2.1c Predicted mode, CI and regeneration
- **Order:** M2.1c, then M1.8; M0.4 waits only on M0.4d's deploy, blocked on Pages (Needs Neer)
- **Run:** the first autopilot run; M0.1-M0.4c, M0.4e, M1.1-M1.7, M2.1a and M2.1b have shipped,
  and M0.4d all but its deploy
- **Last updated:** 2026-09-18 (M2.1b2 shipped; M2.1c not started)

## Handoff (overwrite each session)

M2.1b2 (ADR-021): six whole-flight cases. Five pass (64 metrics within 3%), Prometheus is a known
gap (Mach 1.014), and eleven metrics are argued as not scored, nine of them the path in wind:
hpr turns into the wind far more than RocketPy (issue #50, open). The designs fly RocketPy's
thrust as measured (`reference_pressure_pa: null`); the first flight went from 874.0 to 779.0 m.

- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest,
  labels as links to their rows, none in headings, Unicode equations; new pages in `SUMMARY.md`; a
  new library needs a row in `docs/api.md` and a guide link in its `//!`. *Accuracy*'s numbers
  must be in a file the same item links (the report, a case file); its results tables must hold
  every row of the report, cell for cell.
- **Checking a milestone off** in `ROADMAP.md` fails `cargo xtask site` until its row in
  `docs/decisions-and-roadmap.md` says `done`; a new milestone needs a row.
- **When Pages is on:** `gh workflow run CI --ref main`; once `deploy` passes, drop the README's
  "goes live once" sentence, check M0.4d and M0.4 off, and set both rows to `done`.

Next, M2.1c: predicted mode, a CI job for `cargo xtask validate`, and manual regeneration.

- **Predicted mode needs its own reference.** The committed one is same-drag (`C_D0` 0.5).
  Comparing hpr's own drag against it measures hpr's drag against an arbitrary constant. The
  like-for-like reference is RocketPy flying its examples' own drag curves, which live in `refs/`
  and carry their own terms (ADR-009). Results computed from them may be published; the curves
  may not. Decide in an ADR; otherwise report hpr's predicted apogee beside the same-drag one and
  say what it is.
- **Issue #50** (the path in wind) is the largest open physics question; bisect it before M2.2.
- **Prometheus stays a gap** until M1.8; `a_known_gap_is_checked_not_trusted` fails the run once
  hpr flies it. Remove `known_gap` then; its tolerances are already argued.
- **CI:** `cargo test` already runs every case against the committed report, pinned to six
  decimals on three OSes (whole flights reproduced there); a `validate` job makes it explicit.
- **Regeneration** needs `refs/rocketpy` and `refs/venv` (`cargo xtask refs fetch`). A
  `workflow_dispatch` job regenerates the fixtures and uploads the diff; it never commits.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock.
  Oracles run from the repo root with `refs/venv/bin/python`. `cargo xtask designs` and
  `cargo xtask examples` rewrite designs and example outputs; pages quoting them must follow.

## Done log (newest first, keep about 15)

- 2026-09-18: M2.1b2 The whole-flight cases (PR #49, ADR-021): five pass (64 metrics within
  3%), Prometheus a checked gap, the path in wind not scored (#50); the L75 test; Bella Lui added.
- 2026-09-18: M0.4e The reader test (PR #47, ADR-020): cold readers answered 5, then 9 of 10
  questions (the tenth then fixed); every flagged term fixed; four examples in CI; labels link rows
  held to the roadmap, in the API reference too (#44).
- 2026-09-18: M0.4d Publish, all but the deploy (PR #43, ADR-019): the rustdoc of 16 crates
  under the site's `api/`, linked from *The API reference* and linking the guide, both ways
  checked; `site-url`; a `deploy` job that waits for Pages; the README links the site.
- 2026-09-18: M0.4c Getting started, and how a flight is simulated (PR #42, ADR-018): a
  first flight (Valetudo, 874.0 m apogee) and a drag what-if run in CI on three OSes against
  committed output; pages quote them, checked line for line; a flight diagram; `with_wind`.
- 2026-09-18: M0.4b Model pages, Accuracy, Glossary, Checking a claim (PR #40, ADR-017): *In
  short* on all 16 model pages and *Accuracy*'s numbers traced to sources, both checked.
- 2026-09-18: M0.4a The site and its link checks (PR #37, ADR-016): mdBook, links and labels.
- 2026-09-17: M2.1b1 The whole-flight oracle (and PR #34, its bounded step, #33): `flight.py`
  flies the examples pad to landing under a declared `C_D0`; Prometheus reaches Mach 1.014.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Turn on GitHub Pages** (1 minute; the only thing M0.4d waits for). Settings → Pages →
  Build and deployment → Source: **GitHub Actions**. Then `gh workflow run CI --ref main` (or
  merge anything) deploys the guide and the API reference to https://nrdptel.github.io/hpr-sim/,
  which the README already links. They ship mdBook's MPL-2.0 theme and rustdoc's OFL fonts, each
  with its licence (ADR-016, ADR-019). The autopilot may not change repo settings.
- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check`, `site` and three `test (...)` checks; block force
  pushes. Don't require approvals: the autopilot merges its own PRs as you, and authors can't
  self-approve.
- **Loft's flutter calculator overstates flutter speed by √2** (safety). fusionspace-loft
  `lib/sim/flutter.ts:287` uses 1.337·(λ+1)/2; NACA TN 4197 eq. 18 gives 2.674·(λ+1)/2 (39.3 over
  14.7 psi), so its "1.5 margin" is about 1.06. Fix it or post a notice before Loft shuts down.
- **ThrustCurve data** (yes or no). `hpr-motor` bundles ThrustCurve.org's numbers and names for 32
  motors, with attribution; the site states no terms for them (ADR-005). OK to treat them as facts,
  or ask John Coker? If not, the fallback keeps only the numbers the tests check.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`... are unreserved. Reserve them?
- **orhelper** (no action if fine): GPL-2.0, so M2.2 drives OpenRocket via JPype, never imports it.

## Decided without Neer (one line each; significant ones get an ADR)

- M0.4, M1.4, M1.5, M1.6, M1.7 and M2.1b were split into increments, done-when bullets unchanged.
- ADR-016 to ADR-018, the site: mdBook 0.5.4 over `docs/` (its MPL-2.0 theme files ship in the
  site); Unicode equations; our own link and label checks; web links counted, not fetched (#38);
  *In short* in a fixed form; *Accuracy*'s numbers checked against the files they link; the
  records stay files; examples run in CI against committed output, and quotes match line for line.
- ADR-020: the reader test reads the built site cold, twice; every milestone and lesson label links
  a row of plain words, which the site check holds to the roadmap; the rustdoc is checked too.
- ADR-019: the rustdoc is part of the site (`api/`), crates link the guide by its address; CI
  deploys from `main` after every check, and skips with a warning while Pages is off.
- ADR-001 to ADR-007 and M0.3 (details in `DECISIONS.md`): licence and layout; refs pinned by hash;
  body `+z` to the nose, WGS 84 gravity and Coriolis; atmosphere and wind by height above sea
  level; NFPA 1125 motor statistics, 32 curves; full inertia tensors, Crowell's secant ogive;
  origin at the nose tip, one `Component` with a `Part` enum; the doc guards `cargo test` runs.
- ADR-008 to ADR-011: body CP from the real volume with Galejs lift (`K` 1.1) and Diederich fins;
  Niskanen's drag as printed, 20 µm finish; own DOPRI5, discontinuities as stop times; nose-tip
  reference, `M ≥ 1` refused and stops a flight, rail `μ` 0.
- ADR-012: recovery devices live in `hpr-sim`; `C_D0` on Knacke's nominal area, mid-range; a
  point-mass descent, no added mass; devices add and can release one another.
- ADR-013: streamers take Filippone's three curves by default, appendix C's on request; tumble takes
  OpenRocket's §3.5 (−10 to +19% on its own drops, not the 3 to 14% claimed).
- ADR-014: a separation splits the stack at a stage boundary into point-mass bodies with their own
  stages' mass and devices; no ejection impulse; it must follow the last burnout (M1.9 stages).
- #11: `SolidMotor` refuses `c = I/m_p` outside 200–5,000 m/s: a units guard, not a propellant
  filter (it rejects none of the 1,708 surveyed motors, 236 to 3,031 m/s), and a behaviour change.
- M2.1b1: a same-drag case declares its own `C_D0(M)`, which both codes then fly, rather than
  committing or reading RocketPy's exports (their own terms, ADR-009; absent from CI).
- ADR-021: whole flights measured as RocketPy defines them (dry-mass centre, forward-button rail
  exit); Bella Lui added so five can pass; a `known_gap` only for `M ≥ 1`, checked and pinned;
  RocketPy's `reference_pressure=None` transcribed as `None`, not the sea-level stand-in.
- ADR-015: a run reads references, never writes them; every value carries its source and the file
  its hash; every metric is gated at 3% with no floor, or declared not scored; locked cases must
  run; inputs come from the oracle's own record; RocketPy comparisons fly RocketPy's gravity.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C from Abbott Aerospace; WMO-No. 8
  from Mongolia's weather service). Dryden turbulence is an aircraft model, unvalidated for
  rockets, and no flight uses it (#39).
- Only 32 curves are bundled (none in class A); the rest wait for M5's cache. Its checks and the
  1710-file sweep ran on unpinned `refs/samples/` caches.
- Wall and fin mass may differ from OpenRocket's undocumented conventions; M2.2 measures it.
- `.CDX1` has no public spec (the importer relies on samples); ERA5 `.nc` may be netCDF4 (HDF5).
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason. API snapshots can't be reproduced once an API moves: CI checks committed fixtures only.
- Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke have no clear terms: never redistribute.
- Aero (M1.5a) is small-angle only and documented to Mach 0.8; body-lift `K` is uncertain (Galejs:
  1.0 to 1.5) and the Recruiter's six fins miss the printed slope by +3.42% (+2.87% whole; ADR-008).
- Drag (M1.5b): the RASAero comparison can't show 10% agreement without the exports' inputs (fins
  and finish move each case by 20%+). hpr misses Valetudo's suspect table by 47% and Cavour's
  power-on by 18% (open; ADR-009); drag reads low from about Mach 0.6 until M1.8.
- Flight (M1.6b): no tip-off, roll forcing or damping (M1.8), turbulence or thrust misalignment;
  the small-angle aero is used at every `α`. Four `mass_properties` calls are most of an
  evaluation's 0.4 µs (`perf.md`).
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a).
  Streamer pleats are not modelled (hpr reads +58% fast on Kidwell's pleated streamer, +9% on his
  flat one); tumble misses its own finless drop by +19% (M1.7b).
- `refs doctor` "runnable" means the oracle's runtime starts; no oracle runs in CI (M2.1b).
- hpr's descent results are not bit-identical across macOS, Windows and Linux: the committed report
  is pinned to six decimals, where they agree; CI proved full precision does not (M2.1a).
