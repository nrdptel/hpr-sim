//! Benchmarks of the motor calls the flight engine makes on every derivative evaluation, and of
//! reading motor files and the bundled catalog. Results are recorded in `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hpr_motor::catalog::{Catalog, bundled_curve_text};
use hpr_motor::{BatesGrains, MassElement, Propellant, SolidMotor, eng, rse};

fn benches(c: &mut Criterion) {
    let catalog = Catalog::bundled().unwrap();
    // Loki M1378LR: a 54 mm, 4 s curve with 100-odd points.
    let entry = catalog.find("M1378LR").next().unwrap();
    let curve = &entry.curves[0];
    let text = bundled_curve_text(&curve.file).unwrap();
    let envelope = entry.bundled_motor().unwrap();
    let grains = BatesGrains {
        count: 4,
        density_kg_m3: 1750.0,
        outer_radius_m: 0.0235,
        initial_inner_radius_m: 0.0095,
        initial_height_m: 0.2559,
        separation_m: 0.004,
        center_m: 0.56,
        inhibited_ends: false,
    };
    let dry = MassElement::thin_tube(1.731, 0.5, 0.027, 1.108);
    let bates = SolidMotor::new(
        envelope.curve().clone(),
        Propellant::Grains(grains),
        dry,
        None,
    )
    .unwrap();
    let rse_text = rse::write(&rse::RseFile {
        engines: vec![rse::RseEngine {
            manufacturer: "Loki".into(),
            code: "M1378LR".into(),
            motor_type: Some("reloadable".into()),
            diameter_mm: 54.0,
            length_mm: 1108.0,
            initial_mass_g: 4331.0,
            propellant_mass_g: 2600.0,
            delays: Some("1000".into()),
            auto_calc_mass: Some(true),
            auto_calc_cg: Some(true),
            average_thrust_n: None,
            peak_thrust_n: None,
            throat_diameter_mm: None,
            exit_diameter_mm: None,
            total_impulse_ns: None,
            burn_time_s: None,
            mass_fraction_pct: None,
            isp_s: None,
            comments: None,
            points: eng::parse(text).unwrap().value.entries[0]
                .points
                .iter()
                .map(|&(t, f)| rse::RsePoint {
                    time_s: t,
                    thrust_n: f,
                    mass_g: Some(1.0),
                    cg_mm: Some(554.0),
                })
                .collect(),
        }],
    })
    .unwrap();

    c.bench_function("ThrustCurve::thrust_n", |b| {
        b.iter(|| envelope.curve().thrust_n(black_box(2.345)));
    });
    c.bench_function("SolidMotor::state, column", |b| {
        b.iter(|| envelope.state(black_box(2.345)));
    });
    c.bench_function("SolidMotor::state, BATES grains", |b| {
        b.iter(|| bates.state(black_box(2.345)));
    });
    c.bench_function("eng::parse, one bundled curve", |b| {
        b.iter(|| eng::parse(black_box(text)).unwrap());
    });
    c.bench_function("rse::parse, the same curve", |b| {
        b.iter(|| rse::parse(black_box(&rse_text)).unwrap());
    });
    c.bench_function("Catalog::bundled", |b| {
        b.iter(|| Catalog::bundled().unwrap());
    });
}

criterion_group!(motor, benches);
criterion_main!(motor);
