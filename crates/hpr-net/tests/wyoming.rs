//! University of Wyoming soundings against recorded answers (M5.2b): the profile reproduces the
//! pressure, temperature and wind at every level it keeps, and the fetch goes through the cache
//! and its offline mode.
//!
//! The expected values are read from the recordings here, by splitting the lines on commas, not
//! through the parser under test.

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
use hpr_net::wyoming::{
    self, ATTRIBUTION, DropReason, WyomingError, WyomingRequest, WyomingSounding, WyomingSource,
};
use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Transport};

/// 2025-06-21 12:00 UTC: Santa Teresa's recordings.
const JUNE_S: i64 = 1_750_507_200;
/// 2025-01-15 12:00 UTC: Salt Lake City's recording.
const JANUARY_S: i64 = 1_736_942_400;
const FM35: &str = "wyoming-72364-fm35.csv";
const BUFR: &str = "wyoming-72364-bufr.csv";
const WINTER: &str = "wyoming-72572-fm35.csv";

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

/// A recording's rows, each split into its 13 trimmed fields, with its line number.
fn rows(name: &str) -> Vec<(usize, Vec<String>)> {
    let text = String::from_utf8(fixture(name)).unwrap();
    text.lines()
        .enumerate()
        .skip(1)
        .map(|(i, line)| {
            (
                i + 1,
                line.split(',').map(|f| f.trim().to_owned()).collect(),
            )
        })
        .collect()
}

/// Field `i` of a row as a number, or `None` when empty.
fn field(row: &[String], i: usize) -> Option<f64> {
    (!row[i].is_empty()).then(|| row[i].parse().unwrap())
}

