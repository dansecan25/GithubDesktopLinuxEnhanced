mod changes;
mod context_menus;
mod history;
mod toolbar;
mod update_banner;
mod welcome;

pub use changes::can_commit;
pub use toolbar::toolbar;
pub use update_banner::{UpdateBanner, show_update_banner, update_banner};
pub use welcome::welcome;

use eframe::egui::{self, Ui};

use crate::constants::layout::{
    NARROW_LAYOUT_WIDTH, PANEL_PADDING, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH,
};
use crate::constants::text;
use crate::services::editor::Editor;
use crate::services::settings::Settings;
use crate::stores::RepoStore;
use crate::ui::actions::{Action, Tab};
use crate::ui::components::secondary_button;

/// Read-only app state the views need.
pub struct ViewContext<'a> {
    pub settings: &'a Settings,
    pub editors: &'a [Editor],
    /// Label of the running git operation, if any.
    pub busy: Option<&'a str>,
    /// "Open in Visual Studio Code", or a generic label when no editor is known.
    pub open_in_label: &'a str,
}

/// The repository screen: a sidebar (tabs + list) and a detail pane, like GitHub Desktop.
/// Below [`NARROW_LAYOUT_WIDTH`] it shows one column at a time, with a Back button on the detail.
pub fn repository_screen(
    ui: &mut Ui,
    repo: &mut RepoStore,
    tab: Tab,
    detail_open: bool,
    view: &ViewContext<'_>,
    actions: &mut Vec<Action>,
) {
    let narrow = ui.available_width() < NARROW_LAYOUT_WIDTH;
    let margin = egui::Margin::same(PANEL_PADDING);
    let central = egui::Frame::central_panel(ui.style()).inner_margin(margin);
    let side = egui::Frame::side_top_panel(ui.style()).inner_margin(margin);

    if narrow {
        egui::CentralPanel::default().frame(central).show(ui, |ui| {
            if detail_open {
                if secondary_button(ui, &format!("← {}", text::BACK), true).clicked() {
                    actions.push(Action::CloseDetail);
                }
                detail(ui, repo, tab, view, true, actions);
            } else {
                sidebar(ui, repo, tab, view, actions);
            }
        });
        return;
    }

    egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(SIDEBAR_DEFAULT_WIDTH)
        .size_range(SIDEBAR_MIN_WIDTH..=SIDEBAR_MAX_WIDTH)
        .frame(side)
        .show(ui, |ui| sidebar(ui, repo, tab, view, actions));
    egui::CentralPanel::default().frame(central).show(ui, |ui| detail(ui, repo, tab, view, false, actions));
}

fn sidebar(ui: &mut Ui, repo: &mut RepoStore, tab: Tab, view: &ViewContext<'_>, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
        for (value, label) in [(Tab::Changes, text::TAB_CHANGES), (Tab::History, text::TAB_HISTORY)] {
            let label = if value == Tab::Changes {
                format!("{label} ({})", repo.changes().len())
            } else {
                label.to_owned()
            };
            let button = egui::Button::selectable(tab == value, label).min_size(egui::vec2(width, 0.0));
            if ui.add(button).clicked() {
                actions.push(Action::SetTab(value));
            }
        }
    });
    ui.separator();
    match tab {
        Tab::Changes => changes::changes_list(ui, repo, view, actions),
        Tab::History => history::history_list(ui, repo, view, actions),
    }
}

fn detail(
    ui: &mut Ui,
    repo: &RepoStore,
    tab: Tab,
    view: &ViewContext<'_>,
    narrow: bool,
    actions: &mut Vec<Action>,
) {
    match tab {
        Tab::Changes => changes::changes_detail(ui, repo),
        Tab::History => history::history_detail(ui, repo, view, narrow, actions),
    }
}
