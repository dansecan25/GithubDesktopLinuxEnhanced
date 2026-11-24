#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    /// Reference such as `stash@{0}`.
    pub reference: String,
    pub message: String,
}
