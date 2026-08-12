//! Complexity measure: functions whose complexity exceeds the limit.

use crate::code::FileComplexity;
use crate::feature::complexity::FileReport;
use crate::feature::complexity::check::scope;
use crate::feature::complexity::check::{CheckFailure, Offender};

pub fn failures(reports: &[FileReport], limit: usize) -> Vec<CheckFailure> {
    scope::entries(reports, limit, offenders)
}

/// Functions inside a type carry a `Type.function` qualified name; top-level
/// functions are bare.
fn offenders(complexity: &FileComplexity) -> Vec<Offender> {
    let type_functions = complexity.types.iter().flat_map(|complexity_type| {
        complexity_type.functions.iter().map(|function| Offender {
            name: format!("{}.{}", complexity_type.name, function.name),
            value: function.complexity,
        })
    });
    let top_level = complexity.functions.iter().map(|function| Offender {
        name: function.name.clone(),
        value: function.complexity,
    });
    type_functions.chain(top_level).collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

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
            lines: 1,
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
        assert!(failures(&[], 1).is_empty());
    }

    #[test]
    fn complexity_at_the_limit_passes() {
        let reports = [report("a.rs", vec![function("top", 3)], vec![])];
        assert!(failures(&reports, 3).is_empty());
    }

    #[test]
    fn complexity_over_the_limit_fails() {
        let reports = [report("a.rs", vec![function("top", 4)], vec![])];
        let result = failures(&reports, 3);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
        let offenders = result[0].subject.entries();
        assert_eq!(offenders.len(), 1);
        assert_eq!(offenders[0].name, "top");
        assert_eq!(offenders[0].value, 4);
    }

    #[test]
    fn type_functions_are_qualified_with_the_type_name() {
        let reports = [report(
            "a.rs",
            vec![],
            vec![shape(vec![function("area", 9)])],
        )];
        let result = failures(&reports, 3);
        assert_eq!(result[0].subject.entries()[0].name, "Shape.area");
    }

    #[test]
    fn passing_files_are_omitted() {
        let reports = [
            report("ok.rs", vec![function("simple", 1)], vec![]),
            report("bad.rs", vec![function("branchy", 8)], vec![]),
        ];
        let result = failures(&reports, 3);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }

    #[test]
    fn a_file_lists_all_of_its_offending_functions() {
        let reports = [report(
            "a.rs",
            vec![function("top", 5)],
            vec![shape(vec![function("area", 9), function("label", 2)])],
        )];
        let result = failures(&reports, 3);
        let names: Vec<&str> = result[0]
            .subject
            .entries()
            .iter()
            .map(|offender| offender.name.as_str())
            .collect();
        assert_eq!(names, vec!["Shape.area", "top"]);
    }
}
