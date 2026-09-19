# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e2 The body's supersonic normal force in flight
- **Order:** M1.8e2, then M3.1
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1 and M1.8a to M1.8e1 have shipped.
  The site is live at https://nrdptel.github.io/hpr-sim/
- **Last updated:** 2026-09-19 (M1.8e1 done; M1.8e2 not started)

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
- **M1.8a to e1 (ADR-027 to ADR-033):** `cargo xtask aero` writes the aero fixtures (hpr's
  values and errors only). NTRS serves five of ADR-030's PDFs with a 436-byte header (pinned as
  served). Scratch: `refs/scratch/{arcas,stoney,m18b2,m18b3,m18c,m18d,m18e}/`. TN D-4013's
  rolling-moment plots are unread; #76: M1.8a's other TN D-4014 zeros. M1.8e1's Python check,
  `m18e/sose.py` (patch in hpr's Fig. 2), carries the gradient through reduced elements; hpr
  doesn't (#81).
- **M1.8e2** next: fly `hpr_aero::shock_expansion`. Open: blunt or vertical tips (hpr's Arcas
  Robin design is a power series; the fixture's fitted secant ogive has ratio 1.744), Mach below
  3 (Fig. 2 held), the join from subsonic (Prometheus peaks at Mach 1.01 to 1.06), the boattail
  (footnote 8 takes only 0.03 to 0.18 off), crossflow (the long model's extra 0.57), Mach over
  nose fineness outside 0.4 to 2, #81, and `dynamics.rs` caching body stations at Mach 0. Don't read predicted mode's misses as gaps to
  close (ADR-009, ADR-023). ROADMAP is at 999 of 1000 lines: trim a done entry.
- **Regeneration is not bit-identical across machines** (last digits). Regenerate reports with
  `cargo xtask validate` (debug), never `--release`: it rounds differently in the 7th digit.
  Fixture checks (`designs::same`) allow 1e-12 relative, or 1e-13 near zero (M1.8b3's PR).
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock.
  Oracles run from the repo root with `refs/venv/bin/python`. `cargo xtask designs` and
  `cargo xtask examples` rewrite designs and example outputs; pages quoting them must follow.

## Done log (newest first, keep about 15)

- 2026-09-19: M1.8e1 The second-order shock-expansion method (ADR-033): not met, recorded.
  Against TN 3527's measurements 117 of 120 slopes and 109 of 120 CPs within ±0.2; its printed
  values 102 and 125 of 144 (#81 at the limit); Arcas Robin short −18.7% to +16.4%, long to −26.4%.
- 2026-09-19: M1.8d Normal-force overrides (ADR-032): RASAero II's export read per angle of
  attack, flown with hpr's damping kept; every row of Calisto's export re-read, and it flies
  (apogee −1.28 m, 14.2 m upwind); a table's pitch period within 4e-6 of theory.
- 2026-09-19: M1.8c Roll and damping (ADR-031): Barrowman's strip theory; Valetudo canted 1°
  settles on the closed-form roll rate within 1e-11; TN D-4014's roll effectiveness from Mach 2.3
  8 of 8 within 5.3%, +14.3% to +47.8% at Mach 1.5 and 1.8; the Basic Finner's damping −5.9% to
  −16.2%.
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

- ADR-033: M1.8e split into e1 (the method) and e2 (flying it); TN 3527's ten-element tangent
  body; `η < 0` elements reduced to the generalized method with no gradient carried (p. 13); Fig. 2
  held below Mach 3; the Arcas Robin's nose as a fitted secant ogive for the comparison only.
- ADR-032: a normal-force table replaces only the static force, hpr's damping kept; the 0° slope
  from `CN Potential`; past the last angle `sin α` and `sin² α` shares; no new RASAero values.
- ADR-031: roll damping takes the fin's own slope, not the airfoil's Barrowman's text writes (his
  computed curve does); Barrowman's body factors kept; no target set after measuring.
- ADR-030: Fig. 5-122 to the Prandtl–Meyer limit, 16°–30° separation, Fig. 5-141 as a ratio, the
  flow behind boattails shared among their tails, a step down sheltering a lip (a retainer: up to
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
  17–25% low from the body (M1.8e2; ADR-027, ADR-033).
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic, within what the
  unrecorded fins span (ADR-030); against MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and
  high through Mach 1 (nose #67, base #68). Against the Arcas Robin it reads high at every row:
  fins take a blunt edge's formula (#70), a steep boattail in a thick boundary layer reads high
  (#72), the boattail rule over-predicts subsonic (#73). A cylinder's base drag is unmeasured
  past Mach 0.3; behind a boattail its relief matches 12 measured bases.
- In wind, a slow rocket's drift in hpr rests on body lift's uncertain `K`: Juno III's apogee
  drift is 240 to 194 m over Galejs's 1.0 to 1.5 (ADR-026, `wind_response.py`). The oracle carries two unreleased
  RocketPy corrections; if #1196 changes before it merges, revisit `corrections.py`.
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`.
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a).
  Streamer pleats are not modelled (hpr reads +58% fast on Kidwell's pleated streamer, +9% on his
  flat one); tumble misses its own finless drop by +19% (M1.7b).
- `refs doctor` "runnable" means the oracle's runtime starts; no oracle runs in CI (M2.1b).
- Results are not bit-identical across macOS, Windows and Linux: the committed report is pinned to
  six decimals, or 1e-7 relative for whole flights, where they agree (M2.1a, M2.1b2).
