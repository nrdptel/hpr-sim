# Online data and the cache

This page covers `hpr-net`, the one crate that uses the network, and the cache that makes its
answers work offline. It is for anyone who will pull weather, elevation or motor data into a
flight once those sources exist. **Today the crate holds the cache, the offline rule and an HTTP
client** ([M5.1, the online layer](decisions-and-roadmap.md#m5-1)), **but no data sources yet**
(planned from [M5.2, weather](decisions-and-roadmap.md#m5-2)). It is tested against a small
hand-written sample response, replayed from a folder and served by a test server on your own
machine, never the live network. It has not been used against a real service yet; real recorded
responses arrive with each source.

## What it promises

hpr's simulation itself never touches the network ([the architecture's pure-core rule: no filesystem, network or clock](https://github.com/nrdptel/hpr-sim/blob/main/docs/ARCHITECTURE.md#principles)).
Anything fetched from the internet goes through a *client*, which puts a *cache* (a folder of
saved responses) in front of a *transport* (the thing that actually fetches). The client runs in
one of two modes:

- **Online:** if the cache holds a fresh copy, the client returns it without fetching. Otherwise it
  fetches, saves the response, and returns it. If the fetch fails and an old copy exists, it
  returns the old copy, marked stale, with the reason the fetch failed; with no old copy, the
  fetch's error.
- **Offline:** it never calls the transport. It returns whatever the cache holds, marked stale if
  it is old, or an error saying the URL is not cached.

Each answer says how fresh it is (`Fetched`, `Cached` or `Stale`), when it was fetched, and the
data source's attribution, the credit line that its terms ask you to show with the data.

## How long a copy stays fresh

Each source has a *time to live* (TTL): how many seconds a saved copy counts as fresh. A copy
exactly one TTL old is no longer fresh: online it is fetched again, offline it comes back marked
`Stale`. A copy dated later than the time asked (saved while the clock ran fast) is never fresh.
The caller passes the time, in seconds since 1 January 1970 (Unix time); the crate reads no clock,
so the tests can use small numbers.

Here is a worked example with a one-hour TTL (3,600 s), from the tests. Each scenario starts with
an empty cache and fills it at 1,000 s:

| scenario | time asked (s) | cache holds | answer | fetches made |
|---|---|---|---|---|
| A, online | 1,000 | nothing | `Fetched`, from 1,000 | 1 |
| A, online | 4,599 | copy from 1,000 (3,599 s old) | `Cached`, from 1,000 | 1 |
| A, online | 4,600 | copy from 1,000 (3,600 s old) | `Fetched`, from 4,600 | 2 |
| B, offline | 1,000, before filling | nothing | error: not cached | none |
| B, offline | 1,010 | copy from 1,000 | `Cached`, from 1,000 | none |
| B, offline | 2,593,000 (30 days after 1,000) | copy from 1,000 | `Stale`, from 1,000 | none |
| C, online, network down | 8,200 | copy from 1,000 | `Stale`, from 1,000, with the reason | one failed |
| C, online, network down | 1,000 | nothing, for another URL | error: the fetch's | one failed |

## Fetching over HTTP

The HTTP client is `Http` (in the [API reference](api/hpr_net/struct.Http.html)). It is behind the
crate's `http` cargo feature, which the `hpr` library's `net` feature turns on, so a program that
needs only the cache and saved responses builds no network code. It fetches plain HTTP and HTTPS.

| behaviour | what it does |
|---|---|
| encryption (TLS) | rustls, a TLS library written in Rust, with Mozilla's list of trusted certificate authorities built in: no OpenSSL, and the computer's own certificate store is not read |
| time allowed | 60 s for the whole request, connecting and reading included |
| largest answer | 64 MiB after unpacking a compressed answer; one byte more is refused |
| compression | asks for gzip and unpacks it |
| redirects | followed, up to ten; the cache files the answer under the address you asked for |
| proxy | the one named by `ALL_PROXY`, `HTTPS_PROXY` or `HTTP_PROXY`, except for hosts in `NO_PROXY` |
| failures | an error status (such as 404), a timeout, a refused connection or a too-long answer is a failed fetch; online, the client then falls back to an old copy as above |
| identification | sends `User-Agent: hpr-sim/<version> (+https://github.com/nrdptel/hpr-sim)`, so a data provider can see who is asking |

The time and size limits can be changed with `Http::with_limits`.

## Where the cache lives

The cache folder is whatever you pass. `Cache::platform_dir` gives the usual place for caches on
each system, and the folder is created on the first save:

| system | folder |
|---|---|
| macOS | `~/Library/Caches/hpr-sim` |
| Windows | `%LOCALAPPDATA%\hpr-sim\cache` |
| Linux and other Unix | `$XDG_CACHE_HOME/hpr-sim` if that variable holds a full path, otherwise `~/.cache/hpr-sim` |

Setting the environment variable `HPR_CACHE_DIR` puts the cache in that folder instead, on every
system. If none of these variables is set, there is no standard folder and you must name one.

## How it is checked

The tests in [`crates/hpr-net/tests/offline.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/offline.rs)
replay a hand-written sample from a folder instead of using the network. The offline test gives the
client a transport that fails the test if it is ever called, then asks for a URL before and after
it is cached, fresh and thirty days stale. The cache's own tests check that a saved body reads
back, and that another URL's entry in the same file reads as a miss. They also check that the
hash that names each file (FNV-1a, a standard 64-bit hash, so names stay the same on every
platform) matches its published test values. Offline, a corrupt entry is an error; online, it is
fetched again and overwritten. They check each system's cache folder rule, and `HPR_CACHE_DIR`.
The API reference for [`hpr_net`](api/hpr_net/index.html) has a runnable example.

The HTTP tests in [`crates/hpr-net/tests/http.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/http.rs)
start a small web server inside the test, on the computer's own loopback address (`127.0.0.1`),
which serves the same sample. Nothing leaves the machine. They check that:

- a fetch saves the sample in the cache, a second fetch inside the time to live sends no request,
  and offline mode answers from the cache without a request while the server is still running;
- a compressed answer arrives unpacked, and the request names hpr-sim and asks for gzip;
- a redirect is followed and saved under the address asked for;
- a 404 is a failed fetch and saves nothing;
- an answer one byte over the limit is refused and one at the limit passes, and a megabyte of
  zeros that compresses to under 64 KiB is refused at a 64 KiB limit;
- a server that never answers times out in well under five seconds at a 0.3 s limit;
- with the server gone, an old copy comes back marked stale, with the reason.

## What it leaves out

- No test makes an encrypted (HTTPS) connection: the test server speaks plain HTTP, since a local
  encrypted server would need certificates made for the test. One manual run on a Mac fetched a
  real Open-Meteo forecast over HTTPS and refused a site with an expired certificate
  ([ADR-118: M5.1b, HTTP over the cache](https://github.com/nrdptel/hpr-sim/blob/main/docs/DECISIONS.md#adr-118-m51b-http-over-the-cache-2026-09-30)), but that is not repeated in
  CI.
- Freshness comes only from the source's TTL: a server's cache headers are ignored, and the cache
  key is the URL alone.
- If saving a fetched body fails (a full disk, a read-only folder), the fetch fails too.
- Nothing prunes old entries, and nothing locks the cache. Each file is written whole under a
  temporary name and then renamed, so no file is ever half written. But two programs saving the
  same URL at once, or a crash between the two renames, can leave a body beside another fetch's
  time. Temporary files a crash leaves behind are not cleaned up.
