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
