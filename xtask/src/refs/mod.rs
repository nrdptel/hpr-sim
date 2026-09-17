//! `cargo xtask refs fetch|verify|doctor`: the reference library in the gitignored `refs/`.
//!
//! `validation/refs.lock.toml` pins every outside reference the validation work uses:
//!
//! - **git** repositories at a full commit id (RocketPy, openrocket-database, Loft and its
//!   fixtures). A checkout may carry a `sha256sum` manifest whose own hash is pinned too.
//! - **file** downloads that never change (the OpenRocket jar, public papers) by sha256.
//! - **snapshot** captures of live APIs (ThrustCurve, motor.fusionspace.co) by the sha256 and
//!   date of one capture. When the API has moved on, `fetch` fails and keeps the new capture
//!   aside; `fetch --adopt-snapshots` moves it into place and repins the lock.
//! - **python**: a `uv` project whose `uv.lock` pins every package by hash, installed into
//!   `refs/venv`.
//!
//! `fetch` is idempotent: anything already in its pinned state is left alone, so a second run
//! downloads nothing. `verify` re-hashes everything. `doctor` reports which tools and oracles
//! work on this machine. Private repositories that can't be reached (as in CI) are skipped with
//! a note rather than failing.

mod doctor;
mod download;
mod git;
mod hash;
mod java;
mod lock;
mod python;
mod tool;

use std::path::{Path, PathBuf};

use download::{Drift, Repin};
use lock::{Item, Lock};
use tool::Tally;

/// The name that selects the Python environment on the command line.
const PYTHON: &str = "python";

pub const USAGE: &str = "\
  refs fetch [--adopt-snapshots] [name...]
                           Fetch the references pinned in validation/refs.lock.toml into refs/
                           (all, or the named items; `python` names the oracle environment).
                           Items already in their pinned state are left alone.
  refs verify [name...]    Re-hash every fetched reference against the lock.
  refs doctor              Report which tools, references and oracles are usable here.";

pub fn run(args: &[String]) -> Result<(), String> {
    let root = workspace_root()?;
    let (command, rest) = args
        .split_first()
        .ok_or_else(|| format!("`refs` needs a command:\n{USAGE}"))?;
    let lock = Lock::load(&root)?;
    match command.as_str() {
        "fetch" => {
            let adopt = rest.iter().any(|arg| arg == "--adopt-snapshots");
            let names = names(rest, &["--adopt-snapshots"])?;
            let drift = if adopt { Drift::Adopt } else { Drift::Fail };
            let (tally, repins) = fetch(&root, &lock, &names, drift)?;
            if !repins.is_empty() {
                write_repins(&root, &repins)?;
            }
            summary("fetch", &tally)
        }
        "verify" => {
            let names = names(rest, &[])?;
            summary("verify", &verify(&root, &lock, &names)?)
        }
        "doctor" if rest.is_empty() => doctor::run(&root, &lock),
        other => Err(format!(
            "unknown `refs` command or arguments: `{other}`\n{USAGE}"
        )),
    }
}

/// The workspace root: `xtask/` sits directly inside it (ADR-001).
fn workspace_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask has no parent directory".to_owned())
}

fn names(args: &[String], flags: &[&str]) -> Result<Vec<String>, String> {
    let mut names = Vec::new();
    for arg in args {
        if flags.contains(&arg.as_str()) {
            continue;
        }
        if arg.starts_with('-') {
            return Err(format!("unknown option `{arg}`\n{USAGE}"));
        }
        names.push(arg.clone());
    }
    Ok(names)
}

/// The selected items, and whether the Python environment is selected. No names selects all.
fn select<'a>(lock: &'a Lock, names: &[String]) -> Result<(Vec<Item<'a>>, bool), String> {
    let items = lock.items();
    for name in names {
        let known = items.iter().any(|item| item.name() == name)
            || (name == PYTHON && lock.python.is_some());
        if !known {
            return Err(format!("`{name}` is not in {}", lock::LOCK_PATH));
        }
    }
    let wanted = |name: &str| names.is_empty() || names.iter().any(|n| n == name);
    let python = lock.python.is_some() && wanted(PYTHON);
    Ok((
        items
            .into_iter()
            .filter(|item| wanted(item.name()))
            .collect(),
        python,
    ))
}

/// Brings the selected references into their pinned state.
fn fetch(
    root: &Path,
    lock: &Lock,
    names: &[String],
    drift: Drift,
) -> Result<(Tally, Vec<Repin>), String> {
    let (items, python) = select(lock, names)?;
    let mut tally = Tally::default();
    let mut repins = Vec::new();
    for item in items {
        let outcome = match item {
            Item::Git(source) => git::fetch(root, source),
            Item::File(source) => download::fetch_file(root, source),
            Item::Snapshot(source) => {
                let (outcome, repin) = download::fetch_snapshot(root, source, drift);
                repins.extend(repin);
                outcome
            }
        };
        tool::report("fetch", item.name(), &outcome);
        tally.add(&outcome);
    }
    if let (true, Some(env)) = (python, &lock.python) {
        let outcome = python::fetch(root, env);
        tool::report("fetch", PYTHON, &outcome);
        tally.add(&outcome);
    }
    Ok((tally, repins))
}

/// Checks the selected references against every pinned hash.
fn verify(root: &Path, lock: &Lock, names: &[String]) -> Result<Tally, String> {
    let (items, python) = select(lock, names)?;
    let mut tally = Tally::default();
    for item in items {
        let outcome = match item {
            Item::Git(source) => git::verify(root, source),
            Item::File(source) => download::verify_file(root, &source.dest, &source.sha256),
            Item::Snapshot(source) => download::verify_file(root, &source.dest, &source.sha256),
        };
        tool::report("verify", item.name(), &outcome);
        tally.add(&outcome);
    }
    if let (true, Some(env)) = (python, &lock.python) {
        let outcome = python::verify(root, env);
        tool::report("verify", PYTHON, &outcome);
        tally.add(&outcome);
    }
    Ok(tally)
}

/// Repins adopted snapshots in the lock file, then checks the result still parses.
fn write_repins(root: &Path, repins: &[Repin]) -> Result<(), String> {
    let path = root.join(lock::LOCK_PATH);
    let mut text = std::fs::read_to_string(&path)
        .map_err(|err| format!("could not read {}: {err}", path.display()))?;
    for repin in repins {
        text = download::repin(&text, repin)?;
    }
    Lock::parse(&text).map_err(|err| format!("repinning broke the lock file: {err}"))?;
    std::fs::write(&path, text)
        .map_err(|err| format!("could not write {}: {err}", path.display()))?;
    for repin in repins {
        println!(
            "refs fetch: repinned {} in {} (captured {}); commit the lock change",
            repin.name,
            lock::LOCK_PATH,
            repin.captured
        );
    }
    Ok(())
}

fn summary(command: &str, tally: &Tally) -> Result<(), String> {
    println!(
        "refs {command}: {} ok, {} fetched, {} skipped, {} failed",
        tally.ok, tally.fetched, tally.skipped, tally.failed
    );
    if tally.failed == 0 {
        Ok(())
    } else {
        Err(format!("refs {command}: {} item(s) failed", tally.failed))
    }
}

#[cfg(test)]
mod tests;
