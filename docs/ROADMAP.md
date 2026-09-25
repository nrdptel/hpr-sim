# Roadmap

**Rules:**

- Work top to bottom. The current milestone is the first unchecked one. IDs name the phase; the
  **list order** is the execution order.
- A milestone is done only when every *done when* bullet is demonstrated by a command's output and
  CI is green on all three operating systems.
- A milestone bigger than one session is split in place into `a`, `b`, `c`... increments, each with
  its own *done when*.
- Blocked milestones are marked `[blocked]` with a pointer into `STATUS.md`.
- New milestones may be added (at the right position, with a *done when*). Existing *done when*
  bullets may be tightened but never loosened without an ADR. Milestones are never removed,
  renumbered or moved later in the order without an ADR.
- A `Loft lessons:` line lists ids from `docs/research/loft-lessons.md`. A milestone that owns a
  lesson (listed first on its row) ships its named tests; `cargo test -p xtask` checks this once
  the milestone is checked off.

## Phase 0: Foundations

- [x] **M0.1 Workspace, CI, licenses:** the workspace from `ARCHITECTURE.md` (edition 2024, pinned
  stable, shared lints and dependencies); dual licenses and notices; `deny.toml` denying copyleft;
  `xtask wasm-check`; CI (fmt, clippy, test on three OSes, doc, wasm-check, deny) on **every** PR,
  no `paths-ignore`; README "pre-alpha"; `.gitignore` covers `refs/`, `.autopilot/`, `corpus/`.

  *Done when:* the local gate passes, CI is green on all three operating systems, `cargo deny
  check` passes, and ADR-001 records the license choice and workspace layout. *Met.*

- [x] **M0.2 Reference library:** `cargo xtask refs fetch|verify|doctor`, driven by
  `validation/refs.lock.toml`, populates `refs/` with RocketPy (pinned tag), the OpenRocket 24.12
  jar (sha256 pinned), the public PDFs listed in `VALIDATION.md`, `openrocket-database` (pinned
  commit), ThrustCurve and motor.fusionspace.co snapshots, and `fusionspace-loft` plus the private
  `loft-fixtures` repo (skipped with a note when unavailable, as in CI); a `uv`-managed Python venv
  in `refs/venv` with `rocketpy==1.13.0` and JPype; a Java 17 check.

  *Done when:* `fetch` is idempotent, `verify` checks every hash, `doctor` prints which oracles are
  runnable, `git status` shows nothing from `refs/`, and `THIRD-PARTY-NOTICES.md` lists every
  source with its license and usage mode (bundled, fetched or run-only). *Met.*

- [x] **M0.3 Lessons from Loft.** Read `refs/fusionspace-loft` and write
  `docs/research/loft-lessons.md` (at most 200 lines): the models used, known weaknesses, importer
  quirks, test cases worth porting, and process mistakes to avoid. *Done when:* the file exists, and
  every quirk it lists maps to a roadmap milestone or a named test to write.

- [x] **M0.4 A documentation site people can read.** Added by Neer on 2026-09-17 (VISION V15,
  CLAUDE.md "Documentation is a deliverable"). Retrofit everything shipped so far; later milestones
  keep the site current. Tool and layout by ADR (mdBook first); each page has one source, equations
  render on the site and on GitHub, and workspace rustdoc is published next to the guide, each
  linking the other. Pages: *Start here*; *Getting started* (a runnable first flight); *How a
  flight is simulated*; one page per model, each opening with *In short*; *Accuracy* (every result,
  gaps included); *Glossary*; *Checking a claim*; the decisions; the roadmap.

  *Done when:* split on 2026-09-17 into M0.4a to M0.4e, which carry its four done-when bullets
  unchanged. Done 2026-09-18, with M0.4d's first deploy.

  - [x] **M0.4a The site and its link checks** (ADR-016 to ADR-018): CI builds the site on every PR,
    and a broken link or a bare internal label fails it. *Met.*
  - [x] **M0.4b Model pages, Accuracy, Glossary, Checking a claim**: a model page without *In short*
    fails CI, and *Accuracy* gives every result so far from the committed report. *Met.*
  - [x] **M0.4c Getting started, and how a flight is simulated**: the first-flight example runs in
    CI, and the page walks pad to landing with a diagram. *Met.*
  - [x] **M0.4d Publish** (ADR-019): a workflow deploys the site and the rustdoc to GitHub Pages
    from `main`. *Met* 2026-09-18; Neer turned Pages on, CI run 35396233336 deployed it.
  - [x] **M0.4e The reader test**: a reviewer with no project context answers ten questions from the
    site alone, citing a page each. *Met.*
## Phase 1: Physics core (the heart), with validation interleaved

- [x] **M1.1 Core math, frames, Earth.** `hpr-core`: vectors and quaternions (glam f64) and
  interpolation tables (linear/cubic, clamped, with extrapolation flags); the frames spec in
  `docs/physics/frames.md` (ENU launch frame, body frame, Euler conventions, geodetic/ECEF); WGS84
  Somigliana gravity with altitude, optional Earth-rotation terms.
  - Loft lessons: L1.
  *Done when:* gravity matches the published formula values at at least 6 latitude/altitude points
  to 1e-6 relative; frame round-trip property tests pass; quaternion integration keeps the norm
  within 1e-12 over 1e6 steps; everything compiles for wasm32.

- [x] **M1.2 Atmosphere and wind.** USSA76 from 0 to 86 km (temperature, pressure, density, speed
  of sound, dynamic viscosity by Sutherland); ISA temperature offset; custom profiles from
  soundings (p, T, RH, wind vs height); wind models constant, power/log law, tabulated layers and
  seeded Dryden turbulence.
  - Loft lessons: L2, L3, L4, L5, L6.
  *Done when:* USSA76 matches the tables at at least 25 altitudes to at most 0.1% (the small table
  fixture is committed with its citation); a Dryden spectrum test passes (PSD within tolerance of
  theory); and `docs/physics/atmosphere.md` cites every equation.

- [x] **M1.3 Solid motors:** `.eng` and `.rse` readers and writers (clean room, from the public
  specs); thrust(t), propellant mass by impulse fraction (default) with an optional grain-geometry
  model, CG and inertia over time, nozzle exit area; delays (including plugged) and case/retainer
  mass; an offline catalog type with per-curve provenance and license, bundling only curves with
  clear terms, the rest cached later (M5).
  - Loft lessons: L36, L37, L38, L39, L40, L41, L42, L43.
  *Done when:* for every bundled curve, total impulse, average thrust and burn time match the
  ThrustCurve metadata within 1%; parse-write-parse round trips are identical; mass and inertia
  evolution matches RocketPy's SolidMotor for 3 motors within 1% (reference JSON generated by a
  script in `validation/oracles/rocketpy/`).

- [x] **M1.4 Design model and mass properties:** `hpr-design` components — nose cones (conical,
  tangent/secant ogive, elliptical, power series, parabolic series, Haack/LV-Haack, von Kármán),
  body tubes, transitions, couplers, fin sets (trapezoidal, elliptical, freeform, tube fins),
  launch lugs, rail buttons, inner tubes, motor mounts, centering rings, bulkheads, mass
  components, parachutes, streamers, shock cords; cited clean-room materials; stages and
  configurations; mass, CG and full inertia tensor from geometry, with overrides; structural checks
  with typed warnings; a small public test-design set, `validation/designs/`, so tests never
  snapshot the private corpus.

  *Done when:* analytic volume, area and CG tests pass for every shape; the inertia tensor of
  composite test bodies matches hand calculations; mass, CG and inertia match RocketPy's example
  rockets where RocketPy exposes them; the OpenRocket stored-value comparison is deferred to M2.2
  and noted there.

  - [x] **M1.4a Shapes, materials and component mass properties:** every nose and transition shape
    above (clipped or not), solids of revolution filled or with a wall, fin planforms,
    cross-sections and tabs, and every other component listed, each with mass, CG and full inertia
    tensor from geometry in its own frame; cited materials; `MassProperties` with the parallel-axis
    theorem and rotations.
    - Loft lessons: L44, L45, L46, L48, L49, L91.

    *Done when:* analytic volume, area and CG tests pass for every shape, and the inertia tensor
    of composite test bodies matches hand calculations.

  - [x] **M1.4b Design tree, configurations and checks:** stages and component placement,
    configurations with motors, overrides, reference diameter, structural checks with typed
    warnings, and `validation/designs/`.
    - Loft lessons: L47, L50.

    *Done when:* mass, CG and inertia match RocketPy's example rockets where RocketPy exposes
    them, and the OpenRocket stored-value comparison is deferred to M2.2 and noted there.

