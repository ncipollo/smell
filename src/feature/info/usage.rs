//! The `usage` topic: invocation forms, piping a diff via stdin, output
//! modes, and exit-code behavior.

pub fn render() -> String {
    String::from(
        "USAGE\n\
         smell [OPTIONS] <PATH>...\n\n\
         PATH accepts files or directories (directories are searched\n\
         recursively); multiple paths are merged into one report.\n\n\
         STDIN / DIFF MODE\n\
         A single `-` argument reads newline-separated paths from stdin\n\
         instead, so a diff's changed files can be piped straight in:\n\n\
         \x20 git diff --name-only <base> | smell -\n\n\
         Unsupported or filtered-out entries in that list are silently\n\
         skipped rather than erroring, since a raw `git diff` file list\n\
         mixes in files smell can't analyze (README.md, generated code,\n\
         binaries).\n\n\
         OUTPUT MODES\n\
         The default is a per-file table report on stdout. --quiet (-q)\n\
         suppresses it, leaving only errors and any --max-* failure report\n\
         (see --info checks). --json prints a single JSON document instead\n\
         of the table, with any check result embedded; --json and --quiet\n\
         conflict.\n\n\
         RULE SELECTION\n\
         --rule <NAME> picks a named rule from smell.toml (see --info\n\
         config).\n\n\
         EXIT CODES\n\
         Zero on success. Non-zero if any path fails to read (the rest of\n\
         the paths still get analyzed) or if any configured --max-* check\n\
         fails.\n\n\
         See also: --info config, --info languages, --info branches,\n\
         --info filters, --info checks.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_diff_piping() {
        let page = render();
        assert!(page.contains("git diff"));
        assert!(page.contains("smell -"));
    }

    #[test]
    fn page_documents_output_modes() {
        let page = render();
        assert!(page.contains("--quiet"));
        assert!(page.contains("--json"));
    }

    #[test]
    fn page_documents_exit_codes() {
        let page = render();
        assert!(page.contains("EXIT CODES"));
        assert!(page.contains("Non-zero"));
    }
}
