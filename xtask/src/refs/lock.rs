//! `validation/refs.lock.toml`: what the reference library holds and how each item is pinned.

use std::collections::BTreeSet;
use std::path::{Component, Path};

use serde::Deserialize;

/// The lock file's path, relative to the workspace root.
pub const LOCK_PATH: &str = "validation/refs.lock.toml";

/// The lock format this code reads.
const FORMAT_VERSION: u32 = 1;

/// Every item destination lives under this directory, which `.gitignore` excludes.
pub const REFS_DIR: &str = "refs";

/// The parsed lock file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    /// The lock format version.
    pub version: u32,
    /// Git repositories, each pinned to a commit.
    #[serde(default)]
    pub git: Vec<GitSource>,
    /// Immutable downloads, each pinned by sha256.
    #[serde(default)]
    pub file: Vec<FileSource>,
    /// Captures of live APIs, each pinned by the sha256 of one dated capture.
    #[serde(default)]
    pub snapshot: Vec<SnapshotSource>,
    /// The Python environment for the RocketPy and OpenRocket oracles.
    pub python: Option<PythonEnv>,
    /// The Java runtime the OpenRocket oracle needs.
    pub java: Option<JavaReq>,
}

/// Where a source came from and on what terms, for `THIRD-PARTY-NOTICES.md`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Terms {
    /// The license, as an SPDX expression where one exists.
    pub license: String,
    /// How the project uses the source: `fetched` or `run-only` (see `THIRD-PARTY-NOTICES.md`).
    pub mode: String,
}

/// A git repository checked out at a pinned commit.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitSource {
    pub name: String,
    pub url: String,
    /// The full 40-hex commit id. This is the pin; `tag` is informational.
    pub commit: String,
    pub tag: Option<String>,
    /// The checkout directory, relative to the workspace root and under `refs/`.
    pub dest: String,
    /// Fetch only the pinned commit (`--depth 1`), not the history.
    #[serde(default)]
    pub shallow: bool,
    /// Needs the user's GitHub credentials. When it can't be fetched, it is skipped with a note
    /// instead of failing, as in CI.
    #[serde(default)]
    pub private: bool,
    /// A `sha256sum`-style manifest in the checkout (`<hex>  <path>` lines, paths relative to
    /// the checkout root), and its own sha256. `verify` hashes every file it lists.
    pub manifest: Option<String>,
    pub manifest_sha256: Option<String>,
    #[serde(flatten)]
    pub terms: Terms,
}

/// A download whose bytes never change, pinned by sha256.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSource {
    pub name: String,
    pub url: String,
    pub sha256: String,
    pub dest: String,
    /// A human-readable description (title, author, year). A test checks it against the row in
    /// `THIRD-PARTY-NOTICES.md`.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "documents the source; only the notices test reads it"
        )
    )]
    pub title: Option<String>,
    #[serde(flatten)]
    pub terms: Terms,
}

/// A capture of a live API response. The API changes over time, so the pin is the sha256 of one
/// capture, made on the date in `captured`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotSource {
    pub name: String,
    pub url: String,
    pub sha256: String,
    /// The capture date, `YYYY-MM-DD`.
    pub captured: String,
    pub dest: String,
    #[serde(flatten)]
    pub terms: Terms,
}

/// A `uv` project whose locked environment lives under `refs/`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonEnv {
    /// The directory holding `pyproject.toml` and `uv.lock`, relative to the workspace root.
    pub project: String,
    /// The environment directory, under `refs/`.
    pub venv: String,
    /// Python modules that must import, with the version each must report (empty: any).
    #[serde(default)]
    pub checks: Vec<ModuleCheck>,
}

/// A module the oracle environment must provide.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleCheck {
    pub module: String,
    /// The version the module must report: its `__version__`, or else the version of the
    /// installed distribution with the module's name.
    pub version: Option<String>,
}

/// The Java runtime requirement.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JavaReq {
    pub min_major: u32,
}

/// One named, pinned item, whatever its kind.
#[derive(Debug, Clone, Copy)]
pub enum Item<'a> {
    Git(&'a GitSource),
    File(&'a FileSource),
    Snapshot(&'a SnapshotSource),
}

impl<'a> Item<'a> {
    pub fn name(&self) -> &'a str {
        match self {
            Item::Git(source) => &source.name,
            Item::File(source) => &source.name,
            Item::Snapshot(source) => &source.name,
        }
    }

    pub fn dest(&self) -> &'a str {
        match self {
            Item::Git(source) => &source.dest,
            Item::File(source) => &source.dest,
            Item::Snapshot(source) => &source.dest,
        }
    }

    pub fn terms(&self) -> &'a Terms {
        match self {
            Item::Git(source) => &source.terms,
            Item::File(source) => &source.terms,
            Item::Snapshot(source) => &source.terms,
        }
    }
}

