//! Matches functions across two [`FileComplexity`] snapshots and classifies
//! each as added, removed, changed, or unchanged. Pure — no `git2` — so it's
//! unit-testable against hand-built fixtures.

use std::collections::HashMap;

use crate::code::{FileComplexity, FunctionComplexity};

/// (enclosing type name, or `None` for a top-level function; function name).
/// `FunctionComplexity` has no line span or other stable id, so this pair is
/// the only thing a function can be matched on across two revisions.
type Key = (Option<String>, String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffStatus {
    Added { complexity: usize },
    Removed { complexity: usize },
    Changed { before: usize, after: usize },
    Unchanged { complexity: usize },
}

impl DiffStatus {
    /// `after - before` for a changed function; zero otherwise.
    pub fn delta(&self) -> i64 {
        match self {
            DiffStatus::Changed { before, after } => *after as i64 - *before as i64,
            _ => 0,
        }
    }
}

/// Call and mutation counts on both sides of a function whose complexity
/// (and therefore [`DiffStatus`]) didn't move — cyclomatic complexity can't
/// see a new call or assignment, so this is the signal that catches it.
/// Only ever set alongside `DiffStatus::Unchanged`, and only when at least
/// one of the two counts actually moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlatActivity {
    pub calls_before: usize,
    pub calls_after: usize,
    pub mutations_before: usize,
    pub mutations_after: usize,
}

impl FlatActivity {
    pub fn calls_delta(&self) -> i64 {
        self.calls_after as i64 - self.calls_before as i64
    }

    pub fn mutations_delta(&self) -> i64 {
        self.mutations_after as i64 - self.mutations_before as i64
    }

