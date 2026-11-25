use eframe::egui::{self, Ui};

use super::{DialogContext, footer_buttons};
use crate::constants::text;
use crate::stores::Loadable;
use crate::ui::actions::Action;
use crate::ui::components::{field_label, muted_text, primary_button, rounded, secondary_button};

#[derive(Debug, Clone)]
pub struct PublishForm {
    pub name: String,
    pub description: String,
    pub private: bool,
    pub remote_url: String,
    /// Where to create the repository: `None` for the signed-in account.
    pub organization: Option<String>,
    /// The account's organizations, loaded when the dialog opens.
    pub organizations: Loadable<Vec<String>>,
}

impl PublishForm {
    pub fn new(repository_name: &str) -> Self {
        Self {
            name: repository_name.to_owned(),
            description: String::new(),
            private: true,
            remote_url: String::new(),
            organization: None,
            organizations: Loadable::Loading,
        }
    }
}

/// Publishes a repository that has no remote: create it on GitHub (on the account or in one of
/// its organizations), or connect an existing URL.
pub fn show(ui: &mut Ui, context: &DialogContext<'_>, form: &mut PublishForm, actions: &mut Vec<Action>) {
    let busy = context.busy;
    if let Some(login) = &context.settings.github_login {
        field_label(ui, text::PUBLISH_OWNER);
        owner_picker(ui, login, form);
        field_label(ui, text::NAME);
        ui.add(egui::TextEdit::singleline(&mut form.name).desired_width(f32::INFINITY));
        field_label(ui, text::PUBLISH_DESCRIPTION);
        ui.add(egui::TextEdit::singleline(&mut form.description).desired_width(f32::INFINITY));
        rounded(ui, |ui| ui.checkbox(&mut form.private, text::PUBLISH_PRIVATE));
        let ready = !form.name.trim().is_empty() && !busy;
        if primary_button(ui, text::PUBLISH_TO_GITHUB, ready).clicked() {
            actions.push(Action::PublishToGitHub {
                name: form.name.trim().to_owned(),
                description: form.description.trim().to_owned(),
                private: form.private,
                organization: form.organization.clone(),
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
        egui::TextEdit::singleline(&mut form.remote_url)
            .hint_text(text::REMOTE_URL_HINT)
            .desired_width(f32::INFINITY),
    );
    let ready = !form.remote_url.trim().is_empty() && !busy;
    if footer_buttons(ui, text::ADD_REMOTE_AND_PUSH, ready, true, actions) {
        actions.push(Action::AddRemote(form.remote_url.trim().to_owned()));
    }
}

/// The account first, then its organizations.
fn owner_picker(ui: &mut Ui, login: &str, form: &mut PublishForm) {
    let selected = form.organization.clone().unwrap_or_else(|| login.to_owned());
    egui::ComboBox::from_id_salt("publish-owner")
        .selected_text(selected)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut form.organization, None, login);
            if let Loadable::Ready(organizations) = &form.organizations {
                for organization in organizations {
                    ui.selectable_value(&mut form.organization, Some(organization.clone()), organization);
                }
            }
        });
    match &form.organizations {
        Loadable::Loading => {
            ui.label(muted_text(ui, text::LOADING_ORGANIZATIONS));
        }
        Loadable::Failed(error) => {
            ui.label(muted_text(ui, &text::organizations_failed(error)));
        }
        _ => {}
    }
}
