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

mod aero;
mod aero_blunt;
mod aero_body;
mod aero_bullet;
mod aero_crossflow;
mod aero_drag;
mod aero_flare;
mod aero_gap;
mod aero_lip;
mod aero_mach;
mod aero_override;
mod aero_roll;
mod designs;
#[cfg(test)]
mod docs;
mod examples;
mod layering;
mod ork;
mod ork_corpus_flights;
mod ork_extensions;
mod ork_flights;
mod ork_geometry;
mod ork_mass;
mod ork_motors;
mod ork_recovery;
mod ork_simulations;
mod ork_supply;
mod refs;
mod site;
mod validate;
mod wasm_check;
mod workspace;

use std::process::ExitCode;

const USAGE_TEMPLATE: &str = "\
Usage: cargo xtask <command> [args]

Commands:
  wasm-check [cargo args]  Check the crate layering rules, and that the pure-core crates
                           build for wasm32-unknown-unknown. Extra arguments (for example
                           --locked) are passed on to `cargo check`.
{REFS}
{DESIGNS}
{AERO}
{VALIDATE}
{SITE}
{EXAMPLES}
{ORK}
{ORK_FLIGHTS}
  help                     Print this message.";

fn usage() -> String {
    USAGE_TEMPLATE
        .replace("{REFS}", refs::USAGE)
        .replace("{DESIGNS}", designs::USAGE)
        .replace("{AERO}", aero::USAGE)
        .replace("{VALIDATE}", validate::USAGE)
        .replace("{SITE}", site::USAGE)
        .replace("{EXAMPLES}", examples::USAGE)
        .replace("{ORK}", ork::USAGE)
        .replace("{ORK_FLIGHTS}", ork_flights::USAGE)
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let result = match args.next().as_deref() {
        Some("wasm-check") => wasm_check::run(&args.collect::<Vec<_>>()),
        Some("refs") => refs::run(&args.collect::<Vec<_>>()),
        Some("designs") => designs::run(&args.collect::<Vec<_>>()),
        Some("aero") => aero::run(&args.collect::<Vec<_>>()),
        Some("validate") => validate::run(&args.collect::<Vec<_>>()),
        Some("site") => site::run(&args.collect::<Vec<_>>()),
        Some("examples") => examples::run(&args.collect::<Vec<_>>()),
        Some("ork") => ork::run(&args.collect::<Vec<_>>()),
        Some("ork-flights") => ork_flights::run(&args.collect::<Vec<_>>()),
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
