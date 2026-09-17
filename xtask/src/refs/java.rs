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
}

impl Java {
    /// The runtime's home directory (`<home>/bin/java`), for `JAVA_HOME`.
    pub fn home(&self) -> Option<&Path> {
        self.java.parent()?.parent()
    }
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
    // `java_home` above already covers the registered runtimes.
    if !macos {
        out.push(PathBuf::from(exe));
    }
    out
}

fn probe(java: &Path) -> Option<Java> {
    let output = Command::new(java).arg("-version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    // `java -version` prints to stderr.
    let text = String::from_utf8_lossy(&output.stderr);
    let (version, major) = parse_version(&text)?;
    // Resolve symlinks (Homebrew's `opt/openjdk@21/bin/java` points into the real JDK) so that
    // `home` is a directory JPype can load the JVM from. Windows paths stay as given, since
    // canonical `\\?\` paths confuse other tools.
    let resolved = if cfg!(windows) {
        None
    } else {
        java.canonicalize().ok()
    };
    Some(Java {
        java: resolved.unwrap_or_else(|| java.to_path_buf()),
        major,
        version,
    })
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
    fn home_is_two_levels_above_the_executable() {
        let java = Java {
            java: PathBuf::from("/jdk/bin/java"),
            major: 21,
            version: "21".to_owned(),
        };
        assert_eq!(java.home(), Some(Path::new("/jdk")));
    }
}
