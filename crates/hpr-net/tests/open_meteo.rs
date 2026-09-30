//! Open-Meteo's weather against recorded responses (M5.2a): the profile reproduces the pressure,
//! temperature and wind at every pressure level above the ground, and the fetch goes through the
//! cache and its offline mode.
//!
//! The expected values are read from the recordings here, with `serde_json`, not through the
//! parser under test.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests read the committed fixtures and write a cache; not the pure core"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::path::Path;

use hpr_atmos::profile::geometric_from_wmo_geopotential_m;
use hpr_atmos::wind::velocity_from_speed_direction;
use hpr_atmos::{Wind, WindInterpolation};
use hpr_net::open_meteo::{
    self, ATTRIBUTION, DropReason, OpenMeteoApi, OpenMeteoError, OpenMeteoProfile,
    OpenMeteoRequest, PRESSURE_LEVELS_HPA,
};
use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Transport};
use serde_json::Value;

/// Spaceport America's launch area, as recorded.
const LATITUDE_DEG: f64 = 32.99;
const LONGITUDE_DEG: f64 = -106.97;
/// 2025-06-21 15:00 UTC, the historical recording's first hour.
const HISTORICAL_S: i64 = 1_750_518_000;
/// 2026-10-02 18:00 UTC, the forecast recording's first hour.
const FORECAST_S: i64 = 1_790_964_000;
const HOUR_S: i64 = 3_600;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/replay")
            .join(name),
    )
    .unwrap()
}

fn replay() -> Replay {
    Replay::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay")).unwrap()
}

