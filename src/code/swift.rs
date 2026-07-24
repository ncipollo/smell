use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::FunctionComplexity;
use crate::code::collector;
use crate::code::collector::{LanguageRules, Visit};

const BRANCH_KINDS: &[&str] = &[
    "if_statement",
    "guard_statement",
    "for_statement",
    "while_statement",
    "repeat_while_statement",
    "switch_entry",
    "catch_block",
    "ternary_expression",
    "nil_coalescing_expression",
    "conjunction_expression",
    "disjunction_expression",
];

const TYPE_KINDS: &[&str] = &["class_declaration", "protocol_declaration"];

/// Parses Swift source and returns the cyclomatic complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str) -> FileComplexity {
    collector::file_complexity(&tree_sitter_swift::LANGUAGE.into(), &SwiftRules, source)
}

struct SwiftRules;

impl LanguageRules for SwiftRules {
    fn visit(&self, node: Node, source: &str) -> Visit {
        match node.kind() {
            kind if TYPE_KINDS.contains(&kind) => {
                Visit::Type(collector::field_text(node, "name", source))
            }
            "function_declaration" => Visit::Functions(vec![FunctionComplexity {
                name: collector::field_text(node, "name", source),
                complexity: collector::complexity(node, source, self),
            }]),
            "property_declaration" => Visit::Functions(property_functions(node, source)),
            _ => Visit::Skip,
        }
    }

    fn is_branch(&self, node: Node, source: &str) -> bool {
        match node.kind() {
            // `try?` hides a branch to nil; plain `try` and `try!` do not
            // create an in-function branch.
            "try_expression" => is_optional_try(node, source),
            kind => BRANCH_KINDS.contains(&kind),
        }
    }
}

fn is_optional_try(node: Node, source: &str) -> bool {
    collector::find_child(node, "try_operator")
        .and_then(|operator| operator.utf8_text(source.as_bytes()).ok())
        .is_some_and(|text| text == "try?")
}

fn property_functions(node: Node, source: &str) -> Vec<FunctionComplexity> {
    let name = property_name(node, source);
    let mut functions = Vec::new();
    if let Some(computed) = node.child_by_field_name("computed_value") {
        collect_computed_accessors(computed, source, &name, &mut functions);
    }
    if let Some(observers) = collector::find_child(node, "willset_didset_block") {
        collect_observers(observers, source, &name, &mut functions);
    }
    functions
}

fn collect_computed_accessors(
    computed: Node,
    source: &str,
    name: &str,
    functions: &mut Vec<FunctionComplexity>,
) {
    let mut cursor = computed.walk();
    let accessors: Vec<Node> = computed
        .children(&mut cursor)
        .filter(|child| matches!(child.kind(), "computed_getter" | "computed_setter"))
        .collect();
    if accessors.is_empty() {
        functions.push(FunctionComplexity {
            name: name.to_string(),
            complexity: collector::complexity(computed, source, &SwiftRules),
        });
        return;
    }
    for accessor in accessors {
        let suffix = if accessor.kind() == "computed_getter" {
            "get"
        } else {
            "set"
        };
        functions.push(FunctionComplexity {
            name: format!("{name}.{suffix}"),
            complexity: collector::complexity(accessor, source, &SwiftRules),
        });
    }
}

fn collect_observers(
    observers: Node,
    source: &str,
    name: &str,
    functions: &mut Vec<FunctionComplexity>,
) {
    let mut cursor = observers.walk();
    for clause in observers.children(&mut cursor) {
        let suffix = match clause.kind() {
            "willset_clause" => "willSet",
            "didset_clause" => "didSet",
            _ => continue,
        };
        functions.push(FunctionComplexity {
            name: format!("{name}.{suffix}"),
            complexity: collector::complexity(clause, source, &SwiftRules),
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
    use crate::testing;

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(&testing::fixture("swift/complexity.swift"));
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
    fn file_complexity_handles_empty_source() {
        let complexity = file_complexity("");
        assert!(complexity.functions.is_empty());
        assert!(complexity.types.is_empty());
    }
}