/// The done-when of M5.2b: a row is kept when it has every value and lies above, at a lower
/// pressure than, the last row kept; sampling the profile at a kept row's geometric height gives
/// back its pressure, temperature, humidity and wind as recorded. The rows dropped are exactly the
/// others: in the coded messages the top row, which has no wind; in the BUFR file the 1,931 rows
/// whose pressure, to 0.1 hPa, repeats the one below.
#[test]
fn the_profile_reproduces_every_level_it_keeps() {
    // (recording, rows, kept, rows missing a value, rows not above)
    for (name, total, kept, no_data, not_above) in [
        (FM35, 228, 227, vec![229], vec![]),
        (WINTER, 242, 241, vec![243], vec![]),
        (BUFR, 5_851, 3_920, vec![], (0..1_931).collect::<Vec<_>>()),
    ] {
        let rows = rows(name);
        assert_eq!(rows.len(), total, "{name}");
        let latitude_rad = field(&rows[0].1, 2).unwrap().to_radians();

        // (height, pressure, temperature, humidity, speed, direction) as recorded.
        let mut expected: Vec<(f64, f64, f64, f64, f64, f64)> = Vec::new();
        let (mut missing, mut repeated) = (Vec::new(), Vec::new());
        for (line, row) in &rows {
            let values: Option<Vec<f64>> = [3, 4, 5, 8, 12, 11]
                .into_iter()
                .map(|i| field(row, i))
                .collect();
            let Some(v) = values else {
                missing.push(*line);
                continue;
            };
            let height_m = geometric_from_wmo_geopotential_m(v[1], latitude_rad).unwrap();
            let pressure_pa = v[0] * 100.0;
            if let Some(below) = expected.last()
                && (height_m <= below.0 || pressure_pa >= below.1)
            {
                repeated.push(*line);
                continue;
            }
            expected.push((height_m, pressure_pa, v[2] + 273.15, v[3], v[4], v[5]));
        }
        assert_eq!(missing, no_data, "{name}");
        assert_eq!(repeated.len(), not_above.len(), "{name}");
        assert_eq!(expected.len(), kept, "{name}");

        let sounding = WyomingSounding::parse(&fixture(name)).unwrap();
        let dropped: Vec<_> = sounding
            .dropped
            .iter()
            .map(|d| (d.line, d.reason))
            .collect();
        let mut as_dropped: Vec<_> = missing
            .iter()
            .map(|&l| (l, DropReason::NoData))
            .chain(repeated.iter().map(|&l| (l, DropReason::NotAbove)))
            .collect();
        as_dropped.sort_by_key(|d| d.0);
        assert_eq!(dropped, as_dropped, "{name}");
        assert_eq!(sounding.levels.len(), kept, "{name}");

        let air = sounding
            .sounding(WindInterpolation::SpeedDirection)
            .unwrap();
        let wind = air.wind().unwrap();
        for (height_m, pressure_pa, temperature_k, humidity_pct, speed, direction_deg) in expected {
            let at = format!("{name}, {pressure_pa} Pa at {height_m} m");
            let sample = air.sample(height_m).unwrap();
            assert!(sample.extrapolated.is_none(), "{at}");
            let rel = (sample.air.pressure_pa - pressure_pa).abs() / pressure_pa;
            assert!(rel < 1e-12, "{at}: pressure {} Pa", sample.air.pressure_pa);
            let dt = (sample.air.temperature_k - temperature_k).abs();
            assert!(
                dt < 1e-9,
                "{at}: temperature {} K",
                sample.air.temperature_k
            );
            let level = air.levels().iter().find(|l| l.height_msl_m == height_m);
            let humidity = level.and_then(|l| l.relative_humidity).unwrap();
            assert!(
                (humidity - humidity_pct / 100.0).abs() < 1e-15,
                "{at}: {humidity}"
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

/// The ground and the release: the first row, as recorded.
#[test]
fn the_first_row_is_the_ground() {
    let sounding = WyomingSounding::parse(&fixture(FM35)).unwrap();
    assert_eq!(
        (sounding.latitude_deg, sounding.longitude_deg),
        (31.86, -106.7)
    );
    // 2025-06-21 11:02:00 UTC, 58 minutes before the nominal 12 UTC.
    assert_eq!(sounding.release_unix_s, JUNE_S - 58 * 60);
    let ground = sounding.levels[0];
    assert_eq!(ground.pressure_pa, 87_200.0);
    assert_eq!(ground.geopotential_height_m, 1_252.0);
    assert!((ground.temperature_k - 301.55).abs() < 1e-12);
    assert_eq!(ground.wind_speed_m_s, 5.7);
    let winter = WyomingSounding::parse(&fixture(WINTER)).unwrap();
    assert_eq!(winter.release_unix_s, JANUARY_S - 55 * 60);
}

/// The heights are geopotential metres, as the column's name says. Across the 13 layers between
/// the standard levels from 850 to 10 hPa, the recorded thickness matches the hypsometric
/// thickness from the rows' virtual temperatures (with the file's own mixing ratios) to within
/// 0.02% to 0.07% on average, the recorded layers a little thin. Read as geometric heights, the
/// recorded layers would be 0.51% to 0.64% thinner than the geometric thickness. The asserts hold
/// each mean to the quoted range, widened by half a unit of its last digit.
#[test]
fn recorded_heights_are_geopotential() {
    use hpr_atmos::moist::WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL;
    use hpr_atmos::profile::wmo_geopotential_from_geometric_m;
    use hpr_atmos::ussa76::{
        DRY_AIR_GAS_CONSTANT_J_PER_KG_K, SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL,
    };
    /// Standard gravity, which defines the geopotential metre, m/s².
    const G0: f64 = 9.806_65;
    const STANDARD_HPA: [f64; 14] = [
        850.0, 700.0, 500.0, 400.0, 300.0, 250.0, 200.0, 150.0, 100.0, 70.0, 50.0, 30.0, 20.0, 10.0,
    ];
    let epsilon =
        WATER_VAPOUR_MOLECULAR_WEIGHT_KG_PER_KMOL / SEA_LEVEL_MOLECULAR_WEIGHT_KG_PER_KMOL;
    for name in [FM35, WINTER, BUFR] {
        // (pressure hPa, geopotential m, virtual temperature K) of the rows kept.
        let mut kept: Vec<(f64, f64, f64)> = Vec::new();
        for (_, row) in rows(name) {
            let (Some(p), Some(z), Some(t), Some(w)) = (
                field(&row, 3),
                field(&row, 4),
                field(&row, 5),
                field(&row, 10),
            ) else {
                continue;
            };
            if kept.last().is_some_and(|b| z <= b.1 || p >= b.0) {
                continue;
            }
            let w = w / 1_000.0;
            kept.push((p, z, (t + 273.15) * (1.0 + w / epsilon) / (1.0 + w)));
        }
        let latitude_rad = field(&rows(name)[0].1, 2).unwrap().to_radians();
        let at = |p: f64| kept.iter().position(|k| k.0 == p).unwrap();
        let (mut as_geopotential, mut as_geometric) = (0.0, 0.0);
        for pair in STANDARD_HPA.windows(2) {
            let (low, high) = (at(pair[0]), at(pair[1]));
            let thickness: f64 = kept[low..=high]
                .windows(2)
                .map(|l| {
                    DRY_AIR_GAS_CONSTANT_J_PER_KG_K / G0
                        * 0.5
                        * (l[0].2 + l[1].2)
                        * (l[0].0 / l[1].0).ln()
                })
                .sum();
            let recorded = kept[high].1 - kept[low].1;
            as_geopotential += (recorded - thickness) / thickness;
            let base = wmo_geopotential_from_geometric_m(kept[low].1, latitude_rad).unwrap();
            let top = geometric_from_wmo_geopotential_m(base + thickness, latitude_rad).unwrap();
            as_geometric += (recorded - (top - kept[low].1)) / (top - kept[low].1);
        }
        let layers = 13.0;
        let (geopotential, geometric) = (as_geopotential / layers, as_geometric / layers);
        assert!(
            (-0.075e-2..-0.020e-2).contains(&geopotential),
            "{name}: {geopotential} as geopotential"
        );
        assert!(
            (-0.65e-2..-0.50e-2).contains(&geometric),
            "{name}: {geometric} as geometric"
        );
    }
}

/// The request's URL is the one recorded, so the replay answers it; a second fetch comes from the
/// cache, and offline the cache answers without the transport, stale after a day.
#[test]
fn fetch_goes_through_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    // A launch at 15:30 UTC had the 12 UTC sounding.
    let request = WyomingRequest::latest_before("72364", JUNE_S + 3 * 3_600 + 1_800);
    assert_eq!(request.time_unix_s, JUNE_S);
    let now_s = 1_790_000_000;

    let (sounding, fetched) = wyoming::fetch(&online, &request, now_s).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Fetched, 1)
    );
    assert_eq!(fetched.attribution, ATTRIBUTION);
    assert_eq!(fetched.body, fixture(FM35));
    assert_eq!(sounding.levels.len(), 227);

    let (again, fetched) = wyoming::fetch(&online, &request, now_s + 60).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Cached, 1)
    );
    assert_eq!(again, sounding);

    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    let (later, fetched) = wyoming::fetch(&offline, &request, now_s + 86_401).unwrap();
    assert_eq!(fetched.freshness, Freshness::Stale);
    assert_eq!(later, sounding);

    let mut bufr = request.clone();
    bufr.source = WyomingSource::Bufr;
    let (detailed, fetched) = wyoming::fetch(&online, &bufr, now_s).unwrap();
    assert_eq!(
        (fetched.freshness, transport.calls()),
        (Freshness::Fetched, 2)
    );
    assert_eq!(detailed.levels.len(), 3_920);

    let (_, fetched) =
        wyoming::fetch(&online, &WyomingRequest::new("72572", JANUARY_S), now_s).unwrap();
    assert_eq!(fetched.body, fixture(WINTER));

    let uncached = WyomingRequest::new("72364", JUNE_S + 12 * 3_600);
    match wyoming::fetch(&offline, &uncached, now_s) {
        Err(WyomingError::Net(NetError::NotCached { .. })) => {}
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

/// A transport that answers every URL with the same bytes.
struct Canned(Vec<u8>);

impl Transport for Canned {
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        Ok(self.0.clone())
    }
}

