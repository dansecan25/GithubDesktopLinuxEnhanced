use eframe::egui::{self, Ui};

use super::focus_once;
use crate::constants::github::GITHUB_HOST;
use crate::constants::text;
use crate::services::askpass::is_secret_prompt;
use crate::ui::actions::Action;
use crate::ui::components::{DialogResult, dialog, muted_text, primary_button, secondary_button};

/// The credential prompt git or ssh is waiting on. Cancelling makes the git command fail.
pub fn show_prompt(
    ctx: &egui::Context,
    prompt: &str,
    answer: &mut String,
    opening: u64,
    actions: &mut Vec<Action>,
) {
    let result = dialog(ctx, "askpass", text::AUTHENTICATION_TITLE, |ui: &mut Ui| {
        ui.add(egui::Label::new(prompt.trim()).wrap());
        let field = ui.add(
            egui::TextEdit::singleline(answer)
                .password(is_secret_prompt(prompt))
                .desired_width(f32::INFINITY),
        );
        focus_once(ui, &field, opening);
        if prompt.contains(GITHUB_HOST) {
            ui.add(egui::Label::new(muted_text(ui, text::AUTHENTICATION_GITHUB_HINT)).wrap());
        }
        ui.add_space(8.0);
        let enter = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        ui.horizontal(|ui| {
            if primary_button(ui, text::OK, true).clicked() || enter {
                actions.push(Action::AnswerPrompt(Some(std::mem::take(answer))));
            }
            if secondary_button(ui, text::CANCEL, true).clicked() {
                actions.push(Action::AnswerPrompt(None));
            }
        });
    });
    if matches!(result, DialogResult::Closed) {
        actions.push(Action::AnswerPrompt(None));
    }
}
