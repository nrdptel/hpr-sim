//! The on-disk cache: one body file and one metadata file per URL.

use std::ffi::OsString;
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

    /// The standard place for hpr's cache on this platform, or `None` if the environment names no
    /// home folder.
    ///
    /// `HPR_CACHE_DIR`, when set and not empty, wins on every platform. Otherwise:
    ///
    /// | platform | folder |
    /// |---|---|
    /// | macOS | `$HOME/Library/Caches/hpr-sim` |
    /// | Windows | `%LOCALAPPDATA%\hpr-sim\cache` |
    /// | Linux and other Unix | `$XDG_CACHE_HOME/hpr-sim`, or `$HOME/.cache/hpr-sim` |
    ///
    /// `XDG_CACHE_HOME` counts only when it is an absolute path, as the XDG Base Directory
    /// Specification says. The folder is not created until the first write.
    pub fn platform_dir() -> Option<PathBuf> {
        platform_dir_from(std::env::consts::OS, |name| std::env::var_os(name))
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

/// [`Cache::platform_dir`] for operating system `os` (as [`std::env::consts::OS`] names it), with
/// `var` reading the environment.
fn platform_dir_from(os: &str, var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let set = |name: &str| {
        var(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    if let Some(dir) = set("HPR_CACHE_DIR") {
        return Some(dir);
    }
    match os {
        "macos" => set("HOME").map(|home| home.join("Library").join("Caches").join("hpr-sim")),
        "windows" => set("LOCALAPPDATA").map(|local| local.join("hpr-sim").join("cache")),
        _ => set("XDG_CACHE_HOME")
            .filter(|dir| dir.is_absolute())
            .or_else(|| set("HOME").map(|home| home.join(".cache")))
            .map(|cache| cache.join("hpr-sim")),
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

    /// `platform_dir_from` with the environment given as pairs.
    fn dir_with(os: &str, vars: &[(&str, &str)]) -> Option<PathBuf> {
        platform_dir_from(os, |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        })
    }

    #[test]
    fn platform_dirs_follow_each_systems_convention() {
        let home = [("HOME", "/home/u")];
        assert_eq!(
            dir_with("macos", &[("HOME", "/Users/u")]),
            Some(PathBuf::from("/Users/u/Library/Caches/hpr-sim"))
        );
        assert_eq!(
            dir_with("windows", &[("LOCALAPPDATA", "C:/Users/u/AppData/Local")]),
            Some(PathBuf::from("C:/Users/u/AppData/Local/hpr-sim/cache"))
        );
        assert_eq!(
            dir_with("linux", &home),
            Some(PathBuf::from("/home/u/.cache/hpr-sim"))
        );
        // Whether a path is absolute is the host's rule, and `/var/cache/u` has no drive letter
        // for Windows; the XDG branch only ever runs on Unix hosts.
        #[cfg(unix)]
        assert_eq!(
            dir_with(
                "freebsd",
                &[("HOME", "/home/u"), ("XDG_CACHE_HOME", "/var/cache/u")]
            ),
            Some(PathBuf::from("/var/cache/u/hpr-sim"))
        );
    }

    #[test]
    fn platform_dir_skips_what_the_conventions_skip() {
        // A relative or empty XDG_CACHE_HOME is ignored, per the XDG Base Directory Specification.
        for xdg in ["relative/cache", ""] {
            assert_eq!(
                dir_with("linux", &[("HOME", "/home/u"), ("XDG_CACHE_HOME", xdg)]),
                Some(PathBuf::from("/home/u/.cache/hpr-sim"))
            );
        }
        // Windows reads LOCALAPPDATA, never HOME; nothing named, no folder.
        assert_eq!(dir_with("windows", &[("HOME", "/home/u")]), None);
        assert_eq!(dir_with("macos", &[("HOME", "")]), None);
        assert_eq!(dir_with("linux", &[]), None);
    }

    #[test]
    fn hpr_cache_dir_overrides_every_platform() {
        for os in ["macos", "windows", "linux"] {
            let vars = [
                ("HPR_CACHE_DIR", "/tmp/hpr"),
                ("HOME", "/home/u"),
                ("LOCALAPPDATA", "C:/L"),
                ("XDG_CACHE_HOME", "/x"),
            ];
            assert_eq!(dir_with(os, &vars), Some(PathBuf::from("/tmp/hpr")));
        }
        // An empty override is no override.
        assert_eq!(
            dir_with("linux", &[("HPR_CACHE_DIR", ""), ("HOME", "/home/u")]),
            Some(PathBuf::from("/home/u/.cache/hpr-sim"))
        );
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
