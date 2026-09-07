//! Tree-shaped analysis: like [`super::analyze`], but preserving the
//! directory structure each root was traversed through and which root each
//! file came from, rather than a flat, cross-root-deduped report list.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::feature::complexity::filter::FileFilter;
use crate::feature::complexity::options::AnalysisOptions;
use crate::feature::complexity::router;
use crate::feature::complexity::{
    FileReport, PathError, analyze_files, explicit_file_included, matches_relative,
};

/// One tree per input root — root provenance is preserved (unlike
/// [`super::analyze`], which dedups across roots): overlapping roots each
/// carry their own copy of the files they share.
///
/// A directory whose failure to read comes up mid-walk is attributed to
/// that directory rather than the root, and traversal continues with its
/// siblings; a root that can't be read at all contributes no tree, matching
/// how [`super::analyze`] treats a bad root.
#[derive(Debug)]
pub struct TreeAnalysis {
    pub roots: Vec<TreeNode>,
    pub errors: Vec<PathError>,
}

/// One entry encountered while traversing a root.
#[derive(Debug, Clone)]
pub enum TreeNode {
    Directory(DirectoryNode),
    File(FileNode),
}

/// A directory traversed while building a tree. Absent from its parent's
/// children when it has no included descendants, so `.git`/docs noise never
/// appears.
#[derive(Debug, Clone)]
pub struct DirectoryNode {
    pub path: PathBuf,
    /// Sorted by path; directories and files interleave alphabetically.
    pub children: Vec<TreeNode>,
}

/// A file that matched traversal filters.
#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    /// `None` when the file matched traversal filters but produced no
    /// report (the type filter left nothing, or it errored — see
    /// [`TreeAnalysis::errors`]).
    pub report: Option<FileReport>,
}

impl TreeNode {
    pub fn path(&self) -> &Path {
        match self {
            TreeNode::Directory(directory) => &directory.path,
            TreeNode::File(file) => &file.path,
        }
    }
}

impl TreeAnalysis {
    /// Every report in the tree, sorted by path and deduplicated, ready to
    /// pass to [`super::check::check`]. Overlapping roots contribute a file
    /// only once, matching [`super::analyze`]'s cross-root dedup.
    pub fn reports(&self) -> Vec<FileReport> {
        let mut reports = Vec::new();
        collect_reports(&self.roots, &mut reports);
        reports.sort_by(|a, b| a.path.cmp(&b.path));
        reports.dedup_by(|a, b| a.path == b.path);
        reports
    }
}

fn collect_reports(nodes: &[TreeNode], reports: &mut Vec<FileReport>) {
    for node in nodes {
        match node {
            TreeNode::Directory(directory) => collect_reports(&directory.children, reports),
            TreeNode::File(file) => reports.extend(file.report.clone()),
        }
    }
}

/// Analyzes the source files at the given paths (files or directories,
/// searched recursively), returning the traversed structure with a report
/// attached to each included file.
pub fn analyze_tree(paths: &[PathBuf], options: &AnalysisOptions) -> TreeAnalysis {
    let mut walk = Walk::new(&options.files);
    let roots: Vec<TreeNode> = paths.iter().filter_map(|path| walk.root(path)).collect();
    let (files, mut errors) = walk.finish();
    let (reports, report_errors) = analyze_files(files, options);
    errors.extend(report_errors);
    let reports = index(reports);
    let roots = roots
        .into_iter()
        .map(|node| attach_reports(node, &reports))
        .collect();
    TreeAnalysis { roots, errors }
}

fn index(reports: Vec<FileReport>) -> HashMap<PathBuf, FileReport> {
    reports
        .into_iter()
        .map(|report| (report.path.clone(), report))
        .collect()
}

fn attach_reports(node: TreeNode, reports: &HashMap<PathBuf, FileReport>) -> TreeNode {
    match node {
        TreeNode::Directory(directory) => TreeNode::Directory(DirectoryNode {
            path: directory.path,
            children: directory
                .children
                .into_iter()
                .map(|child| attach_reports(child, reports))
                .collect(),
        }),
        TreeNode::File(file) => TreeNode::File(FileNode {
            report: reports.get(&file.path).cloned(),
            path: file.path,
        }),
    }
}

/// Accumulates discovered files and per-directory errors while building the
/// tree skeleton for every root, before any file is analyzed.
struct Walk<'a> {
    filter: &'a FileFilter,
    files: Vec<PathBuf>,
    errors: Vec<PathError>,
}

