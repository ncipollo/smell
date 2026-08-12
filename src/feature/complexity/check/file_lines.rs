//! Line-count measure: files longer than the limit.
//!
//! Unlike the other measures, the subject is the file itself rather than a
//! named entry inside it. A file dropped by `--implements` (post-filter,
//! no `FileReport` at all) is not line-checked either, same as method count.

use crate::feature::complexity::FileReport;
use crate::feature::complexity::check::CheckFailure;
use crate::feature::complexity::check::scope;

pub fn failures(reports: &[FileReport], limit: usize) -> Vec<CheckFailure> {
    scope::file(reports, limit, |report| report.lines)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::FileComplexity;
    use crate::feature::complexity::check::Subject;

    fn report(path: &str, lines: usize) -> FileReport {
        FileReport {
            path: PathBuf::from(path),
            lines,
            complexity: FileComplexity {
                functions: Vec::new(),
                types: Vec::new(),
            },
        }
    }

    #[test]
    fn no_reports_pass() {
        assert!(failures(&[], 1).is_empty());
    }

    #[test]
    fn line_count_at_the_limit_passes() {
        let reports = [report("a.rs", 100)];
        assert!(failures(&reports, 100).is_empty());
    }

    #[test]
    fn line_count_over_the_limit_fails() {
        let reports = [report("a.rs", 101)];
        let result = failures(&reports, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
        match result[0].subject {
            Subject::File(lines) => assert_eq!(lines, 101),
            Subject::Entries(_) => panic!("expected a file subject"),
        }
    }

    #[test]
    fn passing_files_are_omitted() {
        let reports = [report("ok.rs", 10), report("bad.rs", 200)];
        let result = failures(&reports, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
