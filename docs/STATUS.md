# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8 Aerodynamics II (transonic and supersonic, damping, overrides)
- **Order:** M1.8, then M3.1
- **Run:** M0.1-M0.4, M1.1-M1.7 and M2.1 have shipped. The site is live at
  https://nrdptel.github.io/hpr-sim/
- **Last updated:** 2026-09-18 (M2.1d3 done, and with it M2.1; M1.8 not started)

## Handoff (overwrite each session)

- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest,
  labels as links to their rows, none in headings, Unicode equations; new pages in `SUMMARY.md`; a
  new library needs a row in `docs/api.md` and a guide link in its `//!`. *Accuracy*'s numbers
  must be in a file the same item links (the report, a case file); its results tables must hold
  every row of the report, cell for cell.
- **Checking a milestone off** in `ROADMAP.md` fails `cargo xtask site` until its row in
  `docs/decisions-and-roadmap.md` says `done`; a new milestone needs a row.

- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024); a reference that moves moves those bounds with it.
- **M2.1d3, the path in wind (ADR-026):** the whole-flight oracle flies RocketPy 1.13.0 with
  upstream PRs #1188 (merged) and #1196 (open) applied by `corrections.py`: as released,
  `u_dot_generalized` took its moments during the burn about the wrong point. When RocketPy
  releases #1196, re-pin, regenerate and delete `corrections.py`. `wind_response.py` flies every
  case as released, corrected, and with hpr's rail release, body lift and thin fins added. Five
  drifts stay reported as measured model differences (Juno III and Bella Lui in wind, NDRT's
  apogee drift); Prometheus's drifts are gated for when hpr flies it.
- **M1.8** next: don't read predicted mode's +10% (Valetudo, NDRT) as gaps to close. hpr's drag
  there runs on placeholder fin edges and finishes, and Valetudo's table is suspect (ADR-009).
  Both Prometheus cases are checked `M ≥ 1` gaps that fail the run once hpr flies them; its
  drifts are then held to 3% like every other metric. M1.8's damping must keep hpr's local-flow
  pitch damping, which ADR-026 found agrees with corrected RocketPy's.
- **Regeneration is not bit-identical across machines:** the first *Regenerate references* run
  (GitHub's macOS runner) moved RocketPy's fixtures in their last digits (descents at most 3.6e-11
  relative), changing their hashes and so the report's; the script prints each fixture's move.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock.
  Oracles run from the repo root with `refs/venv/bin/python`. `cargo xtask designs` and
  `cargo xtask examples` rewrite designs and example outputs; pages quoting them must follow.

## Done log (newest first, keep about 15)

- 2026-09-18: M2.1d3 The path in wind (ADR-026, issue #50), and with it M2.1: RocketPy's equations
  corrected as upstream PRs #1188 and #1196 do; six drifts now gated, five measured as body lift
  and rail release.
- 2026-09-18: M0.4d Publish, and with it M0.4 (PR #59, ADR-019): Neer turned Pages on; CI run
  35396233336 on `main` deployed the guide and the rustdoc. ThrustCurve's catalog values confirmed
  as facts (ADR-005).
- 2026-09-18: M2.1d2 The calm-air cases (PR #57, ADR-025): Calisto and Bella Lui pass; Juno III's
  drifts not scored, 1.6 of their 3.7 points measured as the rail release.
- 2026-09-18: M2.1d1 The time-series RMS (PR #54, ADR-024): height and speed RMS on the ten flown
  whole flights, aligned at ignition; ten same-drag RMS rows pass, three predicted outside target.
- 2026-09-18: M2.1c2 Predicted mode (PR #52, ADR-023): hpr's own drag against RocketPy on each
  example's own drag; 3% targets, not gates; 56 of 75 within; M2.1d split off for the RMS and #50.
- 2026-09-18: M2.1c1 Validation in CI and regeneration by hand (PR #51, ADR-022): `validate
  --check` on three OSes; a `workflow_dispatch`-only workflow that uploads the references' diff.
- 2026-09-18: M2.1b2 The whole-flight cases (PR #49, ADR-021): five pass, Prometheus a checked gap.
- 2026-09-18: M0.4e The reader test (PR #47, ADR-020): cold readers answered 9 of 10, then all.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and three
  `validate (...)` checks; block force
  pushes. Don't require approvals: the autopilot merges its own PRs as you, and authors can't
  self-approve.
- **Loft's flutter calculator overstates flutter speed by √2** (safety). fusionspace-loft
  `lib/sim/flutter.ts:287` uses 1.337·(λ+1)/2; NACA TN 4197 eq. 18 gives 2.674·(λ+1)/2 (39.3 over
  14.7 psi), so its "1.5 margin" is about 1.06. Fix it or post a notice before Loft shuts down.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`... are unreserved. Reserve them?
- **orhelper** (no action if fine): GPL-2.0, so M2.2 drives OpenRocket via JPype, never imports it.

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-026: the oracle flies RocketPy 1.13.0 with two upstream corrections (PR #1188 merged, #1196
  open, both on RocketPy's record); hpr keeps body lift and its rail release; a drift is gated
  unless a measurement excuses it, so Prometheus's drifts are gated for when it flies.
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
- ADR-023: predicted mode scores against RocketPy on the examples' own drag as RocketPy really
  flies it (post-construction rescalings ignored); its 3% is a target, reported, never gated.
- ADR-022: CI checks the committed report to the digits the platforms share; references are
  regenerated only by hand and reviewed as a diff; M2.1c split into c1 (CI) and c2 (predicted).
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
- In wind, a slow rocket's drift in hpr rests on body lift's uncertain `K`: Juno III's apogee
  drift is 240 to 194 m over Galejs's 1.0 to 1.5 (ADR-026, `wind_response.py`). The oracle carries two unreleased
  RocketPy corrections; if #1196 changes before it merges, revisit `corrections.py`.
- Flight (M1.6b): no tip-off, roll forcing or damping (M1.8), turbulence or thrust misalignment;
  the small-angle aero is used at every `α`. Four `mass_properties` calls are most of an
  evaluation's 0.4 µs (`perf.md`).
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a).
  Streamer pleats are not modelled (hpr reads +58% fast on Kidwell's pleated streamer, +9% on his
  flat one); tumble misses its own finless drop by +19% (M1.7b).
- `refs doctor` "runnable" means the oracle's runtime starts; no oracle runs in CI (M2.1b).
- Results are not bit-identical across macOS, Windows and Linux: the committed report is pinned to
  six decimals, or 1e-7 relative for whole flights, where they agree (M2.1a, M2.1b2).
