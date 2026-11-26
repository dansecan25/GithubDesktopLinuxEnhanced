//! Merging another branch into the current one, as GitHub Desktop does it.

use std::path::Path;

use super::runner::{GitError, GitResult, run, run_raw, run_with_input};
use crate::constants::text;
use crate::models::{ChangeKind, FileChange, MergePreview};

fn count(path: &Path, range: &str) -> GitResult<usize> {
    Ok(run(path, &["rev-list", "--count", range])?.trim().parse().unwrap_or(0))
}

/// What merging `branch` into HEAD would do, without touching the working tree. The conflicts are
/// a merge's; a rebase applies commits one by one, so it can stop on others.
pub fn preview(path: &Path, branch: &str) -> GitResult<MergePreview> {
    let commits = count(path, &format!("HEAD..{branch}"))?;
    let ahead = count(path, &format!("{branch}..HEAD"))?;
    if commits == 0 {
        return Ok(MergePreview { commits, ahead, conflicts: Some(Vec::new()) });
    }
    // Exit code 1 means conflicts. Older git (before 2.38) doesn't know `--write-tree`: then
    // conflicts are unknown until the merge runs.
    let output =
        run_raw(path, &["merge-tree", "--write-tree", "--name-only", "--no-messages", "HEAD", branch], None)?;
    let conflicts = match output.exit_code {
        0 => Some(Vec::new()),
        1 => Some(
            output.stdout.lines().skip(1).take_while(|line| !line.is_empty()).map(str::to_owned).collect(),
        ),
        _ => None,
    };
    Ok(MergePreview { commits, ahead, conflicts })
}

/// Merges `branch` into the current branch. On conflicts the merge stays in progress, so the user
/// can resolve the files and commit, or abort.
pub fn merge(path: &Path, branch: &str) -> GitResult<()> {
    match run(path, &["merge", "--no-edit", branch]) {
        Err(_) if message(path).is_some() => Err(GitError::new(format!(
            "Merging {branch} caused conflicts. Resolve the files marked with !, then commit to finish \
             the merge, or abort it."
        ))),
        result => result.map(|_| ()),
    }
}

pub fn abort(path: &Path) -> GitResult<()> {
    run(path, &["merge", "--abort"])?;
    Ok(())
}

/// GitHub Desktop's "Squash and merge": all of `branch`'s changes become one new commit on the
/// current branch. On conflicts nothing is committed; the prepared message waits in the form.
pub fn squash(path: &Path, branch: &str, into: &str) -> GitResult<()> {
    let summaries = run(path, &["log", "--reverse", "--format=%s", &format!("HEAD..{branch}")])?;
    let message = text::squash_message(branch, into, &summaries.lines().collect::<Vec<_>>());
    let result = run(path, &["merge", "--squash", "--no-commit", branch]);
    // Git's own SQUASH_MSG lists every commit in full; the form shows this one instead.
    write_git_file(path, SQUASH_MSG, &message)?;
    match result {
        Err(_) if has_unmerged(path) => Err(GitError::new(text::squash_conflicts(branch))),
        Err(error) => {
            // Git refused before changing anything (local changes in the way, for example).
            remove_squash_message(path);
            Err(error)
        }
        Ok(_) => {
            let committed = run_with_input(path, &["commit", "--file", "-"], Some(&message));
            let nothing_staged = run(path, &["diff", "--cached", "--quiet"]).is_ok();
            if committed.is_ok() || nothing_staged {
                remove_squash_message(path);
            }
            committed.map(|_| ())
        }
    }
}

fn remove_squash_message(path: &Path) {
    if let Ok(file) = git_path(path, SQUASH_MSG) {
        let _ = std::fs::remove_file(file);
    }
}

/// Undoes a squash merge that stopped on conflicts. Changes the merge didn't touch are kept.
pub fn abort_squash(path: &Path) -> GitResult<()> {
    run(path, &["reset", "--merge"])?;
    remove_squash_message(path);
    Ok(())
}

const SQUASH_MSG: &str = "SQUASH_MSG";

/// The prepared message while a squash merge waits to be committed. Git removes the file when the
/// commit is made.
pub fn squash_message(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(git_path(path, SQUASH_MSG).ok()?).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}

fn git_path(path: &Path, name: &str) -> GitResult<std::path::PathBuf> {
    Ok(path.join(run(path, &["rev-parse", "--git-path", name])?.trim()))
}

fn write_git_file(path: &Path, name: &str, content: &str) -> GitResult<()> {
    std::fs::write(git_path(path, name)?, content)
        .map_err(|error| GitError::new(format!("Could not write {name}: {error}")))
}

fn has_unmerged(path: &Path) -> bool {
    run(path, &["ls-files", "--unmerged"]).is_ok_and(|output| !output.trim().is_empty())
}

/// The prepared commit summary while a merge is in progress (`Merge branch 'x' into main`).
pub fn message(path: &Path) -> Option<String> {
    run(path, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).ok()?;
    let file = run(path, &["rev-parse", "--git-path", "MERGE_MSG"]).ok()?;
    let text = std::fs::read_to_string(path.join(file.trim())).unwrap_or_default();
    let summary = text.lines().next().unwrap_or_default().trim().to_owned();
    Some(if summary.is_empty() { "Merge".to_owned() } else { summary })
}

/// Conflicted files that still contain conflict markers, so they can't be committed yet.
pub fn unresolved(path: &Path, changes: &[FileChange]) -> Vec<String> {
    changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Conflicted)
        .filter(|change| {
            std::fs::read(path.join(&change.path)).is_ok_and(|bytes| has_conflict_markers(&bytes))
        })
        .map(|change| change.path.clone())
        .collect()
}

fn has_conflict_markers(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.lines().any(|line| line.starts_with("<<<<<<< "))
        && text.lines().any(|line| line.starts_with(">>>>>>> "))
}

#[cfg(test)]
mod tests {
    use super::has_conflict_markers;

    #[test]
    fn finds_conflict_markers() {
        assert!(has_conflict_markers(b"a\n<<<<<<< HEAD\nb\n=======\nc\n>>>>>>> feature\n"));
        assert!(!has_conflict_markers(b"a\nb\n"));
        assert!(!has_conflict_markers(b"<<<<<<< only an opening marker\n"));
    }
}
