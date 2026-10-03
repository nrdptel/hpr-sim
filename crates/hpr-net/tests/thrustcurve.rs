//! ThrustCurve.org's API against recorded answers, and the join with the motor finder (M5.4b):
//! the designation to ThrustCurve id mapping covers the in-stock motors, with a report of the
//! misses, and a mapped motor's recorded curve reads with `hpr_motor`.
//!
//! The expected values are read from the recordings here, with `serde_json`, not through the
//! code under test. The two downloads were recorded on 2026-10-01 at 08:22 UTC; the motor
//! finder's in-stock list is its build of 07:07:29 UTC that day (M5.4a). The three searches are
//! stand-ins in the API's shape (ADR-145): ThrustCurve.org grants no licence for its records, so
//! each search holds five invented motors and one record for each of the maker's motors in the
//! finder's in-stock list, carrying that list's own values (CC BY 4.0) and an invented id, bar
//! J450DM's and F27R/L's, the ids of the two downloads.

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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use hpr_net::motor_finder;
use hpr_net::thrustcurve::{
    self, ATTRIBUTION, Curve, Download, Format, MissReason, Search, TTL_S, ThrustCurveError,
};
use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Transport};
use serde::Serialize;
use serde_json::Value;

/// 2026-10-01 08:30 UTC, just after the answers were recorded.
const NOW_S: u64 = 1_790_843_400;

/// The stand-in searches (ADR-145): the motor finder's name for the maker, and the file.
const SEARCHES: [(&str, &str); 3] = [
    ("AeroTech", "thrustcurve-search-aerotech.json"),
    ("Cesaroni Technology", "thrustcurve-search-cesaroni.json"),
    ("Loki Research", "thrustcurve-search-loki.json"),
];

/// AeroTech J450DM, in stock on the recorded morning: its id and its one RASP file.
const J450DM_ID: &str = "5f4294d2000231000000044f";
const J450DM_FILE: &str = "thrustcurve-download-J450DM.json";

/// AeroTech F27R/L: its id and its one public-domain RockSim file.
const F27_ID: &str = "5f4294d200023100000001d4";
const F27_FILE: &str = "thrustcurve-download-F27R_L-rocksim.json";

const REPORT_START: &str = "<!-- join: written by crates/hpr-net/tests/thrustcurve.rs -->";
const REPORT_END: &str = "<!-- end of join -->";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/replay")
            .join(name),
    )
    .unwrap()
}

fn recorded(name: &str) -> Value {
    serde_json::from_slice(&fixture(name)).unwrap()
}

fn replay() -> Replay {
    Replay::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay")).unwrap()
}

/// Asserts that `read`, written back as JSON, holds exactly the recording's keys and values:
/// numbers compared as numbers, so a recorded `54` matches a read `54.0`.
fn assert_reads_back(read: &impl Serialize, recording: &Value, at: &str) {
    assert_same(&serde_json::to_value(read).unwrap(), recording, at);
}

fn assert_same(written: &Value, recorded: &Value, at: &str) {
    match (written, recorded) {
        (Value::Number(w), Value::Number(r)) => {
            assert_eq!(w.as_f64(), r.as_f64(), "{at}");
            if r.is_u64() {
                assert_eq!(w.as_f64().map(|v| v.fract()), Some(0.0), "{at}");
            }
        }
        (Value::Object(w), Value::Object(r)) => {
            let (wk, rk): (Vec<_>, Vec<_>) = (w.keys().collect(), r.keys().collect());
            assert_eq!(wk, rk, "{at}: keys");
            for (key, value) in r {
                assert_same(&w[key], value, &format!("{at}.{key}"));
            }
        }
        (Value::Array(w), Value::Array(r)) => {
            assert_eq!(w.len(), r.len(), "{at}: length");
            for (i, (w, r)) in w.iter().zip(r).enumerate() {
                assert_same(w, r, &format!("{at}[{i}]"));
            }
        }
        _ => assert_eq!(written, recorded, "{at}"),
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
struct Always(Vec<u8>);

impl Transport for Always {
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        Ok(self.0.clone())
    }
}

/// A recording with one change made by `edit`, as bytes.
fn edited(name: &str, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value = recorded(name);
    edit(&mut value);
    serde_json::to_vec(&value).unwrap()
}

/// The text of a download's `i`th file, decoded here from the recording.
fn recorded_file_text(name: &str, i: usize) -> String {
    use base64::Engine as _;
    let data = recorded(name)["results"][i]["data"]
        .as_str()
        .unwrap()
        .to_owned();
    String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .unwrap(),
    )
    .unwrap()
}

