//! Merging another branch into the current one, as GitHub Desktop does it.

use std::path::Path;

use super::runner::{GitError, GitResult, run, run_raw};
use crate::models::{ChangeKind, FileChange, MergePreview};

/// What merging `branch` into HEAD would do, without touching the working tree.
pub fn preview(path: &Path, branch: &str) -> GitResult<MergePreview> {
    let range = format!("HEAD..{branch}");
    let commits = run(path, &["rev-list", "--count", &range])?.trim().parse().unwrap_or(0);
    if commits == 0 {
        return Ok(MergePreview { commits, conflicts: Some(Vec::new()) });
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
    Ok(MergePreview { commits, conflicts })
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
