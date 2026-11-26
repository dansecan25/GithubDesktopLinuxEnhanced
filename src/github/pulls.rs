//! Pull requests through the GitHub REST API: list, read the conversation, files, line comments and
//! checks, comment, review, merge, edit and create. Changing the draft state needs GraphQL.

use serde::Deserialize;
use serde::de::DeserializeOwned;

use super::{agent, authorized, read};
use crate::constants::github::{API_BASE, API_TIMEOUT, MAX_PULL_PAGES, MAX_PULLS, PER_PAGE};
use crate::git::parse::parse_diff;
use crate::models::{
    ChangeKind, Checks, DiffSide, FileChange, LineTarget, MergeMethod, Mergeability, PullComment, PullDetail,
    PullFile, PullList, PullRequest, ReviewComment, ReviewEvent,
};

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
    // Only shown to people who can push; GitHub's defaults allow all three.
    #[serde(default = "yes")]
    allow_merge_commit: bool,
    #[serde(default = "yes")]
    allow_squash_merge: bool,
    #[serde(default = "yes")]
    allow_rebase_merge: bool,
}

fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct Comment {
    user: User,
    body: Option<String>,
    created_at: String,
}

#[derive(Deserialize)]
struct Review {
    user: User,
    body: Option<String>,
    state: String,
    submitted_at: Option<String>,
}

#[derive(Deserialize)]
struct LineComment {
    id: u64,
    in_reply_to_id: Option<u64>,
    path: String,
    line: Option<u32>,
    side: Option<String>,
    user: User,
    body: String,
    created_at: String,
}

#[derive(Deserialize)]
struct File {
    filename: String,
    previous_filename: Option<String>,
    status: String,
    #[serde(default)]
    additions: u32,
    #[serde(default)]
    deletions: u32,
    patch: Option<String>,
}

#[derive(Deserialize)]
struct PullState {
    node_id: String,
    mergeable: Option<bool>,
    #[serde(default)]
    mergeable_state: String,
    #[serde(default)]
    merged: bool,
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
    let merge_methods = [
        (repository.allow_merge_commit, MergeMethod::Merge),
        (repository.allow_squash_merge, MergeMethod::Squash),
        (repository.allow_rebase_merge, MergeMethod::Rebase),
    ]
    .into_iter()
    .filter_map(|(allowed, method)| allowed.then_some(method))
    .collect();
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
    Ok(PullList { pulls, default_branch: repository.default_branch, merge_methods })
}

/// Every page of a list (`url` already has a `?`), up to `MAX_PULL_PAGES` pages.
fn all_pages<T: DeserializeOwned>(agent: &ureq::Agent, token: &str, url: &str) -> Result<Vec<T>, String> {
    let mut items = Vec::new();
    for page in 1..=MAX_PULL_PAGES {
        let batch: Vec<T> =
            read(authorized(agent.get(format!("{url}&per_page={PER_PAGE}&page={page}")), token).call())?;
        let last = batch.len() < PER_PAGE as usize;
        items.extend(batch);
        if last {
            break;
        }
    }
    Ok(items)
}

/// The conversation (comments and reviews), the changed files with their line comments, whether it
/// can be merged, and the state of its checks.
pub fn detail(token: &str, slug: &str, number: u64, head_sha: &str) -> Result<PullDetail, String> {
    let agent = agent(API_TIMEOUT);
    let pull_url = format!("{API_BASE}/repos/{slug}/pulls/{number}");
    let state: PullState = read(authorized(agent.get(&pull_url), token).call())?;
    let comments: Vec<Comment> =
        all_pages(&agent, token, &format!("{API_BASE}/repos/{slug}/issues/{number}/comments?"))?;
    let reviews: Vec<Review> = all_pages(&agent, token, &format!("{pull_url}/reviews?"))?;
    let line_comments: Vec<LineComment> = all_pages(&agent, token, &format!("{pull_url}/comments?"))?;
    let files: Vec<File> = all_pages(&agent, token, &format!("{pull_url}/files?"))?;
    let url = format!("{API_BASE}/repos/{slug}/commits/{head_sha}/check-runs?per_page=100");
    // Checks are extra information: a repository without them (or a token without access) still
    // shows the conversation.
    let runs: Vec<CheckRun> = read::<CheckRuns>(authorized(agent.get(url), token).call())
        .map(|runs| runs.check_runs)
        .unwrap_or_default();
    Ok(PullDetail {
        comments: conversation(comments, reviews),
        checks: summarize(&runs),
        files: files.into_iter().map(pull_file).collect(),
        review_comments: line_comments.into_iter().map(review_comment).collect(),
        mergeability: Mergeability {
            mergeable: state.mergeable,
            state: state.mergeable_state,
            merged: state.merged,
        },
        node_id: state.node_id,
    })
}

