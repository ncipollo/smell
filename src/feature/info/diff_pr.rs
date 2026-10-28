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
         smell pr <REFERENCE> [PATH]...\n\
         Surfaces the areas of a pull request that deserve attention\n\
         during review. Resolves REFERENCE to a base/head commit pair,\n\
         then runs the same complexity diff. REFERENCE is one of: an explicit `<old>..<new>` range\n\
         (forwarded as-is), a GitHub PR URL, `owner/repo#123`, or a bare PR\n\
         number resolved against the current repo's `origin` remote. PATH\n\
         defaults to \".\" (the whole repository). Run it from the PR's\n\
         repository, or point at that repository with --repo <PATH>.\n\n\
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
         Use pr when finding the areas that deserve focus in a PR review:\n\
         it resolves a GitHub PR identifier to the base/head commits\n\
         itself (no manual `git fetch`/`merge-base`) and ties the report\n\
         to an actual review target. Use diff when both commit refs are already\n\
         in hand and only the raw complexity-delta numbers are wanted:\n\
         CI with known SHAs, comparing two tags.\n\n\
         Producing reviewer-facing focus points (see --info review) is a\n\
         separate question from which refs are in hand: prefer pr there\n\
         even when both commit refs are already known. Reach for diff\n\
         only when the low-level numbers are wanted on their own.\n\n\
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
        assert!(page.contains("smell pr <REFERENCE> [PATH]..."));
        assert!(page.contains("defaults to \".\""));
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
    fn page_frames_pr_as_review_focus_finder() {
        let page = render();
        assert!(page.contains("deserve attention\nduring review"));
        assert!(page.contains("Use pr when finding the areas"));
    }

    #[test]
    fn page_documents_shared_flags() {
        let page = render();
        assert!(page.contains("--delta-methods"));
        assert!(page.contains("--repo"));
        assert!(page.contains("--json"));
    }
}