- [x] **M1.5 Aerodynamics I (subsonic):** Barrowman CNα and CP for every component with
  Prandtl–Glauert, body lift at angle of attack and fin–body interference; the drag buildup — skin
  friction (laminar/turbulent with roughness), nose/transition pressure drag, base drag power-on
  and power-off, fin profile and thickness, protuberances — and Cd at angle of attack; Cd-vs-Mach
  override tables (power-on/off) from CSV, including RocketPy/RASAero exports, so the dynamics are
  validated on the oracle's drag first. `docs/physics/aero.md` cites each term.

  *Done when:* split below into M1.5a and M1.5b, which carry its three bullets unchanged.

  *Result:* see M1.5a (the Recruiter's six-fin slopes, ADR-008) and M1.5b (Valetudo, ADR-009).
  Skin friction is fully turbulent with roughness, as in Niskanen; laminar and transitional
  friction were not built (ADR-009).

  - [x] **M1.5a Normal force and centre of pressure:** Barrowman CNα and CP for every component
    with Prandtl–Glauert, body lift at angle of attack, fin–body interference, and the `aero.md`
    sections for them.
    - Loft lessons: L8, L9, L10, L89.

    *Done when:* CNα and CP reproduce Barrowman's worked example(s) within 1%.

  - [x] **M1.5b Drag and override tables:** the drag buildup, Cd at angle of attack, Cd-vs-Mach
    override tables from CSV, and the `aero.md` sections for them.
    - Loft lessons: L11, L12, L13, L14, L15, L16, L90.

    *Done when:* subsonic Cd for the RocketPy example rockets is within 10% of their RASAero CSVs
    at Mach 0.3 (tighten this later), and unit tests cover every drag term's limits.

    *Result (ADR-009):* not met for Valetudo (−47% power-off, −50% power-on; its table is 1.44 times
    its own OpenRocket export, which hpr matches to 2%) or Cavour power-on (−18.3%, cause open).
    Calisto, Juno III and Cavour power-off are within 10% (+4.4%, −6.0%, −8.3%) under a declared
    input rule that can't pin the unrecorded inputs.

- [x] **M1.6 6-DOF flight engine:** state — position, velocity, attitude quaternion, angular
  velocity, time-varying mass properties; a guided rail/tower phase with friction and rail-button
  geometry; powered and coast phases with jet damping; adaptive Dormand–Prince 5(4) with dense
  output and event root-finding (liftoff, rail exit, burnout, apogee, ground hit, user events), a
  fixed-step RK4 option; a recorder with a configurable channel set, an observer trait and a
  `criterion` benchmark.

  *Done when:* analytic tests pass (vacuum ballistic, terminal velocity, torque-free precession,
  and pitch oscillation frequency vs linear theory); step-halving convergence shows the expected
  order; events are located to ≤1e-6 s; and a single typical L2 flight simulates in ≤5 ms
  release-mode (number recorded in `docs/perf.md`).

  - [x] **M1.6a Integrator and events:** adaptive Dormand–Prince 5(4) with dense output, event
    root-finding and stop times that put discontinuities on step boundaries; the fixed-step RK4
    option; `docs/physics/integration.md`.
    - Loft lessons: L21, L22, L23.

    *Done when:* step-halving convergence shows the expected order, and events are located to
    ≤1e-6 s.

  - [x] **M1.6b Rigid-body flight:** the state, the rail phase, powered and coast phases with jet
    damping, the flight events, the recorder and observer, and the `criterion` benchmark.
    - Loft lessons: L20, L24, L25, L26.

    *Done when:* analytic tests pass (vacuum ballistic, terminal
    velocity, torque-free precession, and pitch oscillation frequency vs linear theory), and a
    single typical L2 flight simulates in ≤5 ms release-mode (number recorded in `docs/perf.md`).

- [x] **M1.7 Recovery.** Parachutes (Cd·S, inflation time or area growth), streamers and tumble;
  drogue and main with their triggers; descent with wind drift, separated bodies tracked
  independently, landing detection. *Done when:* analytic tests for terminal velocity, descent
  time and drift pass, and descent rate and drift match RocketPy's for 3 example rockets within
  3%. *Result:* met by M1.7a; M1.7b and M1.7c add the streamers, tumble and separation the entry
  lists (ADR-012, ADR-013, ADR-014).

  - [x] **M1.7a Parachutes and descent:** parachutes (Cd·S, inflation time or area-growth model),
    drogue and main with deployment triggers (apogee, altitude, timer, motor delay), drogue
    release, descent with wind drift and landing detection.
    - Loft lessons: L27, L28, L29, L92.

    *Done when:* analytic tests for terminal velocity, descent time and drift pass, and descent
    rate and drift match RocketPy's for 3 example rockets within 3%. *Result (ADR-012):* met.
    Analytic descents to 2.1e-8; five RocketPy examples within 0.71% (time), 0.03% (rate), 0.27%
    (drift); worst drift component 2.87% (NDRT's added mass).
  - [x] **M1.7b Streamers and tumble,** each with a cited drag model. *Done when:* a streamer's and
    a tumbling body's descent rates match the terminal velocity of their cited drag models
    (analytic tests). *Result (ADR-013):* met. Streamers: Carruthers and Filippone (within 9% of
    Kidwell's flat streamer; appendix C 88% fast). Tumble: OpenRocket §3.5, −10 to +19% on its own
    drop tests.
  - [x] **M1.7c Separated bodies:** separation, with every body flown to its own landing and its
    own mass properties and drag. *Done when:* a separation gives every body a landing, and the
    bodies' masses sum to the rocket's. *Result (ADR-014):* met. A `Separation` splits the stack at
    a stage boundary, each body a point mass under its devices; on the two-stage test design both
    land (2.11 m/s under a canopy, 16.74 m/s tumbling), masses to 1e-12, momenta to 1e-9. Every
    body carries a device.

- [x] **M2.1 Validation harness plus the RocketPy code-to-code suite.** The first end-to-end
  milestone: `hpr-validate` and `cargo xtask validate [--fast]`, TOML cases and reference JSON with
  provenance, Markdown and JSON reports, oracle scripts in `validation/oracles/rocketpy/`. At least
  five RocketPy example rockets fly in two modes — **same-drag**, which isolates dynamics,
  environment and motor, and **predicted**, hpr's own aero, whose supersonic gaps are reported
  rather than hidden — and CI compares against the stored references.
  *Done when:* at least 5 cases pass their same-drag tolerances; predicted-mode results are
  reported, with explained gaps; `validation/reports/latest.md` is generated; the CI job is green;
  and a separate, manually triggered workflow regenerates the references.
  - [x] **M2.1a The harness.**
    - Loft lessons: L76, L77, L78, L79.
    *Done when:* `cargo xtask validate` runs every case in the lock against its stored reference
    and writes `validation/reports/latest.md`; and a case whose metric has no tolerance, a
    reference value with no provenance, and a run with fewer cases than the lock expects each fail.
    *Result (ADR-015):* met. Five descent cases, 30 metrics, all within 3% with no floor (largest
    NDRT's northward drift, +2.86%); the command refuses every malformed case the done-when names
    and more, with twelve tests, four of them L76–L79. Valetudo's northward drift first read 28x
    RocketPy's: hpr's gravity had a horizontal part RocketPy's lacks (issue #27).
  - [x] **M2.1b Whole flights against RocketPy, same-drag:** a
    `validation/oracles/rocketpy/flight.py` generator (M2.1b1) and a `Flight::WholeFlight` case
    variant taking the oracle's `C_D0(M)` through `Simulation::with_drag_table` (M2.1b2).
    - Loft lessons: L75.
    *Done when:* split below into M2.1b1 and M2.1b2; M2.1b2 carries M2.1b's three bullets.
    - [x] **M2.1b1 The whole-flight oracle.** *Done when:* `refs/venv/bin/python
      validation/oracles/rocketpy/flight.py` writes a fixture of all five cases, each with the
      metrics M2.1 names, a time series, a loose-solver run and a source for every value, and
      re-running it reproduces the committed fixture byte for byte; no RocketPy data file is
      committed, and `git status` shows nothing from `refs/`; and every case's declared drag table
      is argued in the generator, with its source.
      *Result:* met. A declared constant `C_D0` of 0.5; the fixture reproduces byte for byte;
      apogees 779 to 3,623 m AGL, Prometheus to Mach 1.014. The oracle's own step-size cliff
      (thrust(0) = 0, an unbounded step) was fixed by bounding `max_time_step` (issue #33).
    - [x] **M2.1b2 The whole-flight cases:** a `Flight::WholeFlight` case variant beside
      `RecoveryDescent`, taking the case's `C_D0(M)` through `Simulation::with_drag_table`, and the
      five cases in the lock.
      - Loft lessons: L75.
      *Done when:* at least 5 whole-flight cases run in the lock and pass their same-drag
      tolerances, with each metric's tolerance argued in the case file;
      `hpr_validate::rocketpy::tests::oracle_inputs_come_from_the_case_file_not_hpr_outputs` exists
      and passes; and `validation/reports/latest.md` carries them under ADR-015's gravity rule (the
      comparison flies the oracle's models where hpr has them).
      *Result (ADR-021):* met. Six whole-flight cases, fifteen metrics each; five pass every scored
      metric within 3% (largest +1.783%, Bella Lui's power-on peak). The drifts in wind stayed
      unscored until issue #50 (M2.1d3); Prometheus was a checked `M ≥ 1` gap until M1.8a. The
      comparison flies RocketPy's gravity, atmosphere, rail and thrust.
  - [x] **M2.1c Predicted mode, CI and regeneration:** the same cases flown with hpr's own aero,
    reported beside the same-drag ones; a CI job that runs `cargo xtask validate` against the
    stored references, and a separate, manually triggered workflow that regenerates them.
    *Done when:* split below into M2.1c1 and M2.1c2, which carry M2.1c's three bullets between
    them (the first in M2.1c2, the other two in M2.1c1).
    - [x] **M2.1c1 The CI job and the regeneration workflow.** *Done when:* the CI job is green on
      macOS, Windows and Linux, and the regeneration workflow runs only when a human triggers it,
      its output a diff to review rather than a commit. *Result (ADR-022):* met. `cargo xtask
      validate --check` writes nothing and fails on a metric outside tolerance or a committed report
      this run does not reproduce; CI runs it on three OSes. *Regenerate references* is
      `workflow_dispatch` only, with a read-only token, and uploads the diff; run locally it
      reproduced every fixture and the report byte for byte.
    - [x] **M2.1c2 Predicted mode.** *Done when:* predicted-mode results are in the report for every
      case, each gap explained in the case file or `docs/VALIDATION.md`, with `M ≥ 1` cases reported
      as gaps until M1.8. *Result (ADR-023):* met. `flight.py --own-drag` (hashes, never values).
      Six `predicted-*` cases, 3% targets: 56 of 75 within; Valetudo and NDRT 2020 +10% in apogee;
      misses pinned.
  - [x] **M2.1d The time-series RMS and the path in wind:** the two items of M2.1's list that
    M2.1a to M2.1c leave open — the time-series RMS after alignment (each whole-flight fixture
    already carries its series), and the landing offset, reported but not scored until issue #50
    finds why hpr turns into the wind less than RocketPy. *Done when:* split below into M2.1d1 to
    M2.1d3, which carry these two bullets between them.
    - [x] **M2.1d1 The time-series RMS.** *Done when:* every whole-flight case reports its
      time-series RMS after alignment against the reference's series, gated with its tolerance
      argued in the case file. *Result (ADR-024):* met for every case hpr flies: RMS at RocketPy's
      120 series times, held to 3% of apogee and max speed; same-drag 1.4–39.2 m and 0.13–2.06 m/s,
      all pass; predicted, three outside (the drag), pinned.
    - [x] **M2.1d2 The calm-air cases (issue #50).** *Done when:* the three calm-air cases are in
      the suite, their apogee and landing drifts scored at 3%, and each passes or is a gap its case
      file explains. *Result (ADR-025):* met: Calisto and Bella Lui pass; Juno III's drifts miss
      (−3.7%), reported not scored, 1.6 points being the rail release (`rail_release.py`).
    - [x] **M2.1d3 The path in wind (issue #50).** *Done when:* issue #50's cause is found and the
      drifts are scored within their tolerances, or an ADR records the measured cause and why they
      cannot be, with the gap left visible in the report. *Result (ADR-026):* met: mostly RocketPy's
      mirrored moment point (#1186, PR #1196) and nozzle tensor (PR #1188), both corrected; RocketPy
      then within 1.4% of hpr in wind (`wind_response.py`). Six drifts gated; five not.

- [ ] **M1.8 Aerodynamics II (transonic and supersonic, damping, overrides).**
  - Transonic drag rise and supersonic wave drag.
  - Supersonic fin normal force (Ackeret/Busemann or a documented alternative).
  - CP shift with Mach.
  - Pitch, yaw and roll damping; roll forcing from cant.
  - Extend the M1.5 override tables to CNα and CP vs Mach and AoA, importable from RASAero CSV.
  - Loft lessons: L7, L17, L18.
  *Done when:*
  - Cd vs Mach is within 10% of RocketPy's RASAero CSVs across Mach 0.1–2.0 for the available
    rockets. Per-band errors are in the report.
  - The supersonic cases in the M2.1 suite are within their tolerances.
  - Roll-rate steady state matches the analytic cant/damping balance.

  Split into M1.8a to M1.8e. The measured reference throughout is NASA's Arcas Robin wind-tunnel
  model: TN D-4013 (Mach 0.6–1.2) and TN D-4014 (Mach 1.5–4.63).

  - [x] **M1.8a Normal force and centre of pressure through Mach 1:** fin slope through the
    transonic region to supersonic linear theory, and the fin CP shift with Mach. The normal force
    accepts Mach numbers past 1, so a flight on a drag table flies through Mach 1. Loft lesson L7.
    *Done when:* L7's test passes (the fin slope and CP are Barrowman's at Mach 0 and change with
    Mach); a committed fixture, pinned by a test, holds hpr's `C_Nα` and CP against the Arcas Robin
    measurements at every Mach they give and RASAero II's Calisto export from Mach 0.1 to 2.0,
    against targets set before measuring — CP within 0.5 calibers, `C_Nα` within 15% — with every
    miss explained; and the same-drag Prometheus 2022 case flies through Mach 1 and passes.

    *Result (ADR-027):* met. Linear theory from `M_s`, a join from Mach 0.8. L7 passes; 37 rows
    pinned, 16 outside the targets, explained. Mach 1.5–2.96: `C_Nα` −13.4% to +3.3%, CP within
    0.42 calibers; past Mach 3, 17–25% low (M1.8e). Prometheus flies through Mach 1.010. Since
    M1.8e6's body lift (ADR-037): 17 outside; Mach 1.5–2.96 −16.3% to +1.4%, CP within 0.47.

  - [x] **M1.8b Transonic and supersonic drag.** Every drag term's transonic and supersonic branch,
    and nose wave drag. Loft lessons L17 and L18. *Done when:* M1.8's Cd bullet is met or an ADR
    records why not, with the gap in the report; the Arcas Robin's measured axial force is compared;
    the predicted Prometheus case flies. Split below into M1.8b1 to M1.8b3, which carry these
    bullets between them.

    - [x] **M1.8b1 The drag buildup through Mach 1.**
      - Nose, shoulder and step pressure drag through Mach 1 (Niskanen eq. 3.87 and appendix B,
        Stoney's fineness-3 curves); the buildup accepts Mach 0 to 5. Loft lesson L17.
      *Done when:* L17's test passes; the predicted Prometheus case flies; the Arcas Robin's
      measured axial force is compared: a committed fixture, pinned by a test, holds hpr's
      forebody drag against TN D-4013's and TN D-4014's at every Mach they give, fins on and off.
      *Result (ADR-028):* met. L17's test passes; `drag_against_mach` pins 44 rows, 8 within 10%
      (misses: blunt fin edges, the base lip, the boattail rule). Predicted Prometheus flies.

    - [x] **M1.8b2 Drag against RASAero through Mach 2.** Cd against Mach from RocketPy's RASAero
      CSVs, per band (L18). *Done when:* M1.8's Cd bullet is met or an ADR records why not. *Result
      (ADR-029):* not met, recorded. Calisto's export: 15/15 subsonic, 2/7 transonic, 0/17
      supersonic within 10%; no fin input is within 10% subsonic and supersonic both. MIL-HDBK-762's
      worked example: 6/12, the body 6–10% low past Mach 1.6. Boattail a candidate.

    - [x] **M1.8b3 The boattail and base faster than sound.**
      - A conical boattail's supersonic wave drag (MIL-HDBK-762 Fig. 5-122), the base behind it,
        and a lip in its wake.
      *Done when* (targets set before measuring; or an ADR records why not): the Arcas Robin's 11
      fins-off rows from Mach 1.5 within 10%, the 2 now within staying; Calisto's 17 supersonic
      rows against RASAero II within 10%; each row's change reported (ADR-029).
      *Result (ADR-030):* not met, recorded. Boattails of 3° to 10° from Mach 1.2: −21.9% to
      +28.3%; Arcas Robin fins off from Mach 1.5: 0 of 11, +13.5% to +24.1% (#72); Calisto: 8 of
      17, −14.9% to −5.1%.

  - [x] **M1.8c Roll and damping.** Roll forcing from fin cant and roll damping; pitch and yaw keep
    hpr's local-flow damping (ADR-011). *Done when:* M1.8's roll bullet is met, and hpr's roll
    forcing is compared with the Arcas Robin's measured roll effectiveness (TN D-4014). *Result
    (ADR-031):* met. Barrowman's strip theory with his body factors. Valetudo canted 1° at 100 m/s
    settles on the closed-form balance, −16.948 rad/s, within 1e-11. Against TN D-4014: from Mach
    2.3, 8 of 8 within 5.3%; at Mach 1.5 and 1.8, +14.3% to +47.8%. Damping against the Basic
    Finner: −5.9% to −16.2%.

  - [x] **M1.8d Normal-force overrides.** `C_Nα` and CP tables against Mach and angle of attack from
    a RASAero II export. *Done when:* its `C_Nα` and CP columns replace hpr's in a flight, and the
    reading is tested on the Calisto export. *Result (ADR-032):* met. The table's force at the
    centre of mass's flow, hpr's damping kept. Calisto's export (0°, 2°, 4°): 4,999 rows re-read
    with `refs/`, M1.8a's 30 values in CI; Calisto flies on it; a table's pitch period within 4e-6
    of linear theory, pitch and yaw.

  - [ ] **M1.8e The body's supersonic normal force.** M1.8a measured the gap: past Mach 3 the
    Arcas Robin's body alone lifts 3.9 to 4.6 per rad, where slender-body theory gives 2.3 to 2.8
    with body lift. A cited supersonic method for noses, boattails and crossflow. *Done when:*
    the Arcas Robin's body-alone `C_Nα` (fins off, TN D-4014) is within 15% at every Mach number
    from 1.5, and both configurations' `C_Nα` within 15% at Mach 3.96 and 4.63, or an ADR records
    why not with the gap in the report. Split below into M1.8e1 to e12; e9 judged this bullet.

    - [x] **M1.8e1 The second-order shock-expansion method.** NACA TN 3527's method for a pointed
      body's `C_Nα` and CP at `α → 0`, the cylinder's lift behind the nose included; its Fig. 2's
      tangent-cone slopes read by hand. *Done when* (targets set before measuring): a committed
      fixture, pinned by a test, holds hpr's values against every row of TN 3527's Tables I and II
      (cones and tangent ogives of fineness 3, 5 and 7, cylinders of 0 to 10 calibers, Mach 3 to
      6.28) within 0.05 per radian and 0.1 calibers of its second-order values and within its stated
      ±0.2 of its measurements, every miss explained; and the Arcas Robin's nose and cylinder, with
      and without its boattail (footnote 8), at each Mach number of TN D-4014, beside the measured
      body alone. *Result (ADR-033):* not met, recorded: slopes and CPs 102 and 125 of 144 within
      its values (#81 at its limit), 117 and 109 of 120 of its measurements; Arcas Robin −18.7% to
      +16.4%.
    - [x] **M1.8e2 The body's supersonic normal force in flight.** The body's terms take Mach:
      M1.8e1's method for a pointed nose and its cylinder where it holds, joined to slender-body
      theory below it. *Done when* (targets set before measuring): a flight takes the body's `C_Nα`
      and CP at its Mach number, and a test probes the join at ±1e-9 in Mach and finds no jump; and
      the Arcas Robin's body alone (TN D-4014) through the flight's path at each Mach number from
      1.5, beside the measurement and M1.8a's, in the regenerated report, its changed rows in the
      PR.
    - [x] **M1.8e3 The supersonic join's start without grid steps** (#87's grid half). *Done when*
      (set after building): a test finds a 20° cone's start off the grid, moved under 1e-7 in Mach
      by 1e-6° and strictly by each 0.1° to 20.5°, its cylinder share under 1e-5, no jump at ±1e-9
      at the join's ends or first even row. *Result:* met: Mach 1.341910 (was 1.35); report same.
    - [x] **M1.8e4 The boattail's share faster than sound** (footnote 8; a station rule for shares
      that cross zero). *Done when:* a boattailed body flies with no jump at ±1e-9 in Mach; the long
      Arcas Robin through a flight's path in the report. *Result:* met, long to −27.0%.
    - [x] **M1.8e5 The remaining gap, source by source.** *Done when:* a `docs/research/` page sizes
      each candidate from cited sources against the Arcas Robin's gaps, ranked. *Result:* met; like
      for like hpr reads 15–73% high: crossflow's size, then the boattail.
    - [x] **M1.8e6 Crossflow and the boattail faster than sound** (ADR-036, ADR-037). *Done when:*
      flown with no jump at ±1e-9 in Mach; the Arcas Robin through a flight's path in the report.
      *Result:* met; like for like +3.4% to +41.0% (was +14.9% to +73.2%); M1.8a gains a miss.
    - [x] **M1.8e7 Blunt tips faster than sound** (from e6; split, ADR-038). A vertical or blunt
      nose tip flies a Newtonian cap ahead of TN 3527's method (NASA TN D-4865). *Done when:* flown
      with no jump at ±1e-9 in Mach; the Arcas Robin's committed nose (lip left off) through a
      flight's path in the report; TN D-4865's sphere-cone against its measured normal force.
      *Result:* met; like for like, sphere-cone −1.2% to +32.1%; the nose lip off −4.8% to +37.2%.
    - [x] **M1.8e8 The lip faster than sound** (from e7; ADR-039). *Done when:* flown with no jump
      at ±1e-9 in Mach; the committed Arcas Robin designs through a flight's path in the report.
      *Result:* met; a lip in a boattail's wake carries nothing, so both designs fly the method to
      their base: M1.8a's rows from Mach 1.5 are all within the slope's 15% (+9.4% to −3.3%).
    - [x] **M1.8e9 #90's boattail cap, and M1.8e's 15% bullet judged** (split from e9's pair;
      ADR-040). *Done when:* #90 closed (a bound on the boattail angle, and footnote 8's size pinned
      by a hand calculation); M1.8e's 15% bullet met for the Arcas Robin's body alone, or an ADR
      records why not with the gap in the report. *Result:* met; the correlation is read no steeper
      than 16° and the boattail with its tube integrated by hand; the bullet met at Mach 3.96 and
      4.63, the body alone outside on six rows.
    - [x] **M1.8e10 The lip's shelter, weighed not switched.** #87's five switches all flip one gate
      — whether the method covers the body at all — so each is worth the whole body. The largest is
      the lip's, and the drag buildup already grades its shelter continuously (`share_by_rise`, a
      quarter to a half of the boattail's drop) where the normal force reads a threshold. *Done
      when:* every switch's size is measured by a test; the lip's is gone, its two sides agreeing
      across the old threshold in proportion to the change in shape; no committed fixture moves; an
      ADR records the weight. *Result:* met (ADR-041); the lip's **rise** is a weight, not a switch,
      and five keep measured sizes — a step −8.7%/1.03 cal, a flare −27.5%/0.29 cal, a pointed tip
      −10.4%/1.14 cal, a vertical tip −7.0%/0.64 cal, and the lip's own length −33.0%/1.77 cal,
      which #87 didn't list.
    - [x] **M1.8e11 Cone slopes past Fig. 2's edge, from Sims.** NASA SP-3007 (pinned) tabulates the
      same theory to 30°, where TN 3527's Fig. 2 stops at 24°. *Done when:* the method flies a
      tangent cone of 24° to 30°, a fineness-1 cone among them, from Sims's slopes checked against
      Fig. 2 where they overlap; no committed fixture moves. *Result:* met (ADR-042); SP-3007 Table
      2 at 25°, 27.5° and 30°, agreeing with the chart to 0.0021 per rad at 22.5°; the pointed tip's
      switch moves to 30° and falls to −7.7%/0.81 cal.
    - [x] **M1.8e12 What the handover's cap is worth, and what stops it** (split from the old e12,
      whose aim is now M1.8e13). With slopes to 30° a cap can hand over at the detachment angle, not
      Fig. 2's edge — but moving it there takes the march out of its range on a nose that flattens
      fast, so this increment measures the move and the next one makes it. *Done when:* the cap is a
      parameter, its flown default unchanged so that no committed fixture moves; a sweep of it is
      measured into a fixture (each cap's sphere-cone error, and the nose's readings over 10 to 160
      elements with their reduced counts); tests pin both ends; an ADR records why the default
      stays; and the blocker is a GitHub issue. *Result:* met (ADR-043); 30° follows TN D-4865's
      rule to Mach 2.52 and reads nearer its sphere-cone wherever a cap binds, but puts 109 of 160
      elements into `η < 0` at Mach 4.63, where the answer follows the element count (3.047 to
      3.260). 24° stands.
    - [x] **M1.8e13 What the answer follows when it follows the mesh** (split from the old e13,
      whose aim is now M1.8e16). Issue #108 asked for a reading of `η < 0` that settles as the nose
      is cut finer; before writing one, find out what the answer actually follows. *Done when:* the
      count that separates a settled reading from a moving one over the cap sweep's meshes is
      measured and stored beside every reading; a pointed body of TN 3527's own is shown reducing
      without moving; tests pin both and the mechanism; an ADR and the guide say what it means and
      does not; and #108 is re-scoped to it. *Result:* met (ADR-044); it is the surface pressure
      **crossing** its tangent cone's, not `η < 0`. Across 10, 40 and 160 elements the 27 readings
      without a crossing hold to 0.012 per radian and the 5 with one move 0.035 or more — a flag,
      not a verdict.
    - [x] **M1.8e14 Where the flare's march stops** (the first of three the old e14 splits into;
      M1.8e17 and M1.8e18 carry its other clauses word for word and go next, ADR-045). The method
      already marches a flare — a cone, a tube and a flare return a finite `C_Nα` at Mach 3 — and
      the model around it refuses one, stopping at the first widening body. *Done when:* the
      steepest flare it marches is bisected to f64 resolution over Mach, tests pin it and what stops
      it, and an ADR records what happens where the flare's shock is detached. *Result:* met
      (ADR-045). The edge is the corner's **isentropic** turn running out, not the shock detaching,
      and lands either side of a wedge's limit: 11.9312175° at Mach 1.5 against 12.1126689°,
      26.4714031° at Mach 2 against 22.9735318°, then the tables' 30° from Mach 2.129702032593.
      Which side is the body's doing (no tube: 14.194333° at Mach 1.5), so a march that answers is
      no evidence of attachment.
    - [x] **M1.8e17 The flare through the method** (the second of the old e14's three, ADR-045).
      *Done when:* a flared body flies the method where the flare's shock is attached, with no jump
      at ±1e-9 in Mach or in the flare's angle across that boundary. *Result:* met (ADR-047). The
      test is NACA 1135's wedge limit read at the flow the march delivers to the corner — TN D-4865
      p. 5's own — under the cone tables' 30°, which binds from Mach 2.5192034260. A steeper flare
      reads as the same radii drawn out to that turn, so at the limit the branches are one body: at
      Mach 2 (22.969761173077°) a ±1e-9° probe moves the slope 4.527e-11 and a ±1e-5° probe
      4.527e-7; in Mach at 18.5°, 3.622e-10 and 3.622e-6. The value is continuous; its slope is not
      (−31.4%). The near-flat region it refuses is M1.8e19's (#117).
    - [x] **M1.8e18 What a marched flare is worth** (the third, ADR-045). TN D-4865's model 2 is a
      2.75° blunted cone with an 18.5° flare; its fig. 8 carries normal force and pitching moment
      from Mach 1.50 to 4.63, integrated from the pressures its tables VII to XII print, and from
      Mach 2.96 up its boundary layer separates ahead of the juncture. *Done when:* those readings
      are committed with provenance, and the guide says what it is worth and leaves out. *Result:*
      met (ADR-048). Fig. 8(b) read by M1.8e7's pipeline into `tn-d-4865-flared-cone.json`. hpr
      reads −1.9%, +7.0% and +13.4% at Mach 1.90, 2.30 and 2.96, then +51.5% and +50.4% at 3.95 and
      4.63, where the shadowgraphs show that flare separated (unflared, model 1 reads +29.7% and
      +32.1%). No reading below about Mach 1.5289.
    - [x] **M1.8e15 The step in radius.** A step is a discontinuous profile, which the march refuses
      outright, so unlike the flare it needs a model of its own rather than a decision about one
      that exists. *Done when:* #87 closed or narrowed to the step alone, its measured size in an
      ADR and the guide. *Result:* met (ADR-049). No source gives a step's normal force faster than
      sound, so the size is published and the model left alone. The threshold is a **pair**, both
      bisected: 2.7e-11 m (a billionth of the radius) between two tubes either way, and 1.3e-13 m
      stepping **up** where the slope changes too, which is 1e-12 × the body's length × the change
      of slope and so is not a property of the step at all. Worth −8.65% and 1.03 calibres at the
      threshold wherever it sits and whichever way, −12.55% and 1.36 at 2 mm down, −4.75% and 0.71
      at 2 mm up, and −11.34% and 1.10 on a boattailed body. Stopping the march *at* the step was
      built and rejected: it re-opens ADR-034's mixture (the mixed reading's CP lands **forward of
      both** pure models) and does not close the boattail's band. #87 is narrowed to the step; #120
      and #121 split off it.
    - [x] **M1.8e19 The near-flat flare the march refuses.** Below about 0.059° on the tests' rocket
      a flare's one element is reduced aft of the nose (issue #81), so the march refuses Mach rows
      from the top down: from 0.00090182° the join's start steps (1.2 → 2.2, −4.6% and 0.75
      calibres), and from 0.03816° to 0.05882° the table goes altogether (−8.3% and 1.16 calibres).
      Not monotone in the angle either, and separate from #87 (ADR-047, #117). *Done when:* the
      region's edges are derived rather than bisected, a rule carries the reading across it or the
      refusal is shown to be right, and a test pins whichever it is with the switches' sizes
      measured on both sides. *Result:* met (ADR-050) — the edges are the corner's **crossing** and
      **balance**, solved from its own state, the reduction is read there, so both switches go. What
      is left: ADR-050.
    - [ ] [blocked] **M1.8e16 The blunt tip's handover, past 24°** (the rest of the old e13,
      ADR-044;
      the next free number, so the flare and the step keep theirs). On issue #108; see `STATUS.md`.
      *Done when:* the vertical-tip switch is gone or measured again, fixtures and the guide moving
      together; and, ahead of that, issue #108 closed — a rule for the loading through a crossing
      whose answer settles as the nose is cut finer, on a body that crosses (the committed nose
      under a 30° cap at Mach 4.63), and that leaves TN 3527's printed ogives where they are.
- [x] **M3.1 OpenRocket `.ork` import.** Handles zip, gz and raw XML, schema 1.0 to 1.10, plus the
  documented 1.11 additions; reads components, materials, finishes, motor configurations, recovery,
  stages, and stored simulation results; unknown content is kept in `extensions.x-openrocket` for a
  lossless round trip, and warnings are graceful, never failures.
  - Loft lessons: L49, L56, L57, L58, L59, L60, L61, L62, L63, L64, L65, L66.

  *Done when:* every `.ork` in `refs/loft-fixtures` and the OR example set imports with zero
  errors; committed `insta` snapshots use only public files (the Loft demo fixtures and synthetic
  designs), and private-corpus results go to a gitignored `corpus-out/`, as counts only; the
  RocketSerializer cross-check agrees on the key geometry. *Result:* met by M3.1a to M3.1d.

  Split into M3.1a to M3.1d (ADR-051): the container and the document first, because everything
  after it walks that tree.
  - [x] **M3.1a The container and the design document.** Sniff zip, gzip and raw XML by their first
    bytes; take the design out of the archive and keep every other entry; read the XML into a tree
    that keeps everything the file said, with its schema version and creator; warn, never crash.
    Loft lesson L56. *Done when:* every `.ork` in the reference library and in the OpenRocket jar's
    example set either reads or is shown by a second XML parser not to be well-formed; each one
    written back out and read again gives the same document; `cargo xtask ork` prints those counts
    and writes the per-file detail to a gitignored `corpus-out/`; and
    `hpr_io::ork::tests::malformed_inputs_error_not_panic` is live. *Result:* met (ADR-051): 76 of
    78 read and round-trip; 2 Loft test fixtures are not well-formed XML, as expat agrees.
  - [x] **M3.1b The component tree.** Components, shapes, materials, finishes and overrides into
    `hpr-design` types, automatic dimensions resolved (L49, L58 to L63). *Done when:* every design
    in the reference library gives a `hpr_design::Rocket` whose `layout()` succeeds, each of those
    lessons' named tests is live, and the counts go to `corpus-out/` as above. *Result:* met by
    M3.1b1 to M3.1b4 (ADR-052 to ADR-054): 75 of 75 designs lay out; 1 document holds none.
    - [x] **M3.1b1 The values inside the tags.** What every later step asks the tree for: numbers,
      counts, flags, and the dimensions OpenRocket works out for itself; the tags it writes under
      two names; the overrides. Loft lessons L58, L62, L63. *Done when:* those three lessons' named
      tests are live, each resting on what the corpus shows rather than on an assumption, and `cargo
      xtask ork` prints the counts they rest on. *Result:* met (ADR-052): `auto <number>` keeps both;
      a stated `0` is a value.
    - [x] **M3.1b2 The spine.** The stages and the body components stacked in them, with their
      shapes, lengths, radii, walls, materials and overrides, every automatic radius marked for
      `layout()` to resolve rather than filled in, across a stage boundary too (L59). *Done when:*
      L59's named test is live and `cargo xtask ork` says how many spines lay out and what was left
      off them. *Result:* met: 73 of 76 spines lay out; a wall-less shoulder reads solid, for M2.2.
    - [x] **M3.1b3 The parts on and inside the body**, with their positions, what they take from
      their parents, and a sourced finish (L49, L60, L61). It split M3.1b's bullets rather than
      rewriting them; M3.1b4 carried the rest unchanged. *Done when:*
      every part OpenRocket writes on or inside a body component is read into an `hpr_design` part
      or left out with its reason; those lessons' tests are live; and `cargo xtask ork` says how
      many parts were read and how many left out. *Result:* met (ADR-053): 765 parts, 5 left out
      with a reason; angles are degrees, their sense unsettled; 67 of 71 cached answers match.
    - [x] **M3.1b4 The designs that still do not lay out**, carrying M3.1b's bullet. Three of the
      76: one holds no `<rocket>` with components in it at all, and two have a chain of automatic
      radii with no fixed radius anywhere to resolve against, one caching a number and one not.
      *Done when:* each either lays out or is shown to hold no design, with its reason on the
      `.ork` page; a rule for an unresolvable chain, if there is to be one, rests on something
      written down rather than on a cached number; and `cargo xtask ork` says so. *Result:* met
      (ADR-054): 75 of 75 lay out, 1 document holds none; 7 radii take OpenRocket's 25 mm default;
      67 of 67 body radii agree with OpenRocket.
  - [x] **M3.1c Motors, recovery, stages and what OpenRocket last did** (L57, L64, L65, L66).
    *Done when:* those lessons' named tests are live, a design's stored results are read back, and
    a document with unknown content round-trips through `extensions.x-openrocket`. Split c1 to c4
    (ADR-055). *Result:* met (ADR-055 to ADR-058), bar the text of a tag's unread second copy.
    - [x] **M3.1c1 Motors and their configurations.** The configurations a design declares, the
      motor each mount holds in each, when it ignites, its delay, and a thrust curve from the
      archive's `thrustcurves/<digest>.rse` or the bundled catalog (L57, L65). *Done when:* L57's
      and L65's named tests are live; every `<motor>` in the reference library is read into its
      configuration or left out with its reason, and flies from a curve or is named as unresolved;
      and `cargo xtask ork` prints those counts and how many designs assemble a configuration.
      *Result:* met (ADR-055): 206 motors in 174 configurations, 6 left out in pods and parallel
      stages; 6 curves found; 1 configuration flies, on an airframe read without a warning.
    - [x] **M3.1c2 Recovery and separation.** When each parachute and streamer opens and each
      stage separates, per configuration, and the drag coefficient each device states. *Done
      when:* every recovery device and stage in the library has its settings read or left out
      with a reason, and `cargo xtask ork` prints the counts. *Result:* met (ADR-056): 137 devices
      read, 2 left out in pods; 18 of 93 stages separate, 2 parallel stages' left out.
    - [x] **M3.1c3 What OpenRocket last did.** Stored launch conditions and results: the summary,
      the time series and the events (L64). *Done when:* L64's named test is live, a design's
      stored results are read back, and `cargo xtask ork` prints the counts. *Result:* met
      (ADR-057): 174 simulations read, 142 with a time series; units measured by a probe.
    - [x] **M3.1c4 Pods, parallel stages and the rest** (L66). *Done when:* L66's named test is
      live, and a document with unknown content round-trips through `extensions.x-openrocket`.
      *Result:* met (ADR-058): 17 parts, 87 sections, 1,888 tags, 3,128 attributes, all 5,120 found again.
  - [x] **M3.1d The corpus and the cross-check.** `insta` snapshots on public files only, and the
    RocketSerializer cross-check. *Done when:* the parent's four bullets above are met. Split: d1, d2.
    - [x] **M3.1d1 Snapshots of public designs.** *Done when:* committed `insta` snapshots use only
      public files (the Loft demo fixtures and synthetic designs), and private-corpus results go to
      a gitignored `corpus-out/`, as counts only. *Result:* met: Loft's 7 demos and a synthetic one.
    - [x] **M3.1d2 The cross-check.** *Done when:* every `.ork` in `refs/loft-fixtures` and the OR
      example set imports with zero errors, and the RocketSerializer cross-check agrees on the key
      geometry. *Result:* met (ADR-059): 27 and 17 files, 0 errors; the original record had 1,212 numbers over 74
      designs, none of hpr's is apart from both RocketSerializer and OpenRocket.

- [ ] **M2.2 OpenRocket oracle and corpus.** `validation/oracles/openrocket/` (JPype, OR 24.12)
  flies the OR examples and the corpus; the stored results inside the `.ork` files are used as a
  second reference; the deferred M1.4 mass/CG checks run against OR values. *Done when:* at least
  20 designs are in the report with an error distribution (apogee, max velocity, stability margin,
  mass, CG); every design with apogee error above 5% has a written hypothesis; private designs
  appear only as anonymised ids. Split into M2.2a to M2.2e (ADR-060): mass first.
  - Loft lessons: L19, L51, L80, L81, L82, L87.
  - [x] **M2.2a Structure mass, CG and inertia** (the M1.4 deferral). *Done when:* `cargo xtask
    ork` holds every design OpenRocket opens to its structure's mass, CG and inertias and prints
    the spread; the Loft demo record is checked in CI; every design outside 1% in mass or 1% of
    length in CG has a written hypothesis. *Result:* met (ADR-060): 57 and 58 of 74 within 1% in the original snapshot; the
    17 outside, five causes hpr warns of; roll inertia unexplained (median 2.1%).
  - [ ] **M2.2b OpenRocket's mass conventions** (L51, L87). *Done when:* L51, L87 are live and each
    convention ADR-060 lists, and roll inertia, is hpr's rule or a written departure, M2.2a rerun.
    Split into b1 to b5 (ADR-061 to ADR-063).
    - [x] **M2.2b1 What a `.ork` leaves unsaid, and overrides** (L51). *Done when:* a wall-less
      shoulder, a part with no material and inertia under an override are each hpr's rule or a
      written departure, measured on probe designs OpenRocket reads; L51 is live; M2.2a rerun.
      *Result:* met (ADR-061): walls, shoulders and materials read as OpenRocket's; two override
      departures pinned; the original rerun had 61 and 62 of 74 within 1%; 13 outside, each a cluster, fillets or unread.
    - [x] **M2.2b2 Fins, rail buttons and roll inertia.** *Done when:* roll inertia's 2.1% explained
      or bounded; airfoil, rounded and elliptical fins each hpr's rule or a written departure; #151
      settled; M2.2a rerun. *Result:* met (ADR-062): the roll gap is OpenRocket's fin rule (hpr's
      exact integral kept); with it, median 0.001% and each file outside 1% has a cause; #151 fixed.
    - [x] **M2.2b3 Packed parts.** *Done when:* a packed part with no size, and an override on one
      that weighs nothing, are each hpr's rule or a written departure; M2.2a rerun. *Result:* met
      (ADR-063): both OpenRocket's on probes, to 1e-12; roll within 1% on 57 of 74 (was 55).
    - [x] **M2.2b4 Clusters, fillets and unread parts.** *Done when:* each is hpr's rule or a
      written departure, M2.2a rerun. *Result:* met (ADR-064): the 3-ring cluster is read as one
      tube and pinned, 5 and 10 mm fillets are omitted and pinned, unread parts stay in
      `x-openrocket` and mark designs reduced; the 2026-09-22 as-of rerun gave 61/74 mass,
      62/74 centre, 53/74 pitch and 57/74 roll within 1% (the last with OpenRocket's fin rule).
      The reproducible 2026-09-23 scratch-excluding survey gives 58/71, 59/71, 50/71 and 56/71.
    - [x] **M2.2b5 Stored results as found** (L87). *Done when:* L87 is live: stored runs remain readable, but stale statuses and structurally implausible results are excluded from reference gates and census denominators with stable reasons (ADR-065). *Result:* met: 137 of 174 stored runs are eligible; 37 are excluded by stable reason (17 external, 11 outdated, 7 not-simulated, 2 internally inconsistent).
  - [ ] **M2.2c The motors OpenRocket flies.** *Done when:* every configuration held back only for
    want of a curve flies or is named with its reason, each curve's impulse within 0.1% of OR's.
  - [ ] **M2.2d Flights to apogee on the public designs** (L80, L81). *Done when:* those that fly
    are in a report against OR (apogee, max velocity, stability margin) and L80, L81 are live.
  - [ ] **M2.2e The corpus** (L19, L82). *Done when:* the parent's *done when* is met, unchanged.

- [ ] **M1.9 Staging, clusters, airstarts (COTS).**
  - Stage separation triggers (burnout plus delay, altitude, time); sustainer ignition.
  - Booster tracked through recovery.
  - Clustered motor mounts, with mass and thrust summed and the thrust offset handled.
  - Loft lessons: L30, L31, L93.
  *Done when:*
  - A two-stage design and a cluster design each match OpenRocket within the per-case tolerance.
  - Event ordering tests pass.

- [ ] **M1.10 Outputs and derived metrics.**
  - Static and dynamic stability margin over the flight.
  - Optimum ejection delay; max q; flutter velocity and margin (primary source cited).
  - Landing point in lat/lon.
  - Exports: CSV, JSON, Parquet (feature), KML and GeoJSON.
  - Loft lessons: L32, L33, L34, L35, L94.
  *Done when:*
  - Metrics are unit-tested.
  - Exported files are validated (GeoJSON by schema, KML by parsing).
  - Flutter matches the worked example in the cited source.

- [ ] **M2.3 Real flights.**
  - Cases from the RocketPy flight data with their ERA5 environments, which needs a weather-file
    reader: a netCDF reader or a documented conversion.
  - Also the corpus flights that have logs.
  - Loft lessons: L83.
  *Done when:*
  - At least 6 real flights are in the report, with apogee error and altitude-trace RMS.
  - Mean absolute apogee error is reported against the 5% target.
  - Each outlier has an explanation.

- [ ] **M2.4 Accuracy census gate.** Generate a summary census (a README table and badge) from the
  report. CI fails on any per-case regression beyond tolerance.
  - Loft lessons: L84, L85, L86, L88.
  *Done when:* a deliberately perturbed drag coefficient on a throwaway draft PR makes CI fail.
  The failing run is linked from the real PR's description, and the throwaway PR is closed with
  `gh pr close --delete-branch`.

- [ ] **M1.11 Ejected sections and payloads.** Added by Neer on 2026-09-18 (VISION V17).
  - A separation at any joint, not only a stage boundary (ADR-014): an ejected nose cone, a body
    section, or a payload carried inside, each flown to its own landing under its own recovery
    device, or tumbling. Pieces joined by a shock cord fly as one.
  - Ejection triggers as for recovery devices (apogee, altitude, timer, motor delay), and an
    optional ejection impulse.

  *Done when:*
  - A design that ejects its nose cone and a payload, each under its own parachute, lands every
    piece and reports each landing point.
  - The pieces' masses sum to the rocket's and momentum is conserved at each split, to M1.7c's
    tolerances; each descent rate matches the analytic terminal velocity for its device and mass.

- [ ] **M1.12 Mass that moves or leaves in flight.** Added by Neer on 2026-09-18 (VISION V18).
  - Payload mass that moves along the airframe, or leaves it (released ballast or payload), on an
    event or a schedule, with the mass, centre of gravity and inertia updated through the flight.
  - The equations of motion carry the moving mass's relative-motion terms, or an ADR shows, with
    numbers, that they are negligible.

  *Done when:*
  - Mass properties before, during and after a change match hand-computed values, and a release
    conserves mass and momentum.
  - A test shows a moving mass shifting the stability margin as the hand calculation predicts.

- [ ] **M1.13 Pods.** Added by Neer on 2026-09-18 (VISION V19).
  - External bodies beside the airframe: side pods, and outboard motor pods using M1.9's clusters.
    Mass properties off the axis; each pod's normal force and drag, and its interference with the
    body, from a cited source.
  - The `.ork` importer (M3.1) reads pods.

  *Done when:*
  - A pod's mass properties match the hand-computed parallel-axis values.
  - A pod design matches OpenRocket within the per-case tolerance, with the limits of both codes'
    pod models stated in the docs.
## Phase 2: Library surfaces and interop

- [ ] **M4.1 Facade API.** The `hpr` crate offers a RocketPy-like builder (`Environment`, `Motor`,
  `Rocket`, `Flight`) plus trait-based custom models. Add `examples/` (at least 4) and a rustdoc
  guide.
  - Loft lessons: L95.
  *Done when:*
  - The examples run in CI.
  - rustdoc has zero warnings.
  - A "custom aero model" example overrides a built-in model through the trait.

- [ ] **M4.2 CLI.** `hpr sim|validate|convert|motors|mc|optimize|compare|analyze|diagnose` (stubs
  are fine for commands whose milestone hasn't come yet), `--json` everywhere, and shell
  completions. The README's command and format table is generated from the registered commands
  (Loft lesson P10). `hpr analyze <log>` reads a flight log and prints its readings; it takes no
  design and runs no simulation (ADR-046), which is how the analyzer reaches a user who only ever
  wants that.

  *Done when:* `assert_cmd` tests cover every implemented command and the JSON output validates
  against the published schemas, and `hpr analyze` is tested on a log with no design file
  present.

- [ ] **M3.2 OpenRocket `.ork` export** (schema 1.10).
  - Loft lessons: L67, L68.
  *Done when:*
  - `.ork` → hpr → `.ork` → OR 24.12 (oracle) loads every corpus design.
  - OR re-simulation of the exported file matches the original's within 0.5% apogee.

- [ ] **M3.3 The hpr open design format v0.1.** See the brief in `ARCHITECTURE.md`.
  - Spec in `docs/format/`; JSON Schema generated by `schemars` under `schema/`.
  - Migrations framework; zip container.
  - TypeScript and Python type generation.

  *Done when:*
  - Every corpus design round-trips `.ork` → hpr format → `.ork`, and hpr's own simulated apogee
    for the result matches the original import's to within 1e-9 relative.
  - Schema validation runs in tests.
  - The spec includes a comparison table against `.ork`, `.rkt`, `.CDX1` and `.rpy`.
  - ADR records the file extensions and versioning policy.

- [ ] **M4.3 Python bindings.** `hpr-py` (PyO3 abi3 + maturin) with numpy outputs, a RocketPy-like
  API, and Python callbacks for custom models. pytest suite; CI builds wheels on 3 operating
  systems (no publishing).

  *Done when:*
  - pytest passes in CI.
  - A notebook-style example reproduces a RocketPy example flight via hpr within the M2.1
    tolerance.

- [ ] **M5.1 Online layer and cache.** `hpr-net`: HTTP client (rustls), on-disk cache (platform
  dirs), TTLs, an explicit offline mode, attribution strings.

  *Done when:*
  - Tests run against recorded fixtures (no live network in CI).
  - Offline mode never touches the network (asserted by test).

- [ ] **M5.2 Weather.**
  - Open-Meteo forecast and historical-forecast with pressure-level winds, turned into an
    atmosphere/wind profile.
  - GFS/RAP GRIB2 (pure Rust).
  - U. Wyoming soundings.
  - Offline import of ERA5/GFS files the user provides.

  *Done when:*
  - Recorded-fixture tests pass.
  - A profile built from a recorded Open-Meteo response reproduces the pressure, temperature and
    wind values at the pressure levels.

- [ ] **M5.3 Site data.** Elevation (Open-Meteo API with cache; optional user GeoTIFF/DEM file),
  geodetic helpers, magnetic declination (WMM2025).

  *Done when:*
  - WMM matches NOAA test values.
  - Elevation lookups are cached and work offline after the first fetch.

- [ ] **M5.4 Motor stock and prices.**
  - motor.fusionspace.co client (`meta`, `motors`, `in-stock`, `vendors`, and per-motor
    endpoints), joined with ThrustCurve curves.
  - Offline snapshot; `hpr motors search --in-stock --class L --max-price 150`.

  *Done when:*
  - Recorded-fixture tests pass.
  - The designation to ThrustCurve id mapping covers at least 95% of in-stock motors, with a
    report of the misses.
  - Attribution is displayed as the API asks.

- [ ] **M5.5 Parts catalog.** Import the OpenRocket `.orc` component database (Apache-2.0, with
  notices). Lookup by vendor and part number; parts can be used from the design API.

  *Done when:* all `.orc` files parse, and a design built from catalog parts simulates.
## Phase 3: Uncertainty, optimization, challenges

- [ ] **M6.1 Monte Carlo and sensitivity.**
  - Seeded, parallel dispersion over: mass/CG, Cd scale, motor impulse and timing (certification
    tolerances), wind, launch angle, deployment delays.
  - Landing ellipses at confidence levels; apogee distribution.
  - Sensitivity analysis (Morris screening and Sobol indices).
  - Loft lessons: L52, L53, L54, L55, L96.
  *Done when:*
  - Results are bit-reproducible for the same seed.
  - The ellipse math is tested against analytic Gaussians.
  - 10,000 flights of an L2 design finish in ≤10 s on the dev machine (recorded).

- [ ] **M6.2 Optimization engine.**
  - Continuous and discrete design variables, including motor choice and catalog parts.
  - Constraints and single- or multi-objective goals.
  - Algorithms: CMA-ES, NSGA-II, Bayesian/EGO; robust (MC-in-the-loop) mode.

  *Done when:*
  - Benchmark functions converge to known optima within tolerance.
  - A "hit 3,048 m" design problem is solved with the result validated by re-simulation.

- [ ] **M6.3 Challenge specs and presets.**
  - A TOML/JSON challenge format: target apogee and scoring, impulse limits, stability
    min/max, rail-exit velocity, mass/length/diameter limits, budget (including live motor
    prices), in-stock-only, drift and landing limits, descent-rate and landing kinetic-energy
    limits, payload.
  - Presets (each cited, dated, and flagged "verify against current rules"): Spaceport America
    Cup/IREC-style, NASA Student Launch-style, TARC-style, and a generic target apogee.

  *Done when:* each preset is solved end to end from both the CLI and Python, with a Pareto report
  and a check that the winners satisfy every constraint by re-simulation.

- [ ] **M6.4 Airbrakes.** Added by Neer on 2026-09-18 (VISION V16).
  - Deployable drag surfaces: added drag as a function of deployment and Mach, from a table or a
    cited semi-empirical estimate; deployment rate limits.
  - A controller interface: a user's controller, sampled at a fixed rate, reads simulated sensors
    (seeded noise) and commands deployment. A reference apogee-targeting controller ships.

  *Done when:*
  - RocketPy's air-brakes example, flown with the same drag table and controller, matches RocketPy
    within 3% in apogee and in the deployment history.
  - A challenge spec (M6.3) can target an apogee using airbrakes.

- [ ] **M6.5 Canards.** Added by Neer on 2026-09-18 (VISION V16).
  - Fixed canards first: fins ahead of the centre of gravity, with the canards' downwash on the aft
    fins from a cited interference model (Pitts, Nielsen and Kaattari, NACA Report 1307, is the
    first candidate).
  - Then movable canards: lift from deflection, actuator rate limits, and roll control using
    M1.8's roll damping and M6.4's controller interface.
  - Scope: stabilization and roll control, as student competitions fly them. Steering to a target
    point is out of scope.

  *Done when:*
  - Fixed canards match OpenRocket's normal force and centre of pressure within the per-case
    tolerance.
  - A roll-control case damps a step roll disturbance as the linearised analytic response
    predicts.
## Phase 4: More formats and embeddings

- [ ] **M3.4 RockSim `.rkt` import/export** (clean room, from the RockSim XML doc and samples).
  - Loft lessons: L69, L70, L71.
  *Done when:* the corpus `.rkt` files import, and the exports reopen in our importer with
  semantic equality.

- [ ] **M3.5 RASAero `.CDX1` import/export** (from samples only). Fix the Loft `<Location>` bug
  class.
  - Loft lessons: L72, L73, L74.
  *Done when:* the corpus `.CDX1` files import with overall length within 0.5% of the stated
  values.

- [ ] **M3.6 RocketPy interop.** Export a runnable RocketPy script and `.rpy`; import `.rpy` where
  feasible.

  *Done when:* exported corpus designs run in RocketPy (oracle venv) and agree with hpr within the
  M2.1 tolerances.

- [ ] **M4.4 C ABI and WASM.** `hpr-ffi` with a cbindgen header and a C example built in CI;
  `hpr-wasm` package with generated TS types and a Node test.

  *Done when:* the C example and the Node test run in CI and reproduce a reference flight.
## Phase 5: Flight data and forensics

M7.1 and M7.2 must work with no design file and no simulator, so that analysing a flight stands on
its own (ADR-046): `hpr-flightdata` may not depend on `hpr-sim`, and `cargo xtask wasm-check`
enforces it. M7.3 and M7.4 compare a flight with a simulation of it and live in `hpr-forensics`.
The formats, reading methods and log corpus come from Debrief (`refs/fusionspace-debrief`), written
up in `docs/research/`.

- [ ] **M7.1 Flight log importers.**
  - AltOS (TeleMetrum/TeleMega/EasyMega, CSV and eeprom), Missile Works RRC3 and RFF, Eggtimer,
    Featherweight Raven/Blue Raven and Featherweight GPS, PerfectFlite, Entacore AIM,
    AltimeterCloud, spreadsheet exports, CATS.
  - The EuRoC/Juno CSV layouts; generic CSV with column mapping; unit detection.
  - One canonical flight record every importer maps into, carrying every sample the logger wrote.

  *Done when:* every sample file in refs imports, snapshot-tested, and the crate builds with no
  dependency on `hpr-sim` (`cargo xtask wasm-check`). Formats without samples are listed as
  "needs samples" in `STATUS.md`.

- [ ] **M7.2 Readings, reconstruction and ghost data.**
  - The readings a flight gives — apogee, maximum velocity and acceleration, burnout, descent
    rates per phase, flight time — each carrying its provenance: measured, derived, or clipped
    when the sensor saturated. A reading the log cannot support is withheld with a reason.
  - RTS/Kalman smoothing fusing baro, accel and GNSS; liftoff detection and time alignment.
  - Two recordings of one flight read side by side, never averaged into one number; per-stage logs
    assembled onto one timeline.
  - A "ghost" data product: time-synced trajectories in a common frame, as JSON and CZML/glTF.

  *Done when:*
  - On synthetic data (a simulated flight plus a noise model), the smoother recovers the truth
    within the stated error, and real RocketPy flights produce ghost files.
  - Every reading names its method and its provenance, and the corpus has a case for each
    withheld reading.
  - A flight is read end to end with no design file present, from the library and from
    `hpr analyze` (M4.2's command).

- [ ] **M7.3 A flight against its simulation.** Sim-versus-real residuals, then fitting Cd scale,
  mass, motor impulse scale and wind to a log (Levenberg–Marquardt plus a Bayesian option with
  uncertainty). The first milestone in `hpr-forensics`.

  *Done when:* residuals are reported for a real flight against its simulation, synthetic-truth
  recovery is within tolerance, and real-flight fits are reported. A reading keeps the provenance
  M7.2 gave it wherever it meets a simulated number.

- [ ] **M7.4 Fault diagnosis.**
  - A hypothesis library with simulate-able fault models: motor under/over-performance, CATO or
    early burnout, high drag or damage, weathercocking, marginal stability/coning,
    early/late/no drogue, main failure, separation → ballistic, baro-port or ejection pressure
    artifacts, Mach baro error, fin flutter or fin loss, delay mismatch, airstart failure.
  - Rank the hypotheses by evidence; explain in plain language; suggest the data that would
    disambiguate.

  *Done when:*
  - A seeded synthetic benchmark with at least 200 faulty flights reaches top-1 accuracy ≥80%
    and top-3 ≥95%, reported.
  - At least 2 real anomalous flights are analyzed (if the data exists; otherwise note it).
## Phase 6: Design experience (library level)

- [ ] **M8.1 Design assistant.**
  - Templates: minimum diameter, 3FNC, dual deploy L1/L2/L3.
  - Automatic checks (stability window, motor fit, rail buttons, flutter margin, recovery
    sizing).
  - Auto-size parachutes to a target descent rate.
  - Suggestions with reasons.
  - Loft lessons: L97.
  *Done when:* every template simulates and passes its own checks, and each check has
  positive/negative tests.

- [ ] **M8.2 Edit model for UIs.** A command/undo model over the design tree, stable ids, change
  events, and cheap incremental re-simulation hooks.

  *Done when:* property tests show undo/redo is a round trip, and re-simulation after an edit is
  measured.
## Phase 7: UI, 3D, web, mobile (only after the phases above)

- [ ] **M9.0 UI architecture ADR plus a spike.** Compare the leading option (web UI plus WASM core,
  PWA, Tauri v2 for desktop and mobile) against all-Rust. Build a throwaway 3D trajectory spike in
  each and measure bundle size, frame rate on a phone-class device profile, and development
  effort.
  - Loft lessons: P15 (read Loft's `OWNER-NOTES.md` UI notes before the spike).
  *Done when:* the ADR is merged with measurements.

- [ ] **M9.1 Desktop app shell.** Design editor (2D profile plus 3D model), simulation runner,
  plots, file import/export. Inspired by the workflows of OpenRocket, RASAero and RockSim, but
  modern.
- [ ] **M9.2 3D flight replay with a ghost.** Real vs simulated flight on terrain, wind
  visualization, time scrubber, chase/ground/onboard cameras.
- [ ] **M9.3 Web PWA.** Fully client-side, offline, installable; the same UI.
- [ ] **M9.4 Mobile.** Offline PWA first, then Tauri mobile iOS/Android; a touch-first "pad day"
  mode.

(The M9.1–M9.4 *done when* criteria are written in M9.0.)

- [ ] **M9.5 Accounts and cloud saves.** Added by Neer on 2026-09-18 (VISION V20).
  - Optional accounts that save designs and flights and sync them across devices. Everything still
    works signed out and offline; accounts only add sync and sharing.
  - A saved flight stores its inputs and seed and is re-flown on open, so storage stays small.
  - Neer will pay for hosting if the project takes off. Choosing and signing up for a provider is
    his call ("Needs Neer" when this milestone starts).

  *Done when:*
  - An ADR is merged comparing hosted and self-hostable options, with monthly costs at 100, 1,000
    and 10,000 users.
  - Sign-in, save, sync and delete-my-data are tested end to end, and edits made offline merge on
    reconnect without loss (a property test).
