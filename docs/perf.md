# Performance

Measured numbers only, newest first within each section. Record the machine, the toolchain, and
the command, so a later run can be compared like for like.

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
