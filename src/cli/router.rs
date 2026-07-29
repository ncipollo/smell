use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::complexity;
use crate::cli::info;

#[derive(Parser)]
#[command(name = "smell", version, about)]
struct Cli {
    /// Source file or directory to analyze (Swift, Rust, Kotlin, Java;
    /// directories are searched recursively).
    #[arg(required_unless_present = "info")]
    path: Option<PathBuf>,

    /// Only analyze files matching this glob (repeatable).
    #[arg(long)]
    include: Vec<String>,

    /// Skip files matching this glob (repeatable).
    #[arg(long)]
    exclude: Vec<String>,

    /// Count only these branch kinds: friendly names (see --info) or raw
    /// tree-sitter node kinds, comma-separated.
    #[arg(long, value_delimiter = ',')]
    branches: Vec<String>,

    /// Print the branch-kind and glob vocabulary docs and exit.
    #[arg(long)]
    info: bool,
}

pub fn run() -> ExitCode {
    let cli = Cli::parse();
    if cli.info {
        return info::run();
    }
    let path = cli.path.expect("clap enforces path unless --info");
    complexity::run(path, cli.include, cli.exclude, cli.branches)
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
        assert_eq!(cli.path, Some(PathBuf::from("src")));
    }

    #[test]
    fn requires_path_argument() {
        assert!(Cli::try_parse_from(["smell"]).is_err());
    }

    #[test]
    fn parses_repeatable_include_and_exclude() {
        let cli = Cli::try_parse_from([
            "smell",
            "src",
            "--include",
            "*.rs",
            "--include",
            "*.swift",
            "--exclude",
            "**/gen/**",
        ])
        .expect("globs should parse");
        assert_eq!(cli.include, vec!["*.rs", "*.swift"]);
        assert_eq!(cli.exclude, vec!["**/gen/**"]);
    }

    #[test]
    fn parses_comma_separated_branches() {
        let cli = Cli::try_parse_from(["smell", "src", "--branches", "switch,loop"])
            .expect("branches should parse");
        assert_eq!(cli.branches, vec!["switch", "loop"]);
    }

    #[test]
    fn info_does_not_require_a_path() {
        let cli = Cli::try_parse_from(["smell", "--info"]).expect("--info should parse");
        assert!(cli.info);
        assert_eq!(cli.path, None);
    }
}
