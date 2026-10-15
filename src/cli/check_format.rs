//! Shared stderr formatting for a run's `CheckResult`s, used by both the
//! top-level `complexity` command (absolute `--max-*` limits) and the `diff`
//! subcommand (relative `--delta-*` thresholds) — the types are identical,
//! only the configured measures and their meaning differ.

use crate::{CheckFailure, CheckResult, Measure, Subject};

const RED_BOLD: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

/// One section per failing measure, blank-line separated. A run with a
/// single failing measure renders byte-for-byte what a lone check did before
/// multiple measures existed.
pub fn format_results(results: &[CheckResult], color: bool) -> String {
    results
        .iter()
        .filter(|result| result.failed())
        .map(|result| format_result(result, color))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_result(result: &CheckResult, color: bool) -> String {
    let header = format!(
        "✗ {} check failed: {} file(s) exceed limit {}",
        label(result.measure),
        result.failures.len(),
        result.limit
    );
    let mut text = paint(&header, color);
    text.push('\n');
    for failure in &result.failures {
        text.push_str(&format_failure(failure, color));
    }
    text
}

/// A file-level failure (line count) is one flat `path  value` line; an
/// entry-scoped failure (complexity, method count) lists its named offenders
/// indented beneath the path.
fn format_failure(failure: &CheckFailure, color: bool) -> String {
    match &failure.subject {
        Subject::File(value) => format!(
            "{}  {}\n",
            failure.path.display(),
            paint(&value.to_string(), color)
        ),
        Subject::Entries(offenders) => {
            let mut text = format!("{}\n", failure.path.display());
            for offender in offenders {
                text.push_str(&format!(
                    "  {}  {}\n",
                    offender.name,
                    paint(&offender.value.to_string(), color)
                ));
            }
            text
        }
    }
}

/// User-facing wording for a measure's stderr header. Kept in `cli` since
/// `feature::complexity::check::Measure` is a bare discriminant.
fn label(measure: Measure) -> &'static str {
    match measure {
        Measure::Complexity => "complexity",
        Measure::Methods => "method count",
        Measure::Lines => "line count",
        Measure::Declarations => "declaration count",
        Measure::CommentLines => "comment length",
        Measure::Comments => "comment count",
    }
}

/// Wraps text in bold red when stderr is a terminal; plain otherwise so
/// piped and CI output stays free of escape codes.
fn paint(text: &str, color: bool) -> String {
    if color {
        format!("{RED_BOLD}{text}{RESET}")
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::Offender;

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

    fn complexity_failures() -> Vec<CheckFailure> {
        vec![
            entries_failure("src/a.rs", vec![offender("Shape.area", 12)]),
            entries_failure("src/b.rs", vec![offender("top", 11)]),
        ]
    }

    fn complexity_result(failures: Vec<CheckFailure>) -> CheckResult {
        CheckResult {
            measure: Measure::Complexity,
            limit: 10,
            failures,
        }
    }

    fn methods_result(failures: Vec<CheckFailure>) -> CheckResult {
        CheckResult {
            measure: Measure::Methods,
            limit: 5,
            failures,
        }
    }

    fn lines_result(failures: Vec<CheckFailure>) -> CheckResult {
        CheckResult {
            measure: Measure::Lines,
            limit: 100,
            failures,
        }
    }

    #[test]
    fn format_result_leads_with_a_summary_then_lists_files_and_offenders() {
        let result = complexity_result(complexity_failures());
        assert_eq!(
            format_result(&result, false),
            "✗ complexity check failed: 2 file(s) exceed limit 10\n\
             src/a.rs\n  Shape.area  12\nsrc/b.rs\n  top  11\n"
        );
    }

    #[test]
    fn format_result_paints_the_header_and_values_red_when_colored() {
        let result = complexity_result(complexity_failures());
        let text = format_result(&result, true);
        assert!(text.starts_with(&format!(
            "{RED_BOLD}✗ complexity check failed: 2 file(s) exceed limit 10{RESET}\n"
        )));
        assert!(text.contains(&format!("  Shape.area  {RED_BOLD}12{RESET}\n")));
    }

    #[test]
    fn format_result_without_color_has_no_escape_codes() {
        let result = complexity_result(complexity_failures());
        assert!(!format_result(&result, false).contains('\x1b'));
    }

    #[test]
    fn format_results_uses_the_method_count_label() {
        let result = methods_result(vec![entries_failure(
            "src/a.rs",
            vec![offender("Shape", 8)],
        )]);
        let text = format_results(&[result], false);
        assert!(text.starts_with("✗ method count check failed: 1 file(s) exceed limit 5\n"));
    }

    #[test]
    fn format_results_uses_the_line_count_label() {
        let result = lines_result(vec![CheckFailure {
            path: PathBuf::from("src/a.rs"),
            subject: Subject::File(150),
        }]);
        let text = format_results(&[result], false);
        assert!(text.starts_with("✗ line count check failed: 1 file(s) exceed limit 100\n"));
    }

    #[test]
    fn format_results_uses_the_declaration_count_label() {
        let result = CheckResult {
            measure: Measure::Declarations,
            limit: 5,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/a.rs"),
                subject: Subject::File(6),
            }],
        };
        let text = format_results(&[result], false);
        assert!(text.starts_with("✗ declaration count check failed: 1 file(s) exceed limit 5\n"));
    }

    #[test]
    fn format_results_uses_the_comment_count_label() {
        let result = CheckResult {
            measure: Measure::Comments,
            limit: 5,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/a.rs"),
                subject: Subject::File(6),
            }],
        };
        let text = format_results(&[result], false);
        assert!(text.starts_with("✗ comment count check failed: 1 file(s) exceed limit 5\n"));
    }

    #[test]
    fn format_results_uses_the_comment_length_label() {
        let result = CheckResult {
            measure: Measure::CommentLines,
            limit: 5,
            failures: vec![entries_failure("src/a.rs", vec![offender("lines 3-9", 7)])],
        };
        let text = format_results(&[result], false);
        assert!(text.starts_with("✗ comment length check failed: 1 file(s) exceed limit 5\n"));
    }

    #[test]
    fn format_result_renders_a_file_subject_as_a_flat_line() {
        let result = lines_result(vec![CheckFailure {
            path: PathBuf::from("src/a.rs"),
            subject: Subject::File(150),
        }]);
        assert_eq!(
            format_result(&result, false),
            "✗ line count check failed: 1 file(s) exceed limit 100\nsrc/a.rs  150\n"
        );
    }

    #[test]
    fn format_results_joins_multiple_failing_measures_with_a_blank_line() {
        let results = [
            complexity_result(complexity_failures()),
            methods_result(vec![entries_failure(
                "src/a.rs",
                vec![offender("Shape", 8)],
            )]),
        ];
        let text = format_results(&results, false);
        assert!(text.contains("✗ complexity check failed"));
        assert!(text.contains("✗ method count check failed"));
        assert!(text.contains("check failed: 1 file(s) exceed limit 5\nsrc/a.rs\n"));
    }

    #[test]
    fn format_results_omits_passing_measures() {
        let results = [complexity_result(vec![])];
        assert_eq!(format_results(&results, false), "");
    }

    #[test]
    fn format_results_is_empty_when_no_measures_are_configured() {
        assert_eq!(format_results(&[], false), "");
    }
}
