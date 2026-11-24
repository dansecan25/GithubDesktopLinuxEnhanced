use eframe::egui::{self, FontId, Sense, Ui};

use super::messages::{empty_state, loadable_status};
use crate::constants::layout::{DIFF_FONT_SIZE, DIFF_LINE_NUMBER_WIDTH, TAB_AS_SPACES};
use crate::constants::text;
use crate::models::{Diff, DiffLine, DiffLineKind};
use crate::stores::Loadable;
use crate::ui::theme;

/// Scrollable unified diff with old/new line numbers. Only visible rows are laid out, so very long
/// diffs stay fast.
pub fn diff_view(ui: &mut Ui, diff: &Loadable<Diff>, placeholder: &str, id: &str) {
    let diff = match diff {
        Loadable::Ready(diff) => diff,
        Loadable::Idle => return empty_state(ui, placeholder, ""),
        other => return loadable_status(ui, other),
    };
    if diff.is_binary {
        return empty_state(ui, text::BINARY_FILE, "");
    }
    if diff.lines.is_empty() {
        return empty_state(ui, text::EMPTY_DIFF, "");
    }
    if diff.is_truncated {
        ui.colored_label(theme::palette(ui.visuals()).conflicted, text::DIFF_TRUNCATED);
    }

    let font = FontId::monospace(DIFF_FONT_SIZE);
    let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font)) + 2.0;
    egui::ScrollArea::both().id_salt(id).auto_shrink([false, false]).show_rows(
        ui,
        row_height,
        diff.lines.len(),
        |ui, rows| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for line in &diff.lines[rows] {
                diff_row(ui, line, &font, row_height);
            }
        },
    );
}

fn diff_row(ui: &mut Ui, line: &DiffLine, font: &FontId, row_height: f32) {
    let palette = theme::palette(ui.visuals());
    let muted = palette.muted_text;
    let text_color = match line.kind {
        DiffLineKind::Header | DiffLineKind::Note => muted,
        DiffLineKind::Hunk => palette.accent,
        _ => ui.visuals().text_color(),
    };
    let code_offset = 2.0 * DIFF_LINE_NUMBER_WIDTH + 8.0;
    let code = line.text.replace('\t', TAB_AS_SPACES);
    let galley = ui.painter().layout_no_wrap(code, font.clone(), text_color);
    let width = (code_offset + galley.size().x).max(ui.available_width());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, row_height), Sense::hover());

    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, theme::diff_background(&palette, line.kind));
    let number = |value: Option<u32>| value.map(|n| n.to_string()).unwrap_or_default();
    let y = rect.center().y;
    for (column, value) in [(1.0, line.old_number), (2.0, line.new_number)] {
        let position = egui::pos2(rect.left() + column * DIFF_LINE_NUMBER_WIDTH - 6.0, y);
        painter.text(position, egui::Align2::RIGHT_CENTER, number(value), font.clone(), muted);
    }
    let code_top = y - galley.size().y / 2.0;
    painter.galley(egui::pos2(rect.left() + code_offset, code_top), galley, text_color);
}
