//! All `git2` usage for the diff feature lives here: opening the repo,
//! resolving refs, and reading blob content at a given tree, so the rest of
//! the feature never touches `git2` types directly.

use std::fmt;
use std::path::{Path, PathBuf};

use git2::{Commit, ObjectType, Repository, Tree, TreeWalkMode, TreeWalkResult};

use crate::feature::complexity::router;

/// Opens the repository containing `repo_path`, walking up to find `.git`
/// like `git` itself does — `repo_path` doesn't need to be the repo root.
pub fn open(repo_path: &Path) -> Result<Repository, git2::Error> {
    Repository::discover(repo_path)
}

pub fn resolve_commit<'repo>(
    repo: &'repo Repository,
    reference: &str,
) -> Result<Commit<'repo>, git2::Error> {
    repo.revparse_single(reference)?.peel_to_commit()
}

#[derive(Debug)]
pub enum BlobError {
    Git(git2::Error),
    NotUtf8,
}

impl fmt::Display for BlobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlobError::Git(error) => write!(f, "{error}"),
            BlobError::NotUtf8 => write!(f, "blob is not valid UTF-8"),
        }
    }
}

/// `Ok(None)` means `path` doesn't exist at this tree's revision (or is a
/// directory, not a file) — not an error, since a path present in only one
/// of the two commits is the normal "file added" / "file removed" case.
pub fn blob_at(repo: &Repository, tree: &Tree, path: &Path) -> Result<Option<String>, BlobError> {
    let Ok(entry) = tree.get_path(path) else {
        return Ok(None);
    };
    let object = entry.to_object(repo).map_err(BlobError::Git)?;
    let Some(blob) = object.as_blob() else {
        return Ok(None);
    };
    std::str::from_utf8(blob.content())
        .map(|text| Some(text.to_string()))
        .map_err(|_| BlobError::NotUtf8)
}

/// Every supported-extension file reachable under any requested root in
/// either tree (unioned and deduplicated), plus the roots that matched
/// nothing in either tree.
pub fn discover_paths(
    old_tree: &Tree,
    new_tree: &Tree,
    roots: &[PathBuf],
) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut found = walk(old_tree);
    found.extend(walk(new_tree));
    found.sort();
    found.dedup();

    let mut matched = Vec::new();
    let mut unmatched_roots = Vec::new();
    for root in roots {
        let hits: Vec<PathBuf> = found
            .iter()
            .filter(|path| matches_root(path, root))
            .cloned()
            .collect();
        if hits.is_empty() {
            unmatched_roots.push(root.clone());
        } else {
            matched.extend(hits);
        }
    }
    matched.sort();
    matched.dedup();
    (matched, unmatched_roots)
}

/// `.` (the conventional "whole repo" root, and this subcommand's `--repo`
/// default) matches every discovered path — `Path::starts_with` treats `.`
/// as a literal leading component, so `"old.rs".starts_with(".")` is false
/// and would otherwise make the common "diff the whole repo" case match
/// nothing. Any other root matches by the usual path-prefix rule.
fn matches_root(path: &Path, root: &Path) -> bool {
    root.as_os_str() == "." || path.starts_with(root)
}

