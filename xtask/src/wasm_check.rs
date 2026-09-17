//! `cargo xtask wasm-check`: the pure core must build for `wasm32-unknown-unknown`.
//!
//! A crate joins the pure core by declaring this in its manifest:
//!
//! ```toml
//! [package.metadata.hpr]
//! wasm = true
//! ```
//!
//! The check has two parts:
//!
//! 1. A layering rule. Every normal dependency of a pure crate on another workspace crate must be
//!    a pure crate too. Optional dependencies count when the default features enable them, and
//!    target-specific dependencies count regardless of their target.
//! 2. `cargo check --target wasm32-unknown-unknown` on every pure crate, with default features.

use std::collections::BTreeSet;
use std::process::Command;

use crate::workspace::{self, DependencyKind, Package};

/// The WebAssembly target the pure core must build for.
pub const TARGET: &str = "wasm32-unknown-unknown";

/// Runs the check. `cargo_args` are appended to the `cargo check` command line.
pub fn run(cargo_args: &[String]) -> Result<(), String> {
    let workspace = workspace::load()?;
    let pure = pure_crates(&workspace.packages);
    if pure.is_empty() {
        return Err("no crate declares `[package.metadata.hpr] wasm = true`".to_owned());
    }
    let violations = layering_violations(&workspace.packages);
    if !violations.is_empty() {
        return Err(format!(
            "pure-core crates depend on crates outside the pure core:\n{}",
            violations.join("\n")
        ));
    }
    println!(
        "wasm-check: {} pure-core crates: {}",
        pure.len(),
        pure.join(", ")
    );

    let mut command = Command::new(workspace::cargo());
    command
        .current_dir(&workspace.root)
        .args(["check", "--target", TARGET]);
    for name in &pure {
        command.args(["--package", name]);
    }
    command.args(cargo_args);
    let status = command
        .status()
        .map_err(|err| format!("could not run `cargo check`: {err}"))?;
    if !status.success() {
        return Err(format!(
            "`cargo check --target {TARGET}` failed ({status}). If the target is missing, \
             run `rustup target add {TARGET}`."
        ));
    }
    println!("wasm-check: ok");
    Ok(())
}

/// The names of the pure-core crates, sorted.
pub fn pure_crates(packages: &[Package]) -> Vec<String> {
    let mut names: Vec<_> = packages
        .iter()
        .filter(|package| package.wasm)
        .map(|package| package.name.clone())
        .collect();
    names.sort();
    names
}

/// Every `pure crate -> non-pure workspace crate` edge that a default-feature build would compile,
/// one per line.
pub fn layering_violations(packages: &[Package]) -> Vec<String> {
    let members: BTreeSet<&str> = packages.iter().map(|p| p.name.as_str()).collect();
    let pure: BTreeSet<&str> = packages
        .iter()
        .filter(|p| p.wasm)
        .map(|p| p.name.as_str())
        .collect();
    let mut violations = Vec::new();
    for package in packages.iter().filter(|p| p.wasm) {
        let defaults = default_feature_entries(package);
        for dep in &package.dependencies {
            let outside_core =
                members.contains(dep.name.as_str()) && !pure.contains(dep.name.as_str());
            let compiled = dep.kind == DependencyKind::Normal
                && (!dep.optional || enables_dependency(&defaults, &dep.key));
            if outside_core && compiled {
                violations.push(format!("  {} -> {}", package.name, dep.name));
            }
        }
    }
    violations
}

/// Every entry reachable from the `default` feature, following feature-to-feature references.
fn default_feature_entries(package: &Package) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut pending = vec!["default".to_owned()];
    while let Some(feature) = pending.pop() {
        for entry in package.features.get(&feature).into_iter().flatten() {
            if seen.insert(entry.clone()) && package.features.contains_key(entry) {
                pending.push(entry.clone());
            }
        }
    }
    seen
}

