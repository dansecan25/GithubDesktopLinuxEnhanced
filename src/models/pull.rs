/// An open pull request, as listed in the Pull requests tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author: String,
    /// The branch with the changes, and the branch they go into.
    pub head_branch: String,
    pub base_branch: String,
    /// The head commit, for the checks.
    pub head_sha: String,
    /// False for pull requests from a fork.
    pub same_repository: bool,
    pub draft: bool,
    pub url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullComment {
    pub author: String,
    pub body: String,
    pub created_at: String,
}

/// How the pull request's checks (GitHub Actions and other CI) are doing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Checks {
    pub passed: usize,
    pub failed: usize,
    pub pending: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullDetail {
    pub comments: Vec<PullComment>,
    pub checks: Checks,
}

/// The open pull requests and the repository's default branch (the usual base for a new one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullList {
    pub pulls: Vec<PullRequest>,
    pub default_branch: String,
}
