//! Runs the git layer against real throwaway repositories.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use github_desktop_enhanced::git::{
    branches, changes, conflicts, history, merge, partial, remote, repository, stash, tags,
};
use github_desktop_enhanced::models::{ChangeKind, ConflictChoice, FileChange, LineSelection};

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

#[test]
fn huge_untracked_build_output_is_not_stashed_or_committed() {
    let repo = TempRepo::new();
    repo.write("README.md", "hi\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    // Sparse: 1.5 GB on paper, nothing on disk. Git is never asked to read it.
    std::fs::create_dir_all(repo.0.join("target")).unwrap();
    let file = std::fs::File::create(repo.0.join("target/app.bin")).unwrap();
    file.set_len(1536 * 1024 * 1024).unwrap();

    let error = stash::push_all(&repo.0, "wip").unwrap_err();
    assert!(error.message.contains("target/"), "{}", error.message);
    let changes = repo.changes();
    let error = changes::commit(&repo.0, "add build", &changes, true).unwrap_err();
    assert!(error.message.contains("target/"), "{}", error.message);
    assert!(repo.0.join("target/app.bin").exists());
    assert!(stash::list(&repo.0).unwrap().is_empty());
}

#[test]
fn clone_reports_progress() {
    use github_desktop_enhanced::git::control::{OperationControl, with_control};
    let source = TempRepo::new();
    source.write("a.txt", &"some content\n".repeat(1000));
    git(&source.0, &["add", "."]);
    git(&source.0, &["commit", "-m", "first"]);
    let parent = TempRepo::new();
    let control = std::sync::Arc::new(OperationControl::default());
    // file:// makes git transfer a pack, as over the network, instead of copying files.
    let url = format!("file://{}", source.0.display());
    let cloned = with_control(control.clone(), || repository::clone(&url, &parent.0)).unwrap();
    assert!(cloned.join("a.txt").exists());
    assert!(control.progress().is_some(), "git reported no progress");
}

#[test]
fn cancelling_stops_a_running_push() {
    use github_desktop_enhanced::git::control::{OperationControl, with_control};
    let remote = bare_remote();
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    git(&repo.0, &["remote", "add", "origin", &remote.0.to_string_lossy()]);
    // A hook that would keep the push waiting for a minute.
    let hook = repo.0.join(".git/hooks/pre-push");
    std::fs::write(&hook, "#!/bin/sh\nsleep 60\n").unwrap();
    Command::new("chmod").arg("+x").arg(&hook).status().unwrap();

    let control = std::sync::Arc::new(OperationControl::default());
    let canceller = control.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        canceller.cancel();
    });
    let started = std::time::Instant::now();
    let status = repository::load_snapshot(&repo.0).unwrap().status;
    let error = with_control(control, || remote::push(&repo.0, &status, Some("origin"))).unwrap_err();
    assert!(error.cancelled, "{}", error.message);
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
}

fn head_content(repo: &TempRepo, file: &str) -> String {
    let output =
        Command::new("git").current_dir(&repo.0).args(["show", &format!("HEAD:{file}")]).output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/// Picks lines of `file`'s working diff: `leave_out` lists the texts (with +/-) to leave out.
fn pick(repo: &TempRepo, file: &str, leave_out: &[&str]) -> (FileChange, LineSelection) {
    let change = find(&repo.changes(), file).clone();
    let diff = changes::working_diff(&repo.0, &change, true).unwrap();
    let mut selection = LineSelection::new(diff);
    let indexes: Vec<usize> = leave_out
        .iter()
        .map(|text| selection.diff.lines.iter().position(|line| line.text == *text).expect(text))
        .collect();
    for index in indexes {
        selection.toggle_line(index);
    }
    (change, selection)
}

#[test]
fn commits_only_the_chosen_lines() {
    let repo = TempRepo::new();
    let original: String = (1..=20).map(|n| format!("line {n}\n")).collect();
    repo.write("f.txt", &original);
    repo.write("other.txt", "other\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    // Two separate hunks, and a second file committed whole.
    let edited = original.replace("line 2\n", "line two\n").replace("line 19\n", "line nineteen\n");
    repo.write("f.txt", &edited);
    repo.write("other.txt", "other changed\n");

    let partial_file = pick(&repo, "f.txt", &["-line 19", "+line nineteen"]);
    let chosen = repo.changes();
    partial::commit_with_lines(&repo.0, "first hunk only", &chosen, &[partial_file], true).unwrap();

    assert_eq!(head_content(&repo, "f.txt"), original.replace("line 2\n", "line two\n"));
    assert_eq!(head_content(&repo, "other.txt"), "other changed\n");
    // What was left out is still there, uncommitted.
    assert_eq!(std::fs::read_to_string(repo.0.join("f.txt")).unwrap(), edited);
    let left = repo.changes();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].path, "f.txt");
}

