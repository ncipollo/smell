use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::cli::complexity;

#[derive(Parser)]
#[command(name = "smell", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report branch complexity per function, broken down by file.
    Complexity {
        /// Source file or directory to analyze (Swift, Rust, Kotlin, Java;
        /// directories are searched recursively).
        path: PathBuf,
    },
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Complexity { path } => complexity::run(path),
    }
}
