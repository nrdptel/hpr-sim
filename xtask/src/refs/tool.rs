//! Running the external tools the reference library relies on: git, curl, uv, python and java.

use std::ffi::OsStr;
use std::path::Path;
use std::process::{Command, Output};

/// What happened to one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Already in the pinned state; nothing was done.
    Ok(String),
    /// Brought into the pinned state by this run.
    Fetched(String),
    /// Left out on purpose (for example a private repo without access).
    Skipped(String),
    /// Not in the pinned state.
    Failed(String),
}

impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Outcome::Ok(_) => "ok",
            Outcome::Fetched(_) => "fetched",
            Outcome::Skipped(_) => "skipped",
            Outcome::Failed(_) => "FAILED",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Outcome::Ok(detail)
            | Outcome::Fetched(detail)
            | Outcome::Skipped(detail)
            | Outcome::Failed(detail) => detail,
        }
    }
}

/// Counts of outcomes, printed at the end of a run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub ok: usize,
    pub fetched: usize,
    pub skipped: usize,
    pub failed: usize,
}

impl Tally {
    pub fn add(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Ok(_) => self.ok += 1,
            Outcome::Fetched(_) => self.fetched += 1,
            Outcome::Skipped(_) => self.skipped += 1,
            Outcome::Failed(_) => self.failed += 1,
        }
    }
}

/// Prints one aligned result line.
pub fn report(command: &str, name: &str, outcome: &Outcome) {
    println!(
        "refs {command}: {name:<32} {:<8} {}",
        outcome.label(),
        outcome.detail()
    );
}

/// Runs a command to completion, capturing its output. Failing to start it is an error; a
/// non-zero exit is not.
pub fn output(command: &mut Command) -> Result<Output, String> {
    let program = command.get_program().to_string_lossy().into_owned();
    command
        .output()
        .map_err(|err| format!("could not run `{program}` (is it installed?): {err}"))
}

/// Runs a command that must succeed and returns its trimmed stdout.
pub fn stdout(command: &mut Command) -> Result<String, String> {
    let out = output(command)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
    } else {
        Err(failure(command, &out))
    }
}

/// Describes a failed command with its stderr.
pub fn failure(command: &Command, out: &Output) -> String {
    let args: Vec<_> = command.get_args().map(OsStr::to_string_lossy).collect();
    let stderr = String::from_utf8_lossy(&out.stderr);
    format!(
        "`{} {}` failed ({}): {}",
        command.get_program().to_string_lossy(),
        args.join(" "),
        out.status,
        stderr.trim()
    )
}

/// Variables that point git at another repository. Git hooks set them, and they override `-C`.
pub const GIT_REPO_VARS: [&str; 6] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
];

/// A git command in `dir` that never prompts for credentials and ignores any repository the
/// caller's environment points at.
pub fn git(dir: &Path) -> Command {
    let mut command = Command::new("git");
    for var in GIT_REPO_VARS {
        command.env_remove(var);
    }
    command
        .arg("-C")
        .arg(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never");
    command
}

/// The first line a tool prints for `--version` (or the given flag), if it runs.
pub fn version_line(program: &str, flag: &str) -> Option<String> {
    let out = Command::new(program).arg(flag).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines().next().map(|line| line.trim().to_owned())
}

/// Human-readable byte count.
pub fn size(bytes: u64) -> String {
    const MB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / MB)
    } else {
        format!("{:.1} kB", bytes as f64 / 1024.0)
    }
}
