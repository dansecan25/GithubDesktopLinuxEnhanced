use std::path::Path;

use super::runner::{GitError, GitResult, run};
use crate::constants::git::DEFAULT_REMOTE;
use crate::constants::text;
use crate::models::{Branch, BranchStatus, PullRequest};

/// `origin` when it exists, otherwise the first configured remote.
pub fn preferred_remote(path: &Path) -> GitResult<Option<String>> {
    let output = run(path, &["remote"])?;
    let remotes: Vec<&str> = output.lines().map(str::trim).filter(|r| !r.is_empty()).collect();
    let chosen = remotes.iter().find(|r| **r == DEFAULT_REMOTE).or(remotes.first());
    Ok(chosen.map(|r| (*r).to_owned()))
}

pub fn fetch(path: &Path) -> GitResult<()> {
    run(path, &["fetch", "--progress", "--all", "--prune"])?;
    Ok(())
}

/// Pulls with the user's own `pull.rebase` / `pull.ff` configuration.
pub fn pull(path: &Path) -> GitResult<()> {
    run(path, &["pull", "--progress"])?;
    Ok(())
}

/// Pushes, and publishes the branch with `--set-upstream` the first time.
pub fn push(path: &Path, status: &BranchStatus, remote: Option<&str>) -> GitResult<()> {
    if status.upstream.is_some() {
        run(path, &["push", "--progress"])?;
        return Ok(());
    }
    if !status.has_commits {
        return Err(GitError::new(text::NOTHING_TO_PUBLISH));
    }
    let branch =
        status.head.as_deref().ok_or_else(|| GitError::new("Switch to a branch before publishing"))?;
    let remote = remote.ok_or_else(|| GitError::new("This repository has no remote to push to"))?;
    run(path, &["push", "--progress", "--set-upstream", remote, branch])?;
    Ok(())
}

/// Pushes a branch whose history was rewritten (an amended, undone or rebased commit that was
/// already pushed). `--force-with-lease` refuses if someone else pushed meanwhile.
pub fn force_push(path: &Path) -> GitResult<()> {
    run(path, &["push", "--progress", "--force-with-lease"])?;
    Ok(())
}

/// Fetches a pull request's branch and returns the branch to switch to: the branch itself for a
/// pull request from this repository, or `pr/<number>` for one from a fork.
pub fn fetch_pull(path: &Path, remote: &str, pull: &PullRequest, local: &[String]) -> GitResult<Branch> {
    let branch = |name: String, is_remote: bool| Branch {
        name,
        is_remote,
        is_current: false,
        upstream: None,
        committed: None,
    };
    if pull.same_repository {
        run(path, &["fetch", "--progress", remote, &pull.head_branch])?;
        return Ok(if local.contains(&pull.head_branch) {
            branch(pull.head_branch.clone(), false)
        } else {
            branch(format!("{remote}/{}", pull.head_branch), true)
        });
    }
    let local_name = format!("pr/{}", pull.number);
    let refspec = format!("+refs/pull/{}/head:refs/heads/{local_name}", pull.number);
    run(path, &["fetch", "--progress", remote, &refspec])?;
    Ok(branch(local_name, false))
}

/// Connects the repository to a remote (`origin` when it has none yet).
pub fn add(path: &Path, name: &str, url: &str) -> GitResult<()> {
    if url.trim().is_empty() {
        return Err(GitError::new("Enter the remote's URL"));
    }
    run(path, &["remote", "add", "--", name, url.trim()])?;
    Ok(())
}

pub fn url(path: &Path, remote: &str) -> GitResult<String> {
    Ok(run(path, &["remote", "get-url", "--", remote])?.trim().to_owned())
}
