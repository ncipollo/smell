use std::env;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::{Attribute, Cell, Table};

use crate::code::{ComplexityRollup, FunctionComplexity};
use crate::{
    AnalysisOptions, CheckFailure, FileReport, Overrides, analyze, check, resolve_options,
};

pub fn run(path: PathBuf, overrides: Overrides) -> ExitCode {
    let options = match options(&overrides) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    match analyze(&path, &options) {
        Ok(reports) => {
            for report in &reports {
                print_file(report);
            }
            enforce_limit(&reports, options.max_complexity)
        }
        Err(error) => {
            eprintln!("error: {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}

/// Applies the complexity limit check, reporting any failures on stderr.
fn enforce_limit(reports: &[FileReport], limit: Option<usize>) -> ExitCode {
    let Some(limit) = limit else {
        return ExitCode::SUCCESS;
    };
    let failures = check(reports, limit);
    if failures.is_empty() {
        return ExitCode::SUCCESS;
    }
    eprint!("{}", format_failures(&failures, limit));
    ExitCode::FAILURE
}

fn format_failures(failures: &[CheckFailure], limit: usize) -> String {
    let mut text = String::new();
    for failure in failures {
        text.push_str(&format!("{}\n", failure.path.display()));
        for function in &failure.functions {
            text.push_str(&format!("  {}  {}\n", function.name, function.complexity));
        }
    }
    text.push_str(&format!(
        "{} file(s) exceed complexity limit {limit}\n",
        failures.len()
    ));
    text
}

fn options(overrides: &Overrides) -> io::Result<AnalysisOptions> {
    let dir = env::current_dir()?;
    resolve_options(&dir, overrides)
}

fn print_file(report: &FileReport) {
    println!("{}", report.path.display());
    let mut table = Table::new();
    table.set_header(["Function", "Complexity"]);
    for complexity_type in &report.complexity.types {
        add_group_rows(
            &mut table,
            &complexity_type.name,
            &complexity_type.functions,
            &complexity_type.rollup(),
        );
    }
    if !report.complexity.functions.is_empty() {
        add_group_rows(
            &mut table,
            "(top-level)",
            &report.complexity.functions,
            &ComplexityRollup::of(&report.complexity.functions),
        );
    }
    add_rollup_row(&mut table, "file", &report.complexity.rollup());
    println!("{table}\n");
}

fn add_group_rows(
    table: &mut Table,
    name: &str,
    functions: &[FunctionComplexity],
    rollup: &ComplexityRollup,
) {
    add_rollup_row(table, name, rollup);
    for function in functions {
        table.add_row([
            format!("  {}", function.name),
            function.complexity.to_string(),
        ]);
    }
}

fn add_rollup_row(table: &mut Table, name: &str, rollup: &ComplexityRollup) {
    table.add_row([
        Cell::new(name).add_attribute(Attribute::Bold),
        Cell::new(format_rollup(rollup)).add_attribute(Attribute::Bold),
    ]);
}

fn format_rollup(rollup: &ComplexityRollup) -> String {
    format!(
        "total {} · max {} · avg {:.1}",
        rollup.total, rollup.max, rollup.average
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::complexity::check::FailedFunction;

    #[test]
    fn format_failures_lists_files_functions_and_summary() {
        let failures = vec![
            CheckFailure {
                path: PathBuf::from("src/a.rs"),
                functions: vec![FailedFunction {
                    name: "Shape.area".to_string(),
                    complexity: 12,
                }],
            },
            CheckFailure {
                path: PathBuf::from("src/b.rs"),
                functions: vec![FailedFunction {
                    name: "top".to_string(),
                    complexity: 11,
                }],
            },
        ];
        assert_eq!(
            format_failures(&failures, 10),
            "src/a.rs\n  Shape.area  12\nsrc/b.rs\n  top  11\n2 file(s) exceed complexity limit 10\n"
        );
    }
}