/// Comments and reviews in one timeline, oldest first. Reviews without a message or verdict (the
/// container of line comments) are left out.
fn conversation(comments: Vec<Comment>, reviews: Vec<Review>) -> Vec<PullComment> {
    let mut timeline: Vec<PullComment> = comments
        .into_iter()
        .map(|comment| PullComment {
            author: comment.user.login,
            body: comment.body.unwrap_or_default(),
            created_at: comment.created_at,
            verdict: None,
        })
        .collect();
    for review in reviews {
        let body = review.body.unwrap_or_default();
        let verdict = match review.state.as_str() {
            "APPROVED" => crate::constants::text::REVIEW_APPROVED,
            "CHANGES_REQUESTED" => crate::constants::text::REVIEW_CHANGES_REQUESTED,
            "COMMENTED" if !body.trim().is_empty() => crate::constants::text::REVIEW_COMMENTED,
            // Pending (not submitted), dismissed, or only line comments.
            _ => continue,
        };
        timeline.push(PullComment {
            author: review.user.login,
            body,
            created_at: review.submitted_at.unwrap_or_default(),
            verdict: Some(verdict.to_owned()),
        });
    }
    // ISO 8601 timestamps sort as text.
    timeline.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    timeline
}

fn pull_file(file: File) -> PullFile {
    let kind = match file.status.as_str() {
        "added" => ChangeKind::Added,
        "removed" => ChangeKind::Deleted,
        "renamed" => ChangeKind::Renamed,
        "copied" => ChangeKind::Copied,
        _ => ChangeKind::Modified,
    };
    let diff_missing = file.patch.is_none();
    PullFile {
        change: FileChange { path: file.filename, original_path: file.previous_filename, kind },
        additions: file.additions,
        deletions: file.deletions,
        diff: file.patch.as_deref().map(parse_diff).unwrap_or_default(),
        diff_missing,
    }
}

