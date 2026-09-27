# Released mass

## In short

- **What it models:** a part carried inside the airframe, such as ballast or a payload, let go
  during the flight on a trigger. The rest of the rocket flies on in
  [six degrees of freedom](../glossary.md#6-dof-six-degrees-of-freedom) with its mass,
  [centre of mass](../glossary.md#centre-of-gravity-cg) and inertia stepped to the rest's, and the
  part falls to the ground on its own. Use it to see what dropping ballast or a payload does to
  the flight, and where the part comes down. This is new with
  [M1.12b](../decisions-and-roadmap.md#m1-12b) (a mass released in flight).
- **Sources:** the [parallel-axis theorem](../glossary.md#parallel-axis-theorem) and the kinetics
  of a system of particles, from Meriam and Kraige's *Engineering Mechanics: Dynamics*. The part's
  fall uses the equations a separated body already has ([Recovery](recovery.md#the-descent)).
- **How well it is validated:** against exact answers only. No other simulator or real flight has
  been compared with it. The mass properties after a release match a hand calculation to 1e-15.
  In free flight with nothing acting on the rocket, the rest and the part together keep the
  rocket's momentum to 1.5e-13 of itself and its angular momentum to 3.9e-13.
- **What it leaves out:** a push that throws the part out; a release in a flight that also
  separates, ejects pieces or moves a mass; a release before the rocket leaves the rail;
  parachutes on the part (its [drag area](../glossary.md#drag-area) is one number you give); and
  the part's own spin once it is out.

## Describing a release

A `MassRelease` names the part, when it leaves, and its drag area once it is out:

| field | meaning |
|---|---|
| `trigger` | when it leaves: the same triggers a parachute has (apogee, a height on the way down, a time after launch, a motor's burnout or delay) |
| `component` | the `id` of the internal component that leaves; everything inside it leaves too |
| `drag_area_m2` | the part's drag area `C_D S` once it is out, in m²: positive |

Give the list to a flight with `Simulation::with_releases`. The part must be carried inside the
airframe, such as a mass component or an inner tube, and be one part. These are refused, each
with an error that names the part and the rule:

- a body component, or a part outside the airframe (fins, rail buttons);
- one of several copies in a cluster of tubes;
- a part that holds a motor: the rocket's motors stay with it;
- a part whose mass is under an override, its stage's or a holder's that covers what it holds,
  because the override doesn't say how much of the mass is the part's;
- a part released twice, or inside another part that is released;
- a drag area that is zero, negative or not finite. A part with no drag would fall as if in a
  vacuum, which is a wrong number rather than a model.

A release that would come on the pad or the rail is refused when the flight reaches it: the part
has nowhere to go. A flight that separates, ejects pieces or moves a mass can't have a release
yet, because their pieces and parts are fixed before the flight.

## What happens at the release

At the instant the part leaves, three things happen.

**The rocket becomes the rest.** With `M` the rocket's mass and `cg` its centre, `m` the part's
mass, `c` its centre and `I_p` its own inertia about `c`, the rest has

```text
M'  = M − m
cg' = (M cg − m c) / M'
I'  = I_about_cg' − (I_p + m (|c − cg'|² E − (c − cg')(c − cg')ᵀ))
```

where `I_about_cg'` is the rocket's inertia moved to the new centre by the parallel-axis theorem,
`E` is the identity matrix, and the bracket is the part's inertia about that centre. It is the
sum that builds a rocket from its parts, run backwards.

**The rest flies on from the same state.** The flight's state is the nose tip's position and
velocity, the attitude and the turning rates ([Rigid-body flight](flight.md)). The rest keeps the
nose tip, so none of these changes. Only the mass properties step, and the integrator starts
afresh at the same instant, as it does when a sustainer flies on after a separation.

**The part leaves with the velocity it had.** Every point of a rigid body moves at `v_O + ω × r`,
with `v_O` the nose tip's velocity, `ω` the rocket's turning rate and `r` the point's place in
the rocket. The part leaves from where it was, with its centre's velocity `v_O + ω × c`. So the
rest's momentum plus the part's is the rocket's just before, and the same holds for the angular
momentum: nothing is created or lost at the release. Nothing pushes the two apart, either.

## The part's fall

Once out, the part is a point mass under its drag area, weight and the Earth's rotation, the
equations a separated body falls by:

```text
m a = −½ ρ (C_D S) |v − w| (v − w) + m (g + a_Coriolis)
```

with `ρ` the air's density, `w` the wind and `g` gravity. It falls from its release to the ground
or to the flight's time cap. If it left climbing, its own apogee is recorded. Its flight is in
the result's `released` list, beside the rocket's flight, not among the separated bodies'
landings.

The drag area is yours to give. hpr has no model of a small part's drag as it tumbles, so give the
area of its side times a drag coefficient near 1 for a tumbling weight, or a parachute's drag area
if the part carries one (taken as open from the instant it leaves).

## A worked example

The example
[`released_ballast.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/released_ballast.rs)
flies the project's 54 mm test design (`validation/designs/synthetic-54mm-three-fin.json`) on an
I175 motor ([motor designation](../glossary.md#motor-designation)), with 200 g of ballast in its
airframe: a cylinder 50 mm long and 30 mm across, on the axis, its centre 0.375 m aft of the nose
tip. A drogue with a drag area of 0.3 m² opens at apogee. At 5 s, well after the 2.5 s burn, the
ballast is let go and tumbles down under an assumed 0.002 m² (about its side, 50 mm by 30 mm, at a
drag coefficient near 1.3). What the example prints:

| time (s) | mass (kg) | centre of mass (m aft of the nose tip) | transverse inertia (kg·m²) |
|---:|---:|---:|---:|
| 4.00 | 0.8188 | 0.6681 | 0.09556 |
| 6.00 | 0.6188 | 0.7628 | 0.07277 |

By hand, the rest's centre is at `(0.8188 × 0.6681 − 0.2 × 0.375) / 0.6188 = 0.7628` m aft of the
tip. It moves aft because the ballast sat forward of the centre. (The transverse inertia is about
an axis across the rocket through its centre of mass, the one it pitches about.)

The upward momentum at 5 s is 131.5839 kg·m/s before the release. After it, the rest carries
99.4433 and the ballast 32.1406, which add to the same 131.5839.

| | apogee (m) | lands at (s) | landing speed (m/s) |
|---|---:|---:|---:|
| the rocket, ballast dropped | 1670.0 | 276.3 | 6.15 |
| the ballast | 1309.4 | 43.7 | 43.01 |
| the rocket, ballast kept | 1749.9 | 253.5 | 7.07 |

The rocket without its ballast climbs 80 m less: drag slows a lighter rocket more. It comes down
more slowly under the same drogue, and so lands later. The ballast, coasting up on its own,
peaks at 1309.4 m and comes down at 43 m/s, its terminal speed under so small a drag area.

## How it is checked

The tests are in `hpr_sim::releases` and `hpr_design::mass`. The last column is what each test
measured, where it records it, and in brackets the bound it holds the code to.

| test | what it shows | measured (bound) |
|---|---|---|
| `a_part_taken_out_of_a_body_leaves_the_hand_computed_rest` | a box taken out of a cube leaves the cube's mass, centre and inertia, and putting it back gives the whole | (1e-15) |
| `mass_properties_after_a_release_match_the_hand_calculation` | the example's rocket, with the ballast 1 cm off the axis, before and after the release, against the two-body formula; every step flies the rest's mass and centre; the part leaves from its place with `v_O + ω × c` | (1e-15 m, kg and kg·m²; 1e-12 m and m/s) |
| `a_release_conserves_mass_and_momentum_in_free_space` | no air or gravity, the rocket turning about all three axes: the rest and the part keep the rocket's mass, momentum and angular momentum about a fixed point | momentum 1.5e-13 (1e-12), angular momentum 3.9e-13 (1e-10) |
| `a_payload_let_go_under_the_drogue_lands_slower_and_falls_at_its_own_speed` | in uniform air, under a drogue, a release at 150 m on the way down: the rest lands at the lighter rocket's terminal speed, the part at its own | (1e-6 m/s) |
| `a_release_comes_at_apogee_and_a_part_let_go_climbing_has_its_own` | a release at apogee comes at the rocket's apogee; a part let go climbing records its own apogee, then its landing | (1e-6 m/s at its apogee) |

In the free-flight test the ballast leaves 0.146 m/s faster than the rocket's centre, across it,
because the rocket turns. Leaving out `ω × c` would put the momentum off by about 3e-3 of itself;
it measures 1.5e-13. The part's own spin, which a point mass drops, is 2.7e-8 of the angular
momentum there.

Every refusal has a test that checks which rule fired, in `releases_that_cannot_be_made_are_refused`,
`triggers_a_release_cannot_have_are_refused_in_its_own_words` and
`a_release_is_refused_with_partings_or_shifts_in_either_order`.

## What it leaves out

- **A push.** Nothing throws the part out: a spring or a charge would add to its velocity and
  take from the rocket's, as an ejection's impulse does
  ([Recovery: ejected pieces](recovery.md#ejected-pieces)).
- **Releases with a separation, ejections or mass shifts**, as above.
- **Releases on the pad or the rail**, as above.
- **Parachutes on the part.** It falls under one drag area from the instant it leaves; it has no
  devices of its own, and no opening time.
- **The part's spin.** As a point mass it keeps its path but not its turning, a tiny share of the
  angular momentum for a small part.
- **Warnings.** hpr doesn't warn when a release moves the centre of mass aft of a safe margin; read
  the margin from the flight's metrics ([Flight metrics](metrics.md#stability-margins)). The
  aerodynamics don't change: the part was inside the airframe.

The decision record is [ADR-088][adr-088]. A part that moves along the airframe without leaving
it is [Moving mass](moving-mass.md).

## References

- **[MK]** J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*: chapter 4 (kinetics
  of systems of particles: the momentum and angular momentum of a system summed over its
  particles, and their conservation with no external force) and appendix B (the parallel-axis
  theorem and the inertia tensor).
- The equations of motion the rest flies on: the RocketPy technical documentation, "Equations of
  Motion" v0 and v1 ([Rigid-body flight](flight.md)).

[adr-088]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-088-mass-released-in-flight-2026-09-26
