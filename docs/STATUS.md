# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8c Roll and damping
- **Order:** M1.8c, M1.8d, M1.8e, then M3.1
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1, M1.8a and M1.8b (b1 to b3) have shipped. The site is live
  at https://nrdptel.github.io/hpr-sim/
- **Last updated:** 2026-09-18 (M1.8b3 done; M1.8c not started)

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
- **The path in wind (ADR-026):** the oracle flies RocketPy 1.13.0 with upstream PRs #1188 and
  #1196 applied by `corrections.py`; when RocketPy releases #1196, re-pin, regenerate and delete
  it. `wind_response.py` measures the seven drifts reported as model differences.
- **M1.8a/b (ADR-027 to ADR-029):** `cargo xtask aero` writes `normal-force-vs-mach.json`,
  `drag-vs-mach.json` and `rocketpy-drag-curves.json` (hpr's values and errors only). Working
  files, uncommitted: `refs/scratch/arcas/` (roll data for M1.8c), `refs/scratch/stoney/`,
  `refs/scratch/m18b2/` (`range` sweeps Calisto's fin inputs by band).
- **M1.8b3 (ADR-030):** `hpr_aero::afterbody`: Fig. 5-122 (Jack's second-order theory) held to
  the Prandtl–Meyer limit, 16°–30° separation, Fig. 5-141 base relief, a lip in a boattail's wake.
  `measured-boattails.json` holds the transcribed NACA/NASA readings (checked twice against the
  scans); `cargo xtask aero` compares them in `drag-vs-mach.json`. NTRS serves five of the new
  PDFs with a 436-byte header (pinned as served); readable cut copies and all research notes are
  in `refs/scratch/m18b3/` (`carved/`, `*.md`, `cubbage-transonic.json`). Open: #72 (steep
  boattails in a thick boundary layer read high), #73 (the subsonic rule gives long boattails 0).
- **M1.8c** next: roll forcing from cant and roll damping; its done-when compares hpr's roll
  forcing with the Arcas Robin's measured roll effectiveness (TN D-4014; roll readings in
  `refs/scratch/arcas/`). Damping must keep hpr's local-flow pitch damping (ADR-026). Issues #67 to
  #70, #72 and #73 hold the drag gaps. Don't read predicted mode's misses as gaps to close
  (ADR-009, ADR-023). ROADMAP is at its 1000-line budget: trim a done entry when adding.
- **Regeneration is not bit-identical across machines** (last digits). Regenerate reports with
  `cargo xtask validate` (debug), never `--release`: it rounds differently in the 7th digit.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock.
  Oracles run from the repo root with `refs/venv/bin/python`. `cargo xtask designs` and
  `cargo xtask examples` rewrite designs and example outputs; pages quoting them must follow.

## Done log (newest first, keep about 15)

- 2026-09-18: M1.8b3 The boattail and base faster than sound (ADR-030), and with it M1.8b: not
  met, recorded. Measured boattails of 3° to 10° −21.9% to +28.3%; Arcas Robin fins off from
  Mach 1.5 0 of 11 (+13.5% to +24.1%, the steep boattail, #72); Calisto supersonic 8 of 17.
- 2026-09-18: M1.8b2 Drag against RASAero through Mach 2 (ADR-029): not met, recorded. Calisto's
  export 15/15 subsonic, 2/7 transonic, 0/17 supersonic (−29.8% to −24.4%), no fin input closes
  it; MIL-HDBK-762's worked example (fins left out) 6/12, the body 6–10% low past Mach 1.6; a
  boattail's wave drag is a candidate (M1.8b3).
- 2026-09-18: M1.8b1 The drag buildup through Mach 1 (ADR-028): Niskanen's appendix B with
  Stoney's digitized curves, Mach 0 to 5; against the Arcas Robin's forebody axial force 8 of 44
  within 10% (high past Mach 1.2 with fins); predicted Prometheus flies; no known gap left.
- 2026-09-18: M1.8a The normal force through Mach 1 (ADR-027): supersonic linear theory and a
  transonic join; against NASA's Arcas Robin wind tunnel, Mach 1.5–2.96 within 13.4% and 0.42
  calibers; Prometheus flies and is scored; M1.8 split into a to e.
- 2026-09-18: M2.1d3 The path in wind (ADR-026, issue #50), and with it M2.1: RocketPy's equations
  corrected as upstream PRs #1188 and #1196 do; six drifts now gated, five measured as body lift
  and rail release.
- 2026-09-18: M0.4d Publish, and with it M0.4 (PR #59, ADR-019): Neer turned Pages on; CI run
  35396233336 on `main` deployed the guide and the rustdoc. ThrustCurve's catalog values confirmed
  as facts (ADR-005).
- 2026-09-18: M2.1d2 The calm-air cases (PR #57, ADR-025): Calisto and Bella Lui pass; Juno III's
  drifts not scored, 1.6 of their 3.7 points measured as the rail release.

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
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027), and `rocketpy-drag-curves.json` hpr's
  values and errors from which 147 values of five RocketPy drag curves can be rebuilt (ADR-029),
  more than ADR-009's one per curve. If not fine, say so in an issue; the next session keeps band
  summaries only.

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-030: Fig. 5-122 to the Prandtl–Meyer limit, 16°–30° separation, Fig. 5-141 as a ratio, the
  flow behind boattails shared among their tails, a step down sheltering a lip (a retainer: 5% to
  23% less `C_D0`, unmeasured); targets not met, not tuned.
- ADR-029: M1.8's drag bullet recorded as not met, not chased; MIL-HDBK-762's worked example
  added (fins left out); L18's test renamed to measure and pin; M1.8b3 added for the afterbody.
- ADR-028: M1.8b split into b1 and b2; Stoney's Figure 12 read by hand into the code (panel (a),
  (b) for two shapes); cones and ogives below fineness 1 scale toward a flat face (L15 holds);
  the buildup refuses bulged ogives and Haack past `C = ⅓`; the known gap means a refusal at Mach 5.
- ADR-027: M1.8 split into a to e; fins' supersonic slope counts both faces (Niskanen's eq. 3.49
  counts one); the transonic join is not fitted to the wind tunnel; NASA's plots were read by hand
  into a committed fixture; the body's supersonic gap became M1.8e.
- ADR-026: the oracle flies RocketPy 1.13.0 with two upstream corrections; hpr keeps body lift and
  its rail release; a drift is gated unless a measurement excuses it.
- M0.4, M1.4, M1.5, M1.6, M1.7 and M2.1b were split into increments, done-when bullets unchanged.
- ADR-001 to ADR-025 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
  nose; Niskanen's drag as printed at 20 µm; own DOPRI5; recovery in `hpr-sim`; the site's own link,
  label and number checks; references read, never written; 3% gates or a written reason;
  predicted mode's 3% a target. #11: `SolidMotor` refuses `c = I/m_p` outside 200–5,000 m/s (a
  units guard). M2.1b1: same-drag cases declare their own `C_D0(M)`.

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
- Aero (M1.5a) is small-angle only; body-lift `K` is uncertain (Galejs: 1.0 to 1.5) and the
  Recruiter's six fins miss the printed slope by +3.42% (+2.87% whole; ADR-008). Through Mach 1
  (M1.8a) the normal force misses the wind tunnel between Mach 0.8 and 1.2, and past Mach 3 reads
  17–25% low from the body (M1.8e; ADR-027).
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic, within what the
  unrecorded fins span (ADR-030); against MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and
  high through Mach 1 (nose #67, base #68). Against the Arcas Robin it reads high at every row:
  fins take a blunt edge's formula (#70), a steep boattail in a thick boundary layer reads high
  (#72), the boattail rule over-predicts subsonic (#73). A cylinder's base drag is unmeasured
  past Mach 0.3; behind a boattail its relief matches 12 measured bases.
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
