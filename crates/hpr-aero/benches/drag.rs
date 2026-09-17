//! Benchmarks of the drag buildup: evaluating it at a flow condition, which the flight engine asks
//! for at every derivative evaluation, and the override-table lookup that replaces it. Results are
//! recorded in `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hpr_aero::{AeroModel, DragConditions, DragTable, Flow};
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
    let flow = Flow::new(0.6, 0.05, 0.3);
    // Sea level at Mach 0.6, one 54 mm motor thrusting.
    let conditions = DragConditions::thrusting(0.6 * 340.294 / 1.4607e-5, 2.29e-3);
    let two_stage = AeroModel::new(&two_stage.layout().unwrap()).unwrap();
    let calisto = AeroModel::new(&calisto.layout().unwrap()).unwrap();
    c.bench_function("AeroModel::drag, synthetic two-stage", |b| {
        b.iter(|| {
            black_box(&two_stage)
                .drag(black_box(&flow), black_box(&conditions))
                .unwrap()
        })
    });
    c.bench_function("AeroModel::drag, Calisto", |b| {
        b.iter(|| {
            black_box(&calisto)
                .drag(black_box(&flow), black_box(&conditions))
                .unwrap()
        })
    });
    let mach: Vec<f64> = (0..200).map(|k| 0.01 * f64::from(k + 1)).collect();
    let cd: Vec<f64> = mach.iter().map(|m| 0.4 + 0.1 * m * m).collect();
    let text: String = mach
        .iter()
        .zip(&cd)
        .map(|(m, c)| format!("{m},{c}\n"))
        .collect();
    let table = DragTable::from_csv(&text, Some(&text)).unwrap();
    let overridden = calisto.clone().with_drag_table(table);
    c.bench_function("AeroModel::drag, Calisto with a 200-row table", |b| {
        b.iter(|| {
            black_box(&overridden)
                .drag(black_box(&flow), black_box(&conditions))
                .unwrap()
        })
    });
}

criterion_group!(group, benches);
criterion_main!(group);
