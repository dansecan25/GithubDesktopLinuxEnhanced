use super::{Branch, FileChange, StashEntry};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchStatus {
    /// `None` when HEAD is detached.
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// False in a freshly initialised repository.
    pub has_commits: bool,
}

/// Everything the main window needs about a repository, loaded in one background task.
#[derive(Debug, Clone, Default)]
pub struct RepoSnapshot {
    pub status: BranchStatus,
    pub changes: Vec<FileChange>,
    pub branches: Vec<Branch>,
    pub stashes: Vec<StashEntry>,
    pub remote: Option<String>,
}
