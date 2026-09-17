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
  - Cargo workspace with the crate skeletons from `ARCHITECTURE.md` (empty crates are fine).
  - `rust-toolchain.toml` pinned to the current stable.
  - Edition 2024; `rustfmt.toml`, clippy lints and `[workspace.dependencies]`.
  - Dual `LICENSE-MIT` and `LICENSE-APACHE`; `THIRD-PARTY-NOTICES.md` started.
  - `deny.toml` (cargo-deny: licenses allowlist with copyleft denied, advisories, bans).
  - `xtask` with `wasm-check` (checks the pure crates for `wasm32-unknown-unknown`).
  - GitHub Actions: fmt, clippy, test (ubuntu/macos/windows), doc, wasm-check, deny. Rust cache on.
    CI runs on **every** PR with no `paths-ignore`, so docs-only PRs also get green checks to merge
    on.
  - README with status "pre-alpha" and the project goals.
  - `.gitignore` already covers `refs/`, `.autopilot/`, `corpus/`.

  *Done when:*
  - The local gate passes.
  - The PR's CI is green on all 3 operating systems.
  - `cargo deny check` passes.
  - ADR-001 records the license choice and workspace layout.

- [x] **M0.2 Reference library.**
  - `cargo xtask refs fetch|verify|doctor`, driven by `validation/refs.lock.toml`. It populates
    `refs/` with:
    - RocketPy (pinned tag; shallow clone).
    - The OpenRocket 24.12 jar (sha256 pinned).
    - The public PDFs listed in `VALIDATION.md`.
    - `openrocket-database` (pinned commit).
    - A ThrustCurve metadata snapshot and a motor.fusionspace.co snapshot.
    - `fusionspace-loft` (public, MIT) and the private `loft-fixtures` repo. `scripts/preflight.sh`
      normally clones both already; if not, clone them with the user's `gh` credentials, or skip
      with a note when unavailable, as in CI.
  - A `uv`-managed Python venv in `refs/venv` with `rocketpy==1.13.0` and orhelper from git.
  - A Java 17+ check.

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
  - Delays (including plugged); case/retainer mass.
  - Offline catalog type with per-curve provenance and license. Bundle only curves with clear
    terms; the rest are fetched and cached later (M5).
  - Loft lessons: L36, L37, L38, L39, L40, L41, L42, L43 (tests named in
    `docs/research/loft-lessons.md`).

  *Done when:*
  - For every bundled curve, total impulse, average thrust and burn time match the ThrustCurve
    metadata within 1%.
  - Parse-write-parse round trips are identical.
  - Mass and inertia evolution matches RocketPy's SolidMotor for 3 motors within 1% (reference
    JSON generated by a script in `validation/oracles/rocketpy/`).

- [x] **M1.4 Design model and mass properties.**
  - `hpr-design` components:
    - Nose cones: conical, tangent/secant ogive, elliptical, power series, parabolic series,
      Haack/LV-Haack, von Kármán.
    - Body tubes, transitions, and couplers.
    - Fin sets: trapezoidal, elliptical, freeform, tube fins.
    - Launch lugs and rail buttons.
    - Inner tubes and motor mounts, centering rings, bulkheads.
    - Mass components; parachutes, streamers and shock cords.
  - Materials: clean-room values, each cited.
  - Stages and configurations; mass, CG and full inertia tensor from geometry, with overrides.
  - Structural design checks (motor fits the mount, etc.) returning typed warnings.
  - A small public test-design set under `validation/designs/`, built from the RocketPy examples
    and synthetic rockets. Tests use it because the private corpus must not appear in committed
    snapshots.

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
  - Barrowman CNα and CP for every component, with Prandtl–Glauert correction.
  - Body lift at angle of attack; fin–body interference.
  - Drag buildup: skin friction (laminar/turbulent with roughness), nose/transition pressure drag,
    base drag power-on and power-off, fin profile and thickness, protuberances.
  - Cd at angle of attack.
  - Override tables: Cd vs Mach (power-on/off) imported from CSV, including RocketPy/RASAero
    exports. The dynamics can then be validated with the same drag as the oracle before our own
    aero predictions are compared.
  - `docs/physics/aero.md` with a citation for each term.

  *Done when:*
  - CNα and CP reproduce Barrowman's worked example(s) within 1%.
  - Subsonic Cd for the RocketPy example rockets is within 10% of their RASAero CSVs at Mach 0.3
    (tighten this later).
  - Unit tests cover every drag term's limits.

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
    properties.
  - Rail/tower phase: guided, with friction and rail-button geometry.
  - Powered and coast phases with jet damping.
  - Adaptive Dormand–Prince 5(4) with dense output and event root-finding (liftoff, rail exit,
    burnout, apogee, ground hit, user events); fixed-step RK4 option.
  - Recorder with a configurable channel set; observer trait.
  - `criterion` benchmark.

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

