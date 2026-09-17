//! The `hpr` command-line tool.
//!
//! Status: pre-alpha skeleton. The commands arrive in M4.2.

#![allow(
    clippy::print_stdout,
    reason = "a command-line tool reports on standard output"
)]

fn main() {
    println!(
        "hpr {}: pre-alpha, no commands are available yet",
        env!("CARGO_PKG_VERSION")
    );
}
