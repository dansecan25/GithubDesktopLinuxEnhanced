//! Modal dialogs. Each holds its own form state and turns a confirmed form into an [`Action`].

mod merge;
mod preferences;
mod prompt;
mod publish;
mod tag;

use std::path::PathBuf;

use eframe::egui::{self, RichText, Ui};

use crate::constants::text;
use crate::git::branches::sanitize_name;
use crate::models::Branch;
use crate::services::editor::Editor;
use crate::services::settings::Settings;
use crate::services::update;
use crate::ui::actions::Action;
use crate::ui::components::{
    DialogResult, danger_button, dialog, field_label, folder_picker, primary_button, secondary_button,
};
use crate::ui::theme;

#[derive(Debug, Clone)]
pub struct Confirmation {
    pub title: String,
    pub body: String,
    pub confirm_label: String,
    pub action: Box<Action>,
}

#[derive(Debug, Clone)]
pub enum Dialog {
    AddRepository { path: String },
    CloneRepository { url: String, parent: String },
    CreateRepository { name: String, parent: String },
    CreateBranch { name: String, from: String },
    Confirm(Confirmation),
    Preferences { token: String, custom_editor: String, custom_mode: bool },
    Publish { name: String, description: String, private: bool, remote_url: String },
    CreateTag(tag::TagForm),
    DeleteTag { name: String, has_remote: bool },
    Merge(MergeForm),
    SwitchBranch { branch: Branch, from: String, leave_changes: bool },
}

/// Read-only app state some dialogs show.
pub struct DialogContext<'a> {
    pub settings: &'a Settings,
    pub editors: &'a [Editor],
    pub update_status: &'a update::Status,
    pub can_update: bool,
    pub signing_in: bool,
    pub busy: bool,
    /// Changes every time a dialog opens, so dialogs can focus their first field once.
    pub opening: u64,
}

pub use merge::MergeForm;
pub use prompt::show_prompt;
pub use tag::TagForm;

impl Dialog {
    pub fn add_repository(settings: &Settings) -> Self {
        Self::AddRepository { path: projects_folder(settings) }
    }

    pub fn clone_repository(settings: &Settings) -> Self {
        Self::CloneRepository { url: String::new(), parent: projects_folder(settings) }
    }

    pub fn create_repository(settings: &Settings) -> Self {
        Self::CreateRepository { name: String::new(), parent: projects_folder(settings) }
    }

    pub fn create_branch(from: &str) -> Self {
        Self::CreateBranch { name: String::new(), from: from.to_owned() }
    }

    pub fn preferences(settings: &Settings, editors: &[Editor]) -> Self {
        let custom = settings.editor.clone().filter(|command| !editors.iter().any(|e| &e.command == command));
        Self::Preferences {
            token: String::new(),
            custom_mode: custom.is_some(),
            custom_editor: custom.unwrap_or_default(),
        }
    }

    pub fn publish(repository_name: &str) -> Self {
        Self::Publish {
            name: repository_name.to_owned(),
            description: String::new(),
            private: true,
            remote_url: String::new(),
        }
    }

    pub fn confirm(title: &str, body: String, confirm_label: &str, action: Action) -> Self {
        Self::Confirm(Confirmation {
            title: title.to_owned(),
            body,
            confirm_label: confirm_label.to_owned(),
            action: Box::new(action),
        })
    }
}

fn projects_folder(settings: &Settings) -> String {
    settings.projects_directory().to_string_lossy().into_owned()
}

