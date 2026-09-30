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
    self, ATTRIBUTION, DropReason, MAX_MISFITS, MAX_ROWS, SETTLE_S, WyomingError, WyomingRequest,
    WyomingSounding, WyomingVersion,
};
use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Transport};

/// 2025-06-21 12:00 UTC: Santa Teresa's recordings.
const JUNE_S: i64 = 1_750_507_200;
/// 2025-01-15 12:00 UTC: Salt Lake City's recording.
const JANUARY_S: i64 = 1_736_942_400;
const FM35: &str = "wyoming-72364-fm35.csv";
const BUFR: &str = "wyoming-72364-bufr.csv";
const WINTER: &str = "wyoming-72572-fm35.csv";
const DAY_S: u64 = 86_400;

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

/// A row as recorded: (line, pressure hPa, geopotential m, temperature °C, humidity %, speed
/// m/s, direction °, mixing ratio g/kg).
type Recorded = (usize, f64, f64, f64, f64, f64, f64, f64);

/// A row's pressure (hPa), geopotential height (m) and virtual temperature (K), when it has them.
/// Here the virtual temperature comes from the file's mixing ratio `w` (g/kg),
/// `T_v = T (1 + w/ε) / (1 + w)`, not from the humidity as in the parser.
fn thermo(row: &[String]) -> Option<(f64, f64, f64)> {
    const EPSILON: f64 = 18.015_28 / 28.964_4;
    let (p, z, t) = (field(row, 3)?, field(row, 4)?, field(row, 5)?);
    let w = field(row, 10).unwrap_or(0.0) / 1000.0;
    Some((p, z, (t + 273.15) * (1.0 + w / EPSILON) / (1.0 + w)))
}

/// How far, in metres, `upper`'s height above `lower` misses the hypsometric thickness beyond what
/// rounding each pressure to its step can move it (never below zero). A coded message's
/// pressures of 100 hPa or more are rounded to 1 hPa (`coarse`), the rest to 0.1 hPa.
fn thickness_miss_m(lower: (f64, f64, f64), upper: (f64, f64, f64), coarse: bool) -> f64 {
    let scale_m = 287.053 * 0.5 * (lower.2 + upper.2) / 9.806_65;
    let thickness_m = scale_m * (lower.0 / upper.0).ln();
    let half_step = |p: f64| if coarse && p >= 100.0 { 0.5 } else { 0.05 };
    let rounding_m = scale_m * (half_step(lower.0) / lower.0 + half_step(upper.0) / upper.0);
    ((upper.1 - lower.1 - thickness_m).abs() - rounding_m).max(0.0)
}

/// The rule, applied here on its own: the first row is the ground; of each run of complete rows
/// with the same pressure the middle one (the lower of two) is a candidate; a candidate is kept
/// when it is higher than, and at a lower pressure than, the last row kept (every row in the
/// recordings fits: see `recorded_rows_fit_their_layers`). Returns the rows kept, the lines
/// missing a value, and the lines of the other rows of each run.
fn apply_rule(name: &str) -> (Vec<Recorded>, Vec<usize>, Vec<usize>) {
    let mut complete: Vec<Recorded> = Vec::new();
    let mut missing = Vec::new();
    for (line, row) in rows(name) {
        let values: Option<Vec<f64>> = [3, 4, 5, 8, 12, 11]
            .into_iter()
            .map(|i| field(&row, i))
            .collect();
        match values {
            Some(v) => complete.push((
                line,
                v[0],
                v[1],
                v[2],
                v[3],
                v[4],
                v[5],
                field(&row, 10).unwrap(),
            )),
            None => missing.push(line),
        }
    }
    let mut kept = vec![complete[0]];
    let mut repeats = Vec::new();
    let mut start = 1;
    while start < complete.len() {
        let mut end = start;
        while end < complete.len() && complete[end].1 == complete[start].1 {
            end += 1;
        }
        let middle = start + (end - start - 1) / 2;
        for (i, row) in complete.iter().enumerate().take(end).skip(start) {
            let below = kept.last().unwrap();
            if i == middle && row.2 > below.2 && row.1 < below.1 {
                kept.push(*row);
            } else {
                repeats.push(row.0);
            }
        }
        start = end;
    }
    (kept, missing, repeats)
}

