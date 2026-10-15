use std::env;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::{Attribute, Cell, Table};

mod json;

use crate::cli::check_format::format_results;
use crate::code::{ComplexityRollup, FunctionComplexity};
use crate::{
    AnalysisOptions, CheckResult, FileReport, Overrides, PathError, analyze, check, resolve_options,
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
    let exit = emit(&analysis.reports, &analysis.errors, &options, quiet, json);
    if analysis.errors.is_empty() {
        exit
    } else {
        ExitCode::FAILURE
    }
}

/// Renders the analysis in the requested format, running every configured
/// measure once and sharing its result across both: embedded in the
/// document for `--json`, or a colored stderr section per failing measure
/// for the table. Path errors are embedded in the JSON document; for the
/// table they've already been printed to stderr by the caller.
fn emit(
    reports: &[FileReport],
    errors: &[PathError],
    options: &AnalysisOptions,
    quiet: bool,
    json: bool,
) -> ExitCode {
    let results = check(reports, options);
    if json {
        println!("{}", self::json::render(reports, &results, errors));
    } else {
        print!("{}", format_reports(reports, quiet));
        let report = format_results(&results, io::stderr().is_terminal());
        if !report.is_empty() {
            eprint!("{report}");
        }
    }
    exit_code(&results)
}

fn exit_code(results: &[CheckResult]) -> ExitCode {
    if results.iter().any(CheckResult::failed) {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
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

/// The extra rollup figure(s) shown alongside a group's row: a type's method
/// count, or the file's line and declaration counts together (a file is
/// never a `Methods` row and a type is never a `File` row).
#[derive(Clone, Copy)]
enum Extra {
    None,
    Methods(usize),
    File {
        lines: usize,
        declarations: usize,
        comments: usize,
    },
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
            Extra::Methods(complexity_type.functions.len()),
        );
    }
    if !report.complexity.functions.is_empty() {
        add_group_rows(
            &mut table,
            "(top-level)",
            &report.complexity.functions,
            &ComplexityRollup::of(&report.complexity.functions),
            Extra::None,
        );
    }
    add_rollup_row(
        &mut table,
        "file",
        &report.complexity.rollup(),
        Extra::File {
            lines: report.lines,
            declarations: report.complexity.declarations(),
            comments: report.complexity.comment_count(),
        },
    );
    text.push_str(&format!("{table}\n\n"));
    text
}

fn add_group_rows(
    table: &mut Table,
    name: &str,
    functions: &[FunctionComplexity],
    rollup: &ComplexityRollup,
    extra: Extra,
) {
    add_rollup_row(table, name, rollup, extra);
    for function in functions {
        table.add_row([
            format!("  {}", function.name),
            function.complexity.to_string(),
        ]);
    }
}

fn add_rollup_row(table: &mut Table, name: &str, rollup: &ComplexityRollup, extra: Extra) {
    table.add_row([
        Cell::new(name).add_attribute(Attribute::Bold),
        Cell::new(format_rollup(rollup, extra)).add_attribute(Attribute::Bold),
    ]);
}

fn format_rollup(rollup: &ComplexityRollup, extra: Extra) -> String {
    let base = format!(
        "total {} · max {} · avg {:.1}",
        rollup.total, rollup.max, rollup.average
    );
    match extra {
        Extra::None => base,
        Extra::Methods(count) => format!("{base} · methods {count}"),
        Extra::File {
            lines,
            declarations,
            comments,
        } => {
            format!("{base} · lines {lines} · decls {declarations} · comments {comments}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{CommentSpan, FileComplexity, TypeComplexity};

    fn sample_reports() -> Vec<FileReport> {
        vec![FileReport {
            path: PathBuf::from("src/a.rs"),
            lines: 42,
            complexity: FileComplexity {
                functions: vec![FunctionComplexity {
                    name: "top".to_string(),
                    complexity: 3,
                    calls: 0,
                    mutations: 0,
                }],
                comments: vec![CommentSpan {
                    start_line: 1,
                    end_line: 1,
                }],
                ..Default::default()
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

    #[test]
    fn format_reports_shows_line_count_on_the_file_row() {
        let text = format_reports(&sample_reports(), false);
        let file_line = text
            .lines()
            .find(|line| line.contains("file"))
            .expect("file rollup row");
        assert!(file_line.contains("lines 42"));
    }

    #[test]
    fn format_reports_shows_declaration_count_on_the_file_row() {
        let text = format_reports(&sample_reports(), false);
        let file_line = text
            .lines()
            .find(|line| line.contains("file"))
            .expect("file rollup row");
        assert!(file_line.contains("decls 1"));
    }

    #[test]
    fn format_reports_shows_comment_count_on_the_file_row() {
        let text = format_reports(&sample_reports(), false);
        let file_line = text
            .lines()
            .find(|line| line.contains("file"))
            .expect("file rollup row");
        assert!(file_line.contains("comments 1"));
    }

    #[test]
    fn format_reports_shows_method_count_on_type_rows_only() {
        let reports = vec![FileReport {
            path: PathBuf::from("src/a.rs"),
            lines: 10,
            complexity: FileComplexity {
                functions: vec![FunctionComplexity {
                    name: "top".to_string(),
                    complexity: 1,
                    calls: 0,
                    mutations: 0,
                }],
                types: vec![TypeComplexity {
                    name: "Shape".to_string(),
                    supertypes: Vec::new(),
                    functions: vec![
                        FunctionComplexity {
                            name: "area".to_string(),
                            complexity: 1,
                            calls: 0,
                            mutations: 0,
                        },
                        FunctionComplexity {
                            name: "label".to_string(),
                            complexity: 1,
                            calls: 0,
                            mutations: 0,
                        },
                    ],
                }],
                ..Default::default()
            },
        }];
        let text = format_reports(&reports, false);
        assert!(text.contains("methods 2"));
        let top_level_line = text
            .lines()
            .find(|line| line.contains("(top-level)"))
            .expect("top-level row");
        assert!(!top_level_line.contains("methods"));
    }
}
