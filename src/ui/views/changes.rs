use eframe::egui::{self, RichText, Ui};

use crate::constants::layout::{COMMIT_DESCRIPTION_ROWS, MAX_CO_AUTHOR_SUGGESTIONS, PANEL_PADDING};
use crate::constants::text;
use crate::models::{ChangeKind, CoAuthor, FileChange};
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::Action;
use crate::ui::components::{
    LinePicker, diff_view, empty_state, file_row, left_clicked, loadable_status, primary_button,
    right_click_menu, rounded, secondary_button,
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
    // Like GitHub Desktop, a branch only shows the changes left on it (plus stashes made elsewhere).
    let stashes: Vec<_> = snapshot
        .stashes
        .iter()
        .filter(|stash| stash.branch.is_none() || stash.branch == snapshot.status.head)
        .cloned()
        .collect();
    let has_stashes = !stashes.is_empty();

    if snapshot.merge_message.is_some() {
        egui::Panel::top("merge-in-progress")
            .frame(egui::Frame::new().inner_margin(egui::Margin::same(PANEL_PADDING)))
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(text::MERGE_IN_PROGRESS).strong()).wrap());
                if secondary_button(ui, text::ABORT_MERGE, !busy).clicked() {
                    actions.push(Action::OpenDialog(Dialog::confirm(
                        text::ABORT_MERGE,
                        text::ABORT_MERGE_BODY.to_owned(),
                        text::ABORT_MERGE,
                        Action::AbortMerge,
                    )));
                }
            });
    }

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
                            let label = match &stash.branch {
                                Some(branch) => text::changes_left_on(branch),
                                None => stash.message.clone(),
                            };
                            ui.label(&label);
                            ui.horizontal(|ui| {
                                if secondary_button(ui, text::RESTORE_STASH, !busy).clicked() {
                                    actions.push(Action::RestoreStash(stash.reference.clone()));
                                }
                                if secondary_button(ui, text::DISCARD_STASH, !busy).clicked() {
                                    actions.push(Action::OpenDialog(Dialog::confirm(
                                        text::DISCARD_STASH,
                                        text::drop_stash_body(&label),
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
        let toggle = rounded(ui, |ui| {
            ui.add(egui::Checkbox::new(&mut all, text::changed_files(changes.len())).indeterminate(partly))
        })
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
            let partial = included && repo.is_partial(&change.path);
            let row = file_row(ui, change, selected, Some(&mut included), partial);
            if included != repo.is_included(&change.path) {
                repo.set_included(&change.path, included);
            }
            if left_clicked(&row) {
                actions.push(Action::SelectFile(change.path.clone()));
            }
            right_click_menu(&row, |ui| context_menus::working_file(ui, view, &repo.path, change, actions));
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
    co_authors(ui, repo);
    let can_commit = can_commit(repo) && !busy;
    let label = text::commit_button(branch);
    ui.scope(|ui| {
        ui.set_width(ui.available_width());
        if primary_button(ui, &label, can_commit).clicked() {
            actions.push(Action::Commit);
        }
    });
}

/// Lines can be picked in modified and new files, but not during a merge (git commits a merge
/// whole) or in renames, deletions and conflicts.
fn can_pick_lines(repo: &RepoStore, change: &FileChange) -> bool {
    let merging = repo.snapshot.ready().is_some_and(|snapshot| snapshot.merge_message.is_some());
    !merging && matches!(change.kind, ChangeKind::Modified | ChangeKind::Added | ChangeKind::Untracked)
}

/// GitHub Desktop's "Add co-authors": people credited on the commit with `Co-authored-by:` lines.
/// Type `Name <email>` and press Enter, or pick someone who has committed to this repository.
fn co_authors(ui: &mut Ui, repo: &mut RepoStore) {
    if !repo.show_co_authors {
        if ui.small_button(text::ADD_CO_AUTHORS).clicked() {
            repo.show_co_authors = true;
        }
        return;
    }
    ui.horizontal_wrapped(|ui| {
        let mut removed = None;
        for (index, author) in repo.co_authors.iter().enumerate() {
            let chip = ui.small_button(format!("{} ×", author.name)).on_hover_text(&author.email);
            if chip.clicked() {
                removed = Some(index);
            }
        }
        if let Some(index) = removed {
            repo.co_authors.remove(index);
        }
    });
    let field = ui.add(
        egui::TextEdit::singleline(&mut repo.co_author_query)
            .hint_text(text::CO_AUTHOR_HINT)
            .desired_width(f32::INFINITY),
    );
    if field.lost_focus()
        && ui.input(|input| input.key_pressed(egui::Key::Enter))
        && let Some(author) = CoAuthor::parse(&repo.co_author_query)
    {
        add_co_author(repo, author);
        field.request_focus();
    }
    let query = repo.co_author_query.trim().to_lowercase();
    if !query.is_empty() {
        let matches: Vec<CoAuthor> = repo
            .known_authors()
            .into_iter()
            .filter(|author| {
                !repo.co_authors.iter().any(|added| added.email.eq_ignore_ascii_case(&author.email))
            })
            .filter(|author| {
                author.name.to_lowercase().contains(&query) || author.email.to_lowercase().contains(&query)
            })
            .take(MAX_CO_AUTHOR_SUGGESTIONS)
            .collect();
        for author in matches {
            let label = format!("{} <{}>", author.name, author.email);
            if ui.add(egui::Button::selectable(false, label).truncate()).clicked() {
                add_co_author(repo, author);
            }
        }
    }
    if ui.small_button(text::REMOVE_CO_AUTHORS).clicked() {
        repo.show_co_authors = false;
        repo.co_authors.clear();
        repo.co_author_query.clear();
    }
}

fn add_co_author(repo: &mut RepoStore, author: CoAuthor) {
    if !repo.co_authors.iter().any(|added| added.email.eq_ignore_ascii_case(&author.email)) {
        repo.co_authors.push(author);
    }
    repo.co_author_query.clear();
}

pub fn can_commit(repo: &RepoStore) -> bool {
    !repo.commit_summary.trim().is_empty() && repo.changes().iter().any(|c| repo.is_included(&c.path))
}

/// Right pane of the Changes tab.
pub fn changes_detail(ui: &mut Ui, repo: &mut RepoStore, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    if let Some(path) = &repo.selected_file {
        ui.label(RichText::new(path).strong());
        ui.separator();
    }
    if repo.selected_change().is_some_and(|change| change.kind == ChangeKind::Conflicted) {
        return super::conflict::conflict_view(ui, repo, view, actions);
    }
    let placeholder = if repo.changes().is_empty() { text::NO_CHANGES_TITLE } else { text::SELECT_FILE };
    let diff = if repo.selected_file.is_some() { &repo.working_diff } else { &Loadable::Idle };
    let path = repo.selected_file.clone().unwrap_or_default();
    let picker = repo.selected_change().filter(|change| can_pick_lines(repo, change)).map(|_| LinePicker {
        file_included: repo.is_included(&path),
        excluded: repo.line_selections.get(&path).map(|selection| &selection.excluded),
    });
    let toggle = diff_view(ui, diff, placeholder, &format!("working-diff-{path}"), picker);
    if let (Some(toggle), Loadable::Ready(diff)) = (toggle, &repo.working_diff) {
        let diff = diff.clone();
        repo.toggle_lines(&path, &diff, toggle);
    }
}
