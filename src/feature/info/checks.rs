//! The `checks` topic: `--max-*` limit checks and `--quiet` mode.

pub fn render() -> String {
    format!("{}\n{}", limit_section(), quiet_section())
}

fn limit_section() -> String {
    String::from(
        "LIMIT CHECKS\n\
         --max-complexity <N>, --max-methods <N>, --max-lines <N>,\n\
         --max-declarations <N>, and --max-comment-lines <N> (or\n\
         max_complexity/max_methods/max_lines/max_declarations/\n\
         max_comment_lines in smell.toml) each make the run a check for\n\
         their measure: complexity per function, method count per type,\n\
         line count per file, declaration count per file (types plus\n\
         top-level functions), comment run length per run. A check\n\
         exits non-zero when any analyzed subject's value is strictly\n\
         greater than N (equal to N passes), printing the offending\n\
         files and subjects to stderr after the normal report, one\n\
         section per failing measure. Every check covers whatever the\n\
         other filters selected and runs independently: any combination\n\
         may be configured. Without a limit, smell only reports and\n\
         always exits zero on success.\n",
    )
}

fn quiet_section() -> String {
    String::from(
        "QUIET MODE\n\
         --quiet (or -q) suppresses the per-file complexity report on\n\
         stdout. Errors and, when --max-complexity, --max-methods,\n\
         --max-lines, --max-declarations, or --max-comment-lines is set,\n\
         the failure report on stderr are still printed, so a quiet CI run\n\
         stays silent on success and prints only what a failure requires.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_max_complexity() {
        let page = render();
        assert!(page.contains("--max-complexity"));
        assert!(page.contains("exits non-zero"));
    }

    #[test]
    fn page_documents_max_methods() {
        assert!(render().contains("--max-methods"));
    }

    #[test]
    fn page_documents_max_lines() {
        assert!(render().contains("--max-lines"));
    }

    #[test]
    fn page_documents_max_declarations() {
        assert!(render().contains("--max-declarations"));
    }

    #[test]
    fn page_documents_max_comment_lines() {
        assert!(render().contains("--max-comment-lines"));
    }

    #[test]
    fn page_documents_quiet() {
        let page = render();
        assert!(page.contains("--quiet"));
        assert!(page.contains("silent on success"));
    }
}