/// Each search and download, fetched through the cache, reads back to every value recorded, with
/// ThrustCurve's credit; a second read comes from the cache, and offline, with a transport that
/// may not be called, from the cache again: fresh for a day, stale after.
#[test]
fn each_answer_reads_to_its_values_then_works_offline_from_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);

    for (maker, file) in SEARCHES {
        let search = Search::manufacturer(maker);
        let (answer, fetched) = thrustcurve::fetch_search(&online, &search, NOW_S).unwrap();
        assert_reads_back(&answer, &recorded(file), file);
        assert!(answer.is_complete(), "{file}");
        assert_eq!(
            (fetched.freshness, fetched.attribution.as_str()),
            (Freshness::Fetched, ATTRIBUTION)
        );
        let (again, cached) = thrustcurve::fetch_search(&online, &search, NOW_S + 60).unwrap();
        assert_eq!((&again, cached.freshness), (&answer, Freshness::Cached));
        for (now_s, freshness) in [
            (NOW_S + TTL_S - 1, Freshness::Cached),
            (NOW_S + TTL_S, Freshness::Stale),
        ] {
            let (read, f) = thrustcurve::fetch_search(&offline, &search, now_s).unwrap();
            assert_eq!((&read, f.freshness), (&answer, freshness), "{file}");
            assert_eq!(f.attribution, ATTRIBUTION);
        }
    }
    for (id, format, file) in [
        (J450DM_ID, Format::Rasp, J450DM_FILE),
        (F27_ID, Format::RockSim, F27_FILE),
    ] {
        let download = Download::new(id, format).unwrap();
        let (answer, fetched) = thrustcurve::fetch_download(&online, &download, NOW_S).unwrap();
        assert_reads_back(&answer, &recorded(file), file);
        assert_eq!(fetched.attribution, ATTRIBUTION);
        let (read, f) = thrustcurve::fetch_download(&offline, &download, NOW_S + TTL_S).unwrap();
        assert_eq!((&read, f.freshness), (&answer, Freshness::Stale));
    }
    assert_eq!(transport.calls(), 5);
}

/// The searches' counts and words, read from the stand-ins: each answer whole (as many records
/// as it says match), every record of one maker, with the maker's full name as the motor finder
/// writes it.
#[test]
fn each_search_is_whole_and_of_one_maker() {
    for ((maker, file), count) in SEARCHES.into_iter().zip([158, 104, 35]) {
        let answer = thrustcurve::parse_search(&fixture(file)).unwrap();
        let raw = recorded(file);
        assert_eq!(raw["matches"].as_u64(), Some(count), "{file}");
        assert_eq!(
            (answer.matches, answer.results.len()),
            (count as u32, count as usize)
        );
        assert!(
            answer.results.iter().all(|r| r.manufacturer == maker),
            "{file}"
        );
        assert_eq!(answer.criteria[0].name, "manufacturer");
        assert_eq!(answer.criteria[0].value, maker);
    }
    let aerotech = thrustcurve::parse_search(&fixture(SEARCHES[0].1)).unwrap();
    let j450 = aerotech
        .results
        .iter()
        .find(|r| r.motor_id == J450DM_ID)
        .unwrap();
    assert_eq!(j450.designation, "J450DM");
    // An invented count, unlike the other stand-ins' 1, so this reads J450DM's own record.
    assert_eq!(j450.data_files, Some(3));
    // A DMS, AeroTech's single-use motor.
    assert_eq!(j450.motor_type.as_deref(), Some("SU"));
}

