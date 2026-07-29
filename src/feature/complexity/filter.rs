//! Include/exclude glob filtering for the files an analysis visits.

use std::io;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

/// Compiled `--include`/`--exclude` globs. Patterns match paths relative to
/// the analysis root, and globset's default separator behavior is kept so
/// `*.rs` matches nested files.
#[derive(Debug)]
pub struct FileFilter {
    /// `None` when no include patterns were given: everything is included.
    include: Option<GlobSet>,
    exclude: GlobSet,
}

impl Default for FileFilter {
    fn default() -> FileFilter {
        FileFilter {
            include: None,
            exclude: GlobSet::empty(),
        }
    }
}

impl FileFilter {
    pub fn new(include: &[String], exclude: &[String]) -> io::Result<FileFilter> {
        let include = if include.is_empty() {
            None
        } else {
            Some(glob_set(include)?)
        };
        Ok(FileFilter {
            include,
            exclude: glob_set(exclude)?,
        })
    }

    /// Whether the analysis should visit the file. `path` must be relative to
    /// the analysis root.
    pub fn matches(&self, path: &Path) -> bool {
        let included = self
            .include
            .as_ref()
            .is_none_or(|include| include.is_match(path));
        included && !self.exclude.is_match(path)
    }
}

fn glob_set(patterns: &[String]) -> io::Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid glob pattern: {pattern}: {error}"),
            )
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid glob patterns: {error}"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(patterns: &[&str]) -> Vec<String> {
        patterns.iter().map(|pattern| pattern.to_string()).collect()
    }

    #[test]
    fn default_filter_matches_everything() {
        let filter = FileFilter::default();
        assert!(filter.matches(Path::new("src/main.rs")));
        assert!(filter.matches(Path::new("notes.md")));
    }

    #[test]
    fn include_only_limits_matches() {
        let filter = FileFilter::new(&strings(&["*.rs"]), &[]).expect("valid globs");
        assert!(filter.matches(Path::new("main.rs")));
        assert!(!filter.matches(Path::new("main.swift")));
    }

    #[test]
    fn include_matches_nested_files() {
        let filter = FileFilter::new(&strings(&["*.rs"]), &[]).expect("valid globs");
        assert!(filter.matches(Path::new("deeply/nested/dir/file.rs")));
    }

    #[test]
    fn exclude_only_rejects_matches() {
        let filter = FileFilter::new(&[], &strings(&["**/generated/**"])).expect("valid globs");
        assert!(filter.matches(Path::new("src/main.rs")));
        assert!(!filter.matches(Path::new("src/generated/api.rs")));
    }

    #[test]
    fn exclude_wins_over_include() {
        let filter = FileFilter::new(&strings(&["*.rs"]), &strings(&["**/generated/**"]))
            .expect("valid globs");
        assert!(filter.matches(Path::new("src/main.rs")));
        assert!(!filter.matches(Path::new("src/generated/api.rs")));
    }

    #[test]
    fn invalid_glob_reports_invalid_input_naming_the_pattern() {
        let error = FileFilter::new(&strings(&["["]), &[]).expect_err("invalid glob");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("["));
    }
}
