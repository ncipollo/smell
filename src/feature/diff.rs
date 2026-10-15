//! Commit-aware complexity diffing: resolves file contents at two git
//! commits directly through `git2` (no pre-checked-out directories needed)
//! and classifies each function as added, removed, changed, or unchanged.
//!
//! [`diff`] is a reusable library entry point, not CLI glue — it takes two
//! already-chosen refs and a repo path, so a caller that resolves a PR down
//! to a base/head commit pair (rather than named refs) can call it directly
//! without going through the CLI layer.

use std::fmt;
use std::path::{Path, PathBuf};

use git2::{Repository, Tree};
use rayon::prelude::*;

use crate::code::FileComplexity;
use crate::code::branch::BranchFilter;
use crate::feature::complexity::router;

pub mod classify;
pub mod delta;
mod git;

pub use classify::{DiffStatus, FlatActivity, FunctionDiff};

/// A file's measures at one revision, kept around after classifying
/// functions so [`delta`] can diff the other measures without re-parsing.
#[derive(Debug)]
pub struct Snapshot {
    pub lines: usize,
    pub complexity: FileComplexity,
}

#[derive(Debug)]
pub struct FileDiff {
    pub path: PathBuf,
    pub functions: Vec<FunctionDiff>,
    pub old: Option<Snapshot>,
    pub new: Option<Snapshot>,
}

/// A path that couldn't be diffed: absent from both revisions, or whose
/// blob content couldn't be read.
#[derive(Debug)]
pub struct DiffPathError {
    pub path: PathBuf,
    pub message: String,
}

/// The result of diffing every given path: successful file diffs plus any
/// per-path errors, so one bad path doesn't discard the rest.
#[derive(Debug)]
pub struct DiffReport {
    pub files: Vec<FileDiff>,
    pub errors: Vec<DiffPathError>,
}

/// A setup failure that aborts the whole diff: the repo or a ref couldn't be
/// resolved.
#[derive(Debug)]
pub enum DiffError {
    OpenRepo(PathBuf, git2::Error),
    ResolveRef(String, git2::Error),
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::OpenRepo(path, error) => {
                write!(f, "opening repo at {}: {error}", path.display())
            }
            DiffError::ResolveRef(reference, error) => {
                write!(f, "resolving ref \"{reference}\": {error}")
            }
        }
    }
}

/// Diffs cyclomatic complexity for `paths` (relative to the repository root;
/// directories are searched recursively) between `old_ref` and `new_ref` in
/// the repository at or above `repo_path`.
pub fn diff(
    repo_path: &Path,
    old_ref: &str,
    new_ref: &str,
    paths: &[PathBuf],
) -> Result<DiffReport, DiffError> {
    let repo = git::open(repo_path)
        .map_err(|error| DiffError::OpenRepo(repo_path.to_path_buf(), error))?;
    let old_tree = tree_at(&repo, old_ref)?;
    let new_tree = tree_at(&repo, new_ref)?;

    let (candidates, unmatched) = git::discover_paths(&old_tree, &new_tree, paths);
    let mut errors: Vec<DiffPathError> = unmatched
        .into_iter()
        .map(|path| DiffPathError {
            path,
            message: "not found in either revision".to_string(),
        })
        .collect();

    let blobs = read_blobs(&repo, &old_tree, &new_tree, candidates, &mut errors);
    let files = blobs
        .into_par_iter()
        .filter_map(|(path, old_src, new_src)| build_file_diff(&path, old_src, new_src))
        .collect();

    Ok(DiffReport { files, errors })
}

fn tree_at<'repo>(repo: &'repo Repository, reference: &str) -> Result<Tree<'repo>, DiffError> {
    let commit = git::resolve_commit(repo, reference)
        .map_err(|error| DiffError::ResolveRef(reference.to_string(), error))?;
    commit
        .tree()
        .map_err(|error| DiffError::ResolveRef(reference.to_string(), error))
}

/// Reads every candidate's blob content from both trees up front, serially —
/// `git2` objects aren't safely shareable across the rayon pool used below
/// for the CPU-bound parse-and-classify phase.
fn read_blobs(
    repo: &Repository,
    old_tree: &Tree,
    new_tree: &Tree,
    candidates: Vec<PathBuf>,
    errors: &mut Vec<DiffPathError>,
) -> Vec<(PathBuf, Option<String>, Option<String>)> {
    let mut blobs = Vec::new();
    for path in candidates {
        match (
            git::blob_at(repo, old_tree, &path),
            git::blob_at(repo, new_tree, &path),
        ) {
            (Ok(old_src), Ok(new_src)) => blobs.push((path, old_src, new_src)),
            (Err(error), _) | (_, Err(error)) => errors.push(DiffPathError {
                path,
                message: error.to_string(),
            }),
        }
    }
    blobs
}

