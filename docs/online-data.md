# Online data and the cache

This page covers `hpr-net`, the one crate that uses the network, and the cache that makes its
answers work offline. It is for anyone who will pull weather, elevation or motor data into a
flight once those sources exist. **Today the crate holds only the cache and the offline rule**
([M5.1a](decisions-and-roadmap.md#m5-1a)). It has no HTTP client yet (planned for
[M5.1b, the HTTP transport](decisions-and-roadmap.md#m5-1b)) and no data sources yet (planned from
[M5.2, weather](decisions-and-roadmap.md#m5-2)). It is tested with a stand-in transport that
replays a small hand-written sample from a folder, never the live network. It has not been used
against a real service yet; real recorded responses arrive with each source.

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

## How it is checked

The tests in [`crates/hpr-net/tests/offline.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/offline.rs)
replay a hand-written sample from a folder instead of using the network. The offline test gives the
client a transport that fails the test if it is ever called, then asks for a URL before and after
it is cached, fresh and thirty days stale. The cache's own tests check that a saved body reads
back, and that another URL's entry in the same file reads as a miss. They also check that the
hash that names each file (FNV-1a, a standard 64-bit hash, so names stay the same on every
platform) matches its published test values. Offline, a corrupt entry is an error; online, it is
fetched again and overwritten. The API reference for [`hpr_net`](api/hpr_net/index.html) has a
runnable example.

## What it leaves out

- No HTTP yet: the only transports are [`Replay`](api/hpr_net/struct.Replay.html), which reads
  saved responses from a folder, and your own.
- The cache folder is whatever you pass; the platform's standard cache folder comes with
  [M5.1b, the HTTP transport](decisions-and-roadmap.md#m5-1b).
- Freshness comes only from the source's TTL: a server's cache headers are ignored, and the cache
  key is the URL alone.
- If saving a fetched body fails (a full disk, a read-only folder), the fetch fails too.
- Nothing prunes old entries, and nothing locks the cache. Each file is written whole under a
  temporary name and then renamed, so no file is ever half written. But two programs saving the
  same URL at once, or a crash between the two renames, can leave a body beside another fetch's
  time. Temporary files a crash leaves behind are not cleaned up.
