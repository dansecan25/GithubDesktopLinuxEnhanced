use std::path::Path;

use super::parse;
use super::runner::{GitResult, run};
use crate::models::StashEntry;

pub fn list(path: &Path) -> GitResult<Vec<StashEntry>> {
    let format = format!("--format={}", parse::stash_format());
    let output = run(path, &["stash", "list", &format])?;
    Ok(parse::parse_stashes(&output))
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