/// The stand-ins hold no fact beyond the committed answers (ADR-145): each record not named
/// `-INVENTED` is a motor of the finder's in-stock answer, carries only fields that answer
/// states, with its values (availability from its `discontinued`), and adds only an invented
/// file count and an invented id (`e000…`), bar the ids of the two recorded downloads. Every
/// motor in stock has one.
#[test]
fn each_stand_in_record_carries_only_the_finders_values() {
    // A stand-in field, and the finder's field it copies.
    const COPIED: [(&str, &str); 14] = [
        ("manufacturer", "manufacturer"),
        ("designation", "designation"),
        ("commonName", "common_name"),
        ("impulseClass", "impulse_class"),
        ("diameter", "diameter_mm"),
        ("type", "motor_type"),
        ("avgThrustN", "avg_thrust_n"),
        ("totImpulseNs", "total_impulse_ns"),
        ("burnTimeS", "burn_time_s"),
        ("delays", "delays"),
        ("delayAdjustable", "delay_adjustable"),
        ("caseInfo", "case_info"),
        ("propInfo", "propellant"),
        ("sparky", "sparky"),
    ];
    let stock = recorded("motor-finder-in-stock.json");
    let finder: BTreeMap<(&str, &str), &Value> = stock["motors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            let name = |k: &str| m[k].as_str().unwrap();
            ((name("manufacturer"), name("designation")), m)
        })
        .collect();
    let searches: Vec<Value> = SEARCHES.iter().map(|(_, f)| recorded(f)).collect();
    let mut seen = std::collections::BTreeSet::new();
    for r in searches
        .iter()
        .flat_map(|s| s["results"].as_array().unwrap())
    {
        let key = (
            r["manufacturer"].as_str().unwrap(),
            r["designation"].as_str().unwrap(),
        );
        if key.1.ends_with("-INVENTED") {
            continue;
        }
        let m = finder
            .get(&key)
            .unwrap_or_else(|| panic!("{key:?} is not in the finder's in-stock answer"));
        for (field, value) in r.as_object().unwrap() {
            match field.as_str() {
                "motorId" => {
                    let id = value.as_str().unwrap();
                    match id {
                        J450DM_ID => assert_eq!(key.1, "J450DM"),
                        F27_ID => assert_eq!(key.1, "F27R/L"),
                        _ => assert!(id.starts_with("e000"), "{key:?}: {id}"),
                    }
                }
                "dataFiles" => {}
                "availability" => {
                    let discontinued = m["discontinued"].as_bool().unwrap();
                    let expected = if discontinued { "OOP" } else { "regular" };
                    assert_eq!(value, expected, "{key:?}");
                }
                _ => {
                    let (_, from) = COPIED
                        .iter()
                        .find(|(f, _)| f == field)
                        .unwrap_or_else(|| panic!("{key:?}: the finder doesn't state {field}"));
                    assert_eq!(value, &m[*from], "{key:?}.{field}");
                }
            }
        }
        assert!(seen.insert(key), "{key:?} twice");
    }
    assert_eq!(seen.len(), finder.len());
}

