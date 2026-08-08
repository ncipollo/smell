use tree_sitter::Node;

use crate::code::FileComplexity;
use crate::code::branch::{BranchFilter, BranchKind, BranchRule};
use crate::code::collector;
use crate::code::collector::{FunctionDecl, LanguageRules, TypeDecl, Visit};

pub const BRANCH_RULES: &[BranchRule] = &[
    BranchRule::new(BranchKind::If, "if_statement"),
    BranchRule::new(BranchKind::Switch, "switch_section"),
    BranchRule::new(BranchKind::Switch, "switch_expression_arm"),
    BranchRule::new(BranchKind::Switch, "when_clause"),
    BranchRule::new(BranchKind::Loop, "for_statement"),
    BranchRule::new(BranchKind::Loop, "foreach_statement"),
    BranchRule::new(BranchKind::Loop, "while_statement"),
    BranchRule::new(BranchKind::Loop, "do_statement"),
    BranchRule::new(BranchKind::Catch, "catch_clause"),
    BranchRule::new(BranchKind::Ternary, "conditional_expression"),
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
    BranchRule::when(
        BranchKind::NullCoalescing,
        "assignment_expression",
        "operator is ??=",
        is_null_coalescing_assignment,
    ),
];

/// Declarations opening a type scope. Enums are excluded: their members
/// cannot have bodies, so the scope would always end up function-less and be
/// dropped.
const TYPE_KINDS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "record_declaration",
    "struct_declaration",
];

/// Declarations named directly by a `name` field.
const NAMED_FUNCTION_KINDS: &[&str] = &[
    "method_declaration",
    "constructor_declaration",
    "local_function_statement",
];

/// Members that own an `accessor_list`; an accessor is named after its owner.
const ACCESSOR_OWNER_KINDS: &[&str] = &[
    "property_declaration",
    "indexer_declaration",
    "event_declaration",
];

/// Parses C# source and returns the cyclomatic complexity of each function,
/// grouped by containing type.
pub fn file_complexity(source: &str, filter: &BranchFilter) -> FileComplexity {
    collector::file_complexity(
        &tree_sitter_c_sharp::LANGUAGE.into(),
        &CSharpRules,
        source,
        filter,
    )
}

struct CSharpRules;

impl LanguageRules for CSharpRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a> {
        match node.kind() {
            kind if TYPE_KINDS.contains(&kind) => Visit::Type(TypeDecl {
                name: collector::field_text(node, "name", source),
                supertypes: supertypes(node, source),
            }),
            kind if NAMED_FUNCTION_KINDS.contains(&kind) => Visit::Functions(bodied_function(
                node,
                collector::field_text(node, "name", source),
            )),
            "destructor_declaration" => Visit::Functions(destructor(node, source)),
            "operator_declaration" => Visit::Functions(operator(node, source)),
            "conversion_operator_declaration" => {
                Visit::Functions(conversion_operator(node, source))
            }
            "accessor_declaration" => {
                Visit::Functions(bodied_function(node, accessor_name(node, source)))
            }
            "property_declaration" => Visit::Functions(expression_bodied_property(node, source)),
            "lambda_expression" => Visit::Functions(named_lambda(node, source)),
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

fn is_null_coalescing_assignment(node: Node, _source: &str) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| operator.kind() == "??=")
}

/// A declaration only counts when it has a body: interface signatures,
/// abstract and partial members, and auto-property accessors (`{ get; set; }`)
/// declare no code to walk.
fn bodied_function<'a>(node: Node<'a>, name: String) -> Vec<FunctionDecl<'a>> {
    match node.child_by_field_name("body") {
        Some(_) => vec![FunctionDecl { name, body: node }],
        None => Vec::new(),
    }
}

/// Destructors share their type's name; keep the `~` so the report can tell
/// a destructor apart from the constructor.
fn destructor<'a>(node: Node<'a>, source: &str) -> Vec<FunctionDecl<'a>> {
    let name = format!("~{}", collector::field_text(node, "name", source));
    bodied_function(node, name)
}

/// Operators have no `name` field; the `operator` field holds the token.
fn operator<'a>(node: Node<'a>, source: &str) -> Vec<FunctionDecl<'a>> {
    let name = format!(
        "operator {}",
        collector::field_text(node, "operator", source)
    );
    bodied_function(node, name)
}

/// Conversion operators name neither an identifier nor an operator token;
/// the target `type` field is what distinguishes them.
fn conversion_operator<'a>(node: Node<'a>, source: &str) -> Vec<FunctionDecl<'a>> {
    let name = format!("operator {}", collector::field_text(node, "type", source));
    bodied_function(node, name)
}

/// Accessors are named after the member they belong to, e.g. `Area.get`. The
/// `name` field is the `get`/`set`/`init`/`add`/`remove` token itself.
fn accessor_name(node: Node, source: &str) -> String {
    let owner = accessor_owner_name(node, source);
    let accessor = collector::field_text(node, "name", source);
    format!("{owner}.{accessor}")
}

fn accessor_owner_name(node: Node, source: &str) -> String {
    match ancestor(node, ACCESSOR_OWNER_KINDS) {
        Some(owner) if owner.kind() == "indexer_declaration" => "this".to_string(),
        Some(owner) => collector::field_text(owner, "name", source),
        None => "<unknown>".to_string(),
    }
}

