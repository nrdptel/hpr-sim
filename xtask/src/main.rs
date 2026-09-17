//! Development automation for the hpr-sim workspace, run as `cargo xtask <command>`.
//!
//! Run `cargo xtask help` for the list of commands.

#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "a command-line tool reports on the terminal"
)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "dev tooling runs cargo and uses the filesystem; it is not part of the pure core"
)]

#[cfg(test)]
mod docs;
mod refs;
mod wasm_check;
mod workspace;

use std::process::ExitCode;

const USAGE_TEMPLATE: &str = "\
Usage: cargo xtask <command> [args]

Commands:
  wasm-check [cargo args]  Check the pure-core crates for wasm32-unknown-unknown. Extra
                           arguments (for example --locked) are passed on to `cargo check`.
{REFS}
  help                     Print this message.";

fn usage() -> String {
    USAGE_TEMPLATE.replace("{REFS}", refs::USAGE)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let result = match args.next().as_deref() {
        Some("wasm-check") => wasm_check::run(&args.collect::<Vec<_>>()),
        Some("refs") => refs::run(&args.collect::<Vec<_>>()),
        Some("help" | "-h" | "--help") => {
            println!("{}", usage());
            Ok(())
        }
        Some(other) => Err(format!("unknown command `{other}`\n\n{}", usage())),
        None => Err(format!("no command given\n\n{}", usage())),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