pub fn show(
    ctx: &egui::Context,
    dialog_state: &mut Dialog,
    context: &DialogContext<'_>,
    actions: &mut Vec<Action>,
) {
    let busy = context.busy;
    let opening = context.opening;
    let result = match dialog_state {
        Dialog::AddRepository { path } => dialog(ctx, "add-repository", text::ADD_REPOSITORY_TITLE, |ui| {
            field_label(ui, text::LOCAL_PATH);
            folder_picker(ui, path);
            let ready = !path.trim().is_empty() && !busy;
            if footer_buttons(ui, text::ADD_REPOSITORY_BUTTON, ready, true, actions) {
                actions.push(Action::AddRepository(PathBuf::from(path.trim())));
            }
        }),
        Dialog::CloneRepository { url, parent } => dialog(ctx, "clone-repository", text::CLONE_TITLE, |ui| {
            field_label(ui, text::CLONE_URL);
            let field = ui.add(
                egui::TextEdit::singleline(url).hint_text(text::CLONE_URL_HINT).desired_width(f32::INFINITY),
            );
            focus_once(ui, &field, opening);
            field_label(ui, text::CLONE_INTO);
            folder_picker(ui, parent);
            let ready = !url.trim().is_empty() && !parent.trim().is_empty() && !busy;
            if footer_buttons(ui, text::CLONE_BUTTON, ready, true, actions) {
                actions.push(Action::CloneRepository {
                    url: url.trim().to_owned(),
                    parent: PathBuf::from(parent.trim()),
                });
            }
        }),
        Dialog::CreateRepository { name, parent } => {
            dialog(ctx, "create-repository", text::CREATE_TITLE, |ui| {
                field_label(ui, text::NAME);
                let field = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                focus_once(ui, &field, opening);
                field_label(ui, text::CREATE_IN);
                folder_picker(ui, parent);
                let ready = !name.trim().is_empty() && !parent.trim().is_empty() && !busy;
                if footer_buttons(ui, text::CREATE_BUTTON, ready, true, actions) {
                    actions.push(Action::CreateRepository(PathBuf::from(parent.trim()).join(name.trim())));
                }
            })
        }
        Dialog::CreateBranch { name, from } => {
            dialog(ctx, "create-branch", text::CREATE_BRANCH_TITLE, |ui| {
                field_label(ui, text::NAME);
                let field = ui.add(egui::TextEdit::singleline(name).desired_width(f32::INFINITY));
                focus_once(ui, &field, opening);
                let sanitized = sanitize_name(name);
                ui.label(format!("{} {from}.", text::BRANCH_FROM_PREFIX));
                if sanitized.is_empty() && !name.is_empty() {
                    ui.label(text::INVALID_BRANCH_NAME);
                }
                if footer_buttons(
                    ui,
                    text::CREATE_BRANCH_BUTTON,
                    !sanitized.is_empty() && !busy,
                    true,
                    actions,
                ) {
                    actions.push(Action::CreateBranch(sanitized));
                }
            })
        }
        Dialog::Confirm(confirmation) => dialog(ctx, "confirm", &confirmation.title, |ui| {
            ui.add(egui::Label::new(&confirmation.body).wrap());
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if danger_button(ui, &confirmation.confirm_label, !busy).clicked() {
                    actions.push(Action::CloseDialog);
                    actions.push((*confirmation.action).clone());
                }
                if secondary_button(ui, text::CANCEL, true).clicked() {
                    actions.push(Action::CloseDialog);
                }
            });
        }),
        Dialog::Preferences { token, custom_editor, custom_mode } => {
            dialog(ctx, "preferences", text::PREFERENCES_TITLE, |ui| {
                preferences::show(ui, context, token, custom_editor, custom_mode, actions);
            })
        }
        Dialog::Publish { name, description, private, remote_url } => {
            dialog(ctx, "publish", text::PUBLISH_TITLE, |ui| {
                let form = publish::PublishForm { name, description, private, remote_url };
                publish::show(ui, context, form, actions);
            })
        }
        Dialog::CreateTag(form) => dialog(ctx, "create-tag", text::CREATE_TAG_TITLE, |ui| {
            tag::show_create(ui, context, form, actions);
        }),
        Dialog::DeleteTag { name, has_remote } => dialog(ctx, "delete-tag", text::DELETE_TAG, |ui| {
            tag::show_delete(ui, busy, name, *has_remote, actions);
        }),
        Dialog::SwitchBranch { branch, from, leave_changes } => {
            dialog(ctx, "switch-branch", text::SWITCH_BRANCH_TITLE, |ui| {
                ui.label(text::SWITCH_WITH_CHANGES_BODY);
                ui.add_space(8.0);
                choice(ui, leave_changes, true, &text::leave_changes_on(from), text::LEAVE_CHANGES_HINT);
                choice(
                    ui,
                    leave_changes,
                    false,
                    &text::bring_changes_to(&branch.name),
                    text::BRING_CHANGES_HINT,
                );
                if footer_buttons(ui, text::SWITCH_BRANCH_BUTTON, !busy, true, actions) {
                    actions.push(Action::CloseDialog);
                    actions.push(Action::SwitchBranchWithChanges {
                        branch: branch.clone(),
                        leave_changes: *leave_changes,
                    });
                }
            })
        }
        Dialog::Merge(form) => dialog(ctx, "merge", &text::merge_title(&form.into), |ui| {
            merge::show(ui, context, form, actions);
        }),
    };
    if matches!(result, DialogResult::Closed) {
        actions.push(Action::CloseDialog);
    }
}

/// Primary and cancel buttons. Returns true when the primary button was clicked, or (with
/// `submit_on_enter`) Enter was pressed while it is enabled.
pub(super) fn footer_buttons(
    ui: &mut Ui,
    primary_label: &str,
    enabled: bool,
    submit_on_enter: bool,
    actions: &mut Vec<Action>,
) -> bool {
    ui.add_space(8.0);
    let enter = submit_on_enter && ui.input(|input| input.key_pressed(egui::Key::Enter));
    ui.horizontal(|ui| {
        let submitted = primary_button(ui, primary_label, enabled).clicked() || (enabled && enter);
        if secondary_button(ui, text::CANCEL, true).clicked() {
            actions.push(Action::CloseDialog);
        }
        submitted
    })
    .inner
}

/// A radio option with a muted explanation under it.
fn choice(ui: &mut Ui, current: &mut bool, value: bool, label: &str, hint: &str) {
    ui.radio_value(current, value, label);
    let muted = theme::palette(ui.visuals()).muted_text;
    ui.horizontal(|ui| {
        // Lines the hint up with the label, past the radio button.
        ui.add_space(ui.spacing().icon_width + ui.spacing().icon_spacing);
        ui.add(egui::Label::new(RichText::new(hint).color(muted)).wrap());
    });
    ui.add_space(4.0);
}

/// Focuses a dialog's first field once per opening. `opening` changes every time a dialog opens.
pub(super) fn focus_once(ui: &Ui, field: &egui::Response, opening: u64) {
    let key = egui::Id::unique(("dialog-focus", opening));
    let done = ui.data(|data| data.get_temp::<bool>(key).unwrap_or(false));
    if !done {
        field.request_focus();
        ui.data_mut(|data| data.insert_temp(key, true));
    }
}
