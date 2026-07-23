use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::cli::analyze;
use crate::cli::complexity;

#[derive(Parser)]
#[command(name = "smell", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze source code for smells.
    Analyze,
    /// Report branch complexity per function, broken down by file.
    Complexity {
        /// Swift file or directory to analyze (directories are searched recursively).
        path: PathBuf,
    },
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze => analyze::run(),
        Command::Complexity { path } => complexity::run(path),
    }
}
