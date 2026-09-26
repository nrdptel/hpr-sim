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
  tree](design.md#motors-and-configurations)), and the booster's descent is the same descent
  under a recovery device ([Recovery](recovery.md#separation)). What is new is when each motor
  burns, and which rocket is flying.
- **How well it is validated:** **not yet against another simulator or a flight.** Tests pin the
  bookkeeping: ignition times to 1e-12 s, the mass step at the split to 1e-12 of the mass, and,
  while the sustainer is not yet burning, the two parts' momenta to 1e-9 of the stack's. The
  comparison with OpenRocket's staged flights is [M1.9c](../decisions-and-roadmap.md#m1-9c), the
  milestone that also reads a `.ork` file's staging settings.
- **What it leaves out:**
  - **The booster's own airframe drag.** After the split the booster is a point: a mass with only
    its recovery device's drag. So hpr requires a device on it that opens at the separation,
    usually [tumbling](../glossary.md#tumble-recovery), and refuses the flight otherwise. hpr
    tumbles the booster side-on from the instant it separates, while a real finned booster flies
    nose-first for a while first, so it slows far faster than a real one would. Treat the
    booster's peak and landing point as rough, likely too low and too close to the pad ([#179](https://github.com/nrdptel/hpr-sim/issues/179), a model of the booster's own
    drag).
  - Any push from the separation (no charge or spring), the air flowing between the parts as they
    come apart, and the booster's orientation as it falls.
  - A drag or normal-force table from another program (`Simulation::with_drag_table`): it
    describes the whole stack, so a powered separation under one is refused.
  - Several motors in one mount wait for [M1.9b](../decisions-and-roadmap.md#m1-9b).

## Using it today

Staging is set in a JSON design file or in code, not read from a `.ork` file yet. Start from the
two-stage design
[`synthetic-two-stage-75mm-54mm.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/designs/synthetic-two-stage-75mm-54mm.json)
and the [worked example](#a-worked-example) below. The decision record is [ADR-074][adr-074].

## When a motor lights

Each motor in a [configuration](../glossary.md#configuration) has an `ignition`
([`Ignition`][ignition]). The flight's clock starts at launch (`t = 0`). Each motor burns on its own
clock from its ignition, so its thrust curve and its mass are its own curve shifted to that time.

| `ignition` | lights at | example in a design file |
|---|---|---|
| `launch` (the default) | `t = 0` | nothing written |
| `time` | a time after launch | `"ignition": {"time": {"time_s": 4.0}}` |
| `burnout` | a delay after the motor in another mount burns out | `"ignition": {"burnout": {"mount": "booster-motor-mount", "delay_s": 1.0}}` |
| `separation` | a delay after the separation that drops the stages behind this motor's stage | `"ignition": {"separation": {"delay_s": 0.5}}` |

**A design that says nothing lights every motor on the pad**, the sustainer's too, and hpr does not
warn. For a staged flight, give the sustainer's motor its `ignition` and the flight a
`Separation`.

Before a motor lights it is **loaded**: its full propellant mass sits in the rocket, and it gives
no thrust. A motor that never lights, because its separation never comes, is carried loaded to the
ground. Every ignition time known before the flight, and every point of each shifted thrust curve,
is a [stop time](../glossary.md#stop-time): the integrator ends a step there, so no step starts a
burn half way through ([Time integration](integration.md)). An ignition that waits on a
separation becomes a stop time when the separation fires. The design refuses:

- a burnout of a mount with no motor, or a chain of burnouts that comes back to the motor itself;
- a separation ignition in the aft-most stage, which has nothing aft of it to separate;
- a time or delay that is negative or not a number.

## Powered separation

A separation has a trigger and a stage boundary ([Recovery](recovery.md#separation)). Besides
apogee, a height on the way down, a time, and a motor's [ejection
delay](../glossary.md#ejection-delay), it can now fire a delay after a motor's burnout
(`Trigger::Burnout`).

When it fires, hpr looks at the forward part, body 0, which keeps the nose:

- **If it still has a motor to burn**, it is a sustainer. That covers a motor burning now, one due
  to light at a known time, and one lit by this separation. The sustainer flies on in [six degrees
  of freedom](../glossary.md#6-dof-six-degrees-of-freedom), and the booster (body 1) descends
  under its own device.
- **If it has nothing to burn**, both parts descend under their devices, as before.

What hpr refuses:

- **A booster still burning.** Its motors must have burned out. If the separation's time is known
  before the flight, building the `Simulation` returns the error; otherwise `run` returns it when
  the separation fires, with no flight result.
- **A booster with nothing open at the split** (see *What it leaves out*, above). A device on the
  booster with `Trigger::Time { time_s: 0.0 }` and no
  [lag](recovery.md#triggers-lag-and-release) opens there, because a booster's devices
  act only once it flies on its own.
- **A separation that could never fire**, such as one timed from the burnout of the sustainer it
  lights.

**Why the state carries straight across.** hpr's flight state is the motion of the nose tip, not
of the centre of mass ([Rigid-body flight](flight.md)). The sustainer keeps the nose, so the nose
tip's position, velocity, attitude and rotation rate are all still right for it. What changes is
the rocket they belong to:

- **Mass.** The mass drops by exactly the booster's: its stages plus its spent motors. The centre
  of mass and the inertia are the sustainer's own stages and motors.
- **Aerodynamics.** hpr builds the sustainer's aerodynamics from the design cut after the
  stage boundary: the design with the booster's stages removed. It is an ordinary rocket with a
  nose and its own aft end, so its drag includes its own [base](../glossary.md#base-drag). Its
  [reference area](../glossary.md#reference-area) can be smaller than the stack's (the widest body
  may have been the booster's), so a recorded coefficient such as `Sample::axial_coefficient`
  steps at the split even where the force doesn't.

The integrator restarts at the separation, because the mass steps there.

**Momentum.** Nothing pushes the parts apart. Write `m` for a mass, `v` for the velocity of a
centre of mass, and `s`, `b` for the sustainer and the booster. The sustainer keeps the nose tip's
velocity `v_O` and rotation `ω`. The booster leaves with the velocity its own centre of mass
already had, `v_O + ω × r`, where `r` runs from the nose tip to that centre. So the two momenta add
up to the stack's, `m v = m_s v_s + m_b v_b`. A test checks this to 1e-9 of the stack's momentum
on a flight launched 10° off vertical, whose sustainer is not yet lit. With the sustainer burning,
`m v` is not simply additive, because the centre of mass also moves inside the body as propellant
burns, so hpr makes no claim for that case.

**Events.** Along with the separation, the flight records an `Ignition` event for each motor lit
after launch. `Burnout` is recorded when no motor is burning or due to light at a known time. In
the example below that is 5.23 s only: the booster burned out at 1.73 s, but the sustainer was
already due. With a sustainer lit by its separation, whose time isn't known in advance, `Burnout`
is recorded twice, at the booster's burnout and at the sustainer's.

**What the result holds.** The flight's events and final sample are the sustainer's, from the pad
to its landing. [`FlightResult::bodies`][bodies] holds the booster's descent, and
[`FlightResult::landings`][landings] gives both landings, the sustainer's first.

## A worked example

The program
[`crates/hpr-sim/examples/two_stage.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/two_stage.rs)
flies a synthetic two-stage design: a 54 mm sustainer on a 75 mm booster, with a J760 in the
booster and an I175 in the sustainer. The stages come apart 0.5 s after the booster's burnout, and
the sustainer lights 1 s after it. The ignition names the booster by its **mount's id**, as the
design file does. The separation's trigger names it by its **index** among the placed motors, as
every trigger does, so the program looks that up:

```rust
sustainer_motor.ignition = Ignition::Burnout {
    mount: "booster-motor-mount".to_owned(),
    delay_s: 1.0,
};
```

```rust
let booster = simulation
    .assembly()
    .motors
    .iter()
    .position(|motor| motor.mount == "booster-motor-mount")
    .ok_or("no booster motor")?;
```

```rust
Device::new("booster tumble", tumble, Trigger::Time { time_s: 0.0 }).on_body(1),
```

```rust
.with_separation(Separation::new(
    Trigger::Burnout {
        motor: booster,
        delay_s: 0.5,
    },
    0,
))?;
```

The `0` is the stage boundary: the stages from the nose through stage 0 are the sustainer. Run it
with `cargo run --example two_stage -p hpr-sim`. It prints:

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

The booster (1.125 kg) leaves at 2.23 s and 642.7 m, tumbling. It peaks at 745.0 m at 5.11 s and lands at 47.21 s at 17.9 m/s.
```

How to read it:

- **The split.** At 2.23 s the stack weighs 1.904 kg. The booster (its stage and a spent J760)
  takes 1.125 kg of that, so the sustainer flies on at 1.904 − 1.125 = 0.779 kg, which is the mass
  shown when it lights. The booster starts at 642.7 m rather than 643.0 m because its own centre of
  mass sits 0.3 m below the stack's.
- **The coast.** Between the separation and the ignition the sustainer coasts for half a second
  and slows from 341.2 to 280.7 m/s. That is about 121 m/s², mostly drag, on a 0.78 kg rocket near
  Mach 1.
- **The burn.** The I175 burns for 2.5 s and the mass falls to 0.550 kg: the sustainer's
  structure and a spent motor.
- **Two landings.** The sustainer lands under its parachute at 3.4 m/s. The booster tumbles from
  the split, climbs only 102.3 m more (to 745.0 m), and lands at 17.9 m/s. That short climb comes
  from tumbling side-on from near Mach 1 at once, and is likely too short (see *What it leaves
  out*).

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
  to 1e-9 of it, launched 10° off vertical.
- `an_air_start_lights_at_its_time_and_burns_on_its_own_clock`: a motor lit at 3 s, loaded until
  then, burns out at 3 s plus its burn time.
- `staging_refuses_what_it_cannot_fly`: a drag table past a powered separation, a separation
  timed while the booster burns, a delay that is negative or not a number, a booster with nothing
  open at the split, a separation that could never fire, and a powered separation after the
  sustainer's canopy opened.
- `ignition_times_follow_their_events`, `bad_ignitions_are_refused`.

[adr-074]: https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-074-ignition-times-and-powered-staging-the-sustainer-flies-on-as-a-rigid-body-2026-09-25
[ignition]: ../api/hpr_design/config/enum.Ignition.html
[bodies]: ../api/hpr_sim/flight/struct.FlightResult.html#structfield.bodies
[landings]: ../api/hpr_sim/flight/struct.FlightResult.html#method.landings
