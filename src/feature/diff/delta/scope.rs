//! Shared combinators for the two shapes a delta measure's subject takes,
//! mirroring `feature::complexity::check::scope` but keyed on a before/after
//! delta rather than a single value: named entries inside a file (types, for
//! method count), or the file itself (line count, declaration count, comment
//! count, comment-line count). Both fail a file when the delta's magnitude is
//! strictly over the limit (equal to it passes).

use crate::feature::complexity::check::{CheckFailure, Offender, Subject};
use crate::feature::diff::FileDiff;

pub fn file(
    files: &[FileDiff],
    limit: usize,
    delta: impl Fn(&FileDiff) -> i64,
) -> Vec<CheckFailure> {
    files
        .iter()
        .filter_map(|file| {
            let value = delta(file).unsigned_abs() as usize;
            if value > limit {
                Some(CheckFailure {
                    path: file.path.clone(),
                    subject: Subject::File(value),
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn entries(
    files: &[FileDiff],
    limit: usize,
    offenders: impl Fn(&FileDiff) -> Vec<Offender>,
) -> Vec<CheckFailure> {
    files
        .iter()
        .filter_map(|file| {
            let over: Vec<Offender> = offenders(file)
                .into_iter()
                .filter(|offender| offender.value > limit)
                .collect();
            if over.is_empty() {
                None
            } else {
                Some(CheckFailure {
                    path: file.path.clone(),
                    subject: Subject::Entries(over),
                })
            }
        })
        .collect()
}
