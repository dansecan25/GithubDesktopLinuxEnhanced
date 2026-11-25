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
pub const PUBLISH_REPOSITORY: &str = "Publish repository";
pub const NOT_ON_GITHUB: &str = "Not on GitHub yet";
pub const NEVER_FETCHED: &str = "Never fetched";
pub const UP_TO_DATE: &str = "Up to date";
pub const REFRESH: &str = "Refresh";
pub const THEME: &str = "Theme";
pub const APP_MENU: &str = "Menu";
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

// Merge
pub const CREATE_MERGE_COMMIT: &str = "Create a merge commit";
pub const NO_OTHER_BRANCHES: &str = "There are no other branches to merge.";
pub const CHOOSE_BRANCH_TO_MERGE: &str = "Choose a branch to see what merging it would do.";
pub const CHECKING_MERGE: &str = "Checking for conflicts…";
pub const MERGE_IN_PROGRESS: &str =
    "Merge in progress. Resolve the files marked with !, then commit to finish the merge.";
pub const ABORT_MERGE: &str = "Abort merge";
pub const ABORT_MERGE_BODY: &str =
    "Your branch goes back to how it was before the merge. Changes made to resolve conflicts are lost.";

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

// Switching branches with changes
pub const SWITCH_BRANCH_TITLE: &str = "Switch branch";
pub const SWITCH_WITH_CHANGES_BODY: &str =
    "You have changes on this branch. What would you like to do with them?";
pub const LEAVE_CHANGES_HINT: &str =
    "Your in-progress work will be stashed on this branch for you to return to later";
pub const BRING_CHANGES_HINT: &str = "Your in-progress work will follow you to the new branch";
pub const SWITCH_BRANCH_BUTTON: &str = "Switch branch";
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
pub const MERGING: &str = "Merging";
pub const ABORTING_MERGE: &str = "Aborting merge";
pub const STASHING: &str = "Stashing";
pub const RESTORING_STASH: &str = "Restoring stash";
pub const DROPPING_STASH: &str = "Discarding stash";
pub const DISCARDING: &str = "Discarding changes";
pub const IGNORING: &str = "Updating .gitignore";
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

// Context menus
pub const OPEN_IN_EDITOR_FALLBACK: &str = "Open in editor";
pub const OPEN_WITH_DEFAULT_APP: &str = "Open with default program";
pub const COPY_PATH: &str = "Copy full path";
pub const COPY_RELATIVE_PATH: &str = "Copy relative path";
pub const COPY_SHA: &str = "Copy commit SHA";
pub const OPEN_ON_GITHUB: &str = "Open repository on GitHub";
pub const VIEW_BRANCH_ON_GITHUB: &str = "View branch on GitHub";
pub const COPY_BRANCH_NAME: &str = "Copy branch name";
pub const IGNORE_FILE: &str = "Ignore file (add to .gitignore)";
pub const CREATE_TAG_HERE: &str = "Create tag…";
pub const PUSH_TAG: &str = "Push tag";
pub const DELETE_TAG: &str = "Delete tag…";

// Preferences
pub const PREFERENCES: &str = "Preferences…";
pub const PREFERENCES_TITLE: &str = "Preferences";
pub const EDITOR: &str = "Code editor";
pub const EDITOR_AUTOMATIC: &str = "Automatic (first installed)";
pub const EDITOR_CUSTOM: &str = "Custom command…";
pub const EDITOR_CUSTOM_HINT: &str = "for example: code --new-window";
pub const EDITOR_NONE_FOUND: &str = "No known editor found. Enter the command that starts yours.";
pub const GITHUB_ACCOUNT: &str = "GitHub account";
pub const TOKEN: &str = "Personal access token";
pub const TOKEN_HINT: &str = "ghp_… or github_pat_…";
pub const TOKEN_HELP: &str = "Create a classic token with the \"repo\" scope (or a fine-grained token with Contents and Administration access). It's stored in the system keychain and used for pushing, pulling and publishing.";
pub const CREATE_TOKEN: &str = "Create a token on GitHub";
pub const SIGN_IN: &str = "Sign in";
pub const SIGN_OUT: &str = "Sign out";
pub const SIGNING_IN: &str = "Signing in";
pub const UPDATES: &str = "Updates";
pub const AUTO_UPDATE: &str = "Check for updates automatically";
pub const CHECK_NOW: &str = "Check now";
pub const UPDATES_ONLY_INSTALLED: &str = "Only a copy installed with install.sh updates itself.";
pub const CHECKING_UPDATES: &str = "Checking for updates…";
pub const UP_TO_DATE_VERSION: &str = "You have the latest version.";
pub const INSTALL: &str = "Install";
pub const LATER: &str = "Later";
pub const RESTART_NOW: &str = "Restart now";
pub const CLOSE: &str = "Close";
pub const SAVE: &str = "Save";

// Credentials prompt
pub const AUTHENTICATION_TITLE: &str = "Authentication required";
pub const AUTHENTICATION_GITHUB_HINT: &str = "GitHub doesn't accept account passwords here: use a personal access token, or sign in under Menu → Preferences so you aren't asked again.";
pub const OK: &str = "OK";

