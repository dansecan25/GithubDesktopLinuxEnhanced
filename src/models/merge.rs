/// What merging a branch into the current one would do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergePreview {
    /// Commits the branch has that the current branch doesn't.
    pub commits: usize,
    /// Commits the current branch has that the branch doesn't: what a rebase replays.
    pub ahead: usize,
    /// Files that would conflict. `None` when git is too old to tell before merging.
    pub conflicts: Option<Vec<String>>,
}

/// How the merge dialog brings another branch in, as in GitHub Desktop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeMethod {
    /// A merge commit with both histories.
    #[default]
    Merge,
    /// All of the branch's changes as one new commit.
    Squash,
    /// The current branch's commits replayed on top of the branch.
    Rebase,
}

/// A rebase stopped part way, usually on conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseState {
    /// The branch being rebased.
    pub branch: String,
    /// What it is rebased onto: a branch name, or a short SHA.
    pub onto: String,
    /// The commit being applied (1-based) and how many there are.
    pub step: usize,
    pub total: usize,
}

/// A merge, squash or rebase that stopped and is waiting for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InProgress {
    Merge,
    Squash,
    Rebase(RebaseState),
}
