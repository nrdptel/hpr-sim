//! The `uv`-managed Python environment the RocketPy and OpenRocket oracles run in.
//!
//! The environment is a `uv` project (`pyproject.toml` plus `uv.lock`, committed) whose virtual
//! environment lives in the gitignored `refs/`. `uv sync --frozen` installs exactly what
//! `uv.lock` pins and checks every downloaded artifact against the hash recorded there.

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

/// Whether the environment exists and matches `uv.lock` exactly, without changing anything.
fn in_sync(root: &Path, python: &PythonEnv) -> Result<bool, String> {
    if !root.join(&python.venv).is_dir() {
        return Ok(false);
    }
    let out = tool::output(uv(root, python, "sync").args(["--frozen", "--check"]))?;
    Ok(out.status.success())
}

pub fn fetch(root: &Path, python: &PythonEnv) -> Outcome {
    match in_sync(root, python) {
        Ok(true) => return Outcome::Ok(format!("{} matches uv.lock", python.venv)),
        Ok(false) => {}
        Err(err) => return Outcome::Failed(err),
    }
    let mut sync = uv(root, python, "sync");
    sync.args(["--frozen", "--quiet"]);
    match tool::output(&mut sync) {
        Ok(out) if out.status.success() => {
            Outcome::Fetched(format!("synced {} from uv.lock", python.venv))
        }
        Ok(out) => Outcome::Failed(tool::failure(&sync, &out)),
        Err(err) => Outcome::Failed(err),
    }
}

pub fn verify(root: &Path, python: &PythonEnv) -> Outcome {
    if !root.join(&python.venv).is_dir() {
        return Outcome::Failed("missing; run `cargo xtask refs fetch`".to_owned());
    }
    match in_sync(root, python) {
        Ok(true) => Outcome::Ok("installed packages match uv.lock".to_owned()),
        Ok(false) => Outcome::Failed(
            "installed packages differ from uv.lock; run `cargo xtask refs fetch`".to_owned(),
        ),
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
