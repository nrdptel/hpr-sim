# Performance

Measured numbers only, newest first within each section. Record the machine, the toolchain, and
the command, so a later run can be compared like for like.

## Build memory during an unattended run

- **Benchmark:** `scripts/build-memory.sh`, which builds into an empty `CARGO_TARGET_DIR` and sums
  the resident memory of the build's process group once a second. 2026-09-19 on an Apple M5
  (10 cores, 16 GB) with rustc 1.98.1. Each row is the mean of its repetitions. Summing processes
  double-counts the pages they share, so compare rows with each other rather than reading one as
  exact, and a one-second sample can pass over a shorter spike, so these are floors.
- **Why:** a run window ended with macOS suspending applications for want of memory. These numbers
  say how much of a 16 GB machine the build accounts for.

Compiling only, `cargo test --workspace --all-features --no-run`, three repetitions each:

| setting | peak memory | wall time | written to `target/` |
|---|---|---|---|
| one job per core, `debug = 2` | 2.55 GB | 11.7 s | 1201 MB |
| `CARGO_BUILD_JOBS=6`, `debug = 2` | 1.62 GB | 13.7 s | 1200 MB |
| one job per core, `debug = "line-tables-only"` | 2.37 GB | 11.3 s | 1037 MB |

Compiling **and running** the tests, `cargo test --workspace --all-features`, two repetitions each
(`HPR_MEASURE_RUN=1`). `CARGO_BUILD_JOBS` does not reach test execution — each test binary
otherwise runs one thread per core — so the second row caps `RUST_TEST_THREADS` too:

| setting | peak memory | wall time |
|---|---|---|
| one per core | 2.58 GB | 25.5 s |
| `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=6` | 1.72 GB | 29.0 s |

- **What this says.** Building *and* running the whole suite peaks at 2.58 GB, barely above
  compiling alone. So the build contributes to memory pressure but is not the main consumer on a
  16 GB machine; what else is open matters more. Capping both to six takes 0.86 GB off that peak
  for 3.5 s on a 25 s run, which is why [the autopilot](AUTOPILOT.md) exports `CARGO_BUILD_JOBS=6`
  and `RUST_TEST_THREADS=6` for its own cycles.
- **What it rules out.** Thinning debug information does not reliably cut peak memory. Across two
  measurement sessions the default came in at 2.42, 2.47 and 2.55 GB and `line-tables-only` at
  2.53 and 2.37 GB — overlapping ranges, so the 0.18 GB gap in the table above cannot be told
  apart from run-to-run spread. It does cut what is written to `target/` by 14%, consistently.
  That is a disk saving rather than a memory one, so no profile was changed.
- **Not measured here:** `cargo doc`, `cargo xtask site` (which builds rustdoc into a second
  target directory) and `cargo xtask validate`. The session process and its builds together are
  covered instead by [the autopilot](AUTOPILOT.md)'s per-cycle memory line, which sums the
  session and everything descended from it.
- Numbers taken during an autopilot cycle inherit that six-job cap. Measure from a plain shell,
  or set `CARGO_BUILD_JOBS` explicitly, when comparing against the rows above.

## A flare through the method (M1.8e17)

- **Measured:** the supersonic table of a rocket with a conical flare, timed by hand (a throwaway
  release-mode test, best of three), 2026-09-20 on an Apple M5.

| measurement | time |
|---|---|
| a 2° flare, never drawn out, built once per model | 389 ms |
| a 30° flare, drawn out at most rows, built once per model | 716 ms |
| the same rocket on `SupersonicFlare::SlenderBody` | 4.8 µs |

- **What changed.** A flared rocket used to have no table at all: a flare ended the method's run,
  so the third row is what every flared rocket paid before this milestone. Now it builds one, and
  a flare whose corner needs drawing out costs roughly twice a plain one, because each row marches
  the body ahead of the flare for the flow at its corner and then lays out and marches a second
  body with the flare drawn. Both are one-time costs behind the table's `OnceLock`, paid only once
  a flow passes Mach 1.2 and shared by a model's clones, so a Monte Carlo of one design pays them
  once. Two obvious savings are left on the table for a later milestone: reading the flow at an
  interior station instead of marching the fore body again, and swapping the last segment of one
  body instead of rebuilding it.

## Blunt tips faster than sound (M1.8e7)

- **Measured:** the supersonic table of a rocket with a vertical nose tip, timed by hand (a
  throwaway release-mode test, best of three), 2026-09-19 on an Apple M5.

