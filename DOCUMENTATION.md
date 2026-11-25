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
| GitHub API and updates | [ureq](https://github.com/algesten/ureq) | Small blocking HTTP client with rustls, run on background threads |
| GitHub token | [keyring](https://github.com/open-source-cooperative/keyring-rs) | System keychain (Secret Service on Linux), so the token never touches a file |
| UI tests | [egui_kittest](https://github.com/emilk/egui/tree/main/crates/egui_kittest) (dev only) | Drives the real app headlessly and renders it |

## Structure

```
src/
  main.rs            Askpass helper mode, --version, reads settings and flags, opens the window
  app/               Controller
    mod.rs           State, startup and the per-frame loop
    handlers.rs      What each Action does (starts background tasks)
    results.rs       Applies finished tasks to the state
    window.rs        Window size tracking, refresh on focus, keyboard shortcuts
  constants/         Fixed values: app ids, layout, git limits, GitHub/update values, known editors,
                     every UI string (text.rs)
  models/            Plain data: FileChange, Branch, CommitSummary, StashEntry, Tag, Diff, RepoSnapshot
  git/               Git operations, one module per area
    runner.rs        The only place that starts `git`; adds the askpass environment, caps output, times out local commands
    parse.rs         Pure parsers for git's machine-readable output, binary detection (unit tested)
    repository.rs    Find root, init, clone, load the snapshot
    changes.rs       Working-tree diff, commit, discard, .gitignore
    history.rs       Log, files in a commit, diff of a file in a commit
    branches.rs      List, switch, create, delete, sanitize names
    remote.rs        Fetch, pull, push / publish, add a remote
    stash.rs         List, stash all, restore, drop
    tags.rs          List, create (lightweight or annotated), push, delete
    merge.rs         Merge preview (commits, conflicts), merge, abort, unresolved conflict check
  github/            GitHub REST API: check a token, create a repository, latest release
  services/          Non-UI services
    tasks.rs         Background threads and the result channel
    askpass.rs       Credential bridge between git/ssh and the app (see below)
    secrets.rs       GitHub token in the system keychain
    editor.rs        Detect installed editors, open paths in one
    update.rs        Check, download and install new versions
    settings.rs      Load and atomically save settings.json
    window_state.rs  Window size validation and recording (the shrinking-window fix)
    system.rs        Open with the desktop's default app; start programs without zombies
    memory_cap.rs    Relaunch inside a memory-limited systemd scope; start other programs outside it
    watchdog.rs      Freeze and memory reports, and closing the app before it slows the computer
  stores/            UI state for the open repository (RepoStore) and the Loadable wrapper
  ui/
    actions.rs       Every user intent as an Action enum
    theme.rs         Design system: colour palette, spacing, fonts
    shortcuts.rs     Keyboard shortcuts
    components/      Shared widgets: buttons, file row, diff view, dialog frame, folder picker
    dialogs/         Repository, branch, confirmation, preferences, publish, tag and credential dialogs
    views/           Toolbar, Changes, History, welcome, right-click menus, update bar, layout
  utils/             Small pure helpers (path display, version numbers)
tests/
  git_operations.rs  Integration tests against real throwaway repositories and bare remotes
  askpass.rs         The credential bridge end to end, with the real binary and real git
  ui_screens.rs      Headless render of the screens, menus and dialogs (ignored by default)
  readme_screenshots.rs  Renders docs/screenshots/ from a made-up project (ignored by default)
docs/screenshots/    README images
packaging/           Desktop entry, icon, per-user install script (also used by the updater)
release.sh           Checks, tests, release build and package. Used locally and by CI
limited.sh           Runs a build or test with memory and CPU limits
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

## Credentials

Git runs in the background with no terminal, so it can't prompt there (`GIT_TERMINAL_PROMPT=0`).
Instead, like GitHub Desktop, the app is its own askpass helper:

1. At startup `services/askpass.rs` listens on a Unix socket in the user's private runtime folder
   (`$XDG_RUNTIME_DIR`, mode 0600), and the runner sets `GIT_ASKPASS` and `SSH_ASKPASS` (with
   `SSH_ASKPASS_REQUIRE=force`) to the app's own executable, plus the socket path and a random
   token.
2. When git or ssh needs something, it starts the executable with the prompt as its argument.
   `main.rs` sees the socket variable, sends the prompt and token to the running app, prints the
   answer and exits. No window opens.
3. The app answers GitHub username and password prompts itself when signed in (login and token).
   Anything else (another host, an SSH passphrase, an unknown host key) is shown as a dialog.
   Cancelling makes the git command fail with git's own error.

The user's credential helper and `ssh-agent` are still used first, because git only asks the helper
when they have nothing. Without the server (tests, failure to start), ssh runs with
`BatchMode=yes` so it fails instead of hanging.

## Updates

`services/update.rs` follows the same flow as the owner's other desktop apps: on start and every 12
hours, read the latest release of this repository, compare `vX.Y.Z` with the running version,
download `github-desktop-enhanced-<version>-<os>-<arch>.tar.gz` into the cache folder, unpack it,
and offer to run its `install.sh`. The installer first runs the new binary with `--version`, then
renames it over the old one, which works while the old one is running. **Restart now** starts the
installed copy and closes this one. Only the copy at `~/.local/bin/github-desktop-enhanced`
updates itself.

## Git commands

Every call sets `GIT_TERMINAL_PROMPT=0` and the askpass variables described under
[Credentials](#credentials).

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
| Add remote | `git remote add -- origin <url>` |
| Publish to GitHub | `POST /user/repos`, then add `origin` and push with `--set-upstream` |
| Ignore file | Appends `/<escaped path>` to `.gitignore` |
| Tags | `git for-each-ref refs/tags --sort=-creatordate` (annotated tags are peeled to their commit) |
| New tag | `git check-ref-format refs/tags/<name>`, then `git tag` or `git tag --annotate --file -` |
| Push / delete tag | `git push <remote> refs/tags/<name>` / `git push <remote> --delete refs/tags/<name>` and `git tag --delete` |

## Settings file

`~/.config/github-desktop-enhanced/settings.json` (`$XDG_CONFIG_HOME` is respected):

| Key | Meaning |
|---|---|
| `repositories` | Paths of the repositories in the list |
| `selected_repository` | The one opened on start |
| `window` | `width`, `height` (logical points) and `maximized` |
| `theme` | `System`, `Light` or `Dark` |
| `projects_directory` | Folder last used to clone or create a repository (home folder when empty) |
| `editor` | Command for "Open in editor", or empty for the first installed editor |
| `github_login` | Login of the signed-in account. The token itself is in the keychain |
| `auto_update` | Check for and download updates automatically (default on) |

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

## Diffs

Diffs of files that aren't text (a NUL byte near the start, or not valid UTF-8) are shown as binary.
Lines longer than 2000 bytes and diffs larger than 2 MB are cut, and only the rows on screen are laid
out, so a large generated or downloaded file can't slow the window down.
