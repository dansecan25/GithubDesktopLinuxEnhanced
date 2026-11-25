use eframe::egui::{self, PopupCloseBehavior, RichText, Ui};

use super::{ViewContext, context_menus};
use crate::constants::layout::{MIN_TARGET_SIZE, SIDEBAR_DEFAULT_WIDTH};
use crate::constants::text;
use crate::models::{Branch, RepoSnapshot};
use crate::services::settings::ThemeChoice;
use crate::stores::RepoStore;
use crate::ui::actions::Action;
use crate::ui::components::{dropdown_menu, left_clicked, right_click_menu, toolbar_button};
use crate::ui::dialogs::Dialog;
use crate::ui::theme::{self, TOOLBAR_TEXT};
use crate::utils::paths::display_name;

pub fn toolbar(ui: &mut Ui, view: &ViewContext<'_>, repo: Option<&RepoStore>, actions: &mut Vec<Action>) {
    let background = theme::palette(ui.visuals()).toolbar_background;
    egui::Panel::top("toolbar")
        .frame(egui::Frame::new().fill(background).inner_margin(egui::Margin::symmetric(8, 4)))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                repository_menu(ui, view, repo, actions);
                if let Some(repo) = repo
                    && let Some(snapshot) = repo.snapshot.ready()
                {
                    ui.separator();
                    branch_menu(ui, view, repo, snapshot, actions);
                    ui.separator();
                    sync_button(ui, repo, snapshot, view.busy, actions);
                }
                ui.separator();
                if repo.is_some() && icon_button(ui, "↻", text::REFRESH).clicked() {
                    actions.push(Action::Refresh);
                }
                app_menu(ui, view, actions);
            });
        });
}

fn icon_button(ui: &mut Ui, icon: &str, label: &str) -> egui::Response {
    let button = egui::Button::new(RichText::new(icon).size(18.0).color(TOOLBAR_TEXT))
        .frame(false)
        .min_size(egui::vec2(MIN_TARGET_SIZE, MIN_TARGET_SIZE));
    let response = ui.add(button).on_hover_text(label);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    response
}

fn repository_menu(ui: &mut Ui, view: &ViewContext<'_>, repo: Option<&RepoStore>, actions: &mut Vec<Action>) {
    let settings = view.settings;
    let name = repo.map(|repo| display_name(&repo.path)).unwrap_or_else(|| text::NO_REPOSITORY.to_owned());
    let response = toolbar_button(ui, text::CURRENT_REPOSITORY, &format!("{name} ▾"), true);
    if let Some(repo) = repo {
        let github = repo.snapshot.ready().and_then(context_menus::github_url);
        right_click_menu(&response, |ui| {
            context_menus::repository(ui, view, &repo.path, github.as_deref(), actions);
        });
    }
    dropdown_menu(&response).width(SIDEBAR_DEFAULT_WIDTH).show(|ui| {
        ui.label(RichText::new(text::REPOSITORIES).strong());
        if settings.repositories.is_empty() {
            ui.label(text::NO_REPOSITORIES_YET);
        }
        for path in &settings.repositories {
            let selected = repo.is_some_and(|repo| repo.path == *path);
            let row = ui
                .add(
                    egui::Button::selectable(selected, display_name(path))
                        .min_size(egui::vec2(ui.available_width(), MIN_TARGET_SIZE)),
                )
                .on_hover_text(path.to_string_lossy());
            if left_clicked(&row) {
                actions.push(Action::SelectRepository(path.clone()));
            }
            // Only the open repository's remote is known, so only it gets the GitHub link.
            let github = repo
                .filter(|repo| repo.path == *path)
                .and_then(|repo| repo.snapshot.ready())
                .and_then(context_menus::github_url);
            right_click_menu(&row, |ui| {
                context_menus::repository(ui, view, path, github.as_deref(), actions)
            });
        }
        ui.separator();
        if ui.button(text::ADD_EXISTING).clicked() {
            actions.push(Action::OpenDialog(Dialog::add_repository(settings)));
        }
        if ui.button(text::CLONE_REPOSITORY).clicked() {
            actions.push(Action::OpenDialog(Dialog::clone_repository(settings)));
        }
        if ui.button(text::CREATE_REPOSITORY).clicked() {
            actions.push(Action::OpenDialog(Dialog::create_repository(settings)));
        }
    });
}

