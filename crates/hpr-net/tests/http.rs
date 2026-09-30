//! The HTTP transport against a server on the loopback interface that replays the recorded
//! fixture: a fetch fills the cache, offline mode answers from it without a request, and HTTP's
//! failures reach the client as transport errors (M5.1b). No test leaves the machine.

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the tests serve the committed fixture over a socket and write a cache; not the pure core"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "the helpers stop at the failure, as `#[test]` functions may (clippy.toml)"
)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hpr_net::{Cache, Client, Freshness, Http, Mode, NetError, Source};

/// The path and query the fixture was recorded at, on whatever host serves it.
const RECORDED: &str = "/forecast?lat=32.99&lon=-106.97";
const HOUR_S: u64 = 3600;
const DAY_S: u64 = 24 * HOUR_S;

fn source() -> Source {
    Source {
        name: "Example".to_owned(),
        attribution: "Sample data (test fixture)".to_owned(),
        ttl_s: HOUR_S,
    }
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay")
}

fn recorded_body() -> Vec<u8> {
    std::fs::read(fixtures().join("forecast.json")).unwrap()
}

/// A one-thread HTTP/1.1 server on 127.0.0.1 that answers from the recorded fixture's index.
///
/// Each recorded URL is served at its path and query. `/moved` redirects to the recording,
/// `/gzip` serves it gzip-compressed, `/zeros` serves a megabyte of zeros gzipped, `/stall` never
/// answers, and anything else is a 404. Every request's head is kept, so a test can count requests
/// and read their headers.
struct Server {
    addr: SocketAddr,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Server {
    fn start() -> Self {
        let index: BTreeMap<String, String> =
            serde_json::from_slice(&std::fs::read(fixtures().join("index.json")).unwrap()).unwrap();
        // "https://example.test/forecast?..." is served at "/forecast?...".
        let routes: BTreeMap<String, Vec<u8>> = index
            .into_iter()
            .map(|(url, file)| {
                let after_scheme = &url[url.find("://").unwrap() + 3..];
                let path = &after_scheme[after_scheme.find('/').unwrap()..];
                (
                    path.to_owned(),
                    std::fs::read(fixtures().join(file)).unwrap(),
                )
            })
            .collect();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = Arc::clone(&requests);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { continue };
                answer(stream, &routes, &seen);
            }
        });
        Self { addr, requests }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    fn request_count(&self) -> usize {
        self.requests.lock().unwrap().len()
    }
}

fn answer(mut stream: TcpStream, routes: &BTreeMap<String, Vec<u8>>, seen: &Mutex<Vec<String>>) {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let path = head.split_whitespace().nth(1).unwrap_or("").to_owned();
    seen.lock().unwrap().push(head);
    let (status, headers, body) = match path.as_str() {
        "/moved" => ("302 Found", format!("Location: {RECORDED}\r\n"), Vec::new()),
        "/gzip" => {
            let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            gz.write_all(&routes[RECORDED]).unwrap();
            (
                "200 OK",
                "Content-Encoding: gzip\r\n".to_owned(),
                gz.finish().unwrap(),
            )
        }
        "/zeros" => (
            "200 OK",
            "Content-Encoding: gzip\r\n".to_owned(),
            zeros_gzipped(),
        ),
        "/stall" => {
            // Hold the connection open without answering, past any timeout a test sets.
            std::thread::sleep(Duration::from_secs(10));
            return;
        }
        _ => match routes.get(&path) {
            Some(body) => ("200 OK", String::new(), body.clone()),
            None => ("404 Not Found", String::new(), b"not recorded".to_vec()),
        },
    };
    let response = format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    // The client may hang up first (a refused body); that is the client's answer, not an error.
    let _ = stream
        .write_all(response.as_bytes())
        .and_then(|()| stream.write_all(&body));
}

/// A megabyte of zeros, gzipped.
fn zeros_gzipped() -> Vec<u8> {
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    gz.write_all(&[0; 1 << 20]).unwrap();
    gz.finish().unwrap()
}

fn online(dir: &Path) -> Client<Http> {
    Client::new(Http::new(), Cache::new(dir), Mode::Online)
}

#[test]
fn a_loopback_fetch_fills_the_cache_and_offline_reads_it_back() {
    let server = Server::start();
    let dir = tempfile::tempdir().unwrap();
    let url = server.url(RECORDED);

    let first = online(dir.path()).fetch(&source(), &url, 1_000).unwrap();
    assert_eq!(first.freshness, Freshness::Fetched);
    assert_eq!(first.body, recorded_body());
    assert_eq!(first.attribution, "Sample data (test fixture)");
    assert_eq!(server.request_count(), 1);

    // The cache on disk now holds the recorded body, dated at the fetch.
    let entry = Cache::new(dir.path()).get(&url).unwrap().unwrap();
    assert_eq!((entry.body, entry.fetched_at_s), (recorded_body(), 1_000));

    // Inside the TTL the cache answers and the server hears nothing.
    let again = online(dir.path())
        .fetch(&source(), &url, 1_000 + HOUR_S - 1)
        .unwrap();
    assert_eq!(again.freshness, Freshness::Cached);
    assert_eq!(server.request_count(), 1);

    // Offline, with a real HTTP transport and the server still up: thirty days on the copy comes
    // back stale, an uncached URL is refused, and neither reaches the server.
    let offline = Client::new(Http::new(), Cache::new(dir.path()), Mode::Offline);
    let stale = offline.fetch(&source(), &url, 1_000 + 30 * DAY_S).unwrap();
    assert_eq!(
        (stale.freshness, stale.body, stale.fetched_at_s),
        (Freshness::Stale, recorded_body(), 1_000)
    );
    let uncached = server.url("/forecast?lat=0&lon=0");
    assert!(matches!(
        offline.fetch(&source(), &uncached, 1_000),
        Err(NetError::NotCached { url }) if url == uncached
    ));
    assert_eq!(server.request_count(), 1);
}

