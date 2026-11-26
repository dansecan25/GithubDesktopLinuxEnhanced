use eframe::egui::{self, Ui};

use super::{DialogContext, focus_once, footer_buttons};
use crate::constants::layout::{MAX_LISTED_CONFLICTS, MERGE_BRANCH_LIST_HEIGHT, MIN_TARGET_SIZE};
use crate::constants::text;
use crate::models::{Branch, MergeMethod, MergePreview};
use crate::stores::Loadable;
use crate::ui::actions::Action;

/// GitHub Desktop's "Merge into <branch>" dialog: pick a branch, see what the merge would do, then
/// merge, squash and merge, or rebase onto it.
#[derive(Debug, Clone)]
pub struct MergeForm {
    /// The current branch.
    pub into: String,
    pub branches: Vec<String>,
    pub filter: String,
    pub selected: Option<String>,
    pub preview: Loadable<MergePreview>,
    pub method: MergeMethod,
    /// The current branch has uncommitted changes, so it can't be rebased.
    pub has_changes: bool,
}

impl MergeForm {
    /// Local branches first, then remote ones, without the current branch.
    pub fn new(into: &str, branches: &[Branch], has_changes: bool) -> Self {
        let local = branches.iter().filter(|branch| !branch.is_remote);
        let remote = branches.iter().filter(|branch| branch.is_remote);
        let branches = local
            .chain(remote)
            .filter(|branch| !branch.is_current)
            .map(|branch| branch.name.clone())
            .collect();
        Self {
            into: into.to_owned(),
            branches,
            filter: String::new(),
            selected: None,
            preview: Loadable::Idle,
            method: MergeMethod::default(),
            has_changes,
        }
    }
}

pub fn show(ui: &mut Ui, context: &DialogContext<'_>, form: &mut MergeForm, actions: &mut Vec<Action>) {
    if form.branches.is_empty() {
        ui.label(text::NO_OTHER_BRANCHES);
        footer_buttons(ui, text::CREATE_MERGE_COMMIT, false, false, actions);
        return;
    }
    let field = ui.add(
        egui::TextEdit::singleline(&mut form.filter)
            .hint_text(text::FILTER_BRANCHES)
            .desired_width(f32::INFINITY),
    );
    focus_once(ui, &field, context.opening);
    let filter = form.filter.to_lowercase();
    egui::ScrollArea::vertical().max_height(MERGE_BRANCH_LIST_HEIGHT).auto_shrink([false, true]).show(
        ui,
        |ui| {
            for name in
                form.branches.iter().filter(|name| filter.is_empty() || name.to_lowercase().contains(&filter))
            {
                let selected = form.selected.as_deref() == Some(name.as_str());
                let row = egui::Button::selectable(selected, name.as_str())
                    .truncate()
                    .min_size(egui::vec2(ui.available_width(), MIN_TARGET_SIZE));
                if ui.add(row).clicked() && !selected {
                    form.selected = Some(name.clone());
                    form.preview = Loadable::Loading;
                    actions.push(Action::PreviewMerge(name.clone()));
                }
            }
        },
    );
    ui.add_space(8.0);
    ui.add_space(8.0);
    for (method, label, hint) in [
        (MergeMethod::Merge, text::CREATE_MERGE_COMMIT, text::MERGE_METHOD_HINT),
        (MergeMethod::Squash, text::SQUASH_AND_MERGE, text::SQUASH_HINT),
        (MergeMethod::Rebase, text::REBASE, text::REBASE_HINT),
    ] {
        super::choice_value(ui, &mut form.method, method, label, hint);
    }
    let rebase = form.method == MergeMethod::Rebase;
    let summary = match (&form.selected, &form.preview) {
        (None, _) => text::CHOOSE_BRANCH_TO_MERGE.to_owned(),
        (Some(_), Loadable::Idle | Loadable::Loading) => text::CHECKING_MERGE.to_owned(),
        (Some(_), Loadable::Failed(error)) => error.clone(),
        (Some(_), Loadable::Ready(_)) if rebase && form.has_changes => {
            text::REBASE_NEEDS_CLEAN_TREE.to_owned()
        }
        (Some(from), Loadable::Ready(preview)) if rebase => {
            text::rebase_preview(&form.into, from, preview.ahead, preview.commits)
        }
        (Some(from), Loadable::Ready(preview)) => {
            text::merge_preview(from, &form.into, preview.commits, preview.conflicts.as_ref().map(Vec::len))
        }
    };
    ui.add(egui::Label::new(summary).wrap());
    if let Loadable::Ready(preview) = &form.preview
        && !rebase
    {
        for file in preview.conflicts.iter().flatten().take(MAX_LISTED_CONFLICTS) {
            ui.label(format!("! {file}"));
        }
    }
    let ready = form.preview.ready().is_some_and(|preview| preview.commits > 0)
        && !context.busy
        && !(rebase && form.has_changes);
    let label = match &form.selected {
        Some(from) => text::merge_button(form.method, from, &form.into),
        None => text::CREATE_MERGE_COMMIT.to_owned(),
    };
    if footer_buttons(ui, &label, ready, false, actions)
        && let Some(branch) = form.selected.clone()
    {
        actions.push(Action::CloseDialog);
        actions.push(Action::Merge { branch, method: form.method });
    }
}
