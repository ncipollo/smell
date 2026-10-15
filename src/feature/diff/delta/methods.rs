//! Method-count delta measure: types whose method count swung by more than
//! the limit between the two revisions. Types are matched by name — a
//! `TypeComplexity` already unions a type's split declarations (impl blocks,
//! extensions) within one revision, so a name is unique per side.

use std::collections::HashMap;

use crate::code::FileComplexity;
use crate::feature::complexity::check::{CheckFailure, Offender};
use crate::feature::diff::delta::scope;
use crate::feature::diff::{FileDiff, Snapshot};

pub fn failures(files: &[FileDiff], limit: usize) -> Vec<CheckFailure> {
    scope::entries(files, limit, offenders)
}

fn offenders(file: &FileDiff) -> Vec<Offender> {
    let old = method_counts(file.old.as_ref());
    let new = method_counts(file.new.as_ref());

    let mut names: Vec<&String> = old.keys().chain(new.keys()).collect();
    names.sort();
    names.dedup();

    names
        .into_iter()
        .map(|name| {
            let before = old.get(name).copied().unwrap_or(0) as i64;
            let after = new.get(name).copied().unwrap_or(0) as i64;
            Offender {
                name: name.clone(),
                value: (after - before).unsigned_abs() as usize,
            }
        })
        .collect()
}

fn method_counts(snapshot: Option<&Snapshot>) -> HashMap<String, usize> {
    snapshot
        .map(|snapshot| type_method_counts(&snapshot.complexity))
        .unwrap_or_default()
}

fn type_method_counts(complexity: &FileComplexity) -> HashMap<String, usize> {
    complexity
        .types
        .iter()
        .map(|complexity_type| {
            (
                complexity_type.name.clone(),
                complexity_type.functions.len(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::{FunctionComplexity, TypeComplexity};

    fn function(name: &str) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            complexity: 1,
            calls: 0,
            mutations: 0,
        }
    }

    fn shape(name: &str, method_count: usize) -> TypeComplexity {
        let functions = (0..method_count)
            .map(|index| function(&format!("m{index}")))
            .collect();
        TypeComplexity {
            name: name.to_string(),
            supertypes: Vec::new(),
            functions,
        }
    }

    fn snapshot(types: Vec<TypeComplexity>) -> Snapshot {
        Snapshot {
            lines: 1,
            complexity: FileComplexity {
                types,
                ..Default::default()
            },
        }
    }

    fn file(path: &str, old: Vec<TypeComplexity>, new: Vec<TypeComplexity>) -> FileDiff {
        FileDiff {
            path: PathBuf::from(path),
            functions: Vec::new(),
            old: Some(snapshot(old)),
            new: Some(snapshot(new)),
        }
    }

    #[test]
    fn no_files_pass() {
        assert!(failures(&[], 1).is_empty());
    }

    #[test]
    fn method_count_delta_at_the_limit_passes() {
        let files = [file(
            "a.rs",
            vec![shape("Shape", 1)],
            vec![shape("Shape", 3)],
        )];
        assert!(failures(&files, 2).is_empty());
    }

    #[test]
    fn method_count_delta_over_the_limit_fails() {
        let files = [file(
            "a.rs",
            vec![shape("Shape", 1)],
            vec![shape("Shape", 4)],
        )];
        let result = failures(&files, 2);
        assert_eq!(result.len(), 1);
        let offenders = result[0].subject.entries();
        assert_eq!(offenders[0].name, "Shape");
        assert_eq!(offenders[0].value, 3);
    }

    #[test]
    fn a_shrinking_method_count_fails_on_magnitude_too() {
        let files = [file(
            "a.rs",
            vec![shape("Shape", 5)],
            vec![shape("Shape", 1)],
        )];
        assert_eq!(failures(&files, 2).len(), 1);
    }

    #[test]
    fn a_type_added_outright_is_a_delta_from_zero() {
        let files = [file("a.rs", vec![], vec![shape("New", 3)])];
        assert_eq!(failures(&files, 2).len(), 1);
    }

    #[test]
    fn a_type_removed_outright_is_a_delta_to_zero() {
        let files = [file("a.rs", vec![shape("Gone", 3)], vec![])];
        assert_eq!(failures(&files, 2).len(), 1);
    }

    #[test]
    fn a_file_lists_all_of_its_offending_types() {
        let files = [file(
            "a.rs",
            vec![shape("Big", 1), shape("Small", 1), shape("AlsoBig", 1)],
            vec![shape("Big", 6), shape("Small", 2), shape("AlsoBig", 5)],
        )];
        let result = failures(&files, 2);
        let names: Vec<&str> = result[0]
            .subject
            .entries()
            .iter()
            .map(|offender| offender.name.as_str())
            .collect();
        assert_eq!(names, vec!["AlsoBig", "Big"]);
    }

    #[test]
    fn passing_files_are_omitted() {
        let files = [
            file("ok.rs", vec![shape("Small", 1)], vec![shape("Small", 2)]),
            file("bad.rs", vec![shape("Big", 1)], vec![shape("Big", 8)]),
        ];
        let result = failures(&files, 2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
