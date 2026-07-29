//! Shared tree walker used by every language module. Languages only describe
//! which nodes open a type scope, which produce functions, and which branch
//! rules apply; the walk, branch counting, scope tracking, and struct
//! assembly live here.

use tree_sitter::{Language, Node, Parser};

use crate::code::branch::{self, BranchFilter, BranchRule};
use crate::code::{FileComplexity, FunctionComplexity, TypeComplexity};

/// A function a language identified during the walk; the collector counts it.
pub struct FunctionDecl<'a> {
    pub name: String,
    /// The node whose descendants are counted as branches. This is the whole
    /// declaration (a `function_item`, a Kotlin `getter`, a Swift
    /// `computed_value` or observer clause), not just its body block:
    /// counting is exclusive of the node itself, so passing a narrower node
    /// would drop branches that *are* the body (e.g. an expression-bodied
    /// getter whose body is a single `if_expression`).
    pub body: Node<'a>,
}

/// What a language makes of a single syntax node.
pub enum Visit<'a> {
    /// Nothing at this node; keep descending.
    Skip,
    /// The node opens a type scope (class/struct/enum/...) for its subtree.
    Type(String),
    /// Functions produced at this node (one for plain functions, several for
    /// property accessors).
    Functions(Vec<FunctionDecl<'a>>),
}

pub trait LanguageRules {
    fn visit<'a>(&self, node: Node<'a>, source: &str) -> Visit<'a>;
    fn branch_rules(&self) -> &'static [BranchRule];
}

/// Parses the source with the given grammar and assembles the file complexity.
pub fn file_complexity(
    language: &Language,
    rules: &impl LanguageRules,
    source: &str,
    filter: &BranchFilter,
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
    collect(
        tree.root_node(),
        source,
        rules,
        filter,
        &mut Vec::new(),
        &mut file,
    );
    file
}

/// Cyclomatic complexity of a function node: a baseline of 1 for the
/// straight-line path plus one per counted branch node within it.
fn complexity(
    node: Node,
    source: &str,
    rules: &impl LanguageRules,
    filter: &BranchFilter,
) -> usize {
    1 + count_branches(node, source, rules, filter)
}

/// Counts branch nodes within (but not including) the given node. A node
/// counts when its classified friendly kind is selected, or when its literal
/// node kind is in the raw selection (the escape hatch is independent of the
/// classifier).
fn count_branches(
    node: Node,
    source: &str,
    rules: &impl LanguageRules,
    filter: &BranchFilter,
) -> usize {
    let mut branches = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if is_counted(child, source, rules, filter) {
            branches += 1;
        }
        branches += count_branches(child, source, rules, filter);
    }
    branches
}

fn is_counted(node: Node, source: &str, rules: &impl LanguageRules, filter: &BranchFilter) -> bool {
    branch::classify(rules.branch_rules(), node, source)
        .is_some_and(|kind| filter.allows_kind(kind))
        || filter.allows_raw(node.kind())
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
    filter: &BranchFilter,
    type_stack: &mut Vec<String>,
    file: &mut FileComplexity,
) {
    let opened_type = match rules.visit(node, source) {
        Visit::Skip => false,
        Visit::Type(name) => {
            type_stack.push(qualified_name(type_stack, &name));
            true
        }
        Visit::Functions(decls) => {
            let functions = count_functions(decls, source, rules, filter);
            attach_functions(file, type_stack, functions);
            false
        }
    };
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect(child, source, rules, filter, type_stack, file);
    }
    if opened_type {
        type_stack.pop();
    }
}

fn count_functions(
    decls: Vec<FunctionDecl>,
    source: &str,
    rules: &impl LanguageRules,
    filter: &BranchFilter,
) -> Vec<FunctionComplexity> {
    decls
        .into_iter()
        .map(|decl| FunctionComplexity {
            name: decl.name,
            complexity: complexity(decl.body, source, rules, filter),
        })
        .collect()
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
