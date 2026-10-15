//! `--json` rendering: DTOs mirroring the wire format, built from
//! [`DiffReport`] rather than derived on the domain types directly.

use serde::Serialize;

use crate::feature::diff::{DiffPathError, DiffReport, DiffStatus, FileDiff, FunctionDiff};

pub fn render(report: &DiffReport) -> String {
    let document = Document {
        files: report.files.iter().map(File::new).collect(),
        errors: report.errors.iter().map(Error::new).collect(),
    };
    serde_json::to_string_pretty(&document).expect("DTOs are always representable as JSON")
}

#[derive(Serialize)]
struct Document {
    files: Vec<File>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<Error>,
}

#[derive(Serialize)]
struct Error {
    path: String,
    message: String,
}

impl Error {
    fn new(error: &DiffPathError) -> Self {
        Error {
            path: error.path.display().to_string(),
            message: error.message.clone(),
        }
    }
}

#[derive(Serialize)]
struct File {
    path: String,
    functions: Vec<Function>,
}

impl File {
    fn new(file: &FileDiff) -> Self {
        File {
            path: file.path.display().to_string(),
            functions: file.functions.iter().map(Function::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct Function {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_name: Option<String>,
    name: String,
    #[serde(flatten)]
    status: Status,
}

impl Function {
    fn new(function: &FunctionDiff) -> Self {
        Function {
            type_name: function.type_name.clone(),
            name: function.name.clone(),
            status: Status::new(&function.status),
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum Status {
    Added {
        complexity: usize,
    },
    Removed {
        complexity: usize,
    },
    Changed {
        before: usize,
        after: usize,
        delta: i64,
    },
    Unchanged {
        complexity: usize,
    },
}

impl Status {
    fn new(status: &DiffStatus) -> Self {
        match *status {
            DiffStatus::Added { complexity } => Status::Added { complexity },
            DiffStatus::Removed { complexity } => Status::Removed { complexity },
            DiffStatus::Changed { before, after } => Status::Changed {
                before,
                after,
                delta: status.delta(),
            },
            DiffStatus::Unchanged { complexity } => Status::Unchanged { complexity },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::*;

    fn parse(report: &DiffReport) -> Value {
        serde_json::from_str(&render(report)).expect("valid json")
    }

    fn function(type_name: Option<&str>, name: &str, status: DiffStatus) -> FunctionDiff {
        FunctionDiff {
            type_name: type_name.map(str::to_string),
            name: name.to_string(),
            status,
        }
    }

    #[test]
    fn renders_added_removed_changed_and_unchanged() {
        let report = DiffReport {
            files: vec![FileDiff {
                path: PathBuf::from("src/a.rs"),
                functions: vec![
                    function(None, "added", DiffStatus::Added { complexity: 2 }),
                    function(None, "removed", DiffStatus::Removed { complexity: 3 }),
                    function(
                        Some("Shape"),
                        "area",
                        DiffStatus::Changed {
                            before: 3,
                            after: 7,
                        },
                    ),
                    function(None, "same", DiffStatus::Unchanged { complexity: 1 }),
                ],
            }],
            errors: vec![],
        };
        let document = parse(&report);
        let file = &document["files"][0];
        assert_eq!(file["path"], "src/a.rs");
        assert_eq!(
            file["functions"][0],
            json!({ "name": "added", "status": "added", "complexity": 2 })
        );
        assert_eq!(
            file["functions"][1],
            json!({ "name": "removed", "status": "removed", "complexity": 3 })
        );
        assert_eq!(
            file["functions"][2],
            json!({ "type": "Shape", "name": "area", "status": "changed", "before": 3, "after": 7, "delta": 4 })
        );
        assert_eq!(
            file["functions"][3],
            json!({ "name": "same", "status": "unchanged", "complexity": 1 })
        );
    }

    #[test]
    fn errors_are_omitted_when_empty() {
        let report = DiffReport {
            files: vec![],
            errors: vec![],
        };
        assert!(parse(&report).get("errors").is_none());
    }

    #[test]
    fn errors_are_present_when_a_path_failed() {
        let report = DiffReport {
            files: vec![],
            errors: vec![DiffPathError {
                path: PathBuf::from("gone.rs"),
                message: "not found in either revision".to_string(),
            }],
        };
        let document = parse(&report);
        assert_eq!(
            document["errors"],
            json!([{ "path": "gone.rs", "message": "not found in either revision" }])
        );
    }
}
