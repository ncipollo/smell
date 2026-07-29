use std::env;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::{Attribute, Cell, Table};

use crate::code::{ComplexityRollup, FunctionComplexity};
use crate::{AnalysisOptions, FileReport, Overrides, analyze, resolve_options};

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
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
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
