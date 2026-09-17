//! Benchmarks of the design tree: resolving a design once, and the rocket's mass properties at a
//! time in the burn, which the flight engine asks for at every derivative evaluation. Results are
//! recorded in `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
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
    c.bench_function("Rocket::layout, synthetic two-stage", |b| {
        b.iter(|| black_box(&two_stage).layout().unwrap())
    });
    c.bench_function("Rocket::layout, Calisto", |b| {
        b.iter(|| black_box(&calisto).layout().unwrap())
    });
    let assembly = two_stage.assemble("j760-i175").unwrap();
    c.bench_function("Assembly::mass_properties(t), two motors", |b| {
        b.iter(|| black_box(&assembly).mass_properties(black_box(0.7)))
    });
    let assembly = calisto.assemble("example").unwrap();
    c.bench_function(
        "Assembly::mass_properties(t), Calisto (BATES grains)",
        |b| b.iter(|| black_box(&assembly).mass_properties(black_box(1.3))),
    );
}

criterion_group!(group, benches);
criterion_main!(group);
