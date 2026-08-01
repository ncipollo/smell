//! The complexity limit check: which analyzed functions exceed a limit.

use std::path::PathBuf;

use crate::code::FileComplexity;
use crate::feature::complexity::FileReport;

/// A function whose complexity exceeds the limit. Functions inside a type
/// carry a `Type.function` qualified name; top-level functions are bare.
pub struct FailedFunction {
    pub name: String,
    pub complexity: usize,
}

/// A file containing at least one function over the limit.
pub struct CheckFailure {
    pub path: PathBuf,
    pub functions: Vec<FailedFunction>,
}

/// Returns one failure per file with any function whose complexity is
/// strictly greater than `limit` (a complexity equal to the limit passes).
pub fn check(reports: &[FileReport], limit: usize) -> Vec<CheckFailure> {
    reports
        .iter()
        .filter_map(|report| {
            let functions = failed_functions(&report.complexity, limit);
            if functions.is_empty() {
                None
            } else {
                Some(CheckFailure {
                    path: report.path.clone(),
                    functions,
                })
            }
        })
        .collect()
}

fn failed_functions(complexity: &FileComplexity, limit: usize) -> Vec<FailedFunction> {
    let type_functions = complexity.types.iter().flat_map(|complexity_type| {
        complexity_type.functions.iter().map(|function| {
            (
                format!("{}.{}", complexity_type.name, function.name),
                function,
            )
        })
    });
    let top_level = complexity
        .functions
        .iter()
        .map(|function| (function.name.clone(), function));
    type_functions
        .chain(top_level)
        .filter(|(_, function)| function.complexity > limit)
        .map(|(name, function)| FailedFunction {
            name,
            complexity: function.complexity,
        })
        .collect()
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

    fn report(
        path: &str,
        functions: Vec<FunctionComplexity>,
        types: Vec<TypeComplexity>,
    ) -> FileReport {
        FileReport {
            path: PathBuf::from(path),
            complexity: FileComplexity { functions, types },
        }
    }

    fn shape(functions: Vec<FunctionComplexity>) -> TypeComplexity {
        TypeComplexity {
            name: "Shape".to_string(),
            supertypes: Vec::new(),
            functions,
        }
    }

    #[test]
    fn no_reports_pass() {
        assert!(check(&[], 1).is_empty());
    }

    #[test]
    fn complexity_at_the_limit_passes() {
        let reports = [report("a.rs", vec![function("top", 3)], vec![])];
        assert!(check(&reports, 3).is_empty());
    }

    #[test]
    fn complexity_over_the_limit_fails() {
        let reports = [report("a.rs", vec![function("top", 4)], vec![])];
        let failures = check(&reports, 3);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].path, PathBuf::from("a.rs"));
        assert_eq!(failures[0].functions.len(), 1);
        assert_eq!(failures[0].functions[0].name, "top");
        assert_eq!(failures[0].functions[0].complexity, 4);
    }

    #[test]
    fn type_functions_are_qualified_with_the_type_name() {
        let reports = [report(
            "a.rs",
            vec![],
            vec![shape(vec![function("area", 9)])],
        )];
        let failures = check(&reports, 3);
        assert_eq!(failures[0].functions[0].name, "Shape.area");
    }

    #[test]
    fn passing_files_are_omitted() {
        let reports = [
            report("ok.rs", vec![function("simple", 1)], vec![]),
            report("bad.rs", vec![function("branchy", 8)], vec![]),
        ];
        let failures = check(&reports, 3);
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].path, PathBuf::from("bad.rs"));
    }

    #[test]
    fn a_file_lists_all_of_its_offending_functions() {
        let reports = [report(
            "a.rs",
            vec![function("top", 5)],
            vec![shape(vec![function("area", 9), function("label", 2)])],
        )];
        let failures = check(&reports, 3);
        let names: Vec<&str> = failures[0]
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect();
        assert_eq!(names, vec!["Shape.area", "top"]);
    }
}
