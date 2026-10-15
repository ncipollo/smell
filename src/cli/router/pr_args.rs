use std::path::PathBuf;

use clap::Args;

#[derive(Args)]
pub(super) struct PrArgs {
    /// A commit range (`<old>..<new>`) or a GitHub PR reference: a full
    /// URL, `owner/repo#123`, or a bare number resolved against the
    /// current repo's `origin` remote.
    pub(super) reference: String,

    /// Paths (relative to the repository root) to analyze; directories are
    /// searched recursively.
    #[arg(value_name = "PATH", num_args = 1.., required = true)]
    pub(super) paths: Vec<PathBuf>,

    /// Path to the local git repository, or any directory inside it
    /// (defaults to the current working directory).
    #[arg(long, value_name = "PATH", default_value = ".")]
    pub(super) repo: PathBuf,

    /// Fail (exit non-zero) if any type's method count swung by more than
    /// this amount between the two revisions, listing the offending files
    /// and types (see --info checks).
    #[arg(long, value_name = "N")]
    pub(super) delta_methods: Option<usize>,

    /// Fail (exit non-zero) if any file's line count swung by more than this
    /// amount between the two revisions, listing the offending files (see
    /// --info checks).
    #[arg(long, value_name = "N")]
    pub(super) delta_lines: Option<usize>,

    /// Fail (exit non-zero) if any file's declaration count swung by more
    /// than this amount between the two revisions, listing the offending
    /// files (see --info checks).
    #[arg(long, value_name = "N")]
    pub(super) delta_declarations: Option<usize>,

    /// Fail (exit non-zero) if any file's comment-line count swung by more
    /// than this amount between the two revisions, listing the offending
    /// files (see --info checks).
    #[arg(long, value_name = "N")]
    pub(super) delta_comment_lines: Option<usize>,

    /// Fail (exit non-zero) if any file's comment count swung by more than
    /// this amount between the two revisions, listing the offending files
    /// (see --info checks).
    #[arg(long, value_name = "N")]
    pub(super) delta_comments: Option<usize>,

    /// Print the diff as JSON.
    #[arg(long)]
    pub(super) json: bool,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use clap::Parser;

    use super::super::Cli;
    use super::super::diff_args::Commands;

    #[test]
    fn parses_pr_subcommand() {
        let cli = Cli::try_parse_from(["smell", "pr", "123", "src"]).expect("pr should parse");
        let Some(Commands::Pr(args)) = cli.command else {
            panic!("expected a pr subcommand");
        };
        assert_eq!(args.reference, "123");
        assert_eq!(args.paths, vec![PathBuf::from("src")]);
        assert_eq!(args.repo, PathBuf::from("."));
        assert!(!args.json);
        assert_eq!(args.delta_methods, None);
        assert_eq!(args.delta_lines, None);
        assert_eq!(args.delta_declarations, None);
        assert_eq!(args.delta_comment_lines, None);
        assert_eq!(args.delta_comments, None);
    }

    #[test]
    fn parses_pr_explicit_range() {
        let cli = Cli::try_parse_from(["smell", "pr", "HEAD~1..HEAD", "src"])
            .expect("pr range should parse");
        let Some(Commands::Pr(args)) = cli.command else {
            panic!("expected a pr subcommand");
        };
        assert_eq!(args.reference, "HEAD~1..HEAD");
    }

    #[test]
    fn parses_pr_delta_flags_repo_and_json() {
        let cli = Cli::try_parse_from([
            "smell",
            "pr",
            "123",
            "src",
            "--delta-methods",
            "1",
            "--delta-lines",
            "20",
            "--delta-declarations",
            "2",
            "--delta-comment-lines",
            "5",
            "--delta-comments",
            "3",
            "--repo",
            "../other",
            "--json",
        ])
        .expect("pr flags should parse");
        let Some(Commands::Pr(args)) = cli.command else {
            panic!("expected a pr subcommand");
        };
        assert_eq!(args.delta_methods, Some(1));
        assert_eq!(args.delta_lines, Some(20));
        assert_eq!(args.delta_declarations, Some(2));
        assert_eq!(args.delta_comment_lines, Some(5));
        assert_eq!(args.delta_comments, Some(3));
        assert_eq!(args.repo, PathBuf::from("../other"));
        assert!(args.json);
    }

    #[test]
    fn pr_requires_a_reference_and_a_path() {
        assert!(Cli::try_parse_from(["smell", "pr"]).is_err());
        assert!(Cli::try_parse_from(["smell", "pr", "123"]).is_err());
    }
}