fn ancestor<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if kinds.contains(&parent.kind()) {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

/// An expression-bodied property (`int Area => w * h;`) has no accessor
/// list, so it reports under the bare property name, matching Swift's
/// implicit getter. A property with an accessor list never has an arrow
/// `value`, so the two shapes can't double-count.
fn expression_bodied_property<'a>(node: Node<'a>, source: &str) -> Vec<FunctionDecl<'a>> {
    match node.child_by_field_name("value") {
        Some(value) if value.kind() == "arrow_expression_clause" => {
            vec![FunctionDecl {
                name: collector::field_text(node, "name", source),
                body: node,
            }]
        }
        _ => Vec::new(),
    }
}

/// Names a lambda by the variable it is directly assigned to. Lambdas passed
/// inline as arguments stay anonymous and fold into the enclosing function.
fn named_lambda<'a>(node: Node<'a>, source: &str) -> Vec<FunctionDecl<'a>> {
    match node.parent() {
        Some(parent) if parent.kind() == "variable_declarator" => vec![FunctionDecl {
            name: collector::field_text(parent, "name", source),
            body: node,
        }],
        _ => Vec::new(),
    }
}

/// Base classes and implemented interfaces share one `base_list`, matching
/// this project's single supertype key. A record's primary constructor base
/// (`: Point(X, Y)`) carries its arguments alongside the type; only the type
/// is a supertype.
fn supertypes(node: Node, source: &str) -> Vec<String> {
    let Some(base_list) = collector::find_child(node, "base_list") else {
        return Vec::new();
    };
    let mut cursor = base_list.walk();
    base_list
        .named_children(&mut cursor)
        .filter_map(|child| base_type_name(child, source))
        .collect()
}

fn base_type_name(child: Node, source: &str) -> Option<String> {
    match child.kind() {
        "argument_list" => None,
        "primary_constructor_base_type" => child
            .child_by_field_name("type")
            .map(|base| collector::node_text(base, source)),
        _ => Some(collector::node_text(child, source)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::branch::BranchSpec;
    use crate::testing;

    #[test]
    fn file_complexity_reports_functions_grouped_by_type() {
        let complexity = file_complexity(
            &testing::fixture("csharp/complexity.cs"),
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
                        ("~Complexity".to_string(), 2),
                        ("Area".to_string(), 2),
                        ("Label.get".to_string(), 2),
                        ("Label.set".to_string(), 2),
                        ("operator +".to_string(), 2),
                        ("Branchy".to_string(), 23),
                        ("Doubled".to_string(), 2),
                        ("triple".to_string(), 2),
                        ("CanThrow".to_string(), 1),
                    ],
                ),
                ("Labeled".to_string(), vec![("Label".to_string(), 2)]),
                (
                    "Celsius".to_string(),
                    vec![
                        ("Celsius".to_string(), 1),
                        ("operator Celsius".to_string(), 2),
                    ],
                ),
            ]
        );
    }

    #[test]
    fn file_complexity_reports_supertypes() {
        let complexity = file_complexity(
            &testing::fixture("csharp/inherits.cs"),
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
                ("Sub".to_string(), vec!["Describe".to_string()]),
                (
                    "Named".to_string(),
                    vec!["Point".to_string(), "Sized".to_string()],
                ),
                ("Square".to_string(), vec!["Sized".to_string()]),
                (
                    "Ranked".to_string(),
                    vec!["IComparable<Ranked>".to_string()]
                ),
            ]
        );
    }

    #[test]
    fn file_complexity_counts_only_selected_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        let complexity = file_complexity(&testing::fixture("csharp/complexity.cs"), &filter);
        // Branchy: three switch sections, three switch expression arms, and
        // one `when` pattern guard.
        assert_eq!(
            testing::type_summary(&complexity)[0],
            (
                "Complexity".to_string(),
                vec![
                    ("Complexity".to_string(), 1),
                    ("~Complexity".to_string(), 1),
                    ("Area".to_string(), 1),
                    ("Label.get".to_string(), 1),
                    ("Label.set".to_string(), 1),
                    ("operator +".to_string(), 1),
                    ("Branchy".to_string(), 8),
                    ("Doubled".to_string(), 1),
                    ("triple".to_string(), 1),
                    ("CanThrow".to_string(), 1),
                ],
            )
        );
    }

    #[test]
    fn file_complexity_counts_raw_node_kinds_literally() {
        let filter =
            BranchFilter::from_specs(&[BranchSpec::Raw("switch_expression_arm".to_string())]);
        let complexity = file_complexity(&testing::fixture("csharp/complexity.cs"), &filter);
        // Branchy: three arms in the switch expression.
        assert_eq!(
            testing::type_summary(&complexity)[0],
            (
                "Complexity".to_string(),
                vec![
                    ("Complexity".to_string(), 1),
                    ("~Complexity".to_string(), 1),
                    ("Area".to_string(), 1),
                    ("Label.get".to_string(), 1),
                    ("Label.set".to_string(), 1),
                    ("operator +".to_string(), 1),
                    ("Branchy".to_string(), 4),
                    ("Doubled".to_string(), 1),
                    ("triple".to_string(), 1),
                    ("CanThrow".to_string(), 1),
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
