//! The Pull requests tab: open pull requests on GitHub, their conversation and checks.

use eframe::egui::{self, RichText, Ui};

use super::ViewContext;
use crate::constants::layout::{COMMIT_DESCRIPTION_ROWS, LIST_ROW_HEIGHT, MIN_TARGET_SIZE, PANEL_PADDING};
use crate::constants::text;
use crate::models::{MergeMethod, PullDetail, PullRequest, ReviewEvent};
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::{Action, PullView};
use crate::ui::components::{
    empty_state, loadable_status, muted_text, option_hint, primary_button, secondary_button,
};
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
    egui::Panel::top(("pull-header", pull.number))
        .frame(egui::Frame::new().inner_margin(egui::Margin { bottom: PANEL_PADDING, ..Default::default() }))
        .show(ui, |ui| header(ui, repo, view, &pull, actions));
    match repo.pull_view {
        PullView::Conversation => {
            egui::ScrollArea::vertical()
                .id_salt(("pull", pull.number))
                .auto_shrink([false, false])
                .show(ui, |ui| conversation(ui, repo, &pull, actions));
        }
        PullView::Files => super::pull_files::files(ui, repo, &pull, actions),
    }
}

/// Title, branches, the buttons, the checks, and the Conversation / Files changed switch.
fn header(
    ui: &mut Ui,
    repo: &RepoStore,
    view: &ViewContext<'_>,
    pull: &PullRequest,
    actions: &mut Vec<Action>,
) {
    let busy = view.busy.is_some();
    let detail = repo.pull_detail.ready();
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
        let open = detail.is_some_and(|detail| !detail.mergeability.merged) && !repo.pull_sending;
        if secondary_button(ui, text::EDIT_PULL, open).clicked() {
            actions.push(Action::OpenEditPull);
        }
        let draft_label = if pull.draft { text::MARK_READY } else { text::CONVERT_TO_DRAFT };
        if secondary_button(ui, draft_label, open).clicked() {
            actions.push(Action::SetDraft { number: pull.number, draft: !pull.draft });
        }
    });
    if let Some(detail) = detail {
        checks(ui, detail);
    }
    ui.horizontal(|ui| {
        let files =
            detail.map_or_else(|| text::FILES_CHANGED.to_owned(), |d| text::files_changed(d.files.len()));
        for (value, label) in
            [(PullView::Conversation, text::CONVERSATION.to_owned()), (PullView::Files, files)]
        {
            let tab = egui::Button::selectable(repo.pull_view == value, label)
                .min_size(egui::vec2(0.0, MIN_TARGET_SIZE));
            if ui.add(tab).clicked() && repo.pull_view != value {
                actions.push(Action::SetPullView(value));
            }
        }
    });
}

fn conversation(ui: &mut Ui, repo: &mut RepoStore, pull: &PullRequest, actions: &mut Vec<Action>) {
    ui.label(muted_text(ui, &text::opened_on(&pull.author, date(&pull.created_at))));
    let body = if pull.body.trim().is_empty() { text::NO_DESCRIPTION } else { pull.body.as_str() };
    ui.add(egui::Label::new(body).wrap());
    ui.add_space(f32::from(PANEL_PADDING));
    let methods = repo.pulls.ready().map(|list| list.merge_methods.clone()).unwrap_or_default();
    if let Loadable::Ready(detail) = &repo.pull_detail {
        merge_box(ui, pull, detail, &methods, &mut repo.pull_merge_method, repo.pull_sending, actions);
        ui.add_space(f32::from(PANEL_PADDING));
    }

    ui.label(RichText::new(text::CONVERSATION).strong());
    match &repo.pull_detail {
        Loadable::Ready(detail) => {
            let outdated: Vec<_> =
                detail.review_comments.iter().filter(|comment| comment.line.is_none()).collect();
            if detail.comments.is_empty() && outdated.is_empty() {
                ui.label(muted_text(ui, text::NO_COMMENTS));
            }
            for comment in &detail.comments {
                let author = match &comment.verdict {
                    Some(verdict) => text::review_entry(&comment.author, verdict),
                    None => comment.author.clone(),
                };
                entry(ui, &author, date(&comment.created_at), "", &comment.body);
            }
            for comment in outdated {
                entry(
                    ui,
                    &comment.author,
                    date(&comment.created_at),
                    &text::outdated_on(&comment.path),
                    &comment.body,
                );
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
    ui.add_space(f32::from(PANEL_PADDING));
    review_box(ui, repo, actions);
}

/// One comment or review in the conversation.
fn entry(ui: &mut Ui, author: &str, date: &str, context: &str, body: &str) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(author).strong());
            ui.label(muted_text(ui, date));
            if !context.is_empty() {
                ui.label(muted_text(ui, context));
            }
        });
        if !body.trim().is_empty() {
            ui.add(egui::Label::new(body).wrap());
        }
    });
}

