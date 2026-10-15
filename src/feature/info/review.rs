//! The `review` topic: turning `smell diff`/`smell pr` output into a PR
//! description, PR comments, or plain conversational text.

pub fn render() -> String {
    format!("{}\n{}", report_section(), destination_section())
}

fn report_section() -> String {
    String::from(
        "WHAT THE REPORT CONTAINS\n\
         `smell diff`/`smell pr` always print a per-function complexity\n\
         table (added/removed/changed, before/after/Δ; unchanged functions\n\
         are omitted automatically). Any configured --delta-* flag adds a\n\
         check-failure section for a swing in method count, line count,\n\
         declaration count, comment length, or comment count between the\n\
         two revisions (same shape as the --max-* failures in --info\n\
         checks, but relative instead of absolute).\n\n\
         WHAT TO FOREGROUND\n\
         A failing --delta-* check first — it's the loudest signal and\n\
         usually the point of the run. Then the functions with the\n\
         largest complexity Δ: growth is more actionable than shrinkage.\n\
         Then newly added functions that start out complex. Removed\n\
         functions and small deltas are noise; skip them.\n",
    )
}

fn destination_section() -> String {
    String::from(
        "DESTINATION CHANGES THE SHAPE, NOT THE SUBSTANCE\n\
         PR DESCRIPTION: a compact \"complexity impact\" section. Lead with\n\
         any failing check, in bold; follow with 2-4 bullet highlights of\n\
         the biggest swings. Never paste the full table.\n\n\
         PR COMMENTS: one comment per hotspot (file or function), dropped\n\
         near the relevant diff hunk rather than one monolithic dump\n\
         comment. Keep each comment to the function name and its\n\
         before-to-after delta.\n\n\
         PLAIN CHAT TEXT: conversational prose, no markdown tables or\n\
         headers. Cite the 1-3 figures that matter, e.g. \"process()'s\n\
         complexity went from 8 to 19\".\n\n\
         See also: --info diff-pr, --info checks.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_what_to_foreground() {
        let page = render();
        assert!(page.contains("WHAT TO FOREGROUND"));
        assert!(page.contains("failing --delta-* check first"));
    }

    #[test]
    fn page_documents_pr_description() {
        assert!(render().contains("PR DESCRIPTION"));
    }

    #[test]
    fn page_documents_pr_comments() {
        let page = render();
        assert!(page.contains("PR COMMENTS"));
        assert!(page.contains("one comment per hotspot"));
    }

    #[test]
    fn page_documents_plain_chat_text() {
        let page = render();
        assert!(page.contains("PLAIN CHAT TEXT"));
        assert!(page.contains("conversational"));
    }
}
