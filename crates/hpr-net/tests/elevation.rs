//! Open-Meteo's elevation API against recorded answers (M5.3b): a lookup's heights are the
//! answer's, and a second lookup works offline from the cache.
//!
//! The expected heights are read from the recordings here, with `serde_json`, not through the
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

use std::cell::Cell;
use std::path::Path;

use hpr_net::Fetched;
use hpr_net::elevation::{self, ATTRIBUTION, ElevationError, ElevationRequest, Place, TTL_S};
use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Transport};
use serde_json::Value;

/// 2026-10-01 00:00 UTC, about when the answers were recorded.
const NOW_S: u64 = 1_790_812_800;

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

/// A recording's heights, straight from the JSON.
fn recorded_heights(name: &str) -> Vec<f64> {
    let json: Value = serde_json::from_slice(&fixture(name)).unwrap();
    json["elevation"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h.as_f64().unwrap())
        .collect()
}

/// The recordings, each with the places it was asked for: Spaceport America's launch area alone;
/// then with the Dead Sea's shore, below sea level, and the open Atlantic.
fn recordings() -> [(&'static str, Vec<Place>); 2] {
    let spaceport = Place::new(32.99, -106.97);
    [
        ("open-meteo-elevation.json", vec![spaceport]),
        (
            "open-meteo-elevation-three.json",
            vec![spaceport, Place::new(31.5, 35.5), Place::new(0.0, -30.0)],
        ),
    ]
}

/// [`elevation::fetch`], with only the heights, in order.
fn fetch_heights<T: Transport>(
    client: &Client<T>,
    request: &ElevationRequest,
    now_s: u64,
) -> Result<(Vec<f64>, Fetched), ElevationError> {
    let (read, fetched) = elevation::fetch(client, request, now_s)?;
    Ok((read.iter().map(|e| e.height_msl_m).collect(), fetched))
}

/// A transport that fails the test if it is called at all.
struct Forbidden;

impl Transport for Forbidden {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        panic!("offline mode called the transport for {url}");
    }
}

/// The done-when of M5.3b: a recorded lookup's heights are the answer's, a second lookup online
/// is served from the cache, and offline, with a transport that may not be called, the lookup
/// still answers from the cache, a year later too.
#[test]
fn a_lookup_gives_the_answers_heights_then_works_offline_from_the_cache() {
    for (name, places) in recordings() {
        let expected = recorded_heights(name);
        assert_eq!(expected.len(), places.len(), "{name}");
        let request = ElevationRequest::new(places.clone());
        let dir = tempfile::tempdir().unwrap();

        let transport = replay();
        let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
        let (heights, fetched) = fetch_heights(&online, &request, NOW_S).unwrap();
        assert_eq!(heights, expected, "{name}");
        assert_eq!(fetched.freshness, Freshness::Fetched);
        assert_eq!(fetched.attribution, ATTRIBUTION);
        assert_eq!(fetched.body, fixture(name));
        assert_eq!(transport.calls(), 1);
        // Each height comes with the place it was asked for.
        let (read, _) = elevation::fetch(&online, &request, NOW_S).unwrap();
        let asked: Vec<Place> = read.iter().map(|e| e.place).collect();
        assert_eq!(asked, places, "{name}");

        let (again, cached) = fetch_heights(&online, &request, NOW_S + 60).unwrap();
        assert_eq!(
            (again, cached.freshness),
            (expected.clone(), Freshness::Cached)
        );
        assert_eq!(transport.calls(), 1, "{name}: the second lookup fetched");

        let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
        let (heights, fetched) = fetch_heights(&offline, &request, NOW_S + 3_600).unwrap();
        assert_eq!(heights, expected, "{name}");
        assert_eq!(fetched.freshness, Freshness::Cached);
        assert_eq!(fetched.fetched_at_s, NOW_S);
        let (heights, fetched) = fetch_heights(&offline, &request, NOW_S + TTL_S).unwrap();
        assert_eq!(heights, expected, "{name}");
        assert_eq!(fetched.freshness, Freshness::Stale);
    }
}

/// The recordings hold what the guide page quotes: Spaceport America at 1,400 m, the Dead Sea's
/// shore below sea level, the open ocean at 0 m. The weather recordings at the same place give the
/// same ground height: Open-Meteo's forecasts use this model for their `elevation`.
#[test]
fn the_recordings_hold_the_heights_the_guide_quotes() {
    assert_eq!(recorded_heights("open-meteo-elevation.json"), [1_400.0]);
    assert_eq!(
        recorded_heights("open-meteo-elevation-three.json"),
        [1_400.0, -427.0, 0.0]
    );
    for weather in ["open-meteo-historical.json", "open-meteo-forecast.json"] {
        let json: Value = serde_json::from_slice(&fixture(weather)).unwrap();
        assert_eq!(json["elevation"].as_f64(), Some(1_400.0), "{weather}");
    }
}