| measurement | before | after |
|---|---|---|
| Calisto's supersonic table (von Kármán nose, boattail), built once per model | none: slender-body theory | 363 ms |

- **What changed.** A vertical tip (power-series, Haack or elliptical nose) now flies the
  shock-expansion method behind a Newtonian cap, so its rocket builds the table a pointed nose
  already did: about 125 marches of the method, each laying out the nose's elements again from
  the cap's handover, which moves with the Mach number. The table is built only once a flow passes
  Mach 1.2, so a subsonic flight never pays for it, and a model's clones share it. Pointed noses
  are unchanged.

## Body lift and the measured boattail (M1.8e6)

- **Benchmarks:** `cargo bench -p hpr-aero --bench normal_force -- normal_force` and
  `cargo bench -p hpr-sim --bench flight -- "K400C to the ground"`, criterion, release profile,
  2026-09-19 on an Apple M5, `main` before M1.8e6 and the milestone's branch run back to back.
  The supersonic table was timed by hand (a throwaway release-mode test, best of three).

| measurement | before | after |
|---|---|---|
| `AeroModel::normal_force`, synthetic two-stage, Mach 0.6 at 0.05 rad | 26.8 ns | 51.2 ns |
| `AeroModel::normal_force`, Calisto, the same flow | 23.1 ns | 44.0 ns |
| Valetudo K400C to the ground | 1.321 ms | 1.385 ms |
| NDRT 2020's supersonic table, built once per model | 316 ms | 616 ms |

- **Where the time goes.** Body lift now takes Jorgensen's factor at the flow's crossflow Mach
  number: two table lookups and two divisions per evaluation, where Galejs's was a constant. A
  whole flight pays 4.9% more, well under the M1.6 budget of 5 ms. A fast path below crossflow
  Mach 0.2, where most flights stay and both tables are straight lines, would win most of it back.
