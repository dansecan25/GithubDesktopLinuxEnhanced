//! GitHub's REST API. One shared HTTP agent, and one function per call the app makes.

use std::time::Duration;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::constants::app::{APP_ID, VERSION};
use crate::constants::github::{API_BASE, API_TIMEOUT, CONNECT_TIMEOUT, GITHUB_HOST};

/// The github.com page of a repository, from its remote URL (HTTPS or SSH). `None` for other hosts.
pub fn web_url(remote_url: &str) -> Option<String> {
    let url = remote_url.trim().trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    let path = if let Some(rest) = url.strip_prefix("git@github.com:") {
        rest
    } else {
        let without_scheme = url.split_once("://").map(|(_, rest)| rest)?;
        // Drops `user@` or `user:token@` before the host.
        let host_and_path = without_scheme.rsplit_once('@').map_or(without_scheme, |(_, rest)| rest);
        let (host, path) = host_and_path.split_once('/')?;
        if !host.eq_ignore_ascii_case(GITHUB_HOST) && host != "ssh.github.com:443" {
            return None;
        }
        path
    };
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let (owner, name) = (parts.next()?, parts.next()?);
    parts.next().is_none().then(|| format!("https://{GITHUB_HOST}/{owner}/{name}"))
}

/// The github.com page of a branch.
pub fn branch_url(repository_url: &str, branch: &str) -> String {
    let encoded: String = branch
        .chars()
        .map(|c| match c {
            '#' | '%' | '?' | ' ' | '&' => format!("%{:02X}", c as u32),
            c => c.to_string(),
        })
        .collect();
    format!("{repository_url}/tree/{encoded}")
}

pub fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(format!("{APP_ID}/{VERSION}"))
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(timeout))
        // Error statuses are read as bodies, so GitHub's own message can be shown.
        .http_status_as_error(false)
        .build()
        .new_agent()
}

#[derive(Deserialize)]
struct ErrorBody {
    message: String,
    #[serde(default)]
    errors: Vec<ErrorDetail>,
}

#[derive(Deserialize)]
struct ErrorDetail {
    #[serde(default)]
    message: Option<String>,
}

fn read<T: DeserializeOwned>(
    response: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<T, String> {
    let mut response = response.map_err(|error| format!("Could not reach GitHub: {error}"))?;
    let status = response.status();
    let text = response
        .body_mut()
        .read_to_string()
        .map_err(|error| format!("Could not read GitHub's answer: {error}"))?;
    if status.is_success() {
        return serde_json::from_str(&text)
            .map_err(|error| format!("Unexpected answer from GitHub: {error}"));
    }
    if status == 401 {
        return Err("GitHub rejected the token. Check it hasn't expired, and sign in again.".to_owned());
    }
    let message = serde_json::from_str::<ErrorBody>(&text)
        .map(|body| {
            let details: Vec<String> = body.errors.into_iter().filter_map(|detail| detail.message).collect();
            if details.is_empty() {
                body.message
            } else {
                format!("{}: {}", body.message, details.join(", "))
            }
        })
        .unwrap_or_else(|_| format!("GitHub answered with status {status}"));
    Err(message)
}

fn authorized(
    request: ureq::RequestBuilder<ureq::typestate::WithoutBody>,
    token: &str,
) -> ureq::RequestBuilder<ureq::typestate::WithoutBody> {
    request.header("Authorization", format!("Bearer {token}")).header("Accept", "application/vnd.github+json")
}

#[derive(Deserialize)]
struct User {
    login: String,
}

/// Checks a token and returns the account's login name.
pub fn login_for_token(token: &str) -> Result<String, String> {
    let agent = agent(API_TIMEOUT);
    let user: User = read(authorized(agent.get(format!("{API_BASE}/user")), token).call())?;
    Ok(user.login)
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatedRepository {
    pub clone_url: String,
    pub html_url: String,
}

/// Creates a repository on the signed-in account.
pub fn create_repository(
    token: &str,
    name: &str,
    description: &str,
    private: bool,
) -> Result<CreatedRepository, String> {
    let agent = agent(API_TIMEOUT);
    let body = serde_json::json!({ "name": name, "description": description, "private": private });
    let request = agent
        .post(format!("{API_BASE}/user/repos"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", "application/json");
    read(request.send(body.to_string()))
}

#[derive(Debug, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// The newest published (not draft) release of `repository` (`owner/name`). No token needed.
pub fn latest_release(repository: &str) -> Result<Release, String> {
    let agent = agent(API_TIMEOUT);
    read(
        agent
            .get(format!("{API_BASE}/repos/{repository}/releases/latest"))
            .header("Accept", "application/vnd.github+json")
            .call(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_urls_from_remote_urls() {
        let expected = Some("https://github.com/owner/repo".to_owned());
        assert_eq!(web_url("https://github.com/owner/repo.git"), expected);
        assert_eq!(web_url("https://user:token@github.com/owner/repo"), expected);
        assert_eq!(web_url("git@github.com:owner/repo.git"), expected);
        assert_eq!(web_url("ssh://git@github.com/owner/repo.git/"), expected);
        assert_eq!(web_url("https://gitlab.com/owner/repo.git"), None);
        assert_eq!(web_url("/home/me/repo"), None);
    }

    #[test]
    fn branch_urls_escape_special_characters() {
        let base = "https://github.com/o/r";
        assert_eq!(branch_url(base, "feature/login"), "https://github.com/o/r/tree/feature/login");
        assert_eq!(branch_url(base, "fix#12"), "https://github.com/o/r/tree/fix%2312");
    }
}