impl<'a> Walk<'a> {
    fn new(filter: &'a FileFilter) -> Walk<'a> {
        Walk {
            filter,
            files: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn finish(self) -> (Vec<PathBuf>, Vec<PathError>) {
        let mut files = self.files;
        files.sort();
        files.dedup();
        (files, self.errors)
    }

    fn root(&mut self, path: &Path) -> Option<TreeNode> {
        if path.is_file() {
            self.file_root(path)
        } else {
            self.directory(path, path)
        }
    }

    fn file_root(&mut self, path: &Path) -> Option<TreeNode> {
        if !explicit_file_included(path, self.filter) {
            return None;
        }
        self.files.push(path.to_path_buf());
        Some(TreeNode::File(FileNode {
            path: path.to_path_buf(),
            report: None,
        }))
    }

    /// Builds the node for `dir` (relative to `root`), or `None` when it has
    /// no included descendants: pruning propagates upward naturally, since
    /// a parent whose every child is pruned ends up with an empty list too.
    fn directory(&mut self, root: &Path, dir: &Path) -> Option<TreeNode> {
        let entries = self.entries(dir)?;
        let children: Vec<TreeNode> = entries
            .into_iter()
            .filter_map(|entry| self.child(root, &entry))
            .collect();
        if children.is_empty() {
            None
        } else {
            Some(TreeNode::Directory(DirectoryNode {
                path: dir.to_path_buf(),
                children,
            }))
        }
    }

    fn child(&mut self, root: &Path, path: &Path) -> Option<TreeNode> {
        if path.is_dir() {
            if self.filter.excludes_subtree(relative(root, path)) {
                return None;
            }
            self.directory(root, path)
        } else if router::is_supported(path) && matches_relative(root, path, self.filter) {
            self.files.push(path.to_path_buf());
            Some(TreeNode::File(FileNode {
                path: path.to_path_buf(),
                report: None,
            }))
        } else {
            None
        }
    }

    /// Reads and sorts a directory's entries, recording (and pruning past)
    /// a read failure instead of aborting the rest of the walk.
    fn entries(&mut self, dir: &Path) -> Option<Vec<PathBuf>> {
        match read_entries(dir) {
            Ok(mut paths) => {
                paths.sort();
                Some(paths)
            }
            Err(error) => {
                self.errors.push(PathError {
                    path: dir.to_path_buf(),
                    error,
                });
                None
            }
        }
    }
}

fn read_entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect()
}

/// The same relative-path rule `matches_relative` applies for files, used
/// directly for the subtree-exclude check on directories.
fn relative<'a>(root: &Path, path: &'a Path) -> &'a Path {
    path.strip_prefix(root).unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feature::complexity::analyze;
    use crate::testing::fixture_path;

    fn fixtures_dir() -> PathBuf {
        fixture_path("")
    }

    fn render(nodes: &[TreeNode], root: &Path, output: &mut Vec<String>) {
        for node in nodes {
            let relative = node.path().strip_prefix(root).expect("node under root");
            let relative = if relative.as_os_str().is_empty() {
                ".".to_string()
            } else {
                relative.display().to_string()
            };
            match node {
                TreeNode::Directory(directory) => {
                    output.push(format!("d {relative}"));
                    render(&directory.children, root, output);
                }
                TreeNode::File(_) => output.push(format!("f {relative}")),
            }
        }
    }

    fn flatten(analysis: &TreeAnalysis, root: &Path) -> Vec<String> {
        let mut output = Vec::new();
        render(&analysis.roots, root, &mut output);
        output
    }

    fn include(patterns: &[&str]) -> AnalysisOptions {
        let include: Vec<String> = patterns.iter().map(|pattern| pattern.to_string()).collect();
        AnalysisOptions {
            files: FileFilter::new(&include, &[]).expect("valid globs"),
            ..AnalysisOptions::default()
        }
    }

    fn exclude(patterns: &[&str]) -> AnalysisOptions {
        let exclude: Vec<String> = patterns.iter().map(|pattern| pattern.to_string()).collect();
        AnalysisOptions {
            files: FileFilter::new(&[], &exclude).expect("valid globs"),
            ..AnalysisOptions::default()
        }
    }

    fn summary(reports: &[FileReport]) -> Vec<(String, usize)> {
        reports
            .iter()
            .map(|report| (report.path.display().to_string(), report.lines))
            .collect()
    }

    #[test]
    fn analyze_tree_builds_nested_structure_sorted_by_path() {
        let root = fixtures_dir().join("rust");
        let analysis = analyze_tree(std::slice::from_ref(&root), &AnalysisOptions::default());
        assert!(analysis.errors.is_empty());
        assert_eq!(
            flatten(&analysis, &root),
            vec!["d .", "f comments.rs", "f complexity.rs", "f inherits.rs"]
        );
    }

    #[test]
    fn analyze_tree_prunes_directories_without_included_files() {
        let root = fixtures_dir();
        let analysis = analyze_tree(std::slice::from_ref(&root), &AnalysisOptions::default());
        assert!(analysis.errors.is_empty());
        let top_level = flatten(&analysis, &root)
            .into_iter()
            .filter(|line| line.starts_with("d ") && line.matches('/').count() == 0)
            .collect::<Vec<_>>();
        assert!(!top_level.contains(&"d config".to_string()));
        assert!(top_level.contains(&"d rust".to_string()));
    }

    #[test]
    fn analyze_tree_applies_include_globs_relative_to_root() {
        let root = fixtures_dir();
        let analysis = analyze_tree(std::slice::from_ref(&root), &include(&["*.rs"]));
        assert!(analysis.errors.is_empty());
        assert_eq!(
            flatten(&analysis, &root),
            vec![
                "d .",
                "d rust",
                "f rust/comments.rs",
                "f rust/complexity.rs",
                "f rust/inherits.rs"
            ]
        );
    }

