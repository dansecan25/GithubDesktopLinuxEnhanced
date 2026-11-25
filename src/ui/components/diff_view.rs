use std::collections::BTreeSet;

use eframe::egui::{self, FontId, Response, Sense, Ui};

use super::clicks::right_click_menu;
use super::image_diff::image_diff;

use super::messages::{empty_state, loadable_status};
use crate::constants::layout::{
    DIFF_DIVIDER_OPACITY, DIFF_FONT_SIZE, DIFF_LEFT_OUT_OPACITY, DIFF_LINE_NUMBER_WIDTH,
    DIFF_PICKED_GUTTER_OPACITY, DIFF_SELECTION_OPACITY, TAB_AS_SPACES,
};
use crate::constants::text;
use crate::models::{Diff, DiffLine, DiffLineKind, LineToggle};
use crate::stores::Loadable;
use crate::ui::theme;

/// Which lines of the working diff go into the next commit. Clicking a line's gutter (the line
/// numbers) includes or leaves it out; clicking an `@@` line's gutter does the whole hunk.
pub struct LinePicker<'a> {
    pub file_included: bool,
    /// Lines left out; `None` when the whole file is chosen.
    pub excluded: Option<&'a BTreeSet<usize>>,
}

impl LinePicker<'_> {
    fn includes(&self, index: usize) -> bool {
        self.file_included && !self.excluded.is_some_and(|excluded| excluded.contains(&index))
    }

    /// `Some(included)` for lines that can be picked, `None` for the rest.
    fn state(&self, diff: &Diff, index: usize) -> Option<bool> {
        match diff.lines[index].kind {
            kind if kind.is_change() => Some(self.includes(index)),
            DiffLineKind::Hunk => Some(diff.hunk_changes(index).into_iter().any(|line| self.includes(line))),
            _ => None,
        }
    }
}

/// Scrollable unified diff with old/new line numbers. Only visible rows are laid out, so very long
/// diffs stay fast. Lines can be selected and copied, and with a `picker` chosen for the commit.
pub fn diff_view(
    ui: &mut Ui,
    diff: &Loadable<Diff>,
    placeholder: &str,
    id: &str,
    picker: Option<LinePicker<'_>>,
) -> Option<LineToggle> {
    let diff = match diff {
        Loadable::Ready(diff) => diff,
        Loadable::Idle => {
            empty_state(ui, placeholder, "");
            return None;
        }
        other => {
            loadable_status(ui, other);
            return None;
        }
    };
    if let Some(pair) = &diff.image {
        image_diff(ui, pair, id);
        return None;
    }
    if diff.is_binary {
        empty_state(ui, text::BINARY_FILE, "");
        return None;
    }
    if diff.lines.is_empty() {
        empty_state(ui, text::EMPTY_DIFF, "");
        return None;
    }
    let picker = picker.filter(|_| diff.can_pick_lines());
    let mut toggle = None;
    if diff.is_truncated {
        ui.colored_label(theme::palette(ui.visuals()).conflicted, text::DIFF_TRUNCATED);
    }

    let font = FontId::monospace(DIFF_FONT_SIZE);
    let row_height = ui.fonts_mut(|fonts| fonts.row_height(&font)) + 2.0;
    let selection_id = ui.make_persistent_id((id, "selection"));
    let mut selection: Option<Selection> = ui.data(|data| data.get_temp(selection_id));
    if selection.is_some_and(|selection| selection.last() >= diff.lines.len()) {
        selection = None;
    }
    let mut copy = None;
    // `show_rows` works out the visible rows with the spacing set here, so it must match the rows,
    // which touch. With the default spacing it drew too few rows and left the bottom empty.
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::both().id_salt(id).auto_shrink([false, false]).show_rows(
        ui,
        row_height,
        diff.lines.len(),
        |ui, rows| {
            let first = rows.start;
            for (offset, line) in diff.lines[rows].iter().enumerate() {
                let index = first + offset;
                let selected = selection.is_some_and(|selection| selection.contains(index));
                let pick = picker.as_ref().and_then(|picker| picker.state(diff, index));
                let row = diff_row(ui, line, &font, row_height, selected, pick);
                let gutter_right = row.rect.left() + 2.0 * DIFF_LINE_NUMBER_WIDTH;
                let in_gutter = row.interact_pointer_pos().is_some_and(|pos| pos.x < gutter_right);
                if pick.is_some() {
                    if row.hover_pos().is_some_and(|pos| pos.x < gutter_right) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if row.clicked() && in_gutter {
                        toggle = Some(if line.kind == DiffLineKind::Hunk {
                            LineToggle::Hunk(index)
                        } else {
                            LineToggle::Line(index)
                        });
                    }
                }
                let gutter = pick.map(|_| gutter_right);
                select_with_pointer(ui, &row, index, gutter, &mut selection);
                right_click_menu(&row, |ui| {
                    if ui.button(text::COPY_LINES).clicked() {
                        copy = Some(false);
                    }
                    if ui.button(text::COPY_LINES_WITH_MARKERS).clicked() {
                        copy = Some(true);
                    }
                    ui.separator();
                    if ui.button(text::SELECT_ALL_LINES).clicked() {
                        selection =
                            Some(Selection { anchor: 0, cursor: diff.lines.len() - 1, dragging: false });
                    }
                });
            }
        },
    );
    // Ctrl+C, unless a text field has focus and is copying its own text.
    let copy_key = ui.input(|input| input.events.iter().any(|event| matches!(event, egui::Event::Copy)));
    if copy_key && ui.memory(|memory| memory.focused().is_none()) {
        copy = copy.or(Some(false));
    }
    if let (Some(with_markers), Some(selection)) = (copy, selection) {
        ui.ctx().copy_text(copied_text(&diff.lines[selection.first()..=selection.last()], with_markers));
    }
    ui.data_mut(|data| match selection {
        Some(selection) => {
            data.insert_temp(selection_id, selection);
        }
        None => data.remove::<Selection>(selection_id),
    });
    toggle
}

