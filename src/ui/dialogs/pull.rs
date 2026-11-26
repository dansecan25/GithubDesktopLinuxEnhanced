use eframe::egui::{self, Ui};

use super::{DialogContext, focus_once, footer_buttons};
use crate::constants::layout::COMMIT_DESCRIPTION_ROWS;
use crate::constants::text;
use crate::ui::actions::Action;
use crate::ui::components::{field_label, muted_text, rounded};

/// A new pull request from the current branch.
#[derive(Debug, Clone)]
pub struct PullForm {
    pub head: String,
    pub base: String,
    /// Branches on the remote it can go into.
    pub bases: Vec<String>,
    pub title: String,
    pub body: String,
    pub draft: bool,
}

pub fn show(ui: &mut Ui, context: &DialogContext<'_>, form: &mut PullForm, actions: &mut Vec<Action>) {
    ui.label(muted_text(ui, &text::pull_from(&form.head)));
    field_label(ui, text::PULL_INTO);
    egui::ComboBox::from_id_salt("pull-base")
        .selected_text(form.base.as_str())
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for base in &form.bases {
                ui.selectable_value(&mut form.base, base.clone(), base);
            }
        });
    field_label(ui, text::PULL_TITLE);
    let field = ui.add(egui::TextEdit::singleline(&mut form.title).desired_width(f32::INFINITY));
    focus_once(ui, &field, context.opening);
    field_label(ui, text::PULL_DESCRIPTION);
    ui.add(
        egui::TextEdit::multiline(&mut form.body)
            .desired_rows(COMMIT_DESCRIPTION_ROWS * 2)
            .desired_width(f32::INFINITY),
    );
    rounded(ui, |ui| ui.checkbox(&mut form.draft, text::PULL_DRAFT));
    let ready = !form.title.trim().is_empty() && form.base != form.head && !context.sending;
    if form.base == form.head {
        ui.label(muted_text(ui, text::PULL_SAME_BRANCH));
    }
    // Enter in the description adds a line, so only the button submits.
    if footer_buttons(ui, text::CREATE_PULL_BUTTON, ready, false, actions) {
        actions.push(Action::CreatePull(form.clone()));
    }
}

/// Changing an open pull request's title and description.
#[derive(Debug, Clone)]
pub struct PullEditForm {
    pub number: u64,
    pub title: String,
    pub body: String,
}

pub fn show_edit(
    ui: &mut Ui,
    context: &DialogContext<'_>,
    form: &mut PullEditForm,
    actions: &mut Vec<Action>,
) {
    field_label(ui, text::PULL_TITLE);
    let field = ui.add(egui::TextEdit::singleline(&mut form.title).desired_width(f32::INFINITY));
    focus_once(ui, &field, context.opening);
    field_label(ui, text::PULL_DESCRIPTION);
    ui.add(
        egui::TextEdit::multiline(&mut form.body)
            .desired_rows(COMMIT_DESCRIPTION_ROWS * 2)
            .desired_width(f32::INFINITY),
    );
    let ready = !form.title.trim().is_empty() && !context.sending;
    if footer_buttons(ui, text::SAVE, ready, false, actions) {
        actions.push(Action::SavePull(form.clone()));
    }
}
