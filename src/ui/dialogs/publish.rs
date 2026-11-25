use eframe::egui::{self, Ui};

use super::{DialogContext, footer_buttons};
use crate::constants::text;
use crate::ui::actions::Action;
use crate::ui::components::{field_label, muted_text, primary_button, secondary_button};

pub struct PublishForm<'a> {
    pub name: &'a mut String,
    pub description: &'a mut String,
    pub private: &'a mut bool,
    pub remote_url: &'a mut String,
}

/// Publishes a repository that has no remote: create it on GitHub, or connect an existing URL.
pub fn show(ui: &mut Ui, context: &DialogContext<'_>, form: PublishForm<'_>, actions: &mut Vec<Action>) {
    let busy = context.busy;
    if context.settings.github_login.is_some() {
        field_label(ui, text::NAME);
        ui.add(egui::TextEdit::singleline(form.name).desired_width(f32::INFINITY));
        field_label(ui, text::PUBLISH_DESCRIPTION);
        ui.add(egui::TextEdit::singleline(form.description).desired_width(f32::INFINITY));
        ui.checkbox(form.private, text::PUBLISH_PRIVATE);
        let ready = !form.name.trim().is_empty() && !busy;
        if primary_button(ui, text::PUBLISH_TO_GITHUB, ready).clicked() {
            actions.push(Action::PublishToGitHub {
                name: form.name.trim().to_owned(),
                description: form.description.trim().to_owned(),
                private: *form.private,
            });
        }
    } else {
        ui.add(egui::Label::new(muted_text(ui, text::PUBLISH_SIGN_IN_FIRST)).wrap());
        if secondary_button(ui, text::PREFERENCES, true).clicked() {
            let preferences = super::Dialog::preferences(context.settings, context.editors);
            actions.push(Action::OpenDialog(preferences));
        }
    }
    ui.separator();

    field_label(ui, text::PUBLISH_EXISTING_REMOTE);
    ui.add(
        egui::TextEdit::singleline(form.remote_url)
            .hint_text(text::REMOTE_URL_HINT)
            .desired_width(f32::INFINITY),
    );
    let ready = !form.remote_url.trim().is_empty() && !busy;
    if footer_buttons(ui, text::ADD_REMOTE_AND_PUSH, ready, true, actions) {
        actions.push(Action::AddRemote(form.remote_url.trim().to_owned()));
    }
}
