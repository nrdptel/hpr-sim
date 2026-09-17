# RocketPy's solid motor model

RocketPy v1.13.0 (MIT), commit `9bd6ad3`, read from `refs/rocketpy`; paths are relative to it.
This is the reference for M1.3's comparison ("mass and inertia evolution matches RocketPy's
SolidMotor for 3 motors within 1%"); the oracle is `validation/oracles/rocketpy/solid_motor.py`.
"Findings" are our reading of the code, backed by the measurements quoted there.

## Construction order

`SolidMotor.__init__` (`rocketpy/motors/solid_motor.py:202`) calls `Motor.__init__`
(`rocketpy/motors/motor.py:172`). That call builds the thrust curve, burn time, impulse and average
thrust. SolidMotor then computes the grain mass (`solid_motor.py:361-366`) and solves the grain ODE
(`solid_motor.py:368`, `evaluate_geometry`). Every other output is a lazily cached `Function` built
by `funcify_method` (`rocketpy/mathutils/function.py:4248`, cache at `:4363`).

## Thrust input

- **Sources:** a `.eng` path goes through `import_eng` (`motor.py:1106`), which always prepends
  `(0, 0)` (`motor.py:1133`) and splits the header with `split(" ")` (`motor.py:1145`). A `.rse`
  path goes through `import_rse` (`motor.py:1029`). Lists and arrays become `Function(source,
  interpolation=interpolation_method, extrapolation="zero")` (`motor.py:315-317`).
- **Interpolation:** `interpolation_method` defaults to `"linear"` (`solid_motor.py:220`). The
  other options are `"spline"` and `"akima"`.
- **Callable or constant thrust** is sampled at 50 points over `burn_time` (`motor.py:325-326`).
- **`burn_time`:** a float `b` means `(0, b)`, a pair is used as given (`rocketpy/tools.py:95-119`),
  and `None` means the first and last knot (`motor.py:363-381`).
- **`reshape_thrust_curve=(t_b, I)`** (`motor.py:912-963`) scales the times linearly onto the new
  burn time, then scales thrust by `I / I_old`, where `I_old` is the integral over the new window.
  `burn_time` then becomes the reshaped knot range (`motor.py:332`).
- **`clip_thrust`** (`motor.py:965-1026`) keeps the knots strictly inside `burn_time` and adds
  interpolated end points at both bounds. A window outside the data is pulled in to the data range,
  with a warning.
- **Scalars:** `burn_start_time`, `burn_out_time`, `burn_duration` come from `burn_time`
  (`motor.py:338-340`); `max_thrust` is the largest knot (`motor.py:343-345`).

## Impulse, exhaust velocity, mass flow

- **Total impulse:** `I = ∫ F dt` over `burn_time` (`motor.py:419-429`), via the interpolant's
  analytic integral (`function.py:3060-3066`): for linear thrust, the exact trapezoid sum over the
  clipped knots (`rocketpy/mathutils/_calc/_fitting.py:223-245`); for spline, the spline's.
- **Average thrust:** `F_avg = I / (t_out − t_start)` (`motor.py:346`). This is the knot span, not
  a threshold-based burn time.
- **Exhaust velocity:** `v_e = I / m_p0` is constant (`solid_motor.py:401-418`).
- **Mass flow rate:** `ṁ(t) = −F(t) / v_e`, on the thrust knots, linear, zero outside the burn
  (`motor.py:483-524`). `SolidMotor.mass_flow_rate` is the same Function (`solid_motor.py:431-452`).
- **Initial propellant mass** comes from the grain geometry, not from the file header:
  `m_p0 = N ρ π (R_o² − r_0²) h_0` (`solid_motor.py:361-366`, `:420-429`).

## Propellant mass comes from the geometry

`propellant_mass = N ρ V_g(t)`, `V_g = π (R_o² − r_i(t)²) h(t)` (`solid_motor.py:374-399`).
It overrides the base class integral `m_p0 + ∫ ṁ dt` (`motor.py:470-481`). Because
`v_e = I/m_p0` and the ODE below conserves volume, the two agree to ODE tolerance:
`m_p(t) ≈ m_p0 (1 − I(t)/I)`.

Measured on the oracle's 203-point grid, the largest `|m_p − m_p0(1 − I(t)/I)|` is 5.9e-5,
6.9e-5 and 2.3e-4 kg for its three cases, which is 1.5e-4, 1.2e-4 and 7.9e-5 of `m_p0`. So hpr's
impulse-fraction default reproduces SolidMotor's mass curve well inside 1%, if `m_p0` is the
geometry mass. `total_mass = m_p + m_dry` (`motor.py:458-468`). `dry_mass` is required. If it is
`None`, the `.eng` header's total minus propellant is used instead (`motor.py:403-417`).

