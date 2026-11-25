use std::path::Path;

use super::parse;
use super::runner::{GitResult, run, run_capped};
use crate::constants::git::{HISTORY_LIMIT, MAX_DIFF_BYTES};
use crate::models::{CommitSummary, Diff, FileChange};

pub fn log(path: &Path) -> GitResult<Vec<CommitSummary>> {
    let format = format!("--format={}", parse::log_format());
    let limit = format!("--max-count={HISTORY_LIMIT}");
    let output = run(path, &["log", &limit, &format])?;
    Ok(parse::parse_log(&output))
}

/// Files changed by a commit, compared with its first parent (so merges list what they brought in).
pub fn commit_files(path: &Path, sha: &str) -> GitResult<Vec<FileChange>> {
    let output = run(
        path,
        &["diff-tree", "--no-commit-id", "-r", "-z", "--name-status", "--root", "-m", "--first-parent", sha],
    )?;
    Ok(parse::parse_name_status(&output))
}

pub fn commit_file_diff(path: &Path, sha: &str, file: &str) -> GitResult<Diff> {
    let output = run_capped(
        path,
        &["show", "--format=", "--no-color", "--no-ext-diff", "-m", "--first-parent", sha, "--", file],
        MAX_DIFF_BYTES + 1,
    )?;
    Ok(parse::parse_diff(&output))
}
