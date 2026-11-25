use eframe::egui::{self, RichText, Ui};

use crate::constants::layout::{COMMIT_DESCRIPTION_ROWS, PANEL_PADDING};
use crate::constants::text;
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::Action;
use crate::ui::components::{
    diff_view, empty_state, file_row, loadable_status, primary_button, secondary_button,
};
use crate::ui::dialogs::Dialog;

use super::{ViewContext, context_menus};

/// Left pane of the Changes tab: file list, stashes and the commit form.
pub fn changes_list(ui: &mut Ui, repo: &mut RepoStore, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    let busy = view.busy.is_some();
    let Some(snapshot) = repo.snapshot.ready() else {
        return loadable_status(ui, &repo.snapshot);
    };
    let branch = snapshot.status.head.clone().unwrap_or_else(|| text::DETACHED_HEAD.to_owned());
    let has_stashes = !snapshot.stashes.is_empty();
    let stashes = snapshot.stashes.clone();

    egui::Panel::bottom("commit-form")
        .frame(egui::Frame::new().inner_margin(egui::Margin::same(PANEL_PADDING)))
        .show(ui, |ui| commit_form(ui, repo, &branch, busy, actions));

    if has_stashes {
        egui::Panel::bottom("stashes")
            .frame(egui::Frame::new().inner_margin(egui::Margin::same(PANEL_PADDING)))
            .show(ui, |ui| {
                egui::CollapsingHeader::new(format!("{} ({})", text::STASHED_CHANGES, stashes.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        for stash in &stashes {
                            ui.label(&stash.message);
                            ui.horizontal(|ui| {
                                if secondary_button(ui, text::RESTORE_STASH, !busy).clicked() {
                                    actions.push(Action::RestoreStash(stash.reference.clone()));
                                }
                                if secondary_button(ui, text::DISCARD_STASH, !busy).clicked() {
                                    actions.push(Action::OpenDialog(Dialog::confirm(
                                        text::DISCARD_STASH,
                                        text::drop_stash_body(&stash.message),
                                        text::DELETE_BUTTON,
                                        Action::DropStash(stash.reference.clone()),
                                    )));
                                }
                            });
                        }
                    });
            });
    }

    egui::CentralPanel::no_frame().show(ui, |ui| file_list(ui, repo, view, actions));
}

fn file_list(ui: &mut Ui, repo: &mut RepoStore, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    let busy = view.busy.is_some();
    let changes = repo.changes().to_vec();
    if changes.is_empty() {
        return empty_state(ui, text::NO_CHANGES_TITLE, text::NO_CHANGES_BODY);
    }
    let included_count = changes.iter().filter(|c| repo.is_included(&c.path)).count();

    ui.horizontal(|ui| {
        ui.add_space(f32::from(PANEL_PADDING));
        let mut all = included_count == changes.len();
        let partly = included_count > 0 && !all;
        let toggle = ui
            .add(egui::Checkbox::new(&mut all, text::changed_files(changes.len())).indeterminate(partly))
            .on_hover_text(text::INCLUDE_ALL);
        if toggle.changed() {
            repo.set_all_included(all);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.menu_button("⋯", |ui| {
                if ui.add_enabled(!busy, egui::Button::new(text::STASH_ALL_CHANGES)).clicked() {
                    actions.push(Action::StashAll);
                }
                if ui.add_enabled(!busy, egui::Button::new(text::DISCARD_ALL_CHANGES)).clicked() {
                    actions.push(context_menus::discard_confirmation(changes.clone()));
                }
            });
        });
    });
    ui.separator();

    egui::ScrollArea::vertical().id_salt("changes").auto_shrink([false, false]).show(ui, |ui| {
        for change in &changes {
            let mut included = repo.is_included(&change.path);
            let selected = repo.selected_file.as_deref() == Some(change.path.as_str());
            let row = file_row(ui, change, selected, Some(&mut included));
            if included != repo.is_included(&change.path) {
                repo.set_included(&change.path, included);
            }
            if row.clicked() {
                actions.push(Action::SelectFile(change.path.clone()));
            }
            row.context_menu(|ui| context_menus::working_file(ui, view, &repo.path, change, actions));
        }
    });
}

fn commit_form(ui: &mut Ui, repo: &mut RepoStore, branch: &str, busy: bool, actions: &mut Vec<Action>) {
    ui.add(
        egui::TextEdit::singleline(&mut repo.commit_summary)
            .hint_text(text::SUMMARY_HINT)
            .desired_width(f32::INFINITY),
    );
    ui.add(
        egui::TextEdit::multiline(&mut repo.commit_description)
            .hint_text(text::DESCRIPTION_HINT)
            .desired_rows(COMMIT_DESCRIPTION_ROWS)
            .desired_width(f32::INFINITY),
    );
    let can_commit = can_commit(repo) && !busy;
    let label = text::commit_button(branch);
    ui.scope(|ui| {
        ui.set_width(ui.available_width());
        if primary_button(ui, &label, can_commit).clicked() {
            actions.push(Action::Commit);
        }
    });
}

pub fn can_commit(repo: &RepoStore) -> bool {
    !repo.commit_summary.trim().is_empty() && repo.changes().iter().any(|c| repo.is_included(&c.path))
}

/// Right pane of the Changes tab.
pub fn changes_detail(ui: &mut Ui, repo: &RepoStore) {
    if let Some(path) = &repo.selected_file {
        ui.label(RichText::new(path).strong());
        ui.separator();
    }
    let placeholder = if repo.changes().is_empty() { text::NO_CHANGES_TITLE } else { text::SELECT_FILE };
    let diff = if repo.selected_file.is_some() { &repo.working_diff } else { &Loadable::Idle };
    diff_view(ui, diff, placeholder, "working-diff");
}
