//! The `languages` topic: per-language tree-sitter node kinds behind each
//! friendly branch kind, generated from the same `BRANCH_RULES` tables that
//! drive counting.

use crate::code::branch::BranchRule;
use crate::code::{csharp, java, javascript, kotlin, python, rust, swift, typescript};

const LANGUAGES: &[(&str, &[BranchRule])] = &[
    ("C#", csharp::BRANCH_RULES),
    ("Java", java::BRANCH_RULES),
    ("JavaScript", javascript::BRANCH_RULES),
    ("Kotlin", kotlin::BRANCH_RULES),
    ("Python", python::BRANCH_RULES),
    ("Rust", rust::BRANCH_RULES),
    ("Swift", swift::BRANCH_RULES),
    ("TypeScript", typescript::BRANCH_RULES),
];

pub fn render() -> String {
    let mut section = String::from(
        "LANGUAGE RULES\n\
         The tree-sitter node kinds each friendly kind maps to, per language.\n\
         A parenthesized note means the node only counts when that condition\n\
         holds.\n",
    );
    for (language, rules) in LANGUAGES {
        section.push_str(&format!("\n  {language}\n"));
        for rule in *rules {
            section.push_str(&format!(
                "    {:<18} {}{}\n",
                rule.kind.name(),
                rule.node_kind,
                match &rule.condition {
                    Some(condition) => format!(" (when {})", condition.description),
                    None => String::new(),
                }
            ));
        }
    }
    section
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_every_language_rule() {
        let page = render();
        for (language, rules) in LANGUAGES {
            assert!(page.contains(language), "missing language: {language}");
            for rule in *rules {
                assert!(
                    page.contains(rule.node_kind),
                    "missing node kind: {}",
                    rule.node_kind
                );
                if let Some(condition) = &rule.condition {
                    assert!(
                        page.contains(condition.description),
                        "missing condition: {}",
                        condition.description
                    );
                }
            }
        }
    }
}
