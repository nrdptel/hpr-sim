# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e, all done bar M1.8e16 (`[blocked]` on #108); the work is
  **M3.1b2**, the `.ork` spine, in progress on `m3.1b2-ork-spine` (draft PR): the reader and its
  tests are in, the corpus run is not. M3.1b was split further: M3.1b2 is the spine, M3.1b3 what hangs off it.
- **Order:** finish M3.1b2, then M3.1b3, M3.1c, M3.1d, then M2.2, then M1.9; M1.8e16 (past 24°)
  waits on #108
- **Run:** M0.1-M0.4, M1.1-M1.7, M2.1, M1.8a-e19 bar e16, M3.1a, M3.1b1; the site is published.
- **Neer, 2026-09-20:** Debrief is sunset; a flight log analyzer usable **on its own** is part of
  this project (ADR-046, V21). Phase 5 re-cut.
- **Last updated:** 2026-09-20 (M3.1b2 split; M3.1b2a part-built on its branch)

## Handoff (overwrite each session)

- **Resume M3.1b2 here.** On `m3.1b2-ork-spine`: `hpr_io::ork::component::rocket` reads a
  document's stages and body components, and four tests pass (`cargo test -p hpr-io --lib`),
  including L59's `auto_fore_radius_resolves_across_stage_boundary`. **What is left:** wire it into
  `cargo xtask ork` — build a `Rocket` per corpus design, call `layout()`, print how many lay out
  and a tally of the tags left off the spine, per-file detail to `corpus-out/` — then the guide's
  `docs/format/ork.md` section, an ADR for the two decisions below, the reviewers and the gate.
  Two decisions to check against the corpus run: a `<bodytube>` that is `filled` with an automatic
  radius (warns, reads the cached radius), and `shapeclipped` defaulting to clipped on a transition
  (1 of 21 state it; the M2.2 oracle settles it).
- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest
  (rustdoc too), labels as links to their rows — a lesson a page names needs a row in
  `decisions-and-roadmap.md` — none in headings; new pages in `SUMMARY.md`, a new library a row in
  `docs/api.md`, a milestone's `decisions-and-roadmap.md` row saying `done` to match. `STATUS.md`
  holds 150 lines and `ROADMAP.md` 1000: reflow a long-met `*Done when:*` list into prose to fit.
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024). The wind oracle flies RocketPy 1.13.0 with #1188 and #1196 by
  `corrections.py` (re-pin and delete when #1196 releases). **Regeneration is not bit-identical**:
  use `cargo xtask validate` (debug), never `--release`; checks allow 1e-12 relative but **compare
  a fixture's strings as text**. `cargo xtask aero` rewrites `arcas-robin-gap.json`'s last digits
  whatever you changed — `git checkout` it.
- **M1.8a to e19** (ADR-027 to ADR-050). The measurements are in the guide's aero page and in
  *Known issues*; the working notes: `cargo xtask aero` writes the aero fixtures (five of ADR-030's
  PDFs come from NTRS with a 436-byte header, scratch `refs/scratch/m18*/`); `SupersonicBody`
  tabulates every 0.05 Mach from max(1.2, its start), joined over 0.3; `BEFORE_M1_8E6` keeps the
  old rules and `CONE_SLOPES` runs to 30°; a mesh-following answer is marked by the pressure
  **crossing** its tangent cone's, not `η < 0`; the near-flat flare's edges come from
  `flare_reduction_turns_rad`, leaving the crossing's pole (#108); a step in radius still takes the
  body off the method (#87, #120, #121), and a boattail's or cylinder's reduced element still
  refuses (#123), keeping ADR-038's rows in `blunt-tips.json`.
- **Debrief, folded in** (ADR-046): `hpr-flightdata` is off `hpr-sim` and must stay off it
  (`forbids = ["hpr-sim"]`, walked by `cargo xtask wasm-check`); sim-versus-flight goes in
  `hpr-forensics`; notes in `debrief-{log-formats,flight-readings,porting-boundary}.md`. **Port
  from its `lib/`, never its `COMPETITION.md`** (GPL-3 Java); its 12 public fixtures may be used.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock;
  oracles run from the repo root with `refs/venv/bin/python`; `xtask designs`, `examples` and `ork`
  rewrite their outputs. **M2.2's oracle** (ADR-035): orhelper is dropped, so decide how to drive
  the jar — JPype loads the JVM in-process, only a subprocess isolates, Java 17 only.

## Done log (newest first, keep about 15)

- 2026-09-20: M3.1b1 The values inside a `.ork`'s tags (ADR-052): an automatic dimension keeps its
  flag and its cached number (413 in the corpus, 309 caching nothing); either name of a rename may
  be read where the corpus shows both agree on the number *and* the frame (642 and 109 elements),
  and two lookalike pairs are left unread because it shows they do not; a stated `0` is a value,
  and the six override tags are read independently. L58, L62 and L63's named tests are live.
## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require `fmt`,
  `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and `validate (...)`
  checks; block force pushes. Don't require approvals (authors can't self-approve).
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`… unreserved. Reserve them?
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers, and
  `rocketpy-drag-curves.json` enough to rebuild 147 values of five curves (ADR-029).

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-052: M3.1b splits in two, the values first; an automatic dimension keeps both halves; either
  name of a rename may be read where the corpus shows the two agree on the number *and* the frame,
  and the two lookalike pairs are left unread because it shows they do not; the pre-1.9
  subcomponent-override flag sets all three, with a warning.
- ADR-051: M3.1 is split a to d, the container first; a `.ork` document is kept whole and read by
  nobody, because it has no schema; the writer is canonical, so read-write-read is the guarantee
  rather than byte fidelity; nesting is capped at 64 before parsing; `zip` and `flate2` added.
- ADR-050: a reduced element takes the generalized method wherever it has a tangent cone of its own;
  a cylinder's and a boattail's keep the refusal (#123). Edges from the corner.
- ADR-049: a step in radius keeps the model it has (no source gives a step's normal force faster
  than sound); its size is published instead. #87 narrowed to the step, #120 and #121 carry the rest.
- ADR-048: model 2's drawing is closed on the base, not on its printed lengths, and the spread
  published; a blunt nose may span more than one curved segment but a cap may not reach a cylinder;
  the Mach 1.50 refusal stands; the three separated rows are a flow hpr doesn't model.
- ADR-047: a flare's attachment test is NACA 1135's wedge limit at the flow reaching the corner (TN
  D-4865 p. 5's) under the cone tables' 30°; a steeper flare reads the same radii drawn out to it;
  the run ends at the flare, and only a conical one not behind a boattail joins it.
- ADR-046: Debrief folded in; `hpr-flightdata` re-layered off `hpr-sim`, `hpr-forensics` added,
  Phase 5 re-cut, `hpr analyze` added to M4.2. Its `.ork` parser is clean room, `COMPETITION.md`
  is not.
- ADR-038 to ADR-040: the march behind a blunt tip starts from the tangent cone, not TN D-4865's
  Newtonian state (which fails on the Arcas nose from Mach 3.96), handover capped at 24°; a lip in a
  boattail's wake carries nothing faster than sound, bounded not measured; a boattail past 16° reads
  its correlation as a 16° one, and M1.8e's 15% bullet is **not met** for the body alone.
- ADR-027 to ADR-037 (details in `DECISIONS.md`): M1.8 split a to e; fins' supersonic slope counts
  both faces, the transonic join not fitted to the tunnel; Stoney's Fig. 12 read by hand, bulged
  ogives and Haack past `C = ⅓` refused; Fig. 5-122 to the Prandtl–Meyer limit, 16°–30° separation,
  a step down sheltering a lip, targets not tuned; a table replaces only the static force; roll
  damping takes the fin's own slope. **Two gaps visible:** M1.8's drag bullet (ADR-029) and M1.8a's
  miss (ADR-037), both not met.
- ADR-001 to ADR-026 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
  nose; Niskanen's drag as printed at 20 µm; own DOPRI5; recovery in `hpr-sim`; the site's link,
  label and number checks; references read, never written; 3% gates or a written reason. #11:
  `SolidMotor` refuses `c = I/m_p` outside 200–5,000 m/s. M2.1b1: same-drag cases declare `C_D0(M)`.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C, WMO-No. 8). Dryden turbulence
  is an aircraft model, unvalidated for rockets, and no flight uses it (#39). Only 32 motor curves
  are bundled (none in class A); the rest wait for M5's cache, whose checks ran on unpinned
  `refs/samples/`. Wall and fin mass may differ from OpenRocket's (M2.2); `.CDX1` has no public
  spec, ERA5 `.nc` may be netCDF4. A new RustSec notice can turn CI red with no code change:
  upgrade, replace, or `ignore` with a reason. Barrowman 1966, TIR-33, Galejs, the `.rse` spec and
  Knacke: never redistribute.
- Aero (M1.5a) is small-angle only; the Recruiter's six fins miss the printed slope by +3.42%
  (ADR-008). Body lift (Jorgensen, M1.8e6) reads 1–16% high where the crossflow is supersonic. The
  normal force misses the wind tunnel between Mach 0.8 and 1.2; fins off, the body reads 14–38% high
  from Mach 1.5 to 2.96 (M1.8e9's bullet). A blunt tip's cap (M1.8e7) is checked only on a
  sphere-cone (#101); a lip's share (M1.8e8) is bounded, not measured; a boattail past 16° (M1.8e9)
  is worth 0.67 to 1.35 calibres of doubt. A marched flare (M1.8e18) reads +51.5% and +50.4% at Mach
  3.95 and 4.63 on the one measured flare, separated there, and none below Mach 1.5289; a near-flat
  flare leaves the crossing's pole — +0.129% and 0.0051 calibres on the tests' rocket, +4.3% and
  0.19 on a short shoulder (#108); a step in radius takes the body off the method past 2.7e-11 m
  tube to tube or 1.3e-13 m up at a boattail — −8.65% to −11.34% (#87).
- `.ork` (M3.1a, M3.1b1) reads the container, the document and the values; no rocket is built from
  one yet, and an instanced component's angle and radius wait on M3.1b2 finding a source for the
  frame the older tag names (ADR-052).
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic (ADR-030); against
  MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and high through Mach 1 (#67, #68); against
  the Arcas Robin it reads high at every row (#70, #72, #73). A cylinder's base drag is unmeasured
  past Mach 0.3. In wind, a slow rocket's drift rests on body lift's size: Juno III's apogee drift
  is 245 m in hpr, 240 to 194 m over Galejs's `K` 1.0 to 1.5 (oracle corrections, #1196).
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`. Recovery
  (M1.7a, M1.7b): no canopy overshoot or opening-load factor (1.6 kN where Knacke's infinite-mass
  `C_x` gives 5.1 kN), no added mass or airframe drag, the attitude freezes at deployment, his
  filling time is stated only for 150 to 500 ft/s, streamer pleats are not modelled (+58% on
  Kidwell's) and tumble reads +19%. Results are not bit-identical across the three OSes: the
  report is pinned to six decimals, or 1e-7 relative for whole flights; no oracle runs in CI.
