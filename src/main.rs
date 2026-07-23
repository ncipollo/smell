use std::process::ExitCode;

mod cli;
mod code;
mod feature;

use cli::router;

fn main() -> ExitCode {
    router::run()
}
