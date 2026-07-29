use std::process::ExitCode;

use crate::feature::info;

pub fn run() -> ExitCode {
    println!("{}", info::text());
    ExitCode::SUCCESS
}
