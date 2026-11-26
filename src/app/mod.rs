//! The controller: turns UI actions into background tasks, and task results into state.
//!
//! - `mod.rs`: state, startup and the per-frame loop
//! - `handlers.rs`: what each [`Action`] does
//! - `pulls.rs`: pull requests on GitHub
//! - `results.rs`: applying finished background tasks
//! - `window.rs`: window size tracking and keyboard shortcuts

mod handlers;
mod pulls;
mod results;
mod window;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use eframe::egui;

pub use window::initial_window;

use crate::constants::layout::PROGRESS_REPAINT_INTERVAL;
use crate::constants::text;
use crate::git::control::OperationControl;
use crate::services::askpass::{AskpassRequest, AskpassServer, GitHubAccount, SharedAccount};
use crate::services::editor::{self, Editor};
use crate::services::graphics::FrameLimiter;
use crate::services::secrets;
use crate::services::settings::{self, Settings};
use crate::services::tasks::{TaskOutput, Tasks};
use crate::services::update::{self, Updater};
use crate::services::watchdog;
use crate::services::window_state::WindowGeometry;
use crate::stores::RepoStore;
use crate::ui::actions::{Action, Tab};
use crate::ui::components::{error_banner, record_secondary_presses};
use crate::ui::dialogs::{self, Dialog, DialogContext};
use crate::ui::theme;
use crate::ui::views::{self, ViewContext};

pub struct DesktopApp {
    settings: Settings,
    tasks: Tasks,
    repo: Option<RepoStore>,
    tab: Tab,
    /// In the narrow layout, whether the detail pane (diff) is showing instead of the list.
    detail_open: bool,
    dialog: Option<Dialog>,
    /// Bumped every time a dialog or prompt opens, so it can focus its first field once.
    dialog_opening: u64,
    error: Option<String>,
    /// Label of the running git operation. Only one runs at a time.
    busy: Option<&'static str>,
    /// Progress and cancellation for the running operation.
    control: Option<Arc<OperationControl>>,
    cancellable: bool,

    /// Installed code editors, found once at startup.
    editors: Vec<Editor>,
    /// The signed-in GitHub account, shared with the askpass server.
    account: SharedAccount,
    signing_in: bool,
    /// Set to stop waiting for a browser sign-in.
    browser_sign_in: Option<Arc<AtomicBool>>,
    askpass: Option<AskpassServer>,
    /// The credential prompt on screen, and what the user has typed so far.
    prompt: Option<(AskpassRequest, String)>,
    updater: Updater,

    was_focused: bool,
    checked_monitor_fit: bool,
    /// Window geometry as last written to disk, and when it was written (seconds since start).
    saved_window: WindowGeometry,
    last_window_save: f64,
    frame_limiter: FrameLimiter,
}

impl DesktopApp {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        theme::apply(&cc.egui_ctx, settings.theme);
        watchdog::attach(&cc.egui_ctx);
        if let Some(render) = cc.wgpu_render_state.as_ref() {
            let info = render.adapter.get_info();
            watchdog::set_graphics(format!(
                "{} ({:?}, {:?}, driver {} {})",
                info.name, info.device_type, info.backend, info.driver, info.driver_info
            ));
        }
        let account: SharedAccount = Arc::new(Mutex::new(None));
        let (askpass, askpass_error) = match AskpassServer::start(cc.egui_ctx.clone(), Arc::clone(&account)) {
            Ok(server) => (Some(server), None),
            Err(error) => (None, Some(format!("Credential prompts are unavailable: {error}"))),
        };
        let selected = settings.selected_repository.clone();
        let saved_window = settings.window;
        let login = settings.github_login.clone();
        let mut app = Self {
            settings,
            tasks: Tasks::new(cc.egui_ctx.clone()),
            repo: None,
            tab: Tab::Changes,
            detail_open: false,
            dialog: None,
            dialog_opening: 0,
            error: askpass_error.or_else(watchdog::take_last_exit_notice),
            busy: None,
            control: None,
            cancellable: false,
            editors: editor::detect(),
            account,
            signing_in: false,
            browser_sign_in: None,
            askpass,
            prompt: None,
            updater: Updater::default(),
            was_focused: true,
            checked_monitor_fit: false,
            saved_window,
            last_window_save: 0.0,
            frame_limiter: FrameLimiter::default(),
        };
        if let Some(login) = login {
            app.tasks.spawn(move || {
                let account = secrets::load_token()
                    .map(|token| token.map(|token| GitHubAccount { login: login.clone(), token }));
                TaskOutput::AccountLoaded(account)
            });
        }
        if let Some(path) = selected {
            app.open_repository(path);
        }
        app
    }

    fn repo_path(&self) -> Option<PathBuf> {
        self.repo.as_ref().map(|repo| repo.path.clone())
    }

    fn save_settings(&mut self) {
        if let Err(error) = settings::save(&self.settings) {
            self.error = Some(format!("Could not save settings: {error}"));
        }
    }

    /// The command for "Open in editor": the chosen one, else the first installed editor.
    fn editor_command(&self) -> Option<String> {
        self.settings.editor.clone().or_else(|| self.editors.first().map(|editor| editor.command.clone()))
    }

    fn open_in_label(&self) -> String {
        let Some(command) = self.editor_command() else { return text::OPEN_IN_EDITOR_FALLBACK.to_owned() };
        let name = self
            .editors
            .iter()
            .find(|editor| editor.command == command)
            .map(|editor| editor.name.clone())
            .or_else(|| editor::split_command(&command).into_iter().next())
            .unwrap_or(command);
        text::open_in(&name)
    }

    /// Takes the next credential prompt from git, if none is showing.
    fn poll_prompts(&mut self) {
        if self.prompt.is_none()
            && let Some(request) = self.askpass.as_ref().and_then(AskpassServer::next_request)
        {
            self.prompt = Some((request, String::new()));
            self.dialog_opening += 1;
        }
    }

    fn update_banner(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        if !self.updater.dismissed
            && let Some(banner) = views::update_banner(&self.updater.status)
        {
            egui::Panel::top("update").show(ui, |ui| views::show_update_banner(ui, banner, actions));
        }
    }
}

