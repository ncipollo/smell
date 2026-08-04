use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::Overrides;
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

    /// Only analyze types implementing/extending this interface, protocol,
    /// trait, or superclass (repeatable; see --info).
    #[arg(long, value_name = "NAME")]
    implements: Vec<String>,

    /// Fail (exit non-zero) if any function's complexity exceeds this limit,
    /// listing the offending files and functions (see --info).
    #[arg(long, value_name = "N")]
    max_complexity: Option<usize>,

    /// Use the named rule from smell.toml instead of the "default" rule.
    #[arg(long, value_name = "NAME")]
    rule: Option<String>,

    /// Suppress the per-file complexity report; errors and --max-complexity
    /// failures are still printed.
    #[arg(short, long)]
    quiet: bool,

    /// Print the analysis as JSON instead of the table report; the
    /// --max-complexity check result is embedded in the document.
    #[arg(long, conflicts_with = "quiet")]
    json: bool,

    /// Print the branch-kind and glob vocabulary docs and exit.
    #[arg(long)]
    info: bool,
}

pub fn run() -> ExitCode {
    let Cli {
        path,
        include,
        exclude,
        branches,
        implements,
        max_complexity,
        rule,
        quiet,
        json,
        info,
    } = Cli::parse();
    if info {
        return info::run();
    }
    let path = path.expect("clap enforces path unless --info");
    complexity::run(
        path,
        Overrides {
            include,
            exclude,
            branches,
            implements,
            max_complexity,
            rule,
        },
        quiet,
        json,
    )
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
    fn parses_repeatable_implements() {
        let cli = Cli::try_parse_from([
            "smell",
            "src",
            "--implements",
            "Describe",
            "--implements",
            "Labeled",
        ])
        .expect("implements should parse");
        assert_eq!(cli.implements, vec!["Describe", "Labeled"]);
    }

    #[test]
    fn implements_defaults_to_empty() {
        let cli = Cli::try_parse_from(["smell", "src"]).expect("path should parse");
        assert!(cli.implements.is_empty());
    }

    #[test]
    fn parses_max_complexity() {
        let cli = Cli::try_parse_from(["smell", "src", "--max-complexity", "12"])
            .expect("max complexity should parse");
        assert_eq!(cli.max_complexity, Some(12));
    }

    #[test]
    fn max_complexity_defaults_to_none() {
        let cli = Cli::try_parse_from(["smell", "src"]).expect("path should parse");
        assert_eq!(cli.max_complexity, None);
    }

    #[test]
    fn info_does_not_require_a_path() {
        let cli = Cli::try_parse_from(["smell", "--info"]).expect("--info should parse");
        assert!(cli.info);
        assert_eq!(cli.path, None);
    }

    #[test]
    fn parses_rule_name() {
        let cli =
            Cli::try_parse_from(["smell", "src", "--rule", "swift"]).expect("rule should parse");
        assert_eq!(cli.rule, Some("swift".to_string()));
    }

    #[test]
    fn rule_defaults_to_none() {
        let cli = Cli::try_parse_from(["smell", "src"]).expect("path should parse");
        assert_eq!(cli.rule, None);
    }

    #[test]
    fn parses_quiet_long() {
        let cli = Cli::try_parse_from(["smell", "src", "--quiet"]).expect("--quiet should parse");
        assert!(cli.quiet);
    }

    #[test]
    fn parses_quiet_short() {
        let cli = Cli::try_parse_from(["smell", "src", "-q"]).expect("-q should parse");
        assert!(cli.quiet);
    }

    #[test]
    fn quiet_defaults_to_false() {
        let cli = Cli::try_parse_from(["smell", "src"]).expect("path should parse");
        assert!(!cli.quiet);
    }

    #[test]
    fn parses_json() {
        let cli = Cli::try_parse_from(["smell", "src", "--json"]).expect("--json should parse");
        assert!(cli.json);
    }

    #[test]
    fn json_defaults_to_false() {
        let cli = Cli::try_parse_from(["smell", "src"]).expect("path should parse");
        assert!(!cli.json);
    }

    #[test]
    fn json_conflicts_with_quiet() {
        assert!(Cli::try_parse_from(["smell", "src", "--json", "--quiet"]).is_err());
        assert!(Cli::try_parse_from(["smell", "src", "--json", "-q"]).is_err());
    }
}
