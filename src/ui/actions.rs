//! What the user asked for. Views only push actions; `app.rs` decides what runs.

use std::path::PathBuf;

use crate::models::{Branch, FileChange};
use crate::services::settings::ThemeChoice;
use crate::ui::dialogs::Dialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Changes,
    History,
}

#[derive(Debug, Clone)]
pub enum Action {
    OpenDialog(Dialog),
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
    Discard(Vec<FileChange>),
    IgnoreFile(String),

    Fetch,
    Pull,
    Push,
    PublishToGitHub {
        name: String,
        description: String,
        private: bool,
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
    Merge(String),
    AbortMerge,

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
