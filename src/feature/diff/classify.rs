//! Matches functions across two [`FileComplexity`] snapshots and classifies
//! each as added, removed, changed, or unchanged. Pure — no `git2` — so it's
//! unit-testable against hand-built fixtures.

use std::collections::HashMap;

use crate::code::FileComplexity;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDiff {
    pub type_name: Option<String>,
    pub name: String,
    pub status: DiffStatus,
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

fn bucket(complexity: Option<&FileComplexity>) -> HashMap<Key, Vec<usize>> {
    let mut map: HashMap<Key, Vec<usize>> = HashMap::new();
    let Some(complexity) = complexity else {
        return map;
    };
    for function in &complexity.functions {
        map.entry((None, function.name.clone()))
            .or_default()
            .push(function.complexity);
    }
    for complexity_type in &complexity.types {
        for function in &complexity_type.functions {
            map.entry((Some(complexity_type.name.clone()), function.name.clone()))
                .or_default()
                .push(function.complexity);
        }
    }
    map
}

/// Pairs up the (possibly multiple, for overloads) complexities recorded
/// under one key on each side positionally, emitting one diff per pair plus
/// one per unmatched surplus entry on either side.
fn pair(key: Key, old: Option<&Vec<usize>>, new: Option<&Vec<usize>>) -> Vec<FunctionDiff> {
    let old = old.map(Vec::as_slice).unwrap_or(&[]);
    let new = new.map(Vec::as_slice).unwrap_or(&[]);
    let paired = old.len().min(new.len());

    let mut diffs = Vec::new();
    for i in 0..paired {
        diffs.push(function_diff(&key, status_for(old[i], new[i])));
    }
    for &complexity in &old[paired..] {
        diffs.push(function_diff(&key, DiffStatus::Removed { complexity }));
    }
    for &complexity in &new[paired..] {
        diffs.push(function_diff(&key, DiffStatus::Added { complexity }));
    }
    diffs
}

fn status_for(before: usize, after: usize) -> DiffStatus {
    if before == after {
        DiffStatus::Unchanged { complexity: before }
    } else {
        DiffStatus::Changed { before, after }
    }
}

fn function_diff(key: &Key, status: DiffStatus) -> FunctionDiff {
    FunctionDiff {
        type_name: key.0.clone(),
        name: key.1.clone(),
        status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{FunctionComplexity, TypeComplexity};

    fn function(name: &str, complexity: usize) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            complexity,
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
}
