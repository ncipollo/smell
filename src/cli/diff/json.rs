//! `--json` rendering: DTOs mirroring the wire format, built from
//! [`DiffReport`] rather than derived on the domain types directly.

use serde::Serialize;

use crate::CheckResult;
use crate::feature::diff::{
    DiffPathError, DiffReport, DiffStatus, FileDiff, FlatActivity, FunctionDiff,
};

mod check;

use check::Check;

/// Renders the diff (and, when any delta measure is configured, its check
/// result) as a pretty-printed JSON document.
pub fn render(report: &DiffReport, results: &[CheckResult]) -> String {
    let document = Document {
        files: report.files.iter().map(File::new).collect(),
        check: Check::new(results),
        errors: report.errors.iter().map(Error::new).collect(),
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
    #[serde(skip_serializing_if = "is_false")]
    new_file: bool,
    #[serde(skip_serializing_if = "is_false")]
    removed_file: bool,
    functions: Vec<Function>,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl File {
    fn new(file: &FileDiff) -> Self {
        File {
            path: file.path.display().to_string(),
            new_file: file.is_new(),
            removed_file: file.is_removed(),
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
    /// Set when `status` is `unchanged` but calls or mutations moved — a
    /// "flat but active" change complexity alone can't see.
    #[serde(skip_serializing_if = "Option::is_none")]
    flat_active: Option<FlatActiveDto>,
}

impl Function {
    fn new(function: &FunctionDiff) -> Self {
        Function {
            type_name: function.type_name.clone(),
            name: function.name.clone(),
            status: Status::new(&function.status),
            flat_active: function.flat_active.as_ref().map(FlatActiveDto::new),
        }
    }
}

#[derive(Serialize)]
struct FlatActiveDto {
    calls: Delta,
    mutations: Delta,
}

#[derive(Serialize)]
struct Delta {
    before: usize,
    after: usize,
    delta: i64,
}

impl FlatActiveDto {
    fn new(activity: &FlatActivity) -> Self {
        FlatActiveDto {
            calls: Delta {
                before: activity.calls_before,
                after: activity.calls_after,
                delta: activity.calls_delta(),
            },
            mutations: Delta {
                before: activity.mutations_before,
                after: activity.mutations_after,
                delta: activity.mutations_delta(),
            },
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
    use crate::feature::diff::Snapshot;

    fn parse(report: &DiffReport) -> Value {
        serde_json::from_str(&render(report, &[])).expect("valid json")
    }

    fn function(type_name: Option<&str>, name: &str, status: DiffStatus) -> FunctionDiff {
        FunctionDiff {
            type_name: type_name.map(str::to_string),
            name: name.to_string(),
            status,
            flat_active: None,
        }
    }

    #[test]
    fn renders_added_removed_changed_and_unchanged() {
        let report = DiffReport {
            files: vec![FileDiff {
                path: PathBuf::from("src/a.rs"),
                old: None,
                new: None,
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
    fn renders_flat_active_as_a_sibling_of_an_unchanged_status() {
        let report = DiffReport {
            files: vec![FileDiff {
                path: PathBuf::from("src/a.rs"),
                old: None,
                new: None,
                functions: vec![FunctionDiff {
                    type_name: None,
                    name: "a".to_string(),
                    status: DiffStatus::Unchanged { complexity: 1 },
                    flat_active: Some(FlatActivity {
                        calls_before: 2,
                        calls_after: 4,
                        mutations_before: 1,
                        mutations_after: 1,
                    }),
                }],
            }],
            errors: vec![],
        };
        let document = parse(&report);
        assert_eq!(
            document["files"][0]["functions"][0],
            json!({
                "name": "a",
                "status": "unchanged",
                "complexity": 1,
                "flat_active": {
                    "calls": { "before": 2, "after": 4, "delta": 2 },
                    "mutations": { "before": 1, "after": 1, "delta": 0 },
                }
            })
        );
    }

    fn snapshot() -> Snapshot {
        Snapshot {
            lines: 1,
            complexity: crate::code::FileComplexity::default(),
        }
    }

    #[test]
    fn new_file_is_flagged_and_removed_file_is_not() {
        let report = DiffReport {
            files: vec![FileDiff {
                path: PathBuf::from("src/brandnew.rs"),
                old: None,
                new: Some(snapshot()),
                functions: vec![function(None, "a", DiffStatus::Added { complexity: 1 })],
            }],
            errors: vec![],
        };
        let document = parse(&report);
        assert_eq!(document["files"][0]["new_file"], json!(true));
        assert!(document["files"][0].get("removed_file").is_none());
    }

    #[test]
    fn removed_file_is_flagged_and_new_file_is_not() {
        let report = DiffReport {
            files: vec![FileDiff {
                path: PathBuf::from("src/gone.rs"),
                old: Some(snapshot()),
                new: None,
                functions: vec![function(None, "a", DiffStatus::Removed { complexity: 1 })],
            }],
            errors: vec![],
        };
        let document = parse(&report);
        assert_eq!(document["files"][0]["removed_file"], json!(true));
        assert!(document["files"][0].get("new_file").is_none());
    }

    #[test]
    fn an_existing_file_flags_neither() {
        let report = DiffReport {
            files: vec![FileDiff {
                path: PathBuf::from("src/a.rs"),
                old: Some(snapshot()),
                new: Some(snapshot()),
                functions: vec![function(
                    None,
                    "a",
                    DiffStatus::Changed {
                        before: 1,
                        after: 2,
                    },
                )],
            }],
            errors: vec![],
        };
        let document = parse(&report);
        assert!(document["files"][0].get("new_file").is_none());
        assert!(document["files"][0].get("removed_file").is_none());
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

    #[test]
    fn check_is_omitted_when_no_delta_measure_is_configured() {
        let report = DiffReport {
            files: vec![],
            errors: vec![],
        };
        let document = serde_json::from_str::<Value>(&render(&report, &[])).expect("valid json");
        assert!(document.get("check").is_none());
    }

    #[test]
    fn check_is_present_when_a_delta_measure_is_configured() {
        use crate::feature::complexity::check::{CheckFailure, Measure, Subject};

        let report = DiffReport {
            files: vec![],
            errors: vec![],
        };
        let results = [CheckResult {
            measure: Measure::Lines,
            limit: 10,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/a.rs"),
                subject: Subject::File(15),
            }],
        }];
        let document =
            serde_json::from_str::<Value>(&render(&report, &results)).expect("valid json");
        assert_eq!(
            document["check"]["lines"],
            json!({ "limit": 10, "failures": [{ "path": "src/a.rs", "delta": 15 }] })
        );
    }
}
