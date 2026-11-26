//! Shared widgets. Views build on these instead of styling raw egui widgets themselves.

mod buttons;
mod clicks;
mod diff_view;
mod file_row;
mod image_diff;
mod messages;
mod modal;
mod progress;

pub use buttons::{
    danger_button, primary_button, rounded, secondary_button, toolbar_button, toolbar_reach,
    with_hover_shading,
};
pub use clicks::{dropdown_menu, left_clicked, record_secondary_presses, right_click_menu};
pub use diff_view::{LineNote, LineNotes, LinePicker, diff_view, diff_view_with_notes};
pub use file_row::{file_row, list_divider};
pub use messages::{empty_state, error_banner, loadable_status, muted_text, option_hint};
pub use modal::{DialogResult, dialog, field_label, folder_picker};
pub use progress::operation_status;
