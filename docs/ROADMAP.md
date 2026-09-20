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

- [x] **M0.1 Workspace, CI, licenses.**
  - The workspace from `ARCHITECTURE.md` (edition 2024, pinned stable, shared lints and
    dependencies); dual licenses and notices; `deny.toml` denying copyleft; `xtask wasm-check`; CI
    (fmt, clippy, test on three OSes, doc, wasm-check, deny) on **every** PR, no `paths-ignore`;
    README "pre-alpha"; `.gitignore` covers `refs/`, `.autopilot/`, `corpus/`.

  *Done when:*
  - The local gate passes.
  - The PR's CI is green on all 3 operating systems.
  - `cargo deny check` passes.
  - ADR-001 records the license choice and workspace layout.

- [x] **M0.2 Reference library.**
  - `cargo xtask refs fetch|verify|doctor`, driven by `validation/refs.lock.toml`, populates
    `refs/` with RocketPy (pinned tag), the OpenRocket 24.12 jar (sha256 pinned), the public PDFs
    listed in `VALIDATION.md`, `openrocket-database` (pinned commit), ThrustCurve and
    motor.fusionspace.co snapshots, and `fusionspace-loft` plus the private `loft-fixtures` repo
    (skipped with a note when unavailable, as in CI).
  - A `uv`-managed Python venv in `refs/venv` with `rocketpy==1.13.0` and JPype; a Java 17 check.

  *Done when:*
  - `fetch` is idempotent.
  - `verify` checks every hash.
  - `doctor` prints a table of which oracles are runnable.
  - `git status` shows nothing from `refs/`.
  - `THIRD-PARTY-NOTICES.md` lists every source with its license and usage mode (bundled /
    fetched / run-only).

- [x] **M0.3 Lessons from Loft.** Read `refs/fusionspace-loft` (docs/methods, the limitations
  page, `COMPETITION.md`, the importer-bug entries in `BACKLOG.md`, `lib/sim`, `lib/ork`,
  `lib/validation`). Write `docs/research/loft-lessons.md` (at most 200 lines) covering the models
  used, known weaknesses, importer quirks, test cases worth porting, and process mistakes to avoid.

  *Done when:* the file exists, and every quirk or weakness it lists maps to a roadmap milestone or
  a named test to write.

- [x] **M0.4 A documentation site people can read.** Added by Neer on 2026-09-17 (VISION V15,
  CLAUDE.md "Documentation is a deliverable"). Retrofit everything shipped so far; later milestones
  keep the site current. Split it into increments if it is bigger than one session.
  Tool and layout by ADR (mdBook first); each page has one source, equations render on the site
  and on GitHub, and workspace rustdoc is published next to the guide, each linking the other.
  Pages: *Start here*; *Getting started* (a runnable first flight); *How a flight is simulated*;
  one page per model, each opening with *In short*; *Accuracy* (every result, gaps included);
  *Glossary*; *Checking a claim*; the decisions; the roadmap.

  *Done when:* split on 2026-09-17 into M0.4a to M0.4e, which carry its four done-when bullets
  unchanged (M0.4a checks links and bare labels, M0.4b *In short*). Done 2026-09-18, with M0.4d's
  first deploy.

  - [x] **M0.4a The site and its link checks.** An ADR picks the tool (mdBook is the first
    candidate) and the layout. `docs/physics/` and `docs/format/` move into the site's source, so
    each page has one source, and equations render on the site and on GitHub. *Start here* page.
    *Done when:* CI builds the site on every PR and checks its links. A broken link, or a bare
    internal label (`L\d+`, `ADR-\d+`, a milestone id that isn't a link), fails CI, and a test
    shows each failing.
  - [x] **M0.4b Model pages, Accuracy, Glossary, Checking a claim.** Every model page opens with
    *In short*: what it models, its source, how well it is validated, what it leaves out.
    *Accuracy* gives every validation result so far, gaps included, from the committed report.
    The decisions and the roadmap are reachable from the site.
    *Done when:* a model page without *In short* fails CI, a test shows it, and every model page
    passes.
  - [x] **M0.4c Getting started, and how a flight is simulated.** A runnable `examples/` program
    flies a first rocket; *How a flight is simulated* walks pad to landing with a diagram.
    *Done when:* the *Getting started* example runs in CI.
  - [x] **M0.4d Publish.** Workspace rustdoc sits next to the guide, each linking to the
    other.
    *Done when:* a workflow deploys the site and the rustdoc to GitHub Pages from `main`, and the
    README's first lines link to it. (Needs Neer to enable Pages. Until he does, only this bullet
    is blocked.)

    Done 2026-09-18 (ADR-019): Neer turned Pages on, and CI run 35396233336 on `main` deployed
    the guide and the rustdoc to https://nrdptel.github.io/hpr-sim/.
  - [x] **M0.4e The reader test.**

    *Done when:* a reviewer with no project context, given only the site, answers ten questions a
    new user would ask, listed in the PR (for example: "How far can I trust the descent drift, and
    what was it checked against?"). Each answer cites a page. Every term it flags as unclear is
    fixed.

## Phase 1: Physics core (the heart), with validation interleaved

- [x] **M1.1 Core math, frames, Earth.**
  - `hpr-core`: vectors and quaternions (glam f64), interpolation tables (linear/cubic, clamped,
    with extrapolation flags).
  - Frames spec in `docs/physics/frames.md`: ENU launch frame, body frame, Euler conventions,
    geodetic/ECEF conversion.
  - WGS84 Somigliana gravity with altitude; optional Earth-rotation terms.
  - Loft lessons: L1 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - Gravity matches the published formula values at at least 6 latitude/altitude points to 1e-6
    relative.
  - Frame round-trip property tests pass.
  - Quaternion integration keeps the norm within 1e-12 over 1e6 steps in tests.
  - Everything compiles for wasm32.

- [x] **M1.2 Atmosphere and wind.**
  - USSA76 from 0 to 86 km: temperature, pressure, density, speed of sound, dynamic viscosity
    (Sutherland).
  - ISA temperature offset; custom profile from soundings (p, T, RH, wind vs height) with
    interpolation.
  - Wind models: constant, power/log law, tabulated layers, seeded Dryden turbulence.
  - Loft lessons: L2, L3, L4, L5, L6 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - USSA76 matches the tables at at least 25 altitudes to at most 0.1% (the small table fixture is
    committed with its citation).
  - A Dryden spectrum test passes (PSD within tolerance of theory).
  - `docs/physics/atmosphere.md` cites every equation.

- [x] **M1.3 Solid motors.**
  - `.eng` and `.rse` readers and writers (clean room, from the public specs).
  - Motor model: thrust(t), propellant mass by impulse fraction (default) with an optional
    grain-geometry model; CG and inertia over time; nozzle exit area.
  - Delays (including plugged); case/retainer mass. Offline catalog type with per-curve
    provenance and license; bundle only curves with clear terms, the rest cached later (M5).
  - Loft lessons: L36, L37, L38, L39, L40, L41, L42, L43 (`docs/research/loft-lessons.md`).

  *Done when:*
  - For every bundled curve, total impulse, average thrust and burn time match the ThrustCurve
    metadata within 1%.
  - Parse-write-parse round trips are identical.
  - Mass and inertia evolution matches RocketPy's SolidMotor for 3 motors within 1% (reference
    JSON generated by a script in `validation/oracles/rocketpy/`).

