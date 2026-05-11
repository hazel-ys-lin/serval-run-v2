//! `servalrun` CLI library entry point.
//!
//! The `servalrun` binary in `src/bin/servalrun.rs` is a thin wrapper
//! over [`run`]. Keeping the parser + dispatcher here means tests can
//! drive the CLI without forking a process.
//!
//! Public surface (v0.x, still growing):
//! - `servalrun status [--server URL] [--json]` — hit `/health`
//!
//! Exit codes are documented in [`exit`].

pub mod commands;
pub mod exit;
pub mod output;

use std::ffi::OsString;

use clap::{Parser, Subcommand};

use crate::cli::output::OutputFormat;

/// `servalrun` — spec execution layer for v0.x.
///
/// More subcommands land as Phase 1 progresses (`config`, `login`,
/// `api`, `run`, `diff`, `history`, ...). For now the CLI exists
/// primarily to validate the scaffold.
#[derive(Debug, Parser)]
#[command(name = "servalrun", version, about, long_about = None)]
struct Cli {
    /// Emit JSON instead of the human-friendly table.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the upstream server's health report.
    Status(commands::status::StatusArgs),
}

/// Parse the given argv and run the matching subcommand.
///
/// Returns the CLI's exit code; the binary in `src/bin/servalrun.rs`
/// passes this straight to `std::process::exit`.
pub fn run<I, T>(argv: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(argv) {
        Ok(c) => c,
        Err(e) => {
            // clap already formatted the message. Use its suggested
            // exit code (0 for --help / --version, 2 for usage errors).
            let _ = e.print();
            return e.exit_code();
        }
    };

    let format = if cli.json {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    };

    match cli.command {
        Command::Status(args) => commands::status::run(args, format),
    }
}
