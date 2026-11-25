use eframe::egui::{self, RichText, Ui};

use super::DialogContext;
use crate::constants::app::VERSION;
use crate::constants::github::TOKEN_SETTINGS_URL;
use crate::constants::text;
use crate::services::settings::ThemeChoice;
use crate::services::update::Status;
use crate::ui::actions::Action;
use crate::ui::components::{field_label, muted_text, primary_button, rounded, secondary_button};

pub fn show(
    ui: &mut Ui,
    context: &DialogContext<'_>,
    token: &mut String,
    custom_editor: &mut String,
    custom_mode: &mut bool,
    actions: &mut Vec<Action>,
) {
    let settings = context.settings;

    field_label(ui, text::THEME);
    ui.horizontal(|ui| {
        for (choice, label) in [
            (ThemeChoice::System, text::THEME_SYSTEM),
            (ThemeChoice::Light, text::THEME_LIGHT),
            (ThemeChoice::Dark, text::THEME_DARK),
        ] {
            if ui.add(egui::Button::selectable(settings.theme == choice, label)).clicked() {
                actions.push(Action::SetTheme(choice));
            }
        }
    });
    ui.separator();

    field_label(ui, text::EDITOR);
    editor_choice(ui, context, custom_editor, custom_mode, actions);
    ui.separator();

    field_label(ui, text::GITHUB_ACCOUNT);
    match &settings.github_login {
        Some(login) => {
            ui.horizontal(|ui| {
                ui.label(text::signed_in_as(login));
                if secondary_button(ui, text::SIGN_OUT, !context.busy).clicked() {
                    actions.push(Action::SignOut);
                }
            });
        }
        None => {
            ui.horizontal(|ui| {
                if primary_button(ui, text::SIGN_IN_WITH_BROWSER, !context.signing_in).clicked() {
                    actions.push(Action::StartBrowserSignIn);
                }
                if context.signing_in {
                    ui.spinner();
                }
            });
            ui.add_space(4.0);
            ui.label(text::OR_USE_TOKEN);
            ui.add(egui::Label::new(muted_text(ui, text::TOKEN_HELP)).wrap());
            ui.hyperlink_to(text::CREATE_TOKEN, TOKEN_SETTINGS_URL);
            ui.add(
                egui::TextEdit::singleline(token)
                    .password(true)
                    .hint_text(text::TOKEN_HINT)
                    .desired_width(f32::INFINITY),
            );
            ui.horizontal(|ui| {
                let ready = !token.trim().is_empty() && !context.signing_in;
                if primary_button(ui, text::SIGN_IN, ready).clicked() {
                    actions.push(Action::SignIn(token.trim().to_owned()));
                    token.clear();
                }
                if context.signing_in {
                    ui.spinner();
                }
            });
        }
    }
    ui.separator();

    field_label(ui, text::UPDATES);
    let mut automatic = settings.auto_update;
    if rounded(ui, |ui| ui.checkbox(&mut automatic, text::AUTO_UPDATE)).changed() {
        actions.push(Action::SetAutoUpdate(automatic));
    }
    ui.horizontal(|ui| {
        let checking = matches!(context.update_status, Status::Checking | Status::Downloading(_));
        if secondary_button(ui, text::CHECK_NOW, context.can_update && !checking).clicked() {
            actions.push(Action::CheckForUpdates);
        }
        ui.label(muted_text(ui, &text::current_version(VERSION)));
    });
    if !context.can_update {
        ui.label(muted_text(ui, text::UPDATES_ONLY_INSTALLED));
    }
    if let Some(status) = update_status_text(context.update_status) {
        ui.add(egui::Label::new(status).wrap());
    }

    ui.add_space(8.0);
    if secondary_button(ui, text::CLOSE, true).clicked() {
        actions.push(Action::CloseDialog);
    }
}

fn editor_choice(
    ui: &mut Ui,
    context: &DialogContext<'_>,
    custom_editor: &mut String,
    custom_mode: &mut bool,
    actions: &mut Vec<Action>,
) {
    let current = context.settings.editor.as_deref();
    let selected_text = if *custom_mode {
        text::EDITOR_CUSTOM.to_owned()
    } else {
        current
            .and_then(|command| context.editors.iter().find(|e| e.command == command))
            .map(|editor| editor.name.clone())
            .unwrap_or_else(|| text::EDITOR_AUTOMATIC.to_owned())
    };
    egui::ComboBox::from_id_salt("editor-choice")
        .selected_text(selected_text)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            if ui
                .add(egui::Button::selectable(current.is_none() && !*custom_mode, text::EDITOR_AUTOMATIC))
                .clicked()
            {
                *custom_mode = false;
                actions.push(Action::SetEditor(None));
            }
            for editor in context.editors {
                let selected = !*custom_mode && current == Some(editor.command.as_str());
                if ui.add(egui::Button::selectable(selected, &editor.name)).clicked() {
                    *custom_mode = false;
                    actions.push(Action::SetEditor(Some(editor.command.clone())));
                }
            }
            if ui.add(egui::Button::selectable(*custom_mode, text::EDITOR_CUSTOM)).clicked() {
                *custom_mode = true;
            }
        });
    if context.editors.is_empty() && !*custom_mode {
        ui.label(muted_text(ui, text::EDITOR_NONE_FOUND));
    }
    if *custom_mode {
        ui.horizontal(|ui| {
            let field = egui::TextEdit::singleline(custom_editor)
                .hint_text(text::EDITOR_CUSTOM_HINT)
                .desired_width(ui.available_width() - 80.0);
            ui.add(field);
            if secondary_button(ui, text::SAVE, !custom_editor.trim().is_empty()).clicked() {
                actions.push(Action::SetEditor(Some(custom_editor.trim().to_owned())));
            }
        });
    }
}

fn update_status_text(status: &Status) -> Option<RichText> {
    let message = match status {
        Status::Idle => return None,
        Status::Checking => text::CHECKING_UPDATES.to_owned(),
        Status::UpToDate => text::UP_TO_DATE_VERSION.to_owned(),
        Status::CheckFailed(error) | Status::InstallFailed(error) => error.clone(),
        Status::Downloading(version) => text::update_downloading(version),
        Status::Ready(update) => text::update_ready(&update.version),
        Status::Installing(version) => text::update_installing(version),
        Status::Installed(version) => text::update_installed(version),
    };
    Some(RichText::new(message))
}