/// The coded-message recording with one text replacement, which must hit exactly once.
fn edited(from: &str, to: &str) -> Vec<u8> {
    let text = String::from_utf8(fixture(FM35)).unwrap();
    assert_eq!(text.matches(from).count(), 1, "{from}");
    text.replacen(from, to, 1).into_bytes()
}

/// The first row, and a row at 557 hPa, as recorded.
const GROUND: &str = "2025-06-21 11:02:00,-106.7000,31.8600, 872.0, 1252, 28.4,  9.4,  9.4, 31, \
                      31, 8.52,265, 5.7";
const ROW_557: &str = " 557.0, 5035, -1.7, -4.9, -4.3, 79, 80, 4.79,200, 9.8";

#[test]
fn parse_refuses_what_it_cannot_read() {
    let parse = |body: &[u8]| WyomingSounding::parse(body);
    let missing = |body: &[u8]| match parse(body) {
        Err(WyomingError::Missing { field }) => field,
        other => panic!("{other:?}"),
    };
    assert_eq!(missing(b""), "header");
    assert_eq!(missing(b"<html><body>Oops</body></html>\n"), "time");
    assert_eq!(missing(&[0xff, 0xfe]), "header");
    assert_eq!(
        missing(&edited("temperature_C,dew", "temp_C,dew")),
        "temperature"
    );

    match parse(&edited("wind speed_m/s", "wind speed_knot")) {
        Err(WyomingError::Units {
            field,
            found,
            expected,
        }) => assert_eq!(
            (field, found.as_str(), expected),
            ("wind speed", "knot", "m/s")
        ),
        other => panic!("{other:?}"),
    }
    match parse(&edited(
        "pressure_hPa",
        &format!("pressure_{}", "Pa".repeat(500)),
    )) {
        Err(WyomingError::Units { found, .. }) => assert_eq!(found.chars().count(), 41),
        other => panic!("{other:?}"),
    }

    let row = |body: &[u8]| match parse(body) {
        Err(WyomingError::Row { line, reason }) => (line, reason),
        other => panic!("{other:?}"),
    };
    let (line, reason) = row(&edited(ROW_557, &format!("{ROW_557},1")));
    assert_eq!((line, reason.as_str()), (38, "14 fields, not 13"));
    let (line, reason) = row(&edited(ROW_557, &ROW_557.replace("557.0", "55x.0")));
    assert_eq!(line, 38);
    assert!(reason.contains("pressure \"55x.0\""), "{reason}");
    let (_, reason) = row(&edited(ROW_557, &ROW_557.replace("-1.7", "inf")));
    assert!(reason.contains("temperature"), "{reason}");
    let (line, reason) = row(&edited(
        "2025-06-21 11:02:00,-106.7000,31.8600, 872.0",
        "2025-06-21,-106.7000,31.8600, 872.0",
    ));
    assert_eq!(line, 2);
    assert!(reason.contains("time \"2025-06-21\""), "{reason}");

    let header = String::from_utf8(fixture(FM35)).unwrap();
    let header = header.lines().next().unwrap();
    assert!(matches!(
        parse(format!("{header}\n\n").as_bytes()),
        Err(WyomingError::NoGround { line: 2 })
    ));
    let no_ground_wind = GROUND.replace("265, 5.7", "   ,    ");
    assert!(matches!(
        parse(&edited(GROUND, &no_ground_wind)),
        Err(WyomingError::NoGround { line: 2 })
    ));
    let dry_ground = GROUND.replacen(" 31,", " -1,", 1);
    assert!(matches!(
        parse(&edited(GROUND, &dry_ground)),
        Err(WyomingError::NoGround { line: 2 })
    ));
}