## Grain regression ODE (`solid_motor.py:487-632`)

- **State:** `y = (r_i, h)`, starting at `(r_0, h_0)` (`:498`). The span is the first to last
  knot of the clipped thrust (`:501-502`).
- **Per-grain volume rate:** `V̇ = ṁ(t) / (N ρ)`, which is negative (`:511`). `ṁ` is read with
  `__call__`.
- **Default, both ends burning** (`:522-533`):
  `A_b = 2π (R_o² − r_i² + r_i h)`, `ṙ_i = −V̇ / A_b`, `ḣ = −2 ṙ_i`.
  This conserves volume exactly: `d/dt[π(R_o² − r_i²)h] = −ṙ_i A_b`.
- **`only_radial_burn=True`** (`:515-519`): `A_b = 2π r_i h` and `ḣ = 0`.
- **Solver:** `scipy.integrate.solve_ivp` with LSODA, an analytic Jacobian (`:538-593`),
  `rtol = 1e-11` and `atol = 1e-12`, with no `t_eval` and no dense output (`:603-612`).
- **Stop:** a terminal event on `(R_o − r_i)·h` falling through zero (`:595-600`). Radial burnout
  happens first when the web `R_o − r_0` is under `h_0/2`, otherwise axial burnout. The event
  fired 1.6e-5 to 2.1e-5 s before `t_out` in both invented cases (M1670 reached `t_out`).
  `grain_burn_out = sol.t[-1]` (`:614`).
- **Output:** `grain_inner_radius` and `grain_height` are Functions on the solver's own step
  knots (185 to 456 in the oracle cases), with `interpolation_method` and `"constant"`
  extrapolation (`:617-630`).
- **Diagnostics only:** `burn_area`, `burn_rate` and `Kn` (`:634-702`). The throat radius affects
  only `Kn` (`:348-349`).

## Centre of mass and coordinates

- **Axis:** positions are scalars along the motor axis, from an origin the user picks.
  `"nozzle_to_combustion_chamber"` sets `_csys = +1` and `"combustion_chamber_to_nozzle"` sets
  `−1` (`motor.py:269-278`). The motor never applies `_csys`: every motor output stays in the
  user's own coordinate.
- **Propellant CoM:** `z_p(t) = grains_center_of_mass_position`, a constant held on the ODE knots
  (`solid_motor.py:471-484`). A symmetric burn from both ends keeps each grain's centre fixed.
- **Motor CoM:** `z_cm = (z_p m_p + z_d m_d) / (m_p + m_d)`, where `z_d` is
  `center_of_dry_mass_position` (`motor.py:555-569`).
