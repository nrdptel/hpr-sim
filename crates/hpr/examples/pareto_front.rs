//! Two goals at once: NSGA-II traces the trade-off between a rocket's apogee and its static
//! margin, over its nose ballast and fin span, and the front it finds is checked twice: each
//! design flown again, and the best apogee at two margins found again by CMA-ES alone.
//!
//! ```text
//! cargo run --example pareto_front -p hpr
//! ```
//!
//! The documentation site's page *Optimization* (`docs/optimization.md`) walks through it. What it
//! prints is kept next to it in `pareto_front.output.txt`, and CI checks that the two still agree
//! (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_aero::AeroModel;
use hpr::hpr_analysis::optimize::cmaes::Cmaes;
use hpr::hpr_analysis::optimize::nsga2::{Goals, Member, Nsga2};
use hpr::hpr_analysis::optimize::{Evaluation, Variable};
use hpr::hpr_sim::FlightMetrics;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

/// The least static margin a design may have, calibres.
const LEAST_MARGIN_CAL: f64 = 1.5;

/// The margins at which the front's apogee is shown, calibres.
const SHOWN_CAL: [f64; 4] = [2.0, 2.5, 3.0, 3.5];

/// The margin at which the front is checked against CMA-ES, calibres.
const CHECK_CAL: f64 = 2.5;

/// How far apart the front's apogee and CMA-ES's may be at the check, as a share of CMA-ES's.
const WITHIN: f64 = 0.005;

fn main() -> Result<(), Box<dyn Error>> {
    let seed = 2026;
    println!("A 66 mm rocket on a J760: how much apogee each calibre of static margin costs, over");
    println!("its nose ballast (0 to 0.8 kg) and fin span (4 to 10 cm), with a margin of at least");
    println!("{LEAST_MARGIN_CAL} calibres at launch mass, at Mach 0.3");
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");

    let variables = vec![
        Variable::new("nose ballast (kg)", 0.3, 0.15)?.within(0.0, 0.8)?,
        Variable::new("fin span (m)", 0.06, 0.015)?.within(0.04, 0.10)?,
    ];
    let optimizer = Nsga2::new(variables.clone(), 2)?
        .with_population(20)?
        .with_generations(25)?;
    // Goals: the apogee and the margin, both to be as large as they can be, so their negatives
    // are minimized. A design the simulator refuses is a failed one.
    let mut flyer = Flyer::default();
    let front = optimizer.minimize_constrained(seed, |x| {
        flyer.design(x[0], x[1]).map_or_else(
            |_| Goals::failed(),
            |(apogee, margin)| {
                Goals::constrained(vec![-apogee, -margin], &[LEAST_MARGIN_CAL - margin])
            },
        )
    })?;
    println!(
        "NSGA-II: {} designs a generation, {} generations, {} flights (seed {seed})",
        optimizer.population(),
        front.generations,
        front.evaluations
    );
    if !front.is_feasible() {
        return Err("a design on the front breaks the least margin".into());
    }
    // The front's designs, smallest margin first.
    let mut members: Vec<&Member> = front.members.iter().collect();
    members.sort_by(|a, b| margin_of(a).total_cmp(&margin_of(b)));

    // Every design of the front, flown again from a fresh build with a table of its own, gives
    // the same goals to the bit.
    for m in &members {
        let (apogee, margin) = Flyer::default().design(m.point[0], m.point[1])?;
        if [-apogee, -margin] != m.objectives[..] {
            return Err("a design flown again doesn't give the optimizer's goals".into());
        }
    }
    println!(
        "The front: {} designs, each flown again to the same apogee and margin",
        members.len()
    );
    println!();
    println!("margin (cal)   apogee on the front (m, between the designs either side)");
    for margin in SHOWN_CAL {
        let apogee = front_apogee(&members, margin).ok_or("the front doesn't reach a margin")?;
        println!("{margin:>12.1}   {:>8}", round_to_ten(apogee));
    }

    // CMA-ES alone, for the highest apogee with at least the check's margin, run for a fixed
    // number of flights: the front should give the same apogee there.
    let flights = 200;
    let cmaes = Cmaes::new(variables)?
        .with_max_evaluations(flights)?
        .with_tolerance_x(0.0)?
        .with_tolerance_value(0.0)?
        .minimize_constrained(seed, |x| {
            flyer.design(x[0], x[1]).map_or_else(
                |_| Evaluation::failed(),
                |(apogee, margin)| Evaluation::constrained(-apogee, &[CHECK_CAL - margin]),
            )
        })?;
    if cmaes.violation != 0.0 {
        return Err("CMA-ES found no design with the check's margin".into());
    }
    let (best, on_front) = (
        -cmaes.value,
        front_apogee(&members, CHECK_CAL).ok_or("the front doesn't reach the check")?,
    );
    let close = (on_front - best).abs() <= WITHIN * best;
    println!();
    println!(
        "At {CHECK_CAL} calibres, CMA-ES alone ({flights} flights): apogee {} m; the front within 0.5%: {}",
        round_to_ten(best),
        if close { "yes" } else { "no" }
    );
    if !close {
        return Err("the front misses CMA-ES's apogee".into());
    }
    Ok(())
}

