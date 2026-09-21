# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e, all done bar M1.8e16 (`[blocked]` on #108); the work is **M3.1b4**,
  the three `.ork` designs of the 76 whose rocket still does not lay out, now M3.1b3 reads the
  parts (M3.1b split again: b2 the spine, b3 what hangs off it, b4 the three left).
- **Order:** M3.1b4, M3.1c, M3.1d, M2.2, M1.9; M1.8e16 waits on #108. **Run:** M0.1-M0.4,
  M1.1-M1.7, M2.1, M1.8a-e19 bar e16, M3.1a, M3.1b1-b3; the site is published.
- **Neer, 2026-09-20:** Debrief is sunset; a flight log analyzer usable **on its own** is part of
  this project (ADR-046, V21); Phase 5 re-cut.
- **Last updated:** 2026-09-20 (M3.1b3 shipped: the `.ork` parts on and inside the body)

## Handoff (overwrite each session)

- **M3.1b4 next:** 73 of 76 designs lay out; the 3 left are public files. Debrief's
  `sample-design.ork` has a `<rocket>` with no `<subcomponents>` at all, so there is no design in
  it; Loft's `demo-quirks.ork` chains nose → tube → transition all `auto` with only the aft end
  fixed (the nose caches 0.033); `openrocket-database/ork/parachutes.ork` has four tubes all bare
  `auto`. Only a rule for an unresolvable chain reaches the second and nothing reaches the other
  two — decide and publish, don't reach for the cache by default. **Then M3.1c:** motors, recovery
  settings, pods and parallel stages (9 `podset` and 3 `parallelstage` hold 19 parts nobody reads).
- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest
  (rustdoc too), labels as links to their rows — a lesson a page names needs a row in
  `decisions-and-roadmap.md` — none in headings; new pages in `SUMMARY.md`, a new library a row in
  `docs/api.md`, a milestone's row saying `done` to match. `STATUS.md` holds 150 lines and
  `ROADMAP.md` 1000: reflow a long-met `*Done when:*` list into prose, keeping its lesson bullet.
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024). The wind oracle flies RocketPy 1.13.0 with #1188 and #1196 by
  `corrections.py` (re-pin and delete when #1196 releases). **Regeneration is not bit-identical**:
  use `cargo xtask validate` (debug), never `--release`; checks allow 1e-12 relative but **compare
  a fixture's strings as text**. `cargo xtask aero` rewrites `arcas-robin-gap.json`'s last digits
  whatever you changed — `git checkout` it.
- **M1.8a to e19** (ADR-027 to ADR-050). Measurements: the guide's aero page and *Known issues*.
  Working notes: `cargo xtask aero` writes the aero fixtures (five of ADR-030's PDFs come from NTRS
  with a 436-byte header, scratch `refs/scratch/m18*/`); `SupersonicBody` tabulates every 0.05 Mach
  from max(1.2, its start), joined over 0.3; `BEFORE_M1_8E6` keeps the old rules and `CONE_SLOPES`
  runs to 30°; a mesh-following answer is marked by the pressure **crossing** its tangent cone's,
  not `η < 0`; the near-flat flare's edges come from `flare_reduction_turns_rad` (#108).
- **Debrief, folded in** (ADR-046): `hpr-flightdata` is off `hpr-sim` and must stay off it
  (`forbids = ["hpr-sim"]`, walked by `cargo xtask wasm-check`); sim-versus-flight goes in
  `hpr-forensics`; notes in `debrief-{log-formats,flight-readings,porting-boundary}.md`. **Port
  from its `lib/`, never its `COMPETITION.md`** (GPL-3 Java); its 12 public fixtures may be used.
- **`.ork` parts (M3.1b3, ADR-053):** angles in a `.ork` are **degrees**; `radialposition` is on
  the parts inside a body and `radiusoffset` on the parts on it, never both, which is what let
  ADR-052's pair be read at last. `cargo xtask ork` now runs an oracle needing no OpenRocket — a
  cached `auto` is OpenRocket's own answer — and reports 67 of 71 agreeing. The finish heights come
  from the author's forum post, cached under `refs/sources/openrocket-finish/`.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock;
  oracles run from the repo root with `refs/venv/bin/python`; `xtask designs`, `examples` and `ork`
  rewrite their outputs. **M2.2's oracle** (ADR-035): orhelper is dropped, so decide how to drive
  the jar — JPype loads the JVM in-process, only a subprocess isolates, Java 17 only.

## Done log (newest first, keep about 15)

- 2026-09-20: M3.1b3 The `.ork` parts on and inside the body (ADR-053): 765 parts over the 73
  designs that lay out, against 285 body components, 327 automatic dimensions marked, 5 left out
  with a reason. Angles are degrees (86 of the corpus's exceed 2π); the five finish words are
  sourced (500/150/60/20/2 µm). New in-file oracle: hpr's layout matches 67 of the 71 automatic
  dimensions OpenRocket cached an answer for, the 4 apart being one design's stale cache. L49, L60
  and L61 live. #130 to #132 fixed bar a flipped nose and a parameter range; #133 filed.
- 2026-09-20: M3.1b2 The `.ork` spine into `hpr_design` types, every automatic radius marked for
  `layout()` to resolve, across a stage boundary too (L59 live). OpenRocket's ogive κ is the
  reciprocal of this project's radius ratio (Niskanen A.3). Open for M2.2: a shoulder of no wall
  reads as solid, an unstated `shapeclipped` as clipped.
- 2026-09-20: M3.1b1 What a `.ork` value means (ADR-052): an automatic dimension keeps both its
  flag and its cached number (413, 309 caching nothing); either name of a rename may be read where
  the corpus shows both agree on the number *and* the frame; a stated `0` is a value; L58, L62 and
  L63 live.
## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require `fmt`,
  `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and `validate (...)`
  checks; block force pushes. Don't require approvals (authors can't self-approve).
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`… unreserved. Reserve them?
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers, and
  `rocketpy-drag-curves.json` enough to rebuild 147 values of five curves (ADR-029).

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-053: `.ork` angles are degrees; `radialposition` and `radiusoffset` are read on the parts
  that carry them, so ADR-052's pair needs no source; a part that cannot be shaped honestly is left
  out with its reason; a tube of no wall is weightless where a body component's zero is solid; an
  inner tube's automatic radius is its parent's bore; the finish words take published heights.
- ADR-052: M3.1b splits in two, the values first; an automatic dimension keeps both halves; either
  name of a rename may be read where the corpus shows the two agree on the number *and* the frame;
  the pre-1.9 subcomponent-override flag sets all three, with a warning.
- ADR-051: M3.1 is split a to d, the container first; a `.ork` document is kept whole because it
  has no schema, and read-write-read is the guarantee rather than byte fidelity; nesting is capped
  at 64 before parsing; `zip` and `flate2` added.
- ADR-050: a reduced element takes the generalized method wherever it has a tangent cone of its
  own; a cylinder's and a boattail's keep the refusal (#123). Edges from the corner.
- ADR-049: a step in radius keeps the model it has (no source gives a step's normal force faster
  than sound); its size is published. #87 narrowed to it, #120 and #121 carry the rest.
- ADR-048: model 2's drawing is closed on the base, not on its printed lengths, and the spread
  published; a blunt nose may span more than one curved segment but a cap may not reach a cylinder;
  the Mach 1.50 refusal stands; the three separated rows are a flow hpr doesn't model.
- ADR-047: a flare's attachment test is NACA 1135's wedge limit at the flow reaching the corner (TN
  D-4865 p. 5's) under the cone tables' 30°; a steeper flare reads the same radii drawn out to it.
- ADR-046: Debrief folded in; `hpr-flightdata` off `hpr-sim`, `hpr-forensics` added, Phase 5
  re-cut, `hpr analyze` in M4.2. Its `.ork` parser is clean room, `COMPETITION.md` is not.
- ADR-038 to ADR-040: the march behind a blunt tip starts from the tangent cone, not TN D-4865's
  Newtonian state (which fails on the Arcas nose from Mach 3.96), handover capped at 24°; a lip in
  a boattail's wake is bounded, not measured; M1.8e's 15% bullet is **not met** for the body alone.
- ADR-027 to ADR-037 (details in `DECISIONS.md`): M1.8 split a to e; fins' supersonic slope counts
  both faces; Stoney's Fig. 12 read by hand, bulged ogives and Haack past `C = ⅓` refused; a table
  replaces only the static force. **Two gaps visible:** M1.8's drag bullet, M1.8a's miss.
- ADR-001 to ADR-026 (details in `DECISIONS.md`), among them: refs pinned by hash; body `+z` to the
  nose; Niskanen's drag as printed at 20 µm; own DOPRI5; recovery in `hpr-sim`; the site's link,
  label and number checks; 3% gates or a written reason. #11: `SolidMotor` refuses `c = I/m_p`
  outside 200–5,000 m/s. M2.1b1: same-drag cases declare `C_D0(M)`.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C, WMO-No. 8). Dryden turbulence
  is an aircraft model, unvalidated for rockets, and no flight uses it (#39). Only 32 motor curves
  are bundled (none in class A); the rest wait for M5's cache, whose checks ran on unpinned
  `refs/samples/`. Wall and fin mass may differ from OpenRocket's (M2.2); `.CDX1` has no public
  spec, ERA5 `.nc` may be netCDF4. A new RustSec notice can turn CI red with no code change.
  Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke: never redistribute.
- Aero (M1.5a) is small-angle only; the Recruiter's six fins miss the printed slope by +3.42%
  (ADR-008). Body lift (Jorgensen, M1.8e6) reads 1–16% high where the crossflow is supersonic. The
  normal force misses the tunnel between Mach 0.8 and 1.2; fins off, the body reads 14–38% high
  from Mach 1.5 to 2.96 (M1.8e9's bullet). A blunt tip's cap (M1.8e7) is checked only on a
  sphere-cone (#101); a lip's share (M1.8e8) is bounded, not measured; a boattail past 16° (M1.8e9)
  is worth 0.67 to 1.35 calibres of doubt. A marched flare (M1.8e18) reads +51.5% and +50.4% at
  Mach 3.95 and 4.63 on the one measured flare, separated there, and none below Mach 1.5289; a
  near-flat flare leaves the crossing's pole — +0.129% on the tests' rocket, +4.3% on a short
  shoulder (#108); a step in radius takes the body off the method past 2.7e-11 m tube to tube or
  1.3e-13 m up at a boattail — −8.65% to −11.34% (#87).
- `.ork` (M3.1a to M3.1b3) builds a rocket, but 3 of 76 designs do not lay out (M3.1b4) and no
  motor, recovery setting, pod or parallel stage is read (M3.1c). 5 parts are left out with a
  reason, among them the corpus's only tube fins (#133); fin fillets, a rail button's screw head
  and motor clusters are read as the simpler part, with a warning. `polished` is 2 µm here and may
  be 0.5 µm in a newer OpenRocket; a zero-wall tube is weightless, which M2.2 can settle.
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic (ADR-030); against
  MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and high through Mach 1 (#67, #68); against
  the Arcas Robin it reads high at every row (#70, #72, #73); a cylinder's base drag is unmeasured
  past Mach 0.3. In wind, a slow rocket's drift rests on body lift: Juno III's apogee drift is 245
  m in hpr, 240 to 194 m over Galejs's `K` 1.0 to 1.5 (oracle corrections, #1196).
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`. Recovery
  (M1.7a, M1.7b): no canopy overshoot or opening-load factor (1.6 kN where Knacke's infinite-mass
  `C_x` gives 5.1 kN), no added mass or airframe drag, the attitude freezes at deployment, his
  filling time is stated only for 150 to 500 ft/s, streamer pleats are not modelled (+58% on
  Kidwell's) and tumble reads +19%. Results are not bit-identical across the three OSes: the report
  is pinned to six decimals, or 1e-7 relative for whole flights; no oracle runs in CI.
