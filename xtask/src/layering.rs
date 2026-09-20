//! Which workspace crates are allowed to reach which others.
//!
//! [`wasm_check`](crate::wasm_check) already keeps the pure core free of crates that do I/O. This
//! is the other kind of layering rule: a crate that has to be usable **on its own** says which
//! crates must never reach it, directly or through anything else.
//!
//! A crate declares the rule in its own manifest, so it is read where the dependency would be
//! added:
//!
//! ```toml
//! [package.metadata.hpr]
//! forbids = ["hpr-sim"]
//! ```
//!
//! `hpr-flightdata` is the case this was written for (ADR-046). Reading a flight log and working
//! out what it says must not require the simulator, because analysing a flight is a use of this
//! project in its own right: someone who flew a rocket and has a log, but no design file and no
//! wish to simulate anything, is a first-class user. Anything that compares a flight with a
//! simulation of it belongs in `hpr-forensics`, which depends on both.
//!
//! The check walks workspace crates only. An outside crate from crates.io is not a layer of this
//! project and is governed by `deny.toml` instead.
//!
//! It is deliberately conservative about *which* dependencies count. Normal, build and
//! target-specific ones all do, and so does an optional one whose feature is off — `cargo
//! metadata` lists it either way — because a crate that has to stand on its own has to do so in
//! every configuration, not just the default one. Dev-dependencies are the exception and are
//! excluded: they never reach someone who depends on the crate, so a test here may fly a
//! simulated flight and compare it with a parsed one.

use std::collections::{BTreeSet, VecDeque};

use crate::workspace::{Package, Workspace};

/// Fails if any crate's `forbids` list is reachable from it.
///
/// The error names the path that reaches the forbidden crate, because the offending dependency is
/// usually not the one that was just added.
pub fn check(workspace: &Workspace) -> Result<(), String> {
    let mut problems = Vec::new();
    for package in &workspace.packages {
        for forbidden in &package.forbids {
            if !workspace.packages.iter().any(|p| &p.name == forbidden) {
                problems.push(format!(
                    "{}: forbids `{forbidden}`, which is not a crate in this workspace",
                    package.name
                ));
                continue;
            }
            if let Some(path) = path_to(&workspace.packages, &package.name, forbidden) {
                problems.push(format!(
                    "{} must not depend on `{forbidden}`, but does: {}",
                    package.name,
                    path.join(" -> ")
                ));
            }
        }
    }
    if problems.is_empty() {
        return Ok(());
    }
    Err(format!(
        "layering: {}\nSee the layering rules in xtask/src/layering.rs.",
        problems.join("\n           ")
    ))
}

/// The shortest dependency path from `from` to `to` through workspace crates, `from` and `to`
/// included, or `None` when `to` is not reachable. Breadth-first, so the path it reports is the
/// shortest one and the walk terminates on a dependency cycle.
fn path_to(packages: &[Package], from: &str, to: &str) -> Option<Vec<String>> {
    let dependencies = |name: &str| -> &[String] {
        packages
            .iter()
            .find(|package| package.name == name)
            .map_or(&[][..], |package| &package.dependencies)
    };
    let mut seen: BTreeSet<&str> = BTreeSet::from([from]);
    let mut queue: VecDeque<Vec<String>> = VecDeque::from([vec![from.to_owned()]]);
    while let Some(path) = queue.pop_front() {
        let tail = path.last().map(String::as_str).unwrap_or(from);
        for dependency in dependencies(tail) {
            // Only workspace crates are layers of this project; anything else is a crates.io
            // dependency and `cargo deny` governs it.
            if !packages.iter().any(|package| &package.name == dependency) {
                continue;
            }
            let mut next = path.clone();
            next.push(dependency.clone());
            if dependency == to {
                return Some(next);
            }
            if seen.insert(dependency) {
                queue.push_back(next);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::workspace;

    fn package(name: &str, dependencies: &[&str], forbids: &[&str]) -> Package {
        Package {
            name: name.to_owned(),
            wasm: true,
            forbids: forbids.iter().map(|name| (*name).to_owned()).collect(),
            lib: Some(name.replace('-', "_")),
            dependencies: dependencies.iter().map(|name| (*name).to_owned()).collect(),
            examples: Vec::new(),
        }
    }

    fn workspace_of(packages: Vec<Package>) -> Workspace {
        Workspace {
            root: PathBuf::from("/nowhere"),
            packages,
        }
    }

    #[test]
    fn a_crate_with_no_rule_may_depend_on_anything() {
        let workspace = workspace_of(vec![
            package("logs", &["sim"], &[]),
            package("sim", &[], &[]),
        ]);
        assert!(check(&workspace).is_ok());
    }

    #[test]
    fn a_direct_forbidden_dependency_is_caught() {
        let workspace = workspace_of(vec![
            package("logs", &["sim"], &["sim"]),
            package("sim", &[], &[]),
        ]);
        let err = check(&workspace).unwrap_err();
        assert!(err.contains("logs must not depend on `sim`"), "{err}");
        assert!(err.contains("logs -> sim"), "{err}");
    }

    #[test]
    fn a_forbidden_dependency_reached_through_another_crate_is_caught() {
        // The point of the check: nobody adds `sim` to `logs` directly. It arrives through a
        // helper crate that looked harmless.
        let workspace = workspace_of(vec![
            package("logs", &["shared"], &["sim"]),
            package("shared", &["sim"], &[]),
            package("sim", &[], &[]),
        ]);
        let err = check(&workspace).unwrap_err();
        assert!(err.contains("logs -> shared -> sim"), "{err}");
    }

    #[test]
    fn the_shortest_path_is_reported() {
        let workspace = workspace_of(vec![
            package("logs", &["long", "sim"], &["sim"]),
            package("long", &["longer"], &[]),
            package("longer", &["sim"], &[]),
            package("sim", &[], &[]),
        ]);
        let err = check(&workspace).unwrap_err();
        assert!(err.contains("logs -> sim"), "{err}");
        assert!(!err.contains("longer"), "{err}");
    }

    #[test]
    fn a_dependency_cycle_does_not_hang_the_walk() {
        let workspace = workspace_of(vec![
            package("logs", &["a"], &["sim"]),
            package("a", &["b"], &[]),
            package("b", &["a"], &[]),
            package("sim", &[], &[]),
        ]);
        assert!(check(&workspace).is_ok());
    }

    #[test]
    fn an_outside_crate_of_the_same_name_is_not_a_layer() {
        // `serde` is not a workspace member, so a rule naming it is a mistake worth reporting
        // rather than a silent pass.
        let workspace = workspace_of(vec![package("logs", &["serde"], &["serde"])]);
        let err = check(&workspace).unwrap_err();
        assert!(err.contains("not a crate in this workspace"), "{err}");
    }

    /// The rule that ADR-046 is about, checked against the real workspace rather than a fixture,
    /// so that deleting the manifest line fails here.
    #[test]
    fn the_real_flight_data_crate_stands_on_its_own() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let workspace = workspace::load(root).unwrap();
        let flightdata = workspace
            .packages
            .iter()
            .find(|package| package.name == "hpr-flightdata")
            .expect("hpr-flightdata is a workspace member");
        assert!(
            flightdata.forbids.iter().any(|name| name == "hpr-sim"),
            "hpr-flightdata must keep `forbids = [\"hpr-sim\"]` (ADR-046): reading a flight log \
             does not require the simulator"
        );
        check(&workspace).unwrap();
    }
}
