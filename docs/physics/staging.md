# Staging

## In short

- **What it models:** motors that light at their own times, and a rocket that drops its booster
  under power. A motor can light at launch, at a time, a delay after another motor burns out, or
  a delay after its stage is freed. When the stack comes apart
  ([separation](../glossary.md#separation)) with the forward part still to burn, that part (the
  [sustainer](../glossary.md#sustainer)) flies on with its own shape and mass, and the aft part
  (the [booster](../glossary.md#booster)) falls back to its own landing.
- **Sources:** no new physics. The flight is the same rigid-body flight
  ([Rigid-body flight](flight.md)), the masses are the same sums ([The design
  tree](design.md#motors-and-configurations)), and the booster's descent is the same point mass
  ([Recovery](recovery.md#separation)). What is new is when each motor burns, and which rocket is
  flying.
- **How well it is validated:** **not yet against another simulator or a flight.** Tests pin the
  bookkeeping: ignition times to 1e-12 s, the mass step at the split to 1e-12 of the mass, the
  bodies' momenta to 1e-9, and the order of the events. The comparison with OpenRocket's staged
  flights is [M1.9c](../decisions-and-roadmap.md#m1-9c), the milestone that reads a `.ork` file's
  staging settings.
- **What it leaves out:** any push from the separation (no charge or spring), the air flowing
  between the parts as they come apart, and the booster's orientation as it falls. Motor clusters
  in one mount wait for [M1.9b](../decisions-and-roadmap.md#m1-9b). The decision record is
  [ADR-074][adr-074].

## When a motor lights

Each motor in a configuration has an `ignition` ([`Ignition`][ignition]). The flight's clock
starts at launch (`t = 0`), and each motor burns on its own clock from its ignition, so its thrust
curve and its mass are its own curve shifted to that time.

| `ignition` | lights at | example in a design file |
|---|---|---|
| `launch` (the default) | `t = 0` | nothing written |
| `time` | a time after launch | `"ignition": {"time": {"time_s": 4.0}}` |
| `burnout` | a delay after the motor in another mount burns out | `"ignition": {"burnout": {"mount": "booster-motor-mount", "delay_s": 1.0}}` |
| `separation` | a delay after the stage aft of the motor's stage comes away | `"ignition": {"separation": {"delay_s": 0.5}}` |

Before a motor lights it is **loaded**: its full propellant mass sits in the rocket, and it gives
no thrust. A motor that never lights (its separation never comes) is carried loaded to the ground.
Every ignition time that is known before the flight, and every knot of each shifted thrust curve,
is a stop time for the integrator, so no step starts a burn half way through
([Time integration](integration.md)). The design refuses a burnout of a mount with no motor, a
chain of burnouts that comes back to the motor itself, and a time or delay that is negative.

## Powered separation

A separation has a trigger and a stage boundary ([Recovery](recovery.md#separation)). It can now
also fire a delay after a motor's burnout (`Trigger::Burnout`), as well as at apogee, at a height
on the way down, at a time, or at a motor's ejection delay.

When it fires, hpr looks at the forward part, body 0, which keeps the nose:

- **If it still has a motor to burn**, it is a sustainer. That covers a motor burning now, one due
  to light at a known time, and one lit by this separation. The sustainer flies on in six degrees
  of freedom, and the booster (body 1) descends as a point mass under its own device.
- **If it has nothing to burn**, both parts descend as point masses, as before.

The booster's motors must have burned out: hpr refuses a separation while one burns, before the
flight if the time is known and in flight otherwise.

**Why the state carries straight across.** hpr's flight state is the motion of the nose tip, not
of the centre of mass ([Rigid-body flight](flight.md)). The sustainer keeps the nose, so the nose
tip's position, velocity, attitude and rotation rate are all still right for it. What changes is
the rocket they belong to:

- **Mass.** The mass drops by exactly the booster's: its stages plus its spent motors. The centre
  of mass and the inertia are the sustainer's own stages and motors.
- **Aerodynamics.** hpr builds the sustainer's aerodynamics from the design cut after the
  boundary. It is an ordinary rocket with a nose, and its own aft end, so its drag includes its own
  base.

The integrator restarts at the separation, because the mass steps there.

**Momentum.** Nothing pushes the parts apart. The sustainer keeps the nose tip's velocity and
rotation, and the booster leaves with the velocity its own centre of mass already had,
`v_O + ω × r_cg`. So the two momenta add up to the stack's, `m v_cg = m_s v_s + m_b v_b`. A test
checks this to 1e-9 of the stack's momentum on a flight whose sustainer is not yet lit. With the
sustainer burning, `m v_cg` is not simply additive: the centre of mass also moves inside the body
as propellant burns. So hpr makes no claim for that case.

**Events.** Along with the separation, the flight records an `Ignition` event for each motor lit
after launch. `Burnout` is recorded when every motor lit, or due at a known time, has burned out.
With a sustainer lit by its separation, that happens twice: once at the booster's burnout and once
at the sustainer's.

**What the result holds.** The flight's events and final sample are the sustainer's, from the pad
to its landing. `FlightResult::bodies` holds the booster's descent, and `landings()` gives both
landings, the sustainer's first.

## A worked example

The program
[`crates/hpr-sim/examples/two_stage.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/two_stage.rs)
flies a synthetic two-stage design: a 54 mm sustainer on a 75 mm booster, with a J760 in the
booster and an I175 in the sustainer. The stages come apart 0.5 s after the booster's burnout, and
the sustainer lights 1 s after it:

```rust
sustainer_motor.ignition = Ignition::Burnout {
    mount: "booster-motor-mount".to_owned(),
    delay_s: 1.0,
};
// ...
.with_separation(Separation::new(
    Trigger::Burnout { motor: booster, delay_s: 0.5 },
    0, // the boundary after stage 0, the sustainer
))?;
```

Run it with `cargo run --example two_stage -p hpr-sim`. It prints:

<!-- quote: crates/hpr-sim/examples/two_stage.output.txt -->
```text
A 54 mm sustainer on a 75 mm booster, J760 then I175, calm air
Not yet validated: see the Accuracy page before trusting these numbers.

event                   time (s)   CG height (m)   speed (m/s)   mass (kg)
liftoff                     0.00             0.5           0.0       2.480
rail exit                   0.20             6.3          61.0       2.410
separation                  2.23           643.0         341.2       1.904
sustainer lights            2.73           797.4         280.7       0.779
sustainer burnout           5.23          1798.9         403.2       0.550
apogee                     18.42          3103.2           0.1       0.550
parachute charge           18.42          3103.2           0.1       0.550
parachute opens            18.42          3103.2           0.1       0.550
sustainer lands           865.34             0.0           3.4       0.550

The booster (1.125 kg) leaves at 2.23 s and 642.7 m, and lands at 350.18 s at 17.9 m/s.
```

How to read it:

- **The split.** At 2.23 s the stack weighs 1.904 kg. The booster, its stage and a spent J760,
  takes 1.125 kg of that, so the sustainer flies on at 1.904 − 1.125 = 0.779 kg, which is the mass
  shown when it lights.
- **The coast.** Between the separation and the ignition the sustainer coasts for half a second
  and slows from 341.2 to 280.7 m/s. That is about 121 m/s², mostly drag, on a 0.78 kg rocket near
  Mach 1.
- **The burn.** The I175 burns for 2.5 s and the mass falls to 0.550 kg: the sustainer's
  structure and a spent motor.
- **Two landings.** The sustainer lands under its parachute at 3.4 m/s. The booster tumbles down on
  its own and lands at 17.9 m/s.

These numbers are not validated: see the note at the top of this page.

## Tests

In `crates/hpr-sim/src/staging.rs` and `crates/hpr-design/src/config.rs`:

- `serial_plan_timing_and_mass_step` ([Loft lesson L93](../decisions-and-roadmap.md#l93), a check
  carried over from hpr's predecessor): the sustainer lights at the booster's burnout plus its
  delay, to 1e-12 s, and the mass steps down by exactly the booster's, to 1e-12 of the mass, and
  holds until the sustainer lights. It also checks a sustainer whose separation never comes: it
  never lights, and it lands loaded.
- `apogee_separation_fires_in_flight_and_booster_flies_to_landing`
  ([Loft lesson L30](../decisions-and-roadmap.md#l30)): an apogee separation fires at the
  flight's own apogee, and a height separation where the flight crosses the height. Both bodies
  land, powered or not.
- `staged_events_come_in_order`: liftoff, rail exit, separation, ignition, burnout, apogee and
  landing, in that order, with the burnout recorded once at the sustainer's, or twice with a
  sustainer lit by its separation.
- `a_powered_separation_conserves_linear_momentum`: the two bodies' momenta add to the stack's,
  to 1e-9 of it.
- `staging_refuses_what_it_cannot_fly`: a drag table past a powered separation, a separation
  timed while the booster burns, and a delay that is negative or not a number.
- `ignition_times_follow_their_events`, `bad_ignitions_are_refused`.

[adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25
[ignition]: ../api/hpr_design/config/enum.Ignition.html
