//! Right-click menus, matching GitHub Desktop's. Each function fills an already open menu.

use std::path::Path;

use eframe::egui::{self, Ui};

use super::ViewContext;
use crate::constants::text;
use crate::models::{ChangeKind, CommitSummary, FileChange, RepoSnapshot};
use crate::ui::actions::Action;
use crate::ui::dialogs::{Dialog, TagForm};

fn item(ui: &mut Ui, label: &str, enabled: bool, actions: &mut Vec<Action>, action: impl FnOnce() -> Action) {
    if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
        actions.push(action());
        ui.close();
    }
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

/// For a repository in the list or the current repository button.
pub fn repository(ui: &mut Ui, view: &ViewContext<'_>, path: &Path, actions: &mut Vec<Action>) {
    item(ui, view.open_in_label, true, actions, || Action::OpenInEditor(path.to_path_buf()));
    item(ui, text::SHOW_IN_FILE_MANAGER, true, actions, || Action::ShowInFileManager(path.to_path_buf()));
    item(ui, text::COPY_PATH, true, actions, || Action::CopyText(path_text(path)));
    ui.separator();
    item(ui, text::REMOVE_REPOSITORY, true, actions, || Action::RemoveRepository(path.to_path_buf()));
}

/// For a changed file in the Changes tab.
pub fn working_file(
    ui: &mut Ui,
    view: &ViewContext<'_>,
    repo: &Path,
    change: &FileChange,
    actions: &mut Vec<Action>,
) {
    let busy = view.busy.is_some();
    let full = repo.join(&change.path);
    let exists = change.kind != ChangeKind::Deleted;
    item(ui, text::DISCARD_CHANGES, !busy, actions, || discard_confirmation(vec![change.clone()]));
    let can_ignore = change.kind == ChangeKind::Untracked && !busy;
    item(ui, text::IGNORE_FILE, can_ignore, actions, || Action::IgnoreFile(change.path.clone()));
    ui.separator();
    file_items(ui, view, &full, &change.path, exists, actions);
}

/// For a file in a commit in the History tab (opens the current copy on disk).
pub fn commit_file(
    ui: &mut Ui,
    view: &ViewContext<'_>,
    repo: &Path,
    file: &FileChange,
    actions: &mut Vec<Action>,
) {
    let full = repo.join(&file.path);
    let exists = full.exists();
    file_items(ui, view, &full, &file.path, exists, actions);
}

fn file_items(
    ui: &mut Ui,
    view: &ViewContext<'_>,
    full: &Path,
    relative: &str,
    exists: bool,
    actions: &mut Vec<Action>,
) {
    item(ui, view.open_in_label, exists, actions, || Action::OpenInEditor(full.to_path_buf()));
    item(ui, text::OPEN_WITH_DEFAULT_APP, exists, actions, || Action::OpenWithSystem(full.to_path_buf()));
    let folder = full.parent().unwrap_or(full).to_path_buf();
    item(ui, text::SHOW_IN_FILE_MANAGER, folder.exists(), actions, || Action::ShowInFileManager(folder));
    item(ui, text::COPY_PATH, true, actions, || Action::CopyText(path_text(full)));
    item(ui, text::COPY_RELATIVE_PATH, true, actions, || Action::CopyText(relative.to_owned()));
}

/// For a commit in the History tab.
pub fn commit(
    ui: &mut Ui,
    view: &ViewContext<'_>,
    snapshot: Option<&RepoSnapshot>,
    commit: &CommitSummary,
    actions: &mut Vec<Action>,
) {
    let busy = view.busy.is_some();
    item(ui, text::CREATE_TAG_HERE, !busy && snapshot.is_some(), actions, || {
        tag_dialog(snapshot.expect("enabled only with a snapshot"), commit)
    });
    item(ui, text::COPY_SHA, true, actions, || Action::CopyText(commit.sha.clone()));
}

/// For a tag shown on a commit.
pub fn tag(ui: &mut Ui, view: &ViewContext<'_>, name: &str, has_remote: bool, actions: &mut Vec<Action>) {
    let busy = view.busy.is_some();
    item(ui, text::PUSH_TAG, has_remote && !busy, actions, || Action::PushTag(name.to_owned()));
    item(ui, text::DELETE_TAG, !busy, actions, || {
        Action::OpenDialog(Dialog::DeleteTag { name: name.to_owned(), has_remote })
    });
}

pub fn discard_confirmation(changes: Vec<FileChange>) -> Action {
    Action::OpenDialog(Dialog::confirm(
        text::DISCARD_CHANGES,
        text::discard_files_body(changes.len()),
        text::DISCARD_BUTTON,
        Action::Discard(changes),
    ))
}

/// The "Create a tag" dialog for `commit`, with the next version prefilled.
pub fn tag_dialog(snapshot: &RepoSnapshot, commit: &CommitSummary) -> Action {
    let names: Vec<String> = snapshot.tags.iter().map(|tag| tag.name.clone()).collect();
    let label = text::tag_target(&commit.short_sha, &commit.summary);
    let form = TagForm::new(&names, commit.sha.clone(), label, snapshot.remote.is_some());
    Action::OpenDialog(Dialog::CreateTag(form))
}
