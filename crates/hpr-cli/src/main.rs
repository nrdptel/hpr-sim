//! The `hpr` command-line tool. The commands are in the `hpr_cli` library; this hands them the
//! process's arguments and exits with the status they return.

#![allow(
    clippy::disallowed_methods,
    reason = "the command-line tool reads its arguments; it is not part of the pure core"
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    let exit = hpr_cli::run(
        std::env::args_os(),
        &mut std::io::stdout().lock(),
        &mut std::io::stderr().lock(),
    );
    ExitCode::from(exit.code())
}
