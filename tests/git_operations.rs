//! Runs the git layer against real throwaway repositories.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use github_desktop_enhanced::git::{branches, changes, history, merge, remote, repository, stash, tags};
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

/// A bare repository standing in for GitHub.
fn bare_remote() -> TempRepo {
    let remote = TempRepo::new();
    git(&remote.0, &["config", "core.bare", "true"]);
    let _ = std::fs::remove_dir_all(remote.0.join(".git"));
    git(&remote.0, &["init", "-q", "--bare", "-b", "main"]);
    remote
}

#[test]
fn publishes_a_branch_that_is_not_on_the_remote_yet() {
    let origin = bare_remote();
    let repo = TempRepo::new();
    repo.write("a.txt", "1\n");
    changes::commit(&repo.0, "Initial", &repo.changes(), true).unwrap();
    remote::add(&repo.0, "origin", &origin.0.to_string_lossy()).unwrap();
    // Like the reported case: a local branch the remote has never seen.
    branches::create(&repo.0, "develop").unwrap();

    let snapshot = repository::load_snapshot(&repo.0).unwrap();
    assert_eq!(snapshot.status.upstream, None);
    remote::push(&repo.0, &snapshot.status, snapshot.remote.as_deref()).unwrap();

    let after = repository::load_snapshot(&repo.0).unwrap();
    assert_eq!(after.status.upstream.as_deref(), Some("origin/develop"));
    assert!(git(&origin.0, &["branch", "--list", "develop"]).contains("develop"));
}

#[test]
fn publishing_before_the_first_commit_explains_why_not() {
    let origin = bare_remote();
    let repo = TempRepo::new();
    remote::add(&repo.0, "origin", &origin.0.to_string_lossy()).unwrap();
    let snapshot = repository::load_snapshot(&repo.0).unwrap();
    let error = remote::push(&repo.0, &snapshot.status, Some("origin")).unwrap_err();
    assert!(error.message.contains("first commit"), "{error}");
    assert!(remote::add(&repo.0, "origin", "  ").is_err());
}

#[test]
fn tags_create_push_and_delete() {
    let origin = bare_remote();
    let repo = TempRepo::new();
    repo.write("a.txt", "1\n");
    changes::commit(&repo.0, "Initial", &repo.changes(), true).unwrap();
    remote::add(&repo.0, "origin", &origin.0.to_string_lossy()).unwrap();
    let head = history::log(&repo.0).unwrap()[0].sha.clone();

    assert!(!tags::is_valid_name(&repo.0, "bad..tag"));
    assert!(tags::create(&repo.0, "-x", &head, "").is_err());
    tags::create(&repo.0, "v0.1.0", &head, "First release\n\nNotes").unwrap();
    tags::create(&repo.0, "light", &head, "").unwrap();

    let list = tags::list(&repo.0).unwrap();
    assert_eq!(list.len(), 2);
    assert!(list.iter().all(|tag| tag.commit == head), "annotated tags must point at the commit: {list:?}");
    assert_eq!(git(&repo.0, &["cat-file", "-t", "v0.1.0"]).trim(), "tag");
    assert_eq!(repository::load_snapshot(&repo.0).unwrap().tags.len(), 2);

    tags::push(&repo.0, "origin", "v0.1.0").unwrap();
    assert!(git(&origin.0, &["tag"]).contains("v0.1.0"));
    tags::delete(&repo.0, "v0.1.0", Some("origin")).unwrap();
    assert!(!git(&origin.0, &["tag"]).contains("v0.1.0"));
    tags::delete(&repo.0, "light", None).unwrap();
    assert!(tags::list(&repo.0).unwrap().is_empty());
}

#[test]
fn ignore_file_adds_an_anchored_escaped_pattern_once() {
    let repo = TempRepo::new();
    repo.write("report [draft].pdf", "x");
    repo.write(".gitignore", "target");
    let pattern = changes::ignore_pattern("report [draft].pdf");
    changes::ignore(&repo.0, &pattern).unwrap();
    changes::ignore(&repo.0, &pattern).unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.0.join(".gitignore")).unwrap(),
        "target\n/report \\[draft\\].pdf\n"
    );
    let left = repo.changes();
    assert!(left.iter().all(|c| c.path != "report [draft].pdf"), "{left:?}");
}

