//! Every user-facing string, so wording stays consistent and can be translated later.

// Toolbar
pub const CURRENT_REPOSITORY: &str = "Current repository";
pub const CURRENT_BRANCH: &str = "Current branch";
pub const NO_REPOSITORY: &str = "No repository";
pub const DETACHED_HEAD: &str = "Detached HEAD";
pub const FETCH_ORIGIN: &str = "Fetch origin";
pub const PULL_ORIGIN: &str = "Pull origin";
pub const PUSH_ORIGIN: &str = "Push origin";
pub const PUBLISH_BRANCH: &str = "Publish branch";
pub const NO_REMOTE: &str = "No remote";
pub const NEVER_FETCHED: &str = "Never fetched";
pub const UP_TO_DATE: &str = "Up to date";
pub const REFRESH: &str = "Refresh";
pub const THEME: &str = "Theme";
pub const THEME_SYSTEM: &str = "Follow system";
pub const THEME_LIGHT: &str = "Light";
pub const THEME_DARK: &str = "Dark";

// Repository menu
pub const REPOSITORIES: &str = "Repositories";
pub const ADD_EXISTING: &str = "Add existing repository…";
pub const CLONE_REPOSITORY: &str = "Clone repository…";
pub const CREATE_REPOSITORY: &str = "Create new repository…";
pub const REMOVE_REPOSITORY: &str = "Remove from list";
pub const SHOW_IN_FILE_MANAGER: &str = "Show in file manager";
pub const NO_REPOSITORIES_YET: &str = "No repositories added yet";

// Branch menu
pub const NEW_BRANCH: &str = "New branch…";
pub const LOCAL_BRANCHES: &str = "Branches";
pub const REMOTE_BRANCHES: &str = "Remote branches";
pub const DELETE_BRANCH: &str = "Delete branch…";
pub const FILTER_BRANCHES: &str = "Filter branches";

// Tabs
pub const TAB_CHANGES: &str = "Changes";
pub const TAB_HISTORY: &str = "History";
pub const BACK: &str = "Back";

// Changes
pub const NO_CHANGES_TITLE: &str = "No local changes";
pub const NO_CHANGES_BODY: &str = "There are no uncommitted changes in this repository.";
pub const SELECT_FILE: &str = "Select a file to see its changes";
pub const SUMMARY_HINT: &str = "Summary (required)";
pub const DESCRIPTION_HINT: &str = "Description";
pub const DISCARD_CHANGES: &str = "Discard changes…";
pub const DISCARD_ALL_CHANGES: &str = "Discard all changes…";
pub const STASH_ALL_CHANGES: &str = "Stash all changes";
pub const STASHED_CHANGES: &str = "Stashed changes";
pub const RESTORE_STASH: &str = "Restore";
pub const DISCARD_STASH: &str = "Discard…";
pub const STASH_MESSAGE: &str = "Stashed from GitHub Desktop Linux Enhanced";
pub const INCLUDE_ALL: &str = "Include all changed files in the commit";

// History
pub const NO_HISTORY: &str = "No commits yet";
pub const SELECT_COMMIT: &str = "Select a commit to see what it changed";
pub const SELECT_COMMIT_FILE: &str = "Select a file to see its diff";

// Diff
pub const BINARY_FILE: &str = "This binary file can't be shown.";
pub const DIFF_TRUNCATED: &str = "This diff is too large and was cut short.";
pub const EMPTY_DIFF: &str = "No text changes (for example only the file mode changed).";
pub const LOADING: &str = "Loading…";

// Welcome
pub const WELCOME_TITLE: &str = "Let's get started!";
pub const WELCOME_BODY: &str = "Add a repository to GitHub Desktop Linux Enhanced to start collaborating.";

// Dialogs
pub const CANCEL: &str = "Cancel";
pub const CHOOSE: &str = "Choose…";
pub const LOCAL_PATH: &str = "Local path";
pub const ADD_REPOSITORY_TITLE: &str = "Add local repository";
pub const ADD_REPOSITORY_BUTTON: &str = "Add repository";
pub const CLONE_TITLE: &str = "Clone a repository";
pub const CLONE_URL: &str = "Repository URL or GitHub username/repository";
pub const CLONE_URL_HINT: &str = "https://github.com/owner/repo.git or owner/repo";
pub const CLONE_INTO: &str = "Clone into this folder";
pub const CLONE_BUTTON: &str = "Clone";
pub const CREATE_TITLE: &str = "Create a new repository";
pub const NAME: &str = "Name";
pub const CREATE_IN: &str = "Create in this folder";
pub const CREATE_BUTTON: &str = "Create repository";
pub const CREATE_BRANCH_TITLE: &str = "Create a branch";
pub const CREATE_BRANCH_BUTTON: &str = "Create branch";
pub const INVALID_BRANCH_NAME: &str = "That isn't a valid branch name.";
pub const BRANCH_FROM_PREFIX: &str = "Your new branch will start from";
pub const DISCARD_BUTTON: &str = "Discard changes";
pub const DELETE_BUTTON: &str = "Delete";
pub const DISMISS: &str = "Dismiss";
pub const ERROR_TITLE: &str = "Something went wrong";

// Background operations, shown next to a spinner
pub const COMMITTING: &str = "Committing";
pub const FETCHING: &str = "Fetching";
pub const PULLING: &str = "Pulling";
pub const PUSHING: &str = "Pushing";
pub const SWITCHING_BRANCH: &str = "Switching branch";
pub const CREATING_BRANCH: &str = "Creating branch";
pub const DELETING_BRANCH: &str = "Deleting branch";
pub const STASHING: &str = "Stashing";
pub const RESTORING_STASH: &str = "Restoring stash";
pub const DROPPING_STASH: &str = "Discarding stash";
pub const DISCARDING: &str = "Discarding changes";
pub const OPENING_REPOSITORY: &str = "Opening repository";
pub const CLONING: &str = "Cloning";
pub const CREATING_REPOSITORY: &str = "Creating repository";

pub fn commit_button(branch: &str) -> String {
    format!("Commit to {branch}")
}

pub fn include_in_commit(path: &str) -> String {
    format!("Include {path} in the commit")
}

pub fn changed_files(count: usize) -> String {
    match count {
        1 => "1 changed file".to_owned(),
        _ => format!("{count} changed files"),
    }
}

pub fn discard_files_body(count: usize) -> String {
    let files = if count == 1 { "this file".to_owned() } else { format!("these {count} files") };
    format!("Discard the changes to {files}? This can't be undone. New files are deleted from disk.")
}

pub fn delete_branch_body(name: &str) -> String {
    format!("Delete the local branch \"{name}\"? Git refuses if it has commits that aren't merged.")
}

pub fn drop_stash_body(message: &str) -> String {
    format!("Discard the stash \"{message}\"? This can't be undone.")
}

pub fn ahead_behind(ahead: u32, behind: u32) -> String {
    match (ahead, behind) {
        (0, 0) => UP_TO_DATE.to_owned(),
        (ahead, 0) => format!("↑ {ahead}"),
        (0, behind) => format!("↓ {behind}"),
        (ahead, behind) => format!("↑ {ahead}  ↓ {behind}"),
    }
}

pub fn busy(operation: &str) -> String {
    format!("{operation}…")
}
