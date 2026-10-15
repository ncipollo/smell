//! Comment-count delta measure: files whose comment count (coalesced runs)
//! swung by more than the limit between the two revisions.

use crate::feature::complexity::check::CheckFailure;
use crate::feature::diff::FileDiff;
use crate::feature::diff::delta::scope;

pub fn failures(files: &[FileDiff], limit: usize) -> Vec<CheckFailure> {
    scope::file(files, limit, delta)
}

fn delta(file: &FileDiff) -> i64 {
    let old = file
        .old
        .as_ref()
        .map_or(0, |snapshot| snapshot.complexity.comment_count()) as i64;
    let new = file
        .new
        .as_ref()
        .map_or(0, |snapshot| snapshot.complexity.comment_count()) as i64;
    new - old
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::{CommentSpan, FileComplexity};
    use crate::feature::diff::Snapshot;

    fn snapshot(comments: usize) -> Snapshot {
        let comments = (0..comments)
            .map(|index| CommentSpan {
                start_line: index,
                end_line: index,
            })
            .collect();
        Snapshot {
            lines: 1,
            complexity: FileComplexity {
                comments,
                ..Default::default()
            },
        }
    }

    fn file(path: &str, old: Option<usize>, new: Option<usize>) -> FileDiff {
        FileDiff {
            path: PathBuf::from(path),
            functions: Vec::new(),
            old: old.map(snapshot),
            new: new.map(snapshot),
        }
    }

    #[test]
    fn no_files_pass() {
        assert!(failures(&[], 1).is_empty());
    }

    #[test]
    fn delta_at_the_limit_passes() {
        let files = [file("a.rs", Some(1), Some(3))];
        assert!(failures(&files, 2).is_empty());
    }

    #[test]
    fn delta_over_the_limit_fails() {
        let files = [file("a.rs", Some(1), Some(4))];
        assert_eq!(failures(&files, 2).len(), 1);
    }

    #[test]
    fn comments_lost_outright_is_a_delta_to_zero() {
        let files = [file("a.rs", Some(3), None)];
        assert_eq!(failures(&files, 2).len(), 1);
    }

    #[test]
    fn passing_files_are_omitted() {
        let files = [
            file("ok.rs", Some(1), Some(2)),
            file("bad.rs", Some(1), Some(9)),
        ];
        let result = failures(&files, 2);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
