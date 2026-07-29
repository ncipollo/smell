use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::branch::{BranchFilter, BranchKind, BranchRule};
use crate::code::collector;
use crate::code::collector::{FunctionDecl, LanguageRules, TypeDecl, Visit};

pub const BRANCH_RULES: &[BranchRule] = &[
    BranchRule::new(BranchKind::If, "if_expression"),
    BranchRule::new(BranchKind::Switch, "when_entry"),
    BranchRule::new(BranchKind::Loop, "for_statement"),
    BranchRule::new(BranchKind::Loop, "while_statement"),
    BranchRule::new(BranchKind::Loop, "do_while_statement"),
    BranchRule::new(BranchKind::Catch, "catch_block"),
    BranchRule::when(
        BranchKind::BooleanOperator,
        "binary_expression",
        "operator is && or ||",
        is_boolean_operator,
    ),
    BranchRule::when(
        BranchKind::NullCoalescing,
        "binary_expression",
        "operator is the elvis operator ?:",
        is_elvis_operator,
    ),
];

/// Parses Kotlin source and returns the cyclomatic complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str, filter: &BranchFilter) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_kotlin_ng::LANGUAGE.into(),
        &KotlinRules,
        source,
        filter,
    )
}

struct KotlinRules;

impl LanguageRules for KotlinRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a> {
        match node.kind() {
            "class_declaration" | "object_declaration" => Visit::Type(TypeDecl {
                name: collector::field_text(node, "name", source),
                supertypes: supertypes(node, source),
            }),
            "companion_object" => Visit::Type(TypeDecl {
                name: companion_name(node, source),
                supertypes: supertypes(node, source),
            }),
            "function_declaration" => Visit::Functions(vec![named_function(node, source)]),
            "secondary_constructor" => Visit::Functions(vec![function(node, "constructor")]),
            "anonymous_initializer" => Visit::Functions(vec![function(node, "init")]),
            "getter" => Visit::Functions(vec![accessor(node, source, "get")]),
            "setter" => Visit::Functions(vec![accessor(node, source, "set")]),
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

/// The elvis operator (`?:`) branches on null.
fn is_elvis_operator(node: Node, _source: &str) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| operator.kind() == "?:")
}

fn named_function<'a>(node: Node<'a>, source: &str) -> FunctionDecl<'a> {
    function(node, &collector::field_text(node, "name", source))
}

fn function<'a>(node: Node<'a>, name: &str) -> FunctionDecl<'a> {
    FunctionDecl {
        name: name.to_string(),
        body: node,
    }
}

/// Accessors are named after the property they belong to, e.g. `label.get`.
fn accessor<'a>(node: Node<'a>, source: &str, suffix: &str) -> FunctionDecl<'a> {
    let property = property_name(node, source);
    function(node, &format!("{property}.{suffix}"))
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

/// Superclasses and interfaces from a declaration's delegation specifiers.
/// A specifier holds a constructor invocation (`Base(1)` — take just the
/// type), an explicit delegation (`Describe by impl` — take just the type),
/// or a bare type; annotations are skipped.
fn supertypes(node: Node, source: &str) -> Vec<String> {
    let Some(specifiers) = collector::find_child(node, "delegation_specifiers") else {
        return Vec::new();
    };
    let mut cursor = specifiers.walk();
    specifiers
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "delegation_specifier")
        .filter_map(|specifier| supertype_name(specifier, source))
        .collect()
}

fn supertype_name(specifier: Node, source: &str) -> Option<String> {
    let inner = specifier
        .named_child(0)
        .filter(|child| child.kind() != "annotation")?;
    let named = match inner.kind() {
        "constructor_invocation" | "explicit_delegation" => inner.named_child(0)?,
        _ => inner,
    };
    Some(collector::node_text(named, source))
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
    use crate::code::branch::BranchSpec;
    use crate::testing;

    #[test]
    fn file_complexity_reports_supertypes() {
        let complexity = file_complexity(
            &testing::fixture("kotlin/inherits.kt"),
            &BranchFilter::default(),
        );
        assert_eq!(
            testing::supertype_summary(&complexity),
            vec![
                ("Describe".to_string(), vec![]),
                ("Base".to_string(), vec![]),
                (
                    "Circle".to_string(),
                    vec!["Base".to_string(), "Describe".to_string()],
                ),
                ("Plain".to_string(), vec![]),
                ("Ranked".to_string(), vec!["Comparable<Ranked>".to_string()]),
                ("Registry".to_string(), vec!["Describe".to_string()]),
            ]
        );
    }

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(
            &testing::fixture("kotlin/complexity.kt"),
            &BranchFilter::default(),
        );
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 14),
                ("canThrow".to_string(), 1),
            ]
        );
        assert_eq!(
            testing::type_summary(&complexity),
            vec![
                (
                    "Shape".to_string(),
                    vec![
                        ("area.get".to_string(), 2),
                        ("label.get".to_string(), 2),
                        ("label.set".to_string(), 2),
                        ("init".to_string(), 2),
                        ("constructor".to_string(), 2),
                        ("describe".to_string(), 2),
                    ],
                ),
                ("Shape.Companion".to_string(), vec![("unit".to_string(), 1)]),
                ("Registry".to_string(), vec![("register".to_string(), 1)]),
                ("Labeled".to_string(), vec![("label".to_string(), 2)]),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_only_selected_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        let complexity = file_complexity(&testing::fixture("kotlin/complexity.kt"), &filter);
        // branchy: three when entries (1, 2, else).
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 4),
                ("canThrow".to_string(), 1),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_raw_node_kinds_literally() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Raw("catch_block".to_string())]);
        let complexity = file_complexity(&testing::fixture("kotlin/complexity.kt"), &filter);
        // branchy: one catch block.
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 2),
                ("canThrow".to_string(), 1),
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
