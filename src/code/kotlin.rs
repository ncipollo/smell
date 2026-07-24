use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::FunctionComplexity;
use crate::code::collector;
use crate::code::collector::{LanguageRules, Visit};

const BRANCH_KINDS: &[&str] = &[
    "if_expression",
    "when_entry",
    "for_statement",
    "while_statement",
    "do_while_statement",
    "catch_block",
];

/// Parses Kotlin source and returns the branch complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_kotlin_ng::LANGUAGE.into(),
        &KotlinRules,
        source,
    )
}

struct KotlinRules;

impl LanguageRules for KotlinRules {
    fn visit(&self, node: Node, source: &str) -> Visit {
        match node.kind() {
            "class_declaration" | "object_declaration" => {
                Visit::Type(collector::field_text(node, "name", source))
            }
            "companion_object" => Visit::Type(companion_name(node, source)),
            "function_declaration" => Visit::Functions(vec![named_function(node, source)]),
            "secondary_constructor" => {
                Visit::Functions(vec![function(node, source, "constructor")])
            }
            "anonymous_initializer" => Visit::Functions(vec![function(node, source, "init")]),
            "getter" => Visit::Functions(vec![accessor(node, source, "get")]),
            "setter" => Visit::Functions(vec![accessor(node, source, "set")]),
            _ => Visit::Skip,
        }
    }

    fn is_branch(&self, node: Node, _source: &str) -> bool {
        match node.kind() {
            "binary_expression" => is_branch_operator(node),
            kind => BRANCH_KINDS.contains(&kind),
        }
    }
}

/// Boolean operators and elvis (`?:`) each add a branch.
fn is_branch_operator(node: Node) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| matches!(operator.kind(), "&&" | "||" | "?:"))
}

fn named_function(node: Node, source: &str) -> FunctionComplexity {
    function(node, source, &collector::field_text(node, "name", source))
}

fn function(node: Node, source: &str, name: &str) -> FunctionComplexity {
    FunctionComplexity {
        name: name.to_string(),
        branches: collector::count_branches(node, source, &KotlinRules),
    }
}

/// Accessors are named after the property they belong to, e.g. `label.get`.
fn accessor(node: Node, source: &str, suffix: &str) -> FunctionComplexity {
    let property = property_name(node, source);
    function(node, source, &format!("{property}.{suffix}"))
}

fn property_name(node: Node, source: &str) -> String {
    ancestor(node, "property_declaration")
        .and_then(|property| collector::find_child(property, "variable_declaration"))
        .and_then(|variable| collector::find_child(variable, "identifier"))
        .and_then(|identifier| identifier.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unknown>")
        .to_string()
}

fn ancestor<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if parent.kind() == kind {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

fn companion_name(node: Node, source: &str) -> String {
    let name = collector::field_text(node, "name", source);
    if name == "<unknown>" {
        "Companion".to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(&testing::fixture("kotlin/complexity.kt"));
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 0),
                ("branchy".to_string(), 13),
                ("canThrow".to_string(), 0),
            ]
        );
        assert_eq!(
            testing::type_summary(&complexity),
            vec![
                (
                    "Shape".to_string(),
                    vec![
                        ("area.get".to_string(), 1),
                        ("label.get".to_string(), 1),
                        ("label.set".to_string(), 1),
                        ("init".to_string(), 1),
                        ("constructor".to_string(), 1),
                        ("describe".to_string(), 1),
                    ],
                ),
                ("Shape.Companion".to_string(), vec![("unit".to_string(), 0)]),
                ("Registry".to_string(), vec![("register".to_string(), 0)]),
                ("Labeled".to_string(), vec![("label".to_string(), 1)]),
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