/// Whether feature entries switch on the optional dependency known as `key`: `dep:key`, `key`,
/// or `key/feature`. The weak form `key?/feature` does not.
fn enables_dependency(entries: &BTreeSet<String>, key: &str) -> bool {
    entries.iter().any(|entry| {
        entry.strip_prefix("dep:") == Some(key)
            || entry == key
            || entry
                .strip_prefix(key)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::workspace::Dependency;
    use DependencyKind::{Build, Dev, Normal};

    fn package(name: &str, wasm: bool, deps: &[(&str, DependencyKind, bool)]) -> Package {
        Package {
            name: name.to_owned(),
            wasm,
            dependencies: deps
                .iter()
                .map(|&(dep, kind, optional)| Dependency {
                    name: dep.to_owned(),
                    key: dep.to_owned(),
                    kind,
                    optional,
                })
                .collect(),
            features: BTreeMap::new(),
        }
    }

    fn with_features(mut package: Package, features: &[(&str, &[&str])]) -> Package {
        for (feature, entries) in features {
            package.features.insert(
                (*feature).to_owned(),
                entries.iter().map(|e| (*e).to_owned()).collect(),
            );
        }
        package
    }

    #[test]
    fn a_pure_crate_may_depend_on_pure_crates_and_external_crates() {
        let packages = [
            package("core", true, &[("glam", Normal, false)]),
            package("sim", true, &[("core", Normal, false)]),
        ];
        assert!(layering_violations(&packages).is_empty());
        assert_eq!(pure_crates(&packages), ["core", "sim"]);
    }

    #[test]
    fn a_pure_crate_may_not_depend_on_an_impure_workspace_crate() {
        let packages = [
            package("net", false, &[]),
            package("core", true, &[("net", Normal, false)]),
        ];
        assert_eq!(layering_violations(&packages), ["  core -> net"]);
    }

    #[test]
    fn dev_and_build_dependencies_are_not_compiled_for_the_target() {
        let packages = [
            package("validate", false, &[]),
            package(
                "core",
                true,
                &[("validate", Dev, false), ("validate", Build, false)],
            ),
        ];
        assert!(layering_violations(&packages).is_empty());
    }

    #[test]
    fn impure_crates_are_not_constrained() {
        let packages = [
            package("net", false, &[]),
            package("cli", false, &[("net", Normal, false)]),
        ];
        assert!(layering_violations(&packages).is_empty());
    }

    #[test]
    fn optional_dependencies_count_only_when_default_features_enable_them() {
        let net = || package("net", false, &[]);
        let facade = || package("facade", true, &[("net", Normal, true)]);

        let off = [
            net(),
            with_features(facade(), &[("default", &[]), ("net", &["dep:net"])]),
        ];
        assert!(layering_violations(&off).is_empty());

        let direct = [net(), with_features(facade(), &[("default", &["dep:net"])])];
        assert_eq!(layering_violations(&direct), ["  facade -> net"]);

        let chained = [
            net(),
            with_features(
                facade(),
                &[("default", &["online"]), ("online", &["net/cache"])],
            ),
        ];
        assert_eq!(layering_violations(&chained), ["  facade -> net"]);

        let weak = [
            net(),
            with_features(facade(), &[("default", &["net?/cache"])]),
        ];
        assert!(layering_violations(&weak).is_empty());
    }

    #[test]
    fn a_dependency_whose_name_is_a_prefix_is_not_confused() {
        let entries: BTreeSet<String> = ["dep:hpr-netx".to_owned(), "hpr-netx/a".to_owned()].into();
        assert!(!enables_dependency(&entries, "hpr-net"));
        assert!(enables_dependency(&entries, "hpr-netx"));
    }

    /// The pure core named in docs/ARCHITECTURE.md (ADR-001). Changing it needs an ADR.
    #[test]
    fn the_workspace_pure_core_matches_the_architecture() {
        let workspace = workspace::load().unwrap();
        assert_eq!(
            pure_crates(&workspace.packages),
            [
                "hpr",
                "hpr-aero",
                "hpr-analysis",
                "hpr-atmos",
                "hpr-core",
                "hpr-design",
                "hpr-flightdata",
                "hpr-format",
                "hpr-io",
                "hpr-motor",
                "hpr-sim",
                "hpr-wasm",
            ]
        );
        assert_eq!(
            layering_violations(&workspace.packages),
            Vec::<String>::new()
        );
    }
}
