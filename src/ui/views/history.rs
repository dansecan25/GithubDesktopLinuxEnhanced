use eframe::egui::{self, RichText, Ui};

use crate::constants::layout::{LIST_ROW_HEIGHT, PANEL_PADDING, SIDEBAR_MIN_WIDTH};
use crate::constants::text;
use crate::models::CommitSummary;
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::Action;
use crate::ui::components::{diff_view, empty_state, file_row, loadable_status, muted_text};

/// Left pane of the History tab: the commit list.
pub fn history_list(ui: &mut Ui, repo: &RepoStore, actions: &mut Vec<Action>) {
    let Some(commits) = repo.history.ready() else {
        return loadable_status(ui, &repo.history);
    };
    if commits.is_empty() {
        return empty_state(ui, text::NO_HISTORY, "");
    }
    let row_height = 2.0 * LIST_ROW_HEIGHT * 0.75;
    egui::ScrollArea::vertical().id_salt("history").auto_shrink([false, false]).show_rows(
        ui,
        row_height,
        commits.len(),
        |ui, rows| {
            for commit in &commits[rows] {
                let selected = repo.selected_commit.as_deref() == Some(commit.sha.as_str());
                if commit_row(ui, commit, selected, row_height).clicked() {
                    actions.push(Action::SelectCommit(commit.sha.clone()));
                }
            }
        },
    );
}

fn commit_row(ui: &mut Ui, commit: &CommitSummary, selected: bool, height: f32) -> egui::Response {
    let byline = format!("{} · {}", commit.author, commit.relative_date);
    let style = ui.style();
    let mut label = egui::text::LayoutJob::default();
    RichText::new(format!("{}\n", commit.summary)).append_to(
        &mut label,
        style,
        egui::FontSelection::Default,
        egui::Align::LEFT,
    );
    muted_text(ui, &byline).append_to(&mut label, style, egui::FontSelection::Default, egui::Align::LEFT);
    ui.add(egui::Button::selectable(selected, label).min_size(egui::vec2(ui.available_width(), height)))
        .on_hover_text(&commit.short_sha)
}

/// Right pane of the History tab: commit details, its files and the selected file's diff.
pub fn history_detail(ui: &mut Ui, repo: &RepoStore, narrow: bool, actions: &mut Vec<Action>) {
    let Some(commit) = repo.selected_commit_summary() else {
        return empty_state(ui, text::SELECT_COMMIT, "");
    };
    ui.label(RichText::new(&commit.summary).heading());
    ui.label(muted_text(ui, &format!("{} · {} · {}", commit.short_sha, commit.author, commit.relative_date)));
    if !commit.body.is_empty() {
        ui.add(egui::Label::new(&commit.body).wrap());
    }
    ui.separator();

    let files = |ui: &mut Ui, actions: &mut Vec<Action>| match &repo.commit_files {
        Loadable::Ready(files) => {
            egui::ScrollArea::vertical().id_salt("commit-files").auto_shrink([false, true]).show(ui, |ui| {
                for file in files {
                    let selected = repo.selected_commit_file.as_deref() == Some(file.path.as_str());
                    if file_row(ui, file, selected, None).clicked() {
                        actions.push(Action::SelectCommitFile(file.path.clone()));
                    }
                }
            });
        }
        other => loadable_status(ui, other),
    };
    let diff = if repo.selected_commit_file.is_some() { &repo.commit_diff } else { &Loadable::Idle };

    if narrow {
        egui::Panel::top("commit-files-panel")
            .resizable(true)
            .default_size(200.0)
            .show(ui, |ui| files(ui, actions));
    } else {
        egui::Panel::left("commit-files-panel")
            .frame(
                egui::Frame::new().inner_margin(egui::Margin { right: PANEL_PADDING, ..Default::default() }),
            )
            .resizable(true)
            .default_size(SIDEBAR_MIN_WIDTH)
            .min_size(SIDEBAR_MIN_WIDTH * 0.75)
            .show(ui, |ui| files(ui, actions));
    }
    egui::CentralPanel::no_frame()
        .show(ui, |ui| diff_view(ui, diff, text::SELECT_COMMIT_FILE, "commit-diff"));
}
