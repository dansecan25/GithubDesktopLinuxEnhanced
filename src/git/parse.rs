//! Pure parsers for git's machine-readable output. No process calls here, so everything is testable.

use crate::constants::git::{
    BINARY_SNIFF_BYTES, BRANCH_STASH_PREFIX, BRANCH_STASH_SUFFIX, FIELD_SEP, MAX_DIFF_BYTES,
    MAX_DIFF_LINE_BYTES, RECORD_SEP,
};
use crate::models::{
    Branch, BranchStatus, ChangeKind, CommitSummary, Diff, DiffLine, DiffLineKind, FileChange, StashEntry,
    Tag,
};

/// Parses `git status --porcelain=v2 --branch -z`.
pub fn parse_status(output: &str) -> (BranchStatus, Vec<FileChange>) {
    let mut status = BranchStatus { has_commits: true, ..BranchStatus::default() };
    let mut changes = Vec::new();
    let mut entries = output.split('\0').filter(|entry| !entry.is_empty());

    while let Some(entry) = entries.next() {
        if let Some(header) = entry.strip_prefix("# ") {
            apply_branch_header(&mut status, header);
            continue;
        }
        let Some((tag, rest)) = entry.split_once(' ') else { continue };
        let change = match tag {
            "1" => parse_ordinary_entry(rest),
            // Renames carry the original path as the next NUL-separated entry.
            "2" => parse_ordinary_entry_fields(rest, 8).map(|(xy, path)| FileChange {
                path,
                original_path: entries.next().map(str::to_owned),
                kind: kind_from_xy(xy),
            }),
            "u" => parse_ordinary_entry_fields(rest, 9).map(|(_, path)| FileChange {
                path,
                original_path: None,
                kind: ChangeKind::Conflicted,
            }),
            "?" => {
                Some(FileChange { path: rest.to_owned(), original_path: None, kind: ChangeKind::Untracked })
            }
            _ => None,
        };
        changes.extend(change);
    }
    (status, changes)
}

fn apply_branch_header(status: &mut BranchStatus, header: &str) {
    let Some((key, value)) = header.split_once(' ') else { return };
    match key {
        "branch.oid" => status.has_commits = value != "(initial)",
        "branch.head" => status.head = (value != "(detached)").then(|| value.to_owned()),
        "branch.upstream" => status.upstream = Some(value.to_owned()),
        "branch.ab" => {
            for part in value.split_whitespace() {
                if let Some(ahead) = part.strip_prefix('+') {
                    status.ahead = ahead.parse().unwrap_or(0);
                } else if let Some(behind) = part.strip_prefix('-') {
                    status.behind = behind.parse().unwrap_or(0);
                }
            }
        }
        _ => {}
    }
}

fn parse_ordinary_entry(rest: &str) -> Option<FileChange> {
    parse_ordinary_entry_fields(rest, 7).map(|(xy, path)| FileChange {
        path,
        original_path: None,
        kind: kind_from_xy(xy),
    })
}

/// Splits `XY <fields…> <path>` where the path is the last of `field_count` space-separated fields
/// and may itself contain spaces.
fn parse_ordinary_entry_fields(rest: &str, field_count: usize) -> Option<(&str, String)> {
    let mut parts = rest.splitn(field_count + 1, ' ');
    let xy = parts.next()?;
    let path = parts.nth(field_count - 1)?;
    Some((xy, path.to_owned()))
}

/// Picks one kind from the index (X) and worktree (Y) letters, preferring the index letter.
fn kind_from_xy(xy: &str) -> ChangeKind {
    let mut letters = xy.chars();
    let index = letters.next().unwrap_or('.');
    let worktree = letters.next().unwrap_or('.');
    let chosen = if index != '.' { index } else { worktree };
    ChangeKind::from_status_letter(chosen).unwrap_or(ChangeKind::Modified)
}

/// Parses `git diff-tree -r -z --name-status`.
pub fn parse_name_status(output: &str) -> Vec<FileChange> {
    let mut changes = Vec::new();
    let mut fields = output.split('\0').filter(|field| !field.is_empty());
    while let Some(code) = fields.next() {
        let Some(letter) = code.chars().next() else { continue };
        let kind = ChangeKind::from_status_letter(letter).unwrap_or(ChangeKind::Modified);
        let has_original = matches!(kind, ChangeKind::Renamed | ChangeKind::Copied);
        let original_path = if has_original { fields.next().map(str::to_owned) } else { None };
        let Some(path) = fields.next() else { break };
        changes.push(FileChange { path: path.to_owned(), original_path, kind });
    }
    changes
}

