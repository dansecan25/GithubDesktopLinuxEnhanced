use std::path::Path;

use eframe::egui::{self, Popup, PopupCloseBehavior, RichText, Ui};

use crate::constants::layout::{MIN_TARGET_SIZE, SIDEBAR_DEFAULT_WIDTH};
use crate::constants::text;
use crate::models::{Branch, RepoSnapshot};
use crate::services::settings::{Settings, ThemeChoice};
use crate::stores::RepoStore;
use crate::ui::actions::Action;
use crate::ui::components::toolbar_button;
use crate::ui::dialogs::Dialog;
use crate::ui::theme::{self, TOOLBAR_TEXT};
use crate::utils::paths::display_name;

pub fn toolbar(
    ui: &mut Ui,
    settings: &Settings,
    repo: Option<&RepoStore>,
    busy: Option<&str>,
    actions: &mut Vec<Action>,
) {
    let background = theme::palette(ui.visuals()).toolbar_background;
    egui::Panel::top("toolbar")
        .frame(egui::Frame::new().fill(background).inner_margin(egui::Margin::symmetric(8, 4)))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                repository_menu(ui, settings, repo, actions);
                if let Some(snapshot) = repo.and_then(|repo| repo.snapshot.ready()) {
                    ui.separator();
                    branch_menu(ui, snapshot, busy.is_some(), actions);
                    ui.separator();
                    sync_button(ui, snapshot, busy, actions);
                }
                ui.separator();
                if repo.is_some() && icon_button(ui, "↻", text::REFRESH).clicked() {
                    actions.push(Action::Refresh);
                }
                theme_menu(ui, settings.theme, actions);
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

fn repository_menu(ui: &mut Ui, settings: &Settings, repo: Option<&RepoStore>, actions: &mut Vec<Action>) {
    let name = repo.map(|repo| display_name(&repo.path)).unwrap_or_else(|| text::NO_REPOSITORY.to_owned());
    let response = toolbar_button(ui, text::CURRENT_REPOSITORY, &format!("{name} ▾"), true);
    Popup::menu(&response).width(SIDEBAR_DEFAULT_WIDTH).show(|ui| {
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
            if row.clicked() {
                actions.push(Action::SelectRepository(path.clone()));
            }
            repository_context_menu(&row, path, actions);
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

fn repository_context_menu(row: &egui::Response, path: &Path, actions: &mut Vec<Action>) {
    row.context_menu(|ui| {
        if ui.button(text::SHOW_IN_FILE_MANAGER).clicked() {
            actions.push(Action::ShowInFileManager(path.to_path_buf()));
        }
        if ui.button(text::REMOVE_REPOSITORY).clicked() {
            actions.push(Action::RemoveRepository(path.to_path_buf()));
        }
    });
}

fn branch_menu(ui: &mut Ui, snapshot: &RepoSnapshot, busy: bool, actions: &mut Vec<Action>) {
    let current = snapshot.status.head.clone().unwrap_or_else(|| text::DETACHED_HEAD.to_owned());
    let response = toolbar_button(ui, text::CURRENT_BRANCH, &format!("{current} ▾"), !busy);
    Popup::menu(&response)
        .width(SIDEBAR_DEFAULT_WIDTH)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if ui.button(text::NEW_BRANCH).clicked() {
                actions.push(Action::OpenDialog(Dialog::create_branch(&current)));
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
                    branch_row(ui, branch, actions);
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
                        branch_row(ui, branch, actions);
                    }
                }
            });
        });
}

fn branch_row(ui: &mut Ui, branch: &Branch, actions: &mut Vec<Action>) {
    let row = ui.add(
        egui::Button::selectable(branch.is_current, &branch.name)
            .min_size(egui::vec2(ui.available_width(), MIN_TARGET_SIZE)),
    );
    if row.clicked() && !branch.is_current {
        actions.push(Action::SwitchBranch(branch.clone()));
        ui.close();
    }
    if !branch.is_remote && !branch.is_current {
        row.context_menu(|ui| {
            if ui.button(text::DELETE_BRANCH).clicked() {
                actions.push(Action::OpenDialog(Dialog::confirm(
                    text::DELETE_BRANCH,
                    text::delete_branch_body(&branch.name),
                    text::DELETE_BUTTON,
                    Action::DeleteBranch(branch.name.clone()),
                )));
            }
        });
    }
}

/// One button that becomes Publish, Pull, Push or Fetch depending on the branch state, as in
/// GitHub Desktop.
fn sync_button(ui: &mut Ui, snapshot: &RepoSnapshot, busy: Option<&str>, actions: &mut Vec<Action>) {
    let status = &snapshot.status;
    if let Some(operation) = busy {
        ui.spinner();
        ui.label(RichText::new(text::busy(operation)).color(TOOLBAR_TEXT));
        return;
    }
    if snapshot.remote.is_none() {
        toolbar_button(ui, text::NO_REMOTE, "", false);
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

fn theme_menu(ui: &mut Ui, current: ThemeChoice, actions: &mut Vec<Action>) {
    let response = icon_button(ui, "◐", text::THEME);
    Popup::menu(&response).show(|ui| {
        for (choice, label) in [
            (ThemeChoice::System, text::THEME_SYSTEM),
            (ThemeChoice::Light, text::THEME_LIGHT),
            (ThemeChoice::Dark, text::THEME_DARK),
        ] {
            if ui.add(egui::Button::selectable(current == choice, label)).clicked() {
                actions.push(Action::SetTheme(choice));
            }
        }
    });
}
