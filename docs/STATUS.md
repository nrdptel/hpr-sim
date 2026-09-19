# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e7 Blunt tips and the lip faster than sound
- **Order:** M1.8e7, e8, then M3.1
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1, M1.8a to M1.8e6 shipped; https://nrdptel.github.io/hpr-sim/
- **Last updated:** 2026-09-19 (M1.8e6 done)

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
  rolling-moment plots are unread; #76: M1.8a's other TN D-4014 zeros. `m18e/sose.py` (M1.8e1's
  Python check) carries the gradient through reduced elements; hpr doesn't (#81).
- **M1.8e2 to e4** (ADR-034): `SupersonicBody` (`hpr-aero/src/model.rs`) tabulates the method's
  shares every 0.05 Mach, lazily, joined from max(1.2, its bisected start) over 0.3; boattails too.
- **M1.8e6** (ADR-037): body lift is Jorgensen's `η C_dn` (`hpr-aero/src/crossflow.rs`) at every
  speed; a supersonic boattail W&P's increment (`supersonic_boattail.rs`) on the method's cylinder
  in its place. `BodyModel::BEFORE_M1_8E6` reproduces the old model (M1.8e5's fixture uses it).
  `arcas-robin-crossflow.json` (`xtask/src/aero_crossflow.rs`) compares four models, the 62
  high-angle points and the body's CP; its readings are `arcas-robin-high-alpha.json` and
  `arcas-robin-fins-off-moment.json`. `wind_response.py` carries the same tables (a test checks).
  #98: a covered boattail doubles the supersonic table's build (NDRT 316 → 616 ms).
- **M1.8e7 next:** the committed Arcas Robin designs (power-series nose, vertical tip; the lip, a
  flare behind the boattail) keep slender-body theory, so M1.8a's short@2.96 now misses (−16.3%).
  Blunt tip: NASA TN D-4865 puts a Newtonian cap ahead of TN 3527's method (Mach 1.5 to 4.63).
  The lip: `rest_carries_nothing` refuses it; check any rule by the moment about the CG. #97: the
  long model's M1.8a readings may be biased (page skew); settle it before judging e8's 15%.
- **Autopilot memory:** each command gets its own process group; the run notes and reaps them.
- **M2.2's OpenRocket oracle** (ADR-035): orhelper is dropped, so decide how to drive the jar
  when M2.2 starts. JPype still loads the JVM in-process; only a subprocess isolates. The jar
  needs Java 17 exactly; `[java] max_major` in the refs lock now keeps doctor off a newer one.
- **Regeneration is not bit-identical across machines** (last digits). Regenerate reports with
  `cargo xtask validate` (debug), never `--release`: it rounds differently in the 7th digit.
  Fixture checks (`designs::same`) allow 1e-12 relative, or 1e-13 near zero (M1.8b3's PR).
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock.
  Oracles run from the repo root with `refs/venv/bin/python`. `cargo xtask designs` and
  `cargo xtask examples` rewrite designs and example outputs; pages quoting them must follow.

## Done log (newest first, keep about 15)

- 2026-09-19: M1.8e6 Crossflow and the boattail faster than sound (ADR-037): Jorgensen's body lift
  at every speed and Washington and Pettis's measured boattail; like for like the Arcas Robin's
  body reads +3.4% to +41.0% (was +14.9% to +73.2%), its CP 0.9–3.9 cal nearer the tunnel's; 48
  of 62 high-angle points within 15%; M1.8a's short@2.96 now misses (−16.3%); split e7, e8.
- 2026-09-19: M1.8e5 The remaining gap, source by source (ADR-036): like for like hpr's body
  reads 15–73% high, not low; crossflow's size ranks first, the boattail second, then the lip
  (+0.18), the blunt tip (≤ 0.07), Fig. 2 below Mach 3 (≤ 0.06, SP-3007); #81 zero.
- 2026-09-19: M1.8e4 The boattail's share faster than sound: boattails and tubes behind them
  fly the method (their stations slender-body theory's); no jump at ±1e-9; the Arcas Robin
  through the flight with its boattail equals the method's (long −18.3% to −27.0%).
- 2026-09-19: M1.8e3 The supersonic join's start (#87's grid half): bisected where the method
  starts to hold, not on the 0.05 grid; a 20° cone joins from Mach 1.341910 (was 1.35), moving
  smoothly to 1.355500 at 20.5°; no jump at ±1e-9 in Mach; the report unchanged.
- 2026-09-19: M1.8e2 The body's supersonic normal force in flight (ADR-034): nose and cylinder
  take the method's shares, joined over Mach 1.2 to 1.5, no jump at ±1e-9; Arcas Robin nose and
  cylinder through the flight equal the method (+16.4% to −26.4%); boattailed bodies unchanged.
- 2026-09-19: Autopilot memory: the run notes its commands' process groups while sampling and
  reaps them (a cycle's own group missed every build); jobs capped at 6, old transcripts pruned.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and three
  `validate (...)` checks; block force pushes. Don't require approvals: the autopilot merges its
  own PRs as you, and authors can't self-approve.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`... are unreserved. Reserve them?
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027), and `rocketpy-drag-curves.json` hpr's
  values and errors from which 147 values of five RocketPy drag curves can be rebuilt (ADR-029),
  more than ADR-009's one per curve. If not fine, say so in an issue; the next session keeps band
  summaries only.

## Decided without Neer (one line each; significant ones get an ADR)

- M1.8e splits (one id level): e3 join, e4 boattail, e5 measure, e6 fly, e7 blunt tips, e8 rest.
- ADR-037: Jorgensen's body lift at every speed, his two `η`s blended (a judgement), sampled at
  Fig. 6's points; W&P's measured boattail; old model kept selectable; M1.8a's new miss recorded.
- ADR-036: the Arcas Robin judged at the tunnel's angles; e6 retitled to crossflow and the boattail.
- ADR-033/034: TN 3527's method (ten-element tangent body, `η < 0` reduced, Fig. 2 held below
  Mach 3), tabulated every 0.05 Mach lazily, joined over Mach 1.2 to 1.5; boattails since M1.8e4.
- ADR-032: a normal-force table replaces only the static force; hpr's damping kept. ADR-031: roll
  damping takes the fin's own slope (Barrowman's computed curve does); no target set after.
