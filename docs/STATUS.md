# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e18 What a marched flare is worth
- **Order:** M1.8e18, M1.8e15, M1.8e19, M3.1; M1.8e16 (the handover past 24°) waits on #108
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1, M1.8a to M1.8e17 shipped; https://nrdptel.github.io/hpr-sim/
- **Neer, 2026-09-20:** Debrief is sunset; its use case — a universal flight log analyzer, usable
  **on its own** — is part of this project (ADR-046, V21). Phase 5 re-cut.
- **Last updated:** 2026-09-20 (M1.8e17 shipped, ADR-047; M1.8e18 is next)

## Handoff (overwrite each session)

- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest,
  labels as links to their rows, none in headings, Unicode equations; new pages in `SUMMARY.md`; a
  new library needs a row in `docs/api.md`. *Accuracy*'s numbers live in a file the item links and
  its tables hold every report row; each aero fixture has a `the_guide_quotes_the_fixture`. Checking
  a milestone off fails `cargo xtask site` until its row in `decisions-and-roadmap.md` says `done`;
  a new one needs a row, and an id carries one increment level (e14's siblings are e17 to e19).
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's 3%
  are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024). The wind oracle flies RocketPy 1.13.0 with #1188 and #1196 by
  `corrections.py`, ADR-026 (re-pin and delete when #1196 releases).
- **M1.8a to e8** (ADR-027 to ADR-039): `cargo xtask aero` writes the aero fixtures (five of
  ADR-030's PDFs come from NTRS with a 436-byte header, scratch `refs/scratch/m18*/`);
  `SupersonicBody` tabulates the shares every 0.05 Mach lazily from max(1.2, its bisected start)
  over 0.3; `BEFORE_M1_8E6` keeps the old rules.
- **M1.8e9 to e13** (ADR-040 to ADR-044): #90's cap holds W&P's correlation at 16°, M1.8e's 15%
  bullet (`arcas-robin-body-gap.json`) outside on six rows; #87's switches flip one gate
  (`supersonic_run`), so the lip's **rise** is a weight (`shape_weight`); `CONE_SLOPES` runs to 30°
  (Fig. 2 to 24°, then SP-3007 Table 2); the handover's cap is a parameter
  (`with_handover_cap_rad`), 24° stands; a mesh-following answer is marked by the pressure
  **crossing** its tangent cone's (`tangent_cone_crossings`), not `η < 0` — a flag, not a verdict.
  #108 is re-scoped to the loading through a crossing; the rest of e13 is **M1.8e16**, blocked.
- **M1.8e14 and e17** (ADR-045, ADR-047): the march's edge is the corner's isentropic turn, not the
  shock detaching, so e17 **chose** the attachment test — NACA 1135's wedge limit at `aft_flow`'s
  surface Mach (TN D-4865 p. 5's, as the blunt tip's cap uses) under the cone tables' 30°, binding
  from Mach 2.5192. A steeper flare reads one of the **same radii drawn out** to that turn, CP
  mapped back on the real flare, so the branches meet at the limit (4.527e-11 per 1e-9°). Only a
  conical flare not behind a boattail joins the run, which ends at it;
  `SupersonicFlare::SlenderBody` keeps the old rule. **Left visible:** a 0.0382°-0.0588° band the
  march refuses, −8.3% and 1.16 calibres (#117, M1.8e19). **M1.8e18 is next:** TN D-4865 model 2's
  fig. 8 readings, nothing having been compared with a measured flare yet. #97: the long model's
  M1.8a readings may be biased.
- **Debrief, folded in** (ADR-046): `hpr-flightdata` is off `hpr-sim` and must stay off it (`forbids
  = ["hpr-sim"]`, walked transitively by `cargo xtask wasm-check`); sim-versus-flight work goes in
  the new `hpr-forensics`. Three research notes: `debrief-log-formats.md`,
  `debrief-flight-readings.md`, `debrief-porting-boundary.md`. **Port from Debrief's `lib/`, never
  from its `COMPETITION.md`** (OpenRocket rows read out of GPL-3 Java). Its 12 public fixtures may
  be used; `refs/debrief-fixtures` may not.
- **M2.2's OpenRocket oracle** (ADR-035): orhelper is dropped, so decide how to drive the jar when
  M2.2 starts; JPype loads the JVM in-process, only a subprocess isolates, Java 17 only
  (`[java] max_major` in the refs lock keeps doctor off a newer one).
- **Regeneration is not bit-identical across machines** (last digits): regenerate with `cargo xtask
  validate` (debug), never `--release`; fixture checks allow 1e-12 relative, `handover_caps` six
  decimals (a reduced march drifts 2.4e-12).
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock
  (STATUS 150 lines, ROADMAP 1000: trim an old entry when adding one); oracles run from the repo
  root with `refs/venv/bin/python`; `xtask designs` and `examples` rewrite their outputs.

## Done log (newest first, keep about 15)

- 2026-09-20: M1.8e17 The flare through the method (ADR-047): a conical flare flies the method while
  its corner's shock is attached and reads as one of the same radii drawn out where it is not, so
  nothing jumps across that boundary (4.527e-11 per ±1e-9°, 1.235e-9 per ±1e-9 in Mach). A flared
  rocket reads 5-6% below slender-body theory, unchecked against a measured flare (M1.8e18).
- 2026-09-20: Debrief folded in (ADR-046): a flight log analyzer that stands without the simulator —
  `hpr-flightdata` re-layered, `hpr-forensics` added, Phase 5 re-cut, both repos mirrored and
  written up. No physics moved.
- 2026-09-20: M1.8e14 Where the flare's march stops (ADR-045): the corner's isentropic turn running
  out, not the shock detaching; which side of a wedge's limit it lands on is the tube's doing. The
  rest of the old e14 is M1.8e17 and M1.8e18, next up.
- 2026-09-20: M1.8e13 What the answer follows when it follows the mesh (ADR-044): a crossing of the
  tangent cone, not a reduced element; across three meshes the 27 readings without one hold to 0.012
  per radian and the 5 with one move 0.035 or more. No fixture number moved.

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and three
  `validate (...)` checks; block force pushes. Don't require approvals: the autopilot merges its own
  PRs as you, and authors can't self-approve.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`… are unreserved. Reserve them?
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers from its secant
  columns; `rocketpy-drag-curves.json` enough to rebuild 147 values of five RocketPy drag curves
  (ADR-029). If not, open an issue; the next session summarises.

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-047: a flare's attachment test is NACA 1135's wedge limit at the flow reaching the corner (TN
  D-4865 p. 5's, already the blunt tip's) under the cone tables' 30°; a steeper flare reads one of
  the same radii drawn out to it (ADR-039's boattail rule turned around); the run ends at the flare,
  and only a conical one not behind a boattail joins it; the near-flat band the march refuses is
  left visible as #117 and M1.8e19, not folded into #87.
- ADR-046: Debrief folded in; `hpr-flightdata` re-layered off `hpr-sim` while still a stub, with a
  `forbids` rule and a graph-walking check; `hpr-forensics` added; Phase 5 re-cut (M7.1/M7.2 need no
  design or simulator, M7.3 takes the residuals); `hpr analyze` added to M4.2. Debrief's `.ork`
  parser is clean room from the published page; its `COMPETITION.md` is not, and is excluded.
- ADR-038 to ADR-040: the march behind a blunt tip starts from the tangent cone, not TN D-4865's
  Newtonian state (which fails on the Arcas nose from Mach 3.96), handover capped at 24°; a lip in a
  boattail's wake carries nothing faster than sound, bounded not measured; a boattail past 16° reads
  its correlation as a 16° one (a fade to zero was rejected as flattering stability), and M1.8e's
  15% bullet is **not met** for the body alone.
- ADR-027 to ADR-037 (details in `DECISIONS.md`): M1.8 split a to e, one id level per split; fins'
  supersonic slope counts both faces, the transonic join not fitted to the tunnel; Stoney's Fig. 12
  read by hand, bulged ogives and Haack past `C = ⅓` refused; Fig. 5-122 to the Prandtl–Meyer limit,
  16°–30° separation, a step down sheltering a lip, targets not tuned; a table replaces only the
  static force; roll damping takes the fin's own slope; Jorgensen's body lift with his two `η`s
  blended (a judgement), W&P's measured boattail, the old model still selectable. **Two gaps left
  visible:** M1.8's drag bullet (ADR-029) and M1.8a's miss (ADR-037), both recorded not met.
- ADR-001 to ADR-026 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
  nose; Niskanen's drag as printed at 20 µm; own DOPRI5; recovery in `hpr-sim`; the site's link,
  label and number checks; references read, never written; 3% gates or a written reason. #11:
  `SolidMotor` refuses `c = I/m_p` outside 200–5,000 m/s. M2.1b1: same-drag cases declare `C_D0(M)`.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C, WMO-No. 8). Dryden turbulence
  is an aircraft model, unvalidated for rockets, and no flight uses it (#39). Only 32 motor curves
  are bundled (none in class A); the rest wait for M5's cache, whose checks ran on unpinned
  `refs/samples/` caches. Wall and fin mass may differ from OpenRocket's undocumented conventions
  (M2.2 measures it); `.CDX1` has no public spec, and ERA5 `.nc` may be netCDF4.
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason. Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke: never redistribute.
- Aero (M1.5a) is small-angle only; the Recruiter's six fins miss the printed slope by +3.42%
  (ADR-008). Body lift (Jorgensen, M1.8e6) reads 1–16% high where the crossflow is supersonic and
  leaves out the drop past the critical Reynolds number. The normal force misses the wind tunnel
  between Mach 0.8 and 1.2; fins off, the body reads 14–38% high from Mach 1.5 to 2.96 (M1.8e9's
  bullet). A blunt tip's cap (M1.8e7) is checked only on a sphere-cone and keeps its start cone's
  entropy (#101). A lip's share (M1.8e8) is bounded, not measured; a boattail past 16° (M1.8e9) is
  worth 0.67 to 1.35 calibres of doubt. **A marched flare (M1.8e17) is compared with nothing until
  M1.8e18**, and a 0.0382°-0.0588° flare switches the whole body off the method (#117).
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic (ADR-030); against
  MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and high through Mach 1 (#67, #68). Against
  the Arcas Robin it reads high at every row (#70 blunt fin edges, #72 a steep boattail in a thick
  boundary layer, #73 the subsonic boattail rule). A cylinder's base drag is unmeasured past Mach
  0.3. In wind, a slow rocket's drift rests on body lift's size: Juno III's apogee drift is 245 m in
  hpr (Jorgensen's), 240 to 194 m over Galejs's `K` 1.0 to 1.5. The oracle carries two unreleased
  RocketPy corrections; if #1196 changes, revisit `corrections.py`.
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`. Recovery: no
  canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where Knacke's
  infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the attitude
  freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a). Streamer
  pleats are not modelled (+58% fast on Kidwell's pleated streamer); tumble +19% (M1.7b). Results
  are not bit-identical across macOS, Windows and Linux: the report is pinned to six decimals, or
  1e-7 relative for whole flights (M2.1a, M2.1b2). `refs doctor` "runnable" means the oracle's
  runtime starts; no oracle runs in CI (M2.1b).
