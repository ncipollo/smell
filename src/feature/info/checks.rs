//! The `checks` topic: `--max-*` limit checks and `--quiet` mode.

pub fn render() -> String {
    format!("{}\n{}", limit_section(), quiet_section())
}

fn limit_section() -> String {
    String::from(
        "LIMIT CHECKS\n\
         --max-complexity <N>, --max-methods <N>, --max-lines <N>, and\n\
         --max-declarations <N> (or max_complexity/max_methods/max_lines/\n\
         max_declarations in smell.toml) each make the run a check for\n\
         their measure: complexity per function, method count per type,\n\
         line count per file, declaration count per file (types plus\n\
         top-level functions). A check exits non-zero when any analyzed\n\
         subject's value is strictly greater than N (equal to N passes),\n\
         printing the offending files and subjects to stderr after the\n\
         normal report, one section per failing measure. Every check\n\
         covers whatever the other filters selected and runs\n\
         independently: any combination may be configured. Without a\n\
         limit, smell only reports and always exits zero on success.\n",
    )
}

fn quiet_section() -> String {
    String::from(
        "QUIET MODE\n\
         --quiet (or -q) suppresses the per-file complexity report on\n\
         stdout. Errors and, when --max-complexity, --max-methods,\n\
         --max-lines, or --max-declarations is set, the failure report on\n\
         stderr are still printed, so a quiet CI run stays silent on success\n\
         and prints only what a failure requires.\n",
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
    fn page_documents_quiet() {
        let page = render();
        assert!(page.contains("--quiet"));
        assert!(page.contains("silent on success"));
    }
}
