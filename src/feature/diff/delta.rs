//! Delta checks: which files/types swung by more than a configured
//! threshold between the two diffed revisions. Mirrors
//! `feature::complexity::check` one-for-one, reusing its `Measure`,
//! `CheckResult`, `CheckFailure`, `Subject`, and `Offender` types — a measure
//! with no configured threshold does not run.

use crate::feature::complexity::check::{CheckResult, Measure};
use crate::feature::diff::FileDiff;

mod comment_lines;
mod comments;
mod declarations;
mod lines;
mod methods;
mod scope;

/// One `Option<usize>` threshold per `--delta-*` flag.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeltaOptions {
    pub max_methods: Option<usize>,
    pub max_lines: Option<usize>,
    pub max_declarations: Option<usize>,
    pub max_comment_lines: Option<usize>,
    pub max_comments: Option<usize>,
}

/// Runs every measure that has a configured threshold, in flag declaration
/// order.
pub fn check(files: &[FileDiff], options: &DeltaOptions) -> Vec<CheckResult> {
    let configured = [
        (Measure::Methods, options.max_methods),
        (Measure::Lines, options.max_lines),
        (Measure::Declarations, options.max_declarations),
        (Measure::CommentLines, options.max_comment_lines),
        (Measure::Comments, options.max_comments),
    ];
    configured
        .into_iter()
        .filter_map(|(measure, limit)| limit.map(|limit| result(files, measure, limit)))
        .collect()
}

fn result(files: &[FileDiff], measure: Measure, limit: usize) -> CheckResult {
    let failures = match measure {
        Measure::Methods => methods::failures(files, limit),
        Measure::Lines => lines::failures(files, limit),
        Measure::Declarations => declarations::failures(files, limit),
        Measure::CommentLines => comment_lines::failures(files, limit),
        Measure::Comments => comments::failures(files, limit),
        Measure::Complexity => {
            unreachable!("complexity diffing is unconditional, not a delta check")
        }
    };
    CheckResult {
        measure,
        limit,
        failures,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn file(path: &str) -> FileDiff {
        FileDiff {
            path: PathBuf::from(path),
            functions: Vec::new(),
            old: None,
            new: None,
        }
    }

    #[test]
    fn no_configured_thresholds_runs_no_measures() {
        let files = [file("a.rs")];
        assert!(check(&files, &DeltaOptions::default()).is_empty());
    }

    #[test]
    fn one_configured_threshold_runs_one_measure() {
        let files = [file("a.rs")];
        let options = DeltaOptions {
            max_lines: Some(5),
            ..DeltaOptions::default()
        };
        let results = check(&files, &options);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].measure, Measure::Lines);
        assert_eq!(results[0].limit, 5);
    }

    #[test]
    fn all_configured_thresholds_run_in_declaration_order() {
        let files = [file("a.rs")];
        let options = DeltaOptions {
            max_methods: Some(1),
            max_lines: Some(2),
            max_declarations: Some(3),
            max_comment_lines: Some(4),
            max_comments: Some(5),
        };
        let results = check(&files, &options);
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].measure, Measure::Methods);
        assert_eq!(results[1].measure, Measure::Lines);
        assert_eq!(results[2].measure, Measure::Declarations);
        assert_eq!(results[3].measure, Measure::CommentLines);
        assert_eq!(results[4].measure, Measure::Comments);
    }
}
