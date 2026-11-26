use super::{Branch, FileChange, InProgress, RebaseState, StashEntry, Tag};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchStatus {
    /// `None` when HEAD is detached.
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// False in a freshly initialised repository.
    pub has_commits: bool,
    /// The last commit's SHA, `None` before the first commit.
    pub head_sha: Option<String>,
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
    /// While a squash merge waits to be committed, its prepared message.
    pub squash_message: Option<String>,
    /// While a rebase is stopped part way.
    pub rebase: Option<RebaseState>,
    /// The last commit is already on a remote branch, so amending or undoing it needs a force push.
    pub head_pushed: bool,
}

impl RepoSnapshot {
    /// The merge, squash or rebase waiting for the user, if any.
    pub fn in_progress(&self) -> Option<InProgress> {
        if let Some(rebase) = &self.rebase {
            return Some(InProgress::Rebase(rebase.clone()));
        }
        if self.merge_message.is_some() {
            return Some(InProgress::Merge);
        }
        self.squash_message.is_some().then_some(InProgress::Squash)
    }

    /// Amending, undoing or starting another merge has to wait while one is in progress.
    pub fn is_busy_merging(&self) -> bool {
        self.in_progress().is_some()
    }
}