fn review_comment(comment: LineComment) -> ReviewComment {
    ReviewComment {
        id: comment.id,
        reply_to: comment.in_reply_to_id,
        path: comment.path,
        line: comment.line,
        side: if comment.side.as_deref() == Some("LEFT") { DiffSide::Left } else { DiffSide::Right },
        author: comment.user.login,
        body: comment.body,
        created_at: comment.created_at,
    }
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

/// Sends JSON with `method` (POST, PUT, PATCH) and returns GitHub's answer.
fn send(token: &str, method: &str, url: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let agent = agent(API_TIMEOUT);
    let request = match method {
        "PUT" => agent.put(url),
        "PATCH" => agent.patch(url),
        _ => agent.post(url),
    };
    let request = request
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", "application/json");
    read(request.send(body.to_string()))
}

pub fn comment(token: &str, slug: &str, number: u64, body: &str) -> Result<(), String> {
    let url = format!("{API_BASE}/repos/{slug}/issues/{number}/comments");
    send(token, "POST", &url, &serde_json::json!({ "body": body }))?;
    Ok(())
}

/// A comment on one line of the diff, or an answer in that line's thread.
pub fn line_comment(
    token: &str,
    slug: &str,
    number: u64,
    head_sha: &str,
    target: &LineTarget,
    body: &str,
) -> Result<(), String> {
    let pull_url = format!("{API_BASE}/repos/{slug}/pulls/{number}");
    match target.reply_to {
        Some(id) => send(
            token,
            "POST",
            &format!("{pull_url}/comments/{id}/replies"),
            &serde_json::json!({ "body": body }),
        )?,
        None => send(
            token,
            "POST",
            &format!("{pull_url}/comments"),
            &serde_json::json!({
                "body": body, "commit_id": head_sha, "path": target.path, "line": target.line,
                "side": target.side.api_name(),
            }),
        )?,
    };
    Ok(())
}

/// Submits a review: approve, request changes, or comment.
pub fn review(
    token: &str,
    slug: &str,
    number: u64,
    head_sha: &str,
    event: ReviewEvent,
    body: &str,
) -> Result<(), String> {
    let url = format!("{API_BASE}/repos/{slug}/pulls/{number}/reviews");
    let mut request = serde_json::json!({ "event": event.api_name(), "commit_id": head_sha });
    if !body.trim().is_empty() {
        request["body"] = serde_json::Value::String(body.to_owned());
    }
    send(token, "POST", &url, &request)?;
    Ok(())
}

/// Merges on GitHub. `head_sha` makes GitHub refuse if new commits arrived meanwhile.
pub fn merge(
    token: &str,
    slug: &str,
    number: u64,
    head_sha: &str,
    method: MergeMethod,
) -> Result<(), String> {
    let method = match method {
        MergeMethod::Merge => "merge",
        MergeMethod::Squash => "squash",
        MergeMethod::Rebase => "rebase",
    };
    let url = format!("{API_BASE}/repos/{slug}/pulls/{number}/merge");
    let answer = send(token, "PUT", &url, &serde_json::json!({ "merge_method": method, "sha": head_sha }))?;
    if answer["merged"].as_bool() == Some(false) {
        return Err(answer["message"].as_str().unwrap_or(crate::constants::text::MERGE_REFUSED).to_owned());
    }
    Ok(())
}

pub fn edit(token: &str, slug: &str, number: u64, title: &str, body: &str) -> Result<(), String> {
    let url = format!("{API_BASE}/repos/{slug}/pulls/{number}");
    send(token, "PATCH", &url, &serde_json::json!({ "title": title, "body": body }))?;
    Ok(())
}

/// Marks a draft ready for review, or turns a pull request back into a draft. The REST API can't do
/// either, so this uses GraphQL, which answers errors with a success status.
pub fn set_draft(token: &str, node_id: &str, draft: bool) -> Result<(), String> {
    let mutation = if draft { "convertPullRequestToDraft" } else { "markPullRequestReadyForReview" };
    let query =
        format!("mutation($id: ID!) {{ {mutation}(input: {{pullRequestId: $id}}) {{ clientMutationId }} }}");
    let answer = send(
        token,
        "POST",
        &format!("{API_BASE}/graphql"),
        &serde_json::json!({ "query": query, "variables": { "id": node_id } }),
    )?;
    match answer["errors"].as_array().and_then(|errors| errors.first()) {
        Some(error) => {
            Err(error["message"].as_str().unwrap_or(crate::constants::text::GITHUB_REFUSED).to_owned())
        }
        None => Ok(()),
    }
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
    let body = serde_json::json!({
        "title": pull.title, "body": pull.body, "head": pull.head, "base": pull.base, "draft": pull.draft,
    });
    let created: Created =
        serde_json::from_value(send(token, "POST", &format!("{API_BASE}/repos/{slug}/pulls"), &body)?)
            .map_err(|error| format!("Unexpected answer from GitHub: {error}"))?;
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

    #[test]
    fn reads_files_and_line_comments() {
        let file: File = serde_json::from_str(
            r#"{"filename": "src/a.rs", "status": "modified", "additions": 1, "deletions": 1,
                "patch": "@@ -1,2 +1,2 @@\n keep\n-old\n+new"}"#,
        )
        .unwrap();
        let file = pull_file(file);
        assert_eq!(file.diff.lines.len(), 4);
        assert_eq!(file.diff.lines[3].new_number, Some(2));
        let binary = pull_file(serde_json::from_str(r#"{"filename": "a.png", "status": "added"}"#).unwrap());
        assert!(binary.diff_missing);
        assert_eq!(binary.change.kind, ChangeKind::Added);

        let comment: LineComment = serde_json::from_str(
            r#"{"id": 5, "in_reply_to_id": null, "path": "src/a.rs", "line": 2, "side": "RIGHT",
                "user": {"login": "sam"}, "body": "Why?", "created_at": "2026-11-24T10:00:00Z"}"#,
        )
        .unwrap();
        let comment = review_comment(comment);
        assert_eq!((comment.line, comment.side), (Some(2), DiffSide::Right));
    }

    #[test]
    fn reviews_join_the_conversation_in_order() {
        let comment = |at: &str| Comment {
            user: User { login: "a".into() },
            body: Some("hi".into()),
            created_at: at.into(),
        };
        let review = |state: &str, body: &str, at: &str| Review {
            user: User { login: "b".into() },
            body: Some(body.into()),
            state: state.into(),
            submitted_at: Some(at.into()),
        };
        let timeline = conversation(
            vec![comment("2026-01-03"), comment("2026-01-01")],
            vec![
                review("APPROVED", "", "2026-01-02"),
                review("COMMENTED", "", "2026-01-04"),
                review("PENDING", "x", ""),
            ],
        );
        let verdicts: Vec<_> = timeline.iter().map(|entry| entry.verdict.as_deref()).collect();
        assert_eq!(verdicts, [None, Some(crate::constants::text::REVIEW_APPROVED), None]);
    }
}
