use super::{Branch, FileChange, StashEntry, Tag};

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
    /// Newest first.
    pub tags: Vec<Tag>,
    pub remote: Option<String>,
    /// URL of `remote`, for example `git@github.com:owner/repo.git`.
    pub remote_url: Option<String>,
    /// While a merge is in progress (stopped on conflicts), its prepared commit summary.
    pub merge_message: Option<String>,
}