/// GitHub's merge box: whether it can be merged, the allowed methods and the button.
fn merge_box(
    ui: &mut Ui,
    pull: &PullRequest,
    detail: &PullDetail,
    methods: &[MergeMethod],
    method: &mut MergeMethod,
    sending: bool,
    actions: &mut Vec<Action>,
) {
    egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(PANEL_PADDING)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        let state = &detail.mergeability;
        if state.merged {
            ui.label(RichText::new(text::MERGED).strong());
            return;
        }
        let palette = theme::palette(ui.visuals());
        let (status, color) = match state.state.as_str() {
            _ if pull.draft => (text::MERGE_DRAFT, palette.muted_text),
            _ if state.mergeable.is_none() => (text::MERGE_CHECKING, palette.muted_text),
            "dirty" => (text::MERGE_CONFLICTS, palette.removed),
            "blocked" => (text::MERGE_BLOCKED, palette.conflicted),
            "behind" => (text::MERGE_BEHIND, palette.conflicted),
            "unstable" => (text::MERGE_UNSTABLE, palette.conflicted),
            _ => (text::MERGE_CLEAN, palette.added),
        };
        ui.add(egui::Label::new(RichText::new(status).color(color)).wrap());
        if methods.is_empty() {
            return;
        }
        if !methods.contains(method) {
            *method = methods[0];
        }
        ui.horizontal_wrapped(|ui| {
            for &choice in methods {
                ui.radio_value(method, choice, merge_label(choice));
            }
        });
        let ready = !pull.draft && state.mergeable != Some(false) && !sending;
        if primary_button(ui, merge_label(*method), ready).clicked() {
            actions.push(Action::OpenDialog(Dialog::confirm(
                merge_label(*method),
                text::MERGE_PULL_BODY.to_owned(),
                merge_label(*method),
                Action::MergePull { number: pull.number, method: *method },
            )));
        }
    });
}

fn merge_label(method: MergeMethod) -> &'static str {
    match method {
        MergeMethod::Merge => text::MERGE_PULL,
        MergeMethod::Squash => text::MERGE_PULL_SQUASH,
        MergeMethod::Rebase => text::MERGE_PULL_REBASE,
    }
}

/// GitHub's "Review changes": a message and a verdict.
fn review_box(ui: &mut Ui, repo: &mut RepoStore, actions: &mut Vec<Action>) {
    ui.label(RichText::new(text::REVIEW_CHANGES).strong());
    ui.add(
        egui::TextEdit::multiline(&mut repo.review_body)
            .hint_text(text::REVIEW_HINT)
            .desired_rows(COMMIT_DESCRIPTION_ROWS)
            .desired_width(f32::INFINITY),
    );
    for (event, label, hint) in [
        (ReviewEvent::Comment, text::REVIEW_COMMENT, text::REVIEW_COMMENT_HINT),
        (ReviewEvent::Approve, text::REVIEW_APPROVE, text::REVIEW_APPROVE_HINT),
        (ReviewEvent::RequestChanges, text::REVIEW_REQUEST_CHANGES, text::REVIEW_REQUEST_CHANGES_HINT),
    ] {
        ui.radio_value(&mut repo.review_event, event, label);
        option_hint(ui, hint);
    }
    let has_body = !repo.review_body.trim().is_empty();
    let ready = !repo.pull_sending && (has_body || !repo.review_event.needs_body());
    if primary_button(ui, text::SUBMIT_REVIEW, ready).clicked() {
        actions.push(Action::SubmitReview);
    }
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
pub(super) fn date(timestamp: &str) -> &str {
    timestamp.get(..10).unwrap_or(timestamp)
}
