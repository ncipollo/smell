use std::env;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::{Attribute, Cell, Table};

mod json;

use crate::code::{ComplexityRollup, FunctionComplexity};
use crate::{
    AnalysisOptions, CheckFailure, FileReport, Overrides, PathError, analyze, check,
    resolve_options,
};

pub fn run(paths: Vec<PathBuf>, overrides: Overrides, quiet: bool, json: bool) -> ExitCode {
    let options = match options(&overrides) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };
    let paths = match crate::cli::paths::resolve(paths) {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("error: reading paths from stdin: {error}");
            return ExitCode::FAILURE;
        }
    };
    let analysis = analyze(&paths, &options);
    if !json {
        for error in &analysis.errors {
            eprintln!("error: {}: {}", error.path.display(), error.error);
        }
    }
    let exit = emit(
        &analysis.reports,
        &analysis.errors,
        options.max_complexity,
        quiet,
        json,
    );
    if analysis.errors.is_empty() {
        exit
    } else {
        ExitCode::FAILURE
    }
}

/// Renders the analysis in the requested format, running the
/// --max-complexity check once and sharing its result across both: embedded
/// in the document for `--json`, or a colored stderr report for the table.
/// Path errors are embedded in the JSON document; for the table they've
/// already been printed to stderr by the caller.
fn emit(
    reports: &[FileReport],
    errors: &[PathError],
    limit: Option<usize>,
    quiet: bool,
    json: bool,
) -> ExitCode {
    let failures = limit.map(|limit| check(reports, limit)).unwrap_or_default();
    if json {
        println!("{}", self::json::render(reports, limit, &failures, errors));
    } else {
        print!("{}", format_reports(reports, quiet));
        if let Some(limit) = limit
            && !failures.is_empty()
        {
            let color = io::stderr().is_terminal();
            eprint!("{}", format_failures(&failures, limit, color));
        }
    }
    if failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

const RED_BOLD: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

fn format_failures(failures: &[CheckFailure], limit: usize, color: bool) -> String {
    let header = format!(
        "✗ complexity check failed: {} file(s) exceed limit {limit}",
        failures.len()
    );
    let mut text = paint(&header, color);
    text.push('\n');
    for failure in failures {
        text.push_str(&format!("{}\n", failure.path.display()));
        for function in &failure.functions {
            text.push_str(&format!(
                "  {}  {}\n",
                function.name,
                paint(&function.complexity.to_string(), color)
            ));
        }
    }
    text
}

/// Wraps text in bold red when stderr is a terminal; plain otherwise so
/// piped and CI output stays free of escape codes.
fn paint(text: &str, color: bool) -> String {
    if color {
        format!("{RED_BOLD}{text}{RESET}")
    } else {
        text.to_string()
    }
}

fn options(overrides: &Overrides) -> io::Result<AnalysisOptions> {
    let dir = env::current_dir()?;
    resolve_options(&dir, overrides)
}

/// Renders the per-file complexity tables, or nothing when `quiet` — the
/// point of a quiet run is a silent stdout on success.
fn format_reports(reports: &[FileReport], quiet: bool) -> String {
    if quiet {
        return String::new();
    }
    reports.iter().map(format_file).collect()
}

fn format_file(report: &FileReport) -> String {
    let mut text = format!("{}\n", report.path.display());
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
    text.push_str(&format!("{table}\n\n"));
    text
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

    fn failures() -> Vec<CheckFailure> {
        vec![
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
        ]
    }

    #[test]
    fn format_failures_leads_with_a_summary_then_lists_files_and_functions() {
        assert_eq!(
            format_failures(&failures(), 10, false),
            "✗ complexity check failed: 2 file(s) exceed limit 10\n\
             src/a.rs\n  Shape.area  12\nsrc/b.rs\n  top  11\n"
        );
    }

    #[test]
    fn format_failures_paints_the_header_and_complexities_red_when_colored() {
        let text = format_failures(&failures(), 10, true);
        assert!(text.starts_with(&format!(
            "{RED_BOLD}✗ complexity check failed: 2 file(s) exceed limit 10{RESET}\n"
        )));
        assert!(text.contains(&format!("  Shape.area  {RED_BOLD}12{RESET}\n")));
    }

    #[test]
    fn format_failures_without_color_has_no_escape_codes() {
        assert!(!format_failures(&failures(), 10, false).contains('\x1b'));
    }

    fn sample_reports() -> Vec<FileReport> {
        vec![FileReport {
            path: PathBuf::from("src/a.rs"),
            complexity: crate::code::FileComplexity {
                functions: vec![FunctionComplexity {
                    name: "top".to_string(),
                    complexity: 3,
                }],
                types: vec![],
            },
        }]
    }

    #[test]
    fn format_reports_is_empty_when_quiet() {
        assert_eq!(format_reports(&sample_reports(), true), "");
    }

    #[test]
    fn format_reports_includes_each_file_when_not_quiet() {
        let text = format_reports(&sample_reports(), false);
        assert!(text.contains("src/a.rs"));
        assert!(text.contains("top"));
    }
}
