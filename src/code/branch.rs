//! Branch vocabulary shared by every language: friendly cross-language kinds,
//! the declarative rules languages publish, and the filter that selects which
//! kinds count toward complexity.

use tree_sitter::Node;

/// Friendly, cross-language names for the constructs that count as branches.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BranchKind {
    If,
    Guard,
    Switch,
    Loop,
    Catch,
    Ternary,
    BooleanOperator,
    NullCoalescing,
    Try,
}

impl BranchKind {
    pub const ALL: [BranchKind; 9] = [
        BranchKind::If,
        BranchKind::Guard,
        BranchKind::Switch,
        BranchKind::Loop,
        BranchKind::Catch,
        BranchKind::Ternary,
        BranchKind::BooleanOperator,
        BranchKind::NullCoalescing,
        BranchKind::Try,
    ];

    /// The kebab-case name used on the command line and in `--info`.
    pub fn name(self) -> &'static str {
        match self {
            BranchKind::If => "if",
            BranchKind::Guard => "guard",
            BranchKind::Switch => "switch",
            BranchKind::Loop => "loop",
            BranchKind::Catch => "catch",
            BranchKind::Ternary => "ternary",
            BranchKind::BooleanOperator => "boolean-operator",
            BranchKind::NullCoalescing => "null-coalescing",
            BranchKind::Try => "try",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            BranchKind::If => "if statements and expressions",
            BranchKind::Guard => "early-exit guards (Swift guard, Rust let-else)",
            BranchKind::Switch => {
                "switch/match/when cases, including pattern guards on individual cases"
            }
            BranchKind::Loop => "for, while, and do/repeat loops",
            BranchKind::Catch => "catch clauses in try/catch blocks",
            BranchKind::Ternary => "ternary conditional expressions (cond ? a : b)",
            BranchKind::BooleanOperator => "short-circuiting boolean operators (&& and ||)",
            BranchKind::NullCoalescing => "null-coalescing operators (Swift ??, Kotlin ?:)",
            BranchKind::Try => {
                "error-propagating try operators that hide a branch (Rust ?, Swift try?)"
            }
        }
    }

    /// Looks up a kind by its kebab-case name.
    pub fn from_name(name: &str) -> Option<BranchKind> {
        BranchKind::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

/// One user-supplied `--branches` selection: a friendly kind, or a raw
/// tree-sitter node kind used as an escape hatch.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BranchSpec {
    Kind(BranchKind),
    Raw(String),
}

impl BranchSpec {
    /// Parses a selection. Known friendly names become [`BranchSpec::Kind`];
    /// anything else is kept verbatim as a raw node kind. Never fails — raw
    /// kinds are open-ended, so a typo'd friendly name just matches nothing.
    pub fn parse(input: &str) -> BranchSpec {
        match BranchKind::from_name(input) {
            Some(kind) => BranchSpec::Kind(kind),
            None => BranchSpec::Raw(input.to_string()),
        }
    }
}

/// A compiled `--branches` selection. The default counts everything; any
/// explicit selection replaces the default (only the selected kinds count).
#[derive(Default)]
pub struct BranchFilter {
    selection: Option<Selection>,
}

struct Selection {
    kinds: Vec<BranchKind>,
    raw: Vec<String>,
}

impl BranchFilter {
    /// Compiles specs into a filter. Empty specs mean "count everything".
    pub fn from_specs(specs: &[BranchSpec]) -> BranchFilter {
        if specs.is_empty() {
            return BranchFilter::default();
        }
        let mut selection = Selection {
            kinds: Vec::new(),
            raw: Vec::new(),
        };
        for spec in specs {
            match spec {
                BranchSpec::Kind(kind) => selection.kinds.push(*kind),
                BranchSpec::Raw(raw) => selection.raw.push(raw.clone()),
            }
        }
        BranchFilter {
            selection: Some(selection),
        }
    }

