//! The controller: turns UI actions into background git tasks and task results into state.

use std::path::{Path, PathBuf};

use eframe::egui;

use crate::constants::app::WINDOW_SAVE_DELAY_SECONDS;
use crate::constants::text;
use crate::git::{GitError, GitResult, branches, changes, history, remote, repository, stash};
use crate::services::settings::{self, Settings};
use crate::services::system;
use crate::services::tasks::{OperationKind, TaskOutput, Tasks};
use crate::services::window_state::{self, WindowGeometry, WindowObservation};
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::{Action, Tab};
use crate::ui::components::error_banner;
use crate::ui::dialogs::{self, Dialog};
use crate::ui::{shortcuts, theme, views};

pub struct DesktopApp {
    settings: Settings,
    tasks: Tasks,
    repo: Option<RepoStore>,
    tab: Tab,
    /// In the narrow layout, whether the detail pane (diff) is showing instead of the list.
    detail_open: bool,
    dialog: Option<Dialog>,
    /// Bumped every time a dialog opens, so dialogs can focus their first field once.
    dialog_opening: u64,
    error: Option<String>,
    /// Label of the running git operation. Only one runs at a time.
    busy: Option<&'static str>,
    was_focused: bool,
    checked_monitor_fit: bool,
    /// Window geometry as last written to disk, and when it was written (seconds since start).
    saved_window: WindowGeometry,
    last_window_save: f64,
}