    fn of(before: &FunctionComplexity, after: &FunctionComplexity) -> Option<FlatActivity> {
        let activity = FlatActivity {
            calls_before: before.calls,
            calls_after: after.calls,
            mutations_before: before.mutations,
            mutations_after: after.mutations,
        };
        let changed = activity.calls_delta() != 0 || activity.mutations_delta() != 0;
        changed.then_some(activity)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDiff {
    pub type_name: Option<String>,
    pub name: String,
    pub status: DiffStatus,
    pub flat_active: Option<FlatActivity>,
}

/// Matches functions across the two sides by (enclosing type name, function
/// name) and classifies each. `old`/`new` is `None` when the file doesn't
/// exist at that revision, which reports every function on the other side
/// as wholly `Added` or `Removed`.
///
/// Two accepted v1 limitations, both a direct consequence of having no
/// better matching key: a renamed function shows as a remove + add, and
/// same-name overloads within a type pair up positionally (in whatever
/// order the parser emitted them), with any count mismatch between the two
/// sides reported as surplus `Added`/`Removed` entries.
pub fn classify_file(
    old: Option<&FileComplexity>,
    new: Option<&FileComplexity>,
) -> Vec<FunctionDiff> {
    let old_map = bucket(old);
    let new_map = bucket(new);

    let mut keys: Vec<Key> = old_map.keys().chain(new_map.keys()).cloned().collect();
    keys.sort();
    keys.dedup();

    keys.into_iter()
        .flat_map(|key| {
            let old = old_map.get(&key);
            let new = new_map.get(&key);
            pair(key, old, new)
        })
        .collect()
}

fn bucket(complexity: Option<&FileComplexity>) -> HashMap<Key, Vec<FunctionComplexity>> {
    let mut map: HashMap<Key, Vec<FunctionComplexity>> = HashMap::new();
    let Some(complexity) = complexity else {
        return map;
    };
    for function in &complexity.functions {
        map.entry((None, function.name.clone()))
            .or_default()
            .push(function.clone());
    }
    for complexity_type in &complexity.types {
        for function in &complexity_type.functions {
            map.entry((Some(complexity_type.name.clone()), function.name.clone()))
                .or_default()
                .push(function.clone());
        }
    }
    map
}

/// Pairs up the (possibly multiple, for overloads) functions recorded under
/// one key on each side positionally, emitting one diff per pair plus one
/// per unmatched surplus entry on either side.
fn pair(
    key: Key,
    old: Option<&Vec<FunctionComplexity>>,
    new: Option<&Vec<FunctionComplexity>>,
) -> Vec<FunctionDiff> {
    let old = old.map(Vec::as_slice).unwrap_or(&[]);
    let new = new.map(Vec::as_slice).unwrap_or(&[]);
    let paired = old.len().min(new.len());

    let mut diffs = Vec::new();
    for i in 0..paired {
        diffs.push(changed_or_unchanged(&key, &old[i], &new[i]));
    }
    for function in &old[paired..] {
        diffs.push(function_diff(
            &key,
            DiffStatus::Removed {
                complexity: function.complexity,
            },
            None,
        ));
    }
    for function in &new[paired..] {
        diffs.push(function_diff(
            &key,
            DiffStatus::Added {
                complexity: function.complexity,
            },
            None,
        ));
    }
    diffs
}

/// Classifies a matched pair by complexity, and — only when that leaves the
/// pair `Unchanged` — layers on [`FlatActivity`] if calls or mutations moved
/// anyway (see its docs for why that case needs a second signal).
fn changed_or_unchanged(
    key: &Key,
    before: &FunctionComplexity,
    after: &FunctionComplexity,
) -> FunctionDiff {
    let status = status_for(before.complexity, after.complexity);
    let flat_active = match status {
        DiffStatus::Unchanged { .. } => FlatActivity::of(before, after),
        _ => None,
    };
    function_diff(key, status, flat_active)
}

fn status_for(before: usize, after: usize) -> DiffStatus {
    if before == after {
        DiffStatus::Unchanged { complexity: before }
    } else {
        DiffStatus::Changed { before, after }
    }
}

fn function_diff(key: &Key, status: DiffStatus, flat_active: Option<FlatActivity>) -> FunctionDiff {
    FunctionDiff {
        type_name: key.0.clone(),
        name: key.1.clone(),
        status,
        flat_active,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{FunctionComplexity, TypeComplexity};

    fn function(name: &str, complexity: usize) -> FunctionComplexity {
        active_function(name, complexity, 0, 0)
    }

    fn active_function(
        name: &str,
        complexity: usize,
        calls: usize,
        mutations: usize,
    ) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            complexity,
            calls,
            mutations,
        }
    }

    fn file(functions: Vec<FunctionComplexity>, types: Vec<TypeComplexity>) -> FileComplexity {
        FileComplexity {
            functions,
            types,
            ..Default::default()
        }
    }

    fn find<'a>(
        diffs: &'a [FunctionDiff],
        type_name: Option<&str>,
        name: &str,
    ) -> &'a FunctionDiff {
        diffs
            .iter()
            .find(|diff| diff.type_name.as_deref() == type_name && diff.name == name)
            .unwrap_or_else(|| panic!("no diff for {type_name:?}.{name}"))
    }

    #[test]
    fn classifies_an_added_top_level_function() {
        let new = file(vec![function("a", 1)], vec![]);
        let diffs = classify_file(None, Some(&new));
        assert_eq!(
            diffs,
            vec![FunctionDiff {
                type_name: None,
                name: "a".to_string(),
                status: DiffStatus::Added { complexity: 1 },
                flat_active: None,
            }]
        );
    }

    #[test]
    fn classifies_a_removed_top_level_function() {
        let old = file(vec![function("a", 1)], vec![]);
        let diffs = classify_file(Some(&old), None);
        assert_eq!(
            diffs,
            vec![FunctionDiff {
                type_name: None,
                name: "a".to_string(),
                status: DiffStatus::Removed { complexity: 1 },
                flat_active: None,
            }]
        );
    }

    #[test]
    fn classifies_a_changed_function_both_directions() {
        let old = file(vec![function("a", 3), function("b", 7)], vec![]);
        let new = file(vec![function("a", 7), function("b", 3)], vec![]);
        let diffs = classify_file(Some(&old), Some(&new));
        assert_eq!(
            find(&diffs, None, "a").status,
            DiffStatus::Changed {
                before: 3,
                after: 7
            }
        );
        assert_eq!(
            find(&diffs, None, "b").status,
            DiffStatus::Changed {
                before: 7,
                after: 3
            }
        );
    }

    #[test]
    fn classifies_an_unchanged_function() {
        let old = file(vec![function("a", 4)], vec![]);
        let new = file(vec![function("a", 4)], vec![]);
        let diffs = classify_file(Some(&old), Some(&new));
        assert_eq!(
            diffs,
            vec![FunctionDiff {
                type_name: None,
                name: "a".to_string(),
                status: DiffStatus::Unchanged { complexity: 4 },
                flat_active: None,
            }]
        );
    }

    #[test]
    fn a_function_moved_to_a_different_type_shows_as_remove_plus_add() {
        let old = file(
            vec![],
            vec![TypeComplexity {
                name: "A".to_string(),
                supertypes: vec![],
                functions: vec![function("f", 2)],
            }],
        );
        let new = file(
            vec![],
            vec![TypeComplexity {
                name: "B".to_string(),
                supertypes: vec![],
                functions: vec![function("f", 2)],
            }],
        );
        let diffs = classify_file(Some(&old), Some(&new));
        assert_eq!(
            find(&diffs, Some("A"), "f").status,
            DiffStatus::Removed { complexity: 2 }
        );
        assert_eq!(
            find(&diffs, Some("B"), "f").status,
            DiffStatus::Added { complexity: 2 }
        );
    }

    #[test]
    fn duplicate_names_in_the_same_type_pair_up_positionally() {
        let old = file(
            vec![],
            vec![TypeComplexity {
                name: "Overloaded".to_string(),
                supertypes: vec![],
                functions: vec![function("f", 1), function("f", 2)],
            }],
        );
        let new = file(
            vec![],
            vec![TypeComplexity {
                name: "Overloaded".to_string(),
                supertypes: vec![],
                functions: vec![function("f", 1), function("f", 2), function("f", 9)],
            }],
        );
        let diffs = classify_file(Some(&old), Some(&new));
        let statuses: Vec<DiffStatus> = diffs.iter().map(|diff| diff.status).collect();
        assert_eq!(
            statuses,
            vec![
                DiffStatus::Unchanged { complexity: 1 },
                DiffStatus::Unchanged { complexity: 2 },
                DiffStatus::Added { complexity: 9 },
            ]
        );
    }

    #[test]
    fn whole_file_added_reports_every_function_as_added() {
        let new = file(
            vec![function("top", 1)],
            vec![TypeComplexity {
                name: "Shape".to_string(),
                supertypes: vec![],
                functions: vec![function("area", 3)],
            }],
        );
        let diffs = classify_file(None, Some(&new));
        assert_eq!(diffs.len(), 2);
        assert!(
            diffs
                .iter()
                .all(|diff| matches!(diff.status, DiffStatus::Added { .. }))
        );
    }

    #[test]
    fn whole_file_removed_reports_every_function_as_removed() {
        let old = file(vec![function("top", 1)], vec![]);
        let diffs = classify_file(Some(&old), None);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0].status, DiffStatus::Removed { .. }));
    }

    #[test]
    fn delta_is_zero_except_for_changed() {
        assert_eq!(DiffStatus::Added { complexity: 5 }.delta(), 0);
        assert_eq!(DiffStatus::Removed { complexity: 5 }.delta(), 0);
        assert_eq!(DiffStatus::Unchanged { complexity: 5 }.delta(), 0);
        assert_eq!(
            DiffStatus::Changed {
                before: 3,
                after: 7
            }
            .delta(),
            4
        );
        assert_eq!(
            DiffStatus::Changed {
                before: 7,
                after: 3
            }
            .delta(),
            -4
        );
    }

    #[test]
    fn flat_active_is_none_when_calls_and_mutations_are_unchanged() {
        let old = file(vec![active_function("a", 1, 2, 3)], vec![]);
        let new = file(vec![active_function("a", 1, 2, 3)], vec![]);
        let diffs = classify_file(Some(&old), Some(&new));
        assert_eq!(find(&diffs, None, "a").flat_active, None);
    }

    #[test]
    fn flat_active_is_set_when_calls_change_but_complexity_does_not() {
        let old = file(vec![active_function("a", 1, 2, 0)], vec![]);
        let new = file(vec![active_function("a", 1, 4, 0)], vec![]);
        let diffs = classify_file(Some(&old), Some(&new));
        let flat_active = find(&diffs, None, "a").flat_active.expect("flagged");
        assert_eq!(flat_active.calls_delta(), 2);
        assert_eq!(flat_active.mutations_delta(), 0);
    }

    #[test]
    fn flat_active_is_set_when_mutations_change_but_complexity_does_not() {
        let old = file(vec![active_function("a", 1, 0, 1)], vec![]);
        let new = file(vec![active_function("a", 1, 0, 3)], vec![]);
        let diffs = classify_file(Some(&old), Some(&new));
        let flat_active = find(&diffs, None, "a").flat_active.expect("flagged");
        assert_eq!(flat_active.calls_delta(), 0);
        assert_eq!(flat_active.mutations_delta(), 2);
    }

    #[test]
    fn flat_active_is_none_for_a_genuinely_changed_function() {
        // Complexity moved too, so the ordinary `Changed` status already
        // surfaces this function — flat_active only matters when complexity
        // alone would have hidden it.
        let old = file(vec![active_function("a", 1, 2, 0)], vec![]);
        let new = file(vec![active_function("a", 2, 4, 0)], vec![]);
        let diffs = classify_file(Some(&old), Some(&new));
        assert_eq!(find(&diffs, None, "a").flat_active, None);
    }

    #[test]
    fn flat_active_is_none_for_added_or_removed_functions() {
        let old = file(vec![active_function("gone", 1, 1, 1)], vec![]);
        let new = file(vec![active_function("new", 1, 1, 1)], vec![]);
        let diffs = classify_file(Some(&old), Some(&new));
        assert_eq!(find(&diffs, None, "gone").flat_active, None);
        assert_eq!(find(&diffs, None, "new").flat_active, None);
    }
}
