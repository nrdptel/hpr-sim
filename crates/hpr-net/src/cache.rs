//! The on-disk cache: one body file and one metadata file per URL.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::NetError;

/// A directory of cached responses, keyed by URL.
///
/// Each URL is stored as `<key>.bin` (the bytes) and `<key>.json` (a [`CacheEntry`] without the
/// bytes), where `<key>` is the 64-bit FNV-1a hash of the URL in hex. The metadata records the URL
/// itself, so two URLs whose hashes collide read as a miss rather than as each other's data. Each
/// file is written whole to a temporary name and renamed, the metadata last. Nothing locks the
/// cache: two writers of one URL may leave one's body beside the other's fetch time.
#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

/// What the cache knows about one URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The URL the bytes came from.
    pub url: String,
    /// When they were fetched, in seconds since the Unix epoch.
    pub fetched_at_s: u64,
    /// The response body.
    #[serde(skip)]
    pub body: Vec<u8>,
}

impl Cache {
    /// A cache in `dir`, created on the first write.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The directory the cache lives in.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn paths(&self, url: &str) -> (PathBuf, PathBuf) {
        let key = format!("{:016x}", fnv1a64(url.as_bytes()));
        (
            self.dir.join(format!("{key}.json")),
            self.dir.join(format!("{key}.bin")),
        )
    }

    /// The cached entry for `url`, or `None` if there is none.
    ///
    /// # Errors
    /// [`NetError::Cache`] if a file exists but cannot be read; [`NetError::CorruptEntry`] if the
    /// metadata does not parse.
    pub fn get(&self, url: &str) -> Result<Option<CacheEntry>, NetError> {
        let (meta_path, body_path) = self.paths(url);
        let Some(meta) = read_optional(&meta_path)? else {
            return Ok(None);
        };
        let mut entry: CacheEntry =
            serde_json::from_slice(&meta).map_err(|e| NetError::CorruptEntry {
                path: meta_path.clone(),
                reason: e.to_string(),
            })?;
        if entry.url != url {
            // A hash collision: this slot holds another URL.
            return Ok(None);
        }
        let Some(body) = read_optional(&body_path)? else {
            return Ok(None);
        };
        entry.body = body;
        Ok(Some(entry))
    }

    /// Stores `body` as `url`'s response, fetched at `fetched_at_s`.
    ///
    /// # Errors
    /// [`NetError::Cache`] if the directory or a file cannot be written.
    pub fn put(&self, url: &str, body: &[u8], fetched_at_s: u64) -> Result<(), NetError> {
        fs::create_dir_all(&self.dir).map_err(|source| NetError::Cache {
            path: self.dir.clone(),
            source,
        })?;
        let (meta_path, body_path) = self.paths(url);
        let meta = CacheEntry {
            url: url.to_owned(),
            fetched_at_s,
            body: Vec::new(),
        };
        let meta = serde_json::to_vec(&meta).map_err(|e| NetError::CorruptEntry {
            path: meta_path.clone(),
            reason: e.to_string(),
        })?;
        write_atomic(&body_path, body)?;
        write_atomic(&meta_path, &meta)
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, NetError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(NetError::Cache {
            path: path.to_owned(),
            source,
        }),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), NetError> {
    // A name no other process or call is writing, so concurrent writers never share a file.
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let tmp = path.with_extension(format!("{}.{n}.tmp", std::process::id()));
    let err = |source| NetError::Cache {
        path: path.to_owned(),
        source,
    };
    fs::write(&tmp, bytes).map_err(err)?;
    fs::rename(&tmp, path).map_err(err)
}

/// The 64-bit FNV-1a hash (Fowler, Noll and Vo), stable across platforms and releases.
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a64_matches_published_vectors() {
        // Test vectors from the FNV reference page (Noll, "FNV Hash").
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn a_stored_body_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path().join("sub"));
        assert_eq!(cache.get("https://example.test/a").unwrap(), None);
        cache.put("https://example.test/a", b"abc", 42).unwrap();
        let entry = cache.get("https://example.test/a").unwrap().unwrap();
        assert_eq!(entry.body, b"abc");
        assert_eq!(entry.fetched_at_s, 42);
        assert_eq!(cache.get("https://example.test/b").unwrap(), None);
    }

    #[test]
    fn another_urls_entry_in_the_slot_reads_as_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path());
        cache.put("https://example.test/a", b"abc", 1).unwrap();
        // Forge a collision: move a's files into b's slot.
        let (meta_a, body_a) = cache.paths("https://example.test/a");
        let (meta_b, body_b) = cache.paths("https://example.test/b");
        fs::rename(meta_a, meta_b).unwrap();
        fs::rename(body_a, body_b).unwrap();
        assert_eq!(cache.get("https://example.test/b").unwrap(), None);
    }

    #[test]
    fn corrupt_metadata_is_an_error_not_a_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path());
        cache.put("u", b"x", 1).unwrap();
        fs::write(cache.paths("u").0, b"{not json").unwrap();
        assert!(matches!(cache.get("u"), Err(NetError::CorruptEntry { .. })));
    }
}
