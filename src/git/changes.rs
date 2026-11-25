use std::io::Read;
use std::path::Path;

use super::parse;
use super::runner::{GitError, GitResult, run, run_with_input};
use crate::constants::git::{EMPTY_TREE_HASH, MAX_DIFF_BYTES};
use crate::models::{ChangeKind, Diff, FileChange};

/// Diff of one working-tree file against the last commit (staged and unstaged together, like
/// GitHub Desktop shows it).
pub fn working_diff(repo: &Path, change: &FileChange, has_commits: bool) -> GitResult<Diff> {
    if change.kind == ChangeKind::Untracked {
        return untracked_file_diff(&repo.join(&change.path));
    }
    let base = if has_commits { "HEAD" } else { EMPTY_TREE_HASH };
    let mut args = vec!["diff", "--no-color", "--no-ext-diff", "-M", base, "--"];
    args.extend(change.original_path.as_deref());
    args.push(&change.path);
    Ok(parse::parse_diff(&run(repo, &args)?))
}

fn untracked_file_diff(file: &Path) -> GitResult<Diff> {
    if file.is_dir() {
        return Ok(Diff::default());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(file)
        .and_then(|handle| handle.take(MAX_DIFF_BYTES as u64 + 1).read_to_end(&mut bytes))
        .map_err(|error| GitError::new(format!("Could not read {}: {error}", file.display())))?;
    if parse::looks_binary(&bytes) {
        return Ok(Diff { is_binary: true, ..Diff::default() });
    }
    Ok(parse::diff_for_new_file(&String::from_utf8_lossy(&bytes)))
}

/// Commits the chosen files. When every changed file is chosen it commits the whole tree, which
/// also covers finishing a merge; otherwise only the chosen paths go in.
pub fn commit(repo: &Path, message: &str, chosen: &[FileChange], all_chosen: bool) -> GitResult<()> {
    if chosen.is_empty() {
        return Err(GitError::new("Choose at least one file to commit"));
    }
    if message.trim().is_empty() {
        return Err(GitError::new("Write a commit summary"));
    }
    if all_chosen {
        run(repo, &["add", "--all"])?;
        run_with_input(repo, &["commit", "--file", "-"], Some(message))?;
        return Ok(());
    }

    let paths = paths_with_originals(chosen);
    // Paths already gone from both index and disk (a staged deletion) would make `git add` fail.
    let addable: Vec<&str> = chosen
        .iter()
        .filter(|change| repo.join(&change.path).exists() || is_in_index(repo, &change.path))
        .map(|change| change.path.as_str())
        .collect();
    if !addable.is_empty() {
        let mut args = vec!["add", "--all", "--"];
        args.extend(&addable);
        run(repo, &args)?;
    }
    let mut args = vec!["commit", "--file", "-", "--only", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_with_input(repo, &args, Some(message))?;
    Ok(())
}

fn is_in_index(repo: &Path, path: &str) -> bool {
    run(repo, &["ls-files", "--error-unmatch", "--", path]).is_ok()
}

fn paths_with_originals(changes: &[FileChange]) -> Vec<String> {
    let mut paths = Vec::new();
    for change in changes {
        paths.extend(change.original_path.clone());
        paths.push(change.path.clone());
    }
    paths
}

/// Throws away changes to the given files. Untracked and newly added files are deleted.
pub fn discard(repo: &Path, changes: &[FileChange]) -> GitResult<()> {
    let mut untracked = Vec::new();
    let mut new_in_index = Vec::new();
    let mut restore = Vec::new();
    for change in changes {
        match change.kind {
            ChangeKind::Untracked => untracked.push(change.path.as_str()),
            ChangeKind::Added | ChangeKind::Copied => new_in_index.push(change.path.as_str()),
            ChangeKind::Renamed => {
                new_in_index.push(change.path.as_str());
                restore.extend(change.original_path.as_deref());
            }
            _ => restore.push(change.path.as_str()),
        }
    }
    run_for_paths(repo, &["clean", "--force", "--quiet"], &untracked)?;
    run_for_paths(repo, &["rm", "--force", "--quiet"], &new_in_index)?;
    run_for_paths(repo, &["restore", "--source=HEAD", "--staged", "--worktree"], &restore)?;
    Ok(())
}

fn run_for_paths(repo: &Path, command: &[&str], paths: &[&str]) -> GitResult<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut args = command.to_vec();
    args.push("--");
    args.extend(paths);
    run(repo, &args)?;
    Ok(())
}

/// Adds a line to the repository's top-level `.gitignore`, creating it if needed.
pub fn ignore(repo: &Path, pattern: &str) -> GitResult<()> {
    let file = repo.join(".gitignore");
    let existing = std::fs::read_to_string(&file).unwrap_or_default();
    if existing.lines().any(|line| line.trim() == pattern) {
        return Ok(());
    }
    let separator = if existing.is_empty() || existing.ends_with('\n') { "" } else { "\n" };
    std::fs::write(&file, format!("{existing}{separator}{pattern}\n"))
        .map_err(|error| GitError::new(format!("Could not update .gitignore: {error}")))
}

/// The `.gitignore` pattern for one file: anchored to the repository root, with special
/// characters escaped.
pub fn ignore_pattern(path: &str) -> String {
    let mut pattern = String::from("/");
    for c in path.chars() {
        if matches!(c, '*' | '?' | '[' | ']' | '\\' | '!' | '#') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern
}
