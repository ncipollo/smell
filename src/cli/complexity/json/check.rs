//! Per-measure `--json` check DTOs: one `*Check`/`*Failure` pair per
//! `Measure`, assembled into the `check` object `super::render` embeds in
//! the document.

use serde::Serialize;

use super::Function;
use crate::feature::complexity::check::{CheckFailure, CheckResult, Measure, Offender, Subject};

/// One key per measure, present only when that measure was configured; the
/// whole object is omitted when no measure was.
#[derive(Serialize)]
pub(super) struct Check {
    #[serde(skip_serializing_if = "Option::is_none")]
    complexity: Option<ComplexityCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    methods: Option<MethodsCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lines: Option<LinesCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    declarations: Option<DeclarationsCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment_lines: Option<CommentLinesCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    comments: Option<CommentsCheck>,
}

impl Check {
    pub(super) fn new(results: &[CheckResult]) -> Option<Self> {
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
        let lines = results
            .iter()
            .find(|result| result.measure == Measure::Lines)
            .map(LinesCheck::new);
        let declarations = results
            .iter()
            .find(|result| result.measure == Measure::Declarations)
            .map(DeclarationsCheck::new);
        let comment_lines = results
            .iter()
            .find(|result| result.measure == Measure::CommentLines)
            .map(CommentLinesCheck::new);
        let comments = results
            .iter()
            .find(|result| result.measure == Measure::Comments)
            .map(CommentsCheck::new);
        Some(Check {
            complexity,
            methods,
            lines,
            declarations,
            comment_lines,
            comments,
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
                .subject
                .entries()
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

#[derive(Serialize)]
struct LinesCheck {
    limit: usize,
    failures: Vec<LinesFailure>,
}

impl LinesCheck {
    fn new(result: &CheckResult) -> Self {
        LinesCheck {
            limit: result.limit,
            failures: result.failures.iter().map(LinesFailure::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct LinesFailure {
    path: String,
    lines: usize,
}

impl LinesFailure {
    fn new(failure: &CheckFailure) -> Self {
        let Subject::File(lines) = &failure.subject else {
            unreachable!("a Measure::Lines result only ever produces a File subject")
        };
        LinesFailure {
            path: failure.path.display().to_string(),
            lines: *lines,
        }
    }
}

#[derive(Serialize)]
struct DeclarationsCheck {
    limit: usize,
    failures: Vec<DeclarationsFailure>,
}

impl DeclarationsCheck {
    fn new(result: &CheckResult) -> Self {
        DeclarationsCheck {
            limit: result.limit,
            failures: result
                .failures
                .iter()
                .map(DeclarationsFailure::new)
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct DeclarationsFailure {
    path: String,
    declarations: usize,
}

impl DeclarationsFailure {
    fn new(failure: &CheckFailure) -> Self {
        let Subject::File(declarations) = &failure.subject else {
            unreachable!("a Measure::Declarations result only ever produces a File subject")
        };
        DeclarationsFailure {
            path: failure.path.display().to_string(),
            declarations: *declarations,
        }
    }
}

#[derive(Serialize)]
struct CommentLinesCheck {
    limit: usize,
    failures: Vec<CommentLinesFailure>,
}

impl CommentLinesCheck {
    fn new(result: &CheckResult) -> Self {
        CommentLinesCheck {
            limit: result.limit,
            failures: result
                .failures
                .iter()
                .map(CommentLinesFailure::new)
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct CommentLinesFailure {
    path: String,
    comments: Vec<CommentOffender>,
}

impl CommentLinesFailure {
    fn new(failure: &CheckFailure) -> Self {
        CommentLinesFailure {
            path: failure.path.display().to_string(),
            comments: failure
                .subject
                .entries()
                .iter()
                .map(CommentOffender::new)
                .collect(),
        }
    }
}

#[derive(Serialize)]
struct CommentOffender {
    lines: String,
    length: usize,
}

impl CommentOffender {
    fn new(offender: &Offender) -> Self {
        CommentOffender {
            lines: offender.name.clone(),
            length: offender.value,
        }
    }
}

#[derive(Serialize)]
struct CommentsCheck {
    limit: usize,
    failures: Vec<CommentsFailure>,
}

impl CommentsCheck {
    fn new(result: &CheckResult) -> Self {
        CommentsCheck {
            limit: result.limit,
            failures: result.failures.iter().map(CommentsFailure::new).collect(),
        }
    }
}

#[derive(Serialize)]
struct CommentsFailure {
    path: String,
    comments: usize,
}

impl CommentsFailure {
    fn new(failure: &CheckFailure) -> Self {
        let Subject::File(comments) = &failure.subject else {
            unreachable!("a Measure::Comments result only ever produces a File subject")
        };
        CommentsFailure {
            path: failure.path.display().to_string(),
            comments: *comments,
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
    fn complexity_check_is_present_with_a_limit_and_no_failures() {
        let results = [CheckResult {
            measure: Measure::Complexity,
            limit: 10,
            failures: vec![],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({ "complexity": { "limit": 10, "failures": [] } })
        );
    }

    #[test]
    fn complexity_check_failures_include_qualified_function_names() {
        let results = [CheckResult {
            measure: Measure::Complexity,
            limit: 2,
            failures: vec![entries_failure(
                "src/foo.rs",
                vec![offender("Shape.area", 3)],
            )],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
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
        let results = [CheckResult {
            measure: Measure::Methods,
            limit: 5,
            failures: vec![entries_failure("src/foo.rs", vec![offender("Shape", 8)])],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "methods": {
                    "limit": 5,
                    "failures": [
                        { "path": "src/foo.rs", "types": [{ "name": "Shape", "methods": 8 }] }
                    ]
                }
            })
        );
        assert!(document.get("complexity").is_none());
    }

    #[test]
    fn lines_check_uses_path_and_lines_fields() {
        let results = [CheckResult {
            measure: Measure::Lines,
            limit: 100,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                subject: Subject::File(150),
            }],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "lines": {
                    "limit": 100,
                    "failures": [{ "path": "src/foo.rs", "lines": 150 }]
                }
            })
        );
    }

    #[test]
    fn declarations_check_uses_path_and_declarations_fields() {
        let results = [CheckResult {
            measure: Measure::Declarations,
            limit: 3,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/foo.rs"),
                subject: Subject::File(4),
            }],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "declarations": {
                    "limit": 3,
                    "failures": [{ "path": "src/foo.rs", "declarations": 4 }]
                }
            })
        );
    }

    #[test]
    fn comment_lines_check_uses_lines_and_length_fields() {
        let results = [CheckResult {
            measure: Measure::CommentLines,
            limit: 5,
            failures: vec![entries_failure(
                "src/foo.rs",
                vec![offender("lines 3-9", 7)],
            )],
        }];
        let document = parse(&results);
        assert_eq!(
            document,
            json!({
                "comment_lines": {
                    "limit": 5,
                    "failures": [
                        { "path": "src/foo.rs", "comments": [{ "lines": "lines 3-9", "length": 7 }] }
                    ]
                }
            })
        );
    }

    #[test]
    fn comments_check_uses_path_and_comments_fields() {
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
                    "failures": [{ "path": "src/foo.rs", "comments": 6 }]
                }
            })
        );
    }

    #[test]
    fn all_measures_are_present_when_all_are_configured() {
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
            CheckResult {
                measure: Measure::Lines,
                limit: 100,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::Declarations,
                limit: 20,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::CommentLines,
                limit: 40,
                failures: vec![],
            },
            CheckResult {
                measure: Measure::Comments,
                limit: 25,
                failures: vec![],
            },
        ];
        let document = parse(&results);
        assert!(document["complexity"].is_object());
        assert!(document["methods"].is_object());
        assert!(document["lines"].is_object());
        assert!(document["declarations"].is_object());
        assert!(document["comment_lines"].is_object());
        assert!(document["comments"].is_object());
    }
}
