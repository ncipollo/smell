//! The `filters` topic: file glob semantics and `--implements` type
//! filtering.

pub fn render() -> String {
    format!("{}\n{}", glob_section(), implements_section())
}

fn glob_section() -> String {
    String::from(
        "FILE GLOBS\n\
         --include and --exclude take glob patterns (repeatable). A file is\n\
         analyzed when it matches any include pattern (or none were given)\n\
         and no exclude pattern. Patterns match against the path relative to\n\
         the analysis root, so `**/generated/**` behaves the same regardless\n\
         of the current directory. `*` also crosses directory separators, so\n\
         `*.rs` matches nested files. A single explicit file argument\n\
         bypasses the filters entirely.\n",
    )
}

fn implements_section() -> String {
    String::from(
        "TYPE FILTERING\n\
         --implements <NAME> (repeatable) analyzes only types that implement\n\
         or extend the named supertype: one key covers interfaces,\n\
         protocols, traits, and superclasses (Swift inheritance clauses and\n\
         Kotlin delegation specifiers do not syntactically distinguish\n\
         them). Multiple names OR together. Generic arguments are stripped\n\
         from both sides, so `Comparable<String>` matches `Comparable`; a\n\
         name matches a supertype's full text or its trailing simple name\n\
         (`Display` matches `std::fmt::Display`). Top-level functions\n\
         implement nothing, so any selection drops them, and files left\n\
         with no matching types are omitted. Matching is per type: in Rust,\n\
         if any impl block matches, all of the type's functions are\n\
         included.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_implements() {
        let page = render();
        assert!(page.contains("--implements"));
        assert!(page.contains("`Comparable<String>` matches `Comparable`"));
    }
}
