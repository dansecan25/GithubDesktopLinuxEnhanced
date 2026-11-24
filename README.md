# GitHub Desktop Linux Enhanced

A GitHub Desktop style git client for Linux, written in Rust. It started because the unofficial
GitHub Desktop port for Linux has a bug where the window shrinks a little on every launch until it
can't be opened at all. This app is a native rewrite that fixes that, with first-class Linux support
(Wayland and X11), and it will add a pull request view so you can create and comment on PRs without
opening the browser.

It is not affiliated with GitHub. It uses your installed `git`, so your config, hooks, credential
helper and SSH keys work as they do in the terminal.

## Features

Available now (the GitHub Desktop basics):

- **Repositories**: add an existing local repository, clone one (full URL, SSH address or the
  `owner/repo` shorthand), create a new one, switch between them, and remove them from the list.
- **Changes**: see every changed file with a coloured diff, choose which files go into the commit
  with checkboxes, and commit with a summary and description.
- **Discard**: throw away the changes to one file or to all of them, after confirming.
- **Stash**: stash all changes (including new files), restore a stash, or discard it.
- **History**: browse the last 500 commits on the current branch, and see each commit's files and
  diffs.
- **Branches**: switch, filter, create (names are cleaned up the way GitHub Desktop does) and delete
  local branches, and check out remote branches.
- **Sync**: one button that publishes, pulls, pushes or fetches depending on the branch, with an
  ahead/behind count.
- **Light, dark and system themes**, a layout that works from a narrow 360 px window up to a wide
  screen, keyboard shortcuts, and screen reader labels.
- **A window that can't get lost**: sizes are only saved when the window is visible and in a normal
  state, and every saved size is checked again before use. See [Window size](#window-size).

Planned:

- **Pull requests**: list the open PRs on the repository, create a PR from the current branch, and
  read and write comments.
- Conflict resolution helpers, partial (line-by-line) commits, and amend/undo last commit.
- Self-updating from GitHub releases.

## Install

### From a release

Download `github-desktop-enhanced-<version>-linux-x86_64.tar.gz` from the
[releases page](https://github.com/dansecan25/GithubDesktopLinuxEnhanced/releases), then:

```sh
tar -xzf github-desktop-enhanced-*-linux-x86_64.tar.gz
cd github-desktop-enhanced-*-linux-x86_64
./install.sh               # installs to ~/.local, no sudo
./install.sh --uninstall   # removes it again (keeps your settings)
```

### From source

You need Rust 1.95 or newer ([rustup](https://rustup.rs)) and `git` 2.23 or newer.

```sh
git clone https://github.com/dansecan25/GithubDesktopLinuxEnhanced.git
cd GithubDesktopLinuxEnhanced
cargo run --release
```

`./release.sh` runs the checks and tests, builds in release mode, and puts an installable package in
`dist/`.

## Usage

Start it from your app menu or run `github-desktop-enhanced`. Pushing and pulling use the same
credentials as `git` in a terminal. If `git push` works there, it works here. The app never asks
for passwords itself: set up a [credential helper](https://git-scm.com/doc/credential-helpers) for
HTTPS, or load your key into `ssh-agent` for SSH.

### Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Ctrl+Enter | Commit |
| Ctrl+1 / Ctrl+2 | Changes / History tab |
| F5 | Refresh |
| Ctrl+P | Push |
| Ctrl+Shift+P | Pull |
| Ctrl+Shift+T | Fetch |
| Ctrl+Shift+N | New branch |
| Ctrl+N | New repository |
| Ctrl+O | Add existing repository |
| Ctrl+Shift+O | Clone repository |

### Window size

If the window ever opens at a size you don't want, start it once with:

```sh
github-desktop-enhanced --reset-window
```

The app menu entry also has an **Open with default window size** action. Settings are stored in
`~/.config/github-desktop-enhanced/settings.json`.

## Development

```sh
cargo test                     # unit tests and git integration tests
cargo clippy --all-targets     # lints
cargo fmt                      # formatting
GDE_SCREENSHOT_DIR=/tmp/shots cargo test --test ui_screens -- --ignored   # renders the screens to PNG
```

How the code is organised is described in [DOCUMENTATION.md](DOCUMENTATION.md). Project rules and
known issues are in [CONTEXT.md](CONTEXT.md).
