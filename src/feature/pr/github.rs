//! Resolves a GitHub pull request's base/head commit SHAs via the GitHub
//! REST API, authenticating the way `gh` itself would: the `GITHUB_TOKEN`,
//! `GH_TOKEN`, `GITHUB_ACCESS_TOKEN` env vars (same set/order as
//! `ncipollo/git-ops`'s HTTPS auth), then `gh auth token`, then
//! unauthenticated — which works for public repos within GitHub's
//! unauthenticated rate limit.

use std::fmt;
use std::process::Command;

use serde::Deserialize;

const TOKEN_ENV_VARS: [&str; 3] = ["GITHUB_TOKEN", "GH_TOKEN", "GITHUB_ACCESS_TOKEN"];

pub struct ShaPair {
    pub base: String,
    pub head: String,
}

#[derive(Debug)]
pub enum GitHubError {
    AuthRequired,
    NotFound,
    Status(u16),
    Request(String),
    Decode(String),
}

impl fmt::Display for GitHubError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitHubError::AuthRequired => write!(
                f,
                "GitHub API requires authentication for this PR; run `gh auth login` or set GH_TOKEN/GITHUB_TOKEN"
            ),
            GitHubError::NotFound => write!(f, "pull request not found"),
            GitHubError::Status(code) => write!(f, "GitHub API returned HTTP {code}"),
            GitHubError::Request(message) => write!(f, "GitHub API request failed: {message}"),
            GitHubError::Decode(message) => write!(f, "decoding GitHub API response: {message}"),
        }
    }
}

#[derive(Deserialize)]
struct PullResponse {
    base: CommitRef,
    head: CommitRef,
}

#[derive(Deserialize)]
struct CommitRef {
    sha: String,
}

/// Resolves `owner/repo`'s pull request `number` to its base and head commit
/// SHAs.
pub fn resolve_pr(owner: &str, repo: &str, number: u64) -> Result<ShaPair, GitHubError> {
    let token = resolve_token();
    let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls/{number}");
    let client = reqwest::blocking::Client::new();
    let mut request = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "smell-cli");
    if let Some(token) = &token {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let response = request
        .send()
        .map_err(|error| GitHubError::Request(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(status_error(status.as_u16(), token.is_some()));
    }
    let body = response
        .text()
        .map_err(|error| GitHubError::Request(error.to_string()))?;
    parse_response(&body)
}

fn parse_response(body: &str) -> Result<ShaPair, GitHubError> {
    let parsed: PullResponse =
        serde_json::from_str(body).map_err(|error| GitHubError::Decode(error.to_string()))?;
    Ok(ShaPair {
        base: parsed.base.sha,
        head: parsed.head.sha,
    })
}

/// A 401/403 only means "you need to authenticate" when we didn't already
/// send a token — with one sent, it's some other access problem.
fn status_error(code: u16, had_token: bool) -> GitHubError {
    match code {
        404 => GitHubError::NotFound,
        401 | 403 if !had_token => GitHubError::AuthRequired,
        _ => GitHubError::Status(code),
    }
}

fn resolve_token() -> Option<String> {
    token_from_env(|name| std::env::var(name).ok()).or_else(gh_auth_token)
}

fn token_from_env(lookup: impl Fn(&str) -> Option<String>) -> Option<String> {
    TOKEN_ENV_VARS
        .iter()
        .find_map(|name| lookup(name).filter(|value| !value.is_empty()))
}

fn gh_auth_token() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8(output.stdout).ok()?;
    let token = token.trim();
    (!token.is_empty()).then(|| token.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_from_env_prefers_github_token() {
        let lookup = |name: &str| match name {
            "GITHUB_TOKEN" => Some("github-token".to_string()),
            "GH_TOKEN" => Some("gh-cli-token".to_string()),
            _ => None,
        };
        assert_eq!(token_from_env(lookup), Some("github-token".to_string()));
    }

    #[test]
    fn token_from_env_falls_back_through_the_precedence_order() {
        let lookup = |name: &str| (name == "GITHUB_ACCESS_TOKEN").then(|| "fallback".to_string());
        assert_eq!(token_from_env(lookup), Some("fallback".to_string()));
    }

    #[test]
    fn token_from_env_skips_empty_values() {
        let lookup = |name: &str| match name {
            "GITHUB_TOKEN" => Some(String::new()),
            "GH_TOKEN" => Some("real".to_string()),
            _ => None,
        };
        assert_eq!(token_from_env(lookup), Some("real".to_string()));
    }

    #[test]
    fn token_from_env_is_none_when_nothing_is_set() {
        assert_eq!(token_from_env(|_| None), None);
    }

    #[test]
    fn status_error_maps_404_to_not_found() {
        assert!(matches!(status_error(404, false), GitHubError::NotFound));
    }

    #[test]
    fn status_error_maps_401_without_a_token_to_auth_required() {
        assert!(matches!(
            status_error(401, false),
            GitHubError::AuthRequired
        ));
    }

    #[test]
    fn status_error_maps_403_with_a_token_to_a_plain_status() {
        assert!(matches!(status_error(403, true), GitHubError::Status(403)));
    }

    #[test]
    fn parse_response_extracts_base_and_head_shas() {
        let body = r#"{"base":{"sha":"aaa"},"head":{"sha":"bbb"}}"#;
        let pair = parse_response(body).unwrap();
        assert_eq!(pair.base, "aaa");
        assert_eq!(pair.head, "bbb");
    }

    #[test]
    fn parse_response_errors_on_malformed_json() {
        assert!(parse_response("not json").is_err());
    }
}
