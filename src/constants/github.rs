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

/// The app's OAuth app, for signing in with the browser (GitHub's device flow). A client ID is
/// public by design: the device flow uses no client secret.
pub const OAUTH_CLIENT_ID: &str = "Ov23liIvv68N8dJJ8FLd";
pub const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
pub const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
/// Pull requests and private repositories, organizations (publishing), and pushing workflow files.
pub const OAUTH_SCOPES: &str = "repo read:org workflow";
/// GitHub asks to wait this much longer each time it answers "slow_down".
pub const DEVICE_SLOW_DOWN: Duration = Duration::from_secs(5);

/// Open pull requests listed at most (GitHub's largest page).
pub const MAX_PULLS: u32 = 100;
/// Items per page when reading every page of a list (GitHub's largest page).
pub const PER_PAGE: u32 = 100;
/// Pages read at most per list: 3,000 files, which is all GitHub returns for a pull request.
pub const MAX_PULL_PAGES: u32 = 30;
