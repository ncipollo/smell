//! Comment-count measure: files with more comments (coalesced runs) than
//! the limit.
//!
//! Like line count and declaration count, the subject is the file itself
//! rather than a named entry inside it. Counts run on the already-filtered
//! report (post `--implements`), same as the other file-scoped measures.

use crate::feature::complexity::FileReport;
use crate::feature::complexity::check::CheckFailure;
use crate::feature::complexity::check::scope;

pub fn failures(reports: &[FileReport], limit: usize) -> Vec<CheckFailure> {
    scope::file(reports, limit, |report| report.complexity.comment_count())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::{CommentSpan, FileComplexity};
    use crate::feature::complexity::check::Subject;

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
    fn comment_count_at_the_limit_passes() {
        let reports = [report("a.rs", vec![span(1, 1), span(3, 3)])];
        assert!(failures(&reports, 2).is_empty());
    }

    #[test]
    fn comment_count_over_the_limit_fails() {
        let reports = [report("a.rs", vec![span(1, 1), span(3, 3), span(5, 5)])];
        let result = failures(&reports, 2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
        match result[0].subject {
            Subject::File(count) => assert_eq!(count, 3),
            Subject::Entries(_) => panic!("expected a file subject"),
        }
    }

    #[test]
    fn passing_files_are_omitted() {
        let reports = [
            report("ok.rs", vec![span(1, 1)]),
            report("bad.rs", vec![span(1, 1), span(3, 3), span(5, 5)]),
        ];
        let result = failures(&reports, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
