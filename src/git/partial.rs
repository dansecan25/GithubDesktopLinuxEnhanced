//! Committing only some lines of a file, as GitHub Desktop does.
//!
//! The index is rebuilt from scratch: reset to HEAD, whole files added, and for each partly chosen
//! file a patch with only the chosen lines applied to the index. Then the index is committed. The
//! lines left out stay in the working tree as uncommitted changes.

use std::path::Path;

use super::changes::{self, check_untracked_size};
use super::runner::{GitError, GitResult, run, run_with_bytes, run_with_input};
use crate::constants::text;
use crate::models::{ChangeKind, Diff, DiffLineKind, FileChange, LineSelection};

/// Commits `chosen` whole, except the files in `partial`, of which only the chosen lines go in.
/// With `amend` the commit replaces the last one.
pub fn commit_with_lines(
    repo: &Path,
    message: &str,
    chosen: &[FileChange],
    partial: &[(FileChange, LineSelection)],
    has_commits: bool,
    amend: bool,
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
    let mut new_files = Vec::new();
    for (change, selection) in partial {
        let current = changes::working_diff(repo, change, has_commits)?;
        if current.lines != selection.diff.lines {
            return Err(GitError::new(text::lines_changed(&change.path)));
        }
        if change.kind == ChangeKind::Untracked {
            new_files.push((change.path.as_str(), new_file_content(repo, &change.path, selection)?));
        } else {
            patches.extend(patch(&change.path, selection));
        }
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
    for (path, content) in &new_files {
        add_to_index(repo, path, content)?;
    }
    let args: &[&str] =
        if amend { &["commit", "--amend", "--file", "-"] } else { &["commit", "--file", "-"] };
    run_with_input(repo, args, Some(message))?;
    Ok(())
}

/// The chosen lines of a new file, byte for byte as they are on disk (endings included).
fn new_file_content(repo: &Path, path: &str, selection: &LineSelection) -> GitResult<Vec<u8>> {
    let bytes = std::fs::read(repo.join(path))
        .map_err(|error| GitError::new(format!("Could not read {path}: {error}")))?;
    // A new file's diff has one added line per line of the file, in order.
    let mut content = Vec::new();
    for (index, line) in bytes.split_inclusive(|&byte| byte == b'\n').enumerate() {
        if !selection.excluded.contains(&index) {
            content.extend_from_slice(line);
        }
    }
    Ok(content)
}

/// Stores `content` as `path` in the index. `hash-object --path` applies the same filters and
/// line-ending conversion as `git add`, so `core.autocrlf` and `.gitattributes` are respected.
fn add_to_index(repo: &Path, path: &str, content: &[u8]) -> GitResult<()> {
    let hash_args = ["hash-object", "-w", "--stdin", &format!("--path={path}")];
    // Git reads the content as bytes, so a file that isn't UTF-8 goes in unchanged.
    let blob = run_with_bytes(repo, &hash_args, content)?;
    let mode = if is_executable(&repo.join(path)) { "100755" } else { "100644" };
    run(repo, &["update-index", "--add", "--cacheinfo", &format!("{mode},{},{path}", blob.trim())])?;
    Ok(())
}

#[cfg(unix)]
fn is_executable(file: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(file).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(_file: &Path) -> bool {
    false
}

/// A patch with only the chosen lines of a tracked file: left-out additions are dropped, and
/// left-out removals stay as unchanged lines. Lines keep their `\r\n` endings. `None` when no line
/// is chosen.
///
/// A deletion with some lines left out is no longer a deletion: the file stays, without the chosen
/// lines. A rename keeps its rename headers, so the file moves with only the chosen changes.
pub fn patch(path: &str, selection: &LineSelection) -> Option<String> {
    let diff: &Diff = &selection.diff;
    let included = |index: usize| !selection.excluded.contains(&index);
    let partial_deletion = !selection.excluded.is_empty()
        && diff
            .lines
            .iter()
            .any(|line| line.kind == DiffLineKind::Header && line.text.starts_with("deleted file mode"));
    let mut out = String::new();
    for line in diff.lines.iter().take_while(|line| line.kind == DiffLineKind::Header) {
        if partial_deletion {
            if let Some(header) = kept_deletion_header(&line.text) {
                out.push_str(&header);
                out.push('\n');
            }
            continue;
        }
        out.push_str(&line.text);
        out.push('\n');
    }
    if out.is_empty() {
        out.push_str(&format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n"));
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
    for (index, line) in diff.lines.iter().enumerate() {
        let body = line.text.get(1..).unwrap_or_default();
        let ending = if line.crlf { "\r\n" } else { "\n" };
        let push = |hunk: &mut String, prefix: char| {
            hunk.push(prefix);
            hunk.push_str(body);
            hunk.push_str(ending);
        };
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
                push(&mut hunk, ' ');
                last_kept = true;
            }
            DiffLineKind::Added if included(index) => {
                push(&mut hunk, '+');
                hunk_has_change = true;
                last_kept = true;
            }
            DiffLineKind::Added => last_kept = false,
            DiffLineKind::Removed if included(index) => {
                push(&mut hunk, '-');
                hunk_has_change = true;
                last_kept = true;
            }
            DiffLineKind::Removed => {
                push(&mut hunk, ' ');
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
    flush(&mut out, &mut hunk, &mut hunk_has_change, &mut any);
    any.then_some(out)
}

/// A deletion's header turned into a modification's: the mode and index lines go, and `+++
/// /dev/null` points at the file again.
fn kept_deletion_header(line: &str) -> Option<String> {
    if ["deleted file mode", "index ", "+++ "].iter().any(|prefix| line.starts_with(prefix)) {
        return None;
    }
    if let Some(old) = line.strip_prefix("--- ") {
        let new = match old.strip_prefix("\"a/") {
            Some(rest) => format!("\"b/{rest}"),
            None => format!("b/{}", old.strip_prefix("a/").unwrap_or(old)),
        };
        return Some(format!("{line}\n+++ {new}"));
    }
    Some(line.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::parse::parse_diff;

    const DIFF: &str = "diff --git a/f.txt b/f.txt\nindex 1..2 100644\n--- a/f.txt\n+++ b/f.txt\n\
                        @@ -1,3 +1,3 @@\n one\n-two\n+TWO\n three\n";

    fn index_of(selection: &LineSelection, text: &str) -> usize {
        selection.diff.lines.iter().position(|line| line.text == text).unwrap()
    }

    #[test]
    fn left_out_lines_are_dropped_or_kept_as_context() {
        let mut selection = LineSelection::new(parse_diff(DIFF));
        // Leave out the addition: the removal goes in alone.
        let added = index_of(&selection, "+TWO");
        selection.toggle_line(added);
        let patch = patch("f.txt", &selection).unwrap();
        assert!(patch.ends_with("@@ -1,3 +1,3 @@\n one\n-two\n three\n"), "{patch}");
        // Leave out the removal too: nothing left to commit.
        selection.toggle_line(added - 1);
        assert_eq!(super::patch("f.txt", &selection), None);
    }

    #[test]
    fn crlf_lines_keep_their_endings() {
        let diff = DIFF.replace(" one\n", " one\r\n").replace("+TWO\n", "+TWO\r\n");
        let mut selection = LineSelection::new(parse_diff(&diff));
        assert_eq!(selection.diff.lines[index_of(&selection, " one")].text, " one");
        selection.toggle_line(index_of(&selection, "-two"));
        let patch = patch("f.txt", &selection).unwrap();
        assert!(patch.ends_with("@@ -1,3 +1,3 @@\n one\r\n two\n+TWO\r\n three\n"), "{patch:?}");
    }

    #[test]
    fn a_partly_chosen_deletion_becomes_a_modification() {
        let diff = "diff --git a/d.txt b/d.txt\ndeleted file mode 100644\nindex 1..0\n--- a/d.txt\n\
                    +++ /dev/null\n@@ -1,2 +0,0 @@\n-a\n-b\n";
        let mut selection = LineSelection::new(parse_diff(diff));
        selection.toggle_line(index_of(&selection, "-b"));
        assert_eq!(
            patch("d.txt", &selection).unwrap(),
            "diff --git a/d.txt b/d.txt\n--- a/d.txt\n+++ b/d.txt\n@@ -1,2 +0,0 @@\n-a\n b\n"
        );
    }
}