/// The done-when's first half: the designation to ThrustCurve id mapping covers at least 95% of
/// the motors in stock, and its report of the misses is the one committed in
/// `validation/reports/thrustcurve-join.md` (`HPR_WRITE_THRUSTCURVE_JOIN=1` rewrites it). The
/// expected mapping is worked out here from the recordings' JSON.
#[test]
fn the_in_stock_motors_map_to_thrustcurve_ids_with_a_report_of_the_misses() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let client = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let (in_stock, _) = motor_finder::fetch_in_stock(&client, NOW_S).unwrap();
    let (records, fetched) = thrustcurve::fetch_finder_records(&client, NOW_S).unwrap();
    assert_eq!((records.len(), fetched.len()), (297, 3));
    assert!(fetched.iter().all(|f| f.attribution == ATTRIBUTION));
    let join = thrustcurve::join(&in_stock.motors, &records);

    let mut ids: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (_, file) in SEARCHES {
        for r in recorded(file)["results"].as_array().unwrap() {
            let key = (
                r["manufacturer"].as_str().unwrap().to_owned(),
                r["designation"].as_str().unwrap().to_owned(),
            );
            ids.entry(key)
                .or_default()
                .push(r["motorId"].as_str().unwrap().to_owned());
        }
    }
    let stock = recorded("motor-finder-in-stock.json");
    let stock = stock["motors"].as_array().unwrap();
    let expected: Vec<(String, String, String)> = stock
        .iter()
        .filter_map(|m| {
            let key = (
                m["manufacturer"].as_str().unwrap().to_owned(),
                m["designation"].as_str().unwrap().to_owned(),
            );
            match ids.get(&key).map(Vec::as_slice) {
                Some([id]) => Some((key.0, key.1, id.clone())),
                _ => None,
            }
        })
        .collect();
    let mapped: Vec<_> = join
        .mapped
        .iter()
        .map(|m| {
            (
                m.manufacturer.clone(),
                m.designation.clone(),
                m.record.motor_id.clone(),
            )
        })
        .collect();
    assert_eq!(mapped, expected);
    assert_eq!(join.misses.len(), stock.len() - expected.len());

    let (hits, total) = join.coverage();
    assert_eq!((hits, total), (282, 282));
    assert!(hits * 100 >= total * 95, "{hits} of {total}");
    // True by construction on the stand-ins, whose file counts are invented (ADR-145): this checks
    // the code only. That all 282 records list a file is the measurement on ThrustCurve's own
    // answers of 2026-10-01 (ADR-130), last run at commit 5150e0d.
    assert!(join.mapped.iter().all(|m| m.record.data_files >= Some(1)));
    // Each stand-in record carries its finder motor's figures, so each match is the right record
    // by more than its name: diameter, total impulse, average thrust and burn time agree. (On
    // ThrustCurve.org's own records, recorded 2026-10-01, they agreed for all 282: ADR-130.)
    for m in &join.mapped {
        let motor = &in_stock.motors[m.finder_index];
        assert_eq!(
            (motor.manufacturer.as_str(), motor.designation.as_str()),
            (m.manufacturer.as_str(), m.designation.as_str())
        );
        let figures = [
            (Some(motor.diameter_mm), m.record.diameter_mm),
            (motor.total_impulse_ns, m.record.total_impulse_ns),
            (motor.avg_thrust_n, m.record.avg_thrust_n),
            (motor.burn_time_s, m.record.burn_time_s),
        ];
        for (finder, record) in figures {
            assert!(
                finder.is_some() && finder == record,
                "{}: {figures:?}",
                m.designation
            );
        }
    }

    let path = root().join("validation/reports/thrustcurve-join.md");
    let report = std::fs::read_to_string(&path).unwrap();
    let start = report.find(REPORT_START).unwrap() + REPORT_START.len();
    let end = report.find(REPORT_END).unwrap();
    let written = join.report();
    if std::env::var_os("HPR_WRITE_THRUSTCURVE_JOIN").is_some() {
        let rewritten = format!("{}\n{written}{}", &report[..start], &report[end..]);
        std::fs::write(&path, rewritten).unwrap();
    } else {
        assert_eq!(
            report[start..end].trim(),
            written.trim(),
            "rerun with HPR_WRITE_THRUSTCURVE_JOIN=1"
        );
    }
}

