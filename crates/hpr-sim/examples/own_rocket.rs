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
