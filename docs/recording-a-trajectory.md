# Recording a trajectory

This page shows how to get a whole flight out of hpr-sim as a table of numbers over time, which a
spreadsheet or a plotting tool reads. It flies the rocket of [Getting started](getting-started.md)
again, in the same weather, and keeps its height, speed and position every 5 seconds. It needs the
first page's setup, and a little Rust.

> **These numbers are not validated.** They are the first flight's, and
> [How far to trust it](getting-started.md#how-far-to-trust-it) on that page applies to them too.

## Run it

```bash
cargo run --example trajectory -p hpr-sim > trajectory.csv
```

This runs
[`crates/hpr-sim/examples/trajectory.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/trajectory.rs)
and saves what it prints in `trajectory.csv`, a CSV (comma-separated values) file: a header line
naming each column, then one line per moment of the flight. It holds this:

<!-- quote: crates/hpr-sim/examples/trajectory.output.txt -->
```text
time_s,height_above_ground_m,vertical_speed_m_s,airspeed_m_s,cg_east_m,cg_north_m,cg_up_m
0.000,0.9,0.0,5.0,0.0,0.0,0.9
0.002,0.9,0.0,5.0,0.0,0.0,0.9
0.371,3.9,16.2,17.0,0.0,0.0,3.9
3.259,216.0,110.6,111.4,-9.7,0.0,216.0
5.000,390.8,90.3,91.3,-24.1,0.0,390.8
10.000,707.8,37.5,39.4,-62.1,0.0,707.8
13.830,778.7,0.0,11.4,-88.7,0.0,778.7
14.330,777.6,-4.5,12.0,-91.9,0.0,777.6
15.000,772.6,-10.2,14.3,-95.6,0.0,772.6
20.000,665.5,-26.4,26.5,-98.4,0.1,665.5
25.000,531.1,-27.0,27.0,-78.6,0.1,531.1
30.000,396.4,-26.8,26.8,-54.4,0.1,396.4
35.000,262.6,-26.7,26.7,-29.5,0.1,262.6
39.234,150.0,-26.5,26.5,-8.4,0.1,150.0
40.000,129.7,-26.5,26.5,-4.5,0.0,129.7
40.234,123.5,-26.5,26.5,-3.3,0.0,123.5
45.000,89.0,-6.4,6.4,20.5,0.0,89.0
50.000,57.0,-6.4,6.4,45.5,0.0,57.0
55.000,25.1,-6.4,6.4,70.5,0.0,25.1
58.933,0.0,-6.4,6.4,90.2,0.0,0.0
```

Like the first flight's, this output is committed in
[`trajectory.output.txt`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/trajectory.output.txt),
and CI runs the program on macOS, Windows and Linux and fails if it prints anything else.

## Reading the table

| column | what it is | unit |
|---|---|---|
| `time_s` | Time since the motor lit | s |
| `height_above_ground_m` | The height of the [centre of gravity](glossary.md#centre-of-gravity-cg) above the launch pad | m |
| `vertical_speed_m_s` | How fast that height changes: positive going up, negative coming down | m/s |
| `airspeed_m_s` | The rocket's speed through the air, which the wind adds to or takes from | m/s |
| `cg_east_m`, `cg_north_m`, `cg_up_m` | Where the centre of gravity is: metres east, north and up of the pad, in the [launch frame](glossary.md#launch-frame-enu) | m |

There is a row every 5 seconds, and one at every [event](glossary.md#event), at the moment it
happened:

| time (s) | event |
|---|---|
| 0.002 | [liftoff](glossary.md#liftoff): the push up the rail first beats the weight, and the rocket starts to move |
| 0.371 | the rocket leaves the 3 m rail |
| 3.259 | burnout |
| 13.830 | apogee; the drogue's charge fires at the same moment, so it shares this row |
| 14.330 | the drogue opens, half a second later |
| 39.234 | the main's charge fires, as the rocket falls past 150 m |
| 40.234 | the main opens, a second later |
| 58.933 | landing |

What the numbers show:

- **On the pad the airspeed is 5.0 m/s** with the rocket standing still: that is the wind.
- **The rocket climbs into the wind.** The wind blows from the west, and off the rail a stable
  rocket turns its nose toward the wind it feels
  ([weathercocking](glossary.md#weathercocking)), so it drifts west: `cg_east_m` is −88.7 m at
  apogee.
- **Under the drogue alone it falls at about 27 m/s**, and the wind carries it east.
- **The main slows it from 26.5 to 6.4 m/s** between the row where it opens (40.234) and the
  next (45.000), and it lands at 6.4 m/s, 90.2 m east of the pad.
- **`cg_north_m` shows 0.1 m for a while, with no wind from the south.** The Earth's rotation
  nudges a moving rocket sideways, to the right of its motion in the northern hemisphere
  ([Coriolis acceleration](glossary.md#coriolis-acceleration)). While the rocket moves west, that
  is north: it drifts up to 6.3 cm, which the table rounds to 0.0 or 0.1 m.
- **`cg_up_m` and `height_above_ground_m` agree here, and don't in general.** The launch frame is
  a flat plane, and the Earth curves away below it, so far from the pad `cg_up_m` reads low: on
  the ground 10 km from the pad, it is −7.8 m. At this landing, 94 m out, it is under a
  millimetre low. For heights, use `height_above_ground_m`.

## Record something else

The program below makes its recorder with
`Recorder::new(vec![Channel::Time, Channel::HeightAboveGround, ...], Some(5.0))`, and passes it to
`run`, which feeds it every step of the flight. Change either argument:

- **The interval** is `Some(5.0)`, a row every 5 s. `Some(0.1)` gives a smooth plot, about 600
  rows for this flight. `None` keeps a row at the end of every step the integrator takes, which is
  as fine as the flight was computed ([Time integration](physics/integration.md)).
- **The channels** are the columns. Others include the Mach number, the
  [angle of attack](glossary.md#angle-of-attack), the [dynamic pressure](glossary.md#dynamic-pressure),
  the thrust, the mass, the attitude and the rotation rates. The
  [`Channel` page](api/hpr_sim/recorder/enum.Channel.html) of the API reference lists every one
  with its unit, and `Channel::ALL` keeps them all.
- **After the flight,** `recorder.columns()` gives the column names, units included, and
  `recorder.rows()` gives one list of numbers per row, in the same order.

To save a file straight from Rust instead of printing, write the same lines to a
`std::fs::File`.

## The program

This is the whole program, line for line the file CI runs. Everything above the recorder is the
first flight's setup, which [Getting started](getting-started.md#the-program-step-by-step)
explains step by step.

<!-- quote: crates/hpr-sim/examples/trajectory.rs -->
```rust
//! A trajectory to plot: the flight of `first_flight.rs`, recorded every 5 s and at every event,
//! printed as CSV (comma-separated values), which a spreadsheet or a plotting tool reads.
//!
//! Run it from anywhere in the repository, and save what it prints to a file:
//!
//! ```text
//! cargo run --example trajectory -p hpr-sim > trajectory.csv
//! ```
//!
//! The documentation site's *Recording a trajectory* page (`docs/recording-a-trajectory.md`)
//! walks through it. What it prints is kept next to it in `trajectory.output.txt`, and CI checks
//! that the two still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::{
    CanopyType, Channel, Device, DeviceDrag, Environment, FlightSettings, Rail, Recorder,
    Simulation, Termination, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The first flight: Valetudo on a K400C, from a 3 m vertical rail in New Mexico, in 5 m/s of
    // wind from the west, with a drogue at apogee and a main at 150 m.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let environment =
        Environment::standard(site)?.with_wind(ConstantWind::new(5.0, 270_f64.to_radians())?);
    let simulation = Simulation::new(
        &rocket,
        "example",
        environment,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )?
    .with_recovery(vec![
        Device::new(
            "drogue",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.6),
            Trigger::Apogee,
        )
        .with_lag_s(0.5),
        Device::new(
            "main",
            DeviceDrag::canopy(CanopyType::FlatCircular, 2.4),
            Trigger::Altitude {
                height_above_ground_m: 150.0,
            },
        )
        .with_lag_s(1.0),
    ])?;

    // What to keep, and how often: the time, the height above the pad, the vertical speed, the
    // airspeed and where the centre of gravity is (metres east, north and up of the pad), every
    // 5 s. A recorder also keeps a row at every event, such as burnout or a parachute opening.
    // For a smooth plot, use `Some(0.1)`; `None` keeps a row at the end of every step.
    let mut recorder = Recorder::new(
        vec![
            Channel::Time,
            Channel::HeightAboveGround,
            Channel::VerticalSpeed,
            Channel::Airspeed,
            Channel::CgPosition,
        ],
        Some(5.0),
    )?;
    let flight = simulation.run(&mut recorder)?;
    if flight.termination != Termination::GroundHit {
        return Err(format!("the flight ended with {:?}", flight.termination).into());
    }

    // One header line with each column's name and unit, then one line per row: the time to
    // 0.001 s, the rest to 0.1.
    println!("{}", recorder.columns().join(","));
    for row in recorder.rows() {
        let fields: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(column, &value)| rounded(value, if column == 0 { 3 } else { 1 }))
            .collect();
        println!("{}", fields.join(","));
    }
    Ok(())
}

/// `value` to `decimals` places, without the minus sign of a value that rounds to zero. Some values
/// that are zero in principle come out a hair below it: the vertical speed at ignition and at
/// apogee, the landing height, which is found just below the ground, and the landing's `cg_up_m`,
/// under a millimetre below the pad's level because the ground curves away.
fn rounded(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    match text.strip_prefix('-') {
        Some(unsigned) if unsigned.chars().all(|c| c == '0' || c == '.') => unsigned.to_owned(),
        _ => text,
    }
}
```
