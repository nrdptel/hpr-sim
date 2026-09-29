//! Drag models of your own, flown in place of hpr's: hpr's drag made 10% higher, and a drag
//! curve against Mach number such as a wind tunnel or a flight's data would give.
//!
//! ```text
//! cargo run --example custom_drag -p hpr
//! ```
//!
//! The documentation site's page *Models of your own* (`docs/custom-models.md`) walks through
//! it. What it prints is kept next to it in `custom_drag.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_aero::{AeroError, DragModel, DragQuery};
use hpr::hpr_sim::SimError;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

/// hpr's own drag, scaled: `factor` 1.1 is 10% more drag, as a rougher finish than the design
/// says might give.
#[derive(Debug)]
struct Scaled {
    factor: f64,
}

impl DragModel for Scaled {
    fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
        // `buildup` is what hpr would have flown without this model.
        Ok(self.factor * query.buildup()?.zero_lift_coefficient)
    }
}

/// A drag curve: the zero-lift drag coefficient at a few Mach numbers, with the motor off and
/// burning, joined by straight lines. A burning motor fills the base, so its curve is lower.
#[derive(Debug)]
struct Curve {
    /// `(Mach number, C_D0 coasting, C_D0 under thrust)`, in rising Mach number.
    points: Vec<(f64, f64, f64)>,
}

impl DragModel for Curve {
    fn zero_lift_drag(&self, query: &DragQuery<'_>) -> Result<f64, AeroError> {
        let mach = query.mach();
        let thrusting = query.conditions().thrusting;
        let pick = |&(_, off, on): &(f64, f64, f64)| if thrusting { on } else { off };
        // Refuse a speed past the data rather than guess: the flight stops, naming it. The
        // integrator tries speeds a little past the flight's own as it steps, so a real curve
        // wants some margin past the top speed.
        let beyond = AeroError::Domain {
            what: "Mach number past the drag curve's last point",
            value: mach,
        };
        let last = self.points.last().ok_or(beyond.clone())?;
        if mach > last.0 {
            return Err(beyond);
        }
        for pair in self.points.windows(2) {
            let (below, above) = (&pair[0], &pair[1]);
            if mach <= above.0 {
                let t = ((mach - below.0) / (above.0 - below.0)).max(0.0);
                return Ok(pick(below) + t * (pick(above) - pick(below)));
            }
        }
        Ok(pick(last))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket of the `build_and_fly` example on an H54, with its parachute opened at apogee
    // rather than by the motor's charge, so the charge doesn't cut the climb short.
    let mut rocket = Rocket::new("My 54 mm rocket", 0.0563)?;
    rocket
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.22,
                0.0015,
                material("abs")?,
            )
            .with_capped_shoulder(0.06, 0.0015),
        )?
        .add_tube(Tube::new(0.9, 0.00115, material("kraft_phenolic")?))?
        .add_motor_tube(
            MotorTube::new(0.2, 0.029, 0.001, material("kraft_phenolic")?).with_overhang_m(0.005),
        )?
        .add_fins(
            Fins::new(
                3,
                FinPlanform::Trapezoidal {
                    root_chord_m: 0.1,
                    tip_chord_m: 0.04,
                    span_m: 0.045,
                    sweep_m: 0.05,
                },
                0.003175,
                material("birch_plywood")?,
            )
            .with_cross_section(FinCrossSection::Rounded),
        )?
        .add_mass(
            Mass::new(0.2, Position::Top { aft_offset_m: 0.07 })
                .packed(0.15, 0.05)
                .named("recovery bay"),
        )?
        .set_motor(Motor::from_catalog("H54")?)?
        .add_parachute(Device::new(
            "parachute",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
            Trigger::Apogee,
        ));

    // Spaceport America, 1,400 m up, in calm air, from a vertical 1.8 m rail.
    let environment = Environment::new(32.99, -106.97, 1400.0)?;
    let launch = Flight::builder(&rocket, &environment, 1.8);

    // An invented curve, shaped like a small rocket's: not a measurement of this one.
    let curve = Curve {
        points: vec![
            (0.0, 0.50, 0.44),
            (0.3, 0.46, 0.40),
            (0.6, 0.48, 0.42),
            (0.8, 0.55, 0.49),
            (1.0, 0.75, 0.69),
        ],
    };
    let flights = [
        ("hpr's own drag", launch.fly()?),
        (
            "hpr's, 10% higher",
            launch.clone().drag_model(Scaled { factor: 1.1 }).fly()?,
        ),
        (
            "a curve (invented)",
            launch.clone().drag_model(curve).fly()?,
        ),
    ];

    println!(
        "{} on a {}, from a 1.8 m vertical rail in calm air",
        rocket.design().name,
        rocket.configuration_id().ok_or("no motor")?
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("drag                  apogee  apogee at  top speed");
    println!("                         (m)        (s)      (m/s)");
    for (name, flight) in &flights {
        println!(
            "{name:<20} {:>8.1} {:>10.2} {:>10.0}",
            flight.apogee_m().ok_or("no apogee")?,
            flight.apogee_time_s().ok_or("no apogee")?,
            flight.max_speed_m_s().ok_or("no top speed")?,
        );
    }

    // A curve that stops short of the flight's speed stops the flight, and says why.
    let short = Curve {
        points: vec![(0.0, 0.50, 0.44), (0.4, 0.47, 0.41)],
    };
    match launch.drag_model(short).fly() {
        // hpr wraps the model's own error, so it says where it came from.
        Err(hpr::Error::Sim(SimError::Aero(AeroError::DragModel { source }))) => match *source {
            AeroError::Domain { what, value } => {
                println!();
                println!("A curve that ends at Mach 0.4 stops the flight: {what}, {value:.2}");
            }
            other => return Err(other.into()),
        },
        other => return Err(format!("the short curve flew: {other:?}").into()),
    }
    Ok(())
}
