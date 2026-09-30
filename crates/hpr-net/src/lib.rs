//! Optional online data sources with an on-disk cache and an explicit offline mode: weather,
//! soundings, elevation, ThrustCurve and motor stock.
//!
//! **Guide:** [Online data and the cache][guide-page] says what works today and what is planned.
//!
//! [guide-page]: https://nrdptel.github.io/hpr-sim/online-data.html
//! [weather]: https://nrdptel.github.io/hpr-sim/weather.html
//! [soundings]: https://nrdptel.github.io/hpr-sim/soundings.html
//! [nomads]: https://nrdptel.github.io/hpr-sim/nomads.html
//! [roadmap]: https://github.com/nrdptel/hpr-sim/blob/main/docs/ROADMAP.md
//!
//! Status: pre-alpha, with three data sources: [`open_meteo`], a launch site's weather as a
//! sounding ([M5.2a][roadmap], the first weather increment; the [weather page][weather] explains
//! it); [`wyoming`], weather-balloon soundings from the University of Wyoming's archive
//! ([M5.2b][roadmap]; the [soundings page][soundings]); and [`nomads`], NOAA's GFS and RAP
//! forecasts as GRIB2 cuts from NOMADS ([M5.2c][roadmap]; the [NOAA forecasts page][nomads]). This
//! crate does network and file I/O, so it is never a dependency of the pure core. Milestone
//! [M5.1][roadmap] added the cache, the offline mode and HTTP: a [`Client`] asks a [`Transport`] for a URL's bytes only when it is [`Mode::Online`]
//! and its [`Cache`] holds no fresh copy. In [`Mode::Offline`] it never calls the transport; it
//! answers from the cache, stale or not, and says which. The `http` feature adds `Http`, the
//! transport over HTTP and HTTPS (rustls, no OpenSSL), and [`Cache::platform_dir`] names the
//! platform's usual cache folder.
//!
//! ```
//! use hpr_net::{Cache, Client, Freshness, Mode, NetError, Source, Transport};
//!
//! /// A transport that answers every URL with the same bytes.
//! struct Canned;
//! impl Transport for Canned {
//!     fn get(&self, _url: &str) -> Result<Vec<u8>, String> {
//!         Ok(b"42".to_vec())
//!     }
//! }
//!
//! let dir = std::env::temp_dir().join(format!("hpr-net-doc-{}", std::process::id()));
//! let source = Source {
//!     name: "Example".into(),
//!     attribution: "Example data".into(),
//!     ttl_s: 3600,
//! };
//! let url = "https://example.test/x";
//! let online = Client::new(Canned, Cache::new(&dir), Mode::Online);
//! assert_eq!(online.fetch(&source, url, 1_000)?.freshness, Freshness::Fetched);
//!
//! let offline = Client::new(Canned, Cache::new(&dir), Mode::Offline);
//! let later = offline.fetch(&source, url, 1_000 + 7_200)?;
//! assert_eq!((later.freshness, later.body.as_slice()), (Freshness::Stale, &b"42"[..]));
//! # std::fs::remove_dir_all(&dir).ok();
//! # Ok::<(), NetError>(())
//! ```

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the online layer does network and cache I/O; it is not part of the pure core"
)]

mod cache;
mod civil;
mod client;
#[cfg(feature = "http")]
mod http;
pub mod nomads;
pub mod open_meteo;
pub mod wyoming;

pub use cache::{Cache, CacheEntry};
pub use client::{Client, Fetched, Freshness, Mode, Replay, Source, Transport};
#[cfg(feature = "http")]
pub use http::{Http, HttpConfig};

/// Why a fetch failed.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum NetError {
    /// Offline, and the cache holds no copy of the URL.
    #[error("offline and {url} is not in the cache")]
    NotCached {
        /// The URL asked for.
        url: String,
    },
    /// The transport failed (no connection, an HTTP error status, a timeout).
    #[error("fetching {url} failed: {reason}")]
    Transport {
        /// The URL asked for.
        url: String,
        /// What the transport said.
        reason: String,
    },
    /// A body was fetched or cached, but the data source's check refused it
    /// ([`Client::fetch_checked`]); nothing was cached.
    #[error("the answer for {url} was refused: {reason}")]
    Refused {
        /// The URL asked for.
        url: String,
        /// Why the check refused it.
        reason: String,
    },
    /// Reading or writing the cache failed.
    #[error("cache I/O at {path}: {source}")]
    Cache {
        /// The file involved.
        path: std::path::PathBuf,
        /// The underlying error.
        source: std::io::Error,
    },
    /// A cache entry's metadata did not parse.
    #[error("cache entry {path} is corrupt: {reason}")]
    CorruptEntry {
        /// The metadata file.
        path: std::path::PathBuf,
        /// What was wrong.
        reason: String,
    },
}
