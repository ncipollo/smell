//! Static code analysis. The `smell` binary is a thin CLI over this library.

use std::io;
use std::path::{Path, PathBuf};

pub mod cli;
pub mod code;
pub mod feature;
#[cfg(test)]
mod testing;

pub use feature::complexity::check::{
    CheckFailure, CheckResult, Measure, Offender, Subject, check,
};
pub use feature::complexity::options::AnalysisOptions;
pub use feature::complexity::resolve::Overrides;
pub use feature::complexity::tree::{DirectoryNode, FileNode, TreeAnalysis, TreeNode};
pub use feature::complexity::{Analysis, FileReport, PathError};
pub use feature::diff::{DiffError, DiffPathError, DiffReport, DiffStatus, FileDiff, FunctionDiff};

use feature::complexity;

/// Analyzes the source files at the given paths (files or directories,
/// searched recursively) and reports cyclomatic complexity per function.
pub fn analyze(paths: &[PathBuf], options: &AnalysisOptions) -> Analysis {
    complexity::analyze(paths, options)
}

/// Analyzes the source files at the given paths like [`analyze`], but
/// preserving the directory structure each root was traversed through and
/// which root each file came from.
pub fn analyze_tree(paths: &[PathBuf], options: &AnalysisOptions) -> TreeAnalysis {
    complexity::tree::analyze_tree(paths, options)
}

/// Resolves CLI flags against an optional `smell.toml` in `config_dir` into
/// the options an analysis runs with.
pub fn resolve_options(config_dir: &Path, overrides: &Overrides) -> io::Result<AnalysisOptions> {
    complexity::resolve::resolve(config_dir, overrides)
}

/// Diffs cyclomatic complexity for `paths` between `old_ref` and `new_ref` in
/// the git repository at or above `repo_path`, resolving file contents at
/// each commit directly through git rather than requiring a checked-out
/// working directory at either revision.
pub fn diff(
    repo_path: &Path,
    old_ref: &str,
    new_ref: &str,
    paths: &[PathBuf],
) -> Result<DiffReport, DiffError> {
    feature::diff::diff(repo_path, old_ref, new_ref, paths)
}
