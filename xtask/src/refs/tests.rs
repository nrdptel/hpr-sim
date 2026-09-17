//! End-to-end tests of `fetch` and `verify` against local `file://` sources (no network), plus
//! consistency checks on the real lock file.

use std::fs;
use std::path::Path;
use std::process::Command;

use super::*;

fn file_url(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

fn git_ok(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
        .args(["-c", "commit.gpgsign=false", "-c", "core.autocrlf=false"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// A local upstream: a git repository with a manifest, a file and an API response.
struct Upstream {
    _dir: tempfile::TempDir,
    repo: String,
    commit: String,
    manifest_sha256: String,
    paper: String,
    paper_sha256: String,
    api_path: std::path::PathBuf,
    api: String,
    api_sha256: String,
}

fn upstream() -> Upstream {
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path().join("repo");
    fs::create_dir(&repo).unwrap();
    git_ok(&repo, &["init", "--quiet"]);
    // Shallow fetches name the commit, which a server must allow (GitHub does).
    git_ok(&repo, &["config", "uploadpack.allowAnySHA1InWant", "true"]);
    fs::write(repo.join("data.csv"), "t,z\n0,0\n").unwrap();
    let manifest = format!("{}  data.csv\n", hash::sha256_bytes(b"t,z\n0,0\n"));
    fs::write(repo.join("CHECKSUMS.sha256"), &manifest).unwrap();
    git_ok(&repo, &["add", "."]);
    git_ok(&repo, &["commit", "--quiet", "-m", "one"]);
    let commit = git_ok(&repo, &["rev-parse", "HEAD"]);
    // A later commit, so a shallow fetch of `commit` really is shallow.
    fs::write(repo.join("later.txt"), "later").unwrap();
    git_ok(&repo, &["add", "."]);
    git_ok(&repo, &["commit", "--quiet", "-m", "two"]);

    let paper = dir.path().join("paper.pdf");
    fs::write(&paper, b"%PDF-1.4 test").unwrap();
    let api_path = dir.path().join("api.json");
    fs::write(&api_path, br#"{"motors":[]}"#).unwrap();
    Upstream {
        repo: file_url(&repo),
        commit,
        manifest_sha256: hash::sha256_bytes(manifest.as_bytes()),
        paper: file_url(&paper),
        paper_sha256: hash::sha256_bytes(b"%PDF-1.4 test"),
        api: file_url(&api_path),
        api_sha256: hash::sha256_bytes(br#"{"motors":[]}"#),
        api_path,
        _dir: dir,
    }
}

fn lock_text(up: &Upstream) -> String {
    format!(
        r#"
        version = 1

        [[git]]
        name = "repo"
        url = "{}"
        commit = "{}"
        dest = "refs/repo"
        shallow = true
        manifest = "CHECKSUMS.sha256"
        manifest_sha256 = "{}"
        license = "MIT"
        mode = "fetched"

        [[git]]
        name = "private-repo"
        url = "{}/does-not-exist"
        commit = "{}"
        dest = "refs/private"
        private = true
        license = "private"
        mode = "fetched"

        [[file]]
        name = "paper"
        url = "{}"
        sha256 = "{}"
        dest = "refs/papers/paper.pdf"
        license = "public domain"
        mode = "fetched"

        [[snapshot]]
        name = "api"
        url = "{}"
        sha256 = "{}"
        captured = "2026-09-17"
        dest = "refs/snapshots/api.json"
        license = "unknown"
        mode = "fetched"
        "#,
        up.repo,
        up.commit,
        up.manifest_sha256,
        up.repo,
        up.commit,
        up.paper,
        up.paper_sha256,
        up.api,
        up.api_sha256
    )
}

fn tally(ok: usize, fetched: usize, skipped: usize, failed: usize) -> Tally {
    Tally {
        ok,
        fetched,
        skipped,
        failed,
    }
}

#[test]
fn fetch_is_idempotent_and_verify_checks_every_hash() {
    let up = upstream();
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let lock = Lock::parse(&lock_text(&up)).unwrap();

    let (first, repins) = fetch(root, &lock, &[], Drift::Fail).unwrap();
    assert_eq!(first, tally(0, 3, 1, 0));
    assert!(repins.is_empty());
    assert!(
        !root.join("refs/private").exists(),
        "a failed private clone leaves nothing behind"
    );
    let shallow = git_ok(
        &root.join("refs/repo"),
        &["rev-parse", "--is-shallow-repository"],
    );
    assert_eq!(shallow, "true");

    let (second, _) = fetch(root, &lock, &[], Drift::Fail).unwrap();
    assert_eq!(second, tally(3, 0, 1, 0), "a second fetch changes nothing");
    assert_eq!(verify(root, &lock, &[]).unwrap(), tally(3, 0, 1, 0));

    // A tracked file edited in the checkout: verify fails, and fetch refuses to touch it.
    let data = root.join("refs/repo/data.csv");
    fs::write(&data, "t,z\n0,1\n").unwrap();
    assert_eq!(
        verify(root, &lock, &["repo".into()]).unwrap(),
        tally(0, 0, 0, 1)
    );
    assert_eq!(
        fetch(root, &lock, &["repo".into()], Drift::Fail).unwrap().0,
        tally(0, 0, 0, 1)
    );
    git_ok(&root.join("refs/repo"), &["checkout", "--", "data.csv"]);
    assert_eq!(
        verify(root, &lock, &["repo".into()]).unwrap(),
        tally(1, 0, 0, 0)
    );

    // A same-size edit that keeps the file's timestamp can hide from `git status`, which trusts
    // its cached stat data (made certain here by ignoring ctime and giving the file an old
    // mtime, so git's same-second "racy" re-check doesn't kick in). Verify still catches it.
    let checkout = root.join("refs/repo");
    git_ok(&checkout, &["config", "core.trustctime", "false"]);
    git_ok(&checkout, &["config", "core.checkStat", "minimal"]);
    let old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let set_mtime = |time| {
        fs::File::options()
            .write(true)
            .open(&data)
            .unwrap()
            .set_modified(time)
    };
    set_mtime(old).unwrap();
    assert_eq!(
        git_ok(&checkout, &["status", "--porcelain"]),
        "",
        "refreshes the stat cache"
    );
    fs::write(&data, "t,z\n0,9\n").unwrap();
    set_mtime(old).unwrap();
    let status = git_ok(&checkout, &["status", "--porcelain"]);
    assert_eq!(status, "", "the stat cache hides the edit from git status");
    assert_eq!(
        verify(root, &lock, &["repo".into()]).unwrap(),
        tally(0, 0, 0, 1)
    );
    fs::write(&data, "t,z\n0,0\n").unwrap();
    assert_eq!(
        verify(root, &lock, &["repo".into()]).unwrap(),
        tally(1, 0, 0, 0)
    );

    // A corrupted download: verify fails, and fetch replaces it.
    fs::write(root.join("refs/papers/paper.pdf"), b"truncated").unwrap();
    assert_eq!(
        verify(root, &lock, &["paper".into()]).unwrap(),
        tally(0, 0, 0, 1)
    );
    assert_eq!(
        fetch(root, &lock, &["paper".into()], Drift::Fail)
            .unwrap()
            .0,
        tally(0, 1, 0, 0)
    );
    assert_eq!(
        verify(root, &lock, &["paper".into()]).unwrap(),
        tally(1, 0, 0, 0)
    );
}

#[test]
fn verify_checks_the_manifest_pin_and_the_commit() {
    let up = upstream();
    let root = tempfile::tempdir().unwrap();
    let root = root.path();
    let text = lock_text(&up);
    let lock = Lock::parse(&text).unwrap();
    fetch(root, &lock, &["repo".into()], Drift::Fail).unwrap();

    let wrong_manifest = text.replace(&up.manifest_sha256, &"0".repeat(64));
    let lock = Lock::parse(&wrong_manifest).unwrap();
    assert_eq!(
        verify(root, &lock, &["repo".into()]).unwrap(),
        tally(0, 0, 0, 1)
    );

    let wrong_commit = text.replacen(&up.commit, &"1".repeat(40), 1);
    let lock = Lock::parse(&wrong_commit).unwrap();
    assert_eq!(
        verify(root, &lock, &["repo".into()]).unwrap(),
        tally(0, 0, 0, 1)
    );
}

#[test]
fn a_moved_snapshot_fails_unless_adopted() {
    let up = upstream();
    fs::write(&up.api_path, br#"{"motors":[1]}"#).unwrap();
    let lock = Lock::parse(&lock_text(&up)).unwrap();
    let only_api = ["api".to_owned()];

    let root = tempfile::tempdir().unwrap();
    let (outcome, repins) = fetch(root.path(), &lock, &only_api, Drift::Fail).unwrap();
    assert_eq!(outcome, tally(0, 0, 0, 1));
    assert!(repins.is_empty());
    assert!(
        root.path()
            .join("refs/snapshots/api.json.unpinned")
            .is_file()
    );
    assert!(!root.path().join("refs/snapshots/api.json").exists());

    let (outcome, repins) = fetch(root.path(), &lock, &only_api, Drift::Adopt).unwrap();
    assert_eq!(outcome, tally(0, 1, 0, 0));
    let new_sha256 = hash::sha256_bytes(br#"{"motors":[1]}"#);
    assert_eq!(
        repins,
        [Repin {
            name: "api".into(),
            sha256: new_sha256.clone()
        }]
    );

    let repinned = download::repin(&lock_text(&up), &repins[0], "2026-09-18").unwrap();
    let lock = Lock::parse(&repinned).unwrap();
    assert_eq!(lock.snapshot[0].sha256, new_sha256);
    assert_eq!(lock.snapshot[0].captured, "2026-09-18");
    assert_eq!(
        verify(root.path(), &lock, &only_api).unwrap(),
        tally(1, 0, 0, 0)
    );
}

#[test]
fn unknown_names_are_rejected() {
    let up = upstream();
    let lock = Lock::parse(&lock_text(&up)).unwrap();
    let root = tempfile::tempdir().unwrap();
    let err = verify(root.path(), &lock, &["nope".into()]).unwrap_err();
    assert!(err.contains("`nope` is not in"), "{err}");
    let err = verify(root.path(), &lock, &[PYTHON.into()]).unwrap_err();
    assert!(
        err.contains("`python` is not in"),
        "the test lock has no python environment"
    );
}

// The real lock file.

fn real_lock() -> (PathBuf, Lock) {
    let root = workspace_root().unwrap();
    let lock = Lock::load(&root).unwrap();
    (root, lock)
}

#[test]
fn the_real_lock_uses_https_and_lives_in_the_ignored_refs_dir() {
    let (root, lock) = real_lock();
    for item in lock.items() {
        let url = match item {
            Item::Git(source) => &source.url,
            Item::File(source) => &source.url,
            Item::Snapshot(source) => &source.url,
        };
        assert!(url.starts_with("https://"), "{}: {url}", item.name());
    }
    let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(
        gitignore.lines().any(|line| line.trim() == "/refs/"),
        ".gitignore must ignore /refs/"
    );
    let python = lock
        .python
        .as_ref()
        .expect("the lock pins the oracle environment");
    for file in ["pyproject.toml", "uv.lock", ".python-version"] {
        assert!(
            root.join(&python.project).join(file).is_file(),
            "{} missing",
            file
        );
    }
    assert!(
        lock.file
            .iter()
            .any(|file| file.name == doctor::OPENROCKET_JAR)
    );
    assert!(lock.java.is_some());
}

#[test]
fn every_real_source_is_in_the_third_party_notices() {
    let (root, lock) = real_lock();
    let notices = fs::read_to_string(root.join("THIRD-PARTY-NOTICES.md")).unwrap();
    for item in lock.items() {
        let name = item.name();
        let row = notices
            .lines()
            .find(|line| line.starts_with(&format!("| `{name}` |")))
            .unwrap_or_else(|| panic!("THIRD-PARTY-NOTICES.md has no row for `{name}`"));
        let terms = item.terms();
        assert!(
            row.contains(&format!("| {} |", terms.license)),
            "`{name}` license: {row}"
        );
        assert!(
            row.contains(&format!("| {} |", terms.mode)),
            "`{name}` mode: {row}"
        );
        if let Item::File(lock::FileSource {
            title: Some(title), ..
        }) = item
        {
            assert!(row.contains(title.as_str()), "`{name}` title: {row}");
        }
    }
}