impl DesktopApp {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, settings.theme);
        let selected = settings.selected_repository.clone();
        let saved_window = settings.window;
        let mut app = Self {
            settings,
            tasks: Tasks::new(cc.egui_ctx.clone()),
            repo: None,
            tab: Tab::Changes,
            detail_open: false,
            dialog: None,
            dialog_opening: 0,
            error: None,
            busy: None,
            was_focused: true,
            checked_monitor_fit: false,
            saved_window,
            last_window_save: 0.0,
        };
        if let Some(path) = selected {
            app.open_repository(path);
        }
        app
    }

    fn open_repository(&mut self, path: PathBuf) {
        self.repo = Some(RepoStore::new(path));
        self.tab = Tab::Changes;
        self.detail_open = false;
        self.load_snapshot();
        self.load_history();
    }

    fn repo_path(&self) -> Option<PathBuf> {
        self.repo.as_ref().map(|repo| repo.path.clone())
    }

    fn load_snapshot(&mut self) {
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.snapshot.ready().is_none() {
            repo.snapshot = Loadable::Loading;
        }
        let path = repo.path.clone();
        self.tasks
            .spawn(move || TaskOutput::Snapshot { result: repository::load_snapshot(&path), repo: path });
    }

    fn load_history(&mut self) {
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.history.ready().is_none() {
            repo.history = Loadable::Loading;
        }
        let path = repo.path.clone();
        let has_commits = repo.snapshot.ready().is_none_or(|snapshot| snapshot.status.has_commits);
        self.tasks.spawn(move || {
            // A repository without commits has no log; git reports that as an error.
            let result = if has_commits { history::log(&path) } else { Ok(Vec::new()) };
            TaskOutput::History { result: result.or_else(|_| Ok(Vec::new())), repo: path }
        });
    }

    fn load_working_diff(&mut self) {
        let Some(repo) = self.repo.as_mut() else { return };
        let Some(change) = repo.selected_change().cloned() else { return };
        let has_commits = repo.snapshot.ready().is_some_and(|s| s.status.has_commits);
        if repo.working_diff.ready().is_none() {
            repo.working_diff = Loadable::Loading;
        }
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::WorkingDiff {
            result: changes::working_diff(&path, &change, has_commits),
            file: change.path,
            repo: path,
        });
    }

    /// Runs a git command that changes the repository, then refreshes everything.
    fn run_operation(
        &mut self,
        kind: OperationKind,
        job: impl FnOnce(&Path) -> GitResult<()> + Send + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        let Some(path) = self.repo_path() else { return };
        self.busy = Some(kind.label());
        self.tasks.spawn(move || TaskOutput::Operation { result: job(&path), repo: path, kind });
    }

    fn open_repository_in_background(
        &mut self,
        label: &'static str,
        job: impl FnOnce() -> GitResult<PathBuf> + Send + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(label);
        self.tasks.spawn(move || TaskOutput::RepositoryOpened { result: job() });
    }

    fn save_settings(&mut self) {
        if let Err(error) = settings::save(&self.settings) {
            self.error = Some(format!("Could not save settings: {error}"));
        }
    }

    fn handle(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::OpenDialog(dialog) => {
                self.dialog = Some(dialog);
                self.dialog_opening += 1;
            }
            Action::CloseDialog => self.dialog = None,
            Action::SelectRepository(path) => {
                self.settings.selected_repository = Some(path.clone());
                self.save_settings();
                self.open_repository(path);
            }
            Action::RemoveRepository(path) => {
                self.settings.remove_repository(&path);
                self.save_settings();
                match self.settings.selected_repository.clone() {
                    Some(next) => self.open_repository(next),
                    None => self.repo = None,
                }
            }
            Action::AddRepository(path) => {
                self.open_repository_in_background(text::OPENING_REPOSITORY, move || {
                    repository::find_root(&path)
                });
            }
            Action::CloneRepository { url, parent } => {
                self.open_repository_in_background(text::CLONING, move || repository::clone(&url, &parent));
            }
            Action::CreateRepository(path) => {
                self.open_repository_in_background(text::CREATING_REPOSITORY, move || {
                    if path.exists() && path.read_dir().is_ok_and(|mut entries| entries.next().is_some()) {
                        return Err(GitError::new(format!(
                            "{} already exists and isn't empty",
                            path.display()
                        )));
                    }
                    repository::init(&path)
                });
            }
            Action::ShowInFileManager(path) => {
                if let Err(error) = system::open_in_file_manager(&path) {
                    self.error = Some(error.to_string());
                }
            }
            Action::SetTheme(choice) => {
                self.settings.theme = choice;
                theme::apply(ctx, choice);
                self.save_settings();
            }
            Action::SetTab(tab) => {
                self.tab = tab;
                self.detail_open = false;
            }
            Action::CloseDetail => self.detail_open = false,
            Action::Refresh => {
                self.load_snapshot();
                self.load_history();
            }
            Action::SelectFile(file) => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.selected_file = Some(file);
                    repo.working_diff = Loadable::Loading;
                }
                self.detail_open = true;
                self.load_working_diff();
            }
            Action::SelectCommit(sha) => self.select_commit(sha),
            Action::SelectCommitFile(file) => self.select_commit_file(file),
            Action::Commit => self.commit(),
            Action::Discard(files) => {
                self.run_operation(OperationKind::Discard, move |path| changes::discard(path, &files))
            }
            Action::Fetch => self.run_operation(OperationKind::Fetch, remote::fetch),
            Action::Pull => self.run_operation(OperationKind::Pull, remote::pull),
            Action::Push => {
                let Some(snapshot) = self.repo.as_ref().and_then(|repo| repo.snapshot.ready()) else {
                    return;
                };
                let status = snapshot.status.clone();
                let remote_name = snapshot.remote.clone();
                self.run_operation(OperationKind::Push, move |path| {
                    remote::push(path, &status, remote_name.as_deref())
                });
            }
            Action::SwitchBranch(branch) => {
                self.run_operation(OperationKind::SwitchBranch, move |path| branches::switch(path, &branch));
            }
            Action::CreateBranch(name) => {
                self.dialog = None;
                self.run_operation(OperationKind::CreateBranch, move |path| {
                    if !branches::is_valid_name(path, &name) {
                        return Err(GitError::new(text::INVALID_BRANCH_NAME));
                    }
                    branches::create(path, &name)
                });
            }
            Action::DeleteBranch(name) => {
                self.run_operation(OperationKind::DeleteBranch, move |path| branches::delete(path, &name))
            }
            Action::StashAll => {
                self.run_operation(OperationKind::Stash, |path| stash::push_all(path, text::STASH_MESSAGE))
            }
            Action::RestoreStash(reference) => {
                self.run_operation(OperationKind::RestoreStash, move |path| stash::pop(path, &reference));
            }
            Action::DropStash(reference) => {
                self.run_operation(OperationKind::DropStash, move |path| stash::drop(path, &reference));
            }
        }
    }

    fn select_commit(&mut self, sha: String) {
        let Some(repo) = self.repo.as_mut() else { return };
        repo.selected_commit = Some(sha.clone());
        repo.commit_files = Loadable::Loading;
        repo.selected_commit_file = None;
        repo.commit_diff = Loadable::Idle;
        self.detail_open = true;
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::CommitFiles {
            result: history::commit_files(&path, &sha),
            repo: path,
            sha,
        });
    }

    fn select_commit_file(&mut self, file: String) {
        let Some(repo) = self.repo.as_mut() else { return };
        let Some(sha) = repo.selected_commit.clone() else { return };
        repo.selected_commit_file = Some(file.clone());
        repo.commit_diff = Loadable::Loading;
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::CommitDiff {
            result: history::commit_file_diff(&path, &sha, &file),
            repo: path,
            sha,
            file,
        });
    }

    fn commit(&mut self) {
        let Some(repo) = self.repo.as_ref() else { return };
        if !views::can_commit(repo) {
            return;
        }
        let message = repo.commit_message();
        let chosen = repo.included_changes();
        let all_chosen = chosen.len() == repo.changes().len();
        self.run_operation(OperationKind::Commit, move |path| {
            changes::commit(path, &message, &chosen, all_chosen)
        });
    }

    fn apply_task(&mut self, output: TaskOutput) {
        if let TaskOutput::RepositoryOpened { result } = output {
            self.busy = None;
            match result {
                Ok(path) => {
                    self.dialog = None;
                    self.settings.add_repository(path.clone());
                    self.save_settings();
                    self.open_repository(path);
                }
                Err(error) => self.error = Some(error.message),
            }
            return;
        }

        let current = self.repo_path();
        let Some(repo) = self.repo.as_mut() else { return };
        // Results for a repository that is no longer open are dropped.
        let is_current = |path: &PathBuf| current.as_ref() == Some(path);
        match output {
            TaskOutput::Snapshot { repo: path, result } if is_current(&path) => match result {
                Ok(snapshot) => {
                    repo.apply_snapshot(snapshot);
                    self.load_working_diff();
                }
                Err(error) => repo.snapshot = Loadable::Failed(error.message),
            },
            TaskOutput::WorkingDiff { repo: path, file, result } if is_current(&path) => {
                if repo.selected_file.as_deref() == Some(file.as_str()) {
                    repo.working_diff = Loadable::from_result(result);
                }
            }
            TaskOutput::History { repo: path, result } if is_current(&path) => {
                repo.history = Loadable::from_result(result);
            }
            TaskOutput::CommitFiles { repo: path, sha, result } if is_current(&path) => {
                if repo.selected_commit.as_deref() == Some(sha.as_str()) {
                    repo.commit_files = Loadable::from_result(result);
                }
            }
            TaskOutput::CommitDiff { repo: path, sha, file, result } if is_current(&path) => {
                let still_selected = repo.selected_commit.as_deref() == Some(sha.as_str())
                    && repo.selected_commit_file.as_deref() == Some(file.as_str());
                if still_selected {
                    repo.commit_diff = Loadable::from_result(result);
                }
            }
            TaskOutput::Operation { repo: path, kind, result } => {
                self.busy = None;
                match result {
                    Ok(()) if is_current(&path) => {
                        if kind == OperationKind::Commit {
                            repo.clear_commit_message();
                        }
                        self.load_snapshot();
                        self.load_history();
                    }
                    Ok(()) => {}
                    Err(error) => {
                        self.error = Some(error.message);
                        self.load_snapshot();
                    }
                }
            }
            _ => {}
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if self.dialog.is_some() {
            return;
        }
        let pressed = |shortcut| ctx.input_mut(|input| input.consume_shortcut(&shortcut));
        if pressed(shortcuts::NEW_REPOSITORY) {
            actions.push(Action::OpenDialog(Dialog::create_repository(&self.settings)));
        }
        if pressed(shortcuts::ADD_REPOSITORY) {
            actions.push(Action::OpenDialog(Dialog::add_repository(&self.settings)));
        }
        if pressed(shortcuts::CLONE_REPOSITORY) {
            actions.push(Action::OpenDialog(Dialog::clone_repository(&self.settings)));
        }
        let Some(repo) = self.repo.as_ref() else { return };
        // Consumed before the description field sees it, so Ctrl+Enter commits instead of adding a line.
        if pressed(shortcuts::COMMIT) && self.tab == Tab::Changes {
            actions.push(Action::Commit);
        }
        for (shortcut, action) in [
            (shortcuts::REFRESH, Action::Refresh),
            (shortcuts::SHOW_CHANGES, Action::SetTab(Tab::Changes)),
            (shortcuts::SHOW_HISTORY, Action::SetTab(Tab::History)),
            (shortcuts::PULL, Action::Pull),
            (shortcuts::PUSH, Action::Push),
            (shortcuts::FETCH, Action::Fetch),
        ] {
            if pressed(shortcut) {
                actions.push(action);
            }
        }
        if pressed(shortcuts::NEW_BRANCH) {
            let head = repo.snapshot.ready().and_then(|s| s.status.head.clone()).unwrap_or_default();
            actions.push(Action::OpenDialog(Dialog::create_branch(&head)));
        }
    }

    /// Records the window size for next launch, and refreshes when the window regains focus
    /// (so changes made in an editor or terminal show up, like in GitHub Desktop).
    fn track_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let (observation, monitor) = ctx.input(|input| {
            let viewport = input.viewport();
            let observation = WindowObservation {
                inner_size: viewport.inner_rect.map(|rect| (rect.width(), rect.height())),
                maximized: viewport.maximized,
                minimized: viewport.minimized,
                fullscreen: viewport.fullscreen,
                focused: viewport.focused,
            };
            (observation, viewport.monitor_size.map(|size| (size.x, size.y)))
        });

        if !self.checked_monitor_fit && monitor.is_some() {
            self.checked_monitor_fit = true;
            let fitted = self.settings.window.sanitized(monitor);
            if fitted != self.settings.window && !fitted.maximized {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    fitted.width,
                    fitted.height,
                )));
                self.settings.window = fitted;
            }
        }
        window_state::record(&mut self.settings.window, observation);
        // Saved shortly after a change too, not only on exit, so a crash can't lose it.
        let now = ctx.input(|input| input.time);
        if self.settings.window != self.saved_window {
            if now - self.last_window_save > WINDOW_SAVE_DELAY_SECONDS {
                self.saved_window = self.settings.window;
                self.last_window_save = now;
                self.save_settings();
            } else {
                // Wake up again so the final size of a resize is saved even if no input follows.
                ctx.request_repaint_after(std::time::Duration::from_secs_f64(WINDOW_SAVE_DELAY_SECONDS));
            }
        }

        let focused = observation.focused.unwrap_or(true);
        if focused && !self.was_focused && self.repo.is_some() {
            actions.push(Action::Refresh);
        }
        self.was_focused = focused;
    }
}

