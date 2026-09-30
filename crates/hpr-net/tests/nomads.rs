//! GFS and RAP cuts from NOMADS against what ecCodes decodes from them (M5.2c): every field's
//! identity, every value and every grid point's position; then the profile at the site, which
//! gives back the values interpolated from ecCodes' at every level above the ground, and the fetch
//! through the cache.
//!
//! The expected values are ecCodes 2.49.0's, written by `validation/oracles/grib2/eccodes_dump.py`
//! to `tests/fixtures/nomads-eccodes.json` and read here with `serde_json`, not through the decoder
//! under test.

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
use hpr_io::grib2;
use hpr_net::nomads::{
    self, ATTRIBUTION, DropReason, NomadsError, NomadsModel, NomadsProfile, NomadsRequest,
};
use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Transport};
use serde_json::Value;

/// Spaceport America's launch area, as in the Open-Meteo tests.
const LATITUDE_DEG: f64 = 32.99;
const LONGITUDE_DEG: f64 = -106.97;
/// 2026-09-30 00:00 UTC, the GFS run recorded; its hour 18 is recorded.
const GFS_CYCLE_S: i64 = 1_790_726_400;
/// 2026-09-30 12:00 UTC, the RAP run recorded; its hour 6 is recorded.
const RAP_CYCLE_S: i64 = 1_790_769_600;
/// Both forecasts are for 2026-09-30 18:00 UTC.
const VALID_S: i64 = 1_790_791_200;

fn fixtures() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join("replay").join(name)).unwrap()
}

fn eccodes() -> Value {
    serde_json::from_slice(&std::fs::read(fixtures().join("nomads-eccodes.json")).unwrap()).unwrap()
}

fn replay() -> Replay {
    Replay::open(fixtures().join("replay")).unwrap()
}

/// The two recordings: file, request.
fn recordings() -> [(&'static str, NomadsRequest); 2] {
    [
        (
            "nomads-gfs.grib2",
            NomadsRequest::new(
                LATITUDE_DEG,
                LONGITUDE_DEG,
                NomadsModel::Gfs,
                GFS_CYCLE_S,
                18,
            ),
        ),
        (
            "nomads-rap.grib2",
            NomadsRequest::new(
                LATITUDE_DEG,
                LONGITUDE_DEG,
                NomadsModel::Rap,
                RAP_CYCLE_S,
                6,
            ),
        ),
    ]
}

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap()
}

fn int(m: &Value, key: &str) -> i64 {
    m[key].as_i64().unwrap()
}

/// The largest relative difference over every value, and the largest difference in degrees over
/// every grid point, between the decoder and ecCodes; the bounds are the measured values rounded
/// up.
#[test]
fn every_field_decodes_to_what_eccodes_prints() {
    let reference = eccodes();
    assert_eq!(reference["eccodes"], "2.49.0");
    let mut worst_value: f64 = 0.0;
    let mut worst_point: f64 = 0.0;
    let mut values = 0;
    for (name, _) in recordings() {
        let bytes = fixture(name);
        let fields = grib2::parse(&bytes).unwrap();
        let file = &reference["files"][name];
        let messages = file["messages"].as_array().unwrap();
        assert_eq!(fields.len(), messages.len(), "{name}");
        for (field, m) in fields.iter().zip(messages) {
            let at = format!("{name}, message {}: {}", field.message, m["shortName"]);
            let p = &field.product;
            assert_eq!(i64::from(field.discipline), int(m, "discipline"), "{at}");
            assert_eq!(i64::from(p.category), int(m, "parameterCategory"), "{at}");
            assert_eq!(i64::from(p.number), int(m, "parameterNumber"), "{at}");
            assert_eq!(
                i64::from(p.surface.kind),
                int(m, "typeOfFirstFixedSurface"),
                "{at}"
            );
            let level = int(m, "scaledValueOfFirstFixedSurface") as f64
                / 10_f64.powi(i32::try_from(int(m, "scaleFactorOfFirstFixedSurface")).unwrap());
            assert_eq!(p.surface.value, Some(level), "{at}");
            assert_eq!(p.forecast_time, int(m, "forecastTime"), "{at}");
            assert_eq!(
                i64::from(p.time_unit),
                int(m, "indicatorOfUnitOfTimeRange"),
                "{at}"
            );
            let t = &field.reference_time;
            let date = i64::from(t.year) * 10_000 + i64::from(t.month) * 100 + i64::from(t.day);
            assert_eq!(date, int(m, "dataDate"), "{at}");
            assert_eq!(
                i64::from(t.hour) * 100 + i64::from(t.minute),
                int(m, "dataTime"),
                "{at}"
            );
            assert_eq!(i64::from(field.grid.ni), int(m, "Ni"), "{at}");
            assert_eq!(i64::from(field.grid.nj), int(m, "Nj"), "{at}");
            assert_eq!(
                field.grid.winds_grid_relative,
                int(m, "uvRelativeToGrid") == 1,
                "{at}"
            );
            let expected = m["values"].as_array().unwrap();
            let decoded = field.values();
            assert_eq!(decoded.len(), expected.len(), "{at}");
            for (k, (d, e)) in decoded.iter().zip(expected).enumerate() {
                let (d, e) = (d.unwrap(), num(e));
                assert_eq!(field.value(k as u64).unwrap(), Some(d), "{at}, point {k}");
                worst_value = worst_value.max((d - e).abs() / e.abs().max(1e-300));
                values += 1;
            }
        }
        let grid = fields[0].grid;
        for (k, point) in file["points"].as_array().unwrap().iter().enumerate() {
            let (lat, lon) = grid.point_deg(k as u64).unwrap();
            worst_point = worst_point
                .max((lat - num(&point[0])).abs())
                .max((lon - num(&point[1])).abs());
        }
    }
    assert_eq!(values, 147 * 9 + 192 * 25);
    // Measured: 2.2e-16, one rounding (ecCodes multiplies by an inexact 10^−D, the decoder
    // divides by an exact 10^D), in IEEE arithmetic alone, so the same everywhere; 5.7e-14° on
    // macOS (GFS's points exact, RAP's to the two projections' rounding), bounded at 1e-12° since
    // RAP's go through `tan`, `powf` and `atan`, whose last digits vary between maths libraries.
    assert!(worst_value < 2.5e-16, "values: {worst_value:e}");
    assert!(worst_point < 1e-12, "points: {worst_point:e}°");
}

