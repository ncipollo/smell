//! Static code analysis. The `smell` binary is a thin CLI over this library.

use std::io;
use std::path::Path;

pub mod cli;
pub mod code;
pub mod feature;
#[cfg(test)]
mod testing;

pub use feature::complexity::FileReport;

use feature::complexity;

/// Analyzes the source files at the given path (a single file or a directory
/// searched recursively) and reports branch complexity per function.
pub fn analyze(path: &Path) -> io::Result<Vec<FileReport>> {
    complexity::analyze(path)
}
