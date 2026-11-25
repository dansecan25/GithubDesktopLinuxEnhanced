use eframe::egui::{self, Color32, RichText, Ui};

use crate::constants::layout::PROGRESS_BAR_WIDTH;
use crate::constants::text;
use crate::git::control::GitProgress;
use crate::ui::components::secondary_button;

/// The running operation: what git is doing, how far along it is, and a Cancel button when the
/// operation can be stopped. Returns true when Cancel was clicked.
pub fn operation_status(
    ui: &mut Ui,
    label: &str,
    progress: Option<&GitProgress>,
    can_cancel: bool,
    color: Option<Color32>,
) -> bool {
    let paint = |value: String| match color {
        Some(color) => RichText::new(value).color(color),
        None => RichText::new(value),
    };
    ui.horizontal(|ui| {
        ui.spinner();
        match progress {
            Some(progress) => {
                ui.label(paint(progress.phase.clone()));
                if let Some(percent) = progress.percent {
                    ui.add(
                        egui::ProgressBar::new(f32::from(percent) / 100.0)
                            .desired_width(PROGRESS_BAR_WIDTH)
                            .text(format!("{percent}%")),
                    );
                }
            }
            None => {
                ui.label(paint(text::busy(label)));
            }
        }
        can_cancel && secondary_button(ui, text::CANCEL_OPERATION, true).clicked()
    })
    .inner
}
