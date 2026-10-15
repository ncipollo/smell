//! Parses a `pr` subcommand reference into either an explicit `<old>..<new>`
//! commit range or a GitHub PR reference (a full URL, `owner/repo#123`, or a
//! bare number).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRef {
    pub owner: String,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PrReference {
    pub repo: Option<RepoRef>,
    pub number: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PrSource {
    ExplicitRange { old: String, new: String },
    Reference(PrReference),
}

#[derive(Debug)]
pub struct ParseError(String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parses the `pr` subcommand's single positional argument.
pub fn parse(reference: &str) -> Result<PrSource, ParseError> {
    if let Some((old, new)) = reference.split_once("..") {
        return Ok(PrSource::ExplicitRange {
            old: old.to_string(),
            new: new.to_string(),
        });
    }
    parse_pr_reference(reference).map(PrSource::Reference)
}

fn parse_pr_reference(reference: &str) -> Result<PrReference, ParseError> {
    if let Some(parsed) = parse_url(reference).or_else(|| parse_owner_repo_number(reference)) {
        return Ok(parsed);
    }
    parse_bare_number(reference)
}

/// `https://github.com/owner/repo/pull/123`, ignoring any trailing path
/// segments (e.g. `/files`).
fn parse_url(reference: &str) -> Option<PrReference> {
    let path = reference
        .strip_prefix("https://github.com/")
        .or_else(|| reference.strip_prefix("http://github.com/"))?;
    let mut segments = path.split('/');
    let owner = segments.next()?;
    let name = segments.next()?;
    let marker = segments.next()?;
    let number = segments.next()?;
    if marker != "pull" || owner.is_empty() || name.is_empty() {
        return None;
    }
    Some(PrReference {
        repo: Some(RepoRef {
            owner: owner.to_string(),
            name: name.to_string(),
        }),
        number: number.parse().ok()?,
    })
}

fn parse_owner_repo_number(reference: &str) -> Option<PrReference> {
    let (repo_part, number_part) = reference.split_once('#')?;
    let (owner, name) = repo_part.split_once('/')?;
    if owner.is_empty() || name.is_empty() {
        return None;
    }
    Some(PrReference {
        repo: Some(RepoRef {
            owner: owner.to_string(),
            name: name.to_string(),
        }),
        number: number_part.parse().ok()?,
    })
}

fn parse_bare_number(reference: &str) -> Result<PrReference, ParseError> {
    reference
        .parse()
        .map(|number| PrReference { repo: None, number })
        .map_err(|_| {
            ParseError(format!(
                "not a commit range, PR URL, owner/repo#N, or PR number: \"{reference}\""
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_explicit_range() {
        let source = parse("HEAD~1..HEAD").unwrap();
        assert_eq!(
            source,
            PrSource::ExplicitRange {
                old: "HEAD~1".to_string(),
                new: "HEAD".to_string(),
            }
        );
    }

    #[test]
    fn parses_a_sha_range() {
        let source = parse("abc123..def456").unwrap();
        assert_eq!(
            source,
            PrSource::ExplicitRange {
                old: "abc123".to_string(),
                new: "def456".to_string(),
            }
        );
    }

    #[test]
    fn parses_a_pull_request_url() {
        let source = parse("https://github.com/ncipollo/smell/pull/49").unwrap();
        let PrSource::Reference(reference) = source else {
            panic!("expected a PR reference");
        };
        assert_eq!(reference.number, 49);
        let repo = reference.repo.unwrap();
        assert_eq!(repo.owner, "ncipollo");
        assert_eq!(repo.name, "smell");
    }

    #[test]
    fn parses_a_pull_request_url_with_trailing_path() {
        let source = parse("https://github.com/ncipollo/smell/pull/49/files").unwrap();
        let PrSource::Reference(reference) = source else {
            panic!("expected a PR reference");
        };
        assert_eq!(reference.number, 49);
    }

    #[test]
    fn parses_owner_repo_hash_number() {
        let source = parse("ncipollo/smell#49").unwrap();
        let PrSource::Reference(reference) = source else {
            panic!("expected a PR reference");
        };
        assert_eq!(reference.number, 49);
        let repo = reference.repo.unwrap();
        assert_eq!(repo.owner, "ncipollo");
        assert_eq!(repo.name, "smell");
    }

    #[test]
    fn parses_a_bare_number() {
        let source = parse("49").unwrap();
        let PrSource::Reference(reference) = source else {
            panic!("expected a PR reference");
        };
        assert_eq!(reference.number, 49);
        assert!(reference.repo.is_none());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("not-a-reference!!").is_err());
    }

    #[test]
    fn rejects_a_non_github_url() {
        assert!(parse("https://example.com/owner/repo/pull/49").is_err());
    }
}
