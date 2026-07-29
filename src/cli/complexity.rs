use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::{Attribute, Cell, Table};

use crate::code::branch::{BranchFilter, BranchSpec};
use crate::code::{ComplexityRollup, FunctionComplexity};
use crate::feature::complexity::filter::FileFilter;
use crate::{AnalysisOptions, FileReport, analyze};

pub fn run(
    path: PathBuf,
    include: Vec<String>,
    exclude: Vec<String>,
    branches: Vec<String>,
) -> ExitCode {
    let options = match build_options(&include, &exclude, &branches) {
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

fn build_options(
    include: &[String],
    exclude: &[String],
    branches: &[String],
) -> io::Result<AnalysisOptions> {
    let specs: Vec<BranchSpec> = branches
        .iter()
        .map(|branch| BranchSpec::parse(branch))
        .collect();
    Ok(AnalysisOptions {
        files: FileFilter::new(include, exclude)?,
        branches: BranchFilter::from_specs(&specs),
    })
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
