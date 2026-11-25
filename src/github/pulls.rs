//! Pull requests through the GitHub REST API: list, read the conversation and checks, comment,
//! and create.

use serde::Deserialize;

use super::{agent, authorized, read};
use crate::constants::github::{API_BASE, API_TIMEOUT, MAX_PULLS};
use crate::models::{Checks, PullComment, PullDetail, PullList, PullRequest};

#[derive(Deserialize)]
struct User {
    login: String,
}

#[derive(Deserialize)]
struct RepoRef {
    full_name: String,
}

#[derive(Deserialize)]
struct Branch {
    #[serde(rename = "ref")]
    name: String,
    sha: String,
    /// `None` when the fork was deleted.
    repo: Option<RepoRef>,
}

#[derive(Deserialize)]
struct Pull {
    number: u64,
    title: String,
    body: Option<String>,
    user: User,
    head: Branch,
    base: Branch,
    #[serde(default)]
    draft: bool,
    html_url: String,
    created_at: String,
}

#[derive(Deserialize)]
struct Repository {
    default_branch: String,
}

#[derive(Deserialize)]
struct Comment {
    user: User,
    body: Option<String>,
    created_at: String,
}

#[derive(Deserialize)]
struct CheckRuns {
    check_runs: Vec<CheckRun>,
}

#[derive(Deserialize)]
struct CheckRun {
    status: String,
    conclusion: Option<String>,
}

/// `owner/name` of a github.com repository, from its web URL (see [`super::web_url`]).
pub fn slug(web_url: &str) -> Option<String> {
    let rest = web_url.split_once("github.com/")?.1;
    (rest.split('/').count() == 2).then(|| rest.to_owned())
}

pub fn list(token: &str, slug: &str) -> Result<PullList, String> {
    let agent = agent(API_TIMEOUT);
    let repository: Repository =
        read(authorized(agent.get(format!("{API_BASE}/repos/{slug}")), token).call())?;
    let url = format!("{API_BASE}/repos/{slug}/pulls?state=open&per_page={MAX_PULLS}");
    let pulls: Vec<Pull> = read(authorized(agent.get(url), token).call())?;
    let pulls = pulls
        .into_iter()
        .map(|pull| PullRequest {
            same_repository: pull.head.repo.as_ref().map(|repo| &repo.full_name)
                == pull.base.repo.as_ref().map(|repo| &repo.full_name),
            number: pull.number,
            title: pull.title,
            body: pull.body.unwrap_or_default(),
            author: pull.user.login,
            head_branch: pull.head.name,
            base_branch: pull.base.name,
            head_sha: pull.head.sha,
            draft: pull.draft,
            url: pull.html_url,
            created_at: pull.created_at,
        })
        .collect();
    Ok(PullList { pulls, default_branch: repository.default_branch })
}

/// The conversation (comments on the pull request) and the state of its checks.
pub fn detail(token: &str, slug: &str, number: u64, head_sha: &str) -> Result<PullDetail, String> {
    let agent = agent(API_TIMEOUT);
    let url = format!("{API_BASE}/repos/{slug}/issues/{number}/comments?per_page=100");
    let comments: Vec<Comment> = read(authorized(agent.get(url), token).call())?;
    let url = format!("{API_BASE}/repos/{slug}/commits/{head_sha}/check-runs?per_page=100");
    // Checks are extra information: a repository without them (or a token without access) still
    // shows the conversation.
    let runs: Vec<CheckRun> = read::<CheckRuns>(authorized(agent.get(url), token).call())
        .map(|runs| runs.check_runs)
        .unwrap_or_default();
    Ok(PullDetail {
        comments: comments
            .into_iter()
            .map(|comment| PullComment {
                author: comment.user.login,
                body: comment.body.unwrap_or_default(),
                created_at: comment.created_at,
            })
            .collect(),
        checks: summarize(&runs),
    })
}

fn summarize(runs: &[CheckRun]) -> Checks {
    let mut checks = Checks::default();
    for run in runs {
        match (run.status.as_str(), run.conclusion.as_deref()) {
            ("completed", Some("success" | "neutral" | "skipped")) => checks.passed += 1,
            ("completed", _) => checks.failed += 1,
            _ => checks.pending += 1,
        }
    }
    checks
}

pub fn comment(token: &str, slug: &str, number: u64, body: &str) -> Result<(), String> {
    let request = agent(API_TIMEOUT)
        .post(format!("{API_BASE}/repos/{slug}/issues/{number}/comments"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", "application/json");
    read::<serde_json::Value>(request.send(serde_json::json!({ "body": body }).to_string()))?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct NewPull {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
    pub draft: bool,
}

/// Creates a pull request and returns its number.
pub fn create(token: &str, slug: &str, pull: &NewPull) -> Result<u64, String> {
    #[derive(Deserialize)]
    struct Created {
        number: u64,
    }
    let request = agent(API_TIMEOUT)
        .post(format!("{API_BASE}/repos/{slug}/pulls"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", "application/json");
    let body = serde_json::json!({
        "title": pull.title, "body": pull.body, "head": pull.head, "base": pull.base, "draft": pull.draft,
    });
    let created: Created = read(request.send(body.to_string()))?;
    Ok(created.number)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_from_web_urls() {
        assert_eq!(slug("https://github.com/owner/repo").as_deref(), Some("owner/repo"));
        assert_eq!(slug("https://example.com/x"), None);
    }

    #[test]
    fn summarizes_check_runs() {
        let run = |status: &str, conclusion: Option<&str>| CheckRun {
            status: status.to_owned(),
            conclusion: conclusion.map(str::to_owned),
        };
        let runs = [
            run("completed", Some("success")),
            run("completed", Some("skipped")),
            run("completed", Some("failure")),
            run("in_progress", None),
        ];
        assert_eq!(summarize(&runs), Checks { passed: 2, failed: 1, pending: 1 });
    }

    #[test]
    fn reads_a_pull_from_github_json() {
        let json = r#"{"number": 7, "title": "Fix", "body": null, "user": {"login": "sam"},
            "head": {"ref": "fix", "sha": "abc", "repo": {"full_name": "fork/r"}},
            "base": {"ref": "main", "sha": "def", "repo": {"full_name": "o/r"}},
            "draft": true, "html_url": "https://github.com/o/r/pull/7", "created_at": "2026-11-24T10:00:00Z"}"#;
        let pull: Pull = serde_json::from_str(json).unwrap();
        assert_eq!(pull.head.name, "fix");
        assert!(pull.draft);
    }
}