/// The done-when's second half: J450DM, a mapped motor in stock, has its recorded curve read by
/// `hpr_motor`, and it is the very file `hpr_motor` bundles from ThrustCurve.org (public domain,
/// downloaded 2026-09-17), byte for byte.
#[test]
fn a_mapped_motors_recorded_curve_reads_with_hpr_motor() {
    let in_stock = motor_finder::parse_in_stock(&fixture("motor-finder-in-stock.json")).unwrap();
    let records: Vec<_> = SEARCHES
        .iter()
        .flat_map(|(_, f)| thrustcurve::parse_search(&fixture(f)).unwrap().results)
        .collect();
    let join = thrustcurve::join(&in_stock.motors, &records);
    let j450 = join
        .mapped
        .iter()
        .find(|m| m.designation == "J450DM")
        .unwrap();
    assert_eq!(
        (j450.manufacturer.as_str(), j450.record.motor_id.as_str()),
        ("AeroTech", J450DM_ID)
    );

    let answer = thrustcurve::parse_download(&fixture(J450DM_FILE)).unwrap();
    assert_eq!(answer.results.len(), 1);
    let file = &answer.results[0];
    assert_eq!(
        (file.motor_id.as_str(), file.format),
        (J450DM_ID, Format::Rasp)
    );
    assert_eq!(
        (file.source.as_deref(), file.license.as_deref()),
        (Some("cert"), Some("PD"))
    );
    let text = file.text().unwrap();
    assert_eq!(text, recorded_file_text(J450DM_FILE, 0));
    let bundled =
        hpr_motor::catalog::bundled_curve_text("curves/5f4294d20002e9000000086b.eng").unwrap();
    assert_eq!(text, bundled);

    let Curve::Rasp(parsed) = file.read().unwrap() else {
        panic!("a RASP file read as another format");
    };
    assert_eq!(parsed, hpr_motor::eng::parse(bundled).unwrap());
    let entry = &parsed.value.entries[0];
    assert_eq!((entry.name.as_str(), entry.diameter_mm), ("J450DM", 54.0));
    let curve = Curve::Rasp(parsed.clone()).thrust_curve().unwrap();
    // The points, read here from the file's own lines: the curve is them after its start at zero.
    let points: Vec<(f64, f64)> = text
        .lines()
        .skip(2)
        .filter_map(|l| {
            let mut v = l.split_whitespace().map(|x| x.parse::<f64>().unwrap());
            Some((v.next()?, v.next()?))
        })
        .collect();
    assert_eq!(points.len(), 36);
    assert_eq!((curve.times_s()[0], curve.thrusts_n()[0]), (0.0, 0.0));
    let read: Vec<(f64, f64)> = curve.times_s()[1..]
        .iter()
        .copied()
        .zip(curve.thrusts_n()[1..].iter().copied())
        .collect();
    assert_eq!(read, points);
}

/// A RockSim file reads with `hpr_motor`'s `.rse` reader: F27R/L's one public-domain file, its
/// points the ones its XML holds.
#[test]
fn a_rocksim_file_reads_with_hpr_motor() {
    let answer = thrustcurve::parse_download(&fixture(F27_FILE)).unwrap();
    let file = &answer.results[0];
    assert_eq!(
        (file.format, file.license.as_deref()),
        (Format::RockSim, Some("PD"))
    );
    let Curve::RockSim(parsed) = file.read().unwrap() else {
        panic!("a RockSim file read as another format");
    };
    let text = recorded_file_text(F27_FILE, 0);
    assert_eq!(parsed, hpr_motor::rse::parse(&text).unwrap());
    let curve = Curve::RockSim(parsed.clone()).thrust_curve().unwrap();
    let attribute = |line: &str, name: &str| -> f64 {
        let at = line.find(&format!(" {name}=\"")).unwrap() + name.len() + 3;
        line[at..at + line[at..].find('"').unwrap()]
            .parse()
            .unwrap()
    };
    let points: Vec<(f64, f64)> = text
        .lines()
        .filter(|l| l.contains("<eng-data"))
        .map(|l| (attribute(l, "t"), attribute(l, "f")))
        .collect();
    assert!(points.len() > 10, "{}", points.len());
    let read: Vec<(f64, f64)> = curve
        .times_s()
        .iter()
        .copied()
        .zip(curve.thrusts_n().iter().copied())
        .filter(|&(t, _)| t > 0.0 || points[0].0 == 0.0)
        .collect();
    assert_eq!(read, points);
}