fn walk(tree: &Tree) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let _ = tree.walk(TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() != Some(ObjectType::Blob) {
            return TreeWalkResult::Ok;
        }
        if let Ok(name) = entry.name() {
            let path = Path::new(root).join(name);
            if router::is_supported(&path) {
                paths.push(path);
            }
        }
        TreeWalkResult::Ok
    });
    paths
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    /// A throwaway repo with two commits: `old.rs` present only at the first
    /// commit, `shared.rs` present (with different content) at both, and
    /// `new.rs` present only at the second.
    struct Fixture {
        dir: TempDir,
        repo: Repository,
        old_oid: git2::Oid,
        new_oid: git2::Oid,
    }

    fn commit(repo: &Repository, message: &str, parent: Option<git2::Oid>) -> git2::Oid {
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_oid = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_oid).unwrap();
        let parent_commit = parent.map(|oid| repo.find_commit(oid).unwrap());
        let parents: Vec<&Commit> = parent_commit.iter().collect();
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .unwrap()
    }

    fn write(dir: &Path, repo: &Repository, name: &str, content: &[u8]) {
        fs::write(dir.join(name), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
    }

    fn fixture() -> Fixture {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        write(dir.path(), &repo, "old.rs", b"fn old() {}");
        write(dir.path(), &repo, "shared.rs", b"fn shared() {}");
        let old_oid = commit(&repo, "first", None);

        fs::remove_file(dir.path().join("old.rs")).unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("old.rs")).unwrap();
        index.write().unwrap();
        write(
            dir.path(),
            &repo,
            "shared.rs",
            b"fn shared() { if true {} }",
        );
        write(dir.path(), &repo, "new.rs", b"fn new() {}");
        let new_oid = commit(&repo, "second", Some(old_oid));

        Fixture {
            dir,
            repo,
            old_oid,
            new_oid,
        }
    }

    impl Fixture {
        fn old_tree(&self) -> Tree<'_> {
            self.repo.find_commit(self.old_oid).unwrap().tree().unwrap()
        }

        fn new_tree(&self) -> Tree<'_> {
            self.repo.find_commit(self.new_oid).unwrap().tree().unwrap()
        }
    }

    #[test]
    fn open_discovers_repo_from_a_subdirectory() {
        let fixture = fixture();
        let sub = fixture.dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        assert!(open(&sub).is_ok());
    }

    #[test]
    fn resolve_commit_resolves_head() {
        let fixture = fixture();
        let commit = resolve_commit(&fixture.repo, "HEAD").unwrap();
        assert_eq!(commit.id(), fixture.new_oid);
    }

    #[test]
    fn resolve_commit_fails_for_unknown_ref() {
        let fixture = fixture();
        assert!(resolve_commit(&fixture.repo, "does-not-exist").is_err());
    }

    #[test]
    fn blob_at_reads_content_present_at_a_commit() {
        let fixture = fixture();
        let content = blob_at(&fixture.repo, &fixture.old_tree(), Path::new("old.rs"))
            .unwrap()
            .unwrap();
        assert_eq!(content, "fn old() {}");
    }

    #[test]
    fn blob_at_is_none_for_a_path_absent_at_that_revision() {
        let fixture = fixture();
        let result = blob_at(&fixture.repo, &fixture.new_tree(), Path::new("old.rs")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn blob_at_reports_non_utf8_content() {
        let fixture = fixture();
        write(
            fixture.dir.path(),
            &fixture.repo,
            "binary.rs",
            &[0xff, 0xfe, 0x00],
        );
        // Parent must be the current HEAD tip (new_oid) — git2 refuses to
        // update the HEAD ref via a commit whose first parent isn't already
        // its current target.
        let oid = commit(&fixture.repo, "binary", Some(fixture.new_oid));
        let tree = fixture.repo.find_commit(oid).unwrap().tree().unwrap();
        let result = blob_at(&fixture.repo, &tree, Path::new("binary.rs"));
        assert!(matches!(result, Err(BlobError::NotUtf8)));
    }

    #[test]
    fn discover_paths_unions_both_trees_under_a_root() {
        let fixture = fixture();
        let (matched, unmatched) = discover_paths(
            &fixture.old_tree(),
            &fixture.new_tree(),
            &[PathBuf::from(".")],
        );
        assert!(unmatched.is_empty());
        let mut names: Vec<String> = matched
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["new.rs", "old.rs", "shared.rs"]);
    }

    #[test]
    fn discover_paths_reports_an_unmatched_root() {
        let fixture = fixture();
        let (matched, unmatched) = discover_paths(
            &fixture.old_tree(),
            &fixture.new_tree(),
            &[PathBuf::from("does-not-exist.rs")],
        );
        assert!(matched.is_empty());
        assert_eq!(unmatched, vec![PathBuf::from("does-not-exist.rs")]);
    }
}
