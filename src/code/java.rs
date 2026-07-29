use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::branch::{BranchFilter, BranchKind, BranchRule};
use crate::code::collector;
use crate::code::collector::{FunctionDecl, LanguageRules, Visit};

pub const BRANCH_RULES: &[BranchRule] = &[
    BranchRule::new(BranchKind::If, "if_statement"),
    BranchRule::new(BranchKind::Switch, "switch_block_statement_group"),
    BranchRule::new(BranchKind::Switch, "switch_rule"),
    BranchRule::new(BranchKind::Loop, "for_statement"),
    BranchRule::new(BranchKind::Loop, "enhanced_for_statement"),
    BranchRule::new(BranchKind::Loop, "while_statement"),
    BranchRule::new(BranchKind::Loop, "do_statement"),
    BranchRule::new(BranchKind::Catch, "catch_clause"),
    BranchRule::new(BranchKind::Ternary, "ternary_expression"),
    BranchRule::when(
        BranchKind::BooleanOperator,
        "binary_expression",
        "operator is && or ||",
        is_boolean_operator,
    ),
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

/// Parses Java source and returns the cyclomatic complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str, filter: &BranchFilter) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_java::LANGUAGE.into(),
        &JavaRules,
        source,
        filter,
    )
}

struct JavaRules;

impl LanguageRules for JavaRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a> {
        match node.kind() {
            kind if TYPE_KINDS.contains(&kind) => {
                Visit::Type(collector::field_text(node, "name", source))
            }
            kind if FUNCTION_KINDS.contains(&kind) => Visit::Functions(vec![FunctionDecl {
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
            &testing::fixture("java/Complexity.java"),
            &BranchFilter::default(),
        );
        assert!(complexity.functions.is_empty());
        assert_eq!(
            testing::type_summary(&complexity),
            vec![
                (
                    "Complexity".to_string(),
                    vec![
                        ("Complexity".to_string(), 2),
                        ("branchy".to_string(), 17),
                        ("canThrow".to_string(), 1),
                    ],
                ),
                ("Labeled".to_string(), vec![("label".to_string(), 2)]),
                ("Kind".to_string(), vec![("isCircle".to_string(), 1)]),
                ("Point".to_string(), vec![("Point".to_string(), 2)]),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_only_selected_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        let complexity = file_complexity(&testing::fixture("java/Complexity.java"), &filter);
        // branchy: three classic switch groups plus two arrow switch rules.
        assert_eq!(
            testing::type_summary(&complexity)[0],
            (
                "Complexity".to_string(),
                vec![
                    ("Complexity".to_string(), 1),
                    ("branchy".to_string(), 6),
                    ("canThrow".to_string(), 1),
                ],
            )
        );
    }

    #[test]
    fn file_complexity_counts_raw_node_kinds_literally() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Raw("ternary_expression".to_string())]);
        let complexity = file_complexity(&testing::fixture("java/Complexity.java"), &filter);
        // branchy: one ternary expression.
        assert_eq!(
            testing::type_summary(&complexity)[0],
            (
                "Complexity".to_string(),
                vec![
                    ("Complexity".to_string(), 1),
                    ("branchy".to_string(), 2),
                    ("canThrow".to_string(), 1),
                ],
            )
        );
    }

    #[test]
    fn file_complexity_handles_empty_source() {
        let complexity = file_complexity("", &BranchFilter::default());
        assert!(complexity.functions.is_empty());
        assert!(complexity.types.is_empty());
    }
}
