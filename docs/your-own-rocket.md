# Your own rocket

This page builds a rocket of your own in Rust, part by part: your dimensions, your materials, and a
motor from the catalog that comes with hpr-sim. It then finds the rocket's
[centre of gravity](glossary.md#centre-of-gravity-cg) (CG), its
[centre of pressure](glossary.md#centre-of-pressure-cp) (CP) and its
[stability margin](glossary.md#stability-margin), flies it, and turns it into a
[design file](glossary.md#design-file) and back. It needs the setup from
[Getting started](getting-started.md), and some Rust.

> **How far to trust it.** The CP comes from [Barrowman's method](glossary.md#barrowmans-method),
> which hpr checks against Barrowman's own worked examples at Mach 0. hpr's CP agrees with all five
> within 1%; for one of them, a six-fin rocket, hpr's
> [normal-force slope](glossary.md#normal-force-slope) is 2.87% high
> ([Aerodynamics](physics/aero.md#verification)). The mass and CG come from each part's shape and a
> published density, so glue, paint and hardware are missing until you weigh the parts and enter
> the weights ([What else a design can hold](#what-else-a-design-can-hold)). The flight is not
> validated: no whole flight from hpr has yet been compared with another simulator or a real flight
> ([Getting started](getting-started.md#how-far-to-trust-it)).

## Run it

```bash
cargo run --example own_rocket -p hpr-sim
```

This runs
[`crates/hpr-sim/examples/own_rocket.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/own_rocket.rs),
which prints this:

<!-- quote: crates/hpr-sim/examples/own_rocket.output.txt -->
```text
My 54 mm rocket (Cesaroni 168H54-10A): 1.120 m long, 56.3 mm across
Not yet validated: see the Accuracy page before trusting these numbers.

                                   liftoff   burnout
mass (kg)                            0.675     0.579
centre of gravity (m from nose)      0.671     0.610
centre of pressure (m from nose)     0.779     0.779
stability margin (calibres)           1.92      2.99

Normal-force slope (per radian) and centre of pressure, at Mach 0.3:
nose        2.00 at 0.102 m
fins        4.82 at 1.060 m
rocket      6.82 at 0.779 m

From a 1.8 m vertical rail, with no wind:
Rail exit:  21.7 m/s
Apogee:     1144.5 m above the pad, at 13.92 s
Top speed:  187 m/s (Mach 0.56)
Ejection:   at 13.50 s, at 5.5 m/s

The fin set, as the design file stores it:
{
  "id": "fins",
  "name": "",
  "part": {
    "fin_set": {
      "count": 3,
      "planform": {
        "kind": "trapezoidal",
        "root_chord_m": 0.1,
        "tip_chord_m": 0.04,
        "span_m": 0.045,
        "sweep_m": 0.05
      },
      "thickness_m": 0.003175,
      "cross_section": "rounded",
      "tab": null,
      "cant_rad": 0.0,
      "base_angle_rad": 0.0,
      "material": {
        "name": "Birch plywood",
        "density": {
          "kind": "bulk",
          "kg_m3": 680.0
        }
      }
    }
  },
  "position": {
    "from": "bottom",
    "aft_offset_m": 0.0
  }
}
```

Like the first flight's in [Getting started](getting-started.md), this output is committed in
[`own_rocket.output.txt`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-sim/examples/own_rocket.output.txt),
and CI (the project's automated checks) runs the program on macOS, Windows and Linux and fails if
it prints anything else.

To fly your own rocket, change the numbers in the program and run it again. Or copy it to a new
file in the same folder, say `my_rocket.rs`, and run that with
`cargo run --example my_rocket -p hpr-sim`.

## The rocket

The rocket has a 54 mm airframe (the tube's inside diameter) and flies on a Cesaroni H54, a 29 mm
reloadable motor (a propellant load for a reusable case). These are the program's inputs:

| part | Rust type | in the program |
|---|---|---|
| nose cone | [`NoseCone`](api/hpr_design/parts/struct.NoseCone.html) | a [tangent ogive](glossary.md#tangent-ogive) ([Shapes](physics/shapes.md)) 0.22 m long, of ABS with a 1.5 mm wall, and a 6 cm shoulder that slides into the tube |
| airframe | [`BodyTube`](api/hpr_design/parts/struct.BodyTube.html) | 0.9 m of kraft phenolic tube, outer radius 0.02815 m (56.3 mm across), 1.15 mm wall |
| motor mount | [`InnerTube`](api/hpr_design/parts/struct.InnerTube.html) | 0.2 m long, outer radius 0.0155 m, 1 mm wall, so a 29 mm bore; flush with the airframe's aft end, with the nozzle 5 mm past it |
| fins | [`FinSet`](api/hpr_design/fins/struct.FinSet.html) | three trapezoidal fins of 1/8 in (3.175 mm) birch plywood with rounded edges: root chord 0.1 m, tip chord 0.04 m, span 0.045 m, and the tip's leading edge 0.05 m aft of the root's |
| recovery bay | [`MassComponent`](api/hpr_design/parts/struct.MassComponent.html) | 200 g standing in for the parachute, shock cord and altimeter, packed as a cylinder 0.15 m long and 50 mm across ([packing](#packing)), 7 cm below the airframe's top |
| motor | [`MountedMotor`](api/hpr_design/config/struct.MountedMotor.html) | the Cesaroni 168H54-10A from the bundled catalog, with a 10 s [ejection delay](glossary.md#ejection-delay) |

Every size is in metres, and every round part takes a **radius**, not a diameter: halve the
diameters on your drawings.

## What it printed

### Mass, centre of gravity and centre of pressure

The table has two columns. **liftoff** is the rocket on the pad, with the motor full.
**burnout** is the rocket with the motor spent: its case and nozzle are still aboard, its
propellant is gone.

- **Mass:** 0.675 kg on the pad and 0.579 kg at burnout. The 0.096 kg between them is the
  propellant; the catalog lists 96.6 g for this motor.
- **Centre of gravity:** where the mass balances, as a [station](glossary.md#station): metres
  aft of the nose tip. It moves forward, from 0.671 m to 0.610 m, as the propellant in the tail
  burns away.
- **Centre of pressure:** where the air's sideways push acts, 0.779 m aft of the tip. It depends on
  the rocket's shape and speed, not its mass, so it is the same in both columns. Both use
  [Mach](glossary.md#mach-number) 0.3, with the air straight along the rocket's axis.
- **Stability margin:** how far the CP lies behind the CG, in
  [calibres](glossary.md#calibre-caliber), that is, in body diameters. At liftoff it is
  (0.779 − 0.671) m ÷ 0.0563 m ≈ 1.9; the program works from the unrounded values and prints 1.92.
  At burnout the CG has moved forward, so the margin has grown to 2.99. This program works the
  margin out by hand; hpr also gives it from the rail exit to apogee or the first deployment, with an optimum ejection
  delay, as [Flight metrics](physics/metrics.md) shows.

A positive margin means that when something tips the rocket, the air turns its nose back into the
oncoming air. That oncoming air is the *relative wind*: the airflow the rocket feels, from its
motion over the ground combined with the wind. In a crosswind, the same turn swings the rocket upwind
([weathercocking](glossary.md#weathercocking)).

hpr doesn't judge whether a margin is enough; your club's or range's rules do.

### Where the centre of pressure comes from

Barrowman's method works out each nose cone, transition and fin set on its own. Each gets a CP and
a normal-force slope: how fast its sideways push grows with the
[angle of attack](glossary.md#angle-of-attack), per radian. A plain body tube's slope is zero, so
it has no line of its own. (A tube's sideways push appears only at larger angles, so it adds
nothing to the slope.)
The rocket's CP is the average of the parts' CPs, each weighted by its slope:

| part | slope (per radian) | CP (m from the nose tip) | slope × CP |
|---|---|---|---|
| nose | 2.00 | 0.102 | 0.204 |
| fins | 4.82 | 1.060 | 5.109 |
| rocket | 6.82 | | 5.313 |

So the CP is 5.313 ÷ 6.82 ≈ 0.779 m. The fins sit far aft and have more than twice the nose's
slope, so they pull the CP toward the tail. Moving the CP aft (bigger fins, or fins farther aft)
or the CG forward (a heavier nose) raises the margin; run the program to see by how much.

**How speed moves the CP.** In hpr, only a fin set's terms change with the Mach number: its slope
grows as the rocket speeds up toward Mach 1, and from Mach 0.8 its own CP moves aft too. The
slopes and CPs of nose cones, transitions and body tubes stay where they are
([Aerodynamics](physics/aero.md#your-rockets-centre-of-pressure)).

- With the fins at the tail, as here, the growing fin slope pulls the rocket's CP aft as it speeds
  up.
- A rocket with canards (a second fin set near the nose) is different. The canards' slope grows
  too and pulls the CP forward, so which way the CP moves depends on both fin sets.
- hpr keeps each fin set's CP a quarter of the way back along its
  [mean aerodynamic chord](glossary.md#mean-aerodynamic-chord-mac), a kind of average chord, up to
  Mach 0.8, and moves it aft from there toward where supersonic linear theory puts it
  ([Aerodynamics](physics/aero.md#fins-through-mach-1)). Niskanen's 2009 thesis, which hpr's
  aerodynamics also draw on, starts moving it at Mach 0.5; NASA's wind tunnel found an Arcas Robin
  rocket's CP moving forward, not aft, between Mach 0.6 and 0.8, so hpr doesn't. This rocket's top
  speed, Mach 0.56, is well below either.

`Flow::axial(0.0)` in place of `Flow::axial(0.3)` gives the low-speed value that Barrowman's method
gives by hand.

### The flight

The rocket flies from a 1.8 m vertical rail, 1,400 m up in New Mexico, with no wind. It has one
parachute, 0.9 m across ([nominal diameter](glossary.md#nominal-area)), which opens when the
motor's ejection charge fires.

- **Rail exit:** 21.7 m/s. The design has no rail buttons, so hpr takes the rocket as off the rail
  when its aft end passes the top ([rail exit](glossary.md#rail-exit-and-rail-exit-velocity)).
- **Apogee:** 1144.5 m above the pad, 13.92 s after ignition ([apogee](glossary.md#apogee)).
- **Top speed:** 187 m/s, Mach 0.56: the fastest airspeed at the end of any of the
  [time steps](glossary.md#adaptive-time-step) the flight was computed in. With no wind, the
  airspeed is also the speed over the ground.
- **Ejection:** at 13.50 s, at 5.5 m/s. The charge fires the 10 s delay after
  [burnout](glossary.md#burnout), which in hpr is the time of the thrust curve's last point, 3.50 s
  for this motor. That is 0.42 s before apogee, while the rocket is still climbing slowly.

This motor has three times near the end of its burn, and they measure different things:

| time | what it is | where it comes from |
|---|---|---|
| 3.12 s | the [burn time](glossary.md#burn-time) [ThrustCurve.org](glossary.md#thrustcurveorg) publishes for the motor | the bundled catalog, which copies ThrustCurve.org's values |
| 3.13 s | the burn time hpr works out from this motor's thrust curve, by the same [NFPA 1125](glossary.md#nfpa-1125) rule: from when the thrust first reaches 5% of its peak to when it last falls to 5% | [Solid motors](physics/motor.md#the-bundled-motors) lists it |
| 3.50 s | [burnout](glossary.md#burnout): the curve's last point, where the thrust reaches zero | the thrust curve; the ejection delay counts from here |

- The first two differ by 0.01 s. hpr bundles a motor only if its computed burn time is within 1%
  of ThrustCurve.org's ([Solid motors](physics/motor.md#the-bundled-motors)).
- From 3.13 s to 3.50 s the motor still pushes, with under 5% of its peak thrust (the curve's peak
  is 103 N, so under about 5 N). The burn time leaves that tail out; the flight doesn't.

### The fin set as a design file

The last lines are the fin set as a design file stores it, in JSON.
[As a design file](#as-a-design-file), below, explains the form.

## The program, step by step

This is the whole program, line for line the file CI runs.

<!-- quote: crates/hpr-sim/examples/own_rocket.rs -->
```rust
//! Your own rocket: a 54 mm rocket built part by part in Rust, with a motor from the bundled
//! catalog. It prints the rocket's mass, centre of gravity, centre of pressure and stability
//! margin, then flies it.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example own_rocket -p hpr-sim
//! ```
//!
//! The documentation site's *Your own rocket* page (`docs/your-own-rocket.md`) walks through it.
//! What it prints is kept next to it in `own_rocket.output.txt`, and CI checks that the two still
//! agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_aero::{AeroModel, Flow};
use hpr_core::geodesy::Geodetic;
use hpr_design::{
    AutoDimension, BodyTube, Component, Configuration, FinCrossSection, FinPlanform, FinSet,
    Ignition, InnerTube, MassComponent, Material, MotorMount, MountedMotor, NoseCone, NoseShape,
    Overrides, Packing, Part, Position, ReferenceDiameter, Rocket, Shoulder, Stage, Wall,
    materials,
};
use hpr_motor::{Catalog, Delay};
use hpr_sim::{
    CanopyType, Channel, Device, DeviceDrag, Environment, EventKind, FlightSettings, Rail,
    Recorder, Simulation, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    // The nose: a 22 cm tangent ogive of ABS with a 1.5 mm wall and a 6 cm shoulder. Its base
    // radius and its shoulder's radius are automatic: they fit the tube behind it.
    let mut nose = component(
        "nose",
        Part::NoseCone(NoseCone {
            shape: NoseShape::Ogive { radius_ratio: 1.0 },
            length_m: 0.22,
            base_radius_m: 0.0,
            wall: Wall::Shell {
                thickness_m: 0.0015,
            },
            shoulder: Some(Shoulder {
                length_m: 0.06,
                outer_radius_m: 0.0,
                thickness_m: 0.0015,
                capped: true,
            }),
            material: material("abs")?,
        }),
        None,
    );
    nose.auto = vec![AutoDimension::BaseRadius, AutoDimension::ShoulderRadius];

    // The airframe: 90 cm of kraft phenolic tube, 56.3 mm across, with a 1.15 mm wall.
    let mut airframe = component(
        "airframe",
        Part::BodyTube(BodyTube {
            length_m: 0.9,
            outer_radius_m: 0.02815,
            thickness_m: 0.00115,
            material: material("kraft_phenolic")?,
        }),
        None,
    );

    // Inside it, flush with its aft end, a 20 cm motor mount tube with a 29 mm bore. The motor's
    // nozzle will stick out 5 mm past it.
    let mut mount = component(
        "motor-mount",
        Part::InnerTube(InnerTube {
            length_m: 0.2,
            outer_radius_m: 0.0155,
            thickness_m: 0.001,
            radial_offset_m: 0.0,
            angle_rad: 0.0,
            material: material("kraft_phenolic")?,
            cluster_m: Vec::new(),
        }),
        Some(Position::Bottom { aft_offset_m: 0.0 }),
    );
    mount.motor_mount = Some(MotorMount { overhang_m: 0.005 });

    // Three trapezoidal fins of 1/8 in birch plywood, flush with the aft end.
    let fins = component(
        "fins",
        Part::FinSet(FinSet {
            count: 3,
            planform: FinPlanform::Trapezoidal {
                root_chord_m: 0.1,
                tip_chord_m: 0.04,
                span_m: 0.045,
                sweep_m: 0.05,
            },
            thickness_m: 0.003175,
            cross_section: FinCrossSection::Rounded,
            tab: None,
            cant_rad: 0.0,
            base_angle_rad: 0.0,
            material: material("birch_plywood")?,
        }),
        Some(Position::Bottom { aft_offset_m: 0.0 }),
    );

    // The parachute, shock cord and altimeter, as one 200 g mass 7 cm below the tube's top, clear
    // of the nose's shoulder. Its packing is the cylinder the mass fills: 15 cm long, 5 cm across.
    let packing = Packing {
        length_m: 0.15,
        radius_m: 0.025,
        radial_offset_m: 0.0,
        angle_rad: 0.0,
    };
    let bay = MassComponent {
        mass_kg: 0.2,
        packing,
    };
    let top = Position::Top { aft_offset_m: 0.07 };
    let bay = component("recovery-bay", Part::MassComponent(bay), Some(top));
    // The fin set as a design file stores it, to print at the end.
    let fins_json = serde_json::to_string_pretty(&fins)?;
    airframe.children = vec![mount, fins, bay];

    // The motor: a Cesaroni H54 from the bundled catalog, found by its designation, with the
    // catalog's size, masses and thrust curve, and the 10 s delay its designation names.
    let catalog = Catalog::bundled()?;
    let entry = catalog
        .find("168H54-10A")
        .next()
        .ok_or("not in the catalog")?;
    let motor = MountedMotor {
        mount: "motor-mount".to_owned(),
        designation: entry.designation.clone(),
        diameter_m: entry.diameter_mm / 1000.0,
        length_m: entry.length_mm / 1000.0,
        motor: entry.bundled_motor()?,
        delay: Some(Delay::Seconds(10.0)),
        ignition: Ignition::Launch,
        failed_tubes: Vec::new(),
    };

    // The rocket: one stage, and one configuration, "h54", with that motor in the mount.
    let rocket = Rocket {
        name: "My 54 mm rocket".to_owned(),
        stages: vec![Stage {
            id: "sustainer".to_owned(),
            name: String::new(),
            components: vec![nose, airframe],
            overrides: Overrides::default(),
        }],
        reference_diameter: ReferenceDiameter::Maximum {},
        configurations: vec![Configuration {
            id: "h54".to_owned(),
            name: String::new(),
            motors: vec![motor],
        }],
    };

    // Place the parts and the motor, and find the mass properties with the motor full and spent.
    // A point `s` metres aft of the nose tip is at z = -s in the body frame.
    let assembly = rocket.assemble("h54")?;
    let full = assembly.mass_properties(0.0);
    let spent = assembly.dry_mass_properties();
    let (cg_liftoff_m, cg_burnout_m) = (-full.cg_m.z, -spent.cg_m.z);

    // The centre of pressure by Barrowman's method, at Mach 0.3 with the air straight along the
    // axis. Barrowman's slopes are the small-angle limit, so this is the CP at small angles.
    let flow = Flow::axial(0.3);
    let aero = AeroModel::new(&assembly.layout)?;
    let total = aero.normal_force(&flow)?;
    let cp_m = total.cp_station_m.ok_or("no normal force")?;
    let calibres = |cg_m: f64| (cp_m - cg_m) / assembly.layout.reference_diameter_m;

    println!(
        "{} ({} {}): {:.3} m long, {:.1} mm across",
        rocket.name,
        entry.manufacturer_abbrev,
        entry.designation,
        assembly.layout.length_m,
        assembly.layout.reference_diameter_m * 1000.0,
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("                                   liftoff   burnout");
    let (m0, m1) = (full.mass_kg, spent.mass_kg);
    println!("mass (kg)                          {m0:>7.3} {m1:>9.3}");
    println!("centre of gravity (m from nose)    {cg_liftoff_m:>7.3} {cg_burnout_m:>9.3}");
    println!("centre of pressure (m from nose)   {cp_m:>7.3} {cp_m:>9.3}");
    let (s0, s1) = (calibres(cg_liftoff_m), calibres(cg_burnout_m));
    println!("stability margin (calibres)        {s0:>7.2} {s1:>9.2}");
    println!();
    println!("Normal-force slope (per radian) and centre of pressure, at Mach 0.3:");
    for part in aero.components(&flow)? {
        let force = part.normal_force;
        if let Some(station_m) = force.cp_station_m {
            let (id, slope) = (&part.id, force.slope_per_rad);
            println!("{id:<10} {slope:>5.2} at {station_m:.3} m");
        }
    }
    let slope = total.slope_per_rad;
    println!("{:<10} {slope:>5.2} at {cp_m:.3} m", "rocket");

    // Fly it from a 1.8 m vertical rail, 1,400 m up in New Mexico, with no wind. The parachute
    // opens when the motor's ejection charge fires, 10 s after burnout.
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0)?;
    let parachute = Device::new(
        "parachute",
        DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
        Trigger::MotorDelay { motor: 0 },
    );
    let simulation = Simulation::new(
        &rocket,
        "h54",
        Environment::standard(site)?,
        Rail::vertical(1.8),
        FlightSettings::default(),
    )?
    .with_recovery(vec![parachute])?;

    // A recorder keeps the airspeed and the Mach number at the end of every step.
    let mut recorder = Recorder::new(vec![Channel::Airspeed, Channel::Mach], None)?;
    let flight = simulation.run(&mut recorder)?;
    let rows = recorder.rows();
    let fastest = rows.iter().max_by(|a, b| a[0].total_cmp(&b[0]));
    let fastest = fastest.ok_or("no steps")?;
    let rail_exit = flight.event(EventKind::RailExit).ok_or("no rail exit")?;
    let apogee = flight.event(EventKind::Apogee).ok_or("no apogee")?;
    let ejection = flight.event(EventKind::Trigger(0)).ok_or("no ejection")?;
    let (rail_exit, apogee, ejection) = (rail_exit.sample, apogee.sample, ejection.sample);

    println!();
    println!("From a 1.8 m vertical rail, with no wind:");
    let speed_m_s = rail_exit.cg_velocity_enu_m_s.length();
    println!("Rail exit:  {speed_m_s:.1} m/s");
    let (height_m, time_s) = (apogee.height_above_ground_m, apogee.time_s);
    println!("Apogee:     {height_m:.1} m above the pad, at {time_s:.2} s");
    println!("Top speed:  {:.0} m/s (Mach {:.2})", fastest[0], fastest[1]);
    let (time_s, speed_m_s) = (ejection.time_s, ejection.cg_velocity_enu_m_s.length());
    println!("Ejection:   at {time_s:.2} s, at {speed_m_s:.1} m/s");

    // The whole rocket as a design file's text, JSON, and read back from it.
    let text = serde_json::to_string_pretty(&rocket)?;
    let read_back: Rocket = serde_json::from_str(&text)?;
    if read_back != rocket {
        return Err("the design file doesn't read back as the same rocket".into());
    }
    println!();
    println!("The fin set, as the design file stores it:");
    println!("{fins_json}");
    Ok(())
}

/// A built-in material by its id; `hpr_design::materials` lists them, each with its source.
fn material(id: &str) -> Result<Material, String> {
    materials::find(id)
        .map(|builtin| builtin.material())
        .ok_or_else(|| format!("no built-in material `{id}`"))
}

/// A node of the design tree holding `part`, placed at `position` along its parent. Body
/// components (nose cones, body tubes and transitions) have no position: they stack from the nose.
fn component(id: &str, part: Part, position: Option<Position>) -> Component {
    Component {
        id: id.to_owned(),
        name: String::new(),
        part,
        position,
        auto: Vec::new(),
        motor_mount: None,
        finish: None,
        overrides: Overrides::default(),
        overrides_include_children: false,
        children: Vec::new(),
    }
}
```

It has eight steps.

1. **The parts.** Each part is a Rust value from the `hpr_design` [crate](glossary.md#crate),
   wrapped in a [`Component`](api/hpr_design/tree/struct.Component.html): a node of the design
   tree with an id, the part, and where it sits. The `component` helper at the bottom of the
   program fills in the fields a part seldom needs: a display name, a surface finish, mass
   overrides and children.
   [`Part`](api/hpr_design/tree/enum.Part.html) lists every kind of part.
2. **Where each part sits.** Nose cones, body tubes and transitions are *body components*: they
   go in a stage's list, nose first, and stack from the nose tip aft, so they have no position.
   Every other part hangs from a body component, or from an inner tube, and has a
   [`Position`](api/hpr_design/tree/enum.Position.html): `Top`, `Middle`, `Bottom`, `After` or
   `Absolute`, with an offset in metres, positive aft ([Positions](physics/design.md#positions)).
   The mount and the fins are at `Bottom` with no offset, flush with the aft end.
   - **Automatic dimensions.** A dimension named in a component's `auto` list is taken from the
     parts around it, and the value stored for it (0 here) is ignored. The nose's base radius and
     its shoulder's radius follow the airframe
     ([Automatic dimensions](physics/design.md#automatic-dimensions)).
   - **Motor mount.** Setting `motor_mount` makes a body tube or inner tube a mount, and its
     `overhang_m` is how far the nozzle sits aft of the mount's end.
   - <a id="packing"></a>**Packing.** A
     [`MassComponent`](api/hpr_design/parts/struct.MassComponent.html) is a mass and its
     [`Packing`](api/hpr_design/parts/struct.Packing.html): the size of the solid cylinder hpr
     spreads the mass through. Here it is 0.15 m long with `radius_m` 0.025, so 50 mm across,
     inside the airframe's 54 mm bore.
     - The length places the mass. Its CG is the cylinder's middle, 0.145 m below the airframe's
       top, since the cylinder starts 7 cm down.
     - Of the mass properties, the radius changes only the moments of inertia: how hard the mass
       is to turn.
     - A cylinder wider than the tube's bore is an error. `AutoDimension::PackedRadius` in the
       component's `auto` list fits it to the bore instead.
     - `radial_offset_m` and `angle_rad` move it off the rocket's axis. Parachutes, streamers and
       shock cords have a packing too.
3. **Materials.** `material("abs")` looks up one of hpr's 49 built-in materials by its id, each
   with the source of its density ([Mass properties](physics/mass.md#materials)). The
   [`materials`](api/hpr_design/materials/index.html) page of the API reference lists them. For a
   material of your own, `Material::bulk(name, kg_m3)` takes a name and a density in kg/m³.
4. **The motor.** `Catalog::bundled()` is the catalog of 32
   [ThrustCurve.org](glossary.md#thrustcurveorg) motors that comes with hpr.
   [`find`](api/hpr_motor/catalog/struct.Catalog.html#method.find) looks one up by its
   [designation](glossary.md#motor-designation) or common name, ignoring case, spaces and hyphens,
   so `"h54"` finds this one too. `bundled_motor()` builds the motor from its thrust curve and the
   catalog's size and masses. The [`MountedMotor`](api/hpr_design/config/struct.MountedMotor.html)
   names the mount by its id, and carries the ejection delay, the case's diameter and length, and
   when the motor lights: `Ignition::Launch` here, and a later time for an air start or a
   sustainer ([Staging](physics/staging.md)).
   The design checks compare the case with its mount: a case wider than the mount's bore is an
   error. While the motor burns, hpr also uses the case's diameter for the
   [base drag](glossary.md#base-drag), the drag on the rocket's flat aft end: the part of that end
   the burning case covers gets none. [Solid motors](physics/motor.md#using-a-motor)
   lists [the bundled motors](physics/motor.md#the-bundled-motors), and shows how to use
   [a motor file of your own](physics/motor.md#a-motor-from-a-file), such as one from
   ThrustCurve.org, instead.
5. **The rocket.** A [`Rocket`](api/hpr_design/tree/struct.Rocket.html) holds its stages (one
   here), how its reference diameter is chosen, and its
   [configurations](glossary.md#configuration). `ReferenceDiameter::Maximum {}` takes the widest
   body part, the 56.3 mm airframe, as the diameter that the margin and the aerodynamic
   coefficients are measured by ([reference area](glossary.md#reference-area)). A configuration is
   one choice of motors, at most one per mount, under an id; this rocket has one, `"h54"`. Add
   another to compare motors in the same rocket.
6. **Mass, CG and CP.**
   - [`assemble`](api/hpr_design/tree/struct.Rocket.html#method.assemble) places every part and
     the configuration's motor, and returns an
     [`Assembly`](api/hpr_design/config/struct.Assembly.html).
   - Its `mass_properties(t)` is the whole rocket `t` seconds after ignition, and
     `dry_mass_properties()` is the rocket with every motor spent.
   - Each gives a mass, a CG and the moments of inertia (how hard the rocket is to turn), which
     the flight needs. The CG, `cg_m`, is in the
     [body frame](glossary.md#body-frame), whose origin is the nose tip and whose `z` axis points
     forward, out through the nose. So a point `s` metres aft of the tip has `z = −s`, and the CG's
     station is `−cg_m.z`.
   - [`AeroModel::new`](api/hpr_aero/model/struct.AeroModel.html#method.new) builds the
     aerodynamic model from the placed parts. Its
     [`normal_force`](api/hpr_aero/model/struct.AeroModel.html#method.normal_force), at
     `Flow::axial(0.3)` (Mach 0.3, with the air straight along the axis), returns the rocket's
     slope and its CP, `cp_station_m`. Barrowman's slopes are the small-angle limit, so this is the
     CP at small angles of attack.
     [`components`](api/hpr_aero/model/struct.AeroModel.html#method.components), at the same flow,
     returns each part's share.
   - The margin is the CP's station less the CG's, divided by the reference diameter,
     `assembly.layout.reference_diameter_m`.
7. **The flight.** This is as in [Getting started](getting-started.md#the-program-step-by-step),
   with three differences.
   - `Simulation::new` names the configuration to fly, `"h54"`, and runs the design's checks first
     ([Checks](physics/design.md#checks)).
   - The parachute's trigger is `Trigger::MotorDelay { motor: 0 }`: the ejection charge of the
     configuration's first motor ([Recovery](physics/recovery.md#triggers-lag-and-release)).
   - A [`Recorder`](api/hpr_sim/recorder/struct.Recorder.html) keeps the airspeed and the Mach
     number at the end of every step, and the program takes the row with the highest airspeed.
     [Recording a trajectory](recording-a-trajectory.md) explains recorders.
8. **The design file.** The end of the program writes the rocket as JSON, reads it back, and prints
   the fin set's part of it. [As a design file](#as-a-design-file), below, explains.

## As a design file

A design file is a rocket written as text, to keep, share or edit outside Rust. Today it is the
rocket's JSON:

- `serde_json::to_string_pretty(&rocket)` gives the text, and `std::fs::write` saves it to a file.
- `serde_json::from_str::<Rocket>(&text)` reads it back. The program checks that the rocket it
  reads back is the one it wrote.

The JSON follows the Rust types, so the [API reference](api.md) documents every key. The fin set
at the end of the output shows the rules:

| in Rust | in the JSON | in the output |
|---|---|---|
| a struct's field | a key with the field's name | `"thickness_m": 0.003175` |
| the unit in the name | SI units: `_m` metres, `_kg` kilograms, `_rad` radians, `kg_m3` kg/m³ | `"span_m": 0.045` |
| a `Part` | an object with one key, the kind of part | `"part": { "fin_set": { … } }` |
| a shape, planform, wall, density, finish or delay | an object whose `"kind"` names it | `"planform": { "kind": "trapezoidal", … }` |
| a `Position` | an object whose `"from"` names it | `"position": { "from": "bottom", "aft_offset_m": 0.0 }` |
| a choice with no values | a string | `"cross_section": "rounded"` |
| `None` | `null` | `"tab": null` |

- Some keys can be left out, and take a default: a component's `name`, or a fin set's `tab` and
  `cant_rad`, for example. `cant_rad` is the fins' [cant](glossary.md#cant) in radians, 0 by
  default; a cant spins the rocket
  ([Roll: forcing and damping](physics/aero.md#roll-forcing-and-damping)).
- A key hpr doesn't know is refused, so a misspelt key is an error rather than silently ignored.
- A mounted motor is stored whole: its thrust curve, masses and size. Its `designation` is only a
  label, so a design file doesn't depend on the catalog.

For a complete file of a similar rocket, with centering rings, rail buttons, a parachute and a
shock cord as parts, on a 38 mm Cesaroni I175, see
[`synthetic-54mm-three-fin.json`](https://github.com/nrdptel/hpr-sim/blob/main/validation/designs/synthetic-54mm-three-fin.json).
A program in the repository writes the files in that folder, so edit a copy rather than the file.

**The format is provisional.** It is hpr's own, and the open design format ([M3.3](decisions-and-roadmap.md#m3-3), a
documented and versioned design file with a schema) will replace it and convert the repository's
own designs.

## What else a design can hold

The example leaves out several kinds of part and setting that a design can have:

- **More parts:** transitions, centering rings (whose radii can be automatic), launch lugs, rail
  buttons, parachutes, streamers, shock cords, and elliptical or freeform fins
  ([The design tree](physics/design.md#the-tree)).
- **Rail guides.** With rail buttons or launch lugs, the rocket leaves the rail when its last guide
  passes the top, not its aft end.
- **Weighed masses.** An [`Overrides`](api/hpr_design/tree/struct.Overrides.html) sets a part's
  mass, CG or inertia to measured values, for the part alone or with everything attached to it
  ([Overrides](physics/design.md#overrides)).
- **Checks.** [`hpr_design::checks::check`](api/hpr_design/checks/fn.check.html) lists a design's
  problems ([Checks](physics/design.md#checks)). Errors, such as a motor wider than its mount,
  describe a rocket that can't exist, and a simulation refuses them: put the 38 mm `H170M` in this
  program's 29 mm mount and it stops with `MotorWiderThanMount`. Warnings, such as a step in the
  body's radius, don't stop a flight.

## What it can't do yet

- **Only in Rust, or in JSON.** A simpler builder ([M4.1](decisions-and-roadmap.md#m4-1), the simpler library interface),
  a command-line tool ([M4.2](decisions-and-roadmap.md#m4-2)) and Python ([M4.3](decisions-and-roadmap.md#m4-3)) are planned.
- **No import from other programs.** [OpenRocket](glossary.md#openrocket) `.ork` files ([M3.1](decisions-and-roadmap.md#m3-1), OpenRocket import)
  and RockSim `.rkt` files ([M3.4](decisions-and-roadmap.md#m3-4), RockSim import) can't be read yet.
- **Drag near and past Mach 1 is lightly checked.** Since
  [M1.8b1](decisions-and-roadmap.md#m1-8b1) (drag through Mach 1), hpr's own drag, like its normal
  force, carries a flight from Mach 0 to 5, and a flight that reaches Mach 5 stops with an error.
  Near and above the speed of sound the drag has been checked against one wind tunnel, which
  measured from Mach 0.6 to 4.63. hpr reads high there at most speeds, most of all with fins past
  Mach 1 ([Aerodynamics](physics/aero.md#drag-against-the-arcas-robin-wind-tunnel)). Against a
  worked example in a U.S. Army design handbook, the body alone reads a little low faster than
  sound. Against [RASAero II](glossary.md#rasaero-ii)'s drag for a rocket with a short, steep
  [boattail](glossary.md#boattail), the whole rocket reads about a quarter low faster than sound,
  for reasons not yet pinned down
  ([Aerodynamics](physics/aero.md#drag-against-rasaero-ii-through-mach-2)). So if your rocket
  goes supersonic, its drag there may be off by a quarter or more either way: possibly low with a
  steep boattail, high with thin, sharp fins. Treat a supersonic flight's apogee as rough until
  [M1.8b3](decisions-and-roadmap.md#m1-8b3) (a boattail's drag faster than sound) and the issues
  it leaves are done.
- **Staging needs its settings.** A motor lights at launch unless its `ignition` says otherwise, so a
  two-stage design flies with both stages burning at once until you give the sustainer its
  ignition and the flight a separation ([Staging](physics/staging.md)). A `.ork` file's own
  ignitions and one powered separation are read for you. The ignitions come with the rocket, but
  the separation doesn't: turn the configuration's `staging` into the flight's separation with
  `hpr::ork::separation` and pass it to the flight, with a recovery device on each part, since hpr
  refuses the flight without them. The example
  [`ork_two_stage.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr/examples/ork_two_stage.rs)
  does both. Staged, clustered and air-start flights of OpenRocket's examples are within 5% of
  OpenRocket's apogee and largest speed. Three cluster apogees are compared with OpenRocket's
  flight with no parachute, since its parachute opened before apogee
  ([M1.9c](decisions-and-roadmap.md#m1-9c), a two-stage and a cluster design against OpenRocket;
  [results](format/ork.md#staged-clustered-and-air-start-flights)).
- **Commercial solid motors only** ([COTS motors](glossary.md#cots-motor)). With only catalog data,
  a motor's own CG stays at its mid-length, full or spent ([Solid motors](physics/motor.md)).
- **Tube fins are refused** by the aerodynamics until a cited method for them exists. Tube fins
  are open tubes that run along the body, touching it, in place of flat fins. A design can hold
  them, but a flight or a CP can't be worked out with them.
- **Two nose shapes have no drag of hpr's own**, on a nose cone or on a transition that widens,
  because no drag data covers them: a bulged secant ogive (`NoseShape::Ogive` with a
  `radius_ratio` below 1, which bulges wider than the body just ahead of its base) and a Haack
  shape whose parameter `C` is above 1/3, past the LV-Haack ([Shapes](physics/shapes.md#profiles)).
  Since [M1.8b1](decisions-and-roadmap.md#m1-8b1), the drag through Mach 1, the drag buildup
  refuses them, naming the part. The CP still works, and so does a flight on a drag table from
  another tool; a flight on hpr's own drag stops with that error.
- **Fin sections and supersonic drag.** Faster than sound, every fin section takes a blunt
  leading edge's drag: the square section a flat face's, the rounded and airfoil sections a
  rounded edge's. The airfoil section differs from the rounded only in having no trailing-edge
  base drag. That reads far high for thin, sharp fins, and the one wind tunnel hpr has been
  measured against tested only double-wedge fins, so how well square or rounded edges fare is
  unmeasured ([Drag limits](physics/aero.md#drag-limits)).

## Where next

- [Getting started](getting-started.md#change-it) adds wind, tilts the rail and uses a drogue and a
  main parachute; the same code works with this rocket.
- [Recording a trajectory](recording-a-trajectory.md) keeps the whole flight as a table.
- [The design tree](physics/design.md), [Mass properties](physics/mass.md) and
  [Aerodynamics](physics/aero.md) explain the models behind these numbers.

