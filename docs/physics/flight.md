# Rigid-body flight

Code: `hpr_sim::{dynamics, flight, rail, recorder, state}`. Decisions: ADR-011. The integrator and
events are in `integration.md`, and the frames in `frames.md`.

Sources:

- **[RP-EOM]** RocketPy technical documentation, "Equations of Motion" v0 (Kane's method with the
  Reynolds transport theorem) and v1 (the form solved), `docs/technical/equations_of_motion.rst`
  and `equations_of_motion_v1.rst` in RocketPy 1.13.0 (`refs/rocketpy`, MIT).
- **[C21]** G. H. Ceotto, R. N. Schmitt, G. F. Alves, L. A. Pezente and B. S. Carmo, "RocketPy:
  Six Degree-of-Freedom Rocket Trajectory Simulator", *J. Aerosp. Eng.* 34(6), 2021,
  doi:10.1061/(ASCE)AS.1943-5525.0001331. Cited, not fetched.

## State

- **Components.** The state is the nose tip `O`'s position `r_O` and velocity `v_O` in the launch
  frame `L`, the attitude quaternion `q` (body to `L`), and the body rates `ω` relative to `L`, in
  body axes: 13 components.
- **Why the nose tip.** It is fixed in the body and is the body origin (ADR-007). The centre of
  mass `r` (from `O`, body axes) moves as propellant burns.
- **The quaternion.** Its norm drifts slightly between steps. Every use normalizes it, and it is
  never reset at an event (`integration.md`).

## Equations of motion

[RP-EOM] v0 derives the motion of a variable-mass system about a body-fixed point: Kane's
equations for the rigid parts, and the Reynolds transport theorem for the propellant and the gas
leaving the nozzle. Its assumptions:

- the gas flow inside the motor is quasi-steady, so its control-volume integrals don't change;
- the flow is axisymmetric;
- the exhaust momentum is lumped into the thrust at the nozzle exit.

v1 solves the result for `v̇` and `ω̇`. In body axes, with primes for body-frame time derivatives:

```text
T03 = 2 Σ ṁ_k (n_k − r) − 2 m r′
T04 = T − m r″ − 2 ṁ r′ + Σ m̈_k (n_k − r)
T20 = −ω×(ω×m r) + ω×T03 + T04 + W + A
T21 = −ω×(I_O ω) + (Σ ṁ_k S_k − I_O′) ω + r×W + M_A + M_T
ω̇   = I_c⁻¹ (T21 − r × T20)
a_O = T20/m − ω̇ × r            (then a_L = q a_O q*)
q̇   = ½ q ⊗ (0, ω)
```

- **Mass terms.**
  - `m`, `ṁ = Σ ṁ_k ≤ 0` and `m̈_k` are the mass, its rate and each motor's mass acceleration.
  - `I_O` and `I_c` are the inertia about `O` and about the centre of mass (`I_O = I_c + m(|r|² 1 − r rᵀ)`).
  - `n_k` is motor `k`'s nozzle exit.
- **Forces and moments.**
  - `T` is the thrust along `z_B`, the motors' `thrust_at_pressure_n`, with moment `M_T = Σ n_k × T_k` about `O`.
  - `W` is the weight: normal gravity (centrifugal term included) plus the Coriolis force
    `−2m Ω×v_cg`, both acting at the centre of mass.
  - `A` and `M_A` are the aerodynamic force and its moment about `O` (below).
- **Nozzle gyration tensor.** `S_k = (r_e²/4) diag(1, 1, 2) + |n_k|² 1 − n_k n_kᵀ`, integrated here
  from v0's boxed rotational equation. The jet term there is `∫ r×(ω×r) dṁ` over the exit disc of
  radius `r_e`, with a uniform jet. For a disc at `n_k` on the axis, this gives
  `(r_e²/4 + n_k², r_e²/4 + n_k², r_e²/2)`.
  - RocketPy 1.13.0's code (`rocket.py:984-985`, `evaluate_nozzle_gyration_tensor`) uses
    `0.25·n²` for the transverse distance term instead of `n²`. Its `n` is measured from the centre
    of dry mass, not from the nose tip, so the entries can't be compared directly. M2.1 should
    compare the jet-damping coefficient about the centre of mass.
  - A motor that gives no nozzle exit radius contributes `r_e = 0`.
