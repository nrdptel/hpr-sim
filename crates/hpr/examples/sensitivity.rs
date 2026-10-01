//! Sensitivity analysis: Sobol' indices and Morris's screening checked against two test functions
//! whose answers are known, then a Morris screening of which inputs move a rocket's apogee and
//! landing most.
//!
//! ```text
//! cargo run --example sensitivity -p hpr
//! ```
//!
//! The documentation site's page *Sensitivity analysis* (`docs/sensitivity.md`) walks through it.
//! What it prints is kept next to it in `sensitivity.output.txt`, and CI checks that the two
//! still agree (`cargo xtask examples --check`).

#![allow(
    clippy::print_stdout,
    reason = "the project's lints forbid printing in library code, and this program exists to print"
)]

use std::error::Error;

use hpr::hpr_analysis::montecarlo::{Dispersion, MonteCarlo};
use hpr::hpr_analysis::sensitivity::Factor;
use hpr::hpr_analysis::sensitivity::benchmark::{Ishigami, SobolG};
use hpr::hpr_analysis::sensitivity::morris::Morris;
use hpr::hpr_analysis::sensitivity::sobol::Sobol;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinCrossSection, FinPlanform, Flight, Motor,
    NoseShape, Position, Rocket, Trigger,
};

fn main() -> Result<(), Box<dyn Error>> {
    let seed = 2026;

    // Sobol' indices of Ishigami's function, against their closed forms.
    let ishigami = Ishigami::STANDARD;
    let sobol = Sobol::new(Ishigami::factors()?, 8192)?;
    let indices = sobol.indices(seed, |x| ishigami.evaluate(x))?;
    println!(
        "Sobol' indices of Ishigami's function (a = 7, b = 0.1), {} rows, {} runs",
        sobol.rows(),
        sobol.rows() * 5
    );
    println!("        first order: known  estimate ± error      total: known  estimate ± error");
    for (index, (s, t)) in indices
        .factors
        .iter()
        .zip(ishigami.first_order().into_iter().zip(ishigami.total()))
    {
        println!(
            "{:<6} {:>18.4} {:>9.4} ± {:.4} {:>18.4} {:>9.4} ± {:.4}",
            index.name,
            s,
            index.first_order,
            index.first_order_standard_error,
            t,
            index.total,
            index.total_standard_error
        );
    }
    println!();

    // The same for Sobol's g function: aᵢ = 0 matters most, 99 hardly at all.
    let g = SobolG::new(vec![0.0, 1.0, 4.5, 9.0, 99.0, 99.0, 99.0, 99.0])?;
    let sobol = Sobol::new(g.factors()?, 8192)?;
    let indices = sobol.indices(seed, |x| g.evaluate(x))?;
    println!(
        "Sobol' indices of the g function, a = 0, 1, 4.5, 9, 99 (four times), {} rows, {} runs",
        sobol.rows(),
        sobol.rows() * 10
    );
    println!("        first order: known  estimate ± error      total: known  estimate ± error");
    for (index, (s, t)) in indices
        .factors
        .iter()
        .zip(g.first_order().into_iter().zip(g.total()))
        .take(5)
    {
        println!(
            "{:<6} {:>18.4} {:>9.4} ± {:.4} {:>18.4} {:>9.4} ± {:.4}",
            index.name,
            s,
            index.first_order,
            index.first_order_standard_error,
            t,
            index.total,
            index.total_standard_error
        );
    }
    println!("(x6 to x8 are as x5)");
    println!();

    // Morris's screening of Ishigami's function: 100 paths against every effect on the grid.
    let morris = Morris::new(Ishigami::factors()?, 4, 100)?;
    let screening = morris.screen(seed, |x| ishigami.evaluate(x))?;
    let population = morris.population(|x| ishigami.evaluate(x))?;
    println!(
        "Morris screening of Ishigami's function, {} levels, {} paths, {} runs",
        morris.levels(),
        morris.paths(),
        morris.paths() * 4
    );
    println!("            μ*: whole grid  estimate ± error       σ: whole grid  estimate");
    for (e, exact) in screening.effects.iter().zip(&population) {
        println!(
            "{:<6} {:>21.3} {:>9.3} ± {:.3} {:>20.3} {:>9.3}",
            e.name,
            exact.mean_absolute,
            e.mean_absolute,
            e.mean_absolute_standard_error,
            exact.standard_deviation,
            e.standard_deviation
        );
    }
    println!();

    rocket_screening(seed)?;
    Ok(())
}

