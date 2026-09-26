# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M1.8e is held at M1.8e16 (`[blocked]` on #108), M2.2e5 on #173, #174,
  M1.13, #133, M2.3c on Neer (no private design has a log); active work is M2.4, the census gate.
- **Order:** M1.8e16 waits on #108, M2.2e5 on its four, M2.3c on a design with its log.
  **Run:** M0.1-M0.4, M1.1-M1.7, M1.9, M1.10, M2.1, M1.8a-e19 bar e16, M3.1, M2.2a-e4, M2.3a-b.
- **Neer, 2026-09-20:** Debrief is sunset; a flight log analyzer usable **on its own** is part of
  this project (ADR-046, V21); Phase 5 re-cut.
- **Last updated:** 2026-09-26; M2.3c blocked (ADR-083); M2.4 next.

## Handoff (overwrite each session)

- **Start M2.4** on `m2.4-<slug>`: a census (README table and badge) generated from the committed
  reports, and CI failing on any per-case regression beyond tolerance; *done when* a perturbed drag
  coefficient on a throwaway draft PR turns CI red (close that PR after). Lessons L84-L86, L88.
  M2.3c (ADR-083): once Neer adds a pair, fly the `.ork` with `hpr_validate::real_flight`
  (`parse_log`, `Barometer`, `compare_traces`) under M2.2e3's ids; commit only statistics.
  `cargo xtask real-flights --check` needs `refs/rocketpy`. Astra and Andromeda (EuRoC 2022
  netCDF-4, ADR-081's conversion) are M2.3b's leftovers.
  Flutter (ADR-078): moduli in `materials::SHEAR_MODULI`; metrics (ADR-077): margin `None` past `κ = √10`.
  `.ork` since M1.9c (ADR-076): ignitions, clusters, one powered split fly; open: #183, #184, #185.
  Leads, not causes: #177, private flights above sea level reading low, `C03`, `C09` margins
  (#172). After any physics change run `cargo xtask ork-flights --check`, `--library --check`
  and `real-flights --check`: CI can't fly them; corpus reruns jitter (≤5e-7).
- **Page rules** (ADR-016 to ADR-020): relative links between pages, GitHub URLs for the rest
  (rustdoc too), labels as links to their rows — a lesson a page names needs a row in
  `decisions-and-roadmap.md` — none in headings; new pages in `SUMMARY.md`, a new library a row in
  `docs/api.md`, a milestone's row saying `done` to match. `STATUS.md` holds 150 lines and
  `ROADMAP.md` 1000: reflow a long-met `*Done when:*` list into prose (M2.2a to b4, M3.1c3, c4 are
  reflowed), and shorten old done-log entries.
- **Validation (M2.1, ADR-021 to ADR-026):** CI checks the report on three OSes; predicted mode's
  3% are *targets*; every whole flight names both RMS metrics, each held to 3% of its reference's
  apogee or max speed (ADR-024). The wind oracle flies RocketPy 1.13.0 with #1188 and #1196 by
  `corrections.py`. **Regeneration is not bit-identical**: use `cargo xtask validate` (debug), never
  `--release`; checks allow 1e-12 but **compare a fixture's strings as text**. `cargo xtask aero`
  rewrites `arcas-robin-gap.json`'s last digits whatever you changed.
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
  assumed (OR's `+x` aft, hpr's `+z` at the nose): on the guide's not-settled list for M2.2. A
  cached `auto` number is what OpenRocket last resolved, never an input — 24.12 ignores it on
  reading — and `cargo xtask ork` holds 67 of 71 to it; nothing caches an `outerradius` or
  `innerradius`. Finish heights: the author's forum post, in `refs/sources/openrocket-finish/`.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock;
  oracles run from the repo root with `refs/venv/bin/python` (Java 17 for the OpenRocket ones);
  `xtask designs`, `examples` and `ork` rewrite their outputs.
## Done log (newest first, keep about 15)

- 2026-09-26: M2.3b Real flights (ADR-082): 7 logged flights read as barometers, mean |apogee error| 6.04%, outside the 5% target; 5 outliers checked.
- 2026-09-26: M2.3a ERA5 weather (ADR-081; M2.3 split a to c): netCDF classic from the spec; RocketPy's levels to 1e-12.
- 2026-09-26: M1.10c2 Parquet (ADR-080): in-house writer; Apache's reader agrees bit for bit; M1.10 done.
- 2026-09-26: M1.10c1 Text exports (ADR-079): CSV, JSON, GeoJSON by the published schema, KML by parsing; exact.
- 2026-09-26: M1.10b Fin flutter (ADR-078): TN 4197 eq. 18; Martin's examples at his resolution; 14 moduli; L32.
- 2026-09-26: M1.10a Flight metrics (ADR-077; M1.10 split a to c): peaks on the dense output, margins, delay, landings; L33-35, L94.
- 2026-09-25: M1.9c `.ork` staging and clusters (ADR-076): all within 5% of OR (3 vs no chute); M1.9 done; 13 of 20.
- 2026-09-25: M2.2e4 The causes (ADR-073): all 5 sized by OR without them; 4 within 5%, #177 left.
## Needs Neer (blocking or one-way decisions; the session keeps working on other things)
- **M2.3c needs a design with its flight's log** (ADR-083): no `loft-fixtures` design is the rocket
  of a `debrief-fixtures` log. Add one pair (design file as flown, plus log, date, site, motor) to
  those repos, and an ERA5 file of the day unless cached (Data Store account). Or drop M2.3c.
- **Scrub #186's first revision** (1 minute): it quotes a private design's sizes. On issue #186 click
  *edited* → the oldest revision (marked *created*) → *Delete revision from history*. No API can.
- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require `fmt`,
  `clippy`, `doc`, `deny`, `wasm-check`, `site` and the three `test (...)` and `validate (...)`
  checks; block force pushes. Don't require approvals (authors can't self-approve).
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`… unreserved. Reserve them?
- **OpenRocket example outputs in fixtures** (no action if fine): `openrocket-automatic-radius.json`
  and `openrocket-flights.json` commit radii and flight numbers OR computed for its 17 GPL examples.
- **A glance at GPL source** (no action if fine): M3.1d2's research read about 15 lines of
  `orhelper`'s (GPL-2.0) signatures before its licence was checked; nothing derived (ADR-059 §5).
- **RASAero values in fixtures** (no action if fine): `normal-force-vs-mach.json` commits 30 values
  of RocketPy's 2018 Calisto RASAero II export (ADR-027) plus four summary numbers, and
  `rocketpy-drag-curves.json` enough to rebuild 147 values of five curves (ADR-029).
## Decided without Neer (one line each; significant ones get an ADR)
- ADR-081 to ADR-083 (M2.3): netCDF classic by hand; real flights from `refs/`, hpr read as a
  barometer, outliers' explanations checked; M2.3c blocked, nothing flown on a guessed pair.
- ADR-077 to ADR-080 (M1.10): peaks on the dense output, no margin past κ = √10; flutter by TN 4197
  eq. 18, the lower reading; exports as core text, GeoJSON on the ellipsoid; Parquet by hand.
- ADR-073 to ADR-076: a cause sized by OR flying without it; M1.9's body 0 flies on, a motor per tube.
- ADR-071, ADR-072: M2.2e's corpus is the library's 27 `.ork` (`.CDX1`, `.rkt` wait, #168); private
  flights by id, differences only; public copies out.
- ADR-069: hpr flies OR's record unrecovered, design checks recorded not enforced; causes named.
- ADR-068: M2.2d split d1, d2; OR flies public designs in calm air; its speed point is "unstated".
- ADR-062 to ADR-066: M2.2b2-b5, c split; exact fin inertia and sections, clusters, fillets pinned
  as departures; packed parts as OR packs them; screens apart; the oracle integrates one file twice.
- ADR-060, ADR-061: M2.2 split a to e, mass first, thresholds set first; a `.ork`'s unsaid is OR's.
- ADR-059: key geometry agrees unless apart from both RocketSerializer and OR; `orhelper` never installed.
- ADR-055 to ADR-058: a motor's curve is its file's own first; only what lights at launch flies;
  recovery and stored simulations read as written, not flown; the unread kept in `x-openrocket`.
- ADR-051 to ADR-054: M3.1 split a to d; a `.ork` document kept whole; an automatic dimension
  keeps both halves; angles are degrees; a radius with nothing to take is OpenRocket's 25 mm.
- ADR-050: a reduced element takes the generalized method wherever it has a tangent cone of its
  own; a cylinder's and a boattail's keep the refusal (#123). Edges from the corner.
- ADR-048, ADR-049: model 2 closed on the base; a step in radius keeps its model (#87, #120, #121).
- ADR-047: a flare attaches by NACA 1135's wedge limit under the cone tables' 30°.
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
  normal force misses the tunnel between Mach 0.8 and 1.2; fins off, the body reads 14–38% high from
  Mach 1.5 to 2.96 (M1.8e9's bullet). A blunt tip's cap (M1.8e7) is checked only on a sphere-cone
  (#101); a boattail past 16° is worth 0.67 to 1.35 calibres of doubt. A marched flare (M1.8e18)
  reads +51.5% and +50.4% at Mach 3.95 and 4.63 on the one measured flare; a near-flat flare leaves
  the crossing's pole — +0.129% on the tests' rocket, +4.3% on a short shoulder (#108); a step in
  radius takes the body off the method past 2.7e-11 m tube to tube or 1.3e-13 m up at a boattail —
  −8.65% to −11.34% (#87).
- `.ork` (M3.1) builds all 72 designs' rockets, motors and recovery, but hpr alone flies 4 of 170
  configurations (93 with OpenRocket's database supplied, ADR-067), one powered split at most (#183),
  and recovery is read, not flown. Pods are kept, not read (M1.13). 5 parts are left out with a
  reason, among them the corpus's only tube fins (#133); fin fillets and a rail button's screw head
  are read as the simpler part, with a warning; OR stacks a cluster's tubes in its inertia. `polished` is 2 µm here and may be
  0.5 µm in a newer OpenRocket; a zero-wall part weighs nothing, as in OpenRocket (ADR-061).
- Drag: against RASAero II's Calisto hpr reads −14.9% to −5.1% supersonic (ADR-030); against
  MIL-HDBK-762 the body reads 6–10% low past Mach 1.6 and high through Mach 1 (#67, #68); against
  the Arcas Robin it reads high at every row (#70, #72, #73); a cylinder's base drag is unmeasured
  past Mach 0.3. In wind, a slow rocket's drift rests on body lift: Juno III's apogee drift is 245 m
  in hpr, 240 to 194 m over Galejs's `K` 1.0 to 1.5 (oracle corrections, #1196).
- Flight: no tip-off, turbulence or thrust misalignment; small-angle aero at every `α`. Recovery
  omits canopy overshoot, opening-load factor, added mass and airframe drag; attitude freezes at
  deployment, streamer pleats are not modelled (+58% on Kidwell's), and tumble reads +19%. Reports
  are pinned to six decimals or 1e-7 relative; no oracle runs in CI.
