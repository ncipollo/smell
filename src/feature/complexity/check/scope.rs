//! Shared combinators for the two shapes a measure's subject takes: named
//! entries inside a file (functions for complexity, types for method count),
//! or the file itself (line count, declaration count). Both fail a file when
//! its value is strictly over the limit (equal to it passes).

use crate::code::FileComplexity;
use crate::feature::complexity::FileReport;
use crate::feature::complexity::check::{CheckFailure, Offender, Subject};

pub fn entries(
    reports: &[FileReport],
    limit: usize,
    offenders: impl Fn(&FileComplexity) -> Vec<Offender>,
) -> Vec<CheckFailure> {
    reports
        .iter()
        .filter_map(|report| {
            let over: Vec<Offender> = offenders(&report.complexity)
                .into_iter()
                .filter(|offender| offender.value > limit)
                .collect();
            if over.is_empty() {
                None
            } else {
                Some(CheckFailure {
                    path: report.path.clone(),
                    subject: Subject::Entries(over),
                })
            }
        })
        .collect()
}

pub fn file(
    reports: &[FileReport],
    limit: usize,
    value: impl Fn(&FileReport) -> usize,
) -> Vec<CheckFailure> {
    reports
        .iter()
        .filter_map(|report| {
            let count = value(report);
            if count > limit {
                Some(CheckFailure {
                    path: report.path.clone(),
                    subject: Subject::File(count),
                })
            } else {
                None
            }
        })
        .collect()
}
