use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::branch::{BranchFilter, BranchKind, BranchRule};
use crate::code::collector;
use crate::code::collector::{FunctionDecl, LanguageRules, Visit};

pub const BRANCH_RULES: &[BranchRule] = &[
    BranchRule::new(BranchKind::If, "if_statement"),
    BranchRule::new(BranchKind::Guard, "guard_statement"),
    BranchRule::new(BranchKind::Switch, "switch_entry"),
    BranchRule::new(BranchKind::Loop, "for_statement"),
    BranchRule::new(BranchKind::Loop, "while_statement"),
    BranchRule::new(BranchKind::Loop, "repeat_while_statement"),
    BranchRule::new(BranchKind::Catch, "catch_block"),
    BranchRule::new(BranchKind::Ternary, "ternary_expression"),
    BranchRule::new(BranchKind::BooleanOperator, "conjunction_expression"),
    BranchRule::new(BranchKind::BooleanOperator, "disjunction_expression"),
    BranchRule::new(BranchKind::NullCoalescing, "nil_coalescing_expression"),
    // `try?` hides a branch to nil; plain `try` and `try!` do not create an
    // in-function branch.
    BranchRule::when(
        BranchKind::Try,
        "try_expression",
        "try operator text is `try?`",
        is_optional_try,
    ),
];

const TYPE_KINDS: &[&str] = &["class_declaration", "protocol_declaration"];

/// Parses Swift source and returns the cyclomatic complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str, filter: &BranchFilter) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_swift::LANGUAGE.into(),
        &SwiftRules,
        source,
        filter,
    )
}

struct SwiftRules;

impl LanguageRules for SwiftRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a> {
        match node.kind() {
            kind if TYPE_KINDS.contains(&kind) => {
                Visit::Type(collector::field_text(node, "name", source))
            }
            "function_declaration" => Visit::Functions(vec![FunctionDecl {
                name: collector::field_text(node, "name", source),
                body: node,
            }]),
            "property_declaration" => Visit::Functions(property_functions(node, source)),
            _ => Visit::Skip,
        }
    }

    fn branch_rules(&self) -> &'static [BranchRule] {
        BRANCH_RULES
    }
}

fn is_optional_try(node: Node, source: &str) -> bool {
    collector::find_child(node, "try_operator")
        .and_then(|operator| operator.utf8_text(source.as_bytes()).ok())
        .is_some_and(|text| text == "try?")
}

fn property_functions<'a>(node: Node<'a>, source: &str) -> Vec<FunctionDecl<'a>> {
    let name = property_name(node, source);
    let mut functions = Vec::new();
    if let Some(computed) = node.child_by_field_name("computed_value") {
        collect_computed_accessors(computed, &name, &mut functions);
    }
    if let Some(observers) = collector::find_child(node, "willset_didset_block") {
        collect_observers(observers, &name, &mut functions);
    }
    functions
}

fn collect_computed_accessors<'a>(
    computed: Node<'a>,
    name: &str,
    functions: &mut Vec<FunctionDecl<'a>>,
) {
    let mut cursor = computed.walk();
    let accessors: Vec<Node> = computed
        .children(&mut cursor)
        .filter(|child| matches!(child.kind(), "computed_getter" | "computed_setter"))
        .collect();
    if accessors.is_empty() {
        functions.push(FunctionDecl {
            name: name.to_string(),
            body: computed,
        });
        return;
    }
    for accessor in accessors {
        let suffix = if accessor.kind() == "computed_getter" {
            "get"
        } else {
            "set"
        };
        functions.push(FunctionDecl {
            name: format!("{name}.{suffix}"),
            body: accessor,
        });
    }
}

fn collect_observers<'a>(observers: Node<'a>, name: &str, functions: &mut Vec<FunctionDecl<'a>>) {
    let mut cursor = observers.walk();
    for clause in observers.children(&mut cursor) {
        let suffix = match clause.kind() {
            "willset_clause" => "willSet",
            "didset_clause" => "didSet",
            _ => continue,
        };
        functions.push(FunctionDecl {
            name: format!("{name}.{suffix}"),
            body: clause,
        });
    }
}

fn property_name(node: Node, source: &str) -> String {
    node.child_by_field_name("name")
        .and_then(|pattern| pattern.child_by_field_name("bound_identifier"))
        .and_then(|identifier| identifier.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unknown>")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::branch::BranchSpec;
    use crate::testing;

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(
            &testing::fixture("swift/complexity.swift"),
            &BranchFilter::default(),
        );
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("canThrow".to_string(), 1),
                ("simple".to_string(), 1),
                ("branchy".to_string(), 17),
            ]
        );
        assert_eq!(
            testing::type_summary(&complexity),
            vec![(
                "Shape".to_string(),
                vec![
                    ("area".to_string(), 2),
                    ("label.get".to_string(), 2),
                    ("label.set".to_string(), 2),
                    ("count.willSet".to_string(), 2),
                    ("count.didSet".to_string(), 2),
                    ("describe".to_string(), 2),
                ],
            )]
        );
    }

    #[test]
    fn file_complexity_counts_only_selected_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        let complexity = file_complexity(&testing::fixture("swift/complexity.swift"), &filter);
        // branchy: three switch entries (case 1, case 2, default).
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("canThrow".to_string(), 1),
                ("simple".to_string(), 1),
                ("branchy".to_string(), 4),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_raw_node_kinds_literally() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Raw("guard_statement".to_string())]);
        let complexity = file_complexity(&testing::fixture("swift/complexity.swift"), &filter);
        // branchy: one guard statement.
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("canThrow".to_string(), 1),
                ("simple".to_string(), 1),
                ("branchy".to_string(), 2),
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
