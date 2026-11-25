use std::path::Path;

use super::parse;
use super::runner::{GitResult, run};
use crate::constants::git::{BRANCH_STASH_PREFIX, BRANCH_STASH_SUFFIX};
use crate::models::StashEntry;

pub fn list(path: &Path) -> GitResult<Vec<StashEntry>> {
    let format = format!("--format={}", parse::stash_format());
    let output = run(path, &["stash", "list", &format])?;
    Ok(parse::parse_stashes(&output))
}

/// The message that ties a stash to `branch`, in GitHub Desktop's format.
pub fn branch_message(branch: &str) -> String {
    format!("{BRANCH_STASH_PREFIX}{branch}{BRANCH_STASH_SUFFIX}")
}

/// The newest stash's commit, or `None` when there are no stashes.
fn newest(path: &Path) -> Option<String> {
    run(path, &["rev-parse", "-q", "--verify", "refs/stash"]).ok().map(|sha| sha.trim().to_owned())
}

/// Stashes every change and reports whether anything was stashed (git does nothing, successfully,
/// when there are no changes).
pub fn push_all_checked(path: &Path, message: &str) -> GitResult<bool> {
    let before = newest(path);
    push_all(path, message)?;
    Ok(newest(path) != before)
}

/// Stashes every change, including untracked files, like GitHub Desktop's "Stash all changes".
pub fn push_all(path: &Path, message: &str) -> GitResult<()> {
    run(path, &["stash", "push", "--include-untracked", "--message", message])?;
    Ok(())
}

/// Applies a stash and removes it (GitHub Desktop's "Restore").
pub fn pop(path: &Path, reference: &str) -> GitResult<()> {
    run(path, &["stash", "pop", reference])?;
    Ok(())
}

pub fn drop(path: &Path, reference: &str) -> GitResult<()> {
    run(path, &["stash", "drop", reference])?;
    Ok(())
}
