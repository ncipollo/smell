use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::branch::{BranchFilter, BranchKind, BranchRule};
use crate::code::collector;
use crate::code::collector::{FunctionDecl, LanguageRules, TypeDecl, Visit};

pub const BRANCH_RULES: &[BranchRule] = &[
    BranchRule::new(BranchKind::If, "if_statement"),
    BranchRule::new(BranchKind::Switch, "switch_case"),
    BranchRule::new(BranchKind::Switch, "switch_default"),
    BranchRule::new(BranchKind::Loop, "for_statement"),
    BranchRule::new(BranchKind::Loop, "for_in_statement"),
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
    BranchRule::when(
        BranchKind::NullCoalescing,
        "binary_expression",
        "operator is ??",
        is_null_coalescing,
    ),
];

/// Parses JavaScript source and returns the cyclomatic complexity of each
/// function, grouped by containing class.
pub fn file_complexity(source: &str, filter: &BranchFilter) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_javascript::LANGUAGE.into(),
        &JavaScriptRules,
        source,
        filter,
    )
}

struct JavaScriptRules;

impl LanguageRules for JavaScriptRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a> {
        match node.kind() {
            "class_declaration" | "class" => Visit::Type(TypeDecl {
                name: class_name(node, source),
                supertypes: supertypes(node, source),
            }),
            "function_declaration" | "generator_function_declaration" => {
                Visit::Functions(vec![named_function(node, source)])
            }
            "method_definition" => Visit::Functions(vec![method(node, source)]),
            "arrow_function" | "function_expression" => match declarator_name(node, source) {
                Some(name) => Visit::Functions(vec![function(node, name)]),
                None => Visit::Skip,
            },
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

fn is_null_coalescing(node: Node, _source: &str) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| operator.kind() == "??")
}

fn named_function<'a>(node: Node<'a>, source: &str) -> FunctionDecl<'a> {
    function(node, collector::field_text(node, "name", source))
}

fn function<'a>(node: Node<'a>, name: String) -> FunctionDecl<'a> {
    FunctionDecl { name, body: node }
}

/// A method definition's name is its property name; getters and setters are
/// suffixed (`area.get`, `area.set`) since a class can declare both for the
/// same property. `get`/`set` are anonymous tokens preceding the `name`
/// field, distinct from a method that is merely named `get` or `set`.
fn method<'a>(node: Node<'a>, source: &str) -> FunctionDecl<'a> {
    let name = collector::field_text(node, "name", source);
    match accessor_suffix(node) {
        Some(suffix) => function(node, format!("{name}.{suffix}")),
        None => function(node, name),
    }
}

fn accessor_suffix(node: Node) -> Option<&'static str> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find_map(|child| match child.kind() {
            "get" => Some("get"),
            "set" => Some("set"),
            _ => None,
        })
}

/// Names an anonymous function by the variable it is directly assigned to
/// (`const label = () => ...`). Only the immediate parent counts, so a
/// callback nested inside that same binding's initializer (e.g. inside
/// `.map(...)`) stays anonymous and folds into the enclosing function.
fn declarator_name(node: Node, source: &str) -> Option<String> {
    let parent = node.parent()?;
    if parent.kind() != "variable_declarator" {
        return None;
    }
    Some(collector::field_text(parent, "name", source))
}

fn class_name(node: Node, source: &str) -> String {
    match node.child_by_field_name("name") {
        Some(name) => collector::node_text(name, source),
        None => declarator_name(node, source).unwrap_or_else(|| "<unknown>".to_string()),
    }
}

/// JavaScript has no interfaces, so a class names at most one supertype: the
/// single expression in its `extends` clause.
fn supertypes(node: Node, source: &str) -> Vec<String> {
    collector::find_child(node, "class_heritage")
        .and_then(|heritage| heritage.named_child(0))
        .map(|expression| vec![collector::node_text(expression, source)])
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::branch::BranchSpec;
    use crate::testing;

    #[test]
    fn file_complexity_reports_supertypes() {
        let complexity = file_complexity(
            &testing::fixture("javascript/inherits.js"),
            &BranchFilter::default(),
        );
        assert_eq!(
            testing::supertype_summary(&complexity),
            vec![
                ("Describe".to_string(), vec![]),
                ("Circle".to_string(), vec!["Describe".to_string()]),
                ("Plain".to_string(), vec![]),
                ("Wide".to_string(), vec!["ns.Container".to_string()]),
            ]
        );
    }

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(
            &testing::fixture("javascript/complexity.js"),
            &BranchFilter::default(),
        );
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 16),
                ("canThrow".to_string(), 1),
                ("double".to_string(), 2),
            ]
        );
        assert_eq!(
            testing::type_summary(&complexity),
            vec![(
                "Shape".to_string(),
                vec![
                    ("constructor".to_string(), 1),
                    ("area.get".to_string(), 2),
                    ("area.set".to_string(), 2),
                    ("describe".to_string(), 2),
                ],
            )]
        );
    }

    #[test]
    fn file_complexity_counts_only_selected_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        let complexity = file_complexity(&testing::fixture("javascript/complexity.js"), &filter);
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 4),
                ("canThrow".to_string(), 1),
                ("double".to_string(), 1),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_raw_node_kinds_literally() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Raw("catch_clause".to_string())]);
        let complexity = file_complexity(&testing::fixture("javascript/complexity.js"), &filter);
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 2),
                ("canThrow".to_string(), 1),
                ("double".to_string(), 1),
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
