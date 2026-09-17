//! Finding a Java runtime for the OpenRocket oracle.

use std::path::{Path, PathBuf};
use std::process::Command;

/// A Java runtime that ran `java -version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java {
    /// The `java` executable.
    pub java: PathBuf,
    /// The feature release (the `21` in `21.0.5`; the `8` in `1.8.0_402`).
    pub major: u32,
    /// The version string `java -version` printed.
    pub version: String,
    /// The runtime's own `java.home` property, for `JAVA_HOME`. Asking the runtime is reliable
    /// where the executable is a shim or a symlink (Homebrew, asdf, Windows `javapath`).
    pub home: Option<PathBuf>,
}

/// Finds the first runtime with at least `min_major`, looking in `JAVA_HOME`, then (on macOS)
/// `/usr/libexec/java_home`, then Homebrew's keg-only `openjdk` formulae, then `PATH`.
/// Returns the best runtime found when none is new enough.
pub fn find(min_major: u32) -> Result<Java, Option<Java>> {
    let mut best: Option<Java> = None;
    for candidate in candidates(min_major) {
        let Some(java) = probe(&candidate) else {
            continue;
        };
        if java.major >= min_major {
            return Ok(java);
        }
        if best.as_ref().is_none_or(|b| java.major > b.major) {
            best = Some(java);
        }
    }
    Err(best)
}

fn candidates(min_major: u32) -> Vec<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        out.push(PathBuf::from(home).join("bin").join(exe));
    }
    let java_home = Path::new("/usr/libexec/java_home");
    let macos = cfg!(target_os = "macos") && java_home.exists();
    if macos
        && let Ok(output) = Command::new(java_home)
            .args(["-v", &format!("{min_major}+")])
            .output()
        && output.status.success()
    {
        let home = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        out.push(PathBuf::from(home).join("bin").join(exe));
    }
    for prefix in [
        "/opt/homebrew/opt",
        "/usr/local/opt",
        "/home/linuxbrew/.linuxbrew/opt",
    ] {
        let Ok(entries) = std::fs::read_dir(prefix) else {
            continue;
        };
        let mut kegs: Vec<_> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("openjdk"))
            })
            .collect();
        kegs.sort();
        out.extend(kegs.into_iter().rev().map(|keg| keg.join("bin").join(exe)));
    }
    // On macOS, /usr/bin/java is a stub that offers to install Java when none is registered;
    // `java_home` above already covers the registered runtimes. Elsewhere, search PATH for full
    // paths, so the runtime's home (and so JAVA_HOME for JPype) is known.
    if !macos && let Some(path) = std::env::var_os("PATH") {
        out.extend(
            std::env::split_paths(&path)
                .map(|dir| dir.join(exe))
                .filter(|java| java.is_file()),
        );
    }
    out
}

fn probe(java: &Path) -> Option<Java> {
    let output = Command::new(java)
        .args(["-XshowSettings:properties", "-version"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    // Both the properties and the version go to stderr.
    let text = String::from_utf8_lossy(&output.stderr);
    let (version, major) = parse_version(&text)?;
    Some(Java {
        java: java.to_path_buf(),
        major,
        version,
        // A home path with characters outside the console's code page can come back mangled;
        // then fall back to the directory above the (resolved) executable's `bin`.
        home: parse_home(&text)
            .filter(|home| home.is_dir())
            .or_else(|| executable_home(java)),
    })
}

fn executable_home(java: &Path) -> Option<PathBuf> {
    // Windows paths stay as given: canonical `\\?\` paths confuse other tools.
    let resolved = if cfg!(windows) {
        java.to_path_buf()
    } else {
        java.canonicalize().ok()?
    };
    Some(resolved.parent()?.parent()?.to_path_buf())
}

/// The `java.home = <dir>` line of `-XshowSettings:properties`.
pub fn parse_home(text: &str) -> Option<PathBuf> {
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("java.home"))
        .find_map(|rest| rest.trim_start().strip_prefix('='))
        .map(|home| PathBuf::from(home.trim()))
        .filter(|home| !home.as_os_str().is_empty())
}

/// Parses the first line of `java -version`, for example `openjdk version "21.0.5" 2024-10-15`.
pub fn parse_version(text: &str) -> Option<(String, u32)> {
    let line = text.lines().find(|line| line.contains(" version \""))?;
    let version = line.split('"').nth(1)?.to_owned();
    let mut parts = version.split(['.', '_', '-', '+']);
    let first: u32 = parts.next()?.parse().ok()?;
    let major = if first == 1 {
        parts.next()?.parse().ok()?
    } else {
        first
    };
    Some((version, major))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modern_and_legacy_version_lines() {
        let modern = "openjdk version \"21.0.5\" 2024-10-15\nOpenJDK Runtime Environment";
        assert_eq!(parse_version(modern), Some(("21.0.5".to_owned(), 21)));
        let legacy = "java version \"1.8.0_402\"\nJava(TM) SE Runtime Environment";
        assert_eq!(parse_version(legacy), Some(("1.8.0_402".to_owned(), 8)));
        let early_access = "openjdk version \"26-ea\" 2026-03-17";
        assert_eq!(parse_version(early_access), Some(("26-ea".to_owned(), 26)));
        let picked_up = "Picked up JAVA_TOOL_OPTIONS: -Xmx1g\nopenjdk version \"17\" 2021-09-14";
        assert_eq!(parse_version(picked_up), Some(("17".to_owned(), 17)));
    }

    #[test]
    fn rejects_output_without_a_version() {
        assert_eq!(parse_version("Unable to locate a Java Runtime."), None);
        assert_eq!(parse_version("openjdk version \"x.y\""), None);
    }

    #[test]
    fn home_comes_from_the_java_home_property() {
        let text = "Property settings:\n    java.class.path = \n    java.home = /opt/jdk-21/Contents/Home\n    java.version = 21.0.5\n\nopenjdk version \"21.0.5\" 2024-10-15\n";
        assert_eq!(
            parse_home(text),
            Some(PathBuf::from("/opt/jdk-21/Contents/Home"))
        );
        assert_eq!(parse_version(text), Some(("21.0.5".to_owned(), 21)));
        assert_eq!(parse_home("java.homeless = x\njava.home =\n"), None);
    }
}
