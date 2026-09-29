# Models of your own

This page shows how to fly a model of your own in place of one of hpr-sim's: a drag model, a wind,
or an atmosphere. You might have a drag curve from a wind tunnel, from another program, or from
your own flights, or a wind profile from a weather balloon that none of the built-in winds fits.
The page runs two example programs and walks through them. It follows on from
[The builder](the-builder.md), and needs some Rust.

> **How far to trust it.** As far as your model. hpr-sim refuses a drag coefficient that is
> negative or not a finite number, and a wind that isn't finite stops the flight too, but it can't
> know whether a model is right. What it does check is the plumbing: a test flies a drag model that hands back
> hpr's own drag and gets the same flight, bit for bit, as flying without one, and a model that
> gives the same drag at every speed flies exactly as a [drag table](glossary.md#drag-coefficient)
> of that value does
> ([`a_drag_model_is_flown_in_place_of_hprs_drag`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/src/tests.rs)).
> An atmosphere of your own has no such test yet.
> The invented numbers on this page are not measurements of any rocket.

## Which models you can replace

Each is a Rust [trait](https://doc.rust-lang.org/book/ch10-02-traits.html): a set of methods your
own type provides. Write the type, give it the method, and hand it over.

| Model | What your type gives | How to fly it |
|---|---|---|
| Drag, `DragModel` | The rocket's zero-lift [drag coefficient](glossary.md#drag-coefficient) at a flow | `Flight::builder(...).drag_model(model)` |
| Wind, `Wind` | The wind's velocity at a height | `Environment::with_wind(wind)` |
| Atmosphere, `Atmosphere` | The air's pressure, temperature and density at a height | `Environment::with_atmosphere(air)` |

The zero-lift drag coefficient is the drag with the rocket pointed straight into the air, divided
by the [dynamic pressure](glossary.md#dynamic-pressure) and the rocket's reference area, the area
of a circle of its diameter. A drag model replaces that number and nothing else:

- **At an angle of attack**, hpr-sim scales your number up and down with the angle as it scales its
  own ([Drag](physics/aero.md#drag)).
- **The normal force, centre of pressure, roll and damping** stay hpr-sim's. So the stability
  margin a rocket reports doesn't change with the drag model.
- **Recovery** doesn't use it: under a parachute the drag is the parachute's.

A drag table read from another program's export (`with_drag_table`, which
[Getting started](getting-started.md#how-far-to-trust-it) uses) works the same way; a drag model
is the same idea with your code in place of the table. The last one set is the one flown.

## A drag model

```bash
cargo run --example custom_drag -p hpr
```

This flies the rocket of [The builder](the-builder.md) three times on a Cesaroni H54, from a
vertical rail in calm air: with hpr-sim's own drag, with that drag made 10% higher, and with a drag
curve of invented numbers. Then it tries a curve that stops short of the rocket's speed. It prints
this:

<!-- quote: crates/hpr/examples/custom_drag.output.txt -->
```text
My 54 mm rocket on a 168H54-10A, from a 1.8 m vertical rail in calm air
Not yet validated: see the Accuracy page before trusting these numbers.

drag                  apogee  apogee at  top speed
                         (m)        (s)      (m/s)
hpr's own drag         1144.5      13.92        187
hpr's, 10% higher      1091.2      13.65        183
a curve (invented)     1146.2      13.89        188

A curve that ends at Mach 0.4 stops the flight: Mach number past the drag curve's last point, 0.40
```

What the lines say:

- **10% more drag** costs 53.3 m of [apogee](glossary.md#apogee), 4.7%, and reaches it 0.27 s
  sooner. That is a quick way to see how much a rougher finish, or an uncertain drag, could
  matter.
- **The curve** was made up to look like a small rocket's, so it lands near hpr-sim's own. A curve
  from your own data would put your numbers here.
- **The short curve** refuses to guess past its last point, so the flight stops there and says
  why, instead of flying on made-up drag.

### The program

The program is
[`crates/hpr/examples/custom_drag.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/custom_drag.rs).
Each model is a type with one method, `zero_lift_drag`, which gets a `DragQuery` and returns the
coefficient:

- `query.mach()` is the [Mach number](glossary.md#mach-number).
- `query.conditions().thrusting` says whether a motor is burning. A burning motor fills the base of
  the rocket, so its drag is lower ([power-on and power-off drag](glossary.md#power-on-and-power-off-drag)).
  The example's curve has a column for each.
- `query.conditions().reynolds_per_m` is the [Reynolds number](glossary.md#reynolds-number) per
  metre, for a model that depends on it.
- `query.buildup()` is hpr-sim's own drag at that flow, what the flight would have used without
  the model. The 10% model multiplies it by 1.1.

The method returns a `Result`, so a model can refuse a question it can't answer, as the curve does
past Mach 0.4; the flight stops with that error. `Flight::builder(...).drag_model(model)` flies
the model. A model is asked several times every step of the flight, so it should be quick, and
give the same answer to the same question: a flight is only as repeatable as its models.

## A wind model

```bash
cargo run --example custom_wind -p hpr
```

hpr-sim has steady, power-law, log-law and layered winds built in ([Wind](physics/wind.md)). This
example writes one of its own: a wind that grows from 4 m/s at the ground to 10 m/s at 1,000 m, and
veers, turning clockwise, from the west (270°) to the north-west (315°) on the way up. Above
1,000 m it holds steady. It flies the same rocket in that wind and in a steady 4 m/s west wind:

<!-- quote: crates/hpr/examples/custom_wind.output.txt -->
```text
My 54 mm rocket on a 168H54-10A, from a 1.8 m vertical rail
Not yet validated: see the Accuracy page before trusting these numbers.

wind                    apogee   landing east  landing south  descent
                           (m)            (m)            (m)    (m/s)
steady, from the west   1137.3            855              0      4.6
veering                 1135.0           1397            834      4.6
```

Both winds are the same at the ground, where a flier would measure them. Aloft, the veering wind is
stronger and comes from further north, so the rocket drifts further under its parachute, and to
the south-east rather than due east. The parachute's descent rate is the same in both.

The program is
[`crates/hpr/examples/custom_wind.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/custom_wind.rs).
Its `Veering` type has one method, `wind`, which gets a height above mean sea level and returns
the air's velocity as east, north and up components. A wind is asked by height above sea level,
not above the ground, so the type keeps the ground's elevation to measure from.
`Environment::with_wind` flies it. A wind that returns a NaN or an infinity stops the flight
with an error that names the Reynolds number, the first check the air's speed reaches
([`a_wind_that_is_not_finite_stops_the_flight`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/src/tests.rs)).
The [wind direction](glossary.md#wind-direction) is where the
wind blows from, so a west wind moves the air east.

## An atmosphere

An atmosphere of your own works the same way, through `Environment::with_atmosphere`. hpr-sim's
own are the [standard atmosphere](glossary.md#standard-atmosphere) and a weather balloon's
sounding ([Atmosphere](physics/atmosphere.md)); there is no example program for this one yet.

## Where next

- The API reference's [`guide`](api/hpr/guide/index.html) module says the same in five short
  chapters, each with code that CI runs, and [`hpr_aero::custom`](api/hpr_aero/custom/index.html)
  documents the drag trait.
- [The builder](the-builder.md) builds the rocket these examples fly.
- [Accuracy](accuracy.md) says how well hpr-sim's own models have been checked, which is what a
  model of your own replaces.
