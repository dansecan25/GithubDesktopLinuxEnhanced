//! Sizes in logical points. Window limits live here so the saved geometry can always be validated
//! against the same values the window is created with.

pub const DEFAULT_WINDOW_WIDTH: f32 = 1200.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 800.0;

/// The window can never be made, or restored, smaller than this.
pub const MIN_WINDOW_WIDTH: f32 = 360.0;
pub const MIN_WINDOW_HEIGHT: f32 = 480.0;

/// Upper bound used when the monitor size is unknown, to reject corrupted values.
pub const MAX_WINDOW_DIMENSION: f32 = 16384.0;

/// Below this width the two-pane layout collapses into a single column.
pub const NARROW_LAYOUT_WIDTH: f32 = 760.0;

pub const SIDEBAR_DEFAULT_WIDTH: f32 = 280.0;
pub const SIDEBAR_MIN_WIDTH: f32 = 240.0;
pub const SIDEBAR_MAX_WIDTH: f32 = 560.0;

/// Minimum height of anything clickable (buttons, list rows).
pub const MIN_TARGET_SIZE: f32 = 40.0;
pub const LIST_ROW_HEIGHT: f32 = MIN_TARGET_SIZE;

pub const DIFF_FONT_SIZE: f32 = 13.0;
pub const DIFF_LINE_NUMBER_WIDTH: f32 = 48.0;

pub const COMMIT_DESCRIPTION_ROWS: usize = 4;
pub const DIALOG_WIDTH: f32 = 460.0;
pub const PANEL_PADDING: i8 = 8;
pub const TAB_AS_SPACES: &str = "    ";

/// A left click this soon after a right-click press is treated as part of the right-click
/// (some touchpads send both).
pub const CLICK_AFTER_RIGHT_CLICK_SECONDS: f64 = 0.4;
pub const RIGHT_CLICK_MENU_MIN_WIDTH: f32 = 200.0;
