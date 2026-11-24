use std::path::PathBuf;

use eframe::egui::{self, RichText, Ui};

use super::buttons::secondary_button;
use crate::constants::layout::DIALOG_WIDTH;
use crate::constants::text;

pub enum DialogResult {
    Open,
    Closed,
}

/// A modal dialog with a title. Escape or a click on the backdrop closes it.
pub fn dialog(ctx: &egui::Context, id: &str, title: &str, content: impl FnOnce(&mut Ui)) -> DialogResult {
    let response = egui::Modal::new(egui::Id::unique(id)).show(ctx, |ui| {
        let width = DIALOG_WIDTH.min(ctx.content_rect().width() - 32.0);
        ui.set_width(width);
        ui.heading(title);
        ui.separator();
        content(ui);
    });
    if response.should_close() { DialogResult::Closed } else { DialogResult::Open }
}

pub fn field_label(ui: &mut Ui, label: &str) {
    ui.label(RichText::new(label).strong());
}

/// Text field for a folder plus a "Choose…" button that opens the system folder picker.
pub fn folder_picker(ui: &mut Ui, value: &mut String) {
    ui.horizontal(|ui| {
        let button_space = 96.0;
        ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width() - button_space));
        if secondary_button(ui, text::CHOOSE, true).clicked() {
            let start = PathBuf::from(value.as_str());
            let mut picker = rfd::FileDialog::new();
            if start.is_dir() {
                picker = picker.set_directory(start);
            }
            if let Some(folder) = picker.pick_folder() {
                *value = folder.to_string_lossy().into_owned();
            }
        }
    });
}
