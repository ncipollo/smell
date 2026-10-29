//! The `review` topic: turning `smell pr` output into a PR
//! description, PR comments, or plain conversational text.

pub fn render() -> String {
    format!(
        "{}\n{}\n{}",
        report_section(),
        structure_section(),
        destination_section()
    )
}

fn report_section() -> String {
    String::from(
        "WHICH COMMAND TO RUN\n\
         `smell pr <reference> [path]...` finds the areas to focus on in\n\
         a PR review; run it to get review focus points.\n\n\
         WHAT THE REPORT CONTAINS\n\
         `smell pr` always prints a per-function complexity table\n\
         (added/removed/changed, before/after/Δ). Unchanged functions and\n\
         files are omitted automatically; --verbose lists every file.\n\
         Any configured --delta-* flag adds a\n\
         check-failure section for a swing in method count, line count,\n\
         declaration count, comment length, or comment count between the\n\
         two revisions (same shape as the --max-* failures in --info\n\
         checks, but relative instead of absolute).\n",
    )
}

fn structure_section() -> String {
    String::from(
        "STRUCTURE OF A REVIEW SUMMARY\n\
         Lead with a 1-2 sentence human-readable summary of what changed\n\
         and why, in plain prose — no table, no jargon. Follow it with\n\
         bullets that each name a specific place to focus review, not a\n\
         restatement of the diff.\n\n\
         Make every bullet explicit about what actually happened. A\n\
         complexity swing names the function and states that complexity\n\
         was added to it, with the before/after numbers, e.g.:\n\
         \x20 added complexity to process(), 8 to 19\n\
         A new function or call site is named outright, e.g.:\n\
         \x20 adds a call to validate() inside save()\n\
         Never summarize these as \"some complexity changed\" or \"new\n\
         functions were added\" — name the function.\n\n\
         A WHOLE NEW FILE IS ITS OWN BULLET\n\
         The table marks a file with no old-revision content `(new file)`\n\
         and prints its function count plus total complexity on a trailing\n\
         line. Don't fold that into ordinary per-function \"added\"\n\
         bullets — a function added to an existing file and a function\n\
         that arrived inside a brand new file read as the same `added`\n\
         row, but they are not the same review task: the whole file needs\n\
         a pass, not just the one function. Give it a dedicated bullet\n\
         naming the file and its total, e.g.:\n\
         \x20 new file validator.rs: 4 functions, total complexity 17\n\
         A `(removed)` file is noise unless it's large; skip small ones\n\
         the same way removed functions are skipped.\n\n\
         WHAT TO FOREGROUND\n\
         A failing --delta-* check first — it's the loudest signal and\n\
         usually the point of the run. Then any new file, named with its\n\
         total (see above) — reviewers routinely miss that a file is\n\
         wholesale new when it's buried as a string of per-function\n\
         `added` rows. Then the functions with the largest complexity Δ,\n\
         each written as a complexity-added bullet (see above); growth is\n\
         more actionable than shrinkage. Then other newly added functions\n\
         and call sites that start out complex, named explicitly. Removed\n\
         functions and small deltas are noise; skip them.\n",
    )
}

fn destination_section() -> String {
    String::from(
        "DESTINATION ADAPTS PRESENTATION, NOT THE EXPLICITNESS\n\
         PR DESCRIPTION: the summary-then-bullets structure above, in\n\
         full — lead with any failing check, in bold; follow with the\n\
         explicit complexity-added/new-call bullets. Never paste the\n\
         full table.\n\n\
         PR COMMENTS: the same explicitness, one comment per hotspot\n\
         (file or function), dropped near the relevant diff hunk instead\n\
         of one monolithic dump comment.\n\n\
         PLAIN CHAT TEXT: the same explicitness in conversational prose\n\
         instead of bullets — still name the function and the\n\
         before-to-after numbers, just without markdown structure.\n\n\
         See also: --info pr, --info checks.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_which_command_to_run() {
        let page = render();
        assert!(page.contains("WHICH COMMAND TO RUN"));
        assert!(page.contains("`smell pr <reference> [path]...` finds"));
        assert!(!page.contains("smell diff"));
    }

    #[test]
    fn page_mentions_verbose() {
        assert!(render().contains("--verbose"));
    }

    #[test]
    fn page_documents_the_summary_then_bullets_structure() {
        let page = render();
        assert!(page.contains("STRUCTURE OF A REVIEW SUMMARY"));
        assert!(page.contains("1-2 sentence human-readable summary"));
    }

    #[test]
    fn page_documents_explicit_complexity_bullets() {
        let page = render();
        assert!(page.contains("complexity\nwas added to it"));
        assert!(page.contains("added complexity to process(), 8 to 19"));
    }

    #[test]
    fn page_documents_explicit_new_call_bullets() {
        let page = render();
        assert!(page.contains("A new function or call site is named outright"));
        assert!(page.contains("adds a call to validate() inside save()"));
    }

    #[test]
    fn page_documents_what_to_foreground() {
        let page = render();
        assert!(page.contains("WHAT TO FOREGROUND"));
        assert!(page.contains("failing --delta-* check first"));
    }

    #[test]
    fn page_documents_new_files_as_their_own_bullet() {
        let page = render();
        assert!(page.contains("A WHOLE NEW FILE IS ITS OWN BULLET"));
        assert!(page.contains("(new file)"));
        assert!(page.contains("new file validator.rs: 4 functions, total complexity 17"));
    }

    #[test]
    fn page_foregrounds_new_files_ahead_of_ordinary_complexity_deltas() {
        let page = render();
        let new_file_mention = page.find("Then any new file").expect("mentions new files");
        let complexity_delta_mention = page
            .find("the functions with the largest complexity")
            .expect("mentions complexity deltas");
        assert!(new_file_mention < complexity_delta_mention);
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
