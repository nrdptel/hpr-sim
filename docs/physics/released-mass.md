# Released mass

## In short

- **What it models:** a part carried inside the airframe, such as ballast or a payload, let go
  during the flight on a trigger. The rest of the rocket flies on in
  [six degrees of freedom](../glossary.md#6-dof-six-degrees-of-freedom), its mass,
  [centre of mass](../glossary.md#centre-of-gravity-cg) and inertia changing at once to those of
  what remains. The part falls to the ground on its own. Use it to see what dropping ballast or a
  payload does to the flight, and where the part comes down. This is new with
  [M1.12b](../decisions-and-roadmap.md#m1-12b) (a mass released in flight).
- **Sources:** the [parallel-axis theorem](../glossary.md#parallel-axis-theorem) and the kinetics
  of a system of particles, from Meriam and Kraige's *Engineering Mechanics: Dynamics*. The part's
  fall uses the equations a separated body already has ([Recovery](recovery.md#the-descent)).
- **How well it is validated:** against exact answers only. No other simulator or real flight has
  been compared with it. The mass properties after a release match a hand calculation, and the
  design built without the part, within the tests' bound of 1e-15 (kg, m, kg·m²). In free flight
  with nothing acting on the rocket, the rest and the part together keep the rocket's momentum to
  a relative error of 1.5e-13, and its angular momentum to 7.3e-12.
- **What it leaves out:** a push that throws the part out; a release in a flight that also
  separates, ejects pieces or moves a mass; a release before the rocket leaves the rail;
  parachutes on the part; and the part's own spin once it is out. Where and how fast the part
  lands depends on its [drag area](../glossary.md#drag-area), which you give. hpr doesn't warn
  when a release leaves the rocket unstable: dropping a part forward of the centre of mass moves
  the centre aft, and the [stability margin](../glossary.md#stability-margin) falls by 1.683
  [calibres](../glossary.md#calibre-caliber) in the example below. Read it from the flight's
  metrics ([Flight metrics](metrics.md#stability-margins)).

## Describing a release

A `MassRelease` names the part, when it leaves, and its drag area once it is out:

| field | meaning |
|---|---|
| `trigger` | when it leaves: the same triggers a parachute has ([Recovery: triggers](recovery.md#triggers-lag-and-release)): apogee, a height on the way down, a time after launch, a motor's [burnout](../glossary.md#burnout) or its [ejection delay](../glossary.md#ejection-delay) |
| `component` | the `id` of the component in the design that leaves; everything inside it leaves too |
| `drag_area_m2` | the part's drag area `C_D S` once it is out, in m²: positive |

Give the list to a flight with `Simulation::with_releases`. The part must be one component carried
inside the airframe, such as a mass component or an inner tube
([Rocket design: the tree](design.md#the-tree)). These are refused, each with an error that names
the part and the rule:

- an `id` that names no component;
- a body component (a nose cone, a tube, a transition), or a part outside the airframe (fins, rail
  buttons);
- one of several copies in a [cluster](../glossary.md#cluster) of tubes;
- a part that holds a motor: the rocket's motors stay with it;
- a part whose mass is set by an [override](../glossary.md#override), on its stage or on a
  component around it that includes it, because the override doesn't say how much of the mass is
  the part's;
- a part released twice, or inside another part that is released;
- a part with no mass, or releases that would leave the airframe, its motors aside, with none;
- a drag area that is zero, negative or not finite. A part with no drag would fall as if in a
  vacuum, which is a wrong number rather than a model;
- a trigger a parachute couldn't have either: a time before launch, a height that isn't positive,
  a motor that isn't there, has no ejection delay, or is set to fail to light.

A release that would come on the pad or the rail makes `Simulation::run` return an error, with no
flight: the part has nowhere to go. A flight can't combine a release with a separation, ejected
pieces or a [moving mass](moving-mass.md) yet.

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
sum that builds a rocket from its parts, run backwards. The aerodynamics don't change: the part
was inside the airframe.

**The rest flies on from the same state.** The flight's state is the nose tip's position and
velocity, the attitude and the turning rates ([Rigid-body flight](flight.md#state)). The rest
keeps the nose tip, so none of these changes. The mass properties step, and the integrator (the
numerical method that steps the flight forward in time) starts afresh at the same instant, as it
does when a [sustainer](../glossary.md#sustainer) flies on after a separation
([Staging](staging.md#powered-separation)). What the flight reports about the centre of mass
steps too: its position moves by `cg' − cg` (9.5 cm aft in the example), and its height with it.

**The part leaves with the velocity it had.** Every point of a rigid body moves at `v_O + ω × r`,
with `v_O` the nose tip's velocity, `ω` the rocket's turning rate and `r` the point's place in
the rocket. The part leaves from where it was, with its centre's velocity `v_O + ω × c`, and the
rest's centre goes on at `v_O + ω × cg'`. Nothing pushes the two apart, so every bit of the rocket
keeps its velocity: once the motor has burned out, the rest's momentum plus the part's is the
rocket's just before, and the same holds for the angular momentum.

While the motor burns, the centre of mass drifts along the airframe as propellant is used, and the
reported centre-of-mass velocity includes that drift. A release steps the centre by `cg' − cg`, so
the drift's share of the velocity steps too. The reported momenta then differ by
`Ṁ (cg' − cg)`, with `Ṁ` the propellant's mass flow, turned into the
[launch frame](../glossary.md#launch-frame-enu). No part of the rocket changes its velocity; only
the reported centre does.

**The flight has one apogee.** The rest's centre is not where the rocket's was, so on a flight
that turns, it can rise for a moment after the rocket's apogee, or already be falling when a part
leaves just before it. The flight records one [apogee](../glossary.md#apogee): the rocket's, or
the release itself when it leaves the rest already falling. A part let go at apogee leaves at
that one, even when another part leaving first sets the rest rising again, and whatever order
the parts are listed in. A flight started part way (`Simulation::run_free`) already falling lets
a part waiting for the apogee go at its start; it records an apogee only if the rest later rises
and falls again.

**The rest can land at the release.** A part let go just above the ground, forward of the centre
of mass of a rocket falling nose up, steps the rest's centre down, to the ground or below it. The
rocket has then landed, at the release. A recorded trajectory's last row is at that time, but of
the rocket as it was before the part left. A release that steps the rest below the ground while it is still climbing is an
error instead.

## The part's fall

Once out, the part is a point mass under its drag area, its weight and the Earth's rotation, the
equations a separated body falls by:

```text
m a = −½ ρ (C_D S) |v − w| (v − w) + m (g + a_Coriolis)
```

with `v` and `a` the part's velocity and acceleration, `ρ` the air's density, `w` the wind, `g`
gravity and `a_Coriolis` the [Coriolis acceleration](../glossary.md#coriolis-acceleration). It
falls from its release to the ground or to the flight's time cap (`FlightSettings::max_time_s`).
If it left climbing, its own apogee is recorded.

The drag area is yours to give. For a part tumbling at random, the tumble model's body term gives
`0.56` times its side profile, its diameter times its length
([Recovery: tumble](recovery.md#tumble)). That is half the `1.12` of a cylinder broadside. It was
fitted to whole rockets 44 to 103 mm across falling at 5 to 6.6 m/s, and its one drop test without
fins wanted 0.79, which gives a speed 16% lower. So treat a tumbling part's landing speed as
uncertain by at least that much. hpr doesn't build the area for you, because its tumble model
needs body tubes and fins, and a released part has neither. If the part carries a parachute, give
the parachute's drag area; it is taken as open from the instant the part leaves.

## Reading a release back

- `EventKind::MassRelease(i)` among the flight's events marks release `i` (its place in the list
  given to `with_releases`); its sample is the rocket just before the part left.
- `FlightResult::released` holds each part's flight (`ReleasedFlight`): its start, its events, its
  landing. The separated bodies of a separation or an ejection are elsewhere, in
  `FlightResult::bodies`.
- `Simulation::mass_properties(&flight, t)` gives the rocket's mass properties at time `t` as the
  flight flew it, without every part released at or before `t`.

## A worked example

The example
[`released_ballast.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/released_ballast.rs)
flies the project's 54 mm test design
([`synthetic-54mm-three-fin.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/designs/synthetic-54mm-three-fin.json),
a body 56.3 mm across) on an I175 motor
([motor designation](../glossary.md#motor-designation)), with 200 g of ballast in its airframe: a
cylinder 50 mm long and 30 mm across, on the axis, its centre 0.375 m aft of the nose tip. It flies
in calm standard air from a site in New Mexico 1,400 m up, and a drogue with a drag area of 0.3 m²
opens at apogee. At 5 s, well after the 2.5 s burn, the ballast is let
go to tumble down under `0.56 × 0.05 × 0.03 = 0.00084` m², the tumble model's body term.

In the table the transverse inertia is about an axis across the rocket through its centre of
mass, the one it pitches about. What the example prints:

| time (s) | mass (kg) | centre of mass (m aft of the nose tip) | transverse inertia (kg·m²) |
|---:|---:|---:|---:|
| 4.00 | 0.8188 | 0.6681 | 0.09556 |
| 6.00 | 0.6188 | 0.7628 | 0.07277 |

By hand, the rest's centre is at `(0.8188 × 0.6681 − 0.2 × 0.375) / 0.6188 = 0.7628` m aft of the
tip. It moves 0.09474 m aft, because the ballast sat forward of the centre, and the static margin
falls by `0.09474 / 0.05630 = 1.683` calibres (the body's diameter is 0.05630 m), from 4.297 to
2.615: still stable.

At the release the rocket carries 131.5839 kg·m/s of upward momentum. Every point of the airframe
moves at `v_O + ω × r`, so it divides between the rest, 99.4433, and the ballast, 32.1406; the
free-flight test below checks that the flight keeps it so.

| | apogee (m) | lands at (s after launch) | landing speed (m/s) |
|---|---:|---:|---:|
| the rocket, ballast dropped | 1670.0 | 276.3 | 6.15 |
| the ballast | 1496.5 | 40.0 | 66.74 |
| the rocket, ballast kept | 1749.9 | 253.5 | 7.07 |

- **The rocket climbs 80 m less** without its ballast: the same drag slows a lighter rocket more.
- **It comes down more slowly** under the same drogue, and so lands later. A terminal speed goes
  as the square root of the mass: `7.07 × √(0.6188 / 0.8188) = 6.15` m/s.
- **The ballast peaks lower than the rocket**, having more drag for its mass. Its terminal speed
  at the ground is `√(2 × 0.2 × 9.79 / (1.069 × 0.00084)) = 66.0` m/s, with the standard air's
  density 1.069 kg/m³ at the site, 1,400 m up ([Atmosphere](atmosphere.md)), and gravity
  9.79 m/s² there ([Gravity](gravity.md)). It lands a little faster, 66.74 m/s, because it is
  still slowing as the air thickens.

## How it is checked

The tests are in `hpr_sim::releases` and `hpr_design::mass`. The last column gives the value each
test measured, where its comments record one, and in brackets the bound it holds the code to.

| test | what it shows | measured (bound) |
|---|---|---|
| `a_part_taken_out_of_a_body_leaves_the_hand_computed_rest` | a box taken out of a cube leaves the cube's mass, centre and inertia, and putting it back gives the whole | (1e-15) |
| `mass_properties_after_a_release_match_the_hand_calculation` | the example's rocket, with the ballast 1 cm off the axis, before and after the release, against the two-body formula and the design built without the ballast; every step flies the rest's mass and centre; the part leaves from its place with `v_O + ω × c` | (1e-15 m, kg and kg·m²; 1e-12 m and m/s) |
| `a_release_conserves_mass_and_momentum_in_free_space` | no air or gravity, the motor spent, the rocket turning about all three axes: the rest and the part keep the rocket's mass, momentum, and angular momentum about their common centre of mass | momentum 1.5e-13 (1e-12), angular momentum 7.3e-12 (1e-10) |
| `two_releases_leave_the_design_without_both_parts` | two parts released at different times: between them only the first is gone, after both the rocket is the design built without either | (1e-15) |
| `a_payload_let_go_under_the_drogue_lands_slower_and_falls_at_its_own_speed` | in uniform air, under a drogue, a release at 150 m on the way down: the rest lands at the lighter rocket's terminal speed, the part at its own | (1e-6 m/s) |
| `a_release_comes_at_apogee_and_a_part_let_go_climbing_has_its_own` | a release at apogee comes at the rocket's apogee; a part let go climbing records its own apogee, then its landing | (1e-6 m/s at its apogee) |
| `a_release_at_apogee_on_a_tilted_rail_leaves_one_apogee` | off a rail 5° from vertical, in wind, with and without a drogue, the flight records one apogee | exactly one |
| `a_part_let_go_just_before_apogee_can_make_the_apogee_there` | a release that leaves the rest already falling makes the apogee, and fires the drogue, at the release; a part waiting for the apogee, listed before or after, leaves there too | exactly one, at the release |
| `parts_waiting_for_the_apogee_all_leave_at_it` | two parts let go at apogee off the tilted rail, listed either way round, both leave at the flight's one apogee | exactly one |
| `a_release_that_puts_the_rest_on_the_ground_lands_it` | under a drogue, a release 5 cm above the ground steps the rest's centre 9.5 cm down, below it: the rocket lands at the release; climbing, the release is refused | 1e-8 m, 1e-15 kg |
| `the_optimum_delay_holds_a_release_on_the_motor_s_charge` | a release or a mass shift fired by the motor's ejection charge is held with the charge when hpr works out the [optimum delay](metrics.md#optimum-ejection-delay) (the delay that fires the charge at apogee), so the answer doesn't depend on the delay flown | equal |
| `a_release_and_its_flight_read_back_as_written` | a release, and a flight with a released part, write to JSON and read back unchanged | equal |
| `a_part_let_go_at_the_ground_has_landed` | a part let go as the rocket hits the ground, already at or below it, has landed | — |

In the free-flight test the ballast leaves 0.146 m/s away from the rocket centre's velocity,
because the rocket turns. Had it left at the nose tip's velocity, the momentum would be off by
5.0e-3 of itself; at the rocket centre's, by 3.4e-3. The flight keeps it to 1.5e-13. The part's
own spin, which a point mass drops, is 1.5e-3 of the rocket's angular momentum there.

Every refusal has a test that checks which rule fired, in `releases_that_cannot_be_made_are_refused`,
`parts_with_no_mass_are_refused`, `triggers_a_release_cannot_have_are_refused_in_its_own_words` and
`a_release_is_refused_with_partings_or_shifts_in_either_order`.

## What it leaves out

- **A push.** Nothing throws the part out: a spring or a charge would add to its velocity and
  take from the rocket's, as an ejection's impulse does
  ([Recovery: ejected pieces](recovery.md#ejected-pieces)).
- **Releases with a separation, ejections or mass shifts.** hpr can't combine these yet.
- **Releases on the pad or the rail**, as above.
- **Parachutes on the part.** It falls under one drag area from the instant it leaves; it has no
  devices of its own, and no opening time.
- **The part's spin.** Its spin is dropped: 1.5e-3 of the rocket's angular momentum in the
  free-flight test.
- **Warnings.** hpr doesn't warn when a release leaves the rocket unstable, as above.

The decision record, [ADR-088][adr-088], sets out these choices: the rest flying on from the same
state, the part leaving at its own velocity and falling under a drag area you give, and what is
refused. A part that moves along the airframe without leaving it is
[Moving mass](moving-mass.md).

## References

- **[MK]** J. L. Meriam and L. G. Kraige, *Engineering Mechanics: Dynamics*: chapter 4 (kinetics
  of systems of particles: the momentum and angular momentum of a system summed over its
  particles, and their conservation with no external force) and appendix B (the parallel-axis
  theorem and the inertia tensor).
- The equations of motion the rest flies on: the RocketPy technical documentation, "Equations of
  Motion" v0 and v1 ([Rigid-body flight](flight.md)).
- The tumble model's body term: the OpenRocket technical documentation, §3.5
  ([Recovery: tumble](recovery.md#tumble)).

[adr-088]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-088-mass-released-in-flight-2026-09-26
