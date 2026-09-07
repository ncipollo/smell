//! Comment-length measure: comment runs longer than the limit.

use crate::code::FileComplexity;
use crate::feature::complexity::FileReport;
use crate::feature::complexity::check::scope;
use crate::feature::complexity::check::{CheckFailure, Offender};

pub fn failures(reports: &[FileReport], limit: usize) -> Vec<CheckFailure> {
    scope::entries(reports, limit, offenders)
}

fn offenders(complexity: &FileComplexity) -> Vec<Offender> {
    complexity
        .comments
        .iter()
        .map(|span| Offender {
            name: format!("lines {}-{}", span.start_line, span.end_line),
            value: span.lines(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::CommentSpan;

    fn span(start_line: usize, end_line: usize) -> CommentSpan {
        CommentSpan {
            start_line,
            end_line,
        }
    }

    fn report(path: &str, comments: Vec<CommentSpan>) -> FileReport {
        FileReport {
            path: PathBuf::from(path),
            lines: 1,
            complexity: FileComplexity {
                comments,
                ..Default::default()
            },
        }
    }

    #[test]
    fn no_reports_pass() {
        assert!(failures(&[], 1).is_empty());
    }

    #[test]
    fn comment_run_at_the_limit_passes() {
        let reports = [report("a.rs", vec![span(1, 3)])];
        assert!(failures(&reports, 3).is_empty());
    }

    #[test]
    fn comment_run_over_the_limit_fails() {
        let reports = [report("a.rs", vec![span(3, 5)])];
        let result = failures(&reports, 2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
        let offenders = result[0].subject.entries();
        assert_eq!(offenders[0].name, "lines 3-5");
        assert_eq!(offenders[0].value, 3);
    }

    #[test]
    fn file_with_no_comments_yields_nothing() {
        let reports = [report("a.rs", vec![])];
        assert!(failures(&reports, 0).is_empty());
    }

    #[test]
    fn a_file_lists_all_of_its_offending_runs() {
        let reports = [report("a.rs", vec![span(1, 5), span(10, 10), span(20, 24)])];
        let result = failures(&reports, 2);
        let names: Vec<&str> = result[0]
            .subject
            .entries()
            .iter()
            .map(|offender| offender.name.as_str())
            .collect();
        assert_eq!(names, vec!["lines 1-5", "lines 20-24"]);
    }

    #[test]
    fn passing_files_are_omitted() {
        let reports = [
            report("ok.rs", vec![span(1, 2)]),
            report("bad.rs", vec![span(1, 10)]),
        ];
        let result = failures(&reports, 3);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
