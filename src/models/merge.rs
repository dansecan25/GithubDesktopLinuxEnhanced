/// What merging a branch into the current one would do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MergePreview {
    /// Commits the branch has that the current branch doesn't.
    pub commits: usize,
    /// Files that would conflict. `None` when git is too old to tell before merging.
    pub conflicts: Option<Vec<String>>,
}
