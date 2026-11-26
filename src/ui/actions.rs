//! What the user asked for. Views only push actions; `app.rs` decides what runs.

use std::path::PathBuf;

use crate::models::{Branch, ConflictChoice, FileChange, LineTarget, MergeMethod};
use crate::services::settings::ThemeChoice;
use crate::ui::dialogs::{Dialog, PullEditForm, PullForm};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Changes,
    History,
    PullRequests,
}

/// The two parts of a pull request's page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PullView {
    #[default]
    Conversation,
    Files,
}

#[derive(Debug, Clone)]
pub enum Action {
    OpenDialog(Dialog),
    /// Stops the running fetch, pull, push, publish or clone.
    CancelOperation,
    CloseDialog,
    /// Answers the pending git or ssh credential prompt. `None` cancels it.
    AnswerPrompt(Option<String>),

    SelectRepository(PathBuf),
    RemoveRepository(PathBuf),
    AddRepository(PathBuf),
    CloneRepository {
        url: String,
        parent: PathBuf,
    },
    CreateRepository(PathBuf),

    ShowInFileManager(PathBuf),
    OpenWithSystem(PathBuf),
    OpenInEditor(PathBuf),
    OpenUrl(String),
    CopyText(String),

    SetTheme(ThemeChoice),
    SetEditor(Option<String>),
    SetAutoUpdate(bool),
    SignIn(String),
    /// Starts GitHub's device flow: a code to enter on github.com.
    StartBrowserSignIn,

    LoadPulls,
    SelectPull(u64),
    /// Fetches the pull request's branch and switches to it.
    CheckoutPull(u64),
    PostComment {
        number: u64,
        body: String,
    },
    OpenCreatePull,
    CreatePull(PullForm),
    SetPullView(PullView),
    SelectPullFile(String),
    /// Opens the comment field for a line of the selected pull request's diff.
    StartLineComment(LineTarget),
    CancelLineComment,
    PostLineComment,
    SubmitReview,
    MergePull {
        number: u64,
        method: MergeMethod,
    },
    OpenEditPull,
    SavePull(PullEditForm),
    SetDraft {
        number: u64,
        draft: bool,
    },
    SignOut,
    CheckForUpdates,
    InstallUpdate,
    RestartAfterUpdate,
    DismissUpdate,

    SetTab(Tab),
    /// Leaves the detail pane in the narrow single-column layout.
    CloseDetail,
    Refresh,

    SelectFile(String),
    SelectCommit(String),
    SelectCommitFile(String),
    Commit,
    /// Puts the last commit's message in the commit form; the next commit replaces it.
    StartAmend,
    CancelAmend,
    /// Removes the last commit, keeping its changes uncommitted and its message in the form.
    UndoCommit,
    Discard(Vec<FileChange>),
    IgnoreFile(String),

    Fetch,
    Pull,
    Push,
    /// Pushes a branch whose history was rewritten here (`--force-with-lease`).
    ForcePush,
    PublishToGitHub {
        name: String,
        description: String,
        private: bool,
        /// `None` publishes to the signed-in account.
        organization: Option<String>,
    },
    AddRemote(String),
    /// Switches right away when there are no changes, otherwise asks what to do with them.
    SwitchBranch(Branch),
    /// The answer to that question: leave the changes stashed on this branch, or bring them along.
    SwitchBranchWithChanges {
        branch: Branch,
        leave_changes: bool,
    },
    CreateBranch(String),
    DeleteBranch(String),
    /// Works out what merging this branch into the current one would do, for the merge dialog.
    PreviewMerge(String),
    Merge {
        branch: String,
        method: MergeMethod,
    },
    /// Aborts the merge or squash merge in progress.
    AbortMerge,
    ContinueRebase,
    AbortRebase,
    /// Resolves one conflict block of a file with your side, theirs or both.
    ResolveConflict {
        file: String,
        index: usize,
        choice: ConflictChoice,
    },
    /// Takes the whole file from your side (`ours`) or theirs, and marks it resolved.
    TakeConflictSide {
        file: String,
        ours: bool,
    },
    MarkResolved(String),

    CreateTag {
        name: String,
        target: String,
        message: String,
        push: bool,
    },
    PushTag(String),
    DeleteTag {
        name: String,
        on_remote: bool,
    },

    StashAll,
    RestoreStash(String),
    DropStash(String),
}
