//! Benchmarks of the normal-force model: building it from a resolved design once, and evaluating
//! it at a flow condition, which the flight engine asks for at every derivative evaluation.
//! Results are recorded in `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hpr_aero::{AeroModel, Flow};
use hpr_design::Rocket;

fn benches(c: &mut Criterion) {
    let two_stage: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/synthetic-two-stage-75mm-54mm.json"
    ))
    .unwrap();
    let calisto: Rocket = serde_json::from_str(include_str!(
        "../../../validation/designs/rocketpy-calisto-getting-started-motor-at-minus-1.255.json"
    ))
    .unwrap();
    let two_stage = two_stage.layout().unwrap();
    let calisto = calisto.layout().unwrap();
    c.bench_function("AeroModel::new, synthetic two-stage", |b| {
        b.iter(|| AeroModel::new(black_box(&two_stage)).unwrap())
    });
    c.bench_function("AeroModel::new, Calisto", |b| {
        b.iter(|| AeroModel::new(black_box(&calisto)).unwrap())
    });
    let flow = Flow::new(0.6, 0.05, 0.3);
    let model = AeroModel::new(&two_stage).unwrap();
    c.bench_function("AeroModel::normal_force, synthetic two-stage", |b| {
        b.iter(|| black_box(&model).normal_force(black_box(&flow)).unwrap())
    });
    let model = AeroModel::new(&calisto).unwrap();
    c.bench_function("AeroModel::normal_force, Calisto", |b| {
        b.iter(|| black_box(&model).normal_force(black_box(&flow)).unwrap())
    });
    let model = AeroModel::new(&two_stage).unwrap();
    for mach in [0.6, 1.0, 2.0] {
        c.bench_function(
            &format!("AeroModel::roll, synthetic two-stage, Mach {mach}"),
            |b| b.iter(|| black_box(&model).roll(black_box(mach)).unwrap()),
        );
    }
}

criterion_group!(group, benches);
criterion_main!(group);
