use eframe::egui::{self, Button, Response, RichText, Ui};

use crate::constants::layout::LIST_ROW_HEIGHT;
use crate::constants::text;
use crate::models::FileChange;
use crate::ui::theme;
use crate::utils::paths::split_file_path;

/// One row of a file list. Pass `included` to show the "include in commit" checkbox.
/// Returns the row's response so callers can attach a context menu.
pub fn file_row(ui: &mut Ui, change: &FileChange, selected: bool, included: Option<&mut bool>) -> Response {
    let palette = theme::palette(ui.visuals());
    let (letter, kind_name) = theme::change_label(change.kind);
    let (name, folder) = split_file_path(&change.path);

    ui.horizontal(|ui| {
        ui.set_min_height(LIST_ROW_HEIGHT);
        if let Some(included) = included {
            let accessible_name = text::include_in_commit(&change.path);
            ui.add(egui::Checkbox::without_text(included)).on_hover_text(&accessible_name).widget_info(
                || egui::WidgetInfo::selected(egui::Role::CheckBox, true, *included, &accessible_name),
            );
        }
        let mut label = if folder.is_empty() { name.to_owned() } else { format!("{name}  {folder}") };
        if let Some(original) = &change.original_path {
            label = format!("{original} → {label}");
        }
        let badge =
            RichText::new(letter).monospace().strong().color(theme::change_color(&palette, change.kind));
        // Cut long paths with "…" instead of widening the panel. The full path is in the tooltip.
        let row = Button::selectable(selected, label)
            .truncate()
            .right_text(badge)
            .min_size(egui::vec2(ui.available_width(), LIST_ROW_HEIGHT));
        ui.add(row).on_hover_text(format!("{kind_name}: {}", change.path))
    })
    .inner
}