/// The done-when of M5.2b: sampling the profile at a kept row's geometric height gives back its
/// pressure, temperature, humidity and wind as recorded. The rows dropped are exactly the others:
/// in the coded messages the top row, which has no wind; in the BUFR file the 1,931 rows whose
/// pressure, to 0.1 hPa, repeats a neighbour's.
#[test]
fn the_profile_reproduces_every_level_it_keeps() {
    // (recording, rows, kept, rows missing a value, other rows of a run)
    for (name, total, kept, no_data, repeats) in [
        (FM35, 228, 227, vec![229], 0),
        (WINTER, 242, 241, vec![243], 0),
        (BUFR, 5_851, 3_920, vec![], 1_931),
    ] {
        assert_eq!(rows(name).len(), total, "{name}");
        let latitude_rad = field(&rows(name)[0].1, 2).unwrap().to_radians();
        let (expected, missing, repeated) = apply_rule(name);
        assert_eq!(missing, no_data, "{name}");
        assert_eq!(repeated.len(), repeats, "{name}");
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
            .chain(repeated.iter().map(|&l| (l, DropReason::SamePressure)))
            .collect();
        as_dropped.sort_by_key(|d| d.0);
        assert_eq!(dropped, as_dropped, "{name}");
        assert_eq!(sounding.levels.len(), kept, "{name}");

        let air = sounding
            .sounding(WindInterpolation::SpeedDirection)
            .unwrap();
        let wind = air.wind().unwrap();
        for (_, pressure_hpa, z, temperature_c, humidity_pct, speed, direction_deg, _) in expected {
            let height_m = geometric_from_wmo_geopotential_m(z, latitude_rad).unwrap();
            let pressure_pa = pressure_hpa * 100.0;
            let at = format!("{name}, {pressure_pa} Pa at {height_m} m");
            let sample = air.sample(height_m).unwrap();
            assert!(sample.extrapolated.is_none(), "{at}");
            let rel = (sample.air.pressure_pa - pressure_pa).abs() / pressure_pa;
            assert!(rel < 1e-12, "{at}: pressure {} Pa", sample.air.pressure_pa);
            let dt = (sample.air.temperature_k - (temperature_c + 273.15)).abs();
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

/// Every row with a pressure, height and temperature fits the one before it: its height misses
/// the hypsometric thickness by at most 1 m beyond what rounding the two pressures can move it (0
/// in the coded messages, 0.97 m in the BUFR file), against the parser's 30 m, and with none of its
/// 5%. The thickness is worked out here on its own, from the file's mixing ratio.
#[test]
fn recorded_rows_fit_their_layers() {
    for name in [FM35, WINTER, BUFR] {
        let rows: Vec<_> = rows(name).iter().filter_map(|(_, r)| thermo(r)).collect();
        let coarse = rows.iter().all(|r| r.0 < 100.0 || r.0.fract() == 0.0);
        assert_eq!(coarse, name != BUFR);
        let worst = rows
            .windows(2)
            .map(|w| thickness_miss_m(w[0], w[1], coarse))
            .fold(0.0, f64::max);
        assert!(worst < 1.0, "{name}: {worst} m");
    }
}

/// Every BUFR row, kept or not, is close to the profile: at each one's height the profile's
/// pressure is within 0.08 hPa of the row's (at worst 0.071 hPa). A kept row's own pressure is
/// rounded by up to 0.05 hPa, and a run's end rows lie up to half a step from its middle. Keeping
/// the first row of each run instead put 10 hPa 26 m low and misses by 0.093 hPa, which this
/// bound refuses (the physics review of M5.2b).
#[test]
fn repeated_bufr_rows_lie_on_the_profile() {
    let rows = rows(BUFR);
    let latitude_rad = field(&rows[0].1, 2).unwrap().to_radians();
    let air = WyomingSounding::parse(&fixture(BUFR))
        .unwrap()
        .sounding(WindInterpolation::SpeedDirection)
        .unwrap();
    let mut worst_hpa: f64 = 0.0;
    for (_, row) in &rows {
        let (p, z) = (field(row, 3).unwrap(), field(row, 4).unwrap());
        let height_m = geometric_from_wmo_geopotential_m(z, latitude_rad).unwrap();
        let sample = air.sample(height_m).unwrap();
        worst_hpa = worst_hpa.max((sample.air.pressure_pa / 100.0 - p).abs());
    }
    assert!(worst_hpa < 0.08, "{worst_hpa} hPa");
}

/// The ground and the release: the first row, as recorded; the wind there, independently of the
/// library's conversion: 5.7 m/s from 265° blows toward 85°, east (5.678 m/s) and a little north
/// (0.497 m/s).
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
    let air = sounding
        .sounding(WindInterpolation::SpeedDirection)
        .unwrap();
    let v = air
        .wind()
        .unwrap()
        .wind(ground.height_msl_m)
        .unwrap()
        .velocity_enu_m_s;
    assert!(
        (v.x - 5.678).abs() < 5e-4 && (v.y - 0.497).abs() < 5e-4,
        "{v}"
    );
    let winter = WyomingSounding::parse(&fixture(WINTER)).unwrap();
    assert_eq!(winter.release_unix_s, JANUARY_S - 55 * 60);
}

/// The heights are geopotential metres, as the column's name says. Across the 13 layers between
/// the standard levels from 850 to 10 hPa, the recorded thickness matches the hypsometric
/// thickness from the rows' virtual temperatures (with the file's own mixing ratios) to 0.01% to
/// 0.03% on average; single layers scatter by 0.05% to 0.13% on average either way. Read as
/// geometric heights, the recorded layers would be 0.51% to 0.60% thinner on average than the
/// geometric thickness. The asserts hold the mean under 0.1% as geopotential and beyond −0.4% as
/// geometric.
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
        let (kept, _, _) = apply_rule(name);
        // (pressure hPa, geopotential m, virtual temperature K)
        let kept: Vec<(f64, f64, f64)> = kept
            .iter()
            .map(|r| {
                let w = r.7 / 1_000.0;
                (r.1, r.2, (r.3 + 273.15) * (1.0 + w / epsilon) / (1.0 + w))
            })
            .collect();
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
        assert!(geopotential.abs() < 0.1e-2, "{name}: {geopotential}");
        assert!(geometric < -0.4e-2, "{name}: {geometric} as geometric");
    }
}

