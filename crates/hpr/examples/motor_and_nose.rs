//! Optimization with choices: CMA-ES picks one of five 54 mm motors and one of four nose cones
//! from Madcow Rocketry's catalogue, with the nose ballast and the fin span, to send a 2.6 in
//! rocket to 3,048 m (10,000 ft) within a competition's stability and rail-exit limits. The
//! winner is checked by flying it again.
//!
//! ```text
//! cargo run --example motor_and_nose -p hpr
//! ```
//!
//! The documentation site's page *Optimization* (`docs/optimization.md`) walks through it. What it
//! prints is kept next to it in `motor_and_nose.output.txt`, and CI checks that the two still
//! agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_aero::AeroModel;
use hpr::hpr_analysis::optimize::cmaes::Cmaes;
use hpr::hpr_analysis::optimize::{Evaluation, Variable};
use hpr::hpr_io::orc;
use hpr::hpr_sim::{Adaptive, FlightMetrics, FlightSettings, Method};
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    Position, Rocket, Trigger,
};

/// The apogee to hit, m above the pad: 10,000 ft.
const TARGET_M: f64 = 3048.0;

/// How close counts as a hit when the winner is flown again, m.
const HIT_M: f64 = 0.1;

/// The margins' limits over the ascent, calibres, as the International Rocket Engineering
/// Competition's rules set them (Design, Test & Evaluation Guide, 2025, §10.3.1 and §10.4.1): a
/// dynamic margin of at least 1.5, a static margin of at most 4 and a dynamic one of at most 6.
/// The dynamic margin is read as the flight margin, at the flight's Mach number.
const LEAST_FLIGHT_CAL: f64 = 1.5;
const MOST_STATIC_CAL: f64 = 4.0;
const MOST_FLIGHT_CAL: f64 = 6.0;

/// The least speed off the rail, m/s: the same rules' 100 ft/s (§10.2.1).
const RAIL_EXIT_M_S: f64 = 30.0;

/// The launch rail's length, m.
const RAIL_M: f64 = 3.0;

/// The body tube's length, m: fixed, so that every design with one nose cone has one shape.
const BODY_M: f64 = 1.0;

/// The five 54 mm motors of the built-in catalog that fit the 18 in motor tube.
const MOTORS: [&str; 5] = ["J450DM", "J760", "J300LR", "K400C", "K940"];

/// Four nose cones for a 2.6 in tube in Madcow Rocketry's catalogue, shortest first, with what
/// they are.
const NOSES: [(&str, &str); 4] = [
    ("FWNC26T-K", "3:1 ogive, fiberglass"),
    ("PNC26K-W", "3:1 ogive, plastic"),
    ("FWNC26T-C", "5:1 cone, fiberglass"),
    ("FWNC26T-YY", "5:1 ogive, fiberglass"),
];

