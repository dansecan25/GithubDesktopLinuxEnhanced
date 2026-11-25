use eframe::egui::{self, Ui};

use crate::constants::text;
use crate::services::update::Status;
use crate::ui::actions::Action;
use crate::ui::components::{primary_button, secondary_button};

/// What the update bar shows. Checking and downloading stay quiet; only results ask for attention.
pub enum UpdateBanner {
    Ready(String),
    Installing(String),
    Installed(String),
    Failed(String),
}

pub fn update_banner(status: &Status) -> Option<UpdateBanner> {
    match status {
        Status::Ready(update) => Some(UpdateBanner::Ready(update.version.clone())),
        Status::Installing(version) => Some(UpdateBanner::Installing(version.clone())),
        Status::Installed(version) => Some(UpdateBanner::Installed(version.clone())),
        Status::InstallFailed(error) => Some(UpdateBanner::Failed(error.clone())),
        _ => None,
    }
}

pub fn show_update_banner(ui: &mut Ui, banner: UpdateBanner, actions: &mut Vec<Action>) {
    ui.horizontal_wrapped(|ui| match banner {
        UpdateBanner::Ready(version) => {
            ui.label(text::update_ready(&version));
            if primary_button(ui, text::INSTALL, true).clicked() {
                actions.push(Action::InstallUpdate);
            }
            if secondary_button(ui, text::LATER, true).clicked() {
                actions.push(Action::DismissUpdate);
            }
        }
        UpdateBanner::Installing(version) => {
            ui.spinner();
            ui.label(text::update_installing(&version));
        }
        UpdateBanner::Installed(version) => {
            ui.label(text::update_installed(&version));
            if primary_button(ui, text::RESTART_NOW, true).clicked() {
                actions.push(Action::RestartAfterUpdate);
            }
            if secondary_button(ui, text::LATER, true).clicked() {
                actions.push(Action::DismissUpdate);
            }
        }
        UpdateBanner::Failed(error) => {
            ui.add(egui::Label::new(error).wrap());
            if secondary_button(ui, text::DISMISS, true).clicked() {
                actions.push(Action::DismissUpdate);
            }
        }
    });
}
