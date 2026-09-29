//! A wind model of your own: a wind that grows and turns with height, flown against a steady
//! wind of the same speed at the ground.
//!
//! ```text
//! cargo run --example custom_wind -p hpr
//! ```
//!
//! The documentation site's page *Custom models* (`docs/custom-models.md`) walks through it. What
//! it prints is kept next to it in `custom_wind.output.txt`, and CI checks that the two still
//! agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_atmos::{AtmosError, Wind, WindSample};
use hpr::hpr_core::DVec3;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

/// A wind that strengthens and veers (turns clockwise) as it rises, from the ground up to
/// `top_m` above it, and holds steady above that. Its numbers are made up.
#[derive(Debug)]
struct Veering {
    /// The ground's height above mean sea level, m: a wind is asked for by that height.
    ground_msl_m: f64,
    /// The height above the ground where the change ends, m.
    top_m: f64,
    /// Speed at the ground and at the top, m/s.
    speed_m_s: (f64, f64),
    /// Where it blows from at the ground and at the top, degrees clockwise from north.
    from_deg: (f64, f64),
}

impl Wind for Veering {
    fn wind(&self, height_msl_m: f64) -> Result<WindSample, AtmosError> {
        if !height_msl_m.is_finite() {
            return Err(AtmosError::Domain {
                what: "height above mean sea level, m",
                value: height_msl_m,
            });
        }
        // How far up the change it is: 0 at the ground (and below it), 1 at the top and above.
        let t = ((height_msl_m - self.ground_msl_m) / self.top_m).clamp(0.0, 1.0);
        let speed_m_s = self.speed_m_s.0 + t * (self.speed_m_s.1 - self.speed_m_s.0);
        let from_rad = (self.from_deg.0 + t * (self.from_deg.1 - self.from_deg.0)).to_radians();
        // The air moves away from where it blows from: a west wind (from 270°) moves east, +x.
        Ok(WindSample {
            velocity_enu_m_s: DVec3::new(
                -speed_m_s * from_rad.sin(),
                -speed_m_s * from_rad.cos(),
                0.0,
            ),
            extrapolated: None,
        })
    }
}

/// `x` to the nearest metre, with no minus sign on a zero: in a steady west wind the rocket lands
/// a rounding error north or south of the pad's east line, and which side isn't worth printing.
fn metres(x: f64) -> f64 {
    x.round() + 0.0
}

fn main() -> Result<(), Box<dyn Error>> {
    // The rocket of the `build_and_fly` example, with an H54 and a 10 s delay.
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
        .set_motor(Motor::from_catalog("H54")?.with_delay_s(10.0)?)?
        .add_parachute(Device::new(
            "parachute",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.9),
            Trigger::MotorDelay { motor: 0 },
        ));

    // Spaceport America, 1,400 m up. A steady 4 m/s from the west, and a wind that starts the
    // same at the ground but grows to 10 m/s and turns to come from the north-west (315°) by
    // 1,000 m up.
    let elevation_m = 1400.0;
    let site = Environment::new(32.99, -106.97, elevation_m)?;
    let steady = site.clone().with_constant_wind(4.0, 270.0)?;
    let veering = site.with_wind(Veering {
        ground_msl_m: elevation_m,
        top_m: 1000.0,
        speed_m_s: (4.0, 10.0),
        from_deg: (270.0, 315.0),
    });

    println!(
        "{} on a {}, from a 1.8 m vertical rail",
        rocket.design().name,
        rocket.configuration_id().ok_or("no motor")?
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!();
    println!("wind                    apogee   landing east  landing south  descent");
    println!("                           (m)            (m)            (m)    (m/s)");
    for (name, environment) in [("steady, from the west", &steady), ("veering", &veering)] {
        let flight = Flight::builder(&rocket, environment, 1.8).fly()?;
        let landing = flight.landing().ok_or("no landing")?;
        println!(
            "{name:<21} {:>8.1} {:>14.0} {:>14.0} {:>8.1}",
            flight.apogee_m().ok_or("no apogee")?,
            metres(landing.east_m),
            metres(-landing.north_m),
            landing.descent_rate_m_s,
        );
    }
    Ok(())
}
