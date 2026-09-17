//! The workspace layout, read from `cargo metadata`.

use std::collections::BTreeMap;
use std::path::PathBuf;
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
    pub dependencies: Vec<Dependency>,
    /// The `[features]` table: feature name to the entries it enables.
    pub features: BTreeMap<String, Vec<String>>,
}

/// One entry of a package's dependency tables.
#[derive(Debug)]
pub struct Dependency {
    /// The name of the package depended on.
    pub name: String,
    /// The name the dependency is known by in `[features]` (differs from `name` when renamed).
    pub key: String,
    pub kind: DependencyKind,
    pub optional: bool,
}

/// Which dependency table an entry comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    Normal,
    Dev,
    Build,
}

/// The cargo binary that is running this xtask, so the pinned toolchain is used throughout.
pub fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
}

/// Runs `cargo metadata --no-deps` for the workspace that contains this crate.
pub fn load() -> Result<Workspace, String> {
    let output = Command::new(cargo())
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
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
    let name = str_field(package, "name")?;
    let wasm = match &package["metadata"]["hpr"]["wasm"] {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        other => {
            return Err(format!(
                "{name}: [package.metadata.hpr] wasm must be true or false, found {other}"
            ));
        }
    };
    let dependencies = package["dependencies"]
        .as_array()
        .ok_or_else(|| format!("{name}: no dependencies array"))?
        .iter()
        .map(|dep| parse_dependency(&name, dep))
        .collect::<Result<_, _>>()?;
    let mut features = BTreeMap::new();
    if let Some(table) = package["features"].as_object() {
        for (feature, entries) in table {
            let entries = entries
                .as_array()
                .ok_or_else(|| format!("{name}: feature `{feature}` is not an array"))?
                .iter()
                .map(|entry| {
                    entry.as_str().map(str::to_owned).ok_or_else(|| {
                        format!("{name}: feature `{feature}` has a non-string entry")
                    })
                })
                .collect::<Result<_, _>>()?;
            features.insert(feature.clone(), entries);
        }
    }
    Ok(Package {
        name,
        wasm,
        dependencies,
        features,
    })
}

fn parse_dependency(package: &str, dep: &Value) -> Result<Dependency, String> {
    let name = str_field(dep, "name").map_err(|err| format!("{package}: {err}"))?;
    let kind = match &dep["kind"] {
        Value::Null => DependencyKind::Normal,
        Value::String(kind) if kind == "dev" => DependencyKind::Dev,
        Value::String(kind) if kind == "build" => DependencyKind::Build,
        other => {
            return Err(format!(
                "{package}: dependency {name} has unknown kind {other}"
            ));
        }
    };
    let key = dep["rename"].as_str().unwrap_or(&name).to_owned();
    Ok(Dependency {
        key,
        kind,
        optional: dep["optional"].as_bool().unwrap_or(false),
        name,
    })
}

fn str_field(value: &Value, field: &str) -> Result<String, String> {
    value[field]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("missing string field `{field}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &str = r#"{
        "workspace_root": "/work/hpr-sim",
        "packages": [
            {
                "name": "pure",
                "metadata": { "hpr": { "wasm": true } },
                "dependencies": [
                    { "name": "base", "kind": null, "optional": false, "rename": null },
                    { "name": "helper", "kind": "dev", "optional": false, "rename": null },
                    { "name": "gen", "kind": "build", "optional": false, "rename": null },
                    { "name": "net", "kind": null, "optional": true, "rename": "online" }
                ],
                "features": { "default": [], "net": ["dep:online"] }
            },
            {
                "name": "tool",
                "metadata": null,
                "dependencies": [],
                "features": {}
            }
        ]
    }"#;

    #[test]
    fn parses_packages_dependencies_and_features() {
        let workspace = parse(METADATA).unwrap();
        assert_eq!(workspace.root, PathBuf::from("/work/hpr-sim"));
        assert_eq!(workspace.packages.len(), 2);

        let pure = &workspace.packages[0];
        assert!(pure.wasm);
        let kinds: Vec<_> = pure.dependencies.iter().map(|d| d.kind).collect();
        assert_eq!(
            kinds,
            [
                DependencyKind::Normal,
                DependencyKind::Dev,
                DependencyKind::Build,
                DependencyKind::Normal
            ]
        );
        let net = &pure.dependencies[3];
        assert_eq!((net.name.as_str(), net.key.as_str()), ("net", "online"));
        assert!(net.optional);
        assert_eq!(pure.features["net"], ["dep:online"]);

        assert!(!workspace.packages[1].wasm);
    }

    #[test]
    fn rejects_a_non_boolean_wasm_flag() {
        let json = METADATA.replace(r#""wasm": true"#, r#""wasm": "yes""#);
        let err = parse(&json).unwrap_err();
        assert!(err.contains("wasm must be true or false"), "{err}");
    }

    #[test]
    fn rejects_an_unknown_dependency_kind() {
        let json = METADATA.replace(r#""kind": "dev""#, r#""kind": "weird""#);
        let err = parse(&json).unwrap_err();
        assert!(err.contains("unknown kind"), "{err}");
    }
}
