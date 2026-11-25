use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use super::parse;
use super::runner::{GitError, GitResult, run, run_binary, run_capped, run_with_input};
use crate::constants::git::{EMPTY_TREE_HASH, MAX_DIFF_BYTES, MAX_UNTRACKED_BYTES, MAX_UNTRACKED_FILES};
use crate::constants::text;
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
    Ok(parse::parse_diff(&run_capped(repo, &args, MAX_DIFF_BYTES + 1)?))
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
    check_untracked_size(
        repo,
        chosen
            .iter()
            .filter(|change| change.kind == ChangeKind::Untracked)
            .map(|change| change.path.as_str()),
    )?;
    if all_chosen {
        run(repo, &["add", "--all"])?;
        run_with_input(repo, &["commit", "--file", "-"], Some(message))?;
        return Ok(());
    }

    let paths = paths_with_originals(chosen);
    stage(repo, chosen)?;
    let mut args = vec!["commit", "--file", "-", "--only", "--"];
    args.extend(paths.iter().map(String::as_str));
    run_with_input(repo, &args, Some(message))?;
    Ok(())
}

/// A file's content in a commit (`rev` like `HEAD` or a SHA), as stored: no text conversion.
pub fn file_at(repo: &Path, rev: &str, path: &str, max_bytes: usize) -> GitResult<Vec<u8>> {
    run_binary(repo, &["cat-file", "blob", &format!("{rev}:{path}")], max_bytes)
}

/// Adds `changes` to the index: new and modified files, deletions, and both sides of renames.
pub fn stage(repo: &Path, changes: &[FileChange]) -> GitResult<()> {
    // Paths already gone from both index and disk (a staged deletion) would make `git add` fail.
    let addable: Vec<String> = paths_with_originals(changes)
        .into_iter()
        .filter(|path| repo.join(path).exists() || is_in_index(repo, path))
        .collect();
    if addable.is_empty() {
        return Ok(());
    }
    let mut args = vec!["add", "--all", "--"];
    args.extend(addable.iter().map(String::as_str));
    run(repo, &args)?;
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

/// Refuses to stash or commit new files when they add up to more than `MAX_UNTRACKED_BYTES` or
/// `MAX_UNTRACKED_FILES`, naming the top-level folder that holds most of them.
pub fn check_untracked_size<'a>(repo: &Path, paths: impl IntoIterator<Item = &'a str>) -> GitResult<()> {
    let (mut files, mut bytes) = (0, 0);
    let mut by_folder: HashMap<String, u64> = HashMap::new();
    for path in paths {
        let size = std::fs::symlink_metadata(repo.join(path)).map_or(0, |metadata| metadata.len());
        files += 1;
        bytes += size;
        let top = match path.split_once('/') {
            Some((folder, _)) => format!("{folder}/"),
            None => path.to_owned(),
        };
        *by_folder.entry(top).or_default() += size;
    }
    if bytes <= MAX_UNTRACKED_BYTES && files <= MAX_UNTRACKED_FILES {
        return Ok(());
    }
    let biggest =
        by_folder.into_iter().max_by_key(|(_, size)| *size).map(|(folder, _)| folder).unwrap_or_default();
    Err(GitError::new(text::too_many_untracked(files, bytes, &biggest)))
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
