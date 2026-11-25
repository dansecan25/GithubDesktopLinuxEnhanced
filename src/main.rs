use eframe::egui;

use github_desktop_enhanced::app::{DesktopApp, initial_window};
use github_desktop_enhanced::constants::app::{APP_ID, APP_NAME, RESET_WINDOW_FLAG, VERSION, VERSION_FLAG};
use github_desktop_enhanced::constants::layout::{MIN_WINDOW_HEIGHT, MIN_WINDOW_WIDTH};
use github_desktop_enhanced::services::{askpass, settings};

fn main() -> eframe::Result {
    // Git starts this executable as its credential helper: answer and exit, no window.
    if let Some(code) = askpass::run_helper() {
        std::process::exit(code);
    }
    if std::env::args().any(|arg| arg == VERSION_FLAG) {
        println!("{APP_NAME} {VERSION}");
        return Ok(());
    }
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
