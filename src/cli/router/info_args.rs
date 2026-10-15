use clap::Args;

#[derive(Args)]
pub(super) struct InfoArgs {
    /// Topic to print (bare `smell info` lists available topics).
    pub(super) topic: Option<String>,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::super::Cli;
    use super::super::diff_args::Commands;

    #[test]
    fn parses_info_subcommand() {
        let cli = Cli::try_parse_from(["smell", "info"]).expect("info should parse");
        let Some(Commands::Info(args)) = cli.command else {
            panic!("expected an info subcommand");
        };
        assert_eq!(args.topic, None);
    }

    #[test]
    fn info_subcommand_topic_parses() {
        let cli = Cli::try_parse_from(["smell", "info", "usage"]).expect("info usage should parse");
        let Some(Commands::Info(args)) = cli.command else {
            panic!("expected an info subcommand");
        };
        assert_eq!(args.topic, Some("usage".to_string()));
    }

    #[test]
    fn info_subcommand_does_not_require_a_path() {
        let cli = Cli::try_parse_from(["smell", "info"]).expect("info should parse");
        assert!(cli.paths.is_empty());
    }
}
