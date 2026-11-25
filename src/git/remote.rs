use std::path::Path;

use super::runner::{GitError, GitResult, run};
use crate::constants::git::DEFAULT_REMOTE;
use crate::constants::text;
use crate::models::BranchStatus;

/// `origin` when it exists, otherwise the first configured remote.
pub fn preferred_remote(path: &Path) -> GitResult<Option<String>> {
    let output = run(path, &["remote"])?;
    let remotes: Vec<&str> = output.lines().map(str::trim).filter(|r| !r.is_empty()).collect();
    let chosen = remotes.iter().find(|r| **r == DEFAULT_REMOTE).or(remotes.first());
    Ok(chosen.map(|r| (*r).to_owned()))
}

pub fn fetch(path: &Path) -> GitResult<()> {
    run(path, &["fetch", "--all", "--prune"])?;
    Ok(())
}

/// Pulls with the user's own `pull.rebase` / `pull.ff` configuration.
pub fn pull(path: &Path) -> GitResult<()> {
    run(path, &["pull"])?;
    Ok(())
}

/// Pushes, and publishes the branch with `--set-upstream` the first time.
pub fn push(path: &Path, status: &BranchStatus, remote: Option<&str>) -> GitResult<()> {
    if status.upstream.is_some() {
        run(path, &["push"])?;
        return Ok(());
    }
    if !status.has_commits {
        return Err(GitError::new(text::NOTHING_TO_PUBLISH));
    }
    let branch =
        status.head.as_deref().ok_or_else(|| GitError::new("Switch to a branch before publishing"))?;
    let remote = remote.ok_or_else(|| GitError::new("This repository has no remote to push to"))?;
    run(path, &["push", "--set-upstream", remote, branch])?;
    Ok(())
}

/// Connects the repository to a remote (`origin` when it has none yet).
pub fn add(path: &Path, name: &str, url: &str) -> GitResult<()> {
    if url.trim().is_empty() {
        return Err(GitError::new("Enter the remote's URL"));
    }
    run(path, &["remote", "add", "--", name, url.trim()])?;
    Ok(())
}
