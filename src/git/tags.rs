use std::path::Path;

use super::parse;
use super::runner::{GitError, GitResult, run, run_with_input};
use crate::models::Tag;

/// All tags, newest first.
pub fn list(path: &Path) -> GitResult<Vec<Tag>> {
    let format = format!("--format={}", parse::tag_format());
    let output = run(path, &["for-each-ref", "--sort=-creatordate", &format, "refs/tags"])?;
    Ok(parse::parse_tags(&output))
}

pub fn is_valid_name(path: &Path, name: &str) -> bool {
    !name.trim().is_empty()
        && !name.starts_with('-')
        && run(path, &["check-ref-format", &format!("refs/tags/{name}")]).is_ok()
}

/// Creates a tag on `target` (a commit). With a message it is an annotated tag, which is what
/// release tools such as `git describe` and GitHub releases expect.
pub fn create(path: &Path, name: &str, target: &str, message: &str) -> GitResult<()> {
    if !is_valid_name(path, name) {
        return Err(GitError::new(format!("\"{name}\" isn't a valid tag name")));
    }
    if message.trim().is_empty() {
        run(path, &["tag", "--", name, target])?;
    } else {
        run_with_input(path, &["tag", "--annotate", "--file", "-", "--", name, target], Some(message))?;
    }
    Ok(())
}

pub fn push(path: &Path, remote: &str, name: &str) -> GitResult<()> {
    run(path, &["push", remote, &format!("refs/tags/{name}")])?;
    Ok(())
}

/// Deletes the tag locally, and on `remote` when given.
pub fn delete(path: &Path, name: &str, remote: Option<&str>) -> GitResult<()> {
    if let Some(remote) = remote {
        run(path, &["push", remote, "--delete", &format!("refs/tags/{name}")])?;
    }
    run(path, &["tag", "--delete", "--", name])?;
    Ok(())
}