/// The `--format` used by [`parse_log`]: sha, short sha, author, email, relative date, subject, body.
pub fn log_format() -> String {
    format!("%H{FIELD_SEP}%h{FIELD_SEP}%an{FIELD_SEP}%ae{FIELD_SEP}%ar{FIELD_SEP}%s{FIELD_SEP}%b{RECORD_SEP}")
}

pub fn parse_log(output: &str) -> Vec<CommitSummary> {
    output
        .split(RECORD_SEP)
        .filter_map(|record| {
            let mut fields = record.trim_start_matches('\n').splitn(7, FIELD_SEP);
            let sha = fields.next()?.trim();
            if sha.is_empty() {
                return None;
            }
            Some(CommitSummary {
                sha: sha.to_owned(),
                short_sha: fields.next()?.to_owned(),
                author: fields.next()?.to_owned(),
                author_email: fields.next()?.to_owned(),
                relative_date: fields.next()?.to_owned(),
                summary: fields.next()?.to_owned(),
                body: fields.next().unwrap_or_default().trim().to_owned(),
            })
        })
        .collect()
}

/// The `--format` used by [`parse_branches`].
pub fn branch_format() -> String {
    format!("%(refname){FIELD_SEP}%(refname:short){FIELD_SEP}%(HEAD){FIELD_SEP}%(upstream:short)")
}

pub fn parse_branches(output: &str) -> Vec<Branch> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(FIELD_SEP);
            let full_name = fields.next()?;
            let name = fields.next()?;
            let is_current = fields.next()? == "*";
            let upstream = fields.next().filter(|value| !value.is_empty()).map(str::to_owned);
            // `refs/remotes/origin/HEAD` is a pointer, not a branch.
            if full_name.ends_with("/HEAD") {
                return None;
            }
            Some(Branch {
                name: name.to_owned(),
                is_remote: full_name.starts_with("refs/remotes/"),
                is_current,
                upstream,
            })
        })
        .collect()
}

/// The `--format` used by [`parse_stashes`].
pub fn stash_format() -> String {
    format!("%gd{FIELD_SEP}%s")
}

pub fn parse_stashes(output: &str) -> Vec<StashEntry> {
    output
        .lines()
        .filter_map(|line| {
            let (reference, message) = line.split_once(FIELD_SEP)?;
            Some(StashEntry {
                reference: reference.to_owned(),
                message: message.to_owned(),
                branch: stash_branch(message),
            })
        })
        .collect()
}

/// The branch in a stash subject such as `On main: !!GitHub_Desktop<main>`.
fn stash_branch(subject: &str) -> Option<String> {
    let start = subject.find(BRANCH_STASH_PREFIX)? + BRANCH_STASH_PREFIX.len();
    let branch = subject[start..].strip_suffix(BRANCH_STASH_SUFFIX)?;
    (!branch.is_empty()).then(|| branch.to_owned())
}

/// The `--format` used by [`parse_tags`]: name, object, and the commit an annotated tag points to.
pub fn tag_format() -> String {
    format!("%(refname:short){FIELD_SEP}%(objectname){FIELD_SEP}%(*objectname)")
}

pub fn parse_tags(output: &str) -> Vec<Tag> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(FIELD_SEP);
            let name = fields.next()?.to_owned();
            let object = fields.next()?;
            let peeled = fields.next().filter(|value| !value.is_empty());
            Some(Tag { name, commit: peeled.unwrap_or(object).to_owned() })
        })
        .collect()
}

/// Parses unified diff output into numbered lines.
pub fn parse_diff(output: &str) -> Diff {
    let (text, is_truncated) = truncate_at_char_boundary(output, MAX_DIFF_BYTES);
    let mut diff = Diff { is_truncated, ..Diff::default() };
    let mut old_line = 0;
    let mut new_line = 0;
    let mut in_hunk = false;

    for raw in text.lines() {
        if raw.starts_with("Binary files ") || raw == "GIT binary patch" {
            diff.is_binary = true;
        }
        let (kind, old_number, new_number) = if let Some((old_start, new_start)) = parse_hunk_header(raw) {
            in_hunk = true;
            old_line = old_start;
            new_line = new_start;
            (DiffLineKind::Hunk, None, None)
        } else if raw.starts_with("diff --git") {
            in_hunk = false;
            (DiffLineKind::Header, None, None)
        } else if !in_hunk {
            (DiffLineKind::Header, None, None)
        } else if raw.starts_with('+') {
            new_line += 1;
            (DiffLineKind::Added, None, Some(new_line - 1))
        } else if raw.starts_with('-') {
            old_line += 1;
            (DiffLineKind::Removed, Some(old_line - 1), None)
        } else if raw.starts_with('\\') {
            (DiffLineKind::Note, None, None)
        } else {
            old_line += 1;
            new_line += 1;
            (DiffLineKind::Context, Some(old_line - 1), Some(new_line - 1))
        };
        let (text, cut) = shorten_line(raw);
        diff.is_truncated |= cut;
        diff.lines.push(DiffLine { kind, text, old_number, new_number });
    }
    diff
}

