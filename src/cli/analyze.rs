use std::process::ExitCode;

use crate::feature::analyze;

pub fn run() -> ExitCode {
    analyze::run();
    ExitCode::SUCCESS
}
