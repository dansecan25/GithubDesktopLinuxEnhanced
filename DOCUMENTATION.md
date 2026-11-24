# Documentation

How the app is built. For features and setup see [README.md](README.md). For rules and known issues
see [CONTEXT.md](CONTEXT.md).

## Stack

| Part | Choice | Why |
|---|---|---|
| UI | [egui / eframe](https://github.com/emilk/egui) 0.37 | Native Rust, immediate mode, Wayland and X11, AccessKit screen reader support |
| Git | The `git` CLI | Same approach as GitHub Desktop: the user's config, hooks, credential helpers and SSH keys work unchanged |
| Folder picker | [rfd](https://github.com/PolyMeilex/rfd) with the XDG portal backend | Native dialog on GNOME and KDE without linking GTK |
| Settings | `serde_json` in the platform config folder (`dirs`) | Plain, readable file |
| UI tests | [egui_kittest](https://github.com/emilk/egui/tree/main/crates/egui_kittest) (dev only) | Drives the real app headlessly and renders it |

## Structure

```
src/
  main.rs            Reads settings and flags, opens the window
  app.rs             Controller: turns actions into background tasks, task results into state
  constants/         Fixed values: app ids, layout sizes, git limits, every UI string (text.rs)
  models/            Plain data: FileChange, Branch, CommitSummary, StashEntry, Diff, RepoSnapshot
  git/               Git operations, one module per area
    runner.rs        The only place that starts `git`
    parse.rs         Pure parsers for git's machine-readable output (unit tested)
    repository.rs    Find root, init, clone, load the snapshot
    changes.rs       Working-tree diff, commit, discard
    history.rs       Log, files in a commit, diff of a file in a commit
    branches.rs      List, switch, create, delete, sanitize names
    remote.rs        Fetch, pull, push / publish
    stash.rs         List, stash all, restore, drop
  services/          Non-UI services
    tasks.rs         Background threads and the result channel
    settings.rs      Load and atomically save settings.json
    window_state.rs  Window size validation and recording (the shrinking-window fix)
    system.rs        Open a folder in the file manager
  stores/            UI state for the open repository (RepoStore) and the Loadable wrapper
  ui/
    actions.rs       Every user intent as an Action enum
    theme.rs         Design system: colour palette, spacing, fonts
    shortcuts.rs     Keyboard shortcuts
    components/      Shared widgets: buttons, file row, diff view, dialogs frame, messages
    dialogs/         Add, clone, create repository, create branch, confirmation
    views/           Toolbar, Changes, History, welcome, and the two-pane layout
  utils/             Small pure helpers (path display)
tests/
  git_operations.rs  Integration tests against real throwaway repositories
  ui_screens.rs      Headless render of the main screens (ignored by default, needs a GPU or lavapipe)
packaging/           Desktop entry, icon, per-user install script
release.sh           Checks, tests, release build and package. Used locally and by CI
```

## Data flow

1. Views draw from the state and push `Action`s. They never run git.
2. `app.rs` handles each action. Anything that touches git runs on a background thread through
   `services::tasks::Tasks`, so the window never blocks.
3. When a task finishes it sends a `TaskOutput` back and requests a repaint. The next frame applies
   it to the `RepoStore`. Results for a repository, file or commit that is no longer selected are
   dropped.
4. Only one repository-changing operation (commit, push, pull, …) runs at a time. While it runs,
   the toolbar shows a spinner and the buttons that change the repository are disabled.
5. After an operation finishes, the snapshot (status, branches, stashes, remote) and the history are
   reloaded. The app also refreshes when its window regains focus, so edits made elsewhere show up.

## Git commands

Every call sets `GIT_TERMINAL_PROMPT=0`, and `GIT_SSH_COMMAND="ssh -o BatchMode=yes"` unless the
user has set their own, so git fails with a message instead of waiting for input nobody can type.

| Feature | Command |
|---|---|
| Status | `git status --porcelain=v2 --branch -z --untracked-files=all` |
| Branches | `git for-each-ref refs/heads refs/remotes` with a field-separated format |
| Stashes | `git stash list` with a field-separated format |
| Working diff | `git diff -M HEAD -- <path>` (the empty tree hash before the first commit). New files are read from disk |
| Commit all | `git add --all`, then `git commit --file -` (message on stdin) |
| Commit some | `git add --all -- <paths>`, then `git commit --file - --only -- <paths>` |
| Discard | `git clean -f` for new files, `git rm -f` for added files, `git restore --source=HEAD --staged --worktree` for the rest |
| History | `git log --max-count=500` with a field- and record-separated format |
| Commit files | `git diff-tree -r -z --name-status --root -m --first-parent <sha>` |
| Commit diff | `git show --format= -m --first-parent <sha> -- <path>` |
| Switch | `git switch <branch>`, or `git switch --track <remote/branch>` for remote branches |
| New branch | `git check-ref-format --branch`, then `git switch --create` |
| Delete branch | `git branch --delete` (git refuses unmerged branches) |
| Fetch / Pull | `git fetch --all --prune` / `git pull` (uses the user's pull.rebase setting) |
| Push | `git push`, or `git push --set-upstream <remote> <branch>` to publish |
| Stash | `git stash push --include-untracked`, `git stash pop`, `git stash drop` |
| Clone | `git clone -- <url> <parent>/<name>` |
| Create | `git init --initial-branch main` |

## Settings file

`~/.config/github-desktop-enhanced/settings.json` (`$XDG_CONFIG_HOME` is respected):

| Key | Meaning |
|---|---|
| `repositories` | Paths of the repositories in the list |
| `selected_repository` | The one opened on start |
| `window` | `width`, `height` (logical points) and `maximized` |
| `theme` | `System`, `Light` or `Dark` |
| `projects_directory` | Default folder for clones and new repositories (home folder when empty) |

A missing or unreadable file falls back to defaults. Writes go to a temporary file that is then
renamed, so a crash can't leave a half-written file.

## Window size handling

The old port saved whatever size the window reported, including the shrunken sizes some compositors
report while a window is minimized, restoring, or changing scale. Each launch started a little
smaller. Here (`services/window_state.rs`):

- A size is recorded only when the window is focused, not minimized, not fullscreen and not
  maximized. Maximized is stored as a flag, separately from the size.
- Sizes below the minimum (360 × 480) are never recorded.
- On load, any size that is missing, not a number, below the minimum or larger than the monitor is
  replaced with the default, and the corrected value is saved straight away.
- The window manager also enforces the minimum, through the window's minimum inner size.
- eframe's own window persistence is turned off, so there is a single source of truth.
- `--reset-window` ignores the saved size for one launch.

## Layout

At 760 px and wider, the window has a resizable sidebar (Changes or History list) and a detail pane
(diff or commit). Below that, it shows one column: the list, and the detail with a Back button when
you pick a file or commit. The toolbar wraps onto a second row when it doesn't fit. Buttons and list
rows are at least 40 px tall.