/// Returns the starting old and new line numbers of `@@ -a,b +c,d @@`.
fn parse_hunk_header(line: &str) -> Option<(u32, u32)> {
    let rest = line.strip_prefix("@@ -")?;
    let (ranges, _) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(" +")?;
    let start = |range: &str| range.split(',').next()?.parse::<u32>().ok();
    Some((start(old)?, start(new)?))
}

/// Builds an all-added diff for a file git doesn't track yet.
pub fn diff_for_new_file(content: &str) -> Diff {
    let (text, mut is_truncated) = truncate_at_char_boundary(content, MAX_DIFF_BYTES);
    is_truncated |= text.lines().any(|line| line.len() > MAX_DIFF_LINE_BYTES);
    let lines = text
        .lines()
        .zip(1..)
        .map(|(line, number)| DiffLine {
            kind: DiffLineKind::Added,
            text: format!("+{}", shorten_line(line).0),
            old_number: None,
            new_number: Some(number),
        })
        .collect();
    Diff { lines, is_binary: false, is_truncated, image: None }
}

/// Like git: a NUL byte near the start means binary. Content that isn't UTF-8 is treated as binary
/// too, because showing it as text produces unreadable, very long lines.
pub fn looks_binary(bytes: &[u8]) -> bool {
    if bytes[..bytes.len().min(BINARY_SNIFF_BYTES)].contains(&0) {
        return true;
    }
    match std::str::from_utf8(bytes) {
        Ok(_) => false,
        // A multi-byte character cut by the read limit at the very end is still text.
        Err(error) => error.error_len().is_some() || error.valid_up_to() + 4 < bytes.len(),
    }
}

/// Cuts lines longer than [`MAX_DIFF_LINE_BYTES`], so a minified or generated file can't make
/// every frame lay out megabytes of text.
fn shorten_line(line: &str) -> (String, bool) {
    if line.len() <= MAX_DIFF_LINE_BYTES {
        return (line.to_owned(), false);
    }
    let (start, _) = truncate_at_char_boundary(line, MAX_DIFF_LINE_BYTES);
    (format!("{start}…"), true)
}

