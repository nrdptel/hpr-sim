//! The `uv`-managed Python environment the RocketPy and OpenRocket oracles run in.
//!
//! The environment is a `uv` project (`pyproject.toml` plus `uv.lock`, committed) whose virtual
//! environment lives in the gitignored `refs/`. `uv sync --locked` installs exactly what `uv.lock`
//! pins, refuses a lock that no longer matches `pyproject.toml`, and checks every downloaded
//! artifact against the hash in the lock. `uv sync --check` compares package versions only, so
//! the installed files are also re-hashed against the sha256 each package's `RECORD` lists, and
//! any file no `RECORD` lists (apart from the venv's own scaffolding) is reported.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::lock::PythonEnv;
use super::tool::{self, Outcome};

/// A `uv` command for the project, with its environment redirected into `refs/`.
fn uv(root: &Path, python: &PythonEnv, subcommand: &str) -> Command {
    let mut command = Command::new("uv");
    command
        .current_dir(root)
        .arg(subcommand)
        .arg("--project")
        .arg(root.join(&python.project))
        .env("UV_PROJECT_ENVIRONMENT", root.join(&python.venv))
        // Copies, not hard links into uv's cache: an edited venv file must not also corrupt the
        // cached copy that a reinstall would link back in.
        .env("UV_LINK_MODE", "copy")
        // An activated environment in the caller's shell must not redirect the install.
        .env_remove("VIRTUAL_ENV");
    command
}

