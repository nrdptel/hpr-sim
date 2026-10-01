//! The `hpr` command-line tool: its commands, the types their `--json` output takes, and the
//! registry the README's command table is generated from.
//!
//! **Guide:** [The command line][guide-cli] says what each command does, shows its output, and
//! lists the exit codes. This crate is the tool's own code; programs should use the [`hpr`]
//! library instead, which this crate calls.
//!
//! [guide-cli]: https://nrdptel.github.io/hpr-sim/cli.html
//!
//! [`run`] is the whole tool: the `hpr` binary is a `main` that hands it the process's arguments
//! and exits with the [`Exit`] it returns. Tests and `cargo xtask cli` call it the same way, so the
//! output they check is the output a user sees.
//!
//! Every command prints text for a person, or, with `--json`, exactly one JSON document on
//! standard output: the command's own output type ([`output`]) when it succeeds, and an
//! [`output::ErrorDocument`] when it doesn't. [`schemas`] generates the published JSON Schema
//! of each, which `cargo xtask cli` writes to `schema/cli/`.
//!
//! Commands whose milestone hasn't come yet are registered with the rest, so that `hpr --help`,
//! the completions and the README list them, and they refuse with [`Exit::NotAvailable`] and the
//! milestone that brings them ([`registry::availability`]).

#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the command-line tool reads files; it is not part of the pure core"
)]

pub mod analyze;
pub mod convert;
mod convert_design;
pub mod motor_search;
pub mod motors;
pub mod output;
pub mod registry;
pub mod sim;
pub mod validate;
pub mod weather;

use std::ffi::OsString;
use std::io::{self, Write};

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use serde::Serialize;

use crate::output::{Completions, ErrorDocument, ErrorKind};
use crate::registry::Availability;

/// How a run of `hpr` ended: its process exit status.
///
/// | code | meaning |
/// |---|---|
/// | 0 | the command did what was asked |
/// | 1 | it couldn't: an input was missing, unreadable or refused |
/// | 2 | the command line was wrong: an unknown command or option, or a missing argument |
/// | 3 | the command is registered but its milestone hasn't come yet |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// The command did what was asked.
    Success,
    /// The command couldn't do it: an input was missing, unreadable or refused.
    Failure,
    /// The command line was wrong. Clap's own convention, which its usage errors keep.
    Usage,
    /// The command exists but isn't available yet.
    NotAvailable,
}

impl Exit {
    /// The process exit status.
    pub fn code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Failure => 1,
            Self::Usage => 2,
            Self::NotAvailable => 3,
        }
    }
}

/// The command line: global options and one command.
#[derive(Debug, Parser)]
#[command(
    name = "hpr",
    version,
    about = "Flight simulation for hobby and high-power rockets",
    long_about = "Flight simulation for hobby and high-power rockets.\n\n\
                  Every command prints text, or one JSON document with --json. \
                  Commands marked \"not available yet\" name the milestone that brings them. \
                  Guide: https://nrdptel.github.io/hpr-sim/cli.html",
    propagate_version = true,
    arg_required_else_help = true
)]
pub struct Cli {
    /// Print one JSON document on standard output instead of text; errors too.
    #[arg(long, global = true)]
    pub json: bool,
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The commands, in the order `hpr --help` lists them.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Fly a .ork, an .hpr or .hprz design, or a rocket's .json from a rail and print its flight;
    /// export its recording
    Sim(sim::SimArgs),
    /// Run the validation cases and check them against the committed reports and the census
    Validate(validate::ValidateArgs),
    /// Convert a motor file between .eng and .rse, or a catalog motor to either; or a design
    /// between .ork, .hpr and .hprz
    Convert(convert::ConvertArgs),
    /// Look up motors in the bundled catalog, read a .eng or .rse motor file, or search vendors'
    /// stock and prices
    #[command(subcommand)]
    Motors(motors::MotorsCommand),
    /// Fetch a launch day's weather, or read a weather file, as a profile of air and wind
    #[command(subcommand)]
    Weather(weather::WeatherCommand),
    /// Fly a design many times, each with randomly scattered inputs (not available yet)
    Mc(Planned),
    /// Search a design's parameters for a goal (not available yet)
    Optimize(Planned),
    /// Compare a flight log with its simulation (not available yet)
    Compare(Planned),
    /// Read a flight log and print its readings, with no design file
    Analyze(analyze::AnalyzeArgs),
    /// Diagnose what went wrong in a flight from its log (not available yet)
    Diagnose(Planned),
    /// Print a shell completion script for hpr
    Completions(CompletionsArgs),
}

