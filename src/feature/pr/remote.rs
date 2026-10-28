//! Resolves the local repository's `origin` remote to a GitHub `owner/repo`
//! pair, for a bare PR number with no explicit repo in the reference.

use std::fmt;
use std::path::Path;

use git2::Repository;

use super::REPO_HINT;
use super::reference::RepoRef;

#[derive(Debug)]
pub struct RemoteError(String);

impl fmt::Display for RemoteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Opens the repo at or above `repo_path` and parses its `origin` remote URL
/// into an `owner/repo` pair.
pub fn origin_repo(repo_path: &Path) -> Result<RepoRef, RemoteError> {
    let repo = Repository::discover(repo_path).map_err(|error| {
        RemoteError(format!(
            "not inside a git repository ({}): {error}; {REPO_HINT}",
            repo_path.display()
        ))
    })?;
    let origin = repo.find_remote("origin").map_err(|error| {
        RemoteError(format!(
            "no \"origin\" remote in the repository at {}: {error}; {REPO_HINT}",
            repo_path.display()
        ))
    })?;
    let url = origin.url().map_err(|error| {
        RemoteError(format!(
            "\"origin\" remote has no URL: {error}; {REPO_HINT}"
        ))
    })?;
    parse_github_remote(url).ok_or_else(|| {
        RemoteError(format!(
            "\"origin\" (\"{url}\") is not a GitHub remote; {REPO_HINT}"
        ))
    })
}

/// Parses `https://github.com/owner/repo.git` and `git@github.com:owner/repo.git`
/// forms (with or without a trailing `.git`).
fn parse_github_remote(url: &str) -> Option<RepoRef> {
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("git@github.com:"))?;
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, name) = path.split_once('/')?;
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    Some(RepoRef {
        owner: owner.to_string(),
        name: name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use git2::Repository;
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn parses_an_https_remote() {
        let repo = parse_github_remote("https://github.com/ncipollo/smell.git").unwrap();
        assert_eq!(repo.owner, "ncipollo");
        assert_eq!(repo.name, "smell");
    }

    #[test]
    fn parses_an_https_remote_without_a_git_suffix() {
        let repo = parse_github_remote("https://github.com/ncipollo/smell").unwrap();
        assert_eq!(repo.owner, "ncipollo");
        assert_eq!(repo.name, "smell");
    }

    #[test]
    fn parses_an_ssh_remote() {
        let repo = parse_github_remote("git@github.com:ncipollo/smell.git").unwrap();
        assert_eq!(repo.owner, "ncipollo");
        assert_eq!(repo.name, "smell");
    }

    #[test]
    fn rejects_a_non_github_remote() {
        assert!(parse_github_remote("https://gitlab.com/ncipollo/smell.git").is_none());
    }

    #[test]
    fn origin_repo_resolves_from_a_real_remote() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo.remote("origin", "https://github.com/ncipollo/smell.git")
            .unwrap();
        let resolved = origin_repo(dir.path()).unwrap();
        assert_eq!(resolved.owner, "ncipollo");
        assert_eq!(resolved.name, "smell");
    }

    #[test]
    fn origin_repo_errors_without_an_origin_remote() {
        let dir = TempDir::new().unwrap();
        Repository::init(dir.path()).unwrap();
        let error = origin_repo(dir.path()).unwrap_err();
        assert!(error.to_string().contains("--repo"));
    }

    #[test]
    fn origin_repo_errors_outside_a_repo() {
        let dir = TempDir::new().unwrap();
        let error = origin_repo(Path::new(dir.path())).unwrap_err();
        assert!(error.to_string().contains("--repo"));
    }

    #[test]
    fn origin_repo_errors_for_a_non_github_origin() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo.remote("origin", "https://gitlab.com/a/b.git").unwrap();
        let error = origin_repo(dir.path()).unwrap_err();
        assert!(error.to_string().contains("--repo"));
    }
}