impl Lock {
    /// Reads and validates the lock file under `root`.
    pub fn load(root: &Path) -> Result<Lock, String> {
        let path = root.join(LOCK_PATH);
        let text = std::fs::read_to_string(&path)
            .map_err(|err| format!("could not read {}: {err}", path.display()))?;
        Lock::parse(&text).map_err(|err| format!("{LOCK_PATH}: {err}"))
    }

    /// Parses and validates lock-file text.
    pub fn parse(text: &str) -> Result<Lock, String> {
        let lock: Lock = toml::from_str(text).map_err(|err| err.to_string())?;
        lock.validate()?;
        Ok(lock)
    }

    /// Every git, file and snapshot item, in lock order.
    pub fn items(&self) -> Vec<Item<'_>> {
        let git = self.git.iter().map(Item::Git);
        let file = self.file.iter().map(Item::File);
        let snapshot = self.snapshot.iter().map(Item::Snapshot);
        git.chain(file).chain(snapshot).collect()
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != FORMAT_VERSION {
            return Err(format!(
                "lock format version {} is not supported (expected {FORMAT_VERSION})",
                self.version
            ));
        }
        let mut names = BTreeSet::new();
        let mut dests: Vec<(&str, &str)> = Vec::new();
        for item in self.items() {
            let name = item.name();
            if !is_slug(name) {
                return Err(format!(
                    "`{name}`: names use lowercase letters, digits, `-` and `.`"
                ));
            }
            if !names.insert(name) {
                return Err(format!("`{name}` appears more than once"));
            }
            check_dest(name, item.dest())?;
            for (other, other_dest) in &dests {
                if nested(item.dest(), other_dest) {
                    return Err(format!(
                        "`{name}` and `{other}` have overlapping destinations"
                    ));
                }
            }
            dests.push((name, item.dest()));
            let terms = item.terms();
            if terms.license.trim().is_empty() {
                return Err(format!("`{name}`: license is empty"));
            }
            if !matches!(terms.mode.as_str(), "fetched" | "run-only") {
                return Err(format!(
                    "`{name}`: mode must be `fetched` or `run-only`, found `{}`",
                    terms.mode
                ));
            }
            match item {
                Item::Git(source) => validate_git(source)?,
                Item::File(source) => check_sha256(name, "sha256", &source.sha256)?,
                Item::Snapshot(source) => {
                    check_sha256(name, "sha256", &source.sha256)?;
                    if !is_date(&source.captured) {
                        return Err(format!("`{name}`: captured must be YYYY-MM-DD"));
                    }
                }
            }
        }
        if let Some(python) = &self.python {
            check_relative("python", "project", &python.project)?;
            check_dest("python", &python.venv)?;
            for (name, dest) in &dests {
                if nested(&python.venv, dest) {
                    return Err(format!("the python venv overlaps `{name}`"));
                }
            }
        }
        Ok(())
    }
}

fn validate_git(source: &GitSource) -> Result<(), String> {
    let name = &source.name;
    if source.commit.len() != 40 || !is_lower_hex(&source.commit) {
        return Err(format!("`{name}`: commit must be a full 40-hex commit id"));
    }
    match (&source.manifest, &source.manifest_sha256) {
        (None, None) => Ok(()),
        (Some(manifest), Some(sha256)) => {
            check_relative(name, "manifest", manifest)?;
            check_sha256(name, "manifest_sha256", sha256)
        }
        _ => Err(format!(
            "`{name}`: manifest and manifest_sha256 go together"
        )),
    }
}

/// A destination must be a plain relative path strictly inside `refs/`.
fn check_dest(name: &str, dest: &str) -> Result<(), String> {
    check_relative(name, "dest", dest)?;
    let mut components = Path::new(dest).components();
    let first = components.next();
    if first != Some(Component::Normal(REFS_DIR.as_ref())) || components.next().is_none() {
        return Err(format!(
            "`{name}`: dest `{dest}` must be inside `{REFS_DIR}/`"
        ));
    }
    Ok(())
}

