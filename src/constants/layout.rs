//! Sizes in logical points. Window limits live here so the saved geometry can always be validated
//! against the same values the window is created with.

use eframe::egui;

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

/// Height of the branch list in the merge dialog.
pub const MERGE_BRANCH_LIST_HEIGHT: f32 = 240.0;
/// Conflicted files listed in the merge dialog before the rest are left out.
pub const MAX_LISTED_CONFLICTS: usize = 10;

/// Width of the progress bar shown while git fetches, pulls, pushes or clones.
pub const PROGRESS_BAR_WIDTH: f32 = 120.0;
/// How often the window redraws while an operation reports progress.
pub const PROGRESS_REPAINT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// How strongly selected diff lines are tinted with the selection colour.
pub const DIFF_SELECTION_OPACITY: f32 = 0.45;
/// Background strength of diff lines left out of the next commit.
pub const DIFF_LEFT_OUT_OPACITY: f32 = 0.3;
/// Strength of the gutter tint on lines chosen for the next commit.
pub const DIFF_PICKED_GUTTER_OPACITY: f32 = 0.55;
/// Lines of each side shown in a conflict block; the rest are summarised.
pub const CONFLICT_SIDE_MAX_LINES: usize = 200;
/// Text size of the code shown for browser sign-in.
pub const SIGN_IN_CODE_SIZE: f32 = 28.0;
/// Below this tab width the Pull requests tab is labelled "PRs".
pub const PULLS_TAB_FULL_LABEL_WIDTH: f32 = 110.0;
/// Strength of the line between diff rows (list rows use the full divider colour).
pub const DIFF_DIVIDER_OPACITY: f32 = 0.35;
/// Space between a file's checkbox and its name.
pub const CHECKBOX_GAP: f32 = 2.0;
/// People suggested at once while typing a co-author.
pub const MAX_CO_AUTHOR_SUGGESTIONS: usize = 5;
/// Inner padding of a file row, small so the name sits next to its checkbox.
pub const FILE_ROW_PADDING: egui::Vec2 = egui::vec2(4.0, 2.0);
/// Height of a row in the changed-files and commit-files lists.
pub const FILE_ROW_HEIGHT: f32 = 28.0;
/// Rounding kept on checkboxes; every other widget has square corners.
pub const CHECKBOX_RADIUS: u8 = 3;
/// The toolbar's inner margin (horizontal, vertical).
pub const TOOLBAR_MARGIN: egui::Vec2 = egui::vec2(8.0, 4.0);
/// Half the width egui gives a separator, so toolbar shading reaches its line.
pub const SEPARATOR_HALF_WIDTH: f32 = 3.0;
/// Longest side, in pixels, an image preview is decoded at. Larger images are shrunk.
pub const MAX_PREVIEW_SIDE: u32 = 2048;
/// Size of the squares in the checkerboard behind transparent images.
pub const CHECKER_SIZE: f32 = 8.0;
/// Below this width the before and after images are shown one above the other.
pub const IMAGE_DIFF_SIDE_BY_SIDE_WIDTH: f32 = 520.0;