/// A motor with no record of its name, or several, is a miss with its reason, and the report
/// lists it.
#[test]
fn a_miss_is_reported_with_its_reason() {
    let mut in_stock =
        motor_finder::parse_in_stock(&fixture("motor-finder-in-stock.json")).unwrap();
    let mut records: Vec<_> = SEARCHES
        .iter()
        .flat_map(|(_, f)| thrustcurve::parse_search(&fixture(f)).unwrap().results)
        .collect();
    let renamed = in_stock
        .motors
        .iter()
        .position(|m| m.designation == "J450DM")
        .unwrap();
    in_stock.motors[renamed].designation = "J450-DM".to_owned();
    let twice = records
        .iter()
        .find(|r| r.designation == "H125-CT")
        .unwrap()
        .clone();
    let mut copy = twice.clone();
    copy.motor_id = "000000000000000000000000".to_owned();
    records.push(copy);
    // A record in another case is not the same name.
    let lower = in_stock
        .motors
        .iter()
        .position(|m| m.designation == "E26W")
        .unwrap();
    in_stock.motors[lower].designation = "e26w".to_owned();
    // A record given twice, as two overlapping searches would, is one record.
    let again = records
        .iter()
        .find(|r| r.designation == "26E31-15A")
        .unwrap()
        .clone();
    records.push(again);
    // One record listing no data file, one leaving the count out: both counted in the report.
    let no_files = records
        .iter_mut()
        .find(|r| r.designation == "D13W")
        .unwrap();
    no_files.data_files = Some(0);
    let unstated = records
        .iter_mut()
        .find(|r| r.designation == "26E31-15A")
        .unwrap();
    unstated.data_files = None;

    // Another maker's record of the same designation is another motor: K400C still maps to
    // AeroTech's.
    let k400 = records
        .iter()
        .find(|r| r.designation == "K400C")
        .unwrap()
        .clone();
    let mut namesake = k400.clone();
    namesake.manufacturer = "Loki Research".to_owned();
    namesake.motor_id = "000000000000000000000002".to_owned();
    records.push(namesake);

    let join = thrustcurve::join(&in_stock.motors, &records);
    assert_eq!(join.coverage(), (279, 282));
    let k400_mapped = join
        .mapped
        .iter()
        .find(|m| m.designation == "K400C")
        .unwrap();
    assert_eq!(
        (k400_mapped.manufacturer.as_str(), &k400_mapped.record),
        ("AeroTech", &k400)
    );
    let misses: Vec<_> = join
        .misses
        .iter()
        .map(|m| (m.designation.as_str(), m.reason.clone()))
        .collect();
    let several = MissReason::SeveralRecords(vec![
        twice.motor_id.clone(),
        "000000000000000000000000".to_owned(),
    ]);
    for miss in [
        ("J450-DM", MissReason::NoRecord),
        ("H125-CT", several),
        ("e26w", MissReason::NoRecord),
    ] {
        assert!(misses.contains(&miss), "{miss:?} in {misses:?}");
    }
    let report = join.report();
    assert!(report.starts_with("279 of 282 motors mapped"), "{report}");
    for row in [
        "| AeroTech | 153 | 151 | 1 | 2 |",
        "| Cesaroni Technology | 99 | 99 | 1 | 0 |",
        "| Loki Research | 30 | 29 | 0 | 1 |",
    ] {
        assert!(report.contains(row), "{row} in {report}");
    }
    assert!(
        report.contains("| AeroTech | `J450-DM` | no record of that name |"),
        "{report}"
    );
    assert!(
        report.contains(&format!(
            "| Loki Research | `H125-CT` | 2 records of that name: {}, 000000000000000000000000 |",
            twice.motor_id
        )),
        "{report}"
    );
}

