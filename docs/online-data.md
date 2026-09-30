# Online data and the cache

This page covers `hpr-net`, the one crate that uses the network, and the cache that makes its
answers work offline. It is for anyone who will pull weather, elevation or motor data into a
flight once those sources exist. **Today the crate holds the cache, the offline rule and an HTTP
client** ([M5.1, the online layer](decisions-and-roadmap.md#m5-1)), **but no data sources yet**
(planned from [M5.2, weather](decisions-and-roadmap.md#m5-2)). Its tests replay a small
hand-written sample response, from a folder and from a test web server on the machine running the
tests, never the live network. Those tests speak plain HTTP only: encrypted HTTPS was checked once
by hand against a real weather service (Open-Meteo), not in CI. Real recorded responses arrive
with each data source.

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

The HTTP client is [`Http`](api/hpr_net/struct.Http.html). It is behind `hpr-net`'s `http` cargo
feature, so a program that depends on `hpr-net` alone and needs only the cache and saved responses
builds no network code. The `hpr` library's `net` feature turns HTTP on: with
`hpr = { ..., features = ["net"] }` in `Cargo.toml` it is `hpr::hpr_net::Http`. Its API page has
an example, compiled in CI, that fetches into the platform's cache folder. Building with HTTP needs
a C compiler, since the encryption library compiles some C and assembly.

| behaviour | what it does | checked by |
|---|---|---|
| encryption (TLS) | rustls, a TLS library written in Rust, with Mozilla's list of trusted certificate authorities built in: no OpenSSL, and the computer's own certificate store is not read | one manual fetch; not in CI |
| time allowed | 60 s for the whole request: finding the host, connecting, redirects and reading | a test, at 0.3 s |
| largest answer | 64 MiB, counted after any unpacking; one byte more is refused | a test, at and one byte past the limit |
| compression | asks for gzip and unpacks it | a test |
| error status | anything but a 2xx success, such as 404 or 304, is a failed fetch | a test each for 404 and 304 |
| redirects | followed, up to ten; the cache files the answer under the address you asked for | a test with one redirect; the cap of ten is `ureq`'s default, not tested |
| proxy | the first set of `ALL_PROXY`, `HTTPS_PROXY` and `HTTP_PROXY` (either case), for both `http` and `https` addresses; hosts listed in `NO_PROXY` connect directly; an unreadable value is ignored. A SOCKS proxy is refused with an error, not bypassed | a test that a SOCKS proxy is refused; the rest is `ureq`'s, not tested |
| identification | sends `User-Agent: hpr-sim/<version> (+https://github.com/nrdptel/hpr-sim)`, so a data provider can see who is asking | a test |

A failed fetch (an error status, a timeout, a refused or dropped connection, a too-long answer)
is handled as above: online, the client falls back to an old copy marked stale. The time and size
limits, and whether to use the environment's proxy, are fields of
[`HttpConfig`](api/hpr_net/struct.HttpConfig.html).

Because the computer's certificate store is not read, a network that inspects encrypted traffic
with its own certificate (common on company and school networks) makes every HTTPS fetch fail, and
there is no setting yet to add a certificate.

## Where the cache lives

The cache folder is whatever you pass. [`Cache::platform_dir`](api/hpr_net/struct.Cache.html#method.platform_dir)
gives the usual place for caches on each system, and the folder is created on the first save:

| system | folder |
|---|---|
| macOS | `$HOME/Library/Caches/hpr-sim` |
| Windows | `%LOCALAPPDATA%\hpr-sim\cache` |
| Linux and other Unix | `$XDG_CACHE_HOME/hpr-sim` if that variable holds a full path, otherwise `$HOME/.cache/hpr-sim` |

Setting the environment variable `HPR_CACHE_DIR` puts the cache in that folder instead, on every
system; a relative path there is taken from the current folder. The folders come from environment
variables alone: on Windows `LOCALAPPDATA` is read rather than asking the system, which gives the
same folder unless the variable was changed. If `HOME` (macOS, Linux) or `LOCALAPPDATA` (Windows)
is unset, `Cache::platform_dir` returns nothing and you must name a folder.

## How it is checked

The tests in [`crates/hpr-net/tests/offline.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/offline.rs)
replay a hand-written sample from a folder instead of using the network:

- The offline test gives the client a transport that fails the test if it is ever called, then
  asks for a URL before and after it is cached, fresh and thirty days stale.
- Offline, a corrupt entry is an error; online, it is fetched again and overwritten.

The cache's own tests check that a saved body reads back, and that another URL's entry in the same
file reads as a miss. They also check that the hash that names each file (FNV-1a, a standard 64-bit
hash, so names stay the same on every platform) matches its published test values. The cache
folder rules are checked for each system with a made-up environment, `HPR_CACHE_DIR` included; the
`XDG_CACHE_HOME` case runs only on Unix test machines. The API reference for
[`hpr_net`](api/hpr_net/index.html) has a runnable example.

The HTTP tests in [`crates/hpr-net/tests/http.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/http.rs)
start a small web server inside the test, on the computer's own loopback address (`127.0.0.1`),
which serves the same sample. Nothing leaves the machine, and the tests turn off any proxy the
environment names. They check that:

- a fetch saves the sample in the cache, a second fetch inside the time to live sends no request,
  and offline mode answers from the cache without a request while the server is still running;
- a compressed answer arrives unpacked, and the request names hpr-sim and asks for gzip;
- a redirect is followed and saved under the address asked for;
- a 404 and a 304 are failed fetches and save nothing;
- an answer one byte over the limit is refused and one at the limit passes, and a megabyte of
  zeros that compresses to under 64 KiB is refused at a 64 KiB limit;
- a server that never answers is cut off at a 0.3 s limit (the test requires under 5 s; the
  server stalls for 10 s), and the longest possible timeout, which means none, does not crash;
- when the server hangs up without answering, an old copy comes back marked stale, with the
  reason.

## What it leaves out

- No retries: a failed fetch is tried once. Plain `http://` addresses are accepted. The 60 s
  time limit covers the whole transfer, so a large answer on a slow connection can time out; raise
  it in `HttpConfig`.
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