fn main() -> Result<(), Box<dyn Error>> {
    let seed = 2026;
    // The motors in order of total impulse, smallest first, so that neighbouring choices are
    // alike: the optimizer treats a choice as a whole number, and steps between neighbours.
    let mut motors = MOTORS
        .iter()
        .map(|&name| {
            let motor = Motor::from_catalog(name)?;
            let impulse_ns = motor.solid_motor().curve().total_impulse_ns();
            Ok((name, motor, impulse_ns))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    motors.sort_by(|a, b| a.2.total_cmp(&b.2));
    let mut flyer = Flyer::default();

    println!(
        "A 2.6 in rocket of Madcow Rocketry's parts: choose the motor, the nose cone, the nose"
    );
    println!("ballast and the fin span for a 3,048 m apogee, with margins over the ascent of at");
    println!(
        "least {LEAST_FLIGHT_CAL} calibres (flight), at most {MOST_STATIC_CAL} (static) and {MOST_FLIGHT_CAL} (flight), and at least {RAIL_EXIT_M_S} m/s off a {RAIL_M} m rail"
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");

    // From the middle motor and nose, a step of one choice.
    let variables = vec![
        Variable::new("motor", 2.0, 1.0)?
            .within(0.0, last(MOTORS.len()))?
            .integer()?,
        Variable::new("nose", 1.0, 1.0)?
            .within(0.0, last(NOSES.len()))?
            .integer()?,
        Variable::new("nose ballast (kg)", 0.4, 0.15)?.within(0.0, 1.5)?,
        Variable::new("fin span (m)", 0.07, 0.015)?.within(0.03, 0.15)?,
    ];
    // Stop within a centimetre of the target, or after 3,000 flights.
    let mut run = Cmaes::new(variables)?
        .with_target(1e-4)?
        .with_max_evaluations(3_000)?
        .start(seed)?;
    // Per motor: the designs flown, and the apogee nearest the target of those within the limits.
    let mut flown = [0_usize; MOTORS.len()];
    let mut nearest: [Option<f64>; MOTORS.len()] = [None; MOTORS.len()];
    let optimum = loop {
        let mut evaluations = Vec::new();
        for x in run.candidates() {
            let (motor, nose) = (choice(x[0]), choice(x[1]));
            let design = Design {
                nose,
                ballast_kg: x[2],
                fin_span_m: x[3],
            };
            flown[motor] += 1;
            let evaluation = match flyer.fly(&motors[motor].1, &design) {
                // The squared miss is the goal; the limits are constraints.
                Ok(design) => {
                    let miss = design.apogee_m - TARGET_M;
                    if design.keeps_limits()
                        && nearest[motor].is_none_or(|m| miss.abs() < (m - TARGET_M).abs())
                    {
                        nearest[motor] = Some(design.apogee_m);
                    }
                    Evaluation::constrained(miss * miss, &design.limits())
                }
                // A design the simulator refuses ranks below every flight.
                Err(_) => Evaluation::failed(),
            };
            evaluations.push(evaluation);
        }
        if let Some(optimum) = run.tell_constrained(&evaluations)? {
            break optimum;
        }
    };

    println!("motor    total impulse (N·s)   designs flown   nearest apogee within the limits");
    for (k, (name, _, impulse_ns)) in motors.iter().enumerate() {
        let nearest = nearest[k].map_or("none".to_owned(), |m| format!("{m:.0} m"));
        println!("{name:<8} {impulse_ns:>19.1} {:>15}   {nearest}", flown[k]);
    }
    if optimum.violation > 0.0 {
        return Err("no design kept the limits".into());
    }
    let [motor, nose, ballast_kg, fin_span_m] = optimum.point[..] else {
        return Err("four variables expected".into());
    };
    let (motor, nose) = (choice(motor), choice(nose));
    let winner = Design {
        nose,
        ballast_kg,
        fin_span_m,
    };
    println!(
        "Found: the {} with the {} nose ({}), {ballast_kg:.2} kg of ballast, fins {:.1} cm in span",
        motors[motor].0,
        NOSES[nose].0,
        NOSES[nose].1,
        fin_span_m * 100.0
    );
    println!(
        "(stopped: {:?}, after {} flights)",
        optimum.stop, optimum.evaluations
    );

    // Fly the winner again from a fresh build and fresh tables, and check every limit on that
    // flight; then again with the integrator's tolerances 100 times tighter, so that the hit is
    // the flight's, not the integrator's.
    let again = Flyer::default().fly(&motors[motor].1, &winner)?;
    println!(
        "Flown again: apogee {:.1} m, top speed Mach {:.2}, rail exit {:.1} m/s, liftoff mass {:.2} kg;",
        again.apogee_m, again.max_mach, again.rail_exit_m_s, again.mass_kg
    );
    println!(
        "margins over the ascent: flight {:.2} to {:.2} calibres, static at most {:.2}",
        again.least_flight_cal, again.most_flight_cal, again.most_static_cal
    );
    let tight = FlightSettings {
        method: Method::DormandPrince54(Adaptive {
            relative_tolerance: 1e-10,
            absolute_tolerance: 1e-10,
            ..Adaptive::default()
        }),
        ..FlightSettings::default()
    };
    let tighter = Flyer {
        settings: tight,
        ..Flyer::default()
    }
    .fly(&motors[motor].1, &winner)?;
    println!(
        "Flown again, tolerances 100 times tighter: apogee {:.1} m, rail exit {:.1} m/s",
        tighter.apogee_m, tighter.rail_exit_m_s
    );
    let hit = |flown: &Flown| (flown.apogee_m - TARGET_M).abs() <= HIT_M && flown.keeps_limits();
    let met_again = hit(&again) && hit(&tighter);
    println!(
        "Both flights within {HIT_M} m of 3,048 m and within every limit: {}",
        if met_again { "yes" } else { "no" }
    );
    if !met_again {
        return Err("the winner flown again misses the target or breaks a limit".into());
    }
    Ok(())
}

/// The place a choice's encoded value names in its list.
fn choice(x: f64) -> usize {
    // Cast: an integer variable's value is a whole number within its bounds, 0 to a list's last.
    x as usize
}

/// The last place in a list of `len` choices, as a variable's high bound.
fn last(len: usize) -> f64 {
    // Cast: a handful of choices.
    (len - 1) as f64
}

/// The one part in the bundled catalogue that Madcow Rocketry numbers `number`.
fn part(number: &str) -> Result<&'static orc::Part, Box<dyn Error>> {
    match orc::bundled().find("Madcow", number).as_slice() {
        [part] => Ok(part),
        found => Err(format!("Madcow {number}: {} parts, not one", found.len()).into()),
    }
}

/// What the optimizer chooses, besides the motor.
struct Design {
    /// The nose cone's place in [`NOSES`].
    nose: usize,
    /// The ballast in the nose's tip, kg.
    ballast_kg: f64,
    /// The three fins' span, m.
    fin_span_m: f64,
}

impl Design {
    /// The rocket: Madcow's 2.6 in fiberglass tube cut to [`BODY_M`], the nose cone with the
    /// ballast in its tip, an 18 in 54 mm motor tube, three fiberglass fins, and `motor`.
    fn rocket(&self, motor: &Motor) -> Result<Rocket, Box<dyn Error>> {
        let tube = part("FT26-THIN-600-NAT")?;
        let orc::PartKind::BodyTube(airframe) = &tube.kind else {
            return Err("FT26-THIN-600-NAT is a body tube".into());
        };
        let mut rocket = Rocket::new("2.6 in, Madcow parts", airframe.outer_diameter_m)?;
        rocket
            .add_nose(Nose::from_catalog(part(NOSES[self.nose].0)?)?)?
            .add_tube(Tube::from_catalog(tube)?.with_length_m(BODY_M))?
            .add_motor_tube(MotorTube::from_catalog(part("T54-180")?)?)?
            .add_fins(
                Fins::new(
                    3,
                    FinPlanform::Trapezoidal {
                        root_chord_m: 0.14,
                        tip_chord_m: 0.05,
                        span_m: self.fin_span_m,
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
                Mass::new(self.ballast_kg, Position::Top { aft_offset_m: 0.0 })
                    .packed(0.03, 0.06)
                    .named("nose ballast"),
            )?
            .set_motor(motor.clone())?
            .add_parachute(Device::new(
                "parachute",
                DeviceDrag::canopy(CanopyType::FlatCircular, 1.2),
                Trigger::Apogee,
            ));
        Ok(rocket)
    }
}

/// Flies designs, keeping one aerodynamic model per nose cone. Designs with one nose have one
/// shape, as the body's length is fixed; past Mach 1.2 a flight needs its shape's supersonic
/// table, a fifth of a second or more to build, so they share it and it is built once.
#[derive(Default)]
struct Flyer {
    settings: FlightSettings,
    shapes: [Option<AeroModel>; NOSES.len()],
}

/// What a design does: its flight's apogee, speed off the rail and top Mach number, its margins
/// over the ascent (from the rail exit to apogee) and its mass at launch.
struct Flown {
    apogee_m: f64,
    rail_exit_m_s: f64,
    least_flight_cal: f64,
    most_static_cal: f64,
    most_flight_cal: f64,
    mass_kg: f64,
    max_mach: f64,
}

impl Flown {
    /// The limits as constraints `g ≤ 0`, each in a unit that makes them count alike: the margins
    /// in hundredths of a calibre, the rail-exit speed in tenths of a m/s.
    fn limits(&self) -> [f64; 4] {
        [
            (LEAST_FLIGHT_CAL - self.least_flight_cal) / 0.01,
            (self.most_static_cal - MOST_STATIC_CAL) / 0.01,
            (self.most_flight_cal - MOST_FLIGHT_CAL) / 0.01,
            (RAIL_EXIT_M_S - self.rail_exit_m_s) / 0.1,
        ]
    }

    /// Whether it keeps every limit.
    fn keeps_limits(&self) -> bool {
        self.limits().iter().all(|g| *g <= 0.0)
    }
}

impl Flyer {
    /// Flies `design` on `motor` from a 3 m rail, 85° above the horizon, into 5 m/s of wind from
    /// the west. The least flight margin is searched for inside steps; the most of each margin is
    /// taken at the steps' ends, where the flight keeps them. A margin that isn't defined at some
    /// step's end fails the design: the rocket is unstable there, or its parts' normal forces
    /// cancel too nearly for a margin to mean anything.
    fn fly(&mut self, motor: &Motor, design: &Design) -> Result<Flown, Box<dyn Error>> {
        let rocket = design.rocket(motor)?;
        let environment =
            Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(5.0, 270.0)?;
        let mut simulation = Flight::builder(&rocket, &environment, RAIL_M)
            .inclination_deg(85.0)
            .heading_deg(270.0)
            .settings(self.settings)
            .simulation()?;
        // The flight is the same, bit for bit, on a shared table as on its own. A shape with no
        // table keeps slender-body theory past Mach 1.2, and has none to share: the PNC26K-W is
        // 0.05 mm narrower than the tube, a step at the joint the table's method doesn't take.
        match &self.shapes[design.nose] {
            Some(shape) => {
                let _shared = simulation.share_supersonic_table(shape);
            }
            None => self.shapes[design.nose] = Some(simulation.aero().clone()),
        }
        let mut metrics = FlightMetrics::new();
        let result = simulation.run(&mut metrics)?;
        let summary = metrics.summary(&result, environment.sim())?;
        let (mut most_static_cal, mut most_flight_cal) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for at in metrics.stability() {
            let undefined = "a margin undefined during the ascent";
            most_static_cal = most_static_cal.max(at.static_margin.margin_cal.ok_or(undefined)?);
            most_flight_cal = most_flight_cal.max(at.flight_margin.margin_cal.ok_or(undefined)?);
        }
        Ok(Flown {
            apogee_m: summary.apogee.ok_or("no apogee")?.height_above_ground_m,
            rail_exit_m_s: summary.rail_exit_speed_m_s.ok_or("no rail exit")?.value,
            least_flight_cal: summary.min_flight_margin_cal.ok_or("no margin")?.value,
            most_static_cal,
            most_flight_cal,
            mass_kg: rocket.mass_properties(0.0)?.mass_kg,
            max_mach: summary.max_mach.ok_or("no top speed")?.value,
        })
    }
}
