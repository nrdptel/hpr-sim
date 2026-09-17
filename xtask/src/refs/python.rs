//! The `uv`-managed Python environment the RocketPy and OpenRocket oracles run in.
//!
//! The environment is a `uv` project (`pyproject.toml` plus `uv.lock`, committed) whose virtual
//! environment lives in the gitignored `refs/`. `uv sync --locked` installs exactly what `uv.lock`
//! pins, refuses a lock that no longer matches `pyproject.toml`, and checks every downloaded
//! artifact against the hash in the lock. `uv sync --check` compares package versions only, so
//! the installed files are also re-hashed against the sha256 each package's `RECORD` lists.

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
        // An activated environment in the caller's shell must not redirect the install.
        .env_remove("VIRTUAL_ENV");
    command
}

/// Whether the installed packages match `uv.lock`. A lock that is stale against
/// `pyproject.toml`, or any other uv failure, is an error rather than "outdated".
fn packages_current(root: &Path, python: &PythonEnv) -> Result<bool, String> {
    if !root.join(&python.venv).is_dir() {
        return Ok(false);
    }
    let mut check = uv(root, python, "sync");
    check.args(["--locked", "--check"]);
    let out = tool::output(&mut check)?;
    if out.status.success() {
        return Ok(true);
    }
    if String::from_utf8_lossy(&out.stderr).contains("environment is outdated") {
        Ok(false)
    } else {
        Err(tool::failure(&check, &out))
    }
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
            "{} of {} installed files differ from their RECORD hashes: {names}",
            self.bad.len(),
            self.checked
        )
    }
}

const RECORD_SCRIPT: &str = r#"
import base64, hashlib, importlib.metadata, json
checked, bad = 0, []
for dist in importlib.metadata.distributions():
    for path in dist.files or []:
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
            bad.append(str(path))
print(json.dumps({"checked": checked, "bad": bad}))
"#;

/// Re-hashes every installed file that a package's `RECORD` lists with a hash.
fn check_records(root: &Path, python: &PythonEnv) -> Result<Records, String> {
    let mut command = Command::new(interpreter(root, python));
    command.arg("-c").arg(RECORD_SCRIPT);
    let json = tool::stdout(&mut command)?;
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
    let reinstall = match packages_current(root, python) {
        Ok(false) => false,
        Ok(true) => match check_records(root, python) {
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
        sync.arg("--reinstall");
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
    match packages_current(root, python) {
        Ok(true) => {}
        Ok(false) => {
            return Outcome::Failed(
                "installed packages differ from uv.lock; run `cargo xtask refs fetch`".to_owned(),
            );
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
    let mut command = Command::new(&interpreter);
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
            summary.starts_with("7 of 10 installed files differ"),
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