/// The recordings, each with its first hour.
fn recordings() -> [(&'static str, i64); 2] {
    [
        ("open-meteo-historical.json", HISTORICAL_S),
        ("open-meteo-forecast.json", FORECAST_S),
    ]
}

/// A variable's value at hour index `i`, straight from the JSON.
fn raw(json: &Value, name: &str, i: usize) -> f64 {
    json["hourly"][name][i].as_f64().unwrap()
}

/// The done-when of M5.2a: at each recorded hour, sampling the profile at a level's geometric
/// height gives back that level's pressure, temperature and wind as recorded. The ground is
/// checked the same way, with its 2 m temperature and 10 m wind.
#[test]
fn the_profile_reproduces_every_level_above_the_ground() {
    for (name, first_s) in recordings() {
        let body = fixture(name);
        let json: Value = serde_json::from_slice(&body).unwrap();
        let latitude_rad = json["latitude"].as_f64().unwrap().to_radians();
        let elevation_m = json["elevation"].as_f64().unwrap();
        for (i, time_s) in [(0, first_s), (1, first_s + HOUR_S)] {
            let profile = OpenMeteoProfile::parse(&body, time_s).unwrap();
            assert_eq!(profile.hours, vec![(time_s, 1.0)]);
            let air = profile.sounding(WindInterpolation::SpeedDirection).unwrap();
            let wind = air.wind().unwrap();
            let surface_pa = raw(&json, "surface_pressure", i) * 100.0;

            // (height, pressure, temperature, speed, direction) as recorded: the ground first.
            let mut expected = vec![(
                elevation_m,
                surface_pa,
                raw(&json, "temperature_2m", i) + 273.15,
                raw(&json, "wind_speed_10m", i),
                raw(&json, "wind_direction_10m", i),
            )];
            let mut below_ground = Vec::new();
            for p in PRESSURE_LEVELS_HPA {
                let pressure_pa = f64::from(p) * 100.0;
                let z = raw(&json, &format!("geopotential_height_{p}hPa"), i);
                let height_m = geometric_from_wmo_geopotential_m(z, latitude_rad).unwrap();
                if pressure_pa >= surface_pa || height_m <= elevation_m {
                    below_ground.push(pressure_pa);
                    continue;
                }
                expected.push((
                    height_m,
                    pressure_pa,
                    raw(&json, &format!("temperature_{p}hPa"), i) + 273.15,
                    raw(&json, &format!("wind_speed_{p}hPa"), i),
                    raw(&json, &format!("wind_direction_{p}hPa"), i),
                ));
            }
            // At this 1,400 m site the five levels from 1000 to 900 hPa are underground.
            assert_eq!(
                below_ground,
                [100_000.0, 97_500.0, 95_000.0, 92_500.0, 90_000.0]
            );
            assert_eq!(expected.len(), 15, "{name}: the ground and 14 levels");
            let dropped: Vec<_> = profile
                .dropped
                .iter()
                .map(|d| (d.pressure_pa, d.reason))
                .collect();
            let as_dropped: Vec<_> = below_ground
                .iter()
                .map(|&p| (p, DropReason::BelowGround))
                .collect();
            assert_eq!(dropped, as_dropped, "{name}");

            for (height_m, pressure_pa, temperature_k, speed, direction_deg) in expected {
                let sample = air.sample(height_m).unwrap();
                assert!(sample.extrapolated.is_none(), "{name} at {height_m} m");
                let at = format!("{name}, hour {i}, {pressure_pa} Pa");
                let rel = (sample.air.pressure_pa - pressure_pa).abs() / pressure_pa;
                assert!(rel < 1e-12, "{at}: pressure {} Pa", sample.air.pressure_pa);
                let dt = (sample.air.temperature_k - temperature_k).abs();
                assert!(
                    dt < 1e-9,
                    "{at}: temperature {} K",
                    sample.air.temperature_k
                );
                let recorded = velocity_from_speed_direction(speed, direction_deg.to_radians());
                let flown = wind.wind(height_m).unwrap().velocity_enu_m_s;
                assert!(
                    (flown - recorded).length() < 1e-9,
                    "{at}: wind {flown} against {recorded}"
                );
            }
        }
    }
}

/// Half past the hour is the mean of the two hours: temperatures and pressures directly, the wind
/// by its components.
#[test]
fn between_hours_is_linear_in_time() {
    let body = fixture("open-meteo-historical.json");
    let json: Value = serde_json::from_slice(&body).unwrap();
    let profile = OpenMeteoProfile::parse(&body, HISTORICAL_S + HOUR_S / 2).unwrap();
    assert_eq!(
        profile.hours,
        vec![(HISTORICAL_S, 0.5), (HISTORICAL_S + HOUR_S, 0.5)]
    );
    let mean = |name: &str| 0.5 * (raw(&json, name, 0) + raw(&json, name, 1));
    let velocity = |i| {
        let direction = raw(&json, "wind_direction_500hPa", i).to_radians();
        velocity_from_speed_direction(raw(&json, "wind_speed_500hPa", i), direction)
    };

    assert!((profile.surface.pressure_pa - mean("surface_pressure") * 100.0).abs() < 1e-9);
    let level = profile
        .levels
        .iter()
        .find(|l| l.pressure_pa == 50_000.0)
        .unwrap();
    assert!((level.temperature_k - (mean("temperature_500hPa") + 273.15)).abs() < 1e-12);
    assert!((level.geopotential_height_m - mean("geopotential_height_500hPa")).abs() < 1e-9);
    let expected = 0.5 * (velocity(0) + velocity(1));
    let got = velocity_from_speed_direction(level.wind_speed_m_s, level.wind_direction_from_rad);
    assert!(
        (got - expected).length() < 1e-12,
        "{got} against {expected}"
    );
}

/// The request's URL is the one recorded, so the replay answers it; a second fetch comes from the
/// cache, and offline the cache answers without the transport.
#[test]
fn fetch_goes_through_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let request = OpenMeteoRequest::new(
        LATITUDE_DEG,
        LONGITUDE_DEG,
        HISTORICAL_S + 1_800,
        OpenMeteoApi::HistoricalForecast,
    );
    let now_s = 1_790_000_000;

    let (profile, fetched) = open_meteo::fetch(&online, &request, now_s).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Fetched, 1)
    );
    assert_eq!(fetched.attribution, ATTRIBUTION);
    assert_eq!(fetched.body, fixture("open-meteo-historical.json"));
    assert_eq!(profile.levels.len(), 14);

    let (again, fetched) = open_meteo::fetch(&online, &request, now_s + 60).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Cached, 1)
    );
    assert_eq!(again, profile);

    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    let (later, fetched) = open_meteo::fetch(&offline, &request, now_s + 40 * 86_400).unwrap();
    assert_eq!(fetched.freshness, Freshness::Stale);
    assert_eq!(later, profile);

    let forecast = OpenMeteoRequest::new(
        LATITUDE_DEG,
        LONGITUDE_DEG,
        FORECAST_S,
        OpenMeteoApi::Forecast,
    );
    let (_, fetched) = open_meteo::fetch(&online, &forecast, now_s).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Fetched, 2)
    );
    match open_meteo::fetch(
        &offline,
        &OpenMeteoRequest {
            time_unix_s: FORECAST_S + 7_200,
            ..forecast
        },
        now_s,
    ) {
        Err(OpenMeteoError::Net(NetError::NotCached { .. })) => {}
        other => panic!("{other:?}"),
    }
}

