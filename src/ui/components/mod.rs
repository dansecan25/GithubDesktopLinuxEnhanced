//! Shared widgets. Views build on these instead of styling raw egui widgets themselves.

mod buttons;
mod diff_view;
mod file_row;
mod messages;
mod modal;

pub use buttons::{danger_button, primary_button, secondary_button, toolbar_button};
pub use diff_view::diff_view;
pub use file_row::file_row;
pub use messages::{empty_state, error_banner, loadable_status, muted_text};
pub use modal::{DialogResult, dialog, field_label, folder_picker};