/// The value a field of `file` takes at the site: ecCodes' values at the profile's four grid
/// points, weighted by the profile's weights.
fn interpolated(
    file: &Value,
    profile: &NomadsProfile,
    pick: impl Fn(&Value) -> bool,
) -> Option<f64> {
    let m = file["messages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| pick(m))?;
    let values = m["values"].as_array().unwrap();
    Some(
        profile
            .grid_points
            .iter()
            .map(|g| g.weight * num(&values[usize::try_from(g.index).unwrap()]))
            .sum(),
    )
}

fn is(m: &Value, category: i64, number: i64, surface: i64, value: i64) -> bool {
    int(m, "parameterCategory") == category
        && int(m, "parameterNumber") == number
        && int(m, "typeOfFirstFixedSurface") == surface
        && int(m, "scaledValueOfFirstFixedSurface") == value
}

/// The four grid points and their weights place the site: the weighted sum of ecCodes' positions
/// of the points is the site, to the curvature of a cell's edges.
#[test]
fn the_four_grid_points_surround_the_site() {
    let reference = eccodes();
    for (name, request) in recordings() {
        let profile = NomadsProfile::parse(&fixture(name), LATITUDE_DEG, LONGITUDE_DEG).unwrap();
        let points = reference["files"][name]["points"].as_array().unwrap();
        let (mut lat, mut lon, mut total) = (0.0, 0.0, 0.0);
        for g in profile.grid_points {
            assert!((0.0..=1.0).contains(&g.weight), "{name}: {g:?}");
            let p = &points[usize::try_from(g.index).unwrap()];
            lat += g.weight * num(&p[0]);
            lon += g.weight * num(&p[1]);
            total += g.weight;
        }
        assert!(
            (total - 1.0).abs() < 1e-15,
            "{name}: weights sum to {total}"
        );
        let site_lon = LONGITUDE_DEG.rem_euclid(360.0);
        assert!(
            (lat - LATITUDE_DEG).abs() < 1e-5 && (lon - site_lon).abs() < 1e-5,
            "{name}: ({lat}, {lon})"
        );
        assert_eq!(profile.cycle_unix_s, request.cycle_unix_s, "{name}");
        assert_eq!(profile.valid_unix_s, VALID_S, "{name}");
    }
}

/// RAP's winds are along its grid. The turn is `θ = sin 25° · (λ − 265°)`, −5.06° at the site, and
/// ecCodes' own grid points show it: the grid's `+x` axis bears `90° + θ` at the midpoint of the
/// cell's lower edge, measured on the sphere from ecCodes' positions of its two ends (agreeing to
/// 2.2e-6°).
#[test]
fn rap_winds_are_turned_by_the_grids_bearing() {
    let reference = eccodes();
    let rap =
        NomadsProfile::parse(&fixture("nomads-rap.grib2"), LATITUDE_DEG, LONGITUDE_DEG).unwrap();
    let theta = 25_f64.to_radians().sin() * (LONGITUDE_DEG.rem_euclid(360.0) - 265.0).to_radians();
    assert!((rap.wind_turn_rad - theta).abs() < 1e-15);
    assert!((rap.wind_turn_rad.to_degrees() + 5.06).abs() < 0.005);
    let points = reference["files"]["nomads-rap.grib2"]["points"]
        .as_array()
        .unwrap();
    let p = |g: usize| {
        let pt = &points[usize::try_from(rap.grid_points[g].index).unwrap()];
        (num(&pt[0]).to_radians(), num(&pt[1]).to_radians())
    };
    let ((la1, lo1), (la2, lo2)) = (p(0), p(1));
    // The great circle's bearing at the edge's midpoint, as the mean of its bearing leaving the
    // first point and its bearing arriving at the second; the grid line and the great circle
    // part by their curvature, which cancels there to first order.
    let bearing = |(a1, o1): (f64, f64), (a2, o2): (f64, f64)| {
        let y = (o2 - o1).sin() * a2.cos();
        let x = a1.cos() * a2.sin() - a1.sin() * a2.cos() * (o2 - o1).cos();
        y.atan2(x)
    };
    let leaving = bearing((la1, lo1), (la2, lo2));
    let arriving = bearing((la2, lo2), (la1, lo1)) + std::f64::consts::PI;
    let mid = (leaving + arriving) / 2.0;
    let mid_lon_deg = ((lo1 + lo2) / 2.0).to_degrees();
    let expected = std::f64::consts::FRAC_PI_2
        + 25_f64.to_radians().sin() * (mid_lon_deg - 265.0).to_radians();
    assert!(
        (mid - expected).abs() < 3e-6_f64.to_radians(),
        "grid +x bears {}°, θ gives {}°",
        mid.to_degrees(),
        expected.to_degrees()
    );
    let gfs =
        NomadsProfile::parse(&fixture("nomads-gfs.grib2"), LATITUDE_DEG, LONGITUDE_DEG).unwrap();
    assert_eq!(gfs.wind_turn_rad, 0.0);
}

/// The ground and every level above it, from each cut, as a sounding: the pressure, temperature,
/// humidity and wind at each level's height are ecCodes' values interpolated to the site (RAP's
/// winds turned by `θ`). Levels at or below the ground are dropped, and listed.
#[test]
fn the_profile_reproduces_every_level_above_the_ground() {
    let reference = eccodes();
    let lat_rad = LATITUDE_DEG.to_radians();
    for (name, request) in recordings() {
        let file = &reference["files"][name];
        let profile = NomadsProfile::parse(&fixture(name), LATITUDE_DEG, LONGITUDE_DEG).unwrap();
        let air = profile.sounding(WindInterpolation::SpeedDirection).unwrap();
        let wind = air.wind().unwrap();
        let (sin, cos) = profile.wind_turn_rad.sin_cos();
        let turn = |u: f64, v: f64| (u * cos + v * sin, -u * sin + v * cos);
        let get = |c, n, s, v| interpolated(file, &profile, |m| is(m, c, n, s, v)).unwrap();

        let surface_pa = get(3, 0, 1, 0);
        let ground_m = geometric_from_wmo_geopotential_m(get(3, 5, 1, 0), lat_rad).unwrap();
        // (height, pressure, temperature, humidity %, east, north): the ground first.
        let (east, north) = turn(get(2, 2, 103, 10), get(2, 3, 103, 10));
        let mut expected = vec![(
            ground_m,
            surface_pa,
            get(0, 0, 103, 2),
            get(1, 1, 103, 2),
            east,
            north,
        )];
        let mut below_ground = Vec::new();
        for &hpa in request.model.levels_hpa() {
            let pa = i64::from(hpa) * 100;
            let height_m = geometric_from_wmo_geopotential_m(get(3, 5, 100, pa), lat_rad).unwrap();
            if pa as f64 >= surface_pa || height_m <= ground_m {
                below_ground.push(pa as f64);
                continue;
            }
            let (east, north) = turn(get(2, 2, 100, pa), get(2, 3, 100, pa));
            expected.push((
                height_m,
                pa as f64,
                get(0, 0, 100, pa),
                get(1, 1, 100, pa),
                east,
                north,
            ));
        }
        // At this 1,400 m site the six levels from 1000 to 850 (GFS) or 875 hPa (RAP) are
        // underground.
        assert_eq!(below_ground.len(), 6, "{name}: {below_ground:?}");
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
        assert_eq!(
            expected.len(),
            1 + request.model.levels_hpa().len() - 6,
            "{name}"
        );
        assert_eq!(profile.levels.len() + 1, expected.len(), "{name}");

        for (height_m, pressure_pa, temperature_k, humidity_pct, east, north) in expected {
            let at = format!("{name}, {pressure_pa} Pa");
            let sample = air.sample(height_m).unwrap();
            assert!(sample.extrapolated.is_none(), "{at}");
            let rel = (sample.air.pressure_pa - pressure_pa).abs() / pressure_pa;
            assert!(rel < 1e-13, "{at}: pressure {} Pa", sample.air.pressure_pa);
            let dt = (sample.air.temperature_k - temperature_k).abs();
            assert!(
                dt < 1e-11,
                "{at}: temperature {} K",
                sample.air.temperature_k
            );
            let level = air
                .levels()
                .iter()
                .find(|l| (l.height_msl_m - height_m).abs() < 1e-6)
                .unwrap();
            let humidity = level.relative_humidity.unwrap();
            let recorded = (humidity_pct / 100.0).clamp(0.0, 1.0);
            assert!((humidity - recorded).abs() < 1e-14, "{at}: {humidity}");
            let flown = wind.wind(height_m).unwrap().velocity_enu_m_s;
            let (speed, from) = (east.hypot(north), (-east).atan2(-north));
            let recorded = velocity_from_speed_direction(speed, from);
            assert!(
                (flown - recorded).length() < 1e-11,
                "{at}: wind {flown} against ({east}, {north})"
            );
        }
    }
}

/// The two models' forecasts for the same hour at the site, side by side: not a check of either
/// (nothing here says which is nearer the air), but a guard that they read alike. Their 2 m
/// temperatures are within 3 K, their 500 hPa heights within 30 m and their 500 hPa winds within
/// 5 m/s of each other.
#[test]
fn gfs_and_rap_read_alike_for_the_same_hour() {
    let gfs =
        NomadsProfile::parse(&fixture("nomads-gfs.grib2"), LATITUDE_DEG, LONGITUDE_DEG).unwrap();
    let rap =
        NomadsProfile::parse(&fixture("nomads-rap.grib2"), LATITUDE_DEG, LONGITUDE_DEG).unwrap();
    assert_eq!(gfs.valid_unix_s, rap.valid_unix_s);
    assert!((gfs.surface.temperature_k - rap.surface.temperature_k).abs() < 3.0);
    let at_500 = |p: &NomadsProfile| *p.levels.iter().find(|l| l.pressure_pa == 50_000.0).unwrap();
    let (g, r) = (at_500(&gfs), at_500(&rap));
    assert!(
        (g.height_msl_m - r.height_msl_m).abs() < 30.0,
        "{g:?} {r:?}"
    );
    let dw = (g.wind_east_m_s - r.wind_east_m_s).hypot(g.wind_north_m_s - r.wind_north_m_s);
    assert!(dw < 5.0, "{g:?} {r:?}");
}

/// A transport that fails the test if it is called at all.
struct Forbidden;

impl Transport for Forbidden {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        panic!("offline mode called the transport for {url}");
    }
}