fn truncate_at_char_boundary(text: &str, max_bytes: usize) -> (&str, bool) {
    if text.len() <= max_bytes {
        return (text, false);
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_reads_branch_headers_and_entries() {
        let output = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0\
            # branch.ab +2 -1\0\
            1 .M N... 100644 100644 100644 aaa bbb src/my file.rs\0\
            1 A. N... 000000 100644 100644 000 ccc new.rs\0\
            2 R. N... 100644 100644 100644 ddd eee R100 renamed.rs\0old.rs\0\
            u UU N... 100644 100644 100644 100644 a b c conflict.rs\0\
            ? untracked.txt\0";
        let (status, changes) = parse_status(output);

        assert_eq!(status.head.as_deref(), Some("main"));
        assert_eq!(status.upstream.as_deref(), Some("origin/main"));
        assert_eq!((status.ahead, status.behind), (2, 1));
        assert!(status.has_commits);

        let summary: Vec<_> = changes.iter().map(|c| (c.path.as_str(), c.kind)).collect();
        assert_eq!(
            summary,
            vec![
                ("src/my file.rs", ChangeKind::Modified),
                ("new.rs", ChangeKind::Added),
                ("renamed.rs", ChangeKind::Renamed),
                ("conflict.rs", ChangeKind::Conflicted),
                ("untracked.txt", ChangeKind::Untracked),
            ]
        );
        assert_eq!(changes[2].original_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn status_detects_initial_and_detached() {
        let (status, _) = parse_status("# branch.oid (initial)\0# branch.head (detached)\0");
        assert!(!status.has_commits);
        assert_eq!(status.head, None);
    }

    #[test]
    fn name_status_handles_renames() {
        let changes = parse_name_status("M\0a.rs\0R087\0old.rs\0new.rs\0D\0gone.rs\0");
        assert_eq!(changes.len(), 3);
        assert_eq!(changes[1].path, "new.rs");
        assert_eq!(changes[1].original_path.as_deref(), Some("old.rs"));
        assert_eq!(changes[2].kind, ChangeKind::Deleted);
    }

    #[test]
    fn log_splits_records_with_multiline_bodies() {
        let s = FIELD_SEP;
        let r = RECORD_SEP;
        let output = format!(
            "aaa{s}a{s}Ana{s}ana@x.y{s}2 days ago{s}First{s}line one\nline two\n{r}\nbbb{s}b{s}Bo{s}bo@x.y{s}now{s}Second{s}{r}\n"
        );
        let commits = parse_log(&output);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].body, "line one\nline two");
        assert_eq!(commits[1].summary, "Second");
        assert_eq!(commits[1].body, "");
        assert_eq!(commits[1].author_email, "bo@x.y");
    }

    #[test]
    fn branches_skip_remote_head_pointer() {
        let s = FIELD_SEP;
        let output = format!(
            "refs/heads/main{s}main{s}*{s}origin/main\nrefs/remotes/origin/HEAD{s}origin{s} {s}\n\
             refs/remotes/origin/dev{s}origin/dev{s} {s}\n"
        );
        let branches = parse_branches(&output);
        assert_eq!(branches.len(), 2);
        assert!(branches[0].is_current && !branches[0].is_remote);
        assert_eq!(branches[0].upstream.as_deref(), Some("origin/main"));
        assert!(branches[1].is_remote);
    }

    #[test]
    fn diff_numbers_lines() {
        let output = "diff --git a/f b/f\nindex 1..2 100644\n--- a/f\n+++ b/f\n@@ -3,3 +3,3 @@ fn\n ctx\n-old\n+new\n ctx2\n";
        let diff = parse_diff(output);
        let added = diff.lines.iter().find(|l| l.kind == DiffLineKind::Added).unwrap();
        let removed = diff.lines.iter().find(|l| l.kind == DiffLineKind::Removed).unwrap();
        assert_eq!(added.new_number, Some(4));
        assert_eq!(removed.old_number, Some(4));
        assert_eq!(diff.lines.last().unwrap().new_number, Some(5));
        // `---` and `+++` are headers, not changes.
        assert_eq!(diff.lines[2].kind, DiffLineKind::Header);
    }

    #[test]
    fn tags_use_the_peeled_commit_of_annotated_tags() {
        let s = FIELD_SEP;
        let tags = parse_tags(&format!("v1.0.0{s}tagobj{s}commit1\nlight{s}commit2{s}\n"));
        assert_eq!(tags[0], Tag { name: "v1.0.0".into(), commit: "commit1".into() });
        assert_eq!(tags[1].commit, "commit2");
    }

    #[test]
    fn diff_flags_binary() {
        assert!(parse_diff("diff --git a/x b/x\nBinary files a/x and b/x differ\n").is_binary);
    }

    #[test]
    fn binary_detection() {
        assert!(looks_binary(b"abc\0def"));
        assert!(looks_binary(&[0x25, 0x50, 0xff, 0xfe, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46]));
        assert!(!looks_binary("plain text ñ\n".as_bytes()));
        // A character cut at the end by the read limit.
        assert!(!looks_binary(&"ñ".as_bytes()[..1]));
    }

    #[test]
    fn very_long_lines_are_cut() {
        let long = "x".repeat(MAX_DIFF_LINE_BYTES * 3);
        let diff = diff_for_new_file(&long);
        assert!(diff.is_truncated);
        assert!(diff.lines[0].text.len() < MAX_DIFF_LINE_BYTES + 8);
    }

    #[test]
    fn truncation_keeps_utf8_valid() {
        let (text, cut) = truncate_at_char_boundary("ñññ", 3);
        assert!(cut);
        assert_eq!(text, "ñ");
    }

    #[test]
    fn stashes_know_their_branch() {
        let sep = FIELD_SEP;
        let output = format!(
            "stash@{{0}}{sep}On feature/x: !!GitHub_Desktop<feature/x>\nstash@{{1}}{sep}On main: wip\n"
        );
        let stashes = parse_stashes(&output);
        assert_eq!(stashes[0].branch.as_deref(), Some("feature/x"));
        assert_eq!(stashes[1].branch, None);
    }
}
