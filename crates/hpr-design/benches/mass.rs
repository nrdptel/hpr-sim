//! Benchmarks of mass properties from geometry: the calls a design edit or a Monte Carlo sample
//! that perturbs dimensions makes. Results are recorded in `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hpr_design::{
    FinCrossSection, FinPlanform, FinSet, Material, NoseCone, NoseShape, Profile, Wall, revolve,
};

fn benches(c: &mut Criterion) {
    // A 4" (101.6 mm) 5:1 von Kármán nose and a 3" boattail.
    let nose = Profile::nose(NoseShape::VON_KARMAN, 0.508, 0.0508).unwrap();
    let boattail =
        Profile::transition(NoseShape::TANGENT_OGIVE, 0.1, 0.0508, 0.0381, false).unwrap();
    let shell = Wall::Shell { thickness_m: 0.002 };
    c.bench_function("revolve von Karman nose, filled", |b| {
        b.iter(|| revolve(black_box(&nose), Wall::Filled {}).unwrap())
    });
    c.bench_function("revolve von Karman nose, 2 mm wall", |b| {
        b.iter(|| revolve(black_box(&nose), shell).unwrap())
    });
    c.bench_function("revolve ogive boattail, 2 mm wall", |b| {
        b.iter(|| revolve(black_box(&boattail), shell).unwrap())
    });
    let cone = NoseCone {
        shape: NoseShape::VON_KARMAN,
        length_m: 0.508,
        base_radius_m: 0.0508,
        wall: shell,
        shoulder: None,
        material: Material::bulk("fiberglass", 1990.0),
    };
    c.bench_function("NoseCone::mass_properties, 2 mm wall", |b| {
        b.iter(|| black_box(&cone).mass_properties().unwrap())
    });
    let mut fins = FinSet {
        count: 4,
        planform: FinPlanform::Trapezoidal {
            root_chord_m: 0.2,
            tip_chord_m: 0.08,
            span_m: 0.11,
            sweep_m: 0.12,
        },
        thickness_m: 0.00318,
        cross_section: FinCrossSection::Airfoil,
        tab: None,
        cant_rad: 0.0,
        base_angle_rad: 0.0,
        material: Material::bulk("G10", 1800.0),
    };
    c.bench_function("FinSet::mass_properties, trapezoidal airfoil", |b| {
        b.iter(|| black_box(&fins).mass_properties(0.0508).unwrap())
    });
    fins.planform = FinPlanform::Freeform {
        points_m: vec![
            [0.0, 0.0],
            [0.08, 0.1],
            [0.12, 0.11],
            [0.16, 0.06],
            [0.2, 0.0],
        ],
    };
    fins.cross_section = FinCrossSection::Rounded;
    c.bench_function("FinSet::mass_properties, freeform rounded", |b| {
        b.iter(|| black_box(&fins).mass_properties(0.0508).unwrap())
    });
}

criterion_group!(group, benches);
criterion_main!(group);
