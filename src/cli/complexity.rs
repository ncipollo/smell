use std::env;
use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::{Attribute, Cell, Table};

mod json;

use crate::code::{ComplexityRollup, FunctionComplexity};
use crate::{
    AnalysisOptions, CheckFailure, CheckResult, FileReport, Measure, Overrides, PathError, Subject,
    analyze, check, resolve_options,
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

const RED_BOLD: &str = "\x1b[1;31m";
const RESET: &str = "\x1b[0m";

/// One section per failing measure, blank-line separated. A run with a
/// single failing measure renders byte-for-byte what a lone check did before
/// multiple measures existed.
fn format_results(results: &[CheckResult], color: bool) -> String {
    results
        .iter()
        .filter(|result| result.failed())
        .map(|result| format_result(result, color))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_result(result: &CheckResult, color: bool) -> String {
    let header = format!(
        "✗ {} check failed: {} file(s) exceed limit {}",
        label(result.measure),
        result.failures.len(),
        result.limit
    );
    let mut text = paint(&header, color);
    text.push('\n');
    for failure in &result.failures {
        text.push_str(&format_failure(failure, color));
    }
    text
}

/// A file-level failure (line count) is one flat `path  value` line; an
/// entry-scoped failure (complexity, method count) lists its named offenders
/// indented beneath the path.
fn format_failure(failure: &CheckFailure, color: bool) -> String {
    match &failure.subject {
        Subject::File(value) => format!(
            "{}  {}\n",
            failure.path.display(),
            paint(&value.to_string(), color)
        ),
        Subject::Entries(offenders) => {
            let mut text = format!("{}\n", failure.path.display());
            for offender in offenders {
                text.push_str(&format!(
                    "  {}  {}\n",
                    offender.name,
                    paint(&offender.value.to_string(), color)
                ));
            }
            text
        }
    }
}

/// User-facing wording for a measure's stderr header. Kept in `cli` since
/// `feature::complexity::check::Measure` is a bare discriminant.
fn label(measure: Measure) -> &'static str {
    match measure {
        Measure::Complexity => "complexity",
        Measure::Methods => "method count",
        Measure::Lines => "line count",
    }
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

/// The extra rollup figure shown alongside a group's row: a type's method
/// count or the file's line count. Mutually exclusive, since a group is
/// never both.
#[derive(Clone, Copy)]
enum Extra {
    None,
    Methods(usize),
    Lines(usize),
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
        Extra::Lines(report.lines),
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
        Extra::Lines(count) => format!("{base} · lines {count}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{FileComplexity, TypeComplexity};
    use crate::feature::complexity::check::{CheckFailure, Offender};

    fn offender(name: &str, value: usize) -> Offender {
        Offender {
            name: name.to_string(),
            value,
        }
    }

    fn entries_failure(path: &str, offenders: Vec<Offender>) -> CheckFailure {
        CheckFailure {
            path: PathBuf::from(path),
            subject: Subject::Entries(offenders),
        }
    }

    fn complexity_failures() -> Vec<CheckFailure> {
        vec![
            entries_failure("src/a.rs", vec![offender("Shape.area", 12)]),
            entries_failure("src/b.rs", vec![offender("top", 11)]),
        ]
    }

    fn complexity_result(failures: Vec<CheckFailure>) -> CheckResult {
        CheckResult {
            measure: Measure::Complexity,
            limit: 10,
            failures,
        }
    }

    fn methods_result(failures: Vec<CheckFailure>) -> CheckResult {
        CheckResult {
            measure: Measure::Methods,
            limit: 5,
            failures,
        }
    }

    fn lines_result(failures: Vec<CheckFailure>) -> CheckResult {
        CheckResult {
            measure: Measure::Lines,
            limit: 100,
            failures,
        }
    }

    #[test]
    fn format_result_leads_with_a_summary_then_lists_files_and_offenders() {
        let result = complexity_result(complexity_failures());
        assert_eq!(
            format_result(&result, false),
            "✗ complexity check failed: 2 file(s) exceed limit 10\n\
             src/a.rs\n  Shape.area  12\nsrc/b.rs\n  top  11\n"
        );
    }

    #[test]
    fn format_result_paints_the_header_and_values_red_when_colored() {
        let result = complexity_result(complexity_failures());
        let text = format_result(&result, true);
        assert!(text.starts_with(&format!(
            "{RED_BOLD}✗ complexity check failed: 2 file(s) exceed limit 10{RESET}\n"
        )));
        assert!(text.contains(&format!("  Shape.area  {RED_BOLD}12{RESET}\n")));
    }

    #[test]
    fn format_result_without_color_has_no_escape_codes() {
        let result = complexity_result(complexity_failures());
        assert!(!format_result(&result, false).contains('\x1b'));
    }

    #[test]
    fn format_results_uses_the_method_count_label() {
        let result = methods_result(vec![entries_failure(
            "src/a.rs",
            vec![offender("Shape", 8)],
        )]);
        let text = format_results(&[result], false);
        assert!(text.starts_with("✗ method count check failed: 1 file(s) exceed limit 5\n"));
    }

    #[test]
    fn format_results_uses_the_line_count_label() {
        let result = lines_result(vec![CheckFailure {
            path: PathBuf::from("src/a.rs"),
            subject: Subject::File(150),
        }]);
        let text = format_results(&[result], false);
        assert!(text.starts_with("✗ line count check failed: 1 file(s) exceed limit 100\n"));
    }

    #[test]
    fn format_result_renders_a_file_subject_as_a_flat_line() {
        let result = lines_result(vec![CheckFailure {
            path: PathBuf::from("src/a.rs"),
            subject: Subject::File(150),
        }]);
        assert_eq!(
            format_result(&result, false),
            "✗ line count check failed: 1 file(s) exceed limit 100\nsrc/a.rs  150\n"
        );
    }

    #[test]
    fn format_results_joins_multiple_failing_measures_with_a_blank_line() {
        let results = [
            complexity_result(complexity_failures()),
            methods_result(vec![entries_failure(
                "src/a.rs",
                vec![offender("Shape", 8)],
            )]),
        ];
        let text = format_results(&results, false);
        assert!(text.contains("✗ complexity check failed"));
        assert!(text.contains("✗ method count check failed"));
        assert!(text.contains("check failed: 1 file(s) exceed limit 5\nsrc/a.rs\n"));
    }

    #[test]
    fn format_results_omits_passing_measures() {
        let results = [complexity_result(vec![])];
        assert_eq!(format_results(&results, false), "");
    }

    #[test]
    fn format_results_is_empty_when_no_measures_are_configured() {
        assert_eq!(format_results(&[], false), "");
    }

    fn sample_reports() -> Vec<FileReport> {
        vec![FileReport {
            path: PathBuf::from("src/a.rs"),
            lines: 42,
            complexity: FileComplexity {
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
    fn format_reports_shows_method_count_on_type_rows_only() {
        let reports = vec![FileReport {
            path: PathBuf::from("src/a.rs"),
            lines: 10,
            complexity: FileComplexity {
                functions: vec![FunctionComplexity {
                    name: "top".to_string(),
                    complexity: 1,
                }],
                types: vec![TypeComplexity {
                    name: "Shape".to_string(),
                    supertypes: Vec::new(),
                    functions: vec![
                        FunctionComplexity {
                            name: "area".to_string(),
                            complexity: 1,
                        },
                        FunctionComplexity {
                            name: "label".to_string(),
                            complexity: 1,
                        },
                    ],
                }],
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
