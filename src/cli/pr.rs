//! View layer for the `pr` subcommand: resolves a PR reference to a commit
//! pair, diffs complexity between them, and renders the report.

use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

mod json;
mod table;

use crate::CheckResult;
use crate::cli::check_format::format_results;
use crate::diff;
use crate::feature::diff::DiffReport;
use crate::feature::diff::delta::{self, DeltaOptions};
use crate::feature::pr;

use table::format_files;

pub struct PrRunArgs {
    pub repo: PathBuf,
    pub reference: String,
    pub paths: Vec<PathBuf>,
    pub delta: DeltaOptions,
    pub json: bool,
}

pub fn run(args: PrRunArgs) -> ExitCode {
    let (old_ref, new_ref) = match pr::resolve(&args.repo, &args.reference) {
        Ok(refs) => refs,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = match diff(&args.repo, &old_ref, &new_ref, &args.paths) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if !args.json {
        for error in &report.errors {
            eprintln!("error: {}: {}", error.path.display(), error.message);
        }
    }
    let results = delta::check(&report.files, &args.delta);
    emit(&report, &results, args.json);
    exit_code(&report, &results)
}

fn exit_code(report: &DiffReport, results: &[CheckResult]) -> ExitCode {
    if report.errors.is_empty() && !results.iter().any(CheckResult::failed) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn emit(report: &DiffReport, results: &[CheckResult], json: bool) {
    if json {
        println!("{}", self::json::render(report, results));
    } else {
        print!("{}", format_files(&report.files));
        let text = format_results(results, io::stderr().is_terminal());
        if !text.is_empty() {
            eprint!("{text}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_forwards_an_explicit_range_to_the_diff_engine() {
        let args = PrRunArgs {
            repo: PathBuf::from("."),
            reference: "HEAD~1..HEAD".to_string(),
            paths: vec![PathBuf::from("does-not-exist.rs")],
            delta: DeltaOptions::default(),
            json: false,
        };
        // A missing path is a soft per-path error in the diff engine, not a
        // crash here.
        let _ = run(args);
    }

    #[test]
    fn run_reports_an_error_for_an_unparseable_reference() {
        let args = PrRunArgs {
            repo: PathBuf::from("."),
            reference: "not a reference".to_string(),
            paths: vec![PathBuf::from("src")],
            delta: DeltaOptions::default(),
            json: false,
        };
        assert_eq!(run(args), ExitCode::FAILURE);
    }

    fn empty_report() -> DiffReport {
        DiffReport {
            files: vec![],
            errors: vec![],
        }
    }

    #[test]
    fn exit_code_is_success_with_no_errors_or_failures() {
        assert_eq!(exit_code(&empty_report(), &[]), ExitCode::SUCCESS);
    }

    #[test]
    fn exit_code_is_failure_when_a_delta_check_fails() {
        use crate::feature::complexity::check::{CheckFailure, Measure, Subject};

        let results = [CheckResult {
            measure: Measure::Lines,
            limit: 10,
            failures: vec![CheckFailure {
                path: PathBuf::from("src/a.rs"),
                subject: Subject::File(15),
            }],
        }];
        assert_eq!(exit_code(&empty_report(), &results), ExitCode::FAILURE);
    }
}