#[test]
fn requests_name_hpr_sim_and_accept_gzip() {
    let server = Server::start();
    let dir = tempfile::tempdir().unwrap();
    let fetched = online(dir.path())
        .fetch(&source(), &server.url("/gzip"), 1_000)
        .unwrap();
    // The body arrives unpacked, and is cached unpacked.
    assert_eq!(fetched.body, recorded_body());
    let head = server.requests.lock().unwrap()[0].to_ascii_lowercase();
    let user_agent = format!(
        "user-agent: {}\r\n",
        Http::user_agent().to_ascii_lowercase()
    );
    assert!(head.contains(&user_agent), "{head}");
    assert!(head.contains("accept-encoding: gzip"), "{head}");
}

#[test]
fn a_redirect_is_followed_and_cached_under_the_url_asked_for() {
    let server = Server::start();
    let dir = tempfile::tempdir().unwrap();
    let asked = server.url("/moved");
    let fetched = online(dir.path()).fetch(&source(), &asked, 1_000).unwrap();
    assert_eq!(fetched.body, recorded_body());
    assert_eq!(server.request_count(), 2);
    let entry = Cache::new(dir.path()).get(&asked).unwrap().unwrap();
    assert_eq!(entry.body, recorded_body());
}

#[test]
fn an_error_status_is_a_transport_error_and_caches_nothing() {
    let server = Server::start();
    let dir = tempfile::tempdir().unwrap();
    let url = server.url("/not-recorded");
    match online(dir.path()).fetch(&source(), &url, 1_000) {
        Err(NetError::Transport {
            url: failed,
            reason,
        }) => {
            assert_eq!(failed, url);
            assert!(reason.contains("404"), "{reason}");
        }
        other => panic!("expected a transport error, got {other:?}"),
    }
    assert_eq!(Cache::new(dir.path()).get(&url).unwrap(), None);
}

#[test]
fn a_body_past_the_limit_is_refused_and_caches_nothing() {
    let server = Server::start();
    let dir = tempfile::tempdir().unwrap();
    let url = server.url(RECORDED);
    let limit = recorded_body().len() as u64 - 1;
    let client = Client::new(
        Http::with_limits(Http::DEFAULT_TIMEOUT, limit),
        Cache::new(dir.path()),
        Mode::Online,
    );
    assert!(matches!(
        client.fetch(&source(), &url, 1_000),
        Err(NetError::Transport { .. })
    ));
    assert_eq!(Cache::new(dir.path()).get(&url).unwrap(), None);

    // One byte more and the same body fits.
    let client = Client::new(
        Http::with_limits(Http::DEFAULT_TIMEOUT, limit + 1),
        Cache::new(dir.path()),
        Mode::Online,
    );
    assert_eq!(
        client.fetch(&source(), &url, 1_000).unwrap().body,
        recorded_body()
    );
}

#[test]
fn the_limit_is_on_the_unpacked_body() {
    let server = Server::start();
    let dir = tempfile::tempdir().unwrap();
    // The megabyte of zeros is under 64 KiB on the wire, but not once unpacked.
    assert!(zeros_gzipped().len() < 64 * 1024);
    let client = Client::new(
        Http::with_limits(Http::DEFAULT_TIMEOUT, 64 * 1024),
        Cache::new(dir.path()),
        Mode::Online,
    );
    match client.fetch(&source(), &server.url("/zeros"), 1_000) {
        Err(NetError::Transport { reason, .. }) => assert!(reason.contains("65536"), "{reason}"),
        other => panic!("expected the limit to refuse the body, got {other:?}"),
    }
    let whole = online(dir.path())
        .fetch(&source(), &server.url("/zeros"), 1_000)
        .unwrap();
    assert_eq!(whole.body, vec![0; 1 << 20]);
}

#[test]
fn a_server_that_never_answers_times_out() {
    let server = Server::start();
    let dir = tempfile::tempdir().unwrap();
    let client = Client::new(
        Http::with_limits(Duration::from_millis(300), Http::DEFAULT_MAX_BODY_BYTES),
        Cache::new(dir.path()),
        Mode::Online,
    );
    let started = std::time::Instant::now();
    assert!(matches!(
        client.fetch(&source(), &server.url("/stall"), 1_000),
        Err(NetError::Transport { .. })
    ));
    // Well short of the server's ten-second stall: the timeout, not the server, ended it.
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn with_the_server_gone_a_stale_copy_comes_back_with_the_reason() {
    // A port that was just free: bind, note it, close it.
    let addr = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let url = format!("http://{addr}{RECORDED}");
    Cache::new(dir.path())
        .put(&url, &recorded_body(), 1_000)
        .unwrap();

    let stale = online(dir.path())
        .fetch(&source(), &url, 1_000 + DAY_S)
        .unwrap();
    assert_eq!(
        (stale.freshness, stale.body),
        (Freshness::Stale, recorded_body())
    );
    assert!(stale.stale_reason.is_some());

    let uncached = format!("http://{addr}/forecast?lat=0&lon=0");
    assert!(matches!(
        online(dir.path()).fetch(&source(), &uncached, 1_000),
        Err(NetError::Transport { .. })
    ));
}
