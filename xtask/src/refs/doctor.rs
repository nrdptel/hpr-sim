//! `cargo xtask refs doctor`: which tools, references and oracles are usable on this machine.

use std::path::Path;

use super::download;
use super::git;
use super::java;
use super::lock::{Item, Lock};
use super::python::{self, Import};
use super::tool::{self, Outcome};

/// The lock item holding the OpenRocket jar.
pub const OPENROCKET_JAR: &str = "openrocket-jar";

/// Python modules each oracle imports.
const ROCKETPY_MODULES: &[&str] = &["rocketpy"];
const OPENROCKET_MODULES: &[&str] = &["orhelper", "jpype"];

/// A readiness verdict with its reason.
struct Check {
    ok: bool,
    detail: String,
}

impl Check {
    fn yes(detail: impl Into<String>) -> Check {
        Check {
            ok: true,
            detail: detail.into(),
        }
    }

    fn no(detail: impl Into<String>) -> Check {
        Check {
            ok: false,
            detail: detail.into(),
        }
    }
}

pub fn run(root: &Path, lock: &Lock) -> Result<(), String> {
    let min_java = lock.java.as_ref().map_or(17, |java| java.min_major);
    let java = java::find(min_java);

    println!("Tools");
    let mut rows = vec![row(["tool", "status", "detail"])];
    for (name, flag, purpose) in [
        ("git", "--version", "git references"),
        ("curl", "--version", "downloads"),
        ("uv", "--version", "the Python oracle environment"),
    ] {
        let (status, detail) = match tool::version_line(name, flag) {
            Some(line) => ("ok", line),
            None => ("missing", format!("needed for {purpose}")),
        };
        rows.push(row([name, status, &detail]));
    }
    let (status, detail) = match &java {
        Ok(found) => (
            "ok",
            format!("Java {} at {}", found.version, found.java.display()),
        ),
        Err(Some(old)) => (
            "too old",
            format!(
                "Java {} found; the OpenRocket oracle needs {min_java}+",
                old.version
            ),
        ),
        Err(None) => (
            "missing",
            format!("no Java {min_java}+ runtime found; {}", java_hint()),
        ),
    };
    rows.push(row(["java", status, &detail]));
    print_table(&rows);

    println!(
        "\nReferences ({}; `cargo xtask refs verify` re-hashes git checkouts too)",
        super::lock::LOCK_PATH
    );
    let mut rows = vec![row(["name", "kind", "status", "detail"])];
    for item in lock.items() {
        let (kind, outcome) = match item {
            Item::Git(source) => ("git", git::head(root, source)),
            Item::File(source) => (
                "file",
                download::verify_file(root, &source.dest, &source.sha256),
            ),
            Item::Snapshot(source) => (
                "snapshot",
                download::verify_file(root, &source.dest, &source.sha256),
            ),
        };
        rows.push(row([item.name(), kind, outcome.label(), outcome.detail()]));
    }
    if let Some(python) = &lock.python {
        let outcome = python::verify(root, python);
        rows.push(row(["python", "uv env", outcome.label(), outcome.detail()]));
    }
    print_table(&rows);

    println!("\nOracles");
    let mut rows = vec![row(["oracle", "runnable", "detail"])];
    let rocketpy = rocketpy_check(root, lock);
    rows.push(row([
        "RocketPy (Python)",
        yes_no(rocketpy.ok),
        &rocketpy.detail,
    ]));
    let openrocket = openrocket_check(root, lock, java.ok());
    rows.push(row([
        "OpenRocket (Java)",
        yes_no(openrocket.ok),
        &openrocket.detail,
    ]));
    print_table(&rows);
    println!(
        "  (runnable: the oracle's runtime starts. RocketPy imports at the locked version; the \
         JVM loads the pinned OpenRocket jar. No flight is simulated here; the oracle scripts \
         that later milestones add do that.)"
    );
    Ok(())
}

fn java_hint() -> &'static str {
    if cfg!(target_os = "macos") {
        "install one (`brew install openjdk@21`) or set JAVA_HOME"
    } else {
        "install a JDK 17+ or set JAVA_HOME"
    }
}

/// Imports the modules and compares versions pinned in the lock's `[python] checks`.
fn modules_check(root: &Path, lock: &Lock, modules: &[&str]) -> Check {
    let Some(python) = &lock.python else {
        return Check::no("the lock has no [python] environment");
    };
    let imports = match python::import(root, python, modules) {
        Ok(imports) => imports,
        Err(err) => return Check::no(format!("{err}; run `cargo xtask refs fetch`")),
    };
    let mut found = Vec::new();
    for (module, import) in imports {
        let expected = python
            .checks
            .iter()
            .find(|check| check.module == module)
            .and_then(|check| check.version.as_deref());
        match (import, expected) {
            (Import::Failed(err), _) => return Check::no(format!("import {module}: {err}")),
            (Import::Ok(version), Some(expected)) if version.as_deref() != Some(expected) => {
                return Check::no(format!(
                    "{module} is {}, the lock expects {expected}",
                    version.as_deref().unwrap_or("unversioned")
                ));
            }
            (Import::Ok(Some(version)), _) => found.push(format!("{module} {version}")),
            (Import::Ok(None), _) => found.push(module),
        }
    }
    Check::yes(format!("{}: {}", python.venv, found.join(", ")))
}

