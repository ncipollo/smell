use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::FunctionComplexity;
use crate::code::collector;
use crate::code::collector::{LanguageRules, Visit};

const BRANCH_KINDS: &[&str] = &[
    "if_expression",
    "while_expression",
    "loop_expression",
    "for_expression",
    "match_arm",
    // The `?` operator hides an early return, so it counts as a branch.
    "try_expression",
];

const TYPE_KINDS: &[&str] = &["struct_item", "enum_item", "trait_item", "union_item"];

/// Parses Rust source and returns the branch complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str) -> FileComplexity {
    collector::file_complexity(&tree_sitter_rust::LANGUAGE.into(), &RustRules, source)
}

struct RustRules;

impl LanguageRules for RustRules {
    fn visit(&self, node: Node, source: &str) -> Visit {
        match node.kind() {
            kind if TYPE_KINDS.contains(&kind) => {
                Visit::Type(collector::field_text(node, "name", source))
            }
            // Impl blocks scope to the implemented type's name so that
            // `impl Shape` and `impl Display for Shape` merge with `Shape`.
            "impl_item" => Visit::Type(collector::field_text(node, "type", source)),
            "function_item" => Visit::Functions(vec![FunctionComplexity {
                name: collector::field_text(node, "name", source),
                branches: collector::count_branches(node, source, self),
            }]),
            _ => Visit::Skip,
        }
    }

    fn is_branch(&self, node: Node, _source: &str) -> bool {
        match node.kind() {
            "binary_expression" => is_boolean_operator(node),
            // let-else hides an early exit in its else block.
            "let_declaration" => node.child_by_field_name("alternative").is_some(),
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
        let complexity = file_complexity(&testing::fixture("rust/complexity.rs"));
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 0),
                ("branchy".to_string(), 12),
                ("fallible".to_string(), 1),
                ("maybe".to_string(), 0),
                ("parse".to_string(), 0),
            ]
        );
        assert_eq!(
            testing::type_summary(&complexity),
            vec![
                (
                    "Shape".to_string(),
                    vec![("area".to_string(), 1), ("fmt".to_string(), 1)],
                ),
                ("Kind".to_string(), vec![("label".to_string(), 2)]),
                ("Describe".to_string(), vec![("describe".to_string(), 0)]),
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
