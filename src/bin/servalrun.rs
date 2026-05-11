//! Entry point for the `servalrun` CLI.
//!
//! Thin wrapper around `serval_run::cli::run`; all logic lives in the library
//! so the same surface can be exercised from integration tests without
//! re-implementing argument parsing.

fn main() {
    let exit_code = serval_run::cli::run(std::env::args_os());
    std::process::exit(exit_code);
}