/// `None` when the installed packages match `uv.lock`; otherwise uv's reason (an outdated
/// environment, or a lock that is stale against `pyproject.toml`).
fn package_mismatch(root: &Path, python: &PythonEnv) -> Result<Option<String>, String> {
    if !root.join(&python.venv).is_dir() {
        return Ok(Some("the environment does not exist".to_owned()));
    }
    let mut check = uv(root, python, "sync");
    check.args(["--locked", "--check"]);
    let out = tool::output(&mut check)?;
    if out.status.success() {
        return Ok(None);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    let reason = stderr
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map_or_else(
            || format!("`uv sync --check` failed ({})", out.status),
            str::to_owned,
        );
    Ok(Some(reason))
}

/// The result of re-hashing the installed files.
struct Records {
    checked: usize,
    bad: Vec<String>,
}

impl Records {
    fn summary(&self) -> String {
        const SHOWN: usize = 5;
        let mut names = self.bad[..self.bad.len().min(SHOWN)].join(", ");
        if self.bad.len() > SHOWN {
            names.push_str(&format!(" and {} more", self.bad.len() - SHOWN));
        }
        format!(
            "{} problem(s) among {} installed files: {names}",
            self.bad.len(),
            self.checked
        )
    }
}

const RECORD_SCRIPT: &str = r#"
import base64, hashlib, importlib.metadata, json, os, sys

def key(path):
    return os.path.normcase(os.path.abspath(path))

checked, bad, listed = 0, [], set()
for dist in importlib.metadata.distributions():
    for path in dist.files or []:
        listed.add(key(dist.locate_file(path)))
        if path.hash is None:
            continue
        if path.hash.mode != "sha256":
            bad.append(f"{path} (hash {path.hash.mode})")
            continue
        try:
            data = dist.locate_file(path).read_bytes()
        except OSError:
            bad.append(f"{path} (unreadable)")
            continue
        checked += 1
        digest = base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()
        if digest != path.hash.value:
            bad.append(f"{path} (hash differs)")

# Files no RECORD lists could still run (a sitecustomize.py or .pth file, say). Only the venv's
# own scaffolding and bytecode caches may be unlisted.
ROOT_FILES = {".gitignore", ".lock", "CACHEDIR.TAG", "pyvenv.cfg"}
def scaffolding(parts):
    name = parts[-1]
    if len(parts) == 1:
        return name in ROOT_FILES
    if len(parts) == 2 and parts[0] in ("bin", "Scripts"):
        return (name.startswith(("activate", "deactivate", "pydoc")) or name == "python"
                or name.startswith("python3") or name in ("python.exe", "pythonw.exe"))
    return parts[-2] == "site-packages" and name in ("_virtualenv.py", "_virtualenv.pth")

for dirpath, dirnames, filenames in os.walk(sys.prefix):
    dirnames[:] = [d for d in dirnames if d != "__pycache__"]
    for name in filenames:
        full = os.path.join(dirpath, name)
        parts = os.path.relpath(full, sys.prefix).split(os.sep)
        if key(full) not in listed and not scaffolding(parts):
            bad.append(f"{'/'.join(parts)} (not in any RECORD)")
print(json.dumps({"checked": checked, "bad": bad}))
"#;

/// Re-hashes every installed file that a package's `RECORD` lists with a hash, and reports files
/// that no `RECORD` lists.
fn check_records(root: &Path, python: &PythonEnv) -> Result<Records, String> {
    let json = tool::stdout(python_command(root, python).arg("-c").arg(RECORD_SCRIPT))?;
    parse_records(&json)
}

fn parse_records(json: &str) -> Result<Records, String> {
    let value: Value = serde_json::from_str(json)
        .map_err(|err| format!("unexpected RECORD-check output: {err}"))?;
    let checked = value["checked"]
        .as_u64()
        .ok_or("the RECORD check did not report a count")?;
    let bad = value["bad"]
        .as_array()
        .ok_or("the RECORD check did not report its failures")?
        .iter()
        .map(|path| path.as_str().unwrap_or("?").to_owned())
        .collect();
    Ok(Records {
        checked: usize::try_from(checked).unwrap_or(usize::MAX),
        bad,
    })
}

pub fn fetch(root: &Path, python: &PythonEnv) -> Outcome {
    let reinstall = match package_mismatch(root, python) {
        Ok(Some(_)) => false,
        Ok(None) => match check_records(root, python) {
            Ok(records) if records.bad.is_empty() => {
                return Outcome::Ok(format!(
                    "{} matches uv.lock; {} files match their RECORD hashes",
                    python.venv, records.checked
                ));
            }
            Ok(_) => true,
            Err(err) => return Outcome::Failed(err),
        },
        Err(err) => return Outcome::Failed(err),
    };
    let mut sync = uv(root, python, "sync");
    sync.args(["--locked", "--quiet"]);
    if reinstall {
        // Fresh downloads, in case the cache itself is what changed.
        sync.args(["--reinstall", "--no-cache"]);
    }
    match tool::output(&mut sync) {
        Ok(out) if out.status.success() => {}
        Ok(out) => return Outcome::Failed(tool::failure(&sync, &out)),
        Err(err) => return Outcome::Failed(err),
    }
    match check_records(root, python) {
        Ok(records) if records.bad.is_empty() => Outcome::Fetched(format!(
            "{} {} from uv.lock; {} files match their RECORD hashes",
            if reinstall { "reinstalled" } else { "synced" },
            python.venv,
            records.checked
        )),
        Ok(records) => Outcome::Failed(records.summary()),
        Err(err) => Outcome::Failed(err),
    }
}

pub fn verify(root: &Path, python: &PythonEnv) -> Outcome {
    if !root.join(&python.venv).is_dir() {
        return Outcome::Failed("missing; run `cargo xtask refs fetch`".to_owned());
    }
    match package_mismatch(root, python) {
        Ok(None) => {}
        Ok(Some(reason)) => {
            return Outcome::Failed(format!("{reason}; run `cargo xtask refs fetch`"));
        }
        Err(err) => return Outcome::Failed(err),
    }
    match check_records(root, python) {
        Ok(records) if records.bad.is_empty() => Outcome::Ok(format!(
            "packages match uv.lock; {} files match their RECORD hashes",
            records.checked
        )),
        Ok(records) => Outcome::Failed(records.summary()),
        Err(err) => Outcome::Failed(err),
    }
}

/// The environment's interpreter.
pub fn interpreter(root: &Path, python: &PythonEnv) -> PathBuf {
    let venv = root.join(&python.venv);
    if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    }
}

