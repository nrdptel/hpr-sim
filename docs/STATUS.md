# Status

Keep this file under ~150 lines. Overwrite the sections; don't let them pile up.

## Now

- **Current milestone:** M0.4a The site and its link checks (M0.4 is split into M0.4a-e)
- **Order:** M0.4a to M0.4e, then M2.1b2 (handoff below)
- **Run:** the first autopilot run; M0.1-M0.3, M1.1-M1.7, M2.1a and M2.1b1 have shipped
- **Last updated:** 2026-09-17 (PR #34 merged after review; M0.4 split, M0.4a not started)

## Handoff (overwrite each session)

M0.4 is split into M0.4a-e (ROADMAP). Nothing of M0.4a is built yet (this cycle went on
reviewing and merging PR #34). Start M0.4a here:

- **Write the tool ADR first** (ADR-016). A research pass on 2026-09-17, not yet reproduced,
  suggests mdBook 0.5.4 (MPL-2.0: a build tool we only run, never link; say so in the ADR and
  `THIRD-PARTY-NOTICES.md`), `mdbook-katex` 0.10.0 (MIT) for math, because mdBook's built-in
  MathJax does not accept `$...$` and GitHub renders only `$...$` and `$$...$$`, and lychee 0.24
  (MIT OR Apache-2.0) with `--offline --include-fragments` over the built HTML and the `.md`
  sources. `mdbook-linkcheck` is abandoned. Check every version and licence before citing it.
- **One source per page:** move `docs/physics/` and `docs/format/` into the book's source, or
  point `SUMMARY.md` at them; never copy. Keep every path `xtask/src/docs.rs` reads working, or
  change it in the same PR.
- **The bare-label check** goes in `xtask/src/docs.rs` beside the existing guards: `L\d+`,
  `ADR-\d+` and milestone ids outside a link fail, each with a test. Fix the pages it flags; don't
  exempt them. CI gets a Linux `docs` job that builds the book and checks links on every PR.
  Pages is still under "Needs Neer"; only M0.4d waits on it.

After M0.4 comes M2.1b2: a `Flight::WholeFlight` variant, five cases in the lock, the L75 test.

- **The reference** is `validation/fixtures/flight/rocketpy-whole-flight.json`. Teach
  `crates/hpr-validate/src/rocketpy.rs` its shape, as it knows `recovery.py`'s; only it may.
- **The drag is the case's, not RocketPy's.** Its exports carry their own terms (ADR-009) and CI
  has no `refs/`, so the fixture declares a constant `C_D0` of 0.5 for both codes. Feed it to hpr
  through `Simulation::with_drag_table` (`crates/hpr-sim/src/flight.rs:292`); do not invent a Mach
  curve there, which is L18 rebuilt inside the reference. Pin the area too: the fixture records
  `reference_radius_m` and `reference_area_m2` for the L75 test to assert.
- **Thrust starts at (0, 0):** RocketPy's `.eng` reader inserts that point, so thrust ramps
  linearly to the file's first (0.008 to 0.038 s). Model it the same way or argue the difference.
- **Gate `max_acceleration_power_on_m_s2`**, not the whole-flight maximum (the parachute for NDRT
  and Prometheus). It is sampled at solver steps; see #36 for that and the oracle's follow-ups.
- **One gap to report, not hide:** Prometheus peaks at Mach 1.014; hpr refuses `M >= 1` until M1.8.
- **Argue each tolerance in the case file** and fly `GravityModel::VerticalTaylor` (ADR-015);
  expect differences from RocketPy's added mass, its rail exit and `0.25*n^2` (ADR-011).
- **Open conventions for the jar (M2.2/M3.1):** override order (L51), radii, positions, ogive,
  walls, fin mass, cant pivot, drag-at-angle, lug diameter.
- **Process notes:** `cargo test -p xtask` guards STATUS, ROADMAP, notices, lessons and the lock.
  The oracles need `refs/rocketpy` (run from the repo root with `refs/venv/bin/python`); its data
  is never committed. Scanned PDFs: `pdftoppm -r 90 -gray -png`; born-digital: `pdftotext -layout`.
  On snapshot drift, run `cargo xtask refs fetch --adopt-snapshots`.

## Done log (newest first, keep about 15)

- 2026-09-17: PR #34 merged after a physics review and a validation audit: the M2.1b1 oracle's
  step is bounded, and the cliff is its own 6000 s `max_time`, not RocketPy's defaults (#33).
- 2026-09-17: M2.1b1 The whole-flight oracle: `validation/oracles/rocketpy/flight.py` flies the
  five examples pad to landing under a declared constant `C_D0` (RocketPy's own exports carry
  their own terms), reproducible byte for byte. Apogees 779 to 3,623 m AGL; Prometheus reaches
  Mach 1.014. Bounding `max_time_step` fixed a step-size cliff, of the oracle's own 6000 s
  `max_time`, that kept every case on the rail (#33).
- 2026-09-17: #11 closed (PR #30): `SolidMotor` refuses an impossible exhaust velocity, the range
  measured over 1,708 catalog motors. #27 closed (PR #28): the M1.7a RocketPy comparison flies
  RocketPy's gravity and asserts the vector, not the magnitude, which is what hid the difference.
- 2026-09-17: M2.1a Validation harness (ADR-015): TOML cases, references with per-value provenance
  and a hash, per-metric 3% gates, a case lock and committed reports. Five descent cases, 30
  metrics, all inside 3% (worst +2.86%). L76–L79 have live tests. It found a gravity-model
  mismatch in its own comparison on the way in (#27).

## Needs Neer (blocking or one-way decisions; the session keeps working on other things)

- **Turn on GitHub Pages** (1 minute; M0.4 publishes the docs there). Settings → Pages → Build and
  deployment → Source: **GitHub Actions**. The autopilot may not change repo settings.
- **Protect `main`** (2 minutes, optional). Settings → Branches → rule for `main`: require the
  `fmt`, `clippy`, `doc`, `deny`, `wasm-check` and three `test (...)` checks; block force pushes.
  Don't require approvals: the autopilot merges its own PRs as you, and authors can't self-approve.
- **Loft's flutter calculator overstates flutter speed by √2** (safety). fusionspace-loft
  `lib/sim/flutter.ts:287` uses 1.337·(λ+1)/2; NACA TN 4197 eq. 18 gives 2.674·(λ+1)/2 (39.3 over
  14.7 psi), so its "1.5 margin" is about 1.06. Fix it or post a notice before Loft shuts down.
- **ThrustCurve data** (yes or no). `hpr-motor` bundles ThrustCurve.org's numbers and names for 32
  motors, with attribution; the site states no terms for them (ADR-005). OK to treat them as facts,
  or ask John Coker? If not, the fallback keeps only the numbers the tests check.
- **crates.io names** (whenever): `hpr`, `hpr-sim`, `hpr-core`... are unreserved. Reserve them?
- **orhelper** (no action if fine): GPL-2.0, so M2.2 drives OpenRocket via JPype, never imports it.

## Decided without Neer (one line each; significant ones get an ADR)

- M0.4 split into M0.4a-e; its four *done when* bullets are unchanged, shared out among them.
- ADR-001 to ADR-004 and M0.3, all in `DECISIONS.md`: the licence and workspace layout; refs
  pinned by hash; body `+z` toward the nose with WGS 84 normal gravity and Coriolis by default;
  the atmosphere and wind by height above sea level; and the doc guards `cargo test -p xtask` runs.
- ADR-005: NFPA 1125 statistics as ThrustCurve computes them; constant exhaust velocity; 32 curves.
- ADR-006: full inertia tensors; part frames forward; Crowell's secant ogive; materials by value.
- ADR-007: origin at the nose tip; one `Component` with a `Part` enum; offsets aft; overrides
  rescale the tensor; comparisons use bundled public-domain curves.
- M1.4, M1.5, M1.6, M1.7 and M2.1b were split into increments, done-when bullets unchanged.
- ADR-008: body CP from the real volume with `sin α/α` and Galejs lift (`K` 1.1); Diederich fins;
  over eight fins, tube fins (#15) and `M ≥ 1` refused.
- ADR-009: Niskanen's drag as printed; lug `d` outer; buttons as pins; 20 µm finish.
- ADR-010: own DOPRI5 (no ODE crate); events stop past the zero; discontinuities are stop times.
- ADR-011: nose-tip reference; nozzle gyration from the integral; `M ≥ 1` stops a flight; rail `μ` 0.
- ADR-012: recovery devices live in `hpr-sim`; `C_D0` on Knacke's nominal area, mid-range; a
  point-mass descent, no added mass; devices add and can release one another.
- ADR-013: streamers take Filippone's three curves by default, appendix C's on request; tumble takes
  OpenRocket's §3.5 (−10 to +19% on its own drops, not the 3 to 14% claimed).
- ADR-014: a separation splits the stack at a stage boundary into two point-mass bodies with their
  own stages' mass and devices; no ejection impulse (linear momentum only), every body needs a
  device, only body 0's act before the split, and it must follow the last burnout (M1.9 stages).
- #11: `SolidMotor` refuses `c = I/m_p` outside 200–5,000 m/s: a units guard, not a propellant
  filter (it rejects none of the 1,708 surveyed motors, 236 to 3,031 m/s), and a behaviour change.
- M2.1b1: a same-drag case declares its own `C_D0(M)`, which both codes then fly, rather than
  committing or reading RocketPy's exports (their own terms, ADR-009; absent from CI).
- ADR-015: a run reads references and never writes them (no update flag); every value carries its
  generator's source and the file its hash; every reported metric is gated at the milestone's 3%
  with no absolute floor, or declared not scored in writing; the locked cases must all run; a
  case's inputs, design and mass come from the reference's own record of what the oracle flew; a
  RocketPy comparison flies RocketPy's gravity model; the report is committed and carries no date.

## Known issues and risks

- Two M1.2 sources are pinned from third-party mirrors (MIL-F-8785C from Abbott Aerospace; WMO-No. 8
  from Mongolia's weather service). Dryden turbulence is an aircraft model, unvalidated until M2.3.
- Only 32 curves are bundled (none in class A); the rest wait for M5's cache. Its checks and the
  1710-file sweep ran on unpinned `refs/samples/` caches.
- Wall and fin mass may differ from OpenRocket's undocumented conventions; M2.2 measures it.
- `.CDX1` has no public spec (the importer relies on samples); ERA5 `.nc` may be netCDF4 (HDF5).
- A new RustSec notice can turn CI red with no code change: upgrade, replace, or `ignore` with a
  reason. API snapshots can't be reproduced once an API moves: CI checks committed fixtures only.
- Barrowman 1966, TIR-33, Galejs, the `.rse` spec and Knacke have no clear terms: never redistribute.
- Aero (M1.5a) is small-angle only and documented to Mach 0.8; body-lift `K` is uncertain (Galejs:
  1.0 to 1.5) and the Recruiter's six fins miss TIR-33 by +3.4% (ADR-008).
- Drag (M1.5b): the RASAero comparison can't show 10% agreement without the exports' inputs (fins
  and finish move each case by 20%+). hpr misses Valetudo's suspect table by 47% and Cavour's
  power-on by 18% (open; ADR-009); drag reads low from about Mach 0.6 until M1.8.
- Flight (M1.6b): no tip-off, roll forcing or damping (M1.8), turbulence or thrust misalignment;
  the small-angle aero is used at every `α`. Four `mass_properties` calls are most of an
  evaluation's 0.4 µs (`perf.md`).
- Recovery: no canopy overshoot or opening-load factor (a 1.5 m canopy peaks at 1.6 kN where
  Knacke's infinite-mass `C_x` gives 5.1 kN), no added mass or airframe drag under a canopy, the
  attitude freezes at deployment, and his filling time is stated only for 150 to 500 ft/s (M1.7a).
  Streamer pleats are not modelled (Kidwell's pleated streamer descends 27% slower); tumble misses
  its own finless drop by +19% (M1.7b).
- `refs doctor` "runnable" means the oracle's runtime starts, not that a flight ran; no oracle runs
  in CI, which compares against stored output (M2.1b).
- hpr's descent results are not bit-identical across macOS, Windows and Linux: the committed report
  is pinned to six decimals, where they agree; CI proved full precision does not (M2.1a).
- Normal gravity's meridional term is pinned only where M1.7a's fixtures sample it (23°S to 41°N,
  under 4.4 km). It is asserted against its first-order closed form to 2%; higher or nearer the
  poles is unmeasured (#27, closed).
