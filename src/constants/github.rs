use std::time::Duration;

pub const GITHUB_HOST: &str = "github.com";
pub const API_BASE: &str = "https://api.github.com";

/// This app's own repository, where updates are published.
pub const APP_REPOSITORY: &str = "dansecan25/GithubDesktopLinuxEnhanced";

/// Where people create the personal access token used to sign in.
pub const TOKEN_SETTINGS_URL: &str = "https://github.com/settings/tokens";

/// Keychain service name for the saved GitHub token.
pub const KEYRING_SERVICE: &str = "github-desktop-enhanced";
/// Keychain account name for the saved GitHub token.
pub const KEYRING_ACCOUNT: &str = "github.com";

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub const API_TIMEOUT: Duration = Duration::from_secs(60);
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub const UPDATE_CHECK_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);
pub const MAX_PACKAGE_BYTES: u64 = 200 * 1024 * 1024;