fn build_file_diff(
    path: &Path,
    old_src: Option<String>,
    new_src: Option<String>,
) -> Option<FileDiff> {
    let filter = BranchFilter::default();
    let old = snapshot(path, old_src.as_deref(), &filter);
    let new = snapshot(path, new_src.as_deref(), &filter);
    let functions = classify::classify_file(
        old.as_ref().map(|snapshot| &snapshot.complexity),
        new.as_ref().map(|snapshot| &snapshot.complexity),
    );
    if functions.is_empty() {
        None
    } else {
        Some(FileDiff {
            path: path.to_path_buf(),
            functions,
            old,
            new,
        })
    }
}

fn snapshot(path: &Path, source: Option<&str>, filter: &BranchFilter) -> Option<Snapshot> {
    let source = source?;
    let complexity = router::file_complexity(path, source, filter)?;
    Some(Snapshot {
        lines: source.lines().count(),
        complexity,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn init_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        Repository::init(dir.path()).unwrap();
        dir
    }

    fn commit_all(dir: &Path, message: &str) {
        let repo = Repository::open(dir).unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<_> = parent.iter().collect();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .unwrap();
    }

    #[test]
    fn diff_reports_an_added_a_changed_and_an_unchanged_function() {
        let dir = init_repo();
        fs::write(dir.path().join("lib.rs"), "fn a() {}\nfn b() {}\n").unwrap();
        commit_all(dir.path(), "first");

        fs::write(
            dir.path().join("lib.rs"),
            "fn a() { if true {} }\nfn b() {}\nfn c() {}\n",
        )
        .unwrap();
        commit_all(dir.path(), "second");

        let report =
            diff(dir.path(), "HEAD~1", "HEAD", &[PathBuf::from("lib.rs")]).expect("diff succeeds");

        assert!(report.errors.is_empty());
        assert_eq!(report.files.len(), 1);
        let file = &report.files[0];
        assert_eq!(file.path, PathBuf::from("lib.rs"));

        let status = |name: &str| {
            file.functions
                .iter()
                .find(|f| f.name == name)
                .unwrap()
                .status
        };
        assert_eq!(
            status("a"),
            DiffStatus::Changed {
                before: 1,
                after: 2
            }
        );
        assert_eq!(status("b"), DiffStatus::Unchanged { complexity: 1 });
        assert_eq!(status("c"), DiffStatus::Added { complexity: 1 });
    }

    #[test]
    fn diff_reports_an_error_for_an_unresolvable_ref() {
        let dir = init_repo();
        fs::write(dir.path().join("lib.rs"), "fn a() {}\n").unwrap();
        commit_all(dir.path(), "first");

        let error = diff(
            dir.path(),
            "does-not-exist",
            "HEAD",
            &[PathBuf::from("lib.rs")],
        )
        .expect_err("unresolvable ref fails");
        assert!(
            matches!(error, DiffError::ResolveRef(reference, _) if reference == "does-not-exist")
        );
    }

    #[test]
    fn diff_reports_an_error_for_an_unresolvable_repo() {
        let dir = TempDir::new().unwrap();
        let error = diff(dir.path(), "HEAD~1", "HEAD", &[PathBuf::from("lib.rs")])
            .expect_err("non-repo path fails");
        assert!(matches!(error, DiffError::OpenRepo(_, _)));
    }

    #[test]
    fn diff_soft_errors_a_path_missing_from_both_revisions() {
        let dir = init_repo();
        fs::write(dir.path().join("lib.rs"), "fn a() {}\n").unwrap();
        commit_all(dir.path(), "first");
        commit_all(dir.path(), "second");

        let report = diff(dir.path(), "HEAD~1", "HEAD", &[PathBuf::from("missing.rs")])
            .expect("diff succeeds with soft errors");
        assert!(report.files.is_empty());
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].path, PathBuf::from("missing.rs"));
    }

    #[test]
    fn diff_carries_old_and_new_snapshots_for_delta_checks() {
        let dir = init_repo();
        fs::write(dir.path().join("lib.rs"), "fn a() {}\n").unwrap();
        commit_all(dir.path(), "first");

        fs::write(
            dir.path().join("lib.rs"),
            "fn a() {}\nfn b() {}\nfn c() {}\n",
        )
        .unwrap();
        commit_all(dir.path(), "second");

        let report =
            diff(dir.path(), "HEAD~1", "HEAD", &[PathBuf::from("lib.rs")]).expect("diff succeeds");
        let file = &report.files[0];

        assert_eq!(file.old.as_ref().unwrap().lines, 1);
        assert_eq!(file.new.as_ref().unwrap().lines, 3);
        assert_eq!(
            delta::check(
                &report.files,
                &delta::DeltaOptions {
                    max_lines: Some(1),
                    ..delta::DeltaOptions::default()
                }
            )
            .len(),
            1
        );
    }
}
