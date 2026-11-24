use eframe::egui::{self, Button, Response, RichText, Ui};

use crate::constants::layout::MIN_TARGET_SIZE;
use crate::ui::theme::{self, TOOLBAR_TEXT};

fn sized(button: Button<'_>) -> Button<'_> {
    button.min_size(egui::vec2(MIN_TARGET_SIZE, MIN_TARGET_SIZE))
}

pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let accent = theme::palette(ui.visuals()).accent;
    let label = RichText::new(text).strong().color(egui::Color32::WHITE);
    ui.add_enabled(enabled, sized(Button::new(label).fill(accent)))
}

pub fn secondary_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    ui.add_enabled(enabled, sized(Button::new(text)))
}

pub fn danger_button(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let danger = theme::palette(ui.visuals()).danger;
    let label = RichText::new(text).strong().color(egui::Color32::WHITE);
    ui.add_enabled(enabled, sized(Button::new(label).fill(danger)))
}

/// Two-line toolbar button: a small caption over the value, like GitHub Desktop's toolbar.
pub fn toolbar_button(ui: &mut Ui, caption: &str, value: &str, enabled: bool) -> Response {
    let text = format!("{caption}\n{value}");
    let label = RichText::new(text).color(TOOLBAR_TEXT);
    ui.add_enabled(enabled, sized(Button::new(label).frame(false)))
}
