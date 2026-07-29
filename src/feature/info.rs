//! Vocabulary documentation for `--info`, assembled from the same rule
//! tables that drive counting so the docs cannot drift from behavior.

use crate::code::branch::{BranchKind, BranchRule};
use crate::code::{java, kotlin, rust, swift};

const LANGUAGES: &[(&str, &[BranchRule])] = &[
    ("Java", java::BRANCH_RULES),
    ("Kotlin", kotlin::BRANCH_RULES),
    ("Rust", rust::BRANCH_RULES),
    ("Swift", swift::BRANCH_RULES),
];

const CONFIG_EXAMPLE: &str = "\
[[rule]]
name = \"default\"
include = [\"*.rs\"]
exclude = [\"**/generated/**\"]
branches = [\"switch\", \"boolean-operator\"]
";

/// Renders the full vocabulary documentation: friendly branch kinds, the
/// per-language node kinds behind them, raw escape-hatch semantics, and glob
/// filtering rules. Written to assist AI agents composing smell commands.
pub fn text() -> String {
    let sections = [
        branch_kinds_section(),
        language_sections(),
        raw_section(),
        glob_section(),
        config_section(),
    ];
    sections.join("\n")
}

fn branch_kinds_section() -> String {
    let mut section = String::from(
        "BRANCH KINDS\n\
         Friendly, cross-language names accepted by --branches. Selecting any\n\
         kinds replaces the default of counting everything.\n\n",
    );
    for kind in BranchKind::ALL {
        section.push_str(&format!("  {:<18} {}\n", kind.name(), kind.description()));
    }
    section
}

fn language_sections() -> String {
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

fn raw_section() -> String {
    String::from(
        "RAW NODE KINDS\n\
         Any --branches value that is not a friendly kind is treated as a raw\n\
         tree-sitter node kind and matched literally against node.kind(),\n\
         independent of the classifier. Raw matches skip the conditions above:\n\
         for example `--branches binary_expression` counts all binary\n\
         expressions, not just && and ||. Friendly names carry the dynamic\n\
         logic; prefer them unless you need a node kind with no friendly name.\n",
    )
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

fn config_section() -> String {
    format!(
        "CONFIG FILE\n\
         An optional smell.toml in the directory smell is invoked from (not\n\
         necessarily the analyzed path) declares named [[rule]] entries.\n\
         --rule <NAME> selects one; without it, the rule named \"default\" is\n\
         used if present, else the built-in defaults (a config file's mere\n\
         presence does not change a bare `smell <path>` invocation). Explicit\n\
         --include/--exclude/--branches flags replace a rule's value for that\n\
         field entirely rather than merging with it.\n\n{CONFIG_EXAMPLE}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::complexity::config::Config;

    #[test]
    fn text_documents_every_branch_kind() {
        let text = text();
        for kind in BranchKind::ALL {
            assert!(
                text.contains(kind.name()),
                "missing branch kind: {}",
                kind.name()
            );
        }
    }

    #[test]
    fn text_documents_every_language_rule() {
        let text = text();
        for (language, rules) in LANGUAGES {
            assert!(text.contains(language), "missing language: {language}");
            for rule in *rules {
                assert!(
                    text.contains(rule.node_kind),
                    "missing node kind: {}",
                    rule.node_kind
                );
                if let Some(condition) = &rule.condition {
                    assert!(
                        text.contains(condition.description),
                        "missing condition: {}",
                        condition.description
                    );
                }
            }
        }
    }

    #[test]
    fn config_example_deserializes() {
        let config: Config = toml::from_str(CONFIG_EXAMPLE).expect("example is valid config");
        assert_eq!(config.rules.len(), 1);
        assert_eq!(config.rules[0].name, "default");
    }

    #[test]
    fn config_example_cites_real_branch_kinds() {
        let config: Config = toml::from_str(CONFIG_EXAMPLE).expect("example is valid config");
        for branch in &config.rules[0].branches {
            assert!(
                BranchKind::from_name(branch).is_some(),
                "not a branch kind: {branch}"
            );
        }
    }

    #[test]
    fn text_includes_the_config_example() {
        assert!(text().contains(CONFIG_EXAMPLE));
    }
}
