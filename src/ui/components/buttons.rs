use eframe::egui::{self, Button, Response, RichText, Ui};

use crate::constants::layout::{CHECKBOX_RADIUS, MIN_TARGET_SIZE, SEPARATOR_HALF_WIDTH, TOOLBAR_MARGIN};
use crate::ui::theme::{self, TOOLBAR_HOVER, TOOLBAR_TEXT};

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
    // The repository, branch and sync buttons sit between separators.
    let reach = toolbar_reach(ui, true, true);
    with_hover_shading(ui, reach, |ui| ui.add_enabled(enabled, sized(Button::new(label).frame(false))))
}

/// Adds a frameless toolbar button, shaded while the pointer is over it or it is pressed. The
/// shading is square and reaches past the button by `reach` (to the separator lines and the
/// toolbar's edges), and is drawn behind it, in a slot reserved before it.
pub fn with_hover_shading(
    ui: &mut Ui,
    reach: egui::Margin,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    let slot = ui.painter().add(egui::Shape::Noop);
    let response = add(ui);
    if response.enabled() && (response.hovered() || response.is_pointer_button_down_on()) {
        let area = response.rect + reach;
        ui.painter().set(slot, egui::Shape::rect_filled(area, 0.0, TOOLBAR_HOVER));
    }
    response
}

/// How far toolbar shading reaches on each side of a button: to the separator lines, or half the
/// gap to a neighbour without one.
pub fn toolbar_reach(ui: &Ui, separator_left: bool, separator_right: bool) -> egui::Margin {
    let gap = ui.spacing().item_spacing.x;
    let side = |separator: bool| if separator { gap + SEPARATOR_HALF_WIDTH } else { gap / 2.0 };
    let vertical = TOOLBAR_MARGIN.y as i8;
    egui::Margin {
        left: side(separator_left) as i8,
        right: side(separator_right) as i8,
        top: vertical,
        bottom: vertical,
    }
}

/// Draws `add` (a checkbox) with rounded corners; every other widget is square.
pub fn rounded<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        let widgets = &mut ui.visuals_mut().widgets;
        for state in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
            state.corner_radius = egui::CornerRadius::same(CHECKBOX_RADIUS);
        }
        add(ui)
    })
    .inner
}
