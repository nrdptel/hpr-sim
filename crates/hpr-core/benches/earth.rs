//! Benchmarks of the Earth model calls the flight engine makes on every derivative evaluation.
//! Results are recorded in `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hpr_core::DVec3;
use hpr_core::earth::{Earth, EarthRotation, GravityModel};
use hpr_core::geodesy::Geodetic;
use hpr_core::gravity::NormalGravity;

fn gravity_models(c: &mut Criterion) {
    let site = Geodetic::from_degrees(32.99, -106.97, 1400.0).unwrap();
    // A point on a typical L2 trajectory: 3 km up, a few hundred metres downrange.
    let position = DVec3::new(350.0, -120.0, 3000.0);
    let velocity = DVec3::new(20.0, -5.0, 180.0);
    let mut group = c.benchmark_group("gravity_enu_mps2");
    for (name, model) in [
        ("constant", GravityModel::Constant { g_mps2: 9.80665 }),
        ("vertical_taylor", GravityModel::VerticalTaylor),
        ("vertical", GravityModel::Vertical),
        ("ellipsoidal", GravityModel::Ellipsoidal),
    ] {
        let earth =
            Earth::new(NormalGravity::wgs84(), site, model, EarthRotation::Coriolis).unwrap();
        group.bench_function(name, |b| {
            b.iter(|| earth.gravity_enu_mps2(black_box(position)).unwrap());
        });
    }
    group.finish();

    let earth = Earth::wgs84(site).unwrap();
    c.bench_function("coriolis_enu_mps2", |b| {
        b.iter(|| earth.rotation_acceleration_enu_mps2(black_box(velocity)));
    });
    c.bench_function("geodetic_from_enu", |b| {
        b.iter(|| {
            earth
                .frame()
                .geodetic_from_enu(black_box(position))
                .unwrap()
        });
    });
}

criterion_group!(benches, gravity_models);
criterion_main!(benches);
