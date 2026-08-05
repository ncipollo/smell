use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::branch::{BranchFilter, BranchKind, BranchRule};
use crate::code::collector;
use crate::code::collector::{FunctionDecl, LanguageRules, TypeDecl, Visit};

pub const BRANCH_RULES: &[BranchRule] = &[
    BranchRule::new(BranchKind::If, "if_statement"),
    BranchRule::new(BranchKind::If, "elif_clause"),
    BranchRule::new(BranchKind::Switch, "case_clause"),
    BranchRule::when(
        BranchKind::Switch,
        "if_clause",
        "guarding a case clause",
        is_case_guard,
    ),
    BranchRule::when(
        BranchKind::If,
        "if_clause",
        "condition inside a comprehension",
        is_comprehension_condition,
    ),
    BranchRule::new(BranchKind::Loop, "for_statement"),
    BranchRule::new(BranchKind::Loop, "while_statement"),
    BranchRule::new(BranchKind::Catch, "except_clause"),
    BranchRule::new(BranchKind::Ternary, "conditional_expression"),
    BranchRule::new(BranchKind::BooleanOperator, "boolean_operator"),
];

/// Parses Python source and returns the cyclomatic complexity of each
/// function, grouped by containing class.
pub fn file_complexity(source: &str, filter: &BranchFilter) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_python::LANGUAGE.into(),
        &PythonRules,
        source,
        filter,
    )
}

struct PythonRules;

impl LanguageRules for PythonRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a> {
        match node.kind() {
            "class_definition" => Visit::Type(TypeDecl {
                name: collector::field_text(node, "name", source),
                supertypes: supertypes(node, source),
            }),
            "function_definition" => Visit::Functions(vec![FunctionDecl {
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

/// An `if_clause` is either the guard on a `case` clause or the condition
/// inside a comprehension; a guard adds a decision on top of its case.
fn is_case_guard(node: Node, _source: &str) -> bool {
    node.parent()
        .is_some_and(|parent| parent.kind() == "case_clause")
}

fn is_comprehension_condition(node: Node, source: &str) -> bool {
    !is_case_guard(node, source)
}

/// Base classes from a definition's `superclasses` argument list. Keyword
/// arguments like `metaclass=...` are not bases and are skipped; generic
/// bases (`Comparable[int]`) pass through as their raw text.
fn supertypes(node: Node, source: &str) -> Vec<String> {
    let Some(superclasses) = node.child_by_field_name("superclasses") else {
        return Vec::new();
    };
    let mut cursor = superclasses.walk();
    superclasses
        .named_children(&mut cursor)
        .filter(|child| !matches!(child.kind(), "keyword_argument" | "comment"))
        .map(|child| collector::node_text(child, source))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::branch::BranchSpec;
    use crate::testing;

    #[test]
    fn file_complexity_reports_supertypes() {
        let complexity = file_complexity(
            &testing::fixture("python/inherits.py"),
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
                ("Ranked".to_string(), vec!["Comparable[int]".to_string()]),
                ("Registry".to_string(), vec!["Describe".to_string()]),
            ]
        );
    }

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(
            &testing::fixture("python/complexity.py"),
            &BranchFilter::default(),
        );
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 15),
                ("can_throw".to_string(), 1),
            ]
        );
        assert_eq!(
            testing::type_summary(&complexity),
            vec![
                (
                    "Shape".to_string(),
                    vec![
                        ("__init__".to_string(), 2),
                        ("area".to_string(), 2),
                        ("describe".to_string(), 2),
                        ("resize".to_string(), 2),
                        ("unit".to_string(), 1),
                    ],
                ),
                (
                    "Shape.Config".to_string(),
                    vec![("validate".to_string(), 1)],
                ),
                ("Registry".to_string(), vec![("register".to_string(), 1)]),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_only_selected_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        let complexity = file_complexity(&testing::fixture("python/complexity.py"), &filter);
        // branchy: three case clauses plus one case guard.
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 5),
                ("can_throw".to_string(), 1),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_raw_node_kinds_literally() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Raw("except_clause".to_string())]);
        let complexity = file_complexity(&testing::fixture("python/complexity.py"), &filter);
        // branchy: one except clause.
        assert_eq!(
            testing::top_level_summary(&complexity),
            vec![
                ("simple".to_string(), 1),
                ("branchy".to_string(), 2),
                ("can_throw".to_string(), 1),
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
