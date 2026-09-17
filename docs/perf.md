# Performance

Measured numbers only, newest first within each section. Record the machine, the toolchain, and
the command, so a later run can be compared like for like.

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
