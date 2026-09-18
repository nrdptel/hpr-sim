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
//! 1. A layering rule. No workspace crate outside the pure core may appear in the pure core's
//!    normal dependency graph, on any target. The graph comes from `cargo tree`, so it reflects
//!    the features cargo actually resolves, including features one pure crate turns on in
//!    another.
//! 2. `cargo clippy --target wasm32-unknown-unknown -- -D warnings` on the pure core, with
//!    default features. This compiles the core for the target and fails on any warning,
//!    including warnings that only appear under `cfg(target_arch = "wasm32")`.
//!
//! Compiling for wasm32 does not prove the absence of I/O: `std::fs` and `std::time::Instant`
//! compile there and fail at run time. The `disallowed-methods` and `disallowed-types` lists in
//! `clippy.toml` cover that part.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::workspace::{self, Package, Workspace};

/// The WebAssembly target the pure core must build for.
pub const TARGET: &str = "wasm32-unknown-unknown";

/// Runs the check. `cargo_args` (for example `--locked`) are passed to every cargo command.
pub fn run(cargo_args: &[String]) -> Result<(), String> {
    let workspace = workspace::load(Path::new(env!("CARGO_MANIFEST_DIR")))?;
    let pure = pure_crates(&workspace.packages);
    if pure.is_empty() {
        return Err("no crate declares `[package.metadata.hpr] wasm = true`".to_owned());
    }
    println!(
        "wasm-check: {} pure-core crates: {}",
        pure.len(),
        pure.join(", ")
    );

    check_layering(&workspace, &pure, cargo_args)?;
    println!("wasm-check: layering ok (no workspace crate outside the core in its graph)");

    let mut command = Command::new(workspace::cargo());
    command
        .current_dir(&workspace.root)
        .args(["clippy", "--target", TARGET]);
    for name in &pure {
        command.args(["--package", name]);
    }
    command.args(cargo_args).args(["--", "-D", "warnings"]);
    let status = command
        .status()
        .map_err(|err| format!("could not run `cargo clippy`: {err}"))?;
    if !status.success() {
        return Err(format!(
            "`cargo clippy --target {TARGET}` failed ({status}). If the target is missing, \
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

/// Fails if a workspace crate outside the pure core is in the pure core's resolved normal
/// dependency graph for any target. The error shows how each one is reached.
pub fn check_layering(
    workspace: &Workspace,
    pure: &[String],
    cargo_args: &[String],
) -> Result<(), String> {
    let tree = cargo_tree(&workspace.root, pure, &["--prefix", "none"], cargo_args)?;
    let outside = outside_core(&tree, &workspace.packages);
    if outside.is_empty() {
        return Ok(());
    }
    let mut message = format!(
        "the pure core depends on workspace crates outside it: {}",
        outside.join(", ")
    );
    for name in &outside {
        let paths = cargo_tree(&workspace.root, pure, &["--invert", name], cargo_args)?;
        message.push_str(&format!("\n\n{}", paths.trim_end()));
    }
    Err(message)
}

/// Runs `cargo tree` over the normal dependencies of `packages` for all targets.
fn cargo_tree(
    root: &Path,
    packages: &[String],
    extra: &[&str],
    cargo_args: &[String],
) -> Result<String, String> {
    let mut command = Command::new(workspace::cargo());
    command.current_dir(root).args([
        "tree", "--target", "all", "--edges", "normal", "--format", "{p}",
    ]);
    for name in packages {
        command.args(["--package", name]);
    }
    command.args(extra).args(cargo_args);
    let output = command
        .output()
        .map_err(|err| format!("could not run `cargo tree`: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "`cargo tree` failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|err| format!("`cargo tree` printed invalid UTF-8: {err}"))
}

/// The non-pure workspace crates named in `cargo tree --prefix none --format {p}` output, whose
/// lines look like `hpr-core v0.1.0 (/path/to/crate)`, sorted.
fn outside_core(tree: &str, packages: &[Package]) -> Vec<String> {
    let listed: BTreeSet<&str> = tree
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    packages
        .iter()
        .filter(|package| !package.wasm && listed.contains(package.name.as_str()))
        .map(|package| package.name.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    fn package(name: &str, wasm: bool) -> Package {
        Package {
            name: name.to_owned(),
            wasm,
            examples: Vec::new(),
        }
    }

    #[test]
    fn pure_crates_are_sorted_by_name() {
        let packages = [
            package("sim", true),
            package("net", false),
            package("core", true),
        ];
        assert_eq!(pure_crates(&packages), ["core", "sim"]);
    }

    #[test]
    fn outside_core_reads_package_names_from_tree_output() {
        let packages = [
            package("core", true),
            package("net", false),
            package("cli", false),
        ];
        let tree =
            "core v0.1.0 (/w/core)\nnet v0.1.0 (/w/net)\nglam v0.33.0\n\nnet v0.1.0 (/w/net) (*)\n";
        assert_eq!(outside_core(tree, &packages), ["net"]);
        assert!(outside_core("core v0.1.0 (/w/core)\nnetwork v1.0.0\n", &packages).is_empty());
    }

    /// A throwaway workspace under the system temp directory, removed on drop.
    struct Scratch(PathBuf);

    impl Scratch {
        /// A workspace with a non-pure `net` crate and two pure crates: `facade`, which has
        /// `facade_deps` and `facade_extra` in its manifest, and `bind`, which depends on
        /// `facade` as `bind_dep`.
        fn new(case: &str, facade_deps: &str, facade_extra: &str, bind_dep: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "hpr-xtask-wasm-check-{case}-{}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            let scratch = Self(dir);
            let pure = "[package.metadata.hpr]\nwasm = true\n";
            scratch.write(
                "Cargo.toml",
                "[workspace]\nresolver = \"3\"\nmembers = [\"net\", \"facade\", \"bind\"]\n",
            );
            scratch.write("net/Cargo.toml", &manifest("net", "", ""));
            scratch.write(
                "facade/Cargo.toml",
                &manifest("facade", facade_deps, &format!("{pure}{facade_extra}")),
            );
            scratch.write(
                "bind/Cargo.toml",
                &manifest("bind", &format!("facade = {bind_dep}\n"), pure),
            );
            for krate in ["net", "facade", "bind"] {
                scratch.write(&format!("{krate}/src/lib.rs"), "");
            }
            scratch
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.0.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, contents).unwrap();
        }

        fn layering(&self) -> Result<(), String> {
            let workspace = workspace::load(&self.0).unwrap();
            let pure = pure_crates(&workspace.packages);
            assert_eq!(pure, ["bind", "facade"]);
            check_layering(&workspace, &pure, &["--offline".to_owned()])
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn manifest(name: &str, dependencies: &str, extra: &str) -> String {
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
             publish = false\n\n[dependencies]\n{dependencies}\n{extra}"
        )
    }

    const OPTIONAL_NET: &str = "net = { path = \"../net\", optional = true }\n";
    const NET_FEATURE: &str = "[features]\nnet = [\"dep:net\"]\n";

    fn assert_rejects_net(result: Result<(), String>) {
        let err = result.unwrap_err();
        assert!(
            err.contains("outside it: net") && err.contains("facade"),
            "{err}"
        );
    }

    #[test]
    fn layering_passes_when_the_optional_crate_stays_off() {
        let scratch = Scratch::new("off", OPTIONAL_NET, NET_FEATURE, "{ path = \"../facade\" }");
        assert_eq!(scratch.layering(), Ok(()));
    }

    #[test]
    fn layering_catches_a_feature_enabled_by_another_pure_crate() {
        let scratch = Scratch::new(
            "dependent",
            OPTIONAL_NET,
            NET_FEATURE,
            "{ path = \"../facade\", features = [\"net\"] }",
        );
        assert_rejects_net(scratch.layering());
    }

    #[test]
    fn layering_catches_a_default_feature() {
        let features = "[features]\ndefault = [\"net\"]\nnet = [\"dep:net\"]\n";
        let scratch = Scratch::new(
            "default",
            OPTIONAL_NET,
            features,
            "{ path = \"../facade\" }",
        );
        assert_rejects_net(scratch.layering());
    }

    #[test]
    fn layering_catches_a_renamed_dependency() {
        let deps = "online = { package = \"net\", path = \"../net\" }\n";
        let scratch = Scratch::new("renamed", deps, "", "{ path = \"../facade\" }");
        assert_rejects_net(scratch.layering());
    }

    #[test]
    fn layering_catches_a_dependency_for_another_target() {
        let target = "[target.'cfg(windows)'.dependencies]\nnet = { path = \"../net\" }\n";
        let scratch = Scratch::new("target", "", target, "{ path = \"../facade\" }");
        assert_rejects_net(scratch.layering());
    }

    #[test]
    fn layering_ignores_dev_dependencies() {
        let dev = "[dev-dependencies]\nnet = { path = \"../net\" }\n";
        let scratch = Scratch::new("dev", "", dev, "{ path = \"../facade\" }");
        assert_eq!(scratch.layering(), Ok(()));
    }

    /// The pure core named in docs/ARCHITECTURE.md (ADR-001). Changing it needs an ADR.
    #[test]
    fn the_workspace_pure_core_matches_the_architecture() {
        let workspace = workspace::load(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let pure = pure_crates(&workspace.packages);
        assert_eq!(
            pure,
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
            check_layering(&workspace, &pure, &["--locked".to_owned()]),
            Ok(())
        );
    }
}
