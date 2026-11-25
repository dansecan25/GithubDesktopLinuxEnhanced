//! Runs git work on background threads so the window never freezes, and hands results back to the
//! UI thread through a channel.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use eframe::egui;

use crate::constants::text;
use crate::git::GitResult;
use crate::models::{CommitSummary, Diff, FileChange, RepoSnapshot};
use crate::services::askpass::GitHubAccount;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Commit,
    Fetch,
    Pull,
    Push,
    SwitchBranch,
    CreateBranch,
    DeleteBranch,
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
    pub fn label(self) -> &'static str {
        match self {
            Self::Commit => text::COMMITTING,
            Self::Fetch => text::FETCHING,
            Self::Pull => text::PULLING,
            Self::Push => text::PUSHING,
            Self::SwitchBranch => text::SWITCHING_BRANCH,
            Self::CreateBranch => text::CREATING_BRANCH,
            Self::DeleteBranch => text::DELETING_BRANCH,
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
    /// Result of adding, cloning or creating a repository. `remember_folder` becomes the default
    /// folder for the next clone or new repository.
    RepositoryOpened {
        result: GitResult<PathBuf>,
        remember_folder: Option<PathBuf>,
    },
    /// The saved GitHub account, read from the keychain at startup.
    AccountLoaded(Result<Option<GitHubAccount>, String>),
    SignedIn(Result<GitHubAccount, String>),
    SignedOut(Result<(), String>),
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
