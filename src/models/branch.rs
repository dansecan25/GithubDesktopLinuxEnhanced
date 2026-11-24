#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// Short name: `main` for local branches, `origin/main` for remote ones.
    pub name: String,
    pub is_remote: bool,
    pub is_current: bool,
    pub upstream: Option<String>,
}
