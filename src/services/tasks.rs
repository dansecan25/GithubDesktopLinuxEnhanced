//! Runs git work on background threads so the window never freezes, and hands results back to the
//! UI thread through a channel.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use crate::constants::text;
use crate::git::GitResult;
use crate::github::device::DeviceCode;
use crate::models::{
    Branch, CommitSummary, ConflictFile, Diff, FileChange, MergePreview, ProjectVersion, PullDetail,
    PullList, PullSend, RepoSnapshot,
};
use crate::services::askpass::GitHubAccount;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Commit,
    Amend,
    UndoCommit,
    ForcePush,
    Fetch,
    Pull,
    Push,
    SwitchBranch,
    CreateBranch,
    DeleteBranch,
    Merge,
    AbortMerge,
    Rebase,
    AbortRebase,
    Resolve,
    Stash,
    RestoreStash,
    DropStash,
    Discard,
    Ignore,
    Publish,
    CreateTag,
    PushTag,
    DeleteTag,
}

impl OperationKind {
    /// Network operations, which can take long and are safe to stop part way.
    pub fn can_cancel(self) -> bool {
        matches!(
            self,
            Self::Fetch | Self::Pull | Self::Push | Self::ForcePush | Self::Publish | Self::PushTag
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Commit => text::COMMITTING,
            Self::Amend => text::AMENDING,
            Self::UndoCommit => text::UNDOING_COMMIT,
            Self::ForcePush => text::FORCE_PUSHING,
            Self::Fetch => text::FETCHING,
            Self::Pull => text::PULLING,
            Self::Push => text::PUSHING,
            Self::SwitchBranch => text::SWITCHING_BRANCH,
            Self::CreateBranch => text::CREATING_BRANCH,
            Self::DeleteBranch => text::DELETING_BRANCH,
            Self::Merge => text::MERGING,
            Self::AbortMerge => text::ABORTING_MERGE,
            Self::Rebase => text::REBASING,
            Self::AbortRebase => text::ABORTING_REBASE,
            Self::Resolve => text::RESOLVING,
            Self::Stash => text::STASHING,
            Self::RestoreStash => text::RESTORING_STASH,
            Self::DropStash => text::DROPPING_STASH,
            Self::Discard => text::DISCARDING,
            Self::Ignore => text::IGNORING,
            Self::Publish => text::PUBLISHING,
            Self::CreateTag => text::CREATING_TAG,
            Self::PushTag => text::PUSHING_TAG,
            Self::DeleteTag => text::DELETING_TAG,
        }
    }
}

pub enum TaskOutput {
    Snapshot {
        repo: PathBuf,
        result: GitResult<RepoSnapshot>,
    },
    WorkingDiff {
        repo: PathBuf,
        file: String,
        result: GitResult<Diff>,
    },
    History {
        repo: PathBuf,
        result: GitResult<Vec<CommitSummary>>,
    },
    CommitFiles {
        repo: PathBuf,
        sha: String,
        result: GitResult<Vec<FileChange>>,
    },
    CommitDiff {
        repo: PathBuf,
        sha: String,
        file: String,
        result: GitResult<Diff>,
    },
    Operation {
        repo: PathBuf,
        kind: OperationKind,
        result: GitResult<()>,
    },
    /// Result of adding, cloning or creating a repository: its root folder. The folder it's in
    /// becomes the starting folder for the next add, clone or new repository.
    RepositoryOpened {
        result: GitResult<PathBuf>,
    },
    Conflict {
        repo: PathBuf,
        file: String,
        result: GitResult<ConflictFile>,
    },
    Organizations(Result<Vec<String>, String>),
    /// Version files on the commit the tag dialog is about to tag.
    ProjectVersions {
        repo: PathBuf,
        target: String,
        result: GitResult<Vec<ProjectVersion>>,
    },
    Pulls {
        repo: PathBuf,
        result: Result<PullList, String>,
    },
    PullDetail {
        repo: PathBuf,
        number: u64,
        result: Result<PullDetail, String>,
    },
    /// A comment, review, edit, draft change or merge was sent for pull request `number`.
    PullSent {
        repo: PathBuf,
        number: u64,
        sent: PullSend,
        result: Result<(), String>,
    },
    PullCreated {
        repo: PathBuf,
        result: Result<u64, String>,
    },
    /// A pull request's branch was fetched; the app switches to it next.
    PullFetched {
        repo: PathBuf,
        result: GitResult<Branch>,
    },
    MergePreview {
        repo: PathBuf,
        branch: String,
        result: GitResult<MergePreview>,
    },
    /// The saved GitHub account, read from the keychain at startup.
    AccountLoaded(Result<Option<GitHubAccount>, String>),
    SignedIn(Result<GitHubAccount, String>),
    /// The code for browser sign-in, before the user enters it.
    DeviceCode(Result<DeviceCode, String>),
    SignedOut(Result<(), String>),
}

impl TaskOutput {
    /// For the watchdog's reports.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Snapshot { .. } => "applying a repository refresh",
            Self::WorkingDiff { .. } => "showing a file's changes",
            Self::History { .. } => "showing the history",
            Self::CommitFiles { .. } => "showing a commit's files",
            Self::CommitDiff { .. } => "showing a commit's diff",
            Self::Operation { .. } => "finishing a git operation",
            Self::RepositoryOpened { .. } => "opening a repository",
            Self::MergePreview { .. } => "showing a merge preview",
            Self::Conflict { .. } => "showing a conflict",
            Self::Organizations(_) => "listing organizations",
            Self::ProjectVersions { .. } => "reading the project version",
            Self::Pulls { .. } => "listing pull requests",
            Self::PullDetail { .. } => "showing a pull request",
            Self::PullSent { .. } => "updating a pull request",
            Self::PullCreated { .. } => "creating a pull request",
            Self::PullFetched { .. } => "checking out a pull request",
            Self::DeviceCode(_) => "starting browser sign-in",
            Self::AccountLoaded(_) | Self::SignedIn(_) | Self::SignedOut(_) => "updating the GitHub account",
        }
    }
}

pub struct Tasks {
    sender: Sender<TaskOutput>,
    receiver: Receiver<TaskOutput>,
    repaint: egui::Context,
}

impl Tasks {
    pub fn new(repaint: egui::Context) -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver, repaint }
    }

    pub fn spawn(&self, job: impl FnOnce() -> TaskOutput + Send + 'static) {
        let sender = self.sender.clone();
        let repaint = self.repaint.clone();
        std::thread::spawn(move || {
            // The receiver only disappears when the app is closing, so a failed send is fine.
            let _ = sender.send(job());
            repaint.request_repaint();
        });
    }

    pub fn finished(&self) -> Vec<TaskOutput> {
        self.receiver.try_iter().collect()
    }
}