/// A transport that fails the test if it is called at all.
struct Forbidden;

impl Transport for Forbidden {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        panic!("offline mode called the transport for {url}");
    }
}

/// The historical recording with one text replacement, which must hit exactly once.
fn edited(from: &str, to: &str) -> Vec<u8> {
    let text = String::from_utf8(fixture("open-meteo-historical.json")).unwrap();
    assert_eq!(text.matches(from).count(), 1, "{from}");
    text.replacen(from, to, 1).into_bytes()
}

#[test]
fn parse_refuses_what_it_cannot_read() {
    let parse = |body: &[u8]| OpenMeteoProfile::parse(body, HISTORICAL_S);
    assert!(matches!(parse(b"<html>"), Err(OpenMeteoError::Json(_))));
    let refusal = br#"{"error":true,"reason":"Latitude must be in range of -90 to 90"}"#;
    match parse(refusal) {
        Err(OpenMeteoError::Server { reason }) => assert!(reason.starts_with("Latitude")),
        other => panic!("{other:?}"),
    }
    match parse(&edited(
        r#""temperature_500hPa":"°C""#,
        r#""temperature_500hPa":"°F""#,
    )) {
        Err(OpenMeteoError::Units {
            field,
            found,
            expected,
        }) => {
            assert_eq!(
                (field.as_str(), found.as_str(), expected),
                ("temperature_500hPa", "°F", "°C")
            );
        }
        other => panic!("{other:?}"),
    }
    match parse(&edited(r#""surface_pressure":"hPa","#, "")) {
        Err(OpenMeteoError::Units { field, found, .. }) => {
            assert_eq!(
                (field.as_str(), found.as_str()),
                ("surface_pressure", "nothing")
            );
        }
        other => panic!("{other:?}"),
    }
    match parse(&edited(r#""elevation":1400.0,"#, "")) {
        Err(OpenMeteoError::Missing { field }) => assert_eq!(field, "elevation"),
        other => panic!("{other:?}"),
    }
    match OpenMeteoProfile::parse(&fixture("open-meteo-historical.json"), HISTORICAL_S - 1) {
        Err(OpenMeteoError::TimeOutside {
            first_s, last_s, ..
        }) => {
            assert_eq!((first_s, last_s), (HISTORICAL_S, HISTORICAL_S + HOUR_S));
        }
        other => panic!("{other:?}"),
    }
    let late = OpenMeteoProfile::parse(
        &fixture("open-meteo-historical.json"),
        HISTORICAL_S + HOUR_S + 1,
    );
    assert!(matches!(late, Err(OpenMeteoError::TimeOutside { .. })));
}

/// A `null` at either hour: at the surface the response is refused, on a level the level is
/// dropped as having no data.
#[test]
fn missing_values_drop_a_level_or_refuse_the_surface() {
    let json: Value = serde_json::from_slice(&fixture("open-meteo-historical.json")).unwrap();
    let nulled = |name: &str, i: usize| {
        let mut json = json.clone();
        json["hourly"][name][i] = Value::Null;
        serde_json::to_vec(&json).unwrap()
    };
    for i in [0, 1] {
        match OpenMeteoProfile::parse(&nulled("wind_direction_10m", i), HISTORICAL_S + 60) {
            Err(OpenMeteoError::NoSurface { field }) => assert_eq!(field, "wind_direction_10m"),
            other => panic!("{other:?}"),
        }
        match OpenMeteoProfile::parse(&nulled("relative_humidity_2m", i), HISTORICAL_S + 60) {
            Err(OpenMeteoError::NoSurface { field }) => assert_eq!(field, "relative_humidity_2m"),
            other => panic!("{other:?}"),
        }
        let profile =
            OpenMeteoProfile::parse(&nulled("wind_speed_300hPa", i), HISTORICAL_S + 60).unwrap();
        assert_eq!(profile.levels.len(), 13);
        assert!(profile.levels.iter().all(|l| l.pressure_pa != 30_000.0));
        let last = profile.dropped.last().unwrap();
        assert_eq!(
            (last.pressure_pa, last.reason),
            (30_000.0, DropReason::NoData)
        );
        assert!(profile.sounding(WindInterpolation::Components).is_ok());
    }
    // On the hour, the other hour's null is not read.
    let profile = OpenMeteoProfile::parse(&nulled("wind_speed_300hPa", 1), HISTORICAL_S).unwrap();
    assert_eq!(profile.levels.len(), 14);
}

/// The heights are geopotential metres, as the parser reads them. From 500 to 30 hPa (about 5.5 to
/// 24 km), each layer's thickness from the hypsometric equation, `ΔZ = (R_d/g₀) T̄_v ln(p₁/p₂)`
/// with the mean of its two levels' virtual temperatures, matches the recorded geopotential
/// heights' difference to −0.03% to −0.13% on average. Read as geometric heights, the same
/// differences would fall −0.58% to −0.68% short of the geometric thickness.
#[test]
fn recorded_heights_are_geopotential() {
    use hpr_atmos::moist::{WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL, vapour_pressure_pa};
    use hpr_atmos::profile::wmo_geopotential_from_geometric_m;
    use hpr_atmos::ussa76::{
        DRY_AIR_GAS_CONSTANT_J_PER_KG_K, SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL,
    };
    /// Standard gravity, which defines the geopotential metre, m/s².
    const G0: f64 = 9.806_65;
    let epsilon =
        WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL / SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL;
    let levels: Vec<u32> = PRESSURE_LEVELS_HPA
        .into_iter()
        .filter(|&p| p <= 500)
        .collect();
    for (name, first_s) in recordings() {
        let json: Value = serde_json::from_slice(&fixture(name)).unwrap();
        let latitude_rad = json["latitude"].as_f64().unwrap().to_radians();
        for i in [0, 1] {
            let virtual_k = |p: u32| {
                let t = raw(&json, &format!("temperature_{p}hPa"), i) + 273.15;
                let rh = raw(&json, &format!("relative_humidity_{p}hPa"), i) / 100.0;
                let e = vapour_pressure_pa(t, rh).unwrap();
                t / (1.0 - e / (f64::from(p) * 100.0) * (1.0 - epsilon))
            };
            let z = |p: u32| raw(&json, &format!("geopotential_height_{p}hPa"), i);
            let (mut as_geopotential, mut as_geometric) = (0.0, 0.0);
            for pair in levels.windows(2) {
                let (low, high) = (pair[0], pair[1]);
                let mean_k = 0.5 * (virtual_k(low) + virtual_k(high));
                let thickness = DRY_AIR_GAS_CONSTANT_J_PER_KG_K / G0
                    * mean_k
                    * (f64::from(low) / f64::from(high)).ln();
                let recorded = z(high) - z(low);
                as_geopotential += (recorded - thickness) / thickness;
                let base = wmo_geopotential_from_geometric_m(z(low), latitude_rad).unwrap();
                let top =
                    geometric_from_wmo_geopotential_m(base + thickness, latitude_rad).unwrap();
                as_geometric += (recorded - (top - z(low))) / (top - z(low));
            }
            #[expect(clippy::cast_precision_loss, reason = "nine layers")]
            let layers = (levels.len() - 1) as f64;
            let (geopotential, geometric) = (as_geopotential / layers, as_geometric / layers);
            let at = format!("{name}, hour {first_s} + {i} h");
            assert!(
                geopotential.abs() < 0.2e-2,
                "{at}: {geopotential} as geopotential"
            );
            assert!(geometric < -0.4e-2, "{at}: {geometric} as geometric");
        }
    }
}
