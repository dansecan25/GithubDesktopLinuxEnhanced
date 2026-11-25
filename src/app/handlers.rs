//! What each action does. Anything that touches git, the network or the keychain runs in a
//! background task.

use std::path::{Path, PathBuf};

use eframe::egui;

use super::DesktopApp;
use crate::constants::git::DEFAULT_REMOTE;
use crate::constants::text;
use crate::git::{GitError, GitResult, branches, changes, history, merge, remote, repository, stash, tags};
use crate::github;
use crate::services::askpass::GitHubAccount;
use crate::services::secrets;
use crate::services::system;
use crate::services::tasks::{OperationKind, TaskOutput};
use crate::services::{editor, update};
use crate::stores::{Loadable, RepoStore};
use crate::ui::actions::{Action, Tab};
use crate::ui::dialogs::Dialog;
use crate::ui::{theme, views};

impl DesktopApp {
    pub(super) fn open_repository(&mut self, path: PathBuf) {
        self.repo = Some(RepoStore::new(path));
        self.tab = Tab::Changes;
        self.detail_open = false;
        self.load_snapshot();
        self.load_history();
    }

    pub(super) fn load_snapshot(&mut self) {
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.snapshot.ready().is_none() {
            repo.snapshot = Loadable::Loading;
        }
        let path = repo.path.clone();
        self.tasks
            .spawn(move || TaskOutput::Snapshot { result: repository::load_snapshot(&path), repo: path });
    }

    pub(super) fn load_history(&mut self) {
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.history.ready().is_none() {
            repo.history = Loadable::Loading;
        }
        let path = repo.path.clone();
        let has_commits = repo.snapshot.ready().is_none_or(|snapshot| snapshot.status.has_commits);
        self.tasks.spawn(move || {
            // A repository without commits has no log; git reports that as an error.
            let result = if has_commits { history::log(&path) } else { Ok(Vec::new()) };
            TaskOutput::History { result: result.or_else(|_| Ok(Vec::new())), repo: path }
        });
    }