#[test]
fn leaving_out_a_removal_keeps_the_old_line() {
    let repo = TempRepo::new();
    repo.write("f.txt", "keep\nold\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    repo.write("f.txt", "keep\nnew\n");
    let partial_file = pick(&repo, "f.txt", &["-old"]);
    partial::commit_with_lines(&repo.0, "add new", &repo.changes(), &[partial_file], true).unwrap();
    assert_eq!(head_content(&repo, "f.txt"), "keep\nold\nnew\n");
}

#[test]
fn new_files_can_be_committed_in_part() {
    let repo = TempRepo::new();
    repo.write("README.md", "hi\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    repo.write("new.txt", "a\nb\nc");
    let partial_file = pick(&repo, "new.txt", &["+b"]);
    partial::commit_with_lines(&repo.0, "part of new", &repo.changes(), &[partial_file], true).unwrap();
    assert_eq!(head_content(&repo, "new.txt"), "a\nc");
}

#[test]
fn a_file_changed_after_picking_lines_is_not_committed() {
    let repo = TempRepo::new();
    repo.write("f.txt", "a\nb\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    repo.write("f.txt", "A\nB\n");
    let partial_file = pick(&repo, "f.txt", &["-b", "+B"]);
    repo.write("f.txt", "A\nB\nC\n");
    let error =
        partial::commit_with_lines(&repo.0, "stale", &repo.changes(), &[partial_file], true).unwrap_err();
    assert!(error.message.contains("changed after you picked"), "{}", error.message);
    assert_eq!(head_content(&repo, "f.txt"), "a\nb\n");
}

/// A merge of `feature` into main stopped on a conflict in shared.txt.
fn stopped_merge() -> TempRepo {
    let repo = repo_with_feature_branch();
    repo.write("shared.txt", "one from main\n");
    git(&repo.0, &["commit", "-am", "main change"]);
    let _ = merge::merge(&repo.0, "feature");
    assert_eq!(find(&repo.changes(), "shared.txt").kind, ChangeKind::Conflicted);
    repo
}

#[test]
fn conflicts_resolved_block_by_block_finish_the_merge() {
    let repo = stopped_merge();
    let file = conflicts::read(&repo.0, "shared.txt").unwrap();
    assert_eq!(file.conflict_count(), 1);
    assert!(conflicts::mark_resolved(&repo.0, "shared.txt").is_err(), "markers are still there");

    conflicts::choose(&repo.0, "shared.txt", 0, ConflictChoice::Both).unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.0.join("shared.txt")).unwrap(),
        "one from main\none from feature\n"
    );
    conflicts::mark_resolved(&repo.0, "shared.txt").unwrap();
    let changes = repo.changes();
    assert_ne!(find(&changes, "shared.txt").kind, ChangeKind::Conflicted);
    changes::commit(&repo.0, "Merge branch 'feature'", &changes, true).unwrap();
    assert!(merge::message(&repo.0).is_none());
}

#[test]
fn a_whole_side_can_be_taken() {
    let repo = stopped_merge();
    conflicts::take_side(&repo.0, "shared.txt", false).unwrap();
    assert_eq!(std::fs::read_to_string(repo.0.join("shared.txt")).unwrap(), "one from feature\n");
    assert_ne!(find(&repo.changes(), "shared.txt").kind, ChangeKind::Conflicted);
}

#[test]
fn taking_the_side_that_deleted_the_file_deletes_it() {
    let repo = repo_with_feature_branch();
    git(&repo.0, &["switch", "feature"]);
    git(&repo.0, &["rm", "-q", "shared.txt"]);
    git(&repo.0, &["commit", "-m", "delete on feature"]);
    git(&repo.0, &["switch", "main"]);
    repo.write("shared.txt", "changed on main\n");
    git(&repo.0, &["commit", "-am", "main change"]);
    let _ = merge::merge(&repo.0, "feature");

    conflicts::take_side(&repo.0, "shared.txt", false).unwrap();
    assert!(!repo.0.join("shared.txt").exists());
    assert!(repo.changes().iter().all(|change| change.kind != ChangeKind::Conflicted));
}

fn pull(number: u64, head: &str, same_repository: bool) -> github_desktop_enhanced::models::PullRequest {
    github_desktop_enhanced::models::PullRequest {
        number,
        title: "A change".into(),
        body: String::new(),
        author: "sam".into(),
        head_branch: head.into(),
        base_branch: "main".into(),
        head_sha: String::new(),
        same_repository,
        draft: false,
        url: String::new(),
        created_at: String::new(),
    }
}

#[test]
fn pull_request_branches_can_be_checked_out() {
    let remote_repo = bare_remote();
    let repo = TempRepo::new();
    repo.write("a.txt", "a\n");
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    git(&repo.0, &["remote", "add", "origin", &remote_repo.0.to_string_lossy()]);
    git(&repo.0, &["push", "-q", "origin", "main"]);
    // A branch on the remote, and a fork's pull request, which GitHub keeps under refs/pull/.
    git(&repo.0, &["switch", "-q", "-c", "feature"]);
    repo.write("a.txt", "feature\n");
    git(&repo.0, &["commit", "-qam", "feature"]);
    git(&repo.0, &["push", "-q", "origin", "feature", "feature:refs/pull/9/head"]);
    git(&repo.0, &["switch", "-q", "main"]);
    git(&repo.0, &["branch", "-q", "-D", "feature"]);

    let branch = remote::fetch_pull(&repo.0, "origin", &pull(3, "feature", true), &["main".into()]).unwrap();
    assert_eq!((branch.name.as_str(), branch.is_remote), ("origin/feature", true));
    branches::switch(&repo.0, &branch).unwrap();
    assert_eq!(std::fs::read_to_string(repo.0.join("a.txt")).unwrap(), "feature\n");

    git(&repo.0, &["switch", "-q", "main"]);
    let fork = remote::fetch_pull(&repo.0, "origin", &pull(9, "their-branch", false), &[]).unwrap();
    assert_eq!((fork.name.as_str(), fork.is_remote), ("pr/9", false));
    branches::switch(&repo.0, &fork).unwrap();
    assert_eq!(std::fs::read_to_string(repo.0.join("a.txt")).unwrap(), "feature\n");
}

fn png(color: [u8; 4], width: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, 10, image::Rgba(color));
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Png)
        .unwrap();
    bytes
}

