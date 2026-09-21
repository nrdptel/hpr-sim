# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e, all done bar M1.8e16 (`[blocked]` on #108); the work is
  **M3.1d2**, every `.ork` importing without an error and the RocketSerializer cross-check, now
  M3.1c reads a design whole and M3.1d1 snapshots the public designs (ADR-055 to ADR-058).
- **Order:** M3.1d2, M2.2, M1.9; M1.8e16 waits on #108. **Run:** M0.1-M0.4,
  M1.1-M1.7, M2.1, M1.8a-e19 bar e16, M3.1a-c, M3.1d1; the site is published.
- **Neer, 2026-09-20:** Debrief is sunset; a flight log analyzer usable **on its own** is part of
  this project (ADR-046, V21); Phase 5 re-cut.
- **Last updated:** 2026-09-21 (M3.1d1 shipped: snapshots of public `.ork` designs)

## Handoff (overwrite each session)

- **M3.1d2 next:** the 2 `.ork` files that are not well-formed XML (Loft test fixtures; M3.1a
  refused them) against "imports with zero errors", and RocketSerializer (MIT, not yet in
  `validation/refs.lock.toml`) run on public files for key geometry. **Driving OpenRocket:**
  `validation/oracles/openrocket/automatic_radius.py` (ADR-054) runs 24.12 headless through JPype,
  binding empty motor and preset databases in a Python Guice module; it logs to stdout, so the
  script writes to a path. **Read OpenRocket after it re-resolves** (a save does it): its first
  reading can differ (Dual parachute). M2.2 (ADR-035) can build on it or take a subprocess, which
  alone keeps GPL code out of the process; Java 17 only.
- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest
  (rustdoc too), labels as links to their rows — a lesson a page names needs a row in
  `decisions-and-roadmap.md` — none in headings; new pages in `SUMMARY.md`, a new library a row in
  `docs/api.md`, a milestone's row saying `done` to match. `STATUS.md` holds 150 lines and
  `ROADMAP.md` 1000: reflow a long-met `*Done when:*` list into prose, keeping its lesson bullet.
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024). The wind oracle flies RocketPy 1.13.0 with #1188 and #1196 by
  `corrections.py`. **Regeneration is not bit-identical**: use `cargo xtask validate` (debug),
  never `--release`; checks allow 1e-12 relative but **compare a fixture's strings as text**.
  `cargo xtask aero` rewrites `arcas-robin-gap.json`'s last digits whatever you changed.