- **In a rocket:** `z_rocket = motor_position + csys_rocket · csys_motor · z_motor`, and the same
  holds for `nozzle_position` (`rocketpy/rocket/rocket.py:1115-1125`). With `tail_to_nose`
  (`+z` toward the nose, as in hpr's `z_B`), a `nozzle_to_combustion_chamber` motor maps as
  `z_B = z_B(origin) + z_motor`.

## Inertia (`solid_motor.py:704-801`, `motor.py:584-766`)

Here `m_g = m_p / N` and `p = h_0 + s` is the grain pitch. The pitch uses the initial height and
the separation `s`, because the grain centres don't move.

- **Propellant `I_11`,** about the propellant CoM (`solid_motor.py:724-740`):
  `I_p11 = N m_g [(R_o² + r_i²)/4 + h²/12] + m_g Σ_k d_k²`, `d_k = (k − (N−1)/2) p`, so
  `Σ d_k² = p² N(N²−1)/12`: thick-walled hollow cylinders with their own `h²/12` term, plus Steiner.
- **`I_p22`** equals `I_p11` (`:742-762`). **`I_p33 = ½ m_p (R_o² + r_i²)`** (`:784-789`).
  Products of inertia are 0.
- **Dry inertia:** `dry_inertia = (I_11, I_22, I_33[, I_12, I_13, I_23])` about the dry CoM
  (`motor.py:289-295`).
- **Motor `I_11`,** about the instantaneous motor CoM (`motor.py:604-615`):
  `I_11 = I_p11 + m_p (z_p − z_cm)² + I_d11 + m_d (z_d − z_cm)²`. `I_22` returns `I_11`
  (`:637-638`).
- **Motor `I_33 = I_p33 + I_d33`** (`motor.py:659-666`). Products are summed without a transfer
  term (`:668-766`).
- **Hand check** (oracle case 1, t = 0): `I_p11` = 4.5778e-3, `z_cm` = 0.224732,
  `I_11` = 1.21991e-2.

## Nozzle

`nozzle_area = π r_e²` (`motor.py:284`). It enters only `pressure_thrust = (p_ref − p) A_e` and
`vacuum_thrust = F + p_ref A_e`, and only when `reference_pressure` is set (`motor.py:1154-1191`).
The rocket also uses it in the nozzle gyration tensor (`rocket.py:984-985`).

## Function evaluation: what matters for a 1% match

- **Algebra:** array Functions on identical knots combine pointwise into a new array Function
  with the left operand's interpolation (`function.py:2621-2631`). `_FAST_MATH = False`
  (`function.py:36`), so operands on different knots would become lazy callables (`:2672-2705`).
  All mass, CoM and inertia outputs share the ODE knots (checked by comparing `x_array`s).
- **Between knots,** values are linear interpolants of the products computed at the knots, not
  the formulas evaluated at `t`. The error is small at 185 to 456 knots; see the mass gap above.
- **Linear evaluation:** `bisect_left` picks the interval (`_calc/polation_1d.py:64`), so a knot
  returns its exact value. Queries inside `[x_0, x_n]` inclusive interpolate; outside,
  `"constant"` holds the end value and `"zero"` returns 0 (`polation_1d.py:613`, `:734`).
- **Flight reads:** `Flight` uses `get_value_opt` for `thrust`, `I_11`, `I_33`, `mass_flow_rate`
  and `propellant_mass` (`rocketpy/simulation/flight.py:1940-1956`), and only while
  `t_start < t < t_out` (`:1936`, `:2365`, `:2553`). For array sources `__call__` returns the same
  values.
- **Default flight equations** (`equations_of_motion="standard"`, `flight.py:506`) never read the
  motor `I_11`. The rocket builds its inertia about its own dry CoM from `dry_I_*` (about the motor
  dry CoM) and `propellant_I_*` (about the propellant CoM) (`rocket.py:856-883`, `:928-947`).
- **Comparing near burnout:** `m_p`, `I_p11` and `I_p33` go to 0, so compare them with an absolute
  floor. Compare `total_mass`, `center_of_mass`, `I_11` and `I_33` relatively.
- **`GenericMotor`,** the impulse-fraction class, is different again. Its propellant mass is
  `integral_function()` sampled at 100 linspace points with default spline interpolation
  (`motor.py:470-481`, `function.py:3281`, `:3846`). Its propellant CoM is fixed at
  `chamber_position`, and its inertia is a solid cylinder, `m(3R² + H²)/12`
  (`motor.py:1579-1603`).

## Findings: questionable, or where hpr should differ (our claims)

1. **Grain geometry sets the propellant mass and `v_e`,** with no check against the file's
   propellant mass (`solid_motor.py:361-366`, `:420-429`; `motor.py:523`). For RocketPy's own
   M1670 example, the geometry gives 2.956 kg against a header of 3.101 kg (−4.7%). hpr should
   default to the header or catalog mass and flag a grain model that disagrees beyond a tolerance.
2. **The dry-mass fallback mixes sources:** the header's total minus propellant, combined with
   the geometry's propellant (`motor.py:408-411`), gives a start mass that matches neither. The
   header parse `split(" ")` (`motor.py:1145`) shifts fields when spaces repeat.
3. **A `0 0` first point breaks the impulse.** `import_eng` always prepends `(0, 0)`
   (`motor.py:1133`), so a file that already has one gets a duplicate knot. `thrust(0)` becomes
   NaN and the total impulse becomes NaN, silently (reproduced). hpr must dedupe or reject.
4. **The grain model is kinematic only.** The burn rate comes from `ṁ ∝ F`, not from a burn-rate
   law, so geometry changes CG and inertia but not `m(t)`. That is fine for hpr's "optional grain
   geometry", as long as the docs don't present it as ballistics.
5. **The propellant inertia formulas are correct.** The own `h²/12` term is present
   (`solid_motor.py:726-729`) and `I_p33` is a hollow cylinder's. They assume identical grains,
   equal pitch and both ends uninhibited, unless `only_radial_burn` is set.
6. **The motor `I_11` is about the motor CoM,** but the default flight uses only the component
   inertias about their own CoMs. hpr's motor should expose those components; the motor-CoM
   tensor is a derived convenience.
7. **A `burn_time` shorter than the curve still burns all propellant.** `I` and `v_e` are
   recomputed over the truncated window (`motor.py:335`, `:429`), so the "unburnt" propellant is
   spread over the shorter burn.
8. **`F_avg` uses the knot span** (`motor.py:346`), not a certification-style burn time, so it
   differs from the catalog average thrust.
9. **The derived functions are coarse:** they are linear interpolants on the solver knots, held
   constant after the terminal event, which fires about 2e-5 s before `t_out`. The effect is
   negligible at 1%, but hpr's analytic evaluation differs from it at the 1e-4 level.
