//! M6.1d's measurement: 10,000 dispersed flights of a Level 2 rocket, from ignition to the
//! ground under a drogue and a main, on every core. The budget is 10 s on the development
//! machine; the numbers are recorded in `docs/perf.md`.
//!
//! ```text
//! cargo bench -p hpr --features parallel --bench ten_thousand
//! ```
//!
//! Two rockets: Valetudo on a K400C, which stays below Mach 0.6, and a minimum-diameter 54 mm
//! rocket on a K940, which passes Mach 1.2 and so needs the supersonic table. Not a criterion
//! benchmark: one run is 10,000 flights, so the program times whole runs itself and prints the
//! fastest of three, then where one flight's time goes on one thread.

#![expect(
    clippy::unwrap_used,
    clippy::print_stdout,
    reason = "a measurement with invalid fixed inputs should stop at once, and it exists to print"
)]

use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

use hpr::hpr_analysis::montecarlo::{Dispersion, FlightInputs, MonteCarlo};
use hpr::hpr_atmos::ConstantWind;
use hpr::hpr_core::geodesy::Geodetic;
use hpr::hpr_sim::Rail;
use hpr::hpr_sim::recovery::Inflation;
use hpr::rocket::{Fins, Mass, MotorTube, Nose, Tube, material};
use hpr::{
    CanopyType, Device, DeviceDrag, Environment, FinPlanform, Flight, Motor, NoseShape, Position,
    Rocket, Trigger,
};

/// The flights in one run.
const COUNT: u64 = 10_000;

/// The runs timed; the fastest is reported.
const REPEATS: usize = 3;

/// The flights timed one at a time for the split.
const SPLIT: u64 = 200;

/// The run's seed.
const SEED: u64 = 2026;

/// A 0.6 m drogue at apogee that the 2.4 m main releases at 150 m, as in `hpr-sim`'s recovery
/// benchmark (M1.7a).
fn recovery() -> Vec<Device> {
    vec![
        Device::new(
            "drogue",
            DeviceDrag::canopy(CanopyType::FlatCircular, 0.6),
            Trigger::Apogee,
        )
        .with_lag_s(0.5)
        .with_inflation(Inflation::knacke(CanopyType::FlatCircular).unwrap())
        .with_release_by(1),
        Device::new(
            "main",
            DeviceDrag::canopy(CanopyType::FlatCircular, 2.4),
            Trigger::Altitude {
                height_above_ground_m: 150.0,
            },
        )
        .with_lag_s(1.0)
        .with_inflation(Inflation::knacke(CanopyType::FlatCircular).unwrap()),
    ]
}

/// Valetudo on a K400C (9.7 kg), the Level 2 rocket of `hpr-sim`'s flight benchmark, from a
/// 3 m rail at 85° heading west into a 5 m/s west wind at Spaceport America.
fn valetudo() -> FlightInputs {
    let rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))
    .unwrap();
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0).unwrap();
    let environment = hpr::hpr_sim::Environment {
        wind: Arc::new(ConstantWind::new(5.0, 270_f64.to_radians()).unwrap()),
        ..hpr::hpr_sim::Environment::standard(site).unwrap()
    };
    let rail = Rail {
        elevation_rad: 85_f64.to_radians(),
        azimuth_rad: 270_f64.to_radians(),
        ..Rail::vertical(3.0)
    };
    let mut inputs = FlightInputs::new(rocket, "example", environment, rail);
    inputs.recovery = recovery();
    inputs
}

