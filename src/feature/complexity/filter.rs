//! Filters controlling which files and types an analysis visits: include/
//! exclude globs for files, and `--implements` supertype names for types.

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
    /// Exclude patterns rewritten to match the directory they cover, e.g.
    /// `**/target/**` also compiles as `**/target`. Lets a directory walk
    /// skip a whole subtree instead of checking every file inside it.
    exclude_trees: GlobSet,
}

impl Default for FileFilter {
    fn default() -> FileFilter {
        FileFilter {
            include: None,
            exclude: GlobSet::empty(),
            exclude_trees: GlobSet::empty(),
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
            exclude_trees: glob_set(&subtree_patterns(exclude))?,
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

    /// Whether every path beneath this directory is excluded, so a walk can
    /// skip descending into it. `path` must be relative to the analysis
    /// root. Ignores include patterns (an include like `*.rs` never matches
    /// a directory), so this can only prune directories [`matches`] would
    /// have rejected anyway, never one it would have accepted.
    pub fn excludes_subtree(&self, path: &Path) -> bool {
        self.exclude_trees.is_match(path)
    }
}

/// A `dir/**` exclude pattern covers every path beneath `dir`, so the
/// directory itself can be matched directly. Patterns without that suffix
/// say nothing about a whole subtree and are dropped.
fn subtree_patterns(exclude: &[String]) -> Vec<String> {
    exclude
        .iter()
        .filter_map(|pattern| pattern.strip_suffix("/**").map(str::to_string))
        .collect()
}

/// Supertype names selected by `--implements`. A type matches when any of its
/// supertypes matches any selected name; an empty selection matches every
/// type. Names compare with generic arguments stripped
/// (`Comparable<String>` → `Comparable`), against either the full supertype
/// text or its trailing simple name (`Display` matches `std::fmt::Display`).
#[derive(Debug, Default)]
pub struct TypeFilter {
    implements: Vec<String>,
}

impl TypeFilter {
    pub fn new(implements: &[String]) -> TypeFilter {
        TypeFilter {
            implements: implements.iter().map(|name| normalize(name)).collect(),
        }
    }

    /// Whether no supertype names were selected (every type matches).
    pub fn is_empty(&self) -> bool {
        self.implements.is_empty()
    }

    /// Whether a type with these supertypes should be analyzed.
    pub fn matches(&self, supertypes: &[String]) -> bool {
        self.is_empty()
            || supertypes
                .iter()
                .any(|supertype| self.matches_supertype(supertype))
    }

    fn matches_supertype(&self, supertype: &str) -> bool {
        let normalized = normalize(supertype);
        let simple = simple_name(&normalized);
        self.implements
            .iter()
            .any(|name| *name == normalized || *name == simple)
    }
}

/// Strips generic arguments and surrounding whitespace: `a.b.C<D>` → `a.b.C`.
fn normalize(name: &str) -> String {
    let base = match name.find('<') {
        Some(index) => &name[..index],
        None => name,
    };
    base.trim().to_string()
}

/// The trailing simple name of a possibly qualified type:
/// `std::fmt::Display` → `Display`, `Swift.Codable` → `Codable`.
fn simple_name(normalized: &str) -> &str {
    normalized.rsplit(['.', ':']).next().unwrap_or(normalized)
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
    fn excludes_subtree_matches_directory_covered_by_slash_star_star() {
        let filter = FileFilter::new(&[], &strings(&["**/target/**"])).expect("valid globs");
        assert!(filter.excludes_subtree(Path::new("target")));
        assert!(filter.excludes_subtree(Path::new("nested/target")));
    }

    #[test]
    fn excludes_subtree_ignores_pattern_without_slash_star_star_suffix() {
        let filter = FileFilter::new(&[], &strings(&["target"])).expect("valid globs");
        assert!(!filter.excludes_subtree(Path::new("target")));
    }

    #[test]
    fn excludes_subtree_false_for_unrelated_directory() {
        let filter = FileFilter::new(&[], &strings(&["**/target/**"])).expect("valid globs");
        assert!(!filter.excludes_subtree(Path::new("src")));
    }

    #[test]
    fn excludes_subtree_ignores_include_patterns() {
        let filter =
            FileFilter::new(&strings(&["*.rs"]), &strings(&["**/target/**"])).expect("valid globs");
        assert!(filter.excludes_subtree(Path::new("target")));
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

    #[test]
    fn empty_type_filter_matches_every_type() {
        let filter = TypeFilter::default();
        assert!(filter.is_empty());
        assert!(filter.matches(&strings(&["Describe"])));
        assert!(filter.matches(&[]));
    }

    #[test]
    fn type_filter_matches_full_supertype_text() {
        let filter = TypeFilter::new(&strings(&["std::fmt::Display"]));
        assert!(filter.matches(&strings(&["std::fmt::Display"])));
        assert!(!filter.matches(&strings(&["Display2"])));
    }

    #[test]
    fn type_filter_matches_trailing_simple_name() {
        let filter = TypeFilter::new(&strings(&["Display"]));
        assert!(filter.matches(&strings(&["std::fmt::Display"])));
        let filter = TypeFilter::new(&strings(&["Codable"]));
        assert!(filter.matches(&strings(&["Swift.Codable"])));
    }

    #[test]
    fn type_filter_strips_generic_arguments_from_supertypes() {
        let filter = TypeFilter::new(&strings(&["Comparable"]));
        assert!(filter.matches(&strings(&["Comparable<String>"])));
    }

    #[test]
    fn type_filter_strips_generic_arguments_and_whitespace_from_names() {
        let filter = TypeFilter::new(&strings(&["Comparable <String>"]));
        assert!(filter.matches(&strings(&["Comparable<Circle>"])));
    }

    #[test]
    fn type_filter_ors_across_names() {
        let filter = TypeFilter::new(&strings(&["Describe", "Labeled"]));
        assert!(filter.matches(&strings(&["Labeled"])));
        assert!(filter.matches(&strings(&["Base", "Describe"])));
        assert!(!filter.matches(&strings(&["Base"])));
    }

    #[test]
    fn type_filter_rejects_types_without_supertypes() {
        let filter = TypeFilter::new(&strings(&["Describe"]));
        assert!(!filter.matches(&[]));
    }
}
