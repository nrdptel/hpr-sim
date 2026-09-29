# The builder

This page shows the shortest way to fly a rocket of your own with hpr-sim. The `hpr`
[crate](glossary.md#crate) has four types for it: `Environment`, `Motor`, `Rocket` and `Flight`.
You describe the rocket part by part from the nose back, put a motor in it, and fly it from a
rail. The page runs two example programs and walks through them. It needs the setup from
[Getting started](getting-started.md), and some Rust.

> **How far to trust it.** The builder adds no physics. It writes the same
> [design tree](physics/design.md) that [Your own rocket](your-own-rocket.md) writes by hand, and
> flies it with the same simulation. A test builds that page's rocket with the builder and flies
> it: the mass properties and the whole flight come out the same, bit for bit. So everything that
> page says about trusting its numbers holds here too. They come from each part's shape and a
> published density, and from models whose checks are listed under [Accuracy](accuracy.md). No
> flight of this rocket has been checked against a real one.

## Run it

```bash
cargo run --example build_and_fly -p hpr
```

This builds a small rocket for 29 mm motors and weighs it. It finds its
[centre of gravity](glossary.md#centre-of-gravity-cg) (CG), its
[centre of pressure](glossary.md#centre-of-pressure-cp) (CP) and its
[stability margin](glossary.md#stability-margin). Then it flies the rocket on a Cesaroni H54 from
a rail leaning into a light wind, with a parachute that opens at the motor's
[ejection delay](glossary.md#ejection-delay). It prints this:

<!-- quote: crates/hpr/examples/build_and_fly.output.txt -->
```text
My 29 mm rocket on a 168H54-10A
Not yet validated: see the Accuracy page before trusting these numbers.

                                 liftoff     spent
mass (kg)                          0.675     0.579
centre of gravity (m from nose)    0.671     0.610
centre of pressure (m from nose)   0.779     0.779
stability margin (calibres)         1.92      2.99

From a 1.8 m rail leaning 5° west, in 5 m/s of wind from the west:
Rail exit:  21.7 m/s
Apogee:     1106.6 m above the pad, at 13.69 s
Top speed:  186 m/s (Mach 0.56)
Landing:    877 m from the pad, 877 m east and 0 m north, at 4.6 m/s, at 246.3 s
```

What the lines say:

- **Liftoff and spent.** The rocket loses 96 g of propellant as the motor burns. That moves the CG
  6 cm forward, and the margin grows from 1.92 to 2.99 [calibres](glossary.md#calibre-caliber).
- **The CP** is at Mach 0.3, with the air straight along the rocket, from
  [Barrowman's method](glossary.md#barrowmans-method). It doesn't change as the motor burns.
- **The flight.** The rail leans 5° to the west, into the wind, so the rocket starts west. It turns
  further into the wind as it climbs, a rocket's usual
  [weathercocking](glossary.md#weathercocking). Under the parachute the wind carries it back east
  of the pad, 877 m.
- **Heights** are the rocket's CG's above the launch site. The CG starts above the ground, where
  it sits on the rail, so the first height is not zero.

## The program

The program is
[`crates/hpr/examples/build_and_fly.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/build_and_fly.rs).
It takes four steps: the motor, the rocket, the weighing and the flight.

**The motor.** `Motor::from_catalog("H54")` takes a motor from the catalog built into hpr-sim. It
finds the motor by its designation or its common name, and ignores case, spaces and hyphens. Only
the 32 motors that come with a thrust curve can be found; the [motor page](physics/motor.md) lists
them. A name that matches two different motors is refused rather than guessed. The delay is never
read from the designation: `with_delay_s(10.0)` sets it. `Motor::from_eng` reads a RASP `.eng`
file's text instead ([`.eng` files](format/eng.md)).

**The rocket.** `Rocket::new(name, diameter_m)` starts an empty rocket with a body diameter. Then
each `add_` call adds one part and returns the rocket, so the calls chain. Body parts stack from
the nose tip in the order you add them. The parts that attach inside or on the airframe go on the
last body tube you added:

| part | what it is | where it goes |
|---|---|---|
| `Nose::hollow`, `Nose::solid` | A nose cone of a shape and length, and its wall. `with_shoulder` adds a shoulder that fits the tube behind | First, at the tip. Its base takes the rocket's diameter |
| `Tube::new` | A body tube of a length and a wall. `with_diameter_m` makes it wider or narrower than the rocket | Behind the last body part |
| `Transition::conical` | A shoulder or boattail, to a new diameter at its aft end | Behind the last body part, starting at its diameter |
| `Fins::trapezoidal`, `Fins::elliptical` | A set of identical fins, square-edged unless `with_cross_section` says otherwise | On the last tube, flush with its aft end unless `at` places them |
| `MotorTube::new` | The tube the motor goes in: its length, bore and wall | In the last tube, flush with its aft end unless `at` places it |
| `Mass::new` | Anything else inside: a recovery bay, an altimeter, ballast | In the last tube, where its position puts it |

Every part names its material, and every hollow part its wall. `material("abs")` finds one of the
built-in materials, each with the source of its density; the
[mass page](physics/mass.md) explains how the masses are found. The builder has no default material
or wall, because each would be a guess at your rocket's mass. It does have a few defaults that
change no mass: fins are square-edged (which changes their drag), attached parts sit flush with the
tube's aft end, and a `Mass` is a point unless `packed` gives it a size (which changes only how it
turns).

`set_motor` puts the motor in the motor tube, lit at launch. `add_parachute` adds a recovery device:
here a 90 cm flat parachute, opened by the motor's ejection charge,
`Trigger::MotorDelay { motor: 0 }`. A device adds drag, not mass; the parachute's mass is in the
200 g recovery bay. The [recovery page](physics/recovery.md) explains the devices and what opens
them.

**Weighing it.** `mass_properties(t)` gives the mass, CG and inertia `t` seconds after the motor
lights. `margin(t, mach)` gives the CP and the margin, and `static_margin_cal(t, mach)` the margin
alone. The CG is in the [body frame](glossary.md#body-frame), whose `z` axis points to the nose,
so a point 0.671 m behind the nose tip is at `z = −0.671`.

**Flying it.** `Environment::new(latitude, longitude, elevation)` is the launch site, with the
standard atmosphere and no wind; `with_constant_wind(5.0, 270.0)` adds 5 m/s from the west. The
elevation is taken both as the height above sea level, where the air is read, and as the height
above the [ellipsoid](glossary.md#ellipsoidal-height). `Flight::builder(&rocket, &environment, 1.8)`
sets up a flight from a 1.8 m rail, vertical unless `inclination_deg` and `heading_deg` lean it.
`fly()` flies it to the ground. The flight's `apogee_m`, `max_speed_m_s`, `rail_exit_speed_m_s` and
`landing` are the numbers most asked for; `summary()` has every metric the
[metrics page](physics/metrics.md) describes.

## Which motor

The second example flies the same rocket on each 29 mm motor that comes with hpr-sim:

```bash
cargo run --example motor_choice -p hpr
```

<!-- quote: crates/hpr/examples/motor_choice.output.txt -->
```text
My 29 mm rocket from a 1.8 m vertical rail, in 5 m/s of wind from the west
Not yet validated: see the Accuracy page before trusting these numbers.

motor        liftoff  margin  rail exit   apogee  top speed  best delay
               (kg)   (cal)     (m/s)      (m)     (m/s)        (s)
F52C           0.548    3.32       18.5    438.6        106         8.0
168H54-10A     0.675    1.92       21.7   1134.6        186        10.5
```

`set_motor` swaps the motor in the tube, so one rocket flies on both. The best delay is the time
from burnout to apogee, so the charge fires at the top. It is 10.5 s on the H54, the 10 s delay
that motor's designation names, near enough. The F52 wants 8 s.

The builder has no method for the best delay, but `hpr_sim` has one. The flight builder's
`simulation()` hands over the simulation it would fly, and the program passes it to
`hpr_sim::metrics::optimum_delays`. That is the way to everything the builder doesn't offer:
staging, clusters, pods, moving and released masses, events of your own. A rocket's `design()`
is the design tree, which a [design file](glossary.md#design-file) holds, and
`Rocket::from_design` flies a tree you already have, such as a design file or an OpenRocket file
([`.ork` files](format/ork.md)).

## What it refuses

Each number is checked as you give it. A negative length, a NaN, a zero diameter or zero fins is
refused with an error that names it, such as `tube diameter, m is 0, outside its domain`. So is a
part in an order the tree can't take: fins before any tube, a second motor tube, a nose behind a
tube. A design that comes in whole through `from_design` is checked before it flies. A test tries
each of these and pins which check refuses it; one rocket in it has no fins at all, and it flies,
tumbling, to finite numbers. That test is [Loft lesson L95](decisions-and-roadmap.md#l95)'s.

## What it can't do yet

- **Models of your own.** A drag model of your own, in place of hpr's, is planned
  ([M4.1b](decisions-and-roadmap.md#m4-1b)). Until then `simulation()` takes a drag table from
  another program (`Simulation::with_drag_table`), as
  [Getting started](getting-started.md#how-far-to-trust-it) does.
- **One motor.** A built rocket has one motor tube and one motor, lit at launch. Clusters, staging
  and pods take the crates' own types; the [staging page](physics/staging.md) explains how they
  fly.
- **Launch lugs, rail buttons, centering rings.** The builder has no method for these yet. Their
  mass can go in as a `Mass`.

## Where next

- [The API reference](api.md) documents every method, starting at the `hpr` crate.
- [Your own rocket](your-own-rocket.md) builds the same rocket without the builder, and shows
  every field a part has.
- [Recording a trajectory](recording-a-trajectory.md) keeps the flight's path: pass a recorder to
  `fly_with` instead of calling `fly`.
