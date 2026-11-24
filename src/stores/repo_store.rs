use std::collections::HashSet;
use std::path::PathBuf;

use super::loadable::Loadable;
use crate::models::{CommitSummary, Diff, FileChange, RepoSnapshot};

/// Everything shown for the open repository.
#[derive(Debug, Default)]
pub struct RepoStore {
    pub path: PathBuf,
    pub snapshot: Loadable<RepoSnapshot>,
    /// Files unticked for the next commit. Tracking exclusions (not inclusions) means new files are
    /// included by default, as in GitHub Desktop.
    pub excluded: HashSet<String>,
    pub selected_file: Option<String>,
    pub working_diff: Loadable<Diff>,

    pub history: Loadable<Vec<CommitSummary>>,
    pub selected_commit: Option<String>,
    pub commit_files: Loadable<Vec<FileChange>>,
    pub selected_commit_file: Option<String>,
    pub commit_diff: Loadable<Diff>,

    pub commit_summary: String,
    pub commit_description: String,
}

impl RepoStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path, ..Self::default() }
    }

    pub fn changes(&self) -> &[FileChange] {
        self.snapshot.ready().map(|snapshot| snapshot.changes.as_slice()).unwrap_or_default()
    }

    pub fn apply_snapshot(&mut self, snapshot: RepoSnapshot) {
        let present: HashSet<&str> = snapshot.changes.iter().map(|c| c.path.as_str()).collect();
        self.excluded.retain(|path| present.contains(path.as_str()));
        if self.selected_file.as_deref().is_some_and(|path| !present.contains(path)) {
            self.selected_file = None;
            self.working_diff = Loadable::Idle;
        }
        self.snapshot = Loadable::Ready(snapshot);
    }

    pub fn is_included(&self, path: &str) -> bool {
        !self.excluded.contains(path)
    }

    pub fn set_included(&mut self, path: &str, included: bool) {
        if included {
            self.excluded.remove(path);
        } else {
            self.excluded.insert(path.to_owned());
        }
    }

    pub fn set_all_included(&mut self, included: bool) {
        self.excluded.clear();
        if !included {
            let all: Vec<String> = self.changes().iter().map(|c| c.path.clone()).collect();
            self.excluded.extend(all);
        }
    }

    pub fn included_changes(&self) -> Vec<FileChange> {
        self.changes().iter().filter(|c| self.is_included(&c.path)).cloned().collect()
    }

    pub fn selected_change(&self) -> Option<&FileChange> {
        let selected = self.selected_file.as_deref()?;
        self.changes().iter().find(|change| change.path == selected)
    }

    pub fn selected_commit_summary(&self) -> Option<&CommitSummary> {
        let sha = self.selected_commit.as_deref()?;
        self.history.ready()?.iter().find(|commit| commit.sha == sha)
    }

    /// Summary, blank line, description, the layout git tools expect.
    pub fn commit_message(&self) -> String {
        let summary = self.commit_summary.trim();
        let description = self.commit_description.trim();
        if description.is_empty() { summary.to_owned() } else { format!("{summary}\n\n{description}") }
    }

    pub fn clear_commit_message(&mut self) {
        self.commit_summary.clear();
        self.commit_description.clear();
    }
}
