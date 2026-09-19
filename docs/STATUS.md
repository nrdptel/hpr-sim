# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e10 #87's model switches
- **Order:** M1.8e10, then M3.1
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1, M1.8a to M1.8e9 shipped; https://nrdptel.github.io/hpr-sim/
- **Last updated:** 2026-09-19 (M1.8e9 done)

## Handoff (overwrite each session)

- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest,
  labels as links to their rows, none in headings, Unicode equations; new pages in `SUMMARY.md`; a
  new library needs a row in `docs/api.md` and a guide link in its `//!`. *Accuracy*'s numbers must
  be in a file the item links; its tables must hold every report row, cell for cell.
- **Checking a milestone off** fails `cargo xtask site` until its row in
  `docs/decisions-and-roadmap.md` says `done`; a new milestone needs a row.
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024); a moving reference moves those bounds.
- **The path in wind (ADR-026):** the oracle flies RocketPy 1.13.0 with PRs #1188 and #1196 applied
  by `corrections.py` (re-pin and delete it when #1196 releases); `wind_response.py` measures the
  seven drifts reported as model differences.
- **M1.8a to e1 (ADR-027 to ADR-033):** `cargo xtask aero` writes the aero fixtures. NTRS serves
  five of ADR-030's PDFs with a 436-byte header (pinned as served). Scratch: `refs/scratch/m18*/`.
  TN D-4013's rolling-moment plots are unread; #76: M1.8a's other TN D-4014 zeros. #81:
  `m18e/sose.py` carries the gradient through reduced elements; hpr doesn't.
- **M1.8e2 to e8** (ADR-034, 037, 038, 039): `SupersonicBody` (`model.rs`) tabulates the method's
  shares every 0.05 Mach, lazily, joined from max(1.2, its bisected start) over 0.3. Body lift is
  Jorgensen's (`crossflow.rs`); a boattail W&P's increment (`supersonic_boattail.rs`);
  `BodyModel::BEFORE_M1_8E6` keeps the old rules. A vertical tip takes TN D-4865's Newtonian cap
  (`blunt_tip.rs`), the march started from the tangent cone. Fixtures: `arcas-robin-crossflow.json`,
  `blunt-tips.json` (readings by pixel analysis), `arcas-robin-lip.json`. #98: a boattail doubles
  the table's build (NDRT 616 ms). #101: a vanishing cap keeps its start cone's entropy.
- **M1.8e8** (ADR-039): a lip wholly in a boattail's wake (the drag buildup's `WakeTerm`) carries
  no slope above the join, so the committed Arcas Robin designs fly the method to their base;
  `arcas-robin-lip.json` (`xtask/src/aero_lip.rs`) weighs the candidates against the fins-off
  moment, which bounds the lip (+0.022 ± 0.018 per rad, χ²/dof 4.6) rather than measuring it.
- **M1.8e9** (ADR-040): a boattail's measured increment fades over the angles that separate its
  flow (16° to 30°), so #90's cap is graded, not a switch; footnote 8 is pinned by a hand integral
  of eq. 19 (`ShockExpansionBody::element_flows` is new public API). M1.8e's 15% bullet, judged in
  `arcas-robin-body-gap.json`: met at Mach 3.96 and 4.63, outside on six body-alone rows.
- **M1.8e10 next:** #87's model switches. What remains: a step in radius past a millionth of the
  area, a pointed tip past Fig. 2's 24°, a flare behind the run, a vertical tip steeper than the
  cap's handover to its base, and the lip's shelter threshold (measured: `C_N` −33% and the CP 1.8
  calibres across it, issue #87's comment). A weight on the method's share, blended over a band of
  shape as the Mach join blends over 0.3 Mach, would smooth them; each band is a judgement and
  needs an ADR. #97: the long model's M1.8a readings may be biased (page skew).
- **M2.2's OpenRocket oracle** (ADR-035): orhelper is dropped, so decide how to drive the jar when
  M2.2 starts; JPype loads the JVM in-process, only a subprocess isolates, and the jar needs Java
  17 exactly (`[java] max_major` in the refs lock keeps doctor off a newer one).
- **Regeneration is not bit-identical across machines** (last digits): regenerate with `cargo
  xtask validate` (debug), never `--release`; fixture checks allow 1e-12 relative (1e-13 near 0).
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock;
  oracles run from the repo root with `refs/venv/bin/python`; `cargo xtask designs` and `examples`
  rewrite designs and example outputs, and pages quoting them must follow.

## Done log (newest first, keep about 15)

- 2026-09-19: M1.8e9 #90's cap and M1.8e's 15% bullet (ADR-040): the measured boattail increment
  fades from 16° to 30° as the flow separates; footnote 8 pinned by a hand integral; the bullet met
  at Mach 3.96 and 4.63, the body alone outside on six rows, the resolvable gap all in body lift.
- 2026-09-19: M1.8e8 The lip faster than sound (ADR-039): a lip in a boattail's wake carries
  nothing above the join, so every M1.8a row from Mach 1.5 is within the slope's 15% (+9.4% to
  −3.3%, was −28.0%); the long model's CP at Mach 1.8 and 2.3 is 0.53 and 0.52 calibres out.
- 2026-09-19: M1.8e7 Blunt tips faster than sound (ADR-038): power-series, Haack and elliptical
  noses fly the method behind TN D-4865's Newtonian cap, started from the tangent cone, with no
  jump at ±1e-9 in Mach; its sphere-cone like for like −1.2% to +32.1% (its own method −3.3% to
  +13.6%); the Arcas Robin's nose, lip off, −4.8% to +37.2%.
