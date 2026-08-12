//! Declaration-count measure: files with more top-level declarations
//! (types plus top-level functions) than the limit.
//!
//! Like line count, the subject is the file itself rather than a named
//! entry inside it. Counts run on the already-filtered report (post
//! `--implements`), same as method count and line count.

use crate::feature::complexity::FileReport;
use crate::feature::complexity::check::CheckFailure;
use crate::feature::complexity::check::scope;

pub fn failures(reports: &[FileReport], limit: usize) -> Vec<CheckFailure> {
    scope::file(reports, limit, |report| report.complexity.declarations())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::{FileComplexity, FunctionComplexity, TypeComplexity};
    use crate::feature::complexity::check::Subject;

    fn function(name: &str) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            complexity: 1,
        }
    }

    fn shape(name: &str) -> TypeComplexity {
        TypeComplexity {
            name: name.to_string(),
            supertypes: Vec::new(),
            functions: Vec::new(),
        }
    }

    fn report(
        path: &str,
        functions: Vec<FunctionComplexity>,
        types: Vec<TypeComplexity>,
    ) -> FileReport {
        FileReport {
            path: PathBuf::from(path),
            lines: 1,
            complexity: FileComplexity { functions, types },
        }
    }

    #[test]
    fn no_reports_pass() {
        assert!(failures(&[], 1).is_empty());
    }

    #[test]
    fn declaration_count_at_the_limit_passes() {
        let reports = [report("a.rs", vec![function("top")], vec![shape("Shape")])];
        assert!(failures(&reports, 2).is_empty());
    }

    #[test]
    fn declaration_count_over_the_limit_fails() {
        let reports = [report(
            "a.rs",
            vec![function("top"), function("other")],
            vec![shape("Shape")],
        )];
        let result = failures(&reports, 2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
        match result[0].subject {
            Subject::File(count) => assert_eq!(count, 3),
            Subject::Entries(_) => panic!("expected a file subject"),
        }
    }

    #[test]
    fn types_and_top_level_functions_both_contribute_to_the_count() {
        let reports = [report(
            "a.rs",
            vec![function("top")],
            vec![shape("Shape"), shape("Other")],
        )];
        let result = failures(&reports, 2);
        match result[0].subject {
            Subject::File(count) => assert_eq!(count, 3),
            Subject::Entries(_) => panic!("expected a file subject"),
        }
    }

    #[test]
    fn passing_files_are_omitted() {
        let reports = [
            report("ok.rs", vec![function("top")], vec![]),
            report("bad.rs", vec![], vec![shape("A"), shape("B"), shape("C")]),
        ];
        let result = failures(&reports, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
