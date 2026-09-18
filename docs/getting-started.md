# Getting started

This page takes you from nothing to a first simulated flight: install Rust, build hpr-sim, and run
a short program that flies a rocket from the launch rail to the ground and prints what happened.
Then it walks through that program, so you can change it and fly your own variations. It needs
some Rust, but no knowledge of this project.

> **The numbers this example prints are not validated.** No whole flight from hpr has yet been
> compared with another simulator or with a real flight; [How far to trust it](#how-far-to-trust-it)
> below says what that means for this one.

## What you need

- **Rust**, through [rustup](https://rustup.rs). The repository pins the version it is built with
  (in `rust-toolchain.toml`), and rustup installs that version the first time you build.
- **Git**, to fetch the code.
- **A network connection the first time**, to download Rust and the libraries hpr-sim uses. After
  that, everything here works offline, on macOS, Windows and Linux.

Nothing else: no Python, Java or other simulator.

## Build it and fly

```bash
git clone https://github.com/nrdptel/hpr-sim.git
cd hpr-sim
cargo run --example first_flight -p hpr-sim
```

The first run compiles hpr-sim and its libraries, which takes a few minutes; later runs start at
once. The last command runs the program
[`crates/hpr-sim/examples/first_flight.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/first_flight.rs),
which prints this:

<!-- quote: crates/hpr-sim/examples/first_flight.output.txt -->
```text
Valetudo on a K400C: 9.67 kg at liftoff, a 3 m rail, 5 m/s of wind from 270°
Not yet validated: see the Accuracy page before trusting these numbers.

event                time (s)   CG height (m)   speed (m/s)
liftoff                  0.00             0.9           0.0
rail exit                0.36             3.9          16.7
burnout                  3.26           229.6         119.6
apogee                  14.55           874.0           6.5
drogue charge fires     14.55           874.0           6.5
drogue opens            15.05           872.8           7.6
main charge fires       43.43           150.0          27.0
main opens              44.43           123.5          27.0
landing                 63.13             0.0           8.1

Apogee:     874.0 m (2867 ft) above the pad, 95.7 m from it at a bearing of 270°, at 14.55 s
Top speed:  120.3 m/s (Mach 0.36), at 3.1 s
Rail exit:  16.7 m/s
Landing:    100.4 m from the pad at a bearing of 90°, falling at 6.4 m/s, at 63.13 s
```

You should see exactly these numbers. The project's automated checks (CI, for continuous
integration) run the program on macOS, Windows and Linux on every change, and fail if it prints
anything but what is committed in
[`first_flight.output.txt`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/first_flight.output.txt),
the file quoted above.

## What it printed

The rocket is Valetudo, which Projeto Jupiter, a student team at the University of São Paulo, flew
in 2019. [RocketPy](glossary.md#rocketpy), another open-source simulator, uses it as an example, and
hpr's copy of it is one of the project's [example rockets](glossary.md#example-rockets).

Its motor is not a real K400C. It keeps the size and mass of the motor in RocketPy's example, and
takes the thrust curve of a K400C, a commercial motor
([motor designation](glossary.md#motor-designation)) whose curve hpr bundles, in place of the
original's. The rocket launches from a 3 m vertical rail, in a 5 m/s wind that blows from the west
at every height, and comes down on a [drogue and a main](glossary.md#drogue-and-main) parachute.

The table lists the flight's [events](glossary.md#event) in order:

| column | meaning |
|---|---|
| time | seconds since the motor ignited |
| CG height | the height of the rocket's [centre of gravity](glossary.md#centre-of-gravity-cg) above the launch pad. It starts at 0.9 m, not 0: the rocket stands on the rail with its aft end at the rail's foot, and its centre of gravity is 0.9 m above that |
| speed | the centre of gravity's speed over the ground, not through the air |

And the events:

| event | what happens |
|---|---|
| liftoff | the push up the rail first beats the weight and the rail's friction (zero here): the rocket starts to move ([liftoff](glossary.md#liftoff)) |
| rail exit | the last rail button, one of the small studs that slide in the rail's slot, leaves the top of the rail, and the rocket flies free ([rail exit](glossary.md#rail-exit-and-rail-exit-velocity)) |
| burnout | the motor's thrust curve ends ([burnout](glossary.md#burnout)) |
| apogee | the highest point, where the rocket stops climbing ([apogee](glossary.md#apogee)) |
| drogue charge fires | the drogue is set to fire at apogee |
| drogue opens | half a second later, the drogue's lines are stretched and it takes effect ([deployment](glossary.md#deployment)) |
| main charge fires | the rocket falls past 150 m above the pad, the main's setting |
| main opens | a second later, the main takes effect |
| landing | the centre of gravity reaches the ground |

Liftoff comes a millisecond after ignition, when the thrust is only 73 N against 95 N of weight.
Most of the rest of the push, 21 N, comes from the propellant's
[internal momentum](glossary.md#internal-momentum): the propellant and gas moving inside the motor
as it burns.

- A thrust curve measured on a test stand already includes that effect, and hpr's equations of
  motion add it again, so it is counted twice.
- hpr does this on purpose, as RocketPy does, so that the two codes can be compared like for like.
  It is a known approximation, listed with the other gaps on
  [Start here](start-here.md#what-doesnt-work-yet).
- It is small here: it changes the burnout speed by at most 0.05 m/s
  ([Rigid-body flight](physics/flight.md#equations-of-motion)).

The speed is over the ground, so it includes the drift. At apogee the rocket is still moving
sideways at 6.5 m/s. At landing it falls at 6.4 m/s while the 5 m/s wind carries it east, 8.1 m/s
in all (√(6.4² + 5²) ≈ 8.1).

Below the table:

- **Apogee** is the highest point, in metres and feet, and where it was: 95.7 m from the pad at a
  [bearing](glossary.md#bearing) of 270°. A bearing is a direction clockwise from north, so 270°
  is due west.
- **Top speed** is the fastest the rocket went, over the ground, with its
  [Mach number](glossary.md#mach-number) (its speed through the air as a fraction of the speed of
  sound). It comes just before burnout, once the dwindling thrust no longer beats the drag and the
  weight.
- **Rail exit** is the speed as the rocket leaves the rail. The slower it is in a crosswind, the
  larger the [angle of attack](glossary.md#angle-of-attack) just after it, and that is where hpr's
  models are least trustworthy (see
  [what is left out](how-a-flight-is-simulated.md#what-is-left-out)). hpr sets no minimum
  rail-exit speed and doesn't judge whether this one is enough; that call is your range safety
  officer's.
- **Landing** is where the rocket came down, 100.4 m due east of the pad, and how fast it was
  falling. The rocket [weathercocks](glossary.md#weathercocking): it turns into the wind as it
  climbs, so its apogee is west of the pad. It then drifts east under its parachutes, past the
  pad.

## How far to trust it

- **No whole flight has been validated.** hpr's apogee, top speed and landing point have not yet
  been compared with another simulator's or a real flight's. Comparing whole flights with
  RocketPy's is the next validation milestone, [M2.1b2](decisions-and-roadmap.md#m2-1b2); [Accuracy](accuracy.md) keeps
  every result so far.
- **The drag is the largest doubt, and it moves the apogee by up to a tenth.** hpr computes the
  [drag coefficient](glossary.md#drag-coefficient) from the rocket's shape and surface. For this
  design it is 0.5566 at Mach 0.3, coasting with the motor burnt out
  ([power-off drag](glossary.md#power-on-and-power-off-drag)), with a mirror-smooth surface finish
  (0 µm of roughness; a rougher surface adds skin-friction drag) and rail buttons.
  - That is 23.5% under the 0.728 in the rocket's own [OpenRocket](glossary.md#openrocket) file.
  - With that file's rougher finish (60 µm) and its two launch lugs (short tubes on the outside of
    the body that ride along the rail, like rail buttons), hpr gives 0.714, 1.9% under it.
  - The drag curve in RocketPy's example says 1.05, which the comparison leaves unexplained
    ([Aerodynamics](physics/aero.md#drag-verification)).

  A second program,
  [`drag_what_if.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/drag_what_if.rs),
  flies the same rocket with each of the other two values in place of hpr's drag, held at every
  Mach number, and prints:

  <!-- quote: crates/hpr-sim/examples/drag_what_if.output.txt -->
  ```text
  Valetudo's apogee under three drag models, from a 3 m rail in 5 m/s of wind

  drag coefficient                             apogee (m)
  hpr's own, from the design                       874.0
  0.728, from the rocket's OpenRocket file         842.2
  1.05, from RocketPy's example curve              790.9
  ```

  So 874 m is this design's answer, and with more drag the same design would peak lower. The
  rocket that flew had a different motor, so none of these is a prediction of its flight.
- **The parachutes are simple.** Each opens fully the moment its lines stretch, with no
  [filling time](glossary.md#inflation-and-filling-time) and no drag overshoot (the canopy's drag
  briefly rising above its steady value as it fills). So the opening load hpr reports is no safe
  bound either way. Each canopy's drag coefficient is the middle of the range printed for its type
  in Knacke's *Parachute Recovery Systems Design Manual*, a standard handbook
  ([Recovery](physics/recovery.md)).
- **The descent is the best-checked part.** From the same start near apogee, with the first
  parachute opening at once and RocketPy's formula for gravity, hpr's descent agrees with
  RocketPy's within 3% for five rockets ([Recovery](physics/recovery.md)). This example opens the
  drogue after half a second and uses hpr's own gravity, and its landing point also depends on
  where the apogee is, which no comparison has checked yet.

## The program, step by step

This is the whole program. It is also the file CI runs, line for line.

<!-- quote: crates/hpr-sim/examples/first_flight.rs -->
```rust
//! A first flight: a rocket with a drogue and a main parachute, flown from the launch rail to the
//! ground, with a summary of what happened.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example first_flight -p hpr-sim
//! ```
//!
//! The documentation site's *Getting started* page (`docs/getting-started.md`) walks through it.
//! What it prints is kept next to it in `first_flight.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_atmos::ConstantWind;
use hpr_core::DVec3;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::{
    CanopyType, Device, DeviceDrag, Environment, EventKind, FlightSettings, FlightStep, Observer,
    Rail, SimError, Simulation, Termination, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket: Valetudo, which Projeto Jupiter (University of São Paulo) flew in 2019 and
    // RocketPy uses as an example. The design file describes its parts, materials and motor; the
    // motor keeps the example's size and mass, with a bundled K400C thrust curve.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))?;

    // Where and in what weather: a site in New Mexico 1,400 m up, the 1976 US Standard
    // Atmosphere, and a steady wind from the west (270°) at every height.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let wind_speed_m_s = 5.0;
    let wind_from_deg = 270.0_f64;
    let environment = Environment::standard(site)?.with_wind(ConstantWind::new(
        wind_speed_m_s,
        wind_from_deg.to_radians(),
    )?);

    // A vertical launch rail.
    let rail_length_m = 3.0;
    let rail = Rail::vertical(rail_length_m);

    // The flight: the design's configuration "example" (its K400C), the default settings, and
    // dual deployment. The drogue opens half a second after apogee; the main opens a second after
    // the rocket falls past 150 m above the ground, and the drogue stays attached.
    let simulation = Simulation::new(
        &rocket,
        "example",
        environment,
        rail,
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

    // Fly it. `TopSpeed` (below) watches every step of the flight for its fastest moment.
    let mut top = TopSpeed::default();
    let flight = simulation.run(&mut top)?;
    if flight.termination != Termination::GroundHit {
        return Err(format!("the flight ended with {:?}", flight.termination).into());
    }

    let liftoff = flight
        .event(EventKind::Liftoff)
        .ok_or("the rocket never lifted off")?
        .sample;
    println!(
        "Valetudo on a K400C: {:.2} kg at liftoff, a {rail_length_m} m rail, {wind_speed_m_s} m/s \
         of wind from {wind_from_deg}°",
        liftoff.mass_kg
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("event                time (s)   CG height (m)   speed (m/s)");
    for event in &flight.events {
        let name = match event.kind {
            EventKind::Liftoff => "liftoff".to_owned(),
            EventKind::RailExit => "rail exit".to_owned(),
            EventKind::Burnout => "burnout".to_owned(),
            EventKind::Apogee => "apogee".to_owned(),
            EventKind::Trigger(i) => format!("{} charge fires", simulation.recovery()[i].name),
            EventKind::Deployment(i) => format!("{} opens", simulation.recovery()[i].name),
            EventKind::GroundHit => "landing".to_owned(),
            other => format!("{other:?}"),
        };
        let sample = &event.sample;
        // Heights are the centre of gravity's above the pad. The flight ends where it reaches the
        // ground, found to within a micrometre just below it: print that as 0 (`+ 0.0` turns a
        // -0 into 0).
        println!(
            "{name:<20} {:>8.2} {:>15.1} {:>13.1}",
            sample.time_s,
            sample.height_above_ground_m.max(0.0) + 0.0,
            sample.cg_velocity_enu_m_s.length(),
        );
    }

    let apogee = flight
        .event(EventKind::Apogee)
        .ok_or("the flight has no apogee")?
        .sample;
    let rail_exit = flight
        .event(EventKind::RailExit)
        .ok_or("the flight never left the rail")?
        .sample;
    let landing = flight.final_sample;
    let (apogee_distance_m, apogee_bearing_deg) = from_pad(apogee.cg_enu_m);
    let (landing_distance_m, landing_bearing_deg) = from_pad(landing.cg_enu_m);

    println!();
    println!(
        "Apogee:     {:.1} m ({:.0} ft) above the pad, {apogee_distance_m:.1} m from it at a \
         bearing of {apogee_bearing_deg:.0}°, at {:.2} s",
        apogee.height_above_ground_m,
        apogee.height_above_ground_m / METRES_PER_FOOT,
        apogee.time_s,
    );
    println!(
        "Top speed:  {:.1} m/s (Mach {:.2}), at {:.1} s",
        top.speed_m_s, top.mach, top.time_s,
    );
    println!(
        "Rail exit:  {:.1} m/s",
        rail_exit.cg_velocity_enu_m_s.length()
    );
    println!(
        "Landing:    {landing_distance_m:.1} m from the pad at a bearing of \
         {landing_bearing_deg:.0}°, falling at {:.1} m/s, at {:.2} s",
        -landing.vertical_speed_m_s, landing.time_s,
    );
    Ok(())
}

/// The international foot.
const METRES_PER_FOOT: f64 = 0.3048;

/// Where `position` is from the pad: its distance over the ground, m, and its bearing, clockwise
/// from north, degrees. The launch frame's axes point east, north and up from the pad
/// (`docs/physics/frames.md`).
fn from_pad(position: DVec3) -> (f64, f64) {
    let (east_m, north_m) = (position.x, position.y);
    let bearing_deg = east_m.atan2(north_m).to_degrees().rem_euclid(360.0);
    (east_m.hypot(north_m), bearing_deg)
}

/// The fastest moment of a flight, relative to the ground.
///
/// An [`Observer`] sees every accepted step of the integration as the flight runs. This one looks
/// at the end of each step. Looking at 200 points inside every step instead raises the top speed
/// by 0.005 m/s, and moves its time by 0.013 s, since the speed is flat near its peak. So the
/// speed can read low by one in its last printed digit, and the time is printed to 0.1 s.
#[derive(Debug, Default)]
struct TopSpeed {
    speed_m_s: f64,
    mach: f64,
    time_s: f64,
}

impl Observer for TopSpeed {
    fn step(&mut self, step: &dyn FlightStep) -> Result<(), SimError> {
        let sample = step.sample(step.end_s())?;
        let speed_m_s = sample.cg_velocity_enu_m_s.length();
        if speed_m_s > self.speed_m_s {
            *self = Self {
                speed_m_s,
                mach: sample.mach,
                time_s: sample.time_s,
            };
        }
        Ok(())
    }
}
```

It has six steps.

1. **The rocket.** The program reads Valetudo's design from
   [`validation/designs/rocketpy-valetudo.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/designs/rocketpy-valetudo.json)
   into a `Rocket`: its parts, their shapes, materials and positions, and its motor. The format is
   hpr's own and provisional: the open design format ([M3.3](decisions-and-roadmap.md#m3-3)) will replace it, and import
   from OpenRocket ([M3.1](decisions-and-roadmap.md#m3-1)) comes later. The
   [designs folder](https://github.com/nrdptel/hpr-sim/tree/main/validation/designs) holds the other
   example rockets, and its README says how each was built.
2. **The surroundings.** `Geodetic::from_degrees` places the launch site by latitude, longitude
   and height. `Environment::standard` gives it the Earth's
   [WGS 84 gravity and rotation](physics/gravity.md), the
   [1976 US Standard Atmosphere](physics/atmosphere.md) and no wind. `with_wind` adds a
   [constant wind](physics/wind.md): `wind_speed_m_s`, 5 m/s, *from* `wind_from_deg`, 270°,
   clockwise from north (see [wind direction](glossary.md#wind-direction)). Angles in hpr are in
   radians, hence `to_radians()`.
3. **The rail.** `Rail::vertical(rail_length_m)` is a rail 3 m long pointing straight up, with no
   friction. Its fields also set its heading, its angle above the horizon and its friction.
4. **The simulation.** `Simulation::new` takes the rocket, the name of the configuration to fly
   (a design can hold several, one per motor choice), the surroundings, the rail and the
   integration settings, which say how finely to step through time. It runs the design's checks
   first, and refuses a rocket that can't exist, such as a motor wider than its mount.
   `FlightSettings::default()` steps through time to a tight
   [tolerance](glossary.md#tolerance) ([Time integration](physics/integration.md)).
5. **The parachutes.** Each `Device` has a name, a drag, and a trigger that fires its charge.
   `DeviceDrag::canopy` is a parachute of the given type and
   [nominal diameter](glossary.md#nominal-area) in metres: 0.6 m for the drogue and 2.4 m for the
   main.
   - `CanopyType::FlatCircular` is a flat circular canopy. It is one of thirteen canopy types,
     such as conical, hemispherical, cross and ringslot, each with drag data from Knacke's
     parachute handbook. The [`CanopyType` page](api/hpr_sim/recovery/enum.CanopyType.html) of
     the API reference lists them all, and [Recovery](physics/recovery.md#drag-area) explains the
     data.
   - `Trigger::Apogee` fires at apogee; `Trigger::Altitude` fires when the rocket falls past a
     height above the pad.
   - `with_lag_s` is the time from the charge to the lines stretching. With no filling law given,
     each parachute opens fully at once.
6. **The flight.** `run` flies the rocket until it lands. It returns a
   [`FlightResult`](api/hpr_sim/flight/struct.FlightResult.html), the record of the finished
   flight: how it ended (`termination`), its `events`, each with a `Sample` of the flight at that
   moment, and the final sample.
   - The argument to `run` is an [`Observer`](api/hpr_sim/recorder/trait.Observer.html): any
     type that is shown every step of the flight as it runs. `TopSpeed`, at the bottom of the
     program, is one: it keeps the fastest moment.
   - For a whole trajectory, to plot or save, pass a `Recorder` instead. It keeps the quantities
     you choose, each a `Channel`, such as the height or the Mach number, as a table.
     [Recording a trajectory](recording-a-trajectory.md) shows one, lists the channels, and shows
     what it prints.

A [`Sample`](api/hpr_sim/recorder/struct.Sample.html) is a snapshot of the flight at one instant.
It holds what the program prints, and more: the time, the centre of gravity's position
(`cg_enu_m`, metres east, north and up of the pad) and velocity (`cg_velocity_enu_m_s`), the
height above the pad, the vertical speed, the airspeed, the Mach number, the angle of attack, the
thrust and the mass. Every quantity is in SI units, and its name says which.

## Change it

Edit the program and run it again. The edits below are for you to try. Each says which way the
results move; run it to see by how much. Nothing checks these edits or what they print, so this
page gives no numbers for them. For example:

- **More wind.** Change `let wind_speed_m_s = 5.0;` to `10.0`. The apogee drops a little and the
  landing moves farther east.
- **Angle the rail into the wind.** Replace `Rail::vertical(rail_length_m)` with
  `Rail { elevation_rad: 85_f64.to_radians(), azimuth_rad: 270_f64.to_radians(), ..Rail::vertical(rail_length_m) }`,
  a rail tilted 5° toward the west. The rocket now lands west of the pad, upwind.
- **Open the main higher.** Change `height_above_ground_m: 150.0` to `300.0`. The rocket spends
  longer under its main and lands farther downwind.
- **Drop the drogue.** Delete the first `Device`, the drogue. The rocket now falls at over 100 m/s
  until the main opens, far faster than a real parachute survives. hpr reports the violent
  deceleration as it opens, but it has no model of a parachute or its harness failing, so the
  flight still ends in a gentle landing ([Recovery](physics/recovery.md)).

Your edited program no longer prints what `first_flight.output.txt` says, which is expected.

If you propose a change to the program itself in a pull request, two more commands help.
`cargo xtask` runs the project's own maintenance tasks, a Rust program in the repository's
[`xtask` folder](https://github.com/nrdptel/hpr-sim/tree/main/xtask):

- `cargo xtask examples` runs every example program and writes its new output next to it;
- `cargo xtask site` builds this documentation site, and checks that the quotes on this page still
  match the program and its output.

## Where next

- [Recording a trajectory](recording-a-trajectory.md) keeps the whole flight as a table, to plot
  or save.
- [How a flight is simulated](how-a-flight-is-simulated.md) explains what happens between ignition
  and landing, and links the page for each model.
- [Your own rocket](your-own-rocket.md) builds a rocket of your own, with your dimensions and a
  bundled motor, and shows its centre of pressure, centre of gravity and
  [stability margin](glossary.md#stability-margin). A
  simpler builder ([M4.1](decisions-and-roadmap.md#m4-1), the simpler library interface) and OpenRocket import
  ([M3.1](decisions-and-roadmap.md#m3-1)) are planned.
- [Accuracy](accuracy.md) gathers every validation result, and
  [Checking a claim](checking-a-claim.md) shows how to trace a number to its source and its test.
- [The API reference](api.md) documents every type used here, and
  `cargo doc --open -p hpr-sim` builds it on your machine.

