//! Method-count measure: types with more methods than the limit.
//!
//! Counts run on the already-filtered report (post `--implements`), so a
//! type excluded by that filter is not checked here either.

use crate::code::FileComplexity;
use crate::feature::complexity::FileReport;
use crate::feature::complexity::check::scope;
use crate::feature::complexity::check::{CheckFailure, Offender};

pub fn failures(reports: &[FileReport], limit: usize) -> Vec<CheckFailure> {
    scope::entries(reports, limit, offenders)
}

fn offenders(complexity: &FileComplexity) -> Vec<Offender> {
    complexity
        .types
        .iter()
        .map(|complexity_type| Offender {
            name: complexity_type.name.clone(),
            value: complexity_type.functions.len(),
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

    fn report(path: &str, types: Vec<TypeComplexity>) -> FileReport {
        FileReport {
            path: PathBuf::from(path),
            lines: 1,
            complexity: FileComplexity {
                functions: Vec::new(),
                types,
            },
        }
    }

    #[test]
    fn no_reports_pass() {
        assert!(failures(&[], 1).is_empty());
    }

    #[test]
    fn method_count_at_the_limit_passes() {
        let reports = [report("a.rs", vec![shape("Shape", 3)])];
        assert!(failures(&reports, 3).is_empty());
    }

    #[test]
    fn method_count_over_the_limit_fails() {
        let reports = [report("a.rs", vec![shape("Shape", 4)])];
        let result = failures(&reports, 3);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
        let offenders = result[0].subject.entries();
        assert_eq!(offenders[0].name, "Shape");
        assert_eq!(offenders[0].value, 4);
    }

    #[test]
    fn file_with_no_types_yields_nothing() {
        let reports = [report("a.rs", vec![])];
        assert!(failures(&reports, 0).is_empty());
    }

    #[test]
    fn a_file_lists_all_of_its_offending_types() {
        let reports = [report(
            "a.rs",
            vec![shape("Big", 5), shape("Small", 1), shape("AlsoBig", 4)],
        )];
        let result = failures(&reports, 3);
        let names: Vec<&str> = result[0]
            .subject
            .entries()
            .iter()
            .map(|offender| offender.name.as_str())
            .collect();
        assert_eq!(names, vec!["Big", "AlsoBig"]);
    }

    #[test]
    fn passing_files_are_omitted() {
        let reports = [
            report("ok.rs", vec![shape("Small", 1)]),
            report("bad.rs", vec![shape("Big", 8)]),
        ];
        let result = failures(&reports, 3);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