    #[test]
    fn analyze_tree_applies_exclude_globs_relative_to_root() {
        let root = fixtures_dir();
        let analysis = analyze_tree(std::slice::from_ref(&root), &exclude(&["rust/**"]));
        assert!(analysis.errors.is_empty());
        assert!(!flatten(&analysis, &root).contains(&"d rust".to_string()));
        let expected =
            summary(&analyze(std::slice::from_ref(&root), &exclude(&["rust/**"])).reports);
        assert_eq!(summary(&analysis.reports()), expected);
    }

    #[test]
    fn analyze_tree_keeps_provenance_for_overlapping_roots() {
        let file = fixtures_dir().join("rust/complexity.rs");
        let dir = fixtures_dir().join("rust");
        let analysis = analyze_tree(&[dir.clone(), file.clone()], &AnalysisOptions::default());
        assert!(analysis.errors.is_empty());
        assert_eq!(analysis.roots.len(), 2);
        match &analysis.roots[0] {
            TreeNode::Directory(directory) => assert_eq!(directory.children.len(), 3),
            TreeNode::File(_) => panic!("expected first root to be a directory"),
        }
        match &analysis.roots[1] {
            TreeNode::File(node) => {
                assert_eq!(node.path, file);
                assert!(node.report.is_some());
            }
            TreeNode::Directory(_) => panic!("expected second root to be a file"),
        }
        assert_eq!(analysis.reports().len(), 3);
    }

    #[test]
    fn analyze_tree_single_file_root_yields_file_node() {
        let path = fixtures_dir().join("swift/complexity.swift");
        let analysis = analyze_tree(std::slice::from_ref(&path), &AnalysisOptions::default());
        assert!(analysis.errors.is_empty());
        assert_eq!(analysis.roots.len(), 1);
        match &analysis.roots[0] {
            TreeNode::File(node) => {
                assert_eq!(node.path, path);
                assert!(node.report.is_some());
            }
            TreeNode::Directory(_) => panic!("expected a file node"),
        }
    }

    #[test]
    fn analyze_tree_single_file_root_applies_filters() {
        let path = fixtures_dir().join("swift/complexity.swift");
        let analysis = analyze_tree(&[path], &include(&["*.rs"]));
        assert!(analysis.errors.is_empty());
        assert!(analysis.roots.is_empty());
    }

    #[test]
    fn analyze_tree_reports_missing_root_but_continues() {
        let missing = fixtures_dir().join("does-not-exist");
        let rust = fixtures_dir().join("rust");
        let analysis = analyze_tree(
            &[missing.clone(), rust.clone()],
            &AnalysisOptions::default(),
        );
        assert_eq!(analysis.errors.len(), 1);
        assert_eq!(analysis.errors[0].path, missing);
        assert_eq!(
            flatten(&analysis, &rust),
            vec!["d .", "f comments.rs", "f complexity.rs", "f inherits.rs"]
        );
    }

    #[test]
    fn analyze_tree_reports_match_analyze_with_default_options() {
        let root = fixtures_dir();
        let expected =
            summary(&analyze(std::slice::from_ref(&root), &AnalysisOptions::default()).reports);
        let analysis = analyze_tree(&[root], &AnalysisOptions::default());
        assert_eq!(summary(&analysis.reports()), expected);
    }

    #[test]
    fn analyze_tree_reports_match_analyze_for_overlapping_roots() {
        let file = fixtures_dir().join("rust/complexity.rs");
        let dir = fixtures_dir().join("rust");
        let options = AnalysisOptions::default();
        let expected = summary(&analyze(&[dir.clone(), file.clone()], &options).reports);
        let analysis = analyze_tree(&[dir, file], &options);
        assert_eq!(summary(&analysis.reports()), expected);
    }

    fn implements(names: &[&str]) -> AnalysisOptions {
        use crate::feature::complexity::filter::TypeFilter;
        let names: Vec<String> = names.iter().map(|name| name.to_string()).collect();
        AnalysisOptions {
            types: TypeFilter::new(&names),
            ..AnalysisOptions::default()
        }
    }

    #[test]
    fn analyze_tree_leaves_report_none_when_type_filter_excludes_file() {
        let root = fixtures_dir().join("rust");
        let options = implements(&["NoSuchType"]);
        let analysis = analyze_tree(std::slice::from_ref(&root), &options);
        assert!(analysis.errors.is_empty());
        assert_eq!(
            flatten(&analysis, &root),
            vec!["d .", "f comments.rs", "f complexity.rs", "f inherits.rs"]
        );
        for node in &analysis.roots {
            if let TreeNode::File(file) = node {
                assert!(file.report.is_none());
            }
        }
        assert!(analysis.reports().is_empty());
    }

    #[test]
    fn analysis_options_and_overrides_are_unchanged() {
        use crate::feature::complexity::resolve::Overrides;

        let options = AnalysisOptions {
            max_lines: Some(1),
            ..AnalysisOptions::default()
        };
        let _ = analyze_tree(&[fixtures_dir().join("rust")], &options);
        let _overrides = Overrides {
            ..Overrides::default()
        };
    }
}
