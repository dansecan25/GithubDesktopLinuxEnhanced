//! The conflict editor shown in place of the diff for a conflicted file.

use eframe::egui::{self, Color32, FontId, RichText, Ui};

use super::ViewContext;
use crate::constants::layout::{CONFLICT_SIDE_MAX_LINES, DIFF_FONT_SIZE, PANEL_PADDING, TAB_AS_SPACES};
use crate::constants::text;
use crate::models::ConflictChoice;
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::Action;
use crate::ui::components::{loadable_status, primary_button, secondary_button};
use crate::ui::theme;

pub fn conflict_view(ui: &mut Ui, repo: &RepoStore, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    let Some(path) = repo.selected_file.clone() else { return };
    let Loadable::Ready(file) = &repo.conflict else { return loadable_status(ui, &repo.conflict) };
    let busy = view.busy.is_some();
    let branch = repo.snapshot.ready().and_then(|snapshot| snapshot.status.head.clone());
    // `HEAD` means the current branch, which is clearer by name.
    let ours_name = |label: &str| match (&branch, label) {
        (Some(branch), "HEAD" | "") => branch.clone(),
        _ => label.to_owned(),
    };
    let blocks = file.blocks();
    let first = blocks.first();
    let ours_label = first.map_or_else(|| ours_name(""), |block| ours_name(&block.ours_label));
    let theirs_label = first.map_or_else(String::new, |block| block.theirs_label.clone());

    if blocks.is_empty() {
        ui.label(text::NO_CONFLICTS_LEFT);
        if primary_button(ui, text::MARK_RESOLVED, !busy).clicked() {
            actions.push(Action::MarkResolved(path));
        }
        return;
    }
    ui.label(RichText::new(text::conflicts_left(blocks.len())).strong());
    ui.label(text::CONFLICT_HINT);
    ui.horizontal_wrapped(|ui| {
        if secondary_button(ui, &text::use_whole_file(&ours_label), !busy).clicked() {
            actions.push(Action::TakeConflictSide { file: path.clone(), ours: true });
        }
        if !theirs_label.is_empty()
            && secondary_button(ui, &text::use_whole_file(&theirs_label), !busy).clicked()
        {
            actions.push(Action::TakeConflictSide { file: path.clone(), ours: false });
        }
        if secondary_button(ui, view.open_in_label, true).clicked() {
            actions.push(Action::OpenInEditor(repo.path.join(&path)));
        }
    });
    ui.separator();

    let palette = theme::palette(ui.visuals());
    egui::ScrollArea::vertical().id_salt(("conflicts", &path)).auto_shrink([false, false]).show(ui, |ui| {
        for (index, block) in blocks.iter().enumerate() {
            egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(PANEL_PADDING)).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new(text::conflict_number(index + 1, blocks.len())).strong());
                side(ui, &text::yours(&ours_name(&block.ours_label)), &block.ours, palette.hunk_background);
                if let Some(base) = &block.base {
                    side(ui, text::CONFLICT_BASE, base, Color32::TRANSPARENT);
                }
                side(ui, &text::theirs(&block.theirs_label), &block.theirs, palette.added_background);
                choices(ui, busy, |choice| {
                    actions.push(Action::ResolveConflict { file: path.clone(), index, choice });
                });
            });
            ui.add_space(8.0);
        }
    });
}

/// One side of a conflict, in the diff font on a tinted background.
fn side(ui: &mut Ui, label: &str, content: &str, background: Color32) {
    ui.label(RichText::new(label).small());
    let mut lines: Vec<&str> = content.lines().collect();
    let hidden = lines.len().saturating_sub(CONFLICT_SIDE_MAX_LINES);
    lines.truncate(CONFLICT_SIDE_MAX_LINES);
    let mut code = lines.join("\n").replace('\t', TAB_AS_SPACES);
    if hidden > 0 {
        code.push_str(&format!("\n… {hidden} more lines"));
    }
    egui::Frame::new().fill(background).inner_margin(egui::Margin::same(4)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        egui::ScrollArea::horizontal().id_salt(label).show(ui, |ui| {
            ui.add(egui::Label::new(RichText::new(code).font(FontId::monospace(DIFF_FONT_SIZE))).extend());
        });
    });
}

fn choices(ui: &mut Ui, busy: bool, mut choose: impl FnMut(ConflictChoice)) {
    ui.horizontal_wrapped(|ui| {
        for (label, choice) in [
            (text::USE_OURS, ConflictChoice::Ours),
            (text::USE_THEIRS, ConflictChoice::Theirs),
            (text::USE_BOTH, ConflictChoice::Both),
        ] {
            if secondary_button(ui, label, !busy).clicked() {
                choose(choice);
            }
        }
    });
}
