//! Git checkouts pinned to a commit.

use std::path::Path;

use super::hash;
use super::lock::GitSource;
use super::tool::{self, Outcome};

/// The state of a checkout directory.
#[derive(Debug, PartialEq, Eq)]
enum State {
    Missing,
    /// A git checkout. `head` is `None` in a repository with no commits yet.
    Checkout {
        head: Option<String>,
        modified: bool,
    },
}

fn inspect(dir: &Path) -> Result<State, String> {
    if !dir.exists() {
        return Ok(State::Missing);
    }
    let top =
        tool::stdout(tool::git(dir).args(["rev-parse", "--show-toplevel"])).map_err(|_| {
            format!(
                "{} exists but is not a git checkout; move it aside and fetch again",
                dir.display()
            )
        })?;
    if !same_dir(Path::new(&top), dir) {
        return Err(format!(
            "{} is not the root of a git checkout (the enclosing checkout is {top})",
            dir.display()
        ));
    }
    let head = tool::stdout(tool::git(dir).args(["rev-parse", "--verify", "--quiet", "HEAD"])).ok();
    let changes =
        tool::stdout(tool::git(dir).args(["status", "--porcelain", "--untracked-files=no"]))?;
    Ok(State::Checkout {
        head,
        modified: !changes.is_empty(),
    })
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn short(commit: &str) -> &str {
    commit.get(..10).unwrap_or(commit)
}

/// The pinned commit for messages: `v1.13.0 (9bd6ad3af8)`, or just the short commit.
fn pin(source: &GitSource) -> String {
    match &source.tag {
        Some(tag) => format!("{tag} ({})", short(&source.commit)),
        None => short(&source.commit).to_owned(),
    }
}

/// Brings the checkout to the pinned commit. An existing checkout is reused: if it is already at
/// the commit nothing happens, and otherwise the commit is fetched if needed and checked out
/// detached. Checkouts with local changes to tracked files are never touched.
pub fn fetch(root: &Path, source: &GitSource) -> Outcome {
    let dir = root.join(&source.dest);
    let state = match inspect(&dir) {
        Ok(state) => state,
        Err(err) => return Outcome::Failed(err),
    };
    let created = match state {
        State::Checkout {
            head: Some(ref head),
            modified: false,
        } if *head == source.commit => {
            return Outcome::Ok(format!("at {}", pin(source)));
        }
        State::Checkout { modified: true, .. } => {
            return Outcome::Failed(format!(
                "{} has local changes to tracked files; leaving it alone",
                source.dest
            ));
        }
        State::Checkout { .. } => false,
        State::Missing => {
            if let Err(err) = init(&dir) {
                return Outcome::Failed(err);
            }
            true
        }
    };
    match checkout(&dir, source) {
        Ok(()) if created => Outcome::Fetched(format!(
            "cloned {} at {}",
            if source.shallow {
                "shallow"
            } else {
                "with history"
            },
            pin(source)
        )),
        Ok(()) => Outcome::Fetched(format!("checked out {}", pin(source))),
        Err(err) => {
            if created {
                // Leave no half-made checkout behind, so the next run starts clean.
                let _ = std::fs::remove_dir_all(&dir);
            }
            if source.private {
                Outcome::Skipped(format!(
                    "private repository, not reachable with the current credentials ({})",
                    first_line(&err)
                ))
            } else {
                Outcome::Failed(err)
            }
        }
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or(text)
}

fn init(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|err| format!("could not create {}: {err}", dir.display()))?;
    tool::stdout(tool::git(dir).args(["init", "--quiet"]))?;
    // Keep file bytes exactly as committed on every OS, so manifest hashes hold on Windows too.
    tool::stdout(tool::git(dir).args(["config", "core.autocrlf", "false"]))?;
    Ok(())
}

fn checkout(dir: &Path, source: &GitSource) -> Result<(), String> {
    let have = tool::git(dir)
        .args(["cat-file", "-e"])
        .arg(format!("{}^{{commit}}", source.commit))
        .output()
        .is_ok_and(|out| out.status.success());
    if !have {
        let mut fetch = tool::git(dir);
        fetch.args(["fetch", "--quiet", "--no-tags"]);
        if source.shallow {
            fetch.args(["--depth", "1"]);
        }
        fetch.arg(&source.url).arg(&source.commit);
        tool::stdout(&mut fetch)?;
    }
    tool::stdout(tool::git(dir).args([
        "-c",
        "advice.detachedHead=false",
        "checkout",
        "--quiet",
        "--detach",
        &source.commit,
    ]))?;
    let head = tool::stdout(tool::git(dir).args(["rev-parse", "HEAD"]))?;
    if head == source.commit {
        Ok(())
    } else {
        Err(format!("checked out {head}, expected {}", source.commit))
    }
}

/// Checks that the checkout is at the pinned commit and that every tracked file's content matches
/// that commit (git re-hashes the files), plus every file in the manifest if there is one.
pub fn verify(root: &Path, source: &GitSource) -> Outcome {
    let dir = root.join(&source.dest);
    match inspect(&dir) {
        Ok(State::Missing) if source.private => {
            Outcome::Skipped("private repository, not fetched".to_owned())
        }
        Ok(State::Missing) => Outcome::Failed("missing; run `cargo xtask refs fetch`".to_owned()),
        Ok(State::Checkout { head, .. }) if head.as_deref() != Some(source.commit.as_str()) => {
            Outcome::Failed(format!(
                "at {}, but the lock pins {}",
                head.as_deref().map_or("no commit", short),
                short(&source.commit)
            ))
        }
        Ok(State::Checkout { .. }) => match verify_content(&dir, source) {
            Ok(detail) => Outcome::Ok(detail),
            Err(err) => Outcome::Failed(err),
        },
        Err(err) => Outcome::Failed(err),
    }
}

fn verify_content(dir: &Path, source: &GitSource) -> Result<String, String> {
    // The checkout's own index caches file timestamps and sizes, so a same-size edit that keeps
    // the timestamp would pass a plain `git status`. A fresh index read from HEAD has no cached
    // stat data, which makes git compare the content of every tracked file with its blob.
    let index = tool::stdout(tool::git(dir).args(["rev-parse", "--git-path", "hpr-verify.index"]))?;
    let index = dir.join(index);
    let _ = std::fs::remove_file(&index);
    let mut read_tree = tool::git(dir);
    read_tree
        .env("GIT_INDEX_FILE", &index)
        .args(["read-tree", "HEAD"]);
    let mut status = tool::git(dir);
    status.env("GIT_INDEX_FILE", &index).args([
        "status",
        "--porcelain",
        "--untracked-files=no",
        "--no-renames",
    ]);
    let changed = tool::stdout(&mut read_tree).and_then(|_| tool::stdout(&mut status));
    let _ = std::fs::remove_file(&index);
    let changed = changed?;
    if !changed.is_empty() {
        return Err(format!(
            "{} tracked file(s) differ from {}",
            changed.lines().count(),
            pin(source)
        ));
    }
    let mut detail = format!("at {}, tracked files match", pin(source));
    if let (Some(manifest), Some(pin)) = (&source.manifest, &source.manifest_sha256) {
        let files = verify_manifest(dir, manifest, pin)?;
        detail.push_str(&format!(", {files} manifest hashes match"));
    }
    Ok(detail)
}

/// Checks the manifest's own hash, then the hash of every file it lists. Returns the file count.
fn verify_manifest(dir: &Path, manifest: &str, pin: &str) -> Result<usize, String> {
    let path = dir.join(manifest);
    let actual = hash::sha256_file(&path)?;
    if actual != pin {
        return Err(format!(
            "{manifest} has sha256 {actual}, but the lock pins {pin}"
        ));
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|err| format!("could not read {}: {err}", path.display()))?;
    let entries = hash::parse_manifest(&text).map_err(|err| format!("{manifest}: {err}"))?;
    let mut bad = Vec::new();
    for entry in &entries {
        match hash::sha256_file(&dir.join(&entry.path)) {
            Ok(sha256) if sha256 == entry.sha256 => {}
            Ok(_) => bad.push(format!("{} (hash differs)", entry.path)),
            Err(_) => bad.push(format!("{} (unreadable)", entry.path)),
        }
    }
    if bad.is_empty() {
        Ok(entries.len())
    } else {
        Err(format!(
            "{} of {} manifest entries fail: {}",
            bad.len(),
            entries.len(),
            bad.join(", ")
        ))
    }
}
