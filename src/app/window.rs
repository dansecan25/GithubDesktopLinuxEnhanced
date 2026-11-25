//! Window size tracking, refresh on focus, and keyboard shortcuts.

use eframe::egui;

use super::DesktopApp;
use crate::constants::app::WINDOW_SAVE_DELAY_SECONDS;
use crate::services::settings::{self, Settings};
use crate::services::window_state::{self, WindowGeometry, WindowObservation};
use crate::ui::actions::{Action, Tab};
use crate::ui::dialogs::Dialog;
use crate::ui::shortcuts;

impl DesktopApp {
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if self.dialog.is_some() || self.prompt.is_some() {
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
        if pressed(shortcuts::OPEN_IN_EDITOR) {
            actions.push(Action::OpenInEditor(repo.path.clone()));
        }
        if pressed(shortcuts::SHOW_IN_FILE_MANAGER) {
            actions.push(Action::ShowInFileManager(repo.path.clone()));
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
    pub(super) fn track_window(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
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
