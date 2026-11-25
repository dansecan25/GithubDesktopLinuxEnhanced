use eframe::egui::{self, Ui};

use super::{DialogContext, focus_once, footer_buttons};
use crate::constants::layout::{MAX_LISTED_CONFLICTS, MERGE_BRANCH_LIST_HEIGHT, MIN_TARGET_SIZE};
use crate::constants::text;
use crate::models::{Branch, MergePreview};
use crate::stores::Loadable;
use crate::ui::actions::Action;

/// GitHub Desktop's "Merge into <branch>" dialog: pick a branch, see what the merge would do, then
/// create the merge commit.
#[derive(Debug, Clone)]
pub struct MergeForm {
    /// The current branch.
    pub into: String,
    pub branches: Vec<String>,
    pub filter: String,
    pub selected: Option<String>,
    pub preview: Loadable<MergePreview>,
}

impl MergeForm {
    /// Local branches first, then remote ones, without the current branch.
    pub fn new(into: &str, branches: &[Branch]) -> Self {
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
    let summary = match (&form.selected, &form.preview) {
        (None, _) => text::CHOOSE_BRANCH_TO_MERGE.to_owned(),
        (Some(_), Loadable::Idle | Loadable::Loading) => text::CHECKING_MERGE.to_owned(),
        (Some(_), Loadable::Failed(error)) => error.clone(),
        (Some(from), Loadable::Ready(preview)) => {
            text::merge_preview(from, &form.into, preview.commits, preview.conflicts.as_ref().map(Vec::len))
        }
    };
    ui.add(egui::Label::new(summary).wrap());
    if let Loadable::Ready(preview) = &form.preview {
        for file in preview.conflicts.iter().flatten().take(MAX_LISTED_CONFLICTS) {
            ui.label(format!("! {file}"));
        }
    }
    let ready = form.preview.ready().is_some_and(|preview| preview.commits > 0) && !context.busy;
    if footer_buttons(ui, text::CREATE_MERGE_COMMIT, ready, false, actions)
        && let Some(branch) = form.selected.clone()
    {
        actions.push(Action::CloseDialog);
        actions.push(Action::Merge(branch));
    }
}