- [x] **M1.4 Design model and mass properties.**
  - `hpr-design` components: nose cones (conical, tangent/secant ogive, elliptical, power series,
    parabolic series, Haack/LV-Haack, von Kármán); body tubes, transitions, couplers; fin sets
    (trapezoidal, elliptical, freeform, tube fins); launch lugs, rail buttons; inner tubes, motor
    mounts, centering rings, bulkheads; mass components; parachutes, streamers, shock cords.
  - Cited clean-room materials; stages and configurations; mass, CG and full inertia tensor from
    geometry, with overrides; structural checks with typed warnings. A small public test-design
    set, `validation/designs/`, so tests never snapshot the private corpus.

  *Done when:*
  - Analytic volume, area and CG tests pass for every shape.
  - The inertia tensor of composite test bodies matches hand calculations.
  - Mass, CG and inertia match RocketPy's example rockets where RocketPy exposes them.
  - The OpenRocket stored-value comparison is deferred to M2.2 and noted there.

  - [x] **M1.4a Shapes, materials and component mass properties.**
    - Every nose and transition shape above (clipped or not), solids of revolution filled or with a
      wall, fin planforms, cross-sections and tabs, and every other component listed, each with
      mass, CG and full inertia tensor from geometry in its own frame.
    - Cited materials; `MassProperties` with the parallel-axis theorem and rotations.
    - Loft lessons: L44, L45, L46, L48, L49, L91 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - Analytic volume, area and CG tests pass for every shape.
    - The inertia tensor of composite test bodies matches hand calculations.

  - [x] **M1.4b Design tree, configurations and checks.**
    - Stages and component placement, configurations with motors, overrides, reference diameter,
      structural checks with typed warnings, and `validation/designs/`.
    - Loft lessons: L47, L50 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - Mass, CG and inertia match RocketPy's example rockets where RocketPy exposes them.
    - The OpenRocket stored-value comparison is deferred to M2.2 and noted there.