impl Command {
    /// The command's name, as typed.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sim(_) => "sim",
            Self::Validate(_) => "validate",
            Self::Convert(_) => "convert",
            Self::Motors(_) => "motors",
            Self::Weather(_) => "weather",
            Self::Mc(_) => "mc",
            Self::Optimize(_) => "optimize",
            Self::Compare(_) => "compare",
            Self::Analyze(_) => "analyze",
            Self::Diagnose(_) => "diagnose",
            Self::Completions(_) => "completions",
        }
    }
}

/// The arguments of a command that isn't available yet: anything, so that the refusal names the
/// milestone instead of complaining about an option the command will one day have.
#[derive(Debug, clap::Args)]
pub struct Planned {
    /// Accepted and ignored until the command arrives.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    pub args: Vec<OsString>,
}

/// `hpr completions`'s arguments.
#[derive(Debug, clap::Args)]
pub struct CompletionsArgs {
    /// The shell to write the script for.
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Why a command stopped. [`run`] reports it as text or as an [`ErrorDocument`].
#[derive(Debug)]
pub(crate) enum Failure {
    /// An input was missing, unreadable or refused.
    Input(String),
    /// The command's milestone hasn't come yet.
    NotAvailable {
        /// The command.
        command: &'static str,
        /// The milestone that brings it.
        milestone: &'static str,
    },
    /// Standard output couldn't be written, such as a closed pipe.
    Output(io::Error),
    /// The command wrote its output, and it says the check failed, as `hpr validate`'s does. The
    /// reason goes to standard error, without `--json`; with it, the document says it.
    Checked(String),
}

/// Where a command's output goes, and in which form.
pub(crate) struct Out<'a> {
    /// Standard output.
    pub(crate) out: &'a mut dyn Write,
    /// Whether `--json` was given.
    pub(crate) json: bool,
}

impl Out<'_> {
    /// Writes `value` as one JSON document, or `text` when `--json` wasn't given, and flushes.
    /// Only a failure to write standard output is a [`Failure::Output`].
    pub(crate) fn emit<T: Serialize>(
        &mut self,
        value: &T,
        text: impl FnOnce(&mut dyn Write) -> io::Result<()>,
    ) -> Result<(), Failure> {
        let written = if self.json {
            write_json(self.out, value)
        } else {
            text(self.out)
        };
        written
            .and_then(|()| self.out.flush())
            .map_err(Failure::Output)
    }
}

/// Writes one pretty-printed JSON document and a newline.
fn write_json<T: Serialize>(out: &mut dyn Write, value: &T) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *out, value).map_err(io::Error::from)?;
    writeln!(out)
}

/// Runs `hpr` on a command line, `args[0]` being the program's name, writing to `out` (standard
/// output) and `err` (standard error).
///
/// With `--json` anywhere on the line before a `--`, standard output gets exactly one JSON
/// document, success or failure, and standard error stays empty; `--help` and `--version` still
/// print text. That scan decides, not [`Cli::json`]: a command that isn't available yet takes
/// every argument after its name as its own, `--json` included.
pub fn run<I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> Exit
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    // Scanned before parsing so that a usage error can be reported as JSON too. After `--`,
    // `--json` is an argument, such as a file's name.
    let json = args
        .iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .any(|arg| arg == "--json");
    let cli = match Cli::try_parse_from(&args) {
        Ok(cli) => cli,
        Err(error) => return usage(&error, json, out, err),
    };
    let command = cli.command.name();
    let mut to = Out { out, json };
    // The registry decides what refuses, so the table and the tool can't disagree.
    let outcome = match registry::availability(command) {
        Some(Availability::Planned { milestone }) => {
            Err(Failure::NotAvailable { command, milestone })
        }
        Some(Availability::Available { .. }) => match cli.command {
            Command::Motors(motors) => motors::run(&motors, &mut to),
            Command::Sim(args) => sim::run(&args, &mut to),
            Command::Validate(args) => validate::run(&args, &mut to),
            Command::Convert(args) => convert::run(&args, &mut to),
            Command::Analyze(args) => analyze::run(&args, &mut to),
            Command::Weather(weather) => weather::run(&weather, &mut to),
            Command::Completions(args) => completions(args.shell, &mut to),
            Command::Mc(_) | Command::Optimize(_) | Command::Compare(_) | Command::Diagnose(_) => {
                Err(Failure::Input(format!(
                    "hpr {command} is marked available in the command registry, but this build \
                 has no code for it"
                )))
            }
        },
        None => Err(Failure::Input(format!(
            "hpr {command} has no entry in the command registry"
        ))),
    };
    match outcome {
        Ok(()) => Exit::Success,
        Err(failure) => report(failure, command, json, out, err),
    }
}

