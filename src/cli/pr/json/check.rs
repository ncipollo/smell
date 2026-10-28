//! Per-measure `--json` delta-check DTOs: one `*Check`/`*Failure` pair per
//! `Measure`, assembled into the `check` object `super::render` embeds in
//! the document. Diff's checks only ever cover
//! Methods/Lines/Declarations/CommentLines/Comments — complexity diffing is
//! unconditional, not a delta check — so there's no `complexity` key here,
//! unlike `cli::complexity::json::check::Check`.

use serde::Serialize;

use crate::feature::complexity::check::{CheckFailure, CheckResult, Measure, Offender, Subject};

/// One key per measure, present only when that measure was configured; the
/// whole object is omitted when no measure was.
#[derive(Serialize)]
pub(super) struct Check {
    #[serde(skip_serializing_if = "Option::is_none")]
    methods: Option<MethodsCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<FileCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    declarations: Option<FileCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment_lines: Option<FileCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comments: Option<FileCheck>,
}

impl Check {
    pub(super) fn new(results: &[CheckResult]) -> Option<Self> {
        if results.is_empty() {
            return None;
        }
        let find = |measure: Measure| results.iter().find(|result| result.measure == measure);
        Some(Check {
            methods: find(Measure::Methods).map(MethodsCheck::new),
            lines: find(Measure::Lines).map(FileCheck::new),
            declarations: find(Measure::Declarations).map(FileCheck::new),
            comment_lines: find(Measure::CommentLines).map(FileCheck::new),
            comments: find(Measure::Comments).map(FileCheck::new),
        })
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
            types: failure
                .subject
                .entries()
                .iter()
                .map(TypeOffender::new)
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct TypeOffender {
    name: String,
    delta: usize,
}

impl TypeOffender {
    fn new(offender: &Offender) -> Self {
        TypeOffender {
            name: offender.name.clone(),
            delta: offender.value,
        }
    }
}

/// Shared shape for the four file-scoped measures (lines, declarations,
/// comments, comment-lines): each failure is just a path and the magnitude
/// of its delta between the two revisions.
#[derive(Serialize)]
struct FileCheck {
    limit: usize,
    failures: Vec<FileFailure>,
}

impl FileCheck {
    fn new(result: &CheckResult) -> Self {
        FileCheck {
            limit: result.limit,
            failures: result.failures.iter().map(FileFailure::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct FileFailure {
    path: String,
    delta: usize,
}

impl FileFailure {
    fn new(failure: &CheckFailure) -> Self {
        let Subject::File(delta) = &failure.subject else {
            unreachable!("a file-scoped delta measure only ever produces a File subject")
        };
        FileFailure {
            path: failure.path.display().to_string(),
            delta: *delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};

    use super::*;

    fn parse(results: &[CheckResult]) -> Value {
        let rendered = serde_json::to_string_pretty(&Check::new(results))
            .expect("DTOs are always representable as JSON");
        serde_json::from_str(&rendered).expect("valid json")
    }

    fn offender(name: &str, value: usize) -> Offender {
        Offender {
            name: name.to_string(),
            value,
        }
    }

    fn entries_failure(path: &str, offenders: Vec<Offender>) -> CheckFailure {
        CheckFailure {
            path: PathBuf::from(path),
            subject: Subject::Entries(offenders),
        }
    }

    #[test]
    fn check_is_none_without_any_configured_measure() {
        assert!(Check::new(&[]).is_none());
    }

    #[test]
    fn methods_check_uses_type_and_delta_fields() {
        let results = [CheckResult {
            measure: Measure::Methods,
            limit: 2,
            failures: vec![entries_failure("src/foo.rs", vec![offender("Shape", 5)])],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "methods": {
                    "limit": 2,
                    "failures": [
                        { "path": "src/foo.rs", "types": [{ "name": "Shape", "delta": 5 }] }
                    ]
                }
            })
        );
    }

    #[test]
    fn lines_check_uses_path_and_delta_fields() {
        let results = [CheckResult {
            measure: Measure::Lines,
            limit: 10,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                subject: Subject::File(15),
            }],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "lines": {
                    "limit": 10,
                    "failures": [{ "path": "src/foo.rs", "delta": 15 }]
                }
            })
        );
        assert!(document.get("declarations").is_none());
    }

    #[test]
    fn declarations_check_uses_path_and_delta_fields() {
        let results = [CheckResult {
            measure: Measure::Declarations,
            limit: 2,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                subject: Subject::File(3),
            }],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "declarations": {
                    "limit": 2,
                    "failures": [{ "path": "src/foo.rs", "delta": 3 }]
                }
            })
        );
    }

    #[test]
    fn comment_lines_check_uses_path_and_delta_fields() {
        let results = [CheckResult {
            measure: Measure::CommentLines,
            limit: 4,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                subject: Subject::File(9),
            }],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "comment_lines": {
                    "limit": 4,
                    "failures": [{ "path": "src/foo.rs", "delta": 9 }]
                }
            })
        );
    }

    #[test]
    fn comments_check_uses_path_and_delta_fields() {
        let results = [CheckResult {
            measure: Measure::Comments,
            limit: 5,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                subject: Subject::File(6),
            }],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "comments": {
                    "limit": 5,
                    "failures": [{ "path": "src/foo.rs", "delta": 6 }]
                }
            })
        );
    }

    #[test]
    fn all_measures_are_present_when_all_are_configured() {
        let results = [
            CheckResult {
                measure: Measure::Methods,
                limit: 2,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::Lines,
                limit: 10,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::Declarations,
                limit: 2,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::CommentLines,
                limit: 4,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::Comments,
                limit: 5,
                failures: vec![],
            },
        ];
        let document = parse(&results);
        assert!(document["methods"].is_object());
        assert!(document["lines"].is_object());
        assert!(document["declarations"].is_object());
        assert!(document["comment_lines"].is_object());
        assert!(document["comments"].is_object());
    }
}
