# Moving mass

## In short

- **What it models:** a part carried inside the airframe, such as ballast or a payload, sliding
  along the rocket's axis during the flight on a trigger. The rocket's
  [centre of mass](../glossary.md#centre-of-gravity-cg) and inertia follow the part, and so do
  the equations of motion. This is new with [M1.12a](../decisions-and-roadmap.md#m1-12a) (a mass
  that moves along the airframe).
- **Sources:** the parallel-axis theorem and the kinetics of a system of particles, from Meriam
  and Kraige's *Engineering Mechanics: Dynamics*, applied to the equations of motion on
  [Rigid-body flight](flight.md); the cycloidal motion of cam design, from Norton's *Design of
  Machinery*.
- **How well it is validated:** against exact answers only. No other simulator or real flight has
  been compared with it. The mass properties match a hand calculation to 1e-15, and the
  [stability margin](../glossary.md#stability-margin) to 1e-12
  [calibres](../glossary.md#calibre-caliber). In free flight with nothing acting on the rocket,
  its angular momentum is kept to 6.9e-12 of itself and its centre's velocity to 2.6e-12 m/s.
- **What it leaves out:** a mass that leaves the rocket (the next increment,
  [M1.12b](../decisions-and-roadmap.md#m1-12b)); a shift in a flight that also separates or ejects
  pieces; a part that moves across the axis or turns; and the shock of a part hitting a stop. The
  motion's shape is a choice made here, not something a source gives for rockets.

## Describing a shift

A `MassShift` names the part, how far it moves, how long it takes, and when it starts:

| field | meaning |
|---|---|
| `trigger` | when it starts: the same triggers a parachute has (apogee, a height on the way down, a time after launch, a motor's burnout or delay) |
| `component` | the `id` of the internal component that moves; everything inside it moves too |
| `travel_m` | how far along the axis, in metres: positive toward the tail, negative toward the nose |
| `duration_s` | how long the move takes, in seconds: at least 0.01 s |

Give the shifts to the flight with `Simulation::with_shifts`. Each shift that starts is recorded
as an `EventKind::Shift` event. `Simulation::mass_properties(flight, t)` gives the mass, centre
of mass and inertia the flight had at any time.

Two shifts of the same part add up. The part must stay inside the component that holds it:
hpr adds all of a part's moves toward the nose, and all of its moves toward the tail, and refuses
either total if it would take the part out.

Some parts can't move, and hpr says which rule a refused one breaks:

- a body component, or a part on the outside of the airframe;
- a part holding a motor, since the motor would not go with it;
- a part inside another part that moves;
- one tube of a cluster;
- a part whose mass is inside an override on its stage, or on a component around it, since the
  override doesn't say how much of that mass is the part's.

A flight with a separation or ejections can't have shifts yet.

**The motion.** A part doesn't jump from one place to the next. It follows a cycloid, the curve a
cam designer calls cycloidal motion. It starts at rest, speeds up, and slows to rest again. Its
acceleration is zero at both ends, so neither its speed nor its acceleration jumps. With `τ` the
fraction of the move's time gone, the part is `s(τ)` of the way along:

```text
s(τ) = τ − sin(2πτ) / 2π,     0 ≤ τ ≤ 1
```

At `τ = ½` it is exactly halfway, moving fastest, at twice the average speed. A move shorter than
10 ms is refused. At that length the peak acceleration is 63 km/s² for each metre of travel, which
is closer to an impact than a motion.

## What the rocket's mass properties do

Moving a part inside the rocket doesn't change the rocket's mass. It moves the centre of mass
the same way as the part, by the part's share of the mass. With `M` the rocket's mass, `m` the
part's and `Δ` how far it has moved:

```text
centre of mass moves by  m Δ / M
```

The inertia changes as well. hpr takes the part's contribution out where it was and puts it back
where it is, both about the new centre of mass, by the parallel-axis theorem. The part's own
inertia about its own centre goes with it unchanged. With `c` the part's centre before the move,
`δ` the move, `cg'` the new centre and `J(v) = |v|² E − v vᵀ` (with `E` the unit matrix):

```text
I' = I about cg'  +  m [ J(c + δ − cg') − J(c − cg') ]
```

As a check by hand, think of the rocket as two bodies: the part and everything else. About the
centre of mass, their inertia is each one's own plus `μ J(L)`. Here `μ = m (M − m) / M` is the
reduced mass and `L` is the line from the rest's centre to the part's. Moving the part changes
only `L`. The tests check hpr's inertia against this formula.

**The stability margin.** The static margin is the distance from the centre of mass back to the
[centre of pressure](../glossary.md#centre-of-pressure-cp) in calibres, the rocket's diameter
`d`. The centre of pressure doesn't depend on where the mass is. So a part moving `Δ` toward the
tail changes the margin by:

```text
change in margin = − m Δ / (M d)
```

## What the equations of motion add

The equations of motion ([Rigid-body flight](flight.md)) are written about the nose tip, a point
fixed in the airframe. They already allow for a centre of mass that moves through the airframe
(`r′`, `r″`) and an inertia that changes (`I′`), because burning propellant does both. A part
that moves adds those same terms, and one more.

Sum Newton's law over every particle of the rocket. Each particle's acceleration is the nose
tip's acceleration, plus the airframe's rotation, plus its own motion inside the airframe. The
particles' motion inside the airframe gives the `r′` and `r″` terms in the force equation, with
nothing more. In the moment equation it gives `I′ ω`, and also this term:

```text
ω × h + h′,     h = Σ m ρ × ρ′
```

`h` is the moving parts' angular momentum about the nose tip, relative to the airframe. `ρ` is a
part's centre and `ρ′` its velocity inside the airframe. A part only slides, so every point of it
moves at the same `ρ′`. When the part is on the axis, `ρ` and `ρ′` both lie along the axis and `h`
is zero. When it is off the axis, `h` is not zero, and hpr carries it: the moment equation's
`T21` subtracts `ω × h + h′`.

The part's own rates are exact, from the cycloid, not differenced. The motor's rates are
still differenced, 0.1 ms apart, and the two are combined, so a part can move while the motor
burns:

```text
r′ = r_a′ + Σ m (δ′/M − δ M′/M²)
r″ = r_a″ + Σ m (δ″/M − 2 δ′ M′/M² + δ (2 M′²/M³ − M″/M²))
I′ = I_a′ + Σ m (2 (c · δ′) E − δ′ cᵀ − c δ′ᵀ)
```

Here `r_a` and `I_a` are the rocket's with every part where the design puts it, `M′` and `M″`
are the mass's rates (zero once the motor is out), and `c` is the part's centre now.

## A worked example

The example
[`moving_ballast.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/moving_ballast.rs)
flies the 54 mm test design on an I175 with 200 g of ballast in its airframe. The ballast is a
cylinder 50 mm long on the axis, 0.1 m aft of the airframe's forward end. At 5 s, well after the
2.5 s burn, it slides 0.3 m toward the tail over one second. What it prints:

| time (s) | mass (kg) | centre of mass (m aft of the nose tip) | transverse inertia (kg·m²) |
|---:|---:|---:|---:|
| 4.00 | 0.8188 | 0.6681 | 0.09556 |
| 5.25 | 0.8188 | 0.6748 | 0.09248 |
| 5.50 | 0.8188 | 0.7048 | 0.08138 |
| 5.75 | 0.8188 | 0.7347 | 0.07483 |
| 7.00 | 0.8188 | 0.7414 | 0.07399 |

By hand, the centre of mass moves `0.2 × 0.3 / 0.8188 = 0.0733` m aft. At 5.5 s, halfway through
the move, it has gone exactly half of that. The transverse inertia falls because the ballast ends
up near the centre of mass. The two-body check gives it too: `μ = 0.2 × 0.6188 / 0.8188 = 0.1511`
kg, and the ballast's distance from the rest's centre shrinks from 0.3878 m to 0.0878 m, so the
inertia changes by `0.1511 × (0.0878² − 0.3878²) = −0.0216` kg·m².

The static margin falls from 4.297 calibres to 2.996, a change of −1.302. By hand it is
`−0.0733 / 0.0563 = −1.302`, with `d = 0.0563` m. The apogee barely moves: 1749.8 m with the
ballast moving, 1749.9 m with it held still.

## How it is checked

The tests are in `hpr_sim::shifts` and `hpr_design::mass`:

| test | what it shows | how close |
|---|---|---|
| `a_part_moved_inside_a_body_gives_the_hand_computed_whole` | a point moved inside a cube matches a hand calculation and the whole rebuilt with the point moved | 1e-15 |
| `mass_properties_before_during_and_after_a_shift_match_the_hand_calculation` | the example's rocket before, halfway and after, against the two-body formula | 1e-15 m and kg·m² |
| `a_moving_mass_shifts_the_static_margin_by_the_hand_calculation` | the static margin at every coast sample of the flight equals `−m Δ s(τ)/(M d)` from its start | 1e-12 calibres |
| `a_part_moving_off_the_axis_keeps_both_momenta_in_free_space` | no air or gravity, the rocket turning about all three axes, the ballast 1 cm off the axis: the angular momentum about the centre and the centre's velocity stay constant | 6.9e-12 of the angular momentum, 2.6e-12 m/s |
| `a_shift_s_rates_are_the_derivatives_of_its_mass_properties` | the exact rates against differences of the mass properties, during the burn and after it | `r′` 2e-11 m/s, `r″` 3e-8 of itself |
| `a_shift_starts_at_apogee_or_at_its_height_on_the_way_down` | the watched triggers start the move at the apogee, and at 200 m on the way down | 1e-6 m |

The free-flight test is the one that checks the new term. Without `ω × h + h′`, its error would be
the size of the ballast's own angular momentum relative to the airframe, which peaks at 1.8% of
the whole. It measures 6.9e-12.

A first version took the part's rates from differences 0.1 ms apart, as the motor's are. In free
flight it kept the centre's velocity only to 1e-8 m/s, the error of the difference itself, so the
rates were made exact.

The refusals each have a test that checks which rule fired.

## What it leaves out

- **Releasing a mass.** Ballast or a payload that leaves the rocket while the rest flies on is
  [M1.12b](../decisions-and-roadmap.md#m1-12b).
- **Shifts with a separation or ejections.** Their pieces are fixed before the flight, with every
  part where the design puts it.
- **Other motions.** A part can only slide along the axis, at whatever distance from the axis the
  design puts it. It can't move across the axis or turn.
- **Stops.** The cycloid brings the part to rest smoothly. A real mechanism may stop it with a
  shock, which a rigid airframe can't model.

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