/// Offline, a place never looked up is an error naming its URL, and nothing is fetched.
#[test]
fn offline_a_place_never_looked_up_is_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    let request = ElevationRequest::new(vec![Place::new(32.99, -106.97)]);
    match fetch_heights(&offline, &request, NOW_S) {
        Err(ElevationError::Net(NetError::NotCached { url })) => {
            assert_eq!(url, request.url().unwrap());
        }
        other => panic!("{other:?}"),
    }
}

/// A transport that answers each call with the next of its bodies.
struct Answers {
    bodies: Vec<&'static [u8]>,
    calls: Cell<usize>,
}

impl Transport for Answers {
    fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        Ok(self.bodies[call].to_vec())
    }
}

/// An answer that doesn't parse is never cached: with no copy the lookup fails, naming why;
/// with a good copy gone stale, the copy comes back and stays cached.
#[test]
fn an_answer_that_does_not_parse_is_never_cached() {
    let dir = tempfile::tempdir().unwrap();
    let request = ElevationRequest::new(vec![Place::new(32.99, -106.97)]);
    let transport = Answers {
        bodies: vec![
            br#"{"elevation":[]}"#,
            br#"{"elevation":[1400.0]}"#,
            b"<html>",
        ],
        calls: Cell::new(0),
    };
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    match fetch_heights(&online, &request, NOW_S) {
        Err(ElevationError::Net(NetError::Refused { reason, .. })) => {
            assert_eq!(
                reason,
                "the Open-Meteo elevation answer holds 0 heights, not the 1 asked for"
            );
        }
        other => panic!("{other:?}"),
    }
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    assert!(matches!(
        fetch_heights(&offline, &request, NOW_S),
        Err(ElevationError::Net(NetError::NotCached { .. }))
    ));

    let (heights, _) = fetch_heights(&online, &request, NOW_S).unwrap();
    assert_eq!(heights, [1_400.0]);
    let later = NOW_S + TTL_S;
    let (heights, stale) = fetch_heights(&online, &request, later).unwrap();
    assert_eq!(
        (heights, stale.freshness),
        (vec![1_400.0], Freshness::Stale)
    );
    assert!(stale.stale_reason.unwrap().contains("not JSON"));
    assert_eq!(transport.calls.get(), 3);
    let (heights, _) = fetch_heights(&offline, &request, later).unwrap();
    assert_eq!(heights, [1_400.0]);
}

/// A copy already in the cache that doesn't parse (another program's, through a plain fetch) is
/// the error offline, naming why, and online is fetched again and overwritten.
#[test]
fn a_cached_copy_that_does_not_parse_is_fetched_again() {
    let dir = tempfile::tempdir().unwrap();
    let request = ElevationRequest::new(vec![Place::new(32.99, -106.97)]);
    let url = request.url().unwrap();
    Cache::new(dir.path())
        .put(&url, br#"{"elevation":[-32768]}"#, NOW_S)
        .unwrap();

    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    match fetch_heights(&offline, &request, NOW_S + 60) {
        Err(ElevationError::Net(NetError::Refused { reason, .. })) => {
            assert!(
                reason.contains("height 0 is out of range: -32768 m"),
                "{reason}"
            );
        }
        other => panic!("{other:?}"),
    }

    let transport = replay();
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let (heights, fetched) = fetch_heights(&online, &request, NOW_S + 60).unwrap();
    assert_eq!(
        (heights, fetched.freshness),
        (vec![1_400.0], Freshness::Fetched)
    );
    assert_eq!(transport.calls(), 1);
    let (heights, _) = fetch_heights(&offline, &request, NOW_S + 120).unwrap();
    assert_eq!(heights, [1_400.0]);
}

/// A place kept in radians and turned back into degrees finds its cached answer offline, though
/// its degrees differ in the last digits.
#[test]
fn a_place_rebuilt_from_radians_finds_its_cached_answer() {
    let dir = tempfile::tempdir().unwrap();
    let transport = Answers {
        bodies: vec![br#"{"elevation":[1401.0]}"#],
        calls: Cell::new(0),
    };
    let online = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let (lat, lon) = (32.99, -106.91);
    fetch_heights(
        &online,
        &ElevationRequest::new(vec![Place::new(lat, lon)]),
        NOW_S,
    )
    .unwrap();

    let rebuilt = Place::new(
        f64::to_radians(lat).to_degrees(),
        f64::to_radians(lon).to_degrees(),
    );
    assert_ne!(
        rebuilt.longitude_deg, lon,
        "the round trip changes nothing here"
    );
    let offline = Client::new(Forbidden, Cache::new(dir.path()), Mode::Offline);
    let request = ElevationRequest::new(vec![rebuilt]);
    let (heights, _) = fetch_heights(&offline, &request, NOW_S).unwrap();
    assert_eq!(heights, [1_401.0]);
}
