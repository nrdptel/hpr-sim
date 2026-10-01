# The builder

This page shows the shortest way to fly a rocket of your own with hpr-sim. The `hpr`
[crate](glossary.md#crate) has four types for it: `Environment`, `Motor`, `Rocket` and `Flight`.
You describe the rocket part by part from the nose back, put a motor in it, and fly it from a
rail. The page runs four example programs and walks through them. It needs the setup from
[Getting started](getting-started.md), and some Rust.

> **How far to trust it.** The builder adds no physics. It writes the same
> [design tree](physics/design.md) that [Your own rocket](your-own-rocket.md) writes by hand, and
> flies it with the same simulation. A test builds that page's rocket with the builder and flies
> it:
> the mass properties and the whole flight come out the same, bit for bit
> ([`the_builder_makes_the_rocket_own_rocket_builds_by_hand`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/src/tests.rs)).
> So what that page says about trusting its numbers holds here. The masses come from each part's
> shape and a published density, so glue, paint and hardware are missing until you weigh the
> parts. This rocket has never been flown for real, so no flight checks it. Where it lands in a
> wind is the least certain number of all: on two of RocketPy's example rockets, hpr's drift and
> RocketPy's differ by 11 to 43%
> ([Getting started](getting-started.md#how-far-to-trust-it) explains why).

## Run it

```bash
cargo run --example build_and_fly -p hpr
```

This builds the small rocket of [Your own rocket](your-own-rocket.md). Its airframe is 54 mm
inside and 56.3 mm outside, and it takes 29 mm motors. It weighs the rocket, and finds its
[centre of gravity](glossary.md#centre-of-gravity-cg) (CG), its
[centre of pressure](glossary.md#centre-of-pressure-cp) (CP) and its
[stability margin](glossary.md#stability-margin). Then it flies the rocket on a Cesaroni H54 from
a rail leaning into a light wind, with a parachute that opens at the motor's
[ejection delay](glossary.md#ejection-delay). It prints this:

<!-- quote: crates/hpr/examples/build_and_fly.output.txt -->
```text
My 54 mm rocket on a 168H54-10A
Not yet validated: see the Accuracy page before trusting these numbers.

                                 liftoff     spent
mass (kg)                          0.675     0.579
centre of gravity (m from nose)    0.671     0.610
centre of pressure (m from nose)   0.779     0.779
stability margin (calibres)         1.92      2.99

From a 1.8 m rail leaning 5° west, in 5 m/s of wind from the west:
Rail exit:  21.7 m/s
Apogee:     1106.6 m above the pad, at 13.69 s, 282 m west of it
Top speed:  186 m/s (Mach 0.56)
Landing:    877 m east of the pad, at 4.6 m/s, at 246.3 s
```

What the lines say:

- **Liftoff and spent.** The rocket loses 96 g of propellant as the motor burns. That moves the CG
  6 cm forward, and the margin grows from 1.92 to 2.99 [calibres](glossary.md#calibre-caliber).
- **The CP** is worked out by [Barrowman's method](glossary.md#barrowmans-method) at
  [Mach](glossary.md#mach-number) 0.3, a typical subsonic speed, with the air straight along the
  rocket. It depends on the shape alone, so it doesn't move as the motor burns.
- **The flight.** The [rail exit](glossary.md#rail-exit-and-rail-exit-velocity) speed is how fast the rocket leaves the
  rail. The rail leans 5° west, into the wind, and the rocket turns further into the wind as it
  climbs, a rocket's usual [weathercocking](glossary.md#weathercocking). The lean and the turn
  together put its [apogee](glossary.md#apogee) 282 m west of the pad. Under the parachute the
  wind carries it back, to land 877 m east of the pad.
- **Heights** are the height of the rocket's CG above the launch site. The CG starts above the
  ground, sitting on the rail, so the apogee includes that starting height.

## The program

The program is
[`crates/hpr/examples/build_and_fly.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/build_and_fly.rs).
It takes four steps: the motor, the rocket, the weighing and the flight.

### The motor

`Motor::from_catalog("H54")` takes a motor from the catalog built into hpr-sim. It finds the motor
by its designation, such as `168H54-10A`, or by its common name, the short form such as `H54`,
and ignores case, spaces and hyphens. Only the 32 motors that come with a thrust curve can be
found; the [motor page](physics/motor.md) lists them. A name that matches two motors, as `I175`
does, is refused with both listed rather than guessed. The delay is never read from the
designation: `with_delay_s(10.0)` sets it, and a parachute opened by the motor's charge needs it.
`Motor::from_eng` reads the text of a [RASP `.eng` file](format/eng.md), such as one downloaded
from ThrustCurve.org, instead.

### The rocket

`Rocket::new(name, diameter_m)` starts an empty rocket with an outer body diameter. Then each
`add_` call adds one part and returns the rocket, so the calls chain. Body parts stack from the
nose tip in the order you add them. The parts that go inside or on the airframe attach to the last
body tube you added:

| part | what it is | where it goes |
|---|---|---|
| `Nose::hollow`, `Nose::solid` | A nose cone of a shape and length, and its wall. `with_shoulder` adds the sleeve that fits inside the tube behind; `with_capped_shoulder` closes the sleeve's aft end with a disc | First, at the tip. Its base takes the rocket's diameter |
| `Tube::new` | A body tube of a length and a wall. `with_diameter_m` gives it another diameter | Behind the last body part, at its diameter |
| `Transition::conical` | A cone to a new diameter at its aft end: a [boattail](glossary.md#boattail), or a step up or down in the airframe | Behind the last body part, starting at its diameter |
| `Fins::new` | A set of identical fins of a shape (`FinPlanform`, which names each dimension), square-edged unless `with_cross_section` says otherwise | On the last tube, flush with its aft end unless `at` places them |
| `MotorTube::new` | The tube the motor goes in: its length, bore and wall | In the last tube, flush with its aft end unless `at` places it |
| `Mass::new` | Anything else inside: a recovery bay, an altimeter, ballast | In the last tube, where its position puts it |

A position (`Position`) is measured along the tube the part is on: `Top` places the part's fore end
a distance aft of the tube's, `Bottom` its aft end from the tube's aft end, `Middle` its middle
from the tube's middle, `After` its fore end behind the part before it, and `Absolute` its fore end
from the nose tip. So a `Mass` given a size with `packed` has its centre half that length from the
end its position names, or at the point itself when placed by its `Middle`. In the example,
packing the recovery bay 15 cm long, its top 7 cm down the tube, moves the rocket's CG at liftoff
22 mm aft of where a point mass at the top puts it.

Every part names its material, and every hollow part its wall. `material("abs")` finds one of the
built-in materials, each with the source of its density; the [mass page](physics/mass.md) explains
how the masses are found. The builder has no default material or wall, because each would be a
guess at your rocket's mass. The defaults it does have change the drag, not the mass: fins have
square edges, and every outer surface has the design's default finish, mass-production paint,
which sets the friction of the air on it ([Drag](physics/aero.md#drag)). The builder can't change the finish yet.

`set_motor` puts the motor in the motor tube, lit at launch. `add_parachute` adds a recovery device:
here a 90 cm flat parachute opened by the ejection charge of motor number 0, the first,
`Trigger::MotorDelay { motor: 0 }`. A device adds drag, not mass; the parachute's mass is in the
200 g recovery bay. The [recovery page](physics/recovery.md) explains the devices and what opens
them.

### Weighing it

`mass_properties(t)` gives the mass, CG and inertia `t` seconds after the motor lights.
`margin(t, mach)` gives the CP and the margin, and `static_margin_cal(t, mach)` the margin alone.
The CG is in the [body frame](glossary.md#body-frame), whose `z` axis points to the nose, so a
point 0.671 m behind the nose tip is at `z = −0.671`. The CP is a station, metres aft of the tip,
so the example prints it as it comes and the CG with its sign turned. Weighing runs the same
checks on the design as a flight does, so a motor wider than its tube is refused by both.

### Flying it

`Environment::new(latitude, longitude, elevation)` is the launch site, in degrees north, degrees
east (so the Americas are negative) and metres. It has the
[standard atmosphere](glossary.md#standard-atmosphere) and no wind; `with_constant_wind(5.0, 270.0)`
adds 5 m/s blowing from 270°, the west. The elevation is taken both as the height above sea level,
where the air is read, and as the height above the [ellipsoid](glossary.md#ellipsoidal-height).

`Flight::builder(&rocket, &environment, 1.8)` sets up a flight from a vertical 1.8 m rail.
`inclination_deg` is the rail's angle above the horizon: 90 is vertical, and 85 leans 5° off it.
OpenRocket measures its launch rod angle from the vertical instead, so its 5° is 85 here.
`heading_deg` is the way the rail leans, clockwise from true north (add the
[declination](physics/magnetic.md) to a compass reading). A wind's direction is where it
comes from, so a rail leaning into a west wind has both at 270. `fly()` flies the rocket to the
ground. The flight's `apogee_m`, `max_speed_m_s`, `rail_exit_speed_m_s` and `landing` are the
numbers most asked for; `summary()` has every metric the [metrics page](physics/metrics.md)
describes.

## Which motor

The second example flies the same rocket on each 29 mm motor that comes with hpr-sim, from a
vertical rail, with the parachute opening at apogee whatever the delay:

```bash
cargo run --example motor_choice -p hpr
```

<!-- quote: crates/hpr/examples/motor_choice.output.txt -->
```text
My 54 mm rocket from a 1.8 m vertical rail, in 5 m/s of wind from the west
Not yet validated: see the Accuracy page before trusting these numbers.

motor        liftoff  margin  rail exit   apogee  top speed  best delay
               (kg)   (cal)     (m/s)      (m)     (m/s)        (s)
F15            0.568    3.01       10.0    207.5         55         4.6
F52C           0.548    3.32       18.5    438.6        106         8.0
168H54-10A     0.675    1.92       21.7   1134.6        186        10.5
```

`set_motor` swaps the motor in the tube, so one rocket flies on all three. The H54 reaches
1134.6 m here, from a vertical rail in the wind. The same rocket reaches 1144.5 m on
[Your own rocket](your-own-rocket.md), from a vertical rail in calm air, and 1106.6 m at the top of
this page, from a leaning rail. The parachutes open at different times too: here at apogee, there
at the motor's charge.

The best delay is the time from [burnout](glossary.md#burnout), the end of the thrust curve, to
apogee, so the charge fires at the top. It is 10.5 s on the H54, near the 10 s delay that motor's
designation names. The H54 burns out at 3.5 s, so on the leaning rail at the top of this page its
10 s delay fires at 13.5 s, 0.2 s before its apogee. The F52 wants 8 s. The F15 leaves the rail
at only 10 m/s, the slowest of the three, and a slow rocket's fins have the least air to
steer with.

## Sizing fins

The third example builds the same rocket with fins of five spans, the fin's height from the body
tube to its tip, and flies each from a vertical rail, in calm air and in 5 m/s of wind from the
west. The parachute opens at apogee here, not at the motor's charge, so the charge doesn't cut
the climb short:

```bash
cargo run --example fin_sizing -p hpr
```

<!-- quote: crates/hpr/examples/fin_sizing.output.txt -->
```text
My 54 mm rocket on an H54, fins of five spans, in calm air and a 5 m/s west wind
Not yet validated: see the Accuracy page before trusting these numbers.

fin span  liftoff  margin   apogee (m)       in the wind: drift (m)
    (mm)     (kg)   (cal)   calm   wind   apogee upwind  landing downwind
      25    0.666   -2.11   too little margin to fly
      35    0.671    0.33   too little margin to fly
      45    0.675    1.92   1145   1135             119              1068
      55    0.680    2.97   1126   1110             156              1001
      65    0.684    3.67   1107   1088             176               954
```

Because a rocket is a value built by a function, a design study is a loop. The example's
`rocket(span_m)` builds the rocket with fins of that span, and the loop weighs and flies each.

- **The margin**, at liftoff and at Mach 0.3, grows fast with the span: from −2.11 calibres, a CP
  ahead of the CG, to 3.67. The usual rule of thumb asks for at least one calibre
  ([stability margin](glossary.md#stability-margin)), so the program doesn't fly the two
  smallest; the example's own fins are the 45 mm ones.
- **Bigger fins cost height.** In calm air the 65 mm fins reach 38 m less than the 45 mm ones:
  that is their drag and their extra mass, about 9 g. In the wind they lose 47 m, since they also
  turn the rocket further into it, as the apogee drift shows: 119 m upwind with the 45 mm fins,
  176 m with the 65 mm.
- **The landing** is closer with bigger fins, 114 m closer from the 45 mm to the 65 mm: the
  parachute opens further upwind, and lower, so it drifts for less time.

## Parts from a catalogue

A rocket can also be built from a maker's parts, as sold, from a parts catalogue. hpr-sim bundles
the one OpenRocket ships: 3,449 parts from Estes, LOC Precision and a dozen more makers, read as
the [`.orc` page](format/orc.md) explains. The fourth example finds LOC Precision's 2.56 in
(65 mm) airframe parts in it by maker and part number, builds the rocket from them, and flies it on
an AeroTech H170:

```bash
cargo run --example catalog_rocket -p hpr
```

<!-- quote: crates/hpr/examples/catalog_rocket.output.txt -->
```text
LOC 2.56 in from the catalogue, with motor H170M
Not yet validated: see the Accuracy page before trusting these numbers.

part                                     mass (g)
LOC Precision PNC-2.56                       87.7
LOC Precision BT-2.56                       120.0
LOC Precision BT-1.52, MMT-1.52              43.6
LOC Precision CR-2.56-38mm                    4.2
LOC Precision CR-2.56-38mm                    4.2
fins                                         99.7
LOC Precision LP-36-2022                     70.7
structure                                   430.1

At liftoff: 0.760 kg, centre of gravity 0.728 m from the nose,
stability margin 1.07 calibres at Mach 0.3.

Rail exit:  30.0 m/s
Apogee:     1119.3 m above the pad, at 11.77 s
Top speed:  302 m/s (Mach 0.91)
Landing:    at 4.6 m/s, at 250.8 s
```

Each part is named by its maker and part number, as the design keeps it. The program's steps:

- **Find the part.** `hpr_io::orc::bundled().find("LOC Precision", "PNC-2.56")` returns the
  parts with that maker and number. Use the maker's name as the file writes it ("LOC
  Precision", where OpenRocket shows "LOC/Precision").
- **Make the builder's part from it.** `Nose::from_catalog(part)`, `Tube::from_catalog`,
  `Transition::from_catalog` and `MotorTube::from_catalog` (a body tube used as the motor tube)
  take the catalogue's sizes, material and density. A catalogue nose or transition states its own
  diameters, so the tube behind it takes the nose's diameter, not the one given to
  `Rocket::new`.
- **Cut a tube.** Catalogue tubes are sold long (LOC's motor tube is 34 in). `with_length_m`
  cuts one to length, and a stated mass is cut in proportion.
- **Fittings.** A *fitting* is a part that goes in or on the last body tube: a coupler, an engine
  block, a centering ring, a bulkhead, a launch lug, or a packed parachute or streamer.
  `Fitting::from_catalog(part)` makes one from a catalogue part, and `Rocket::add_fitting` adds
  it, flush with the tube's aft end unless `at` places it. `Fitting::centering_ring`,
  `bulkhead`, `coupler` and `launch_lug` make one to your own sizes.
- **A parachute is two things.** As a fitting it is its weight: canopy and shroud lines, at its
  place in the tube. Its drag is a recovery device, as in the first example. The example gives
  that device the catalogue's diameter.
- **The fins** are made by hand: the catalogue has none.

A part that states its mass in the catalogue weighs that mass. The mass is an override on the
part, which scales its density, so its centre of mass stays where its shape puts it. 229 of the
3,449 parts state a mass.

### What the catalogue leaves unsaid

A catalogue leaves some sizes out. The builder fills each one in the way OpenRocket 24.12 does
when it builds the part, with one exception, a hollow part's shoulder:

| Left unsaid | The builder's choice | OpenRocket's |
|---|---|---|
| An ogive's shape | tangent | the same |
| A parabola's parameter `K′` | 1 | the same |
| A Haack series' `C` | 0, the von Kármán | the same |
| A power series' exponent | ½ | the same |
| Whether a transition is clipped | yes for elliptical, Haack and power series | the same |
| A solid part's shoulder | solid | the same |
| **A hollow part's shoulder wall** | **the part's own wall** | **zero: it weighs nothing** |

A *clipped* transition is cut from a whole nose cone ([Shapes](physics/shapes.md)). A molded
plastic nose cone's shoulder is a tube of the same plastic, so a shoulder that weighs nothing
would be wrong. On the 67 hollow parts with a shoulder and no stated mass, the builder's part is
heavier than OpenRocket's by exactly that shoulder.

### How far to trust it

[`tests/catalog_openrocket.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/tests/catalog_openrocket.rs)
builds every part in the catalogue with the builder. It holds each part's mass and centre of mass
to what OpenRocket builds from it, as recorded by
[`orc_built.py`](https://github.com/nrdptel/hpr-sim/blob/main/validation/oracles/openrocket/orc_built.py).
The tolerances were set before measuring:

| Parts | Allowed | Largest difference found |
|---|---|---|
| Tubes, couplers, rings, bulkheads, lugs, parachutes, streamers (2,228) | 1e-12 of the mass; centre 1e-12 of the length | under 1e-14 of the mass; centre under 1e-13 of the length |
| Nose cones and transitions (1,214, less the departures below) | 1e-3 of the mass; centre 1e-3 of the length | 6.3e-4 of the mass; centre 7.0e-5 of the length |

The second row allows more because OpenRocket's volumes of nose cones and transitions are not
exact. Even a cone, whose volume has a formula, differs by up to 6.8e-7. The test holds hpr's 85
solid cones to that formula, so the difference is OpenRocket's. The test also counts each part
that departs from OpenRocket, and checks why it does:

- **67 hollow shoulders.** Each part weighs OpenRocket's mass plus its shoulder's closed-form
  mass. 74 more hollow parts state a mass and weigh it; their centre is not compared.
- **3 hollow ellipsoid nose cones** (short, blunt ones) weigh up to 0.48% more than OpenRocket's.
  The test works out each wall's volume on its own, from the curve that runs one wall thickness
  inside the ellipse. hpr's agrees with it to 1e-9 on all six hollow ellipsoids, so the
  difference is OpenRocket's.
- **185 masses stated in ounces** differ by OpenRocket's rounded ounce, 8.8e-10 of the mass.
- **1 streamer** states its mass, and hpr uses it; OpenRocket ignores it.
- **6 parts are refused.** Three name a material their file doesn't define, and three are tubes
  or rings whose bore isn't narrower than their outside. OpenRocket weighs all six as nothing.

What this checks is the mass of each part as the catalogue describes it. A catalogue's sizes and
densities are the makers' or the database's, and none was weighed here. Glue, paint and
hardware are still missing: weigh the finished parts when you can.

## Beyond the builder

The builder covers a single-stage rocket with one motor. Two ways lead further, both into the
crates the builder is made of.

- **Change the design.** A rocket's `design()` is its [design tree](physics/design.md), which a
  [design file](glossary.md#design-file) holds. Clone it, add what the builder can't with the
  `hpr_design` crate (a cluster, pods, rail buttons, a stage), and make a rocket of
  it with `Rocket::from_design(tree, configuration)`, which names the configuration, the motors,
  to fly. `from_design` also takes a design file, or an OpenRocket file read as the
  [`.ork` page](format/ork.md) shows. A second stage also needs a separation, and a recovery
  device on each part it makes, both added through `simulation()` below; the
  [`ork_two_stage` example](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/ork_two_stage.rs)
  flies one, building its simulation with `Simulation::new` rather than through the builder.
- **Change the flight.** A flight builder's `simulation()` hands over the simulation `fly()` would
  run. The `motor_choice` example passes it to `hpr_sim::metrics::optimum_delays`. The
  simulation's methods add a separation, events of your own, or moving or released masses. A
  drag model of your own, or another program's drag table, needs no detour: the flight builder's
  `drag_model` and `drag_table` take them ([Models of your own](custom-models.md)). `run(&mut ())` flies it,
  with no observer watching. A flight flown that way returns the simulation's own result, without the
  builder's `Flight` methods; `hpr_sim::FlightMetrics` gives the same metrics
  ([Flight metrics](physics/metrics.md)).

## What it refuses

Each part's numbers are checked when the part is added: a negative length, a NaN, a zero diameter,
zero fins, a shape parameter out of range. The error names the number, such as
`tube diameter, m is 0, outside its domain`. So is a part in an order the tree can't take: fins
before any tube, a second motor tube, a nose behind a tube. The launch site and wind are
checked when they are given, the rail when the flight is set up, and a design that comes in whole
through `from_design` before it flies. Tests try each of these and pin which check refuses it
([`parts_out_of_order_are_refused`, `degenerate_designs_error_or_stay_finite`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/src/tests.rs)).
One rocket in them has no fins at all: it flies, tumbling, to finite numbers. That test is
[Loft lesson L95](decisions-and-roadmap.md#l95)'s: a
[Loft lesson](glossary.md#loft-lesson) is a mistake of the project before this one, with the test
that guards against it here.

## What it can't do yet

- **One motor, one stage.** Clusters, staging and pods go through the design, as above; the
  [staging page](physics/staging.md) explains how they fly.
- **Rail buttons** go through the design. Without them the rocket leaves the rail when its aft
  end passes the rail's top. A launch lug is a fitting (above); with one, the rocket leaves the
  rail when the lug's aft edge passes the rail's top.
- **The finish.** Every surface is painted, as above.

## Where next

- [Models of your own](custom-models.md) flies a drag model and a wind of your own in hpr-sim's
  place.
- The API reference's [`guide`](api/hpr/guide/index.html) module is this page's walk-through in
  five short chapters, each with code that CI runs.
- [The API reference](api.md) documents every method, starting at the `hpr` crate.
- [Your own rocket](your-own-rocket.md) builds the same rocket without the builder, and shows
  every field a part has.
- [Recording a trajectory](recording-a-trajectory.md) keeps the flight's path: pass a recorder to
  `fly_with` instead of calling `fly`.