/// A row's own faults drop it, with the reason, and leave the rest: a missing temperature, a
/// humidity below zero, and a row below the one before it. A humidity above 100% (radiosondes
/// report it in cloud) is kept as recorded and taken as 100% in the profile.
#[test]
fn a_rows_faults_drop_it_or_are_clamped() {
    let reasons = |body: Vec<u8>| {
        let sounding = WyomingSounding::parse(&body).unwrap();
        sounding
            .dropped
            .iter()
            .map(|d| (d.line, d.reason))
            .collect::<Vec<_>>()
    };
    let top = (229, DropReason::NoData);
    let no_temperature = ROW_557.replace(" -1.7,", "     ,");
    assert_eq!(
        reasons(edited(ROW_557, &no_temperature)),
        [(38, DropReason::NoData), top]
    );
    let negative = ROW_557.replace(" 79,", " -1,");
    assert_eq!(
        reasons(edited(ROW_557, &negative)),
        [(38, DropReason::Humidity), top]
    );
    // 557 hPa at 4,000 m lies below the 570 hPa row before it; and at 570 hPa, not above it.
    let low = ROW_557.replace(" 5035,", " 4000,");
    assert_eq!(
        reasons(edited(ROW_557, &low)),
        [(38, DropReason::NotAbove), top]
    );
    let same = ROW_557.replace(" 557.0,", " 570.0,");
    assert_eq!(
        reasons(edited(ROW_557, &same)),
        [(38, DropReason::NotAbove), top]
    );

    let wet = WyomingSounding::parse(&edited(ROW_557, &ROW_557.replace(" 79,", "103,"))).unwrap();
    let level = wet
        .levels
        .iter()
        .find(|l| l.pressure_pa == 55_700.0)
        .unwrap();
    assert!((level.relative_humidity - 1.03).abs() < 1e-15);
    let air = wet.sounding(WindInterpolation::SpeedDirection).unwrap();
    let kept = air
        .levels()
        .iter()
        .find(|l| l.pressure_pa == Some(55_700.0))
        .unwrap();
    assert_eq!(kept.relative_humidity, Some(1.0));
}