- **The table.** A boattail the shock-expansion method covers now marches the forebody twice per
  row, once as it is and once with a cylinder in the boattail's place
  ([issue #98](https://github.com/nrdptel/hpr-sim/issues/98)); bodies without one are unchanged.

## Normal-force tables (M1.8d)

- **Benchmark:** `cargo bench -p hpr-sim --bench flight -- "K400C to the ground|normal-force
  table"`, criterion, release profile, 2026-09-19 on an Apple M5 with rustc 1.98.1.
- **Input:** the M1.6b flight below, and the same flight on a table of hpr's own normal force at
  0°, 2° and 4° and every Mach 0.01 from 0 to 1, the shape of a RASAero II export.

| flight | median |
|---|---|
| Valetudo K400C to the ground | 1.122 ms |
| the same on a normal-force table | 1.502 ms |

- **Where the time goes.** With a table, each component is evaluated twice per derivative
  evaluation, in its local flow and in the centre of mass's, and the table is looked up once:
  34% more here, 3.3 times under the M1.6 budget of 5 ms. The second evaluation repeats the
  centre of mass's flow angles for every component; passing them in would save part of it if a
  table flight ever becomes a hot path.

## Roll (M1.8c)

- **Benchmark:** `cargo bench -p hpr-aero --bench normal_force -- roll`, criterion, release
  profile, 2026-09-19 on an Apple M5 with rustc 1.98.1.
- **Input:** `synthetic-two-stage-75mm-54mm.json`'s two fin sets.

| call | median |
|---|---|
| `AeroModel::roll`, Mach 0.6 | 10.5 ns |
| `AeroModel::roll`, Mach 1.0 (the transonic join) | 6.1 ns |
| `AeroModel::roll`, Mach 2.0 | 35.2 ns |

- **Where the time goes.** Each fin set's span moments and the join's two ends are built with the
  model (`FinRollTerms`), so below Mach 0.8 and on the join a call is a few multiplications per
  set. Faster than sound each set clips its outline against the tip's Mach cone twice, as the
  normal force does. The code review measured the first version, which rebuilt those terms on
  every call, at up to 670 ns for elliptical fins (a 257-sided outline); now only the clip scales
  with the outline.

## Recovery (M1.7a)

- **Benchmark:** `cargo bench -p hpr-sim --bench flight`, criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** the M1.6b flight below, with a 0.6 m flat circular drogue at apogee (0.5 s lag) and a
  2.4 m main at 150 m (1 s lag) that releases the drogue, both filling by Knacke's law.

| call | median |
|---|---|
| `Simulation::run`, Valetudo K400C with a drogue and a main to the ground | 0.88 ms |

- **A recovered flight is cheaper than a ballistic one** (the M1.6b row re-measured at 1.17 ms in
  the same run), although it lasts 61.3 s against 29 s: it reaches the ground at 6.57 m/s.
- **Work.** 2,085 derivative evaluations, 328 accepted steps and 14 rejected, against the
  ballistic flight's 2,578 and 410. The descent phase evaluates no airframe aerodynamics, and
  after burnout the mass properties need no central differences, so a descent step is cheaper as
  well as longer than the ballistic dive it replaces.
- **The recovery scan** (the numeric triggers) adds **24** evaluations over the whole flight, one
  per integration interval while a device is pending, shared by every pending device. `Stats` does
  not count them, so they are 1.1% of the work above and not in that 2,085.
- **The 5 ms budget** for a Level 2 flight therefore also holds with recovery, 5.7 times under it.

## Flight (M1.6b)

- **Benchmark:** `cargo bench -p hpr-sim --bench flight`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:**
  - Valetudo (`rocketpy-valetudo.json`, K400C, 9.7 kg), from a 3 m vertical rail at Spaceport
    America.
  - The 1976 standard atmosphere, 5 m/s wind from the west.
  - The default settings (Dormand–Prince 5(4), `rtol = atol = 1e-8`), from ignition to the ground
    (29 s, apogee 874 m, when measured).
  - Since M2.1b2 the motor gets no pressure-thrust correction at the site, so the same inputs now
    reach 779 m and the ground at 27.5 s. The flight timings here and under Recovery (M1.7a) above
    predate that change.

| call | median |
|---|---|
| `Simulation::run`, Valetudo K400C to the ground | 1.10 ms |
| `Simulation::run`, Valetudo K400C with every channel recorded at 10 ms | 2.49 ms |

- **The M1.6 budget** is 5 ms for a typical Level 2 flight, so this is 4.5 times under it.
- **Work.** 2578 derivative evaluations, 410 accepted steps and 15 rejected, so about 0.4 µs per
  evaluation including the event checks. The event functions reuse the evaluation cached for the
  step's last stage.
- **Where the time goes.**
  - Each evaluation calls `Assembly::mass_properties` four times, for the value and the central
    differences of its rates. That is most of the cost, at about 60–100 ns each (M1.4b).
  - The other calls are one geodetic conversion, the atmosphere and wind, drag (50–110 ns) and a
    normal force per component (about 25 ns each).
  - Analytic motor rates would remove three of the four mass calls.
- **Recording** every channel every 10 ms adds 2900 full evaluations, 1.4 ms.
- **Tolerance.** Loosening to 1e-6 halves the time (0.59 ms), at a 7e-5 m apogee error
  (`docs/physics/flight.md`).

## Drag (M1.5b)

- **Benchmark:** `cargo bench -p hpr-aero --bench drag`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** the two layouts below, at Mach 0.6, `α` 0.05 rad, sea-level Reynolds number, one
  54 mm motor thrusting; and Calisto with a 200-row override table.

| call | median |
|---|---|
| `AeroModel::drag`, synthetic two-stage | 107 ns |
| `AeroModel::drag`, Calisto | 47 ns |
| `AeroModel::drag`, Calisto with a 200-row table | 13 ns |

- **Where the time goes.** Each component's skin friction takes a logarithm and a power, and each
  fin set a power for its leading edge; the table is a binary search.
- **After M1.8b1** (2026-09-18, same machine and inputs): 119 ns, 50 ns and 11.3 ns. Each nose,
  shoulder and step now evaluates its pressure-drag curve, a power at Mach 0.6 (eq. 3.87), and a
  power and a logarithm on Stoney's measured shapes from Mach 0.8. A new point, Calisto coasting
  at Mach 1.5 (`AeroModel::drag, Calisto at Mach 1.5`), takes 110 ns: its von Kármán nose is on
  Stoney's curve there, and the supersonic friction and stagnation terms take more powers.
- **After M1.8b3** (2026-09-18, same machine and inputs; `main` measured in the same session in
  brackets): 117.5 ns (119.1), 54.7 ns (50.3) and 11.0 ns (11.4); Calisto at Mach 1.5, 239.8 ns
  (108.2). Faster than sound a boattail's drag inverts the Prandtl–Meyer function, a bracketed
  Newton iteration of about ten evaluations: the 132 ns. A rocket without a boattail, or below
  Mach 0.8, doesn't pay it; a supersonic flight of a few thousand evaluations pays well under a
  millisecond. A table of each boattail's drag against Mach number, built with the model, would
  remove it if it ever matters.
- **Normal force after M1.5b.** Re-measured in the same session: `AeroModel::new` 3.86 µs and
  11.2 µs, `normal_force` 28.0 ns (two-stage) and 22.2 ns (Calisto), where `main` before M1.5b
  measured 14.8 ns and 11.1 ns in that session (the M1.5a numbers below came from an earlier
  session). Nothing on the normal-force path changed. Removing the new drag fields from
  `AeroModel` recovered part of it (21.4 ns and 19.5 ns), and `#[inline]` on the hot helpers
  changed nothing, so it looks like code layout; M1.6's flight benchmark will show whether it
  matters.

## Normal force (M1.5a)

- **Benchmark:** `cargo bench -p hpr-aero --bench normal_force`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** resolved layouts of `synthetic-two-stage-75mm-54mm.json` (two fin sets, two
  transitions) and `rocketpy-calisto-getting-started-motor-at-minus-1.255.json` (von Kármán nose,
  boattail, one fin set), at Mach 0.6, `α` 0.05 rad.

| call | median |
|---|---|
| `AeroModel::new`, synthetic two-stage | 3.54 µs |
| `AeroModel::new`, Calisto | 10.7 µs |
| `AeroModel::normal_force`, synthetic two-stage | 11.0 ns |
| `AeroModel::normal_force`, Calisto | 8.5 ns |

- **Where the time goes.** Building the model integrates each nose and transition's filled volume
  and planform once. Evaluation is a sum over components with one square root per fin set.

## Design tree and assembly (M1.4b)

- **Benchmark:** `cargo bench -p hpr-design --bench design`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** from `validation/designs/`, `synthetic-two-stage-75mm-54mm.json` (17 components,
  two bundled motors) and `rocketpy-calisto-getting-started-motor-at-minus-1.255.json` (6
  components, a 2 mm von Kármán nose wall, BATES grains).

| call | median |
|---|---|
| `Rocket::layout`, synthetic two-stage | 654 µs |
| `Rocket::layout`, Calisto | 2.38 ms |
| `Assembly::mass_properties(t)`, two motors | 62 ns |
| `Assembly::mass_properties(t)`, Calisto (BATES grains) | 54 ns |

- **Where the time goes.** Resolving a design costs what its walls cost (M1.4a below): Calisto's
  nose wall is 2 ms of its 2.4 ms. Resolve once per design.
- **Per step.** The flight engine calls `mass_properties(t)` at each derivative evaluation. At about
  60 ns, a few thousand evaluations cost well under a millisecond of M1.6's 5 ms budget. Folding
  the motors into the structure pairwise, instead of collecting them into a `Vec`, took it from 85
  and 118 ns (review fix).

## Mass properties from geometry (M1.4a)

The calls a design edit makes, or a Monte Carlo sample that perturbs dimensions.

- **Benchmark:** `cargo bench -p hpr-design --bench mass`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** a 4-inch (101.6 mm) von Kármán nose 5 calibres long, and a tangent-ogive boattail
  from 101.6 to 76.2 mm over 0.1 m, each filled or with a 2 mm wall. The fins are four
  trapezoidal G10 fins (0.2/0.08 m chords, 0.11 m span) with an airfoil section, and the same set
  as a five-point freeform outline with rounded edges.

| call | median |
|---|---|
| `revolve`, von Kármán nose, filled | 10.1 µs |
| `revolve`, von Kármán nose, 2 mm wall | 2.00 ms |
| `revolve`, ogive boattail, 2 mm wall | 274 µs |
| `NoseCone::mass_properties`, 2 mm wall | 1.99 ms |
| `FinSet::mass_properties`, trapezoidal airfoil | 305 ns |
| `FinSet::mass_properties`, freeform rounded | 11.2 µs |

- **Wall cost.** Each quadrature node finds the inner radius by a 32-point scan and a
  golden-section search, about 70 profile evaluations. The first version refined the minimizing
  station to 1e-15 t and scanned 256 stations for where the wall fills in: 2.25 ms. Near the
  minimum the value is quadratic in the station, so 1e-9 t is enough (1.84 ms), and a 64-station
  scan still brackets the one fill-in point a nose has (1.35 ms), with every test unchanged.
  The review fixes first made the end points candidates and split at `t` from each end (1.70 ms).
  Then they dropped tangent extensions and split wherever the nearest surface point moves between
  the surface and a rim, which needs a 128-station scan (2.00 ms). The filled nose went from 8.2 to 10.1 µs with the
  Haack series and the precise arc.
- **Where it matters.** Mass properties are computed once per design, not per derivative
  evaluation. A 10,000-sample Monte Carlo run that perturbs nose dimensions would spend about
  20 s in walls; M6.1 can cache or perturb mass directly.

## Solid motors (M1.3)

The motor calls the flight engine makes on every derivative evaluation, and reading motor data.

- **Benchmark:** `cargo bench -p hpr-motor --bench motor`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** the bundled Loki M1378LR curve (46 points, 4.08 s) at t = 2.345 s. The grain model
  is four BATES grains, 0.256 m long. The `.rse` input is the same curve written as a RockSim
  file.

| call | median |
|---|---|
| `ThrustCurve::thrust_n` | 3.54 ns |
| `SolidMotor::state`, envelope column | 12.7 ns |
| `SolidMotor::state`, BATES grains | 33.3 ns |
| `eng::parse`, one bundled curve | 3.47 µs |
| `rse::parse`, the same curve | 10.9 µs |
| `Catalog::bundled` (index only, 32 motors) | 33.3 µs |

- **Inner-loop cost.** About 10⁴ derivative evaluations per flight at 33 ns is 0.3 ms.
- **Grain solve.** The first version took 441 ns: at the root, Newton's step landed on the edge of
  its bracket, and the solver then bisected about 50 more times. Stopping once a step is within
  rounding brought it to 28 ns with the same results (33 ns once NaN inputs were passed through).
- **`.rse` line numbers.** Looking each node's line up in the text again made reading quadratic
  (1.2 s for 1.35 MB). A table of line starts, built once, fixed it; this curve went from 14.2 to
  10.9 µs.

## Atmosphere, wind and turbulence (M1.2)

The calls the flight engine makes on every derivative evaluation, and one gust-field build.

- **Benchmark:** `cargo bench -p hpr-atmos --bench atmosphere`, which is criterion, release
  profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** 4.4 km above sea level. The sounding has 30 humid levels with wind, from a 1400 m
  site to 12 km, and its pressures above the first level are filled in.

| call | median |
|---|---|
| `Ussa76::sample` | 9.14 ns |
| `SoundingProfile::sample`, 30 levels | 21.7 ns |
| `LayeredWind::wind`, 30 levels | 7.49 ns |
| `LayeredWind::wind`, 1 level | 3.64 ns |
| `GustField::gust` | 1.87 ns |
| `DrydenGenerator::advance`, 1 m | 55.3 ns |
| `GustField::generate`, 20 km at 1 m (20 001 samples) | 1.10 ms |

- **Inner-loop cost.** About 10⁴ derivative evaluations per flight, each calling a 30-level
  sounding, its wind and a gust lookup, is about 0.3 ms: well inside M1.6's 5 ms budget.
- **Generation cost.** Most of a generator step is the incomplete-gamma series and five normal
  draws, 55 ns in all. A gust field is built once per flight, or once per Monte Carlo sample.

## Earth model (M1.1)

The calls the flight engine makes on every derivative evaluation.

- **Benchmark:** `cargo bench -p hpr-core --bench earth`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Position:** 3 km up and a few hundred metres downrange of a 1400 m site.

| call | median |
|---|---|
| `Earth::gravity_enu_mps2`, `constant` | 2.45 ns |
| `Earth::gravity_enu_mps2`, `vertical_taylor` | 3.81 ns |
| `Earth::gravity_enu_mps2`, `ellipsoidal` (the default) | 35.5 ns |
| `Earth::gravity_enu_mps2`, `vertical` | 53.4 ns |
| `Earth::rotation_acceleration_enu_mps2` (Coriolis) | 1.3 ns |
| `LaunchFrame::geodetic_from_enu` (Karney's closed form) | 27.7 ns |

- **Inner-loop cost.** A 6-DOF flight of a few thousand accepted steps evaluates the derivative
  about 10⁴ times. The default model costs about 0.4 ms of that, well inside M1.6's 5 ms budget.
- **Series cut-off.** The `q`/`q′` series stops once its terms fall below rounding. At the Earth's
  eccentricity that takes about 8 terms, where a fixed 40 took 53 ns for the `ellipsoidal` model.
- **Why `vertical` is slower than `ellipsoidal`.** It builds the full vector at a geodetic point,
  converts that point to ECEF, and then rotates the result.