// Publish
pub const PUBLISH_TITLE: &str = "Publish repository";
pub const PUBLISH_TO_GITHUB: &str = "Publish to GitHub";
pub const PUBLISH_DESCRIPTION: &str = "Description";
pub const PUBLISH_PRIVATE: &str = "Keep this code private";
pub const PUBLISH_SIGN_IN_FIRST: &str = "Sign in to GitHub under Menu → Preferences to publish there.";
pub const PUBLISH_EXISTING_REMOTE: &str = "Or connect an existing remote";
pub const REMOTE_URL: &str = "Remote URL";
pub const REMOTE_URL_HINT: &str = "https://github.com/owner/repo.git or git@github.com:owner/repo.git";
pub const ADD_REMOTE_AND_PUSH: &str = "Add remote and push";

// Tags
pub const NEW_TAG: &str = "New tag on current commit…";
pub const CREATE_TAG_TITLE: &str = "Create a tag";
pub const TAG_NAME: &str = "Tag name";
pub const TAG_SUGGESTIONS: &str = "Next version:";
pub const TAG_MESSAGE: &str = "Message (makes an annotated tag, recommended for releases)";
pub const TAG_PUSH: &str = "Push the tag to the remote";
pub const TAG_RELEASE_HINT: &str =
    "Pushing a vX.Y.Z tag starts this project's release workflow, if it has one.";
pub const CREATE_TAG_BUTTON: &str = "Create tag";
pub const TAGS: &str = "Tags";
pub const DELETE_TAG_ON_REMOTE: &str = "Delete locally and on the remote";
pub const DELETE_TAG_LOCALLY: &str = "Delete locally only";
pub const PUBLISHING: &str = "Publishing";
pub const CREATING_TAG: &str = "Creating tag";
pub const PUSHING_TAG: &str = "Pushing tag";
pub const DELETING_TAG: &str = "Deleting tag";
pub const NOTHING_TO_PUBLISH: &str = "Make a first commit, then publish the branch.";
pub const CHOOSE_EDITOR: &str = "No code editor was found. Choose one in Preferences.";

pub fn open_in(editor: &str) -> String {
    format!("Open in {editor}")
}

pub fn signed_in_as(login: &str) -> String {
    format!("Signed in as {login}")
}

pub fn tag_target(short_sha: &str, summary: &str) -> String {
    format!("On commit {short_sha}: {summary}")
}

pub fn delete_tag_body(name: &str) -> String {
    format!("Delete the tag \"{name}\"?")
}

pub fn update_ready(version: &str) -> String {
    format!("Version {version} is ready to install.")
}

pub fn update_downloading(version: &str) -> String {
    format!("Downloading version {version}…")
}

pub fn update_installing(version: &str) -> String {
    format!("Installing version {version}…")
}

pub fn update_installed(version: &str) -> String {
    format!("Version {version} is installed. Restart to use it.")
}

pub fn current_version(version: &str) -> String {
    format!("Current version: {version}")
}

// Watchdog
pub fn window_was_stuck(seconds: u64, report: &str) -> String {
    format!("The window stopped responding for {seconds} s. A report was saved to {report}")
}

pub fn closed_for_memory(used_mib: u64, limit_mib: u64, report: &str) -> String {
    format!(
        "The app closed itself last time because it used {used_mib} MB of memory (limit {limit_mib} MB), \
         before it could slow down your computer. A report was saved to {report}"
    )
}

pub fn merge_into(branch: &str) -> String {
    format!("Choose a branch to merge into {branch}")
}

pub fn merge_title(branch: &str) -> String {
    format!("Merge into {branch}")
}

fn commits(count: usize) -> String {
    if count == 1 { "1 commit".to_owned() } else { format!("{count} commits") }
}

/// The same wording GitHub Desktop uses under its merge button.
pub fn merge_preview(from: &str, into: &str, commit_count: usize, conflicts: Option<usize>) -> String {
    match conflicts {
        _ if commit_count == 0 => format!("This branch is up to date with {into}."),
        Some(0) => format!("This will merge {} from {from} into {into}.", commits(commit_count)),
        Some(1) => format!("There will be 1 conflicted file when merging {from} into {into}."),
        Some(files) => format!("There will be {files} conflicted files when merging {from} into {into}."),
        None => format!(
            "This will merge {} from {from} into {into}. Conflicts can't be checked first with this git version.",
            commits(commit_count)
        ),
    }
}

pub fn unresolved_conflicts(files: &[String]) -> String {
    format!("These files still have conflict markers. Resolve them before committing: {}", files.join(", "))
}

pub fn leave_changes_on(branch: &str) -> String {
    format!("Leave my changes on {branch}")
}

pub fn bring_changes_to(branch: &str) -> String {
    format!("Bring my changes to {branch}")
}

pub fn bring_changes_failed(to: &str, from: &str) -> String {
    format!(
        "Your changes can't come with you: {to} has different versions of the same files. Switch again \
         and choose \u{201c}Leave my changes on {from}\u{201d}, or commit them first."
    )
}

pub fn changes_left_on(branch: &str) -> String {
    format!("Changes left on {branch}")
}