- **The classical limit.** For an axisymmetric rocket turning slowly about a transverse axis, the
  equations reduce exactly to the classical jet damping about the centre of mass,
  `I_c ω̇ = [ṁ (r_e²/4 + l²) − İ_c] ω`, with `l` from the centre of mass to the nozzle exit. The
  `ṁ l²` part damps, and the falling inertia partly offsets it
  (`dynamics::tests::jet_damping_matches_the_classical_form`).
- **Time derivatives of the mass properties.**
  - `r′`, `r″`, `ṁ` and `I_O′` come from central differences of `Assembly::mass_properties` with a
    half-width of 1e-4 s.
  - The stencil is kept inside the current integration interval. Every thrust-curve knot and
    burnout is a stop time, so no difference straddles a change in the thrust's slope. Near an
    interval's end the derivative is taken at the nearest valid centre and extended linearly.
  - `m̈_k` differences each motor's mass flow the same way.
  - After burnout (an interval starting past every burnout) the rates are zero, and so are they over
    intervals shorter than 2e-5 s, where differences would be rounding noise.
- **Motors at the ends of an interval.** A motor that burns through an interval is evaluated at
  its one-sided limit inside the burn, `t` clamped to `(0, t_end)`. The last stage of the step
  ending at burnout (or the first after ignition) then sees the burning motor, including the
  pressure correction that switches off at `t_end`. Without this, fixed-step RK4 converged at
  first order (5.1 mm of apogee at 2 ms).
- **Measured thrust curves and the internal-momentum terms.** A static test measures
  `T_exit − dP_int/dt`, where `P_int = m r′ − ṁ(n − r)` is the internal momentum of the burning
  propellant, so a `.eng` curve already contains it. `T04` subtracts `−m r″ − 2ṁ r′ + m̈(n − r)`
  again.
  - hpr keeps the terms as RocketPy does, for parity in M2.1. The form is exact for the
    [RP-EOM] model, not for a measured curve.
  - The size of the double count on Valetudo: it lifts off at 1.07 ms with 73 N of thrust against
    95 N of weight, 21 N coming from `m̈(n − r)`, and it changes the burnout speed by at most
    0.05 m/s.
- **Earth's rotation.** It enters only through the Coriolis force. The rotational equations use
  `ω` relative to `L`, which differs from the inertial rate by at most 7.3e-5 rad/s (`frames.md`,
  ADR-011).

## Aerodynamics in flight

`hpr-aero` gives coefficients at a flow condition (`aero.md`). The engine applies them as follows.

- **Airspeed.** The air velocity uses the wind at the centre of mass's height. Wind vectors are
  taken in `L`'s axes.
- **Axial force.** `−q A C_A z_B` from the whole rocket's drag, at the centre of mass's airspeed,
  Mach number, angle of attack and Reynolds number per metre (`V/ν`). The drag is power-on
  (`DragConditions::thrusting`, with the burning motors' cross-section) while any motor burns in
  the interval. It acts along the axis, so it has no moment about `O`.
- **Normal and side forces, component by component.** Each body and fin set is evaluated at its
  own local flow: `v_O − wind + ω × p_i` at its small-angle centre of pressure `p_i`
  (`AeroModel::component_station_m`). Its `C_N` acts along the crossing air `ŵ` and its `C_Y`
  along `z_B × ŵ` (`frames.md`), with moments `−q A M_N (z_B × ŵ) + q A M_Y ŵ` about `O` from the
  component's moment coefficients.
- **Fins at any angle of attack.** A fin set's normal force follows the crossflow `V sin α`: its
  model's `C_N = C_Nα α` is used as `C_Nα sin α`, the substitution Niskanen keeps for bodies (eq.
  3.16–3.17). No source in hand gives a large-angle fin model, and stall is not modelled.
  - At small angles nothing changes (1.5% less at 0.3 rad).
  - The force now vanishes for axial flow from the tail as well as from the nose.
  - A force linear in `α` stays large at `α = π` and flips with rounding noise in the lateral
    velocity. A calm vertical flight falling tail first after apogee then collapsed the step size
    and never landed (`tests::a_calm_vertical_flight_falls_tail_first_and_lands`).
