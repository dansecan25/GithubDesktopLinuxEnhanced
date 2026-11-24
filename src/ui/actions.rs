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
    SelectRepository(PathBuf),
    RemoveRepository(PathBuf),
    AddRepository(PathBuf),
    CloneRepository {
        url: String,
        parent: PathBuf,
    },
    CreateRepository(PathBuf),
    ShowInFileManager(PathBuf),
    SetTheme(ThemeChoice),
    SetTab(Tab),
    /// Leaves the detail pane in the narrow single-column layout.
    CloseDetail,
    Refresh,

    SelectFile(String),
    SelectCommit(String),
    SelectCommitFile(String),
    Commit,
    Discard(Vec<FileChange>),

    Fetch,
    Pull,
    Push,
    SwitchBranch(Branch),
    CreateBranch(String),
    DeleteBranch(String),

    StashAll,
    RestoreStash(String),
    DropStash(String),
}
