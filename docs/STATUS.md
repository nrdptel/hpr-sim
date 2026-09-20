# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e13 The handover moved past 24° — it begins by settling #108
- **Order:** M1.8e13 (M1.8e14 first if #108 proves too deep), then M1.8e15 and M3.1
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1, M1.8a to M1.8e12 shipped; https://nrdptel.github.io/hpr-sim/
- **Last updated:** 2026-09-20 (M1.8e12 done)

## Handoff (overwrite each session)

- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest,
  labels as links to their rows, none in headings, Unicode equations; new pages in `SUMMARY.md`; a
  new library needs a row in `docs/api.md`. *Accuracy*'s numbers live in a file the item links, its
  tables hold every report row, and each aero fixture has a `the_guide_quotes_the_fixture`.
- **Checking a milestone off** fails `cargo xtask site` until its row in
  `decisions-and-roadmap.md` says `done`; a new milestone needs a row.
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024).
- **The path in wind (ADR-026):** the oracle flies RocketPy 1.13.0 with #1188 and #1196 applied by
  `corrections.py` (re-pin and delete it when #1196 releases).
- **M1.8a to e1 (ADR-027 to ADR-033):** `cargo xtask aero` writes the aero fixtures; five of
  ADR-030's PDFs come from NTRS with a 436-byte header. Scratch: `refs/scratch/m18*/`.
- **M1.8e2 to e8** (ADR-034, 037, 038, 039): `SupersonicBody` tabulates the method's shares every
  0.05 Mach, lazily, from max(1.2, its bisected start) over 0.3. Body lift is Jorgensen's, a
  boattail W&P's increment, a vertical tip TN D-4865's cap; `BEFORE_M1_8E6` keeps the old rules.
- **M1.8e9** (ADR-040): #90's cap holds W&P's correlation at 16° for steeper boattails. M1.8e's
  15% bullet (`arcas-robin-body-gap.json`): met at Mach 3.96 and 4.63, outside on six rows.
- **M1.8e10** (ADR-041): #87's five switches flip one gate (`supersonic_run`), so each is worth
  the whole body. The lip's **rise** is a weight now (`SupersonicBody::shape_weight`); five keep
  measured sizes, the lip's own length (−33.0%, 1.77 cal) among them. #106: station vs CP.
- **M1.8e11** (ADR-042): `CONE_SLOPES` runs to 30° — Fig. 2's chart to 24°, then SP-3007 Table 2.
- **M1.8e12** (ADR-043): the handover's cap is a parameter (`with_handover_cap_rad`), swept into
  `blunt-tips.json`. 24° stands: steeper caps read nearer TN D-4865's sphere-cone but none keeps its
  answer to Mach 5 (28° is worst); at 30°, 108 of the nose's 160 elements sit above their cone.
- **M1.8e13 starts at #108**: a reading of `η < 0` that settles as the nose is cut finer (#81);
  `|η|` is worse. Too deep? **M1.8e14** is scoped and unblocked: the method marches a flare already,
  and TN D-4865's model 2 measures one. #97: the long model's M1.8a readings may be biased.
- **M2.2's OpenRocket oracle** (ADR-035): orhelper is dropped, so decide how to drive the jar when
  M2.2 starts; JPype loads the JVM in-process, only a subprocess isolates, and the jar needs Java
  17 exactly (`[java] max_major` in the refs lock keeps doctor off a newer one).
- **Regeneration is not bit-identical across machines** (last digits): regenerate with `cargo xtask
  validate` (debug), never `--release`; fixture checks allow 1e-12 relative. A reduced march drifts
  2.4e-12, so `handover_caps` is stored to six decimals (`sweep_number`).
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock;
  oracles run from the repo root with `refs/venv/bin/python`; `cargo xtask designs` and `examples`
  rewrite their outputs, and pages quoting them follow.

## Done log (newest first, keep about 15)

- 2026-09-20: M1.8e12 What the handover's cap is worth (ADR-043): the cap is a method parameter,
  swept into the fixture and the guide; 24° stays because no steeper cap keeps its answer to Mach
  5 (#108), though each reads nearer the report's own sphere-cone; no fixture moved.
- 2026-09-20: M1.8e11 Cone slopes past Fig. 2's edge (ADR-042): SP-3007 Table 2 carries the same
  theory from 24° to 30°, so a fineness-1 cone flies; the pointed tip's switch moves to 30° and
  falls to −7.7%/0.81 cal; no fixture moved.

- 2026-09-20: M1.8e10 The lip's shelter, weighed not switched (ADR-041): the drag buildup's wake
  fraction is now the method's weight, so a lip drawn taller moves a rocket between the models
  instead of switching it (was −33% and 1.77 calibres); five switches left, each measured.

- 2026-09-19: M1.8e9 #90's cap and M1.8e's 15% bullet (ADR-040): a boattail steeper than 16° reads
  W&P's correlation as a 16° one, the conservative end of a 0.67 to 1.35 calibre range; the bullet
  met at Mach 3.96 and 4.63, the body alone outside on six rows.
- 2026-09-19: M1.8e8 The lip faster than sound (ADR-039): a lip in a boattail's wake carries nothing
  above the join, so every M1.8a row from Mach 1.5 is within the slope's 15% (+9.4% to −3.3%, was
  −28.0%); the long model's CP at Mach 1.8 and 2.3 is 0.5 calibres out.
- 2026-09-19: M1.8e7 Blunt tips (ADR-038): noses with a vertical tip fly behind TN D-4865's cap;
  its sphere-cone −1.2% to +32.1%. e2 to e6 before that.


## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and three
  `validate (...)` checks; block force pushes. Don't require approvals: the autopilot merges its own
  PRs as you, and authors can't self-approve.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`... are unreserved. Reserve them?
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers from its secant
  columns; `rocketpy-drag-curves.json` enough to rebuild 147 values of five RocketPy drag curves
  (ADR-029). If not fine, say so in an issue; the next session summarises.

## Decided without Neer (one line each; significant ones get an ADR)

- M1.8e splits (one id level, so siblings are renumbered): e3 join, e4 boattail, e5 measure, e6
  fly, e7 blunt tips, e8 lip, e9 rest, e10 the lip's weight, e11 Fig. 2's edge, e12 step and flare.
  e9's done-when restates the parent bullet, escape clause included, verbatim from main.
- ADR-038: the march behind a blunt tip starts from the tangent cone, not TN D-4865's Newtonian
  state (which fails on the Arcas nose from Mach 3.96); handover capped at 24°. ADR-039: a lip in a
  boattail's wake carries nothing faster than sound, and the fins-off moment bounds it rather than
  measuring it. ADR-040: a boattail steeper than 16° reads its measured correlation as a 16° one (a
  fade to zero was rejected: it flatters stability); M1.8e's 15% bullet is recorded not met for the
  body alone, the gap left visible.
- ADR-037: Jorgensen's body lift at every speed, his two `η`s blended (a judgement), sampled at Fig.
  6's points; W&P's measured boattail; old model kept selectable; M1.8a's new miss recorded.
- ADR-036: the Arcas Robin judged at the tunnel's angles; e6 retitled to crossflow and the boattail.
- ADR-033/034: TN 3527's method (ten-element tangent body, `η < 0` reduced, Fig. 2 held below Mach
  3), tabulated every 0.05 Mach lazily, joined over Mach 1.2 to 1.5; boattails since M1.8e4.
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
- Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke have no clear terms: never
  redistribute.
- Aero (M1.5a) is small-angle only; the Recruiter's six fins miss the printed slope by +3.42%
  (ADR-008). Body lift (Jorgensen, M1.8e6) reads 1–16% high where the crossflow is supersonic and
  leaves out the drop past the critical Reynolds number. The normal force misses the wind tunnel
  between Mach 0.8 and 1.2; fins off, the body reads 14–38% high from Mach 1.5 to 2.96 (M1.8e9's
  bullet). A blunt tip's cap (M1.8e7) is checked only on a sphere-cone and keeps its start cone's
  entropy however small it is (#101). A lip's share (M1.8e8) is bounded, not measured; a boattail
  past 16° (M1.8e9) is worth 0.67 to 1.35 calibres of doubt, nothing measuring it.
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic (ADR-030); against
  MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and high through Mach 1 (#67, #68). Against
  the Arcas Robin it reads high at every row (#70 blunt fin edges, #72 a steep boattail in a thick
  boundary layer, #73 the subsonic boattail rule). A cylinder's base drag is unmeasured past Mach
  0.3.
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
