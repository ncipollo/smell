use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::code::FunctionComplexity;
use crate::code::swift;

pub struct FileComplexity {
    pub path: PathBuf,
    pub functions: Vec<FunctionComplexity>,
}

/// Analyzes the Swift files at the given path (a single file or a directory
/// searched recursively) and reports branch complexity per function.
pub fn analyze(path: &Path) -> io::Result<Vec<FileComplexity>> {
    let mut files = swift_files(path)?;
    files.sort();
    files.into_iter().map(analyze_file).collect()
}

fn analyze_file(path: PathBuf) -> io::Result<FileComplexity> {
    let source = fs::read_to_string(&path)?;
    Ok(FileComplexity {
        functions: swift::function_complexities(&source),
        path,
    })
}

fn swift_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry_path = entry?.path();
        if entry_path.is_dir() {
            files.extend(swift_files(&entry_path)?);
        } else if entry_path
            .extension()
            .is_some_and(|extension| extension == "swift")
        {
            files.push(entry_path);
        }
    }
    Ok(files)
}
