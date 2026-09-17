# Performance

Measured numbers only, newest first within each section. Record the machine, the toolchain, and
the command, so a later run can be compared like for like.

## Solid motors (M1.3)

The motor calls the flight engine makes on every derivative evaluation, and reading motor data.

- **Benchmark:** `cargo bench -p hpr-motor --bench motor`, which is criterion, release profile.
- **When and where:** 2026-09-17 on an Apple M5 with rustc 1.98.1.
- **Inputs:** the bundled Loki M1378LR curve (46 points, 4.08 s) at t = 2.345 s. The grain model
  is four BATES grains, 0.256 m long. The `.rse` input is the same curve written as a RockSim
  file.

| call | median |
|---|---|
| `ThrustCurve::thrust_n` | 3.53 ns |
| `SolidMotor::state`, envelope column | 12.5 ns |
| `SolidMotor::state`, BATES grains | 28.4 ns |
| `eng::parse`, one bundled curve | 3.32 µs |
| `rse::parse`, the same curve | 14.2 µs |
| `Catalog::bundled` (index only, 32 motors) | 33.1 µs |

- **Inner-loop cost.** About 10⁴ derivative evaluations per flight at 28 ns is 0.3 ms.
- **Grain solve.** The first version took 441 ns: at the root, Newton's step landed on the edge of
  its bracket, and the solver then bisected about 50 more times. Stopping once a step is within
  rounding brought it to 28 ns with the same results.

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