#[test]
fn fetch_goes_through_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let now_s = 1_790_800_000;
    let [(gfs_file, gfs), (rap_file, rap)] = recordings();

    let (profile, fetched) = nomads::fetch(&online, &gfs, now_s).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Fetched, 1)
    );
    assert_eq!(fetched.attribution, ATTRIBUTION);
    assert_eq!(fetched.body, fixture(gfs_file));
    assert_eq!(profile.levels.len(), 22);

    let (again, fetched) = nomads::fetch(&online, &gfs, now_s + 60).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Cached, 1)
    );
    assert_eq!(again, profile);

    let (_, fetched) = nomads::fetch(&online, &rap, now_s).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Fetched, 2)
    );
    assert_eq!(fetched.body, fixture(rap_file));

    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    let (later, fetched) = nomads::fetch(&offline, &gfs, now_s + 40 * 86_400).unwrap();
    assert_eq!(fetched.freshness, Freshness::Stale);
    assert_eq!(later, profile);

    let mut uncached = gfs.clone();
    uncached.forecast_hour = 19;
    match nomads::fetch(&offline, &uncached, now_s) {
        Err(NomadsError::Net(NetError::NotCached { .. })) => {}
        other => panic!("{other:?}"),
    }
}

/// A server that answers with another run's cut (here the recorded GFS cut, served for the run
/// six hours before at hour 24, which is for the same time) is refused, and nothing is cached.
#[test]
fn a_cut_of_another_run_is_refused_and_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let mut request = NomadsRequest::new(
        LATITUDE_DEG,
        LONGITUDE_DEG,
        NomadsModel::Gfs,
        GFS_CYCLE_S - 6 * 3_600,
        24,
    );
    request.endpoint = Some("https://example.test/cgi-bin".to_owned());
    assert_eq!(request.valid_unix_s(), VALID_S);
    match nomads::fetch(&online, &request, 1_790_800_000) {
        Err(NomadsError::Net(NetError::Refused { reason, .. })) => {
            assert!(reason.contains("not the run of 1790704800 s"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    assert!(matches!(
        nomads::fetch(&offline, &request, 1_790_800_000),
        Err(NomadsError::Net(NetError::NotCached { .. }))
    ));
}

#[test]
fn requests_out_of_range_are_refused() {
    let base = NomadsRequest::new(
        LATITUDE_DEG,
        LONGITUDE_DEG,
        NomadsModel::Gfs,
        GFS_CYCLE_S,
        18,
    );
    let refused = |r: &NomadsRequest| match r.url() {
        Err(NomadsError::Request { what, .. }) => what,
        other => panic!("{other:?}"),
    };
    let mut r = base.clone();
    r.cycle_unix_s += 3_600; // 01 UTC: GFS runs every 6 hours
    assert_eq!(refused(&r), "cycle (s since 1970)");
    r.model = NomadsModel::Rap; // RAP runs every hour
    assert!(r.url().is_ok());
    r.forecast_hour = 52;
    assert_eq!(refused(&r), "forecast hour");
    let mut r = base.clone();
    r.forecast_hour = 385;
    assert_eq!(refused(&r), "forecast hour");
    // GFS: every hour to 120, then every third.
    r.forecast_hour = 121;
    assert_eq!(refused(&r), "forecast hour");
    r.forecast_hour = 123;
    assert!(r.url().is_ok());
    // RAP: to 21 hours, and to 51 from the 03, 09, 15 and 21 UTC runs.
    let mut r = NomadsRequest::new(
        LATITUDE_DEG,
        LONGITUDE_DEG,
        NomadsModel::Rap,
        RAP_CYCLE_S,
        22,
    );
    assert_eq!(refused(&r), "forecast hour");
    r.cycle_unix_s = RAP_CYCLE_S + 3 * 3_600; // 15 UTC
    r.forecast_hour = 51;
    assert!(r.url().is_ok());
    let mut r = base.clone();
    r.latitude_deg = 90.5;
    assert_eq!(refused(&r), "latitude (deg)");
    let mut r = base.clone();
    r.longitude_deg = f64::NAN;
    assert_eq!(refused(&r), "longitude (deg)");
    let mut r = base.clone();
    r.cycle_unix_s = -21_600;
    assert_eq!(refused(&r), "cycle (s since 1970)");
    let mut r = base.clone();
    r.endpoint = Some("https://example.test/?x".to_owned());
    assert_eq!(refused(&r), "endpoint");
}

/// A site outside the cut, and cuts missing a ground value or mixing runs, are refused.
#[test]
fn parse_refuses_what_it_cannot_read() {
    let gfs = fixture("nomads-gfs.grib2");
    assert!(matches!(
        NomadsProfile::parse(&gfs, 34.0, LONGITUDE_DEG),
        Err(NomadsError::Outside { .. })
    ));
    assert!(matches!(
        NomadsProfile::parse(&gfs, LATITUDE_DEG, -105.0),
        Err(NomadsError::Outside { .. })
    ));
    assert!(matches!(
        NomadsProfile::parse(b"<html>", LATITUDE_DEG, LONGITUDE_DEG),
        Err(NomadsError::Grib(_))
    ));
    assert!(matches!(
        NomadsProfile::parse(&[], LATITUDE_DEG, LONGITUDE_DEG),
        Err(NomadsError::NoSurface { .. })
    ));

    // Without the surface pressure the cut is refused; messages are cut at their recorded
    // lengths.
    let messages = split(&gfs);
    let fields: Vec<_> = messages
        .iter()
        .map(|m| grib2::parse(m).unwrap()[0].product)
        .collect();
    let surface_pressure = fields
        .iter()
        .position(|p| (p.category, p.number, p.surface.kind) == (3, 0, 1))
        .unwrap();
    let without: Vec<u8> = messages
        .iter()
        .enumerate()
        .filter(|(k, _)| *k != surface_pressure)
        .flat_map(|(_, m)| m.iter().copied())
        .collect();
    assert!(matches!(
        NomadsProfile::parse(&without, LATITUDE_DEG, LONGITUDE_DEG),
        Err(NomadsError::NoSurface {
            what: "surface pressure"
        })
    ));
    // A level's temperature left out drops that level as NoData.
    let t500 = fields
        .iter()
        .position(|p| {
            (p.category, p.number, p.surface.kind, p.surface.value) == (0, 0, 100, Some(50_000.0))
        })
        .unwrap();
    let rh500 = fields
        .iter()
        .position(|p| {
            (p.category, p.number, p.surface.kind, p.surface.value) == (1, 1, 100, Some(50_000.0))
        })
        .unwrap();
    let without: Vec<u8> = messages
        .iter()
        .enumerate()
        .filter(|(k, _)| *k != rh500)
        .flat_map(|(_, m)| m.iter().copied())
        .collect();
    let profile = NomadsProfile::parse(&without, LATITUDE_DEG, LONGITUDE_DEG).unwrap();
    assert!(
        profile
            .dropped
            .iter()
            .any(|d| d.pressure_pa == 50_000.0 && d.reason == DropReason::NoData)
    );
    // A field given twice.
    let mut twice = gfs.clone();
    twice.extend_from_slice(&messages[t500]);
    assert!(matches!(
        NomadsProfile::parse(&twice, LATITUDE_DEG, LONGITUDE_DEG),
        Err(NomadsError::Duplicate {
            category: 0,
            number: 0,
            surface: 100
        })
    ));
    // A RAP message among GFS's: another grid.
    let rap = fixture("nomads-rap.grib2");
    let mut mixed = gfs.clone();
    mixed.extend_from_slice(&split(&rap)[0]);
    assert!(matches!(
        NomadsProfile::parse(&mixed, LATITUDE_DEG, LONGITUDE_DEG),
        Err(NomadsError::Mixed { what: "grid" })
    ));
}

/// A file's messages, each by its section 0 length.
fn split(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let length = u64::from_be_bytes(bytes[at + 8..at + 16].try_into().unwrap());
        let length = usize::try_from(length).unwrap();
        out.push(bytes[at..at + length].to_vec());
        at += length;
    }
    out
}

/// The heights are geopotential metres, as GRIB2 defines `HGT` and the parser reads them. From
/// 500 hPa up, each layer's thickness from the hypsometric equation, `ΔZ = (R_d/g₀) T̄_v ln(p₁/p₂)`
/// with the mean of its two levels' virtual temperatures, matches the difference of the profile's
/// geopotential heights on average to −0.002% (GFS, 15 layers to 10 hPa) and −0.08% (RAP, 16 layers
/// to 100 hPa); read as geometric heights, the layers would be 0.63% and 0.51% thinner than the
/// geometric thickness. The test holds each mean to those figures as rounded.
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
    let latitude_rad = LATITUDE_DEG.to_radians();
    for (name, _) in recordings() {
        let profile = NomadsProfile::parse(&fixture(name), LATITUDE_DEG, LONGITUDE_DEG).unwrap();
        let levels: Vec<_> = profile
            .levels
            .iter()
            .filter(|l| l.pressure_pa <= 50_000.0)
            .collect();
        let virtual_k = |l: &nomads::NomadsLevel| {
            let e = vapour_pressure_pa(l.temperature_k, l.relative_humidity.min(1.0)).unwrap();
            l.temperature_k / (1.0 - e / l.pressure_pa * (1.0 - epsilon))
        };
        let (mut as_geopotential, mut as_geometric) = (0.0, 0.0);
        for pair in levels.windows(2) {
            let (low, high) = (pair[0], pair[1]);
            let mean_k = 0.5 * (virtual_k(low) + virtual_k(high));
            let thickness = DRY_AIR_GAS_CONSTANT_J_PER_KG_K / G0
                * mean_k
                * (low.pressure_pa / high.pressure_pa).ln();
            let recorded = high.geopotential_height_m - low.geopotential_height_m;
            as_geopotential += (recorded - thickness) / thickness;
            let z_low = low.geopotential_height_m;
            let base = wmo_geopotential_from_geometric_m(z_low, latitude_rad).unwrap();
            let top = geometric_from_wmo_geopotential_m(base + thickness, latitude_rad).unwrap();
            as_geometric += (recorded - (top - z_low)) / (top - z_low);
        }
        #[expect(clippy::cast_precision_loss, reason = "a few dozen layers")]
        let layers = (levels.len() - 1) as f64;
        let (geopotential, geometric) = (as_geopotential / layers, as_geometric / layers);
        // The means to the rounding quoted: GFS −0.002% and −0.63%, RAP −0.08% and −0.51%.
        let (geopotential_range, geometric_range) = match name {
            "nomads-gfs.grib2" => (-0.0025e-2..-0.0015e-2, -0.635e-2..-0.625e-2),
            _ => (-0.085e-2..-0.075e-2, -0.515e-2..-0.505e-2),
        };
        assert!(
            geopotential_range.contains(&geopotential),
            "{name}: {geopotential} as geopotential"
        );
        assert!(
            geometric_range.contains(&geometric),
            "{name}: {geometric} as geometric"
        );
    }
}

/// A RAP request answered with a GFS cut for the same run and hour (here the recorded GFS cut,
/// served for RAP's 00 UTC run at hour 18) is refused, and nothing is cached.
#[test]
fn a_cut_of_the_other_model_is_refused_and_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let mut request = NomadsRequest::new(
        LATITUDE_DEG,
        LONGITUDE_DEG,
        NomadsModel::Rap,
        GFS_CYCLE_S,
        18,
    );
    request.endpoint = Some("https://example.test/cgi-bin".to_owned());
    match nomads::fetch(&online, &request, 1_790_800_000) {
        Err(NomadsError::Net(NetError::Refused { reason, .. })) => {
            assert!(reason.contains("not Rap's"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    assert!(matches!(
        nomads::fetch(&offline, &request, 1_790_800_000),
        Err(NomadsError::Net(NetError::NotCached { .. }))
    ));
}

/// The recorded message `m` with its section 6 replaced by a bitmap marking every point but
/// `missing`, and section 5's count of packed values one less; the packed values are kept (a few
/// more than needed, which the format allows).
fn with_point_missing(m: &[u8], points: u64, missing: u64) -> Vec<u8> {
    let mut out = m[..16].to_vec();
    let mut at = 16;
    while &m[at..at + 4] != b"7777" {
        let length =
            usize::try_from(u32::from_be_bytes(m[at..at + 4].try_into().unwrap())).unwrap();
        let mut s = m[at..at + length].to_vec();
        match s[4] {
            5 => {
                let count = u32::from_be_bytes(s[5..9].try_into().unwrap());
                s[5..9].copy_from_slice(&(count - 1).to_be_bytes());
            }
            6 => {
                assert_eq!(s[5], 255, "the recording has no bitmap");
                let bytes = usize::try_from(points.div_ceil(8)).unwrap();
                let mut bits = vec![0xFF_u8; bytes];
                let k = usize::try_from(missing).unwrap();
                bits[k / 8] &= !(0x80 >> (k % 8));
                s = [
                    &u32::try_from(6 + bytes).unwrap().to_be_bytes()[..],
                    &[6, 0],
                    &bits,
                ]
                .concat();
            }
            _ => {}
        }
        out.extend_from_slice(&s);
        at += length;
    }
    out.extend_from_slice(b"7777");
    let total = out.len() as u64;
    out[8..16].copy_from_slice(&total.to_be_bytes());
    out
}

/// A level whose field has no value at one of the four points around the site is dropped as
/// `NoData`; the others read as before.
#[test]
fn a_point_missing_around_the_site_drops_its_level() {
    let gfs = fixture("nomads-gfs.grib2");
    let before = NomadsProfile::parse(&gfs, LATITUDE_DEG, LONGITUDE_DEG).unwrap();
    let corner = before.grid_points[3].index;
    let messages = split(&gfs);
    let edited: Vec<u8> = messages
        .iter()
        .flat_map(|m| {
            let p = grib2::parse(m).unwrap()[0].product;
            if (p.category, p.number, p.surface.kind, p.surface.value)
                == (1, 1, 100, Some(50_000.0))
            {
                with_point_missing(m, 9, corner)
            } else {
                m.clone()
            }
        })
        .collect();
    let rh = grib2::parse(&edited)
        .unwrap()
        .into_iter()
        .find(|f| f.product.category == 1 && f.product.surface.value == Some(50_000.0))
        .unwrap();
    assert_eq!(rh.value(corner).unwrap(), None);
    let after = NomadsProfile::parse(&edited, LATITUDE_DEG, LONGITUDE_DEG).unwrap();
    assert!(
        after
            .dropped
            .iter()
            .any(|d| d.pressure_pa == 50_000.0 && d.reason == DropReason::NoData)
    );
    assert_eq!(after.levels.len(), before.levels.len() - 1);
    let kept: Vec<_> = before
        .levels
        .iter()
        .filter(|l| l.pressure_pa != 50_000.0)
        .collect();
    assert!(after.levels.iter().zip(kept).all(|(a, b)| a == b));
}

/// A cut of more than `MAX_FIELDS` fields is refused before it is read: here the recorded GFS cut
/// with 900 copies of its 500 hPa temperature, each moved to a level of its own.
#[test]
fn a_cut_of_too_many_fields_is_refused() {
    let gfs = fixture("nomads-gfs.grib2");
    let messages = split(&gfs);
    let t500 = messages
        .iter()
        .find(|m| {
            let p = grib2::parse(m).unwrap()[0].product;
            (p.category, p.number, p.surface.value) == (0, 0, Some(50_000.0))
        })
        .unwrap();
    let mut body = gfs.clone();
    for level in 0..900_u32 {
        let mut m = t500.clone();
        // Section 4's first fixed surface's scaled value, after sections 1 and 3.
        let mut at = 16;
        while m[at + 4] != 4 {
            at += usize::try_from(u32::from_be_bytes(m[at..at + 4].try_into().unwrap())).unwrap();
        }
        m[at + 24..at + 28].copy_from_slice(&(1_000 + level).to_be_bytes());
        body.extend_from_slice(&m);
    }
    assert!(matches!(
        NomadsProfile::parse(&body, LATITUDE_DEG, LONGITUDE_DEG),
        Err(NomadsError::TooManyFields { count: 1_047 })
    ));
    assert_eq!(nomads::MAX_FIELDS, 1_000);
}

/// Every message of `bytes` with `new` written at byte `offset` of its section `number`.
fn patched(bytes: &[u8], number: u8, offset: usize, new: &[u8]) -> Vec<u8> {
    split(bytes)
        .into_iter()
        .flat_map(|mut m| {
            let mut at = 16;
            while m[at + 4] != number {
                at +=
                    usize::try_from(u32::from_be_bytes(m[at..at + 4].try_into().unwrap())).unwrap();
            }
            m[at + offset..at + offset + new.len()].copy_from_slice(new);
            m
        })
        .collect()
}

/// Each model's grid is its own; a cut on another grid, even a self-consistent one (here GFS's
/// with its steps made 0.5°, which still holds the site), reads but is not the model's, so `fetch`
/// refuses it.
#[test]
fn each_model_knows_its_grid() {
    let gfs = fixture("nomads-gfs.grib2");
    let rap = fixture("nomads-rap.grib2");
    let grid = |b: &[u8]| {
        NomadsProfile::parse(b, LATITUDE_DEG, LONGITUDE_DEG)
            .unwrap()
            .grid
    };
    assert!(NomadsModel::Gfs.has_grid(&grid(&gfs)));
    assert!(NomadsModel::Rap.has_grid(&grid(&rap)));
    assert!(!NomadsModel::Rap.has_grid(&grid(&gfs)));
    assert!(!NomadsModel::Gfs.has_grid(&grid(&rap)));
    // Template 3.0's Di and Dj, octets 64 to 71 of section 3.
    let half = [500_000_u32.to_be_bytes(), 500_000_u32.to_be_bytes()].concat();
    let coarse = patched(&gfs, 3, 63, &half);
    assert!(!NomadsModel::Gfs.has_grid(&grid(&coarse)));
}

/// A forecast time in a unit the reader doesn't convert is refused by name.
#[test]
fn an_unknown_time_unit_is_refused() {
    // Template 4.0's unit of time range, octet 18 of section 4.
    let odd = patched(&fixture("nomads-gfs.grib2"), 4, 17, &[99]);
    assert!(matches!(
        NomadsProfile::parse(&odd, LATITUDE_DEG, LONGITUDE_DEG),
        Err(NomadsError::TimeUnit { code: 99 })
    ));
}
