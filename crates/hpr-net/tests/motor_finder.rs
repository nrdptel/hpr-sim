//! motor.fusionspace.co's API against recorded answers (M5.4a): every endpoint's answer reads to
//! the values in it, a second read works offline from the cache, and every answer carries the
//! credit the API asks for.
//!
//! The expected values are read from the recordings here, with `serde_json`, not through the
//! parser under test. All eight recordings come from one build of the site, 2026-10-01 07:07:29
//! UTC.

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

use std::cell::Cell;
use std::path::Path;

use hpr_net::motor_finder::{
    self, ATTRIBUTION, Endpoint, ListingStatus, MotorFinderError, MotorList, TTL_S,
};
use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Transport};
use serde::Serialize;
use serde_json::Value;

/// 2026-10-01 07:30 UTC, just after the answers were recorded.
const NOW_S: u64 = 1_790_839_800;

/// The recorded motor pages: manufacturer, designation, file.
const PAGES: [(&str, &str, &str); 4] = [
    ("AeroTech", "H128W", "motor-finder-aerotech-H128W.json"),
    ("AeroTech", "F27R/L", "motor-finder-aerotech-F27R_L.json"),
    (
        "Cesaroni Technology",
        "3683L851-P",
        "motor-finder-cesaroni-3683L851-P.json",
    ),
    (
        "Loki Research",
        "J-326-LR",
        "motor-finder-loki-J-326-LR.json",
    ),
];

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
/// numbers compared as numbers, so a recorded `128` matches a read `128.0`.
fn assert_reads_back(read: &impl Serialize, recording: &Value, at: &str) {
    let written = serde_json::to_value(read).unwrap();
    assert_same(&written, recording, at);
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

/// The done-when of M5.4a, for the four lists: each recorded answer reads to every value in it,
/// with the credit the API asks for; a second read online comes from the cache; offline, with a
/// transport that may not be called, it still answers from the cache, stale after an hour.
#[test]
fn each_list_reads_to_its_values_then_works_offline_from_the_cache() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);

    let (meta, fetched) = motor_finder::fetch_meta(&online, NOW_S).unwrap();
    assert_reads_back(&meta, &recorded("motor-finder-meta.json"), "meta");
    assert_eq!(
        (fetched.freshness, fetched.attribution.as_str()),
        (Freshness::Fetched, ATTRIBUTION)
    );
    let (motors, fetched) = motor_finder::fetch_motors(&online, NOW_S).unwrap();
    assert_reads_back(&motors, &recorded("motor-finder-motors.json"), "motors");
    assert_eq!(fetched.attribution, ATTRIBUTION);
    let (in_stock, fetched) = motor_finder::fetch_in_stock(&online, NOW_S).unwrap();
    assert_reads_back(
        &in_stock,
        &recorded("motor-finder-in-stock.json"),
        "in-stock",
    );
    assert_eq!(fetched.attribution, ATTRIBUTION);
    let (vendors, fetched) = motor_finder::fetch_vendors(&online, NOW_S).unwrap();
    assert_reads_back(&vendors, &recorded("motor-finder-vendors.json"), "vendors");
    assert_eq!(fetched.attribution, ATTRIBUTION);
    assert_eq!(transport.calls(), 4);

    let (again, cached) = motor_finder::fetch_in_stock(&online, NOW_S + 60).unwrap();
    assert_eq!((&again, cached.freshness), (&in_stock, Freshness::Cached));
    assert_eq!(transport.calls(), 4, "the second read fetched");

    for (later, freshness) in [
        (NOW_S + TTL_S - 1, Freshness::Cached),
        (NOW_S + TTL_S, Freshness::Stale),
    ] {
        let (read, fetched) = motor_finder::fetch_meta(&offline, later).unwrap();
        assert_eq!((&read, fetched.freshness), (&meta, freshness));
        assert_eq!(
            (fetched.fetched_at_s, fetched.attribution.as_str()),
            (NOW_S, ATTRIBUTION)
        );
        let (read, fetched) = motor_finder::fetch_motors(&offline, later).unwrap();
        assert_eq!((&read, fetched.freshness), (&motors, freshness));
        let (read, fetched) = motor_finder::fetch_in_stock(&offline, later).unwrap();
        assert_eq!((&read, fetched.freshness), (&in_stock, freshness));
        let (read, fetched) = motor_finder::fetch_vendors(&offline, later).unwrap();
        assert_eq!((&read, fetched.freshness), (&vendors, freshness));
        assert_eq!(fetched.attribution, ATTRIBUTION);
    }
}

