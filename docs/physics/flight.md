# Rigid-body flight

## In short

- **What it models:** a rocket's flight from the pad to the ground, rail included: a rigid body
  free to move and turn in every direction (six degrees of freedom) that gets lighter as its motor
  burns.
- **Sources:** the equations of motion in RocketPy's technical documentation (RocketPy 1.13.0),
  and the RocketPy paper (Ceotto et al., 2021), which is cited but was not fetched.
- **How well it is validated:** by analytic and unit tests (a tumbling rocket's centre of mass
  stays on the exact parabola in a vacuum to 1.7e-6 m over 22 s), and against RocketPy. Five of
  its example rockets, flown from the pad to the ground by both codes with the same declared drag,
  agree on height, speed, time and acceleration within 3%; the largest scored difference is
  +1.783%, a peak acceleration on the rail ([validation report][report],
  [whole flights against RocketPy](#whole-flights-against-rocketpy)). So does the path, except
  for rockets that leave the rail slowly in a wind. There hpr's
  [body lift](../glossary.md#body-lift), which RocketPy's normal force leaves out, its later
  release from the rail and, for Juno III, its simpler fin model put the drift 4.7 to 43% from
  RocketPy's ([ADR-026][adr-026]). A sixth, Prometheus 2022, passes Mach 1 on its drag table
  and agrees as well, its drifts again apart from RocketPy's by body lift. With hpr's own drag, against RocketPy flying the drag its
  examples ship, hpr's heights differ from RocketPy's by −6.985% to +10.306%, the larger gaps where
  the two drags differ most: hpr's is well below the example's for two rockets, and above it at
  high speed for Prometheus 2022, which flies through Mach 1 on hpr's drag since
  [M1.8b1](../decisions-and-roadmap.md#m1-8b1) (drag through Mach 1)
  ([Accuracy](../accuracy.md#whole-flights-with-each-codes-own-drag)). No flight has been compared
  with a real one.
- **What it leaves out:** staging and delayed ignition, tip-off (the pivot as the rocket leaves the
  rail), turbulence and thrust misalignment. Its
  small-angle aerodynamics are used at every angle of attack (the angle between the rocket's axis
  and its path through the air), with no stall. A flight that reaches Mach 5, the top of the
  aerodynamic models, stops with an error.

## What the equations do

At each instant of a flight, hpr adds up every force on the rocket and every turning effect (a
moment): the thrust, the weight, the air's forces, and the effects of the propellant burning away.
From those it works out how fast the rocket speeds up and how fast its turning changes. The
integrator ([Time integration](integration.md)) then carries the rocket forward, one short step
at a time.

Burning propellant makes the rocket lighter, moves its centre of mass, and lets the exhaust carry
away some of the rocket's turning (jet damping). The equations keep all three. They are
RocketPy's, from its technical documentation ([RP-EOM], under *Code and sources*), and are written
out under *Equations of motion*.

## Code and sources

Code: `hpr_sim::{dynamics, flight, rail, recorder, state}`. Decisions: [ADR-011][adr-011]
(equations of motion, aerodynamic coupling, rail, phases and termination). The integrator and
events are in [Time integration](integration.md), and the frames in [Frames](frames.md).

Sources:

- **[RP-EOM]** RocketPy technical documentation, "Equations of Motion" v0 (Kane's method with the
  Reynolds transport theorem) and v1 (the form solved), `docs/technical/equations_of_motion.rst`
  and `equations_of_motion_v1.rst` in RocketPy 1.13.0 (`refs/rocketpy`, MIT).
- **[C21]** G. H. Ceotto, R. N. Schmitt, G. F. Alves, L. A. Pezente and B. S. Carmo, "RocketPy:
  Six Degree-of-Freedom Rocket Trajectory Simulator", *J. Aerosp. Eng.* 34(6), 2021,
  doi:10.1061/(ASCE)AS.1943-5525.0001331. Cited, not fetched.

## State

- **Components.** The state is the nose tip `O`'s position `r_O` and velocity `v_O` in the
  [launch frame](../glossary.md#launch-frame-enu) `L`, the attitude quaternion `q` (four numbers
  that give the rocket's orientation, turning body axes into `L`'s), and the body rates `ω` (how
  fast it turns about each [body axis](../glossary.md#body-frame)) relative to `L`, in body axes:
  13 components.
- **Why the nose tip.** It is fixed in the body and is the body origin ([ADR-007][adr-007], the
  design tree). The centre of mass `r` (from `O`, body axes) moves as propellant burns.
- **The quaternion.** Its norm (its length, 1 for a pure rotation) drifts slightly between steps.
  Every use normalizes it, and it is never reset at an event ([Time integration](integration.md)).

## Equations of motion

[RP-EOM] v0 derives the motion of a variable-mass system (one whose mass changes as it flies)
about a point fixed in the body. It uses two standard tools:

- Kane's equations, a systematic way to write the equations of motion, for the rigid parts;
- the Reynolds transport theorem, which accounts for mass flowing across a boundary, for the
  propellant and the gas leaving the nozzle.

Its assumptions:

- the gas flow inside the motor is quasi-steady (it changes slowly enough to treat as steady at
  each instant), so the integrals over the space it flows through, the control volume, don't
  change;
- the flow is axisymmetric: the same all round the rocket's axis;
- the exhaust momentum is lumped into the thrust at the nozzle exit.

v1 solves the result for `v̇` and `ω̇`, the rates of change of the velocity and of the body rates.
The lines below are that form, in body axes, with primes for time derivatives seen from the
rocket. `T03`, `T04`, `T20` and `T21` are labels carried over from [RP-EOM] v1, so each line can
be matched against it:

- `T03` comes from the momentum of mass moving inside the rocket: gas flowing to the nozzles, and
  the centre of mass shifting as propellant burns. The rotation turns that momentum into a force,
  `ω×T03`, as it does for anything moving inside a turning body (a Coriolis force).
- `T04` is the thrust `T` with the propellant's internal-momentum terms (*Measured thrust curves
  and the internal-momentum terms*, below).
- `T20` gathers every force: `T04`, the weight `W`, the air force `A`, and two terms from the
  rotation, `ω×T03` and `−ω×(ω×m r)`, the second because the centre of mass is not at `O`.
- `T21` gathers every moment about `O`: from the rotation itself, from the jet and the changing
  inertia (`Σ ṁ_k S_k − I_O′`), and from the weight, the air and the thrust.
- The last three lines give the angular acceleration `ω̇`, the nose tip's acceleration `a_O`, and
  the rate at which the attitude `q` changes.

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
- **Nozzle gyration tensor.** `S_k` describes how the exhaust leaving motor `k`'s exit disc is
  spread about `O`; times the mass flow `ṁ_k`, it gives the jet damping in `T21`.
  `S_k = (r_e²/4) diag(1, 1, 2) + |n_k|² 1 − n_k n_kᵀ`, integrated here from v0's boxed
  rotational equation. The jet term there is `∫ r×(ω×r) dṁ` over the exit disc of
  radius `r_e`, with a uniform jet. For a disc at `n_k` on the axis, this gives
  `(r_e²/4 + n_k², r_e²/4 + n_k², r_e²/2)`.
  - RocketPy 1.13.0's code (`rocket.py:984-985`, `evaluate_nozzle_gyration_tensor`) uses
    `0.25·n²` for the transverse distance term instead of `n²`. Its `n` is measured from the centre
    of dry mass, not from the nose tip, so the entries can't be compared directly. The RocketPy
    code-to-code suite ([M2.1](../decisions-and-roadmap.md#m2-1)) should compare the jet-damping coefficient about the
    centre of mass.
  - A motor that gives no nozzle exit radius contributes `r_e = 0`.
- **The classical limit.** For an axisymmetric rocket turning slowly about a transverse axis, the
  equations reduce exactly to the classical jet damping about the centre of mass,
  `I_c ω̇ = [ṁ (r_e²/4 + l²) − İ_c] ω`, with `l` from the centre of mass to the nozzle exit. The
  `ṁ l²` part damps, and the falling inertia partly offsets it
  (`dynamics::tests::jet_damping_matches_the_classical_form`).
- **Time derivatives of the mass properties.**
  - `r′`, `r″`, `ṁ` and `I_O′` come from central differences of `Assembly::mass_properties` with a
    half-width of 1e-4 s.
  - The stencil, the times each difference samples, is kept inside the current integration
    interval. Every thrust-curve knot and burnout is a stop time, so no difference straddles a
    change in the thrust's slope. Near an interval's end the derivative is taken at the nearest
    valid centre and extended linearly.
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
  - hpr keeps the terms as RocketPy does, for parity in the RocketPy comparison ([M2.1](../decisions-and-roadmap.md#m2-1)).
    The form is exact for the [RP-EOM] model, not for a measured curve.
  - The size of the double count on Valetudo: it lifts off at 1.56 ms with 73 N of thrust against
    95 N of weight, 21 N coming from `m̈(n − r)`, and it changes the burnout speed by at most
    0.05 m/s.
- **Earth's rotation.** It enters only through the Coriolis force. The rotational equations use
  `ω` relative to `L`, which differs from the inertial rate by at most 7.3e-5 rad/s ([Frames](frames.md),
  and the rigid-body flight decision [ADR-011][adr-011]).

## Aerodynamics in flight

`hpr-aero` gives coefficients at a flow condition ([Aerodynamics](aero.md)). The engine applies them as follows.

- **Airspeed.** The air velocity uses the wind at the centre of mass's height. Wind vectors are
  taken in `L`'s axes.
- **Axial force.** `−q A C_A z_B` from the whole rocket's drag, at the centre of mass's airspeed,
  Mach number, angle of attack and Reynolds number per metre (`V/ν`). The drag is power-on
  (`DragConditions::thrusting`, with the burning motors' cross-section) while any motor burns in
  the interval. It acts along the axis, so it has no moment about `O`.
- **Normal and side forces, component by component.** Each body and fin set is evaluated at its
  own local flow: `v_O − wind + ω × p_i` at its small-angle centre of pressure `p_i`
  (`AeroModel::component_station_m`). Its `C_N` acts along the crossing air `ŵ` and its `C_Y`
  along `z_B × ŵ` ([Frames](frames.md)), with moments `−q A M_N (z_B × ŵ) + q A M_Y ŵ` about `O` from the
  component's moment coefficients.
- **Fins at any angle of attack.** A fin set's normal force follows the crossflow `V sin α`: its
  model's `C_N = C_Nα α` is used as `C_Nα sin α`, the substitution Niskanen keeps for bodies (eq.
  3.16–3.17). No source in hand gives a large-angle fin model, and stall is not modelled.
  - At small angles nothing changes (1.5% less at 0.3 rad).
  - The force now vanishes for axial flow from the tail as well as from the nose.
  - A force linear in `α` stays large at `α = π` and flips with rounding noise in the lateral
    velocity. A calm vertical flight falling tail first after apogee then collapsed the step size
    and never landed (`tests::a_calm_vertical_flight_falls_tail_first_and_lands`).
- **Damping.** The rotation's contribution to each local flow is the only aerodynamic pitch and
  yaw damping. At a component's CP the rotation adds a velocity `ω × p_i`, which changes the angle
  at which the air meets that component.
  - Only components with a [normal-force slope](../glossary.md#normal-force-slope) damp this way:
    nose cones, transitions and fin sets. A boattail's slope is negative, so it takes some away.
  - Body tubes give none at small angles: their own slope is 0, and their body lift grows with
    `sin² α`.
  - `hpr-aero` has no pitch or yaw damping coefficients; they would have to replace the
    local-flow damping, not add to it.
  - A flight on another tool's normal force keeps this damping. The table
    ([The normal force from RASAero II](aero.md#the-normal-force-from-rasaero-ii)) gives the
    normal force at the centre of mass's airflow, and each component adds only its force in its
    own local flow minus its force in the centre of mass's airflow: the part the rotation makes.
    Without rotation that part is exactly zero ([ADR-032][adr-032]).
- **Roll.** The fins' cant drives the roll and the roll rate damps it: a moment
  `q A d (C_l0 cos α + C_lp p d/2V)` about `z_B`, with `q` the dynamic pressure, `A` and `d` the
  reference area and diameter, `C_l0` the cant's rolling moment, `C_lp` the damping and
  `p = ω_z`, at the centre of mass's Mach number; the cant's forcing follows the axial flow,
  `cos α`, so it vanishes broadside and reverses tail first
  (`AeroModel::roll`, [Roll: forcing and damping](aero.md#roll-forcing-and-damping)). The damping
  is written `ρ V A d² C_lp p/4`, so it fades smoothly as the airspeed does. The roll rate also
  changes through inertia coupling (turning about one axis driving turning about another) for a
  rocket whose mass isn't symmetric about its axis, and, while a motor burns, through the jet and
  the falling inertia in `T21`. At constant speed a canted rocket spins up to the steady rate
  where cant and damping balance, as the closed form gives
  (`tests::canted_fins_spin_to_the_analytic_balance`).
- **Limits.**
  - The models are small-angle: no stall, and body lift and fins extended by `sin α`. They
    overstate the forces at large `α`. In normal flights large `α` occurs near apogee, where the
    dynamic pressure is small, and off the rail in strong crosswinds. `Sample::angle_of_attack_rad`
    shows where it happens.
  - The normal force and the drag buildup cover Mach 0 to 5
    ([Fins through Mach 1](aero.md#fins-through-mach-1),
    [Drag through Mach 1](aero.md#drag-through-mach-1), and [ADR-028][adr-028], the drag's
    decision); a fin set's CP moves with the Mach number, and each component takes its local
    airflow at its CP for the centre of mass's Mach number. At Mach 5 or faster they refuse the
    flow, and the flight stops with `SimError::Aero`. A drag override table covers drag at any
    Mach, but the normal force still stops at Mach 5.

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
    rocket guided to the last guide ([Loft lesson L26](../decisions-and-roadmap.md#l26)).
  - The pivot about the last guide ("tip-off") is not modelled, and the rocket leaves the rail
    with no angular velocity.
  - Friction is Coulomb friction: a user coefficient times the net reaction `|ΣN|`, the force
    pressing the guides onto the rail. With a moment across two buttons (a crosswind at low
    speed), the true friction `μ(|N₁| + |N₂|)` is larger. No source gives a default coefficient,
    so the default is zero.

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
  ([Frames](frames.md), [Loft lesson L35](../decisions-and-roadmap.md#l35)).
- **User events.** A function of the `Sample`, in free flight.
- **Heights.** Atmosphere and wind heights are `h − N`, the height
  [above sea level](../glossary.md#height-above-sea-level-msl): `h` is the ellipsoidal height, and
  the geoid undulation `N`, the height of sea level above the ellipsoid, is given in `Environment`
  (no geoid model).
- **Termination** ([Loft lesson L25](../decisions-and-roadmap.md#l25)). The flight ends in exactly one of these ways:
  - `GroundHit`;
  - `NoLiftoff` (the last burnout passes on the pad);
  - `StalledOnRail` (it lifted off, then stopped on the rail after burnout);
  - `TimeCap`;
  - `StepLimit`.

  Any other failure is an error.
- **Not yet modelled.** Staging, delayed ignition (planned with staging and airstarts in
  [M1.9](../decisions-and-roadmap.md#m1-9)), turbulence and thrust misalignment. Recovery is modelled, and has
  [its own page](recovery.md).

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
    located (root-found) on the exact ellipsoidal height.
- **Terminal velocity.** Falling nose down through uniform air with `C_D = 0.5`, the speed follows
  `v_t tanh(g t/v_t)` to 5.5e-9 `v_t`, and is `v_t` to 1e-5 after 150 s.
- **Torque-free precession** (the wobble of a spinning body with no torque on it). Valetudo
  (`I_a/I_t` = 0.0019) spinning at 25 rad/s with a transverse rate turns in body axes at
  `Ω = (I_a − I_t) ω_z/I_t` = −24.95 rad/s. The rate matches to 4.7e-7 rad/s over 3 s, and the
  angular momentum in `L` holds to 1.5e-6.
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
- **[Loft lesson L20](../decisions-and-roadmap.md#l20), weathercocking.** In a 5 m/s wind from the west, Valetudo's unit
  axis has an east component of −0.12 at burnout. Its apogee is 96 m upwind, against 1.0 m (Earth
  rotation) in calm air.
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
- **[Loft lesson L24](../decisions-and-roadmap.md#l24).** Two runs of one `Simulation`, and a second `Simulation` built the
  same way, record the same bits for every channel at every step.
- **[Loft lesson L25](../decisions-and-roadmap.md#l25).** A normal flight hits the ground. A rail with `μ = 20` at 60° gives
  `NoLiftoff` at rest. A 5 s cap gives `TimeCap` at exactly 5 s, and a 40-step limit gives
  `StepLimit`.
- **[Loft lesson L26](../decisions-and-roadmap.md#l26).** On a 3 m rail tilted to 1.3 rad, the rail exit comes at the last
  button's travel to 1e-6 m. Across the rail the rocket stays within 1e-9 m, with no rotation.
  Friction (`μ = 0.3`) delays the exit and slows it.
- **Events and recorder.** Events come in order: liftoff, rail exit, burnout, apogee, ground hit.
  Apogee's vertical speed is below 1e-6 m/s and ground contact's height below 1e-6 m. Recorder rows
  fall on the interval or at events.
- **Through Mach 1.** The synthetic 54 mm rocket on an I175 passes Mach 1 and lands, both on hpr's
  own drag and on a constant drag table.
- **Cost.** About 1.1 ms per Valetudo flight to the ground (`docs/perf.md`).

### Whole flights against RocketPy

`cargo xtask validate` flies six of RocketPy's example rockets from the pad to the ground and
compares fifteen numbers of each with RocketPy 1.13.0's own flight
([ADR-021][adr-021], the whole-flight comparison). Both codes fly one declared drag coefficient,
a constant `C_D0` of 0.5 on the same reference area. So what is compared is the equations of
motion, the motor and the air, not the drag. The rest is RocketPy's where hpr has it: its gravity
formula, standard atmosphere, frictionless rail, the example's rail angles, parachutes and motor,
and a declared wind.

| result | value |
|---|---|
| cases scored | 6, one of them (Prometheus 2022) past Mach 1, since [M1.8a](../decisions-and-roadmap.md#m1-8a) |
| height, speed, time, acceleration | all scored, all within 3% of RocketPy's |
| largest of those | +1.783%, Bella Lui's peak acceleration, on the rail |
| largest in apogee | +1.525%, Prometheus 2022; RocketPy flown with hpr's body lift and rail release comes within 0.01% ([case file](https://github.com/nrdptel/hpr-sim/blob/main/validation/cases/flight-prometheus-2022-generic-motor.toml)) |
| path without wind (drift of apogee and landing) | all scored, within 2.2% (largest −2.141%, Bella Lui's calm landing) |
| path in wind | Calisto's scored (largest +1.433%), and NDRT 2020's landing; Juno III's, Bella Lui's and Prometheus 2022's, and NDRT 2020's apogee drift, differ by 4.7 to 43% and are reported, not scored: hpr's body lift and rail release, and Juno III's fin slope ([ADR-026][adr-026]) |

What the two codes still do differently, and how much it moves:

- **The wind.** A rocket that leaves the rail slowly in a wind meets the air at a steep angle:
  Juno III at 18 m/s in an 8.5 m/s wind, 26° off the airflow. There hpr's normal force includes
  body lift ([Aerodynamics](aero.md#bodies-of-revolution)), which RocketPy's leaves out. Much of
  it acts ahead of the centre of mass, the nose's above all, so it moves the centre of pressure
  forward and weakens the turn into the wind, and hpr turns into it less: Juno III's apogee is
  228.0 m from the pad in hpr and 396.6 m in RocketPy. Given hpr's body lift, its rail release and
  its flat-plate fin slope (it cannot model the airfoil lift curve Juno III's example gives its
  fins), RocketPy puts it 231.1 m out, and every windy drift within 1.4% of hpr's
  ([ADR-026][adr-026]). hpr's growth of drag with the angle of attack moves no drift by more than
  0.1%.
- **RocketPy's equations, corrected.** hpr's equations of motion follow RocketPy's technical
  documentation, which measures the centre of mass from the
  [centre of dry mass](../glossary.md#centre-of-dry-mass). RocketPy 1.13.0's code reads that
  vector the other way round, so during the burn it takes the turning moments about the wrong point
  and its rockets turn into the wind too far. The fix is proposed in [a pull request to RocketPy](https://github.com/RocketPy-Team/RocketPy/pull/1196),
  still open, built on [one that is merged](https://github.com/RocketPy-Team/RocketPy/pull/1188) but not yet released; RocketPy 1.13.0 as installed still has
  the error, and the comparison applies both fixes. Without them,
  hpr's drifts in wind were up to −60.8% short of RocketPy's at apogee and +151% beyond it at
  landing ([ADR-026][adr-026],
  [issue #50](https://github.com/nrdptel/hpr-sim/issues/50)).
- **The rail.** hpr's rail equation keeps the terms for the centre of mass moving inside the
  body as the propellant burns. RocketPy's rail equation (`udot_rail1`) leaves them out. At a
  sharp ignition spike, with thrust and mass the same to five digits, hpr's acceleration is 1.2
  to 1.3 m/s² higher. hpr also guides the rocket until its last rail button leaves, where
  RocketPy frees it at the first.
- **Calisto's two peaks.** Its acceleration peaks twice, 0.9% apart, and the rail terms make hpr's
  maximum the other peak. So the time of the peak (−96.8%) is reported but not scored.
- **The parachutes.** RocketPy counts the air a canopy drags along (added mass); hpr has none. So
  the peak deceleration as NDRT 2020's main opens is 83% higher in hpr, and that number is
  reported but not scored. The speeds at landing agree within 0.03%.

The metrics are measured as RocketPy defines them: speeds and accelerations at the centre of dry
mass; heights from that point's height at launch, since RocketPy's starts at the ground; and the
rail exit when the rocket has travelled RocketPy's `effective_1rl`, the forward button at the top
of the rail, where hpr's own rail-exit event waits for the last one. Each case file argues its
tolerances; [Accuracy][accuracy] gives every result.

[adr-007]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-007-design-tree-stations-placement-automatic-radii-overrides-motors-and-checks-2026-09-17
[adr-011]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-011-rigid-body-flight-equations-of-motion-aerodynamic-coupling-rail-phases-and-termination-2026-09-17
[adr-032]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-032-normal-force-overrides-from-rasaero-ii-the-static-force-replaced-hprs-damping-kept-2026-09-19
[report]: https://github.com/nrdptel/hpr-sim/blob/main/validation/reports/latest.md
[adr-021]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-021-whole-flights-against-rocketpy-what-is-compared-and-the-gaps-it-may-declare-2026-09-18
[adr-026]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-026-the-path-in-wind-rocketpys-corrected-equations-and-hprs-body-lift-2026-09-18
[adr-028]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-028-drag-through-mach-1-niskanens-appendix-b-stoneys-curves-and-the-arcas-robins-axial-force-2026-09-18
[accuracy]: ../accuracy.md#whole-flights-against-rocketpy
