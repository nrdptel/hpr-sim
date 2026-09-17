# Frames and sign conventions

This is the single definition of the frames, and every crate follows it. Code:
`hpr_core::{geodesy, frames, attitude}`. ADR-003 records why these choices were made.

## Units and angles

- SI throughout: metres, seconds, kilograms, radians. Degrees exist only at I/O boundaries
  (`Geodetic::from_degrees`).
- Every rotation is right-handed. `R_x(θ)`, `R_y(θ)` and `R_z(θ)` turn vectors by `+θ` about the
  named axis (active rotations).

## Earth-centred Earth-fixed (ECEF)

The WGS 84 conventional terrestrial frame:

- origin at the Earth's centre of mass;
- `+Z` along the rotation axis toward the north pole;
- `+X` through the prime meridian at the equator;
- `+Y` completes the right-handed set (90° E).

A **geodetic position** `(φ, λ, h)` gives the latitude `φ` (positive north, in `[−π/2, π/2]`),
the longitude `λ` (positive east), and the height `h` above the ellipsoid along its normal.
`h` is **ellipsoidal height, not height above mean sea level**. The two differ by the geoid
undulation `N` (`h = H + N`, with `|N|` up to about 100 m). Inputs quoted above sea level must be
converted before use. `docs/physics/geodesy.md` gives the conversions.

## Launch frame `L` (East-North-Up)

- **Origin:** the launch site's geodetic position `(φ₀, λ₀, h₀)`, at the pad on the ground.
- **Axes:** `x_L` east, `y_L` north, `z_L` up along the ellipsoid normal at the origin.
- **To ECEF:** `r_ECEF = r₀ + R r_L`. The columns of `R` are, in ECEF components:

  ```text
  ê = (−sin λ₀, cos λ₀, 0)
  n̂ = (−sin φ₀ cos λ₀, −sin φ₀ sin λ₀, cos φ₀)
  û = (cos φ₀ cos λ₀, cos φ₀ sin λ₀, sin φ₀)
  ```

- **Earth-fixed, so non-inertial.** It turns with the Earth at
  `Ω = ω (0, cos φ₀, sin φ₀)` in `L` components, with `ω = 7.292115e-5 rad/s`. The translational
  equations in `L` add the Coriolis term `−2Ω × v`. The centrifugal term is already inside normal
  gravity and must not be added again (`docs/physics/gravity.md`).
- **`z_L` is not altitude.** `L` is a tangent plane, so a point at `z_L = 0` at horizontal
  distance `d` from the pad is about `d²/(2R)` above the ellipsoid: 7.8 m at 10 km. Height above
  the ellipsoid comes from `LaunchFrame::geodetic_from_enu`. The flight engine (M1.6) must detect
  ground contact and state its apogee datum with that height, not with `z_L` (Loft lesson L35).

## Body frame `B`

- **Origin:** the nose tip, on the axis (ADR-007). Positions in mass properties are measured
  from it.
- **Axes:** `z_B` lies along the axis of symmetry, positive toward the nose. `x_B` is the design's
  zero radial direction, the angle from which fins, rail buttons and lugs are placed. `y_B = z_B × x_B`.
- **Thrust** of a motor aligned with the axis acts along `+z_B`.
- **Design stations** measured aft from the nose tip, as design files state them, map to
  `z_B = z_ref − s` with `z_ref = 0`, so `z_B = −s` and the whole rocket lies at `z_B ≤ 0`
  (`docs/physics/design.md`).

## Attitude

The attitude is a unit Hamilton quaternion `q` (glam `DQuat`, stored `x, y, z, w`) that maps
body components to launch-frame components:

```text
v_L = q ⊗ v_B ⊗ q*        (glam: q.mul_vec3(v_B))
```

- `q` and `−q` are the same attitude.
- The body angular velocity `ω_B` is the rate of `B` relative to `L`, in `B` components.
- The kinematics are `q̇ = ½ q ⊗ (0, ω_B)` (Solà 2017, eq. 200). The integrator renormalizes `q`
  after every step (`attitude::renormalize`).
- `L` itself turns at `Ω` (above), so `ω_B` differs from the inertial rate by `R(q)ᵀ Ω`, at most
  7.3e-5 rad/s. The flight engine (M1.6) decides whether its rotational dynamics include that
  term, and records the decision.

## Launch angles

Launch-rail-style angles describe the attitude for input and output (`frames::LaunchAngles`):

- **azimuth `A`:** the heading of the body axis, clockwise from true north (`π/2` is east),
  in `[0, 2π)`;
- **elevation `E`:** the angle of the body axis above the horizon (`π/2` is vertical), in
  `[−π/2, π/2]`;
- **roll `φ`:** the turn about `z_B`, in `(−π, π]`.

```text
q = R_z(−A) ⊗ R_x(E − π/2) ⊗ R_z(φ)
z_B in L = (sin A cos E, cos A cos E, sin E)
```

- **Zero roll.** `x_B` is horizontal, to the right of the heading: `(cos A, −sin A, 0)` in `L`.
  `y_B` is `(sin E sin A, sin E cos A, −cos E)`.
- **Vertical singularity.** At `E = ±π/2`, `A` and `φ` turn about the same axis.
  `LaunchAngles::from_quaternion` then reports `A = 0` and puts the whole turn into `φ`. This is
  for reporting only: the state is always the quaternion.
- **RocketPy correspondence.** This is RocketPy's 3-1-3 convention:
  - `A` is the heading and `E` the inclination.
  - `φ` is the rail-button angular position for a `tail_to_nose` rocket, and 2π minus it for a
    `nose_to_tail` rocket.
  - RocketPy 1.13.0 sets precession `ψ = −heading`, nutation `θ = inclination − 90°` and spin `φ`,
    then builds `q = q_z(ψ) q_x(θ) q_z(φ)` (`rocketpy/simulation/flight.py:1557-1579`,
    `rocketpy/tools.py` `euler313_to_quaternions`).
  - `validation/oracles/rocketpy/attitude.py` builds real RocketPy flights for 8 rail setups, with
    both orientations, and records the initial `e0…e3`.
  - `frames::tests::launch_angles_match_the_rocketpy_oracle` checks that `q` (w, x, y, z) is the
    same attitude to 1e-12 rad.
  - RocketPy's body `+z` also points toward the nose.

## Tests that pin this

- **`hpr_core::geodesy::tests`:**
  - geodetic → ECEF → geodetic, from −10 km to 1000 km;
  - ECEF → geodetic → ECEF, from 6250 km to 46,000 km from the centre;
  - axis points, the sphere limit, and the ENU rotation being proper with `û` normal to the
    ellipsoid.
- **`hpr_core::frames::tests`:**
  - angles → quaternion → angles (off vertical), and quaternion → angles → quaternion (everywhere,
    exactly vertical included);
  - ENU ↔ ECEF ↔ geodetic round trips for sites anywhere, within 2000 km and up to 1000 km high;
  - named attitudes, and the tangent-plane rise `d²/(2N)`.
- **`hpr_core::attitude::tests`:** the kinematic equation checked against a closed-form coning
  motion, and norm drift ≤ 1e-12 over 1e6 RK4 steps with attitude error < 1e-9 rad.
