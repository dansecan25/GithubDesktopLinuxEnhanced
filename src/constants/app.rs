/// Shown in the title bar and about text.
pub const APP_NAME: &str = "GitHub Desktop Linux Enhanced";

/// Wayland app id / X11 WM class. Must match the `.desktop` file name in `packaging/`.
pub const APP_ID: &str = "github-desktop-enhanced";

/// Folder name under the platform config directory (for example `~/.config/<this>`).
pub const CONFIG_DIR_NAME: &str = "github-desktop-enhanced";
pub const SETTINGS_FILE_NAME: &str = "settings.json";

/// Command line flag that ignores the saved window size for one launch.
pub const RESET_WINDOW_FLAG: &str = "--reset-window";

/// Minimum time between two saves of the window size while it is being resized.
pub const WINDOW_SAVE_DELAY_SECONDS: f64 = 2.0;
