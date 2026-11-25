use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::loadable::Loadable;
use crate::models::{
    CoAuthor, CommitSummary, ConflictFile, Diff, FileChange, LineSelection, LineToggle, PullDetail, PullList,
    PullRequest, RepoSnapshot,
};

/// Everything shown for the open repository.
#[derive(Debug, Default)]
pub struct RepoStore {
    pub path: PathBuf,
    pub snapshot: Loadable<RepoSnapshot>,
    /// Files unticked for the next commit. Tracking exclusions (not inclusions) means new files are
    /// included by default, as in GitHub Desktop.
    pub excluded: HashSet<String>,
    /// Files of which only some lines go into the next commit.
    pub line_selections: HashMap<String, LineSelection>,
    pub selected_file: Option<String>,
    pub working_diff: Loadable<Diff>,
    /// The selected file's conflict blocks, when it is conflicted.
    pub conflict: Loadable<ConflictFile>,

    pub history: Loadable<Vec<CommitSummary>>,
    pub selected_commit: Option<String>,
    pub commit_files: Loadable<Vec<FileChange>>,
    pub selected_commit_file: Option<String>,
    pub commit_diff: Loadable<Diff>,

    pub commit_summary: String,
    pub commit_description: String,
    /// People credited on the next commit (GitHub Desktop's "Add co-authors").
    pub co_authors: Vec<CoAuthor>,
    pub show_co_authors: bool,
    /// What is typed in the co-author field.
    pub co_author_query: String,

    pub pulls: Loadable<PullList>,
    pub selected_pull: Option<u64>,
    /// The selected pull request's conversation and checks.
    pub pull_detail: Loadable<PullDetail>,
    /// The comment being written on the selected pull request.
    pub pull_comment: String,
    /// A comment or new pull request is being sent.
    pub pull_sending: bool,
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
        self.line_selections.retain(|path, _| present.contains(path.as_str()));
        if self.selected_file.as_deref().is_some_and(|path| !present.contains(path)) {
            self.selected_file = None;
            self.working_diff = Loadable::Idle;
        }
        // Like GitHub Desktop, a merge stopped on conflicts gets git's prepared summary.
        if let Some(message) = &snapshot.merge_message
            && self.commit_summary.trim().is_empty()
        {
            self.commit_summary.clone_from(message);
        }
        self.snapshot = Loadable::Ready(snapshot);
    }

    pub fn is_included(&self, path: &str) -> bool {
        !self.excluded.contains(path)
    }

    /// True when only some lines of the file are chosen.
    pub fn is_partial(&self, path: &str) -> bool {
        self.line_selections.get(path).is_some_and(|selection| !selection.excluded.is_empty())
    }

    /// Applies a click in the gutter of `path`'s diff. Leaving out every line unticks the file, and
    /// picking a line of an unticked file ticks it with only that line.
    pub fn toggle_lines(&mut self, path: &str, diff: &Diff, toggle: LineToggle) {
        let file_included = self.is_included(path);
        let selection = self.line_selections.entry(path.to_owned()).or_insert_with(|| {
            let mut selection = LineSelection::new(diff.clone());
            if !file_included {
                selection.excluded = diff.all_changes();
            }
            selection
        });
        selection.apply(toggle);
        let (nothing, everything) = (selection.includes_nothing(), selection.excluded.is_empty());
        if nothing || everything {
            self.line_selections.remove(path);
        }
        self.set_file_included(path, !nothing);
    }

    /// Drops a line choice made on a diff that no longer matches the file.
    pub fn forget_stale_lines(&mut self, path: &str, diff: &Diff) {
        if self.line_selections.get(path).is_some_and(|selection| selection.diff != *diff) {
            self.line_selections.remove(path);
        }
    }

    pub fn set_included(&mut self, path: &str, included: bool) {
        self.line_selections.remove(path);
        self.set_file_included(path, included);
    }

    fn set_file_included(&mut self, path: &str, included: bool) {
        if included {
            self.excluded.remove(path);
        } else {
            self.excluded.insert(path.to_owned());
        }
    }

    pub fn set_all_included(&mut self, included: bool) {
        self.excluded.clear();
        self.line_selections.clear();
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

    /// Summary, blank line, description, then the co-author lines, the layout git and GitHub expect.
    pub fn commit_message(&self) -> String {
        let summary = self.commit_summary.trim();
        let description = self.commit_description.trim();
        let mut message =
            if description.is_empty() { summary.to_owned() } else { format!("{summary}\n\n{description}") };
        let trailers: Vec<String> =
            self.co_authors.iter().filter(|_| self.show_co_authors).map(CoAuthor::trailer).collect();
        if !trailers.is_empty() {
            message.push_str("\n\n");
            message.push_str(&trailers.join("\n"));
        }
        message
    }

    /// People who have committed here, newest first, for co-author suggestions.
    pub fn known_authors(&self) -> Vec<CoAuthor> {
        let mut seen = HashSet::new();
        self.history
            .ready()
            .into_iter()
            .flatten()
            .filter(|commit| seen.insert(commit.author_email.to_lowercase()))
            .map(|commit| CoAuthor { name: commit.author.clone(), email: commit.author_email.clone() })
            .collect()
    }

    pub fn selected_pull(&self) -> Option<&PullRequest> {
        let number = self.selected_pull?;
        self.pulls.ready()?.pulls.iter().find(|pull| pull.number == number)
    }

    pub fn clear_commit_message(&mut self) {
        self.commit_summary.clear();
        self.commit_description.clear();
        self.co_authors.clear();
        self.co_author_query.clear();
        self.show_co_authors = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn co_authors_go_after_the_description() {
        let mut store = RepoStore {
            commit_summary: "Fix sensors".into(),
            co_authors: vec![CoAuthor { name: "Sam".into(), email: "sam@example.com".into() }],
            show_co_authors: true,
            ..RepoStore::default()
        };
        assert_eq!(store.commit_message(), "Fix sensors\n\nCo-authored-by: Sam <sam@example.com>");
        store.commit_description = "Details".into();
        assert_eq!(store.commit_message(), "Fix sensors\n\nDetails\n\nCo-authored-by: Sam <sam@example.com>");
        // Hiding the field leaves them out, as removing them in GitHub Desktop does.
        store.show_co_authors = false;
        assert_eq!(store.commit_message(), "Fix sensors\n\nDetails");
    }
}
