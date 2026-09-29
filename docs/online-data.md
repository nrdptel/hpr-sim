# Online data and the cache

This page covers `hpr-net`, the one crate that uses the network, and the cache that makes its
answers work offline. It is for anyone who will pull weather, elevation or motor data into a
flight once those sources exist. **Today the crate holds only the cache and the offline rule**
([M5.1a](decisions-and-roadmap.md#m5-1a)). It has no HTTP client yet (planned for
[M5.1b](decisions-and-roadmap.md#m5-1)) and no data sources yet (planned from
[M5.2](decisions-and-roadmap.md#m5-2)). What it does hold is tested against recorded responses,
never the live network.

## What it promises

hpr's simulation itself never touches the network ([the architecture's offline-first rule](https://github.com/nrdptel/hpr-sim/blob/main/docs/ARCHITECTURE.md)).
Anything fetched from the internet goes through a *client*, which puts a *cache* (a folder of
saved responses) in front of a *transport* (the thing that actually fetches). The client runs in
one of two modes:

- **Online:** if the cache holds a fresh copy, the client returns it without fetching. Otherwise it
  fetches, saves the response, and returns it. If the fetch fails and an old copy exists, it
  returns the old copy, marked stale.
- **Offline:** it never calls the transport. It returns whatever the cache holds, marked stale if
  it is old, or an error saying the URL is not cached.

Each answer says how fresh it is (`Fetched`, `Cached` or `Stale`), when it was fetched, and the
data source's attribution, the credit line that its terms ask you to show with the data.

## How long a copy stays fresh

Each source has a *time to live* (TTL): how many seconds a saved copy counts as fresh. A copy
exactly one TTL old is stale. Here is a worked example with a one-hour TTL (3,600 s), a copy
first fetched at time 1,000 s, and the calls the tests make:

| time asked (s) | mode | cache holds | answer | fetches so far |
|---|---|---|---|---|
| 1,000 | online | nothing | `Fetched`, from 1,000 | 1 |
| 4,599 | online | copy from 1,000 (3,599 s old) | `Cached`, from 1,000 | 1 |
| 4,600 | online | copy from 1,000 (3,600 s old) | `Fetched`, from 4,600 | 2 |
| 1,010 | offline | copy from 1,000 | `Cached`, from 1,000 | none made |
| 30 days later | offline | copy from 1,000 | `Stale`, from 1,000 | none made |
| 8,200 | online, network down | copy from 1,000 | `Stale`, from 1,000 | one failed |

## How it is checked

The tests in [`crates/hpr-net/tests/offline.rs`](https://github.com/nrdptel/hpr-sim/blob/main/crates/hpr-net/tests/offline.rs)
replay recorded responses from a folder instead of using the network. The offline test gives the
client a transport that fails the test if it is ever called, then asks for a URL before and after
it is cached, fresh and thirty days stale. The cache's own tests check that a saved body reads
back, that a corrupt entry is an error rather than a silent miss, and that its file names come
from the published FNV-1a hash test values. The rustdoc of
[`hpr_net`](api/hpr_net/index.html) has a runnable example.

## What it leaves out

- No HTTP yet: the only transports are the recorded-response one and your own.
- The cache folder is whatever you pass; the platform's standard cache folder comes with M5.1b.
- Nothing prunes old entries, and nothing locks the cache. Each file is written whole under a
  temporary name and then renamed, so no file is ever half written. But two programs saving the
  same URL at once can leave one program's body beside the other's fetch time.
