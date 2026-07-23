use clap::{Parser, Subcommand};

use crate::cli::analyze;

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
}

pub fn run() {
    let cli = Cli::parse();
    match cli.command {
        Command::Analyze => analyze::run(),
    }
}
