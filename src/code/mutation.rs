//! Mutation-count vocabulary: per-language tree-sitter node kinds that count
//! as an assignment or other mutating operation. Counted alongside
//! complexity so a diff can flag a function where new mutations appear even
//! though branching, and therefore complexity, didn't change.
//!
//! A thin, semantically-named module over
//! [`collector::SignalRule`]/[`collector::count_signal`] — see
//! [`crate::code::call`] for why this is shared rather than its own rule
//! type and walk.

use tree_sitter::Node;

use crate::code::collector::{self, SignalRule};

pub type MutationRule = SignalRule;

/// Counts mutation nodes within (but not including) `node`.
pub fn count(node: Node, source: &str, rules: &[MutationRule]) -> usize {
    collector::count_signal(node, source, rules)
}