fn rocketpy_check(root: &Path, lock: &Lock) -> Check {
    modules_check(root, lock, ROCKETPY_MODULES)
}

fn openrocket_check(root: &Path, lock: &Lock, java: Option<java::Java>) -> Check {
    let mut missing = Vec::new();
    let jar = lock.file.iter().find(|file| file.name == OPENROCKET_JAR);
    match jar {
        Some(jar) => {
            if let Outcome::Failed(err) = download::verify_file(root, &jar.dest, &jar.sha256) {
                missing.push(format!("{} ({err})", jar.dest));
            }
        }
        None => missing.push(format!("the lock has no `{OPENROCKET_JAR}` file")),
    }
    let modules = modules_check(root, lock, OPENROCKET_MODULES);
    if !modules.ok {
        missing.push(modules.detail.clone());
    }
    let min = lock.java.as_ref().map_or(17, |java| java.min_major);
    let Some(java) = java else {
        missing.push(format!("Java {min}+ ({})", java_hint()));
        return Check::no(format!("needs {}", missing.join("; ")));
    };
    if !missing.is_empty() {
        return Check::no(format!("needs {}", missing.join("; ")));
    }
    let (Some(jar), Some(python)) = (jar, &lock.python) else {
        return Check::no("the lock has no jar or no python environment");
    };
    match jvm_smoke(root, python, &root.join(&jar.dest), &java) {
        Ok(main) => Check::yes(format!(
            "Java {}; JVM started and loaded {main} from {}",
            java.version, jar.dest
        )),
        Err(err) => Check::no(format!("the JVM smoke test failed: {err}")),
    }
}

/// Starts the JVM through JPype with the OpenRocket jar on the class path and loads the jar's
/// `Main-Class` (named in its manifest), which proves the Java side of the oracle can run.
fn jvm_smoke(
    root: &Path,
    python: &super::lock::PythonEnv,
    jar: &Path,
    java: &java::Java,
) -> Result<String, String> {
    const SCRIPT: &str = r#"
import sys, zipfile, jpype
with zipfile.ZipFile(sys.argv[1]) as jar:
    manifest = jar.read("META-INF/MANIFEST.MF").decode("utf-8")
main = next(line.split(":", 1)[1].strip() for line in manifest.splitlines()
            if line.startswith("Main-Class:"))
jpype.startJVM(classpath=[sys.argv[1]], convertStrings=False)
jpype.JClass(main)
print("ok", main)
"#;
    let mut command = python::python_command(root, python);
    command.arg("-c").arg(SCRIPT).arg(jar);
    if let Some(home) = &java.home {
        command.env("JAVA_HOME", home);
    }
    let out = tool::stdout(&mut command)?;
    match out.lines().last().and_then(|line| line.strip_prefix("ok ")) {
        Some(main) => Ok(main.to_owned()),
        None => Err(format!("unexpected output: {out}")),
    }
}

fn yes_no(ok: bool) -> &'static str {
    if ok { "yes" } else { "no" }
}

fn row<const N: usize>(cells: [&str; N]) -> Vec<String> {
    cells.iter().map(|cell| (*cell).to_owned()).collect()
}

/// Prints rows as an aligned table; the first row is the header.
fn print_table(rows: &[Vec<String>]) {
    for line in table_lines(rows) {
        println!("{line}");
    }
}

fn table_lines(rows: &[Vec<String>]) -> Vec<String> {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|c| {
            rows.iter()
                .filter_map(|r| r.get(c))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let format_row = |cells: &[String]| {
        let padded: Vec<String> = cells
            .iter()
            .enumerate()
            .map(|(c, cell)| format!("{cell:<width$}", width = widths[c]))
            .collect();
        format!("  {}", padded.join("  ").trim_end())
    };
    let mut lines = Vec::new();
    for (index, cells) in rows.iter().enumerate() {
        lines.push(format_row(cells));
        if index == 0 {
            let rule: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
            lines.push(format!("  {}", rule.join("  ")));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_aligned_under_a_rule() {
        let rows = vec![row(["oracle", "runnable"]), row(["RocketPy", "yes"])];
        assert_eq!(
            table_lines(&rows),
            [
                "  oracle    runnable",
                "  --------  --------",
                "  RocketPy  yes"
            ]
        );
    }
}
