//! Resolves a `pr` subcommand reference — an explicit `<old>..<new>` commit
//! range, or a GitHub PR reference — down to the two commit refs the diff
//! engine needs. A resolution layer in front of [`crate::feature::diff`],
//! not a second diff implementation.

use std::fmt;
use std::path::Path;

mod fetch;
mod github;
mod reference;
mod remote;

use reference::{PrReference, PrSource, RepoRef};

#[derive(Debug)]
pub enum PrError {
    Parse(reference::ParseError),
    Remote(remote::RemoteError),
    GitHub(github::GitHubError),
    Fetch(fetch::FetchError),
}

impl fmt::Display for PrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrError::Parse(error) => write!(f, "{error}"),
            PrError::Remote(error) => write!(f, "{error}"),
            PrError::GitHub(error) => write!(f, "{error}"),
            PrError::Fetch(error) => write!(f, "{error}"),
        }
    }
}

/// Parses `reference` and resolves it to an `(old_ref, new_ref)` pair ready
/// for [`crate::feature::diff::diff`], fetching any PR commit not yet
/// present in the repository at `repo_path`.
pub fn resolve(repo_path: &Path, reference: &str) -> Result<(String, String), PrError> {
    match reference::parse(reference).map_err(PrError::Parse)? {
        PrSource::ExplicitRange { old, new } => Ok((old, new)),
        PrSource::Reference(pr_reference) => resolve_pr_reference(repo_path, &pr_reference),
    }
}

fn resolve_pr_reference(
    repo_path: &Path,
    pr_reference: &PrReference,
) -> Result<(String, String), PrError> {
    let repo_ref = repo_ref_for(repo_path, pr_reference)?;
    let shas = github::resolve_pr(&repo_ref.owner, &repo_ref.name, pr_reference.number)
        .map_err(PrError::GitHub)?;
    fetch::ensure_present(repo_path, &repo_ref, &shas.base).map_err(PrError::Fetch)?;
    fetch::ensure_present(repo_path, &repo_ref, &shas.head).map_err(PrError::Fetch)?;
    Ok((shas.base, shas.head))
}

fn repo_ref_for(repo_path: &Path, pr_reference: &PrReference) -> Result<RepoRef, PrError> {
    match &pr_reference.repo {
        Some(repo_ref) => Ok(repo_ref.clone()),
        None => remote::origin_repo(repo_path).map_err(PrError::Remote),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_forwards_an_explicit_range_without_touching_github() {
        let (old, new) = resolve(Path::new("."), "HEAD~1..HEAD").unwrap();
        assert_eq!(old, "HEAD~1");
        assert_eq!(new, "HEAD");
    }

    #[test]
    fn resolve_rejects_a_bad_reference() {
        assert!(resolve(Path::new("."), "not-a-reference!!").is_err());
    }
}
