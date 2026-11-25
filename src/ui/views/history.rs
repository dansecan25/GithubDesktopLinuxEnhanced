use eframe::egui::{self, RichText, Ui};

use crate::constants::layout::{LIST_ROW_HEIGHT, PANEL_PADDING, SIDEBAR_MIN_WIDTH};
use crate::constants::text;
use crate::models::{CommitSummary, RepoSnapshot};
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::Action;
use crate::ui::components::{diff_view, empty_state, file_row, loadable_status, muted_text};
use crate::ui::theme;

use super::{ViewContext, context_menus};

/// Left pane of the History tab: the commit list.
pub fn history_list(ui: &mut Ui, repo: &RepoStore, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
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
            let snapshot = repo.snapshot.ready();
            for commit in &commits[rows] {
                let selected = repo.selected_commit.as_deref() == Some(commit.sha.as_str());
                let tags = tags_on(snapshot, &commit.sha);
                let row = commit_row(ui, commit, &tags, selected, row_height);
                if row.clicked() {
                    actions.push(Action::SelectCommit(commit.sha.clone()));
                }
                row.context_menu(|ui| context_menus::commit(ui, view, snapshot, commit, actions));
            }
        },
    );
}

/// Names of the tags pointing at `sha`.
fn tags_on<'a>(snapshot: Option<&'a RepoSnapshot>, sha: &str) -> Vec<&'a str> {
    snapshot
        .map(|snapshot| {
            snapshot.tags.iter().filter(|tag| tag.commit == sha).map(|tag| tag.name.as_str()).collect()
        })
        .unwrap_or_default()
}

fn tag_label(tags: &[&str]) -> String {
    tags.iter().map(|tag| format!("◆ {tag}")).collect::<Vec<_>>().join("  ")
}

fn commit_row(
    ui: &mut Ui,
    commit: &CommitSummary,
    tags: &[&str],
    selected: bool,
    height: f32,
) -> egui::Response {
    let mut byline = format!("{} · {}", commit.author, commit.relative_date);
    if !tags.is_empty() {
        byline = format!("{byline}  {}", tag_label(tags));
    }
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
pub fn history_detail(
    ui: &mut Ui,
    repo: &RepoStore,
    view: &ViewContext<'_>,
    narrow: bool,
    actions: &mut Vec<Action>,
) {
    let Some(commit) = repo.selected_commit_summary() else {
        return empty_state(ui, text::SELECT_COMMIT, "");
    };
    let snapshot = repo.snapshot.ready();
    let has_remote = snapshot.is_some_and(|snapshot| snapshot.remote.is_some());
    ui.label(RichText::new(&commit.summary).heading());
    let tags = tags_on(snapshot, &commit.sha);
    if !tags.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(muted_text(ui, text::TAGS));
            let accent = theme::palette(ui.visuals()).accent;
            for tag in tags {
                let chip = ui.add(egui::Button::new(RichText::new(format!("◆ {tag}")).color(accent)));
                // A tag has nothing to select, so left and right click both open its menu.
                egui::Popup::menu(&chip).show(|ui| context_menus::tag(ui, view, tag, has_remote, actions));
                chip.context_menu(|ui| context_menus::tag(ui, view, tag, has_remote, actions));
            }
        });
    }
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
                    let row = file_row(ui, file, selected, None);
                    if row.clicked() {
                        actions.push(Action::SelectCommitFile(file.path.clone()));
                    }
                    row.context_menu(|ui| context_menus::commit_file(ui, view, &repo.path, file, actions));
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
