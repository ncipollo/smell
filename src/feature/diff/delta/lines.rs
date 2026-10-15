//! Line-count delta measure: files whose line count swung by more than the
//! limit between the two revisions (a file present on only one side counts
//! as a swing from/to zero).

use crate::feature::complexity::check::CheckFailure;
use crate::feature::diff::FileDiff;
use crate::feature::diff::delta::scope;

pub fn failures(files: &[FileDiff], limit: usize) -> Vec<CheckFailure> {
    scope::file(files, limit, delta)
}

fn delta(file: &FileDiff) -> i64 {
    let old = file.old.as_ref().map_or(0, |snapshot| snapshot.lines) as i64;
    let new = file.new.as_ref().map_or(0, |snapshot| snapshot.lines) as i64;
    new - old
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::code::FileComplexity;
    use crate::feature::diff::Snapshot;

    fn snapshot(lines: usize) -> Snapshot {
        Snapshot {
            lines,
            complexity: FileComplexity::default(),
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
        let files = [file("a.rs", Some(10), Some(15))];
        assert!(failures(&files, 5).is_empty());
    }

    #[test]
    fn delta_over_the_limit_fails() {
        let files = [file("a.rs", Some(10), Some(16))];
        let result = failures(&files, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("a.rs"));
        assert!(result[0].subject.entries().is_empty());
    }

    #[test]
    fn a_shrinking_delta_fails_on_magnitude_too() {
        let files = [file("a.rs", Some(16), Some(10))];
        assert_eq!(failures(&files, 5).len(), 1);
    }

    #[test]
    fn a_file_added_outright_is_a_delta_from_zero() {
        let files = [file("a.rs", None, Some(10))];
        assert_eq!(failures(&files, 5).len(), 1);
        assert!(failures(&files, 10).is_empty());
    }

    #[test]
    fn a_file_removed_outright_is_a_delta_to_zero() {
        let files = [file("a.rs", Some(10), None)];
        assert_eq!(failures(&files, 5).len(), 1);
    }

    #[test]
    fn passing_files_are_omitted() {
        let files = [
            file("ok.rs", Some(10), Some(11)),
            file("bad.rs", Some(10), Some(50)),
        ];
        let result = failures(&files, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("bad.rs"));
    }
}
