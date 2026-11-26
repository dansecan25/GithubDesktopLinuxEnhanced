use super::{Diff, FileChange, MergeMethod};

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
    /// The head commit, for the checks, reviews and line comments.
    pub head_sha: String,
    /// False for pull requests from a fork.
    pub same_repository: bool,
    pub draft: bool,
    pub url: String,
    pub created_at: String,
}

/// One entry of the conversation: a comment, or a review with its verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullComment {
    pub author: String,
    pub body: String,
    pub created_at: String,
    /// For reviews: "approved", "requested changes" or "reviewed".
    pub verdict: Option<String>,
}

/// Which side of the diff a line comment is on: the old file (removed lines) or the new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Left,
    Right,
}

impl DiffSide {
    pub fn api_name(self) -> &'static str {
        match self {
            Self::Left => "LEFT",
            Self::Right => "RIGHT",
        }
    }
}

/// A comment on a line of the pull request's diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewComment {
    pub id: u64,
    /// The first comment of the thread this one answers.
    pub reply_to: Option<u64>,
    pub path: String,
    /// `None` when the line is gone from the current diff (an outdated comment).
    pub line: Option<u32>,
    pub side: DiffSide,
    pub author: String,
    pub body: String,
    pub created_at: String,
}

/// A file the pull request changes, with its diff. GitHub leaves out the diff of binary and very
/// large files; then `diff` is empty and `diff_missing` is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullFile {
    pub change: FileChange,
    pub additions: u32,
    pub deletions: u32,
    pub diff: Diff,
    pub diff_missing: bool,
}

/// Whether GitHub can merge the pull request now.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mergeability {
    /// `None` while GitHub is still working it out.
    pub mergeable: Option<bool>,
    /// GitHub's `mergeable_state`: clean, dirty (conflicts), blocked, behind, unstable, draft…
    pub state: String,
    pub merged: bool,
}

/// How the pull request's checks (GitHub Actions and other CI) are doing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Checks {
    pub passed: usize,
    pub failed: usize,
    pub pending: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PullDetail {
    /// Comments and reviews, oldest first.
    pub comments: Vec<PullComment>,
    pub checks: Checks,
    pub files: Vec<PullFile>,
    pub review_comments: Vec<ReviewComment>,
    pub mergeability: Mergeability,
    /// GitHub's GraphQL id, needed to change the draft state.
    pub node_id: String,
}

impl PullDetail {
    pub fn file(&self, path: &str) -> Option<&PullFile> {
        self.files.iter().find(|file| file.change.path == path)
    }

    /// Line comments on `path`, oldest first.
    pub fn comments_on<'a>(&'a self, path: &'a str) -> impl Iterator<Item = &'a ReviewComment> + 'a {
        self.review_comments.iter().filter(move |comment| comment.path == path)
    }
}

/// The open pull requests, the repository's default branch (the usual base for a new one) and the
/// merge methods it allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullList {
    pub pulls: Vec<PullRequest>,
    pub default_branch: String,
    pub merge_methods: Vec<MergeMethod>,
}

/// A review's verdict, as GitHub's "Review changes" offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReviewEvent {
    #[default]
    Comment,
    Approve,
    RequestChanges,
}

impl ReviewEvent {
    pub fn api_name(self) -> &'static str {
        match self {
            Self::Comment => "COMMENT",
            Self::Approve => "APPROVE",
            Self::RequestChanges => "REQUEST_CHANGES",
        }
    }

    /// Approving can be sent without a message; the others need one.
    pub fn needs_body(self) -> bool {
        self != Self::Approve
    }
}

/// The line a new line comment goes on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineTarget {
    pub path: String,
    pub line: u32,
    pub side: DiffSide,
    /// When the line already has a thread, the new comment answers its first comment.
    pub reply_to: Option<u64>,
}

/// What was sent to GitHub for a pull request, so its result clears the right form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullSend {
    Comment,
    LineComment,
    Review,
    Edit,
    Draft,
    Merge,
}