    pub fn allows_kind(&self, kind: BranchKind) -> bool {
        match &self.selection {
            Some(selection) => selection.kinds.contains(&kind),
            None => true,
        }
    }

    pub fn allows_raw(&self, raw: &str) -> bool {
        match &self.selection {
            Some(selection) => selection.raw.iter().any(|selected| selected == raw),
            None => false,
        }
    }
}

/// A dynamic check attached to a rule, for node kinds that only branch in
/// certain shapes (e.g. `binary_expression` with a `&&` operator).
pub struct Condition {
    /// Human-readable summary rendered by `--info`.
    pub description: &'static str,
    pub check: fn(Node, &str) -> bool,
}

/// Maps one tree-sitter node kind to a friendly branch kind, optionally
/// gated by a condition.
pub struct BranchRule {
    pub kind: BranchKind,
    pub node_kind: &'static str,
    pub condition: Option<Condition>,
}

impl BranchRule {
    /// A rule that matches every node of the given kind.
    pub const fn new(kind: BranchKind, node_kind: &'static str) -> BranchRule {
        BranchRule {
            kind,
            node_kind,
            condition: None,
        }
    }

    /// A rule gated by a dynamic condition.
    pub const fn when(
        kind: BranchKind,
        node_kind: &'static str,
        description: &'static str,
        check: fn(Node, &str) -> bool,
    ) -> BranchRule {
        BranchRule {
            kind,
            node_kind,
            condition: Some(Condition { description, check }),
        }
    }
}

/// Classifies a node against a rule table: the first rule whose `node_kind`
/// matches and whose condition passes wins. A rule whose condition fails
/// falls through to later rules for the same node kind (Kotlin maps
/// `binary_expression` to both boolean-operator and null-coalescing).
pub fn classify(rules: &[BranchRule], node: Node, source: &str) -> Option<BranchKind> {
    rules
        .iter()
        .find(|rule| {
            rule.node_kind == node.kind()
                && rule
                    .condition
                    .as_ref()
                    .is_none_or(|condition| (condition.check)(node, source))
        })
        .map(|rule| rule.kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip_for_every_kind() {
        for kind in BranchKind::ALL {
            assert_eq!(BranchKind::from_name(kind.name()), Some(kind));
        }
    }

    #[test]
    fn from_name_rejects_unknown_names() {
        assert_eq!(BranchKind::from_name("not-a-kind"), None);
    }

    #[test]
    fn parse_maps_friendly_names_to_kinds() {
        assert_eq!(
            BranchSpec::parse("boolean-operator"),
            BranchSpec::Kind(BranchKind::BooleanOperator)
        );
    }

    #[test]
    fn parse_keeps_unknown_names_as_raw() {
        assert_eq!(
            BranchSpec::parse("binary_expression"),
            BranchSpec::Raw("binary_expression".to_string())
        );
    }

    #[test]
    fn default_filter_allows_every_kind_and_no_raw() {
        let filter = BranchFilter::default();
        for kind in BranchKind::ALL {
            assert!(filter.allows_kind(kind));
        }
        assert!(!filter.allows_raw("binary_expression"));
    }

    #[test]
    fn empty_specs_compile_to_the_default_filter() {
        let filter = BranchFilter::from_specs(&[]);
        for kind in BranchKind::ALL {
            assert!(filter.allows_kind(kind));
        }
    }

    #[test]
    fn selection_replaces_the_default() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Kind(BranchKind::Switch)]);
        assert!(filter.allows_kind(BranchKind::Switch));
        assert!(!filter.allows_kind(BranchKind::If));
        assert!(!filter.allows_raw("match_arm"));
    }

    #[test]
    fn raw_selection_matches_literal_node_kinds() {
        let filter = BranchFilter::from_specs(&[BranchSpec::Raw("if_expression".to_string())]);
        assert!(filter.allows_raw("if_expression"));
        assert!(!filter.allows_raw("match_arm"));
        assert!(!filter.allows_kind(BranchKind::If));
    }
}
