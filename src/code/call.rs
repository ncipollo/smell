//! Call-count vocabulary: per-language tree-sitter node kinds that count as
//! a function or method invocation. Counted alongside complexity so a diff
//! can flag a function where new calls appear even though branching, and
//! therefore complexity, didn't change.
//!
//! A thin, semantically-named module over
//! [`collector::SignalRule`]/[`collector::count_signal`], the rule type and
//! walk shared by every per-function signal that isn't a branch (mutations
//! today, any future one later) — kept as its own module, per the
//! classification pattern in [`crate::code::branch`], so each signal's
//! per-language rule tables live under a name that says what they're for.

use tree_sitter::Node;

use crate::code::collector::{self, SignalRule};

pub type CallRule = SignalRule;

/// Counts call nodes within (but not including) `node`.
pub fn count(node: Node, source: &str, rules: &[CallRule]) -> usize {
    collector::count_signal(node, source, rules)
}