- 2026-09-19: M1.8e6 Crossflow and the boattail faster than sound (ADR-037): Jorgensen's body lift
  at every speed and Washington and Pettis's measured boattail; like for like the body reads +3.4%
  to +41.0% (was +14.9% to +73.2%); 48 of 62 high-angle points within 15%; split e7, e8.
- 2026-09-19: M1.8e2 to e5 (ADR-034, 036): the body's supersonic shares in flight from a bisected
  start, boattails and their tubes, and the gap sized source by source (crossflow first).

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and three
  `validate (...)` checks; block force pushes. Don't require approvals: the autopilot merges its
  own PRs as you, and authors can't self-approve.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`... are unreserved. Reserve them?
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027), and `rocketpy-drag-curves.json` enough
  to rebuild 147 values of five RocketPy drag curves (ADR-029), more than ADR-009's one per curve.
  If not fine, say so in an issue; the next session keeps band summaries only.

## Decided without Neer (one line each; significant ones get an ADR)

- M1.8e splits (one id level): e3 join, e4 boattail, e5 measure, e6 fly, e7 blunt tips, e8 the
  lip, e9 rest (renumbered when e7 split; done-when bullets unchanged in substance).
- ADR-038: the march behind a blunt tip starts from the tangent cone, not TN D-4865's Newtonian
  state (which fails on the Arcas nose from Mach 3.96); handover capped at 24°. ADR-039: a lip in
  a boattail's wake carries nothing faster than sound, and the fins-off moment bounds it rather
  than measuring it. ADR-040: a boattail's measured increment fades over the angles that separate
  it; M1.8e's 15% bullet is recorded not met for the body alone, the gap left visible.
- ADR-037: Jorgensen's body lift at every speed, his two `η`s blended (a judgement), sampled at
  Fig. 6's points; W&P's measured boattail; old model kept selectable; M1.8a's new miss recorded.
- ADR-036: the Arcas Robin judged at the tunnel's angles; e6 retitled to crossflow and the boattail.
- ADR-033/034: TN 3527's method (ten-element tangent body, `η < 0` reduced, Fig. 2 held below
  Mach 3), tabulated every 0.05 Mach lazily, joined over Mach 1.2 to 1.5; boattails since M1.8e4.
- ADR-032: a table replaces only the static force. ADR-031: roll damping takes the fin's own slope.
- ADR-030: Fig. 5-122 to the Prandtl–Meyer limit, 16°–30° separation, Fig. 5-141 as a ratio, the
  flow behind boattails shared among their tails, a step down sheltering a lip; targets not tuned.
- ADR-029: M1.8's drag bullet recorded as not met, not chased; MIL-HDBK-762's example added.
- ADR-028: Stoney's Figure 12 read by hand; cones and ogives below fineness 1 scale toward a flat
  face; bulged ogives and Haack past `C = ⅓` refused. M0.4, M1.4 to M1.7 and M2.1b were split.
- ADR-027: M1.8 split into a to e; fins' supersonic slope counts both faces; the transonic join is
  not fitted to the tunnel; NASA's plots read by hand into a fixture; the body's gap became M1.8e.
- ADR-001 to ADR-026 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
  nose; Niskanen's drag as printed at 20 µm; own DOPRI5; recovery in `hpr-sim`; the site's link,
  label and number checks; references read, never written; 3% gates or a written reason. #11:
  `SolidMotor` refuses `c = I/m_p` outside 200–5,000 m/s. M2.1b1: same-drag cases declare `C_D0(M)`.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C, WMO-No. 8). Dryden turbulence
  is an aircraft model, unvalidated for rockets, and no flight uses it (#39).
- Only 32 curves are bundled (none in class A); the rest wait for M5's cache, and its checks ran on
  unpinned `refs/samples/` caches.
- Wall and fin mass may differ from OpenRocket's undocumented conventions; M2.2 measures it.
- `.CDX1` has no public spec (the importer relies on samples); ERA5 `.nc` may be netCDF4 (HDF5).
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason. API snapshots can't be reproduced once an API moves.
- Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke have no clear terms: never redistribute.
- Aero (M1.5a) is small-angle only; the Recruiter's six fins miss the printed slope by +3.42%
  (ADR-008). Body lift (Jorgensen, M1.8e6) reads 1–16% high where the crossflow is supersonic and
  leaves out the drop past the critical Reynolds number. The normal force misses the wind tunnel
  between Mach 0.8 and 1.2; fins off, the body reads 14–38% high from Mach 1.5 to 2.96 (M1.8e9's
  bullet). A blunt tip's cap (M1.8e7) is checked only on a sphere-cone, covers half a slender nose
  near the join's start, and keeps its start cone's entropy however small it is (#101: a 0.99-power
  nose's cylinder reads 12% low at Mach 4). A lip's share (M1.8e8) is bounded, not measured.
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic (ADR-030); against
  MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and high through Mach 1 (#67, #68). Against
  the Arcas Robin it reads high at every row (#70 blunt fin edges, #72 a steep boattail in a thick
  boundary layer, #73 the subsonic boattail rule). A cylinder's base drag is unmeasured past
  Mach 0.3.
- In wind, a slow rocket's drift rests on body lift's size: Juno III's apogee drift is 245 m in hpr
  (Jorgensen's), 240 to 194 m over Galejs's `K` 1.0 to 1.5. The oracle carries two unreleased
  RocketPy corrections; if #1196 changes, revisit `corrections.py`.
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`.
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a).
  Streamer pleats are not modelled (+58% fast on Kidwell's pleated streamer); tumble +19% (M1.7b).
- Results are not bit-identical across macOS, Windows and Linux: the report is pinned to six
  decimals, or 1e-7 relative for whole flights (M2.1a, M2.1b2). `refs doctor` "runnable" means the
  oracle's runtime starts; no oracle runs in CI (M2.1b).
