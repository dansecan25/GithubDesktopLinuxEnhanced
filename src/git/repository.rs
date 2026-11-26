use std::path::{Path, PathBuf};

use super::runner::{GitError, GitResult, run};
use super::{branches, commits, merge, parse, rebase, remote, stash, tags};
use crate::constants::git::{DEFAULT_BRANCH, GITHUB_HTTPS_PREFIX};
use crate::models::RepoSnapshot;

/// Returns the top-level folder of the repository that contains `path`.
pub fn find_root(path: &Path) -> GitResult<PathBuf> {
    if !path.is_dir() {
        return Err(GitError::new(format!("{} is not a folder", path.display())));
    }
    let output = run(path, &["rev-parse", "--show-toplevel"])
        .map_err(|_| GitError::new(format!("{} is not a git repository", path.display())))?;
    Ok(PathBuf::from(output.trim()))
}

/// Creates an empty repository at `path`, creating the folder if needed.
pub fn init(path: &Path) -> GitResult<PathBuf> {
    std::fs::create_dir_all(path)
        .map_err(|error| GitError::new(format!("Could not create {}: {error}", path.display())))?;
    run(path, &["init", "--initial-branch", DEFAULT_BRANCH])?;
    find_root(path)
}

/// Clones `url` into a new folder named after the repository inside `parent`.
pub fn clone(url: &str, parent: &Path) -> GitResult<PathBuf> {
    let url = normalize_clone_url(url).ok_or_else(|| GitError::new("Enter a repository URL"))?;
    let name = repository_name_from_url(&url)
        .ok_or_else(|| GitError::new("Could not work out a folder name from that URL"))?;
    let destination = parent.join(name);
    if destination.exists() {
        return Err(GitError::new(format!("{} already exists", destination.display())));
    }
    std::fs::create_dir_all(parent)
        .map_err(|error| GitError::new(format!("Could not create {}: {error}", parent.display())))?;
    let destination_text = destination.to_string_lossy();
    run(parent, &["clone", "--progress", "--", &url, &destination_text])?;
    Ok(destination)
}

/// Accepts full URLs, SSH addresses and the `owner/repo` shorthand GitHub Desktop supports.
pub fn normalize_clone_url(input: &str) -> Option<String> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let is_shorthand = !input.contains(':')
        && !input.starts_with('/')
        && input.split('/').filter(|part| !part.is_empty()).count() == 2;
    if is_shorthand {
        return Some(format!("{GITHUB_HTTPS_PREFIX}{}.git", input.trim_end_matches(".git")));
    }
    Some(input.to_owned())
}

pub fn repository_name_from_url(url: &str) -> Option<String> {
    let last = url.trim_end_matches('/').rsplit(['/', ':']).next()?.trim_end_matches(".git");
    (!last.is_empty()).then(|| last.to_owned())
}

/// Loads status, branches, stashes and the remote in one go for the main window.
pub fn load_snapshot(path: &Path) -> GitResult<RepoSnapshot> {
    let status_output = run(path, &["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"])?;
    let (status, changes) = parse::parse_status(&status_output);
    let remote = remote::preferred_remote(path)?;
    let remote_url = match &remote {
        Some(name) => remote::url(path, name).ok(),
        None => None,
    };
    let head_pushed = status.has_commits && commits::head_is_pushed(path);
    Ok(RepoSnapshot {
        head_pushed,
        status,
        changes,
        branches: branches::list(path)?,
        stashes: stash::list(path)?,
        tags: tags::list(path)?,
        remote,
        remote_url,
        merge_message: merge::message(path),
        squash_message: merge::squash_message(path),
        rebase: rebase::state(path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_expands_to_github() {
        assert_eq!(normalize_clone_url(" owner/repo ").as_deref(), Some("https://github.com/owner/repo.git"));
        assert_eq!(
            normalize_clone_url("git@github.com:owner/repo.git").as_deref(),
            Some("git@github.com:owner/repo.git")
        );
        assert_eq!(normalize_clone_url("   "), None);
    }

    #[test]
    fn name_comes_from_last_segment() {
        assert_eq!(repository_name_from_url("https://github.com/a/b.git").as_deref(), Some("b"));
        assert_eq!(repository_name_from_url("git@host:b.git").as_deref(), Some("b"));
        assert_eq!(repository_name_from_url("https://host/a/b/").as_deref(), Some("b"));
    }
}