impl eframe::App for DesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        for output in self.tasks.finished() {
            self.apply_task(output);
        }

        let mut actions = Vec::new();
        self.track_window(&ctx, &mut actions);
        self.shortcuts(&ctx, &mut actions);

        views::toolbar(ui, &self.settings, self.repo.as_ref(), self.busy, &mut actions);
        if let Some(message) = self.error.clone() {
            egui::Panel::top("error").show(ui, |ui| {
                if error_banner(ui, &message) {
                    self.error = None;
                }
            });
        }
        match self.repo.as_mut() {
            Some(repo) => views::repository_screen(
                ui,
                repo,
                self.tab,
                self.detail_open,
                self.busy.is_some(),
                &mut actions,
            ),
            None => {
                egui::CentralPanel::default().show(ui, |ui| views::welcome(ui, &self.settings, &mut actions));
            }
        }
        if let Some(dialog) = self.dialog.as_mut() {
            dialogs::show(&ctx, dialog, self.dialog_opening, self.busy.is_some(), &mut actions);
        }

        for action in actions {
            self.handle(&ctx, action);
        }
    }

    fn on_exit(&mut self) {
        self.save_settings();
    }
}

/// Reads the saved geometry for the first window, honouring the `--reset-window` flag. A saved size
/// that had to be corrected is written back straight away, so a bad value never survives a launch.
pub fn initial_window(settings: &mut Settings, reset: bool) -> WindowGeometry {
    let loaded = settings.window;
    if reset {
        settings.window = WindowGeometry::default();
    }
    settings.window = settings.window.sanitized(None);
    if settings.window != loaded
        && let Err(error) = settings::save(settings)
    {
        eprintln!("Could not save the corrected window size: {error}");
    }
    settings.window
}