#[test]
fn huge_diffs_are_cut_without_reading_all_of_git_output() {
    let repo = TempRepo::new();
    repo.write("data.txt", "start\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    // About 8 MB of diff, four times the display limit.
    repo.write("data.txt", &"a line that changed in a generated file\n".repeat(200_000));
    let change = find(&repo.changes(), "data.txt").clone();
    let diff = changes::working_diff(&repo.0, &change, true).expect("a cut diff, not an error");
    assert!(diff.is_truncated);
}

/// main with one commit, and `feature` with one more commit that changes `shared.txt`.
fn repo_with_feature_branch() -> TempRepo {
    let repo = TempRepo::new();
    repo.write("shared.txt", "one\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    git(&repo.0, &["switch", "-c", "feature"]);
    repo.write("shared.txt", "one from feature\n");
    git(&repo.0, &["commit", "-am", "feature change"]);
    git(&repo.0, &["switch", "main"]);
    repo
}

#[test]
fn merge_preview_and_clean_merge() {
    let repo = repo_with_feature_branch();
    let preview = merge::preview(&repo.0, "feature").unwrap();
    assert_eq!(preview.commits, 1);
    assert_eq!(preview.conflicts, Some(Vec::new()));

    merge::merge(&repo.0, "feature").unwrap();
    assert_eq!(std::fs::read_to_string(repo.0.join("shared.txt")).unwrap(), "one from feature\n");
    assert_eq!(merge::preview(&repo.0, "feature").unwrap().commits, 0);
    assert!(merge::message(&repo.0).is_none());
}

#[test]
fn conflicting_merge_can_be_finished_or_aborted() {
    let repo = repo_with_feature_branch();
    repo.write("shared.txt", "one from main\n");
    git(&repo.0, &["commit", "-am", "main change"]);

    let preview = merge::preview(&repo.0, "feature").unwrap();
    assert_eq!(preview.conflicts, Some(vec!["shared.txt".to_owned()]));

    let error = merge::merge(&repo.0, "feature").unwrap_err();
    assert!(error.message.contains("conflicts"), "{}", error.message);
    let snapshot = repository::load_snapshot(&repo.0).unwrap();
    assert_eq!(snapshot.merge_message.as_deref(), Some("Merge branch 'feature'"));
    assert_eq!(merge::unresolved(&repo.0, &snapshot.changes), ["shared.txt"]);

    // Aborting goes back to main's version.
    merge::abort(&repo.0).unwrap();
    assert!(merge::message(&repo.0).is_none());
    assert_eq!(std::fs::read_to_string(repo.0.join("shared.txt")).unwrap(), "one from main\n");

    // Resolving and committing finishes the merge.
    let _ = merge::merge(&repo.0, "feature");
    repo.write("shared.txt", "one from both\n");
    let changes = repo.changes();
    assert!(merge::unresolved(&repo.0, &changes).is_empty());
    changes::commit(&repo.0, "Merge branch 'feature'", &changes, true).unwrap();
    assert!(merge::message(&repo.0).is_none());
    assert_eq!(merge::preview(&repo.0, "feature").unwrap().commits, 0);
}

fn branch(name: &str) -> github_desktop_enhanced::models::Branch {
    github_desktop_enhanced::models::Branch {
        name: name.to_owned(),
        is_remote: false,
        is_current: false,
        upstream: None,
    }
}

#[test]
fn switching_can_leave_changes_on_the_branch() {
    let repo = repo_with_feature_branch();
    repo.write("shared.txt", "work in progress on main\n");
    repo.write("new.txt", "untracked\n");

    branches::switch_leaving_changes(&repo.0, &branch("feature"), Some("main")).unwrap();
    assert_eq!(std::fs::read_to_string(repo.0.join("shared.txt")).unwrap(), "one from feature\n");
    assert!(!repo.0.join("new.txt").exists());
    let stashes = stash::list(&repo.0).unwrap();
    assert_eq!(stashes.len(), 1);
    assert_eq!(stashes[0].branch.as_deref(), Some("main"));

    // Back on main, the changes are restored from the stash left there.
    branches::switch(&repo.0, &branch("main")).unwrap();
    stash::pop(&repo.0, &stashes[0].reference).unwrap();
    assert_eq!(std::fs::read_to_string(repo.0.join("shared.txt")).unwrap(), "work in progress on main\n");
    assert!(repo.0.join("new.txt").exists());
}

#[test]
fn switching_can_bring_changes_unless_they_conflict() {
    let repo = repo_with_feature_branch();

    // A new file comes along to the other branch.
    repo.write("notes.txt", "notes\n");
    branches::switch_bringing_changes(&repo.0, &branch("feature"), Some("main")).unwrap();
    assert!(repo.0.join("notes.txt").exists());
    branches::switch(&repo.0, &branch("main")).unwrap();

    // A change to a file that differs on feature can't come along, and nothing is lost.
    repo.write("shared.txt", "edited on main\n");
    let error = branches::switch_bringing_changes(&repo.0, &branch("feature"), Some("main")).unwrap_err();
    assert!(error.message.contains("Leave my changes on main"), "{}", error.message);
    assert_eq!(std::fs::read_to_string(repo.0.join("shared.txt")).unwrap(), "edited on main\n");
}
