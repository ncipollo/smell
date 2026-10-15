//! The `diff-pr` topic: invoking `diff` vs `pr`, their shared flags, and
//! which one to reach for.

pub fn render() -> String {
    format!("{}\n{}", invocation_section(), decision_section())
}

fn invocation_section() -> String {
    String::from(
        "INVOKING DIFF AND PR\n\
         smell diff <OLD_REF> <NEW_REF> <PATH>...\n\
         Compares two explicit commit refs against a local repo.\n\n\
         smell pr <REFERENCE> <PATH>...\n\
         Resolves REFERENCE to a base/head commit pair, then runs the same\n\
         diff. REFERENCE is one of: an explicit `<old>..<new>` range\n\
         (forwarded as-is), a GitHub PR URL, `owner/repo#123`, or a bare PR\n\
         number resolved against the current repo's `origin` remote.\n\n\
         SHARED FLAGS\n\
         Both subcommands define the same flags: --repo <PATH> (defaults\n\
         to \".\"), --delta-methods/--delta-lines/--delta-declarations/\n\
         --delta-comment-lines/--delta-comments <N> (fail if the swing\n\
         between the old and new revision exceeds N — relative, unlike\n\
         the absolute --max-* checks in --info checks), and --json.\n",
    )
}

fn decision_section() -> String {
    String::from(
        "WHICH TO USE\n\
         Use diff when both commit refs are already in hand: local branch\n\
         work, CI with known SHAs, comparing two tags. Use pr when only a\n\
         GitHub PR identifier is available and smell should resolve the\n\
         base/head commits itself instead of a manual `git fetch`/\n\
         `merge-base`.\n\n\
         See also: --info review, --info checks.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_diff_invocation() {
        assert!(render().contains("smell diff <OLD_REF> <NEW_REF>"));
    }

    #[test]
    fn page_documents_pr_invocation() {
        let page = render();
        assert!(page.contains("smell pr <REFERENCE>"));
        assert!(page.contains("owner/repo#123"));
    }

    #[test]
    fn page_documents_the_decision_rule() {
        let page = render();
        assert!(page.contains("WHICH TO USE"));
        assert!(page.contains("Use diff when"));
        assert!(page.contains("Use pr when"));
    }

    #[test]
    fn page_documents_shared_flags() {
        let page = render();
        assert!(page.contains("--delta-methods"));
        assert!(page.contains("--repo"));
        assert!(page.contains("--json"));
    }
}
