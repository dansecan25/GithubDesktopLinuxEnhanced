//! Runs the git layer against real throwaway repositories.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use github_desktop_enhanced::git::{branches, changes, history, repository, stash};
use github_desktop_enhanced::models::{ChangeKind, FileChange};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempRepo(PathBuf);

impl TempRepo {
    fn new() -> Self {
        let name = format!("gde-test-{}-{}", std::process::id(), COUNTER.fetch_add(1, Ordering::SeqCst));
        let path = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&path);
        let root = repository::init(&path).expect("init");
        git(&root, &["config", "user.name", "Test"]);
        git(&root, &["config", "user.email", "test@example.invalid"]);
        git(&root, &["config", "commit.gpgsign", "false"]);
        Self(root)
    }

    fn write(&self, file: &str, content: &str) {
        std::fs::write(self.0.join(file), content).unwrap();
    }

    fn changes(&self) -> Vec<FileChange> {
        repository::load_snapshot(&self.0).unwrap().changes
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git").current_dir(dir).args(args).output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn find<'a>(list: &'a [FileChange], path: &str) -> &'a FileChange {
    list.iter().find(|c| c.path == path).unwrap_or_else(|| panic!("{path} not in {list:?}"))
}

#[test]
fn first_commit_and_history() {
    let repo = TempRepo::new();
    repo.write("a.txt", "hello\n");
    let snapshot = repository::load_snapshot(&repo.0).unwrap();
    assert!(!snapshot.status.has_commits);
    assert_eq!(snapshot.status.head.as_deref(), Some("main"));

    let diff = changes::working_diff(&repo.0, &snapshot.changes[0], false).unwrap();
    assert_eq!(diff.lines.len(), 1);

    changes::commit(&repo.0, "Initial\n\nBody", &snapshot.changes, true).unwrap();
    let log = history::log(&repo.0).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].summary, "Initial");
    assert_eq!(log[0].body, "Body");
    let files = history::commit_files(&repo.0, &log[0].sha).unwrap();
    assert_eq!(files[0].path, "a.txt");
    assert!(!history::commit_file_diff(&repo.0, &log[0].sha, "a.txt").unwrap().lines.is_empty());
}

#[test]
fn partial_commit_only_includes_chosen_files() {
    let repo = TempRepo::new();
    repo.write("keep.txt", "1\n");
    repo.write("gone.txt", "1\n");
    repo.write("staged-gone.txt", "1\n");
    changes::commit(&repo.0, "Initial", &repo.changes(), true).unwrap();

    repo.write("keep.txt", "2\n");
    repo.write("new.txt", "new\n");
    std::fs::remove_file(repo.0.join("gone.txt")).unwrap();
    git(&repo.0, &["rm", "-q", "staged-gone.txt"]);

    let all = repo.changes();
    assert_eq!(find(&all, "staged-gone.txt").kind, ChangeKind::Deleted);
    let chosen: Vec<FileChange> = all.iter().filter(|c| c.path != "keep.txt").cloned().collect();
    changes::commit(&repo.0, "Partial", &chosen, false).unwrap();

    let left = repo.changes();
    assert_eq!(left.len(), 1, "{left:?}");
    assert_eq!(left[0].path, "keep.txt");
}

#[test]
fn staged_rename_commits_and_discards() {
    let repo = TempRepo::new();
    repo.write("old.txt", "content that stays the same\n");
    repo.write("other.txt", "x\n");
    changes::commit(&repo.0, "Initial", &repo.changes(), true).unwrap();
    git(&repo.0, &["mv", "old.txt", "new.txt"]);
    repo.write("other.txt", "y\n");

    let renamed = find(&repo.changes(), "new.txt").clone();
    assert_eq!(renamed.kind, ChangeKind::Renamed);
    assert_eq!(renamed.original_path.as_deref(), Some("old.txt"));

    changes::discard(&repo.0, std::slice::from_ref(&renamed)).unwrap();
    assert!(repo.0.join("old.txt").exists());
    assert!(!repo.0.join("new.txt").exists());

    git(&repo.0, &["mv", "old.txt", "new.txt"]);
    let renamed = find(&repo.changes(), "new.txt").clone();
    changes::commit(&repo.0, "Rename", &[renamed], false).unwrap();
    let left = repo.changes();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].path, "other.txt");
}

#[test]
fn discard_restores_and_deletes() {
    let repo = TempRepo::new();
    repo.write("tracked.txt", "original\n");
    changes::commit(&repo.0, "Initial", &repo.changes(), true).unwrap();
    repo.write("tracked.txt", "edited\n");
    repo.write("untracked.txt", "junk\n");
    repo.write("added.txt", "junk\n");
    git(&repo.0, &["add", "added.txt"]);

    changes::discard(&repo.0, &repo.changes()).unwrap();
    assert!(repo.changes().is_empty());
    assert_eq!(std::fs::read_to_string(repo.0.join("tracked.txt")).unwrap(), "original\n");
    assert!(!repo.0.join("untracked.txt").exists());
}

#[test]
fn stash_round_trip() {
    let repo = TempRepo::new();
    repo.write("a.txt", "1\n");
    changes::commit(&repo.0, "Initial", &repo.changes(), true).unwrap();
    repo.write("a.txt", "2\n");
    repo.write("b.txt", "new\n");

    stash::push_all(&repo.0, "work in progress").unwrap();
    assert!(repo.changes().is_empty());
    let stashes = stash::list(&repo.0).unwrap();
    assert_eq!(stashes.len(), 1);
    assert!(stashes[0].message.contains("work in progress"));

    stash::pop(&repo.0, &stashes[0].reference).unwrap();
    assert_eq!(repo.changes().len(), 2);
    stash::push_all(&repo.0, "again").unwrap();
    stash::drop(&repo.0, "stash@{0}").unwrap();
    assert!(stash::list(&repo.0).unwrap().is_empty());
}

#[test]
fn branches_create_switch_delete() {
    let repo = TempRepo::new();
    repo.write("a.txt", "1\n");
    changes::commit(&repo.0, "Initial", &repo.changes(), true).unwrap();

    assert!(branches::is_valid_name(&repo.0, "feature/x"));
    assert!(!branches::is_valid_name(&repo.0, "bad..name"));
    branches::create(&repo.0, "feature/x").unwrap();
    let list = branches::list(&repo.0).unwrap();
    assert!(list.iter().any(|b| b.name == "feature/x" && b.is_current));

    let main = list.iter().find(|b| b.name == "main").unwrap().clone();
    branches::switch(&repo.0, &main).unwrap();
    branches::delete(&repo.0, "feature/x").unwrap();
    assert_eq!(branches::list(&repo.0).unwrap().len(), 1);
}

#[test]
fn clone_from_local_path_and_rejects_existing_folder() {
    let source = TempRepo::new();
    source.write("a.txt", "1\n");
    changes::commit(&source.0, "Initial", &source.changes(), true).unwrap();
    let parent = TempRepo::new();

    let url = source.0.to_string_lossy().into_owned();
    let cloned = repository::clone(&url, &parent.0).unwrap();
    assert!(cloned.join("a.txt").exists());
    let snapshot = repository::load_snapshot(&cloned).unwrap();
    assert_eq!(snapshot.remote.as_deref(), Some("origin"));
    assert_eq!(snapshot.status.upstream.as_deref(), Some("origin/main"));
    assert!(repository::clone(&url, &parent.0).is_err());
}