/// A minimum-diameter rocket on a Cesaroni K940 (Pro54, 1,633 N·s): a 66 mm filament-wound
/// airframe around a 54 mm motor tube, three G10 fins, 3.4 kg on the pad. The same site, rail
/// and wind.
fn k940() -> FlightInputs {
    let fiberglass = || material("fiberglass_filament_wound").unwrap();
    let mut rocket = Rocket::new("Minimum-diameter 54 mm", 0.066).unwrap();
    rocket
        .add_nose(
            Nose::hollow(
                NoseShape::Ogive { radius_ratio: 1.0 },
                0.33,
                0.0015,
                fiberglass(),
            )
            .with_capped_shoulder(0.07, 0.0015),
        )
        .unwrap()
        .add_tube(Tube::new(1.0, 0.0015, fiberglass()))
        .unwrap()
        .add_motor_tube(MotorTube::new(0.42, 0.054, 0.001, fiberglass()).with_overhang_m(0.005))
        .unwrap()
        .add_fins(Fins::new(
            3,
            FinPlanform::Trapezoidal {
                root_chord_m: 0.14,
                tip_chord_m: 0.05,
                span_m: 0.06,
                sweep_m: 0.08,
            },
            0.003175,
            material("fiberglass_g10").unwrap(),
        ))
        .unwrap()
        .add_mass(
            Mass::new(0.4, Position::Top { aft_offset_m: 0.1 })
                .packed(0.2, 0.06)
                .named("avionics and recovery"),
        )
        .unwrap()
        .set_motor(Motor::from_catalog("K940").unwrap())
        .unwrap();
    for device in recovery() {
        rocket.add_parachute(device);
    }
    let environment = Environment::new(32.99, -106.97, 1400.0)
        .unwrap()
        .with_constant_wind(5.0, 270.0)
        .unwrap();
    Flight::builder(&rocket, &environment, 3.0)
        .inclination_deg(85.0)
        .heading_deg(270.0)
        .inputs()
        .unwrap()
}

/// The fastest of [`REPEATS`] calls of `f`, and its last result.
fn fastest<T>(mut f: impl FnMut() -> T) -> (Duration, T) {
    let mut best = Duration::MAX;
    let mut last = None;
    for _ in 0..REPEATS {
        let start = Instant::now();
        let value = black_box(f());
        best = best.min(start.elapsed());
        last = Some(value);
    }
    (best, last.unwrap())
}

/// Times a run of [`COUNT`] flights of `nominal` on every core, then, on one thread, building
/// and flying [`SPLIT`] of its samples apart, and flying them as each flight did before M6.1d,
/// on a supersonic table of its own.
fn measure(name: &str, nominal: FlightInputs) {
    let dispersion = Dispersion {
        dry_mass_sd_fraction: 0.02,
        cg_sd_m: 0.005,
        drag_sd_fraction: 0.05,
        impulse_sd_fraction: 0.03,
        burn_time_sd_fraction: 0.02,
        wind_speed_sd_fraction: 0.25,
        wind_heading_sd_rad: 15_f64.to_radians(),
        rail_elevation_sd_rad: 1_f64.to_radians(),
        rail_azimuth_sd_rad: 2_f64.to_radians(),
        deployment_lag_sd_s: 0.2,
        ..Dispersion::default()
    };
    let monte_carlo = MonteCarlo::new(nominal, dispersion).unwrap();
    let (run_time, run) = fastest(|| monte_carlo.run_parallel(SEED, COUNT));
    let apogee = run.apogee().unwrap().summary();
    let mach = run
        .distribution(|flight| flight.max_mach.as_ref().map(|peak| peak.value))
        .unwrap()
        .summary();
    println!("{name}");
    println!(
        "  {COUNT} flights on {} threads: {:.2} s (fastest of {REPEATS}), {} failed",
        std::thread::available_parallelism().unwrap(),
        run_time.as_secs_f64(),
        run.failed().count(),
    );
    println!(
        "  apogee {:.0} m mean, peak Mach {:.2} to {:.2}",
        apogee.mean.unwrap(),
        mach.min.unwrap(),
        mach.max.unwrap()
    );

    let inputs: Vec<_> = (0..SPLIT)
        .map(|index| monte_carlo.inputs(&monte_carlo.draw(SEED, index)).unwrap())
        .collect();
    let (build_time, _) = fastest(|| {
        inputs
            .iter()
            .map(|inputs| inputs.simulation().unwrap())
            .count()
    });
    let (sample_time, _) = fastest(|| {
        (0..SPLIT)
            .map(|index| monte_carlo.sample(SEED, index))
            .count()
    });
    let (alone_time, _) = fastest(|| inputs.iter().map(|inputs| inputs.fly().unwrap()).count());
    let per = |time: Duration| 1e3 * time.as_secs_f64() / SPLIT as f64;
    println!(
        "  one thread, per flight over {SPLIT}: {:.3} ms a sample, {:.3} ms of it building; \
         {:.3} ms flown on a table of its own",
        per(sample_time),
        per(build_time),
        per(alone_time)
    );
}

fn main() {
    measure("Valetudo, K400C", valetudo());
    measure("Minimum-diameter 54 mm, K940", k940());
}
