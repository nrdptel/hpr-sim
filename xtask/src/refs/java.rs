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

/// Finds a runtime whose feature release is at least `min_major` and, when `max_major` is given,
/// no newer than that. Looks in `JAVA_HOME`, then (on macOS) `/usr/libexec/java_home`, then
/// Homebrew's keg-only `openjdk` formulae, then `PATH`. Returns the closest runtime found when
/// none is acceptable, so the caller can say what is installed.
///
/// The upper bound matters: OpenRocket 24.12 refuses Java 21. With a floor alone, `java_home`
/// answers with the newest runtime, so a machine with both 17 and 21 installed hands back 21 and
/// the jar then rejects it. Asking `java_home` for the exact release does not fix that on its own
/// (it answered `-v 17` with 21 when only 21 was registered); rejecting out-of-range candidates
/// and carrying on to the Homebrew kegs is what finds the keg-only 17.
pub fn find(min_major: u32, max_major: Option<u32>) -> Result<Java, Option<Java>> {
    let probed = candidates(min_major, max_major)
        .into_iter()
        .filter_map(|candidate| probe(&candidate));
    choose(probed, min_major, max_major)
}

/// Picks the first acceptable runtime, or the nearest miss. Separate from `find` so it can be
/// tested without a JDK installed: the floor-only version of this choice is what let a machine
/// with Java 17 and 21 hand the OpenRocket oracle the 21 that the jar refuses.
fn choose(
    runtimes: impl Iterator<Item = Java>,
    min_major: u32,
    max_major: Option<u32>,
) -> Result<Java, Option<Java>> {
    let accepts = |major: u32| major >= min_major && max_major.is_none_or(|max| major <= max);
    let mut best: Option<Java> = None;
    for java in runtimes {
        if accepts(java.major) {
            return Ok(java);
        }
        // Keep the nearest miss, preferring a too-old runtime over a too-new one: "install a
        // newer JDK" is more useful to report than "uninstall the one you have".
        let too_old = java.major < min_major;
        let better = match &best {
            None => true,
            Some(b) => match (too_old, b.major < min_major) {
                (true, false) => true,
                (false, true) => false,
                (true, true) => java.major > b.major, // the newest of the too-old
                (false, false) => java.major < b.major, // the closest above the range
            },
        };
        if better {
            best = Some(java);
        }
    }
    Err(best)
}

/// The version argument for `/usr/libexec/java_home`. `-v 17+` always answers with the newest
/// installed runtime, so a pinned release is asked for without the `+`. That is a hint, not a
/// guarantee: measured on macOS 27 with only Java 21 registered, `-v 17` still answered with 21.
/// The range check in `choose` is what actually keeps an unusable runtime out.
fn java_home_version(min_major: u32, max_major: Option<u32>) -> String {
    match max_major {
        Some(max) if max == min_major => format!("{min_major}"),
        _ => format!("{min_major}+"),
    }
}

fn candidates(min_major: u32, max_major: Option<u32>) -> Vec<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let mut out = Vec::new();
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        out.push(PathBuf::from(home).join("bin").join(exe));
    }
    let java_home = Path::new("/usr/libexec/java_home");
    let macos = cfg!(target_os = "macos") && java_home.exists();
    if macos
        && let Ok(output) = Command::new(java_home)
            .args(["-v", &java_home_version(min_major, max_major)])
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
    let bin = resolved.parent()?;
    let home = bin.parent()?;
    // Only a real runtime layout counts; a shim directory (Windows `javapath`, say) does not.
    let exe = resolved.file_name()?;
    (bin.file_name()? == "bin" && home.join("bin").join(exe).is_file()).then(|| home.to_path_buf())
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
    /// A runtime with only the fields the choice looks at.
    fn jdk(major: u32) -> Java {
        Java {
            java: PathBuf::from(format!("/jdk{major}/bin/java")),
            major,
            version: format!("openjdk version \"{major}.0.1\""),
            home: None,
        }
    }

    #[test]
    fn an_upper_bound_skips_a_runtime_the_oracle_would_refuse() {
        // The real ordering on a Mac with both installed: java_home answers first, with 21.
        let found = choose([jdk(21), jdk(17)].into_iter(), 17, Some(17)).unwrap();
        assert_eq!(found.major, 17, "should skip 21 and take the keg-only 17");
    }

    #[test]
    fn without_an_upper_bound_the_first_new_enough_runtime_wins() {
        let found = choose([jdk(21), jdk(17)].into_iter(), 17, None).unwrap();
        assert_eq!(found.major, 21);
    }

    #[test]
    fn nothing_acceptable_reports_the_nearest_miss() {
        // Too new: report what is installed, so the message can say 21 was found.
        let miss = choose([jdk(21)].into_iter(), 17, Some(17)).unwrap_err();
        assert_eq!(miss.unwrap().major, 21);
        // Too old: the newest of the old ones is the most useful to name.
        let miss = choose([jdk(8), jdk(11)].into_iter(), 17, Some(17)).unwrap_err();
        assert_eq!(miss.unwrap().major, 11);
        // A too-old runtime is reported in preference to a too-new one.
        let miss = choose([jdk(21), jdk(11)].into_iter(), 17, Some(17)).unwrap_err();
        assert_eq!(miss.unwrap().major, 11);
        assert!(
            choose(std::iter::empty(), 17, Some(17))
                .unwrap_err()
                .is_none()
        );
    }

    #[test]
    fn a_pinned_release_is_asked_for_without_the_plus() {
        // A hint only: macOS answered `-v 17` with 21 when just 21 was registered, so the range
        // check is what really protects the choice.
        assert_eq!(java_home_version(17, Some(17)), "17");
        assert_eq!(java_home_version(17, None), "17+");
        assert_eq!(java_home_version(17, Some(21)), "17+");
    }

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
