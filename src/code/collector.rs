//! Shared tree walker used by every language module. Languages only describe
//! which nodes open a type scope, which produce functions, and which count as
//! branches; the walk, scope tracking, and struct assembly live here.

use tree_sitter::{Language, Node, Parser};

use crate::code::{FileComplexity, FunctionComplexity, TypeComplexity};

/// What a language makes of a single syntax node.
pub enum Visit {
    /// Nothing at this node; keep descending.
    Skip,
    /// The node opens a type scope (class/struct/enum/...) for its subtree.
    Type(String),
    /// Functions produced at this node (one for plain functions, several for
    /// property accessors).
    Functions(Vec<FunctionComplexity>),
}

pub trait LanguageRules {
    fn visit(&self, node: Node, source: &str) -> Visit;
    fn is_branch(&self, node: Node, source: &str) -> bool;
}

/// Parses the source with the given grammar and assembles the file complexity.
pub fn file_complexity(
    language: &Language,
    rules: &impl LanguageRules,
    source: &str,
) -> FileComplexity {
    let mut parser = Parser::new();
    parser
        .set_language(language)
        .expect("failed to load grammar");
    let mut file = FileComplexity {
        functions: Vec::new(),
        types: Vec::new(),
    };
    let Some(tree) = parser.parse(source, None) else {
        return file;
    };
    collect(tree.root_node(), source, rules, &mut Vec::new(), &mut file);
    file
}

/// Cyclomatic complexity of a function node: a baseline of 1 for the
/// straight-line path plus one per branch node within it.
pub fn complexity(node: Node, source: &str, rules: &impl LanguageRules) -> usize {
    1 + count_branches(node, source, rules)
}

/// Counts branch nodes within (but not including) the given node.
fn count_branches(node: Node, source: &str, rules: &impl LanguageRules) -> usize {
    let mut branches = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if rules.is_branch(child, source) {
            branches += 1;
        }
        branches += count_branches(child, source, rules);
    }
    branches
}

/// Finds the first direct child of the given kind.
pub fn find_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == kind)
}

/// Extracts the source text of a named field, e.g. a declaration's `name`.
pub fn field_text(node: Node, field: &str, source: &str) -> String {
    node.child_by_field_name(field)
        .and_then(|child| child.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unknown>")
        .to_string()
}

fn collect(
    node: Node,
    source: &str,
    rules: &impl LanguageRules,
    type_stack: &mut Vec<String>,
    file: &mut FileComplexity,
) {
    let opened_type = match rules.visit(node, source) {
        Visit::Skip => false,
        Visit::Type(name) => {
            type_stack.push(qualified_name(type_stack, &name));
            true
        }
        Visit::Functions(functions) => {
            attach_functions(file, type_stack, functions);
            false
        }
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, source, rules, type_stack, file);
    }
    if opened_type {
        type_stack.pop();
    }
}

/// Nested types get dot-qualified names ("Outer.Inner").
fn qualified_name(type_stack: &[String], name: &str) -> String {
    match type_stack.last() {
        Some(outer) => format!("{outer}.{name}"),
        None => name.to_string(),
    }
}

fn attach_functions(
    file: &mut FileComplexity,
    type_stack: &[String],
    functions: Vec<FunctionComplexity>,
) {
    let Some(type_name) = type_stack.last() else {
        file.functions.extend(functions);
        return;
    };
    // Find-or-create by name so split declarations (Rust impl blocks, Swift
    // extensions) merge into a single type.
    let complexity = match file.types.iter_mut().find(|t| &t.name == type_name) {
        Some(existing) => existing,
        None => {
            file.types.push(TypeComplexity {
                name: type_name.clone(),
                functions: Vec::new(),
            });
            file.types.last_mut().expect("just pushed a type")
        }
    };
    complexity.functions.extend(functions);
}
