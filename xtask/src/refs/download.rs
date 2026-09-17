//! Downloads pinned by sha256: immutable files and dated snapshots of live APIs.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::hash;
use super::lock::{FileSource, SnapshotSource};
use super::tool::{self, Outcome};

/// What to do when a snapshot download no longer matches its pinned capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drift {
    /// Keep the new capture beside the destination as `<dest>.unpinned` and fail.
    Fail,
    /// Move the new capture into place and repin it in the lock file.
    Adopt,
}

/// A snapshot whose live API returned different bytes than the pinned capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repin {
    pub name: String,
    pub sha256: String,
}

pub fn fetch_file(root: &Path, source: &FileSource) -> Outcome {
    let dest = root.join(&source.dest);
    if let Some(outcome) = already_pinned(&dest, &source.sha256) {
        return outcome;
    }
    let part = sibling(&dest, "part");
    if let Err(err) = download(&source.url, &part) {
        return Outcome::Failed(err);
    }
    match hash::sha256_file(&part) {
        Ok(sha256) if sha256 == source.sha256 => place(&part, &dest, "downloaded"),
        Ok(sha256) => {
            let _ = std::fs::remove_file(&part);
            Outcome::Failed(format!(
                "downloaded sha256 {sha256} does not match the pinned {}",
                source.sha256
            ))
        }
        Err(err) => Outcome::Failed(err),
    }
}

/// Fetches a snapshot. The outcome comes with a repin request when a new capture was adopted.
pub fn fetch_snapshot(
    root: &Path,
    source: &SnapshotSource,
    drift: Drift,
) -> (Outcome, Option<Repin>) {
    let dest = root.join(&source.dest);
    let unpinned = sibling(&dest, "unpinned");
    if let Some(outcome) = already_pinned(&dest, &source.sha256) {
        let _ = std::fs::remove_file(&unpinned);
        return (outcome, None);
    }
    // Adopting takes the capture an earlier run kept aside, if there is one, so the capture that
    // gets pinned is the one the user could inspect.
    let kept = drift == Drift::Adopt && unpinned.is_file();
    let verb = if kept {
        "moved into place"
    } else {
        "downloaded"
    };
    let part = if kept {
        unpinned.clone()
    } else {
        let part = sibling(&dest, "part");
        if let Err(err) = download(&source.url, &part) {
            return (Outcome::Failed(err), None);
        }
        part
    };
    let sha256 = match hash::sha256_file(&part) {
        Ok(sha256) => sha256,
        Err(err) => return (Outcome::Failed(err), None),
    };
    if sha256 == source.sha256 {
        let outcome = place(&part, &dest, verb);
        let _ = std::fs::remove_file(&unpinned);
        return (outcome, None);
    }
    match drift {
        Drift::Adopt => {
            let outcome = match place(&part, &dest, verb) {
                Outcome::Fetched(detail) => Outcome::Fetched(format!(
                    "{detail}; {} adopted and repinned",
                    if kept {
                        "the capture kept by an earlier run"
                    } else {
                        "the new capture"
                    }
                )),
                other => return (other, None),
            };
            let _ = std::fs::remove_file(&unpinned);
            let repin = Repin {
                name: source.name.clone(),
                sha256,
            };
            (outcome, Some(repin))
        }
        Drift::Fail => {
            let kept = match std::fs::rename(&part, &unpinned) {
                Ok(()) => format!("The new capture is at {}. ", unpinned.display()),
                Err(_) => String::new(),
            };
            let message = format!(
                "the API now returns sha256 {sha256}, not the capture pinned on {} ({}). {kept}\
                 Rerun with --adopt-snapshots to adopt it.",
                source.captured, source.sha256
            );
            (Outcome::Failed(message), None)
        }
    }
}

/// `Some(Ok)` if the destination already holds the pinned bytes. A file with other bytes is
/// downloaded again.
fn already_pinned(dest: &Path, pin: &str) -> Option<Outcome> {
    if !dest.is_file() {
        return None;
    }
    match hash::sha256_file(dest) {
        Ok(sha256) if sha256 == pin => Some(Outcome::Ok("present, sha256 matches".to_owned())),
        _ => None,
    }
}

pub fn verify_file(root: &Path, dest: &str, pin: &str) -> Outcome {
    let path = root.join(dest);
    if !path.is_file() {
        return Outcome::Failed("missing; run `cargo xtask refs fetch`".to_owned());
    }
    match hash::sha256_file(&path) {
        Ok(sha256) if sha256 == pin => Outcome::Ok("sha256 matches".to_owned()),
        Ok(sha256) => Outcome::Failed(format!("sha256 is {sha256}, but the lock pins {pin}")),
        Err(err) => Outcome::Failed(err),
    }
}

fn sibling(dest: &Path, suffix: &str) -> PathBuf {
    let mut name = dest.file_name().unwrap_or_default().to_os_string();
    name.push(".");
    name.push(suffix);
    dest.with_file_name(name)
}

