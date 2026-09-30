//! The client: the cache in front of a transport, and the rule for when the transport is called.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{Cache, NetError};

/// Whether the client may use the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    /// Fetch when the cache holds no fresh copy.
    Online,
    /// Never call the transport; answer from the cache or fail with [`NetError::NotCached`].
    Offline,
}

/// Something that fetches a URL's bytes: `Http` with the `http` feature, [`Replay`]'s recorded
/// responses in tests.
pub trait Transport {
    /// The body at `url`.
    ///
    /// # Errors
    /// A short reason, which the client wraps in [`NetError::Transport`].
    fn get(&self, url: &str) -> Result<Vec<u8>, String>;
}

/// One online data source: its name, the credit its terms ask for, and how long a copy stays fresh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// A short name, such as `"Open-Meteo"`.
    pub name: String,
    /// The attribution to show wherever its data is shown, such as `"Weather data by Open-Meteo.com
    /// (CC BY 4.0)"`.
    pub attribution: String,
    /// How long a cached copy counts as fresh, in seconds.
    pub ttl_s: u64,
}

/// How a [`Fetched`] body relates to its source.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Freshness {
    /// Fetched by this call.
    Fetched,
    /// From the cache, younger than the source's TTL.
    Cached,
    /// From the cache, older than the TTL: offline, or the fetch failed.
    Stale,
}

/// A body and where it came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fetched {
    /// The response body.
    pub body: Vec<u8>,
    /// When it was fetched, in seconds since the Unix epoch.
    pub fetched_at_s: u64,
    /// Fetched now, cached and fresh, or cached and stale.
    pub freshness: Freshness,
    /// The source's attribution, to show with the data.
    pub attribution: String,
    /// Online, why the fetch failed when a stale copy was returned in its place.
    pub stale_reason: Option<String>,
}

/// A cache in front of a transport.
#[derive(Debug)]
pub struct Client<T> {
    transport: T,
    cache: Cache,
    mode: Mode,
}

impl<T: Transport> Client<T> {
    /// A client over `transport` and `cache`, in `mode`.
    pub fn new(transport: T, cache: Cache, mode: Mode) -> Self {
        Self {
            transport,
            cache,
            mode,
        }
    }

    /// The mode the client is in.
    pub fn mode(&self) -> Mode {
        self.mode
    }

    /// The body at `url` from `source`, at time `now_s` (seconds since the Unix epoch).
    ///
    /// A cached copy younger than `source.ttl_s` is returned as [`Freshness::Cached`] without a
    /// fetch. Otherwise, online, the transport is called and its body cached; if it fails, a stale
    /// copy is returned as [`Freshness::Stale`] when there is one. Offline, the transport is never
    /// called: any cached copy is returned, [`Freshness::Stale`] if it is past its TTL.
    ///
    /// A copy dated after `now_s` is never fresh. Online, a cache entry that cannot be read is
    /// treated as missing and overwritten by the fetch.
    ///
    /// # Errors
    /// [`NetError::NotCached`] offline with no copy; [`NetError::Transport`] online when the fetch
    /// fails with no copy; cache errors as [`Cache::get`] and [`Cache::put`] give them.
    pub fn fetch(&self, source: &Source, url: &str, now_s: u64) -> Result<Fetched, NetError> {
        // Online, an unreadable entry is a miss: the fetch below overwrites it. Offline it is the
        // error, since there is nothing else to answer with.
        let cached = match (self.cache.get(url), self.mode) {
            (Ok(cached), _) => cached,
            (Err(_), Mode::Online) => None,
            (Err(e), Mode::Offline) => return Err(e),
        };
        let answer =
            |entry: crate::CacheEntry, fresh: bool, stale_reason: Option<String>| Fetched {
                body: entry.body,
                fetched_at_s: entry.fetched_at_s,
                freshness: if fresh {
                    Freshness::Cached
                } else {
                    Freshness::Stale
                },
                attribution: source.attribution.clone(),
                stale_reason,
            };
        // A copy dated after `now_s` (saved while the clock ran fast) is not fresh.
        let is_fresh = |entry: &crate::CacheEntry| {
            entry.fetched_at_s <= now_s && now_s - entry.fetched_at_s < source.ttl_s
        };
        if let Some(entry) = cached {
            if is_fresh(&entry) {
                return Ok(answer(entry, true, None));
            }
            if self.mode == Mode::Offline {
                return Ok(answer(entry, false, None));
            }
            return match self.transport.get(url) {
                Ok(body) => self.store(source, url, body, now_s),
                Err(reason) => Ok(answer(entry, false, Some(reason))),
            };
        }
        if self.mode == Mode::Offline {
            return Err(NetError::NotCached {
                url: url.to_owned(),
            });
        }
        let body = self
            .transport
            .get(url)
            .map_err(|reason| NetError::Transport {
                url: url.to_owned(),
                reason,
            })?;
        self.store(source, url, body, now_s)
    }

    fn store(
        &self,
        source: &Source,
        url: &str,
        body: Vec<u8>,
        now_s: u64,
    ) -> Result<Fetched, NetError> {
        self.cache.put(url, &body, now_s)?;
        Ok(Fetched {
            body,
            fetched_at_s: now_s,
            freshness: Freshness::Fetched,
            attribution: source.attribution.clone(),
            stale_reason: None,
        })
    }
}

/// A transport that replays recorded responses from a directory, for tests and offline demos.
///
/// The directory holds `index.json`, an object from URL to a file name in the same directory. A
/// URL not in the index fails like a network error. It counts its calls, so a test can assert
/// that none were made.
#[derive(Debug)]
pub struct Replay {
    dir: PathBuf,
    index: BTreeMap<String, String>,
    calls: Cell<usize>,
}

impl Replay {
    /// Reads `dir/index.json`.
    ///
    /// # Errors
    /// [`NetError::Cache`] if the index cannot be read; [`NetError::CorruptEntry`] if it does not
    /// parse.
    pub fn open(dir: impl Into<PathBuf>) -> Result<Self, NetError> {
        let dir = dir.into();
        let path = dir.join("index.json");
        let bytes = std::fs::read(&path).map_err(|source| NetError::Cache {
            path: path.clone(),
            source,
        })?;
        let index = serde_json::from_slice(&bytes).map_err(|e| NetError::CorruptEntry {
            path,
            reason: e.to_string(),
        })?;
        Ok(Self {
            dir,
            index,
            calls: Cell::new(0),
        })
    }

    /// How many times [`Transport::get`] has been called.
    pub fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl Transport for Replay {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        self.calls.set(self.calls.get() + 1);
        let file = self
            .index
            .get(url)
            .ok_or_else(|| format!("no recording of {url}"))?;
        std::fs::read(self.dir.join(file)).map_err(|e| e.to_string())
    }
}

impl<T: Transport + ?Sized> Transport for &T {
    fn get(&self, url: &str) -> Result<Vec<u8>, String> {
        (**self).get(url)
    }
}