/// The interpreter in isolated mode (`-I`): `PYTHONPATH`, the user site directory and the
/// current directory stay off `sys.path`, so only the environment's own packages are seen.
pub fn python_command(root: &Path, python: &PythonEnv) -> Command {
    let mut command = Command::new(interpreter(root, python));
    command.arg("-I");
    command
}

/// The result of importing one module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Import {
    /// Imported; the version it reports, if any.
    Ok(Option<String>),
    Failed(String),
}

const IMPORT_SCRIPT: &str = r#"
import importlib, importlib.metadata, json, sys
out = {}
for name in sys.argv[1:]:
    try:
        module = importlib.import_module(name)
        version = getattr(module, "__version__", None)
        if not isinstance(version, str):
            try:
                version = importlib.metadata.version(name)
            except importlib.metadata.PackageNotFoundError:
                version = None
        out[name] = {"ok": True, "version": version}
    except BaseException as err:
        out[name] = {"ok": False, "error": type(err).__name__ + ": " + str(err)}
print(json.dumps(out))
"#;

/// Imports each module in the environment's interpreter and reports the outcome per module.
pub fn import(
    root: &Path,
    python: &PythonEnv,
    modules: &[&str],
) -> Result<Vec<(String, Import)>, String> {
    let interpreter = interpreter(root, python);
    if !interpreter.exists() {
        return Err(format!("{} does not exist", interpreter.display()));
    }
    let mut command = python_command(root, python);
    command.arg("-c").arg(IMPORT_SCRIPT).args(modules);
    let json = tool::stdout(&mut command)?;
    parse_imports(&json, modules)
}

fn parse_imports(json: &str, modules: &[&str]) -> Result<Vec<(String, Import)>, String> {
    let value: Value = serde_json::from_str(json)
        .map_err(|err| format!("unexpected import-check output: {err}"))?;
    modules
        .iter()
        .map(|&module| {
            let entry = &value[module];
            let import = match entry["ok"].as_bool() {
                Some(true) => Import::Ok(entry["version"].as_str().map(str::to_owned)),
                Some(false) => Import::Failed(entry["error"].as_str().unwrap_or("").to_owned()),
                None => return Err(format!("the import check did not report `{module}`")),
            };
            Ok((module.to_owned(), import))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_record_results_and_caps_the_names_shown() {
        let records = parse_records(r#"{"checked": 4675, "bad": []}"#).unwrap();
        assert_eq!((records.checked, records.bad.len()), (4675, 0));
        let bad: Vec<String> = (0..7).map(|i| format!("\"pkg/f{i}.py\"")).collect();
        let json = format!(r#"{{"checked": 10, "bad": [{}]}}"#, bad.join(","));
        let summary = parse_records(&json).unwrap().summary();
        assert!(
            summary.starts_with("7 problem(s) among 10 installed files"),
            "{summary}"
        );
        assert!(summary.ends_with("pkg/f4.py and 2 more"), "{summary}");
        assert!(parse_records("{}").is_err());
    }

    #[test]
    fn parses_import_results() {
        let json = r#"{"rocketpy": {"ok": true, "version": "1.13.0"},
                       "jpype": {"ok": true, "version": null},
                       "orhelper": {"ok": false, "error": "ModuleNotFoundError: x"}}"#;
        let imports = parse_imports(json, &["rocketpy", "jpype", "orhelper"]).unwrap();
        assert_eq!(
            imports,
            [
                ("rocketpy".to_owned(), Import::Ok(Some("1.13.0".to_owned()))),
                ("jpype".to_owned(), Import::Ok(None)),
                (
                    "orhelper".to_owned(),
                    Import::Failed("ModuleNotFoundError: x".to_owned())
                ),
            ]
        );
        assert!(
            parse_imports(json, &["numpy"])
                .unwrap_err()
                .contains("numpy")
        );
    }
}
