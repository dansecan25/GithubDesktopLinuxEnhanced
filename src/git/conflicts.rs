//! Resolving merge conflicts inside the app: each conflict block can take your side, theirs or
//! both, or the whole file can come from one side. Then the file is marked as resolved.

use std::path::Path;

use super::runner::{GitError, GitResult, run};
use crate::models::{ConflictBlock, ConflictChoice, ConflictFile, Segment};

const OURS: &str = "<<<<<<<";
const BASE: &str = "|||||||";
const SPLIT: &str = "=======";
const THEIRS: &str = ">>>>>>>";

/// Splits a file into plain text and conflict blocks. Lines keep their line endings, so a resolved
/// file is written back byte for byte except for the chosen block.
pub fn parse(content: &str) -> ConflictFile {
    let mut segments = Vec::new();
    let mut text = String::new();
    let mut lines = content.split_inclusive('\n');
    while let Some(line) = lines.next() {
        let Some(ours_label) = marker(line, OURS) else {
            text.push_str(line);
            continue;
        };
        let mut block = ConflictBlock { ours_label, ..ConflictBlock::default() };
        let mut raw = line.to_owned();
        // 0: ours, 1: base (diff3 style), 2: theirs
        let mut part = 0;
        let mut closed = false;
        for line in lines.by_ref() {
            raw.push_str(line);
            if marker(line, BASE).is_some() && part == 0 {
                part = 1;
                block.base = Some(String::new());
            } else if is_split(line) && part < 2 {
                part = 2;
            } else if let Some(label) = marker(line, THEIRS).filter(|_| part == 2) {
                block.theirs_label = label;
                block.raw = std::mem::take(&mut raw);
                closed = true;
                break;
            } else {
                match part {
                    0 => block.ours.push_str(line),
                    1 => block.base.get_or_insert_default().push_str(line),
                    _ => block.theirs.push_str(line),
                }
            }
        }
        if !closed {
            // An unfinished block isn't a conflict: keep it as it is.
            text.push_str(&raw);
            continue;
        }
        if !text.is_empty() {
            segments.push(Segment::Text(std::mem::take(&mut text)));
        }
        segments.push(Segment::Conflict(block));
    }
    if !text.is_empty() {
        segments.push(Segment::Text(text));
    }
    ConflictFile { segments }
}

/// `<<<<<<< HEAD` → `HEAD`.
fn marker(line: &str, marker: &str) -> Option<String> {
    let rest = line.strip_prefix(marker)?;
    let rest = rest.trim_end_matches(['\n', '\r']);
    (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim().to_owned())
}

fn is_split(line: &str) -> bool {
    line.trim_end_matches(['\n', '\r']) == SPLIT
}

/// The file with conflict number `index` replaced by the chosen side.
pub fn resolve(file: &ConflictFile, index: usize, choice: ConflictChoice) -> String {
    let mut conflict = 0;
    let mut out = String::new();
    for segment in &file.segments {
        match segment {
            Segment::Text(text) => out.push_str(text),
            Segment::Conflict(block) => {
                if conflict == index {
                    out.push_str(&block.chosen(choice));
                } else {
                    out.push_str(&block.raw);
                }
                conflict += 1;
            }
        }
    }
    out
}

pub fn read(repo: &Path, path: &str) -> GitResult<ConflictFile> {
    let content = std::fs::read(repo.join(path))
        .map_err(|error| GitError::new(format!("Could not read {path}: {error}")))?;
    Ok(parse(&String::from_utf8_lossy(&content)))
}

/// Resolves one conflict block in the file on disk.
pub fn choose(repo: &Path, path: &str, index: usize, choice: ConflictChoice) -> GitResult<()> {
    let file = read(repo, path)?;
    std::fs::write(repo.join(path), resolve(&file, index, choice))
        .map_err(|error| GitError::new(format!("Could not write {path}: {error}")))
}

/// Takes the whole file from one side and marks it resolved. When that side deleted the file, the
/// file is deleted.
pub fn take_side(repo: &Path, path: &str, ours: bool) -> GitResult<()> {
    let stage = if ours { "2" } else { "3" };
    let stages = run(repo, &["ls-files", "--unmerged", "--", path])?;
    let side_exists = stages.lines().any(|line| line.split_whitespace().nth(2) == Some(stage));
    if side_exists {
        run(repo, &["checkout", if ours { "--ours" } else { "--theirs" }, "--", path])?;
        run(repo, &["add", "--", path])?;
    } else {
        run(repo, &["rm", "--quiet", "--", path])?;
    }
    Ok(())
}

/// Marks a file whose conflicts were all resolved, by adding it.
pub fn mark_resolved(repo: &Path, path: &str) -> GitResult<()> {
    if read(repo, path).is_ok_and(|file| file.conflict_count() > 0) {
        return Err(GitError::new(format!("{path} still has conflicts to resolve")));
    }
    run(repo, &["add", "--", path])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "top\n<<<<<<< HEAD\nmine\n=======\ntheirs\n>>>>>>> feature\nmiddle\n\
                        <<<<<<< HEAD\nmine 2\r\n||||||| base\nbase 2\r\n=======\ntheirs 2\r\n>>>>>>> feature\nend\n";

    #[test]
    fn parses_blocks_with_labels_and_base() {
        let file = parse(FILE);
        assert_eq!(file.conflict_count(), 2);
        let blocks = file.blocks();
        assert_eq!(blocks[0].ours, "mine\n");
        assert_eq!(blocks[0].theirs, "theirs\n");
        assert_eq!(blocks[0].ours_label, "HEAD");
        assert_eq!(blocks[0].theirs_label, "feature");
        assert_eq!(blocks[1].base.as_deref(), Some("base 2\r\n"));
        // Nothing chosen: the file is unchanged.
        let raw: String = file.segments.iter().map(Segment::raw).collect();
        assert_eq!(raw, FILE);
    }

    #[test]
    fn resolves_one_block_at_a_time() {
        let file = parse(FILE);
        let once = resolve(&file, 0, ConflictChoice::Theirs);
        assert!(once.starts_with("top\ntheirs\nmiddle\n<<<<<<< HEAD\n"));
        let twice = resolve(&parse(&once), 0, ConflictChoice::Both);
        assert_eq!(twice, "top\ntheirs\nmiddle\nmine 2\r\ntheirs 2\r\nend\n");
        assert_eq!(parse(&twice).conflict_count(), 0);
    }

    #[test]
    fn unfinished_markers_are_left_alone() {
        let text = "a\n<<<<<<< HEAD\nb\n";
        let file = parse(text);
        assert_eq!(file.conflict_count(), 0);
        assert_eq!(resolve(&file, 0, ConflictChoice::Ours), text);
    }
}