#[test]
fn image_changes_have_before_and_after_previews() {
    use github_desktop_enhanced::models::ImageSide;
    use github_desktop_enhanced::services::image_preview;
    let repo = TempRepo::new();
    std::fs::write(repo.0.join("logo.png"), png([255, 0, 0, 255], 20)).unwrap();
    git(&repo.0, &["add", "."]);
    git(&repo.0, &["commit", "-m", "logo"]);
    std::fs::write(repo.0.join("logo.png"), png([0, 0, 255, 255], 40)).unwrap();

    let change = find(&repo.changes(), "logo.png").clone();
    let pair = image_preview::working(&repo.0, &change, true);
    let (ImageSide::Image(before), ImageSide::Image(after)) = (&pair.before, &pair.after) else {
        panic!("both sides should be pictures: {pair:?}");
    };
    assert_eq!((before.width, after.width), (20, 40));

    // In the first commit there is nothing before.
    let sha = git(&repo.0, &["rev-parse", "HEAD"]);
    let first = image_preview::in_commit(&repo.0, sha.trim(), "logo.png");
    assert_eq!(first.before, ImageSide::Missing);
    assert!(matches!(first.after, ImageSide::Image(_)));
}

#[test]
fn project_versions_come_from_the_tagged_commit() {
    let repo = TempRepo::new();
    let write = |path: &str, content: &str| {
        let file = repo.0.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, content).unwrap();
    };
    // A Flutter app one folder down, like free_player, plus files that must be ignored.
    write("README.md", "player\n");
    write("free_music_player/pubspec.yaml", "name: player\nversion: 1.0.0+4\n");
    write("free_music_player/build/web/package.json", r#"{"version": "9.9.9"}"#);
    write("node_modules/left-pad/package.json", r#"{"version": "1.3.0"}"#);
    write("tools/scripts/deep/Cargo.toml", "[package]\nversion = \"5.0.0\"\n");
    git(&repo.0, &["add", "-f", "."]);
    git(&repo.0, &["commit", "-m", "first"]);
    let first = git(&repo.0, &["rev-parse", "HEAD"]).trim().to_owned();
    write("free_music_player/pubspec.yaml", "name: player\nversion: 1.1.0+5\n");
    git(&repo.0, &["commit", "-am", "bump"]);

    let versions = tags::project_versions(&repo.0, &first).unwrap();
    let found: Vec<(&str, &str)> = versions.iter().map(|v| (v.path.as_str(), v.version.as_str())).collect();
    assert_eq!(found, [("free_music_player/pubspec.yaml", "1.0.0")]);
    let head = tags::project_versions(&repo.0, "HEAD").unwrap();
    assert_eq!(head[0].version, "1.1.0");
}