/// Reports a failure on standard error, or as an [`ErrorDocument`] with `--json`.
fn report(
    failure: Failure,
    command: &'static str,
    json: bool,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Exit {
    let (document, exit) = match failure {
        Failure::Input(message) => (
            ErrorDocument::new(ErrorKind::Input, message, Some(command), None),
            Exit::Failure,
        ),
        Failure::NotAvailable { command, milestone } => {
            let message = format!(
                "hpr {command} is not available yet: it arrives with milestone {milestone} \
                 (https://nrdptel.github.io/hpr-sim/decisions-and-roadmap.html#{})",
                registry::anchor(milestone)
            );
            (
                ErrorDocument::new(
                    ErrorKind::NotAvailable,
                    message,
                    Some(command),
                    Some(milestone),
                ),
                Exit::NotAvailable,
            )
        }
        Failure::Checked(message) => {
            if !json {
                let _ = writeln!(err, "error: {message}");
            }
            return Exit::Failure;
        }
        // A closed pipe (`hpr motors list | head -1`): the reader stopped reading, which is its
        // choice, not a failure, and whether it happens depends on the pipe's buffer. Anything
        // else goes to standard error, as JSON can't be written to the stream that just failed.
        Failure::Output(error) => {
            if error.kind() == io::ErrorKind::BrokenPipe {
                return Exit::Success;
            }
            let _ = writeln!(err, "error: couldn't write the output: {error}");
            return Exit::Failure;
        }
    };
    // Nowhere is left to report a failure to write the report; the exit status still says it.
    let _ = if json {
        write_json(out, &document)
    } else {
        writeln!(err, "error: {}", document.error.message)
    };
    exit
}

/// Reports clap's usage error, help or version text.
fn usage(error: &clap::Error, json: bool, out: &mut dyn Write, err: &mut dyn Write) -> Exit {
    // Help and version requests are not errors: clap sends them to standard output with status 0.
    if !error.use_stderr() {
        let _ = write!(out, "{}", error.render());
        return Exit::Success;
    }
    // As in `report`: the exit status is all that is left if this fails.
    let _ = if json {
        let message = error.render().to_string();
        let document = ErrorDocument::new(ErrorKind::Usage, message.trim_end(), None, None);
        write_json(out, &document)
    } else {
        write!(err, "{}", error.render())
    };
    Exit::Usage
}

/// `hpr completions <shell>`.
fn completions(shell: Shell, to: &mut Out<'_>) -> Result<(), Failure> {
    let mut script = Vec::new();
    clap_complete::generate(shell, &mut Cli::command(), "hpr", &mut script);
    let script = String::from_utf8(script)
        .map_err(|error| Failure::Input(format!("the completion script isn't UTF-8: {error}")))?;
    let document = Completions {
        shell: shell.to_string(),
        script,
    };
    to.emit(&document, |out| out.write_all(document.script.as_bytes()))
}

/// The JSON Schema of each `--json` output, by file name: what `cargo xtask cli` writes to
/// `schema/cli/`.
pub fn schemas() -> Vec<(&'static str, String)> {
    output::schemas()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A writer whose reader has gone, as `hpr motors list | head -1` leaves standard output.
    struct ClosedPipe;

    impl Write for ClosedPipe {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A closed pipe ends the run with status 0 and says nothing: the reader chose to stop.
    #[test]
    fn a_closed_pipe_ends_quietly() {
        for json in [false, true] {
            let mut args = vec!["hpr", "motors", "list"];
            if json {
                args.push("--json");
            }
            let mut err = Vec::new();
            assert_eq!(run(args, &mut ClosedPipe, &mut err), Exit::Success);
            assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
        }
    }

    /// The exit codes the guide's table lists.
    #[test]
    fn exit_codes_are_the_documented_ones() {
        let codes = [
            Exit::Success,
            Exit::Failure,
            Exit::Usage,
            Exit::NotAvailable,
        ]
        .map(Exit::code);
        assert_eq!(codes, [0, 1, 2, 3]);
    }
}