/// A relative path with forward slashes and no `.`, `..` or empty components.
fn check_relative(name: &str, field: &str, path: &str) -> Result<(), String> {
    let plain = !path.is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
        && !Path::new(path).has_root()
        && !path.contains(':');
    if plain {
        Ok(())
    } else {
        Err(format!(
            "`{name}`: {field} `{path}` must be a relative path with `/` separators and no `.` or \
             `..`"
        ))
    }
}

/// True if one path is the other or contains it.
fn nested(a: &str, b: &str) -> bool {
    let (a, b) = (Path::new(a), Path::new(b));
    a.starts_with(b) || b.starts_with(a)
}

fn check_sha256(name: &str, field: &str, value: &str) -> Result<(), String> {
    if value.len() == 64 && is_lower_hex(value) {
        Ok(())
    } else {
        Err(format!("`{name}`: {field} must be 64 lowercase hex digits"))
    }
}

fn is_lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_slug(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'.')
}

fn is_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            _ => b.is_ascii_digit(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "d909aeae6e063b629a161ba36dcc0f8b51e45313997789c8ecbc71e342971510";
    const COMMIT: &str = "37251476e5cfee330c88aa94cf0dd58f93370ccd";

    fn sample() -> String {
        format!(
            r#"
            version = 1

            [[git]]
            name = "repo"
            url = "https://example.com/repo.git"
            commit = "{COMMIT}"
            dest = "refs/repo"
            shallow = true
            license = "MIT"
            mode = "fetched"

            [[file]]
            name = "paper"
            url = "https://example.com/paper.pdf"
            sha256 = "{HASH}"
            dest = "refs/papers/paper.pdf"
            license = "public domain"
            mode = "fetched"

            [[snapshot]]
            name = "api"
            url = "https://example.com/api.json"
            sha256 = "{HASH}"
            captured = "2026-09-17"
            dest = "refs/snapshots/api.json"
            license = "unknown"
            mode = "fetched"

            [python]
            project = "validation/oracles"
            venv = "refs/venv"
            checks = [{{ module = "rocketpy", version = "1.13.0" }}]

            [java]
            min_major = 17
            "#
        )
    }

    fn error_for(from: &str, to: &str) -> String {
        let text = sample().replacen(from, to, 1);
        assert_ne!(text, sample(), "the replacement `{from}` did not apply");
        Lock::parse(&text).unwrap_err()
    }

    #[test]
    fn parses_every_kind_of_item() {
        let lock = Lock::parse(&sample()).unwrap();
        let names: Vec<_> = lock
            .items()
            .iter()
            .map(|item| item.name().to_owned())
            .collect();
        assert_eq!(names, ["repo", "paper", "api"]);
        assert!(lock.git[0].shallow && !lock.git[0].private);
        assert_eq!(
            lock.python.unwrap().checks[0].version.as_deref(),
            Some("1.13.0")
        );
        assert_eq!(lock.java.unwrap().min_major, 17);
    }

    #[test]
    fn rejects_destinations_outside_refs() {
        for bad in [
            "refs",
            "src/repo",
            "refs/../src",
            "/refs/repo",
            "refs\\repo",
            "refs//repo",
        ] {
            let err = error_for(r#"dest = "refs/repo""#, &format!("dest = {bad:?}"));
            assert!(err.contains("dest"), "{bad}: {err}");
        }
    }

    #[test]
    fn rejects_overlapping_destinations() {
        let err = error_for(
            r#"dest = "refs/papers/paper.pdf""#,
            r#"dest = "refs/repo/paper.pdf""#,
        );
        assert!(err.contains("overlapping"), "{err}");
        let err = error_for(r#"venv = "refs/venv""#, r#"venv = "refs/repo""#);
        assert!(err.contains("overlaps"), "{err}");
    }

    #[test]
    fn rejects_duplicate_names_and_bad_pins() {
        assert!(error_for(r#"name = "paper""#, r#"name = "repo""#).contains("more than once"));
        assert!(error_for(COMMIT, "3725147").contains("40-hex"));
        assert!(error_for(HASH, &HASH.to_uppercase()).contains("64 lowercase hex"));
        assert!(error_for("2026-09-17", "17/09/2026").contains("YYYY-MM-DD"));
        assert!(error_for(r#"mode = "fetched""#, r#"mode = "bundled""#).contains("mode"));
        assert!(error_for("shallow = true", "shalow = true").contains("unknown field"));
        assert!(error_for("version = 1", "version = 2").contains("version 2"));
    }

    #[test]
    fn manifest_needs_its_hash() {
        let err = error_for("shallow = true", r#"manifest = "CHECKSUMS.sha256""#);
        assert!(err.contains("go together"), "{err}");
    }
}
