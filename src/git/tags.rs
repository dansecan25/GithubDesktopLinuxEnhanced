use std::path::Path;

use super::runner::{GitError, GitResult, run, run_with_input};
use super::{changes, parse};
use crate::constants::git::{
    MAX_VERSION_FILE_BYTES, VERSION_FILE_MAX_DEPTH, VERSION_FILE_SKIPPED_FOLDERS, VERSION_FILES,
};
use crate::models::{ProjectVersion, Tag};
use crate::utils::version::version_in_file;

/// The project versions written at `commit`: version files at the root and one folder down, the
/// root first. Dependencies' and build folders' files are skipped.
pub fn project_versions(path: &Path, commit: &str) -> GitResult<Vec<ProjectVersion>> {
    let files = run(path, &["ls-tree", "-r", "--name-only", commit])?;
    let mut candidates: Vec<&str> = files
        .lines()
        .filter(|file| {
            let parts: Vec<&str> = file.split('/').collect();
            let name = parts.last().copied().unwrap_or_default();
            VERSION_FILES.contains(&name)
                && parts.len() <= VERSION_FILE_MAX_DEPTH + 1
                && !parts.iter().any(|part| VERSION_FILE_SKIPPED_FOLDERS.contains(part))
        })
        .collect();
    candidates.sort_by_key(|file| (file.matches('/').count(), *file));
    let mut versions = Vec::new();
    for file in candidates {
        let name = file.rsplit('/').next().unwrap_or(file);
        let Ok(bytes) = changes::file_at(path, commit, file, MAX_VERSION_FILE_BYTES) else { continue };
        if let Some(version) = version_in_file(name, &String::from_utf8_lossy(&bytes)) {
            versions.push(ProjectVersion { path: file.to_owned(), version });
        }
    }
    Ok(versions)
}

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