fn place(part: &Path, dest: &Path, verb: &str) -> Outcome {
    let bytes = std::fs::metadata(part).map(|meta| meta.len()).unwrap_or(0);
    match std::fs::rename(part, dest) {
        Ok(()) => Outcome::Fetched(format!("{verb} {}, sha256 matches", tool::size(bytes))),
        Err(err) => Outcome::Failed(format!("could not move the download into place: {err}")),
    }
}

/// Downloads `url` to `out` with curl (on every OS CI covers), creating parent directories.
/// Only `https://` URLs, and `file://` URLs for tests, are accepted, including after redirects.
pub fn download(url: &str, out: &Path) -> Result<(), String> {
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("could not create {}: {err}", parent.display()))?;
    }
    let mut curl = Command::new("curl");
    curl.args([
        "--fail",
        "--location",
        "--silent",
        "--show-error",
        "--retry",
        "3",
        "--connect-timeout",
        "30",
        // Give up on a transfer that stalls below 1 kB/s for a minute instead of hanging.
        "--speed-limit",
        "1024",
        "--speed-time",
        "60",
        "--proto",
        "=https,file",
        "--proto-redir",
        "=https",
        "--user-agent",
        "hpr-sim-refs (+https://github.com/nrdptel/hpr-sim)",
        "--output",
    ]);
    curl.arg(out).arg(url);
    let result = tool::output(&mut curl)?;
    if result.status.success() {
        Ok(())
    } else {
        let _ = std::fs::remove_file(out);
        Err(tool::failure(&curl, &result))
    }
}

/// Rewrites the lock-file text so the named snapshot pins `sha256`, captured on `date`. Only the
/// `sha256` and `captured` lines inside that `[[snapshot]]` table change; comments and layout
/// stay as they are.
pub fn repin(text: &str, repin: &Repin, date: &str) -> Result<String, String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let name_line = format!("name = \"{}\"", repin.name);
    let mut start = None;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            start = (trimmed == "[[snapshot]]").then_some(index);
        } else if trimmed == name_line
            && let Some(first) = start
        {
            let end = lines[index..]
                .iter()
                .position(|line| line.trim().starts_with('['))
                .map_or(lines.len(), |offset| index + offset);
            let mut out = String::with_capacity(text.len());
            let mut replaced = (false, false);
            for (i, line) in lines.iter().enumerate() {
                let key = line.split('=').next().unwrap_or_default().trim();
                let newline = if line.ends_with("\r\n") { "\r\n" } else { "\n" };
                if (first..end).contains(&i) && key == "sha256" {
                    out.push_str(&format!("sha256 = \"{}\"{newline}", repin.sha256));
                    replaced.0 = true;
                } else if (first..end).contains(&i) && key == "captured" {
                    out.push_str(&format!("captured = \"{date}\"{newline}"));
                    replaced.1 = true;
                } else {
                    out.push_str(line);
                }
            }
            return if replaced == (true, true) {
                Ok(out)
            } else {
                Err(format!(
                    "the [[snapshot]] table for `{}` needs its own `sha256 = ` and `captured = ` \
                     lines",
                    repin.name
                ))
            };
        }
    }
    Err(format!("no [[snapshot]] table named `{}`", repin.name))
}

/// Today's UTC date as `YYYY-MM-DD`.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    civil_date(secs / 86_400)
}

/// The proleptic Gregorian date of a day count since 1970-01-01 (H. Hinnant, "chrono-Compatible
/// Low-Level Date Algorithms", `civil_from_days`).
fn civil_date(days: u64) -> String {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCK: &str = "version = 1\n\n\
        [[snapshot]]\n\
        name = \"a\"\n\
        # the pinned capture\n\
        sha256 = \"1111\"\n\
        captured = \"2026-01-01\"\n\n\
        [[snapshot]]\n\
        name = \"b\"\n\
        sha256 = \"2222\"\n\
        captured = \"2026-01-02\"\n\n\
        [python]\n\
        project = \"x\"\n";

    #[test]
    fn repins_only_the_named_snapshot() {
        let repin_b = Repin {
            name: "b".into(),
            sha256: "abcd".into(),
        };
        let out = repin(LOCK, &repin_b, "2026-09-17").unwrap();
        let expected = LOCK
            .replace("sha256 = \"2222\"", "sha256 = \"abcd\"")
            .replace("captured = \"2026-01-02\"", "captured = \"2026-09-17\"");
        assert_eq!(out, expected);
    }

    #[test]
    fn repin_fails_for_an_unknown_snapshot() {
        let repin_c = Repin {
            name: "c".into(),
            sha256: "abcd".into(),
        };
        assert!(
            repin(LOCK, &repin_c, "2026-09-17")
                .unwrap_err()
                .contains("no [[snapshot]]")
        );
    }

    #[test]
    fn civil_dates_match_known_days() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(11_016), "2000-02-29");
        assert_eq!(civil_date(20_713), "2026-09-17");
    }
}
