use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::complexity;

#[derive(Parser)]
#[command(name = "smell", version, about)]
struct Cli {
    /// Source file or directory to analyze (Swift, Rust, Kotlin, Java;
    /// directories are searched recursively).
    path: PathBuf,
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    complexity::run(cli.path)
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_path_argument() {
        let cli = Cli::try_parse_from(["smell", "src"]).expect("path should parse");
        assert_eq!(cli.path, PathBuf::from("src"));
    }

    #[test]
    fn requires_path_argument() {
        assert!(Cli::try_parse_from(["smell"]).is_err());
    }
}
