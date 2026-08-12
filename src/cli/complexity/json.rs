//! `--json` rendering: DTOs mirroring the wire format, built from `FileReport`
//! and `CheckResult` rather than derived on the domain types directly, since
//! the `code` layer shouldn't grow serde derives.

use serde::Serialize;

use crate::code::{ComplexityRollup, FunctionComplexity, TypeComplexity};
use crate::feature::complexity::check::{CheckFailure, CheckResult, Measure, Offender};
use crate::{FileReport, PathError};

/// Renders the analysis (and, when any measure is configured, its check
/// result) as a pretty-printed JSON document.
pub fn render(reports: &[FileReport], results: &[CheckResult], errors: &[PathError]) -> String {
    let document = Document {
        files: reports.iter().map(File::new).collect(),
        check: Check::new(results),
        errors: errors.iter().map(Error::new).collect(),
    };
    serde_json::to_string_pretty(&document).expect("DTOs are always representable as JSON")
}

#[derive(Serialize)]
struct Document {
    files: Vec<File>,
    #[serde(skip_serializing_if = "Option::is_none")]
    check: Option<Check>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<Error>,
}

#[derive(Serialize)]
struct Error {
    path: String,
    message: String,
}

impl Error {
    fn new(error: &PathError) -> Self {
        Error {
            path: error.path.display().to_string(),
            message: error.error.to_string(),
        }
    }
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
    methods: usize,
    functions: Vec<Function>,
    rollup: Rollup,
}

impl Type {
    fn new(complexity_type: &TypeComplexity) -> Self {
        Type {
            name: complexity_type.name.clone(),
            methods: complexity_type.functions.len(),
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

    fn from_offender(offender: &Offender) -> Self {
        Function {
            name: offender.name.clone(),
            complexity: offender.value,
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

/// One key per measure, present only when that measure was configured; the
/// whole object is omitted when no measure was.
#[derive(Serialize)]
struct Check {
    #[serde(skip_serializing_if = "Option::is_none")]
    complexity: Option<ComplexityCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    methods: Option<MethodsCheck>,
}

impl Check {
    fn new(results: &[CheckResult]) -> Option<Self> {
        if results.is_empty() {
            return None;
        }
        let complexity = results
            .iter()
            .find(|result| result.measure == Measure::Complexity)
            .map(ComplexityCheck::new);
        let methods = results
            .iter()
            .find(|result| result.measure == Measure::Methods)
            .map(MethodsCheck::new);
        Some(Check {
            complexity,
            methods,
        })
    }
}

#[derive(Serialize)]
struct ComplexityCheck {
    limit: usize,
    failures: Vec<ComplexityFailure>,
}

impl ComplexityCheck {
    fn new(result: &CheckResult) -> Self {
        ComplexityCheck {
            limit: result.limit,
            failures: result.failures.iter().map(ComplexityFailure::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct ComplexityFailure {
    path: String,
    functions: Vec<Function>,
}

impl ComplexityFailure {
    fn new(failure: &CheckFailure) -> Self {
        ComplexityFailure {
            path: failure.path.display().to_string(),
            functions: failure
                .offenders
                .iter()
                .map(Function::from_offender)
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct MethodsCheck {
    limit: usize,
    failures: Vec<MethodsFailure>,
}

impl MethodsCheck {
    fn new(result: &CheckResult) -> Self {
        MethodsCheck {
            limit: result.limit,
            failures: result.failures.iter().map(MethodsFailure::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct MethodsFailure {
    path: String,
    types: Vec<TypeOffender>,
}

impl MethodsFailure {
    fn new(failure: &CheckFailure) -> Self {
        MethodsFailure {
            path: failure.path.display().to_string(),
            types: failure.offenders.iter().map(TypeOffender::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct TypeOffender {
    name: String,
    methods: usize,
}

impl TypeOffender {
    fn new(offender: &Offender) -> Self {
        TypeOffender {
            name: offender.name.clone(),
            methods: offender.value,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::*;
    use crate::code::{FileComplexity, FunctionComplexity};

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

    fn parse(reports: &[FileReport], results: &[CheckResult]) -> Value {
        serde_json::from_str(&render(reports, results, &[])).expect("valid json")
    }

    fn offender(name: &str, value: usize) -> Offender {
        Offender {
            name: name.to_string(),
            value,
        }
    }

    #[test]
    fn renders_a_file_with_top_level_functions_and_types() {
        let reports = [report_with_functions_and_types()];
        let document = parse(&reports, &[]);
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
        assert_eq!(complexity_type["methods"], 1);
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
    fn check_is_omitted_without_any_configured_measure() {
        let reports = [report_with_functions_and_types()];
        let document = parse(&reports, &[]);
        assert!(document.get("check").is_none());
    }

    #[test]
    fn complexity_check_is_present_with_a_limit_and_no_failures() {
        let reports = [report_with_functions_and_types()];
        let results = [CheckResult {
            measure: Measure::Complexity,
            limit: 10,
            failures: vec![],
        }];
        let document = parse(&reports, &results);
        assert_eq!(
            document["check"],
            json!({ "complexity": { "limit": 10, "failures": [] } })
        );
    }

    #[test]
    fn complexity_check_failures_include_qualified_function_names() {
        let reports = [report_with_functions_and_types()];
        let results = [CheckResult {
            measure: Measure::Complexity,
            limit: 2,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                offenders: vec![offender("Shape.area", 3)],
            }],
        }];
        let document = parse(&reports, &results);
        assert_eq!(
            document["check"],
            json!({
                "complexity": {
                    "limit": 2,
                    "failures": [
                        { "path": "src/foo.rs", "functions": [{ "name": "Shape.area", "complexity": 3 }] }
                    ]
                }
            })
        );
    }

    #[test]
    fn methods_check_uses_type_and_methods_fields() {
        let reports = [report_with_functions_and_types()];
        let results = [CheckResult {
            measure: Measure::Methods,
            limit: 5,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                offenders: vec![offender("Shape", 8)],
            }],
        }];
        let document = parse(&reports, &results);
        assert_eq!(
            document["check"],
            json!({
                "methods": {
                    "limit": 5,
                    "failures": [
                        { "path": "src/foo.rs", "types": [{ "name": "Shape", "methods": 8 }] }
                    ]
                }
            })
        );
        assert!(document["check"].get("complexity").is_none());
    }

    #[test]
    fn both_measures_are_present_when_both_are_configured() {
        let reports = [report_with_functions_and_types()];
        let results = [
            CheckResult {
                measure: Measure::Complexity,
                limit: 10,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::Methods,
                limit: 5,
                failures: vec![],
            },
        ];
        let document = parse(&reports, &results);
        assert!(document["check"]["complexity"].is_object());
        assert!(document["check"]["methods"].is_object());
    }

    #[test]
    fn errors_are_omitted_when_empty() {
        let reports = [report_with_functions_and_types()];
        let document = parse(&reports, &[]);
        assert!(document.get("errors").is_none());
    }

    #[test]
    fn errors_are_present_when_a_path_failed() {
        let reports = [report_with_functions_and_types()];
        let errors = [PathError {
            path: PathBuf::from("gone.rs"),
            error: io::Error::new(io::ErrorKind::NotFound, "No such file or directory"),
        }];
        let document: Value =
            serde_json::from_str(&render(&reports, &[], &errors)).expect("valid json");
        assert_eq!(
            document["errors"],
            json!([{ "path": "gone.rs", "message": "No such file or directory" }])
        );
    }
}
