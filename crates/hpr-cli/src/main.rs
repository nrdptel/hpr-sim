//! The `hpr` command-line tool.
//!
//! Status: pre-alpha skeleton. The commands arrive in M4.2.

#![allow(
    clippy::print_stdout,
    reason = "a command-line tool reports on standard output"
)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "the command-line tool reads and writes files; it is not part of the pure core"
)]

fn main() {
    println!(
        "hpr {}: pre-alpha, no commands are available yet",
        env!("CARGO_PKG_VERSION")
    );
}