- [x] **M1.5 Aerodynamics I (subsonic).**
  - Barrowman CNα and CP for every component, with Prandtl–Glauert; body lift at angle of attack;
    fin–body interference.
  - Drag buildup: skin friction (laminar/turbulent with roughness), nose/transition pressure drag,
    base drag power-on and power-off, fin profile and thickness, protuberances; Cd at angle of
    attack.
  - Cd-vs-Mach override tables (power-on/off) from CSV, including RocketPy/RASAero exports, so the
    dynamics are validated on the oracle's drag first. `docs/physics/aero.md` cites each term.

  *Done when:* split below into M1.5a and M1.5b, which carry its three bullets unchanged.

  *Result:* see M1.5a (the Recruiter's six-fin slopes, ADR-008) and M1.5b (Valetudo, ADR-009).
  Skin friction is fully turbulent with roughness, as in Niskanen; laminar and transitional
  friction were not built (ADR-009).

  - [x] **M1.5a Normal force and centre of pressure.**
    - Barrowman CNα and CP for every component with Prandtl–Glauert, body lift at angle of attack,
      fin–body interference, and the `aero.md` sections for them.
    - Loft lessons: L8, L9, L10, L89 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - CNα and CP reproduce Barrowman's worked example(s) within 1%.

  - [x] **M1.5b Drag and override tables.**
    - The drag buildup, Cd at angle of attack, Cd-vs-Mach override tables from CSV, and the `aero.md`
      sections for them.
    - Loft lessons: L11, L12, L13, L14, L15, L16, L90 (tests named in
      `docs/research/loft-lessons.md`).
    *Done when:*
    - Subsonic Cd for the RocketPy example rockets is within 10% of their RASAero CSVs at Mach 0.3
      (tighten this later).
    - Unit tests cover every drag term's limits.

    *Result (ADR-009):* not met for Valetudo (−47% power-off, −50% power-on; its table is 1.44 times
    its own OpenRocket export, which hpr matches to 2%) or Cavour power-on (−18.3%, cause open).
    Calisto, Juno III and Cavour power-off are within 10% (+4.4%, −6.0%, −8.3%) under a declared
    input rule that can't pin the unrecorded inputs.

- [x] **M1.6 6-DOF flight engine.**
  - State: position, velocity, attitude quaternion, angular velocity, time-varying mass
    properties. A guided rail/tower phase with friction and rail-button geometry; powered and
    coast phases with jet damping.
  - Adaptive Dormand–Prince 5(4) with dense output and event root-finding (liftoff, rail exit,
    burnout, apogee, ground hit, user events); fixed-step RK4 option.
  - Recorder with a configurable channel set; observer trait; `criterion` benchmark.

  *Done when:*
  - Analytic tests pass: vacuum ballistic, terminal velocity, torque-free precession, and pitch
    oscillation frequency vs linear theory.
  - Step-halving convergence shows the expected order.
  - Events are located to ≤1e-6 s.
  - A single typical L2 flight simulates in ≤5 ms release-mode (number recorded in
    `docs/perf.md`).

  - [x] **M1.6a Integrator and events.**
    - Adaptive Dormand–Prince 5(4) with dense output, event root-finding and stop times that put
      discontinuities on step boundaries; the fixed-step RK4 option; `docs/physics/integration.md`.
    - Loft lessons: L21, L22, L23 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - Step-halving convergence shows the expected order.
    - Events are located to ≤1e-6 s.

  - [x] **M1.6b Rigid-body flight.**
    - The state, the rail phase, powered and coast phases with jet damping, the flight events, the
      recorder and observer, and the `criterion` benchmark.
    - Loft lessons: L20, L24, L25, L26 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - Analytic tests pass: vacuum ballistic, terminal velocity, torque-free precession, and pitch
      oscillation frequency vs linear theory.
    - A single typical L2 flight simulates in ≤5 ms release-mode (number recorded in
      `docs/perf.md`).

- [x] **M1.7 Recovery.**
  - Parachutes (Cd·S, inflation time or area-growth model), streamers, tumble.
  - Drogue and main with deployment triggers (apogee, altitude, timer, motor delay).
  - Descent with wind drift; separated bodies tracked independently; landing detection.

  *Done when:*
  - Analytic tests for terminal velocity, descent time and drift pass.
  - Descent rate and drift match RocketPy's for 3 example rockets within 3%.

  *Result:* met by M1.7a; M1.7b and M1.7c add the streamers, tumble and separation the entry
  lists (ADR-012, ADR-013, ADR-014).

  - [x] **M1.7a Parachutes and descent.**
    - Parachutes (Cd·S, inflation time or area-growth model), drogue and main with deployment
      triggers (apogee, altitude, timer, motor delay), drogue release, descent with wind drift
      and landing detection.
    - Loft lessons: L27, L28, L29, L92 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - Analytic tests for terminal velocity, descent time and drift pass.
    - Descent rate and drift match RocketPy's for 3 example rockets within 3%.

    *Result (ADR-012):* met. Analytic descents to 2.1e-8; five RocketPy examples within 0.71%
    (time), 0.03% (rate), 0.27% (drift); worst drift component 2.87% (NDRT's added mass).

  - [x] **M1.7b Streamers and tumble.**
    - Streamers and tumble, each with a cited drag model.

    *Done when:*
    - A streamer's and a tumbling body's descent rates match the terminal velocity of their cited
      drag models (analytic tests).
    *Result (ADR-013):* met. Streamers: Carruthers and Filippone (within 9% of Kidwell's flat
    streamer; appendix C 88% fast). Tumble: OpenRocket §3.5, −10 to +19% on its own drop tests.

  - [x] **M1.7c Separated bodies.**
    - Separation, with every body flown to its own landing and its own mass properties and drag.

    *Done when:*
    - A separation gives every body a landing, and the bodies' masses sum to the rocket's.

    *Result (ADR-014):* met. A `Separation` splits the stack at a stage boundary, each body a point
    mass under its devices; on the two-stage test design both land (2.11 m/s under a canopy, 16.74
    m/s tumbling), masses to 1e-12, momenta to 1e-9. Every body carries a device.

- [x] **M2.1 Validation harness plus the RocketPy code-to-code suite.** The first end-to-end
  milestone: `hpr-validate` and `cargo xtask validate [--fast]`, TOML cases and reference JSON with
  provenance, Markdown and JSON reports, oracle scripts in `validation/oracles/rocketpy/`. Metrics
  cover apogee and its time, maximum velocity, Mach and acceleration, rail exit, burnout, descent
  rates, landing offset and time-series RMS after alignment. At least five RocketPy example
  rockets fly in two modes — **same-drag**, which isolates dynamics, environment and motor, and
  **predicted**, hpr's own aero, whose supersonic gaps are reported rather than hidden — and CI
  compares against the stored references.

  *Done when:*
  - At least 5 cases pass their same-drag tolerances.
  - Predicted-mode results are reported, with explained gaps.
  - `validation/reports/latest.md` is generated.
  - The CI job is green.
  - A separate, manually triggered workflow regenerates the references.

  - [x] **M2.1a The harness.**
    - `hpr-validate` and `cargo xtask validate [--fast]`: case files (TOML), reference JSON with
      provenance, per-case tolerances on every metric, a case lock so a silently skipped case
      fails, and Markdown plus JSON reports.
    - Its first cases are the recovery descents, whose references M1.7a already generated.
    - Loft lessons: L76, L77, L78, L79 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - `cargo xtask validate` runs every case in the lock against its stored reference and writes
      `validation/reports/latest.md`.
    - A case whose metric has no tolerance, a reference value with no provenance, and a run with
      fewer cases than the lock expects each fail.
    *Result (ADR-015):* met. Five descent cases, 30 metrics, all within 3% with no floor (largest
    NDRT's northward drift, +2.86%); the command refuses every malformed case the done-when names
    and more, with twelve tests, four of them L76–L79. Valetudo's northward drift first read 28x
    RocketPy's: hpr's gravity had a horizontal part RocketPy's lacks (issue #27).

  - [x] **M2.1b Whole flights against RocketPy, same-drag.**
    - A `validation/oracles/rocketpy/flight.py` generator (M2.1b1) and a `Flight::WholeFlight`
      case variant taking the oracle's `C_D0(M)` through `Simulation::with_drag_table` (M2.1b2).
    - Loft lessons: L75 (tests named in `docs/research/loft-lessons.md`).

    *Done when:* split below into M2.1b1 and M2.1b2; M2.1b2 carries M2.1b's three bullets.

    - [x] **M2.1b1 The whole-flight oracle.**
      - `validation/oracles/rocketpy/flight.py` flies M2.1b's five example rockets pad to landing,
        built from `validation/fixtures/design/rocketpy-rocket-mass.json` as `recovery.py` builds
        them (rail, inclination and heading cited per case), on drag **declared by the case**:
        RocketPy's data files carry their own terms (ADR-009) and are absent from M2.1c's CI.
      *Done when:*
      - `refs/venv/bin/python validation/oracles/rocketpy/flight.py` writes a fixture of all five
        cases, each with the metrics M2.1 names, a time series, a loose-solver run and a source for
        every value, and re-running it reproduces the committed fixture byte for byte.
      - No RocketPy data file is committed, and `git status` shows nothing from `refs/`.
      - Every case's declared drag table is argued in the generator, with its source.

      *Result:* met. A declared constant `C_D0` of 0.5; the fixture reproduces byte for byte;
      apogees 779 to 3,623 m AGL, Prometheus to Mach 1.014. The oracle's own step-size cliff
      (thrust(0) = 0, an unbounded step) was fixed by bounding `max_time_step` (issue #33).

    - [x] **M2.1b2 The whole-flight cases.**
      - A `Flight::WholeFlight` case variant beside `RecoveryDescent`, taking the case's `C_D0(M)`
        through `Simulation::with_drag_table`, and the five cases in the lock.
      - Loft lessons: L75 (tests named in `docs/research/loft-lessons.md`).

      *Done when:*
      - At least 5 whole-flight cases run in the lock and pass their same-drag tolerances, with the
        tolerance for each metric argued in the case file.
      - `hpr_validate::rocketpy::tests::oracle_inputs_come_from_the_case_file_not_hpr_outputs`
        exists and passes.
      - `validation/reports/latest.md` carries them, and the gravity rule of ADR-015 is applied:
        the comparison flies the oracle's models where hpr has them.
      *Result (ADR-021):* met. Six whole-flight cases, fifteen metrics each; five pass every scored
      metric within 3% (largest +1.783%, Bella Lui's power-on peak). The drifts in wind stayed
      unscored until issue #50 (M2.1d3); Prometheus was a checked `M ≥ 1` gap until M1.8a. The
      comparison flies RocketPy's gravity, atmosphere, rail and thrust.

  - [x] **M2.1c Predicted mode, CI and regeneration.**
    - The same cases flown with hpr's own aero, reported beside the same-drag ones.
    - A CI job that runs `cargo xtask validate` against the stored references, and a separate,
      manually triggered workflow that regenerates them.
    *Done when:* split below into M2.1c1 and M2.1c2, which carry M2.1c's three bullets between
    them (the first in M2.1c2, the other two in M2.1c1).

    - [x] **M2.1c1 The CI job and the regeneration workflow.**
      - `cargo xtask validate --check`, and a `validate` job running it on three OSes.
      - `scripts/regenerate-references.sh` and a `workflow_dispatch`-only workflow that runs it.

      *Done when:*
      - The CI job is green on macOS, Windows and Linux.
      - The regeneration workflow runs only when a human triggers it, and its output is a diff to
        review, never an automatic commit.
      *Result (ADR-022):* met. `cargo xtask validate --check` writes nothing and fails on a metric
      outside tolerance or a committed report this run does not reproduce; CI runs it on three
      OSes. *Regenerate references* is `workflow_dispatch` only, with a read-only token, and
      uploads the diff; run locally it reproduced every fixture and the report byte for byte.

    - [x] **M2.1c2 Predicted mode.**
      - The same cases flown with hpr's own aero, against a reference in which RocketPy flies each
        example's own drag curves; results computed from those curves are committed, the curves
        are not (ADR-009).
      *Done when:*
      - Predicted-mode results are in the report for every case, each gap explained in the case
        file or `docs/VALIDATION.md`; `M ≥ 1` cases are reported as gaps, not hidden, until M1.8.
      *Result (ADR-023):* met. `flight.py --own-drag` (hashes, never values). Six `predicted-*`
      cases, 3% targets: 56 of 75 within; Valetudo and NDRT 2020 +10% in apogee; misses pinned.

  - [x] **M2.1d The time-series RMS and the path in wind.**
    - The two items of M2.1's list that M2.1a to M2.1c leave open: the time-series RMS after
      alignment (each whole-flight fixture already carries its series), and the landing offset,
      reported but not scored until issue #50 finds why hpr turns into the wind less than RocketPy.
    *Done when:* split below into M2.1d1 to M2.1d3, which carry these two bullets between them.

    - [x] **M2.1d1 The time-series RMS.**

      *Done when:*
      - Every whole-flight case reports its time-series RMS after alignment against the
        reference's series, gated with its tolerance argued in the case file.
      *Result (ADR-024):* met for every case hpr flies: RMS at RocketPy's 120 series times, held to
      3% of apogee and max speed; same-drag 1.4–39.2 m and 0.13–2.06 m/s, all pass; predicted,
      three outside (the drag), pinned.

    - [x] **M2.1d2 The calm-air cases (issue #50).**
      - Issue #50's zero-wind runs of Juno III, Calisto and Bella Lui, committed as same-drag
        whole-flight cases with RocketPy references, to measure the wind's effect against.
      *Done when:*
      - The three calm-air cases are in the suite, their apogee and landing drifts scored at 3%,
        and each passes or is a gap its case file explains.
      *Result (ADR-025):* met. Calisto and Bella Lui pass (drifts −1.258% to −2.583%); Juno III's
      drifts miss (−3.7%), reported not scored: 1.6 points are the rail release (`rail_release.py`).

    - [x] **M2.1d3 The path in wind (issue #50).**
      - Fly the windy cases with each suspected cause of the gap matched to RocketPy in turn: rail
        release at the first rail button, drag without the angle-of-attack factor, and each code's
        normal force and damping.
      *Done when:*
      - Issue #50's cause is found and the drifts are scored within their tolerances, or an ADR
        records the measured cause and why they cannot be, and the gap stays visible in the
        report.
      *Result (ADR-026):* met. Mostly RocketPy's: in the burn it took moments about a point
      mirrored across the dry centre of mass (#1186, PR #1196; PR #1188), both corrected in
      `corrections.py`. `wind_response.py` measures the rest: body lift, the last-button release
      and Juno III's thin fins put RocketPy within 1.4% of hpr in wind. Six drifts gated; five not.

- [ ] **M1.8 Aerodynamics II (transonic and supersonic, damping, overrides).**
  - Transonic drag rise and supersonic wave drag.
  - Supersonic fin normal force (Ackeret/Busemann or a documented alternative).
  - CP shift with Mach.
  - Pitch, yaw and roll damping; roll forcing from cant.
  - Extend the M1.5 override tables to CNα and CP vs Mach and AoA, importable from RASAero CSV.
  - Loft lessons: L7, L17, L18 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - Cd vs Mach is within 10% of RocketPy's RASAero CSVs across Mach 0.1–2.0 for the available
    rockets. Per-band errors are in the report.
  - The supersonic cases in the M2.1 suite are within their tolerances.
  - Roll-rate steady state matches the analytic cant/damping balance.

  Split into M1.8a to M1.8e. The measured reference throughout is NASA's Arcas Robin wind-tunnel
  model: TN D-4013 (Mach 0.6–1.2) and TN D-4014 (Mach 1.5–4.63).

  - [x] **M1.8a Normal force and centre of pressure through Mach 1.**
    - Fin slope through the transonic region to supersonic linear theory, and the fin CP shift
      with Mach. The normal force accepts Mach numbers past 1, so a flight on a drag table flies
      through Mach 1. Loft lesson L7.
    *Done when:*
    - L7's test passes: the fin slope and CP are Barrowman's at Mach 0 and change with Mach.
    - A committed fixture, pinned by a test, holds hpr's `C_Nα` and CP against two references:
      the Arcas Robin measurements at every Mach they give, and RASAero II's Calisto export from
      Mach 0.1 to 2.0. Targets, set before measuring: CP within 0.5 calibers, `C_Nα` within 15%.
      Every miss is explained.
    - The same-drag Prometheus 2022 case flies through Mach 1 and passes its tolerances.

    *Result (ADR-027):* met. Linear theory from `M_s`, a join from Mach 0.8. L7 passes; 37 rows
    pinned, 16 outside the targets, explained. Mach 1.5–2.96: `C_Nα` −13.4% to +3.3%, CP within
    0.42 calibers; past Mach 3, 17–25% low (M1.8e). Prometheus flies through Mach 1.010. Since
    M1.8e6's body lift (ADR-037): 17 outside; Mach 1.5–2.96 −16.3% to +1.4%, CP within 0.47.

  - [x] **M1.8b Transonic and supersonic drag.** Every drag term's transonic and supersonic
    branch, and nose wave drag. Loft lessons L17 and L18.
    *Done when:* M1.8's Cd bullet is met or an ADR records why not, with the gap in the report;
    the Arcas Robin's measured axial force is compared; the predicted Prometheus case flies.
    Split below into M1.8b1 to M1.8b3, which carry these bullets between them.

    - [x] **M1.8b1 The drag buildup through Mach 1.**
      - Nose, shoulder and step pressure drag through Mach 1 (Niskanen eq. 3.87 and appendix B,
        Stoney's fineness-3 curves); the buildup accepts Mach 0 to 5. Loft lesson L17.
      *Done when:* L17's test passes; the predicted Prometheus case flies; the Arcas Robin's
      measured axial force is compared: a committed fixture, pinned by a test, holds hpr's
      forebody drag against TN D-4013's and TN D-4014's at every Mach they give, fins on and off.
      *Result (ADR-028):* met. L17's test passes; `drag_against_mach` pins 44 rows, 8 within 10%
      (misses: blunt fin edges, the base lip, the boattail rule). Predicted Prometheus flies.

    - [x] **M1.8b2 Drag against RASAero through Mach 2.** Cd against Mach from RocketPy's RASAero
      CSVs, per band (L18). *Done when:* M1.8's Cd bullet is met or an ADR records why not.
      *Result (ADR-029):* not met, recorded. Calisto's export: 15/15 subsonic, 2/7 transonic,
      0/17 supersonic within 10%; no fin input is within 10% subsonic and supersonic both.
      MIL-HDBK-762's worked example: 6/12, the body 6–10% low past Mach 1.6. Boattail a candidate.

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
    forcing is compared with the Arcas Robin's measured roll effectiveness (TN D-4014).
    *Result (ADR-031):* met. Barrowman's strip theory with his body factors. Valetudo canted 1° at
    100 m/s settles on the closed-form balance, −16.948 rad/s, within 1e-11. Against TN D-4014:
    from Mach 2.3, 8 of 8 within 5.3%; at Mach 1.5 and 1.8, +14.3% to +47.8%. Damping against the
    Basic Finner: −5.9% to −16.2%.

  - [x] **M1.8d Normal-force overrides.** `C_Nα` and CP tables against Mach and angle of attack
    from a RASAero II export. *Done when:* its `C_Nα` and CP columns replace hpr's in a flight,
    and the reading is tested on the Calisto export.
    *Result (ADR-032):* met. The table's force at the centre of mass's flow, hpr's damping kept.
    Calisto's export (0°, 2°, 4°): 4,999 rows re-read with `refs/`, M1.8a's 30 values in CI;
    Calisto flies on it; a table's pitch period within 4e-6 of linear theory, pitch and yaw.

  - [ ] **M1.8e The body's supersonic normal force.** M1.8a measured the gap: past Mach 3 the
    Arcas Robin's body alone lifts 3.9 to 4.6 per rad, where slender-body theory gives 2.3 to 2.8
    with body lift. A cited supersonic method for noses, boattails and crossflow. *Done when:*
    the Arcas Robin's body-alone `C_Nα` (fins off, TN D-4014) is within 15% at every Mach number
    from 1.5, and both configurations' `C_Nα` within 15% at Mach 3.96 and 4.63, or an ADR records
    why not with the gap in the report. Split below into M1.8e1 to e12; e9 judged this bullet.

    - [x] **M1.8e1 The second-order shock-expansion method.** NACA TN 3527's method for a
      pointed body's `C_Nα` and CP at `α → 0`, the cylinder's lift behind the nose included; its
      Fig. 2's tangent-cone slopes read by hand. *Done when* (targets set before measuring):
      - A committed fixture, pinned by a test, holds hpr's values against every row of TN 3527's
        Tables I and II (cones and tangent ogives of fineness 3, 5 and 7, cylinders of 0 to 10
        calibers, Mach 3 to 6.28): within 0.05 per radian and 0.1 calibers of its second-order
        values, and within its stated ±0.2 of its measurements, every miss explained.
      - The Arcas Robin's nose and cylinder, with and without its boattail (footnote 8), at each
        Mach number of TN D-4014, beside the measured body alone.
      *Result (ADR-033):* not met, recorded: slopes and CPs 102 and 125 of 144 within its values
      (#81 at its limit), 117 and 109 of 120 of its measurements; Arcas Robin −18.7% to +16.4%.
    - [x] **M1.8e2 The body's supersonic normal force in flight.** The body's terms take Mach:
      M1.8e1's method for a pointed nose and its cylinder where it holds, joined to slender-body
      theory below it. *Done when* (targets set before measuring):
      - A flight takes the body's `C_Nα` and CP at its Mach number; a test probes the join at
        ±1e-9 in Mach and finds no jump.
      - The Arcas Robin's body alone (TN D-4014) through the flight's path at each Mach number
        from 1.5, beside the measurement and M1.8a's, in the regenerated report, its changed rows
        listed in the PR.
    - [x] **M1.8e3 The supersonic join's start without grid steps** (#87's grid half). *Done when*
      (set after building): a test finds a 20° cone's start off the grid, moved under 1e-7 in Mach
      by 1e-6° and strictly by each 0.1° to 20.5°, its cylinder share under 1e-5, no jump at ±1e-9
      at the join's ends or first even row. *Result:* met: Mach 1.341910 (was 1.35); report same.
    - [x] **M1.8e4 The boattail's share faster than sound** (footnote 8; a station rule for shares
      that cross zero). *Done when:* a boattailed body flies with no jump at ±1e-9 in Mach; the
      long Arcas Robin through a flight's path in the report. *Result:* met, long to −27.0%.
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
      ADR-040). *Done when:* #90 closed (a bound on the boattail angle, and footnote 8's size
      pinned by a hand calculation); M1.8e's 15% bullet met for the Arcas Robin's body alone, or an
      ADR records why not with the gap in the report.
      *Result:* met; the correlation is read no steeper than 16° and the boattail with its tube
      integrated by hand; the bullet met at Mach 3.96 and 4.63, the body alone outside on six rows.
    - [x] **M1.8e10 The lip's shelter, weighed not switched.** #87's five switches all flip one
      gate — whether the method covers the body at all — so each is worth the whole body. The
      largest is the lip's, and the drag buildup already grades its shelter continuously
      (`share_by_rise`, a quarter to a half of the boattail's drop) where the normal force reads
      a threshold. *Done when:* every switch's size is measured by a test; the lip's is gone, its
      two sides agreeing across the old threshold in proportion to the change in shape; no
      committed fixture moves; an ADR records the weight.
      *Result:* met (ADR-041); the lip's **rise** is a weight, not a switch, and five keep measured
      sizes — a step −8.7%/1.03 cal, a flare −27.5%/0.29 cal, a pointed tip −10.4%/1.14 cal, a
      vertical tip −7.0%/0.64 cal, and the lip's own length −33.0%/1.77 cal, which #87 didn't list.
    - [x] **M1.8e11 Cone slopes past Fig. 2's edge, from Sims.** NASA SP-3007 (pinned) tabulates
      the same theory to 30°, where TN 3527's Fig. 2 stops at 24°. *Done when:* the method flies a
      tangent cone of 24° to 30°, a fineness-1 cone among them, from Sims's slopes checked against
      Fig. 2 where they overlap; no committed fixture moves.
      *Result:* met (ADR-042); SP-3007 Table 2 at 25°, 27.5° and 30°, agreeing with the chart to
      0.0021 per rad at 22.5°; the pointed tip's switch moves to 30° and falls to −7.7%/0.81 cal.
    - [ ] **M1.8e12 The blunt tip's handover, past 24°.** With slopes to 30° a cap can hand over at
      the detachment angle, not Fig. 2's edge. *Done when:* the vertical-tip switch is gone or
      measured again, fixtures and the guide moving together.
    - [ ] **M1.8e13 The step and the flare.** *Done when:* #87 closed or narrowed to these two, each switch's measured size in an ADR and the guide.

- [ ] **M3.1 OpenRocket `.ork` import.**
  - Handles zip, gz and raw XML, schema 1.0 to 1.10, plus the documented 1.11 additions.
  - Reads components, materials, finishes, motor configurations, recovery, stages, and stored
    simulation results.
  - Unknown content is kept in `extensions.x-openrocket` for a lossless round trip.
  - Graceful warnings instead of failures.
  - Port Loft's importer lessons (auto radii, stage boundaries).
  - Loft lessons: L49, L56, L57, L58, L59, L60, L61, L62, L63, L64, L65, L66 (tests named in
    `docs/research/loft-lessons.md`).

  *Done when:*
  - Every `.ork` in `refs/loft-fixtures` and the OR example set imports with zero errors.
  - Committed `insta` snapshots use only public files (the Loft demo fixtures and synthetic
    designs).
  - Private-corpus results go to a gitignored `corpus-out/` and are summarised as counts only.
  - The RocketSerializer cross-check agrees on the key geometry.

- [ ] **M2.2 OpenRocket oracle and corpus.**
  - `validation/oracles/openrocket/` (JPype, OR 24.12) flies the OR examples and the
    corpus.
  - The stored results inside the `.ork` files are used as a second reference.
  - The deferred M1.4 mass/CG checks run against OR values.
  - Loft lessons: L19, L51, L80, L81, L82, L87 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - At least 20 designs are in the report with an error distribution (apogee, max velocity,
    stability margin, mass, CG).
  - Every design with apogee error above 5% has a written hypothesis.
  - Private designs appear only as anonymised ids.

- [ ] **M1.9 Staging, clusters, airstarts (COTS).**
  - Stage separation triggers (burnout plus delay, altitude, time); sustainer ignition.
  - Booster tracked through recovery.
  - Clustered motor mounts, with mass and thrust summed and the thrust offset handled.
  - Loft lessons: L30, L31, L93 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - A two-stage design and a cluster design each match OpenRocket within the per-case tolerance.
  - Event ordering tests pass.

- [ ] **M1.10 Outputs and derived metrics.**
  - Static and dynamic stability margin over the flight.
  - Optimum ejection delay; max q; flutter velocity and margin (primary source cited).
  - Landing point in lat/lon.
  - Exports: CSV, JSON, Parquet (feature), KML and GeoJSON.
  - Loft lessons: L32, L33, L34, L35, L94 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - Metrics are unit-tested.
  - Exported files are validated (GeoJSON by schema, KML by parsing).
  - Flutter matches the worked example in the cited source.

- [ ] **M2.3 Real flights.**
  - Cases from the RocketPy flight data with their ERA5 environments, which needs a weather-file
    reader: a netCDF reader or a documented conversion.
  - Also the corpus flights that have logs.
  - Loft lessons: L83 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - At least 6 real flights are in the report, with apogee error and altitude-trace RMS.
  - Mean absolute apogee error is reported against the 5% target.
  - Each outlier has an explanation.

- [ ] **M2.4 Accuracy census gate.** Generate a summary census (a README table and badge) from the
  report. CI fails on any per-case regression beyond tolerance.

  - Loft lessons: L84, L85, L86, L88 (tests named in `docs/research/loft-lessons.md`).

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

  - Loft lessons: L95 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - The examples run in CI.
  - rustdoc has zero warnings.
  - A "custom aero model" example overrides a built-in model through the trait.

- [ ] **M4.2 CLI.** `hpr sim|validate|convert|motors|mc|optimize|compare|diagnose` (stubs are fine
  for commands whose milestone hasn't come yet), `--json` everywhere, and shell completions. The
  README's command and format table is generated from the registered commands (Loft lesson P10).

  *Done when:* `assert_cmd` tests cover every implemented command and the JSON output validates
  against the published schemas.

- [ ] **M3.2 OpenRocket `.ork` export** (schema 1.10).

  - Loft lessons: L67, L68 (tests named in `docs/research/loft-lessons.md`).

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
  - Loft lessons: L52, L53, L54, L55, L96 (tests named in `docs/research/loft-lessons.md`).

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

  - Loft lessons: L69, L70, L71 (tests named in `docs/research/loft-lessons.md`).

  *Done when:* the corpus `.rkt` files import, and the exports reopen in our importer with
  semantic equality.

- [ ] **M3.5 RASAero `.CDX1` import/export** (from samples only). Fix the Loft `<Location>` bug
  class.

  - Loft lessons: L72, L73, L74 (tests named in `docs/research/loft-lessons.md`).

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

- [ ] **M7.1 Flight log importers.**
  - AltOS CSV (TeleMetrum/TeleMega/EasyMega), RRC3/Missile Works, Eggtimer, Featherweight
    Raven/Blue Raven, PerfectFlite, CATS.
  - The EuRoC/Juno CSV layouts; generic CSV with column mapping; unit detection.

  *Done when:* every sample file in refs imports, snapshot-tested. Formats without samples are
  listed as "needs samples" in `STATUS.md`.

- [ ] **M7.2 Reconstruction and ghost data.**
  - RTS/Kalman smoothing fusing baro, accel and GNSS.
  - Liftoff detection and time alignment.
  - Sim-vs-real residuals.
  - A "ghost" data product: time-synced trajectories in a common frame, exported as JSON and
    CZML/glTF-friendly tracks.

  *Done when:*
  - On synthetic data (a simulated flight plus a noise model), the smoother recovers the truth
    within the stated error.
  - Real RocketPy flights produce ghost files.

- [ ] **M7.3 Parameter identification.** Fit Cd scale, mass, motor impulse scale and wind to a log
  (Levenberg–Marquardt plus a Bayesian option with uncertainty).

  *Done when:* synthetic-truth recovery is within tolerance, and real-flight fits are reported.

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
  - Loft lessons: L97 (tests named in `docs/research/loft-lessons.md`).

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