/// A wind from 360° is from north, 0 rad, and stays in `[0, 2π)`.
#[test]
fn a_wind_from_360_degrees_is_from_north() {
    let body = edited(ROW_557, &ROW_557.replace(",200,", ",360,"));
    let sounding = WyomingSounding::parse(&body).unwrap();
    let level = sounding
        .levels
        .iter()
        .find(|l| l.pressure_pa == 55_700.0)
        .unwrap();
    assert_eq!(level.wind_direction_from_rad, 0.0);
    assert!(
        sounding
            .levels
            .iter()
            .all(|l| (0.0..std::f64::consts::TAU).contains(&l.wind_direction_from_rad))
    );
}

/// An answer that doesn't parse is never cached: with no copy the fetch is refused and the cache
/// stays empty; with a stale good copy, that copy comes back with the reason and stays cached.
#[test]
fn an_answer_that_does_not_parse_is_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let request = WyomingRequest::new("72364", JUNE_S);
    let url = request.url().unwrap();
    let page = b"<html><body>Can't get 72364 Observations at 12Z 21 Jun 2025.</body></html>";
    let bad = Client::new(Canned(page.to_vec()), Cache::new(dir.path()), Mode::Online);
    match wyoming::fetch(&bad, &request, 1_000) {
        Err(WyomingError::Net(NetError::Refused { reason, .. })) => {
            assert!(reason.contains("time column"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    assert!(Cache::new(dir.path()).get(&url).unwrap().is_none());

    let good = Client::new(replay(), Cache::new(dir.path()), Mode::Online);
    let (sounding, _) = wyoming::fetch(&good, &request, 1_000).unwrap();
    let a_year_on = 1_000 + 365 * 86_400;
    let (again, fetched) = wyoming::fetch(&bad, &request, a_year_on).unwrap();
    assert_eq!((fetched.freshness, again), (Freshness::Stale, sounding));
    assert!(fetched.stale_reason.unwrap().contains("refused"));
    let kept = Cache::new(dir.path()).get(&url).unwrap().unwrap();
    assert_eq!(kept.body, fixture(FM35));
}
