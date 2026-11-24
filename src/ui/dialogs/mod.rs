//! Modal dialogs. Each holds its own form state and turns a confirmed form into an [`Action`].

use std::path::PathBuf;

use eframe::egui::{self, Ui};

use crate::constants::text;
use crate::git::branches::sanitize_name;
use crate::services::settings::Settings;
use crate::ui::actions::Action;
use crate::ui::components::{
    DialogResult, danger_button, dialog, field_label, folder_picker, primary_button, secondary_button,
};

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
}

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
    opening: u64,
    busy: bool,
    actions: &mut Vec<Action>,
) {
    let result = match dialog_state {
        Dialog::AddRepository { path } => dialog(ctx, "add-repository", text::ADD_REPOSITORY_TITLE, |ui| {
            field_label(ui, text::LOCAL_PATH);
            folder_picker(ui, path);
            let ready = !path.trim().is_empty() && !busy;
            if footer(ui, text::ADD_REPOSITORY_BUTTON, ready, actions) {
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
            if footer(ui, text::CLONE_BUTTON, ready, actions) {
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
                if footer(ui, text::CREATE_BUTTON, ready, actions) {
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
                if footer(ui, text::CREATE_BRANCH_BUTTON, !sanitized.is_empty() && !busy, actions) {
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
    };
    if matches!(result, DialogResult::Closed) {
        actions.push(Action::CloseDialog);
    }
}

/// Primary and cancel buttons. Returns true when the primary button was clicked, or Enter was
/// pressed while it is enabled.
fn footer(ui: &mut Ui, primary_label: &str, enabled: bool, actions: &mut Vec<Action>) -> bool {
    ui.add_space(8.0);
    let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
    ui.horizontal(|ui| {
        let submitted = primary_button(ui, primary_label, enabled).clicked() || (enabled && enter);
        if secondary_button(ui, text::CANCEL, true).clicked() {
            actions.push(Action::CloseDialog);
        }
        submitted
    })
    .inner
}

/// Focuses a dialog's first field once per opening. `opening` changes every time a dialog opens.
fn focus_once(ui: &Ui, field: &egui::Response, opening: u64) {
    let key = egui::Id::unique(("dialog-focus", opening));
    let done = ui.data(|data| data.get_temp::<bool>(key).unwrap_or(false));
    if !done {
        field.request_focus();
        ui.data_mut(|data| data.insert_temp(key, true));
    }
}