- [ ] **M1.7 Recovery.**
  - Parachutes (Cd·S, inflation time or area-growth model), streamers, tumble.
  - Drogue and main with deployment triggers (apogee, altitude, timer, motor delay).
  - Descent with wind drift; separated bodies tracked independently; landing detection.

  *Done when:*
  - Analytic tests for terminal velocity, descent time and drift pass.
  - Descent rate and drift match RocketPy's for 3 example rockets within 3%.

  - [x] **M1.7a Parachutes and descent.**
    - Parachutes (Cd·S, inflation time or area-growth model), drogue and main with deployment
      triggers (apogee, altitude, timer, motor delay), drogue release, descent with wind drift
      and landing detection.
    - Loft lessons: L27, L28, L29, L92 (tests named in `docs/research/loft-lessons.md`).

    *Done when:*
    - Analytic tests for terminal velocity, descent time and drift pass.
    - Descent rate and drift match RocketPy's for 3 example rockets within 3%.

    *Result (ADR-012):* met. Knacke's `v_e` reproduces Loft's case (5.294 m/s); a descent follows
    the closed-form fall to 2.1e-8 of `v_t`; drift is the wind times the time of flight to 1e-8.
    Five RocketPy examples agree within 0.71% in descent time, 0.03% in impact descent rate and
    0.27% in drift, worst single drift component 2.87% (NDRT, where RocketPy's added mass is
    nearly the rocket's).

  - [x] **M1.7b Streamers and tumble.**
    - Streamers and tumble, each with a cited drag model.

    *Done when:*
    - A streamer's and a tumbling body's descent rates match the terminal velocity of their cited
      drag models (analytic tests).

    *Result (ADR-013):* met. Streamers take all three of Carruthers and Filippone's printed
    curves by default and OpenRocket's appendix C on request; tumble takes OpenRocket's §3.5 with
    its fin efficiency table. Against Kidwell's 2001 drop tests the default is within 9% on his
    one flat streamer, where appendix C is 88% fast. The tumble model reproduces its own drop
    tests to −10 to +19%, not the 3 to 14% its source claims, and the docs say so.

  - [ ] **M1.7c Separated bodies.**
    - Separation, with every body flown to its own landing and its own mass properties and drag.

    *Done when:*
    - A separation gives every body a landing, and the bodies' masses sum to the rocket's.

- [ ] **M2.1 Validation harness plus the RocketPy code-to-code suite.** This is the first
  end-to-end milestone.
  - `hpr-validate` and `cargo xtask validate [--fast]`.
  - Case files (TOML) and reference JSON with provenance.
  - Metrics:
    - Apogee and time to apogee.
    - Maximum velocity, Mach and acceleration.
    - Rail-exit velocity; burnout altitude and velocity.
    - Descent rates; landing offset.
    - Time-series RMS after alignment.
  - Markdown and JSON reports.
  - Generator scripts: `validation/oracles/rocketpy/`.
  - Rebuild at least 5 RocketPy example rockets in hpr-sim (for example Calisto, Bella Lui,
    NDRT 2020, Prometheus, Juno III) with a matching environment.
  - Run each case in two modes:
    - **same-drag:** hpr uses the oracle's Cd(M) tables, which isolates dynamics, environment and
      motor.
    - **predicted:** hpr uses its own aero. Supersonic predicted-mode gaps are expected until M1.8
      and are reported, not hidden.
  - A CI job compares against the stored references.
  - Loft lessons: L75, L76, L77, L78, L79 (tests named in `docs/research/loft-lessons.md`).

  *Done when:*
  - At least 5 cases pass their same-drag tolerances.
  - Predicted-mode results are reported, with explained gaps.
  - `validation/reports/latest.md` is generated.
  - The CI job is green.
  - A separate, manually triggered workflow regenerates the references.

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
  - `validation/oracles/openrocket/` (orhelper/JPype, OR 24.12) flies the OR examples and the
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