/// Lines picked for copying, by dragging over them or Shift-clicking.
#[derive(Debug, Clone, Copy)]
struct Selection {
    anchor: usize,
    cursor: usize,
    /// The primary button went down on a line and is still held.
    dragging: bool,
}

impl Selection {
    fn first(self) -> usize {
        self.anchor.min(self.cursor)
    }

    fn last(self) -> usize {
        self.anchor.max(self.cursor)
    }

    fn contains(self, index: usize) -> bool {
        (self.first()..=self.last()).contains(&index)
    }
}

/// `gutter` is the right edge of a clickable gutter: presses there pick lines for the commit
/// instead of selecting text.
fn select_with_pointer(
    ui: &Ui,
    row: &Response,
    index: usize,
    gutter: Option<f32>,
    selection: &mut Option<Selection>,
) {
    let (pressed, down, shift) = ui.input(|input| {
        (input.pointer.primary_pressed(), input.pointer.primary_down(), input.modifiers.shift)
    });
    let in_gutter = gutter.is_some_and(|right| row.hover_pos().is_some_and(|pos| pos.x < right));
    if pressed && row.contains_pointer() && !in_gutter {
        *selection = match *selection {
            Some(current) if shift => Some(Selection { cursor: index, dragging: true, ..current }),
            _ => Some(Selection { anchor: index, cursor: index, dragging: true }),
        };
    } else if let Some(current) = selection.as_mut().filter(|current| current.dragging) {
        if !down {
            current.dragging = false;
        } else if row.contains_pointer() {
            current.cursor = index;
        }
    }
    // Right-clicking a line outside the selection selects it, so the menu acts on what was clicked.
    if row.secondary_clicked() && !selection.is_some_and(|current| current.contains(index)) {
        *selection = Some(Selection { anchor: index, cursor: index, dragging: false });
    }
}

/// The code without the diff's `+`, `-` and space markers, unless `with_markers`.
fn copied_text(lines: &[DiffLine], with_markers: bool) -> String {
    let code = |line: &DiffLine| match line.kind {
        DiffLineKind::Added | DiffLineKind::Removed | DiffLineKind::Context if !with_markers => {
            line.text.get(1..).unwrap_or_default().to_owned()
        }
        _ => line.text.clone(),
    };
    lines.iter().map(code).collect::<Vec<_>>().join("\n")
}

/// One diff line. `pick` is `Some(included)` for lines that can be picked for the commit.
fn diff_row(
    ui: &mut Ui,
    line: &DiffLine,
    font: &FontId,
    row_height: f32,
    selected: bool,
    pick: Option<bool>,
) -> Response {
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
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, row_height), Sense::click());

    let painter = ui.painter();
    let background = theme::diff_background(&palette, line.kind);
    // Left-out lines fade, as in GitHub Desktop, so what will be committed stands out.
    let faded =
        if pick == Some(false) { background.gamma_multiply(DIFF_LEFT_OUT_OPACITY) } else { background };
    painter.rect_filled(rect, 0.0, faded);
    if pick == Some(true) {
        let size = egui::vec2(2.0 * DIFF_LINE_NUMBER_WIDTH, rect.height());
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min, size),
            0.0,
            palette.accent.gamma_multiply(DIFF_PICKED_GUTTER_OPACITY),
        );
    }
    if selected {
        painter.rect_filled(rect, 0.0, ui.visuals().selection.bg_fill.gamma_multiply(DIFF_SELECTION_OPACITY));
    }
    let number = |value: Option<u32>| value.map(|n| n.to_string()).unwrap_or_default();
    let y = rect.center().y;
    for (column, value) in [(1.0, line.old_number), (2.0, line.new_number)] {
        let position = egui::pos2(rect.left() + column * DIFF_LINE_NUMBER_WIDTH - 6.0, y);
        painter.text(position, egui::Align2::RIGHT_CENTER, number(value), font.clone(), muted);
    }
    painter.hline(
        rect.x_range(),
        rect.bottom() - 0.5,
        theme::row_divider(ui.visuals(), DIFF_DIVIDER_OPACITY),
    );
    let code_top = y - galley.size().y / 2.0;
    painter.galley(egui::pos2(rect.left() + code_offset, code_top), galley, text_color);
    response.on_hover_cursor(egui::CursorIcon::Text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: DiffLineKind, text: &str) -> DiffLine {
        DiffLine { kind, text: text.to_owned(), old_number: None, new_number: None }
    }

    #[test]
    fn copies_code_without_markers() {
        let lines = [
            line(DiffLineKind::Context, " fn main() {"),
            line(DiffLineKind::Removed, "-    old();"),
            line(DiffLineKind::Added, "+    new();"),
        ];
        assert_eq!(copied_text(&lines, false), "fn main() {\n    old();\n    new();");
        assert_eq!(copied_text(&lines, true), " fn main() {\n-    old();\n+    new();");
    }
}
