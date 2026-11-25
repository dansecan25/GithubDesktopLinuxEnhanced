//! Pull requests: listing, the conversation, comments, creating one, and checking out its branch.

use super::DesktopApp;
use crate::constants::git::DEFAULT_REMOTE;
use crate::constants::text;
use crate::git::remote;
use crate::github::{self, pulls};
use crate::models::PullRequest;
use crate::services::tasks::{OperationKind, TaskOutput};
use crate::stores::Loadable;
use crate::ui::actions::Tab;
use crate::ui::dialogs::{Dialog, PullForm};

impl DesktopApp {
    /// `owner/name` of the open repository on github.com, if its remote is there.
    pub(super) fn github_slug(&self) -> Option<String> {
        let url = self.repo.as_ref()?.snapshot.ready()?.remote_url.clone()?;
        pulls::slug(&github::web_url(&url)?)
    }

    pub(super) fn load_pulls(&mut self) {
        let (Some(token), Some(slug)) = (self.token(), self.github_slug()) else { return };
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.pulls.ready().is_none() {
            repo.pulls = Loadable::Loading;
        }
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::Pulls { result: pulls::list(&token, &slug), repo: path });
    }

    pub(super) fn select_pull(&mut self, number: u64) {
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.selected_pull != Some(number) {
            repo.pull_comment.clear();
        }
        repo.selected_pull = Some(number);
        repo.pull_detail = Loadable::Loading;
        self.detail_open = true;
        self.load_pull_detail();
    }

    fn load_pull_detail(&mut self) {
        let (Some(token), Some(slug)) = (self.token(), self.github_slug()) else { return };
        let Some(repo) = self.repo.as_ref() else { return };
        let Some(pull) = repo.selected_pull().cloned() else { return };
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::PullDetail {
            result: pulls::detail(&token, &slug, pull.number, &pull.head_sha),
            number: pull.number,
            repo: path,
        });
    }

    pub(super) fn post_comment(&mut self, number: u64, body: String) {
        let (Some(token), Some(slug)) = (self.token(), self.github_slug()) else { return };
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.pull_sending || body.trim().is_empty() {
            return;
        }
        repo.pull_sending = true;
        let path = repo.path.clone();
        self.tasks.spawn(move || TaskOutput::CommentPosted {
            result: pulls::comment(&token, &slug, number, body.trim()),
            repo: path,
            number,
        });
    }

    /// Opens the create dialog for the current branch, into the default branch.
    pub(super) fn open_create_pull(&mut self) {
        let Some(repo) = self.repo.as_ref() else { return };
        let Some(snapshot) = repo.snapshot.ready() else { return };
        let Some(head) = snapshot.status.head.clone() else { return };
        if snapshot.status.upstream.is_none() {
            self.error = Some(text::PUBLISH_BRANCH_FIRST.to_owned());
            return;
        }
        let remote = snapshot.remote.clone().unwrap_or_else(|| DEFAULT_REMOTE.to_owned());
        let prefix = format!("{remote}/");
        let bases: Vec<String> = snapshot
            .branches
            .iter()
            .filter(|branch| branch.is_remote)
            .filter_map(|branch| branch.name.strip_prefix(&prefix).map(str::to_owned))
            .filter(|name| *name != head)
            .collect();
        let default = repo.pulls.ready().map(|list| list.default_branch.clone());
        let base = default
            .filter(|branch| bases.contains(branch))
            .or_else(|| bases.iter().find(|name| ["main", "master"].contains(&name.as_str())).cloned())
            .or_else(|| bases.first().cloned())
            .unwrap_or_default();
        // Like GitHub, the title starts as the last commit's summary.
        let title = repo.history.ready().and_then(|commits| commits.first()).map(|c| c.summary.clone());
        let form = PullForm {
            head,
            base,
            bases,
            title: title.unwrap_or_default(),
            body: String::new(),
            draft: false,
        };
        self.dialog = Some(Dialog::CreatePull(form));
        self.dialog_opening += 1;
    }

    pub(super) fn create_pull(&mut self, form: PullForm) {
        let (Some(token), Some(slug)) = (self.token(), self.github_slug()) else { return };
        let Some(repo) = self.repo.as_mut() else { return };
        if repo.pull_sending {
            return;
        }
        repo.pull_sending = true;
        let path = repo.path.clone();
        let pull = pulls::NewPull {
            title: form.title.trim().to_owned(),
            body: form.body.trim().to_owned(),
            head: form.head,
            base: form.base,
            draft: form.draft,
        };
        self.tasks.spawn(move || TaskOutput::PullCreated {
            result: pulls::create(&token, &slug, &pull),
            repo: path,
        });
    }

    /// Fetches the pull request's branch; switching to it happens when the fetch is done.
    pub(super) fn checkout_pull(&mut self, number: u64) {
        let Some(repo) = self.repo.as_ref() else { return };
        let Some(pull) = repo.selected_pull().filter(|pull| pull.number == number).cloned() else { return };
        let Some(snapshot) = repo.snapshot.ready() else { return };
        let remote = snapshot.remote.clone().unwrap_or_else(|| DEFAULT_REMOTE.to_owned());
        let local: Vec<String> = snapshot
            .branches
            .iter()
            .filter(|branch| !branch.is_remote)
            .map(|branch| branch.name.clone())
            .collect();
        self.start_pull_fetch(remote, pull, local);
    }

    fn start_pull_fetch(&mut self, remote: String, pull: PullRequest, local: Vec<String>) {
        if self.busy.is_some() {
            return;
        }
        let Some(path) = self.repo_path() else { return };
        self.busy = Some(OperationKind::Fetch.label());
        let control = self.start_control(true);
        self.tasks.spawn(move || TaskOutput::PullFetched {
            result: crate::git::control::with_control(control, || {
                remote::fetch_pull(&path, &remote, &pull, &local)
            }),
            repo: path,
        });
    }

    pub(super) fn apply_pull_output(&mut self, output: TaskOutput) {
        let current = self.repo_path();
        let is_current = |path: &std::path::PathBuf| current.as_ref() == Some(path);
        match output {
            TaskOutput::Pulls { repo: path, result } if is_current(&path) => {
                let Some(repo) = self.repo.as_mut() else { return };
                repo.pulls = match result {
                    Ok(list) => Loadable::Ready(list),
                    Err(error) => Loadable::Failed(error),
                };
                // A pull request selected before the list arrived (one just created) gets its detail.
                let waiting = repo.selected_pull().is_some() && matches!(repo.pull_detail, Loadable::Idle);
                if waiting {
                    repo.pull_detail = Loadable::Loading;
                    self.load_pull_detail();
                }
            }
            TaskOutput::PullDetail { repo: path, number, result } if is_current(&path) => {
                let Some(repo) = self.repo.as_mut() else { return };
                if repo.selected_pull == Some(number) {
                    repo.pull_detail = match result {
                        Ok(detail) => Loadable::Ready(detail),
                        Err(error) => Loadable::Failed(error),
                    };
                }
            }
            TaskOutput::CommentPosted { repo: path, number, result } if is_current(&path) => {
                let Some(repo) = self.repo.as_mut() else { return };
                repo.pull_sending = false;
                match result {
                    Ok(()) => {
                        repo.pull_comment.clear();
                        if repo.selected_pull == Some(number) {
                            self.load_pull_detail();
                        }
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            TaskOutput::PullCreated { repo: path, result } if is_current(&path) => {
                let Some(repo) = self.repo.as_mut() else { return };
                repo.pull_sending = false;
                match result {
                    Ok(number) => {
                        self.dialog = None;
                        self.tab = Tab::PullRequests;
                        repo.selected_pull = Some(number);
                        repo.pull_detail = Loadable::Idle;
                        self.detail_open = true;
                        self.load_pulls();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
            TaskOutput::PullFetched { repo: path, result } if is_current(&path) => {
                self.finish_operation();
                match result {
                    Ok(branch) => self.switch_branch(branch),
                    Err(error) if error.cancelled => {}
                    Err(error) => self.error = Some(error.message),
                }
                self.load_snapshot();
            }
            // Results for a repository that was closed meanwhile.
            TaskOutput::CommentPosted { .. } | TaskOutput::PullCreated { .. } => {
                if let Some(repo) = self.repo.as_mut() {
                    repo.pull_sending = false;
                }
            }
            TaskOutput::PullFetched { .. } => self.finish_operation(),
            _ => {}
        }
    }
}
