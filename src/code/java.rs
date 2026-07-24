use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::FunctionComplexity;
use crate::code::collector;
use crate::code::collector::{LanguageRules, Visit};

const BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "for_statement",
    "enhanced_for_statement",
    "while_statement",
    "do_statement",
    "switch_block_statement_group",
    "switch_rule",
    "catch_clause",
    "ternary_expression",
];

const TYPE_KINDS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
    "annotation_type_declaration",
];

const FUNCTION_KINDS: &[&str] = &[
    "method_declaration",
    "constructor_declaration",
    "compact_constructor_declaration",
];

/// Parses Java source and returns the branch complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str) -> FileComplexity {
    collector::file_complexity(&tree_sitter_java::LANGUAGE.into(), &JavaRules, source)
}

struct JavaRules;

impl LanguageRules for JavaRules {
    fn visit(&self, node: Node, source: &str) -> Visit {
        match node.kind() {
            kind if TYPE_KINDS.contains(&kind) => {
                Visit::Type(collector::field_text(node, "name", source))
            }
            kind if FUNCTION_KINDS.contains(&kind) => Visit::Functions(vec![FunctionComplexity {
                name: collector::field_text(node, "name", source),
                branches: collector::count_branches(node, source, self),
            }]),
            _ => Visit::Skip,
        }
    }

    fn is_branch(&self, node: Node, _source: &str) -> bool {
        match node.kind() {
            "binary_expression" => is_boolean_operator(node),
            kind => BRANCH_KINDS.contains(&kind),
        }
    }
}

fn is_boolean_operator(node: Node) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| matches!(operator.kind(), "&&" | "||"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(&testing::fixture("java/Complexity.java"));
        assert!(complexity.functions.is_empty());
        assert_eq!(
            testing::type_summary(&complexity),
            vec![
                (
                    "Complexity".to_string(),
                    vec![
                        ("Complexity".to_string(), 1),
                        ("branchy".to_string(), 16),
                        ("canThrow".to_string(), 0),
                    ],
                ),
                ("Labeled".to_string(), vec![("label".to_string(), 1)]),
                ("Kind".to_string(), vec![("isCircle".to_string(), 0)]),
                ("Point".to_string(), vec![("Point".to_string(), 1)]),
            ]
        );
    }

    #[test]
    fn file_complexity_handles_empty_source() {
        let complexity = file_complexity("");
        assert!(complexity.functions.is_empty());
        assert!(complexity.types.is_empty());
    }
}
