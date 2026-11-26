use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    /// `diff --git`, `index`, `---`, `+++` and similar lines.
    Header,
    /// `@@ -a,b +c,d @@`
    Hunk,
    Context,
    Added,
    Removed,
    /// `\ No newline at end of file`
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub text: String,
    pub old_number: Option<u32>,
    pub new_number: Option<u32>,
    /// The line ended in `\r\n`. `text` never holds the `\r`; a partial commit puts it back.
    pub crlf: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    pub lines: Vec<DiffLine>,
    pub is_binary: bool,
    pub is_truncated: bool,
    /// For images: the picture before and after the change, shown instead of "binary file".
    pub image: Option<ImagePair>,
}

/// An image decoded for display, shrunk to at most `MAX_PREVIEW_SIDE` pixels on its longer side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewImage {
    /// The real size of the image.
    pub width: u32,
    pub height: u32,
    pub file_size: usize,
    /// RGBA pixels of the shown size.
    pub shown_width: u32,
    pub shown_height: u32,
    pub rgba: Arc<Vec<u8>>,
}

/// One side of an image change: the picture, or why it can't be shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSide {
    /// The file doesn't exist on this side (added or deleted).
    Missing,
    Image(PreviewImage),
    /// It exists but can't be shown (too big, unsupported or damaged).
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePair {
    pub before: ImageSide,
    pub after: ImageSide,
}

impl DiffLineKind {
    /// Added and removed lines are the ones that can be left out of a commit.
    pub fn is_change(self) -> bool {
        matches!(self, Self::Added | Self::Removed)
    }
}

impl Diff {
    /// Whether single lines of this diff can be picked for a commit.
    pub fn can_pick_lines(&self) -> bool {
        !self.is_binary && !self.is_truncated && self.lines.iter().any(|line| line.kind.is_change())
    }

    /// The changed lines of the hunk whose `@@` line is at `hunk`, or of the whole diff when it has
    /// no `@@` lines (a new file).
    pub fn hunk_changes(&self, hunk: usize) -> Vec<usize> {
        let start = if self.lines.get(hunk).is_some_and(|line| line.kind == DiffLineKind::Hunk) {
            hunk + 1
        } else {
            0
        };
        self.lines
            .iter()
            .enumerate()
            .skip(start)
            .take_while(|(_, line)| line.kind != DiffLineKind::Hunk)
            .filter(|(_, line)| line.kind.is_change())
            .map(|(index, _)| index)
            .collect()
    }

    pub fn all_changes(&self) -> BTreeSet<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.kind.is_change())
            .map(|(index, _)| index)
            .collect()
    }
}

/// Lines left out of the next commit for one file (GitHub Desktop's line selection). `diff` is the
/// diff the choice was made on: if the file changes, the choice no longer applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSelection {
    pub diff: Diff,
    /// Indexes into `diff.lines` of added or removed lines left out.
    pub excluded: BTreeSet<usize>,
}

/// A click in the diff's gutter: one line, or the whole hunk at an `@@` line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineToggle {
    Line(usize),
    Hunk(usize),
}

impl LineSelection {
    pub fn apply(&mut self, toggle: LineToggle) {
        match toggle {
            LineToggle::Line(index) => self.toggle_line(index),
            LineToggle::Hunk(index) => self.toggle_hunk(index),
        }
    }

    pub fn new(diff: Diff) -> Self {
        Self { diff, excluded: BTreeSet::new() }
    }

    /// Includes or leaves out one changed line.
    pub fn toggle_line(&mut self, index: usize) {
        if self.diff.lines.get(index).is_some_and(|line| line.kind.is_change())
            && !self.excluded.remove(&index)
        {
            self.excluded.insert(index);
        }
    }

    /// Leaves out a whole hunk when all of it is included, otherwise includes all of it.
    pub fn toggle_hunk(&mut self, hunk: usize) {
        let lines = self.diff.hunk_changes(hunk);
        if lines.iter().any(|index| self.excluded.contains(index)) {
            for index in &lines {
                self.excluded.remove(index);
            }
        } else {
            self.excluded.extend(lines);
        }
    }

    pub fn includes_nothing(&self) -> bool {
        self.excluded == self.diff.all_changes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: DiffLineKind) -> DiffLine {
        DiffLine { kind, text: String::new(), old_number: None, new_number: None, crlf: false }
    }

    fn two_hunks() -> Diff {
        use DiffLineKind::*;
        let kinds = [Header, Hunk, Context, Removed, Added, Hunk, Added, Context];
        Diff { lines: kinds.into_iter().map(line).collect(), ..Diff::default() }
    }

    #[test]
    fn toggles_lines_and_hunks() {
        let mut selection = LineSelection::new(two_hunks());
        selection.toggle_line(2); // context: ignored
        assert!(selection.excluded.is_empty());
        selection.toggle_line(3);
        assert_eq!(selection.excluded, [3].into());
        selection.toggle_hunk(1); // partly excluded, so all of it comes back
        assert!(selection.excluded.is_empty());
        selection.toggle_hunk(1);
        assert_eq!(selection.excluded, [3, 4].into());
        selection.toggle_hunk(5);
        assert!(selection.includes_nothing());
    }
}
