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
    if node.kind() == "function_declaration" {
        functions.push(FunctionComplexity {
            name: function_name(node, source),
            branches: count_branches(node),
        });
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, source, functions);
    }
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
    if x > 1 && x < 100 || x == -5 {
        return y
    }
    do {
        try canThrow()
    } catch {
        return -1
    }
    return y
}
"#;

    #[test]
    fn function_complexities_reports_each_function() {
        let functions = function_complexities(SAMPLE);
        let summary: Vec<(&str, usize)> = functions
            .iter()
            .map(|function| (function.name.as_str(), function.branches))
            .collect();
        assert_eq!(summary, vec![("simple", 0), ("branchy", 14)]);
    }

    #[test]
    fn function_complexities_handles_empty_source() {
        assert!(function_complexities("").is_empty());
    }
}
