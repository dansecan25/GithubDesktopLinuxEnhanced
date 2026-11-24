use eframe::egui::{RichText, Ui};

use crate::constants::text;
use crate::services::settings::Settings;
use crate::ui::actions::Action;
use crate::ui::components::{primary_button, secondary_button};
use crate::ui::dialogs::Dialog;
use crate::ui::theme;

/// Shown when no repository is selected.
pub fn welcome(ui: &mut Ui, settings: &Settings, actions: &mut Vec<Action>) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.2);
        ui.label(RichText::new(text::WELCOME_TITLE).heading().size(28.0));
        ui.label(RichText::new(text::WELCOME_BODY).color(theme::palette(ui.visuals()).muted_text));
        ui.add_space(16.0);
        if primary_button(ui, text::CLONE_REPOSITORY, true).clicked() {
            actions.push(Action::OpenDialog(Dialog::clone_repository(settings)));
        }
        if secondary_button(ui, text::CREATE_REPOSITORY, true).clicked() {
            actions.push(Action::OpenDialog(Dialog::create_repository(settings)));
        }
        if secondary_button(ui, text::ADD_EXISTING, true).clicked() {
            actions.push(Action::OpenDialog(Dialog::add_repository(settings)));
        }
    });
}
