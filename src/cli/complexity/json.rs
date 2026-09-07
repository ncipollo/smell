//! `--json` rendering: DTOs mirroring the wire format, built from `FileReport`
//! and `CheckResult` rather than derived on the domain types directly, since
//! the `code` layer shouldn't grow serde derives.

use serde::Serialize;

use crate::code::{ComplexityRollup, FunctionComplexity, TypeComplexity};
use crate::feature::complexity::check::{CheckResult, Offender};
use crate::{FileReport, PathError};

mod check;

use check::Check;

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
    lines: usize,
    declarations: usize,
    /// Coalesced comment runs; adjacent comment lines count once. See
    /// [`crate::code::FileComplexity::comment_count`].
    comments: usize,
    types: Vec<Type>,
    functions: Vec<Function>,
    rollup: Rollup,
}

impl File {
    fn new(report: &FileReport) -> Self {
        File {
            path: report.path.display().to_string(),
            lines: report.lines,
            declarations: report.complexity.declarations(),
            comments: report.complexity.comment_count(),
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

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::*;
    use crate::code::{CommentSpan, FileComplexity};

    fn function(name: &str, complexity: usize) -> FunctionComplexity {
        FunctionComplexity {
            name: name.to_string(),
            complexity,
        }
    }

    fn report_with_functions_and_types() -> FileReport {
        FileReport {
            path: PathBuf::from("src/foo.rs"),
            lines: 42,
            complexity: FileComplexity {
                functions: vec![function("top_level", 1)],
                types: vec![TypeComplexity {
                    name: "Shape".to_string(),
                    supertypes: vec!["Display".to_string()],
                    functions: vec![function("area", 3)],
                }],
                comments: vec![CommentSpan {
                    start_line: 1,
                    end_line: 1,
                }],
                ..Default::default()
            },
        }
    }

    fn parse(reports: &[FileReport], results: &[CheckResult]) -> Value {
        serde_json::from_str(&render(reports, results, &[])).expect("valid json")
    }

    #[test]
    fn renders_a_file_with_top_level_functions_and_types() {
        let reports = [report_with_functions_and_types()];
        let document = parse(&reports, &[]);
        let file = &document["files"][0];
        assert_eq!(file["path"], "src/foo.rs");
        assert_eq!(file["lines"], 42);
        assert_eq!(file["declarations"], 2);
        assert_eq!(file["comments"], 1);
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
