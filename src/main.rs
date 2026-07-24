use std::process::ExitCode;

mod cli;
mod code;
mod feature;
#[cfg(test)]
mod testing;

use cli::router;

fn main() -> ExitCode {
    router::run()
}