/// The request's URL is the one recorded, so the replay answers it; a second fetch comes from the
/// cache, and offline the cache answers without the transport, stale after 30 days.
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
    let (later, fetched) = wyoming::fetch(&offline, &request, now_s + 31 * DAY_S).unwrap();
    assert_eq!(fetched.freshness, Freshness::Stale);
    assert_eq!(later, sounding);

    let mut bufr = request.clone();
    bufr.version = WyomingVersion::Bufr;
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

/// A copy fetched while the sounding may still be filling in is fresh for an hour, and once the
/// sounding has settled it is fetched again, not kept for 30 days.
#[test]
fn a_young_copy_is_fetched_again_once_settled() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let request = WyomingRequest::new("72364", JUNE_S);
    let noon = u64::try_from(JUNE_S).unwrap();
    let calls = |at: u64| {
        let (_, fetched) = wyoming::fetch(&online, &request, at).unwrap();
        (fetched.freshness, transport.calls())
    };
    assert_eq!(calls(noon + 2 * 3_600), (Freshness::Fetched, 1));
    assert_eq!(calls(noon + 2 * 3_600 + 1_800), (Freshness::Cached, 1));
    assert_eq!(calls(noon + 3 * 3_600 + 1), (Freshness::Fetched, 2));
    // Settled: the copy from 3 h is stale, fetched again, then kept.
    assert_eq!(calls(noon + SETTLE_S + 60), (Freshness::Fetched, 3));
    assert_eq!(calls(noon + SETTLE_S + 20 * DAY_S), (Freshness::Cached, 3));
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

/// The coded-message recording with text replacements, each of which must hit exactly once.
fn edited(edits: &[(&str, &str)]) -> Vec<u8> {
    let mut text = String::from_utf8(fixture(FM35)).unwrap();
    for (from, to) in edits {
        assert_eq!(text.matches(from).count(), 1, "{from}");
        text = text.replacen(from, to, 1);
    }
    text.into_bytes()
}

/// The first row, its values, the rows at 557 and 549 hPa (lines 38 and 39), and the last row
/// kept (line 228), as recorded.
const GROUND: &str = "2025-06-21 11:02:00,-106.7000,31.8600, 872.0, 1252, 28.4,  9.4,  9.4, 31, \
                      31, 8.52,265, 5.7";
const GROUND_VALUES: &str = " 872.0, 1252, 28.4,  9.4,  9.4, 31, 31, 8.52,265, 5.7";
const ROW_557: &str = " 557.0, 5035, -1.7, -4.9, -4.3, 79, 80, 4.79,200, 9.8";
const ROW_549: &str = " 549.0, 5151, -2.5, -5.2, -4.6, 81, 83, 4.73,210, 9.3";
const ROW_854: &str = " 854.0, 1438, 26.8,  8.8,  8.8, 32, 32, 8.35,269,10.7";
const ROW_TOP: &str = "   8.0,32801,-40.6,-69.6,-65.0,  3,  5, 0.42, 90,17.0";

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
        missing(&edited(&[("temperature_C,dew", "temp_C,dew")])),
        "temperature"
    );

    match parse(&edited(&[("wind speed_m/s", "wind speed_knot")])) {
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
    let long_unit = format!("pressure_{}", "Pa".repeat(500));
    match parse(&edited(&[("pressure_hPa", &long_unit)])) {
        Err(WyomingError::Units { found, .. }) => assert_eq!(found.chars().count(), 41),
        other => panic!("{other:?}"),
    }

    let row = |body: &[u8]| match parse(body) {
        Err(WyomingError::Row { line, reason }) => (line, reason),
        other => panic!("{other:?}"),
    };
    let (line, reason) = row(&edited(&[(ROW_557, &format!("{ROW_557},1"))]));
    assert_eq!((line, reason.as_str()), (38, "more than 13 fields, not 13"));
    let (line, reason) = row(&edited(&[(ROW_557, &ROW_557.replacen(",200", "", 1))]));
    assert_eq!((line, reason.as_str()), (38, "12 fields, not 13"));
    let (line, reason) = row(&edited(&[(ROW_557, &ROW_557.replace("557.0", "55x.0"))]));
    assert_eq!(line, 38);
    assert!(reason.contains("pressure \"55x.0\""), "{reason}");
    let (_, reason) = row(&edited(&[(ROW_557, &ROW_557.replace("-1.7", "inf"))]));
    assert!(reason.contains("temperature"), "{reason}");
    let dated = "2025-06-21 11:02:00,-106.7000,31.8600, 872.0";
    let (line, reason) = row(&edited(&[(dated, "2025-06-21,-106.7000,31.8600, 872.0")]));
    assert_eq!(line, 2);
    assert!(reason.contains("time \"2025-06-21\""), "{reason}");
    let (line, reason) = row(&edited(&[("time,", "time,pressure_hPa,")]));
    assert_eq!((line, reason.as_str()), (1, "two pressure columns"));

    let header = String::from_utf8(fixture(FM35)).unwrap();
    let header = header.lines().next().unwrap();
    assert!(matches!(
        parse(format!("{header}\n\n").as_bytes()),
        Err(WyomingError::NoGround { line: 2 })
    ));
    for ground in [
        GROUND.replace("265, 5.7", "   ,    "),
        GROUND.replacen(" 31,", " -1,", 1),
        GROUND.replace(" 28.4,", "-300.0,"),
    ] {
        assert!(
            matches!(
                parse(&edited(&[(GROUND, &ground)])),
                Err(WyomingError::NoGround { line: 2 })
            ),
            "{ground}"
        );
    }
}

