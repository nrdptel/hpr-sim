//! The workspace layout, read from `cargo metadata`.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// The workspace root and its member crates.
#[derive(Debug)]
pub struct Workspace {
    pub root: PathBuf,
    pub packages: Vec<Package>,
}

/// A workspace member, as far as the xtask commands need to know it.
#[derive(Debug)]
pub struct Package {
    pub name: String,
    /// `[package.metadata.hpr] wasm = true`: the crate belongs to the pure core.
    pub wasm: bool,
}

/// The cargo binary that is running this xtask, so the pinned toolchain is used throughout.
pub fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// Runs `cargo metadata --no-deps` for the workspace that contains `dir`.
pub fn load(dir: &Path) -> Result<Workspace, String> {
    let output = Command::new(cargo())
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(dir)
        .output()
        .map_err(|err| format!("could not run `cargo metadata`: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo metadata` failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let json = String::from_utf8(output.stdout)
        .map_err(|err| format!("`cargo metadata` printed invalid UTF-8: {err}"))?;
    parse(&json)
}

/// Parses the JSON printed by `cargo metadata --format-version 1 --no-deps`.
pub fn parse(json: &str) -> Result<Workspace, String> {
    let metadata: Value =
        serde_json::from_str(json).map_err(|err| format!("bad `cargo metadata` JSON: {err}"))?;
    let root = metadata["workspace_root"]
        .as_str()
        .ok_or("`cargo metadata` JSON has no workspace_root")?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("`cargo metadata` JSON has no packages array")?
        .iter()
        .map(parse_package)
        .collect::<Result<_, _>>()?;
    Ok(Workspace {
        root: PathBuf::from(root),
        packages,
    })
}

fn parse_package(package: &Value) -> Result<Package, String> {
    let name = package["name"]
        .as_str()
        .ok_or("a package in `cargo metadata` JSON has no name")?
        .to_owned();
    let wasm = match &package["metadata"]["hpr"]["wasm"] {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        other => {
            return Err(format!(
                "{name}: [package.metadata.hpr] wasm must be true or false, found {other}"
            ));
        }
    };
    Ok(Package { name, wasm })
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &str = r#"{
        "workspace_root": "/work/hpr-sim",
        "packages": [
            { "name": "pure", "metadata": { "hpr": { "wasm": true } } },
            { "name": "tool", "metadata": null },
            { "name": "other", "metadata": { "docs": {} } }
        ]
    }"#;

    #[test]
    fn parses_the_root_and_the_wasm_flags() {
        let workspace = parse(METADATA).unwrap();
        assert_eq!(workspace.root, PathBuf::from("/work/hpr-sim"));
        let flags: Vec<_> = workspace
            .packages
            .iter()
            .map(|p| (p.name.as_str(), p.wasm))
            .collect();
        assert_eq!(flags, [("pure", true), ("tool", false), ("other", false)]);
    }

    #[test]
    fn rejects_a_non_boolean_wasm_flag() {
        let json = METADATA.replace(r#""wasm": true"#, r#""wasm": "yes""#);
        let err = parse(&json).unwrap_err();
        assert!(err.contains("wasm must be true or false"), "{err}");
    }
}
