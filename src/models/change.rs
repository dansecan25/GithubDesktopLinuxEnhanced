#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    TypeChanged,
    Untracked,
    Conflicted,
}

impl ChangeKind {
    /// Maps one porcelain status letter (`M`, `A`, `D`, …) to a kind.
    pub fn from_status_letter(letter: char) -> Option<Self> {
        match letter {
            'A' => Some(Self::Added),
            'M' => Some(Self::Modified),
            'D' => Some(Self::Deleted),
            'R' => Some(Self::Renamed),
            'C' => Some(Self::Copied),
            'T' => Some(Self::TypeChanged),
            'U' => Some(Self::Conflicted),
            '?' => Some(Self::Untracked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    /// Previous path for renames and copies.
    pub original_path: Option<String>,
    pub kind: ChangeKind,
}
