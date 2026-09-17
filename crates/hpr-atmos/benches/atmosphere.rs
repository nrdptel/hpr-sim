//! Benchmarks of the atmosphere, wind and gust calls the flight engine makes on every derivative
//! evaluation. Results are recorded in `docs/perf.md`.

#![expect(
    clippy::unwrap_used,
    reason = "a benchmark with invalid fixed inputs should stop at once"
)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use hpr_atmos::dryden::{DrydenGenerator, DrydenParameters, GustField, TurbulenceSeverity};
use hpr_atmos::profile::{SoundingLevel, SoundingProfile};
use hpr_atmos::ussa76::Ussa76;
use hpr_atmos::wind::{LayeredWind, Wind, WindInterpolation, WindLevel};

/// A 30-level humid sounding with wind from a 1400 m site to 12 km, like a radiosonde's
/// significant levels.
fn sounding() -> SoundingProfile {
    let levels = (0..30)
        .map(|i| {
            let z = 1400.0 + f64::from(i) * 365.0;
            SoundingLevel {
                height_msl_m: z,
                temperature_k: 303.0 - 0.0068 * (z - 1400.0),
                pressure_pa: (i == 0).then_some(85_600.0),
                relative_humidity: Some(0.3 - 0.008 * f64::from(i)),
                wind_speed_m_s: Some(3.0 + f64::from(i)),
                wind_direction_from_rad: Some(4.0 + 0.02 * f64::from(i)),
            }
        })
        .collect();
    SoundingProfile::new(
        levels,
        32.99_f64.to_radians(),
        WindInterpolation::SpeedDirection,
    )
    .unwrap()
}

fn atmosphere(c: &mut Criterion) {
    let height = 4_400.0;
    let standard = Ussa76::standard();
    c.bench_function("ussa76_sample", |b| {
        b.iter(|| standard.sample(black_box(height)).unwrap());
    });
    let profile = sounding();
    c.bench_function("sounding_30_levels_sample", |b| {
        b.iter(|| profile.sample(black_box(height)).unwrap());
    });
    let wind: &LayeredWind = profile.wind().unwrap();
    c.bench_function("layered_wind_30_levels", |b| {
        b.iter(|| wind.wind(black_box(height)).unwrap());
    });
    let single = LayeredWind::new(
        vec![WindLevel {
            height_msl_m: 0.0,
            speed_m_s: 5.0,
            direction_from_rad: 1.0,
        }],
        WindInterpolation::default(),
    )
    .unwrap();
    c.bench_function("layered_wind_1_level", |b| {
        b.iter(|| single.wind(black_box(height)).unwrap());
    });
}

fn turbulence(c: &mut Criterion) {
    let parameters = DrydenParameters::mil_f_8785c_low_altitude(
        100.0,
        TurbulenceSeverity::Moderate.wind_speed_20_ft_m_s(),
    )
    .unwrap();
    let field = GustField::generate(7, &parameters, 20_000.0, 1.0).unwrap();
    c.bench_function("gust_field_lookup", |b| {
        b.iter(|| field.gust(black_box(12_345.6)).unwrap());
    });
    let mut generator = DrydenGenerator::new(7);
    c.bench_function("dryden_generator_advance_1_m", |b| {
        b.iter(|| generator.advance(black_box(1.0), &parameters).unwrap());
    });
    c.bench_function("gust_field_generate_20_km_at_1_m", |b| {
        b.iter(|| GustField::generate(black_box(7), &parameters, 20_000.0, 1.0).unwrap());
    });
}

criterion_group!(benches, atmosphere, turbulence);
criterion_main!(benches);