/// Each rule the parsers check refuses an answer that breaks it, naming what broke.
#[test]
fn each_rule_refuses_an_answer_that_breaks_it() {
    let search = SEARCHES[2].1;
    let api_error = edited(search, |v| {
        v["error"] = "Invalid manufacturer value.".into()
    });
    assert!(matches!(
        thrustcurve::parse_search(&api_error),
        Err(ThrustCurveError::Api(e)) if e == "Invalid manufacturer value."
    ));
    // On the second criterion, so a check of the first alone would miss it.
    let criterion_error = edited(search, |v| {
        v["criteria"][1]["error"] = "Invalid maxResults value \"x\".".into();
    });
    assert!(matches!(
        thrustcurve::parse_search(&criterion_error),
        Err(ThrustCurveError::Api(e)) if e == "maxResults: Invalid maxResults value \"x\"."
    ));
    let too_many = edited(search, |v| v["matches"] = 34.into());
    assert!(matches!(
        thrustcurve::parse_search(&too_many),
        Err(ThrustCurveError::Count {
            matches: 34,
            found: 35
        })
    ));
    let bad_id = edited(search, |v| v["results"][3]["motorId"] = "5f42".into());
    assert!(matches!(
        thrustcurve::parse_search(&bad_id),
        Err(ThrustCurveError::Field { field, value }) if field == "results[3].motorId" && value == "5f42"
    ));
    for name in [
        "diameter",
        "length",
        "avgThrustN",
        "maxThrustN",
        "totImpulseNs",
        "burnTimeS",
        "totalWeightG",
        "propWeightG",
    ] {
        let negative = edited(search, |v| v["results"][7][name] = (-1.5).into());
        assert!(
            matches!(
                thrustcurve::parse_search(&negative),
                Err(ThrustCurveError::Field { field, value })
                    if field == format!("results[7].{name}") && value == "-1.5"
            ),
            "{name}"
        );
    }
    let no_designation = edited(search, |v| {
        v["results"][0]
            .as_object_mut()
            .unwrap()
            .remove("designation");
    });
    assert!(matches!(
        thrustcurve::parse_search(&no_designation),
        Err(ThrustCurveError::Json(_))
    ));
    // Every field but the three the join needs may be absent: the API returns only those with
    // values.
    let sparse = edited(search, |v| {
        let motor = v["results"][0].as_object_mut().unwrap();
        motor.retain(|k, _| ["motorId", "manufacturer", "designation"].contains(&k.as_str()));
    });
    let read = thrustcurve::parse_search(&sparse).unwrap();
    assert_reads_back(&read, &serde_json::from_slice(&sparse).unwrap(), "sparse");

    let download_error = edited(J450DM_FILE, |v| {
        v["error"] = "Invalid format value \"ANG\".".into()
    });
    assert!(matches!(
        thrustcurve::parse_download(&download_error),
        Err(ThrustCurveError::Api(e)) if e == "Invalid format value \"ANG\"."
    ));
    let not_base64 = edited(J450DM_FILE, |v| {
        v["results"][0]["data"] = "not base64!".into()
    });
    assert!(matches!(
        thrustcurve::parse_download(&not_base64),
        Err(ThrustCurveError::Field { field, .. }) if field == "results[0].data"
    ));
    // A file that decodes but isn't UTF-8 text is that file's error, not the answer's: the
    // motor's other files stay readable.
    let not_utf8 = edited(J450DM_FILE, |v| {
        let mut second = v["results"][0].clone();
        second["data"] = "/w==".into();
        second["simfileId"] = "000000000000000000000001".into();
        v["results"].as_array_mut().unwrap().push(second);
    });
    let answer = thrustcurve::parse_download(&not_utf8).unwrap();
    assert!(answer.results[0].read().is_ok());
    assert!(matches!(
        answer.results[1].text(),
        Err(ThrustCurveError::Field { field, .. }) if field == "000000000000000000000001.data"
    ));
    let other_format = edited(J450DM_FILE, |v| v["results"][0]["format"] = "ANG".into());
    assert!(matches!(
        thrustcurve::parse_download(&other_format),
        Err(ThrustCurveError::Json(_))
    ));
    let bad_download_id = edited(J450DM_FILE, |v| v["results"][0]["motorId"] = "x".into());
    assert!(matches!(
        thrustcurve::parse_download(&bad_download_id),
        Err(ThrustCurveError::Field { field, value }) if field == "results[0].motorId" && value == "x"
    ));
    let empty = br#"{"results": []}"#;
    assert!(
        thrustcurve::parse_download(empty)
            .unwrap()
            .results
            .is_empty()
    );
}