- **Damping.** The rotation's contribution to each local flow is the only pitch and yaw damping.
  `hpr-aero` has no damping coefficients, and roll forcing and damping arrive with M1.8, so the roll
  rate changes only through inertia coupling. At a component's CP the rotation adds a speed of
  `|ω × p_i|`.
- **Limits.**
  - The models are small-angle: no stall, and body lift and fins extended by `sin α`. They
    overstate the forces at large `α`. In normal flights large `α` occurs near apogee, where the
    dynamic pressure is small, and off the rail in strong crosswinds. `Sample::angle_of_attack_rad`
    shows where it happens.
  - A Mach number of 1 or more anywhere stops the flight with `SimError::Aero` until M1.8. A drag
    override table covers drag at any Mach, but not normal force.

## Phases

- **Pad.**
  - The rocket starts with its aft end (its body's aft end or its aft-most nozzle exit) at the
    rail's foot, at rest, with the rail's attitude.
  - It stays until the force along the rail, `T20·z_B` with `ω = 0`, exceeds friction
    `μ |T20_⊥|` (the liftoff event). `T20_⊥` is the rail's reaction: the weight, the aerodynamic
    force and the mass terms across the axis.
- **Rail.**
  - One degree of freedom: `a = (T20·z_B − μ |T20_⊥|)/m` along the rail, with no rotation.
  - It ends when the aft edge of the aft-most rail button or launch lug passes the top of the
    rail, or the aft end with no guides (Rail geometry, below).
  - If the speed along the rail falls to zero, the rocket returns to the pad phase, at rest where
    it stopped. It is held there, even if the force down the rail exceeds friction and a real
    rocket would slide back.
- **Free.** Six degrees of freedom until the ground.
- **Rail geometry.**
  - A button's axial extent is its outer diameter, and a lug's is its length.
  - The rocket leaves after travelling `L − (s_aft − s_guide)`. RocketPy ends its rail phase when
    the forward button reaches the top (`flight.py:1716-1730`, `effective_1rl`). hpr keeps the
    rocket guided to the last guide (Loft lesson L26).
  - The pivot about the last guide ("tip-off") is not modelled, and the rocket leaves the rail
    with no angular velocity.
  - Friction is Coulomb friction with a user coefficient, on the net reaction `|ΣN|`. With a
    moment across two buttons (a crosswind at low speed), the true friction `μ(|N₁| + |N₂|)` is
    larger. No source gives a default coefficient, so the default is zero.

## Events and termination

- **Stop times.** Thrust-curve knots, burnouts and the time cap. A burnout that is reached records
  a `Burnout` event.
- **Liftoff and stall.** Liftoff is the pad force margin rising through zero (checked also at
  each interval's start, for a thrust that jumps at a knot). Stall is the speed along the rail
  falling through zero.
- **Rail exit.** The travel along the rail reaching the exit travel.
- **Apogee.** The centre of mass's ellipsoidal-height rate, `û(r_cg) · v_cg`, falling through
  zero. `û` is the ellipsoid normal at its position.
- **Ground hit.** The centre of mass's ellipsoidal height reaching the launch site's, descending
  (`frames.md`, Loft lesson L35).
- **User events.** A function of the `Sample`, in free flight.
- **Heights.** Atmosphere and wind heights are `h − N`, with the geoid undulation `N` given in
  `Environment` (no geoid model).
- **Termination** (Loft lesson L25). The flight ends in exactly one of these ways:
  - `GroundHit`;
  - `NoLiftoff` (the last burnout passes on the pad);
  - `StalledOnRail` (it lifted off, then stopped on the rail after burnout);
  - `TimeCap`;
  - `StepLimit`.

  Any other failure is an error.
- **Not yet modelled.** Recovery (M1.7), staging, delayed ignition (M1.9), turbulence and thrust
  misalignment.

## Integration settings

- **Defaults.** `FlightSettings::default()` uses Dormand–Prince 5(4) with `rtol = atol = 1e-8`
  and unit weights, a one-hour cap and 10⁶ steps.
- **Accuracy against cost.** Measured on Valetudo (K400C), with a 5 m/s wind from the west,
  compared with the apogee at 1e-11 (873.98272 m):

  | `rtol = atol` | apogee error | flight time (release) |
  |---|---|---|
  | 1e-4 | 2.3e-2 m | 0.36 ms |
  | 1e-6 | 6.8e-5 m | 0.59 ms |
  | 1e-8 | 1.1e-6 m | 1.13 ms |
  | 1e-10 | 1e-7 m | 2.72 ms |

## Verification

Unit tests in `hpr_sim::tests`, `hpr_sim::dynamics::tests` and `hpr_sim::rail::tests`, at the
default settings. The numbers were measured on 2026-09-17.

- **Vacuum ballistic.** A tumbling, spinning Valetudo in a vacuum, from the nose tip at 500 m with
  `v = (30, −20, 80)` m/s.
  - The centre of mass stays on the closed-form parabola to 1.7e-6 m over 22 s.
  - The angular momentum stays constant to 7.8e-7 (relative).
  - Apogee and ground contact are within 4.7e-7 s and 1.1e-8 s of the parabola's. Those times are
    root-found on the exact ellipsoidal height.
- **Terminal velocity.** Falling nose down through uniform air with `C_D = 0.5`, the speed follows
  `v_t tanh(g t/v_t)` to 5.5e-9 `v_t`, and is `v_t` to 1e-5 after 150 s.
- **Torque-free precession.** Valetudo (`I_a/I_t` = 0.0019) spinning at 25 rad/s with a transverse
  rate turns in body axes at `Ω = (I_a − I_t) ω_z/I_t` = −24.95 rad/s. The rate matches to
  4.7e-7 rad/s over 3 s, and the angular momentum in `L` holds to 1.5e-6.
- **Pitch oscillation against linear theory.** At 100 m/s with no drag or gravity, the two-state
  linear model (path turning, restoring moment `K₁`, rotational damping `K₂`) predicts a
  1.44954 s period. The flight measures 1.44965 s (8e-5), and the decay per half period is within
  0.2% of the model's.
- **Jet damping.** The flight equations give the classical jet damping to 1e-6, and the mass rate
  equals the motor's `−F/c` to 1e-6, including stencils clipped by an interval's end.
- **Powered climb.** A vertical climb in vacuum from 0.5 s to 3.0 s ends at the speed of the axial
  equation, integrated independently from the motor and the assembly, to 4.3e-8 m/s.
- **Rail friction.** At 60°, `μ = 0.3` removes exactly `μ g cos E` from the acceleration along the
  rail.
- **L20, weathercocking.** In a 5 m/s wind from the west, Valetudo's unit axis has an east
  component of −0.12 at burnout. Its apogee is 96 m upwind, against 1.0 m (Earth rotation) in calm air.
- **Calm vertical.** With no wind and no Earth rotation the rocket falls tail first after apogee
  and still lands, in under 20,000 evaluations.
- **Solvers agree.** RK4 at 2, 1 and 0.5 ms gives the same apogee as Dormand–Prince to 1.2e-5 m
  and 2.1e-7 s.
- **Burnout at an event.** A user event on the thrust fires at burnout, and burnout is still
  recorded once.
- **`StalledOnRail`.** A 2000 m rail gives it, with only liftoff and burnout recorded.
- **Errors and reuse.** Observer errors end the flight with that error. Starts before ignition
  or underground are refused. A cleared recorder records the same rows again. `Simulation`,
  `Environment` and `Recorder` are `Send + Sync`.
- **L24.** Two runs of one `Simulation`, and a second `Simulation` built the same way, record the
  same bits for every channel at every step.
- **L25.** A normal flight hits the ground. A rail with `μ = 20` at 60° gives `NoLiftoff` at rest.
  A 5 s cap gives `TimeCap` at exactly 5 s, and a 40-step limit gives `StepLimit`.
- **L26.** On a 3 m rail tilted to 1.3 rad, the rail exit comes at the last button's travel to
  1e-6 m. Across the rail the rocket stays within 1e-9 m, with no rotation. Friction (`μ = 0.3`)
  delays the exit and slows it.
- **Events and recorder.** Events come in order: liftoff, rail exit, burnout, apogee, ground hit.
  Apogee's vertical speed is below 1e-6 m/s and ground contact's height below 1e-6 m. Recorder rows
  fall on the interval or at events.
- **Refusals.** The synthetic 54 mm rocket on an I175 passes Mach 1 and stops with
  `SimError::Aero(Mach)`.
- **Cost.** About 1.1 ms per Valetudo flight to the ground (`docs/perf.md`).
