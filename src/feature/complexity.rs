use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::code::FileComplexity;
use crate::feature::complexity::filter::{FileFilter, TypeFilter};
use crate::feature::complexity::options::AnalysisOptions;

pub mod config;
pub mod filter;
pub mod options;
pub mod resolve;
pub mod router;

pub struct FileReport {
    pub path: PathBuf,
    pub complexity: FileComplexity,
}

/// Analyzes the source files at the given path (a single file or a directory
/// searched recursively) and reports cyclomatic complexity per function.
pub fn analyze(path: &Path, options: &AnalysisOptions) -> io::Result<Vec<FileReport>> {
    let mut files = source_files(path, &options.files)?;
    files.sort();
    let mut reports = Vec::new();
    for file in files {
        if let Some(report) = analyze_file(file, options)? {
            reports.push(report);
        }
    }
    Ok(reports)
}

/// Analyzes one file; `None` when the type filter leaves nothing to report.
fn analyze_file(path: PathBuf, options: &AnalysisOptions) -> io::Result<Option<FileReport>> {
    let source = fs::read_to_string(&path)?;
    let Some(complexity) = router::file_complexity(&path, &source, &options.branches) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported file type: {}", path.display()),
        ));
    };
    Ok(filter_types(complexity, &options.types).map(|complexity| FileReport { path, complexity }))
}

/// Retains only types whose supertypes match the filter. Top-level functions
/// implement nothing, so any selection drops them; a file left with no types
/// drops out of the report entirely.
fn filter_types(complexity: FileComplexity, filter: &TypeFilter) -> Option<FileComplexity> {
    if filter.is_empty() {
        return Some(complexity);
    }
    let mut complexity = complexity;
    complexity.functions.clear();
    complexity
        .types
        .retain(|complexity_type| filter.matches(&complexity_type.supertypes));
    if complexity.types.is_empty() {
        None
    } else {
        Some(complexity)
    }
}

fn source_files(path: &Path, filter: &FileFilter) -> io::Result<Vec<PathBuf>> {
    // A single explicit file bypasses filters: the user pointed at it directly.
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut files = Vec::new();
    collect_files(path, path, filter, &mut files)?;
    Ok(files)
}

fn collect_files(
    root: &Path,
    dir: &Path,
    filter: &FileFilter,
    files: &mut Vec<PathBuf>,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry_path = entry?.path();
        if entry_path.is_dir() {
            collect_files(root, &entry_path, filter, files)?;
        } else if router::is_supported(&entry_path) && matches_relative(root, &entry_path, filter) {
            files.push(entry_path);
        }
    }
    Ok(())
}

/// Globs match against the path relative to the analysis root so patterns
/// like `**/generated/**` behave the same regardless of the current directory.
fn matches_relative(root: &Path, path: &Path, filter: &FileFilter) -> bool {
    filter.matches(path.strip_prefix(root).unwrap_or(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixture_path;

    fn fixtures_dir() -> PathBuf {
        fixture_path("")
    }

    fn include(patterns: &[&str]) -> AnalysisOptions {
        let include: Vec<String> = patterns.iter().map(|pattern| pattern.to_string()).collect();
        AnalysisOptions {
            files: FileFilter::new(&include, &[]).expect("valid globs"),
            ..AnalysisOptions::default()
        }
    }

    #[test]
    fn analyze_reports_all_fixture_files_sorted() {
        let reports = analyze(&fixtures_dir(), &AnalysisOptions::default()).expect("analyze");
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
                "java/Inherits.java",
                "kotlin/complexity.kt",
                "kotlin/inherits.kt",
                "rust/complexity.rs",
                "rust/inherits.rs",
                "swift/complexity.swift",
                "swift/inherits.swift",
            ]
        );
    }

    #[test]
    fn analyze_applies_include_globs() {
        let reports = analyze(&fixtures_dir(), &include(&["*.rs"])).expect("analyze");
        let names: Vec<String> = reports
            .iter()
            .map(|report| report.path.display().to_string())
            .collect();
        assert_eq!(
            names,
            vec![
                fixtures_dir()
                    .join("rust/complexity.rs")
                    .display()
                    .to_string(),
                fixtures_dir()
                    .join("rust/inherits.rs")
                    .display()
                    .to_string(),
            ]
        );
    }

    fn implements(names: &[&str]) -> AnalysisOptions {
        let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        AnalysisOptions {
            types: TypeFilter::new(&names),
            ..AnalysisOptions::default()
        }
    }

    fn relative_names(reports: &[FileReport]) -> Vec<String> {
        reports
            .iter()
            .map(|report| {
                report
                    .path
                    .strip_prefix(fixtures_dir())
                    .expect("fixture path")
                    .display()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn analyze_with_implements_reports_only_matching_types() {
        let reports = analyze(&fixtures_dir(), &implements(&["Describe"])).expect("analyze");
        assert_eq!(
            relative_names(&reports),
            vec![
                "java/Inherits.java",
                "kotlin/inherits.kt",
                "rust/inherits.rs",
                "swift/inherits.swift",
            ]
        );
        for report in &reports {
            assert!(report.complexity.functions.is_empty());
        }
        let type_names: Vec<Vec<String>> = reports
            .iter()
            .map(|report| {
                report
                    .complexity
                    .types
                    .iter()
                    .map(|t| t.name.clone())
                    .collect()
            })
            .collect();
        assert_eq!(
            type_names,
            vec![
                vec!["Circle".to_string(), "Sub".to_string()],
                vec!["Circle".to_string(), "Registry".to_string(),],
                vec!["Circle".to_string(), "Marked".to_string()],
                vec!["Circle".to_string()],
            ]
        );
    }

    #[test]
    fn analyze_with_implements_matches_trailing_simple_name() {
        let reports = analyze(&fixtures_dir(), &implements(&["Display"])).expect("analyze");
        assert_eq!(
            relative_names(&reports),
            vec!["rust/complexity.rs", "rust/inherits.rs"]
        );
    }

    #[test]
    fn analyze_reports_single_file() {
        let path = fixtures_dir().join("swift/complexity.swift");
        let reports = analyze(&path, &AnalysisOptions::default()).expect("analyze single file");
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].complexity.functions.is_empty());
    }

    #[test]
    fn analyze_single_file_bypasses_filters() {
        let path = fixtures_dir().join("swift/complexity.swift");
        let reports = analyze(&path, &include(&["*.rs"])).expect("analyze single file");
        assert_eq!(reports.len(), 1);
    }

    #[test]
    fn analyze_rejects_unsupported_file() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
        let error = match analyze(&path, &AnalysisOptions::default()) {
            Ok(_) => panic!("unsupported file should error"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
}
