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
         explicit `<old>..<new>` range, a GitHub PR URL, `owner/repo#123`,\n\
         or a bare PR number resolved against the current repo's `origin`\n\
         remote. PATH defaults to \".\" (the whole repository). Run it from\n\
         the PR's repository, or point at it with --repo <PATH>.\n\n\
         Only files with changed functions, plus new and removed files,\n\
         are listed; pass --verbose to list every file.\n",
    )
}

fn flags_section() -> String {
    String::from(
        "FLAGS\n\
         --repo <PATH> (defaults to \".\"), --delta-methods/--delta-lines/\n\
         --delta-declarations/--delta-comment-lines/--delta-comments <N>\n\
         (fail if the swing between the old and new revision exceeds N —\n\
         relative, unlike the absolute --max-* checks in --info checks),\n\
         --json (JSON always includes every file), and --verbose.\n\n\
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
    fn page_documents_verbose_and_default_filtering() {
        let page = render();
        assert!(page.contains("--verbose"));
        assert!(page.contains("new and removed files"));
    }

    #[test]
    fn page_does_not_mention_the_diff_command() {
        assert!(!render().contains("smell diff"));
    }
}
