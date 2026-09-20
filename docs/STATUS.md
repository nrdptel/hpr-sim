# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e17 The flare through the method
- **Order:** M1.8e17, then M1.8e18, then M1.8e15, then M3.1; M1.8e16 (the handover past 24°)
  waits on #108
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1, M1.8a to M1.8e14 shipped; https://nrdptel.github.io/hpr-sim/
- **Neer, 2026-09-20:** Debrief is sunset; its use case — a universal flight log analyzer, usable
  **on its own** — is part of this project now (ADR-046, V21). Phase 5 re-cut; M1.8e17 still next.
- **Last updated:** 2026-09-20 (Debrief folded in, ADR-046; M1.8e17 is next)

## Handoff (overwrite each session)

- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest,
  labels as links to their rows, none in headings, Unicode equations; new pages in `SUMMARY.md`; a
  new library needs a row in `docs/api.md`. *Accuracy*'s numbers live in a file the item links, its
  tables hold every report row, and each aero fixture has a `the_guide_quotes_the_fixture`.
  Checking a milestone off fails `cargo xtask site` until its row in `decisions-and-roadmap.md`
  says `done`; a new milestone needs a row, and an id carries one increment level (so e14's
  siblings are e17 and e18, not e14a).
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024). The wind oracle flies RocketPy 1.13.0 with #1188 and #1196 applied
  by `corrections.py`, ADR-026 (re-pin and delete when #1196 releases).
- **M1.8a to e8** (ADR-027 to ADR-039): `cargo xtask aero` writes the aero fixtures (five of
  ADR-030's PDFs come from NTRS with a 436-byte header, scratch `refs/scratch/m18*/`);
  `SupersonicBody` tabulates the method's shares every 0.05 Mach lazily from max(1.2, its bisected
  start) over 0.3; body lift is Jorgensen's, a boattail W&P's increment, a vertical tip TN
  D-4865's cap (`BEFORE_M1_8E6` keeps the old rules).
- **M1.8e9 to e12** (ADR-040 to ADR-043): #90's cap holds W&P's correlation at 16°, M1.8e's 15%
  bullet (`arcas-robin-body-gap.json`) outside on six rows; #87's switches flip one gate
  (`supersonic_run`), so the lip's **rise** is a weight (`shape_weight`); `CONE_SLOPES` runs to 30°
  (Fig. 2's chart to 24°, then SP-3007 Table 2); the handover's cap is a parameter
  (`with_handover_cap_rad`), 24° stands.
- **M1.8e13** (ADR-044): what an answer follows when it follows the mesh is the surface pressure
  **crossing** its tangent cone's (`tangent_cone_crossings`), not `η < 0`; a flag, not a verdict,
  since zero at a coarse mesh only means "not proven". #108 is re-scoped to the loading through a
  crossing; the rest of the old e13 is **M1.8e16**, last because blocked.
- **M1.8e14** (ADR-045): the old e14 split into three; **M1.8e17 (the flare through the method)
  then M1.8e18 (the readings and the guide) are next**. The march's edge is the corner's isentropic
  turn running out, not the shock detaching, landing either side of a wedge's limit depending on
  the tube ahead of the flare — so **e17 must choose an attachment test**
  rather than read one off the march's refusal, and the wedge's limit is a conservative stand-in,
  not the flare's own boundary. #97: the long model's M1.8a readings may be biased.
- **Debrief, folded in** (ADR-046): `hpr-flightdata` is off `hpr-sim` and must stay off it
  (`forbids = ["hpr-sim"]`, walked transitively by `cargo xtask wasm-check`); sim-versus-flight
  work goes in the new `hpr-forensics`. Written up in `docs/research/debrief-log-formats.md` and
  `debrief-flight-readings.md`; both repos are in the refs lock. **Port from Debrief's `lib/`,
  never from its `COMPETITION.md`** (OpenRocket rows read out of GPL-3 Java). Its 12 public
  fixtures may be used; `refs/debrief-fixtures` may not.
- **M2.2's OpenRocket oracle** (ADR-035): orhelper is dropped, so decide how to drive the jar when
  M2.2 starts; JPype loads the JVM in-process, only a subprocess isolates, and the jar needs Java
  17 exactly (`[java] max_major` in the refs lock keeps doctor off a newer one).
- **Regeneration is not bit-identical across machines** (last digits): regenerate with `cargo xtask
  validate` (debug), never `--release`; fixture checks allow 1e-12 relative, and `handover_caps` is
  stored to six decimals since a reduced march drifts 2.4e-12.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock
  (STATUS 150 lines, ROADMAP 1000: trim an old entry when adding one); oracles run from the repo
  root with `refs/venv/bin/python`; `cargo xtask designs` and `examples` rewrite their outputs.

## Done log (newest first, keep about 15)

- 2026-09-20: Debrief folded in (ADR-046): a flight log analyzer that stands without the simulator
  — `hpr-flightdata` re-layered, `hpr-forensics` added, Phase 5 re-cut, both repos mirrored and
  written up in two research notes. No physics moved.
- 2026-09-20: M1.8e14 Where the flare's march stops (ADR-045): the corner's isentropic turn running
  out, not the shock detaching; which side of a wedge's limit it lands on is the tube's doing. The
  rest of the old e14 is M1.8e17 and M1.8e18, next up.
- 2026-09-20: M1.8e13 What the answer follows when it follows the mesh (ADR-044): a crossing of the
  tangent cone, not a reduced element; across three meshes the 27 readings without one hold to
  0.012 per radian and the 5 with one move 0.035 or more, no overlap. No fixture number moved.
- 2026-09-20: M1.8e12 What the handover's cap is worth (ADR-043): the cap is a method parameter,
  swept into the fixture and the guide; 24° stays because no steeper cap keeps its answer to Mach 5
  (#108); no fixture moved.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and three
  `validate (...)` checks; block force pushes. Don't require approvals: the autopilot merges its own
  PRs as you, and authors can't self-approve.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`... are unreserved. Reserve them?
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers from its secant
  columns; `rocketpy-drag-curves.json` enough to rebuild 147 values of five RocketPy drag curves
  (ADR-029). If not fine, open an issue; the next session summarises.

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-046: Debrief folded in; `hpr-flightdata` re-layered off `hpr-sim` while still a stub, with a
  `forbids` rule and a graph-walking check; `hpr-forensics` added; Phase 5 re-cut (M7.1/M7.2 need
  no design or simulator, M7.3 takes the residuals); `hpr analyze` added to M4.2. Debrief's `.ork`
  parser is clean room from the published page; its `COMPETITION.md` is not, and is excluded.
- M1.8e splits (one id level, so siblings are renumbered): e3 join, e4 boattail, e5 measure, e6 fly,
  e7 blunt tips, e8 lip, e9 rest, e10 the lip's weight, e11 Fig. 2's edge, e12 step and flare; e9's
  done-when restates the parent bullet verbatim, escape clause included.
- ADR-038 to ADR-040: the march behind a blunt tip starts from the tangent cone, not TN D-4865's
  Newtonian state (which fails on the Arcas nose from Mach 3.96), handover capped at 24°; a lip in
  a boattail's wake carries nothing faster than sound, bounded by the fins-off moment, not
  measured; a boattail past 16° reads its correlation as a 16° one (a fade to zero was rejected as
  flattering stability), and M1.8e's 15% bullet is recorded **not met** for the body alone.
- ADR-027 to ADR-037 (details in `DECISIONS.md`): M1.8 split a to e; fins' supersonic slope counts
  both faces and the transonic join is not fitted to the tunnel; Stoney's Fig. 12 read by hand,
  bulged ogives and Haack past `C = ⅓` refused; Fig. 5-122 to the Prandtl–Meyer limit, 16°–30°
  separation, a step down sheltering a lip, targets not tuned; a table replaces only the static
  force; roll damping takes the fin's own slope; TN 3527's method tabulated every 0.05 Mach lazily
  and joined over Mach 1.2 to 1.5; Jorgensen's body lift with his two `η`s blended (a judgement),
  W&P's measured boattail, the old model still selectable. **Two gaps left visible rather than
  chased:** M1.8's drag bullet (ADR-029) and M1.8a's new miss (ADR-037), both recorded not met.
- ADR-001 to ADR-026 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
  nose; Niskanen's drag as printed at 20 µm; own DOPRI5; recovery in `hpr-sim`; the site's link,
  label and number checks; references read, never written; 3% gates or a written reason. #11:
  `SolidMotor` refuses `c = I/m_p` outside 200–5,000 m/s. M2.1b1: same-drag cases declare `C_D0(M)`.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C, WMO-No. 8). Dryden turbulence
  is an aircraft model, unvalidated for rockets, and no flight uses it (#39).
- Only 32 curves are bundled (none in class A); the rest wait for M5's cache, whose checks ran on
  unpinned `refs/samples/` caches. Wall and fin mass may differ from OpenRocket's undocumented
  conventions (M2.2 measures it); `.CDX1` has no public spec, and ERA5 `.nc` may be netCDF4.
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason. Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke have no clear terms: never
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
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`. Recovery:
  no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a).
  Streamer pleats are not modelled (+58% fast on Kidwell's pleated streamer); tumble +19% (M1.7b).
- Results are not bit-identical across macOS, Windows and Linux: the report is pinned to six
  decimals, or 1e-7 relative for whole flights (M2.1a, M2.1b2). `refs doctor` "runnable" means the
  oracle's runtime starts; no oracle runs in CI (M2.1b).
