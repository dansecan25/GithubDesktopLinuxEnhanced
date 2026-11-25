/// One `<<<<<<<` … `>>>>>>>` block. Each side keeps its line endings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConflictBlock {
    /// The current branch's side (`<<<<<<< HEAD`).
    pub ours: String,
    /// The common ancestor, only with `merge.conflictStyle = diff3` or `zdiff3`.
    pub base: Option<String>,
    /// The incoming side (`>>>>>>> feature`).
    pub theirs: String,
    pub ours_label: String,
    pub theirs_label: String,
    /// The block exactly as in the file, markers included.
    pub raw: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    Ours,
    Theirs,
    /// Yours, then theirs.
    Both,
}

impl ConflictBlock {
    pub fn chosen(&self, choice: ConflictChoice) -> String {
        match choice {
            ConflictChoice::Ours => self.ours.clone(),
            ConflictChoice::Theirs => self.theirs.clone(),
            ConflictChoice::Both => {
                let separator = if self.ours.is_empty() || self.ours.ends_with('\n') { "" } else { "\n" };
                format!("{}{separator}{}", self.ours, self.theirs)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Conflict(ConflictBlock),
}

impl Segment {
    pub fn raw(&self) -> &str {
        match self {
            Self::Text(text) => text,
            Self::Conflict(block) => &block.raw,
        }
    }
}

/// A conflicted file split into plain text and conflict blocks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConflictFile {
    pub segments: Vec<Segment>,
}

impl ConflictFile {
    pub fn blocks(&self) -> Vec<&ConflictBlock> {
        self.segments
            .iter()
            .filter_map(|segment| match segment {
                Segment::Conflict(block) => Some(block),
                Segment::Text(_) => None,
            })
            .collect()
    }

    pub fn conflict_count(&self) -> usize {
        self.blocks().len()
    }
}
