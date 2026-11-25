//! Applying finished background tasks to the app state.

use std::path::PathBuf;

use super::DesktopApp;
use crate::constants::text;
use crate::services::askpass::GitHubAccount;
use crate::services::tasks::{OperationKind, TaskOutput};
use crate::stores::Loadable;
use crate::ui::dialogs::Dialog;

impl DesktopApp {
    pub(super) fn apply_task(&mut self, output: TaskOutput) {
        match output {
            TaskOutput::RepositoryOpened { result, remember_folder } => {
                self.finish_operation();
                match result {
                    Ok(path) => {
                        self.dialog = None;
                        if remember_folder.is_some() {
                            self.settings.projects_directory = remember_folder;
                        }
                        self.settings.add_repository(path.clone());
                        self.save_settings();
                        self.open_repository(path);
                    }
                    Err(error) if error.cancelled => {}
                    Err(error) => self.error = Some(error.message),
                }
            }
            TaskOutput::AccountLoaded(result) => match result {
                Ok(Some(account)) => self.set_account(Some(account)),
                // The token is gone from the keychain, so the account can't be used: show it as
                // signed out instead of signed in without credentials.
                Ok(None) => {
                    self.settings.github_login = None;
                    self.set_account(None);
                    self.save_settings();
                }
                Err(error) => self.error = Some(format!("Could not read the saved GitHub token: {error}")),
            },
            TaskOutput::SignedIn(result) => {
                self.signing_in = false;
                self.browser_sign_in = None;
                if matches!(self.dialog, Some(Dialog::BrowserSignIn { .. })) {
                    self.dialog = None;
                }
                match result {
                    Ok(account) => {
                        self.settings.github_login = Some(account.login.clone());
                        self.set_account(Some(account));
                        self.save_settings();
                    }
                    Err(error) if error == text::SIGN_IN_CANCELLED => {}
                    Err(error) => self.error = Some(error),
                }
            }
            TaskOutput::DeviceCode(result) => match result {
                Ok(code) => self.wait_for_browser_sign_in(code),
                Err(error) => {
                    self.signing_in = false;
                    self.error = Some(error);
                }
            },
            TaskOutput::SignedOut(result) => {
                if let Err(error) = result {
                    self.error = Some(format!("Signed out, but the token couldn't be removed: {error}"));
                }
            }
            TaskOutput::Operation { repo, kind, result } => {
                // Cleared first, so a result for a repository that was closed meanwhile can't leave
                // the app stuck as busy.
                self.finish_operation();
                let is_current = self.repo_path().as_ref() == Some(&repo);
                match result {
                    Ok(()) if is_current => {
                        if kind == OperationKind::Commit
                            && let Some(store) = self.repo.as_mut()
                        {
                            store.clear_commit_message();
                            store.line_selections.clear();
                        }
                        self.load_snapshot();
                        self.load_history();
                    }
                    Ok(()) => {}
                    Err(error) if error.cancelled => self.load_snapshot(),
                    Err(error) => {
                        self.error = Some(error.message);
                        self.load_snapshot();
                    }
                }
            }
            pull @ (TaskOutput::Pulls { .. }
            | TaskOutput::PullDetail { .. }
            | TaskOutput::CommentPosted { .. }
            | TaskOutput::PullCreated { .. }
            | TaskOutput::PullFetched { .. }) => self.apply_pull_output(pull),
            TaskOutput::Organizations(result) => {
                if let Some(Dialog::Publish(form)) = self.dialog.as_mut() {
                    form.organizations = match result {
                        Ok(organizations) => Loadable::Ready(organizations),
                        Err(error) => Loadable::Failed(error),
                    };
                }
            }
            TaskOutput::MergePreview { repo, branch, result } => {
                let is_current = self.repo_path().as_ref() == Some(&repo);
                if let Some(Dialog::Merge(form)) = self.dialog.as_mut()
                    && is_current
                    && form.selected.as_deref() == Some(branch.as_str())
                {
                    form.preview = Loadable::from_result(result);
                }
            }
            other => self.apply_repository_data(other),
        }
    }

    fn set_account(&mut self, account: Option<GitHubAccount>) {
        if let Ok(mut shared) = self.account.lock() {
            *shared = account;
        }
    }

    /// Loaded data for the open repository. Results for a repository, file or commit that is no
    /// longer selected are dropped.
    fn apply_repository_data(&mut self, output: TaskOutput) {
        let current = self.repo_path();
        let is_current = |path: &PathBuf| current.as_ref() == Some(path);
        let Some(repo) = self.repo.as_mut() else { return };
        match output {
            TaskOutput::Snapshot { repo: path, result } if is_current(&path) => match result {
                Ok(snapshot) => {
                    repo.apply_snapshot(snapshot);
                    self.load_working_diff();
                }
                Err(error) => repo.snapshot = Loadable::Failed(error.message),
            },
            TaskOutput::WorkingDiff { repo: path, file, result } if is_current(&path) => {
                if repo.selected_file.as_deref() == Some(file.as_str()) {
                    if let Ok(diff) = &result {
                        repo.forget_stale_lines(&file, diff);
                    }
                    repo.working_diff = Loadable::from_result(result);
                }
            }
            TaskOutput::Conflict { repo: path, file, result } if is_current(&path) => {
                if repo.selected_file.as_deref() == Some(file.as_str()) {
                    repo.conflict = Loadable::from_result(result);
                }
            }
            TaskOutput::History { repo: path, result } if is_current(&path) => {
                repo.history = Loadable::from_result(result);
            }
            TaskOutput::CommitFiles { repo: path, sha, result } if is_current(&path) => {
                if repo.selected_commit.as_deref() == Some(sha.as_str()) {
                    repo.commit_files = Loadable::from_result(result);
                }
            }
            TaskOutput::CommitDiff { repo: path, sha, file, result } if is_current(&path) => {
                let still_selected = repo.selected_commit.as_deref() == Some(sha.as_str())
                    && repo.selected_commit_file.as_deref() == Some(file.as_str());
                if still_selected {
                    repo.commit_diff = Loadable::from_result(result);
                }
            }
            _ => {}
        }
    }
}
