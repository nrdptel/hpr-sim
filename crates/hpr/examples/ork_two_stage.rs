//! A two-stage rocket read from an OpenRocket `.ork` file and flown with its separation: the
//! booster burns out, the stages come apart at that instant, and the sustainer lights there, as
//! the file says, and flies on while the booster tumbles back.
//!
//! It uses the workspace crates `hpr`, `hpr-io`, `hpr-sim`, `hpr-design` and `hpr-core`; a
//! program of your own outside this repository depends on those five. hpr doesn't yet fly the
//! parachutes a `.ork` file holds, so each part here tumbles instead.
//!
//! Run it from anywhere in the repository:
//!
//! ```text
//! cargo run --example ork_two_stage -p hpr
//! ```
//!
//! The documentation site's staging page (`docs/physics/staging.md`, *Against OpenRocket*) points
//! here. What it prints is kept next to it in `ork_two_stage.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr_core::geodesy::Geodetic;
use hpr_design::Ignition;
use hpr_sim::{
    Device, DeviceDrag, Environment, EventKind, FlightSettings, Rail, Simulation, Trigger,
};

/// A made-up two-stage rocket as OpenRocket 24.12 writes one: 33 mm tubes, three fins on each
/// stage, and an Estes F15 in each stage's tube. The booster separates when its motor burns out
/// (`<separationevent>burnout</separationevent>`), and the sustainer's `automatic` ignition lights
/// it at the booster's ejection charge, which a `0.0` delay fires at burnout.
const ORK: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<openrocket version="1.10" creator="OpenRocket 24.12">
  <rocket><name>Two-stage example</name>
    <motorconfiguration configid="f15-f15" default="true"/>
    <subcomponents>
      <stage><name>Sustainer</name><id>sustainer-stage</id><subcomponents>
        <nosecone><name>Nose</name><id>nose</id>
          <material type="bulk" density="1000.0">Plastic</material>
          <length>0.15</length><thickness>0.002</thickness><shape>ogive</shape>
          <aftradius>0.0165</aftradius></nosecone>
        <bodytube><name>Sustainer tube</name><id>sustainer</id>
          <material type="bulk" density="680.0">Cardboard</material>
          <length>0.45</length><thickness>0.001</thickness><radius>0.0165</radius>
          <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
            <overhang>0.0</overhang>
            <motor configid="f15-f15"><type>single</type><manufacturer>Estes</manufacturer>
              <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
              <delay>4.0</delay></motor></motormount>
          <subcomponents><trapezoidfinset><name>Sustainer fins</name><id>sustainer-fins</id>
            <material type="bulk" density="680.0">Plywood</material>
            <axialoffset method="bottom">0.0</axialoffset><instancecount>3</instancecount>
            <rootchord>0.08</rootchord><tipchord>0.04</tipchord><height>0.05</height>
            <sweeplength>0.03</sweeplength><thickness>0.003</thickness>
            <crosssection>rounded</crosssection></trapezoidfinset></subcomponents>
        </bodytube></subcomponents></stage>
      <stage><name>Booster</name><id>booster-stage</id>
        <separationevent>burnout</separationevent><separationdelay>0.0</separationdelay>
        <subcomponents>
        <bodytube><name>Booster tube</name><id>booster</id>
          <material type="bulk" density="680.0">Cardboard</material>
          <length>0.3</length><thickness>0.001</thickness><radius>0.0165</radius>
          <motormount><ignitionevent>automatic</ignitionevent><ignitiondelay>0.0</ignitiondelay>
            <overhang>0.0</overhang>
            <motor configid="f15-f15"><type>single</type><manufacturer>Estes</manufacturer>
              <designation>F15</designation><diameter>0.029</diameter><length>0.114</length>
              <delay>0.0</delay></motor></motormount>
          <subcomponents><trapezoidfinset><name>Booster fins</name><id>booster-fins</id>
            <material type="bulk" density="680.0">Plywood</material>
            <axialoffset method="bottom">0.0</axialoffset><instancecount>3</instancecount>
            <rootchord>0.1</rootchord><tipchord>0.05</tipchord><height>0.07</height>
            <sweeplength>0.04</sweeplength><thickness>0.003</thickness>
            <crosssection>rounded</crosssection></trapezoidfinset></subcomponents>
        </bodytube></subcomponents></stage>
    </subcomponents></rocket>