fn branch_menu(
    ui: &mut Ui,
    view: &ViewContext<'_>,
    repo: &RepoStore,
    snapshot: &RepoSnapshot,
    actions: &mut Vec<Action>,
) {
    let busy = view.busy.is_some();
    let current = snapshot.status.head.clone().unwrap_or_else(|| text::DETACHED_HEAD.to_owned());
    let response = toolbar_button(ui, text::CURRENT_BRANCH, &format!("{current} ▾"), !busy);
    dropdown_menu(&response)
        .width(SIDEBAR_DEFAULT_WIDTH)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if ui.button(text::NEW_BRANCH).clicked() {
                actions.push(Action::OpenDialog(Dialog::create_branch(&current)));
                ui.close();
            }
            let head_commit = repo.history.ready().and_then(|commits| commits.first());
            if ui.add_enabled(head_commit.is_some(), egui::Button::new(text::NEW_TAG)).clicked()
                && let Some(commit) = head_commit
            {
                actions.push(context_menus::tag_dialog(snapshot, commit));
                ui.close();
            }
            let filter_id = ui.scope_id().with("branch-filter");
            let mut filter: String = ui.data(|data| data.get_temp(filter_id).unwrap_or_default());
            ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text(text::FILTER_BRANCHES)
                    .desired_width(f32::INFINITY),
            );
            ui.data_mut(|data| data.insert_temp(filter_id, filter.clone()));
            let filter = filter.to_lowercase();
            let matches =
                |branch: &&Branch| filter.is_empty() || branch.name.to_lowercase().contains(&filter);

            egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                ui.label(RichText::new(text::LOCAL_BRANCHES).strong());
                for branch in snapshot.branches.iter().filter(|b| !b.is_remote).filter(matches) {
                    branch_row(ui, view, snapshot, branch, actions);
                }
                let local: Vec<&str> =
                    snapshot.branches.iter().filter(|b| !b.is_remote).map(|b| b.name.as_str()).collect();
                let remote_only: Vec<&Branch> = snapshot
                    .branches
                    .iter()
                    .filter(|b| b.is_remote)
                    .filter(|b| {
                        !local
                            .iter()
                            .any(|name| b.name.split_once('/').is_some_and(|(_, short)| short == *name))
                    })
                    .filter(matches)
                    .collect();
                if !remote_only.is_empty() {
                    ui.label(RichText::new(text::REMOTE_BRANCHES).strong());
                    for branch in remote_only {
                        branch_row(ui, view, snapshot, branch, actions);
                    }
                }
            });
        });
}

fn branch_row(
    ui: &mut Ui,
    view: &ViewContext<'_>,
    snapshot: &RepoSnapshot,
    branch: &Branch,
    actions: &mut Vec<Action>,
) {
    let row = ui.add(
        egui::Button::selectable(branch.is_current, &branch.name)
            .min_size(egui::vec2(ui.available_width(), MIN_TARGET_SIZE)),
    );
    if left_clicked(&row) && !branch.is_current {
        actions.push(Action::SwitchBranch(branch.clone()));
        ui.close();
    }
    right_click_menu(&row, |ui| context_menus::branch(ui, view, snapshot, branch, actions));
}

/// One button that becomes Publish, Pull, Push or Fetch depending on the branch state, as in
/// GitHub Desktop.
fn sync_button(
    ui: &mut Ui,
    repo: &RepoStore,
    snapshot: &RepoSnapshot,
    busy: Option<&str>,
    actions: &mut Vec<Action>,
) {
    let status = &snapshot.status;
    if let Some(operation) = busy {
        ui.spinner();
        ui.label(RichText::new(text::busy(operation)).color(TOOLBAR_TEXT));
        return;
    }
    if snapshot.remote.is_none() {
        if toolbar_button(ui, text::PUBLISH_REPOSITORY, text::NOT_ON_GITHUB, true).clicked() {
            actions.push(Action::OpenDialog(Dialog::publish(&display_name(&repo.path))));
        }
        return;
    }
    let ahead_behind = text::ahead_behind(status.ahead, status.behind);
    let (caption, action) = if status.upstream.is_none() {
        (text::PUBLISH_BRANCH, Action::Push)
    } else if status.behind > 0 {
        (text::PULL_ORIGIN, Action::Pull)
    } else if status.ahead > 0 {
        (text::PUSH_ORIGIN, Action::Push)
    } else {
        (text::FETCH_ORIGIN, Action::Fetch)
    };
    let detail = if status.upstream.is_none() { text::NEVER_FETCHED } else { &ahead_behind };
    if toolbar_button(ui, caption, detail, status.head.is_some() || matches!(action, Action::Fetch)).clicked()
    {
        actions.push(action);
    }
}

/// Preferences, theme and updates.
fn app_menu(ui: &mut Ui, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    let response = icon_button(ui, "⋯", text::APP_MENU);
    dropdown_menu(&response).show(|ui| {
        if ui.button(text::PREFERENCES).clicked() {
            actions.push(Action::OpenDialog(Dialog::preferences(view.settings, view.editors)));
        }
        ui.separator();
        ui.label(RichText::new(text::THEME).strong());
        for (choice, label) in [
            (ThemeChoice::System, text::THEME_SYSTEM),
            (ThemeChoice::Light, text::THEME_LIGHT),
            (ThemeChoice::Dark, text::THEME_DARK),
        ] {
            if ui.add(egui::Button::selectable(view.settings.theme == choice, label)).clicked() {
                actions.push(Action::SetTheme(choice));
            }
        }
    });
}
