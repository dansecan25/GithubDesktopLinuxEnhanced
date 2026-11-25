//! The Pull requests tab: open pull requests on GitHub, their conversation and checks.

use eframe::egui::{self, RichText, Ui};

use super::ViewContext;
use crate::constants::layout::{COMMIT_DESCRIPTION_ROWS, LIST_ROW_HEIGHT, PANEL_PADDING};
use crate::constants::text;
use crate::models::{PullDetail, PullRequest};
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::Action;
use crate::ui::components::{empty_state, loadable_status, muted_text, primary_button, secondary_button};
use crate::ui::dialogs::Dialog;
use crate::ui::theme;

/// Why pull requests can't be shown, if they can't.
fn unavailable(repo: &RepoStore, view: &ViewContext<'_>) -> Option<(&'static str, &'static str)> {
    if view.settings.github_login.is_none() {
        return Some((text::PULLS_SIGN_IN_TITLE, text::PULLS_SIGN_IN_BODY));
    }
    let on_github = repo.snapshot.ready().and_then(super::context_menus::github_url).is_some();
    (!on_github).then_some((text::PULLS_NOT_GITHUB_TITLE, text::PULLS_NOT_GITHUB_BODY))
}

pub fn pulls_list(ui: &mut Ui, repo: &RepoStore, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    if let Some((title, body)) = unavailable(repo, view) {
        empty_state(ui, title, body);
        if view.settings.github_login.is_none() && secondary_button(ui, text::PREFERENCES, true).clicked() {
            actions.push(Action::OpenDialog(Dialog::preferences(view.settings, view.editors)));
        }
        return;
    }
    ui.horizontal(|ui| {
        let published = repo.snapshot.ready().is_some_and(|snapshot| snapshot.status.upstream.is_some());
        if primary_button(ui, text::NEW_PULL, published)
            .on_disabled_hover_text(text::PUBLISH_BRANCH_FIRST)
            .clicked()
        {
            actions.push(Action::OpenCreatePull);
        }
        if secondary_button(ui, text::REFRESH, !matches!(repo.pulls, Loadable::Loading)).clicked() {
            actions.push(Action::LoadPulls);
        }
    });
    ui.separator();
    let Loadable::Ready(list) = &repo.pulls else { return loadable_status(ui, &repo.pulls) };
    if list.pulls.is_empty() {
        return empty_state(ui, text::NO_PULLS_TITLE, text::NO_PULLS_BODY);
    }
    egui::ScrollArea::vertical().id_salt("pulls").auto_shrink([false, false]).show(ui, |ui| {
        for pull in &list.pulls {
            if pull_row(ui, pull, repo.selected_pull == Some(pull.number)).clicked() {
                actions.push(Action::SelectPull(pull.number));
            }
        }
    });
}

fn pull_row(ui: &mut Ui, pull: &PullRequest, selected: bool) -> egui::Response {
    let style = ui.style();
    let mut label = egui::text::LayoutJob::default();
    let title =
        if pull.draft { format!("{} ({})\n", pull.title, text::DRAFT) } else { format!("{}\n", pull.title) };
    RichText::new(title).append_to(&mut label, style, egui::FontSelection::Default, egui::Align::LEFT);
    let meta = text::pull_meta(pull.number, &pull.author, &pull.head_branch, &pull.base_branch);
    muted_text(ui, &meta).append_to(&mut label, style, egui::FontSelection::Default, egui::Align::LEFT);
    ui.add(
        egui::Button::selectable(selected, label)
            .wrap()
            .min_size(egui::vec2(ui.available_width(), LIST_ROW_HEIGHT * 1.4)),
    )
}

pub fn pull_detail(ui: &mut Ui, repo: &mut RepoStore, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    if unavailable(repo, view).is_some() {
        return;
    }
    let Some(pull) = repo.selected_pull().cloned() else {
        return empty_state(ui, text::SELECT_PULL, "");
    };
    let busy = view.busy.is_some();
    egui::ScrollArea::vertical().id_salt(("pull", pull.number)).auto_shrink([false, false]).show(ui, |ui| {
        ui.heading(&pull.title);
        ui.label(muted_text(
            ui,
            &text::pull_meta(pull.number, &pull.author, &pull.head_branch, &pull.base_branch),
        ));
        if pull.draft {
            ui.label(RichText::new(text::DRAFT).strong());
        }
        ui.horizontal_wrapped(|ui| {
            if secondary_button(ui, text::CHECKOUT_PULL, !busy).clicked() {
                actions.push(Action::CheckoutPull(pull.number));
            }
            if secondary_button(ui, text::OPEN_PULL_ON_GITHUB, true).clicked() {
                actions.push(Action::OpenUrl(pull.url.clone()));
            }
        });
        if let Loadable::Ready(detail) = &repo.pull_detail {
            checks(ui, detail);
        }
        ui.separator();
        ui.label(muted_text(ui, &text::opened_on(&pull.author, date(&pull.created_at))));
        let body = if pull.body.trim().is_empty() { text::NO_DESCRIPTION } else { pull.body.as_str() };
        ui.add(egui::Label::new(body).wrap());
        ui.add_space(f32::from(PANEL_PADDING));

        ui.label(RichText::new(text::CONVERSATION).strong());
        match &repo.pull_detail {
            Loadable::Ready(detail) if detail.comments.is_empty() => {
                ui.label(muted_text(ui, text::NO_COMMENTS));
            }
            Loadable::Ready(detail) => {
                for comment in &detail.comments {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&comment.author).strong());
                            ui.label(muted_text(ui, date(&comment.created_at)));
                        });
                        ui.add(egui::Label::new(&comment.body).wrap());
                    });
                }
            }
            other => loadable_status(ui, other),
        }
        ui.add_space(f32::from(PANEL_PADDING));
        ui.add(
            egui::TextEdit::multiline(&mut repo.pull_comment)
                .hint_text(text::COMMENT_HINT)
                .desired_rows(COMMIT_DESCRIPTION_ROWS)
                .desired_width(f32::INFINITY),
        );
        let ready = !repo.pull_comment.trim().is_empty() && !repo.pull_sending;
        if primary_button(ui, text::COMMENT_BUTTON, ready).clicked() {
            actions.push(Action::PostComment { number: pull.number, body: repo.pull_comment.clone() });
        }
    });
}

fn checks(ui: &mut Ui, detail: &PullDetail) {
    let checks = detail.checks;
    let Some(summary) = text::checks_summary(checks.passed, checks.failed, checks.pending) else { return };
    let palette = theme::palette(ui.visuals());
    let color = if checks.failed > 0 {
        palette.removed
    } else if checks.pending > 0 {
        palette.conflicted
    } else {
        palette.added
    };
    ui.label(RichText::new(summary).color(color));
}

/// `2026-11-24T10:00:00Z` → `2026-11-24`.
fn date(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
}
