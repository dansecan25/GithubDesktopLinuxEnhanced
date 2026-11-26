//! Rebasing the current branch onto another, as GitHub Desktop's "Rebase" does: its commits are
//! replayed on top of the other branch. A conflict stops the rebase; the user resolves the files in
//! the conflict editor, then continues or aborts.

use std::path::{Path, PathBuf};

use super::runner::{GitError, GitResult, run};
use crate::constants::text;
use crate::models::RebaseState;

/// No editor opens: the replayed commits keep their messages.
const NO_EDITOR: [&str; 2] = ["-c", "core.editor=true"];

pub fn rebase(path: &Path, onto: &str) -> GitResult<()> {
    if !run(path, &["status", "--porcelain", "--untracked-files=no"])?.trim().is_empty() {
        return Err(GitError::new(text::REBASE_NEEDS_CLEAN_TREE));
    }
    stop_on_conflicts(path, run(path, &[NO_EDITOR[0], NO_EDITOR[1], "rebase", onto]).map(|_| ()))
}

/// Goes on after the conflicts of the current commit are resolved. A commit that ends up with no
/// changes (everything resolved to what's already there) is skipped, as git suggests.
pub fn continue_rebase(path: &Path) -> GitResult<()> {
    if !run(path, &["ls-files", "--unmerged"])?.trim().is_empty() {
        return Err(GitError::new(text::RESOLVE_BEFORE_CONTINUING));
    }
    let nothing_staged = run(path, &["diff", "--cached", "--quiet"]).is_ok();
    let step = if nothing_staged { "--skip" } else { "--continue" };
    stop_on_conflicts(path, run(path, &[NO_EDITOR[0], NO_EDITOR[1], "rebase", step]).map(|_| ()))
}

pub fn abort(path: &Path) -> GitResult<()> {
    run(path, &["rebase", "--abort"])?;
    Ok(())
}

/// A failed step that left the rebase stopped is explained; anything else is git's own error.
fn stop_on_conflicts(path: &Path, result: GitResult<()>) -> GitResult<()> {
    match result {
        Err(_) if state(path).is_some() => Err(GitError::new(text::REBASE_STOPPED)),
        other => other,
    }
}

/// The stopped rebase, read from git's state folder (`rebase-merge`, or `rebase-apply` for the
/// older backend).
pub fn state(path: &Path) -> Option<RebaseState> {
    let folder = ["rebase-merge", "rebase-apply"].into_iter().find_map(|name| {
        let folder = PathBuf::from(run(path, &["rev-parse", "--git-path", name]).ok()?.trim());
        let folder = if folder.is_absolute() { folder } else { path.join(folder) };
        folder.is_dir().then_some(folder)
    })?;
    let read = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| std::fs::read_to_string(folder.join(name)).ok())
            .map(|s| s.trim().to_owned())
    };
    let number = |names: &[&str]| read(names).and_then(|value| value.parse().ok()).unwrap_or(0);
    let branch = read(&["head-name"]).unwrap_or_default();
    let branch = branch.strip_prefix("refs/heads/").unwrap_or(&branch).to_owned();
    let onto_sha = read(&["onto"]).unwrap_or_default();
    Some(RebaseState {
        branch,
        onto: name_of(path, &onto_sha),
        step: number(&["msgnum", "next"]),
        total: number(&["end", "last"]),
    })
}

/// A branch pointing at `sha`, or its short form.
fn name_of(path: &Path, sha: &str) -> String {
    let points_at = format!("--points-at={sha}");
    run(
        path,
        &["for-each-ref", &points_at, "--count=1", "--format=%(refname:short)", "refs/heads", "refs/remotes"],
    )
    .ok()
    .map(|name| name.trim().to_owned())
    .filter(|name| !name.is_empty())
    .unwrap_or_else(|| sha.chars().take(7).collect())
}
