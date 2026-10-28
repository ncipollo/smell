use clap::Subcommand;

use super::info_args::InfoArgs;
use super::pr_args::PrArgs;

#[derive(Subcommand)]
pub(super) enum Commands {
    /// Find areas to focus on in a PR review by comparing complexity
    /// between the PR's base and head commits (or an explicit
    /// `<old>..<new>` range).
    Pr(PrArgs),
    /// Print documentation topics (mirrors `--info`/`--info <TOPIC>`).
    Info(InfoArgs),
}
