use eframe::egui::{self, Ui};

use super::{DialogContext, focus_once, footer_buttons};
use crate::constants::layout::COMMIT_DESCRIPTION_ROWS;
use crate::constants::text;
use crate::ui::actions::Action;
use crate::ui::components::{danger_button, field_label, muted_text, secondary_button};
use crate::utils::version::next_versions;

#[derive(Debug, Clone)]
pub struct TagForm {
    pub name: String,
    pub message: String,
    /// Commit the tag will point to.
    pub target: String,
    pub target_label: String,
    pub push: bool,
    pub has_remote: bool,
    pub suggestions: [String; 3],
}

impl TagForm {
    /// Prefills the next patch version after the repository's highest version tag.
    pub fn new(existing_tags: &[String], target: String, target_label: String, has_remote: bool) -> Self {
        let suggestions = next_versions(existing_tags);
        Self {
            name: suggestions[0].clone(),
            message: String::new(),
            target,
            target_label,
            push: has_remote,
            has_remote,
            suggestions,
        }
    }
}

pub fn show_create(ui: &mut Ui, context: &DialogContext<'_>, form: &mut TagForm, actions: &mut Vec<Action>) {
    ui.label(muted_text(ui, &form.target_label));
    field_label(ui, text::TAG_NAME);
    let field = ui.add(egui::TextEdit::singleline(&mut form.name).desired_width(f32::INFINITY));
    focus_once(ui, &field, context.opening);
    ui.horizontal_wrapped(|ui| {
        ui.label(text::TAG_SUGGESTIONS);
        for suggestion in form.suggestions.clone() {
            if ui.add(egui::Button::selectable(form.name == suggestion, &suggestion)).clicked() {
                form.name = suggestion;
            }
        }
    });
    field_label(ui, text::TAG_MESSAGE);
    ui.add(
        egui::TextEdit::multiline(&mut form.message)
            .desired_rows(COMMIT_DESCRIPTION_ROWS)
            .desired_width(f32::INFINITY),
    );
    if form.has_remote {
        ui.checkbox(&mut form.push, text::TAG_PUSH);
        ui.label(muted_text(ui, text::TAG_RELEASE_HINT));
    }
    let ready = !form.name.trim().is_empty() && !context.busy;
    // Enter in the message field adds a line, so only the button submits here.
    if footer_buttons(ui, text::CREATE_TAG_BUTTON, ready, false, actions) {
        actions.push(Action::CloseDialog);
        actions.push(Action::CreateTag {
            name: form.name.trim().to_owned(),
            target: form.target.clone(),
            message: form.message.trim().to_owned(),
            push: form.push && form.has_remote,
        });
    }
}

pub fn show_delete(ui: &mut Ui, busy: bool, name: &str, has_remote: bool, actions: &mut Vec<Action>) {
    ui.label(text::delete_tag_body(name));
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        let mut delete = |on_remote: bool| {
            actions.push(Action::CloseDialog);
            actions.push(Action::DeleteTag { name: name.to_owned(), on_remote });
        };
        if has_remote && danger_button(ui, text::DELETE_TAG_ON_REMOTE, !busy).clicked() {
            delete(true);
        }
        if danger_button(ui, text::DELETE_TAG_LOCALLY, !busy).clicked() {
            delete(false);
        }
        if secondary_button(ui, text::CANCEL, true).clicked() {
            actions.push(Action::CloseDialog);
        }
    });
}