/// The done-when of M5.4a, for one motor's page: each recorded page reads to every value in it,
/// is the motor of the same build's `motors.json`, and reads offline from the cache.
#[test]
fn each_motors_page_reads_to_its_values_then_works_offline_from_the_cache() {
    let all = motor_finder::parse_motors(&fixture("motor-finder-motors.json")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    for (manufacturer, designation, file) in PAGES {
        let (motor, fetched) =
            motor_finder::fetch_motor(&online, manufacturer, designation, NOW_S).unwrap();
        let recording = recorded(file);
        assert_reads_back(&motor, &recording["motor"], file);
        assert_eq!(
            (fetched.freshness, fetched.attribution.as_str()),
            (Freshness::Fetched, ATTRIBUTION)
        );
        let page = motor_finder::parse_motor(&fixture(file)).unwrap();
        assert_reads_back(&page, &recording, file);
        let listed = all
            .motors
            .iter()
            .find(|m| m.manufacturer == manufacturer && m.designation == designation)
            .unwrap();
        assert_eq!(&motor, listed, "{file}");
        assert_eq!(page.generated_at, all.generated_at);

        let (read, fetched) =
            motor_finder::fetch_motor(&offline, manufacturer, designation, NOW_S + 60).unwrap();
        assert_eq!((read, fetched.freshness), (motor, Freshness::Cached));
    }
    assert_eq!(transport.calls(), PAGES.len());
    // The manufacturer may be named by its slug, in any case: the same file, from the cache.
    let (read, _) = motor_finder::fetch_motor(&offline, "AEROTECH", "H128W", NOW_S).unwrap();
    assert_eq!(read.designation, "H128W");
}

/// The recording is one build: `in-stock.json` is `motors.json`'s motors in stock, in order and
/// value for value, and `meta.json` counts both lists and the vendors.
#[test]
fn the_recorded_files_agree_with_each_other() {
    let meta = motor_finder::parse_meta(&fixture("motor-finder-meta.json")).unwrap();
    let all = motor_finder::parse_motors(&fixture("motor-finder-motors.json")).unwrap();
    let in_stock = motor_finder::parse_in_stock(&fixture("motor-finder-in-stock.json")).unwrap();
    let vendors = motor_finder::parse_vendors(&fixture("motor-finder-vendors.json")).unwrap();
    let stocked: Vec<_> = all.motors.iter().filter(|m| m.in_stock).collect();
    assert_eq!(stocked, in_stock.motors.iter().collect::<Vec<_>>());
    assert_eq!(
        (
            meta.counts.motors,
            meta.counts.in_stock,
            meta.counts.vendors
        ),
        (598, 282, 12)
    );
    assert_eq!(all.motors.len(), 598);
    assert_eq!(in_stock.motors.len(), 282);
    assert_eq!(vendors.vendors.len(), 12);
    for list in [&all, &in_stock] {
        assert_eq!(list.generated_at, meta.generated_at);
    }
    assert_eq!(vendors.generated_at, meta.generated_at);
    assert_eq!(
        motor_finder::unix_s(&meta.generated_at),
        Some(1_790_838_449)
    );
    let names: Vec<&str> = motor_finder::MANUFACTURERS.iter().map(|m| m.0).collect();
    assert_eq!(meta.manufacturers, names);
    assert!(
        all.motors
            .iter()
            .all(|m| names.contains(&m.manufacturer.as_str()))
    );
}

/// What the recording shows of rules the API states but the parser leaves unchecked: a motor's
/// page is at its `path`; the unit price is the sticker price over the pack size, rounded half
/// up; the cheapest offer is an in-stock listing of the lowest unit price; a motor is in stock
/// exactly when a listing is; and the vendor counts count distinct vendors.
#[test]
fn the_recording_keeps_the_apis_stated_rules() {
    let all: MotorList = motor_finder::parse_motors(&fixture("motor-finder-motors.json")).unwrap();
    let mut listings = 0;
    for m in &all.motors {
        let page = Endpoint::motor(&m.manufacturer, &m.designation).unwrap();
        assert_eq!(
            page.url(),
            format!("https://motor.fusionspace.co{}", m.path)
        );
        for l in &m.listings {
            let (price, unit) = (l.price_cents.unwrap(), l.unit_price_cents.unwrap());
            let pack = u64::from(l.pack_size);
            assert_eq!(unit, (2 * price + pack) / (2 * pack), "{}", m.designation);
            assert_eq!(l.currency, "USD");
            listings += 1;
        }
        let stocked: Vec<_> = m
            .listings
            .iter()
            .filter(|l| l.status == ListingStatus::InStock)
            .collect();
        assert_eq!(m.in_stock, !stocked.is_empty(), "{}", m.designation);
        if let Some(offer) = &m.cheapest_in_stock {
            let lowest = stocked.iter().filter_map(|l| l.unit_price_cents).min();
            assert_eq!(Some(offer.unit_price_cents), lowest, "{}", m.designation);
            assert!(
                stocked
                    .iter()
                    .any(|l| l.url == offer.url && l.vendor_slug == offer.vendor_slug)
            );
        }
        let distinct = |listed: &[&hpr_net::motor_finder::Listing]| {
            let mut slugs: Vec<&str> = listed.iter().map(|l| l.vendor_slug.as_str()).collect();
            slugs.sort_unstable();
            slugs.dedup();
            slugs.len()
        };
        let every: Vec<_> = m.listings.iter().collect();
        assert_eq!(
            m.vendor_count as usize,
            distinct(&every),
            "{}",
            m.designation
        );
        assert_eq!(
            m.in_stock_vendor_count as usize,
            distinct(&stocked),
            "{}",
            m.designation
        );
    }
    assert_eq!(listings, 3_685);
}

/// A recorded body with one value changed, as JSON.
fn changed(name: &str, change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut json = recorded(name);
    change(&mut json);
    serde_json::to_vec(&json).unwrap()
}

/// A change that breaks a rule: its name, the change, and the field and the start of the value
/// the refusal names.
type Case = (&'static str, fn(&mut Value), &'static str, &'static str);

/// Each rule the parser checks refuses an answer that breaks it, naming the field.
#[test]
fn answers_that_break_the_apis_rules_are_refused() {
    let field = |result: Result<MotorList, MotorFinderError>| match result {
        Err(MotorFinderError::Field { field, value }) => (field, value),
        other => panic!("{other:?}"),
    };
    let motors = "motor-finder-motors.json";
    let cases: [Case; 9] = [
        (
            "class",
            |j| j["motors"][3]["impulse_class"] = "LL".into(),
            "motors[3].impulse_class",
            "LL",
        ),
        (
            "diameter",
            |j| j["motors"][3]["diameter_mm"] = 0.into(),
            "motors[3].diameter_mm",
            "0",
        ),
        (
            "impulse",
            |j| j["motors"][3]["total_impulse_ns"] = (-1.5).into(),
            "motors[3].total_impulse_ns",
            "-1.5",
        ),
        (
            "listing count",
            |j| j["motors"][3]["listing_count"] = 99.into(),
            "motors[3].listing_count",
            "99 for ",
        ),
        (
            "cheapest out of stock",
            |j| j["motors"][1]["in_stock"] = false.into(),
            "motors[1].cheapest_in_stock",
            "in_stock false beside Some(",
        ),
        (
            "no cheapest in stock",
            |j| j["motors"][1]["cheapest_in_stock"] = Value::Null,
            "motors[1].cheapest_in_stock",
            "in_stock true beside None",
        ),
        (
            "pack",
            |j| j["motors"][2]["listings"][1]["pack_size"] = 0.into(),
            "motors[2].listings[1].pack_size",
            "0",
        ),
        (
            "unit price",
            |j| j["motors"][2]["listings"][1]["unit_price_cents"] = 1_000_000.into(),
            "motors[2].listings[1].unit_price_cents",
            "1000000 over a price of ",
        ),
        (
            "offer pack",
            |j| j["motors"][1]["cheapest_in_stock"]["pack_size"] = 0.into(),
            "motors[1].cheapest_in_stock.pack_size",
            "0",
        ),
    ];
    let all = recorded(motors);
    assert!(all["motors"][1]["in_stock"].as_bool().unwrap());
    for (name, change, at, value) in cases {
        let (found_at, found_value) = field(motor_finder::parse_motors(&changed(motors, change)));
        assert_eq!(found_at, at, "{name}");
        assert!(found_value.starts_with(value), "{name}: {found_value}");
    }

    let in_stock = "motor-finder-in-stock.json";
    let out = changed(in_stock, |j| {
        j["motors"][5]["in_stock"] = false.into();
        j["motors"][5]["cheapest_in_stock"] = Value::Null;
    });
    assert_eq!(
        field(motor_finder::parse_in_stock(&out)),
        ("motors[5].in_stock".to_owned(), "false".to_owned())
    );
    let late = changed(in_stock, |j| j["generated_at"] = "yesterday".into());
    assert_eq!(
        field(motor_finder::parse_in_stock(&late)),
        ("generated_at".to_owned(), "yesterday".to_owned())
    );

    let v2 = changed("motor-finder-meta.json", |j| j["schema_version"] = 2.into());
    assert!(matches!(
        motor_finder::parse_meta(&v2),
        Err(MotorFinderError::Schema { found: 2 })
    ));
    let short = changed(motors, |j| j["count"] = 597.into());
    assert!(matches!(
        motor_finder::parse_motors(&short),
        Err(MotorFinderError::Count {
            expected: 597,
            found: 598
        })
    ));
    let vendors = changed("motor-finder-vendors.json", |j| j["count"] = 13.into());
    assert!(matches!(
        motor_finder::parse_vendors(&vendors),
        Err(MotorFinderError::Count {
            expected: 13,
            found: 12
        })
    ));
    let status = changed(motors, |j| {
        j["motors"][0]["listings"][0]["status"] = "sold".into()
    });
    assert!(matches!(
        motor_finder::parse_motors(&status),
        Err(MotorFinderError::Json(_))
    ));
    let missing = changed("motor-finder-aerotech-H128W.json", |j| {
        j["motor"].as_object_mut().unwrap().remove("listings");
    });
    assert!(matches!(
        motor_finder::parse_motor(&missing),
        Err(MotorFinderError::Json(_))
    ));
    // The site answers an unknown motor with an HTML page; read as an answer, it is not JSON.
    assert!(matches!(
        motor_finder::parse_motor(b"<!DOCTYPE html><html>"),
        Err(MotorFinderError::Json(_))
    ));
}

/// A transport that answers each call with the next of its bodies.
struct Answers {
    bodies: Vec<Vec<u8>>,
    calls: Cell<usize>,
}

impl Transport for Answers {
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        Ok(self.bodies[call].clone())
    }
}

/// An answer that doesn't parse is never cached: with no copy the read fails, naming why; with a
/// good copy gone stale, the copy comes back with the reason, and stays cached.
#[test]
fn an_answer_that_does_not_parse_is_never_cached() {
    let dir = tempfile::tempdir().unwrap();
    let good = fixture("motor-finder-vendors.json");
    let transport = Answers {
        bodies: vec![b"<html>".to_vec(), good.clone(), b"<html>".to_vec()],
        calls: Cell::new(0),
    };
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    match motor_finder::fetch_vendors(&online, NOW_S) {
        Err(MotorFinderError::Net(NetError::Refused { reason, .. })) => {
            assert!(reason.starts_with("the motor finder answer is not of the expected shape"));
        }
        other => panic!("{other:?}"),
    }
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    assert!(matches!(
        motor_finder::fetch_vendors(&offline, NOW_S),
        Err(MotorFinderError::Net(NetError::NotCached { .. }))
    ));
    let (vendors, _) = motor_finder::fetch_vendors(&online, NOW_S).unwrap();
    let (stale, fetched) = motor_finder::fetch_vendors(&online, NOW_S + TTL_S).unwrap();
    assert_eq!((stale, fetched.freshness), (vendors, Freshness::Stale));
    assert!(
        fetched
            .stale_reason
            .unwrap()
            .contains("not of the expected shape")
    );
    assert_eq!(fetched.body, good);
}

/// A motor's page that holds another motor is refused, and never cached.
#[test]
fn a_page_for_another_motor_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let transport = Answers {
        bodies: vec![fixture("motor-finder-aerotech-H128W.json")],
        calls: Cell::new(0),
    };
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    match motor_finder::fetch_motor(&online, "loki", "J-326-LR", NOW_S) {
        Err(MotorFinderError::Net(NetError::Refused { reason, .. })) => assert_eq!(
            reason,
            "asked the motor finder for loki/J-326-LR, but its page is AeroTech/H128W's"
        ),
        other => panic!("{other:?}"),
    }
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    assert!(matches!(
        motor_finder::fetch_motor(&offline, "loki", "J-326-LR", NOW_S),
        Err(MotorFinderError::Net(NetError::NotCached { .. }))
    ));
}

/// Offline, a file never fetched is an error naming its URL; a bad request is refused before the
/// client is asked; a motor with no recording fails as a network error would.
#[test]
fn what_cannot_be_answered_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    match motor_finder::fetch_in_stock(&offline, NOW_S) {
        Err(MotorFinderError::Net(NetError::NotCached { url })) => {
            assert_eq!(url, "https://motor.fusionspace.co/api/v1/in-stock.json");
        }
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        motor_finder::fetch_motor(&offline, "Estes", "C6-5", NOW_S),
        Err(MotorFinderError::Request {
            what: "manufacturer",
            ..
        })
    ));
    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    match motor_finder::fetch_motor(&online, "aerotech", "NOPE999", NOW_S) {
        Err(MotorFinderError::Net(NetError::Transport { url, .. })) => {
            assert!(url.ends_with("/motors/aerotech/NOPE999.json"));
        }
        other => panic!("{other:?}"),
    }
}
