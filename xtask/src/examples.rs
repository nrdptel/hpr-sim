//! The workspace's example programs (M0.4c). A page of the documentation site can show what an
//! example prints, so what it prints is committed beside it as `<name>.output.txt`, and a page
//! quotes that file (the site's quote check, `site.rs`).
//!
//! `cargo xtask examples` runs every example and writes those files. `--check` writes nothing and
//! fails if an example fails, has no committed output, or prints anything else. CI runs `--check`
//! on macOS, Windows and Linux, which also shows that the output is the same on each.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::workspace::{self, Example};

pub const USAGE: &str = "  examples [--check] [cargo args]
                           Run every example program and write what it prints beside it, as
                           <name>.output.txt. --check writes nothing, and fails if an example
                           fails or prints anything else. Other arguments (for example
                           --locked) are passed on to `cargo run`.";

/// What an example's output file is called, after the name of its main source file.
const OUTPUT_SUFFIX: &str = ".output.txt";

pub fn run(args: &[String]) -> Result<(), String> {
    let check = args.iter().any(|arg| arg == "--check");
    let cargo_args: Vec<&String> = args.iter().filter(|arg| *arg != "--check").collect();
    let workspace = workspace::load(Path::new(env!("CARGO_MANIFEST_DIR")))?;
    let mut problems = Vec::new();
    let mut count = 0;
    for package in &workspace.packages {
        for example in &package.examples {
            count += 1;
            let path = output_path(&example.src_path);
            let shown = path
                .strip_prefix(&workspace.root)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            let printed = match run_example(&workspace.root, &package.name, example, &cargo_args) {
                Ok(printed) => printed,
                Err(why) => {
                    problems.push(why);
                    continue;
                }
            };
            if check {
                match fs::read_to_string(&path) {
                    Ok(committed) => {
                        if let Some(why) = difference(&committed, &printed) {
                            problems.push(format!(
                                "{shown}: {why}; if the change is meant, run `cargo xtask \
                                 examples` to write it, and update the pages that quote it"
                            ));
                        }
                    }
                    Err(err) => problems.push(format!(
                        "{shown}: can't read it ({err}); run `cargo xtask examples` to write it"
                    )),
                }
            } else {
                fs::write(&path, &printed)
                    .map_err(|err| format!("could not write {shown}: {err}"))?;
                println!("examples: wrote {shown}");
            }
        }
    }
    if count == 0 {
        return Err("no example programs found in the workspace".to_owned());
    }
    if !problems.is_empty() {
        return Err(format!(
            "{} problem(s) with the examples:\n  {}",
            problems.len(),
            problems.join("\n  ")
        ));
    }
    if check {
        println!("examples: {count} example(s) ran and printed what their committed output says");
    }
    Ok(())
}

/// Where an example's output is committed: beside its main source file, `fly.rs` giving
/// `fly.output.txt`.
fn output_path(src_path: &Path) -> PathBuf {
    let stem = src_path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    src_path.with_file_name(format!("{stem}{OUTPUT_SUFFIX}"))
}

/// Runs an example with `cargo run` and returns what it printed on standard output.
fn run_example(
    root: &Path,
    package: &str,
    example: &Example,
    cargo_args: &[&String],
) -> Result<String, String> {
    let output = Command::new(workspace::cargo())
        .current_dir(root)
        .args([
            "run",
            "--quiet",
            "--package",
            package,
            "--example",
            &example.name,
        ])
        .args(cargo_args)
        .output()
        .map_err(|err| {
            format!(
                "could not run `cargo run --example {}`: {err}",
                example.name
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "`cargo run --package {package} --example {}` failed ({}):\n{}",
            example.name,
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|err| format!("example {} printed invalid UTF-8: {err}", example.name))
}

/// How what an example printed differs from its committed output, if it does (a `\r\n` reads as
/// `\n`).
fn difference(committed: &str, printed: &str) -> Option<String> {
    let (committed, printed) = (
        committed.replace("\r\n", "\n"),
        printed.replace("\r\n", "\n"),
    );
    if committed == printed {
        return None;
    }
    let (ours, theirs): (Vec<&str>, Vec<&str>) =
        (committed.lines().collect(), printed.lines().collect());
    let Some(n) = (0..ours.len().max(theirs.len())).find(|&n| ours.get(n) != theirs.get(n)) else {
        return Some("the example's output differs from it only in its final newline".to_owned());
    };
    let shown = |line: Option<&&str>| line.map_or("nothing".to_owned(), |line| format!("`{line}`"));
    Some(format!(
        "the example's output differs from it at line {}: the file has {}, the example printed {}",
        n + 1,
        shown(ours.get(n)),
        shown(theirs.get(n)),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_output_sits_beside_the_source() {
        assert_eq!(
            output_path(Path::new("/w/crates/a/examples/fly.rs")),
            PathBuf::from("/w/crates/a/examples/fly.output.txt")
        );
        assert_eq!(
            output_path(Path::new("/w/crates/a/examples/fly/main.rs")),
            PathBuf::from("/w/crates/a/examples/fly/main.output.txt")
        );
    }

    #[test]
    fn differences_name_the_first_line_that_differs() {
        assert_eq!(difference("a\nb\n", "a\nb\n"), None);
        assert_eq!(difference("a\r\nb\r\n", "a\nb\n"), None);
        assert_eq!(
            difference("a\nb\n", "a\nc\n").unwrap(),
            "the example's output differs from it at line 2: the file has `b`, the example \
             printed `c`"
        );
        assert_eq!(
            difference("a\n", "a\nb\n").unwrap(),
            "the example's output differs from it at line 2: the file has nothing, the example \
             printed `b`"
        );
        assert_eq!(
            difference("a", "a\n").unwrap(),
            "the example's output differs from it only in its final newline"
        );
    }

    #[test]
    fn every_example_has_its_output_committed() {
        let workspace = workspace::load(Path::new(env!("CARGO_MANIFEST_DIR"))).unwrap();
        let examples: Vec<&Example> = workspace
            .packages
            .iter()
            .flat_map(|package| &package.examples)
            .collect();
        assert!(
            examples
                .iter()
                .any(|example| example.name == "first_flight"),
            "{examples:?}"
        );
        for example in examples {
            let path = output_path(&example.src_path);
            assert!(
                path.is_file(),
                "{} has no committed output; run `cargo xtask examples`",
                example.name
            );
        }
    }
}