/// A file a reader refuses is an error from `hpr_motor`, not a curve.
#[test]
fn a_file_the_reader_refuses_is_a_motor_error() {
    use base64::Engine as _;
    let garbage = base64::engine::general_purpose::STANDARD.encode("J450DM 54 359 14 0.51\n1 x\n");
    let body = edited(J450DM_FILE, |v| v["results"][0]["data"] = garbage.into());
    let answer = thrustcurve::parse_download(&body).unwrap();
    assert!(matches!(
        answer.results[0].read(),
        Err(ThrustCurveError::Motor(_))
    ));
}

/// A download holding another motor's file, or a file in another format, is refused and not
/// cached; so is a search cut short by its `maxResults`, when the join's records are fetched.
#[test]
fn an_answer_for_something_else_is_refused_and_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    let j450 = Download::new(J450DM_ID, Format::Rasp).unwrap();
    for (body, found) in [
        (fixture(F27_FILE), format!("{F27_ID} (RockSim)")),
        (
            edited(J450DM_FILE, |v| {
                v["results"][0]["format"] = "RockSim".into()
            }),
            format!("{J450DM_ID} (RockSim)"),
        ),
    ] {
        let online = Client::new(Always(body), Cache::new(dir.path()), Mode::Online);
        let err = thrustcurve::fetch_download(&online, &j450, NOW_S).unwrap_err();
        let ThrustCurveError::Net(NetError::Refused { reason, .. }) = &err else {
            panic!("{err}");
        };
        assert!(reason.contains(&format!("holds {found}'s")), "{reason}");
        assert!(matches!(
            thrustcurve::fetch_download(&offline, &j450, NOW_S),
            Err(ThrustCurveError::Net(NetError::NotCached { .. }))
        ));
    }

    let cut_short = edited(SEARCHES[0].1, |v| {
        v["results"].as_array_mut().unwrap().truncate(20);
    });
    let online = Client::new(Always(cut_short), Cache::new(dir.path()), Mode::Online);
    let err = thrustcurve::fetch_finder_records(&online, NOW_S).unwrap_err();
    let ThrustCurveError::Net(NetError::Refused { reason, .. }) = &err else {
        panic!("{err}");
    };
    assert!(
        reason.contains("matches 158 motors but returns only 20"),
        "{reason}"
    );
    assert!(matches!(
        thrustcurve::fetch_search(&offline, &Search::manufacturer("AeroTech"), NOW_S),
        Err(ThrustCurveError::Net(NetError::NotCached { .. }))
    ));

    // Loki's whole answer, served for AeroTech's search: another maker's records.
    let online = Client::new(
        Always(fixture(SEARCHES[2].1)),
        Cache::new(dir.path()),
        Mode::Online,
    );
    let err = thrustcurve::fetch_finder_records(&online, NOW_S).unwrap_err();
    let ThrustCurveError::Net(NetError::Refused { reason, .. }) = &err else {
        panic!("{err}");
    };
    assert!(
        reason.contains(
            "asked ThrustCurve for AeroTech's motors, but the answer holds one of Loki Research's"
        ),
        "{reason}"
    );
    assert!(matches!(
        thrustcurve::fetch_search(&offline, &Search::manufacturer("AeroTech"), NOW_S),
        Err(ThrustCurveError::Net(NetError::NotCached { .. }))
    ));
}