/// Morris's screening of the `monte_carlo` example's rocket: which of six inputs move its apogee
/// and its landing most.
fn rocket_screening(seed: u64) -> Result<(), Box<dyn Error>> {
    // The rocket of the `monte_carlo` example: the `build_and_fly` one on an H54.
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
    let environment = Environment::new(32.99, -106.97, 1400.0)?.with_constant_wind(4.0, 270.0)?;
    let launch = Flight::builder(&rocket, &environment, 1.8)
        .inclination_deg(85.0)
        .heading_deg(270.0);

    // A Monte Carlo set-up with no dispersion: its draws are the nominal flight, and
    // `MonteCarlo::inputs` turns a draw with some entries changed into the flight it flies.
    let monte_carlo = MonteCarlo::new(launch.inputs()?, Dispersion::default())?;
    let nominal = monte_carlo.draw(seed, 0);

    // Each input's range, made up for the example. NFPA 1125 caps a motor type's impulse
    // standard deviation at 6.7%.
    let factors = vec![
        Factor::new("dry mass factor", 0.95, 1.05)?,
        Factor::new("drag factor", 0.9, 1.1)?,
        Factor::new("impulse factor", 0.94, 1.06)?,
        Factor::new("wind speed (m/s)", 0.0, 8.0)?,
        Factor::new("wind turn (°)", -30.0, 30.0)?,
        Factor::new("rail angle (°)", 80.0, 90.0)?,
    ];
    let morris = Morris::new(factors, 4, 10)?;
    let design = morris.design(seed);
    let mut apogees = Vec::with_capacity(design.len());
    let mut distances = Vec::with_capacity(design.len());
    for x in design.points() {
        let mut draw = nominal.clone();
        draw.dry_mass_scale.iter_mut().for_each(|s| *s = x[0]);
        draw.drag_scale = x[1];
        draw.impulse_scale.iter_mut().for_each(|s| *s = x[2]);
        draw.wind_speed_scale = x[3] / 4.0;
        draw.wind_turn_rad = x[4].to_radians();
        draw.rail_elevation_offset_rad = (x[5] - 85.0).to_radians();
        let flight = monte_carlo.fly(&draw)?;
        let apogee = flight.apogee.as_ref().ok_or("no apogee")?;
        apogees.push(apogee.height_above_ground_m);
        distances.push(flight.landing.as_ref().ok_or("no landing")?.distance_m);
    }
    println!(
        "Morris screening of the Monte Carlo page's rocket, {} levels, {} paths, {} flights",
        morris.levels(),
        morris.paths(),
        design.len()
    );
    println!("Not yet validated: see the Accuracy page before trusting these numbers.");
    println!("Each effect is the change across the input's whole range, m.");
    println!(
        "                     apogee: μ*  ± error      σ     landing distance: μ*  ± error      σ"
    );
    let apogee = design.analyse(&apogees)?;
    let distance = design.analyse(&distances)?;
    for (a, d) in apogee.effects.iter().zip(&distance.effects) {
        println!(
            "{:<18} {:>13.1} ± {:>5.1} {:>6.1} {:>26.1} ± {:>5.1} {:>6.1}",
            a.name,
            a.mean_absolute,
            a.mean_absolute_standard_error,
            a.standard_deviation,
            d.mean_absolute,
            d.mean_absolute_standard_error,
            d.standard_deviation
        );
    }
    Ok(())
}
