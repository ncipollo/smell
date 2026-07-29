use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::branch::{BranchFilter, BranchKind, BranchRule};
use crate::code::collector;
use crate::code::collector::{FunctionDecl, LanguageRules, Visit};

pub const BRANCH_RULES: &[BranchRule] = &[
    BranchRule::new(BranchKind::If, "if_expression"),
    // let-else hides an early exit in its else block.
    BranchRule::when(
        BranchKind::Guard,
        "let_declaration",
        "has an else block (let-else)",
        has_let_else,
    ),
    BranchRule::new(BranchKind::Switch, "match_arm"),
    // A guard adds a decision on top of its match arm.
    BranchRule::when(
        BranchKind::Switch,
        "match_pattern",
        "has a guard condition",
        has_match_guard,
    ),
    BranchRule::new(BranchKind::Loop, "while_expression"),
    BranchRule::new(BranchKind::Loop, "loop_expression"),
    BranchRule::new(BranchKind::Loop, "for_expression"),
    BranchRule::when(
        BranchKind::BooleanOperator,
        "binary_expression",
        "operator is && or ||",
        is_boolean_operator,
    ),
    // The `?` operator hides an early return, so it counts as a branch.
    BranchRule::new(BranchKind::Try, "try_expression"),
];

const TYPE_KINDS: &[&str] = &["struct_item", "enum_item", "trait_item", "union_item"];

/// Parses Rust source and returns the cyclomatic complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str, filter: &BranchFilter) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_rust::LANGUAGE.into(),
        &RustRules,
        source,
        filter,
    )
}

struct RustRules;

impl LanguageRules for RustRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a> {
        match node.kind() {
            kind if TYPE_KINDS.contains(&kind) => {
                Visit::Type(collector::field_text(node, "name", source))
            }
            // Impl blocks scope to the implemented type's name so that
            // `impl Shape` and `impl Display for Shape` merge with `Shape`.
            "impl_item" => Visit::Type(collector::field_text(node, "type", source)),
            "function_item" => Visit::Functions(vec![FunctionDecl {
                name: collector::field_text(node, "name", source),
                body: node,
            }]),
            _ => Visit::Skip,
        }
    }

    fn branch_rules(&self) -> &'static [BranchRule] {
        BRANCH_RULES
    }
}

fn has_let_else(node: Node, _source: &str) -> bool {
    node.child_by_field_name("alternative").is_some()
}

fn has_match_guard(node: Node, _source: &str) -> bool {
    node.child_by_field_name("condition").is_some()
}

fn is_boolean_operator(node: Node, _source: &str) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| matches!(operator.kind(), "&&" | "||"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::branch::BranchSpec;
    use crate::testing;

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(
            &testing::fixture("rust/complexity.rs"),
            &BranchFilter::default(),
        );
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 15),
                ("fallible".to_string(), 2),
                ("maybe".to_string(), 1),
                ("parse".to_string(), 1),
            ]
        );
        assert_eq!(
            testing::type_summary(&complexity),
            vec![
                (
                    "Shape".to_string(),
                    vec![("area".to_string(), 2), ("fmt".to_string(), 2)],
                ),
                ("Kind".to_string(), vec![("label".to_string(), 3)]),
                ("Describe".to_string(), vec![("describe".to_string(), 1)]),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_only_selected_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        let complexity = file_complexity(&testing::fixture("rust/complexity.rs"), &filter);
        // branchy: four match arms plus one match guard.
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 6),
                ("fallible".to_string(), 1),
                ("maybe".to_string(), 1),
                ("parse".to_string(), 1),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_raw_node_kinds_literally() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Raw("if_expression".to_string())]);
        let complexity = file_complexity(&testing::fixture("rust/complexity.rs"), &filter);
        // branchy: three if expressions (the else-if chain nests one).
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 4),
                ("fallible".to_string(), 1),
                ("maybe".to_string(), 1),
                ("parse".to_string(), 1),
            ]
        );
    }

    #[test]
    fn file_complexity_handles_empty_source() {
        let complexity = file_complexity("", &BranchFilter::default());
        assert!(complexity.functions.is_empty());
        assert!(complexity.types.is_empty());
    }
}
