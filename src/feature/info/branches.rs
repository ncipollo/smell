//! The `branches` topic: friendly branch-kind vocabulary plus the raw
//! node-kind escape hatch, generated from `BranchKind::ALL` so the docs
//! cannot drift from the classifier.

use crate::code::branch::BranchKind;

pub fn render() -> String {
    format!("{}\n{}", branch_kinds_section(), raw_section())
}

fn branch_kinds_section() -> String {
    let mut section = String::from(
        "BRANCH KINDS\n\
         Friendly, cross-language names accepted by --branches. Selecting any\n\
         kinds replaces the default of counting everything.\n\n",
    );
    for kind in BranchKind::ALL {
        section.push_str(&format!("  {:<18} {}\n", kind.name(), kind.description()));
    }
    section
}

fn raw_section() -> String {
    String::from(
        "RAW NODE KINDS\n\
         Any --branches value that is not a friendly kind is treated as a raw\n\
         tree-sitter node kind and matched literally against node.kind(),\n\
         independent of the classifier. Raw matches skip the conditions above:\n\
         for example `--branches binary_expression` counts all binary\n\
         expressions, not just && and ||. Friendly names carry the dynamic\n\
         logic; prefer them unless you need a node kind with no friendly name.\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_documents_every_branch_kind() {
        let page = render();
        for kind in BranchKind::ALL {
            assert!(
                page.contains(kind.name()),
                "missing branch kind: {}",
                kind.name()
            );
        }
    }
}
