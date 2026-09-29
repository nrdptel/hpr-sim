//! The client against recorded fixtures: the transport is called only online, and only when the
//! cache holds no fresh copy (M5.1a).

use std::path::Path;

use hpr_net::{Cache, Client, Freshness, Mode, NetError, Replay, Source, Transport};

const URL: &str = "https://example.test/forecast?lat=32.99&lon=-106.97";
const HOUR_S: u64 = 3600;

fn source() -> Source {
    Source {
        name: "Example".to_owned(),
        attribution: "Sample data (test fixture)".to_owned(),
        ttl_s: HOUR_S,
    }
}

fn replay() -> Replay {
    Replay::open(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay")).unwrap()
}

fn recorded_body() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay/forecast.json"))
        .unwrap()
}

/// A transport that fails the test if it is called at all.
struct Forbidden;

impl Transport for Forbidden {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        panic!("offline mode called the transport for {url}");
    }
}

#[test]
fn online_fetches_once_then_serves_the_cache_until_the_ttl() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let client = Client::new(&transport, Cache::new(dir.path()), Mode::Online);

    let first = client.fetch(&source(), URL, 1_000).unwrap();
    assert_eq!(first.freshness, Freshness::Fetched);
    assert_eq!(first.body, recorded_body());
    assert_eq!(first.attribution, "Sample data (test fixture)");
    assert_eq!(transport.calls(), 1);

    let second = client.fetch(&source(), URL, 1_000 + HOUR_S - 1).unwrap();
    assert_eq!(second.freshness, Freshness::Cached);
    assert_eq!(second.fetched_at_s, 1_000);
    assert_eq!(transport.calls(), 1, "a fresh copy must not be refetched");

    let third = client.fetch(&source(), URL, 1_000 + HOUR_S).unwrap();
    assert_eq!(
        third.freshness,
        Freshness::Fetched,
        "at the TTL the copy is stale"
    );
    assert_eq!(third.fetched_at_s, 1_000 + HOUR_S);
    assert_eq!(transport.calls(), 2);
}

#[test]
fn offline_never_calls_the_transport() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path());
    let offline = Client::new(Forbidden, cache.clone(), Mode::Offline);

    // Nothing cached: an error, not a fetch.
    assert!(matches!(
        offline.fetch(&source(), URL, 1_000),
        Err(NetError::NotCached { .. })
    ));

    // Fill the cache online from the recording, then go offline.
    let transport = replay();
    Client::new(&transport, cache, Mode::Online)
        .fetch(&source(), URL, 1_000)
        .unwrap();

    let fresh = offline.fetch(&source(), URL, 1_000 + 10).unwrap();
    assert_eq!(fresh.freshness, Freshness::Cached);
    assert_eq!(fresh.body, recorded_body());

    // Long past the TTL: still served, marked stale, still no fetch.
    let stale = offline
        .fetch(&source(), URL, 1_000 + 30 * 24 * HOUR_S)
        .unwrap();
    assert_eq!(stale.freshness, Freshness::Stale);
    assert_eq!(stale.fetched_at_s, 1_000);
    assert_eq!(stale.body, recorded_body());
}

#[test]
fn a_failed_fetch_falls_back_to_a_stale_copy() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path());
    let transport = replay();
    Client::new(&transport, cache.clone(), Mode::Online)
        .fetch(&source(), URL, 1_000)
        .unwrap();

    /// A transport whose every fetch fails, like a machine with no connection.
    struct Down;
    impl Transport for Down {
        fn get(&self, _: &str) -> Result<Vec<u8>, String> {
            Err("no route to host".to_owned())
        }
    }
    let online = Client::new(Down, cache, Mode::Online);
    let got = online.fetch(&source(), URL, 1_000 + 2 * HOUR_S).unwrap();
    assert_eq!(got.freshness, Freshness::Stale);
    assert_eq!(got.body, recorded_body());

    // With nothing cached, the transport's error comes through.
    let err = online
        .fetch(&source(), "https://example.test/other", 1_000)
        .unwrap_err();
    assert!(matches!(err, NetError::Transport { ref reason, .. } if reason == "no route to host"));
}

#[test]
fn an_unrecorded_url_fails_like_the_network() {
    let dir = tempfile::tempdir().unwrap();
    let transport = replay();
    let client = Client::new(&transport, Cache::new(dir.path()), Mode::Online);
    let err = client
        .fetch(&source(), "https://example.test/unrecorded", 0)
        .unwrap_err();
    assert!(
        matches!(err, NetError::Transport { ref reason, .. } if reason.starts_with("no recording"))
    );
}
