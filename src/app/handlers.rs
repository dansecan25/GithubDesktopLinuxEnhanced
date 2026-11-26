//! What each action does. Anything that touches git, the network or the keychain runs in a
//! background task.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;

use super::DesktopApp;
use crate::constants::git::DEFAULT_REMOTE;
use crate::constants::text;
use crate::git::control::{self, OperationControl};
use crate::git::{
    GitError, GitResult, branches, changes, commits, conflicts, history, merge, partial, rebase, remote,
    repository, stash, tags,
};
use crate::github;
use crate::github::device::{self, DeviceCode};
use crate::models::{Branch, ChangeKind, FileChange, InProgress, LineSelection, MergeMethod};
use crate::services::askpass::GitHubAccount;
use crate::services::secrets;
use crate::services::system;
use crate::services::tasks::{OperationKind, TaskOutput};
use crate::services::{editor, image_preview, update};
use crate::stores::{Loadable, PendingRewrite, RepoStore};
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
        let path = repo.path.clone();
        if change.kind == ChangeKind::Conflicted {
            if repo.conflict.ready().is_none() {
                repo.conflict = Loadable::Loading;
            }
            self.tasks.spawn(move || TaskOutput::Conflict {
                result: conflicts::read(&path, &change.path),
                file: change.path,
                repo: path,
            });
            return;
        }
        if repo.working_diff.ready().is_none() {
            repo.working_diff = Loadable::Loading;
        }
        self.tasks.spawn(move || {
            let result = changes::working_diff(&path, &change, has_commits).map(|mut diff| {
                if image_preview::is_image(&change.path) {
                    diff.image = Some(image_preview::working(&path, &change, has_commits));
                }
                diff
            });
            TaskOutput::WorkingDiff { result, file: change.path, repo: path }
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
        let control = self.start_control(kind.can_cancel());
        self.tasks.spawn(move || TaskOutput::Operation {
            result: control::with_control(control, || job(&path)),
            repo: path,
            kind,
        });
    }

    /// A fresh control for the operation about to start, kept so the UI can show its progress and
    /// cancel it.
    pub(super) fn start_control(&mut self, cancellable: bool) -> Arc<OperationControl> {
        let control = Arc::new(OperationControl::default());
        self.control = Some(Arc::clone(&control));
        self.cancellable = cancellable;
        control
    }

    /// Clears the busy state when an operation's result arrives.
    pub(super) fn finish_operation(&mut self) {
        self.busy = None;
        self.control = None;
        self.cancellable = false;
    }

    /// Opens a repository the user added, cloned or created. Once it opens, the folder it's in
    /// becomes the starting folder for the next add, clone or create.
    fn open_repository_in_background(
        &mut self,
        label: &'static str,
        cancellable: bool,
        job: impl FnOnce() -> GitResult<PathBuf> + Send + 'static,
    ) {
        if self.busy.is_some() {
            return;
        }
        self.busy = Some(label);
        let control = self.start_control(cancellable);
        self.tasks
            .spawn(move || TaskOutput::RepositoryOpened { result: control::with_control(control, job) });
    }

    /// The remote and branch state needed by push and tag operations.
    fn remote_name(&self) -> Option<String> {
        self.repo.as_ref()?.snapshot.ready()?.remote.clone()
    }

    /// Switches right away when there are no changes, otherwise asks what to do with them, as
    /// GitHub Desktop does.
    pub(super) fn switch_branch(&mut self, branch: Branch) {
        let snapshot = self.repo.as_ref().and_then(|repo| repo.snapshot.ready());
        if snapshot.is_some_and(|snapshot| !snapshot.changes.is_empty()) {
            let from = self.current_branch().unwrap_or_else(|| text::DETACHED_HEAD.to_owned());
            self.dialog = Some(Dialog::SwitchBranch { branch, from, leave_changes: true });
            self.dialog_opening += 1;
        } else {
            self.run_operation(OperationKind::SwitchBranch, move |path| branches::switch(path, &branch));
        }
    }

    fn merge(&mut self, branch: String, method: MergeMethod) {
        let Some(into) = self.current_branch() else { return };
        match method {
            MergeMethod::Merge => {
                self.run_operation(OperationKind::Merge, move |path| merge::merge(path, &branch));
            }
            MergeMethod::Squash => {
                self.run_operation(OperationKind::Merge, move |path| merge::squash(path, &branch, &into));
            }
            MergeMethod::Rebase => {
                // A pushed branch diverges from the remote once rebased, so it needs a force push.
                let pushed =
                    self.repo.as_ref().and_then(|repo| repo.snapshot.ready()?.status.upstream.clone());
                if let (Some(repo), Some(_)) = (self.repo.as_mut(), pushed) {
                    repo.rewritten_branches.insert(into);
                }
                self.run_operation(OperationKind::Rebase, move |path| rebase::rebase(path, &branch));
            }
        }
    }

    /// The checked-out branch, `None` when HEAD is detached.
    fn current_branch(&self) -> Option<String> {
        self.repo.as_ref()?.snapshot.ready()?.status.head.clone()
    }

    pub(super) fn token(&self) -> Option<String> {
        self.account.lock().ok()?.as_ref().map(|account| account.token.clone())
    }

    pub(super) fn handle(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::CancelOperation => {
                if let Some(control) = self.control.as_ref().filter(|_| self.cancellable) {
                    control.cancel();
                }
            }
            Action::OpenDialog(dialog) => {
                let publishing = matches!(dialog, Dialog::Publish(_));
                if let Dialog::CreateTag(form) = &dialog {
                    self.load_project_versions(form.target.clone());
                }
                self.dialog = Some(dialog);
                self.dialog_opening += 1;
                if publishing {
                    self.load_organizations();
                }
            }
            Action::CloseDialog => {
                if matches!(self.dialog, Some(Dialog::BrowserSignIn { .. })) {
                    self.cancel_browser_sign_in();
                }
                self.dialog = None;
            }
            Action::StartBrowserSignIn => {
                if !self.signing_in {
                    self.signing_in = true;
                    self.tasks.spawn(|| TaskOutput::DeviceCode(device::request_code()));
                }
            }
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
                self.open_repository_in_background(text::OPENING_REPOSITORY, false, move || {
                    repository::find_root(&path)
                });
            }
            Action::CloneRepository { url, parent } => {
                self.open_repository_in_background(text::CLONING, true, move || {
                    repository::clone(&url, &parent)
                });
            }
            Action::CreateRepository(path) => {
                self.open_repository_in_background(text::CREATING_REPOSITORY, false, move || {
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
                if tab == Tab::PullRequests {
                    self.load_pulls();
                }
            }
            Action::LoadPulls => self.load_pulls(),
            Action::SelectPull(number) => self.select_pull(number),
            Action::CheckoutPull(number) => self.checkout_pull(number),
            Action::PostComment { number, body } => self.post_comment(number, body),
            Action::OpenCreatePull => self.open_create_pull(),
            Action::CreatePull(form) => self.create_pull(form),
            Action::SetPullView(view) => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.pull_view = view;
                }
            }
            Action::SelectPullFile(file) => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.selected_pull_file = Some(file);
                    repo.line_comment = None;
                }
            }
            Action::StartLineComment(target) => self.start_line_comment(target),
            Action::CancelLineComment => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.line_comment = None;
                }
            }
            Action::PostLineComment => self.post_line_comment(),
            Action::SubmitReview => self.submit_review(),
            Action::MergePull { number, method } => self.merge_pull(number, method),
            Action::OpenEditPull => self.open_edit_pull(),
            Action::SavePull(form) => self.save_pull(form),
            Action::SetDraft { number, draft } => self.set_draft(number, draft),
            Action::CloseDetail => self.detail_open = false,
            Action::Refresh => {
                self.load_snapshot();
                self.load_history();
                if self.tab == Tab::PullRequests {
                    self.load_pulls();
                }
            }

            Action::SelectFile(file) => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.selected_file = Some(file);
                    repo.working_diff = Loadable::Loading;
                    repo.conflict = Loadable::Loading;
                }
                self.detail_open = true;
                self.load_working_diff();
            }
            Action::SelectCommit(sha) => self.select_commit(sha),
            Action::SelectCommitFile(file) => self.select_commit_file(file),
            Action::Commit => self.commit(),
            Action::StartAmend => self.start_amend(),
            Action::CancelAmend => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.cancel_amend();
                }
            }
            Action::UndoCommit => self.undo_commit(),
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
            Action::ForcePush => self.run_operation(OperationKind::ForcePush, remote::force_push),
            Action::PublishToGitHub { name, description, private, organization } => {
                self.publish_to_github(name, description, private, organization);
            }
            Action::AddRemote(url) => {
                self.dialog = None;
                let status = self.repo.as_ref().and_then(|r| r.snapshot.ready()).map(|s| s.status.clone());
                self.run_operation(OperationKind::Publish, move |path| {
                    remote::add(path, DEFAULT_REMOTE, &url)?;
                    push_if_possible(path, status.as_ref())
                });
            }
            Action::SwitchBranch(branch) => self.switch_branch(branch),
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
            Action::Merge { branch, method } => self.merge(branch, method),
            Action::AbortMerge => {
                let squash = self
                    .repo
                    .as_ref()
                    .and_then(|repo| repo.snapshot.ready())
                    .is_some_and(|snapshot| snapshot.in_progress() == Some(InProgress::Squash));
                let abort = if squash { merge::abort_squash } else { merge::abort };
                self.run_operation(OperationKind::AbortMerge, abort);
            }
            Action::ContinueRebase => self.run_operation(OperationKind::Rebase, rebase::continue_rebase),
            Action::AbortRebase => self.run_operation(OperationKind::AbortRebase, rebase::abort),
            Action::ResolveConflict { file, index, choice } => {
                self.run_operation(OperationKind::Resolve, move |path| {
                    conflicts::choose(path, &file, index, choice)
                });
            }
            Action::TakeConflictSide { file, ours } => {
                self.run_operation(OperationKind::Resolve, move |path| {
                    conflicts::take_side(path, &file, ours)
                });
            }
            Action::MarkResolved(file) => {
                self.run_operation(OperationKind::Resolve, move |path| conflicts::mark_resolved(path, &file));
            }

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
        self.tasks.spawn(move || TaskOutput::SignedIn(finish_sign_in(token)));
    }

    /// Shows the code and waits in the background for the user to approve it on GitHub.
    pub(super) fn wait_for_browser_sign_in(&mut self, code: DeviceCode) {
        self.cancel_browser_sign_in();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.browser_sign_in = Some(Arc::clone(&cancelled));
        self.dialog = Some(Dialog::BrowserSignIn {
            user_code: code.user_code.clone(),
            verification_uri: code.verification_uri.clone(),
        });
        self.dialog_opening += 1;
        self.tasks.spawn(move || {
            TaskOutput::SignedIn(device::wait_for_token(&code, &cancelled).and_then(finish_sign_in))
        });
    }

    fn cancel_browser_sign_in(&mut self) {
        if let Some(cancelled) = self.browser_sign_in.take() {
            cancelled.store(true, Ordering::Relaxed);
        }
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
    /// Reads the version files on the commit the tag dialog will tag.
    fn load_project_versions(&mut self, target: String) {
        let Some(path) = self.repo_path() else { return };
        self.tasks.spawn(move || TaskOutput::ProjectVersions {
            result: tags::project_versions(&path, &target),
            repo: path,
            target,
        });
    }

    /// Lists the account's organizations for the publish dialog.
    fn load_organizations(&mut self) {
        match self.token() {
            Some(token) => self.tasks.spawn(move || TaskOutput::Organizations(github::organizations(&token))),
            // Signed out: only the "connect an existing remote" part of the dialog is shown.
            None => self.apply_task(TaskOutput::Organizations(Ok(Vec::new()))),
        }
    }

    fn publish_to_github(
        &mut self,
        name: String,
        description: String,
        private: bool,
        organization: Option<String>,
    ) {
        let Some(token) = self.token() else {
            self.error = Some(text::PUBLISH_SIGN_IN_FIRST.to_owned());
            return;
        };
        self.dialog = None;
        let status = self.repo.as_ref().and_then(|r| r.snapshot.ready()).map(|s| s.status.clone());
        self.run_operation(OperationKind::Publish, move |path| {
            let created =
                github::create_repository(&token, organization.as_deref(), &name, &description, private)
                    .map_err(GitError::new)?;
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
        self.tasks.spawn(move || {
            let result = history::commit_file_diff(&path, &sha, &file).map(|mut diff| {
                if image_preview::is_image(&file) {
                    diff.image = Some(image_preview::in_commit(&path, &sha, &file));
                }
                diff
            });
            TaskOutput::CommitDiff { result, repo: path, sha, file }
        });
    }

    /// The last commit's message: from the loaded history, or read from git if that isn't loaded.
    fn head_message(&self) -> Option<String> {
        let repo = self.repo.as_ref()?;
        repo.head_message().or_else(|| commits::last_message(&repo.path).ok())
    }

    /// Branch to remember as rewritten when the last commit is already pushed.
    fn pushed_branch(&self) -> Option<String> {
        let snapshot = self.repo.as_ref()?.snapshot.ready()?;
        snapshot.head_pushed.then(|| snapshot.status.head.clone()).flatten()
    }

    fn start_amend(&mut self) {
        let Some(message) = self.head_message() else {
            self.error = Some(text::NO_LAST_COMMIT_MESSAGE.to_owned());
            return;
        };
        if let Some(repo) = self.repo.as_mut() {
            repo.start_amend(&message);
        }
        self.tab = Tab::Changes;
    }

    fn undo_commit(&mut self) {
        if self.busy.is_some() {
            return;
        }
        let message = self.head_message();
        let pushed_branch = self.pushed_branch();
        let Some(repo) = self.repo.as_mut() else { return };
        // Like GitHub Desktop, the undone commit's message goes back into the form, unless the user
        // is already writing one.
        let message = message.filter(|_| repo.commit_summary.trim().is_empty() && repo.amending.is_none());
        repo.pending_rewrite = Some(PendingRewrite { message, pushed_branch });
        self.run_operation(OperationKind::UndoCommit, |path| commits::undo_last(path).map(|_| ()));
    }

    fn commit(&mut self) {
        let Some(repo) = self.repo.as_ref() else { return };
        if !views::can_commit(repo) {
            return;
        }
        let amend = repo.amending.is_some();
        let message = repo.commit_message();
        let in_progress = repo.snapshot.ready().and_then(|snapshot| snapshot.in_progress());
        let merging = in_progress == Some(InProgress::Merge);
        let squashing = in_progress == Some(InProgress::Squash);
        if amend && in_progress.is_some() {
            self.error = Some(text::NOT_DURING_MERGE.to_owned());
            return;
        }
        // A merge can only be committed whole: git refuses a partial commit during one.
        let chosen = if merging { repo.changes().to_vec() } else { repo.included_changes() };
        let all_chosen = chosen.len() == repo.changes().len();
        let has_commits = repo.snapshot.ready().is_some_and(|snapshot| snapshot.status.has_commits);
        let partial: Vec<(FileChange, LineSelection)> = chosen
            .iter()
            .filter_map(|change| {
                let selection = repo.line_selections.get(&change.path)?;
                (!merging && !selection.excluded.is_empty()).then(|| (change.clone(), selection.clone()))
            })
            .collect();
        if amend {
            let pushed_branch = self.pushed_branch();
            if let Some(repo) = self.repo.as_mut() {
                repo.pending_rewrite = Some(PendingRewrite { message: None, pushed_branch });
            }
        }
        let kind = if amend { OperationKind::Amend } else { OperationKind::Commit };
        self.run_operation(kind, move |path| {
            if !partial.is_empty() {
                return partial::commit_with_lines(path, &message, &chosen, &partial, has_commits, amend);
            }
            if amend {
                return changes::amend(path, &message, &chosen, all_chosen);
            }
            if merging || squashing {
                let unresolved = merge::unresolved(path, &chosen);
                if !unresolved.is_empty() {
                    return Err(GitError::new(text::unresolved_conflicts(&unresolved)));
                }
            }
            changes::commit(path, &message, &chosen, all_chosen)
        });
    }
}

/// Checks a token, saves it in the keychain and returns the account it belongs to.
fn finish_sign_in(token: String) -> Result<GitHubAccount, String> {
    let login = github::login_for_token(&token)?;
    secrets::save_token(&token)
        .map_err(|error| format!("Signed in, but the token couldn't be saved: {error}"))?;
    Ok(GitHubAccount { login, token })
}

/// After connecting a remote: pushes the current branch, unless there is nothing to push yet (no
/// commits), in which case the branch can be published after the first commit.
fn push_if_possible(path: &Path, status: Option<&crate::models::BranchStatus>) -> GitResult<()> {
    match status {
        Some(status) if status.has_commits => remote::push(path, status, Some(DEFAULT_REMOTE)),
        _ => Ok(()),
    }
}
