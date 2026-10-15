use std::path::PathBuf;

use clap::{Args, Subcommand};

use super::info_args::InfoArgs;
use super::pr_args::PrArgs;

#[derive(Subcommand)]
pub(super) enum Commands {
    /// Compare complexity between two git commits.
    Diff(DiffArgs),
    /// Compare complexity between a PR's base and head commits (or an
    /// explicit `<old>..<new>` range).
    Pr(PrArgs),
    /// Print documentation topics (mirrors `--info`/`--info <TOPIC>`).
    Info(InfoArgs),
}

#[derive(Args)]
pub(super) struct DiffArgs {
    /// Commit ref to diff from.
    pub(super) old_ref: String,

    /// Commit ref to diff to.
    pub(super) new_ref: String,

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
    use super::Commands;

    #[test]
    fn parses_diff_subcommand() {
        let cli = Cli::try_parse_from(["smell", "diff", "HEAD~1", "HEAD", "src"])
            .expect("diff should parse");
        let Some(Commands::Diff(args)) = cli.command else {
            panic!("expected a diff subcommand");
        };
        assert_eq!(args.old_ref, "HEAD~1");
        assert_eq!(args.new_ref, "HEAD");
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
    fn parses_diff_delta_flags() {
        let cli = Cli::try_parse_from([
            "smell",
            "diff",
            "HEAD~1",
            "HEAD",
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
        ])
        .expect("delta flags should parse");
        let Some(Commands::Diff(args)) = cli.command else {
            panic!("expected a diff subcommand");
        };
        assert_eq!(args.delta_methods, Some(1));
        assert_eq!(args.delta_lines, Some(20));
        assert_eq!(args.delta_declarations, Some(2));
        assert_eq!(args.delta_comment_lines, Some(5));
        assert_eq!(args.delta_comments, Some(3));
    }

    #[test]
    fn parses_diff_repo_and_json() {
        let cli = Cli::try_parse_from([
            "smell", "diff", "HEAD~1", "HEAD", "src", "--repo", "../other", "--json",
        ])
        .expect("diff should parse");
        let Some(Commands::Diff(args)) = cli.command else {
            panic!("expected a diff subcommand");
        };
        assert_eq!(args.repo, PathBuf::from("../other"));
        assert!(args.json);
    }

    #[test]
    fn diff_requires_both_refs_and_a_path() {
        assert!(Cli::try_parse_from(["smell", "diff"]).is_err());
        assert!(Cli::try_parse_from(["smell", "diff", "HEAD~1"]).is_err());
        assert!(Cli::try_parse_from(["smell", "diff", "HEAD~1", "HEAD"]).is_err());
    }
}
