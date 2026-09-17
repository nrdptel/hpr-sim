//! Benchmarks of whole flights: a typical Level 2 rocket from ignition to the ground, which
//! Monte Carlo and optimization will run thousands of times. Results are recorded in
//! `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hpr_atmos::ConstantWind;
use hpr_core::geodesy::Geodetic;
use hpr_design::Rocket;
use hpr_sim::recovery::{Device, DeviceDrag, Inflation};
use hpr_sim::{CanopyType, Environment, FlightSettings, Rail, Recorder, Simulation, Trigger};

fn benches(c: &mut Criterion) {
    // Valetudo on a K400C: 9.7 kg, 880 m apogee, 29 s to the ground.
    let rocket: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-valetudo.json"
    ))
    .unwrap();
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0).unwrap();
    let environment = Environment {
        wind: std::sync::Arc::new(ConstantWind::new(5.0, 4.712_388_980_384_69).unwrap()),
        ..Environment::standard(site).unwrap()
    };
    let simulation = Simulation::new(
        &rocket,
        "example",
        environment,
        Rail::vertical(3.0),
        FlightSettings::default(),
    )
    .unwrap();
    c.bench_function("Simulation::run, Valetudo K400C to the ground", |b| {
        b.iter(|| black_box(&simulation).run(&mut ()).unwrap())
    });
    c.bench_function(
        "Simulation::run, Valetudo K400C with every channel at 10 ms",
        |b| {
            b.iter(|| {
                let mut recorder =
                    Recorder::new(hpr_sim::Channel::ALL.to_vec(), Some(0.01)).unwrap();
                black_box(&simulation).run(&mut recorder).unwrap();
                recorder
            })
        },
    );

    // The same flight recovered: a 0.6 m drogue at apogee and a 2.4 m main at 150 m, which is
    // what a Level 2 flight actually does and what Monte Carlo will run (M1.7a).
    let recovered = simulation
        .with_recovery(vec![
            Device::new(
                "drogue",
                DeviceDrag::canopy(CanopyType::FlatCircular, 0.6),
                Trigger::Apogee,
            )
            .with_lag_s(0.5)
            .with_inflation(Inflation::knacke(CanopyType::FlatCircular).unwrap())
            .released_by(1),
            Device::new(
                "main",
                DeviceDrag::canopy(CanopyType::FlatCircular, 2.4),
                Trigger::Altitude {
                    height_above_ground_m: 150.0,
                },
            )
            .with_lag_s(1.0)
            .with_inflation(Inflation::knacke(CanopyType::FlatCircular).unwrap()),
        ])
        .unwrap();
    c.bench_function(
        "Simulation::run, Valetudo K400C with a drogue and a main to the ground",
        |b| b.iter(|| black_box(&recovered).run(&mut ()).unwrap()),
    );
}

criterion_group!(group, benches);
criterion_main!(group);
