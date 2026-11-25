use eframe::egui::{self, Button, Response, RichText, Ui};

use super::buttons::rounded;
use crate::constants::layout::{CHECKBOX_GAP, FILE_ROW_HEIGHT, FILE_ROW_PADDING};
use crate::constants::text;
use crate::models::FileChange;
use crate::ui::theme;
use crate::utils::paths::split_file_path;

/// One row of a file list. Pass `included` to show the "include in commit" checkbox.
/// Returns the row's response so callers can attach a context menu.
/// `partial` shows the checkbox as half-ticked: only some of the file's lines are chosen.
pub fn file_row(
    ui: &mut Ui,
    change: &FileChange,
    selected: bool,
    included: Option<&mut bool>,
    partial: bool,
) -> Response {
    let palette = theme::palette(ui.visuals());
    let (letter, kind_name) = theme::change_label(change.kind);
    let (name, folder) = split_file_path(&change.path);

    // `horizontal` starts at `interact_size.y` tall (40 px elsewhere in the app), so it is set first.
    let previous_height = std::mem::replace(&mut ui.spacing_mut().interact_size.y, FILE_ROW_HEIGHT);
    let row = ui.horizontal(|ui| {
        ui.set_min_height(FILE_ROW_HEIGHT);
        ui.spacing_mut().item_spacing.x = CHECKBOX_GAP;
        // The name starts right after the checkbox, as in GitHub Desktop, and the row is compact.
        ui.spacing_mut().button_padding = FILE_ROW_PADDING;
        if let Some(included) = included {
            let accessible_name = text::include_in_commit(&change.path);
            rounded(ui, |ui| {
                // egui makes a checkbox at least `interact_size.y` square; this keeps it to its box.
                let spacing = ui.spacing_mut();
                spacing.interact_size.y = spacing.icon_width;
                ui.add(egui::Checkbox::without_text(included).indeterminate(partial))
            })
            .on_hover_text(&accessible_name)
            .widget_info(|| {
                egui::WidgetInfo::selected(egui::Role::CheckBox, true, *included, &accessible_name)
            });
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
            .min_size(egui::vec2(ui.available_width(), FILE_ROW_HEIGHT));
        ui.add(row).on_hover_text(format!("{kind_name}: {}", change.path))
    });
    ui.spacing_mut().interact_size.y = previous_height;
    list_divider(ui, row.response.rect);
    row.inner
}

/// A line under a list row, halfway into the gap before the next one.
pub fn list_divider(ui: &Ui, row: egui::Rect) {
    let y = row.bottom() + ui.spacing().item_spacing.y / 2.0;
    ui.painter().hline(row.x_range(), y, theme::row_divider(ui.visuals(), 1.0));
}
