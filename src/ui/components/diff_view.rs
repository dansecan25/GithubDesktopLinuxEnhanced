use std::collections::{BTreeSet, HashMap};

use eframe::egui::{self, FontId, Response, Sense, Ui};

use super::clicks::right_click_menu;
use super::image_diff::image_diff;

use super::messages::{empty_state, loadable_status};
use crate::constants::layout::{
    DIFF_DIVIDER_OPACITY, DIFF_FONT_SIZE, DIFF_LEFT_OUT_OPACITY, DIFF_LINE_NUMBER_WIDTH, DIFF_NOTE_MARGIN,
    DIFF_NOTE_MIN_COLUMNS, DIFF_PICKED_GUTTER_OPACITY, DIFF_SELECTION_OPACITY, TAB_AS_SPACES,
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

/// One comment shown under a diff line.
#[derive(Debug, Clone)]
pub struct LineNote {
    /// Who wrote it and when.
    pub header: String,
    pub body: String,
}

/// Comments shown under the diff lines they are about (a pull request's line comments), by index
/// into the diff's lines. Clicking a line number asks to comment on that line.
pub struct LineNotes<'a> {
    pub notes: &'a HashMap<usize, Vec<LineNote>>,
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
    render(ui, diff, id, picker, None)
}

/// A diff with comments under its lines. Returns the index of a line whose number was clicked.
pub fn diff_view_with_notes(ui: &mut Ui, diff: &Diff, id: &str, notes: &LineNotes<'_>) -> Option<usize> {
    match render(ui, diff, id, None, Some(notes)) {
        Some(LineToggle::Line(index)) => Some(index),
        _ => None,
    }
}

/// What one row of the diff shows: a line, or one wrapped line of a comment under it.
enum Row {
    Line(usize),
    NoteHeader(String),
    NoteText(String),
    NoteGap,
}

fn render(
    ui: &mut Ui,
    diff: &Diff,
    id: &str,
    picker: Option<LinePicker<'_>>,
    notes: Option<&LineNotes<'_>>,
) -> Option<LineToggle> {
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
    let rows = layout_rows(ui, diff, notes, &font);
    // `show_rows` works out the visible rows with the spacing set here, so it must match the rows,
    // which touch. With the default spacing it drew too few rows and left the bottom empty.
    ui.spacing_mut().item_spacing.y = 0.0;
    egui::ScrollArea::both().id_salt(id).auto_shrink([false, false]).show_rows(
        ui,
        row_height,
        rows.len(),
        |ui, visible| {
            for row in &rows[visible] {
                let index = match row {
                    Row::Line(index) => *index,
                    note => {
                        note_row(ui, note, row_height);
                        continue;
                    }
                };
                let line = &diff.lines[index];
                let selected = selection.is_some_and(|selection| selection.contains(index));
                let pick = picker.as_ref().and_then(|picker| picker.state(diff, index));
                // With comments, any line with a number can be commented on from its gutter.
                let commentable = notes.is_some() && (line.old_number.is_some() || line.new_number.is_some());
                let row = diff_row(ui, line, &font, row_height, selected, pick);
                let gutter_right = row.rect.left() + 2.0 * DIFF_LINE_NUMBER_WIDTH;
                let in_gutter = row.interact_pointer_pos().is_some_and(|pos| pos.x < gutter_right);
                if commentable {
                    if row.hover_pos().is_some_and(|pos| pos.x < gutter_right) {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if row.clicked() && in_gutter {
                        toggle = Some(LineToggle::Line(index));
                    }
                }
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
                let gutter = (pick.is_some() || commentable).then_some(gutter_right);
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

/// The rows to draw: every line, each followed by its comments wrapped to the diff's width.
fn layout_rows(ui: &Ui, diff: &Diff, notes: Option<&LineNotes<'_>>, font: &FontId) -> Vec<Row> {
    let Some(notes) = notes.filter(|notes| !notes.notes.is_empty()) else {
        return (0..diff.lines.len()).map(Row::Line).collect();
    };
    let char_width = ui.painter().layout_no_wrap("M".to_owned(), font.clone(), egui::Color32::WHITE).size().x;
    let text_width = ui.available_width() - code_offset() - DIFF_NOTE_MARGIN;
    let columns = ((text_width / char_width.max(1.0)) as usize).max(DIFF_NOTE_MIN_COLUMNS);
    let mut rows = Vec::with_capacity(diff.lines.len());
    for index in 0..diff.lines.len() {
        rows.push(Row::Line(index));
        for note in notes.notes.get(&index).into_iter().flatten() {
            rows.push(Row::NoteHeader(note.header.clone()));
            rows.extend(wrap(&note.body, columns).into_iter().map(Row::NoteText));
            rows.push(Row::NoteGap);
        }
    }
    rows
}

/// Splits `text` into lines of at most `columns` characters, at spaces where possible.
fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let fits = line.chars().count() + 1 + word.chars().count() <= columns;
            if !line.is_empty() && !fits {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word);
            // A word longer than a line is cut.
            while line.chars().count() > columns {
                let cut: String = line.chars().take(columns).collect();
                line = line.chars().skip(columns).collect();
                lines.push(cut);
            }
        }
        lines.push(line);
    }
    lines
}

/// Where the code starts, past the two line-number columns.
fn code_offset() -> f32 {
    2.0 * DIFF_LINE_NUMBER_WIDTH + 8.0
}

/// A row of a comment under a line: tinted, indented like the code.
fn note_row(ui: &mut Ui, row: &Row, row_height: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), row_height), Sense::hover());
    let palette = theme::palette(ui.visuals());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
    painter.vline(rect.left() + code_offset() - 4.0, rect.y_range(), egui::Stroke::new(2.0, palette.accent));
    let position = egui::pos2(rect.left() + code_offset(), rect.center().y);
    let font = FontId::proportional(DIFF_FONT_SIZE);
    match row {
        Row::NoteHeader(header) => {
            painter.text(position, egui::Align2::LEFT_CENTER, header, font, palette.accent);
        }
        Row::NoteText(text) => {
            painter.text(position, egui::Align2::LEFT_CENTER, text, font, ui.visuals().text_color());
        }
        Row::NoteGap | Row::Line(_) => {}
    }
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
    let code_offset = code_offset();
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
    #[test]
    fn notes_wrap_at_spaces_and_cut_long_words() {
        assert_eq!(super::wrap("one two three", 7), ["one two", "three"]);
        assert_eq!(super::wrap("abcdefghij", 4), ["abcd", "efgh", "ij"]);
        assert_eq!(super::wrap("a\n\nb", 10), ["a", "", "b"]);
    }

    use super::*;

    fn line(kind: DiffLineKind, text: &str) -> DiffLine {
        DiffLine { kind, text: text.to_owned(), old_number: None, new_number: None, crlf: false }
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
