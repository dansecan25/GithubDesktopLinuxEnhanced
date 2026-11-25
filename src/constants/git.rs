/// The git executable. Users with a non-standard install can put it on `PATH`.
pub const GIT_BINARY: &str = "git";

/// Hash of the empty tree, used as the diff base in a repository without commits.
pub const EMPTY_TREE_HASH: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

pub const DEFAULT_REMOTE: &str = "origin";
pub const DEFAULT_BRANCH: &str = "main";

/// Commits loaded in the History tab.
pub const HISTORY_LIMIT: usize = 500;

/// Diffs larger than this are cut, so a huge generated file can't freeze the UI.
pub const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;

/// How many bytes are checked for a NUL byte to decide whether an untracked file is binary.
pub const BINARY_SNIFF_BYTES: usize = 8000;

/// `owner/repo` shorthand typed in the clone dialog expands to this host.
pub const GITHUB_HTTPS_PREFIX: &str = "https://github.com/";

/// Field and record separators used in `--format` strings, chosen because they never appear in
/// commit messages typed by people.
pub const FIELD_SEP: char = '\u{1f}';
pub const RECORD_SEP: char = '\u{1e}';

/// Longer diff lines are cut for display (minified files, generated data).
pub const MAX_DIFF_LINE_BYTES: usize = 2000;

/// Most git output read into memory. Past this git is stopped, so a runaway command can't use up
/// the computer's memory. Diffs use the much smaller `MAX_DIFF_BYTES`.
pub const MAX_GIT_OUTPUT_BYTES: usize = 64 * 1024 * 1024;

/// Most of git's error output kept.
pub const MAX_GIT_ERROR_BYTES: usize = 1024 * 1024;

/// Local git commands that take longer than this are stopped.
pub const GIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5 * 60);

/// Commands without a time limit: they wait on the network, a credential prompt or commit hooks,
/// which can all take as long as they need.
pub const UNTIMED_GIT_COMMANDS: &[&str] = &["clone", "fetch", "pull", "push", "commit", "ls-remote"];

/// Longest pause between checks on whether a timed git command has finished.
pub const GIT_POLL_MAX: std::time::Duration = std::time::Duration::from_millis(20);