impl eframe::App for DesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.frame_limiter.wait();
        watchdog::frame_started();
        for output in self.tasks.finished() {
            watchdog::set_activity(output.name());
            self.apply_task(output);
        }
        watchdog::set_activity("drawing the window");
        self.updater.poll(&ctx, self.settings.auto_update);
        self.poll_prompts();

        record_secondary_presses(&ctx);
        let mut actions = Vec::new();
        self.track_window(&ctx, &mut actions);
        self.shortcuts(&ctx, &mut actions);

        let open_in_label = self.open_in_label();
        let progress = self.control.as_ref().and_then(|control| control.progress());
        if self.control.is_some() {
            // Keeps the progress bar moving: git's progress doesn't wake the UI by itself.
            ctx.request_repaint_after(PROGRESS_REPAINT_INTERVAL);
        }
        let view = ViewContext {
            settings: &self.settings,
            editors: &self.editors,
            busy: self.busy,
            progress: progress.clone(),
            can_cancel: self.cancellable,
            open_in_label: &open_in_label,
        };
        views::toolbar(ui, &view, self.repo.as_ref(), &mut actions);
        self.update_banner(ui, &mut actions);
        if let Some(message) = self.error.clone() {
            egui::Panel::top("error").show(ui, |ui| {
                if error_banner(ui, &message) {
                    self.error = None;
                }
            });
        }
        let view = ViewContext {
            settings: &self.settings,
            editors: &self.editors,
            busy: self.busy,
            progress: progress.clone(),
            can_cancel: self.cancellable,
            open_in_label: &open_in_label,
        };
        match self.repo.as_mut() {
            Some(repo) => views::repository_screen(ui, repo, self.tab, self.detail_open, &view, &mut actions),
            None => {
                egui::CentralPanel::default().show(ui, |ui| views::welcome(ui, &self.settings, &mut actions));
            }
        }

        if let Some(dialog) = self.dialog.as_mut() {
            let context = DialogContext {
                settings: &self.settings,
                editors: &self.editors,
                update_status: &self.updater.status,
                can_update: update::can_update(),
                signing_in: self.signing_in,
                busy: self.busy.is_some(),
                operation: self.busy,
                progress,
                can_cancel: self.cancellable,
                sending: self.repo.as_ref().is_some_and(|repo| repo.pull_sending),
                opening: self.dialog_opening,
            };
            dialogs::show(&ctx, dialog, &context, &mut actions);
        }
        // The credential prompt goes on top of everything: git is waiting for it.
        if let Some((request, answer)) = self.prompt.as_mut() {
            dialogs::show_prompt(&ctx, &request.prompt, answer, self.dialog_opening, &mut actions);
        }

        for action in actions {
            watchdog::doing("handling", &action);
            self.handle(&ctx, action);
        }
        if let Some(message) = watchdog::frame_finished() {
            self.error = Some(message);
        }
    }

    fn on_exit(&mut self) {
        // A git command waiting on a prompt fails instead of hanging after the window closes.
        if let Some((request, _)) = self.prompt.take() {
            request.answer(None);
        }
        self.save_settings();
    }
}
