use std::path::{Path, PathBuf};

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
///
/// The picker runs on its own thread: it waits for the desktop portal, and doing that on the UI
/// thread froze the whole window whenever the portal was slow to answer.
pub fn folder_picker(ui: &mut Ui, value: &mut String) {
    let result_id = ui.scope_id().with("picked-folder");
    let open_id = ui.scope_id().with("picker-open");
    if let Some(folder) = ui.data_mut(|data| data.remove_temp::<String>(result_id)) {
        *value = folder;
    }
    let picker_open = ui.data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
    ui.horizontal(|ui| {
        let button_space = 96.0;
        ui.add(egui::TextEdit::singleline(value).desired_width(ui.available_width() - button_space));
        if secondary_button(ui, text::CHOOSE, !picker_open).clicked() {
            ui.data_mut(|data| data.insert_temp(open_id, true));
            let start = existing_folder(Path::new(value.trim()));
            let ctx = ui.ctx().clone();
            std::thread::spawn(move || {
                let mut picker = rfd::FileDialog::new();
                if let Some(start) = start {
                    picker = picker.set_directory(start);
                }
                let picked = picker.pick_folder();
                ctx.data_mut(|data| {
                    data.insert_temp(open_id, false);
                    if let Some(folder) = picked {
                        data.insert_temp(result_id, folder.to_string_lossy().into_owned());
                    }
                });
                ctx.request_repaint();
            });
        }
    });
}

/// The folder the picker opens in: the typed one, or the closest folder above it that exists (a
/// half-typed path, or a repository folder that was moved).
fn existing_folder(typed: &Path) -> Option<PathBuf> {
    typed.ancestors().find(|folder| !folder.as_os_str().is_empty() && folder.is_dir()).map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picker_starts_in_the_closest_existing_folder() {
        let temp = std::env::temp_dir();
        assert_eq!(existing_folder(&temp.join("no-such-folder-gde/deeper")), Some(temp.clone()));
        assert_eq!(existing_folder(&temp), Some(temp));
        assert_eq!(existing_folder(Path::new("")), None);
    }
}
