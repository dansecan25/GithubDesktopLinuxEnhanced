#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// Reference such as `stash@{0}`.
    pub reference: String,
    pub message: String,
    /// The branch the changes were left on, for stashes made when switching branches or with
    /// "Stash all changes". `None` for stashes made some other way.
    pub branch: Option<String>,
}
