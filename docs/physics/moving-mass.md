# Moving mass

## In short

- **What it models:** a part carried inside the airframe, such as ballast or a payload, sliding
  along the rocket's axis during the flight on a trigger. The rocket's
  [centre of mass](../glossary.md#centre-of-gravity-cg) and inertia follow the part, and so do
  the equations of motion. Use it to see what a payload that moves, or ballast moved on purpose,
  does to the [stability margin](../glossary.md#stability-margin) and the flight. This is new
  with [M1.12a](../decisions-and-roadmap.md#m1-12a) (a mass that moves along the airframe).
- **Sources:** the [parallel-axis theorem](../glossary.md#parallel-axis-theorem) and the kinetics
  of a system of particles, from Meriam and Kraige's *Engineering Mechanics: Dynamics*, applied to
  the equations of motion on [Rigid-body flight](flight.md); the cycloidal motion of cam design,
  from Norton's *Design of Machinery*.
- **How well it is validated:** against exact answers only. No other simulator or real flight has
  been compared with it. The mass properties match a hand calculation to 1e-15, and the static
  margin to 1e-12 [calibres](../glossary.md#calibre-caliber). In free flight with nothing acting
  on the rocket, its angular momentum is kept to 6.9e-12 of itself and its centre's velocity to
  2.6e-12 m/s.
- **What it leaves out:** a mass that leaves the rocket (the next increment,
  [M1.12b](../decisions-and-roadmap.md#m1-12b)); a shift in a flight that also separates or ejects
  pieces; a shift before the rocket leaves the rail; a part that moves across the axis or turns;
  and the shock of a part hitting a stop. The motion's shape is a choice made here, not something
  a source gives for rockets.

## Describing a shift

A `MassShift` names the part, how far it moves, how long it takes, and when it starts:

| field | meaning |
|---|---|
| `trigger` | when it starts: the same triggers a parachute has (apogee, a height on the way down, a time after launch, a motor's burnout or delay) |
| `component` | the `id` of the internal component that moves; everything inside it moves too |
| `travel_m` | how far along the axis, in metres: positive toward the tail, negative toward the nose |
| `duration_s` | how long the move takes, in seconds: at least 0.01 s |

In code, for a part with the id `ballast` that slides 0.3 m toward the tail over 1 s, starting 5 s
after launch:

```rust,ignore
let flight = simulation.with_shifts(vec![MassShift::new(
    Trigger::Time { time_s: 5.0 },
    "ballast",
    0.3,
    1.0,
)])?;
```

The part is any internal component of your design, such as a mass component. Each shift that
starts is recorded as an `EventKind::Shift` event. `Simulation::mass_properties(flight, t)` gives
the mass, centre of mass and inertia the flight had at any time.

Shifts of one part add up. hpr can't know before the flight which triggers will fire, so it adds
all of a part's moves toward the nose, and separately all of its moves toward the tail, and
refuses either total if it would take the part out of the component that holds it. A part the
design already places partly outside its holder, such as a weight in a nose cone's shoulder, may
go as far as it already reaches.

Some parts can't move, and hpr says which rule a refused one breaks:

- a body component, or a part on the outside of the airframe;
- a part holding a motor, since the motor would not go with it;
- a part inside another part that moves;
- one tube of a [cluster](../glossary.md#cluster);
- a part covered by an [override](../glossary.md#override) of mass: an override on its stage, or
  one on a component around it that covers what that component holds. The override doesn't say
  how much of that mass is the part's.

A flight with a separation or ejections can't have shifts yet. A shift can't start before the
rocket leaves the rail either: the rail has no stop at its foot, so a part thrown toward the tail
on the pad could push the rocket up the rail and leave it hanging there. hpr stops the flight
with an error if one would.

**The motion.** A part doesn't jump from one place to the next. It follows a cycloid, the curve a
cam designer calls cycloidal motion. It starts at rest, speeds up, and slows to rest again. Its
acceleration is zero at both ends, so neither its speed nor its acceleration jumps. With `τ` the
fraction of the move's time gone, the part is `s(τ)` of the way along:

```text
s(τ) = τ − sin(2πτ) / 2π,     0 ≤ τ ≤ 1
```

At `τ = ½` it is exactly halfway, moving fastest, at twice the average speed. Its acceleration
peaks at `2π × travel / T²` for a move taking `T`. A move shorter than 10 ms is refused: the peak
there is 63 km/s² for each metre of travel, which is closer to an impact than a motion.

The move's time is cut into 16 equal parts by [stop times](../glossary.md#stop-time), so the
integrator takes at least 16 steps across it. That matters with fixed-step
[RK4](../glossary.md#dormandprince-and-rk4): a 10 ms step would otherwise take a 10 ms move in one
step.

## What the rocket's mass properties do

Moving a part inside the rocket doesn't change the rocket's mass. It moves the centre of mass
the same way as the part, by the part's share of the mass. With `M` the rocket's mass, `m` the
part's and `Δ` how far it has moved:

```text
centre of mass moves by  m Δ / M
```

The inertia changes as well. hpr takes the part's contribution out where it was and puts it back
where it is, both about the new centre of mass, by the parallel-axis theorem. The part's own
inertia about its own centre goes with it unchanged. Write `c₀` for the part's centre where the
design puts it, `δ` for the move, `cg_new` for the rocket's new centre, `I_new` for its inertia
about that centre, and `J(v) = |v|² E − v vᵀ` (with `E` the unit matrix):

```text
I_new = (the rocket's inertia before the move, about cg_new)
        + m [ J(c₀ + δ − cg_new) − J(c₀ − cg_new) ]
```

As a check by hand, think of the rocket as two bodies: the part and everything else. About the
centre of mass, their inertia is each one's own plus `μ J(L)`. Here `μ = m (M − m) / M` is the
reduced mass and `L` is the line from the rest's centre to the part's. Moving the part changes
only `L`. The tests check hpr's inertia against this formula.

**The stability margin.** The static margin is the distance from the centre of mass back to the
[centre of pressure](../glossary.md#centre-of-pressure-cp), in calibres of the rocket's diameter
`d` ([Flight metrics: stability margins](metrics.md#stability-margins)). It takes the centre of
pressure with the air along the axis at Mach 0, which doesn't depend on where the mass is. So a
part moving `Δ` toward the tail changes it by:

```text
change in static margin = − m Δ / (M d)
```

The flight margin, at the flight's own Mach number, moves by the same amount at any one instant,
since only its centre of pressure differs.

## What the equations of motion add

The equations of motion ([Rigid-body flight](flight.md)) are written about the nose tip, `O`, a
point fixed in the airframe, in body axes. `ω` is the airframe's rate of turn and `r` the centre
of mass seen from `O`. The equations already allow for a centre of mass that moves through the
airframe and an inertia that changes, because burning propellant does both. Primes there, as
below, mean how fast something changes with time as seen from the airframe: `r′` and `r″` are the
centre of mass's velocity and acceleration through the airframe, and `I′` how fast the inertia
about `O` changes. A part that moves adds to those terms, and adds one term more.

Sum Newton's law over every particle of the rocket. Each particle's acceleration is the nose
tip's acceleration, plus the airframe's rotation, plus its own motion inside the airframe. In the
force equation, the particles' motion inside the airframe gives the `r′` and `r″` terms, with
nothing more. In the moment equation it gives `I′ ω`, and also this term:

```text
ω × h + h′,     h = Σ m ρ × ρ′
```

`h` is the moving parts' angular momentum about the nose tip, relative to the airframe. `ρ` is a
part's centre and `ρ′` its velocity inside the airframe. A part only slides, so every point of it
moves at the same `ρ′`. When the part is on the axis, `ρ` and `ρ′` both lie along the axis and `h`
is zero. When it is off the axis, `h` is not zero, and hpr carries it: `T21`, the sum of the
moments about the nose tip in the equations, subtracts `ω × h + h′`.

hpr works out how fast the centre of mass and the inertia change exactly, from the cycloid. For a
burning motor it still estimates them from points 0.1 ms apart. The two add, so a part can move
while the motor burns:

```text
r′ = r_a′ + Σ m (δ′/M − δ M′/M²)
r″ = r_a″ + Σ m (δ″/M − 2 δ′ M′/M² + δ (2 M′²/M³ − M″/M²))
I′ = I_a′ + Σ m (2 (c · δ′) E − δ′ cᵀ − c δ′ᵀ)
```

Here `r_a` and `I_a` are the rocket's with every part where the design puts it, `M′` and `M″`
are how fast its mass changes and how fast that changes (both zero once the motor is out), `δ′`
and `δ″` are the part's velocity and acceleration along its move, and `c = c₀ + δ` is the
part's centre now.

## A worked example

The example
[`moving_ballast.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/moving_ballast.rs)
flies the project's 54 mm test design (`validation/designs/synthetic-54mm-three-fin.json`, a
body 56.3 mm across) on an I175 motor ([motor designation](../glossary.md#motor-designation)),
with 200 g of ballast in its airframe. The ballast is a cylinder 50 mm long on the axis, its
centre 0.375 m aft of the nose tip. At 5 s, well after the 2.5 s burn, it slides 0.3 m toward
the tail over one second. What the example prints (the transverse inertia is about an axis across
the rocket through its centre of mass, the one it pitches about):

| time (s) | mass (kg) | centre of mass (m aft of the nose tip) | transverse inertia (kg·m²) |
|---:|---:|---:|---:|
| 4.00 | 0.8188 | 0.6681 | 0.09556 |
| 5.25 | 0.8188 | 0.6748 | 0.09248 |
| 5.50 | 0.8188 | 0.7048 | 0.08138 |
| 5.75 | 0.8188 | 0.7347 | 0.07483 |
| 7.00 | 0.8188 | 0.7414 | 0.07399 |

By hand, the centre of mass moves `0.2 × 0.3 / 0.8188 = 0.0733` m aft. At 5.5 s, halfway through
the move, it has gone exactly half of that. The transverse inertia falls because the ballast ends
up near the centre of mass. The two-body check gives the change too:

- `μ = 0.2 × 0.6188 / 0.8188 = 0.1511` kg.
- The rest of the rocket, 0.6188 kg, has its centre at `(0.8188 × 0.6681 − 0.2 × 0.375) / 0.6188
  = 0.7628` m aft of the tip.
- The ballast's centre moves from 0.375 m to 0.675 m, so its distance from the rest's centre
  shrinks from 0.3878 m to 0.0878 m.
- The inertia changes by `0.1511 × (0.0878² − 0.3878²) = −0.0216` kg·m², as the table shows.

The static margin falls from 4.297 calibres to 2.996, a change of −1.302 from the unrounded
values. By hand it is `−0.0733 / 0.0563 = −1.302`, with `d = 0.0563` m. The apogee barely moves:
1749.8 m with the ballast moving, 1749.9 m with it held still.

## How it is checked

The tests are in `hpr_sim::shifts` and `hpr_design::mass`. The last column is what each test
measured, and the bound it holds the code to.

| test | what it shows | measured (bound) |
|---|---|---|
| `a_part_moved_inside_a_body_gives_the_hand_computed_whole` | a point moved inside a cube matches a hand calculation, and the whole rebuilt with the point moved | exact (1e-15) |
| `mass_properties_before_during_and_after_a_shift_match_the_hand_calculation` | the example's rocket before, halfway and after, against the two-body formula | (1e-15 m and kg·m²) |
| `a_moving_mass_shifts_the_static_margin_by_the_hand_calculation` | the static margin at every coast sample of the flight equals `−m Δ s(τ)/(M d)` from its start | (1e-12 calibres) |
| `a_part_moving_off_the_axis_keeps_both_momenta_in_free_space` | no air or gravity, the rocket turning about all three axes, the ballast 1 cm off the axis: the angular momentum about the centre and the centre's velocity stay constant | 6.9e-12 of the angular momentum (1e-10), 2.6e-12 m/s (1e-10) |
| `a_shift_s_rates_are_the_derivatives_of_its_mass_properties` | the exact rates against differences of the mass properties, during the burn and after it | `r′` 2e-11 m/s (1e-9), `r″` 3e-8 of itself (1e-7) |
| `a_fixed_step_follows_a_short_shift_in_its_stops` | the free-flight case with RK4 at 10 ms steps and a 10 ms move: 16 steps across it | 1.2e-4 m/s (3e-4) |
| `a_canopy_that_opens_while_the_ballast_moves_keeps_the_centre_s_velocity` | a drogue opening halfway through a move keeps the centre's velocity, and the rocket lands at the rate it has with the ballast still | (1e-12 m/s, 1e-6 m/s) |
| `a_shift_starts_at_apogee_or_at_its_height_on_the_way_down` | the triggers the flight watches for start the move at the apogee, and at 200 m on the way down | (1e-6 m) |

The free-flight test is the one that checks the new term. Without `ω × h + h′`, its error would be
the size of the ballast's own angular momentum relative to the airframe, which peaks at 1.8% of
the whole. It measures 6.9e-12.

A first version took the part's rates from differences 0.1 ms apart, as the motor's are. In free
flight it kept the centre's velocity only to 1e-8 m/s, the error of the difference itself, so the
rates were made exact. With RK4 taking a 10 ms move in a single step, review measured the
centre's velocity off by 0.071 m/s; the 16 stop times across each move bring it to 1.2e-4.

Every refusal has a test that checks which rule fired, in `shifts_that_cannot_be_made_are_refused`,
`refusals_that_need_a_design_of_their_own` and
`triggers_a_shift_cannot_have_are_refused_in_its_own_words`.

## What it leaves out

- **Releasing a mass.** Ballast or a payload that leaves the rocket while the rest flies on is
  [M1.12b](../decisions-and-roadmap.md#m1-12b).
- **Shifts with a separation or ejections.** Their pieces are fixed before the flight, with every
  part where the design puts it.
- **Shifts on the pad or the rail**, as above.
- **Other motions.** A part can only slide along the axis, at whatever distance from the axis the
  design puts it. It can't move across the axis or turn.
- **Stops.** The cycloid brings the part to rest smoothly. A real mechanism may stop it with a
  shock, which a rigid airframe can't model.
- **Warnings.** hpr doesn't warn when a shift takes the margin below a safe value; read the margin
  from the flight's metrics ([Flight metrics](metrics.md#stability-margins)).

The decision record is [ADR-087][adr-087].

## References

- **[MK]** J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*: chapter 4 (kinetics
  of systems of particles: the force and moment equations summed over particles, and angular
  momentum about a moving point) and appendix B (the parallel-axis theorem and the inertia
  tensor).
- **[N]** R. L. Norton, *Design of Machinery*, the chapter on cam design (cycloidal displacement:
  `s = h [θ/β − sin(2πθ/β)/2π]`).
- The equations of motion this adds to: the RocketPy technical documentation, "Equations of
  Motion" v0 and v1 ([Rigid-body flight](flight.md)).

[adr-087]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-087-mass-that-moves-along-the-airframe-2026-09-26