- ADR-030: Fig. 5-122 to the Prandtl–Meyer limit, 16°–30° separation, Fig. 5-141 as a ratio, the
  flow behind boattails shared among their tails, a step down sheltering a lip (a retainer: up to
  23% less `C_D0`, unmeasured); targets not met, not tuned.
- ADR-029: M1.8's drag bullet recorded as not met, not chased; MIL-HDBK-762's example added.
- ADR-028: Stoney's Figure 12 read by hand; cones and ogives below fineness 1 scale toward a flat
  face; the buildup refuses bulged ogives and Haack past `C = ⅓`.
- ADR-027: M1.8 split into a to e; fins' supersonic slope counts both faces (Niskanen's eq. 3.49
  counts one); the transonic join is not fitted to the wind tunnel; NASA's plots were read by hand
  into a committed fixture; the body's supersonic gap became M1.8e.
- M0.4, M1.4, M1.5, M1.6, M1.7 and M2.1b were split into increments, done-when bullets unchanged.
- ADR-001 to ADR-026 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
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
- Aero (M1.5a) is small-angle only; the Recruiter's six fins miss the printed slope by +3.42%
  (ADR-008). Body lift (Jorgensen, M1.8e6) reads 1–16% high where the crossflow is supersonic and
  leaves out the drop past the critical Reynolds number. The normal force misses the wind tunnel
  between Mach 0.8 and 1.2, and past Mach 3 reads 20–28% low on bodies the method can't take.
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic, within what the
  unrecorded fins span (ADR-030); against MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and
  high through Mach 1 (nose #67, base #68). Against the Arcas Robin it reads high at every row:
  fins take a blunt edge's formula (#70), a steep boattail in a thick boundary layer reads high
  (#72), the boattail rule over-predicts subsonic (#73). A cylinder's base drag is unmeasured
  past Mach 0.3; behind a boattail its relief matches 12 measured bases.
- In wind, a slow rocket's drift rests on body lift's size: Juno III's apogee drift is 245 m in
  hpr (Jorgensen's), 240 to 194 m over Galejs's `K` 1.0 to 1.5 (`wind_response.py`). The oracle
  carries two unreleased RocketPy corrections; if #1196 changes, revisit `corrections.py`.
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`.
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a).
  Streamer pleats are not modelled (hpr reads +58% fast on Kidwell's pleated streamer, +9% on his
  flat one); tumble misses its own finless drop by +19% (M1.7b).
- `refs doctor` "runnable" means the oracle's runtime starts; no oracle runs in CI (M2.1b).
- Results are not bit-identical across macOS, Windows and Linux: the committed report is pinned to
  six decimals, or 1e-7 relative for whole flights, where they agree (M2.1a, M2.1b2).
