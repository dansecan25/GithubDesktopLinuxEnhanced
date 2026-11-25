//! Committing only some lines of a file, as GitHub Desktop does.
//!
//! The index is rebuilt from scratch: reset to HEAD, whole files added, and for each partly chosen
//! file a patch with only the chosen lines applied to the index. Then the index is committed. The
//! lines left out stay in the working tree as uncommitted changes.

use std::path::Path;

use super::changes::{self, check_untracked_size};
use super::runner::{GitError, GitResult, run, run_with_input};
use crate::constants::text;
use crate::models::{ChangeKind, Diff, DiffLineKind, FileChange, LineSelection};

/// Commits `chosen` whole, except the files in `partial`, of which only the chosen lines go in.
pub fn commit_with_lines(
    repo: &Path,
    message: &str,
    chosen: &[FileChange],
    partial: &[(FileChange, LineSelection)],
    has_commits: bool,
) -> GitResult<()> {
    if message.trim().is_empty() {
        return Err(GitError::new("Write a commit summary"));
    }
    let whole: Vec<FileChange> = chosen
        .iter()
        .filter(|change| !partial.iter().any(|(file, _)| file.path == change.path))
        .cloned()
        .collect();
    check_untracked_size(
        repo,
        whole.iter().filter(|change| change.kind == ChangeKind::Untracked).map(|change| change.path.as_str()),
    )?;
    // Built before touching the index, so a file that changed since its lines were picked stops the
    // commit with nothing changed.
    let mut patches = Vec::new();
    for (change, selection) in partial {
        let current = changes::working_diff(repo, change, has_commits)?;
        if current.lines != selection.diff.lines {
            return Err(GitError::new(text::lines_changed(&change.path)));
        }
        let new_file = change.kind == ChangeKind::Untracked;
        let ends_with_newline = !new_file
            || std::fs::read(repo.join(&change.path)).map_or(true, |bytes| bytes.last() == Some(&b'\n'));
        patches.extend(patch(&change.path, selection, new_file, ends_with_newline));
    }

    if has_commits {
        run(repo, &["reset", "-q"])?;
    } else {
        run(repo, &["read-tree", "--empty"])?;
    }
    changes::stage(repo, &whole)?;
    for patch in &patches {
        run_with_input(repo, &["apply", "--cached", "--recount", "--whitespace=nowarn", "-"], Some(patch))?;
    }
    run_with_input(repo, &["commit", "--file", "-"], Some(message))?;
    Ok(())
}

/// A patch with only the chosen lines: left-out additions are dropped, and left-out removals stay
/// as unchanged lines. `None` when no line is chosen.
pub fn patch(
    path: &str,
    selection: &LineSelection,
    new_file: bool,
    ends_with_newline: bool,
) -> Option<String> {
    let diff: &Diff = &selection.diff;
    let included = |index: usize| !selection.excluded.contains(&index);
    let mut out = String::new();
    if new_file {
        out.push_str(&format!(
            "diff --git a/{path} b/{path}\nnew file mode 100644\n--- /dev/null\n+++ b/{path}\n"
        ));
    } else {
        for line in diff.lines.iter().take_while(|line| line.kind == DiffLineKind::Header) {
            out.push_str(&line.text);
            out.push('\n');
        }
    }

    let mut any = false;
    let mut hunk = String::new();
    let mut hunk_has_change = false;
    // Whether the last line written to the hunk was kept, so a following "\ No newline" note goes
    // with it.
    let mut last_kept = false;
    let flush = |out: &mut String, hunk: &mut String, has_change: &mut bool, any: &mut bool| {
        if *has_change {
            out.push_str(hunk);
            *any = true;
        }
        hunk.clear();
        *has_change = false;
    };
    if new_file {
        hunk.push_str("@@ -0,0 +1 @@\n");
    }
    for (index, line) in diff.lines.iter().enumerate() {
        let body = line.text.get(1..).unwrap_or_default();
        match line.kind {
            DiffLineKind::Header => continue,
            DiffLineKind::Hunk => {
                flush(&mut out, &mut hunk, &mut hunk_has_change, &mut any);
                hunk.push_str(&line.text);
                hunk.push('\n');
                last_kept = false;
                continue;
            }
            DiffLineKind::Context => {
                hunk.push_str(&format!(" {body}\n"));
                last_kept = true;
            }
            DiffLineKind::Added if included(index) => {
                hunk.push_str(&format!("+{body}\n"));
                hunk_has_change = true;
                last_kept = true;
            }
            DiffLineKind::Added => last_kept = false,
            DiffLineKind::Removed if included(index) => {
                hunk.push_str(&format!("-{body}\n"));
                hunk_has_change = true;
                last_kept = true;
            }
            DiffLineKind::Removed => {
                hunk.push_str(&format!(" {body}\n"));
                last_kept = true;
            }
            DiffLineKind::Note => {
                if last_kept {
                    hunk.push_str(&line.text);
                    hunk.push('\n');
                }
            }
        }
    }
    // A new file's last line, if chosen, keeps its missing newline.
    if new_file && !ends_with_newline && diff.lines.last().is_some_and(|_| included(diff.lines.len() - 1)) {
        hunk.push_str("\\ No newline at end of file\n");
    }
    flush(&mut out, &mut hunk, &mut hunk_has_change, &mut any);
    any.then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::parse::{diff_for_new_file, parse_diff};

    const DIFF: &str = "diff --git a/f.txt b/f.txt\nindex 1..2 100644\n--- a/f.txt\n+++ b/f.txt\n\
                        @@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three\n";

    #[test]
    fn left_out_lines_are_dropped_or_kept_as_context() {
        let mut selection = LineSelection::new(parse_diff(DIFF));
        // Leave out the addition: the removal goes in alone.
        let added = selection.diff.lines.iter().position(|line| line.text == "+TWO").unwrap();
        selection.toggle_line(added);
        let patch = patch("f.txt", &selection, false, true).unwrap();
        assert!(patch.ends_with("@@ -1,3 +1,3 @@\n one\n-two\n three\n"), "{patch}");
        // Leave out the removal too: nothing left to commit.
        selection.toggle_line(added - 1);
        assert_eq!(patch_or_none(&selection), None);
    }

    fn patch_or_none(selection: &LineSelection) -> Option<String> {
        patch("f.txt", selection, false, true)
    }

    #[test]
    fn new_files_get_headers_and_only_chosen_lines() {
        let mut selection = LineSelection::new(diff_for_new_file("a\nb\nc"));
        selection.toggle_line(1);
        let patch = patch("n.txt", &selection, true, false).unwrap();
        assert_eq!(
            patch,
            "diff --git a/n.txt b/n.txt\nnew file mode 100644\n--- /dev/null\n+++ b/n.txt\n\
             @@ -0,0 +1 @@\n+a\n+c\n\\ No newline at end of file\n"
        );
    }
}