</openrocket>"#;

fn main() -> Result<(), Box<dyn Error>> {
    // Read the file. Its one configuration flies: each motor lit as the file says, and the
    // booster's separation read as a `Staging`.
    let file = hpr_io::ork::read(ORK.as_bytes())?;
    let design = hpr_io::ork::design(&file.value).value;
    let configuration = &design.motors.configurations[0];
    // A configuration hpr can't fly is listed among the motors with its reason, but not among the
    // rocket's configurations, so the rocket's is found by id.
    let flown = design
        .rocket
        .configurations
        .iter()
        .find(|flown| flown.id == configuration.id)
        .ok_or("the configuration isn't flown")?;
    let staging = configuration
        .staging
        .as_ref()
        .ok_or("the configuration has no powered separation")?;
    println!("A made-up two-stage rocket read from .ork text, an F15 in each stage, calm air");
    println!("See the Accuracy page before trusting these numbers.");
    println!();
    for motor in &flown.motors {
        let lights = match &motor.ignition {
            Ignition::Launch => "at launch".to_owned(),
            Ignition::Burnout { mount, delay_s } => {
                format!("{delay_s:.1} s after the motor in `{mount}` burns out")
            }
            other => format!("{other:?}"),
        };
        println!(
            "the {} in `{}` lights {lights}",
            motor.designation, motor.mount
        );
    }
    // Stage 0 is the top stage, with the nose: the booster is every stage after it.
    println!(
        "the booster drops away from stage {} at {:.3} s",
        staging.after_stage, staging.time_s
    );

    // The flight: a 1.5 m rail at a site 200 m above sea level, in the standard atmosphere and
    // calm air.
    let site = Geodetic::from_degrees(35.0, -106.0, 200.0)?;
    let simulation = Simulation::new(
        &design.rocket,
        &configuration.id,
        Environment::standard(site)?,
        Rail::vertical(1.5),
        FlightSettings::default(),
    )?;
    // The separation, and a device on each part it makes: hpr's descent of a separated part has
    // no airframe drag of its own, so the booster tumbles from the split and the sustainer from
    // its apogee.
    let assembly = simulation.assembly();
    let separation = hpr::ork::separation(staging, assembly)?;
    let last = assembly.layout.stages.len().saturating_sub(1);
    let sustainer = DeviceDrag::tumbling_stages(assembly, (0, staging.after_stage))?;
    let booster = DeviceDrag::tumbling_stages(assembly, (staging.after_stage + 1, last))?;
    let simulation = simulation
        .with_recovery(vec![
            Device::new("sustainer, tumbling", sustainer, Trigger::Apogee),
            // A booster's devices act only once it flies on its own, so a time of zero is the
            // split.
            Device::new("booster, tumbling", booster, Trigger::Time { time_s: 0.0 }).on_body(1),
        ])?
        .with_separation(separation)?;
    let flight = simulation.run(&mut ())?;

    println!();
    for event in &flight.events {
        let name = match event.kind {
            EventKind::Separation => "separation",
            EventKind::Ignition(_) => "sustainer lights",
            EventKind::Apogee => "apogee",
            EventKind::GroundHit => "sustainer lands",
            _ => continue,
        };
        let sample = &event.sample;
        // The flight ends a micrometre below the ground: print that as 0 (`+ 0.0` turns a -0
        // into 0).
        println!(
            "{name:<17} {:>6.2} s {:>7.1} m {:>6.1} m/s",
            sample.time_s,
            sample.height_above_ground_m.max(0.0) + 0.0,
            sample.cg_velocity_enu_m_s.length(),
        );
    }
    for body in &flight.bodies {
        println!(
            "the booster leaves at {:.2} s and lands at {:.2} s",
            body.start_sample.time_s, body.final_sample.time_s
        );
    }
    Ok(())
}
