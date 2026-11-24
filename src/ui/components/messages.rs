use eframe::egui::{self, RichText, Ui};

use crate::constants::text;
use crate::stores::Loadable;
use crate::ui::theme;

/// Centered title and body for empty lists and panes.
pub fn empty_state(ui: &mut Ui, title: &str, body: &str) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() * 0.3);
        ui.label(RichText::new(title).heading());
        if !body.is_empty() {
            ui.label(RichText::new(body).color(theme::palette(ui.visuals()).muted_text));
        }
    });
}

/// Spinner while loading, the error when loading failed, nothing otherwise.
pub fn loadable_status<T>(ui: &mut Ui, value: &Loadable<T>) {
    match value {
        Loadable::Loading => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(text::LOADING);
            });
        }
        Loadable::Failed(error) => {
            ui.colored_label(theme::palette(ui.visuals()).danger, error);
        }
        Loadable::Idle | Loadable::Ready(_) => {}
    }
}

/// Full-width error bar. Returns true when the user dismissed it.
pub fn error_banner(ui: &mut Ui, message: &str) -> bool {
    let palette = theme::palette(ui.visuals());
    let mut dismissed = false;
    egui::Frame::new().fill(palette.error_background).inner_margin(egui::Margin::same(8)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(text::ERROR_TITLE).strong().color(palette.danger));
            if ui.button(text::DISMISS).clicked() {
                dismissed = true;
            }
        });
        ui.add(egui::Label::new(message).wrap());
    });
    dismissed
}

/// Small, muted secondary text used under list row titles.
pub fn muted_text(ui: &Ui, value: &str) -> RichText {
    RichText::new(value).small().color(theme::palette(ui.visuals()).muted_text)
}
