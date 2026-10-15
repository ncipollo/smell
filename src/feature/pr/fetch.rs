//! Ensures a commit resolved from a PR (its base or head) is present in the
//! local repository's object database, fetching it by SHA from GitHub when
//! it isn't — needed for a PR head on a fork the local clone doesn't know
//! about. GitHub's server allows fetching a commit directly by its full SHA
//! even when it isn't an advertised ref.

use std::fmt;
use std::path::Path;

use git2::Repository;

use super::reference::RepoRef;

#[derive(Debug)]
pub struct FetchError(String);

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Fetches `sha` from `repo_ref`'s GitHub clone URL into the repo at
/// `repo_path`, unless it's already present locally.
pub fn ensure_present(repo_path: &Path, repo_ref: &RepoRef, sha: &str) -> Result<(), FetchError> {
    let repo = Repository::discover(repo_path)
        .map_err(|error| FetchError(format!("opening repo at {}: {error}", repo_path.display())))?;
    if commit_exists(&repo, sha) {
        return Ok(());
    }
    fetch_from(&repo, &clone_url(repo_ref), sha)
}

fn clone_url(repo_ref: &RepoRef) -> String {
    format!(
        "https://github.com/{}/{}.git",
        repo_ref.owner, repo_ref.name
    )
}

fn commit_exists(repo: &Repository, sha: &str) -> bool {
    repo.revparse_single(sha)
        .and_then(|object| object.peel_to_commit())
        .is_ok()
}

fn fetch_from(repo: &Repository, url: &str, sha: &str) -> Result<(), FetchError> {
    let mut remote = repo
        .remote_anonymous(url)
        .map_err(|error| FetchError(format!("creating remote for {url}: {error}")))?;
    let refspec = format!("+{sha}:refs/tmp/smell/pr/{sha}");
    remote
        .fetch(&[refspec.as_str()], None, None)
        .map_err(|error| FetchError(format!("fetching {sha} from {url}: {error}")))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use git2::Repository;
    use tempfile::TempDir;

    use super::*;

    fn commit(repo: &Repository, message: &str) -> git2::Oid {
        let signature = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_oid = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
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
        .unwrap()
    }

    #[test]
    fn clone_url_builds_a_github_https_url() {
        let repo_ref = RepoRef {
            owner: "ncipollo".to_string(),
            name: "smell".to_string(),
        };
        assert_eq!(
            clone_url(&repo_ref),
            "https://github.com/ncipollo/smell.git"
        );
    }

    #[test]
    fn commit_exists_is_false_for_an_absent_sha() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        commit(&repo, "first");
        assert!(!commit_exists(
            &repo,
            "0000000000000000000000000000000000dead"
        ));
    }

    #[test]
    fn ensure_present_is_a_no_op_for_a_locally_known_sha() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let oid = commit(&repo, "first");
        let repo_ref = RepoRef {
            owner: "does-not".to_string(),
            name: "matter".to_string(),
        };
        // No fetch happens: the SHA is already present locally, so a
        // (deliberately bogus) `repo_ref` never gets used as a clone URL.
        assert!(ensure_present(dir.path(), &repo_ref, &oid.to_string()).is_ok());
    }

    #[test]
    fn ensure_present_errors_outside_a_repo() {
        let dir = TempDir::new().unwrap();
        let repo_ref = RepoRef {
            owner: "a".to_string(),
            name: "b".to_string(),
        };
        assert!(ensure_present(Path::new(dir.path()), &repo_ref, "deadbeef").is_err());
    }

    #[test]
    fn fetch_from_pulls_a_commit_from_a_file_remote_by_sha() {
        let source_dir = TempDir::new().unwrap();
        let source_repo = Repository::init(source_dir.path()).unwrap();
        let oid = commit(&source_repo, "fork commit");

        let target_dir = TempDir::new().unwrap();
        let target_repo = Repository::init(target_dir.path()).unwrap();
        commit(&target_repo, "unrelated");
        assert!(!commit_exists(&target_repo, &oid.to_string()));

        let url = format!("file://{}", source_dir.path().display());
        fetch_from(&target_repo, &url, &oid.to_string()).unwrap();

        assert!(commit_exists(&target_repo, &oid.to_string()));
    }
}
