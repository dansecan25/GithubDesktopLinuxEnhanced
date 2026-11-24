#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSummary {
    pub sha: String,
    pub short_sha: String,
    pub summary: String,
    pub body: String,
    pub author: String,
    pub relative_date: String,
}
