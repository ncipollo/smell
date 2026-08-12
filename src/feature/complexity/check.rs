//! Limit checks: which analyzed subjects exceed a configured limit. One
//! `Measure` per --max-* flag; a measure with no configured limit does not
//! run.

use std::path::PathBuf;

use crate::feature::complexity::FileReport;
use crate::feature::complexity::options::AnalysisOptions;

mod file_lines;
mod function_complexity;
mod method_count;
mod scope;

/// The measures a run can enforce. Rendering (labels, JSON keys) lives in
/// `cli`; this is a bare discriminant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Measure {
    Complexity,
    Methods,
    Lines,
}

/// A named subject inside a file that exceeded the limit: a function for
/// complexity, a type for method count.
pub struct Offender {
    pub name: String,
    pub value: usize,
}

/// What exceeded the limit in a failing file: named entries inside it
/// (functions, types), or the file itself (line count).
pub enum Subject {
    Entries(Vec<Offender>),
    File(usize),
}

impl Subject {
    /// The named offenders for an `Entries` subject; empty for a `File`
    /// subject, which has no named entries to list.
    pub fn entries(&self) -> &[Offender] {
        match self {
            Subject::Entries(offenders) => offenders,
            Subject::File(_) => &[],
        }
    }
}

/// A file that failed a measure.
pub struct CheckFailure {
    pub path: PathBuf,
    pub subject: Subject,
}

/// The outcome of one *enabled* measure. Present with an empty `failures`
/// when the measure ran and passed; absent entirely when no limit was
/// configured for it (see `check`).
pub struct CheckResult {
    pub measure: Measure,
    pub limit: usize,
    pub failures: Vec<CheckFailure>,
}

impl CheckResult {
    pub fn failed(&self) -> bool {
        !self.failures.is_empty()
    }
}

/// Runs every measure that has a configured limit, in flag declaration order.
pub fn check(reports: &[FileReport], options: &AnalysisOptions) -> Vec<CheckResult> {
    let configured = [
        (Measure::Complexity, options.max_complexity),
        (Measure::Methods, options.max_methods),
        (Measure::Lines, options.max_lines),
    ];
    configured
        .into_iter()
        .filter_map(|(measure, limit)| limit.map(|limit| result(reports, measure, limit)))
        .collect()
}

fn result(reports: &[FileReport], measure: Measure, limit: usize) -> CheckResult {
    let failures = match measure {
        Measure::Complexity => function_complexity::failures(reports, limit),
        Measure::Methods => method_count::failures(reports, limit),
        Measure::Lines => file_lines::failures(reports, limit),
    };
    CheckResult {
        measure,
        limit,
        failures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{FileComplexity, FunctionComplexity};

    fn report_with_complexity(path: &str, complexity: usize) -> FileReport {
        FileReport {
            path: PathBuf::from(path),
            lines: 1,
            complexity: FileComplexity {
                functions: vec![FunctionComplexity {
                    name: "top".to_string(),
                    complexity,
                }],
                types: vec![],
            },
        }
    }

    #[test]
    fn no_configured_limits_runs_no_measures() {
        let reports = [report_with_complexity("a.rs", 1)];
        assert!(check(&reports, &AnalysisOptions::default()).is_empty());
    }

    #[test]
    fn one_configured_limit_runs_one_measure() {
        let reports = [report_with_complexity("a.rs", 1)];
        let options = AnalysisOptions {
            max_complexity: Some(5),
            ..AnalysisOptions::default()
        };
        let results = check(&reports, &options);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].measure, Measure::Complexity);
        assert_eq!(results[0].limit, 5);
    }

    #[test]
    fn all_configured_limits_run_in_declaration_order() {
        let reports = [report_with_complexity("a.rs", 1)];
        let options = AnalysisOptions {
            max_complexity: Some(5),
            max_methods: Some(2),
            max_lines: Some(100),
            ..AnalysisOptions::default()
        };
        let results = check(&reports, &options);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].measure, Measure::Complexity);
        assert_eq!(results[1].measure, Measure::Methods);
        assert_eq!(results[2].measure, Measure::Lines);
    }

    #[test]
    fn an_enabled_and_passing_measure_has_no_failures() {
        let reports = [report_with_complexity("a.rs", 1)];
        let options = AnalysisOptions {
            max_complexity: Some(5),
            ..AnalysisOptions::default()
        };
        let results = check(&reports, &options);
        assert!(!results[0].failed());
    }

    #[test]
    fn an_enabled_and_failing_measure_reports_failures() {
        let reports = [report_with_complexity("a.rs", 9)];
        let options = AnalysisOptions {
            max_complexity: Some(5),
            ..AnalysisOptions::default()
        };
        let results = check(&reports, &options);
        assert!(results[0].failed());
    }
}
