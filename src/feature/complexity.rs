use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::code::FileComplexity;
use crate::feature::complexity::filter::{FileFilter, TypeFilter};
use crate::feature::complexity::options::AnalysisOptions;

pub mod check;
pub mod config;
pub mod filter;
pub mod options;
pub mod resolve;
pub mod router;

pub struct FileReport {
    pub path: PathBuf,
    pub complexity: FileComplexity,
}

/// A path passed to [`analyze`] that couldn't be read.
pub struct PathError {
    pub path: PathBuf,
    pub error: io::Error,
}

/// The result of analyzing every given path: successful reports plus any
/// per-path or per-file errors, so one bad path doesn't discard the rest.
pub struct Analysis {
    pub reports: Vec<FileReport>,
    pub errors: Vec<PathError>,
}

/// Analyzes the source files at the given paths (files or directories,
/// searched recursively) and reports cyclomatic complexity per function.
/// Overlapping paths are deduplicated.
pub fn analyze(paths: &[PathBuf], options: &AnalysisOptions) -> Analysis {
    let (files, mut errors) = discover(paths, &options.files);
    let (mut reports, file_errors) = analyze_files(files, options);
    errors.extend(file_errors);
    reports.sort_by(|a, b| a.path.cmp(&b.path));
    Analysis { reports, errors }
}

/// Resolves every root to its source files, collecting an error per root that
/// can't be read rather than aborting the whole run.
fn discover(paths: &[PathBuf], filter: &FileFilter) -> (Vec<PathBuf>, Vec<PathError>) {
    let mut files = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        match source_files(path, filter) {
            Ok(found) => files.extend(found),
            Err(error) => errors.push(PathError {
                path: path.clone(),
                error,
            }),
        }
    }
    files.sort();
    files.dedup();
    (files, errors)
}

/// Analyzes every file, partitioning successes from failures instead of
/// short-circuiting on the first unreadable file.
fn analyze_files(
    files: Vec<PathBuf>,
    options: &AnalysisOptions,
) -> (Vec<FileReport>, Vec<PathError>) {
    let results: Vec<(PathBuf, io::Result<Option<FileReport>>)> = files
        .into_par_iter()
        .map(|file| (file.clone(), analyze_file(file, options)))
        .collect();
    let mut reports = Vec::new();
    let mut errors = Vec::new();
    for (path, result) in results {
        match result {
            Ok(Some(report)) => reports.push(report),
            Ok(None) => {}
            Err(error) => errors.push(PathError { path, error }),
        }
    }
    (reports, errors)
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
    if path.is_file() {
        return Ok(explicit_file(path, filter));
    }
    let mut files = Vec::new();
    collect_files(path, path, filter, &mut files)?;
    Ok(files)
}

/// An explicitly-named file is included when it's a supported type and
/// matches the include/exclude filters, same as a file found by directory
/// search; otherwise it's silently skipped rather than erroring, since
/// callers may pass mixed lists (e.g. a `git diff` file list).
fn explicit_file(path: &Path, filter: &FileFilter) -> Vec<PathBuf> {
    if router::is_supported(path) && filter.matches(path) {
        vec![path.to_path_buf()]
    } else {
        Vec::new()
    }
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
        let analysis = analyze(&[fixtures_dir()], &AnalysisOptions::default());
        assert!(analysis.errors.is_empty());
        let names: Vec<String> = analysis
            .reports
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
                "javascript/complexity.js",
                "javascript/inherits.js",
                "kotlin/complexity.kt",
                "kotlin/inherits.kt",
                "python/complexity.py",
                "python/inherits.py",
                "rust/complexity.rs",
                "rust/inherits.rs",
                "swift/complexity.swift",
                "swift/inherits.swift",
                "typescript/complexity.ts",
                "typescript/complexity.tsx",
                "typescript/inherits.ts",
            ]
        );
    }

    #[test]
    fn analyze_applies_include_globs() {
        let reports = analyze(&[fixtures_dir()], &include(&["*.rs"])).reports;
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
        let reports = analyze(&[fixtures_dir()], &implements(&["Describe"])).reports;
        assert_eq!(
            relative_names(&reports),
            vec![
                "java/Inherits.java",
                "javascript/inherits.js",
                "kotlin/inherits.kt",
                "python/inherits.py",
                "rust/inherits.rs",
                "swift/inherits.swift",
                "typescript/inherits.ts",
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
                vec!["Circle".to_string()],
                vec!["Circle".to_string(), "Registry".to_string(),],
                vec!["Circle".to_string(), "Registry".to_string()],
                vec!["Circle".to_string(), "Marked".to_string()],
                vec!["Circle".to_string()],
                vec!["Circle".to_string()],
            ]
        );
    }

    #[test]
    fn analyze_with_implements_matches_trailing_simple_name() {
        let reports = analyze(&[fixtures_dir()], &implements(&["Display"])).reports;
        assert_eq!(
            relative_names(&reports),
            vec!["rust/complexity.rs", "rust/inherits.rs"]
        );
    }

    #[test]
    fn analyze_reports_single_file() {
        let path = fixtures_dir().join("swift/complexity.swift");
        let analysis = analyze(&[path], &AnalysisOptions::default());
        assert!(analysis.errors.is_empty());
        assert_eq!(analysis.reports.len(), 1);
        assert!(!analysis.reports[0].complexity.functions.is_empty());
    }

    #[test]
    fn analyze_single_file_applies_filters() {
        let path = fixtures_dir().join("swift/complexity.swift");
        let analysis = analyze(&[path], &include(&["*.rs"]));
        assert!(analysis.errors.is_empty());
        assert!(analysis.reports.is_empty());
    }

    #[test]
    fn analyze_skips_unsupported_file() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
        let analysis = analyze(&[path], &AnalysisOptions::default());
        assert!(analysis.reports.is_empty());
        assert!(analysis.errors.is_empty());
    }

    #[test]
    fn analyze_merges_multiple_paths() {
        let paths = vec![
            fixtures_dir().join("rust"),
            fixtures_dir().join("swift/complexity.swift"),
        ];
        let analysis = analyze(&paths, &AnalysisOptions::default());
        assert!(analysis.errors.is_empty());
        assert_eq!(
            relative_names(&analysis.reports),
            vec![
                "rust/complexity.rs",
                "rust/inherits.rs",
                "swift/complexity.swift"
            ]
        );
    }

    #[test]
    fn analyze_dedupes_overlapping_paths() {
        let file = fixtures_dir().join("rust/complexity.rs");
        let analysis = analyze(
            &[fixtures_dir().join("rust"), file],
            &AnalysisOptions::default(),
        );
        assert!(analysis.errors.is_empty());
        assert_eq!(
            relative_names(&analysis.reports),
            vec!["rust/complexity.rs", "rust/inherits.rs"]
        );
    }

    #[test]
    fn analyze_reports_missing_path_but_continues() {
        let missing = fixtures_dir().join("does-not-exist");
        let paths = vec![missing.clone(), fixtures_dir().join("rust")];
        let analysis = analyze(&paths, &AnalysisOptions::default());
        assert_eq!(analysis.errors.len(), 1);
        assert_eq!(analysis.errors[0].path, missing);
        assert_eq!(
            relative_names(&analysis.reports),
            vec!["rust/complexity.rs", "rust/inherits.rs"]
        );
    }
}