/// A hostile answer costs no more than a good one: a header or a row of a million commas is
/// refused without splitting it all.
#[test]
fn a_flood_of_commas_is_refused() {
    let commas = ",".repeat(1_000_000);
    match WyomingSounding::parse(commas.as_bytes()) {
        Err(WyomingError::Row { line: 1, reason }) => assert_eq!(reason, "more than 64 columns"),
        other => panic!("{other:?}"),
    }
    let flooded = edited(&[(ROW_557, &format!("{ROW_557}{commas}"))]);
    match WyomingSounding::parse(&flooded) {
        Err(WyomingError::Row { line: 38, reason }) => {
            assert_eq!(reason, "more than 13 fields, not 13");
        }
        other => panic!("{other:?}"),
    }
}

/// A row's own faults drop it, with the reason, and leave the rest: a missing temperature, a value
/// out of range, a row below the one before it. A humidity above 100% (radiosondes report it in
/// cloud) is kept as recorded and taken as 100% in the profile.
#[test]
fn a_rows_faults_drop_it_or_are_clamped() {
    let reasons = |body: Vec<u8>| {
        let sounding = WyomingSounding::parse(&body).unwrap();
        sounding
            .sounding(WindInterpolation::SpeedDirection)
            .unwrap();
        sounding
            .dropped
            .iter()
            .map(|d| (d.line, d.reason))
            .collect::<Vec<_>>()
    };
    let top = (229, DropReason::NoData);
    let one = |from: &str, to: &str| reasons(edited(&[(ROW_557, &ROW_557.replace(from, to))]));
    assert_eq!(one(" -1.7,", "     ,"), [(38, DropReason::NoData), top]);
    for (from, to) in [
        (" 79,", " -1,"),
        (" -1.7,", "-999.,"),
        (" -1.7,", "-150.1,"),
        (" -1.7,", " 80.1,"),
        (" 5035,", " 60001,"),
        (" 9.8", " 300.1"),
        (" 557.0,", " 1200.1,"),
        (" 9.8", "-9.8"),
        (",200,", ",361,"),
        (" 557.0,", "  -1.0,"),
    ] {
        assert_eq!(one(from, to), [(38, DropReason::OutOfRange), top], "{to}");
    }
    // 557 hPa at 4,000 m doesn't fit the 570 hPa row at 4,852 m. At 580 hPa and 4,712 m it fits
    // (it lies between rows 36 and 37), but below the row kept.
    assert_eq!(one(" 5035,", " 4000,"), [(38, DropReason::Thickness), top]);
    let at_580 = " 580.0, 4712,";
    assert_eq!(
        one(" 557.0, 5035,", at_580),
        [(38, DropReason::NotAbove), top]
    );
    // At 571 hPa and 4,860 m it fits and is higher than the 570 hPa row, but not at a lower
    // pressure.
    assert_eq!(
        one(" 557.0, 5035,", " 571.0, 4860,"),
        [(38, DropReason::NotAbove), top]
    );
    // The rule is against the last row kept, not the row before: after 580 hPa is dropped, a row
    // at 575 hPa is below the 570 hPa row kept, though above the dropped one.
    let both = edited(&[
        (ROW_557, &ROW_557.replace(" 557.0, 5035,", at_580)),
        (ROW_549, &ROW_549.replace(" 549.0, 5151,", " 575.0, 4781,")),
    ]);
    assert_eq!(
        reasons(both),
        [(38, DropReason::NotAbove), (39, DropReason::NotAbove), top]
    );
    // The rows left out are listed in line order: row 38 at row 37's pressure is the rest of a
    // run, which closes after row 39, missing its temperature, is dropped.
    let run = edited(&[
        (ROW_557, &ROW_557.replace(" 557.0, 5035,", " 570.0, 4860,")),
        (ROW_549, &ROW_549.replace(" -2.5,", "     ,")),
    ]);
    assert_eq!(
        reasons(run),
        [
            (38, DropReason::SamePressure),
            (39, DropReason::NoData),
            top
        ]
    );
    // Values at the bounds are kept, and make a profile (`reasons` builds it), on the last row
    // kept: its layer's 1 hPa rounding leaves room for any temperature. The unit tests hold each
    // bound's edges.
    let last = |from: &str, to: &str| reasons(edited(&[(ROW_TOP, &ROW_TOP.replace(from, to))]));
    for (from, to) in [
        ("-40.6,", "-150.0,"),
        ("-40.6,", " 80.0,"),
        ("17.0", "300.0"),
    ] {
        assert_eq!(last(from, to), [top], "{to}");
    }
    for (from, to) in [("32801,", "60001,"), ("   8.0,", "  0.09,")] {
        assert_eq!(last(from, to), [(228, DropReason::OutOfRange), top], "{to}");
    }
    for (from, to) in [(" 872.0,", "1200.1,"), (" 1252,", "-1001,")] {
        let edit = GROUND.replace(from, to);
        assert!(
            matches!(
                WyomingSounding::parse(&edited(&[(GROUND, &edit)])),
                Err(WyomingError::NoGround { line: 2 })
            ),
            "{to}"
        );
    }
    // A run at the ground's pressure keeps none of it.
    let at_ground = edited(&[(ROW_854, GROUND_VALUES)]);
    assert_eq!(reasons(at_ground), [(3, DropReason::SamePressure), top]);

    let wet = edited(&[(ROW_557, &ROW_557.replace(" 79,", "103,"))]);
    let wet = WyomingSounding::parse(&wet).unwrap();
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

/// A row with a bad value is left out, not kept to hide the good rows after it: 557 hPa with a
/// digit lost (57 hPa at 5 km) misses its layer's thickness by about 18 km, and 5,035 m raised to
/// 5,890 m by about 850 m. Two bad rows in a row are left out too, and so is a bad row where the
/// balloon then bursts and falls. The profile is the recording's without the row.
#[test]
fn a_bad_row_is_left_out() {
    let thickness = |body: Vec<u8>| {
        let sounding = WyomingSounding::parse(&body).unwrap();
        let air = sounding
            .sounding(WindInterpolation::SpeedDirection)
            .unwrap();
        let pressure_pa = air.sample(5_000.0).unwrap().air.pressure_pa;
        let lines: Vec<_> = sounding
            .dropped
            .iter()
            .filter(|d| d.reason != DropReason::NoData)
            .map(|d| (d.line, d.reason))
            .collect();
        (lines, pressure_pa)
    };
    let recorded = String::from_utf8(fixture(FM35)).unwrap();
    let without: String = recorded
        .lines()
        .filter(|l| !l.contains(ROW_557))
        .map(|l| format!("{l}\n"))
        .collect();
    let (_, without_pa) = thickness(without.into_bytes());
    let digit_lost = (ROW_557, ROW_557.replace(" 557.0,", "  57.0,"));
    for edits in [
        vec![digit_lost.clone()],
        vec![(ROW_557, ROW_557.replace(" 5035,", " 5890,"))],
    ] {
        let edits: Vec<_> = edits.iter().map(|(a, b)| (*a, b.as_str())).collect();
        let (lines, pressure_pa) = thickness(edited(&edits));
        assert_eq!(lines, [(38, DropReason::Thickness)], "{edits:?}");
        assert_eq!(pressure_pa, without_pa);
    }
    for second in [
        ROW_549.replace(" 5151,", "51510,"),
        ROW_549.replace(" 549.0,", "  49.0,"),
    ] {
        let edits = [(ROW_557, digit_lost.1.as_str()), (ROW_549, second.as_str())];
        let (lines, _) = thickness(edited(&edits));
        assert_eq!(
            lines,
            [(38, DropReason::Thickness), (39, DropReason::Thickness)],
            "{second}"
        );
    }

    // The balloon bursts at line 150 and falls back to where row 39 was.
    let bad = String::from_utf8(edited(&[(ROW_557, &digit_lost.1)])).unwrap();
    let lines: Vec<&str> = bad.lines().collect();
    let mut burst: String = lines[..150].iter().map(|l| format!("{l}\n")).collect();
    burst.extend(lines[38..149].iter().rev().map(|l| format!("{l}\n")));
    let (lines, _) = thickness(burst.into_bytes());
    let fall: Vec<_> = (151..=261).map(|l| (l, DropReason::NotAbove)).collect();
    assert_eq!(lines, [vec![(38, DropReason::Thickness)], fall].concat());
}

/// Nothing before the ground checks it, so the rows after it do: a ground they miss while fitting
/// each other refuses the answer. So do a pressure with its decimal point misplaced (87.2 hPa for
/// 872), a height with a digit lost (125 m for 1,252 m), a ground 8 hPa or 80 m off, and values
/// within the bounds that no air above fits. A ground 40 m high, or at 80 °C, fits the first layer
/// (186 m thick) and is kept with every row.
#[test]
fn a_bad_ground_is_refused() {
    let parse = |from: &str, to: &str| {
        WyomingSounding::parse(&edited(&[(GROUND, &GROUND.replace(from, to))]))
    };
    for (from, to) in [
        (" 872.0,", "  87.2,"),
        (" 1252,", "  125,"),
        (" 872.0,", " 880.0,"),
        (" 872.0,", " 864.0,"),
        (" 1252,", " 1332,"),
        (" 872.0,", "1200.0,"),
        (" 1252,", "-1000,"),
        (" 28.4,", "-150.0,"),
    ] {
        match parse(from, to) {
            Err(WyomingError::GroundMisfit { line, first }) => assert_eq!((line, first), (2, 3)),
            other => panic!("{to}: {other:?}"),
        }
    }
    for (from, to) in [(" 1252,", " 1292,"), (" 28.4,", " 80.0,")] {
        let sounding = parse(from, to).unwrap();
        assert_eq!(sounding.levels.len(), 227, "{to}");
    }
    // With a single row after it, nothing tells the ground from the row: the row is left out.
    let text =
        String::from_utf8(edited(&[(GROUND, &GROUND.replace(" 872.0,", "  87.2,"))])).unwrap();
    let two: String = text.lines().take(3).map(|l| format!("{l}\n")).collect();
    let sounding = WyomingSounding::parse(two.as_bytes()).unwrap();
    assert_eq!(sounding.levels.len(), 1);
    assert_eq!(sounding.dropped[0].reason, DropReason::Thickness);
}

/// A row that fits the chain's end only just, which the next row fits only through that end, is
/// the odd one out and is left out alone: 854 hPa 50 m low, 101 hPa 95 m high, and a BUFR row at
/// 112.8 hPa 35 m high. In the coded message, 549 hPa 45 m high fits within its pressures'
/// 1 hPa rounding and is kept; a BUFR row at 150.6 hPa 50 m high, rounded to 0.1 hPa, doesn't.
#[test]
fn a_row_that_only_just_fits_is_left_out() {
    let thickness = |body: Vec<u8>| {
        let sounding = WyomingSounding::parse(&body).unwrap();
        sounding
            .dropped
            .iter()
            .filter(|d| d.reason == DropReason::Thickness)
            .map(|d| d.line)
            .collect::<Vec<_>>()
    };
    for (line, by) in [(3, -50.0), (132, 95.0), (39, 45.0)] {
        let body = with_field(4, |l, height| {
            let z: f64 = height.trim().parse().unwrap();
            format!("{}", if l == line { z + by } else { z })
        });
        let expected = if line == 39 { vec![] } else { vec![line] };
        assert_eq!(thickness(body), expected, "line {line}");
    }
    let bufr = String::from_utf8(fixture(BUFR)).unwrap();
    for (line, by) in [(2889, 35.0), (2528, 50.0)] {
        let mut out = String::new();
        for (i, row) in bufr.lines().enumerate() {
            let mut fields: Vec<String> = row.split(',').map(str::to_owned).collect();
            if i + 1 == line {
                let z: f64 = fields[4].trim().parse().unwrap();
                fields[4] = format!("{}", z + by);
            }
            out.push_str(&fields.join(","));
            out.push('\n');
        }
        assert_eq!(thickness(out.into_bytes()), [line], "BUFR line {line}");
    }
}

/// A run of rows with the same pressure is chosen from the rows that fit: with the middle of a
/// BUFR run of three raised 100 m, the run keeps its first row instead of none.
#[test]
fn a_run_keeps_a_row_that_fits() {
    let bufr = String::from_utf8(fixture(BUFR)).unwrap();
    let pressures: Vec<&str> = bufr
        .lines()
        .map(|l| l.split(',').nth(3).unwrap_or(""))
        .collect();
    // The first run of exactly three, as 0-based line indices.
    let start = (2..pressures.len() - 3)
        .find(|&i| {
            pressures[i] == pressures[i + 1]
                && pressures[i] == pressures[i + 2]
                && pressures[i - 1] != pressures[i]
                && pressures[i + 3] != pressures[i]
        })
        .unwrap();
    let mut out = String::new();
    for (i, row) in bufr.lines().enumerate() {
        let mut fields: Vec<String> = row.split(',').map(str::to_owned).collect();
        if i == start + 1 {
            let z: f64 = fields[4].trim().parse().unwrap();
            fields[4] = format!("{}", z + 100.0);
        }
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    let sounding = WyomingSounding::parse(out.as_bytes()).unwrap();
    assert_eq!(sounding.levels.len(), 3_920);
    let middle = start + 2;
    let reasons: Vec<_> = sounding
        .dropped
        .iter()
        .filter(|d| (middle - 1..=middle + 1).contains(&d.line))
        .map(|d| (d.line, d.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            (middle, DropReason::Thickness),
            (middle + 1, DropReason::SamePressure)
        ]
    );
}

/// The coded-message recording with field `field` of each line set by `edit(line, value)`.
fn with_field(field: usize, edit: impl Fn(usize, &str) -> String) -> Vec<u8> {
    let text = String::from_utf8(fixture(FM35)).unwrap();
    let mut out = String::new();
    for (i, line) in text.lines().enumerate() {
        let mut fields: Vec<String> = line.split(',').map(str::to_owned).collect();
        if i > 0 {
            fields[field] = edit(i + 1, &fields[field]);
        }
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out.into_bytes()
}

/// Rows missing a wind or a humidity still carry the chain: with neither from line 92 to 164 (250
/// to 55 hPa), every row after them fits and is kept, though the layer across the gap is too thick
/// for its two ends' temperatures to give. Two blocks of six raised rows, a good row between them,
/// are left out without refusing. A row whose vapour would outweigh its air (16 hPa at 30 °C,
/// saturated) is left out, not taken to fit anything. A row at the bounds that fits (0.1 hPa at
/// −150 °C, 55.6 km) is kept and makes a profile.
#[test]
fn gaps_blocks_and_extremes() {
    let dropped = |body: &[u8]| {
        let sounding = WyomingSounding::parse(body).unwrap();
        sounding
            .sounding(WindInterpolation::SpeedDirection)
            .unwrap();
        let lines: Vec<_> = sounding
            .dropped
            .iter()
            .map(|d| (d.line, d.reason))
            .collect();
        (sounding.levels.len(), lines)
    };
    for field in [12, 8] {
        let gap = with_field(field, |line, value| {
            if (92..=164).contains(&line) {
                String::new()
            } else {
                value.to_owned()
            }
        });
        let no_data: Vec<_> = (92..=164)
            .chain([229])
            .map(|l| (l, DropReason::NoData))
            .collect();
        assert_eq!(dropped(&gap), (227 - 73, no_data), "field {field}");
    }

    let blocks = with_field(4, |line, height| {
        let z: f64 = height.trim().parse().unwrap();
        let raised = (39..=44).contains(&line) || (46..=51).contains(&line);
        format!("{}", if raised { z + 1_000.0 } else { z })
    });
    let (levels, lines) = dropped(&blocks);
    assert_eq!(levels, 227 - 12);
    let thickness: Vec<_> = (39..=44)
        .chain(46..=51)
        .map(|l| (l, DropReason::Thickness))
        .collect();
    assert_eq!(lines, [thickness, vec![(229, DropReason::NoData)]].concat());

    let vapour = " 16.0, 5100, 30.0, -4.9, -4.3, 99.97301034739507, 80, 4.79,210, 9.3";
    let (levels, lines) = dropped(&edited(&[(ROW_549, vapour)]));
    assert_eq!(levels, 226);
    assert_eq!(
        lines,
        [(39, DropReason::Thickness), (229, DropReason::NoData)]
    );

    let text = after_the_top(&[0.1], -150.0);
    let (levels, lines) = dropped(text.as_bytes());
    assert_eq!((levels, lines), (228, vec![]));
    let top = text.lines().last().unwrap().to_owned();
    assert!(top.contains(" 0.10, 556"), "{top}");
}

/// A block of rows whose heights are all 1 km high misses the row kept below it: ten are left
/// out, and the row after them fits; eleven refuse the answer.
#[test]
fn a_block_that_does_not_fit_is_refused() {
    let raised = |rows: usize| {
        let text = String::from_utf8(fixture(FM35)).unwrap();
        let mut out = String::new();
        for (i, line) in text.lines().enumerate() {
            let mut fields: Vec<String> = line.split(',').map(str::to_owned).collect();
            if (38..38 + rows).contains(&i) {
                let z: f64 = fields[4].trim().parse().unwrap();
                fields[4] = format!("{}", z + 1000.0);
            }
            out.push_str(&fields.join(","));
            out.push('\n');
        }
        WyomingSounding::parse(out.as_bytes())
    };
    let sounding = raised(MAX_MISFITS).unwrap();
    let block: Vec<_> = (39..49).map(|l| (l, DropReason::Thickness)).collect();
    let dropped: Vec<_> = sounding
        .dropped
        .iter()
        .map(|d| (d.line, d.reason))
        .collect();
    assert_eq!(dropped, [block, vec![(229, DropReason::NoData)]].concat());
    match raised(MAX_MISFITS + 1) {
        Err(WyomingError::Misfit { after, line }) => assert_eq!((after, line), (38, 39)),
        other => panic!("{other:?}"),
    }
}

/// The height a row at `to_hpa` and `to_c` has above one at `from_hpa`, `from_m` and `from_c`, by
/// the hypsometric equation for dry air: rows that fit the rows before them.
fn fitting_height_m(from_hpa: f64, from_m: f64, from_c: f64, to_hpa: f64, to_c: f64) -> f64 {
    let mean_k = 0.5 * (from_c + to_c) + 273.15;
    from_m + 287.053 * mean_k / 9.806_65 * (from_hpa / to_hpa).ln()
}

/// The coded-message recording without its last row (7.9 hPa, no wind), then rows at `pressures`
/// and `temperature_c`, each at the height that fits the row before it.
fn after_the_top(pressures: &[f64], temperature_c: f64) -> String {
    let recorded = String::from_utf8(fixture(FM35)).unwrap();
    let (body, _) = recorded.trim_end().rsplit_once('\n').unwrap();
    let mut text = format!("{body}\n");
    let (mut p0, mut z0, mut t0) = (8.0, 32_801.0, -40.6);
    for &p in pressures {
        let z = fitting_height_m(p0, z0, t0, p, temperature_c);
        text.push_str(&format!(
            "2025-06-21 11:02:00,-106.7000,31.8600, {p:.2}, {z:.0}, {temperature_c:.1}, -70.0, \
             -65.5,  3,  5, 0.4, 90, 17.0\n"
        ));
        (p0, z0, t0) = (p, z.round(), temperature_c);
    }
    text
}

/// Rows below the last row kept that fall (a balloon after it bursts) or float are left out,
/// however many: at the end of an answer, even when the float ends a little higher than it began,
/// or crosses the last row kept, and when a row after them is kept.
#[test]
fn a_falling_tail_is_left_out() {
    let not_above = |text: &str| {
        let sounding = WyomingSounding::parse(text.as_bytes()).unwrap();
        let lines: Vec<_> = sounding
            .dropped
            .iter()
            .filter(|d| d.reason == DropReason::NotAbove)
            .map(|d| d.line)
            .collect();
        assert_eq!(
            sounding.dropped.len(),
            lines.len(),
            "{:?}",
            sounding.dropped
        );
        (sounding.levels.len(), lines)
    };
    // Twenty rows falling from 8.5 to 27.5 hPa.
    let fall: Vec<f64> = (0..20).map(|k| 8.5 + f64::from(k)).collect();
    let text = after_the_top(&fall, -45.0);
    assert_eq!(not_above(&text), (227, (229..249).collect::<Vec<_>>()));

    // Thirteen rows floating between 8.4 and 8.6 hPa, ending higher than they began, below the
    // 8.0 hPa row (line 228) and the 8.1 hPa row before it.
    let float: Vec<f64> = (0..13)
        .map(|k| match k {
            0 => 8.5,
            _ if k % 2 == 1 => 8.6,
            _ => 8.4,
        })
        .collect();
    let mut text = after_the_top(&float, -41.0);
    assert_eq!(not_above(&text), (227, (229..242).collect::<Vec<_>>()));
    // The recorded last row after them, with a wind, is kept.
    text.push_str(
        "2025-06-21 11:02:00,-106.7000,31.8600,   7.9,32886,-40.5,-69.5,-64.9,  3,  5, 0.43, 90,17.0\n",
    );
    assert_eq!(not_above(&text), (228, (229..242).collect::<Vec<_>>()));

    // Thirteen rows floating across the last row kept, between 8.05 and 7.95 hPa: the first above
    // it is kept, and the rest are left out.
    let across: Vec<f64> = (0..13)
        .map(|k| if k % 2 == 0 { 8.05 } else { 7.95 })
        .collect();
    let text = after_the_top(&across, -40.6);
    let mut left_out: Vec<usize> = (229..242).collect();
    left_out.remove(1);
    assert_eq!(not_above(&text), (228, left_out));
}

/// An answer of more than [`MAX_ROWS`] rows is refused as it is read, so a flood of empty rows
/// costs no more than a large real one.
#[test]
fn too_many_rows_are_refused() {
    let header = String::from_utf8(fixture(FM35)).unwrap();
    let header = header.lines().next().unwrap().to_owned();
    let body = |rows: usize| format!("{header}\n{GROUND}\n{}", ",,,,,,,,,,,,\n".repeat(rows));
    let sounding = WyomingSounding::parse(body(MAX_ROWS - 1).as_bytes()).unwrap();
    assert_eq!(sounding.dropped.len(), MAX_ROWS - 1);
    match WyomingSounding::parse(body(MAX_ROWS).as_bytes()) {
        Err(WyomingError::Row { line, reason }) => {
            assert_eq!(
                (line, reason.as_str()),
                (MAX_ROWS + 2, "more than 100000 rows")
            );
        }
        other => panic!("{other:?}"),
    }
}

/// A wind from 360° is from north, 0 rad, and stays in `[0, 2π)`.
#[test]
fn a_wind_from_360_degrees_is_from_north() {
    let body = edited(&[(ROW_557, &ROW_557.replace(",200,", ",360,"))]);
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
    let settled = u64::try_from(JUNE_S).unwrap() + SETTLE_S;
    match wyoming::fetch(&bad, &request, settled) {
        Err(WyomingError::Net(NetError::Refused { reason, .. })) => {
            assert!(reason.contains("time column"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    assert!(Cache::new(dir.path()).get(&url).unwrap().is_none());

    let good = Client::new(replay(), Cache::new(dir.path()), Mode::Online);
    let (sounding, _) = wyoming::fetch(&good, &request, settled + 1).unwrap();
    let a_year_on = settled + 365 * DAY_S;
    let (again, fetched) = wyoming::fetch(&bad, &request, a_year_on).unwrap();
    assert_eq!((fetched.freshness, again), (Freshness::Stale, sounding));
    assert!(fetched.stale_reason.unwrap().contains("refused"));
    let kept = Cache::new(dir.path()).get(&url).unwrap().unwrap();
    assert_eq!(kept.body, fixture(FM35));
}