/// A design's margin, from its member's goals.
fn margin_of(m: &Member) -> f64 {
    -m.objectives[1]
}

/// The front's apogee at `margin`: straight between the two designs either side of it.
/// `members` is sorted by margin, smallest first.
fn front_apogee(members: &[&Member], margin: f64) -> Option<f64> {
    members.windows(2).find_map(|pair| {
        let (a, b) = (pair[0], pair[1]);
        let (ma, mb) = (margin_of(a), margin_of(b));
        if !(ma <= margin && margin <= mb) || ma == mb {
            return None;
        }
        let w = (margin - ma) / (mb - ma);
        let (ha, hb) = (-a.objectives[0], -b.objectives[0]);
        Some(ha + w * (hb - ha))
    })
}

/// A number of metres to the nearest ten, as text.
fn round_to_ten(x: f64) -> String {
    format!("{:.0}", (x / 10.0).round() * 10.0)
}

/// Flies designs, sharing one supersonic table among them: the body ahead of the fins is the same
/// in every design, so the table is too, and the flight is the same, bit for bit, on a shared
/// table as on its own.
#[derive(Default)]
struct Flyer {
    shape: Option<AeroModel>,
}

impl Flyer {
    /// The apogee (m) and static margin (calibres, at launch mass, at Mach 0.3) of a design.
    fn design(&mut self, ballast_kg: f64, span_m: f64) -> Result<(f64, f64), Box<dyn Error>> {
        let rocket = rocket(ballast_kg, span_m)?;
        let margin = rocket.static_margin_cal(0.0, 0.3)?.ok_or("no margin")?;
        let environment =
            Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(5.0, 270.0)?;
        let mut simulation = Flight::builder(&rocket, &environment, 3.0)
            .inclination_deg(85.0)
            .heading_deg(270.0)
            .simulation()?;
        match &self.shape {
            Some(shape) => {
                let _shared = simulation.share_supersonic_table(shape);
            }
            None => self.shape = Some(simulation.aero().clone()),
        }
        let mut metrics = FlightMetrics::new();
        let result = simulation.run(&mut metrics)?;
        let summary = metrics.summary(&result, environment.sim())?;
        let apogee = summary.apogee.ok_or("no apogee")?.height_above_ground_m;
        Ok((apogee, margin))
    }
}

/// A 66 mm rocket on a J760 with `ballast_kg` in its nose and fins of span `span_m`.
fn rocket(ballast_kg: f64, span_m: f64) -> Result<Rocket, Box<dyn Error>> {
    let fiberglass = || material("fiberglass_filament_wound");
    let mut rocket = Rocket::new("66 mm on a J760", 0.066)?;
    rocket
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.33,
                0.0015,
                fiberglass()?,
            )
            .with_capped_shoulder(0.07, 0.0015),
        )?
        .add_tube(Tube::new(1.0, 0.0015, fiberglass()?))?
        .add_motor_tube(MotorTube::new(0.42, 0.054, 0.001, fiberglass()?).with_overhang_m(0.005))?
        .add_fins(
            Fins::new(
                3,
                FinPlanform::Trapezoidal {
                    root_chord_m: 0.14,
                    tip_chord_m: 0.05,
                    span_m,
                    sweep_m: 0.08,
                },
                0.003175,
                material("fiberglass_g10")?,
            )
            .with_cross_section(FinCrossSection::Rounded),
        )?
        .add_mass(
            Mass::new(0.4, Position::Top { aft_offset_m: 0.1 })
                .packed(0.2, 0.06)
                .named("avionics and recovery"),
        )?
        .add_mass(
            Mass::new(ballast_kg, Position::Top { aft_offset_m: 0.0 })
                .packed(0.03, 0.06)
                .named("nose ballast"),
        )?
        .set_motor(Motor::from_catalog("J760")?)?
        .add_parachute(Device::new(
            "parachute",
            DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
            Trigger::Apogee,
        ));
    Ok(rocket)
}
