//! `--json` rendering: DTOs mirroring the wire format, built from `FileReport`
//! and `CheckFailure` rather than derived on the domain types directly, since
//! the `code` layer shouldn't grow serde derives.

use serde::Serialize;

use crate::code::{ComplexityRollup, FunctionComplexity, TypeComplexity};
use crate::feature::complexity::check::FailedFunction;
use crate::{CheckFailure, FileReport};

/// Renders the analysis (and, when a limit is set, the check result) as a
/// pretty-printed JSON document.
pub fn render(reports: &[FileReport], limit: Option<usize>, failures: &[CheckFailure]) -> String {
    let document = Document {
        files: reports.iter().map(File::new).collect(),
        check: limit.map(|limit| Check::new(limit, failures)),
    };
    serde_json::to_string_pretty(&document).expect("DTOs are always representable as JSON")
}

#[derive(Serialize)]
struct Document {
    files: Vec<File>,
    #[serde(skip_serializing_if = "Option::is_none")]
    check: Option<Check>,
}

#[derive(Serialize)]
struct File {
    path: String,
    types: Vec<Type>,
    functions: Vec<Function>,
    rollup: Rollup,
}

impl File {
    fn new(report: &FileReport) -> Self {
        File {
            path: report.path.display().to_string(),
            types: report.complexity.types.iter().map(Type::new).collect(),
            functions: report
                .complexity
                .functions
                .iter()
                .map(Function::new)
                .collect(),
            rollup: Rollup::new(&report.complexity.rollup()),
        }
    }
}

#[derive(Serialize)]
struct Type {
    name: String,
    functions: Vec<Function>,
    rollup: Rollup,
}

impl Type {
    fn new(complexity_type: &TypeComplexity) -> Self {
        Type {
            name: complexity_type.name.clone(),
            functions: complexity_type
                .functions
                .iter()
                .map(Function::new)
                .collect(),
            rollup: Rollup::new(&complexity_type.rollup()),
        }
    }
}

#[derive(Serialize)]
struct Function {
    name: String,
    complexity: usize,
}

impl Function {
    fn new(function: &FunctionComplexity) -> Self {
        Function {
            name: function.name.clone(),
            complexity: function.complexity,
        }
    }

    fn from_failed(function: &FailedFunction) -> Self {
        Function {
            name: function.name.clone(),
            complexity: function.complexity,
        }
    }
}

#[derive(Serialize)]
struct Rollup {
    total: usize,
    max: usize,
    average: f64,
}

impl Rollup {
    fn new(rollup: &ComplexityRollup) -> Self {
        Rollup {
            total: rollup.total,
            max: rollup.max,
            average: rollup.average,
        }
    }
}

#[derive(Serialize)]
struct Check {
    limit: usize,
    failures: Vec<Failure>,
}

impl Check {
    fn new(limit: usize, failures: &[CheckFailure]) -> Self {
        Check {
            limit,
            failures: failures.iter().map(Failure::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct Failure {
    path: String,
    functions: Vec<Function>,
}

impl Failure {
    fn new(failure: &CheckFailure) -> Self {
        Failure {
            path: failure.path.display().to_string(),
            functions: failure
                .functions
                .iter()
                .map(Function::from_failed)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::*;
    use crate::code::{FileComplexity, FunctionComplexity};
    use crate::feature::complexity::check::FailedFunction;

    fn function(name: &str, complexity: usize) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            complexity,
        }
    }

    fn report_with_functions_and_types() -> FileReport {
        FileReport {
            path: PathBuf::from("src/foo.rs"),
            complexity: FileComplexity {
                functions: vec![function("top_level", 1)],
                types: vec![TypeComplexity {
                    name: "Shape".to_string(),
                    supertypes: vec!["Display".to_string()],
                    functions: vec![function("area", 3)],
                }],
            },
        }
    }

    fn parse(reports: &[FileReport], limit: Option<usize>, failures: &[CheckFailure]) -> Value {
        serde_json::from_str(&render(reports, limit, failures)).expect("valid json")
    }

    #[test]
    fn renders_a_file_with_top_level_functions_and_types() {
        let reports = [report_with_functions_and_types()];
        let document = parse(&reports, None, &[]);
        let file = &document["files"][0];
        assert_eq!(file["path"], "src/foo.rs");
        assert_eq!(
            file["functions"],
            json!([{ "name": "top_level", "complexity": 1 }])
        );
        assert_eq!(
            file["rollup"],
            json!({ "total": 4, "max": 3, "average": 2.0 })
        );
        let complexity_type = &file["types"][0];
        assert_eq!(complexity_type["name"], "Shape");
        assert_eq!(
            complexity_type["functions"],
            json!([{ "name": "area", "complexity": 3 }])
        );
        assert_eq!(
            complexity_type["rollup"],
            json!({ "total": 3, "max": 3, "average": 3.0 })
        );
    }

    #[test]
    fn check_is_omitted_without_a_limit() {
        let reports = [report_with_functions_and_types()];
        let document = parse(&reports, None, &[]);
        assert!(document.get("check").is_none());
    }

    #[test]
    fn check_is_present_with_a_limit_and_no_failures() {
        let reports = [report_with_functions_and_types()];
        let document = parse(&reports, Some(10), &[]);
        assert_eq!(document["check"], json!({ "limit": 10, "failures": [] }));
    }

    #[test]
    fn check_failures_include_qualified_function_names() {
        let reports = [report_with_functions_and_types()];
        let failures = [CheckFailure {
            path: PathBuf::from("src/foo.rs"),
            functions: vec![FailedFunction {
                name: "Shape.area".to_string(),
                complexity: 3,
            }],
        }];
        let document = parse(&reports, Some(2), &failures);
        assert_eq!(
            document["check"],
            json!({
                "limit": 2,
                "failures": [
                    { "path": "src/foo.rs", "functions": [{ "name": "Shape.area", "complexity": 3 }] }
                ]
            })
        );
    }
}