- **M1.8a to e19** (ADR-027 to ADR-050). Measurements: the guide's aero page and *Known issues*.
  Working notes: `cargo xtask aero` writes the aero fixtures (scratch in `refs/scratch/m18*/`);
  `SupersonicBody` tabulates every 0.05 Mach from max(1.2, its start), joined over 0.3;
  `BEFORE_M1_8E6` keeps the old rules and `CONE_SLOPES` runs to 30°; a mesh-following answer is
  marked by the pressure **crossing** its tangent cone's, not `η < 0`; the near-flat flare's edges
  come from `flare_reduction_turns_rad` (#108).
- **Debrief, folded in** (ADR-046): `hpr-flightdata` is off `hpr-sim` and must stay off it
  (`forbids = ["hpr-sim"]`, walked by `cargo xtask wasm-check`); sim-versus-flight goes in
  `hpr-forensics`; notes in `debrief-{log-formats,flight-readings,porting-boundary}.md`. **Port
  from its `lib/`, never its `COMPETITION.md`** (GPL-3 Java); its 12 public fixtures may be used.
- **`.ork` readings (ADR-052 to ADR-054):** angles are **degrees**, but *which way they turn* is
  assumed (OpenRocket's `+x` points aft, hpr's `+z` at the nose): on the guide's not-settled list
  for M2.2. A cached `auto` number is what OpenRocket last resolved, never an input — 24.12
  ignores it on reading — and `cargo xtask ork` holds 67 of 71 to it; nothing caches an
  `outerradius` or `innerradius`. Finish heights: the author's forum post, cached under
  `refs/sources/openrocket-finish/`.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock;
  oracles run from the repo root with `refs/venv/bin/python`; `xtask designs`, `examples` and `ork`
  rewrite their outputs.

## Done log (newest first, keep about 15)

- 2026-09-21: M3.1d1 Loft's seven demo designs committed and read into `insta` summary snapshots
  (new dev-dependency), with a synthetic design that flies; M3.1d split into d1 and d2.
- 2026-09-21: M3.1c4 What a `.ork` holds that hpr does not model, kept whole in `x-openrocket`
  (ADR-058): parts, sections, tags and attributes, 5,183 found again. L66 live; #145. M3.1c done.
- 2026-09-21: M3.1c3 A `.ork` design's stored simulations read back (ADR-057): 178, 144 with a
  time series (101,955 rows); a probe measures their units. L64 live.
- 2026-09-21: M3.1c2 A `.ork` design's recovery and separation, read not flown (ADR-056): 137
  devices, 2 left out in pods; 18 of 93 stages separate. A committed probe measures its event words,
  `cd auto` (0.8), a deploy height above ground; set above apogee, one did not open.
- 2026-09-21: M3.1c1 A `.ork` design's motors (ADR-055): 206 in 174 configurations; the file's own
  curve first; 1 flies, on a one-stage airframe read without a warning. L57, L65; #138, #139, #141.
- 2026-09-20: M3.1b1 to b4 (ADR-052 to ADR-054): all 75 `.ork` designs lay out; L49, L58-L63 live.
## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require `fmt`,
  `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and `validate (...)`
  checks; block force pushes. Don't require approvals (authors can't self-approve).
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`… unreserved. Reserve them?
- **OpenRocket example radii in a fixture** (no action if fine): `openrocket-automatic-radius.json`
  commits 67 body radii OpenRocket computed: 63 for the jar's 17 GPL example designs, with their
  names, and 4 for the Apache-2.0 parachute catalogue.
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers, and
  `rocketpy-drag-curves.json` enough to rebuild 147 values of five curves (ADR-029).

## Decided without Neer (one line each; significant ones get an ADR)

- ADR-058: what hpr does not read is kept whole in `x-openrocket`, at a path back, down to the
  attribute; readers record what they ask for.
- ADR-057: a `.ork`'s stored simulations are read as written, to compare against; units measured.
- ADR-056: `.ork` recovery read as written, not flown; a deploy height above apogee and `cd auto`
  are left for the step that flies it.
- ADR-055: M3.1c split c1 to c4; a motor's curve is its file's own first, the bundled catalog
  second; the rocket flies only a configuration whose every motor lights at launch.
- ADR-051 to ADR-054: M3.1 split a to d; a `.ork` document kept whole; an automatic dimension
  keeps both halves; angles are degrees; a radius with nothing to take is OpenRocket's 25 mm.
- ADR-050: a reduced element takes the generalized method wherever it has a tangent cone of its
  own; a cylinder's and a boattail's keep the refusal (#123). Edges from the corner.
- ADR-049: a step in radius keeps the model it has (no source gives a step's normal force faster
  than sound); its size is published. #87 narrowed to it, #120 and #121 carry the rest.
- ADR-048: model 2's drawing is closed on the base, not on its printed lengths; a blunt nose may
  span more than one curved segment but a cap may not reach a cylinder.
- ADR-047: a flare's attachment test is NACA 1135's wedge limit at the flow reaching the corner (TN
  D-4865 p. 5's) under the cone tables' 30°; a steeper flare reads the same radii drawn out.
- ADR-046: Debrief folded in; `hpr-flightdata` off `hpr-sim`, `hpr-forensics` added, Phase 5
  re-cut, `hpr analyze` in M4.2. Its `.ork` parser is clean room, `COMPETITION.md` is not.
- ADR-038 to ADR-040: the march behind a blunt tip starts from the tangent cone, not TN D-4865's
  Newtonian state (which fails on the Arcas nose from Mach 3.96), handover capped at 24°; M1.8e's
  15% bullet is **not met** for the body alone.
- ADR-027 to ADR-037 (details in `DECISIONS.md`): M1.8 split a to e; fins' supersonic slope counts
  both faces; bulged ogives and Haack past `C = ⅓` refused. **Two gaps visible:** M1.8's drag
  bullet, M1.8a's miss.
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
  sphere-cone (#101); a boattail past 16° is worth 0.67 to 1.35 calibres of doubt. A marched flare
  (M1.8e18) reads +51.5% and +50.4% at Mach 3.95 and 4.63 on the one measured flare; a near-flat
  flare leaves the crossing's pole — +0.129% on the tests' rocket, +4.3% on a short shoulder
  (#108); a step in radius takes the body off the method past 2.7e-11 m tube to tube or 1.3e-13 m
  up at a boattail — −8.65% to −11.34% (#87).
- `.ork` (M3.1a to c2) builds all 75 designs' rockets, motors and recovery, but only 1 of 174
  configurations flies: 197 motors are not in the 32-motor catalog (M5.1), staging waits for M1.9,
  and recovery is read, not flown. Pods are not read (M3.1c4). 5 parts are left out with a reason,
  among them the corpus's only tube fins (#133); fin fillets, a rail button's screw head and motor
  clusters are read as the simpler part, with a warning. `polished` is 2 µm here and may
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
