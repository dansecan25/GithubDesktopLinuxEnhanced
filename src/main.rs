use eframe::egui;

use github_desktop_enhanced::app::{DesktopApp, initial_window};
use github_desktop_enhanced::constants::app::{APP_ID, APP_NAME, RESET_WINDOW_FLAG};
use github_desktop_enhanced::constants::layout::{MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};
use github_desktop_enhanced::services::settings;

fn main() -> eframe::Result {
    let reset_window = std::env::args().any(|arg| arg == RESET_WINDOW_FLAG);
    let mut settings = settings::load();
    let window = initial_window(&mut settings, reset_window);

    let viewport = egui::ViewportBuilder::default()
        .with_title(APP_NAME)
        .with_app_id(APP_ID)
        .with_inner_size([window.width, window.height])
        .with_min_inner_size([MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT])
        .with_maximized(window.maximized);
    let options = eframe::NativeOptions {
        viewport,
        // The app saves its own, validated window size. See `services/window_state.rs`.
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(DesktopApp::new(cc, settings)))))
}
