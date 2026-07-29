//! Static code analysis. The `smell` binary is a thin CLI over this library.

use std::io;
use std::path::Path;

pub mod cli;
pub mod code;
pub mod feature;
#[cfg(test)]
mod testing;

pub use feature::complexity::FileReport;
pub use feature::complexity::options::AnalysisOptions;
pub use feature::complexity::resolve::Overrides;

use feature::complexity;

/// Analyzes the source files at the given path (a single file or a directory
/// searched recursively) and reports cyclomatic complexity per function.
pub fn analyze(path: &Path, options: &AnalysisOptions) -> io::Result<Vec<FileReport>> {
    complexity::analyze(path, options)
}

/// Resolves CLI flags against an optional `smell.toml` in `config_dir` into
/// the options an analysis runs with.
pub fn resolve_options(config_dir: &Path, overrides: &Overrides) -> io::Result<AnalysisOptions> {
    complexity::resolve::resolve(config_dir, overrides)
}
