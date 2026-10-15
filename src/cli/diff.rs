use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::Table;

mod json;

use crate::CheckResult;
use crate::cli::check_format::format_results;
use crate::diff;
use crate::feature::diff::delta::{self, DeltaOptions};
use crate::feature::diff::{DiffReport, DiffStatus, FileDiff, FunctionDiff};

pub struct DiffRunArgs {
    pub repo: PathBuf,
    pub old_ref: String,
    pub new_ref: String,
    pub paths: Vec<PathBuf>,
    pub delta: DeltaOptions,
    pub json: bool,
}

pub fn run(args: DiffRunArgs) -> ExitCode {
    let report = match diff(&args.repo, &args.old_ref, &args.new_ref, &args.paths) {
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

/// Renders one table per file, omitting unchanged functions — the
/// mechanical-vs-real noise a diff is meant to filter out — with a trailing
/// count of how many were omitted.
fn format_files(files: &[FileDiff]) -> String {
    files.iter().map(format_file).collect()
}

fn format_file(file: &FileDiff) -> String {
    let mut text = format!("{}\n", file.path.display());
    let mut table = Table::new();
    table.set_header(["Function", "Status", "Before", "After", "Δ"]);
    let mut omitted = 0;
    for function in &file.functions {
        match row(function) {
            Some(cells) => {
                table.add_row(cells);
            }
            None => omitted += 1,
        }
    }
    text.push_str(&format!("{table}\n"));
    if omitted > 0 {
        text.push_str(&format!("{omitted} unchanged function(s) omitted\n"));
    }
    text.push('\n');
    text
}

fn name(function: &FunctionDiff) -> String {
    match &function.type_name {
        Some(type_name) => format!("{type_name}.{}", function.name),
        None => function.name.clone(),
    }
}

/// `None` for an unchanged function, which the table omits entirely.
fn row(function: &FunctionDiff) -> Option<[String; 5]> {
    let (status, before, after, delta) = match function.status {
        DiffStatus::Added { complexity } => (
            "added",
            "-".to_string(),
            complexity.to_string(),
            String::new(),
        ),
        DiffStatus::Removed { complexity } => (
            "removed",
            complexity.to_string(),
            "-".to_string(),
            String::new(),
        ),
        DiffStatus::Changed { before, after } => (
            "changed",
            before.to_string(),
            after.to_string(),
            format!("{:+}", after as i64 - before as i64),
        ),
        DiffStatus::Unchanged { .. } => return None,
    };
    Some([name(function), status.to_string(), before, after, delta])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(type_name: Option<&str>, name: &str, status: DiffStatus) -> FunctionDiff {
        FunctionDiff {
            type_name: type_name.map(str::to_string),
            name: name.to_string(),
            status,
        }
    }

    fn file_diff(path: &str, functions: Vec<FunctionDiff>) -> FileDiff {
        FileDiff {
            path: PathBuf::from(path),
            functions,
            old: None,
            new: None,
        }
    }

    #[test]
    fn format_files_includes_the_file_path() {
        let files = [file_diff(
            "src/a.rs",
            vec![function(None, "a", DiffStatus::Added { complexity: 1 })],
        )];
        assert!(format_files(&files).contains("src/a.rs"));
    }

    #[test]
    fn format_files_omits_unchanged_functions_and_counts_them() {
        let files = [file_diff(
            "src/a.rs",
            vec![
                function(None, "a", DiffStatus::Unchanged { complexity: 1 }),
                function(None, "b", DiffStatus::Added { complexity: 2 }),
            ],
        )];
        let text = format_files(&files);
        assert!(!text.contains(" a "));
        assert!(text.contains("b"));
        assert!(text.contains("1 unchanged function(s) omitted"));
    }

    #[test]
    fn format_files_has_no_omitted_line_when_nothing_is_unchanged() {
        let files = [file_diff(
            "src/a.rs",
            vec![function(None, "a", DiffStatus::Added { complexity: 1 })],
        )];
        assert!(!format_files(&files).contains("omitted"));
    }

    #[test]
    fn row_prefixes_the_enclosing_type_name() {
        let diff = function(Some("Shape"), "area", DiffStatus::Added { complexity: 1 });
        let cells = row(&diff).unwrap();
        assert_eq!(cells[0], "Shape.area");
    }

    #[test]
    fn row_shows_a_signed_delta_for_changed_functions() {
        let diff = function(
            None,
            "a",
            DiffStatus::Changed {
                before: 3,
                after: 7,
            },
        );
        let cells = row(&diff).unwrap();
        assert_eq!(
            cells,
            [
                "a".to_string(),
                "changed".to_string(),
                "3".to_string(),
                "7".to_string(),
                "+4".to_string()
            ]
        );
    }

    #[test]
    fn row_is_none_for_unchanged() {
        let diff = function(None, "a", DiffStatus::Unchanged { complexity: 1 });
        assert!(row(&diff).is_none());
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
