use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::code::FileComplexity;

pub mod router;

pub struct FileReport {
    pub path: PathBuf,
    pub complexity: FileComplexity,
}

/// Analyzes the source files at the given path (a single file or a directory
/// searched recursively) and reports branch complexity per function.
pub fn analyze(path: &Path) -> io::Result<Vec<FileReport>> {
    let mut files = source_files(path)?;
    files.sort();
    files.into_iter().map(analyze_file).collect()
}

fn analyze_file(path: PathBuf) -> io::Result<FileReport> {
    let source = fs::read_to_string(&path)?;
    let Some(complexity) = router::file_complexity(&path, &source) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported file type: {}", path.display()),
        ));
    };
    Ok(FileReport { path, complexity })
}

fn source_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry_path = entry?.path();
        if entry_path.is_dir() {
            files.extend(source_files(&entry_path)?);
        } else if router::is_supported(&entry_path) {
            files.push(entry_path);
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures")
    }

    #[test]
    fn analyze_reports_all_fixture_files_sorted() {
        let reports = analyze(&fixtures_dir()).expect("analyze fixtures");
        let names: Vec<String> = reports
            .iter()
            .map(|report| {
                report
                    .path
                    .strip_prefix(fixtures_dir())
                    .expect("fixture path")
                    .display()
                    .to_string()
            })
            .collect();
        assert_eq!(
            names,
            vec![
                "java/Complexity.java",
                "kotlin/complexity.kt",
                "rust/complexity.rs",
                "swift/complexity.swift",
            ]
        );
    }

    #[test]
    fn analyze_reports_single_file() {
        let path = fixtures_dir().join("swift/complexity.swift");
        let reports = analyze(&path).expect("analyze single file");
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].complexity.functions.is_empty());
    }

    #[test]
    fn analyze_rejects_unsupported_file() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
        let error = match analyze(&path) {
            Ok(_) => panic!("unsupported file should error"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