    pub(super) fn load_working_diff(&mut self) {
        let Some(repo) = self.repo.as_mut() else { return };
        let Some(change) = repo.selected_change().cloned() else { return };
        let has_commits = repo.snapshot.ready().is_some_and(|s| s.status.has_commits);
        if repo.working_diff.ready().is_none() {
            repo.working_diff = Loadable::Loading;
        }
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::WorkingDiff {
            result: changes::working_diff(&path, &change, has_commits),
            file: change.path,
            repo: path,
        });
    }

    /// Runs a command that changes the repository, then refreshes everything.
    fn run_operation(
        &mut self,
        kind: OperationKind,
        job: impl FnOnce(&Path) -> GitResult<()> + Send + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        let Some(path) = self.repo_path() else { return };
        self.busy = Some(kind.label());
        self.tasks.spawn(move || TaskOutput::Operation { result: job(&path), repo: path, kind });
    }

    fn open_repository_in_background(
        &mut self,
        label: &'static str,
        remember_folder: Option<PathBuf>,
        job: impl FnOnce() -> GitResult<PathBuf> + Send + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(label);
        self.tasks.spawn(move || TaskOutput::RepositoryOpened { result: job(), remember_folder });
    }

    /// The remote and branch state needed by push and tag operations.
    fn remote_name(&self) -> Option<String> {
        self.repo.as_ref()?.snapshot.ready()?.remote.clone()
    }

    /// The checked-out branch, `None` when HEAD is detached.
    fn current_branch(&self) -> Option<String> {
        self.repo.as_ref()?.snapshot.ready()?.status.head.clone()
    }

    fn token(&self) -> Option<String> {
        self.account.lock().ok()?.as_ref().map(|account| account.token.clone())
    }

    pub(super) fn handle(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::OpenDialog(dialog) => {
                self.dialog = Some(dialog);
                self.dialog_opening += 1;
            }
            Action::CloseDialog => self.dialog = None,
            Action::AnswerPrompt(answer) => {
                if let Some((request, _)) = self.prompt.take() {
                    request.answer(answer);
                }
            }

            Action::SelectRepository(path) => {
                self.settings.selected_repository = Some(path.clone());
                self.save_settings();
                self.open_repository(path);
            }
            Action::RemoveRepository(path) => {
                self.settings.remove_repository(&path);
                self.save_settings();
                match self.settings.selected_repository.clone() {
                    Some(next) => self.open_repository(next),
                    None => self.repo = None,
                }
            }
            Action::AddRepository(path) => {
                self.open_repository_in_background(text::OPENING_REPOSITORY, None, move || {
                    repository::find_root(&path)
                });
            }
            Action::CloneRepository { url, parent } => {
                let remember = Some(parent.clone());
                self.open_repository_in_background(text::CLONING, remember, move || {
                    repository::clone(&url, &parent)
                });
            }
            Action::CreateRepository(path) => {
                let remember = path.parent().map(Path::to_path_buf);
                self.open_repository_in_background(text::CREATING_REPOSITORY, remember, move || {
                    if path.exists() && path.read_dir().is_ok_and(|mut entries| entries.next().is_some()) {
                        return Err(GitError::new(format!(
                            "{} already exists and isn't empty",
                            path.display()
                        )));
                    }
                    repository::init(&path)
                });
            }

            Action::ShowInFileManager(path) | Action::OpenWithSystem(path) => {
                if let Err(error) = system::open_with_system(&path) {
                    self.error = Some(error.to_string());
                }
            }
            Action::OpenInEditor(path) => self.open_in_editor(&path),
            Action::OpenUrl(url) => {
                if let Err(error) = system::open_with_system(&url) {
                    self.error = Some(error.to_string());
                }
            }
            Action::CopyText(value) => ctx.copy_text(value),

            Action::SetTheme(choice) => {
                self.settings.theme = choice;
                theme::apply(ctx, choice);
                self.save_settings();
            }
            Action::SetEditor(command) => {
                self.settings.editor = command;
                self.save_settings();
            }
            Action::SetAutoUpdate(enabled) => {
                self.settings.auto_update = enabled;
                self.save_settings();
            }
            Action::SignIn(token) => self.sign_in(token),
            Action::SignOut => self.sign_out(),
            Action::CheckForUpdates => self.updater.check_now(ctx),
            Action::InstallUpdate => self.updater.install(ctx),
            Action::RestartAfterUpdate => match update::restart() {
                Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                Err(error) => self.error = Some(error),
            },
            Action::DismissUpdate => self.updater.dismissed = true,

            Action::SetTab(tab) => {
                self.tab = tab;
                self.detail_open = false;
            }
            Action::CloseDetail => self.detail_open = false,
            Action::Refresh => {
                self.load_snapshot();
                self.load_history();
            }

            Action::SelectFile(file) => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.selected_file = Some(file);
                    repo.working_diff = Loadable::Loading;
                }
                self.detail_open = true;
                self.load_working_diff();
            }
            Action::SelectCommit(sha) => self.select_commit(sha),
            Action::SelectCommitFile(file) => self.select_commit_file(file),
            Action::Commit => self.commit(),
            Action::Discard(files) => {
                self.run_operation(OperationKind::Discard, move |path| changes::discard(path, &files));
            }
            Action::IgnoreFile(file) => self.run_operation(OperationKind::Ignore, move |path| {
                changes::ignore(path, &changes::ignore_pattern(&file))
            }),

            Action::Fetch => self.run_operation(OperationKind::Fetch, remote::fetch),
            Action::Pull => self.run_operation(OperationKind::Pull, remote::pull),
            Action::Push => {
                let Some(snapshot) = self.repo.as_ref().and_then(|repo| repo.snapshot.ready()) else {
                    return;
                };
                let status = snapshot.status.clone();
                let remote_name = snapshot.remote.clone();
                self.run_operation(OperationKind::Push, move |path| {
                    remote::push(path, &status, remote_name.as_deref())
                });
            }
            Action::PublishToGitHub { name, description, private } => {
                self.publish_to_github(name, description, private);
            }
            Action::AddRemote(url) => {
                self.dialog = None;
                let status = self.repo.as_ref().and_then(|r| r.snapshot.ready()).map(|s| s.status.clone());
                self.run_operation(OperationKind::Publish, move |path| {
                    remote::add(path, DEFAULT_REMOTE, &url)?;
                    push_if_possible(path, status.as_ref())
                });
            }
            Action::SwitchBranch(branch) => {
                let snapshot = self.repo.as_ref().and_then(|repo| repo.snapshot.ready());
                if snapshot.is_some_and(|snapshot| !snapshot.changes.is_empty()) {
                    let from = self.current_branch().unwrap_or_else(|| text::DETACHED_HEAD.to_owned());
                    self.dialog = Some(Dialog::SwitchBranch { branch, from, leave_changes: true });
                    self.dialog_opening += 1;
                } else {
                    self.run_operation(OperationKind::SwitchBranch, move |path| {
                        branches::switch(path, &branch)
                    });
                }
            }
            Action::SwitchBranchWithChanges { branch, leave_changes } => {
                let from = self.current_branch();
                self.run_operation(OperationKind::SwitchBranch, move |path| {
                    if leave_changes {
                        branches::switch_leaving_changes(path, &branch, from.as_deref())
                    } else {
                        branches::switch_bringing_changes(path, &branch, from.as_deref())
                    }
                });
            }
            Action::CreateBranch(name) => {
                self.dialog = None;
                self.run_operation(OperationKind::CreateBranch, move |path| {
                    if !branches::is_valid_name(path, &name) {
                        return Err(GitError::new(text::INVALID_BRANCH_NAME));
                    }
                    branches::create(path, &name)
                });
            }
            Action::DeleteBranch(name) => {
                self.run_operation(OperationKind::DeleteBranch, move |path| branches::delete(path, &name));
            }
            Action::PreviewMerge(branch) => {
                let Some(path) = self.repo_path() else { return };
                self.tasks.spawn(move || TaskOutput::MergePreview {
                    result: merge::preview(&path, &branch),
                    repo: path,
                    branch,
                });
            }
            Action::Merge(branch) => {
                self.run_operation(OperationKind::Merge, move |path| merge::merge(path, &branch));
            }
            Action::AbortMerge => self.run_operation(OperationKind::AbortMerge, merge::abort),

            Action::CreateTag { name, target, message, push } => {
                let remote_name = self.remote_name().filter(|_| push);
                self.run_operation(OperationKind::CreateTag, move |path| {
                    tags::create(path, &name, &target, &message)?;
                    match remote_name {
                        Some(remote_name) => tags::push(path, &remote_name, &name),
                        None => Ok(()),
                    }
                });
            }
            Action::PushTag(name) => {
                let Some(remote_name) = self.remote_name() else { return };
                self.run_operation(OperationKind::PushTag, move |path| tags::push(path, &remote_name, &name));
            }
            Action::DeleteTag { name, on_remote } => {
                let remote_name = self.remote_name().filter(|_| on_remote);
                self.run_operation(OperationKind::DeleteTag, move |path| {
                    tags::delete(path, &name, remote_name.as_deref())
                });
            }

            Action::StashAll => {
                // Tied to the branch, as in GitHub Desktop, so it shows up again on that branch.
                let message = self
                    .current_branch()
                    .map_or_else(|| text::STASH_MESSAGE.to_owned(), |b| stash::branch_message(&b));
                self.run_operation(OperationKind::Stash, move |path| stash::push_all(path, &message));
            }
            Action::RestoreStash(reference) => {
                self.run_operation(OperationKind::RestoreStash, move |path| stash::pop(path, &reference));
            }
            Action::DropStash(reference) => {
                self.run_operation(OperationKind::DropStash, move |path| stash::drop(path, &reference));
            }
        }
    }

    fn open_in_editor(&mut self, path: &Path) {
        match self.editor_command() {
            Some(command) => {
                if let Err(error) = editor::open(&command, path) {
                    self.error = Some(error);
                }
            }
            None => {
                self.error = Some(text::CHOOSE_EDITOR.to_owned());
                self.handle_open_preferences();
            }
        }
    }

    fn handle_open_preferences(&mut self) {
        self.dialog = Some(Dialog::preferences(&self.settings, &self.editors));
        self.dialog_opening += 1;
    }

    fn sign_in(&mut self, token: String) {
        if self.signing_in {
            return;
        }
        self.signing_in = true;
        self.tasks.spawn(move || {
            let result = github::login_for_token(&token).and_then(|login| {
                secrets::save_token(&token)
                    .map_err(|error| format!("Signed in, but the token couldn't be saved: {error}"))?;
                Ok(GitHubAccount { login, token })
            });
            TaskOutput::SignedIn(result)
        });
    }

    fn sign_out(&mut self) {
        self.settings.github_login = None;
        if let Ok(mut account) = self.account.lock() {
            *account = None;
        }
        self.save_settings();
        self.tasks.spawn(|| TaskOutput::SignedOut(secrets::delete_token()));
    }

    /// Creates the repository on GitHub, connects it as `origin` and pushes the current branch.
    fn publish_to_github(&mut self, name: String, description: String, private: bool) {
        let Some(token) = self.token() else {
            self.error = Some(text::PUBLISH_SIGN_IN_FIRST.to_owned());
            return;
        };
        self.dialog = None;
        let status = self.repo.as_ref().and_then(|r| r.snapshot.ready()).map(|s| s.status.clone());
        self.run_operation(OperationKind::Publish, move |path| {
            let created =
                github::create_repository(&token, &name, &description, private).map_err(GitError::new)?;
            remote::add(path, DEFAULT_REMOTE, &created.clone_url)?;
            push_if_possible(path, status.as_ref())
        });
    }

    fn select_commit(&mut self, sha: String) {
        let Some(repo) = self.repo.as_mut() else { return };
        repo.selected_commit = Some(sha.clone());
        repo.commit_files = Loadable::Loading;
        repo.selected_commit_file = None;
        repo.commit_diff = Loadable::Idle;
        self.detail_open = true;
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::CommitFiles {
            result: history::commit_files(&path, &sha),
            repo: path,
            sha,
        });
    }

    fn select_commit_file(&mut self, file: String) {
        let Some(repo) = self.repo.as_mut() else { return };
        let Some(sha) = repo.selected_commit.clone() else { return };
        repo.selected_commit_file = Some(file.clone());
        repo.commit_diff = Loadable::Loading;
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::CommitDiff {
            result: history::commit_file_diff(&path, &sha, &file),
            repo: path,
            sha,
            file,
        });
    }

    fn commit(&mut self) {
        let Some(repo) = self.repo.as_ref() else { return };
        if !views::can_commit(repo) {
            return;
        }
        let message = repo.commit_message();
        let merging = repo.snapshot.ready().is_some_and(|snapshot| snapshot.merge_message.is_some());
        // A merge can only be committed whole: git refuses a partial commit during one.
        let chosen = if merging { repo.changes().to_vec() } else { repo.included_changes() };
        let all_chosen = chosen.len() == repo.changes().len();
        self.run_operation(OperationKind::Commit, move |path| {
            if merging {
                let unresolved = merge::unresolved(path, &chosen);
                if !unresolved.is_empty() {
                    return Err(GitError::new(text::unresolved_conflicts(&unresolved)));
                }
            }
            changes::commit(path, &message, &chosen, all_chosen)
        });
    }
}

/// After connecting a remote: pushes the current branch, unless there is nothing to push yet (no
/// commits), in which case the branch can be published after the first commit.
fn push_if_possible(path: &Path, status: Option<&crate::models::BranchStatus>) -> GitResult<()> {
    match status {
        Some(status) if status.has_commits => remote::push(path, status, Some(DEFAULT_REMOTE)),
        _ => Ok(()),
    }
}
