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

/// Prints the version and exits. `install.sh` uses it to check a build runs before installing it.
pub const VERSION_FLAG: &str = "--version";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Set on git's environment so this executable, started by git as its askpass helper, can find
/// the running app and ask the user. See `services/askpass.rs`.
pub const ASKPASS_SOCKET_ENV: &str = "GDE_ASKPASS_SOCKET";
pub const ASKPASS_TOKEN_ENV: &str = "GDE_ASKPASS_TOKEN";
