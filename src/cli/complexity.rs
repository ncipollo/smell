use std::path::PathBuf;
use std::process::ExitCode;

use comfy_table::Table;

use crate::feature::complexity;
use crate::feature::complexity::FileComplexity;

pub fn run(path: PathBuf) -> ExitCode {
    match complexity::analyze(&path) {
        Ok(files) => {
            for file in &files {
                print_file(file);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}

fn print_file(file: &FileComplexity) {
    println!("{}", file.path.display());
    let mut table = Table::new();
    table.set_header(["Function", "Complexity (branches)"]);
    for function in &file.functions {
        table.add_row([function.name.clone(), function.branches.to_string()]);
    }
    println!("{table}\n");
}
