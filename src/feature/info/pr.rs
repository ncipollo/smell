//! The `pr` topic: invoking `smell pr` and its flags.

pub fn render() -> String {
    format!("{}\n{}", invocation_section(), flags_section())
}

fn invocation_section() -> String {
    String::from(
        "INVOKING PR\n\
         smell pr <REFERENCE> [PATH]...\n\
         Surfaces the areas of a pull request that deserve attention\n\
         during review. Resolves REFERENCE to a base/head commit pair,\n\
         then diffs complexity between them. REFERENCE is one of: an\n\
         explicit `<old>..<new>` range (forwarded as-is), a GitHub PR\n\
         URL, `owner/repo#123`, or a bare PR number resolved against the\n\
         current repo's `origin` remote. PATH defaults to \".\" (the whole\n\
         repository). Run it from the PR's repository, or point at that\n\
         repository with --repo <PATH>.\n",
    )
}

fn flags_section() -> String {
    String::from(
        "FLAGS\n\
         --repo <PATH> (defaults to \".\"), --delta-methods/--delta-lines/\n\
         --delta-declarations/--delta-comment-lines/--delta-comments <N>\n\
         (fail if the swing between the old and new revision exceeds N —\n\
         relative, unlike the absolute --max-* checks in --info checks),\n\
         and --json.\n\n\
         See also: --info review, --info checks.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_pr_invocation() {
        let page = render();
        assert!(page.contains("smell pr <REFERENCE> [PATH]..."));
        assert!(page.contains("defaults to \".\""));
        assert!(page.contains("owner/repo#123"));
    }

    #[test]
    fn page_frames_pr_as_review_focus_finder() {
        assert!(render().contains("deserve attention\nduring review"));
    }

    #[test]
    fn page_documents_flags() {
        let page = render();
        assert!(page.contains("--delta-methods"));
        assert!(page.contains("--repo"));
        assert!(page.contains("--json"));
    }

    #[test]
    fn page_does_not_mention_the_diff_command() {
        assert!(!render().contains("smell diff"));
    }
}
