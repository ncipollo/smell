use tree_sitter::{Node, Parser};

use crate::code::FunctionComplexity;

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

/// Parses Swift source and returns the branch complexity of each function.
pub fn function_complexities(source: &str) -> Vec<FunctionComplexity> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_swift::LANGUAGE.into())
        .expect("failed to load swift grammar");
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut functions = Vec::new();
    collect_functions(tree.root_node(), source, &mut functions);
    functions
}

fn collect_functions(node: Node, source: &str, functions: &mut Vec<FunctionComplexity>) {
    match node.kind() {
        "function_declaration" => functions.push(FunctionComplexity {
            name: function_name(node, source),
            branches: count_branches(node),
        }),
        "property_declaration" => collect_property(node, source, functions),
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, source, functions);
    }
}

fn collect_property(node: Node, source: &str, functions: &mut Vec<FunctionComplexity>) {
    let name = property_name(node, source);
    if let Some(computed) = node.child_by_field_name("computed_value") {
        collect_computed_accessors(computed, &name, functions);
    }
    if let Some(observers) = find_child(node, "willset_didset_block") {
        collect_observers(observers, &name, functions);
    }
}

fn collect_computed_accessors(computed: Node, name: &str, functions: &mut Vec<FunctionComplexity>) {
    let mut cursor = computed.walk();
    let accessors: Vec<Node> = computed
        .children(&mut cursor)
        .filter(|child| matches!(child.kind(), "computed_getter" | "computed_setter"))
        .collect();
    if accessors.is_empty() {
        functions.push(FunctionComplexity {
            name: name.to_string(),
            branches: count_branches(computed),
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
            branches: count_branches(accessor),
        });
    }
}

fn collect_observers(observers: Node, name: &str, functions: &mut Vec<FunctionComplexity>) {
    let mut cursor = observers.walk();
    for clause in observers.children(&mut cursor) {
        let suffix = match clause.kind() {
            "willset_clause" => "willSet",
            "didset_clause" => "didSet",
            _ => continue,
        };
        functions.push(FunctionComplexity {
            name: format!("{name}.{suffix}"),
            branches: count_branches(clause),
        });
    }
}

fn find_child<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .find(|child| child.kind() == kind)
}

fn property_name(node: Node, source: &str) -> String {
    node.child_by_field_name("name")
        .and_then(|pattern| pattern.child_by_field_name("bound_identifier"))
        .and_then(|identifier| identifier.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unknown>")
        .to_string()
}

fn function_name(node: Node, source: &str) -> String {
    node.child_by_field_name("name")
        .and_then(|name| name.utf8_text(source.as_bytes()).ok())
        .unwrap_or("<unknown>")
        .to_string()
}

fn count_branches(node: Node) -> usize {
    let mut branches = 0;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if BRANCH_KINDS.contains(&child.kind()) {
            branches += 1;
        }
        branches += count_branches(child);
    }
    branches
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
func simple() {
    print("hi")
}

func branchy(x: Int) -> Int {
    if x > 0 {
        return 1
    } else if x < -10 {
        return -2
    }
    guard x != 0 else { return 0 }
    for i in 0..<x {
        while i > 2 {
            break
        }
    }
    repeat {
        print("loop")
    } while x > 5
    switch x {
    case 1:
        return 1
    case 2:
        return 2
    default:
        break
    }
    let y = x > 3 ? 1 : 0
    let optional: Int? = nil
    let z = optional ?? y
    if z > 1 && z < 100 || z == -5 {
        return y
    }
    do {
        try canThrow()
    } catch {
        return -1
    }
    return y
}

struct Shape {
    var width = 0
    var height = 0

    var area: Int {
        return width > 0 ? width * height : 0
    }

    var label: String {
        get {
            if area > 10 { return "big" }
            return "small"
        }
        set {
            guard !newValue.isEmpty else { return }
            print(newValue)
        }
    }

    var count: Int = 0 {
        willSet {
            if newValue > 10 { print("big") }
        }
        didSet {
            guard count != oldValue else { return }
            print("changed")
        }
    }
}
"#;

    #[test]
    fn function_complexities_reports_each_function() {
        let functions = function_complexities(SAMPLE);
        let summary: Vec<(&str, usize)> = functions
            .iter()
            .map(|function| (function.name.as_str(), function.branches))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("simple", 0),
                ("branchy", 15),
                ("area", 1),
                ("label.get", 1),
                ("label.set", 1),
                ("count.willSet", 1),
                ("count.didSet", 1),
            ]
        );
    }

    #[test]
    fn function_complexities_handles_empty_source() {
        assert!(function_complexities("").is_empty());
    }
}
